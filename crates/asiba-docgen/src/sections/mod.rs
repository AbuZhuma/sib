mod actions;
mod alerts;
mod anomalies;
mod deploy;
mod description;
mod docker;
mod events;
mod findings;
mod gpu;
mod logs;
mod ports;
mod processes;
mod projects;
mod resources;
mod security;
mod services;
mod system;
mod updates;
mod users;

use crate::section::{Section, SectionId};

pub fn by_id(id: SectionId) -> Option<Box<dyn Section>> {
    let section: Box<dyn Section> = match id {
        SectionId::Description => Box::new(description::DescriptionSection),
        SectionId::System => Box::new(system::SystemSection),
        SectionId::Resources => Box::new(resources::ResourcesSection),
        SectionId::Processes => Box::new(processes::ProcessesSection),
        SectionId::Projects => Box::new(projects::ProjectsSection),
        SectionId::Services => Box::new(services::ServicesSection),
        SectionId::Docker => Box::new(docker::DockerSection),
        SectionId::Ports => Box::new(ports::PortsSection),
        SectionId::Deploy => Box::new(deploy::DeploySection),
        SectionId::Users => Box::new(users::UsersSection),
        SectionId::Updates => Box::new(updates::UpdatesSection),
        SectionId::Logs => Box::new(logs::LogsSection),
        SectionId::Security => Box::new(security::SecuritySection),
        SectionId::Anomalies => Box::new(anomalies::AnomaliesSection),
        SectionId::Gpu => Box::new(gpu::GpuSection),
        SectionId::Alerts => Box::new(alerts::AlertsSection),
        SectionId::Actions => Box::new(actions::ActionsSection),
        SectionId::Events => Box::new(events::EventsSection),
        SectionId::Findings => Box::new(findings::FindingsSection),
    };
    Some(section)
}
