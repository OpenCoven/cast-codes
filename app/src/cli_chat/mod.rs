//! CastCodes Chat Panel.
//!
//! Renders a chat-style transcript of in-terminal CLI agent sessions
//! (claude, codex, gemini, opencode). Subscribes to the existing
//! `CLIAgentSessionsModel`; persists events to a local sqlite store.
//!
//! See `specs/castcodes-chat-panel/PRODUCT.md` and `TECH.md`.

// The chat panel feature is half-wired: model/store/view layers are in
// tree but no caller binds them to a live `CLIAgentSessionsModel` yet.
// Until that wiring lands, scaffolding accessors / strings / setup
// helpers are unconstructed. Allow at each submodule declaration so the
// scaffolding compiles without churn.
#[allow(dead_code)]
pub mod conversation;
#[allow(dead_code)]
pub mod entry;
#[allow(dead_code)]
pub mod feature_flag;
#[allow(dead_code)]
pub mod model;
#[allow(dead_code)]
pub mod paths;
#[allow(dead_code)]
pub mod store;
#[allow(dead_code)]
pub mod store_schema;
#[allow(dead_code)]
pub mod strings;
#[allow(dead_code)]
pub mod view;

pub use model::ChatModel;
pub use view::ChatPanelView;

#[cfg(test)]
mod conversation_tests;
#[cfg(test)]
mod entry_tests;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod store_schema_tests;
#[cfg(test)]
mod store_tests;
