use asiba_docgen::SectionId;

use super::{FORMAT_FLEET, Playbook};

pub const FLEET_SECTIONS: [SectionId; 4] = [
    SectionId::Findings,
    SectionId::Description,
    SectionId::System,
    SectionId::Alerts,
];

const FLEET: Playbook = Playbook {
    sections: &FLEET_SECTIONS,
    task: "Ниже данные по всем серверам (каждый сервер — отдельный блок, начинающийся с «# Server:»). Дай \
сводку по всему парку для администратора, который только что открыл программу: где всё хорошо, где \
проблемы, что делать первым. Активные инциденты и критичные находки всегда важнее остального. Не \
перечисляй всё подряд — только то, что требует внимания, остальное одной фразой.",
    format: FORMAT_FLEET,
};

pub fn fleet_audit() -> Playbook {
    FLEET
}
