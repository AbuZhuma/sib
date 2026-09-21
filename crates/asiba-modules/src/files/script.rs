use asiba_core::transport::shell_quote;

pub const MAX_ENTRIES: usize = 1000;
pub const MAX_SEARCH_RESULTS: usize = 200;
pub const SEARCH_TIMEOUT_SECS: u32 = 15;
const SEARCH_PRUNE: &str = "-path /proc -o -path /sys -o -path /dev -o -path /run";
const GLOB_CHARS: [char; 3] = ['*', '?', '['];
pub const MAX_FILE_BYTES: u64 = 200_000;
pub const BINARY_PROBE_BYTES: u32 = 8192;
pub const EXIT_UNREADABLE: i32 = 2;
pub const EXIT_TOO_LARGE: i32 = 3;
pub const EXIT_BINARY: i32 = 4;

const LIST_FORMAT: &str = "%y\\t%Y\\t%m\\t%u\\t%g\\t%s\\t%T@\\t%l\\t%f\\n";
const SEARCH_FORMAT: &str = "%y\\t%Y\\t%m\\t%u\\t%g\\t%s\\t%T@\\t%l\\t%p\\n";

pub const DETECT: &str = "find / -maxdepth 0 -printf '' && command -v stat";

pub fn list(directory: &str) -> String {
    let quoted = shell_quote(directory);
    format!("find {quoted} -mindepth 1 -maxdepth 1 -printf '{LIST_FORMAT}' | head -n {MAX_ENTRIES}")
}

pub fn search(pattern: &str) -> String {
    let glob = if pattern.chars().any(|c| GLOB_CHARS.contains(&c)) {
        pattern.to_owned()
    } else {
        format!("*{pattern}*")
    };
    format!(
        "timeout {SEARCH_TIMEOUT_SECS} nice -n 19 find / \\( {SEARCH_PRUNE} \\) -prune -o -iname {} -printf '{SEARCH_FORMAT}' 2>/dev/null | head -n {MAX_SEARCH_RESULTS}; true",
        shell_quote(&glob)
    )
}

pub fn read(path: &str) -> String {
    let quoted = shell_quote(path);
    format!(
        "f={quoted}; s=$(stat -c %s -- \"$f\") || exit {EXIT_UNREADABLE}; [ -r \"$f\" ] || exit {EXIT_UNREADABLE}; [ \"$s\" -le {MAX_FILE_BYTES} ] || exit {EXIT_TOO_LARGE}; [ \"$(head -c {BINARY_PROBE_BYTES} -- \"$f\" | tr -cd '\\000' | wc -c)\" -eq 0 ] || exit {EXIT_BINARY}; cat -- \"$f\""
    )
}

pub fn write(path: &str, content: &str) -> String {
    format!(
        "printf '%s' {} > {}",
        shell_quote(content),
        shell_quote(path)
    )
}

pub fn chmod(path: &str, mode: &str) -> String {
    format!("chmod {mode} -- {}", shell_quote(path))
}

pub fn chown(path: &str, owner: &str) -> String {
    format!("chown {} -- {}", shell_quote(owner), shell_quote(path))
}

pub fn make_directory(path: &str) -> String {
    format!("mkdir -- {}", shell_quote(path))
}

pub fn create_file(path: &str) -> String {
    let quoted = shell_quote(path);
    format!("[ ! -e {quoted} ] && : > {quoted}")
}

pub fn move_path(source: &str, destination: &str) -> String {
    format!(
        "[ ! -e {dst} ] && mv -- {src} {dst}",
        src = shell_quote(source),
        dst = shell_quote(destination)
    )
}

pub fn copy_path(source: &str, destination: &str) -> String {
    format!(
        "[ ! -e {dst} ] && cp -a -- {src} {dst}",
        src = shell_quote(source),
        dst = shell_quote(destination)
    )
}

pub fn delete(path: &str) -> String {
    format!("rm -rf -- {}", shell_quote(path))
}
