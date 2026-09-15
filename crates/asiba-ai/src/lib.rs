mod backend;
mod context;
mod error;
mod playbook;
mod prompt;
mod tokens;

pub use backend::{
    ANTHROPIC_DEFAULT_MODEL, Anthropic, Backend, Completion, GEMINI_DEFAULT_MODEL, Gemini,
    OPENAI_DEFAULT_BASE_URL, OPENAI_DEFAULT_MODEL, OpenAi, ProviderConfig,
};
pub use context::{ContextBuilder, Part};
pub use error::AiError;
pub use playbook::{Playbook, fleet_audit, full_audit, playbook, queries, section_audit};
pub use prompt::{SYSTEM_PROMPT, build_user};
pub use tokens::estimate_tokens;
