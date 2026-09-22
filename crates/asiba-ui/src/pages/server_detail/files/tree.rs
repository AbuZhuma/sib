use asiba_core::{QueryRequest, ServerId};
use asiba_modules::files::{self, Listing};
use egui::{RichText, Ui};

use super::browser::FileBrowser;
use super::row::{self, RowClick, RowContext};
use super::state::TreeState;
use super::{draft, request};
use crate::pages::Action;
use crate::text;
use crate::theme::Palette;

pub struct TreeContext<'a> {
    pub server: &'a ServerId,
    pub browser: &'a FileBrowser,
    pub palette: &'a Palette,
}

pub fn directory(
    ui: &mut Ui,
    ctx: &TreeContext<'_>,
    path: &str,
    depth: usize,
    state: &mut TreeState,
) -> Option<Action> {
    let Some(listing) = ctx.browser.listing(path) else {
        return placeholder(ui, ctx, path, depth);
    };
    let mut action = None;
    for entry in &listing.entries {
        let child = files::join_path(path, &entry.name);
        let row_ctx = RowContext {
            server: ctx.server,
            directory: path,
            depth,
            palette: ctx.palette,
        };
        let (click, row_action) = row::show(ui, &row_ctx, entry, state);
        action = action.or(row_action);
        action = action.or(handle_click(ctx, click, &child, state));
        if state.has_draft_for(&child)
            && let Some(current) = state.draft.as_mut()
            && let Some(submitted) = draft::show(ui, ctx.server, current, depth)
        {
            state.draft = None;
            action = action.or(Some(submitted));
        }
        if entry.kind.is_directory() && state.is_expanded(&child) {
            action = action.or(directory(ui, ctx, &child, depth + 1, state));
        }
    }
    listing_notes(ui, ctx, depth, listing);
    action
}

fn listing_notes(ui: &mut Ui, ctx: &TreeContext<'_>, depth: usize, listing: &Listing) {
    if listing.is_truncated {
        note(
            ui,
            ctx,
            depth,
            &format!("{} {}", text::FILES_TRUNCATED, files::MAX_ENTRIES),
        );
    }
    if listing.entries.is_empty() {
        note(ui, ctx, depth, text::FILES_EMPTY);
    }
}

fn handle_click(
    ctx: &TreeContext<'_>,
    click: Option<RowClick>,
    path: &str,
    state: &mut TreeState,
) -> Option<Action> {
    match click? {
        RowClick::Toggle => {
            state.toggle(path);
            None
        }
        RowClick::Refresh => Some(Action::FilesRefresh {
            server: ctx.server.clone(),
            path: path.to_owned(),
        }),
        RowClick::Open => {
            state.pending_open = Some(path.to_owned());
            if ctx.browser.content(path).is_some() || ctx.browser.is_loading(path) {
                return None;
            }
            Some(request(ctx.server, files::QUERY_READ, path))
        }
    }
}

fn placeholder(ui: &mut Ui, ctx: &TreeContext<'_>, path: &str, depth: usize) -> Option<Action> {
    if let Some(error) = ctx.browser.error(path) {
        note_colored(ui, depth, error, ctx.palette.critical);
        return None;
    }
    note(ui, ctx, depth, text::FILES_LOADING);
    if ctx.browser.is_loading(path) {
        return None;
    }
    Some(Action::FilesQuery {
        server: ctx.server.clone(),
        request: QueryRequest::new(files::QUERY_LIST, path),
    })
}

fn note(ui: &mut Ui, ctx: &TreeContext<'_>, depth: usize, message: &str) {
    note_colored(ui, depth, message, ctx.palette.text_muted);
}

fn note_colored(ui: &mut Ui, depth: usize, message: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_space(row::INDENT * (depth + 1) as f32);
        ui.label(RichText::new(message).small().color(color));
    });
}
