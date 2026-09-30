use std::process::{Command, Stdio};

use sib_core::{ServerId, ServerSpec};

use super::SibApp;
use crate::shell::Notice;
use crate::text;

const TERMINALS: [(&str, &[&str]); 11] = [
    ("x-terminal-emulator", &["-e"]),
    ("ptyxis", &["--new-window", "--"]),
    ("gnome-terminal", &["--"]),
    ("konsole", &["-e"]),
    ("tilix", &["-e"]),
    ("wezterm", &["start", "--"]),
    ("xfce4-terminal", &["-x"]),
    ("alacritty", &["-e"]),
    ("kitty", &[]),
    ("foot", &[]),
    ("xterm", &["-e"]),
];

impl SibApp {
    pub(super) fn open_server_file(&mut self, id: &ServerId) {
        let path = self
            .config
            .servers_dir(&self.paths)
            .join(format!("{id}.md"));
        let result = Command::new("xdg-open")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Err(error) = result {
            self.notices.push(Notice::new(format!(
                "{} {}: {error}",
                text::ERR_OPEN_FAILED,
                path.display()
            )));
        }
    }

    pub(super) fn open_terminal(&mut self, id: &ServerId) {
        let spec = self
            .state
            .read()
            .ok()
            .and_then(|state| state.servers.get(id).map(|s| s.spec.clone()));
        let Some(spec) = spec else {
            return;
        };
        let ssh = ssh_arguments(&spec);
        let launched = terminal_candidates()
            .into_iter()
            .any(|(binary, prefix)| launch(&binary, prefix, &ssh));
        if !launched {
            self.notices
                .push(Notice::new(text::ERR_NO_TERMINAL.to_owned()));
        }
    }
}

fn ssh_arguments(spec: &ServerSpec) -> Vec<String> {
    let mut args = vec!["ssh".to_owned(), "-p".to_owned(), spec.port.to_string()];
    if let Some(jump) = &spec.jump {
        args.push("-J".to_owned());
        args.push(format!("{}@{}:{}", jump.user, jump.host, jump.port));
    }
    if let sib_core::AuthMethod::KeyFile { path, .. } = &spec.auth {
        args.push("-i".to_owned());
        args.push(path.display().to_string());
    }
    args.push(format!("{}@{}", spec.user, spec.host));
    args
}

fn terminal_candidates() -> Vec<(String, &'static [&'static str])> {
    let mut candidates = Vec::new();
    if let Ok(preferred) = std::env::var("TERMINAL")
        && !preferred.trim().is_empty()
    {
        candidates.push((preferred, &["-e"][..]));
    }
    candidates.extend(
        TERMINALS
            .iter()
            .map(|(binary, prefix)| ((*binary).to_owned(), *prefix)),
    );
    candidates
}

fn launch(binary: &str, prefix: &[&str], ssh: &[String]) -> bool {
    Command::new(binary)
        .args(prefix)
        .args(ssh)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sib_core::{AuthMethod, JumpHost, ServerDescription, SudoMode};

    #[test]
    fn ssh_arguments_include_port_jump_and_key() {
        let spec = ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "neo.example".into(),
            port: 2222,
            user: "deploy".into(),
            auth: AuthMethod::KeyFile {
                path: "/home/u/.ssh/neo".into(),
                has_passphrase: false,
            },
            jump: Some(JumpHost {
                host: "bastion".into(),
                port: 22,
                user: "j".into(),
            }),
            sudo: SudoMode::None,
            description: ServerDescription::default(),
            location: None,
            modules: Default::default(),
            checks: Vec::new(),
            check_overrides: Default::default(),
        };
        let args = ssh_arguments(&spec);
        assert_eq!(
            args,
            vec![
                "ssh",
                "-p",
                "2222",
                "-J",
                "j@bastion:22",
                "-i",
                "/home/u/.ssh/neo",
                "deploy@neo.example"
            ]
        );
    }
}
