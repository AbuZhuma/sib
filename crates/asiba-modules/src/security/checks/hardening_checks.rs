use super::{Category, Check, CheckStatus, Weight};
use crate::security::model::{FirewallState, MacStatus, SecuritySnapshot};

const ROOT_HINT: &str = "нужен sudo";

pub fn all(snapshot: &SecuritySnapshot) -> Vec<Check> {
    let h = &snapshot.hardening;
    vec![
        firewall(snapshot),
        risky_ports(snapshot),
        extra_uid0(snapshot),
        empty_passwords(snapshot),
        sudo_nopasswd(snapshot),
        writable_keys(snapshot),
        world_writable(snapshot),
        sysctl(
            h,
            "net.ipv4.tcp_syncookies",
            "1",
            "SYN cookies включены",
            Weight::Medium,
        ),
        sysctl(
            h,
            "net.ipv4.conf.all.rp_filter",
            "1",
            "Фильтр обратного пути (rp_filter)",
            Weight::Low,
        ),
        sysctl(
            h,
            "kernel.randomize_va_space",
            "2",
            "ASLR включён полностью",
            Weight::Medium,
        ),
        sysctl(
            h,
            "net.ipv4.conf.all.accept_redirects",
            "0",
            "ICMP-редиректы не принимаются",
            Weight::Low,
        ),
        sysctl(
            h,
            "net.ipv4.conf.all.send_redirects",
            "0",
            "ICMP-редиректы не отправляются",
            Weight::Low,
        ),
        sysctl(
            h,
            "net.ipv4.icmp_echo_ignore_broadcasts",
            "1",
            "Broadcast-ping игнорируется",
            Weight::Low,
        ),
        mac(snapshot),
        ntp(snapshot),
        auto_updates(snapshot),
        auditd(snapshot),
    ]
}

fn firewall(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Network, "Файрвол включён", Weight::High);
    match &snapshot.firewall {
        FirewallState::Active(name) => check.with(CheckStatus::Pass, name.clone()),
        FirewallState::Inactive => {
            check.with(CheckStatus::Fail, "ни один сервис файрвола не активен")
        }
        FirewallState::Unknown => check.with(CheckStatus::Unknown, "нет systemctl"),
    }
}

fn risky_ports(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(
        Category::Network,
        "Нет опасных открытых портов",
        Weight::High,
    );
    let ports = &snapshot.hardening.risky_ports;
    if ports.is_empty() {
        return check.with(
            CheckStatus::Pass,
            "telnet, ftp, rsh, docker api, redis, mongo, elastic — закрыты",
        );
    }
    let list: Vec<String> = ports.iter().map(u16::to_string).collect();
    check.with(CheckStatus::Fail, format!("слушают: {}", list.join(", ")))
}

fn extra_uid0(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Access, "Только root имеет UID 0", Weight::High);
    let extra = &snapshot.hardening.extra_uid0;
    if extra.is_empty() {
        return check.with(CheckStatus::Pass, "");
    }
    check.with(CheckStatus::Fail, extra.join(", "))
}

fn empty_passwords(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(
        Category::Access,
        "Нет учёток с пустым паролем",
        Weight::High,
    );
    match &snapshot.hardening.empty_passwords {
        Some(users) if users.is_empty() => check.with(CheckStatus::Pass, ""),
        Some(users) => check.with(CheckStatus::Fail, users.join(", ")),
        None => check.with(CheckStatus::Unknown, ROOT_HINT),
    }
}

fn sudo_nopasswd(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Access, "sudo требует пароль", Weight::Low);
    match snapshot.hardening.sudo_nopasswd {
        Some(0) => check.with(CheckStatus::Pass, ""),
        Some(count) => check.with(CheckStatus::Warn, format!("{count} правил NOPASSWD")),
        None => check.with(CheckStatus::Unknown, ROOT_HINT),
    }
}

fn writable_keys(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(
        Category::Access,
        "authorized_keys защищены от записи",
        Weight::High,
    );
    let files = &snapshot.hardening.writable_keys;
    if files.is_empty() {
        return check.with(CheckStatus::Pass, "");
    }
    check.with(CheckStatus::Fail, files.join(", "))
}

fn world_writable(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(
        Category::Access,
        "В /etc нет файлов с записью для всех",
        Weight::Medium,
    );
    let files = &snapshot.hardening.world_writable_etc;
    if files.is_empty() {
        return check.with(CheckStatus::Pass, "");
    }
    check.with(CheckStatus::Fail, files.join(", "))
}

fn sysctl(
    hardening: &crate::security::model::Hardening,
    key: &str,
    wanted: &str,
    label: &'static str,
    weight: Weight,
) -> Check {
    let check = Check::new(Category::Kernel, label, weight);
    match hardening.sysctl(key) {
        Some(value) if value == wanted => check.with(CheckStatus::Pass, format!("{key} = {value}")),
        Some(value) => check.with(
            CheckStatus::Warn,
            format!("{key} = {value}, ожидается {wanted}"),
        ),
        None => check.with(CheckStatus::Unknown, ""),
    }
}

fn mac(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(
        Category::System,
        "SELinux или AppArmor активен",
        Weight::Medium,
    );
    match &snapshot.hardening.mac {
        Some(MacStatus::SelinuxEnforcing) => check.with(CheckStatus::Pass, "SELinux enforcing"),
        Some(MacStatus::AppArmor) => check.with(CheckStatus::Pass, "AppArmor"),
        Some(MacStatus::SelinuxPermissive) => check.with(CheckStatus::Warn, "SELinux permissive"),
        Some(MacStatus::Disabled) => check.with(CheckStatus::Fail, "SELinux disabled"),
        Some(MacStatus::Unknown) | None => check.with(CheckStatus::Unknown, "не обнаружен"),
    }
}

fn ntp(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(
        Category::System,
        "Время синхронизировано (NTP)",
        Weight::Low,
    );
    match snapshot.hardening.ntp_synced {
        Some(true) => check.with(CheckStatus::Pass, ""),
        Some(false) => check.with(CheckStatus::Warn, "NTPSynchronized=no"),
        None => check.with(CheckStatus::Unknown, ""),
    }
}

fn auto_updates(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::System, "Автообновления безопасности", Weight::Low);
    if snapshot.hardening.auto_updates {
        return check.with(CheckStatus::Pass, "unattended-upgrades / dnf-automatic");
    }
    check.with(CheckStatus::Warn, "не настроены")
}

fn auditd(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::System, "auditd ведёт аудит", Weight::Low);
    if snapshot.hardening.auditd {
        return check.with(CheckStatus::Pass, "");
    }
    check.with(CheckStatus::Warn, "не запущен")
}
