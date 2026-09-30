use sib_core::{AlertRule, Condition, METRIC_OFFLINE, Severity};
use sib_modules::{anomalies, cpu, disk, logs, memory, security, services, updates};

struct BuiltinRule {
    id: &'static str,
    name: &'static str,
    metric: &'static str,
    condition: Condition,
    threshold: f64,
    for_secs: u64,
    severity: Severity,
}

const BUILTIN: [BuiltinRule; 10] = [
    BuiltinRule {
        id: "offline",
        name: "Server is offline",
        metric: METRIC_OFFLINE,
        condition: Condition::Above,
        threshold: 0.5,
        for_secs: 60,
        severity: Severity::Critical,
    },
    BuiltinRule {
        id: "cpu-high",
        name: "CPU above 90%",
        metric: cpu::KEY_TOTAL,
        condition: Condition::Above,
        threshold: 90.0,
        for_secs: 300,
        severity: Severity::Warning,
    },
    BuiltinRule {
        id: "memory-high",
        name: "Memory above 90%",
        metric: memory::KEY_USED_PCT,
        condition: Condition::Above,
        threshold: 90.0,
        for_secs: 120,
        severity: Severity::Warning,
    },
    BuiltinRule {
        id: "disk-full",
        name: "Root disk 90% full",
        metric: disk::KEY_ROOT_USED_PCT,
        condition: Condition::Above,
        threshold: 90.0,
        for_secs: 0,
        severity: Severity::Critical,
    },
    BuiltinRule {
        id: "disk-warning",
        name: "Root disk 80% full",
        metric: disk::KEY_ROOT_USED_PCT,
        condition: Condition::Above,
        threshold: 80.0,
        for_secs: 0,
        severity: Severity::Warning,
    },
    BuiltinRule {
        id: "services-failed",
        name: "Failed services",
        metric: services::KEY_FAILED,
        condition: Condition::Above,
        threshold: 0.5,
        for_secs: 60,
        severity: Severity::Warning,
    },
    BuiltinRule {
        id: "log-errors",
        name: "Errors in the journal",
        metric: logs::KEY_ERRORS_PER_MIN,
        condition: Condition::Above,
        threshold: 30.0,
        for_secs: 60,
        severity: Severity::Warning,
    },
    BuiltinRule {
        id: "attack",
        name: "Signs of DDoS",
        metric: anomalies::KEY_ATTACK,
        condition: Condition::Above,
        threshold: 0.5,
        for_secs: 0,
        severity: Severity::Critical,
    },
    BuiltinRule {
        id: "brute-force",
        name: "SSH brute force",
        metric: security::KEY_BRUTE_FORCE,
        condition: Condition::Above,
        threshold: 0.5,
        for_secs: 0,
        severity: Severity::Critical,
    },
    BuiltinRule {
        id: "reboot-required",
        name: "Reboot required",
        metric: updates::KEY_REBOOT_REQUIRED,
        condition: Condition::Above,
        threshold: 0.5,
        for_secs: 0,
        severity: Severity::Info,
    },
];

pub fn builtin_rules() -> Vec<AlertRule> {
    BUILTIN
        .iter()
        .map(|rule| AlertRule {
            id: rule.id.to_owned(),
            name: rule.name.to_owned(),
            metric: rule.metric.to_owned(),
            condition: rule.condition,
            threshold: rule.threshold,
            for_secs: rule.for_secs,
            severity: rule.severity,
            enabled: true,
            builtin: true,
        })
        .collect()
}

pub fn merge_rules(custom: &[AlertRule]) -> Vec<AlertRule> {
    let mut rules = builtin_rules();
    for rule in custom {
        rules.retain(|r| r.id != rule.id);
        rules.push(AlertRule {
            builtin: false,
            ..rule.clone()
        });
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rule_ids_are_unique() {
        let rules = builtin_rules();
        let mut ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), rules.len());
    }

    #[test]
    fn custom_rule_with_builtin_id_overrides_it() {
        let custom = AlertRule {
            id: "cpu-high".to_owned(),
            name: "CPU 50".to_owned(),
            metric: cpu::KEY_TOTAL.to_owned(),
            condition: Condition::Above,
            threshold: 50.0,
            for_secs: 10,
            severity: Severity::Critical,
            enabled: true,
            builtin: true,
        };
        let merged = merge_rules(&[custom]);
        let cpu = merged.iter().find(|r| r.id == "cpu-high").expect("rule");
        assert_eq!(cpu.threshold, 50.0);
        assert!(!cpu.builtin);
        assert_eq!(merged.iter().filter(|r| r.id == "cpu-high").count(), 1);
    }
}
