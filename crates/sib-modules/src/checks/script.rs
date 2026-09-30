use sib_core::{CustomCheck, transport::shell_quote};

pub const START_MARKER: &str = "###sib-check";
pub const EXIT_MARKER: &str = "###sib-exit";
pub const TIMEOUT_SECS: u64 = 20;

pub fn body(check: &CustomCheck, local_script: Option<&str>) -> String {
    match check.kind {
        sib_core::CheckKind::RemoteFile => format!("sh {}", shell_quote(check.source.trim())),
        sib_core::CheckKind::LocalFile => local_script.unwrap_or_default().to_owned(),
        sib_core::CheckKind::Script => check.source.clone(),
    }
}

pub fn build(bodies: &[String]) -> String {
    bodies
        .iter()
        .enumerate()
        .map(|(index, body)| fragment(index, body))
        .collect::<Vec<_>>()
        .join("\n")
}

fn fragment(index: usize, body: &str) -> String {
    format!(
        "echo '{START_MARKER} {index}'; printf '%s' {} | timeout {TIMEOUT_SECS} sh 2>&1; echo \"{EXIT_MARKER} $?\"",
        shell_quote(body)
    )
}

#[cfg(test)]
mod tests {
    use sib_core::CheckKind;

    use super::*;

    #[test]
    fn remote_file_body_runs_the_path_through_sh() {
        let check = CustomCheck {
            kind: CheckKind::RemoteFile,
            source: "/opt/checks/disk.sh".to_owned(),
            ..CustomCheck::new("custom:1")
        };
        assert_eq!(body(&check, None), "sh '/opt/checks/disk.sh'");
    }

    #[test]
    fn script_body_is_taken_verbatim_and_quoted_once() {
        let check = CustomCheck {
            kind: CheckKind::Script,
            source: "test \"$(id -u)\" = '0'".to_owned(),
            ..CustomCheck::new("custom:1")
        };
        let script = build(&[body(&check, None)]);
        assert!(script.contains("'test \"$(id -u)\" = '\\''0'\\'''"));
        assert!(script.starts_with("echo '###sib-check 0'"));
        assert!(script.ends_with("echo \"###sib-exit $?\""));
    }

    #[test]
    fn build_joins_every_check_with_its_own_markers() {
        let script = build(&["true".to_owned(), "false".to_owned()]);
        assert_eq!(script.matches(START_MARKER).count(), 2);
        assert_eq!(script.matches(EXIT_MARKER).count(), 2);
    }
}
