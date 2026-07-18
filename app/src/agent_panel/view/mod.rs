//! Render helpers for [`super::AgentPanelView`]. Free functions taking
//! `&AgentPanelView` + `&AppContext`, mirroring the `cli_chat` panel's shape
//! but generalized over `ConversationBackend` and depending only on the
//! `cli_chat` *model* layer + the shared `agent_transcript` renderer (never
//! `cli_chat::view`, which is retired in 2d).

pub mod composer;
pub mod conversation_list;
pub mod header;
pub mod transcript;
