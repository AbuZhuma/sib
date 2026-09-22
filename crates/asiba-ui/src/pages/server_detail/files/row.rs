use asiba_core::{ActionRequest, ServerId};
use asiba_modules::files::{self, Entry, EntryKind};
use chrono::DateTime;
use egui::{Align, CornerRadius, Layout, Rect, RichText, Sense, Ui, UiBuilder, vec2};

use super::state::{Draft, NameKind, TreeState};
use crate::components::{FileIcon, file_icon};
use crate::format;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP_SMALL, Palette, ROW_HEIGHT};

pub const INDENT: f32 = 18.0;
const NAME_COLUMN_WIDTH: f32 = 300.0;
const MIN_NAME_WIDTH: f32 = 120.0;
const ROW_RADIUS: u8 = 2;

pub struct RowContext<'a> {
    pub server: &'a ServerId,
    pub directory: &'a str,
    pub depth: usize,
    pub palette: &'a Palette,
}

pub enum RowClick {
    Toggle,
    Refresh,
    Open,
}

fn row_response(
    ui: &mut Ui,
    ctx: &RowContext<'_>,
    entry: &Entry,
    (path, state): (&str, &TreeState),
) -> egui::Response {
    let builder = UiBuilder::new()
        .id_salt(("files-row", path))
        .sense(Sense::click());
    ui.scope_builder(builder, |ui| {
        let rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), ROW_HEIGHT));
        paint_background(ui, rect, path, state);
        ui.allocate_ui_with_layout(rect.size(), Layout::left_to_right(Align::Center), |ui| {
            content(ui, ctx, entry, state.is_expanded(path));
        });
    })
    .response
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn show(
    ui: &mut Ui,
    ctx: &RowContext<'_>,
    entry: &Entry,
    state: &mut TreeState,
) -> (Option<RowClick>, Option<Action>) {
    let path = files::join_path(ctx.directory, &entry.name);
    let response = row_response(ui, ctx, entry, (&path, state));
    if response.clicked() {
        state.selected = Some(path.clone());
    }
    let mut click = None;
    if response.double_clicked() {
        click = Some(if entry.kind.is_directory() {
            RowClick::Toggle
        } else {
            RowClick::Open
        });
    }
    let mut action = None;
    response.context_menu(|ui| {
        state.selected = Some(path.clone());
        click = click.take().or(menu(ui, entry, &path, state));
        if ui.button(text::FILES_DELETE).clicked() {
            action = Some(delete_action(ctx.server, &path));
            ui.close();
        }
    });
    (click, action)
}

fn paint_background(ui: &Ui, rect: Rect, path: &str, state: &TreeState) {
    let p = Palette::current(ui.ctx());
    let is_selected = state.selected.as_deref() == Some(path);
    let fill = if is_selected {
        p.accent_bg
    } else if ui.response().hovered() {
        p.bg_raised
    } else {
        return;
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(ROW_RADIUS), fill);
}

fn content(ui: &mut Ui, ctx: &RowContext<'_>, entry: &Entry, is_expanded: bool) {
    ui.add_space(GAP_SMALL + INDENT * ctx.depth as f32);
    file_icon(
        ui,
        icon_for(entry, is_expanded),
        entry.link_target.is_some(),
        ctx.palette,
    );
    let name_width = (NAME_COLUMN_WIDTH - INDENT * ctx.depth as f32).max(MIN_NAME_WIDTH);
    ui.allocate_ui_with_layout(
        vec2(name_width, ROW_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_max_width(name_width);
            name_label(ui, ctx, entry);
        },
    );
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add_space(GAP_SMALL);
        ui.add(
            egui::Label::new(
                RichText::new(details(entry))
                    .monospace()
                    .color(ctx.palette.text_secondary),
            )
            .truncate()
            .selectable(false),
        );
    });
}

fn icon_for(entry: &Entry, is_expanded: bool) -> FileIcon {
    match entry.kind {
        EntryKind::Directory | EntryKind::Symlink { is_directory: true } => FileIcon::Folder {
            is_open: is_expanded,
        },
        EntryKind::File
        | EntryKind::Symlink {
            is_directory: false,
        } => FileIcon::File,
        EntryKind::Other => FileIcon::Other,
    }
}

fn menu(ui: &mut Ui, entry: &Entry, path: &str, state: &mut TreeState) -> Option<RowClick> {
    let mut click = None;
    if entry.kind.is_file() && ui.button(text::FILES_OPEN).clicked() {
        click = Some(RowClick::Open);
    }
    if entry.kind.is_directory() {
        let label = if state.is_expanded(path) {
            text::FILES_COLLAPSE
        } else {
            text::FILES_EXPAND
        };
        if ui.button(label).clicked() {
            click = Some(RowClick::Toggle);
        }
        if state.is_expanded(path) && ui.button(text::FILES_REFRESH).clicked() {
            click = Some(RowClick::Refresh);
        }
        creation_items(ui, path, state);
    }
    draft_items(ui, entry, path, state);
    if click.is_some() {
        ui.close();
    }
    click
}

fn draft_items(ui: &mut Ui, entry: &Entry, path: &str, state: &mut TreeState) {
    let items = [
        (text::FILES_RENAME, NameKind::Rename, entry.name.clone()),
        (text::FILES_MOVE_TO, NameKind::Move, path.to_owned()),
        (text::FILES_COPY_TO, NameKind::Copy, path.to_owned()),
    ];
    for (label, kind, value) in items {
        if ui.button(label).clicked() {
            state.toggle_draft(Draft::Name {
                path: path.to_owned(),
                kind,
                value,
            });
            ui.close();
        }
    }
    if ui.button(text::FILES_PERMISSIONS).clicked() {
        state.toggle_draft(Draft::Permissions {
            path: path.to_owned(),
            mode: entry.mode_octal(),
            owner: format!("{}:{}", entry.owner, entry.group),
        });
        ui.close();
    }
}

pub fn creation_items(ui: &mut Ui, directory: &str, state: &mut TreeState) {
    let items = [
        (text::FILES_NEW_FILE, NameKind::NewFile),
        (text::FILES_NEW_DIRECTORY, NameKind::NewDirectory),
    ];
    for (label, kind) in items {
        if ui.button(label).clicked() {
            state.toggle_draft(Draft::Name {
                path: directory.to_owned(),
                kind,
                value: String::new(),
            });
            ui.close();
        }
    }
}

fn name_label(ui: &mut Ui, ctx: &RowContext<'_>, entry: &Entry) {
    let p = ctx.palette;
    let color = if entry.kind.is_directory() {
        p.text
    } else {
        p.text_secondary
    };
    let mut label = entry.name.clone();
    if let Some(target) = &entry.link_target {
        label.push_str(&format!(" {} {target}", text::FILES_LINK_TO));
    }
    let text = RichText::new(label).color(color);
    ui.add(egui::Label::new(text).truncate().selectable(false))
        .on_hover_text(&entry.name);
}

fn details(entry: &Entry) -> String {
    let modified = DateTime::from_timestamp(entry.modified_at, 0)
        .map(format::date_time)
        .unwrap_or_default();
    let size = if entry.kind.is_directory() {
        String::new()
    } else {
        format::bytes(entry.size)
    };
    format!(
        "{} {:>4} {:<12} {:>8} {modified}",
        entry.mode_symbolic(),
        entry.mode_octal(),
        format!("{}:{}", entry.owner, entry.group),
        size
    )
}

fn delete_action(server: &ServerId, path: &str) -> Action {
    Action::AskPerform {
        server: server.clone(),
        spec: files::SPEC_DELETE,
        request: ActionRequest::new(files::ACTION_DELETE, path),
    }
}
