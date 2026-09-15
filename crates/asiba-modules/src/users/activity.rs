use asiba_core::transport::shell_quote;
use asiba_core::{ModuleError, QueryResponse, Transport};

use crate::common::root::exec_prefer_root;
use crate::common::sections::{Sections, script};

pub const QUERY_ACTIVITY: &str = "activity";
pub const COMMAND_PREFIX: &str = "$ ";
pub const HEADING_PREFIX: &str = "## ";
const HISTORY_LINES: usize = 300;
const JOURNAL_LINES: usize = 200;
const SUDO_SINCE: &str = "-30d";
const NOTE: &str = "Результаты команд ОС не записывает - доступны сами команды (история shell, вызовы sudo) и записи журнала от процессов пользователя.";

fn activity_script(user: &str) -> String {
    let quoted = shell_quote(user);
    let home = format!("h=$(getent passwd {quoted} | cut -d: -f6)");
    let uid = format!("u=$(id -u {quoted})");
    let parts = [
        ("passwd", format!("getent passwd {quoted}")),
        (
            "bash",
            format!("{home}; tail -n {HISTORY_LINES} \"$h/.bash_history\""),
        ),
        (
            "zsh",
            format!("{home}; tail -n {HISTORY_LINES} \"$h/.zsh_history\""),
        ),
        (
            "sudo",
            format!(
                "journalctl -t sudo -o short-iso --no-pager -q --since {SUDO_SINCE} | grep -F ' {user} : '"
            ),
        ),
        (
            "journal",
            format!("{uid}; journalctl _UID=\"$u\" -o short-iso --no-pager -q -n {JOURNAL_LINES}"),
        ),
    ];
    let borrowed: Vec<(&str, &str)> = parts.iter().map(|(n, c)| (*n, c.as_str())).collect();
    script(&borrowed)
}

pub async fn query(transport: &dyn Transport, user: &str) -> Result<QueryResponse, ModuleError> {
    if user.is_empty()
        || !user
            .chars()
            .all(|c| c.is_alphanumeric() || "._-".contains(c))
    {
        return Err(ModuleError::UnsupportedQuery(format!("activity {user}")));
    }
    let output = exec_prefer_root(transport, &activity_script(user)).await?;
    let sections = Sections::parse(&output.stdout);
    Ok(QueryResponse {
        title: format!("activity {user}"),
        text: render(user, &sections),
    })
}

pub fn render(user: &str, sections: &Sections) -> String {
    let mut out = String::new();
    out.push_str(NOTE);
    out.push_str("\n\n");
    let bash: Vec<String> = sections
        .get_or_empty("bash")
        .lines()
        .map(str::to_owned)
        .collect();
    let zsh: Vec<String> = sections
        .get_or_empty("zsh")
        .lines()
        .filter_map(zsh_command)
        .collect();
    push_commands(&mut out, "История команд (bash)", &bash);
    push_commands(&mut out, "История команд (zsh)", &zsh);
    let sudo: Vec<String> = sections
        .get_or_empty("sudo")
        .lines()
        .filter_map(sudo_line)
        .collect();
    push_commands(&mut out, "Команды через sudo (журнал, 30 дней)", &sudo);
    let journal = sections.get_or_empty("journal");
    if !journal.trim().is_empty() {
        out.push_str(&format!(
            "{HEADING_PREFIX}Журнал процессов пользователя {user}\n{journal}\n"
        ));
    }
    out
}

fn push_commands(out: &mut String, title: &str, commands: &[String]) {
    if commands.is_empty() {
        return;
    }
    out.push_str(&format!("{HEADING_PREFIX}{title}\n"));
    for command in commands {
        out.push_str(COMMAND_PREFIX);
        out.push_str(command);
        out.push('\n');
    }
    out.push('\n');
}

fn zsh_command(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let command = trimmed
        .strip_prefix(": ")
        .and_then(|rest| rest.split_once(';').map(|(_, c)| c))
        .unwrap_or(trimmed);
    Some(command.to_owned())
}

fn sudo_line(line: &str) -> Option<String> {
    let (time, rest) = line.split_once(' ')?;
    let command = rest.split("COMMAND=").nth(1)?.trim();
    let pwd = rest
        .split("PWD=")
        .nth(1)
        .and_then(|s| s.split(" ;").next())
        .unwrap_or("?");
    Some(format!("sudo {command}    ({time}, pwd {pwd})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_extended_history_strips_timestamp() {
        assert_eq!(
            zsh_command(": 1700000000:0;ls -la").as_deref(),
            Some("ls -la")
        );
        assert_eq!(zsh_command("plain").as_deref(), Some("plain"));
    }

    #[test]
    fn sudo_journal_line_becomes_command_with_context() {
        let line = "2026-09-15T10:00:00+0600 host sudo[12]:   sander : TTY=pts/1 ; PWD=/home/sander ; USER=root ; COMMAND=/usr/bin/dnf update";
        assert_eq!(
            sudo_line(line).as_deref(),
            Some("sudo /usr/bin/dnf update    (2026-09-15T10:00:00+0600, pwd /home/sander)")
        );
    }

    #[test]
    fn render_marks_commands_and_headings() {
        let raw = "###passwd\nsander:x:1000:1000::/home/sander:/bin/bash\n###bash\nls\ncat x\n###zsh\n###sudo\n###journal\n";
        let text = render("sander", &Sections::parse(raw));
        assert!(text.contains("## История команд (bash)\n$ ls\n$ cat x\n"));
        assert!(!text.contains("zsh"));
    }
}
