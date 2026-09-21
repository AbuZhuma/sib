use asiba_core::{ActionRequest, ActionSpec, ServerId};
use asiba_modules::files;
use egui::{RichText, TextEdit, Ui};

use super::row::INDENT;
use super::state::{Draft, NameKind};
use crate::pages::Action;
use crate::text;
use crate::theme::Palette;

const MODE_WIDTH: f32 = 60.0;
const OWNER_WIDTH: f32 = 160.0;
const NAME_WIDTH: f32 = 260.0;
const PATH_WIDTH: f32 = 480.0;

pub fn show(ui: &mut Ui, server: &ServerId, draft: &mut Draft, depth: usize) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.add_space(INDENT * (depth + 1) as f32);
        action = match draft {
            Draft::Permissions { path, mode, owner } => permissions(ui, server, path, mode, owner),
            Draft::Name { path, kind, value } => name(ui, server, path, *kind, value),
        };
    });
    action
}

fn permissions(
    ui: &mut Ui,
    server: &ServerId,
    path: &str,
    mode: &mut String,
    owner: &mut String,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.label(RichText::new(text::FILES_MODE).color(p.text_secondary));
    ui.add(
        TextEdit::singleline(mode)
            .desired_width(MODE_WIDTH)
            .font(egui::TextStyle::Monospace),
    );
    if ui.small_button(text::FILES_APPLY).clicked() {
        action = Some(perform(server, files::SPEC_CHMOD, path, mode));
    }
    ui.separator();
    ui.label(RichText::new(text::FILES_OWNER).color(p.text_secondary));
    ui.add(
        TextEdit::singleline(owner)
            .desired_width(OWNER_WIDTH)
            .font(egui::TextStyle::Monospace),
    );
    if ui.small_button(text::FILES_APPLY).clicked() {
        action = Some(perform(server, files::SPEC_CHOWN, path, owner));
    }
    action
}

fn name(
    ui: &mut Ui,
    server: &ServerId,
    path: &str,
    kind: NameKind,
    value: &mut String,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let (label, width) = match kind {
        NameKind::NewFile => (text::FILES_NEW_FILE, NAME_WIDTH),
        NameKind::NewDirectory => (text::FILES_NEW_DIRECTORY, NAME_WIDTH),
        NameKind::Rename => (text::FILES_RENAME, NAME_WIDTH),
        NameKind::Move => (text::FILES_MOVE_TO, PATH_WIDTH),
        NameKind::Copy => (text::FILES_COPY_TO, PATH_WIDTH),
    };
    ui.label(RichText::new(label).color(p.text_secondary));
    let response = ui.add(
        TextEdit::singleline(value)
            .desired_width(width)
            .font(egui::TextStyle::Monospace),
    );
    let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let trimmed = value.trim();
    let is_valid = is_valid_input(kind, trimmed);
    let clicked = ui
        .add_enabled(is_valid, egui::Button::new(text::FILES_APPLY).small())
        .clicked();
    if !(clicked || (submitted && is_valid)) {
        return None;
    }
    Some(match kind {
        NameKind::NewFile => perform(
            server,
            files::SPEC_CREATE,
            &files::join_path(path, trimmed),
            "",
        ),
        NameKind::NewDirectory => perform(
            server,
            files::SPEC_MKDIR,
            &files::join_path(path, trimmed),
            "",
        ),
        NameKind::Rename => perform(
            server,
            files::SPEC_MOVE,
            path,
            &files::join_path(files::parent_path(path), trimmed),
        ),
        NameKind::Move => perform(server, files::SPEC_MOVE, path, trimmed),
        NameKind::Copy => perform(server, files::SPEC_COPY, path, trimmed),
    })
}

fn is_valid_input(kind: NameKind, value: &str) -> bool {
    match kind {
        NameKind::Move | NameKind::Copy => value.starts_with('/'),
        NameKind::NewFile | NameKind::NewDirectory | NameKind::Rename => {
            !value.is_empty() && !value.contains('/') && value != "." && value != ".."
        }
    }
}

fn perform(server: &ServerId, spec: ActionSpec, path: &str, argument: &str) -> Action {
    let mut request = ActionRequest::new(spec.kind, path);
    if !argument.is_empty() {
        request = request.with_argument(argument);
    }
    Action::AskPerform {
        server: server.clone(),
        spec,
        request,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_reject_slashes_and_dots_but_paths_need_root() {
        assert!(is_valid_input(NameKind::NewFile, "a.txt"));
        assert!(!is_valid_input(NameKind::NewFile, "a/b"));
        assert!(!is_valid_input(NameKind::Rename, ".."));
        assert!(is_valid_input(NameKind::Move, "/tmp/x"));
        assert!(!is_valid_input(NameKind::Copy, "tmp/x"));
    }
}
