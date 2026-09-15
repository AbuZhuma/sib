use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ureq::Agent;
use ureq::http::StatusCode;

use super::{InstallProgress, InstallStep};
use crate::error::LlmError;

const CHUNK: usize = 256 * 1024;
const REPORT_EVERY: u64 = 2 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);
const PART_SUFFIX: &str = ".part";

struct Transfer {
    file: File,
    done_bytes: u64,
    total_bytes: Option<u64>,
}

pub fn fetch(
    url: &str,
    destination: &Path,
    step: InstallStep,
    progress: &dyn Fn(InstallProgress),
) -> Result<(), LlmError> {
    let part = part_path(destination);
    let resume_from = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let agent: Agent = Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_recv_response(Some(RESPONSE_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("Range", &format!("bytes={resume_from}-"))
        .call()
        .map_err(|e| LlmError::Download(e.to_string()))?;
    let resumed = response.status() == StatusCode::PARTIAL_CONTENT;
    let content_length = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let mut transfer = open_part(&part, resumed, resume_from, content_length)?;
    let mut reader = response.body_mut().with_config().limit(u64::MAX).reader();
    copy(&mut reader, &mut transfer, step, progress)?;
    verify(&part, &transfer)?;
    std::fs::rename(&part, destination)?;
    Ok(())
}

fn open_part(
    part: &Path,
    resumed: bool,
    resume_from: u64,
    content_length: Option<u64>,
) -> Result<Transfer, LlmError> {
    if resumed {
        let file = OpenOptions::new().append(true).open(part)?;
        return Ok(Transfer {
            file,
            done_bytes: resume_from,
            total_bytes: content_length.map(|l| l + resume_from),
        });
    }
    Ok(Transfer {
        file: File::create(part)?,
        done_bytes: 0,
        total_bytes: content_length,
    })
}

fn copy(
    reader: &mut impl Read,
    transfer: &mut Transfer,
    step: InstallStep,
    progress: &dyn Fn(InstallProgress),
) -> Result<(), LlmError> {
    let mut reported = transfer.done_bytes;
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| LlmError::Download(e.to_string()))?;
        if read == 0 {
            break;
        }
        transfer.file.write_all(&buffer[..read])?;
        transfer.done_bytes += read as u64;
        if transfer.done_bytes - reported >= REPORT_EVERY {
            reported = transfer.done_bytes;
            progress(InstallProgress {
                step,
                done_bytes: transfer.done_bytes,
                total_bytes: transfer.total_bytes,
            });
        }
    }
    transfer.file.flush()?;
    Ok(())
}

fn verify(part: &Path, transfer: &Transfer) -> Result<(), LlmError> {
    let Some(total) = transfer.total_bytes else {
        return Ok(());
    };
    if total == transfer.done_bytes {
        return Ok(());
    }
    if transfer.done_bytes > total {
        let _ = std::fs::remove_file(part);
    }
    Err(LlmError::Download(format!(
        "incomplete download: {} of {total} bytes (retry to resume)",
        transfer.done_bytes
    )))
}

fn part_path(destination: &Path) -> PathBuf {
    let mut name = destination.as_os_str().to_owned();
    name.push(PART_SUFFIX);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_path_appends_suffix_to_full_name() {
        assert_eq!(
            part_path(Path::new("/m/model.gguf")),
            PathBuf::from("/m/model.gguf.part")
        );
    }
}
