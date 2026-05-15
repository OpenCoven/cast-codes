use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;
use crate::terminal::CLIAgent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
    OpenCode,
}

impl AgentKind {
    pub fn from_cli_agent(agent: &CLIAgent) -> Option<Self> {
        use crate::terminal::CLIAgent::*;
        Some(match agent {
            Claude => AgentKind::Claude,
            Codex => AgentKind::Codex,
            Gemini => AgentKind::Gemini,
            OpenCode => AgentKind::OpenCode,
            _ => return None,
        })
    }

    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
            AgentKind::Gemini => "gemini",
            AgentKind::OpenCode => "opencode",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentKind::Claude => "Claude",
            AgentKind::Codex => "Codex",
            AgentKind::Gemini => "Gemini",
            AgentKind::OpenCode => "OpenCode",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatConversation {
    pub session_id: String,
    pub agent: AgentKind,
    pub title: String,
    pub cwd: Option<String>,
    pub project: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: CLIAgentSessionStatus,
    pub last_model: Option<String>,
    pub entries: Vec<crate::cli_chat::entry::ChatEntry>,
}

impl ChatConversation {
    pub fn new(session_id: String, agent: AgentKind, now: DateTime<Utc>) -> Self {
        Self {
            session_id,
            agent,
            title: String::new(),
            cwd: None,
            project: None,
            created_at: now,
            updated_at: now,
            status: CLIAgentSessionStatus::InProgress,
            last_model: None,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConversationBinding {
    None,
    Live {
        session_id: String,
        terminal_view_id: warpui::EntityId,
    },
    Past {
        session_id: String,
    },
}
