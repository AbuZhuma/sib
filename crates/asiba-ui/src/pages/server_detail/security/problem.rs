use asiba_core::{ModuleId, QueryRequest, ServerId};
use asiba_incidents::{AuditCheck, Evidence, SystemAudit, Weight};
use asiba_modules::files;
use egui::{Label, RichText, Ui};

use super::DetailContext;
use super::state::Subpage;
use crate::components::badge;
use crate::modules::Tab;
use crate::modules::security_view::outcome_color;
use crate::pages::Action;
use crate::pages::server_detail::files::FileBrowser;
use crate::pages::server_detail::select_tab;
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

const FILE_HEIGHT: f32 = 320.0;

pub fn show(
    ui: &mut Ui,
    ctx: &DetailContext<'_>,
    audit: &SystemAudit,
    key: &str,
    subpage: &mut Subpage,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    if ui.button(text::SEC_BACK_TO_LIST).clicked() {
        *subpage = Subpage::Audit;
        return None;
    }
    ui.add_space(GAP);
    let Some(check) = audit.find(key) else {
        ui.label(RichText::new(text::SEC_PROBLEM_GONE).color(p.text_muted));
        return None;
    };
    header(ui, check, &p);
    ui.add_space(GAP);
    section(ui, text::SEC_STATE, &check.detail, &p);
    section(ui, text::SEC_WHAT_IS_CHECKED, check.description, &p);
    section(ui, text::SEC_WHAT_TO_DO, check.advice, &p);
    evidence(ui, ctx, check, subpage)
}

fn header(ui: &mut Ui, check: &AuditCheck, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.heading(check.title());
        badge(ui, outcome_label(check), outcome_color(check.outcome, p));
        ui.label(
            RichText::new(format!(
                "{} · {}",
                check.area.label(),
                weight_label(check.weight)
            ))
            .small()
            .color(p.text_secondary),
        );
    });
}

fn outcome_label(check: &AuditCheck) -> &'static str {
    match check.outcome {
        asiba_incidents::Outcome::Pass => text::SEC_OUTCOME_PASS,
        asiba_incidents::Outcome::Warn => text::SEC_OUTCOME_WARN,
        asiba_incidents::Outcome::Fail => text::SEC_OUTCOME_FAIL,
        asiba_incidents::Outcome::Skipped => text::SEC_OUTCOME_SKIPPED,
    }
}

fn weight_label(weight: Weight) -> &'static str {
    match weight {
        Weight::Low => text::SEC_WEIGHT_LOW,
        Weight::Medium => text::SEC_WEIGHT_MEDIUM,
        Weight::High => text::SEC_WEIGHT_HIGH,
    }
}

fn section(ui: &mut Ui, title: &str, body: &str, p: &Palette) {
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    ui.add(Label::new(body).wrap());
    ui.add_space(GAP);
}

fn evidence(
    ui: &mut Ui,
    ctx: &DetailContext<'_>,
    check: &AuditCheck,
    subpage: &mut Subpage,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let server = &ctx.server.spec.id;
    match &check.evidence {
        Evidence::None => None,
        Evidence::File(path) => {
            ui.label(
                RichText::new(format!("{} {path}", text::SEC_EVIDENCE_FILE).to_uppercase())
                    .small()
                    .color(p.text_secondary),
            );
            file_block(ui, server, ctx.files, path, subpage)
        }
        Evidence::Query {
            module,
            kind,
            target,
        } => {
            let label = format!("{} {kind} {target}", text::SEC_EVIDENCE_SHOW);
            let clicked = ui.button(label).clicked();
            if !clicked && !mark_requested(subpage) {
                return None;
            }
            Some(Action::Query {
                server: server.clone(),
                module: ModuleId(module),
                request: QueryRequest::new(*kind, target),
            })
        }
        Evidence::Tab(key) => {
            open_tab_evidence(ui, key, subpage);
            None
        }
    }
}

fn open_tab_evidence(ui: &mut Ui, key: &str, subpage: &mut Subpage) {
    if !ui.button(text::SEC_EVIDENCE_OPEN).clicked() {
        return;
    }
    if let Some(target) = Subpage::from_key(key) {
        *subpage = target;
    } else if let Some(tab) = Tab::from_key(key) {
        select_tab(ui.ctx(), tab);
    }
}

fn mark_requested(subpage: &mut Subpage) -> bool {
    let Subpage::Problem {
        is_evidence_requested,
        ..
    } = subpage
    else {
        return false;
    };
    if *is_evidence_requested {
        return false;
    }
    *is_evidence_requested = true;
    true
}

fn file_block(
    ui: &mut Ui,
    server: &ServerId,
    browser: Option<&FileBrowser>,
    path: &str,
    subpage: &mut Subpage,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    if let Some(content) = browser.and_then(|b| b.content(path)) {
        crate::components::scroll::vertical()
            .id_salt(("evidence-file", path))
            .max_height(FILE_HEIGHT)
            .show(ui, |ui| {
                ui.add(Label::new(RichText::new(content).monospace()).wrap());
            });
        return None;
    }
    if let Some(error) = browser.and_then(|b| b.error(path)) {
        ui.label(RichText::new(error).color(p.critical));
        return None;
    }
    ui.add_space(GAP_SMALL);
    ui.label(RichText::new(text::FILES_LOADING).color(p.text_muted));
    if browser.is_some_and(|b| b.is_loading(path)) || !mark_requested(subpage) {
        return None;
    }
    Some(Action::FilesQuery {
        server: server.clone(),
        request: QueryRequest::new(files::QUERY_READ, path),
    })
}
