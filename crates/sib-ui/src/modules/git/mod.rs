mod repository;

use egui::{CollapsingHeader, RichText, Ui};
use sib_core::{ModuleId, QueryRequest, ServerState};
use sib_modules::git::{self, GitSnapshot, Repository};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::badge;
use crate::format;
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

const SUMMARY_SHOWN: usize = 5;

pub struct GitView;

impl ModuleView for GitView {
    fn id(&self) -> ModuleId {
        git::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_GIT
    }

    fn tab(&self) -> Tab {
        Tab::Git
    }

    fn has_content(&self, server: &ServerState) -> bool {
        server
            .data::<GitSnapshot>(git::ID)
            .is_some_and(|snapshot| !snapshot.repositories.is_empty())
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<GitSnapshot>(git::ID) else {
            return;
        };
        ui.monospace(format!(
            "{} {}",
            snapshot.repositories.len(),
            text::GIT_REPOSITORIES
        ));
        for repository in snapshot.repositories.iter().take(SUMMARY_SHOWN) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(repository.name()).strong());
                ui.label(
                    RichText::new(branch_label(repository))
                        .monospace()
                        .color(p.text_secondary),
                );
                dirty_badge(ui, repository, &p);
                if let Some(commit) = repository.last_commit() {
                    ui.label(
                        RichText::new(format::date_time(commit.at))
                            .monospace()
                            .color(p.text_muted),
                    );
                }
            });
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<GitSnapshot>(git::ID)?;
        overview_line(ui, snapshot, &p);
        ui.add_space(GAP);
        let mut action = None;
        for repository in &snapshot.repositories {
            let header = CollapsingHeader::new(RichText::new(repository.name()).strong())
                .id_salt(&repository.path)
                .default_open(true);
            header.show(ui, |ui| {
                if let Some(next) = repository_body(ui, repository, &p) {
                    action = Some(next);
                }
            });
            ui.add_space(GAP_SMALL);
        }
        action
    }
}

fn overview_line(ui: &mut Ui, snapshot: &GitSnapshot, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.monospace(format!(
            "{} {}",
            snapshot.repositories.len(),
            text::GIT_REPOSITORIES
        ));
        let dirty = snapshot.dirty_count();
        if dirty > 0 {
            badge(ui, &format!("{dirty} {}", text::GIT_DIRTY), p.warning);
        }
        ui.label(
            RichText::new(format!("{} {}", text::GIT_VERSION, snapshot.version))
                .monospace()
                .color(p.text_muted),
        );
    });
}

fn repository_body(ui: &mut Ui, repository: &Repository, p: &Palette) -> Option<ViewAction> {
    ui.label(
        RichText::new(&repository.path)
            .monospace()
            .color(p.text_muted),
    );
    let action = status_line(ui, repository, p);
    repository::details(ui, repository, p);
    action
}

fn status_line(ui: &mut Ui, repository: &Repository, p: &Palette) -> Option<ViewAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::GIT_BRANCH).color(p.text_secondary));
        ui.label(RichText::new(branch_label(repository)).monospace());
        if let Some(head) = &repository.head {
            ui.label(
                RichText::new(head.get(..git::SHORT_HASH_LEN).unwrap_or(head))
                    .monospace()
                    .color(p.text_muted),
            );
        }
        upstream_badges(ui, repository, p);
        dirty_badge(ui, repository, p);
        if repository.stashes > 0 {
            badge(
                ui,
                &format!("{} {}", repository.stashes, text::GIT_STASHES),
                p.info,
            );
        }
        if let Some(remote) = &repository.remote {
            ui.label(RichText::new(text::GIT_REMOTE).color(p.text_secondary));
            ui.label(RichText::new(remote).monospace());
        }
        if !repository.is_empty() && ui.small_button(text::GIT_FULL_HISTORY).clicked() {
            action = Some(ViewAction::Query(QueryRequest::new(
                git::QUERY_HISTORY,
                &repository.path,
            )));
        }
    });
    action
}

fn upstream_badges(ui: &mut Ui, repository: &Repository, p: &Palette) {
    let Some(upstream) = repository.upstream else {
        return;
    };
    if upstream.ahead > 0 {
        badge(
            ui,
            &format!("{} {}", upstream.ahead, text::GIT_AHEAD),
            p.info,
        );
    }
    if upstream.behind > 0 {
        badge(
            ui,
            &format!("{} {}", upstream.behind, text::GIT_BEHIND),
            p.warning,
        );
    }
}

fn dirty_badge(ui: &mut Ui, repository: &Repository, p: &Palette) {
    if repository.is_dirty() {
        let count = repository.changed_files.len();
        badge(ui, &format!("{count} {}", text::GIT_DIRTY), p.warning);
    } else if !repository.is_empty() {
        badge(ui, text::GIT_CLEAN, p.ok);
    }
}

fn branch_label(repository: &Repository) -> &str {
    if repository.is_empty() {
        return text::GIT_EMPTY;
    }
    repository.branch.as_deref().unwrap_or(text::GIT_DETACHED)
}
