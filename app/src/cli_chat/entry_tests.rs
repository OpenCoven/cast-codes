use chrono::Utc;

use super::entry::{ChatEntry, ChatEntryKind, InfoKind};
use crate::terminal::cli_agent_sessions::event::{parse_event, CLIAgentEvent};

fn parse_fixture(body: &str) -> CLIAgentEvent {
    parse_event(Some("warp://cli-agent"), body).expect("fixture parses")
}

#[test]
fn prompt_submit_becomes_user_prompt() {
    let body = include_str!("tests/fixtures/claude_prompt_submit.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    match entry.kind {
        ChatEntryKind::UserPrompt { text } => assert_eq!(text, "fix the bug"),
        other => panic!("expected UserPrompt, got {:?}", other),
    }
}

#[test]
fn tool_complete_becomes_tool_call() {
    let body = include_str!("tests/fixtures/claude_tool_complete.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    match entry.kind {
        ChatEntryKind::ToolCall {
            tool_name,
            input_preview,
        } => {
            assert_eq!(tool_name, "Bash");
            assert!(input_preview.is_none());
        }
        other => panic!("expected ToolCall, got {:?}", other),
    }
}

#[test]
fn permission_request_becomes_permission_request_entry() {
    let body = include_str!("tests/fixtures/claude_permission_request.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    match entry.kind {
        ChatEntryKind::PermissionRequest {
            summary,
            tool_name,
            tool_input_preview,
        } => {
            assert_eq!(summary, "Wants to run Bash: rm -rf /tmp");
            assert_eq!(tool_name.as_deref(), Some("Bash"));
            assert_eq!(tool_input_preview.as_deref(), Some("rm -rf /tmp"));
        }
        other => panic!("expected PermissionRequest, got {:?}", other),
    }
}

#[test]
fn stop_with_response_carries_response() {
    let body = include_str!("tests/fixtures/claude_stop_with_response.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    match entry.kind {
        ChatEntryKind::Stop {
            response: Some(r), ..
        } => assert!(!r.is_empty()),
        other => panic!("expected Stop with response, got {:?}", other),
    }
}

#[test]
fn idle_prompt_becomes_info() {
    let body = include_str!("tests/fixtures/claude_idle_prompt.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    assert!(matches!(
        entry.kind,
        ChatEntryKind::Info {
            info_kind: InfoKind::IdlePrompt,
            ..
        }
    ));
}

#[test]
fn sequence_and_timestamp_are_preserved() {
    let body = include_str!("tests/fixtures/claude_prompt_submit.json");
    let event = parse_fixture(body);
    let now = Utc::now();
    let entry = ChatEntry::from_event(&event, 42, now).expect("entry produced");
    assert_eq!(entry.sequence, 42);
    assert_eq!(entry.created_at, now);
}
