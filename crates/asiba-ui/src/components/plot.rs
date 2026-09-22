use asiba_core::Series;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use egui::{Color32, RichText, Sense, Stroke, Ui, Vec2};
use egui_plot::{GridMark, HoverPosition, Line, Plot, PlotPoints};

use crate::format;
use crate::theme::Palette;

const LINE_WIDTH: f32 = 1.25;
const FILL_ALPHA: f32 = 0.10;
const MIN_HEIGHT: f32 = 200.0;
const MAX_HEIGHT: f32 = 380.0;
const HEIGHT_RATIO: f32 = 0.26;
const DEFAULT_WINDOW_SECS: i64 = 600;
const MIN_SPAN_SECS: f64 = 30.0;
const SWATCH: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Percent,
    Bytes,
    BytesPerSecond,
    Count,
    Milliseconds,
    Celsius,
}

impl Unit {
    pub fn format(self, value: f64) -> String {
        match self {
            Self::Percent => format!("{value:.0}%"),
            Self::Bytes => format::bytes(value.max(0.0) as u64),
            Self::BytesPerSecond => format::bytes_per_second(value),
            Self::Count => format!("{value:.0}"),
            Self::Milliseconds => format!("{value:.1} ms"),
            Self::Celsius => format!("{value:.0}°C"),
        }
    }

    fn max_bound(self) -> Option<f64> {
        matches!(self, Self::Percent).then_some(100.0)
    }
}

pub struct PlotSeries<'a> {
    pub label: &'a str,
    pub series: &'a Series,
    pub color: Color32,
}

pub struct TimeSeriesPlot<'a> {
    pub id: &'a str,
    pub unit: Unit,
    pub height: Option<f32>,
    pub window_secs: i64,
    pub series: Vec<PlotSeries<'a>>,
    title: Option<&'a str>,
}

impl<'a> TimeSeriesPlot<'a> {
    pub fn new(id: &'a str, unit: Unit) -> Self {
        Self {
            id,
            unit,
            height: None,
            window_secs: DEFAULT_WINDOW_SECS,
            series: Vec::new(),
            title: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    fn caption(&self, ui: &mut Ui, p: &Palette) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if let Some(title) = self.title {
                ui.label(
                    RichText::new(title.to_uppercase())
                        .small()
                        .color(p.text_secondary),
                );
            }
            for entry in &self.series {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(SWATCH), Sense::hover());
                ui.painter().rect_filled(rect, 0.0, entry.color);
                ui.label(RichText::new(entry.label).small().color(p.text_secondary));
            }
        });
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    fn resolved_height(&self, ui: &Ui) -> f32 {
        self.height
            .unwrap_or_else(|| (ui.available_width() * HEIGHT_RATIO).clamp(MIN_HEIGHT, MAX_HEIGHT))
    }

    pub fn series(mut self, label: &'a str, series: &'a Series, color: Color32) -> Self {
        self.series.push(PlotSeries {
            label,
            series,
            color,
        });
        self
    }

    fn left_edge_secs(&self, now: chrono::DateTime<Utc>) -> f64 {
        let window = -(self.window_secs as f64);
        let oldest = self
            .series
            .iter()
            .filter_map(|entry| entry.series.oldest())
            .map(|point| (point.at - now).num_milliseconds() as f64 / 1000.0)
            .fold(f64::INFINITY, f64::min);
        if oldest.is_finite() && oldest > window {
            oldest.min(-MIN_SPAN_SECS)
        } else {
            window
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let unit = self.unit;
        let now = Utc::now();
        let window = ChronoDuration::seconds(self.window_secs);
        let left = self.left_edge_secs(now);
        let height = self.resolved_height(ui);
        self.caption(ui, &p);
        let mut plot = Plot::new(self.id)
            .height(height)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .show_background(false)
            .grid_color(p.border)
            .include_x(left)
            .include_x(0.0)
            .include_y(0.0)
            .x_axis_formatter(|mark: GridMark, _| format_offset(mark.value))
            .y_axis_formatter(move |mark: GridMark, _| unit.format(mark.value))
            .label_formatter(move |hover| hover_label(hover, unit));
        if let Some(max) = unit.max_bound() {
            plot = plot.include_y(max);
        }
        plot.show(ui, |plot_ui| {
            for entry in &self.series {
                plot_ui.line(entry.line(window, now));
            }
        });
    }
}

impl PlotSeries<'_> {
    fn line(&self, window: ChronoDuration, now: DateTime<Utc>) -> Line<'static> {
        let points: Vec<[f64; 2]> = self
            .series
            .since(window)
            .map(|point| {
                [
                    (point.at - now).num_milliseconds() as f64 / 1000.0,
                    point.value,
                ]
            })
            .collect();
        Line::new(self.label, PlotPoints::from(points))
            .stroke(Stroke::new(LINE_WIDTH, self.color))
            .fill(0.0)
            .fill_alpha(FILL_ALPHA)
    }
}

fn format_offset(seconds: f64) -> String {
    let total = (-seconds).round() as i64;
    if total <= 0 {
        return "now".to_owned();
    }
    if total < 60 {
        return format!("-{total}s");
    }
    if total % 60 == 0 {
        return format!("-{}m", total / 60);
    }
    format!("-{}:{:02}", total / 60, total % 60)
}

fn hover_label(hover: &HoverPosition<'_>, unit: Unit) -> Option<String> {
    match hover {
        HoverPosition::NearDataPoint {
            plot_name,
            position,
            ..
        } => Some(format!("{plot_name}: {}", unit.format(position.y))),
        HoverPosition::Elsewhere { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_offset_uses_minutes_and_seconds() {
        assert_eq!(format_offset(0.0), "now");
        assert_eq!(format_offset(-45.0), "-45s");
        assert_eq!(format_offset(-600.0), "-10m");
        assert_eq!(format_offset(-500.0), "-8:20");
    }
}
