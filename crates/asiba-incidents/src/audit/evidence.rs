#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    None,
    File(String),
    Query {
        module: &'static str,
        kind: &'static str,
        target: String,
    },
    Tab(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    None,
    File(&'static str),
    Query {
        module: &'static str,
        kind: &'static str,
        target: &'static str,
    },
    Tab(&'static str),
}

impl EvidenceSource {
    pub fn to_evidence(self) -> Evidence {
        match self {
            Self::None => Evidence::None,
            Self::File(path) => Evidence::File(path.to_owned()),
            Self::Query {
                module,
                kind,
                target,
            } => Evidence::Query {
                module,
                kind,
                target: target.to_owned(),
            },
            Self::Tab(tab) => Evidence::Tab(tab),
        }
    }
}

pub const TAB_PORTS: &str = "ports";
pub const TAB_LOGS: &str = "logs";
pub const TAB_PROCESSES: &str = "processes";
pub const TAB_RESOURCES: &str = "resources";
pub const TAB_SERVICES: &str = "services";
pub const TAB_DOCKER: &str = "docker";
pub const TAB_DEPLOY: &str = "deploy";
pub const TAB_NETWORK: &str = "network";
pub const TAB_ANOMALIES: &str = "anomalies";
pub const TAB_GPU: &str = "gpu";
pub const TAB_SUMMARY: &str = "summary";
pub const TAB_CHECKS: &str = "checks";
pub const SECURITY_ATTACKS: &str = "security/attacks";
pub const SECURITY_ACCESS: &str = "security/access";
