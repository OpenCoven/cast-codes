//! Unit tests for [`super::model::ChatModel::apply_event`].
//!
//! These tests drive the pure-logic `apply_event` helper directly, rather
//! than spinning up a warpui `App::test` harness. PLAN.md Task 2.2 Step 3
//! explicitly authorizes this approach when the warpui harness is
//! prohibitively complex; the trade-off is that we do not verify
//! `ChatModelEvent` emission ordering — that wiring is exercised by the
//! `subscribe_to_model` plumbing in `ChatModel::new`, which is mechanical
//! and identical to every other subscriber in the app (see
//! `app/src/ai/agent_management/agent_management_model.rs:44`).
//!
//! Event payload construction goes through the real OSC 777 parser
//! ([`parse_event`]) using the same fixtures the entry-conversion tests
//! consume, so the production code path is exercised end-to-end from
//! parsed-event onward.

use chrono::Utc;
use warpui::EntityId;

use super::conversation::ConversationBinding;
use super::entry::{ChatEntryKind, StopReason};
use super::model::ChatModel;
use crate::terminal::cli_agent_sessions::event::{parse_event, CLIAgentEvent};

const SENTINEL: &str = "warp://cli-agent";

fn parse_fixture(body: &str) -> CLIAgentEvent {
    parse_event(Some(SENTINEL), body).expect("fixture parses")
}

#[test]
fn first_prompt_creates_conversation_and_binds_live() {
    let mut model = ChatModel::new_unwired();
    let event = parse_fixture(include_str!("tests/fixtures/claude_prompt_submit.json"));
    let terminal_view_id = EntityId::new();

    let outcome = model.apply_event(&event, terminal_view_id, Utc::now());

    assert_eq!(outcome.updated_session_id.as_deref(), Some("abc"));
    assert!(outcome.binding_changed, "first event should bind Live");

    let conv = model
        .conversation("abc")
        .expect("conversation was created for session_id=abc");
    assert_eq!(conv.entries.len(), 1);
    assert!(matches!(
        conv.entries[0].kind,
        ChatEntryKind::UserPrompt { .. }
    ));
    // Title is derived from the first user prompt.
    assert_eq!(conv.title, "fix the bug");
    assert_eq!(conv.cwd.as_deref(), Some("/tmp/proj"));
    assert_eq!(conv.project.as_deref(), Some("proj"));

    match model.binding() {
        ConversationBinding::Live {
            session_id,
            terminal_view_id: bound,
        } => {
            assert_eq!(session_id, "abc");
            assert_eq!(*bound, terminal_view_id);
        }
        other => panic!("expected Live binding, got {:?}", other),
    }
}

#[test]
fn stop_event_with_response_splits_into_assistant_then_stop() {
    let mut model = ChatModel::new_unwired();
    let terminal_view_id = EntityId::new();

    // Seed the conversation with a user prompt first.
    let prompt = parse_fixture(include_str!("tests/fixtures/claude_prompt_submit.json"));
    model.apply_event(&prompt, terminal_view_id, Utc::now());

    // Now fire the Stop event with a response.
    let stop = parse_fixture(include_str!(
        "tests/fixtures/claude_stop_with_response.json"
    ));
    model.apply_event(&stop, terminal_view_id, Utc::now());

    let conv = model.conversation("abc").expect("conversation exists");
    // UserPrompt + AssistantResponse + Stop.
    assert_eq!(
        conv.entries.len(),
        3,
        "expected user/assistant/stop, got {:#?}",
        conv.entries
    );

    let kinds: Vec<&ChatEntryKind> = conv.entries.iter().map(|e| &e.kind).collect();
    assert!(matches!(kinds[0], ChatEntryKind::UserPrompt { .. }));
    match kinds[1] {
        ChatEntryKind::AssistantResponse { text } => assert_eq!(text, "Memory is safe"),
        other => panic!("expected AssistantResponse, got {:?}", other),
    }
    match kinds[2] {
        ChatEntryKind::Stop {
            reason: StopReason::Normal,
            response: Some(text),
        } => assert_eq!(text, "Memory is safe"),
        other => panic!("expected Stop with response, got {:?}", other),
    }

    // Sequences are monotonically increasing across the synthetic split.
    let seqs: Vec<u64> = conv.entries.iter().map(|e| e.sequence).collect();
    assert_eq!(seqs, vec![0, 1, 2]);
}

#[test]
fn unknown_agent_is_ignored() {
    let mut model = ChatModel::new_unwired();
    // `auggie` is a supported listener agent but isn't in [`AgentKind`].
    let body = r#"{"v":1,"agent":"auggie","event":"prompt_submit","session_id":"x","query":"hi"}"#;
    let event = parse_fixture(body);

    let outcome = model.apply_event(&event, EntityId::new(), Utc::now());

    assert!(outcome.updated_session_id.is_none());
    assert!(!outcome.binding_changed);
    assert!(model.conversation("x").is_none());
    assert!(matches!(model.binding(), ConversationBinding::None));
}

#[test]
fn event_without_session_id_is_ignored() {
    let mut model = ChatModel::new_unwired();
    let body = r#"{"v":1,"agent":"claude","event":"prompt_submit","query":"hi"}"#;
    let event = parse_fixture(body);

    let outcome = model.apply_event(&event, EntityId::new(), Utc::now());

    assert!(outcome.updated_session_id.is_none());
    assert!(!outcome.binding_changed);
    assert!(model.conversations_sorted_by_recency().is_empty());
}

#[test]
fn second_session_does_not_rebind_when_already_bound() {
    let mut model = ChatModel::new_unwired();
    let tid_a = EntityId::new();
    let tid_b = EntityId::new();

    let first = parse_fixture(include_str!("tests/fixtures/claude_prompt_submit.json"));
    model.apply_event(&first, tid_a, Utc::now());

    let second_body =
        r#"{"v":1,"agent":"claude","event":"prompt_submit","session_id":"def","query":"another"}"#;
    let second = parse_fixture(second_body);
    let outcome = model.apply_event(&second, tid_b, Utc::now());

    assert_eq!(outcome.updated_session_id.as_deref(), Some("def"));
    assert!(
        !outcome.binding_changed,
        "binding must not change once a Live binding is set"
    );
    match model.binding() {
        ConversationBinding::Live { session_id, .. } => assert_eq!(session_id, "abc"),
        other => panic!("expected to remain bound to first session, got {:?}", other),
    }

    // Both conversations exist; "def" is more recent.
    let sorted = model.conversations_sorted_by_recency();
    assert_eq!(sorted.len(), 2);
    assert_eq!(sorted[0].session_id, "def");
    assert_eq!(sorted[1].session_id, "abc");
}

#[test]
fn sequence_monotonic_across_multiple_events() {
    let mut model = ChatModel::new_unwired();
    let tid = EntityId::new();

    let prompt = parse_fixture(include_str!("tests/fixtures/claude_prompt_submit.json"));
    let tool = parse_fixture(include_str!("tests/fixtures/claude_tool_complete.json"));

    model.apply_event(&prompt, tid, Utc::now());
    model.apply_event(&tool, tid, Utc::now());

    let conv = model.conversation("abc").expect("conversation exists");
    assert_eq!(conv.entries.len(), 2);
    assert_eq!(conv.entries[0].sequence, 0);
    assert_eq!(conv.entries[1].sequence, 1);
}
