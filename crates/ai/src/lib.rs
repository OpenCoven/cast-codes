pub mod agent;
pub mod api_keys;
pub mod aws_credentials;

#[cfg(feature = "cast-agent")]
pub mod cast_agent {
    //! Re-exports + thin host-side facade for the Cast Agent backend.
    //!
    //! Callers in `app` should depend on `ai::cast_agent::*` rather than
    //! `cast_agent` directly so the feature gate remains the single source
    //! of truth. Today this is a pass-through; if more host-side wiring
    //! lands (e.g. wrapping `is_available` with telemetry), it goes here.

    pub use ::cast_agent::{global, is_available, CastAgent, CastAgentConfig, CastAgentRuntime};
}

pub mod llm_id;

pub use llm_id::LLMId;
pub mod diff_validation;
pub mod document;
pub mod gfm_table;
pub mod index;
pub mod paths;
pub mod project_context;
pub mod skills;
mod telemetry;
pub mod workspace;
