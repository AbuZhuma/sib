mod download;
mod extract;

use std::path::{Path, PathBuf};

use crate::error::LlmError;

pub const LLAMA_BUILD: &str = "b10976";
pub const MODEL_FILE: &str = "Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
pub const MODEL_SIZE_BYTES: u64 = 2_497_281_120;
const SERVER_BINARY: &str = "llama-server";

fn llama_url() -> String {
    format!(
        "https://github.com/ggml-org/llama.cpp/releases/download/{LLAMA_BUILD}/llama-{LLAMA_BUILD}-bin-ubuntu-x64.tar.gz"
    )
}

fn model_url() -> String {
    format!("https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/main/{MODEL_FILE}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStep {
    LlamaDownload,
    LlamaExtract,
    ModelDownload,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallProgress {
    pub step: InstallStep,
    pub done_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallTarget {
    pub llama_dir: PathBuf,
    pub models_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub server_binary: PathBuf,
    pub model_path: PathBuf,
}

impl InstallTarget {
    pub fn server_binary(&self) -> PathBuf {
        self.llama_dir.join(SERVER_BINARY)
    }

    pub fn model_path(&self) -> PathBuf {
        self.models_dir.join(MODEL_FILE)
    }
}

pub fn is_installed(target: &InstallTarget) -> bool {
    target.server_binary().is_file() && is_model_complete(&target.model_path())
}

fn is_model_complete(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.len() == MODEL_SIZE_BYTES)
}

pub fn install(
    target: &InstallTarget,
    progress: &dyn Fn(InstallProgress),
) -> Result<Installed, LlmError> {
    std::fs::create_dir_all(&target.llama_dir)?;
    std::fs::create_dir_all(&target.models_dir)?;
    if !target.server_binary().is_file() {
        let archive = target.llama_dir.join("llama.tar.gz");
        download::fetch(&llama_url(), &archive, InstallStep::LlamaDownload, progress)?;
        progress(InstallProgress {
            step: InstallStep::LlamaExtract,
            done_bytes: 0,
            total_bytes: None,
        });
        extract::unpack_server(&archive, &target.llama_dir)?;
        let _ = std::fs::remove_file(&archive);
    }
    let model_path = target.model_path();
    if !is_model_complete(&model_path) {
        download::fetch(
            &model_url(),
            &model_path,
            InstallStep::ModelDownload,
            progress,
        )?;
    }
    progress(InstallProgress {
        step: InstallStep::Done,
        done_bytes: MODEL_SIZE_BYTES,
        total_bytes: Some(MODEL_SIZE_BYTES),
    });
    Ok(Installed {
        server_binary: target.server_binary(),
        model_path,
    })
}
