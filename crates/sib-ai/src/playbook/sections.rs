use sib_docgen::SectionId;

use super::{FORMAT_SECTION, Playbook};

const PROCESSES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Processes,
        SectionId::Resources,
    ],
    task: "Проанализируй процессы сервера: кто съедает CPU и память, есть ли зомби, подозрительные или \
лишние процессы (майнеры, неизвестные бинарники из /tmp, дубликаты). Сопоставь с загрузкой из раздела \
Resources.",
    format: FORMAT_SECTION,
};

const RESOURCES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Resources,
        SectionId::Processes,
    ],
    task: "Проанализируй ресурсы сервера по текущим значениям и сводкам за час и сутки: CPU, память, swap, \
диски, inodes, сеть. Ищи тренды (рост занятости диска, постоянный swap, пики CPU), давление памяти, \
ошибки и потери на интерфейсах. Оцени запас на ближайшие дни.",
    format: FORMAT_SECTION,
};

const PORTS: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Ports,
        SectionId::Security,
        SectionId::Docker,
    ],
    task: "Проанализируй слушающие порты и файрвол: какие сервисы открыты наружу, какие из них не должны \
быть публичными (базы данных, панели, отладочные порты), есть ли порты без процесса, что открыто в \
файрволе без необходимости, чего в файрволе не хватает.",
    format: FORMAT_SECTION,
};

const DOCKER: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Docker, SectionId::Resources],
    task: "Проанализируй контейнеры: остановленные с ненулевым кодом, рестарты, unhealthy, контейнеры без \
политики перезапуска у сервисов, которые должны жить постоянно, порты, проброшенные на 0.0.0.0, \
устаревшие образы, мусор (образы, тома). Учитывай compose-проекты.",
    format: FORMAT_SECTION,
};

const SERVICES: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Services, SectionId::Logs],
    task: "Проанализируй сервисы systemd: упавшие, с частыми рестартами, пользовательские юниты и их \
состояние. Сопоставь с ошибками в журнале.",
    format: FORMAT_SECTION,
};

const LOGS: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Logs,
        SectionId::Services,
        SectionId::Resources,
    ],
    task: "Проанализируй системный журнал: сгруппированные повторяющиеся сообщения и последние записи. \
Отдели шум от реальных проблем, объясни самые частые ошибки, укажи, какие из них связаны с \
упавшими сервисами или ресурсами, и что сделать, чтобы они исчезли.",
    format: FORMAT_SECTION,
};

const USERS: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Users, SectionId::Security],
    task: "Проанализируй пользователей: активные сессии и откуда они, последние входы, учётные записи с \
shell и sudo, количество ключей. Ищи лишние учётки с доступом, вход с незнакомых адресов, root-входы, \
учётки без ключей, но с sudo.",
    format: FORMAT_SECTION,
};

const SECURITY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Security,
        SectionId::Ports,
        SectionId::Users,
        SectionId::Updates,
    ],
    task: "Проанализируй безопасность: проваленные и неизвестные проверки (что нужно, чтобы их выполнить), \
атакующие адреса и баны, входы и sudo-вызовы, обновления безопасности. Расставь приоритеты по \
реальному риску для этого сервера, а не по формальному списку.",
    format: FORMAT_SECTION,
};

const ANOMALIES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Anomalies,
        SectionId::Ports,
        SectionId::Security,
    ],
    task: "Проанализируй сетевую картину: состояния соединений, скорости пакетов и новых соединений, \
sync-cookies и сбросы, доля топовых адресов, conntrack. Скажи, есть ли признаки атаки или сканирования, \
и какие адреса выглядят подозрительно.",
    format: FORMAT_SECTION,
};

const DEPLOY: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Deploy, SectionId::Docker],
    task: "Проанализируй деплои: частота, длительность, ошибки, незавершённые, события compose. \
Ищи нестабильные проекты (частые падения), деплои в нерабочее время, зависшие.",
    format: FORMAT_SECTION,
};

const GPU: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Gpu, SectionId::Processes],
    task: "Проанализируй GPU: загрузка, память, температура, мощность, процессы на GPU. Ищи перегрев, \
переполнение памяти, простаивающие или чужие процессы.",
    format: FORMAT_SECTION,
};

pub fn section_audit(key: &str) -> Option<Playbook> {
    let playbook = match key {
        "processes" => PROCESSES,
        "resources" => RESOURCES,
        "ports" => PORTS,
        "docker" => DOCKER,
        "services" => SERVICES,
        "logs" => LOGS,
        "users" => USERS,
        "security" => SECURITY,
        "anomalies" => ANOMALIES,
        "deploy" => DEPLOY,
        "gpu" => GPU,
        _ => return None,
    };
    Some(playbook)
}
