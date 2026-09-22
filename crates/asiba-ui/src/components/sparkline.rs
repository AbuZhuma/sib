use asiba_core::Series;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

const LINE_WIDTH: f32 = 1.25;
const MIN_TOP: f64 = 1e-9;
const FILL_ALPHA: f32 = 0.18;

fn polyline(series: &Series, rect: Rect, max: Option<f64>) -> Vec<Pos2> {
    let top = max.unwrap_or_else(|| series.max_value()).max(MIN_TOP);
    let values = downsample(series, rect.width() as usize);
    let step = rect.width() / (values.len().max(2) - 1) as f32;
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = rect.left() + step * index as f32;
            let ratio = (value / top).clamp(0.0, 1.0) as f32;
            Pos2::new(x, rect.bottom() - ratio * rect.height())
        })
        .collect()
}

pub fn sparkline(
    ui: &mut Ui,
    series: Option<&Series>,
    size: Vec2,
    color: Color32,
    max: Option<f64>,
) {
    let (allocated, _) = ui.allocate_exact_size(size, Sense::hover());
    let rect = Rect::from_center_size(allocated.center(), size);
    let Some(series) = series.filter(|s| s.len() > 1) else {
        return;
    };
    let points = polyline(series, rect, max);
    let painter = ui.painter();
    let fill = color.gamma_multiply(FILL_ALPHA);
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

fn downsample(series: &Series, columns: usize) -> Vec<f64> {
    let values: Vec<f64> = series.iter().map(|p| p.value).collect();
    let columns = columns.max(2);
    if values.len() <= columns {
        return values;
    }
    let bucket = values.len().div_ceil(columns);
    values
        .chunks(bucket)
        .map(|chunk| chunk.iter().copied().fold(f64::MIN, f64::max))
        .collect()
}
