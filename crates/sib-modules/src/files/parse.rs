use sib_core::ModuleError;

use super::model::{Entry, EntryKind, Listing};
use super::script::MAX_ENTRIES;

const FIELD_COUNT: usize = 9;
const OCTAL_RADIX: u32 = 8;

pub fn listing(raw: &str) -> Result<Listing, ModuleError> {
    let mut entries = Vec::new();
    for line in raw.lines().filter(|l| !l.is_empty()) {
        entries.push(entry(line)?);
    }
    let is_truncated = entries.len() >= MAX_ENTRIES;
    entries.sort_by(|a, b| {
        b.kind
            .is_directory()
            .cmp(&a.kind.is_directory())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(Listing {
        entries,
        is_truncated,
    })
}

fn entry(line: &str) -> Result<Entry, ModuleError> {
    let fields: Vec<&str> = line.splitn(FIELD_COUNT, '\t').collect();
    if fields.len() < FIELD_COUNT {
        return Err(ModuleError::Parse(format!("строка find: {line}")));
    }
    let kind = kind(fields[0], fields[1]);
    let mode = u32::from_str_radix(fields[2], OCTAL_RADIX)
        .map_err(|_| ModuleError::Parse(format!("права {}", fields[2])))?;
    let size = fields[5]
        .parse()
        .map_err(|_| ModuleError::Parse(format!("размер {}", fields[5])))?;
    let modified_at = fields[6]
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ModuleError::Parse(format!("время {}", fields[6])))?;
    let link_target = (!fields[7].is_empty()).then(|| fields[7].to_owned());
    Ok(Entry {
        name: fields[8].to_owned(),
        kind,
        mode,
        owner: fields[3].to_owned(),
        group: fields[4].to_owned(),
        size,
        modified_at,
        link_target,
    })
}

fn kind(own: &str, target: &str) -> EntryKind {
    match own {
        "d" => EntryKind::Directory,
        "f" => EntryKind::File,
        "l" => EntryKind::Symlink {
            is_directory: target == "d",
        },
        _ => EntryKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_fixture_puts_directories_first_sorted_by_name() {
        let listing = listing(include_str!("../../fixtures/files/fedora_root.txt")).expect("root");
        assert!(!listing.is_truncated);
        let names: Vec<&str> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names[0], "afs");
        assert_eq!(names[1], "bin");
        assert_eq!(names.last().copied(), Some(".profile"));
        let bin = listing
            .entries
            .iter()
            .find(|e| e.name == "bin")
            .expect("bin");
        assert_eq!(bin.kind, EntryKind::Symlink { is_directory: true });
        assert_eq!(bin.link_target.as_deref(), Some("usr/bin"));
        let tmp = listing
            .entries
            .iter()
            .find(|e| e.name == "tmp")
            .expect("tmp");
        assert_eq!(tmp.mode, 0o1777);
        assert_eq!(tmp.mode_octal(), "1777");
    }

    #[test]
    fn etc_fixture_parses_dangling_symlink_as_non_directory() {
        let listing = listing(include_str!("../../fixtures/files/fedora_etc.txt")).expect("etc");
        let grub = listing
            .entries
            .iter()
            .find(|e| e.name == "grub2.cfg")
            .expect("grub2.cfg");
        assert_eq!(
            grub.kind,
            EntryKind::Symlink {
                is_directory: false
            }
        );
        assert!(grub.kind.is_file());
    }

    #[test]
    fn names_with_spaces_and_fifo_are_kept() {
        let listing = listing(include_str!("../../fixtures/files/ubuntu_srv.txt")).expect("srv");
        let readme = listing
            .entries
            .iter()
            .find(|e| e.name == "README with space.md")
            .expect("readme");
        assert_eq!(readme.owner, "deploy");
        assert_eq!(readme.size, 0);
        let fifo = listing
            .entries
            .iter()
            .find(|e| e.name == "queue.fifo")
            .expect("fifo");
        assert_eq!(fifo.kind, EntryKind::Other);
        assert_eq!(listing.entries[0].name, "app");
    }

    #[test]
    fn malformed_line_is_parse_error() {
        assert!(listing("d\td\t755\n").is_err());
    }

    #[test]
    fn empty_output_is_empty_listing() {
        assert!(listing("").expect("empty").entries.is_empty());
    }
}
