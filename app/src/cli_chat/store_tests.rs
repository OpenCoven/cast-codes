use chrono::{TimeZone, Utc};

use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

use super::conversation::{AgentKind, ChatConversation};
use super::entry::{ChatEntry, ChatEntryKind};
use super::store::ChatStore;

#[test]
fn round_trip_conversation_and_entries() {
    let store = ChatStore::open_in_memory().unwrap();

    let now = Utc::now();
    let mut conv = ChatConversation::new("sess-1".into(), AgentKind::Claude, now);
    conv.title = "Fix the tests".into();
    conv.cwd = Some("/home/user/project".into());
    conv.last_model = Some("claude-4".into());

    store.upsert_conversation(&conv).unwrap();

    let entry = ChatEntry {
        sequence: 0,
        created_at: now,
        kind: ChatEntryKind::UserPrompt {
            text: "hello world".into(),
        },
    };
    store.insert_entry("sess-1", &entry).unwrap();

    let loaded = store.load_conversation("sess-1").unwrap().unwrap();
    assert_eq!(loaded.title, "Fix the tests");
    assert_eq!(loaded.session_id, "sess-1");
    assert_eq!(loaded.agent.as_protocol_str(), "claude");
    assert_eq!(loaded.cwd.as_deref(), Some("/home/user/project"));
    assert_eq!(loaded.last_model.as_deref(), Some("claude-4"));
    assert_eq!(loaded.entries.len(), 1);

    match &loaded.entries[0].kind {
        ChatEntryKind::UserPrompt { text } => assert_eq!(text, "hello world"),
        other => panic!("expected UserPrompt, got {:?}", other),
    }
}

#[test]
fn list_conversations_returns_in_recency_order() {
    let store = ChatStore::open_in_memory().unwrap();

    let older = Utc.timestamp_millis_opt(1_000_000).unwrap();
    let newer = Utc.timestamp_millis_opt(2_000_000).unwrap();

    let mut conv_old = ChatConversation::new("old".into(), AgentKind::Gemini, older);
    conv_old.title = "Old conversation".into();

    let mut conv_new = ChatConversation::new("new".into(), AgentKind::Codex, newer);
    conv_new.title = "New conversation".into();

    // Insert older first, then newer
    store.upsert_conversation(&conv_old).unwrap();
    store.upsert_conversation(&conv_new).unwrap();

    let list = store.list_conversations().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].session_id, "new");
    assert_eq!(list[1].session_id, "old");
}

#[test]
fn upsert_updates_mutable_fields() {
    let store = ChatStore::open_in_memory().unwrap();

    let t1 = Utc.timestamp_millis_opt(1_000_000).unwrap();
    let t2 = Utc.timestamp_millis_opt(2_000_000).unwrap();

    let conv = ChatConversation::new("sess-u".into(), AgentKind::OpenCode, t1);
    store.upsert_conversation(&conv).unwrap();

    // Update title, status, and updated_at
    let mut updated = conv.clone();
    updated.title = "Updated title".into();
    updated.updated_at = t2;
    updated.status = CLIAgentSessionStatus::Success;
    store.upsert_conversation(&updated).unwrap();

    let loaded = store.load_conversation("sess-u").unwrap().unwrap();
    assert_eq!(loaded.title, "Updated title");
    assert!(matches!(loaded.status, CLIAgentSessionStatus::Success));
    // created_at should be preserved (from the original INSERT)
    assert_eq!(loaded.created_at.timestamp_millis(), t1.timestamp_millis());
    assert_eq!(loaded.updated_at.timestamp_millis(), t2.timestamp_millis());
}

#[test]
fn insert_entry_is_idempotent() {
    let store = ChatStore::open_in_memory().unwrap();

    let now = Utc::now();
    let conv = ChatConversation::new("sess-idem".into(), AgentKind::Claude, now);
    store.upsert_conversation(&conv).unwrap();

    let entry = ChatEntry {
        sequence: 0,
        created_at: now,
        kind: ChatEntryKind::AssistantResponse {
            text: "hi".into(),
        },
    };

    store.insert_entry("sess-idem", &entry).unwrap();
    store.insert_entry("sess-idem", &entry).unwrap(); // duplicate — no error

    let entries = store.load_entries("sess-idem").unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn load_nonexistent_conversation_returns_none() {
    let store = ChatStore::open_in_memory().unwrap();
    assert!(store.load_conversation("ghost").unwrap().is_none());
}

#[test]
fn blocked_status_round_trips() {
    let store = ChatStore::open_in_memory().unwrap();

    let now = Utc::now();
    let mut conv = ChatConversation::new("sess-blk".into(), AgentKind::Claude, now);
    conv.status = CLIAgentSessionStatus::Blocked {
        message: Some("needs approval".into()),
    };
    store.upsert_conversation(&conv).unwrap();

    let loaded = store.load_conversation("sess-blk").unwrap().unwrap();
    match &loaded.status {
        CLIAgentSessionStatus::Blocked { message } => {
            assert_eq!(message.as_deref(), Some("needs approval"));
        }
        other => panic!("expected Blocked, got {:?}", other),
    }
}
