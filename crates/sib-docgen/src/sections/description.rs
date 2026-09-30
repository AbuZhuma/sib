use sib_core::{Environment, Location};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, bullet, field, heading};

pub struct DescriptionSection;

fn environment_key(environment: Environment) -> &'static str {
    match environment {
        Environment::Production => "production",
        Environment::Staging => "staging",
        Environment::Development => "development",
        Environment::Other => "other",
    }
}

fn location_label(location: &Location) -> String {
    let mut label = location.label.clone();
    if !location.provider.is_empty() {
        label.push_str(&format!(" · {}", location.provider));
    }
    format!("{label} ({:.3}, {:.3})", location.lat, location.lon)
}

fn rows(ctx: &DocContext<'_>) -> Vec<(&'static str, &'static str, String)> {
    let spec = &ctx.server.spec;
    let d = &spec.description;
    let mut rows = vec![
        ("Project", "project", d.project.clone()),
        ("Purpose", "purpose", d.purpose.clone()),
        (
            "Environment",
            "environment",
            environment_key(d.environment).to_owned(),
        ),
        ("Owner", "owner", d.owner.clone()),
        ("Tags", "tags", d.tags.join(", ")),
        ("Links", "links", d.links.join(", ")),
        (
            "Address",
            "address",
            format!("{}@{}:{}", spec.user, spec.host, spec.port),
        ),
    ];
    if let Some(location) = &ctx.server.location {
        rows.push(("Location", "location", location_label(location)));
    }
    rows.into_iter().filter(|(_, _, v)| !v.is_empty()).collect()
}

impl Section for DescriptionSection {
    fn id(&self) -> SectionId {
        SectionId::Description
    }

    fn is_available(&self, _ctx: &DocContext<'_>) -> bool {
        true
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Description");
        for (label, _, value) in rows(ctx) {
            bullet(out, label, value);
        }
        let notes = &ctx.server.spec.description.notes;
        if !notes.is_empty() {
            blank(out);
            out.push_str(notes);
            out.push('\n');
        }
        blank(out);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Description");
        for (_, key, value) in rows(ctx) {
            field(out, key, value);
        }
        let notes = &ctx.server.spec.description.notes;
        if !notes.is_empty() {
            field(out, "notes", notes.replace('\n', " "));
        }
        blank(out);
    }
}

#[cfg(test)]
mod tests {
    use sib_core::AppState;

    use super::*;
    use crate::test_support::server;

    #[test]
    fn llm_description_skips_empty_fields() {
        let state = AppState::default();
        let server = server();
        let mut out = String::new();
        DescriptionSection.llm(&mut out, &DocContext::new(&server, &state));
        assert_eq!(
            out,
            "## Description\n\nproject: Shop\nenvironment: production\naddress: u@h:22\n\n"
        );
    }
}
