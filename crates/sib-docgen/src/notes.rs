pub const NOTES_START: &str = "<!-- notes:start -->";
pub const NOTES_END: &str = "<!-- notes:end -->";
pub const DEFAULT_NOTES: &str = "\n";

pub fn merge_notes(existing: Option<&str>, rendered: &str) -> String {
    let notes = existing.and_then(extract_notes).unwrap_or(DEFAULT_NOTES);
    let block = format!("{NOTES_START}{notes}{NOTES_END}");
    rendered.replacen("{{notes}}", &block, 1)
}

fn extract_notes(text: &str) -> Option<&str> {
    let start = text.find(NOTES_START)? + NOTES_START.len();
    let end = text[start..].find(NOTES_END)? + start;
    Some(&text[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_without_existing_inserts_empty_block() {
        let result = merge_notes(None, "# x\n\n{{notes}}\n\nbody");
        assert_eq!(
            result,
            "# x\n\n<!-- notes:start -->\n<!-- notes:end -->\n\nbody"
        );
    }

    #[test]
    fn merge_keeps_manual_notes_from_existing_file() {
        let existing = "old\n<!-- notes:start -->\nмои заметки\n<!-- notes:end -->\nold body";
        let result = merge_notes(Some(existing), "{{notes}}\nnew body");
        assert!(result.contains("\nмои заметки\n"));
        assert!(result.ends_with("new body"));
    }
}
