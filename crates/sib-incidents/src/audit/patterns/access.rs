use sib_modules::security::SecuritySnapshot;

use super::{NEEDS_SUDO, with_security};
use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, SECURITY_ACCESS};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const PASSWORD_METHOD: &str = "password";
const SHOWN: usize = 5;
const SUSPICIOUS_FAILURES: u64 = 5;

const ADVICE_UID0: &str =
    "Смените UID таким учётным записям или удалите их: UID 0 должен быть только у root.";
const ADVICE_EMPTY: &str =
    "Задайте пароль (passwd <user>) или заблокируйте учётную запись (passwd -l <user>).";
const ADVICE_NOPASSWD: &str = "Уберите NOPASSWD из /etc/sudoers и /etc/sudoers.d/, оставив его только для автоматики с узким набором команд.";
const ADVICE_KEYS: &str = "Выполните chmod 600 ~/.ssh/authorized_keys и chmod 700 ~/.ssh.";
const ADVICE_WORLD_WRITABLE: &str =
    "Снимите бит o+w: конфигурацию в /etc не должен менять обычный пользователь.";
const ADVICE_ROOT_PASSWORD_LOGINS: &str =
    "Переведите root на вход по ключу или запретите PermitRootLogin.";
const ADVICE_ATTACKER_LOGIN: &str = "Считайте учётную запись скомпрометированной: смените пароль и ключи, проверьте ~/.ssh, crontab и запущенные процессы.";
const ADVICE_SUDO_FAILURES: &str =
    "Посмотрите в подразделе «Входы и sudo», какой пользователь и какие команды вызывал.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "access.extra_uid0",
        area: Area::Access,
        subject: "Учётки с UID 0",
        description: "Учётная запись с UID 0 равна root по правам.",
        weight: Weight::High,
        advice: ADVICE_UID0,
        evidence: EvidenceSource::File("/etc/passwd"),
        evaluate: extra_uid0,
    },
    Pattern {
        id: "access.empty_passwords",
        area: Area::Access,
        subject: "Пустые пароли",
        description: "Учётная запись с пустым полем пароля в /etc/shadow позволяет войти без пароля локально, а при PermitEmptyPasswords yes - и по SSH.",
        weight: Weight::High,
        advice: ADVICE_EMPTY,
        evidence: EvidenceSource::File("/etc/shadow"),
        evaluate: empty_passwords,
    },
    Pattern {
        id: "access.sudo_nopasswd",
        area: Area::Access,
        subject: "sudo без пароля",
        description: "С правилом NOPASSWD компрометация учётной записи пользователя даёт права root без ввода пароля.",
        weight: Weight::Low,
        advice: ADVICE_NOPASSWD,
        evidence: EvidenceSource::File("/etc/sudoers"),
        evaluate: sudo_nopasswd,
    },
    Pattern {
        id: "access.writable_keys",
        area: Area::Access,
        subject: "Права на authorized_keys",
        description: "Если authorized_keys доступен на запись группе или всем, любой локальный процесс может вписать туда ключ и получить доступ по SSH.",
        weight: Weight::High,
        advice: ADVICE_KEYS,
        evidence: EvidenceSource::None,
        evaluate: writable_keys,
    },
    Pattern {
        id: "access.world_writable_etc",
        area: Area::Access,
        subject: "Файлы в /etc с записью для всех",
        description: "Файл конфигурации с битом o+w может изменить любой пользователь или сервис.",
        weight: Weight::Medium,
        advice: ADVICE_WORLD_WRITABLE,
        evidence: EvidenceSource::None,
        evaluate: world_writable,
    },
    Pattern {
        id: "access.root_password_logins",
        area: Area::Access,
        subject: "Входы root по паролю",
        description: "Успешные входы root по паролю за сутки: учётная запись root доступна для перебора.",
        weight: Weight::Medium,
        advice: ADVICE_ROOT_PASSWORD_LOGINS,
        evidence: EvidenceSource::Tab(SECURITY_ACCESS),
        evaluate: root_password_logins,
    },
    Pattern {
        id: "access.login_from_attacker",
        area: Area::Access,
        subject: "Вход с атакующего адреса",
        description: "Адрес перебирал пароли, а затем успешно вошёл.",
        weight: Weight::High,
        advice: ADVICE_ATTACKER_LOGIN,
        evidence: EvidenceSource::Tab(SECURITY_ACCESS),
        evaluate: login_from_attacker,
    },
    Pattern {
        id: "access.sudo_failures",
        area: Area::Access,
        subject: "Отказы sudo",
        description: "Отказы sudo за сутки: пользователь без прав пытался выполнить команду от root или ошибся паролем несколько раз.",
        weight: Weight::Low,
        advice: ADVICE_SUDO_FAILURES,
        evidence: EvidenceSource::Tab(SECURITY_ACCESS),
        evaluate: sudo_failures,
    },
];

fn extra_uid0(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| {
        let extra = &s.hardening.extra_uid0;
        if extra.is_empty() {
            return Verdict::pass("только root");
        }
        Verdict::fail(format!("кроме root UID 0 имеют: {}", extra.join(", ")))
    })
}

fn empty_passwords(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match &s.hardening.empty_passwords {
        Some(users) if users.is_empty() => Verdict::pass("нет"),
        Some(users) => Verdict::fail(format!("без пароля: {}", users.join(", "))),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn sudo_nopasswd(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.hardening.sudo_nopasswd {
        Some(0) => Verdict::pass("правил NOPASSWD нет"),
        Some(count) => Verdict::warn(format!("{count} правил NOPASSWD")),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn writable_keys(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match &s.hardening.writable_keys {
        Some(files) if files.is_empty() => Verdict::pass("доступны на запись только владельцу"),
        Some(files) => Verdict::fail(format!("открыты на запись: {}", files.join(", "))),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn world_writable(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let files = &s.hardening.world_writable_etc;
        if files.is_empty() {
            return Verdict::pass("нет");
        }
        Verdict::fail(format!("с записью для всех: {}", files.join(", ")))
    })
}

fn root_password_logins(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let from: Vec<&str> = s
            .logins
            .iter()
            .filter(|l| l.is_root() && l.method == PASSWORD_METHOD)
            .map(|l| l.from.as_str())
            .collect();
        if from.is_empty() {
            return Verdict::pass("за сутки не было");
        }
        Verdict::warn(format!(
            "{} за сутки, с адресов: {}",
            from.len(),
            list_or(&from[..from.len().min(SHOWN)], "")
        ))
    })
}

fn login_from_attacker(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let mut suspicious: Vec<String> = Vec::new();
        for login in &s.logins {
            let Some(attacker) = s
                .attackers
                .iter()
                .find(|a| a.ip == login.from && a.failures >= SUSPICIOUS_FAILURES)
            else {
                continue;
            };
            let entry = format!(
                "{} ← {} ({}, {} неудачных попыток с этого адреса)",
                login.user, login.from, login.method, attacker.failures
            );
            if !suspicious.contains(&entry) {
                suspicious.push(entry);
            }
        }
        if suspicious.is_empty() {
            return Verdict::pass("совпадений с атакующими адресами нет");
        }
        Verdict::fail(format!(
            "успешный вход с адреса, который перебирал пароли: {}",
            list_or(&suspicious[..suspicious.len().min(SHOWN)], "")
        ))
    })
}

fn sudo_failures(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let failed: Vec<String> = s
            .sudo_calls
            .iter()
            .filter(|c| !c.is_success)
            .map(|c| format!("{}: {}", c.user, c.command))
            .collect();
        if failed.is_empty() {
            return Verdict::pass("за сутки отказов нет");
        }
        Verdict::warn(format!(
            "{} отказов за сутки: {}",
            failed.len(),
            list_or(&failed[..failed.len().min(SHOWN)], "")
        ))
    })
}
