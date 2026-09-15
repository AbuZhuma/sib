mod model;
mod test;

use asiba_core::{Environment, ServerId, ServerSpec, SudoMode};
use asiba_engine::{TestReport, TestRequest};
use asiba_transport::HostKeyPolicy;
use egui::{ComboBox, Grid, RichText, ScrollArea, TextEdit, Ui};

use super::servers::environment_label;
use super::{Action, Page};
use crate::components::{chip_value, field, page_title, panel, password_field, section_label};
use crate::modules::ModuleView;
use crate::text;
use crate::theme::{FIELD_WIDTH, GAP, Palette};
use model::{AuthChoice, FormFields};
use test::{TestAction, TestState};

pub struct ServerForm {
    fields: FormFields,
    editing: Option<ServerId>,
    error: Option<String>,
    test: TestState,
    next_token: u64,
}

impl ServerForm {
    pub fn new() -> Self {
        Self {
            fields: FormFields::empty(),
            editing: None,
            error: None,
            test: TestState::Idle,
            next_token: 1,
        }
    }

    pub fn edit(spec: &ServerSpec) -> Self {
        Self {
            fields: FormFields::from_spec(spec),
            editing: Some(spec.id.clone()),
            ..Self::new()
        }
    }

    pub fn accept_report(&mut self, report: TestReport) {
        self.test.accept(report);
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        existing: &[ServerId],
        views: &[Box<dyn ModuleView>],
    ) -> Option<Action> {
        let title = if self.editing.is_some() {
            text::FORM_TITLE_EDIT
        } else {
            text::FORM_TITLE_ADD
        };
        page_title(ui, title);
        let mut action = None;
        ScrollArea::vertical().show(ui, |ui| {
            panel(ui, text::FORM_SECTION_CONNECTION, |ui| {
                self.connection_fields(ui)
            });
            ui.add_space(GAP);
            panel(ui, text::FORM_SECTION_DESCRIPTION, |ui| {
                self.description_fields(ui)
            });
            ui.add_space(GAP);
            panel(ui, text::FORM_SECTION_TEST, |ui| {
                if let Some(TestAction::Run(policy)) = test::show(ui, &self.test, views) {
                    action = self.start_test(existing, policy);
                }
            });
            ui.add_space(GAP);
            if let Some(next) = self.footer(ui, existing) {
                action = Some(next);
            }
        });
        action
    }

    fn connection_fields(&mut self, ui: &mut Ui) {
        let f = &mut self.fields;
        Grid::new("form-connection")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                field(ui, text::FORM_NAME, &mut f.name, text::FORM_NAME_HINT);
                field(ui, text::FORM_HOST, &mut f.host, "");
                field(ui, text::FORM_PORT, &mut f.port, "22");
                field(ui, text::FORM_USER, &mut f.user, "");
            });
        section_label(ui, text::FORM_SECTION_AUTH);
        ui.horizontal(|ui| {
            chip_value(ui, &mut f.auth, AuthChoice::Auto, text::FORM_AUTH_AUTO);
            chip_value(ui, &mut f.auth, AuthChoice::KeyFile, text::FORM_AUTH_KEY);
            chip_value(
                ui,
                &mut f.auth,
                AuthChoice::Password,
                text::FORM_AUTH_PASSWORD,
            );
        });
        let secret_hint = if self.editing.is_some() {
            text::FORM_SECRETS_KEPT
        } else {
            ""
        };
        Grid::new("form-auth")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| match f.auth {
                AuthChoice::Auto => {}
                AuthChoice::KeyFile => {
                    field(
                        ui,
                        text::FORM_KEY_PATH,
                        &mut f.key_path,
                        "~/.ssh/id_ed25519",
                    );
                    password_field(ui, text::FORM_PASSPHRASE, &mut f.passphrase, secret_hint);
                }
                AuthChoice::Password => {
                    password_field(ui, text::FORM_PASSWORD, &mut f.password, secret_hint)
                }
            });
        section_label(ui, text::FORM_SECTION_SUDO);
        ui.horizontal(|ui| {
            chip_value(ui, &mut f.sudo, SudoMode::None, text::FORM_SUDO_NONE);
            chip_value(
                ui,
                &mut f.sudo,
                SudoMode::Passwordless,
                text::FORM_SUDO_PASSWORDLESS,
            );
            chip_value(
                ui,
                &mut f.sudo,
                SudoMode::WithPassword,
                text::FORM_SUDO_PASSWORD,
            );
        });
        if f.sudo == SudoMode::WithPassword {
            Grid::new("form-sudo")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    password_field(
                        ui,
                        text::FORM_SUDO_PASSWORD_FIELD,
                        &mut f.sudo_password,
                        secret_hint,
                    );
                });
        }
        section_label(ui, text::FORM_SECTION_JUMP);
        ui.checkbox(&mut f.jump_enabled, text::FORM_JUMP_ENABLE);
        if f.jump_enabled {
            Grid::new("form-jump")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    field(ui, text::FORM_HOST, &mut f.jump_host, "");
                    field(ui, text::FORM_PORT, &mut f.jump_port, "22");
                    field(ui, text::FORM_USER, &mut f.jump_user, "");
                });
        }
    }

    fn description_fields(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let f = &mut self.fields;
        Grid::new("form-description")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                field(ui, text::FORM_PROJECT, &mut f.project, "");
                field(ui, text::FORM_PURPOSE, &mut f.purpose, "");
                ui.label(RichText::new(text::FORM_ENVIRONMENT).color(p.text_secondary));
                environment_picker(ui, &mut f.environment);
                ui.end_row();
                field(ui, text::FORM_TAGS, &mut f.tags, "");
                field(ui, text::FORM_OWNER, &mut f.owner, "");
                ui.label(RichText::new(text::FORM_LINKS).color(p.text_secondary));
                ui.add(
                    TextEdit::multiline(&mut f.links)
                        .desired_width(FIELD_WIDTH)
                        .desired_rows(2),
                );
                ui.end_row();
                ui.label(RichText::new(text::FORM_NOTES).color(p.text_secondary));
                ui.add(
                    TextEdit::multiline(&mut f.notes)
                        .desired_width(FIELD_WIDTH)
                        .desired_rows(3),
                );
                ui.end_row();
                field(
                    ui,
                    text::FORM_LOCATION,
                    &mut f.location,
                    text::FORM_LOCATION_HINT,
                );
                field(ui, text::FORM_LOCATION_LABEL, &mut f.location_label, "");
            });
    }

    fn footer(&mut self, ui: &mut Ui, existing: &[ServerId]) -> Option<Action> {
        let p = Palette::current(ui.ctx());
        let mut action = None;
        ui.horizontal(|ui| {
            if ui.button(text::BTN_SAVE).clicked() {
                action = self.save(existing);
            }
            if ui.button(text::BTN_CANCEL).clicked() {
                action = Some(Action::Navigate(Page::Servers));
            }
            if let Some(error) = &self.error {
                ui.label(RichText::new(error).color(p.critical));
            }
        });
        action
    }

    fn save(&mut self, existing: &[ServerId]) -> Option<Action> {
        match self.fields.build(existing, self.editing.as_ref()) {
            Ok((spec, credentials)) => {
                self.error = None;
                Some(Action::SaveServer {
                    spec,
                    credentials,
                    is_new: self.editing.is_none(),
                })
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    fn start_test(&mut self, existing: &[ServerId], policy: HostKeyPolicy) -> Option<Action> {
        let (spec, credentials) = match self.fields.build(existing, self.editing.as_ref()) {
            Ok(built) => built,
            Err(error) => {
                self.error = Some(error);
                return None;
            }
        };
        self.error = None;
        let token = self.next_token;
        self.next_token += 1;
        self.test = TestState::Running {
            token,
            started: std::time::Instant::now(),
        };
        Some(Action::TestConnection(TestRequest {
            token,
            spec,
            credentials,
            policy,
            probe_module: asiba_modules::system::ID,
        }))
    }
}

fn environment_picker(ui: &mut Ui, value: &mut Environment) {
    let options = [
        Environment::Production,
        Environment::Staging,
        Environment::Development,
        Environment::Other,
    ];
    ComboBox::from_id_salt("form-environment")
        .selected_text(environment_label(*value))
        .show_ui(ui, |ui| {
            for option in options {
                chip_value(ui, value, option, environment_label(option));
            }
        });
}
