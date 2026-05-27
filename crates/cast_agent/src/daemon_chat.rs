//! Helpers for driving a chat turn through the OpenCoven daemon's
//! session lifecycle (`POST /api/v1/sessions` + `GET /api/v1/events`).
//!
//! The daemon doesn't expose a `/messages` endpoint. To send a chat
//! message we create a non-interactive session whose prompt is the
//! caller's message text, then poll the event stream until the session
//! reaches a terminal status, accumulating `output` events into a single
//! response string.
//!
//! These helpers handle the awkward parts:
//! - Extracting a plain-text prompt from the provider-shaped
//!   [`crate::agent::AgentMessage::body`] (the Cast Agent surface stores
//!   provider-shaped JSON, not text, so we sniff for common shapes).
//! - Parsing the daemon's `payload_json` field (which is itself a
//!   JSON-encoded string containing `{"data": "<text>"}` for output events).
//! - Recognizing terminal session statuses (the daemon emits a wider set
//!   than CastCodes' UI tri-state, so the terminal predicate lives here
//!   rather than in [`crate::session`]).

use serde::Deserialize;
use serde_json::Value;

/// Best-effort extract a prompt string from an
/// [`crate::agent::AgentMessage::body`]. The body is provider-shaped
/// JSON, so we try several shapes in priority order:
///
/// 1. Top-level `prompt` (string).
/// 2. Top-level `text` (string).
/// 3. Top-level `message` (string).
/// 4. `messages` array — last entry with `role: "user"`, then last entry
///    of any role, using its `content` string.
///
/// Returns `None` when none of those shapes match; callers should treat
/// that as a programming error (the chat call site should always carry
/// a prompt somewhere).
pub(crate) fn extract_prompt(body: &Value) -> Option<String> {
    if let Some(s) = body.get("prompt").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(s) = body.get("text").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(s) = body.get("message").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        for msg in messages.iter().rev() {
            if msg.get("role").and_then(Value::as_str) == Some("user") {
                if let Some(content) = msg.get("content").and_then(Value::as_str) {
                    return Some(content.to_string());
                }
            }
        }
        if let Some(last) = messages.last() {
            if let Some(content) = last.get("content").and_then(Value::as_str) {
                return Some(content.to_string());
            }
        }
    }
    None
}

/// Whether a daemon session is in a terminal state (no more events
/// will be emitted). The daemon's status vocabulary is wider than the
/// CastCodes UI tri-state, so we match exhaustively here on the
/// known terminal values.
pub(crate) fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "killed" | "failed" | "orphaned")
}

/// One event row from `GET /api/v1/events?sessionId=...`. Only the
/// fields we actually consume are typed; the rest is dropped on
/// deserialize. `payload_json` is itself a JSON-encoded string.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DaemonEvent {
    pub seq: u64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub payload_json: String,
}

/// Envelope returned by `GET /api/v1/events`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DaemonEventsPage {
    pub events: Vec<DaemonEvent>,
    #[serde(default, rename = "hasMore")]
    #[allow(dead_code)]
    pub has_more: bool,
}

/// Pull the `data` text out of an `output` event's `payload_json`. The
/// shape is `{"data": "<text>"}` for terminal output; other event kinds
/// have different shapes (e.g. `input` events also carry `data` but
/// represent user-side bytes — callers should filter by `kind` before
/// calling this).
///
/// Returns `None` when the payload is missing, malformed, or has no
/// `data` string. Logging is the caller's responsibility.
pub(crate) fn parse_output_data(payload_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(payload_json).ok()?;
    v.get("data").and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_prompt_top_level_keys() {
        assert_eq!(extract_prompt(&json!({"prompt": "hi"})), Some("hi".into()));
        assert_eq!(
            extract_prompt(&json!({"text": "hello"})),
            Some("hello".into())
        );
        assert_eq!(extract_prompt(&json!({"message": "yo"})), Some("yo".into()));
    }

    #[test]
    fn extract_prompt_messages_array_prefers_last_user_role() {
        let body = json!({
            "messages": [
                { "role": "system",    "content": "be terse" },
                { "role": "user",      "content": "first user msg" },
                { "role": "assistant", "content": "first reply" },
                { "role": "user",      "content": "latest user msg" },
            ]
        });
        assert_eq!(extract_prompt(&body), Some("latest user msg".into()));
    }

    #[test]
    fn extract_prompt_messages_array_falls_back_to_last_when_no_user_role() {
        let body = json!({
            "messages": [
                { "role": "system",    "content": "system one" },
                { "role": "assistant", "content": "assistant tail" },
            ]
        });
        assert_eq!(extract_prompt(&body), Some("assistant tail".into()));
    }

    #[test]
    fn extract_prompt_returns_none_on_unrelated_shape() {
        assert_eq!(extract_prompt(&json!({"unrelated": 42})), None);
        assert_eq!(extract_prompt(&json!(null)), None);
    }

    #[test]
    fn terminal_status_matches_daemon_values() {
        assert!(is_terminal_status("completed"));
        assert!(is_terminal_status("killed"));
        assert!(is_terminal_status("failed"));
        assert!(is_terminal_status("orphaned"));
    }

    #[test]
    fn non_terminal_status_returns_false() {
        assert!(!is_terminal_status("running"));
        assert!(!is_terminal_status("created"));
        assert!(!is_terminal_status("idle"));
        assert!(!is_terminal_status("future_state"));
    }

    #[test]
    fn parse_output_data_unwraps_inner_json_string() {
        let payload = r#"{"data":"hello world\r\n"}"#;
        assert_eq!(parse_output_data(payload), Some("hello world\r\n".into()));
    }

    #[test]
    fn parse_output_data_returns_none_on_missing_data_field() {
        assert_eq!(parse_output_data(r#"{"other":"value"}"#), None);
        assert_eq!(parse_output_data("not-json"), None);
        assert_eq!(parse_output_data(r#"{"data":42}"#), None);
    }

    #[test]
    fn deserializes_live_events_envelope() {
        let raw = r#"{
            "events": [
                {
                    "seq": 7393,
                    "id": "1c056204-b5e0-4488-ba72-808e4a973dbc",
                    "session_id": "9b1f77a7-8b9b-48dd-ba46-cb626f5d060d",
                    "kind": "output",
                    "payload_json": "{\"data\":\"OpenAI Codex v0.133.0\\r\\n\"}",
                    "created_at": "2026-05-26T19:48:59.693475000Z"
                }
            ],
            "nextCursor": { "afterSeq": 7393 },
            "hasMore": false
        }"#;
        let page: DaemonEventsPage = serde_json::from_str(raw).expect("parse");
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].seq, 7393);
        assert_eq!(page.events[0].kind, "output");
        assert_eq!(
            parse_output_data(&page.events[0].payload_json),
            Some("OpenAI Codex v0.133.0\r\n".into())
        );
    }
}
