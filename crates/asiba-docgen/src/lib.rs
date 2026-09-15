mod notes;
mod render;
mod section;
mod sections;
mod series_stats;
mod write;

#[cfg(test)]
pub(crate) mod test_support;

pub use notes::merge_notes;
pub use render::{
    render_human, render_llm, render_llm_section, render_llm_sections, strip_timestamp,
};
pub use section::{DocContext, Section, SectionId};

use asiba_modules::system;

pub fn has_required_data(server: &asiba_core::ServerState) -> bool {
    server.snapshot(system::ID).is_some()
}
