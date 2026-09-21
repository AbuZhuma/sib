mod activity;
mod model;
mod parse;

use asiba_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings,
    QueryRequest, QueryResponse, Sample, Schedule, Severity, Snapshot, Transport,
};
use async_trait::async_trait;

pub use activity::{COMMAND_PREFIX, HEADING_PREFIX, QUERY_ACTIVITY};
pub use model::{Account, LoginRecord, Session, UsersSnapshot};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("users");
pub const KEY_SESSIONS: &str = "users.sessions";

const SCRIPT_PARTS: [(&str, &str); 5] = [
    ("who", "who"),
    ("last", "last -F -n 30 -w"),
    ("passwd", "getent passwd"),
    ("sudoers", "getent group sudo wheel admin"),
    (
        "keys",
        "for d in /root /home/*; do f=\"$d/.ssh/authorized_keys\"; [ -r \"$f\" ] && printf '%s %s\\n' \"$d\" \"$(grep -c -E '^(ssh|ecdsa)' \"$f\")\"; done",
    ),
];

pub struct UsersModule;

#[async_trait]
impl Module for UsersModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Пользователи"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(
        &self,
        _transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let snapshot = parse::users_snapshot(&output.stdout)?;
        let events = context
            .previous::<UsersSnapshot>()
            .map(|(previous, _)| events_between(previous, &snapshot))
            .unwrap_or_default();
        let samples = vec![Sample::new(KEY_SESSIONS, snapshot.sessions.len() as f64)];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }

    async fn query(
        &self,
        transport: &dyn Transport,
        request: &QueryRequest,
    ) -> Result<QueryResponse, ModuleError> {
        if request.kind != QUERY_ACTIVITY {
            return Err(ModuleError::UnsupportedQuery(request.kind.clone()));
        }
        activity::query(transport, &request.target).await
    }
}

fn events_between(previous: &UsersSnapshot, current: &UsersSnapshot) -> Vec<Event> {
    let mut events = Vec::new();
    for session in &current.sessions {
        if !previous.sessions.iter().any(|s| s.same(session)) {
            let from = session.from.as_deref().unwrap_or("local");
            let message = format!("вход {} с {from} ({})", session.user, session.tty);
            events.push(Event::new(ID, Severity::Info, message));
        }
    }
    for account in &current.accounts {
        if !previous.accounts.iter().any(|a| a.name == account.name) {
            events.push(Event::new(
                ID,
                Severity::Warning,
                format!("новый пользователь {}", account.name),
            ));
        }
    }
    for (home, count) in &current.authorized_keys {
        let before = previous
            .authorized_keys
            .iter()
            .find(|(h, _)| h == home)
            .map(|(_, c)| *c);
        if before.is_some_and(|b| b < *count) {
            events.push(Event::new(
                ID,
                Severity::Warning,
                format!("новый ключ в {home}/.ssh/authorized_keys"),
            ));
        }
    }
    events
}
