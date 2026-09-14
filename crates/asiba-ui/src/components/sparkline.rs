use asiba_core::Series;
use egui::{Color32, Pos2, Sense, Stroke, Ui, Vec2};

const LINE_WIDTH: f32 = 1.25;

pub fn sparkline(
    ui: &mut Ui,
    series: Option<&Series>,
    size: Vec2,
    color: Color32,
    max: Option<f64>,
) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let Some(series) = series.filter(|s| s.len() > 1) else {
        return;
    };
    let top = max.unwrap_or_else(|| series.max_value()).max(1e-9);
    let count = series.len();
    let step = rect.width() / (count - 1) as f32;
    let points: Vec<Pos2> = series
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let x = rect.left() + step * index as f32;
            let ratio = (point.value / top).clamp(0.0, 1.0) as f32;
            Pos2::new(x, rect.bottom() - ratio * rect.height())
        })
        .collect();
    let painter = ui.painter();
    let fill = color.gamma_multiply(0.18);
    for pair in points.windows(2) {
        let quad = [
            pair[0],
            pair[1],
            Pos2::new(pair[1].x, rect.bottom()),
            Pos2::new(pair[0].x, rect.bottom()),
        ];
        painter.add(egui::Shape::convex_polygon(
            quad.to_vec(),
            fill,
            Stroke::NONE,
        ));
    }
    painter.add(egui::Shape::line(points, Stroke::new(LINE_WIDTH, color)));
}

pub fn sparkline_fill(
    ui: &mut Ui,
    series: Option<&Series>,
    height: f32,
    color: Color32,
    max: Option<f64>,
) {
    let width = ui.available_width();
    sparkline(ui, series, Vec2::new(width, height), color, max);
}
