use std::sync::Arc;

use asiba_config::LlmConfig;
use asiba_core::{ModuleId, ModuleRegistry, QueryRequest, SharedState, Transport};
use asiba_docgen::{DocContext, SectionId};
use asiba_llm::{Completion, ContextBuilder, Part, Playbook, SYSTEM_PROMPT, estimate_tokens};

use super::job::AuditJob;

const ANSWER_TOKENS: usize = 1200;
const RESERVE_TOKENS: usize = 300;
const QUERY_TAIL_LINES: usize = 150;
const PRIORITY_FINDINGS: u8 = 0;
const PRIORITY_QUERY: u8 = 1;
const PRIORITY_SECTION_BASE: u8 = 2;

pub struct Prepared {
    pub completion: Completion,
    pub context_tokens: usize,
}

pub async fn prepare(
    job: &AuditJob,
    state: &SharedState,
    registry: &ModuleRegistry,
    config: &LlmConfig,
) -> Option<Prepared> {
    let playbook = match &job.incident {
        Some(incident) => asiba_llm::playbook(incident.kind),
        None => asiba_llm::full_audit(),
    };
    let mut builder = ContextBuilder::new(budget(config, job));
    for part in query_parts(job, registry).await {
        builder = builder.part(part);
    }
    for part in section_parts(job, state, &playbook)? {
        builder = builder.part(part);
    }
    let (context, context_tokens) = builder.build();
    let user = asiba_llm::build_user(&context, playbook.task, job.incident.as_ref());
    Some(Prepared {
        completion: Completion {
            system: SYSTEM_PROMPT.to_owned(),
            user,
            max_tokens: ANSWER_TOKENS,
        },
        context_tokens,
    })
}

fn budget(config: &LlmConfig, job: &AuditJob) -> usize {
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

fn section_parts(job: &AuditJob, state: &SharedState, playbook: &Playbook) -> Option<Vec<Part>> {
    let state = state.read().ok()?;
    let server = state.servers.get(&job.server)?;
    let ctx = DocContext::new(server, &state);
    let parts = playbook
        .sections
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let text = asiba_docgen::render_llm_section(&ctx, *id)?;
            let priority = if *id == SectionId::Findings {
                PRIORITY_FINDINGS
            } else {
                PRIORITY_SECTION_BASE.saturating_add(index as u8)
            };
            Some(Part::new(priority, id.key(), text))
        })
        .collect();
    Some(parts)
}

async fn query_parts(job: &AuditJob, registry: &ModuleRegistry) -> Vec<Part> {
    let (Some(incident), Some(transport)) = (&job.incident, &job.transport) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for (module, request) in asiba_llm::queries(incident) {
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
            tracing::warn!(kind = %request.kind, target = %request.target, %error, "запрос для аудита не выполнен");
            None
        }
    }
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}
