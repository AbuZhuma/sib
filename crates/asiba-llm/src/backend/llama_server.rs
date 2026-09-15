use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ureq::Agent;

use super::{Backend, Completion, LOCALHOST};
use crate::error::LlmError;

const READY_TIMEOUT: Duration = Duration::from_secs(180);
const READY_POLL: Duration = Duration::from_millis(500);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(900);
const TEMPERATURE: f64 = 0.2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaConfig {
    pub binary: PathBuf,
    pub model: PathBuf,
    pub threads: usize,
    pub context_tokens: usize,
}

pub struct LlamaServer {
    child: Child,
    port: u16,
    model_name: String,
    agent: Agent,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: usize,
    temperature: f64,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

impl LlamaServer {
    pub fn start(config: &LlamaConfig) -> Result<Self, LlmError> {
        if !config.model.is_file() {
            return Err(LlmError::ModelNotFound(config.model.display().to_string()));
        }
        let port = free_port()?;
        let child = spawn(config, port)?;
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build()
            .into();
        let server = Self {
            child,
            port,
            model_name: model_name(&config.model),
            agent,
        };
        server.wait_ready()?;
        Ok(server)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    fn url(&self, path: &str) -> String {
        format!("http://{LOCALHOST}:{}{path}", self.port)
    }

    fn wait_ready(&self) -> Result<(), LlmError> {
        let started = Instant::now();
        while started.elapsed() < READY_TIMEOUT {
            if let Ok(response) = self.agent.get(&self.url("/health")).call()
                && response.status() == 200
            {
                return Ok(());
            }
            std::thread::sleep(READY_POLL);
        }
        Err(LlmError::NotReady(READY_TIMEOUT.as_secs()))
    }
}

impl Backend for LlamaServer {
    fn model_name(&self) -> String {
        self.model_name.clone()
    }

    fn complete(&self, request: &Completion) -> Result<String, LlmError> {
        let body = ChatRequest {
            model: &self.model_name,
            messages: vec![
                Message {
                    role: "system",
                    content: &request.system,
                },
                Message {
                    role: "user",
                    content: &request.user,
                },
            ],
            max_tokens: request.max_tokens,
            temperature: TEMPERATURE,
            stream: false,
        };
        let mut response = self
            .agent
            .post(&self.url("/v1/chat/completions"))
            .send_json(&body)
            .map_err(|e| LlmError::Request(e.to_string()))?;
        let parsed: ChatResponse = response
            .body_mut()
            .read_json()
            .map_err(|e| LlmError::Response(e.to_string()))?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_owned())
            .ok_or_else(|| LlmError::Response("no choices".to_owned()))
    }
}

impl Drop for LlamaServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> Result<u16, LlmError> {
    let listener = TcpListener::bind((LOCALHOST, 0))?;
    Ok(listener.local_addr()?.port())
}

fn spawn(config: &LlamaConfig, port: u16) -> Result<Child, LlmError> {
    let mut command = Command::new(&config.binary);
    command
        .arg("-m")
        .arg(&config.model)
        .arg("--host")
        .arg(LOCALHOST)
        .arg("--port")
        .arg(port.to_string())
        .arg("-c")
        .arg(config.context_tokens.to_string())
        .arg("-t")
        .arg(config.threads.to_string())
        .arg("--no-webui")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            LlmError::BinaryNotFound(config.binary.display().to_string())
        } else {
            LlmError::Spawn(error)
        }
    })
}

pub fn model_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "model".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_name_uses_file_stem() {
        assert_eq!(
            model_name(Path::new("/models/Qwen3-4B-Q4_K_M.gguf")),
            "Qwen3-4B-Q4_K_M"
        );
    }

    #[test]
    fn start_with_missing_model_fails_before_spawning() {
        let config = LlamaConfig {
            binary: PathBuf::from("llama-server"),
            model: PathBuf::from("/nonexistent/model.gguf"),
            threads: 2,
            context_tokens: 2048,
        };
        assert!(matches!(
            LlamaServer::start(&config),
            Err(LlmError::ModelNotFound(_))
        ));
    }
}
