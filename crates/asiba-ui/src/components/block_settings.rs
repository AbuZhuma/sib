use egui::{Id, Modal, Ui};

use super::scroll;
use crate::text;
use crate::theme::GAP;

const GEAR: &str = "⚙";
const CLOSE: &str = "×";
const WIDTH: f32 = 980.0;
const MAX_HEIGHT: f32 = 520.0;

pub struct Popup<'a> {
    pub key: &'a str,
    pub title: &'a str,
}

pub fn gear(ui: &mut Ui, key: &str) {
    if !ui
        .small_button(GEAR)
        .on_hover_text(text::BLOCK_SETTINGS)
        .clicked()
    {
        return;
    }
    let was_open = is_open(ui.ctx(), key);
    ui.ctx()
        .data_mut(|d| d.insert_temp(open_id(key), !was_open));
}

pub fn scrolled(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    scroll::vertical()
        .max_height(MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, body);
    ui.add_space(GAP);
    ui.separator();
}

pub fn popup<R>(ui: &mut Ui, popup: &Popup<'_>, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    if !is_open(ui.ctx(), popup.key) {
        return None;
    }
    let modal = Modal::new(Id::new(("block-settings", popup.key))).show(ui.ctx(), |ui| {
        ui.set_width(WIDTH);
        let is_closed = header(ui, popup.title);
        ui.add_space(GAP);
        (content(ui), is_closed)
    });
    let should_close = modal.should_close();
    let (inner, is_closed) = modal.inner;
    if is_closed || should_close {
        close(ui.ctx(), popup.key);
    }
    Some(inner)
}

fn header(ui: &mut Ui, title: &str) -> bool {
    let mut is_closed = false;
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            is_closed = ui.button(CLOSE).on_hover_text(text::BTN_CLOSE).clicked();
        });
    });
    is_closed
}

pub fn close(ctx: &egui::Context, key: &str) {
    ctx.data_mut(|d| d.insert_temp(open_id(key), false));
}

fn is_open(ctx: &egui::Context, key: &str) -> bool {
    ctx.data(|d| d.get_temp(open_id(key))).unwrap_or(false)
}

fn open_id(key: &str) -> Id {
    Id::new(("block-settings-open", key))
}
