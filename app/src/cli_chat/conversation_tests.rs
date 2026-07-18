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

#[test]
fn backend_round_trips_name_and_kind() {
    use super::conversation::ConversationBackend;
    let cli = ConversationBackend::Cli(AgentKind::Codex);
    assert_eq!(cli.name(), "codex");
    assert_eq!(cli.kind_str(), "cli");
    assert_eq!(ConversationBackend::from_persisted("codex", "cli"), cli);

    let daemon = ConversationBackend::Daemon {
        harness: "coven-code".into(),
    };
    assert_eq!(daemon.name(), "coven-code");
    assert_eq!(daemon.kind_str(), "daemon");
    assert_eq!(ConversationBackend::from_persisted("coven-code", "daemon"), daemon);
}

#[test]
fn from_persisted_unknown_cli_agent_defaults_to_claude() {
    use super::conversation::ConversationBackend;
    assert_eq!(
        ConversationBackend::from_persisted("nope", "cli"),
        ConversationBackend::Cli(AgentKind::Claude)
    );
}
