use sib_docgen::SectionId;

use super::{FORMAT_FLEET, Playbook};

pub const FLEET_SECTIONS: [SectionId; 4] = [
    SectionId::Findings,
    SectionId::Description,
    SectionId::System,
    SectionId::Alerts,
];

const FLEET: Playbook = Playbook {
    sections: &FLEET_SECTIONS,
    task: "Below is the data for every server (each server is a separate block starting with \"# Server:\"). \
Give a summary of the whole fleet for an administrator who has just opened the program: where things \
are fine, where there are problems, what to do first. Active incidents and critical findings always \
come before the rest. Do not list everything, only what needs attention, and sum up the rest in one \
sentence.",
    format: FORMAT_FLEET,
};

pub fn fleet_audit() -> Playbook {
    FLEET
}
