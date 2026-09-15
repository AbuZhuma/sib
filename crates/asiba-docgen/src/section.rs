use asiba_core::{AppState, ServerState};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionId {
    Description,
    System,
    Resources,
    Processes,
    Projects,
    Services,
    Docker,
    Ports,
    Deploy,
    Users,
    Updates,
    Logs,
    Security,
    Anomalies,
    Gpu,
    Alerts,
    Actions,
    Events,
    Findings,
}

impl SectionId {
    pub const ALL: [SectionId; 19] = [
        SectionId::Findings,
        SectionId::Description,
        SectionId::System,
        SectionId::Resources,
        SectionId::Alerts,
        SectionId::Security,
        SectionId::Anomalies,
        SectionId::Services,
        SectionId::Docker,
        SectionId::Deploy,
        SectionId::Projects,
        SectionId::Processes,
        SectionId::Ports,
        SectionId::Users,
        SectionId::Updates,
        SectionId::Logs,
        SectionId::Gpu,
        SectionId::Actions,
        SectionId::Events,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::System => "system",
            Self::Resources => "resources",
            Self::Processes => "processes",
            Self::Projects => "projects",
            Self::Services => "services",
            Self::Docker => "docker",
            Self::Ports => "ports",
            Self::Deploy => "deploy",
            Self::Users => "users",
            Self::Updates => "updates",
            Self::Logs => "logs",
            Self::Security => "security",
            Self::Anomalies => "anomalies",
            Self::Gpu => "gpu",
            Self::Alerts => "alerts",
            Self::Actions => "actions",
            Self::Events => "events",
            Self::Findings => "findings",
        }
    }
}

pub struct DocContext<'a> {
    pub server: &'a ServerState,
    pub state: &'a AppState,
    pub now: DateTime<Utc>,
}

impl<'a> DocContext<'a> {
    pub fn new(server: &'a ServerState, state: &'a AppState) -> Self {
        Self {
            server,
            state,
            now: Utc::now(),
        }
    }
}

pub trait Section: Send + Sync {
    fn id(&self) -> SectionId;
    fn is_available(&self, ctx: &DocContext<'_>) -> bool;
    fn human(&self, out: &mut String, ctx: &DocContext<'_>);
    fn llm(&self, out: &mut String, ctx: &DocContext<'_>);
}
