use super::model::{FirewallState, SecuritySnapshot, Switch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub label: &'static str,
    pub detail: String,
    pub status: CheckStatus,
}

pub const CHECK_FIREWALL: &str = "Файрвол включён";
pub const CHECK_FAIL2BAN: &str = "fail2ban защищает SSH";
pub const CHECK_PASSWORD_AUTH: &str = "Вход по паролю отключён";
pub const CHECK_ROOT_LOGIN: &str = "Вход root по SSH запрещён";
pub const CHECK_BRUTE_FORCE: &str = "Нет активного брутфорса";
pub const CHECK_PORT: &str = "SSH не на порту 22";

pub fn checks(snapshot: &SecuritySnapshot) -> Vec<Check> {
    vec![
        firewall(snapshot),
        fail2ban(snapshot),
        password_auth(snapshot),
        root_login(snapshot),
        brute_force(snapshot),
        ssh_port(snapshot),
    ]
}

pub fn score(checks: &[Check]) -> (usize, usize) {
    let known: Vec<&Check> = checks
        .iter()
        .filter(|c| c.status != CheckStatus::Unknown)
        .collect();
    let passed = known
        .iter()
        .filter(|c| c.status == CheckStatus::Pass)
        .count();
    (passed, known.len())
}

fn firewall(snapshot: &SecuritySnapshot) -> Check {
    let (status, detail) = match &snapshot.firewall {
        FirewallState::Active(name) => (CheckStatus::Pass, name.clone()),
        FirewallState::Inactive => (
            CheckStatus::Fail,
            "ни один сервис файрвола не активен".to_owned(),
        ),
        FirewallState::Unknown => (CheckStatus::Unknown, "нет systemctl".to_owned()),
    };
    Check {
        label: CHECK_FIREWALL,
        detail,
        status,
    }
}

fn fail2ban(snapshot: &SecuritySnapshot) -> Check {
    let (status, detail) = if snapshot.has_fail2ban {
        let jails: Vec<&str> = snapshot.jails.iter().map(|j| j.name.as_str()).collect();
        (CheckStatus::Pass, jails.join(", "))
    } else {
        (CheckStatus::Warn, "не установлен или не запущен".to_owned())
    };
    Check {
        label: CHECK_FAIL2BAN,
        detail,
        status,
    }
}

fn password_auth(snapshot: &SecuritySnapshot) -> Check {
    let (status, detail) = match snapshot.sshd.password_auth {
        Some(Switch::Off) => (CheckStatus::Pass, "PasswordAuthentication no".to_owned()),
        Some(Switch::On) => (CheckStatus::Fail, "PasswordAuthentication yes".to_owned()),
        _ => (CheckStatus::Unknown, "нужен sudo для sshd -T".to_owned()),
    };
    Check {
        label: CHECK_PASSWORD_AUTH,
        detail,
        status,
    }
}

fn root_login(snapshot: &SecuritySnapshot) -> Check {
    let (status, detail) = match snapshot.sshd.permit_root_login.as_deref() {
        Some("yes") => (CheckStatus::Fail, "PermitRootLogin yes".to_owned()),
        Some(value) => (CheckStatus::Pass, format!("PermitRootLogin {value}")),
        None => (CheckStatus::Unknown, "нужен sudo для sshd -T".to_owned()),
    };
    Check {
        label: CHECK_ROOT_LOGIN,
        detail,
        status,
    }
}

fn brute_force(snapshot: &SecuritySnapshot) -> Check {
    let active = snapshot.brute_force_count();
    let (status, detail) = if active == 0 {
        (
            CheckStatus::Pass,
            format!("{} неудачных попыток за 24 ч", snapshot.failed_logins),
        )
    } else {
        (
            CheckStatus::Warn,
            format!("{active} IP атакуют прямо сейчас"),
        )
    };
    Check {
        label: CHECK_BRUTE_FORCE,
        detail,
        status,
    }
}

fn ssh_port(snapshot: &SecuritySnapshot) -> Check {
    let (status, detail) = match snapshot.sshd.port {
        Some(22) => (
            CheckStatus::Warn,
            "порт 22 — основная цель сканеров".to_owned(),
        ),
        Some(port) => (CheckStatus::Pass, format!("порт {port}")),
        None => (CheckStatus::Unknown, String::new()),
    };
    Check {
        label: CHECK_PORT,
        detail,
        status,
    }
}
