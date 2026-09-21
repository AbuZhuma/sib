use asiba_modules::security::{SecuritySnapshot, Switch};

use super::{NEEDS_SUDO, with_security};
use crate::audit::context::AuditContext;
use crate::audit::evidence::EvidenceSource;
use crate::audit::pattern::{Area, Outcome, Pattern, Verdict, Weight};

const SSHD_CONFIG: &str = "/etc/ssh/sshd_config";
const MAX_AUTH_TRIES: u32 = 4;
const MAX_LOGIN_GRACE_SECS: u32 = 60;
const DEFAULT_PORT: u16 = 22;

const ADVICE_SSHD: &str = "Поправьте параметр в /etc/ssh/sshd_config (или в sshd_config.d/*.conf) и перезапустите sshd: systemctl restart sshd.";

const fn sshd_pattern(
    id: &'static str,
    subject: &'static str,
    description: &'static str,
    weight: Weight,
    evaluate: fn(&AuditContext<'_>) -> Vec<Verdict>,
) -> Pattern {
    Pattern {
        id,
        area: Area::Ssh,
        subject,
        description,
        weight,
        advice: ADVICE_SSHD,
        evidence: EvidenceSource::File(SSHD_CONFIG),
        evaluate,
    }
}

pub static PATTERNS: &[Pattern] = &[
    sshd_pattern(
        "ssh.password_auth",
        "Вход по паролю",
        "Пароли перебираются ботами круглосуточно. Вход только по ключу (PasswordAuthentication no) делает перебор бессмысленным.",
        Weight::High,
        password_auth,
    ),
    sshd_pattern(
        "ssh.root_login",
        "Вход root по SSH",
        "Учётная запись root есть на каждом сервере, поэтому её перебирают первой. PermitRootLogin no заставляет входить под обычным пользователем и повышать права через sudo.",
        Weight::High,
        root_login,
    ),
    sshd_pattern(
        "ssh.pubkey_auth",
        "Вход по ключу",
        "PubkeyAuthentication yes нужен, чтобы можно было отключить пароли и не потерять доступ.",
        Weight::Medium,
        pubkey_auth,
    ),
    sshd_pattern(
        "ssh.empty_passwords",
        "Пустые пароли",
        "PermitEmptyPasswords yes пускает пользователей без пароля - любой сканер войдёт с первой попытки.",
        Weight::High,
        empty_passwords,
    ),
    sshd_pattern(
        "ssh.max_auth_tries",
        "Лимит попыток за соединение",
        "MaxAuthTries ограничивает число попыток пароля в одном соединении; большое значение ускоряет перебор.",
        Weight::Low,
        max_auth_tries,
    ),
    sshd_pattern(
        "ssh.login_grace_time",
        "Время на ввод пароля",
        "LoginGraceTime - сколько секунд держится неаутентифицированное соединение. Большое значение позволяет занять все слоты sshd и устроить отказ в обслуживании.",
        Weight::Low,
        login_grace_time,
    ),
    sshd_pattern(
        "ssh.max_startups",
        "Лимит одновременных подключений",
        "MaxStartups - сколько неаутентифицированных соединений sshd держит одновременно; это встроенный rate limit против перебора и флуда.",
        Weight::Low,
        max_startups,
    ),
    sshd_pattern(
        "ssh.x11_forwarding",
        "Проброс X11",
        "На сервере без графики X11Forwarding не нужен и расширяет поверхность атаки через клиентский X-сервер.",
        Weight::Low,
        x11_forwarding,
    ),
    sshd_pattern(
        "ssh.tcp_forwarding",
        "Проброс TCP-портов",
        "AllowTcpForwarding yes позволяет любому вошедшему пользователю туннелировать трафик через сервер, в том числе к внутренним сервисам.",
        Weight::Low,
        tcp_forwarding,
    ),
    sshd_pattern(
        "ssh.client_alive",
        "Обрыв висячих сессий",
        "ClientAliveInterval заставляет sshd закрывать сессии, клиент которых пропал; иначе брошенные сессии живут бесконечно.",
        Weight::Low,
        client_alive,
    ),
    sshd_pattern(
        "ssh.port",
        "Порт SSH",
        "Порт 22 сканируют первым; другой порт не защищает сам по себе, но резко снижает шум в журнале и число попыток.",
        Weight::Low,
        port,
    ),
];

fn switch(
    value: Option<Switch>,
    option: &str,
    wanted: Switch,
    bad: Outcome,
    problem: &str,
) -> Verdict {
    let state = |v: Switch| match v {
        Switch::On => "yes",
        Switch::Off => "no",
        Switch::Unknown => "?",
    };
    match value {
        Some(v) if v == wanted => Verdict::pass(format!("{option} {}", state(v))),
        Some(v) => Verdict::new(bad, format!("{problem} ({option} {})", state(v))),
        None => Verdict::skipped(NEEDS_SUDO),
    }
}

fn password_auth(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| {
        switch(
            s.sshd.password_auth,
            "PasswordAuthentication",
            Switch::Off,
            Outcome::Fail,
            "пароли принимаются - сервер открыт для перебора",
        )
    })
}

fn root_login(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.permit_root_login.as_deref() {
        Some("yes") => Verdict::fail("root может входить по паролю (PermitRootLogin yes)"),
        Some("no") => Verdict::pass("запрещён (PermitRootLogin no)"),
        Some(value) => Verdict::warn(format!(
            "root может входить по ключу (PermitRootLogin {value})"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn pubkey_auth(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.pubkey_auth,
            "PubkeyAuthentication",
            Switch::On,
            Outcome::Fail,
            "ключи не принимаются - остаются только пароли",
        )
    })
}

fn empty_passwords(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.permit_empty_passwords,
            "PermitEmptyPasswords",
            Switch::Off,
            Outcome::Fail,
            "разрешён вход без пароля",
        )
    })
}

fn max_auth_tries(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.max_auth_tries {
        Some(tries) if tries <= MAX_AUTH_TRIES => Verdict::pass(format!("MaxAuthTries {tries}")),
        Some(tries) => Verdict::warn(format!(
            "{tries} попыток за соединение (MaxAuthTries), рекомендуется не больше {MAX_AUTH_TRIES}"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn login_grace_time(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.login_grace_time {
        Some(secs) if secs > 0 && secs <= MAX_LOGIN_GRACE_SECS => {
            Verdict::pass(format!("LoginGraceTime {secs}"))
        }
        Some(0) => Verdict::warn("без ограничения (LoginGraceTime 0)"),
        Some(secs) => Verdict::warn(format!(
            "{secs} с на ввод пароля (LoginGraceTime), рекомендуется {MAX_LOGIN_GRACE_SECS}"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn max_startups(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.max_startups.as_deref() {
        Some(value) if value.contains(':') => Verdict::pass(format!("MaxStartups {value}")),
        Some(value) => Verdict::warn(format!(
            "нет случайного отброса соединений (MaxStartups {value}), рекомендуется 10:30:100"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn x11_forwarding(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.x11_forwarding,
            "X11Forwarding",
            Switch::Off,
            Outcome::Warn,
            "включён проброс X11",
        )
    })
}

fn tcp_forwarding(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.allow_tcp_forwarding,
            "AllowTcpForwarding",
            Switch::Off,
            Outcome::Warn,
            "любой пользователь может туннелировать трафик",
        )
    })
}

fn client_alive(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.client_alive_interval {
        Some(0) => Verdict::warn("висячие сессии не закрываются (ClientAliveInterval 0)"),
        Some(secs) => Verdict::pass(format!("ClientAliveInterval {secs}")),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn port(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.port {
        Some(DEFAULT_PORT) => Verdict::warn("стандартный порт 22 - основная цель сканеров"),
        Some(port) => Verdict::pass(format!("порт {port}")),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}
