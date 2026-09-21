use asiba_config::AppConfig;
use asiba_core::{Intervals, Retention};
use egui::{Grid, Id, RichText, TextEdit, Ui};

use super::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "collection-settings-draft";
const FIELD: f32 = 80.0;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Draft {
    fast: String,
    normal: String,
    slow: String,
    raw_hours: String,
    minute_days: String,
    hour_days: String,
}

impl Draft {
    fn from_config(config: &AppConfig) -> Self {
        Self {
            fast: config.intervals.fast_secs.to_string(),
            normal: config.intervals.normal_secs.to_string(),
            slow: config.intervals.slow_secs.to_string(),
            raw_hours: config.retention.raw_hours.to_string(),
            minute_days: config.retention.minute_days.to_string(),
            hour_days: config.retention.hour_days.to_string(),
        }
    }

    fn build(&self) -> Option<(Intervals, Retention)> {
        let intervals = Intervals {
            fast_secs: self.fast.trim().parse().ok()?,
            normal_secs: self.normal.trim().parse().ok()?,
            slow_secs: self.slow.trim().parse().ok()?,
        };
        let retention = Retention {
            raw_hours: self.raw_hours.trim().parse().ok()?,
            minute_days: self.minute_days.trim().parse().ok()?,
            hour_days: self.hour_days.trim().parse().ok()?,
        };
        Some((intervals.clamped(), retention))
    }
}

fn draft_grid(ui: &mut Ui, draft: &mut Draft) {
    let rows: [(&str, &mut String, &str, &str, &mut String, &str); 3] = [
        (
            text::SETTINGS_FAST,
            &mut draft.fast,
            text::SETTINGS_SECONDS,
            text::SETTINGS_RAW,
            &mut draft.raw_hours,
            text::SETTINGS_HOURS,
        ),
        (
            text::SETTINGS_NORMAL,
            &mut draft.normal,
            text::SETTINGS_SECONDS,
            text::SETTINGS_MINUTE,
            &mut draft.minute_days,
            text::SETTINGS_DAYS,
        ),
        (
            text::SETTINGS_SLOW,
            &mut draft.slow,
            text::SETTINGS_SECONDS,
            text::SETTINGS_HOUR,
            &mut draft.hour_days,
            text::SETTINGS_DAYS,
        ),
    ];
    Grid::new("collection-grid")
        .num_columns(4)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (label, value, unit, right_label, right_value, right_unit) in rows {
                row(ui, label, value, unit);
                row(ui, right_label, right_value, right_unit);
                ui.end_row();
            }
        });
}

pub fn show(ui: &mut Ui, config: &AppConfig) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let id = Id::new(DRAFT_KEY);
    let mut draft: Draft = ui
        .ctx()
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| Draft::from_config(config));
    draft_grid(ui, &mut draft);
    ui.label(
        RichText::new(text::SETTINGS_COLLECTION_HINT)
            .small()
            .color(p.text_muted),
    );
    ui.add_space(GAP);
    let parsed = draft.build();
    let is_dirty = draft != Draft::from_config(config);
    let clicked = ui
        .add_enabled(
            parsed.is_some() && is_dirty,
            egui::Button::new(text::BTN_APPLY),
        )
        .clicked();
    ui.ctx().data_mut(|d| d.insert_temp(id, draft));
    if !clicked {
        return None;
    }
    let (intervals, retention) = parsed?;
    ui.ctx().data_mut(|d| d.remove::<Draft>(id));
    Some(Action::SaveCollection {
        intervals,
        retention,
    })
}

fn row(ui: &mut Ui, label: &str, value: &mut String, unit: &str) {
    let p = Palette::current(ui.ctx());
    ui.label(RichText::new(label).color(p.text_secondary));
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(value).desired_width(FIELD));
        ui.label(RichText::new(unit).color(p.text_muted));
    });
}
