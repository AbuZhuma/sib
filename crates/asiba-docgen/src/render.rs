use chrono::Local;

use crate::section::{DocContext, Section, SectionId};
use crate::sections;
use crate::write;

const HUMAN_STAMP: &str = "_Обновлено:";
const LLM_STAMP: &str = "generated_at:";

pub fn render_human(ctx: &DocContext<'_>) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n{{{{notes}}}}\n\n", ctx.server.spec.id));
    for section in available(ctx, &SectionId::ALL) {
        section.human(&mut out, ctx);
    }
    out.push_str(&format!(
        "{HUMAN_STAMP} {}_\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    out
}

pub fn render_llm(ctx: &DocContext<'_>) -> String {
    let mut out = String::new();
    out.push_str(&llm_header(ctx, &SectionId::ALL));
    for section in available(ctx, &SectionId::ALL) {
        section.llm(&mut out, ctx);
    }
    out.push_str(&format!("{LLM_STAMP} {}\n", write::utc_time(ctx.now)));
    out
}

pub fn render_llm_sections(ctx: &DocContext<'_>, ids: &[SectionId]) -> String {
    let mut out = String::new();
    out.push_str(&llm_header(ctx, ids));
    for section in available(ctx, ids) {
        section.llm(&mut out, ctx);
    }
    out
}

fn llm_header(ctx: &DocContext<'_>, ids: &[SectionId]) -> String {
    let keys: Vec<&str> = available(ctx, ids).iter().map(|s| s.id().key()).collect();
    format!(
        "# Server: {}\nhost: {}\nsections: {}\n\n",
        ctx.server.spec.id,
        ctx.server.spec.host,
        keys.join(", ")
    )
}

fn available(ctx: &DocContext<'_>, ids: &[SectionId]) -> Vec<Box<dyn Section>> {
    ids.iter()
        .filter_map(|id| sections::by_id(*id))
        .filter(|section| section.is_available(ctx))
        .collect()
}

pub fn strip_timestamp(text: &str) -> &str {
    let index = text.rfind(HUMAN_STAMP).or_else(|| text.rfind(LLM_STAMP));
    index.map(|index| &text[..index]).unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use asiba_core::AppState;

    use super::*;
    use crate::test_support::server;

    #[test]
    fn render_human_without_snapshots_has_title_and_description() {
        let state = AppState::default();
        let server = server();
        let text = render_human(&DocContext::new(&server, &state));
        assert!(text.starts_with("# neo\n\n{{notes}}\n\n## Что требует внимания\n"));
        assert!(text.contains("## Описание\n"));
        assert!(text.contains("Проект: Shop"));
        assert!(!text.contains("## Docker"));
    }

    #[test]
    fn render_llm_lists_available_sections_in_header() {
        let state = AppState::default();
        let server = server();
        let text = render_llm(&DocContext::new(&server, &state));
        assert!(text.starts_with("# Server: neo\nhost: h\nsections: findings, description\n"));
        assert!(text.contains("project: Shop"));
        assert!(text.trim_end().ends_with('Z'));
    }

    #[test]
    fn strip_timestamp_removes_trailing_line_in_both_formats() {
        let state = AppState::default();
        let server = server();
        let ctx = DocContext::new(&server, &state);
        assert!(!strip_timestamp(&render_human(&ctx)).contains("_Обновлено"));
        assert!(!strip_timestamp(&render_llm(&ctx)).contains("generated_at"));
    }
}
