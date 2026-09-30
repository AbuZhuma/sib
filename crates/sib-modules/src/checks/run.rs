use sib_core::{CheckKind, CustomCheck, ModuleError, SudoMode, Transport};

use super::model::CheckResult;
use super::{parse, script};

const SUDO_MISSING: &str = "sudo не настроен для этого сервера";

pub async fn all(
    transport: &dyn Transport,
    checks: &[CustomCheck],
) -> Result<Vec<CheckResult>, ModuleError> {
    let mut results = Vec::new();
    for as_root in [false, true] {
        let group: Vec<&CustomCheck> = checks.iter().filter(|c| c.as_root == as_root).collect();
        if group.is_empty() {
            continue;
        }
        if as_root && transport.sudo_mode() == SudoMode::None {
            results.extend(group.iter().map(|c| CheckResult::skipped(c, SUDO_MISSING)));
            continue;
        }
        results.extend(group_results(transport, &group, as_root).await?);
    }
    Ok(results)
}

async fn group_results(
    transport: &dyn Transport,
    group: &[&CustomCheck],
    as_root: bool,
) -> Result<Vec<CheckResult>, ModuleError> {
    let mut bodies = Vec::new();
    let mut skipped = Vec::new();
    for check in group {
        match body(check).await {
            Ok(body) => bodies.push(body),
            Err(reason) => {
                bodies.push("true".to_owned());
                skipped.push((check.id.clone(), reason));
            }
        }
    }
    let command = script::build(&bodies);
    let output = match as_root {
        true => transport.exec_root(&command).await?,
        false => transport.exec(&command).await?,
    };
    let mut results = parse::results(&output.stdout, group);
    for result in &mut results {
        if let Some((_, reason)) = skipped.iter().find(|(id, _)| id == &result.check.id) {
            *result = CheckResult::skipped(&result.check, reason);
        }
    }
    Ok(results)
}

async fn body(check: &CustomCheck) -> Result<String, String> {
    if check.kind != CheckKind::LocalFile {
        return Ok(script::body(check, None));
    }
    let path = check.source.trim();
    match tokio::fs::read_to_string(path).await {
        Ok(text) => Ok(text),
        Err(error) => Err(format!("файл {path} не прочитан: {error}")),
    }
}
