use chrono::{DateTime, Utc};

pub const SHORT_HASH_LEN: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub at: DateTime<Utc>,
    pub refs: Vec<String>,
    pub subject: String,
}

impl Commit {
    pub fn short_hash(&self) -> &str {
        self.hash.get(..SHORT_HASH_LEN).unwrap_or(&self.hash)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Upstream {
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub remote: Option<String>,
    pub upstream: Option<Upstream>,
    pub changed_files: Vec<String>,
    pub stashes: u32,
    pub branches: Vec<String>,
    pub tags: Vec<String>,
    pub commits: Vec<Commit>,
}

impl Repository {
    pub fn name(&self) -> &str {
        self.path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&self.path)
    }

    pub fn is_dirty(&self) -> bool {
        !self.changed_files.is_empty()
    }

    pub fn is_detached(&self) -> bool {
        self.head.is_some() && self.branch.is_none()
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn last_commit(&self) -> Option<&Commit> {
        self.commits.first()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSnapshot {
    pub version: String,
    pub repositories: Vec<Repository>,
}

impl GitSnapshot {
    pub fn dirty_count(&self) -> usize {
        self.repositories.iter().filter(|r| r.is_dirty()).count()
    }

    pub fn find(&self, path: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.path == path)
    }
}
