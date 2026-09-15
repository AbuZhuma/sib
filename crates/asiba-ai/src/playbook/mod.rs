mod fleet;
mod incidents;
mod sections;

use asiba_core::{Incident, IncidentKind, ModuleId, QueryRequest};
use asiba_docgen::SectionId;
use asiba_modules::{docker, services};

pub use fleet::fleet_audit;
pub use sections::section_audit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Playbook {
    pub sections: &'static [SectionId],
    pub task: &'static str,
    pub format: &'static str,
}

pub const FORMAT_AUDIT: &str = "Формат ответа (Markdown, только эти заголовки, пустые разделы пропускай):\n\
## Итог\nДва-три предложения: общее состояние и главный риск.\n\
## Критично\nЧто требует действий сейчас. Для каждого пункта: факт (строка данных), риск, точная команда или изменение конфигурации.\n\
## Рекомендуется\nЧто исправить в ближайшее время, в том же формате.\n\
## Замечания\nМелочи и наблюдения одной строкой каждое.";

pub const FORMAT_INCIDENT: &str = "Формат ответа (Markdown, только эти заголовки):\n\
## Причина\nОдна-две фразы: наиболее вероятная причина и уверенность (высокая/средняя/низкая).\n\
## Доказательства\nСтроки данных или логов, на которых основан вывод (цитируй дословно, до 8 строк).\n\
## Что сделать сейчас\nПошагово: команды диагностики и исправления в порядке выполнения.\n\
## Как предотвратить\nНастройки, мониторинг, лимиты — коротко.";

pub const FORMAT_SECTION: &str = "Формат ответа (Markdown, только эти заголовки, коротко — это блок на экране рядом с данными):\n\
## Состояние\nОдно-два предложения по существу.\n\
## Что настораживает\nСписок фактов из данных с пояснением, почему это важно; если ничего — одна строка «ничего существенного».\n\
## Рекомендации\nДо пяти конкретных действий с командами; если всё в порядке — что стоит держать под контролем.";

pub const FORMAT_FLEET: &str = "Формат ответа (Markdown, только эти заголовки):\n\
## Общее состояние\nДва-три предложения по всему парку серверов.\n\
## Требуют внимания\nСерверы в порядке срочности: имя — что не так — что сделать первым. Критичные инциденты всегда выше остальных.\n\
## Первые шаги\nТри самых полезных действия на сегодня.";

const FULL: Playbook = Playbook {
    sections: &SectionId::ALL,
    task: "Проведи полный операционный аудит и аудит безопасности этого сервера. Начни с раздела Findings: \
это уже найденные программой проблемы, их нужно объяснить и приоритизировать, а не повторять. Затем ищи то, \
чего программа не видит: несоответствия между разделами (например, открытый порт без процесса, контейнер \
с рестартами при свободных ресурсах, рост ошибок в журнале), устаревшие или рискованные настройки, \
признаки утечек и деградации по сводкам за час и сутки. Если сервер в порядке, скажи это коротко и \
не выдумывай проблем.",
    format: FORMAT_AUDIT,
};

pub fn full_audit() -> Playbook {
    FULL
}

pub fn playbook(kind: IncidentKind) -> Playbook {
    incidents::for_kind(kind)
}

pub fn queries(incident: &Incident) -> Vec<(ModuleId, QueryRequest)> {
    match incident.kind {
        IncidentKind::ContainerDown => vec![(
            docker::ID,
            QueryRequest::new(docker::QUERY_LOGS, incident.subject.clone()),
        )],
        IncidentKind::UnitFailed => vec![(
            services::ID,
            QueryRequest::new(services::QUERY_JOURNAL, incident.subject.clone()),
        )],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_incident_playbook_starts_with_findings() {
        let kinds = [
            IncidentKind::ContainerDown,
            IncidentKind::UnitFailed,
            IncidentKind::DeployFailed,
            IncidentKind::Anomaly,
            IncidentKind::SecurityCheck,
            IncidentKind::DiskFull,
        ];
        for kind in kinds {
            assert_eq!(playbook(kind).sections[0], SectionId::Findings, "{kind:?}");
        }
        assert_eq!(full_audit().sections[0], SectionId::Findings);
    }

    #[test]
    fn section_audit_exists_for_every_visible_section() {
        for key in [
            "processes",
            "resources",
            "ports",
            "docker",
            "services",
            "projects",
            "logs",
            "users",
            "security",
            "anomalies",
            "deploy",
            "gpu",
        ] {
            assert!(section_audit(key).is_some(), "{key}");
        }
        assert!(section_audit("nonexistent").is_none());
    }
}
