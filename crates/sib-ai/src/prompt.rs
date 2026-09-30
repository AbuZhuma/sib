use sib_core::Incident;

use crate::playbook::Playbook;

pub const SYSTEM_PROMPT: &str = "Ты — старший инженер по эксплуатации и безопасности Linux-серверов. \
Тебе дают структурированный дамп данных, собранный программой мониторинга Sib, и задачу. Правила: \
опирайся только на данные из дампа, не выдумывай числа, имена, строки логов и команды, которых там нет; \
если данных не хватает — скажи, чего именно, и как их получить; давай конкретные команды и фрагменты \
конфигурации под дистрибутив из раздела System (dnf/apt, systemctl, firewalld/ufw/nftables — то, что \
есть на сервере); у каждого вывода указывай, на какой строке данных он основан; отличай причину от \
следствия; не давай общих советов вроде «обновите систему», если данные не показывают проблему; будь \
кратким — это текст для экрана, не статья. Отвечай по-русски, названия команд, путей, юнитов, пакетов \
и метрик оставляй как есть. Уровни: критично — активный вред или открытая уязвимость; рекомендуется — \
исправить в ближайшие дни; замечание — мелочь.";

pub fn build_user(context: &str, playbook: &Playbook, incident: Option<&Incident>) -> String {
    let mut out = String::new();
    if let Some(incident) = incident {
        out.push_str("## Инцидент\n\n");
        out.push_str(&format!("kind: {}\n", incident.kind.key()));
        out.push_str(&format!("severity: {:?}\n", incident.severity).to_lowercase());
        out.push_str(&format!("subject: {}\n", incident.subject));
        out.push_str(&format!("summary: {}\n", incident.summary));
        out.push_str(&format!(
            "started_at: {}\n",
            incident.started_at.format("%Y-%m-%dT%H:%M:%SZ")
        ));
        for line in &incident.evidence {
            out.push_str(&format!("evidence: {line}\n"));
        }
        out.push('\n');
    }
    out.push_str("## Данные\n\n");
    out.push_str(context);
    out.push_str("\n\n## Задача\n\n");
    out.push_str(&playbook.task.replace(
        "{subject}",
        incident.map(|i| i.subject.as_str()).unwrap_or("server"),
    ));
    out.push_str("\n\n");
    out.push_str(playbook.format);
    out
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sib_core::{IncidentDraft, IncidentKind, ServerId, Severity};

    use super::*;
    use crate::playbook;

    #[test]
    fn build_user_substitutes_subject_and_lists_evidence() {
        let draft = IncidentDraft::new(
            IncidentKind::ContainerDown,
            Severity::Critical,
            "web",
            "container web is exited",
        )
        .evidence(["exit code 137".to_owned()]);
        let incident = Incident::open(1, ServerId::parse("neo").expect("id"), draft, Utc::now());
        let text = build_user(
            "data",
            &playbook::playbook(IncidentKind::ContainerDown),
            Some(&incident),
        );
        assert!(text.contains("kind: container_down"));
        assert!(text.contains("severity: critical"));
        assert!(text.contains("evidence: exit code 137"));
        assert!(text.contains("Контейнер «web»"));
        assert!(text.ends_with(playbook::FORMAT_INCIDENT));
    }
}
