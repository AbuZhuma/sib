#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    Apk,
}

impl PackageManager {
    pub fn label(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Yum => "yum",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
            Self::Apk => "apk",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatesSnapshot {
    pub manager: PackageManager,
    pub pending: u32,
    pub security: u32,
    pub reboot_required: bool,
    pub packages: Vec<String>,
}
