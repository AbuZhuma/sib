use asiba_modules::ports::{self, PortsSnapshot, Protocol};
use asiba_modules::security::{FirewallState, SecuritySnapshot, Switch};

use super::{security_snapshot, with_security};
use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, SECURITY_ATTACKS, TAB_PORTS};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const PUBLIC_WARN: usize = 6;
const PUBLIC_LISTED: usize = 12;
const SHOWN: usize = 6;
const DEFAULT_SSH_PORT: u16 = 22;

const ADVICE_FIREWALL: &str =
    "Включите nftables, ufw или firewalld и разрешите снаружи только нужные порты (SSH, 80/443).";
const ADVICE_RISKY: &str = "Закройте порт или привяжите сервис к 127.0.0.1 / внутренней сети; наружу такие сервисы публикуют через VPN или TLS с аутентификацией.";
const ADVICE_EXPOSED: &str = "Порт слушает на всех адресах и доступен снаружи без явного правила файрвола. Закройте его или ограничьте адреса источника.";
const ADVICE_PUBLIC: &str =
    "Привяжите служебные порты к 127.0.0.1: каждый публичный порт увеличивает поверхность атаки.";
const ADVICE_FAIL2BAN: &str = "Установите fail2ban и включите джейл sshd: он блокирует адрес после нескольких неудачных попыток.";
const ADVICE_BRUTE_FORCE: &str =
    "Забаньте адреса в подразделе «Атаки и баны», отключите вход по паролю, включите fail2ban.";
const ADVICE_UNBANNED: &str = "Забаньте адреса вручную или включите fail2ban.";
const ADVICE_SSH_EXPOSED: &str = "Отключите вход по паролю и установите fail2ban.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "firewall.active",
        area: Area::Firewall,
        subject: "Файрвол",
        description: "Без файрвола снаружи доступны все сокеты, слушающие 0.0.0.0, включая служебные.",
        weight: Weight::High,
        advice: ADVICE_FIREWALL,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: firewall,
    },
    Pattern {
        id: "firewall.risky_ports",
        area: Area::Firewall,
        subject: "Опасные открытые порты",
        description: "telnet (23), ftp (21), rsh (512-514), Docker API (2375/2376), Redis (6379), MongoDB (27017), Elasticsearch (9200) работают без шифрования или без аутентификации по умолчанию и не должны быть доступны снаружи.",
        weight: Weight::High,
        advice: ADVICE_RISKY,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: risky_ports,
    },
    Pattern {
        id: "firewall.exposed_ports",
        area: Area::Firewall,
        subject: "Порты снаружи мимо файрвола",
        description: "Порт отвечает на внешнее подключение, правила файрвола для него нет.",
        weight: Weight::Medium,
        advice: ADVICE_EXPOSED,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: exposed_ports,
    },
    Pattern {
        id: "firewall.public_ports",
        area: Area::Firewall,
        subject: "Число публичных портов",
        description: "Каждый порт, слушающий не на localhost, - отдельная точка входа, которую нужно защищать и обновлять.",
        weight: Weight::Low,
        advice: ADVICE_PUBLIC,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: public_ports,
    },
    Pattern {
        id: "firewall.fail2ban",
        area: Area::Firewall,
        subject: "Защита от перебора (fail2ban)",
        description: "fail2ban читает журнал sshd и блокирует адрес после нескольких неудачных попыток.",
        weight: Weight::Medium,
        advice: ADVICE_FAIL2BAN,
        evidence: EvidenceSource::Tab(SECURITY_ATTACKS),
        evaluate: fail2ban,
    },
    Pattern {
        id: "firewall.ssh_exposed",
        area: Area::Firewall,
        subject: "SSH с паролями без защиты",
        description: "Порт SSH доступен снаружи, вход по паролю разрешён, fail2ban отсутствует. Скорость перебора ничем не ограничена.",
        weight: Weight::High,
        advice: ADVICE_SSH_EXPOSED,
        evidence: EvidenceSource::File("/etc/ssh/sshd_config"),
        evaluate: ssh_exposed,
    },
    Pattern {
        id: "firewall.brute_force",
        area: Area::Firewall,
        subject: "Перебор паролей сейчас",
        description: "Адреса с десятью и более неудачными попытками за последние 10 минут.",
        weight: Weight::Medium,
        advice: ADVICE_BRUTE_FORCE,
        evidence: EvidenceSource::Tab(SECURITY_ATTACKS),
        evaluate: brute_force,
    },
    Pattern {
        id: "firewall.unbanned_attackers",
        area: Area::Firewall,
        subject: "Атакующие без бана",
        description: "Адреса, которые перебирают пароли и не заблокированы fail2ban или файрволом.",
        weight: Weight::Medium,
        advice: ADVICE_UNBANNED,
        evidence: EvidenceSource::Tab(SECURITY_ATTACKS),
        evaluate: unbanned_attackers,
    },
];

fn firewall(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    if let Ok(snapshot) = security_snapshot(ctx) {
        match &snapshot.firewall {
            FirewallState::Active(name) => {
                return Verdict::pass(format!("активен ({name})")).single();
            }
            FirewallState::Inactive => {
                return Verdict::fail(
                    "не активен: ufw, firewalld, nftables и iptables не запущены",
                )
                .single();
            }
            FirewallState::Unknown => {}
        }
    }
    let Some(ports) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("нет данных о файрволе").single();
    };
    match &ports.firewall {
        Some(firewall) if firewall.active => Verdict::pass(format!(
            "активен ({:?}), правил на порты: {}",
            firewall.backend,
            firewall.allowed.len()
        )),
        Some(firewall) => {
            Verdict::fail(format!("{:?} установлен, но не активен", firewall.backend))
        }
        None => Verdict::skipped("нет данных о файрволе"),
    }
    .single()
}

fn risky_ports(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| {
        let ports = &s.hardening.risky_ports;
        if ports.is_empty() {
            return Verdict::pass("telnet, ftp, rsh, docker api, redis, mongo, elastic закрыты");
        }
        let list: Vec<String> = ports.iter().map(u16::to_string).collect();
        Verdict::fail(format!("слушают снаружи: {}", list.join(", ")))
    })
}

fn exposed_ports(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("модуль ports не собрал данные").single();
    };
    let exposed: Vec<String> = snapshot
        .exposed_without_firewall()
        .map(|p| format!("{}/{} {}", p.port, p.protocol.label(), p.process_label()))
        .collect();
    if exposed.is_empty() {
        return Verdict::pass("все доступные снаружи порты разрешены файрволом явно").single();
    }
    Verdict::warn(format!(
        "доступны снаружи без правила: {}",
        list_or(&exposed[..exposed.len().min(SHOWN)], "")
    ))
    .single()
}

fn public_ports(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("модуль ports не собрал данные").single();
    };
    let mut public: Vec<String> = snapshot
        .public_ports()
        .map(|p| format!("{}/{}", p.port, p.protocol.label()))
        .collect();
    public.sort();
    public.dedup();
    let total = public.len();
    let hidden = total.saturating_sub(PUBLIC_LISTED);
    public.truncate(PUBLIC_LISTED);
    if hidden > 0 {
        public.push(format!("и ещё {hidden}"));
    }
    Verdict::graded(
        false,
        total >= PUBLIC_WARN,
        format!("{total}: {}", list_or(&public, "-")),
    )
    .single()
}

fn fail2ban(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        if s.has_fail2ban {
            let jails: Vec<&str> = s.jails.iter().map(|j| j.name.as_str()).collect();
            return Verdict::pass(format!("активен, джейлы: {}", list_or(&jails, "нет")));
        }
        Verdict::warn("не установлен или не запущен")
    })
}

fn ssh_exposed(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Ok(security) = security_snapshot(ctx) else {
        return Verdict::skipped(super::NO_SECURITY_DATA).single();
    };
    let Some(ports) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("модуль ports не собрал данные").single();
    };
    let ssh_port = security.sshd.port.unwrap_or(DEFAULT_SSH_PORT);
    let is_public = ports
        .public_ports()
        .any(|p| p.port == ssh_port && p.protocol == Protocol::Tcp);
    let passwords = security.sshd.password_auth == Some(Switch::On);
    match (is_public, passwords, security.has_fail2ban) {
        (true, true, false) => Verdict::fail(format!(
            "порт {ssh_port} открыт наружу, пароли разрешены, fail2ban нет"
        )),
        (true, true, true) => Verdict::warn(format!(
            "порт {ssh_port} открыт наружу с паролями, перебор сдерживает только fail2ban"
        )),
        (true, false, _) => {
            Verdict::pass(format!("порт {ssh_port} открыт наружу, но только по ключу"))
        }
        (false, _, _) => Verdict::pass(format!("порт {ssh_port} снаружи не виден")),
    }
    .single()
}

fn brute_force(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Ok(snapshot) = security_snapshot(ctx) else {
        return Verdict::skipped(super::NO_SECURITY_DATA).single();
    };
    let attackers: Vec<String> = snapshot
        .attackers
        .iter()
        .filter(|a| a.is_brute_force())
        .map(|a| {
            format!(
                "{} ({}, {} попыток за 10 мин)",
                a.ip,
                ctx.state.country_of(&a.ip).unwrap_or("?"),
                a.recent_failures
            )
        })
        .collect();
    if attackers.is_empty() {
        return Verdict::pass(format!(
            "активного перебора нет, за сутки {} адресов с неудачными попытками",
            snapshot.attackers.len()
        ))
        .single();
    }
    Verdict::fail(format!(
        "идёт перебор с {}: {}",
        attackers.len(),
        list_or(&attackers[..attackers.len().min(SHOWN)], "")
    ))
    .single()
}

fn unbanned_attackers(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let unbanned: Vec<&str> = s
            .attackers
            .iter()
            .filter(|a| a.is_brute_force() && !s.is_banned(&a.ip))
            .map(|a| a.ip.as_str())
            .collect();
        if unbanned.is_empty() {
            return Verdict::pass(format!(
                "все атакующие заблокированы, банов всего {}",
                s.bans.len()
            ));
        }
        Verdict::warn(format!(
            "{} без бана: {}",
            unbanned.len(),
            list_or(&unbanned[..unbanned.len().min(SHOWN)], "")
        ))
    })
}
