use asiba_core::{QueryRequest, ServerId};
use asiba_modules::files::{self, Entry, EntryKind};
use egui::{Label, RichText, Sense, TextEdit, Ui, UiBuilder};

use super::browser::{FileBrowser, SearchResult};
use super::state::TreeState;
use crate::components::{FileIcon, file_icon, scroll};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette, ROW_HEIGHT};

const INPUT_WIDTH: f32 = 260.0;
const RESULTS_HEIGHT: f32 = 260.0;

pub fn bar(
    ui: &mut Ui,
    server: &ServerId,
    browser: &FileBrowser,
    state: &mut TreeState,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        let response = ui.add(
            TextEdit::singleline(&mut state.search_text)
                .hint_text(text::FILES_SEARCH_HINT)
                .desired_width(INPUT_WIDTH),
        );
        let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let pattern = state.search_text.trim();
        let can_search = !pattern.is_empty() && !browser.is_searching();
        let clicked = ui
            .add_enabled(can_search, egui::Button::new(text::FILES_SEARCH))
            .clicked();
        if (clicked || submitted) && can_search {
            action = Some(Action::FilesQuery {
                server: server.clone(),
                request: QueryRequest::new(files::QUERY_SEARCH, pattern),
            });
        }
        if browser.is_searching() {
            ui.label(RichText::new(text::FILES_SEARCHING).color(p.text_muted));
        }
    });
    action
}

fn summary_labels(ui: &mut Ui, result: &SearchResult, p: &Palette) {
    ui.label(
        RichText::new(format!(
            "{} «{}»: {}",
            text::FILES_SEARCH_RESULTS,
            result.pattern,
            result.listing.entries.len()
        ))
        .small()
        .color(p.text_secondary),
    );
    if result.listing.is_truncated {
        ui.label(
            RichText::new(format!(
                "{} {}",
                text::FILES_TRUNCATED,
                files::MAX_SEARCH_RESULTS
            ))
            .small()
            .color(p.text_muted),
        );
    }
}

pub fn results(
    ui: &mut Ui,
    server: &ServerId,
    browser: &FileBrowser,
    state: &mut TreeState,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    if let Some(error) = browser.search_error() {
        ui.label(RichText::new(error).color(p.critical));
    }
    let result = browser.search()?;
    let mut action = None;
    ui.horizontal(|ui| {
        summary_labels(ui, result, &p);
        if ui.small_button(text::FILES_SEARCH_CLOSE).clicked() {
            action = Some(Action::FilesClearSearch {
                server: server.clone(),
            });
        }
    });
    ui.add_space(GAP_SMALL);
    scroll::vertical()
        .id_salt("files-search-results")
        .max_height(RESULTS_HEIGHT)
        .show(ui, |ui| {
            for entry in &result.listing.entries {
                result_row(ui, entry, state, &p);
            }
        });
    ui.add_space(GAP);
    action
}

fn result_row(ui: &mut Ui, entry: &Entry, state: &mut TreeState, p: &Palette) {
    let path = entry.name.as_str();
    let is_selected = state.selected.as_deref() == Some(path);
    let builder = UiBuilder::new()
        .id_salt(("files-search-row", path))
        .sense(Sense::click());
    let response = ui
        .scope_builder(builder, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.set_min_height(ROW_HEIGHT);
                file_icon(ui, icon_for(entry), entry.link_target.is_some(), p);
                let color = if is_selected { p.accent } else { p.text };
                ui.add(
                    Label::new(RichText::new(path).monospace().color(color))
                        .truncate()
                        .selectable(false),
                );
            });
        })
        .response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(text::FILES_SEARCH_REVEAL);
    if response.clicked() {
        state.reveal(path);
    }
    if response.double_clicked() && entry.kind.is_file() {
        state.pending_open = Some(path.to_owned());
    }
}

fn icon_for(entry: &Entry) -> FileIcon {
    match entry.kind {
        EntryKind::Directory | EntryKind::Symlink { is_directory: true } => {
            FileIcon::Folder { is_open: false }
        }
        EntryKind::File
        | EntryKind::Symlink {
            is_directory: false,
        } => FileIcon::File,
        EntryKind::Other => FileIcon::Other,
    }
}
