use std::collections::HashMap;

use asiba_core::ModuleError;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::model::{ServicesSnapshot, Timer, Unit};
use crate::common::sections::Sections;

const MICROS: i64 = 1_000_000;
const SYSTEMD_TIME_FORMAT: &str = "%a %Y-%m-%d %H:%M:%S %z";

pub fn services_snapshot(raw: &str) -> Result<ServicesSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let mut units = units(sections.get_or_empty("units"))?;
    let details = show_blocks(sections.get_or_empty("show"));
    for unit in &mut units {
        if let Some(block) = details.get(unit.name.as_str()) {
            apply_details(unit, block);
        }
    }
    Ok(ServicesSnapshot {
        units,
        timers: timers(sections.get_or_empty("timers")),
    })
}

fn units(raw: &str) -> Result<Vec<Unit>, ModuleError> {
    let parsed: Vec<Unit> = raw.lines().filter_map(unit_line).collect();
    if parsed.is_empty() {
        return Err(ModuleError::Parse("пустой список юнитов".to_owned()));
    }
    Ok(parsed)
}

fn unit_line(line: &str) -> Option<Unit> {
    let mut fields = line.trim_start_matches(['●', '*', ' ']).split_whitespace();
    let name = fields.next()?.to_owned();
    let load = fields.next()?.to_owned();
    let active = fields.next()?.to_owned();
    let sub = fields.next()?.to_owned();
    let description = fields.collect::<Vec<_>>().join(" ");
    Some(Unit {
        name,
        load,
        active,
        sub,
        description,
        restarts: 0,
        main_pid: 0,
        active_since: None,
        fragment_path: String::new(),
        working_directory: String::new(),
        result: String::new(),
    })
}

fn show_blocks(raw: &str) -> HashMap<&str, HashMap<&str, &str>> {
    let mut blocks = HashMap::new();
    for block in raw.split("\n\n") {
        let fields: HashMap<&str, &str> = block.lines().filter_map(|l| l.split_once('=')).collect();
        if let Some(id) = fields.get("Id") {
            blocks.insert(*id, fields);
        }
    }
    blocks
}

fn apply_details(unit: &mut Unit, block: &HashMap<&str, &str>) {
    let field = |key: &str| block.get(key).copied().unwrap_or_default();
    unit.restarts = field("NRestarts").parse().unwrap_or(0);
    unit.main_pid = field("MainPID").parse().unwrap_or(0);
    unit.active_since = systemd_time(field("ActiveEnterTimestamp"));
    unit.fragment_path = field("FragmentPath").to_owned();
    unit.working_directory = field("WorkingDirectory").to_owned();
    unit.result = field("Result").to_owned();
}

fn systemd_time(raw: &str) -> Option<DateTime<Utc>> {
    let fields: Vec<&str> = raw.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    let zone = normalize_zone(fields[3]);
    let text = format!("{} {} {} {zone}", fields[0], fields[1], fields[2]);
    DateTime::parse_from_str(&text, SYSTEMD_TIME_FORMAT)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn normalize_zone(zone: &str) -> String {
    let is_numeric = zone.starts_with(['+', '-']) && zone[1..].chars().all(|c| c.is_ascii_digit());
    match (is_numeric, zone.len()) {
        (true, 3) => format!("{zone}00"),
        (true, _) => zone.to_owned(),
        (false, _) => "+0000".to_owned(),
    }
}

#[derive(Deserialize)]
struct TimerJson {
    unit: String,
    activates: String,
    next: Option<i64>,
    last: Option<i64>,
}

fn timers(raw: &str) -> Vec<Timer> {
    let parsed: Vec<TimerJson> = serde_json::from_str(raw.trim()).unwrap_or_default();
    parsed
        .into_iter()
        .map(|t| Timer {
            name: t.unit,
            activates: t.activates,
            next: micros_to_time(t.next),
            last: micros_to_time(t.last),
        })
        .collect()
}

fn micros_to_time(value: Option<i64>) -> Option<DateTime<Utc>> {
    let micros = value.filter(|v| *v > 0)?;
    DateTime::from_timestamp(micros / MICROS, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::model::UnitOrigin;

    const FEDORA: &str = include_str!("../../fixtures/services/fedora.txt");

    #[test]
    fn fixture_parses_units_with_details() {
        let snapshot = services_snapshot(FEDORA).expect("parse");
        let cups = snapshot
            .units
            .iter()
            .find(|u| u.name == "cups.service")
            .expect("cups");
        assert_eq!(cups.main_pid, 1002);
        assert_eq!(cups.origin(), UnitOrigin::Vendor);
        assert!(cups.active_since.is_some());
        let app = snapshot
            .units
            .iter()
            .find(|u| u.name == "myapp.service")
            .expect("myapp");
        assert_eq!(app.origin(), UnitOrigin::Custom);
        assert!(app.is_failed());
        assert_eq!(app.restarts, 5);
        assert_eq!(app.result, "exit-code");
    }

    #[test]
    fn fixture_parses_timers_json() {
        let snapshot = services_snapshot(FEDORA).expect("parse");
        assert_eq!(snapshot.timers.len(), 2);
        assert_eq!(snapshot.timers[0].activates, "dnf-makecache.service");
        assert!(snapshot.timers[1].last.is_none());
    }

    #[test]
    fn systemd_time_accepts_short_and_named_zones() {
        let short = systemd_time("Mon 2026-09-14 11:06:53 +06").expect("short zone");
        let named = systemd_time("Mon 2026-09-14 05:06:53 UTC").expect("named zone");
        assert_eq!(short, named);
        assert_eq!(systemd_time(""), None);
    }

    #[test]
    fn empty_units_is_parse_error() {
        assert!(matches!(
            services_snapshot("###units\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
