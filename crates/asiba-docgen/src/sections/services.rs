use asiba_modules::services::{self, ServicesSnapshot, Unit, UnitOrigin};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, field, heading, list, local_time, table};

pub struct ServicesSection;

const MAX_UNITS: usize = 40;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a ServicesSnapshot> {
    ctx.server.data::<ServicesSnapshot>(services::ID)
}

fn shown(snapshot: &ServicesSnapshot) -> Vec<&Unit> {
    let mut units: Vec<&Unit> = snapshot
        .units
        .iter()
        .filter(|u| u.origin() == UnitOrigin::Custom || u.is_failed())
        .collect();
    units.sort_by_key(|u| !u.is_failed());
    units.truncate(MAX_UNITS);
    units
}

fn row(unit: &Unit) -> Vec<String> {
    vec![
        unit.name.clone(),
        format!("{} ({})", unit.active, unit.sub),
        unit.restarts.to_string(),
        unit.active_since
            .map(local_time)
            .unwrap_or_else(|| NONE.to_owned()),
        unit.description.clone(),
    ]
}

fn summary(snapshot: &ServicesSnapshot) -> (usize, usize, usize) {
    (
        snapshot.units.len(),
        snapshot.active_count(),
        snapshot.failed().count(),
    )
}

impl Section for ServicesSection {
    fn id(&self) -> SectionId {
        SectionId::Services
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        let (total, active, failed) = summary(snapshot);
        heading(out, "Сервисы systemd");
        out.push_str(&format!(
            "Юнитов {total}, активных {active}, упавших {failed}. Показаны пользовательские и упавшие.\n\n"
        ));
        let rows: Vec<Vec<String>> = shown(snapshot).into_iter().map(row).collect();
        table(
            out,
            &["Юнит", "Состояние", "Перезапусков", "Активен с", "Описание"],
            &rows,
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        let (total, active, failed) = summary(snapshot);
        heading(out, "Systemd services");
        field(out, "total", total.to_string());
        field(out, "active", active.to_string());
        field(out, "failed", failed.to_string());
        field(
            out,
            "units",
            "name | state | restarts | active_since | description",
        );
        let rows: Vec<Vec<String>> = shown(snapshot).into_iter().map(row).collect();
        list(out, "", &rows);
        blank(out);
    }
}
