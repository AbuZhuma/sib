use std::sync::Arc;

use sib_ai::{Completion, ContextBuilder, Part, Playbook, SYSTEM_PROMPT, estimate_tokens};
use sib_config::AiConfig;
use sib_core::{
    AppState, AuditScope, AuditTarget, ModuleId, ModuleRegistry, QueryRequest, Severity,
    SharedState, Transport,
};
use sib_docgen::{DocContext, SectionId};

use super::job::AuditJob;

const ANSWER_TOKENS: usize = 4096;
const RESERVE_TOKENS: usize = 500;
const QUERY_TAIL_LINES: usize = 300;
const PRIORITY_FINDINGS: u8 = 0;
const PRIORITY_QUERY: u8 = 1;
const PRIORITY_SECTION_BASE: u8 = 2;
const PRIORITY_FLEET_BASE: u8 = 10;

pub struct Prepared {
    pub completion: Completion,
    pub context_tokens: usize,
}

pub async fn prepare(
    job: &AuditJob,
    state: &SharedState,
    registry: &ModuleRegistry,
    config: &AiConfig,
) -> Option<Prepared> {
    let playbook = select_playbook(job)?;
    let mut builder = ContextBuilder::new(budget(config, job));
    for part in query_parts(job, registry).await {
        builder = builder.part(part);
    }
    let parts = match &job.target {
        AuditTarget::Server(server) => server_parts(state, server, &playbook)?,
        AuditTarget::Fleet => fleet_parts(state, &playbook)?,
    };
    for part in parts {
        builder = builder.part(part);
    }
    let (context, context_tokens) = builder.build();
    let user = sib_ai::build_user(&context, &playbook, job.incident.as_ref());
    Some(Prepared {
        completion: Completion {
            system: SYSTEM_PROMPT.to_owned(),
            user,
            max_tokens: ANSWER_TOKENS,
        },
        context_tokens,
    })
}

fn select_playbook(job: &AuditJob) -> Option<Playbook> {
    match (&job.target, &job.scope) {
        (AuditTarget::Fleet, _) => Some(sib_ai::fleet_audit()),
        (_, AuditScope::Incident { .. }) => job.incident.as_ref().map(|i| sib_ai::playbook(i.kind)),
        (_, AuditScope::Section { key }) => sib_ai::section_audit(key),
        (_, AuditScope::Full) => Some(sib_ai::full_audit()),
    }
}

fn budget(config: &AiConfig, job: &AuditJob) -> usize {
    let incident = job
        .incident
        .as_ref()
        .map(|i| {
            estimate_tokens(&i.summary)
                + i.evidence.iter().map(|e| estimate_tokens(e)).sum::<usize>()
        })
        .unwrap_or(0);
    config
        .context_tokens
        .saturating_sub(estimate_tokens(SYSTEM_PROMPT) + ANSWER_TOKENS + RESERVE_TOKENS + incident)
}

fn section_priority(index: usize, id: SectionId, base: u8) -> u8 {
    if id == SectionId::Findings {
        PRIORITY_FINDINGS
    } else {
        base.saturating_add(index as u8)
    }
}

fn server_parts(
    state: &SharedState,
    server: &sib_core::ServerId,
    playbook: &Playbook,
) -> Option<Vec<Part>> {
    let state = state.read().ok()?;
    let server = state.servers.get(server)?;
    let ctx = DocContext::new(server, &state);
    let parts = playbook
        .sections
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let text = sib_docgen::render_llm_section(&ctx, *id)?;
            Some(Part::new(
                section_priority(index, *id, PRIORITY_SECTION_BASE),
                id.key(),
                text,
            ))
        })
        .collect();
    Some(parts)
}

fn worst_severity(state: &AppState, server: &sib_core::ServerId) -> u8 {
    state
        .active_incidents()
        .filter(|i| &i.server == server)
        .map(|i| match i.severity {
            Severity::Critical => 0,
            Severity::Warning => 1,
            Severity::Info => 2,
        })
        .min()
        .unwrap_or(3)
}

fn fleet_parts(state: &SharedState, playbook: &Playbook) -> Option<Vec<Part>> {
    let state = state.read().ok()?;
    let mut parts = Vec::new();
    for server in state.servers.values() {
        let ctx = DocContext::new(server, &state);
        let mut text = format!("# Server: {}\n\n", server.spec.id);
        for id in playbook.sections {
            if let Some(section) = sib_docgen::render_llm_section(&ctx, *id) {
                text.push_str(&section);
            }
        }
        let priority = PRIORITY_FLEET_BASE.saturating_add(worst_severity(&state, &server.spec.id));
        parts.push(Part::new(priority, server.spec.id.to_string(), text));
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts)
}

async fn query_parts(job: &AuditJob, registry: &ModuleRegistry) -> Vec<Part> {
    let (Some(incident), Some(transport)) = (&job.incident, &job.transport) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for (module, request) in sib_ai::queries(incident) {
        if let Some(text) = run_query(registry, Arc::clone(transport), module, &request).await {
            let title = format!("{}.{} {}", module.0, request.kind, request.target);
            parts.push(Part::new(
                PRIORITY_QUERY,
                title.clone(),
                format!("## {title}\n\n{text}\n"),
            ));
        }
    }
    parts
}

async fn run_query(
    registry: &ModuleRegistry,
    transport: Arc<dyn Transport>,
    module: ModuleId,
    request: &QueryRequest,
) -> Option<String> {
    let module = registry.get(module)?;
    match module.query(transport.as_ref(), request).await {
        Ok(response) => Some(tail(&response.text, QUERY_TAIL_LINES)),
        Err(error) => {
            tracing::warn!(kind = %request.kind, target = %request.target, %error, "the query for the audit failed");
            None
        }
    }
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}
