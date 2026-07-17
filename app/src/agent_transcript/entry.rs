//! The neutral transcript render model.
//!
//! `ChatEntry` is the backend-agnostic unit both agent surfaces render: the
//! OSC-777 CLI path builds it via `cli_chat`'s `ChatEntry::from_event`, and the
//! Coven daemon path builds it via `ai_assistant`'s `daemon_event_to_entry`.
//! This module has no backend, transport, or session dependencies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single transcript entry rendered in an agent surface.
///
/// `sequence` is the monotonically increasing index assigned by the caller
/// (the CLI `ChatModel`, or the Coven stream consumer in the agent panel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
    pub kind: ChatEntryKind,
}

/// Typed transcript entry kinds.
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
    /// A tool returned output. Distinct from `ToolCall` — the Coven daemon
    /// emits `tool_result` events; the CLI (`from_event`) path never produces
    /// this variant, so CLI transcripts are unaffected.
    ToolResult {
        tool_name: String,
        output_preview: Option<String>,
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
