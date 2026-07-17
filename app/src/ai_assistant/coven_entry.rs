//! Maps `cast_agent::CovenAgentEvent`s to the neutral `ChatEntry` render model,
//! so coven-code daemon sessions render through the shared `agent_transcript`
//! views. This adapter lives on the backend side; `agent_transcript` never
//! depends on `CovenAgentEvent`.

use chrono::{DateTime, Utc};

use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind, StopReason};
use ::ai::cast_agent::CovenAgentEvent;

/// Convert one `CovenAgentEvent` into a `ChatEntry`. Returns `None` for events
/// that carry no displayable entry (`Ignored`). Consecutive `AssistantDelta`s
/// are coalesced into one bubble by the caller (the panel), not here.
pub fn daemon_event_to_entry(
    event: CovenAgentEvent,
    sequence: u64,
    now: DateTime<Utc>,
) -> Option<ChatEntry> {
    let kind = match event {
        CovenAgentEvent::AssistantDelta { text } => ChatEntryKind::AssistantResponse { text },
        CovenAgentEvent::ToolCall {
            name,
            input_preview,
        } => ChatEntryKind::ToolCall {
            tool_name: name,
            input_preview,
        },
        CovenAgentEvent::ToolResult { name, output } => ChatEntryKind::ToolResult {
            tool_name: name,
            output_preview: Some(output.chars().take(200).collect()),
        },
        CovenAgentEvent::PermissionRequest { summary } => ChatEntryKind::PermissionRequest {
            summary,
            tool_name: None,
            tool_input_preview: None,
        },
        CovenAgentEvent::Done => ChatEntryKind::Stop {
            reason: StopReason::Normal,
            response: None,
        },
        CovenAgentEvent::Error { message } => ChatEntryKind::Stop {
            reason: StopReason::Errored,
            response: Some(message),
        },
        CovenAgentEvent::Ignored => return None,
    };
    Some(ChatEntry {
        sequence,
        created_at: now,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(ev: CovenAgentEvent) -> Option<ChatEntryKind> {
        daemon_event_to_entry(ev, 0, Utc::now()).map(|e| e.kind)
    }

    #[test]
    fn assistant_delta_becomes_assistant_response() {
        assert!(matches!(
            kind(CovenAgentEvent::AssistantDelta { text: "hi".into() }),
            Some(ChatEntryKind::AssistantResponse { text }) if text == "hi"
        ));
    }

    #[test]
    fn tool_call_becomes_tool_call_entry() {
        assert!(matches!(
            kind(CovenAgentEvent::ToolCall { name: "Read".into(), input_preview: Some("a".into()) }),
            Some(ChatEntryKind::ToolCall { tool_name, .. }) if tool_name == "Read"
        ));
    }

    #[test]
    fn tool_result_becomes_tool_result_entry() {
        assert!(matches!(
            kind(CovenAgentEvent::ToolResult { name: "Read".into(), output: "ok".into() }),
            Some(ChatEntryKind::ToolResult { tool_name, .. }) if tool_name == "Read"
        ));
    }

    #[test]
    fn done_becomes_stop_and_ignored_is_dropped() {
        assert!(matches!(
            kind(CovenAgentEvent::Done),
            Some(ChatEntryKind::Stop { .. })
        ));
        assert!(kind(CovenAgentEvent::Ignored).is_none());
    }
}
