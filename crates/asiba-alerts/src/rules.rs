use asiba_core::{AlertRule, Condition, METRIC_OFFLINE, Severity};
use asiba_modules::{anomalies, cpu, disk, logs, memory, security, services, updates};

fn rule(
    id: &str,
    name: &str,
    metric: &str,
    condition: Condition,
    threshold: f64,
    for_secs: u64,
    severity: Severity,
) -> AlertRule {
    AlertRule {
        id: id.to_owned(),
        name: name.to_owned(),
        metric: metric.to_owned(),
        condition,
        threshold,
        for_secs,
        severity,
        builtin: true,
    }
}

pub fn builtin_rules() -> Vec<AlertRule> {
    use Condition::Above;
    use Severity::{Critical, Warning};
    vec![
        rule(
            "offline",
            "Сервер недоступен",
            METRIC_OFFLINE,
            Above,
            0.5,
            60,
            Critical,
        ),
        rule(
            "cpu-high",
            "CPU выше 90%",
            cpu::KEY_TOTAL,
            Above,
            90.0,
            300,
            Warning,
        ),
        rule(
            "memory-high",
            "Память выше 90%",
            memory::KEY_USED_PCT,
            Above,
            90.0,
            120,
            Warning,
        ),
        rule(
            "disk-full",
            "Корневой диск заполнен на 90%",
            disk::KEY_ROOT_USED_PCT,
            Above,
            90.0,
            0,
            Critical,
        ),
        rule(
            "disk-warning",
            "Корневой диск заполнен на 80%",
            disk::KEY_ROOT_USED_PCT,
            Above,
            80.0,
            0,
            Warning,
        ),
        rule(
            "services-failed",
            "Есть упавшие сервисы",
            services::KEY_FAILED,
            Above,
            0.5,
            60,
            Warning,
        ),
        rule(
            "log-errors",
            "Много ошибок в журнале",
            logs::KEY_ERRORS_PER_MIN,
            Above,
            30.0,
            60,
            Warning,
        ),
        rule(
            "attack",
            "Признаки DDoS",
            anomalies::KEY_ATTACK,
            Above,
            0.5,
            0,
            Critical,
        ),
        rule(
            "brute-force",
            "Брутфорс SSH",
            security::KEY_ATTACKERS,
            Above,
            20.0,
            0,
            Warning,
        ),
        rule(
            "reboot-required",
            "Требуется перезагрузка",
            updates::KEY_REBOOT_REQUIRED,
            Above,
            0.5,
            0,
            Severity::Info,
        ),
    ]
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
