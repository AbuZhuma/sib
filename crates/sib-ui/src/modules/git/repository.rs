use egui::{RichText, Ui};
use sib_modules::git::{Commit, Repository};

use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::{GAP_SMALL, Palette};

const LIST_SEPARATOR: &str = ", ";

pub fn details(ui: &mut Ui, repository: &Repository, p: &Palette) {
    if repository.is_dirty() {
        changed_files(ui, repository);
    }
    named_list(ui, text::GIT_BRANCHES, &repository.branches, p);
    named_list(ui, text::GIT_TAGS, &repository.tags, p);
    if repository.commits.is_empty() {
        return;
    }
    ui.add_space(GAP_SMALL);
    commits_table(ui, repository, p);
}

fn changed_files(ui: &mut Ui, repository: &Repository) {
    let title = format!("{} ({})", text::GIT_CHANGES, repository.changed_files.len());
    ui.collapsing(title, |ui| {
        ui.monospace(repository.changed_files.join("\n"));
    });
}

fn named_list(ui: &mut Ui, title: &str, items: &[String], p: &Palette) {
    if items.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(title).color(p.text_secondary));
        ui.label(RichText::new(items.join(LIST_SEPARATOR)).monospace());
    });
}

fn commits_table(ui: &mut Ui, repository: &Repository, p: &Palette) {
    ui.label(
        RichText::new(
            format!("{} ({})", text::GIT_COMMITS, repository.commits.len()).to_uppercase(),
        )
        .small()
        .color(p.text_secondary),
    );
    let columns = ["", text::COL_TIME, text::GIT_AUTHOR, text::GIT_MESSAGE];
    let id = format!("git-commits-{}", repository.path);
    Table::new(&id, &columns).show(ui, |ui| {
        for commit in &repository.commits {
            commit_row(ui, commit, p);
            ui.end_row();
        }
    });
}

fn commit_row(ui: &mut Ui, commit: &Commit, p: &Palette) {
    ui.label(
        RichText::new(commit.short_hash())
            .monospace()
            .color(p.accent),
    );
    ui.label(
        RichText::new(format::date_time(commit.at))
            .monospace()
            .color(p.text_secondary),
    );
    ui.label(&commit.author);
    ui.horizontal(|ui| {
        ui.label(&commit.subject);
        for reference in &commit.refs {
            badge(ui, reference, p.text_muted);
        }
    });
}
