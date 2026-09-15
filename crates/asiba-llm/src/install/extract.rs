use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::error::LlmError;

const KEEP_EXACT: [&str; 2] = ["llama-server", "libllama-server-impl.so"];
const KEEP_PREFIXES: [&str; 5] = [
    "libllama.so",
    "libllama-common.so",
    "libmtmd.so",
    "libggml.so",
    "libggml-base.so",
];
const KEEP_BACKEND_PREFIX: &str = "libggml-cpu-";

fn is_wanted(name: &str) -> bool {
    KEEP_EXACT.contains(&name)
        || KEEP_PREFIXES.iter().any(|p| name.starts_with(p))
        || name.starts_with(KEEP_BACKEND_PREFIX)
}

pub fn unpack_server(archive: &Path, into: &Path) -> Result<(), LlmError> {
    let file = File::open(archive)?;
    let mut tar = Archive::new(GzDecoder::new(file));
    let mut found_server = false;
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path().map_err(|e| LlmError::Extract(e.to_string()))?;
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        if !is_wanted(&name) {
            continue;
        }
        let target = into.join(&name);
        let _ = std::fs::remove_file(&target);
        entry
            .unpack(&target)
            .map_err(|e| LlmError::Extract(format!("{name}: {e}")))?;
        found_server |= name == "llama-server";
    }
    if !found_server {
        return Err(LlmError::Extract(
            "llama-server not found in archive".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_wanted_keeps_server_and_runtime_libraries_only() {
        assert!(is_wanted("llama-server"));
        assert!(is_wanted("libggml-cpu-alderlake.so"));
        assert!(is_wanted("libllama.so.0.4.1"));
        assert!(!is_wanted("llama-cli"));
        assert!(!is_wanted("libllama-cli-impl.so"));
        assert!(!is_wanted("LICENSE"));
    }
}
