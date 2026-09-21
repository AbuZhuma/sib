use asiba_core::{AlertRule, Condition, METRIC_OFFLINE, Severity};
use asiba_modules::{anomalies, cpu, disk, logs, memory, security, services, updates};

struct BuiltinRule {
    id: &'static str,
    name: &'static str,
    metric: &'static str,
    condition: Condition,
    threshold: f64,
    for_secs: u64,
    severity: Severity,
}

const fn builtin(
    id: &'static str,
    name: &'static str,
    metric: &'static str,
    condition: Condition,
    threshold: f64,
    for_secs: u64,
    severity: Severity,
) -> BuiltinRule {
    BuiltinRule {
        id,
        name,
        metric,
        condition,
        threshold,
        for_secs,
        severity,
    }
}

const BUILTIN: [BuiltinRule; 10] = {
    use Condition::Above;
    use Severity::{Critical, Info, Warning};
    [
        builtin(
            "offline",
            "Сервер недоступен",
            METRIC_OFFLINE,
            Above,
            0.5,
            60,
            Critical,
        ),
        builtin(
            "cpu-high",
            "CPU выше 90%",
            cpu::KEY_TOTAL,
            Above,
            90.0,
            300,
            Warning,
        ),
        builtin(
            "memory-high",
            "Память выше 90%",
            memory::KEY_USED_PCT,
            Above,
            90.0,
            120,
            Warning,
        ),
        builtin(
            "disk-full",
            "Корневой диск заполнен на 90%",
            disk::KEY_ROOT_USED_PCT,
            Above,
            90.0,
            0,
            Critical,
        ),
        builtin(
            "disk-warning",
            "Корневой диск заполнен на 80%",
            disk::KEY_ROOT_USED_PCT,
            Above,
            80.0,
            0,
            Warning,
        ),
        builtin(
            "services-failed",
            "Есть упавшие сервисы",
            services::KEY_FAILED,
            Above,
            0.5,
            60,
            Warning,
        ),
        builtin(
            "log-errors",
            "Много ошибок в журнале",
            logs::KEY_ERRORS_PER_MIN,
            Above,
            30.0,
            60,
            Warning,
        ),
        builtin(
            "attack",
            "Признаки DDoS",
            anomalies::KEY_ATTACK,
            Above,
            0.5,
            0,
            Critical,
        ),
        builtin(
            "brute-force",
            "Брутфорс SSH",
            security::KEY_BRUTE_FORCE,
            Above,
            0.5,
            0,
            Critical,
        ),
        builtin(
            "reboot-required",
            "Требуется перезагрузка",
            updates::KEY_REBOOT_REQUIRED,
            Above,
            0.5,
            0,
            Info,
        ),
    ]
};

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
            builtin: true,
        };
        let merged = merge_rules(&[custom]);
        let cpu = merged.iter().find(|r| r.id == "cpu-high").expect("rule");
        assert_eq!(cpu.threshold, 50.0);
        assert!(!cpu.builtin);
        assert_eq!(merged.iter().filter(|r| r.id == "cpu-high").count(), 1);
    }
}
