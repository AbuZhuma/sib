use asiba_core::transport::shell_quote;

pub const SCAN_DIRS: &str = "/opt /srv /var/www /home/* /root /app /docker /data";
pub const SCAN_DEPTH: u32 = 3;
pub const COMMITS_PER_REPOSITORY: u32 = 100;
pub const HISTORY_COMMITS: u32 = 2000;
pub const MAX_CHANGED_FILES: u32 = 200;
pub const MAX_BRANCHES: u32 = 50;
pub const MAX_TAGS: u32 = 20;

pub const REPOSITORY_MARKER: &str = "@@@repo ";
pub const FIELD_MARKER: &str = "@@@";
pub const LOG_FORMAT: &str = "%H%x09%an%x09%ct%x09%D%x09%s";

const ENVIRONMENT: &str = "export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*' GIT_OPTIONAL_LOCKS=0 GIT_TERMINAL_PROMPT=0";

pub fn find_repositories() -> String {
    format!(
        "for d in {SCAN_DIRS}; do find \"$d\" -maxdepth {SCAN_DEPTH} -name .git -type d 2>/dev/null; done | sort -u"
    )
}

pub fn collect() -> String {
    let fields = [
        ("branch", "rev-parse --abbrev-ref HEAD".to_owned()),
        ("head", "rev-parse HEAD".to_owned()),
        ("remote", "remote get-url origin".to_owned()),
        (
            "upstream",
            "rev-list --left-right --count 'HEAD...@{upstream}'".to_owned(),
        ),
        (
            "status",
            format!("status --porcelain | head -{MAX_CHANGED_FILES}"),
        ),
        ("stash", "stash list | wc -l".to_owned()),
        (
            "branches",
            format!("branch --format='%(refname:short)' | head -{MAX_BRANCHES}"),
        ),
        (
            "tags",
            format!("tag --sort=-creatordate | head -{MAX_TAGS}"),
        ),
        (
            "log",
            format!("log -{COMMITS_PER_REPOSITORY} --format='{LOG_FORMAT}'"),
        ),
    ];
    let body = fields
        .iter()
        .map(|(name, command)| {
            format!("echo '{FIELD_MARKER}{name}'; ( git -C \"$r\" {command} ) 2>/dev/null")
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "{ENVIRONMENT}; {} | while read -r g; do r=\"${{g%/.git}}\"; echo \"{REPOSITORY_MARKER}$r\"; {body}; done; true",
        find_repositories()
    )
}

pub fn history(path: &str) -> String {
    let quoted = shell_quote(path);
    format!(
        "{ENVIRONMENT}; git -C {quoted} log -{HISTORY_COMMITS} --format='{LOG_FORMAT}' 2>/dev/null"
    )
}
