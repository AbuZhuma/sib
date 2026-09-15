use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use ureq::Agent;

use super::{InstallProgress, InstallStep};
use crate::error::LlmError;

const CHUNK: usize = 256 * 1024;
const REPORT_EVERY: u64 = 2 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(60);
const PART_SUFFIX: &str = ".part";

pub fn fetch(
    url: &str,
    destination: &Path,
    step: InstallStep,
    progress: &dyn Fn(InstallProgress),
) -> Result<(), LlmError> {
    let agent: Agent = Agent::config_builder()
        .timeout_per_call(Some(TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| LlmError::Download(e.to_string()))?;
    let total_bytes = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let part = destination.with_extension(extension_with_part(destination));
    let mut file = File::create(&part)?;
    let mut reader = response.body_mut().with_config().limit(u64::MAX).reader();
    let mut done_bytes = 0u64;
    let mut reported = 0u64;
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| LlmError::Download(e.to_string()))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        done_bytes += read as u64;
        if done_bytes - reported >= REPORT_EVERY {
            reported = done_bytes;
            progress(InstallProgress {
                step,
                done_bytes,
                total_bytes,
            });
        }
    }
    file.flush()?;
    drop(file);
    if let Some(total) = total_bytes
        && total != done_bytes
    {
        let _ = std::fs::remove_file(&part);
        return Err(LlmError::Download(format!(
            "incomplete download: {done_bytes} of {total} bytes"
        )));
    }
    std::fs::rename(&part, destination)?;
    Ok(())
}

fn extension_with_part(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some(extension) => format!("{extension}{PART_SUFFIX}"),
        None => PART_SUFFIX.trim_start_matches('.').to_owned(),
    }
}
