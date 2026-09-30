use sib_modules::updates::{self, UpdatesSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, bullet, field, heading, more};

pub struct UpdatesSection;

const MAX_PACKAGES: usize = 25;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a UpdatesSnapshot> {
    ctx.server.data::<UpdatesSnapshot>(updates::ID)
}

fn packages(snapshot: &UpdatesSnapshot) -> (String, usize) {
    let shown: Vec<&str> = snapshot
        .packages
        .iter()
        .take(MAX_PACKAGES)
        .map(String::as_str)
        .collect();
    (
        shown.join(", "),
        snapshot.packages.len().saturating_sub(MAX_PACKAGES),
    )
}

impl Section for UpdatesSection {
    fn id(&self) -> SectionId {
        SectionId::Updates
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Updates");
        bullet(out, "Package manager", snapshot.manager.label());
        bullet(out, "Pending", snapshot.pending.to_string());
        bullet(out, "Security ones", snapshot.security.to_string());
        bullet(
            out,
            "Reboot needed",
            if snapshot.reboot_required {
                "yes"
            } else {
                "no"
            },
        );
        let (list, hidden) = packages(snapshot);
        if !list.is_empty() {
            bullet(out, "Packages", list);
            more(out, hidden, "packages");
        }
        blank(out);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Updates");
        field(out, "package_manager", snapshot.manager.label());
        field(out, "pending", snapshot.pending.to_string());
        field(out, "security_pending", snapshot.security.to_string());
        field(out, "reboot_required", snapshot.reboot_required.to_string());
        let (list, hidden) = packages(snapshot);
        if !list.is_empty() {
            field(out, "packages", list);
            more(out, hidden, "packages");
        }
        blank(out);
    }
}
