#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "integration test helpers build fixtures and may fail loudly"
)]

use chrono::Utc;
use sib_core::{
    AppState, AuthMethod, Availability, IncidentKind, ModuleId, ModuleState, ServerDescription,
    ServerId, ServerSpec, ServerState, Severity, Snapshot, SudoMode,
};
use sib_incidents::{Outcome, detect, system_audit};
use sib_modules::disk::{self, DiskSnapshot, Filesystem};
use sib_modules::security::{
    self, FirewallState, Hardening, SecuritySnapshot, SshdSettings, Switch,
};
use sib_modules::services::{self, ServicesSnapshot, Unit};
use sib_modules::system::{self, LoadAverage, SystemInfo};
use sib_modules::updates::{self, PackageManager, UpdatesSnapshot};

fn server() -> ServerState {
    ServerState::new(ServerSpec {
        id: ServerId::parse("neo").expect("id"),
        host: "h".into(),
        port: 22,
        user: "root".into(),
        auth: AuthMethod::Auto,
        jump: None,
        sudo: SudoMode::None,
        description: ServerDescription::default(),
        location: None,
        modules: Default::default(),
        checks: Vec::new(),
        check_overrides: Default::default(),
    })
}

fn with_snapshot<T: sib_core::ModuleData>(
    mut server: ServerState,
    id: ModuleId,
    data: T,
) -> ServerState {
    let mut state = ModuleState::detected(Availability::Available);
    state.record_snapshot(Snapshot::new(data));
    server.modules.insert(id, state);
    server
}

fn filesystem(mount: &str, used: u64, available: u64) -> Filesystem {
    Filesystem {
        device: "/dev/sda1".into(),
        mount: mount.into(),
        total_bytes: used + available,
        used_bytes: used,
        available_bytes: available,
        inodes_total: 100,
        inodes_used: 10,
    }
}

fn unit(name: &str, active: &str) -> Unit {
    Unit {
        name: name.into(),
        load: "loaded".into(),
        active: active.into(),
        sub: active.into(),
        description: name.into(),
        restarts: 0,
        main_pid: 0,
        active_since: None,
        fragment_path: "/etc/systemd/system/x.service".into(),
        working_directory: String::new(),
        result: "exit-code".into(),
    }
}

fn security_snapshot(password_auth: Switch, firewall: FirewallState) -> SecuritySnapshot {
    SecuritySnapshot {
        attackers: Vec::new(),
        failed_logins: 0,
        logins: Vec::new(),
        sudo_calls: Vec::new(),
        jails: Vec::new(),
        has_fail2ban: false,
        sshd: SshdSettings {
            password_auth: Some(password_auth),
            ..SshdSettings::default()
        },
        firewall,
        file_hashes: Vec::new(),
        bans: Vec::new(),
        ban_backend: None,
        is_root_view: true,
        hardening: Hardening::default(),
        slow_collected_at: Utc::now(),
    }
}

fn outcome_of(server: &ServerState, id: &str) -> Outcome {
    let audit = system_audit(server, &AppState::default());
    audit
        .checks
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.outcome)
        .unwrap_or_else(|| panic!("pattern {id} missing"))
}

#[test]
fn disk_detector_grades_by_usage() {
    let disks = DiskSnapshot {
        filesystems: vec![
            filesystem("/", 95, 5),
            filesystem("/data", 85, 15),
            filesystem("/tmp", 10, 90),
        ],
        devices: Vec::new(),
        pressure: None,
    };
    let server = with_snapshot(server(), disk::ID, disks);
    let drafts = detect(&server, &AppState::default());
    let disk_drafts: Vec<_> = drafts
        .iter()
        .filter(|d| d.kind == IncidentKind::DiskFull)
        .collect();
    assert_eq!(disk_drafts.len(), 2);
    assert_eq!(disk_drafts[0].severity, Severity::Critical);
    assert_eq!(disk_drafts[0].subject, "/");
    assert_eq!(disk_drafts[1].severity, Severity::Warning);
    assert_eq!(outcome_of(&server, "resources.disk_space"), Outcome::Fail);
}

#[test]
fn failed_unit_is_critical_incident_and_reliability_failure() {
    let services = ServicesSnapshot {
        units: vec![unit("app.service", "failed"), unit("ok.service", "active")],
        timers: Vec::new(),
    };
    let server = with_snapshot(server(), services::ID, services);
    let drafts = detect(&server, &AppState::default());
    let failed = drafts
        .iter()
        .find(|d| d.kind == IncidentKind::UnitFailed)
        .expect("unit incident");
    assert_eq!(failed.severity, Severity::Critical);
    assert_eq!(failed.subject, "app.service");
    assert_eq!(
        outcome_of(&server, "reliability.failed_units"),
        Outcome::Fail
    );
}

#[test]
fn security_updates_and_reboot_are_reported() {
    let pending = UpdatesSnapshot {
        manager: PackageManager::Apt,
        pending: 12,
        security: 3,
        reboot_required: true,
        packages: Vec::new(),
    };
    let server = with_snapshot(server(), updates::ID, pending);
    let drafts = detect(&server, &AppState::default());
    let kinds: Vec<(Severity, &str)> = drafts
        .iter()
        .filter(|d| d.kind == IncidentKind::Updates)
        .map(|d| (d.severity, d.subject.as_str()))
        .collect();
    assert!(kinds.contains(&(Severity::Warning, "security")));
    assert!(kinds.contains(&(Severity::Info, "reboot")));
    assert_eq!(outcome_of(&server, "updates.security"), Outcome::Fail);
}

#[test]
fn clock_offset_over_a_minute_is_incident() {
    let info = SystemInfo {
        hostname: "neo".into(),
        os_name: "Fedora".into(),
        os_id: "fedora".into(),
        kernel: "6.1".into(),
        arch: "x86_64".into(),
        uptime: std::time::Duration::from_secs(3600),
        load: LoadAverage {
            one: 0.1,
            five: 0.1,
            fifteen: 0.1,
        },
        cpu_model: "cpu".into(),
        cpu_cores: 2,
        mem_total_bytes: 1,
        swap_total_bytes: 0,
        virtualization: None,
        timezone: None,
        clock_offset_secs: 120,
    };
    let server = with_snapshot(server(), system::ID, info);
    let drafts = detect(&server, &AppState::default());
    assert!(drafts.iter().any(|d| d.kind == IncidentKind::Clock));
}

#[test]
fn ssh_and_firewall_patterns_follow_security_snapshot() {
    let weak = with_snapshot(
        server(),
        security::ID,
        security_snapshot(Switch::On, FirewallState::Inactive),
    );
    assert_eq!(outcome_of(&weak, "ssh.password_auth"), Outcome::Fail);
    assert_eq!(outcome_of(&weak, "firewall.active"), Outcome::Fail);
    let strong = with_snapshot(
        server(),
        security::ID,
        security_snapshot(Switch::Off, FirewallState::Active("nftables".into())),
    );
    assert_eq!(outcome_of(&strong, "ssh.password_auth"), Outcome::Pass);
    assert_eq!(outcome_of(&strong, "firewall.active"), Outcome::Pass);
    let drafts = detect(&weak, &AppState::default());
    assert!(
        drafts
            .iter()
            .any(|d| d.kind == IncidentKind::SecurityCheck && d.subject == "ssh.password_auth")
    );
}

#[test]
fn server_without_data_has_no_incidents() {
    let drafts = detect(&server(), &AppState::default());
    assert!(drafts.is_empty());
}
