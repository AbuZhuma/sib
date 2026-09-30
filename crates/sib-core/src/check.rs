use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event::Severity;

pub const OUTPUT_LIMIT: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Ssh,
    Access,
    Firewall,
    Kernel,
    Hardening,
    Resources,
    Reliability,
    Network,
    Updates,
    Logs,
    Collection,
    Custom,
}

impl Area {
    pub const ALL: [Self; 12] = [
        Self::Ssh,
        Self::Access,
        Self::Firewall,
        Self::Kernel,
        Self::Hardening,
        Self::Resources,
        Self::Reliability,
        Self::Network,
        Self::Updates,
        Self::Logs,
        Self::Collection,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Access => "Доступ и права",
            Self::Firewall => "Сеть и файрвол",
            Self::Kernel => "Ядро",
            Self::Hardening => "Защита системы",
            Self::Resources => "Ресурсы",
            Self::Reliability => "Надёжность",
            Self::Network => "Сетевые интерфейсы",
            Self::Updates => "Обновления",
            Self::Logs => "Журнал",
            Self::Collection => "Сбор данных",
            Self::Custom => "Свои проверки",
        }
    }

    pub fn is_security(self) -> bool {
        matches!(
            self,
            Self::Ssh | Self::Access | Self::Firewall | Self::Kernel | Self::Hardening
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weight {
    Low,
    Medium,
    High,
}

impl Weight {
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    pub fn points(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "низкий",
            Self::Medium => "средний",
            Self::High => "высокий",
        }
    }

    pub fn severity(self) -> Severity {
        match self {
            Self::Low => Severity::Info,
            Self::Medium => Severity::Warning,
            Self::High => Severity::Critical,
        }
    }
}

pub type CheckOverrides = BTreeMap<String, CheckOverride>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOverride {
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<Weight>,
}

impl Default for CheckOverride {
    fn default() -> Self {
        Self {
            enabled: true,
            weight: None,
        }
    }
}

impl CheckOverride {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Script,
    LocalFile,
    RemoteFile,
}

impl CheckKind {
    pub const ALL: [Self; 3] = [Self::Script, Self::LocalFile, Self::RemoteFile];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckExpect {
    ExitZero,
    Contains,
    Missing,
}

impl CheckExpect {
    pub const ALL: [Self; 3] = [Self::ExitZero, Self::Contains, Self::Missing];

    pub fn needs_text(self) -> bool {
        !matches!(self, Self::ExitZero)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCheck {
    pub id: String,
    pub name: String,
    pub kind: CheckKind,
    pub source: String,
    pub expect: CheckExpect,
    #[serde(default)]
    pub expect_text: String,
    pub area: Area,
    pub weight: Weight,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub advice: String,
    #[serde(default)]
    pub as_root: bool,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

impl CustomCheck {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            kind: CheckKind::Script,
            source: String::new(),
            expect: CheckExpect::ExitZero,
            expect_text: String::new(),
            area: Area::Custom,
            weight: Weight::Medium,
            description: String::new(),
            advice: String::new(),
            as_root: false,
            enabled: true,
        }
    }

    pub fn is_complete(&self) -> bool {
        if self.name.trim().is_empty() || self.source.trim().is_empty() {
            return false;
        }
        !self.expect.needs_text() || !self.expect_text.trim().is_empty()
    }

    pub fn is_passing(&self, exit_code: i32, output: &str) -> bool {
        match self.expect {
            CheckExpect::ExitZero => exit_code == 0,
            CheckExpect::Contains => output.contains(self.expect_text.trim()),
            CheckExpect::Missing => !output.contains(self.expect_text.trim()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(expect: CheckExpect, text: &str) -> CustomCheck {
        CustomCheck {
            expect,
            expect_text: text.to_owned(),
            name: "проверка".to_owned(),
            source: "true".to_owned(),
            ..CustomCheck::new("custom:1")
        }
    }

    #[test]
    fn exit_zero_check_passes_only_on_zero() {
        let check = check(CheckExpect::ExitZero, "");
        assert!(check.is_passing(0, "что угодно"));
        assert!(!check.is_passing(1, ""));
    }

    #[test]
    fn contains_check_ignores_exit_code() {
        let check = check(CheckExpect::Contains, "active");
        assert!(check.is_passing(3, "unit active (running)"));
        assert!(!check.is_passing(0, "unit dead"));
    }

    #[test]
    fn missing_check_fails_when_text_is_found() {
        let check = check(CheckExpect::Missing, "DEGRADED");
        assert!(check.is_passing(0, "state: ok"));
        assert!(!check.is_passing(0, "state: DEGRADED"));
    }

    #[test]
    fn check_without_expected_text_is_incomplete() {
        assert!(!check(CheckExpect::Contains, "  ").is_complete());
        assert!(check(CheckExpect::Contains, "ok").is_complete());
        assert!(check(CheckExpect::ExitZero, "").is_complete());
    }

    #[test]
    fn check_without_name_or_source_is_incomplete() {
        let mut check = check(CheckExpect::ExitZero, "");
        check.name = "  ".to_owned();
        assert!(!check.is_complete());
        check.name = "проверка".to_owned();
        check.source = String::new();
        assert!(!check.is_complete());
    }
}
