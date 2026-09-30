mod model;
mod parse;
mod script;

use async_trait::async_trait;
use sib_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings,
    QueryRequest, QueryResponse, Sample, Schedule, Severity, Snapshot, Transport,
};

pub use model::{Commit, GitSnapshot, Repository, SHORT_HASH_LEN, Upstream};
pub use script::{COMMITS_PER_REPOSITORY, HISTORY_COMMITS};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("git");
pub const KEY_REPOSITORIES: &str = "git.repositories";
pub const KEY_DIRTY: &str = "git.dirty";
pub const QUERY_HISTORY: &str = "history";
pub const HISTORY_SEPARATOR: &str = "  ";

const VERSION_COMMAND: &str = "git --version";

pub struct GitModule;

#[async_trait]
impl Module for GitModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Git"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Slow
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        let parts = [
            ("version", VERSION_COMMAND),
            (
                "first",
                &format!("{} | head -1", script::find_repositories()),
            ),
        ];
        let output = transport.exec(&sections::script(&parts)).await?;
        let found = sections::Sections::parse(&output.stdout);
        if found.get_or_empty("version").trim().is_empty() {
            return Ok(Availability::unavailable("git is not installed"));
        }
        if found.get_or_empty("first").trim().is_empty() {
            return Ok(Availability::unavailable("no repositories found"));
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let collect = script::collect();
        let parts = [("version", VERSION_COMMAND), ("repos", collect.as_str())];
        let output = transport.exec(&sections::script(&parts)).await?;
        let snapshot = parse::git_snapshot(&output.stdout)?;
        let events = context
            .previous::<GitSnapshot>()
            .map(|(previous, _)| events_between(previous, &snapshot))
            .unwrap_or_default();
        let samples = vec![
            Sample::new(KEY_REPOSITORIES, snapshot.repositories.len() as f64),
            Sample::new(KEY_DIRTY, snapshot.dirty_count() as f64),
        ];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }

    async fn query(
        &self,
        transport: &dyn Transport,
        request: &QueryRequest,
    ) -> Result<QueryResponse, ModuleError> {
        if request.kind != QUERY_HISTORY {
            return Err(ModuleError::UnsupportedQuery(request.kind.clone()));
        }
        if !is_repository_path(&request.target) {
            return Err(ModuleError::UnsupportedQuery(format!(
                "{QUERY_HISTORY} {}",
                request.target
            )));
        }
        let output = transport.exec(&script::history(&request.target)).await?;
        Ok(QueryResponse {
            title: format!("git log {}", request.target),
            text: render_history(&parse::commits(&output.stdout)),
        })
    }
}

fn is_repository_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains("..") && !path.chars().any(char::is_control)
}

fn render_history(commits: &[Commit]) -> String {
    commits
        .iter()
        .map(|commit| {
            let refs = if commit.refs.is_empty() {
                String::new()
            } else {
                format!(" ({})", commit.refs.join(", "))
            };
            format!(
                "{}{HISTORY_SEPARATOR}{}{HISTORY_SEPARATOR}{}{HISTORY_SEPARATOR}{}{refs}",
                commit.short_hash(),
                commit.at.format("%Y-%m-%d %H:%M"),
                commit.author,
                commit.subject
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn events_between(previous: &GitSnapshot, current: &GitSnapshot) -> Vec<Event> {
    current
        .repositories
        .iter()
        .filter_map(|repository| {
            let before = previous.find(&repository.path)?;
            repository_event(before, repository)
        })
        .collect()
}

fn repository_event(before: &Repository, after: &Repository) -> Option<Event> {
    if before.branch != after.branch {
        let branch = after.branch.as_deref().unwrap_or("detached HEAD");
        let message = format!("{}: switched to {branch}", after.name());
        return Some(Event::new(ID, Severity::Info, message));
    }
    if before.head == after.head {
        return None;
    }
    let commit = after.last_commit()?;
    let message = format!(
        "{}: new commit {} {} - {}",
        after.name(),
        commit.short_hash(),
        commit.author,
        commit.subject
    );
    Some(Event::new(ID, Severity::Info, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> GitSnapshot {
        parse::git_snapshot(include_str!("../../fixtures/git/ubuntu.txt")).expect("fixture")
    }

    #[test]
    fn unchanged_snapshot_raises_no_events() {
        let snapshot = fixture();
        assert!(events_between(&snapshot, &snapshot).is_empty());
    }

    #[test]
    fn new_head_raises_commit_event() {
        let before = fixture();
        let mut after = before.clone();
        let api = &mut after.repositories[0];
        api.head = Some("ffffffffff".to_owned());
        api.commits.insert(
            0,
            Commit {
                hash: "ffffffffff".to_owned(),
                author: "Anna".to_owned(),
                at: api.commits[0].at,
                refs: Vec::new(),
                subject: "deploy".to_owned(),
            },
        );
        let events = events_between(&before, &after);
        assert_eq!(events.len(), 1);
        assert!(
            events[0]
                .message
                .contains("api: new commit fffffff Anna - deploy")
        );
    }

    #[test]
    fn branch_switch_raises_switch_event() {
        let before = fixture();
        let mut after = before.clone();
        after.repositories[0].branch = Some("master".to_owned());
        let events = events_between(&before, &after);
        assert_eq!(events.len(), 1);
        assert!(events[0].message.contains("switched to master"));
    }

    #[test]
    fn history_query_rejects_relative_path() {
        assert!(!is_repository_path("srv/app"));
        assert!(!is_repository_path("/srv/../etc"));
        assert!(is_repository_path("/srv/app"));
    }

    #[test]
    fn render_history_joins_fields_with_double_space() {
        let text = render_history(&fixture().repositories[1].commits);
        let first = text.lines().next().expect("line");
        assert!(first.starts_with("0cecdd9  "));
        assert!(first.ends_with("fix checkout (HEAD, origin/master)"));
    }
}
