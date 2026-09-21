mod browser;
mod draft;
mod editor;
mod row;
mod search;
mod state;
mod tree;

use asiba_core::{QueryRequest, ServerId};
use asiba_modules::files;
use egui::{RichText, Ui};

pub use browser::FileBrowser;

use super::DetailContext;
use crate::components::panel_plain;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let server = &ctx.server.spec.id;
    let Some(browser) = ctx.files else {
        return Some(Action::FilesQuery {
            server: server.clone(),
            request: QueryRequest::new(files::QUERY_LIST, files::ROOT),
        });
    };
    let p = Palette::current(ui.ctx());
    let mut tree_state = state::load_tree(ui.ctx(), server);
    let action = open_pending(ui.ctx(), server, browser, &mut tree_state)
        .or_else(|| editor_block(ui, server, browser));
    let tree_action = panel_plain(ui, |ui| {
        let header_action = header(ui, server, &p, &mut tree_state)
            .or_else(|| search::bar(ui, server, browser, &mut tree_state))
            .or_else(|| search::results(ui, server, browser, &mut tree_state));
        let tree_ctx = tree::TreeContext {
            server,
            browser,
            palette: &p,
        };
        header_action.or(tree::directory(
            ui,
            &tree_ctx,
            files::ROOT,
            0,
            &mut tree_state,
        ))
    });
    state::store_tree(ui.ctx(), server, tree_state);
    action.or(tree_action)
}

fn header(
    ui: &mut Ui,
    server: &ServerId,
    p: &Palette,
    state: &mut state::TreeState,
) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::FILES_ROOT).monospace().color(p.accent));
        if ui.small_button(text::FILES_REFRESH).clicked() {
            action = Some(Action::FilesRefresh {
                server: server.clone(),
                path: files::ROOT.to_owned(),
            });
        }
        ui.menu_button(text::FILES_CREATE_MENU, |ui| {
            row::creation_items(ui, files::ROOT, state);
        });
    });
    if state.has_draft_for(files::ROOT)
        && let Some(current) = state.draft.as_mut()
        && let Some(submitted) = draft::show(ui, server, current, 0)
    {
        state.draft = None;
        action = action.or(Some(submitted));
    }
    ui.add_space(GAP);
    action
}

fn open_pending(
    ctx: &egui::Context,
    server: &ServerId,
    browser: &FileBrowser,
    state: &mut state::TreeState,
) -> Option<Action> {
    let path = state.pending_open.clone()?;
    if let Some(content) = browser.content(&path) {
        state::store_editor(ctx, server, Some(state::Editor::new(&path, content)));
        state.pending_open = None;
        return None;
    }
    if browser.error(&path).is_some() {
        state.pending_open = None;
        return None;
    }
    if browser.is_loading(&path) {
        return None;
    }
    Some(request(server, files::QUERY_READ, &path))
}

fn editor_block(ui: &mut Ui, server: &ServerId, browser: &FileBrowser) -> Option<Action> {
    let mut editor = state::load_editor(ui.ctx(), server)?;
    let mut action = None;
    let outcome = panel_plain(ui, |ui| {
        let (outcome, next) = editor::show(ui, server, &mut editor);
        action = next;
        if let Some(error) = browser.error(&editor.path) {
            let p = Palette::current(ui.ctx());
            ui.label(RichText::new(error).color(p.critical));
        }
        outcome
    });
    ui.add_space(GAP);
    let kept = match outcome {
        editor::EditorOutcome::Keep => Some(editor),
        editor::EditorOutcome::Close => None,
    };
    state::store_editor(ui.ctx(), server, kept);
    action
}

fn request(server: &ServerId, kind: &str, path: &str) -> Action {
    Action::FilesQuery {
        server: server.clone(),
        request: QueryRequest::new(kind, path),
    }
}
