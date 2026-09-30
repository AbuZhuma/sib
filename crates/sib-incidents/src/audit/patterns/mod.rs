mod access;
mod collection;
mod firewall;
mod hardening;
mod kernel;
mod kernel_protection;
mod logs;
mod network;
mod reliability;
mod resources;
mod ssh;
mod updates;

use sib_modules::security::{self, SecuritySnapshot};

use super::context::AuditContext;
use super::pattern::{Pattern, Verdict};

pub const NO_SECURITY_DATA: &str = "the security module has not collected data yet";
pub const NEEDS_SUDO: &str = "not available without sudo";

pub fn all() -> impl Iterator<Item = &'static Pattern> {
    ssh::PATTERNS
        .iter()
        .chain(access::PATTERNS)
        .chain(firewall::PATTERNS)
        .chain(kernel::PATTERNS)
        .chain(kernel_protection::PATTERNS)
        .chain(hardening::PATTERNS)
        .chain(resources::PATTERNS)
        .chain(reliability::PATTERNS)
        .chain(network::PATTERNS)
        .chain(updates::PATTERNS)
        .chain(logs::PATTERNS)
        .chain(collection::PATTERNS)
}

fn security_snapshot<'a>(ctx: &'a AuditContext<'_>) -> Result<&'a SecuritySnapshot, Vec<Verdict>> {
    ctx.data::<SecuritySnapshot>(security::ID)
        .ok_or_else(|| Verdict::skipped(NO_SECURITY_DATA).single())
}

fn with_security(
    ctx: &AuditContext<'_>,
    evaluate: impl FnOnce(&SecuritySnapshot) -> Verdict,
) -> Vec<Verdict> {
    match security_snapshot(ctx) {
        Ok(snapshot) => evaluate(snapshot).single(),
        Err(skipped) => skipped,
    }
}
