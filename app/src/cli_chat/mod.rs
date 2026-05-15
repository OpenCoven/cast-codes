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

pub use conversation::{AgentKind, ChatConversation, ConversationBinding};
pub use entry::{ChatEntry, ChatEntryKind};
pub use model::{ChatModel, ChatModelEvent};
pub use view::ChatPanelView;

#[cfg(test)]
mod conversation_tests;
#[cfg(test)]
mod entry_tests;
