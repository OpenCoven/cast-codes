//! Parser for the harness stream-json emitted by a daemon session launched
//! with `launchMode: "stream"`. Lines are JSON objects tagged by `type`
//! (`system` init / `user` / `assistant` / `tool_result` / `result`), the
//! Claude-Code-style shape the coven-code adapter mirrors.
//!
//! Any line we can't map is surfaced as assistant text so no output is lost —
//! the agent panel degrades to plain text rather than dropping content.

use serde_json::Value;

/// A structured event decoded from one stream-json line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CovenAgentEvent {
    /// A chunk of assistant text (coalesced downstream into one bubble).
    AssistantDelta { text: String },
    /// The harness invoked a tool.
    ToolCall {
        name: String,
        input_preview: Option<String>,
    },
    /// A tool returned output.
    ToolResult { name: String, output: String },
    /// The harness is requesting permission for an action.
    PermissionRequest { summary: String },
    /// Terminal success/stop.
    Done,
    /// Terminal failure with a message.
    Error { message: String },
    /// A line we intentionally drop (e.g. `system` init, echoed `user`).
    Ignored,
}

/// Parse a single stream-json line. Never errors — unparseable or
/// unknown-shaped lines become `AssistantDelta` with the raw text so the
/// panel degrades to plain text rather than dropping output.
pub fn parse_stream_json_line(line: &str) -> CovenAgentEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return CovenAgentEvent::Ignored;
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return CovenAgentEvent::AssistantDelta {
            text: line.to_string(),
        };
    };
    match v.get("type").and_then(Value::as_str) {
        // Session banner / echoed user input carry nothing the panel needs to
        // render (the panel already shows the user's prompt).
        Some("system") | Some("user") => CovenAgentEvent::Ignored,
        Some("assistant") => assistant_event(&v),
        Some("tool_result") => CovenAgentEvent::ToolResult {
            name: v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            output: content_text(&v).unwrap_or_default(),
        },
        Some("result") => match v.get("subtype").and_then(Value::as_str) {
            Some("error") | Some("error_max_turns") | Some("error_during_execution") => {
                CovenAgentEvent::Error {
                    message: v
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("run failed")
                        .to_string(),
                }
            }
            _ => CovenAgentEvent::Done,
        },
        _ => CovenAgentEvent::AssistantDelta {
            text: line.to_string(),
        },
    }
}

/// An `assistant` line carries a `message.content` array whose items are
/// `{type:"text",text}` or `{type:"tool_use",name,input}`. Surface the first
/// actionable item; the text fallback keeps content visible for odd shapes.
fn assistant_event(v: &Value) -> CovenAgentEvent {
    let content = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array);
    if let Some(items) = content {
        for item in items {
            match item.get("type").and_then(Value::as_str) {
                Some("tool_use") => {
                    return CovenAgentEvent::ToolCall {
                        name: item
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        input_preview: item
                            .get("input")
                            .map(|i| i.to_string().chars().take(200).collect::<String>()),
                    };
                }
                Some("text") => {
                    if let Some(t) = item.get("text").and_then(Value::as_str) {
                        return CovenAgentEvent::AssistantDelta {
                            text: t.to_string(),
                        };
                    }
                }
                _ => {}
            }
        }
    }
    // Some harnesses put text directly on a top-level `text` field.
    if let Some(t) = v.get("text").and_then(Value::as_str) {
        return CovenAgentEvent::AssistantDelta {
            text: t.to_string(),
        };
    }
    CovenAgentEvent::Ignored
}

/// Extract textual output from a `tool_result` line (`output` or `content`).
fn content_text(v: &Value) -> Option<String> {
    if let Some(s) = v.get("output").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(s) = v.get("content").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_assistant_text() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            CovenAgentEvent::AssistantDelta {
                text: "Hello".into()
            }
        );
    }

    #[test]
    fn parses_tool_call_from_assistant_tool_use() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file":"a.rs"}}]}}"#;
        match parse_stream_json_line(line) {
            CovenAgentEvent::ToolCall { name, .. } => assert_eq!(name, "Read"),
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn parses_tool_result() {
        let line = r#"{"type":"tool_result","name":"Read","output":"contents"}"#;
        match parse_stream_json_line(line) {
            CovenAgentEvent::ToolResult { name, output } => {
                assert_eq!(name, "Read");
                assert_eq!(output, "contents");
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn parses_result_success_as_done() {
        assert_eq!(
            parse_stream_json_line(r#"{"type":"result","subtype":"success"}"#),
            CovenAgentEvent::Done
        );
    }

    #[test]
    fn parses_result_error_as_error() {
        match parse_stream_json_line(r#"{"type":"result","subtype":"error","error":"boom"}"#) {
            CovenAgentEvent::Error { message } => assert_eq!(message, "boom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn unparseable_line_becomes_fallback_text() {
        assert_eq!(
            parse_stream_json_line("not json"),
            CovenAgentEvent::AssistantDelta {
                text: "not json".into()
            }
        );
    }

    #[test]
    fn system_and_user_are_ignored() {
        assert_eq!(
            parse_stream_json_line(r#"{"type":"system","subtype":"init"}"#),
            CovenAgentEvent::Ignored
        );
        assert_eq!(
            parse_stream_json_line(r#"{"type":"user","message":{"content":"hi"}}"#),
            CovenAgentEvent::Ignored
        );
    }

    #[test]
    fn blank_line_is_ignored() {
        assert_eq!(parse_stream_json_line("   "), CovenAgentEvent::Ignored);
    }
}
