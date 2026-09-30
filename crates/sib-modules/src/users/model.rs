#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub user: String,
    pub tty: String,
    pub since: String,
    pub from: Option<String>,
}

impl Session {
    pub fn same(&self, other: &Self) -> bool {
        self.user == other.user && self.tty == other.tty && self.since == other.since
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRecord {
    pub user: String,
    pub tty: String,
    pub from: String,
    pub when: String,
    pub still_logged_in: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub name: String,
    pub uid: u32,
    pub home: String,
    pub shell: String,
    pub is_sudoer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsersSnapshot {
    pub sessions: Vec<Session>,
    pub logins: Vec<LoginRecord>,
    pub accounts: Vec<Account>,
    pub authorized_keys: Vec<(String, u32)>,
}

impl UsersSnapshot {
    pub fn sudoers(&self) -> impl Iterator<Item = &Account> {
        self.accounts.iter().filter(|a| a.is_sudoer)
    }
}
