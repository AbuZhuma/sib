use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::{ColorImage, Context, Event, UserData, ViewportCommand};

use asiba_core::ServerId;

use crate::modules::Tab;
use crate::pages::Page;

const ENV_SCREENSHOT: &str = "ASIBA_SCREENSHOT";
const ENV_OPEN: &str = "ASIBA_OPEN";
const ENV_SCREENSHOT_DELAY: &str = "ASIBA_SCREENSHOT_DELAY";
const DEFAULT_DELAY: Duration = Duration::from_secs(4);

pub fn start_page() -> Option<(Page, Option<Tab>)> {
    let value = std::env::var(ENV_OPEN).ok()?;
    let fixed = match value.as_str() {
        "overview" => Some(Page::Overview),
        "servers" => Some(Page::Servers),
        "alerts" => Some(Page::Alerts),
        "map" => Some(Page::Map),
        "settings" => Some(Page::Settings),
        _ => None,
    };
    if let Some(page) = fixed {
        return Some((page, None));
    }
    let (server, tab) = match value.split_once('/') {
        Some((server, tab)) => (server, Tab::from_key(tab)),
        None => (value.as_str(), None),
    };
    let id = ServerId::parse(server).ok()?;
    Some((Page::ServerDetail(id), tab))
}

pub struct ScreenshotOnStart {
    target: PathBuf,
    due: Instant,
    requested: bool,
}

impl ScreenshotOnStart {
    pub fn from_env() -> Option<Self> {
        let target = PathBuf::from(std::env::var_os(ENV_SCREENSHOT)?);
        let delay = std::env::var(ENV_SCREENSHOT_DELAY)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_DELAY);
        Some(Self {
            target,
            due: Instant::now() + delay,
            requested: false,
        })
    }

    pub fn tick(&mut self, ctx: &Context) {
        if !self.requested && Instant::now() >= self.due {
            self.requested = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
        }
        let captured = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = captured {
            self.save(&image);
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn save(&self, image: &ColorImage) {
        let [width, height] = image.size;
        let result = image::save_buffer(
            &self.target,
            image.as_raw(),
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        );
        match result {
            Ok(()) => tracing::info!(path = %self.target.display(), "снимок экрана сохранён"),
            Err(error) => tracing::error!(%error, "не удалось сохранить снимок"),
        }
    }
}
