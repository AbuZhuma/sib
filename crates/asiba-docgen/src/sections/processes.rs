use asiba_modules::processes::{self, Process, ProcessSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, bytes, field, heading, list, subheading, table, truncate};

pub struct ProcessesSection;

const TOP: usize = 10;
const COMMAND_CHARS: usize = 80;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a ProcessSnapshot> {
    ctx.server.data::<ProcessSnapshot>(processes::ID)
}

fn process_row(process: &Process) -> Vec<String> {
    vec![
        process.pid.to_string(),
        process.user.clone(),
        format!("{:.1}", process.cpu_pct.unwrap_or(0.0)),
        bytes(process.rss_bytes),
        process.state.to_string(),
        truncate(process.display_name(), COMMAND_CHARS),
    ]
}

fn top_by<F: Fn(&Process) -> f64>(snapshot: &ProcessSnapshot, key: F) -> Vec<Vec<String>> {
    let mut sorted: Vec<&Process> = snapshot.processes.iter().collect();
    sorted.sort_by(|a, b| key(b).total_cmp(&key(a)));
    sorted.iter().take(TOP).map(|p| process_row(p)).collect()
}

fn counts(snapshot: &ProcessSnapshot) -> (usize, usize, usize) {
    (
        snapshot.processes.len(),
        snapshot.running_count(),
        snapshot.processes.iter().filter(|p| p.is_zombie()).count(),
    )
}

impl Section for ProcessesSection {
    fn id(&self) -> SectionId {
        SectionId::Processes
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        let (total, running, zombies) = counts(snapshot);
        heading(out, "Процессы");
        out.push_str(&format!(
            "Всего {total}, выполняются {running}, зомби {zombies}.\n\n"
        ));
        let headers = ["PID", "Пользователь", "CPU %", "RSS", "Сост.", "Команда"];
        subheading(out, "Топ по CPU");
        table(
            out,
            &headers,
            &top_by(snapshot, |p| p.cpu_pct.unwrap_or(0.0)),
        );
        subheading(out, "Топ по памяти");
        table(out, &headers, &top_by(snapshot, |p| p.rss_bytes as f64));
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        let (total, running, zombies) = counts(snapshot);
        heading(out, "Processes");
        field(out, "total", total.to_string());
        field(out, "running", running.to_string());
        field(out, "zombies", zombies.to_string());
        field(
            out,
            "top_cpu",
            "pid | user | cpu_pct | rss | state | command",
        );
        list(out, "", &top_by(snapshot, |p| p.cpu_pct.unwrap_or(0.0)));
        field(
            out,
            "top_memory",
            "pid | user | cpu_pct | rss | state | command",
        );
        list(out, "", &top_by(snapshot, |p| p.rss_bytes as f64));
        blank(out);
    }
}
