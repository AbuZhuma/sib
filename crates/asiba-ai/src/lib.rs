mod backend;
mod context;
mod error;
mod playbook;
mod prompt;
mod tokens;

pub use backend::{Backend, Completion, DEFAULT_MODEL, Gemini, GeminiConfig};
pub use context::{ContextBuilder, Part};
pub use error::AiError;
pub use playbook::{Playbook, full_audit, playbook, queries};
pub use prompt::{SYSTEM_PROMPT, build_user};
pub use tokens::estimate_tokens;
