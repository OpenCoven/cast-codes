//! CastCodes Chat Panel.
//!
//! Renders a chat-style transcript of in-terminal CLI agent sessions
//! (claude, codex, gemini, opencode). Subscribes to the existing
//! `CLIAgentSessionsModel`; persists events to a local sqlite store.
//!
//! See `specs/castcodes-chat-panel/PRODUCT.md` and `TECH.md`.

pub mod conversation;
pub mod entry;
pub mod feature_flag;
pub mod model;
pub mod paths;
pub mod store;
pub mod store_schema;
pub mod strings;
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
