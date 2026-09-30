use sib_modules::users::{self, UsersSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, field, heading, list, subheading, table};

pub struct UsersSection;

const MAX_LOGINS: usize = 10;
const MAX_ACCOUNTS: usize = 30;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a UsersSnapshot> {
    ctx.server.data::<UsersSnapshot>(users::ID)
}

fn session_rows(snapshot: &UsersSnapshot) -> Vec<Vec<String>> {
    snapshot
        .sessions
        .iter()
        .map(|s| {
            vec![
                s.user.clone(),
                s.tty.clone(),
                s.from.clone().unwrap_or_else(|| "local".to_owned()),
                s.since.clone(),
            ]
        })
        .collect()
}

fn login_rows(snapshot: &UsersSnapshot) -> Vec<Vec<String>> {
    snapshot
        .logins
        .iter()
        .take(MAX_LOGINS)
        .map(|l| {
            let state = if l.still_logged_in {
                "active"
            } else {
                "closed"
            };
            vec![
                l.user.clone(),
                l.from.clone(),
                l.when.clone(),
                state.to_owned(),
            ]
        })
        .collect()
}

fn account_rows(snapshot: &UsersSnapshot) -> Vec<Vec<String>> {
    snapshot
        .accounts
        .iter()
        .take(MAX_ACCOUNTS)
        .map(|a| {
            let keys = snapshot
                .authorized_keys
                .iter()
                .find(|(name, _)| name == &a.name)
                .map(|(_, count)| count.to_string())
                .unwrap_or_else(|| NONE.to_owned());
            let sudo = if a.is_sudoer { "sudo" } else { "" };
            vec![
                a.name.clone(),
                a.uid.to_string(),
                a.shell.clone(),
                sudo.to_owned(),
                keys,
            ]
        })
        .collect()
}

impl Section for UsersSection {
    fn id(&self) -> SectionId {
        SectionId::Users
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Users");
        subheading(out, "Active sessions");
        table(
            out,
            &["User", "TTY", "From", "Since"],
            &session_rows(snapshot),
        );
        subheading(out, "Recent logins");
        table(
            out,
            &["User", "From", "When", "Session"],
            &login_rows(snapshot),
        );
        subheading(out, "Accounts with a shell");
        table(
            out,
            &["Name", "UID", "Shell", "Sudo", "Keys"],
            &account_rows(snapshot),
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Users");
        field(out, "active_sessions", "user | tty | from | since");
        list(out, "", &session_rows(snapshot));
        field(out, "recent_logins", "user | from | when | session");
        list(out, "", &login_rows(snapshot));
        field(
            out,
            "accounts_with_shell",
            "name | uid | shell | sudo | authorized_keys",
        );
        list(out, "", &account_rows(snapshot));
        blank(out);
    }
}
