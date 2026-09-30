#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
    Symlink { is_directory: bool },
    Other,
}

impl EntryKind {
    pub fn is_directory(self) -> bool {
        matches!(self, Self::Directory | Self::Symlink { is_directory: true })
    }

    pub fn is_file(self) -> bool {
        matches!(
            self,
            Self::File
                | Self::Symlink {
                    is_directory: false
                }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub mode: u32,
    pub owner: String,
    pub group: String,
    pub size: u64,
    pub modified_at: i64,
    pub link_target: Option<String>,
}

impl Entry {
    pub fn mode_octal(&self) -> String {
        format!("{:o}", self.mode)
    }

    pub fn mode_symbolic(&self) -> String {
        let mut text = String::with_capacity(9);
        for shift in [6, 3, 0] {
            let bits = (self.mode >> shift) & 0o7;
            text.push(if bits & 0o4 != 0 { 'r' } else { '-' });
            text.push(if bits & 0o2 != 0 { 'w' } else { '-' });
            text.push(if bits & 0o1 != 0 { 'x' } else { '-' });
        }
        text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub is_truncated: bool,
}

pub fn join_path(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}

pub fn parent_path(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) | None => "/",
        Some(index) => &trimmed[..index],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_symbolic_renders_rwx_triplets() {
        let entry = Entry {
            name: "x".into(),
            kind: EntryKind::File,
            mode: 0o754,
            owner: String::new(),
            group: String::new(),
            size: 0,
            modified_at: 0,
            link_target: None,
        };
        assert_eq!(entry.mode_symbolic(), "rwxr-xr--");
        assert_eq!(entry.mode_octal(), "754");
    }

    #[test]
    fn join_path_handles_root_and_nested() {
        assert_eq!(join_path("/", "etc"), "/etc");
        assert_eq!(join_path("/etc", "hosts"), "/etc/hosts");
    }

    #[test]
    fn parent_path_of_top_level_is_root() {
        assert_eq!(parent_path("/etc"), "/");
        assert_eq!(parent_path("/etc/ssh/"), "/etc");
        assert_eq!(parent_path("/"), "/");
    }
}
