//! Daemon-turn plumbing for the unified agent panel: build the cast_agent
//! message for a composer submit, and map streamed daemon events into
//! transcript entries. The live stream consumer (spawn + buffer + drain) lives
//! on [`super::AgentPanelView`] and mirrors `ai_assistant::panel.rs`.
//!
//! Gated on `cast-agent` (it depends on the `::ai::cast_agent` facade).

use chrono::{DateTime, Utc};

use crate::agent_transcript::entry::ChatEntry;

/// Build the `AgentMessage` for a daemon turn. The prompt goes in `body.prompt`
/// — `GatewayClient::launch_daemon_session` reads it via `extract_prompt` and
/// delivers it in the `POST /api/v1/sessions` launch body (launchMode:"stream").
/// No stdin user-frame is required; each submit is one launch (session-per-turn).
pub fn build_daemon_message(conversation_id: &str, text: &str) -> ::ai::cast_agent::AgentMessage {
    ::ai::cast_agent::AgentMessage {
        conversation_id: conversation_id.to_string(),
        body: serde_json::json!({ "prompt": text }),
    }
}

/// Map one streamed daemon event to a transcript entry (delegates to the shared
/// `ai_assistant` converter so both panels agree on the mapping).
pub fn drain_event_to_entry(
    event: ::ai::cast_agent::CovenAgentEvent,
    sequence: u64,
    now: DateTime<Utc>,
) -> Option<ChatEntry> {
    crate::ai_assistant::coven_entry::daemon_event_to_entry(event, sequence, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_daemon_message_puts_prompt_in_body() {
        let msg = build_daemon_message("conv-1", "do the thing");
        assert_eq!(msg.conversation_id, "conv-1");
        assert_eq!(
            msg.body.get("prompt").and_then(|v| v.as_str()),
            Some("do the thing")
        );
    }

    #[test]
    fn drain_maps_assistant_delta_to_entry() {
        use crate::agent_transcript::entry::ChatEntryKind;
        let entry = drain_event_to_entry(
            ::ai::cast_agent::CovenAgentEvent::AssistantDelta { text: "hi".into() },
            0,
            Utc::now(),
        );
        assert!(matches!(
            entry.map(|e| e.kind),
            Some(ChatEntryKind::AssistantResponse { text }) if text == "hi"
        ));
    }

    #[test]
    fn drain_ignores_ignored_events() {
        let entry = drain_event_to_entry(::ai::cast_agent::CovenAgentEvent::Ignored, 0, Utc::now());
        assert!(entry.is_none());
    }
}
