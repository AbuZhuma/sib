use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2};

use crate::theme::Palette;

pub const ICON_SIZE: Vec2 = Vec2::new(16.0, 14.0);
const FOLDER_TAB_WIDTH: f32 = 0.45;
const FOLDER_TAB_HEIGHT: f32 = 0.22;
const FOLDER_OPEN_SHIFT: f32 = 0.12;
const FILE_WIDTH: f32 = 0.7;
const FILE_FOLD: f32 = 0.35;
const LINK_BADGE: f32 = 5.0;
const CORNER: u8 = 2;
const LINE: f32 = 1.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIcon {
    Folder { is_open: bool },
    File,
    Other,
}

pub fn file_icon(ui: &mut Ui, icon: FileIcon, is_link: bool, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(ICON_SIZE, Sense::hover());
    match icon {
        FileIcon::Folder { is_open } => folder(ui, rect, is_open, p),
        FileIcon::File => file(ui, rect, p.text_secondary),
        FileIcon::Other => file(ui, rect, p.text_muted),
    }
    if is_link {
        link_badge(ui, rect, p);
    }
}

fn folder(ui: &Ui, rect: Rect, is_open: bool, p: &Palette) {
    let painter = ui.painter();
    let tab = Rect::from_min_size(
        rect.min,
        Vec2::new(
            rect.width() * FOLDER_TAB_WIDTH,
            rect.height() * FOLDER_TAB_HEIGHT * 2.0,
        ),
    );
    painter.rect_filled(tab, CornerRadius::same(CORNER), p.accent);
    let body_top = rect.min.y + rect.height() * FOLDER_TAB_HEIGHT;
    let body = Rect::from_min_max(pos2(rect.min.x, body_top), rect.max);
    painter.rect_filled(body, CornerRadius::same(CORNER), p.accent);
    if !is_open {
        return;
    }
    let shift = rect.width() * FOLDER_OPEN_SHIFT;
    let front = Rect::from_min_max(
        pos2(rect.min.x + shift, body_top + shift),
        pos2(rect.max.x + shift * 0.5, rect.max.y),
    );
    painter.rect_filled(front, CornerRadius::same(CORNER), p.accent_bg);
    painter.rect_stroke(
        front,
        CornerRadius::same(CORNER),
        Stroke::new(LINE, p.accent),
        StrokeKind::Inside,
    );
}

fn file(ui: &Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let width = rect.width() * FILE_WIDTH;
    let left = rect.center().x - width / 2.0;
    let fold = width * FILE_FOLD;
    let outline = [
        pos2(left, rect.min.y),
        pos2(left + width - fold, rect.min.y),
        pos2(left + width, rect.min.y + fold),
        pos2(left + width, rect.max.y),
        pos2(left, rect.max.y),
    ];
    painter.add(egui::Shape::closed_line(
        outline.to_vec(),
        Stroke::new(LINE, color),
    ));
    let corner: Vec<Pos2> = vec![
        pos2(left + width - fold, rect.min.y),
        pos2(left + width - fold, rect.min.y + fold),
        pos2(left + width, rect.min.y + fold),
    ];
    painter.add(egui::Shape::line(corner, Stroke::new(LINE, color)));
}

fn link_badge(ui: &Ui, rect: Rect, p: &Palette) {
    let badge = Rect::from_min_size(
        pos2(rect.min.x, rect.max.y - LINK_BADGE),
        Vec2::splat(LINK_BADGE),
    );
    ui.painter()
        .rect_filled(badge, CornerRadius::same(1), p.bg_panel);
    ui.painter().rect_stroke(
        badge,
        CornerRadius::same(1),
        Stroke::new(1.0, p.text),
        StrokeKind::Inside,
    );
}
