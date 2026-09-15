use asiba_core::IncidentKind;
use asiba_docgen::SectionId;

use super::{FORMAT_INCIDENT, Playbook};

const CONTAINER: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Docker,
        SectionId::Projects,
        SectionId::Resources,
        SectionId::Deploy,
        SectionId::Events,
    ],
    task: "Контейнер «{subject}» остановлен, падает или unhealthy. По его логам (раздел с логами \
контейнера, если он есть), состоянию Docker и проекта определи причину: падение приложения, ошибка \
конфигурации или переменных окружения, недоступная зависимость (БД, сеть, том), нехватка ресурсов, \
проблема образа. Отличай причину от следствия: если в логах есть первая ошибка перед каскадом — \
она главная.",
    format: FORMAT_INCIDENT,
};

const UNIT: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Services,
        SectionId::Logs,
        SectionId::Resources,
        SectionId::Projects,
        SectionId::Events,
    ],
    task: "Юнит systemd «{subject}» в состоянии failed. По его журналу (раздел с journal, если он есть), \
списку сервисов и системному журналу определи причину: ошибка запуска (ExecStart, права, отсутствующий \
файл или порт занят), падение процесса, превышение лимитов, зависимость. Учитывай счётчик рестартов \
и результат (result).",
    format: FORMAT_INCIDENT,
};

const DEPLOY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Deploy,
        SectionId::Projects,
        SectionId::Docker,
        SectionId::Services,
        SectionId::Logs,
    ],
    task: "Деплой «{subject}» завершился ошибкой. По хвосту лога деплоя, стадиям и состоянию контейнеров \
и сервисов определи, на каком шаге и почему он упал (сборка, миграции, запуск, health-check, \
недоступный реестр или репозиторий), и что проверить перед повторным запуском.",
    format: FORMAT_INCIDENT,
};

const ATTACK: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Anomalies,
        SectionId::Security,
        SectionId::Ports,
        SectionId::Resources,
        SectionId::Events,
    ],
    task: "Обнаружен сетевой признак атаки или брутфорс: «{subject}». По счётчикам, списку адресов и \
состоянию защиты реши, это атака (DDoS, сканирование, подбор пароля) или легитимная нагрузка, и \
объясни на чём основан вывод. Дай конкретные меры: какие IP или диапазоны блокировать и чем именно \
на этом сервере (fail2ban / nftables / iptables / ufw / firewalld — выбирай то, что там есть), \
какие sysctl и настройки sshd применить, за чем следить дальше.",
    format: FORMAT_INCIDENT,
};

const SECURITY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Security,
        SectionId::Users,
        SectionId::Ports,
        SectionId::Updates,
    ],
    task: "Провалена проверка безопасности: «{subject}». Объясни риск именно для этого сервера (учитывая \
открытые порты, пользователей, окружение) и дай точное изменение конфигурации или команду для \
исправления и проверку, что исправление применилось.",
    format: FORMAT_INCIDENT,
};

const RESOURCES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Resources,
        SectionId::Processes,
        SectionId::Docker,
        SectionId::Services,
        SectionId::Logs,
    ],
    task: "Проблема с ресурсами: «{subject}». По сводкам за час и сутки, топу процессов, контейнерам и \
сервисам определи, кто потребляет ресурс и характер проблемы: утечка (монотонный рост), всплеск \
(пик и возврат) или нормальный рост. Дай действия для снятия проблемы сейчас (что перезапустить, \
что почистить, что ограничить) и меры на будущее (лимиты, ротация, алерты).",
    format: FORMAT_INCIDENT,
};

pub fn for_kind(kind: IncidentKind) -> Playbook {
    match kind {
        IncidentKind::ContainerDown => CONTAINER,
        IncidentKind::UnitFailed => UNIT,
        IncidentKind::DeployFailed => DEPLOY,
        IncidentKind::Anomaly | IncidentKind::BruteForce => ATTACK,
        IncidentKind::SecurityCheck | IncidentKind::Updates => SECURITY,
        IncidentKind::Alert
        | IncidentKind::DiskFull
        | IncidentKind::Memory
        | IncidentKind::ModuleError
        | IncidentKind::Clock => RESOURCES,
    }
}
