use sib_core::{AuthMethod, ServerDescription, ServerId, ServerSpec, ServerState, SudoMode};

pub fn server() -> ServerState {
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
        checks: Vec::new(),
        check_overrides: Default::default(),
        pipelines: Vec::new(),
    })
}
