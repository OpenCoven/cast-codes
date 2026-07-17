//! CLI adapter: builds the neutral [`ChatEntry`] render model (defined in
//! `crate::agent_transcript::entry`) from OSC-777 `CLIAgentEvent`s. The types
//! themselves live in `agent_transcript`; only this backend-specific
//! conversion lives here.

use chrono::{DateTime, Utc};

// Re-export the moved types so existing `crate::cli_chat::entry::*` paths keep
// resolving after the extraction into `agent_transcript`.
pub use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind, InfoKind, StopReason};

use crate::terminal::cli_agent_sessions::event::{CLIAgentEvent, CLIAgentEventType};

impl ChatEntry {
    /// Build a `ChatEntry` from a parsed `CLIAgentEvent`.
    ///
    /// Returns `None` for event variants that carry no displayable data
    /// (currently only `PromptSubmit` without a `query`).
    pub fn from_event(event: &CLIAgentEvent, sequence: u64, now: DateTime<Utc>) -> Option<Self> {
        let kind = match &event.event {
            CLIAgentEventType::SessionStart => ChatEntryKind::Info {
                info_kind: InfoKind::SessionStart,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::PromptSubmit => {
                let text = event.payload.query.clone()?;
                ChatEntryKind::UserPrompt { text }
            }
            CLIAgentEventType::ToolComplete => ChatEntryKind::ToolCall {
                tool_name: event.payload.tool_name.clone().unwrap_or_default(),
                input_preview: event.payload.tool_input_preview.clone(),
            },
            CLIAgentEventType::PermissionRequest => ChatEntryKind::PermissionRequest {
                summary: event.payload.summary.clone().unwrap_or_default(),
                tool_name: event.payload.tool_name.clone(),
                tool_input_preview: event.payload.tool_input_preview.clone(),
            },
            CLIAgentEventType::PermissionReplied => ChatEntryKind::PermissionReplied {
                // The v1 protocol doesn't yet carry an explicit approved flag,
                // so we default to `true` here. The synthesized `Stop`/follow-up
                // events will reflect cancellation if the user denied.
                approved: true,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::QuestionAsked => ChatEntryKind::Info {
                info_kind: InfoKind::QuestionAsked,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::IdlePrompt => ChatEntryKind::Info {
                info_kind: InfoKind::IdlePrompt,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::Stop => {
                let response = event.payload.response.clone().filter(|s| !s.is_empty());
                // The synthetic `AssistantResponse` entry that mirrors a
                // non-empty `response` is produced by `ChatModel`, not here.
                ChatEntryKind::Stop {
                    reason: StopReason::Normal,
                    response,
                }
            }
            CLIAgentEventType::Unknown(s) => ChatEntryKind::Raw {
                event_type: s.clone(),
                payload_json: serde_json::to_string(&serde_json::json!({
                    "query": event.payload.query,
                    "response": event.payload.response,
                    "summary": event.payload.summary,
                    "tool_name": event.payload.tool_name,
                }))
                .unwrap_or_default(),
            },
        };
        Some(Self {
            sequence,
            created_at: now,
            kind,
        })
    }
}
