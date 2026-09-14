use std::collections::HashMap;

pub const MARKER: &str = "###";

#[derive(Debug, Default)]
pub struct Sections<'a> {
    parts: HashMap<&'a str, &'a str>,
}

impl<'a> Sections<'a> {
    pub fn parse(raw: &'a str) -> Self {
        let mut parts = HashMap::new();
        let mut name: Option<&'a str> = None;
        let mut start = 0;
        for (offset, line) in line_offsets(raw) {
            let Some(marker) = line.strip_prefix(MARKER) else {
                continue;
            };
            if let Some(current) = name {
                parts.insert(current, &raw[start..offset]);
            }
            name = Some(marker.trim());
            start = offset + line.len() + 1;
        }
        if let Some(current) = name {
            parts.insert(current, raw.get(start..).unwrap_or_default());
        }
        Self { parts }
    }

    pub fn get(&self, name: &str) -> Option<&'a str> {
        self.parts.get(name).map(|s| s.trim())
    }

    pub fn get_or_empty(&self, name: &str) -> &'a str {
        self.get(name).unwrap_or_default()
    }
}

pub fn script(sections: &[(&str, &str)]) -> String {
    sections
        .iter()
        .map(|(name, command)| format!("echo '{MARKER}{name}'; {command} 2>/dev/null"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn line_offsets(raw: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0;
    raw.split('\n').map(move |line| {
        let current = offset;
        offset += line.len() + 1;
        (current, line)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_sections_splits_by_marker() {
        let raw = "###a\nline1\nline2\n###b\nx\n";
        let sections = Sections::parse(raw);
        assert_eq!(sections.get("a"), Some("line1\nline2"));
        assert_eq!(sections.get("b"), Some("x"));
    }

    #[test]
    fn parse_empty_section_is_empty_string() {
        let sections = Sections::parse("###a\n###b\nv\n");
        assert_eq!(sections.get("a"), Some(""));
    }

    #[test]
    fn get_missing_section_is_none() {
        assert_eq!(Sections::parse("###a\n1\n").get("zzz"), None);
    }

    #[test]
    fn script_joins_commands_with_markers() {
        let s = script(&[("host", "hostname"), ("k", "uname -r")]);
        assert_eq!(
            s,
            "echo '###host'; hostname 2>/dev/null; echo '###k'; uname -r 2>/dev/null"
        );
    }
}
