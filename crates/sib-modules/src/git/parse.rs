use chrono::DateTime;
use sib_core::ModuleError;

use super::model::{Commit, GitSnapshot, Repository, Upstream};
use super::script::{FIELD_MARKER, REPOSITORY_MARKER};
use crate::common::sections::Sections;

const NO_BRANCH: &str = "HEAD";
const DETACHED_PREFIX: char = '(';

pub fn git_snapshot(raw: &str) -> Result<GitSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let version = sections.get_or_empty("version").trim();
    if version.is_empty() {
        return Err(ModuleError::Parse("git --version is empty".to_owned()));
    }
    Ok(GitSnapshot {
        version: version
            .strip_prefix("git version ")
            .unwrap_or(version)
            .to_owned(),
        repositories: repositories(sections.get_or_empty("repos")),
    })
}

pub fn repositories(raw: &str) -> Vec<Repository> {
    raw.split(REPOSITORY_MARKER)
        .filter(|block| !block.trim().is_empty())
        .filter_map(repository)
        .collect()
}

pub fn commits(raw: &str) -> Vec<Commit> {
    raw.lines().filter_map(commit_line).collect()
}

fn repository(block: &str) -> Option<Repository> {
    let (path, rest) = block.split_once('\n')?;
    let fields = Fields::parse(rest);
    let head = fields.line("head").filter(|h| *h != NO_BRANCH);
    let branch = fields.line("branch").filter(|b| *b != NO_BRANCH);
    Some(Repository {
        path: path.trim().to_owned(),
        branch: branch.map(str::to_owned),
        head: head.map(str::to_owned),
        remote: fields.line("remote").map(str::to_owned),
        upstream: fields.line("upstream").and_then(upstream),
        changed_files: fields.lines("status"),
        stashes: fields
            .line("stash")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        branches: fields
            .lines("branches")
            .into_iter()
            .filter(|b| !b.starts_with(DETACHED_PREFIX))
            .collect(),
        tags: fields.lines("tags"),
        commits: commits(fields.get("log")),
    })
}

fn upstream(line: &str) -> Option<Upstream> {
    let (ahead, behind) = line.split_once('\t')?;
    Some(Upstream {
        ahead: ahead.trim().parse().ok()?,
        behind: behind.trim().parse().ok()?,
    })
}

fn commit_line(line: &str) -> Option<Commit> {
    let fields: Vec<&str> = line.splitn(5, '\t').collect();
    if fields.len() < 5 {
        return None;
    }
    let at = DateTime::from_timestamp(fields[2].parse().ok()?, 0)?;
    Some(Commit {
        hash: fields[0].to_owned(),
        author: fields[1].to_owned(),
        at,
        refs: fields[3]
            .split(", ")
            .filter(|r| !r.is_empty())
            .map(str::to_owned)
            .collect(),
        subject: fields[4].to_owned(),
    })
}

struct Fields<'a> {
    parts: Vec<(&'a str, &'a str)>,
}

impl<'a> Fields<'a> {
    fn parse(raw: &'a str) -> Self {
        let parts = raw
            .split(FIELD_MARKER)
            .filter_map(|part| part.split_once('\n').or(Some((part, ""))))
            .filter(|(name, _)| !name.trim().is_empty())
            .map(|(name, body)| (name.trim(), body))
            .collect();
        Self { parts }
    }

    fn get(&self, name: &str) -> &'a str {
        self.parts
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, body)| *body)
            .unwrap_or_default()
    }

    fn line(&self, name: &str) -> Option<&'a str> {
        self.get(name)
            .lines()
            .next()
            .map(str::trim)
            .filter(|l| !l.is_empty())
    }

    fn lines(&self, name: &str) -> Vec<String> {
        self.get(name)
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fedora() -> GitSnapshot {
        git_snapshot(include_str!("../../fixtures/git/fedora.txt")).expect("fedora")
    }

    fn ubuntu() -> GitSnapshot {
        git_snapshot(include_str!("../../fixtures/git/ubuntu.txt")).expect("ubuntu")
    }

    #[test]
    fn fedora_fixture_lists_three_repositories_with_version() {
        let snapshot = fedora();
        assert_eq!(snapshot.version, "2.53.0");
        assert_eq!(snapshot.repositories.len(), 3);
        let sib = snapshot.find("/home/sander/Desktop/Sib").expect("sib");
        assert_eq!(sib.name(), "Sib");
        assert_eq!(sib.branch.as_deref(), Some("master"));
        assert_eq!(sib.changed_files.len(), 4);
        assert_eq!(sib.commits.len(), 25);
        assert!(sib.remote.is_none());
        assert!(sib.upstream.is_none());
    }

    #[test]
    fn repository_without_commits_is_empty() {
        let snapshot = fedora();
        let repo = snapshot
            .find("/home/sander/Desktop/Rust_learn/hello_cargo")
            .expect("hello_cargo");
        assert!(repo.is_empty());
        assert!(repo.branch.is_none());
        assert!(repo.commits.is_empty());
        assert_eq!(repo.changed_files.len(), 4);
    }

    #[test]
    fn upstream_and_tags_are_parsed() {
        let snapshot = fedora();
        let flutter = snapshot.find("/home/sander/dev/flutter").expect("flutter");
        assert_eq!(
            flutter.upstream,
            Some(Upstream {
                ahead: 0,
                behind: 15
            })
        );
        assert_eq!(
            flutter.remote.as_deref(),
            Some("https://github.com/flutter/flutter.git")
        );
        assert_eq!(flutter.tags.first().map(String::as_str), Some("3.47.4"));
        assert_eq!(flutter.tags.len(), 20);
    }

    #[test]
    fn detached_head_has_no_branch_and_skips_placeholder() {
        let snapshot = ubuntu();
        let shop = snapshot.find("/srv/shop").expect("shop");
        assert!(shop.is_detached());
        assert_eq!(shop.branches, vec!["master"]);
        assert_eq!(shop.tags, vec!["v1.0"]);
        assert_eq!(shop.commits[1].refs, vec!["tag: v1.0"]);
    }

    #[test]
    fn commit_line_splits_refs_and_keeps_tabs_in_subject() {
        let commit = commit_line("abc\tAnna\t1789480271\tHEAD -> main, origin/main\tfix: a\tb")
            .expect("commit");
        assert_eq!(commit.refs, vec!["HEAD -> main", "origin/main"]);
        assert_eq!(commit.subject, "fix: a\tb");
        assert_eq!(commit.short_hash(), "abc");
    }

    #[test]
    fn staged_file_counts_as_dirty() {
        let snapshot = ubuntu();
        let api = snapshot.find("/srv/api").expect("api");
        assert!(api.is_dirty());
        assert_eq!(api.changed_files, vec!["A  README"]);
        assert_eq!(api.branches, vec!["feature/auth", "master"]);
        assert_eq!(snapshot.dirty_count(), 2);
    }

    #[test]
    fn empty_version_is_parse_error() {
        assert!(git_snapshot("###version\n###repos\n").is_err());
    }
}
