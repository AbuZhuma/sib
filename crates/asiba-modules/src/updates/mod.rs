mod model;
mod parse;

use asiba_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, Sample, Schedule, Snapshot,
    Transport,
};
use async_trait::async_trait;

pub use model::{PackageManager, UpdatesSnapshot};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("updates");
pub const KEY_PENDING: &str = "updates.pending";
pub const KEY_REBOOT_REQUIRED: &str = "updates.reboot_required";

const DETECT: &str = "command -v apt-get || command -v dnf || command -v yum || command -v pacman || command -v zypper || command -v apk";
const SCRIPT_PARTS: [(&str, &str); 5] = [
    ("manager", DETECT),
    (
        "apt",
        "if command -v apt-get >/dev/null; then apt-get -s upgrade | grep '^Inst'; fi",
    ),
    (
        "dnf",
        "if command -v dnf >/dev/null; then dnf -q check-update 2>/dev/null | grep -E '^[[:alnum:]]' ; elif command -v yum >/dev/null; then yum -q check-update | grep -E '^[[:alnum:]]'; fi",
    ),
    (
        "other",
        "if command -v pacman >/dev/null; then pacman -Qu; elif command -v zypper >/dev/null; then zypper -q lu | grep '^v'; elif command -v apk >/dev/null; then apk version -l '<' | tail -n +2; fi",
    ),
    (
        "reboot",
        "[ -f /var/run/reboot-required ] && echo yes; command -v needs-restarting >/dev/null && { needs-restarting -r >/dev/null || echo yes; }",
    ),
];

pub struct UpdatesModule;

#[async_trait]
impl Module for UpdatesModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Обновления"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Slow
    }

    async fn detect(&self, transport: &dyn Transport) -> Result<Availability, ModuleError> {
        let output = transport.exec(DETECT).await?;
        if output.is_success() {
            return Ok(Availability::Available);
        }
        Ok(Availability::Unavailable {
            reason: "пакетный менеджер не найден".to_owned(),
        })
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        _context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let snapshot = parse::updates_snapshot(&output.stdout)?;
        let samples = vec![
            Sample::new(KEY_PENDING, snapshot.pending as f64),
            Sample::new(
                KEY_REBOOT_REQUIRED,
                u8::from(snapshot.reboot_required) as f64,
            ),
        ];
        Ok(Snapshot::new(snapshot).with_samples(samples))
    }
}
