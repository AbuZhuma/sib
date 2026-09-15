use super::{Category, Check, CheckStatus, Weight};
use crate::security::model::{SecuritySnapshot, Switch};

const SUDO_HINT: &str = "нужен sudo для sshd -T";
const MAX_AUTH_TRIES: u32 = 6;

pub fn all(snapshot: &SecuritySnapshot) -> Vec<Check> {
    vec![
        password_auth(snapshot),
        root_login(snapshot),
        pubkey(snapshot),
        empty_passwords(snapshot),
        max_tries(snapshot),
        x11(snapshot),
        port(snapshot),
        brute_force(snapshot),
        fail2ban(snapshot),
    ]
}

fn switch_check(
    check: Check,
    value: Option<Switch>,
    wanted: Switch,
    option: &str,
    bad_status: CheckStatus,
) -> Check {
    match value {
        Some(v) if v == wanted => check.with(CheckStatus::Pass, format!("{option} ok")),
        Some(Switch::On) => check.with(bad_status, format!("{option} yes")),
        Some(Switch::Off) => check.with(bad_status, format!("{option} no")),
        _ => check.with(CheckStatus::Unknown, SUDO_HINT),
    }
}

fn password_auth(snapshot: &SecuritySnapshot) -> Check {
    switch_check(
        Check::new(Category::Ssh, "Вход по паролю отключён", Weight::High),
        snapshot.sshd.password_auth,
        Switch::Off,
        "PasswordAuthentication",
        CheckStatus::Fail,
    )
}

fn pubkey(snapshot: &SecuritySnapshot) -> Check {
    switch_check(
        Check::new(Category::Ssh, "Вход по ключу включён", Weight::Medium),
        snapshot.sshd.pubkey_auth,
        Switch::On,
        "PubkeyAuthentication",
        CheckStatus::Fail,
    )
}

fn empty_passwords(snapshot: &SecuritySnapshot) -> Check {
    switch_check(
        Check::new(Category::Ssh, "Пустые пароли запрещены", Weight::High),
        snapshot.sshd.permit_empty_passwords,
        Switch::Off,
        "PermitEmptyPasswords",
        CheckStatus::Fail,
    )
}

fn x11(snapshot: &SecuritySnapshot) -> Check {
    switch_check(
        Check::new(Category::Ssh, "X11 forwarding выключен", Weight::Low),
        snapshot.sshd.x11_forwarding,
        Switch::Off,
        "X11Forwarding",
        CheckStatus::Warn,
    )
}

fn root_login(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Ssh, "Вход root по SSH запрещён", Weight::High);
    match snapshot.sshd.permit_root_login.as_deref() {
        Some("yes") => check.with(CheckStatus::Fail, "PermitRootLogin yes"),
        Some("no") => check.with(CheckStatus::Pass, "PermitRootLogin no"),
        Some(value) => check.with(CheckStatus::Warn, format!("PermitRootLogin {value}")),
        None => check.with(CheckStatus::Unknown, SUDO_HINT),
    }
}

fn max_tries(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Ssh, "Ограничено число попыток входа", Weight::Low);
    match snapshot.sshd.max_auth_tries {
        Some(tries) if tries <= MAX_AUTH_TRIES => {
            check.with(CheckStatus::Pass, format!("MaxAuthTries {tries}"))
        }
        Some(tries) => check.with(CheckStatus::Warn, format!("MaxAuthTries {tries}")),
        None => check.with(CheckStatus::Unknown, SUDO_HINT),
    }
}

fn port(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Ssh, "SSH не на порту 22", Weight::Low);
    match snapshot.sshd.port {
        Some(22) => check.with(CheckStatus::Warn, "порт 22 — основная цель сканеров"),
        Some(port) => check.with(CheckStatus::Pass, format!("порт {port}")),
        None => check.with(CheckStatus::Unknown, String::new()),
    }
}

fn brute_force(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Network, "Нет активного брутфорса", Weight::Medium);
    let active = snapshot.brute_force_count();
    if active == 0 {
        return check.with(
            CheckStatus::Pass,
            format!("{} неудачных попыток за 24 ч", snapshot.failed_logins),
        );
    }
    check.with(
        CheckStatus::Warn,
        format!("{active} IP атакуют прямо сейчас"),
    )
}

fn fail2ban(snapshot: &SecuritySnapshot) -> Check {
    let check = Check::new(Category::Network, "fail2ban защищает SSH", Weight::Medium);
    if snapshot.has_fail2ban {
        let jails: Vec<&str> = snapshot.jails.iter().map(|j| j.name.as_str()).collect();
        return check.with(CheckStatus::Pass, jails.join(", "));
    }
    check.with(CheckStatus::Warn, "не установлен или не запущен")
}
