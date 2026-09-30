use sib_core::{Availability, SudoMode};

use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ADVICE_MODULES: &str = "Look at the error text in the module table on the server summary.";
const ADVICE_PARTIAL: &str = "Set up sudo or add the user to the groups it needs.";
const ADVICE_SUDO: &str = "Set sudo in the server settings.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "collection.module_errors",
        area: Area::Collection,
        subject: "Module collection errors",
        description: "Modules whose last collect failed. Their data in the audit is stale or missing.",
        weight: Weight::Low,
        advice: ADVICE_MODULES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: module_errors,
    },
    Pattern {
        id: "collection.partial",
        area: Area::Collection,
        subject: "Incomplete module data",
        description: "Modules that lack permissions for part of their data.",
        weight: Weight::Low,
        advice: ADVICE_PARTIAL,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: partial,
    },
    Pattern {
        id: "collection.root",
        area: Area::Collection,
        subject: "Root access for checks",
        description: "Without root some security checks are skipped: sshd -T, /etc/shadow, firewall rules.",
        weight: Weight::Low,
        advice: ADVICE_SUDO,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: root,
    },
];

fn module_errors(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let failing: Vec<String> = ctx
        .server
        .modules
        .iter()
        .filter_map(|(id, state)| state.last_error.as_ref().map(|e| format!("{}: {e}", id.0)))
        .collect();
    Verdict::graded(false, !failing.is_empty(), list_or(&failing, "no errors")).single()
}

fn partial(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let partial: Vec<String> = ctx
        .server
        .modules
        .iter()
        .filter_map(|(id, state)| match &state.availability {
            Availability::Partial { missing } => Some(format!("{}: {}", id.0, missing.join(", "))),
            _ => None,
        })
        .collect();
    Verdict::graded(false, !partial.is_empty(), list_or(&partial, "no limits")).single()
}

fn root(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let spec = &ctx.server.spec;
    let has_sudo = spec.sudo != SudoMode::None || spec.user == "root";
    Verdict::graded(
        false,
        !has_sudo,
        format!("user {}, sudo {:?}", spec.user, spec.sudo),
    )
    .single()
}
