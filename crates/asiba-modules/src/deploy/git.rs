use chrono::DateTime;

use super::model::{Deploy, DeployStatus, Source, Stage};

const COMMITS_PER_REPO: u32 = 5;

pub fn script(scan_dirs: &str) -> String {
    format!(
        "for d in {scan_dirs}; do find \"$d\" -maxdepth 3 -name .git -type d 2>/dev/null; done | while read -r g; do r=\"${{g%/.git}}\"; b=$(git -C \"$r\" rev-parse --abbrev-ref HEAD 2>/dev/null); n=$(git -C \"$r\" status --porcelain 2>/dev/null | wc -l); git -C \"$r\" log -{COMMITS_PER_REPO} --format=\"$r%x09$b%x09$n%x09%H%x09%an%x09%ct%x09%s\" 2>/dev/null; done"
    )
}

pub fn deploys(raw: &str) -> Vec<Deploy> {
    raw.lines().filter_map(commit_line).collect()
}

fn commit_line(line: &str) -> Option<Deploy> {
    let fields: Vec<&str> = line.splitn(7, '\t').collect();
    if fields.len() < 7 {
        return None;
    }
    let (repo, branch, dirty, hash, author, timestamp, subject) = (
        fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[6],
    );
    let at = DateTime::from_timestamp(timestamp.parse().ok()?, 0)?;
    let short = hash.get(..7).unwrap_or(hash);
    let dirty: u32 = dirty.trim().parse().unwrap_or(0);
    let dirty_note = if dirty > 0 {
        format!(", {dirty} незакоммиченных")
    } else {
        String::new()
    };
    Some(Deploy {
        key: format!("git:{repo}:{hash}"),
        project: project_name(repo),
        source: Source::Git,
        detail: format!("{branch} {short} {author}: {subject}{dirty_note}"),
        started_at: at,
        finished_at: Some(at),
        stages: vec![Stage::done("commit", Some(at))],
        status: DeployStatus::Success,
        error: None,
        log_tail: Vec::new(),
    })
}

pub fn project_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_lines_become_git_deploys() {
        let raw = "/srv/shop\tmain\t2\tabcdef1234567\tAnna\t1757851200\tfix checkout\n/srv/shop\tmain\t2\t1234567abcdef\tBob\t1757764800\tinitial\n";
        let deploys = deploys(raw);
        assert_eq!(deploys.len(), 2);
        assert_eq!(deploys[0].project, "shop");
        assert_eq!(deploys[0].key, "git:/srv/shop:abcdef1234567");
        assert!(deploys[0].detail.contains("abcdef1"));
        assert!(deploys[0].detail.contains("2 незакоммиченных"));
        assert_eq!(deploys[0].status, DeployStatus::Success);
    }

    #[test]
    fn malformed_line_is_skipped() {
        assert!(deploys("garbage\n").is_empty());
    }
}
