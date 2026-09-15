use asiba_core::ServerState;
use asiba_modules::anomalies::{self, AnomaliesSnapshot};
use asiba_modules::network::{self, NetworkSnapshot};
use asiba_modules::ports::{self, PortsSnapshot};

use super::reliability::list_or;
use super::{Area, AuditCheck};

const ERRORS_WARN: u64 = 1;
const ERRORS_FAIL: u64 = 1000;
const PUBLIC_WARN: usize = 6;
const PUBLIC_LISTED: usize = 12;

const ADVICE_ERRORS: &str =
    "Ошибки и дропы на интерфейсе - проверьте кабель, драйвер, MTU и нагрузку.";
const ADVICE_EXPOSED: &str = "Порт слушает на 0.0.0.0, доступен снаружи и не разрешён файрволом явно - закройте или ограничьте адресами.";
const ADVICE_PUBLIC: &str = "Чем меньше публичных портов, тем меньше поверхность атаки - привяжите служебные порты к 127.0.0.1.";
const ADVICE_FIREWALL: &str = "Включите nftables/ufw/firewalld и разрешите только нужные порты.";
const ADVICE_ATTACK: &str = "Признаки атаки: посмотрите вкладку «Аномалии», забаньте подозрительные адреса, включите rate limit.";

pub fn checks(server: &ServerState) -> Vec<AuditCheck> {
    let mut checks = Vec::new();
    if let Some(snapshot) = server.data::<NetworkSnapshot>(network::ID) {
        checks.push(interface_errors(snapshot));
    }
    if let Some(snapshot) = server.data::<PortsSnapshot>(ports::ID) {
        checks.extend(port_checks(snapshot));
    }
    if let Some(snapshot) = server.data::<AnomaliesSnapshot>(anomalies::ID) {
        checks.push(attack(snapshot));
    }
    checks
}

fn interface_errors(snapshot: &NetworkSnapshot) -> AuditCheck {
    let noisy: Vec<String> = snapshot
        .interfaces
        .iter()
        .filter(|i| !i.is_loopback())
        .map(|i| {
            let errors = i.rx.errors + i.tx.errors + i.rx.drops + i.tx.drops;
            (i.name.as_str(), errors)
        })
        .filter(|(_, errors)| *errors >= ERRORS_WARN)
        .map(|(name, errors)| format!("{name}: {errors}"))
        .collect();
    let worst = snapshot
        .interfaces
        .iter()
        .map(|i| i.rx.errors + i.tx.errors + i.rx.drops + i.tx.drops)
        .max()
        .unwrap_or(0);
    AuditCheck::new(
        Area::Network,
        "Интерфейсы без ошибок и дропов",
        ADVICE_ERRORS,
    )
    .graded(
        worst >= ERRORS_FAIL,
        !noisy.is_empty(),
        list_or(&noisy, "ошибок нет"),
    )
}

fn port_checks(snapshot: &PortsSnapshot) -> Vec<AuditCheck> {
    let exposed: Vec<String> = snapshot
        .exposed_without_firewall()
        .map(|p| format!("{}/{} {}", p.port, p.protocol.label(), p.process_label()))
        .collect();
    let (total, public) = public_ports(snapshot);
    let mut checks = vec![
        AuditCheck::new(
            Area::Network,
            "Нет портов, открытых наружу мимо файрвола",
            ADVICE_EXPOSED,
        )
        .graded(
            false,
            !exposed.is_empty(),
            list_or(&exposed, "таких портов нет"),
        ),
        AuditCheck::new(Area::Network, "Публичных портов немного", ADVICE_PUBLIC).graded(
            false,
            total >= PUBLIC_WARN,
            format!("{total}: {}", list_or(&public, "-")),
        ),
    ];
    if let Some(firewall) = &snapshot.firewall {
        checks.push(
            AuditCheck::new(Area::Network, "Файрвол активен", ADVICE_FIREWALL).graded(
                !firewall.active,
                false,
                format!(
                    "{:?}, правил на порты: {}",
                    firewall.backend,
                    firewall.allowed.len()
                ),
            ),
        );
    }
    checks
}

fn public_ports(snapshot: &PortsSnapshot) -> (usize, Vec<String>) {
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
    (total, public)
}

fn attack(snapshot: &AnomaliesSnapshot) -> AuditCheck {
    let signs: Vec<String> = snapshot
        .signs
        .iter()
        .map(|s| format!("{}: {}", s.kind.label(), s.detail))
        .collect();
    AuditCheck::new(
        Area::Network,
        "Нет признаков DDoS и сканирования",
        ADVICE_ATTACK,
    )
    .graded(
        snapshot.is_under_attack(),
        snapshot.has_signs(),
        list_or(&signs, "трафик в норме"),
    )
}
