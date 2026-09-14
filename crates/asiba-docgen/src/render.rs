use asiba_core::ServerState;
use chrono::Local;

use crate::sections;

pub fn render(server: &ServerState) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n{{{{notes}}}}\n\n", server.spec.id));
    sections::description(&mut out, server);
    sections::system(&mut out, server);
    sections::projects(&mut out, server);
    sections::processes(&mut out, server);
    sections::services(&mut out, server);
    sections::docker(&mut out, server);
    sections::ports(&mut out, server);
    sections::problems(&mut out, server);
    out.push_str(&format!(
        "_Обновлено: {}_\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    out
}

pub fn strip_timestamp(text: &str) -> &str {
    text.rfind("_Обновлено:")
        .map(|index| &text[..index])
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use asiba_core::{AuthMethod, ServerDescription, ServerId, ServerSpec, SudoMode};

    use super::*;

    fn server() -> ServerState {
        ServerState::new(ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "h".into(),
            port: 22,
            user: "u".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription {
                project: "Shop".into(),
                ..Default::default()
            },
            location: None,
            modules: Default::default(),
        })
    }

    #[test]
    fn render_without_snapshots_has_title_and_description() {
        let text = render(&server());
        assert!(text.starts_with("# neo\n\n{{notes}}\n\n## Описание\n"));
        assert!(text.contains("Проект: Shop"));
        assert!(!text.contains("## Docker"));
    }

    #[test]
    fn strip_timestamp_removes_trailing_line() {
        let text = render(&server());
        assert!(!strip_timestamp(&text).contains("_Обновлено"));
    }
}
