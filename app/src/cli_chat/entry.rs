use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::terminal::cli_agent_sessions::event::{CLIAgentEvent, CLIAgentEventType};

/// A single transcript entry rendered in the CastCodes chat panel.
///
/// One `ChatEntry` is produced per actionable `CLIAgentEvent` via
/// [`ChatEntry::from_event`]. The `sequence` is the monotonically
/// increasing index assigned by the caller (the `ChatModel`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
    pub kind: ChatEntryKind,
}

/// Typed transcript entry kinds derived from `CLIAgentEvent` variants.
///
/// `kind` is serialized as a discriminator tag so persisted transcripts
/// round-trip without leaking on-the-wire event names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatEntryKind {
    UserPrompt {
        text: String,
    },
    AssistantResponse {
        text: String,
    },
    ToolCall {
        tool_name: String,
        input_preview: Option<String>,
    },
    PermissionRequest {
        summary: String,
        tool_name: Option<String>,
        tool_input_preview: Option<String>,
    },
    PermissionReplied {
        approved: bool,
        summary: Option<String>,
    },
    Info {
        info_kind: InfoKind,
        summary: Option<String>,
    },
    Stop {
        reason: StopReason,
        response: Option<String>,
    },
    Raw {
        event_type: String,
        payload_json: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InfoKind {
    IdlePrompt,
    QuestionAsked,
    SessionStart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Normal,
    Cancelled,
    Errored,
    Unknown,
}

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
                // non-empty `response` is produced by `ChatModel`, not here —
                // see Task 2.2 for the second insert.
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
