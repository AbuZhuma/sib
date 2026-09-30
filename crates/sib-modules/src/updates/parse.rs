use sib_core::ModuleError;

use super::model::{PackageManager, UpdatesSnapshot};
use crate::common::sections::Sections;

const MAX_PACKAGES: usize = 200;

pub fn updates_snapshot(raw: &str) -> Result<UpdatesSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let manager = manager(sections.get_or_empty("manager"))
        .ok_or_else(|| ModuleError::Parse("пакетный менеджер не определён".to_owned()))?;
    let lines: Vec<&str> = match manager {
        PackageManager::Apt => sections.get_or_empty("apt").lines().collect(),
        PackageManager::Dnf | PackageManager::Yum => sections.get_or_empty("dnf").lines().collect(),
        _ => sections.get_or_empty("other").lines().collect(),
    };
    let packages: Vec<String> = lines
        .iter()
        .filter_map(|l| package_name(manager, l))
        .take(MAX_PACKAGES)
        .collect();
    let security = lines
        .iter()
        .filter(|l| l.to_lowercase().contains("security"))
        .count() as u32;
    Ok(UpdatesSnapshot {
        manager,
        pending: lines.iter().filter(|l| !l.trim().is_empty()).count() as u32,
        security,
        reboot_required: sections.get_or_empty("reboot").contains("yes"),
        packages,
    })
}

fn manager(raw: &str) -> Option<PackageManager> {
    let binary = raw.lines().next()?.trim().rsplit('/').next()?;
    Some(match binary {
        "apt-get" => PackageManager::Apt,
        "dnf" => PackageManager::Dnf,
        "yum" => PackageManager::Yum,
        "pacman" => PackageManager::Pacman,
        "zypper" => PackageManager::Zypper,
        "apk" => PackageManager::Apk,
        _ => return None,
    })
}

fn package_name(manager: PackageManager, line: &str) -> Option<String> {
    let mut fields = line.split_whitespace();
    let name = match manager {
        PackageManager::Apt => fields.nth(1)?,
        PackageManager::Zypper => fields.nth(4)?,
        _ => fields.next()?,
    };
    Some(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const APT: &str = include_str!("../../fixtures/updates/apt.txt");
    const DNF: &str = include_str!("../../fixtures/updates/dnf.txt");

    #[test]
    fn apt_fixture_counts_pending_and_security() {
        let snapshot = updates_snapshot(APT).expect("parse");
        assert_eq!(snapshot.manager, PackageManager::Apt);
        assert_eq!(snapshot.pending, 3);
        assert_eq!(snapshot.security, 1);
        assert!(snapshot.reboot_required);
        assert_eq!(snapshot.packages[0], "openssl");
    }

    #[test]
    fn dnf_fixture_lists_packages() {
        let snapshot = updates_snapshot(DNF).expect("parse");
        assert_eq!(snapshot.manager, PackageManager::Dnf);
        assert_eq!(snapshot.pending, 2);
        assert!(!snapshot.reboot_required);
        assert_eq!(
            snapshot.packages,
            vec!["kernel.x86_64", "vim-enhanced.x86_64"]
        );
    }

    #[test]
    fn unknown_manager_is_parse_error() {
        assert!(matches!(
            updates_snapshot("###manager\n\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
