use std::path::PathBuf;

use asiba_core::{AppState, ConnectionStatus, ServerId, ServerState};
use egui::{Align2, Color32, FontId, Id, Pos2, Response, Stroke, Ui};
use walkers::sources::OpenStreetMap;
use walkers::{
    HeaderValue, HttpOptions, HttpTiles, Map, MapMemory, Plugin, Position, Projector, Tiles,
    lat_lon,
};

use crate::components::status::status_color;
use crate::theme::Palette;

const MARKER_RADIUS: f32 = 6.0;
const HOME_RADIUS: f32 = 4.0;
const HIT_RADIUS: f32 = 12.0;
const LABEL_OFFSET: f32 = 9.0;
const LABEL_PADDING: f32 = 2.0;
const LINE_WIDTH: f32 = 1.0;
const PING_FONT_SIZE: f32 = 11.0;
const LABEL_FONT_SIZE: f32 = 12.0;
const WORLD_ZOOM: f64 = 3.0;
const CLICKED_KEY: &str = "map-clicked-server";
const USER_AGENT: &str = concat!("asiba/", env!("CARGO_PKG_VERSION"));

pub struct MapState {
    tiles: HttpTiles,
    memory: MapMemory,
    centered: bool,
}

impl MapState {
    pub fn new(ctx: &egui::Context, cache: PathBuf) -> Self {
        let options = HttpOptions {
            cache: Some(cache),
            user_agent: Some(HeaderValue::from_static(USER_AGENT)),
            ..Default::default()
        };
        let mut memory = MapMemory::default();
        let _ = memory.set_zoom(WORLD_ZOOM);
        Self {
            tiles: HttpTiles::with_options(OpenStreetMap, options, ctx.clone()),
            memory,
            centered: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &AppState, interactive: bool) -> Option<ServerId> {
        let markers = Markers::collect(state, &Palette::current(ui.ctx()));
        let home = markers.home.unwrap_or_else(|| lat_lon(50.0, 10.0));
        if !self.centered && (markers.home.is_some() || !markers.servers.is_empty()) {
            self.memory.center_at(markers.center().unwrap_or(home));
            self.centered = true;
        }
        let attribution = self.tiles.attribution();
        Map::new(Some(&mut self.tiles), &mut self.memory, home)
            .zoom_gesture(interactive)
            .panning(interactive)
            .double_click_to_zoom(interactive)
            .with_plugin(markers)
            .show(ui, |ui, _response, _projector, _memory| {
                attribution_label(ui, attribution.text, attribution.url);
            });
        let id = Id::new(CLICKED_KEY);
        let clicked: Option<ServerId> = ui.ctx().data(|d| d.get_temp(id));
        if clicked.is_some() {
            ui.ctx().data_mut(|d| d.remove::<ServerId>(id));
        }
        clicked
    }
}

fn attribution_label(ui: &mut Ui, text: &str, url: &str) {
    let rect = ui.max_rect();
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
            ui.hyperlink_to(egui::RichText::new(format!("© {text}")).small(), url);
        });
    });
}

struct Marker {
    id: ServerId,
    position: Position,
    color: Color32,
    label: String,
    ping: Option<String>,
}

struct Markers {
    servers: Vec<Marker>,
    home: Option<Position>,
    line: Color32,
    text: Color32,
    label_bg: Color32,
}

impl Markers {
    fn collect(state: &AppState, p: &Palette) -> Self {
        let servers = state
            .servers
            .values()
            .filter_map(|server| marker(server, p))
            .collect();
        Self {
            servers,
            home: state.self_location.as_ref().map(|l| lat_lon(l.lat, l.lon)),
            line: p.accent,
            text: p.text,
            label_bg: p.bg_panel.gamma_multiply(0.85),
        }
    }

    fn center(&self) -> Option<Position> {
        let mut points: Vec<Position> = self.servers.iter().map(|m| m.position).collect();
        points.extend(self.home);
        if points.is_empty() {
            return None;
        }
        let count = points.len() as f64;
        let lat = points.iter().map(|p| p.y()).sum::<f64>() / count;
        let lon = points.iter().map(|p| p.x()).sum::<f64>() / count;
        Some(lat_lon(lat, lon))
    }
}

fn marker(server: &ServerState, p: &Palette) -> Option<Marker> {
    let location = server.location.as_ref()?;
    let ping = match &server.connection {
        ConnectionStatus::Online { .. } => server
            .ping
            .as_ref()
            .and_then(|ping| ping.rtt_ms)
            .map(|rtt| format!("{rtt:.0} ms")),
        _ => None,
    };
    Some(Marker {
        id: server.spec.id.clone(),
        position: lat_lon(location.lat, location.lon),
        color: status_color(p, &server.connection),
        label: server.spec.id.to_string(),
        ping,
    })
}

impl Markers {
    fn label(&self, painter: &egui::Painter, at: Pos2, anchor: Align2, text: &str, font: FontId) {
        let galley = painter.layout_no_wrap(text.to_owned(), font, self.text);
        let rect = anchor.anchor_size(at, galley.size());
        painter.rect_filled(rect.expand(LABEL_PADDING), 2.0, self.label_bg);
        painter.galley(rect.min, galley, self.text);
    }

    fn ping_line(&self, painter: &egui::Painter, home: Pos2, at: Pos2, ping: Option<&str>) {
        painter.line_segment([home, at], Stroke::new(LINE_WIDTH, self.line));
        let Some(ping) = ping else {
            return;
        };
        let middle = Pos2::new((home.x + at.x) / 2.0, (home.y + at.y) / 2.0);
        self.label(
            painter,
            middle,
            Align2::CENTER_CENTER,
            ping,
            FontId::monospace(PING_FONT_SIZE),
        );
    }
}

impl Plugin for Markers {
    fn run(
        self: Box<Self>,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
        _memory: &MapMemory,
    ) {
        let painter = ui.painter().with_clip_rect(response.rect);
        let home = self.home.map(|h| projector.project(h).to_pos2());
        if let Some(home) = home {
            painter.circle_filled(home, HOME_RADIUS, self.text);
        }
        let pointer = response.interact_pointer_pos();
        for marker in &self.servers {
            let at = projector.project(marker.position).to_pos2();
            if let Some(home) = home {
                self.ping_line(&painter, home, at, marker.ping.as_deref());
            }
            painter.circle_filled(at, MARKER_RADIUS, marker.color);
            painter.circle_stroke(at, MARKER_RADIUS, Stroke::new(LINE_WIDTH, self.text));
            self.label(
                &painter,
                Pos2::new(at.x + LABEL_OFFSET, at.y),
                Align2::LEFT_CENTER,
                &marker.label,
                FontId::proportional(LABEL_FONT_SIZE),
            );
            if response.clicked() && pointer.is_some_and(|pos| pos.distance(at) <= HIT_RADIUS) {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(Id::new(CLICKED_KEY), marker.id.clone()));
            }
        }
    }
}
