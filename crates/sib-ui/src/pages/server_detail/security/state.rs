use egui::{Context, Id};
use sib_core::ServerId;

const SUBPAGE_KEY: &str = "security-subpage";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Subpage {
    #[default]
    Audit,
    Access,
    Attacks,
    Checks,
    Problem {
        key: String,
        is_evidence_requested: bool,
    },
}

impl Subpage {
    pub fn problem(key: impl Into<String>) -> Self {
        Self::Problem {
            key: key.into(),
            is_evidence_requested: false,
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "security/access" => Some(Self::Access),
            "security/attacks" => Some(Self::Attacks),
            _ => None,
        }
    }
}

pub fn load(ctx: &Context, server: &ServerId) -> Subpage {
    ctx.data(|d| d.get_temp(Id::new((SUBPAGE_KEY, server.as_str()))))
        .unwrap_or_default()
}

pub fn store(ctx: &Context, server: &ServerId, subpage: Subpage) {
    ctx.data_mut(|d| d.insert_temp(Id::new((SUBPAGE_KEY, server.as_str())), subpage));
}
