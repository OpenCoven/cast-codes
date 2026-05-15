use chrono::Utc;

use super::conversation::{AgentKind, ChatConversation};
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

#[test]
fn agent_kind_protocol_strings_round_trip() {
    let kinds = [
        AgentKind::Claude,
        AgentKind::Codex,
        AgentKind::Gemini,
        AgentKind::OpenCode,
    ];
    let strs = ["claude", "codex", "gemini", "opencode"];
    for (kind, s) in kinds.iter().zip(strs.iter()) {
        assert_eq!(kind.as_protocol_str(), *s);
    }
}

#[test]
fn new_conversation_defaults_are_in_progress() {
    let conv = ChatConversation::new("abc".into(), AgentKind::Claude, Utc::now());
    assert_eq!(conv.session_id, "abc");
    assert_eq!(conv.entries.len(), 0);
    assert!(matches!(conv.status, CLIAgentSessionStatus::InProgress));
}
