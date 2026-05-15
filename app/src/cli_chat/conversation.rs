use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;
use crate::terminal::CLIAgent;

/// A model available for a given CLI agent.
pub struct ModelOption {
    pub id: &'static str,
    pub display_name: &'static str,
}

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

    /// Returns the curated list of models for this agent.
    pub fn curated_models(&self) -> &'static [ModelOption] {
        match self {
            AgentKind::Claude => &[
                ModelOption {
                    id: "claude-opus-4-7",
                    display_name: "Claude Opus 4.7",
                },
                ModelOption {
                    id: "claude-sonnet-4-6",
                    display_name: "Claude Sonnet 4.6",
                },
                ModelOption {
                    id: "claude-haiku-4-5-20251001",
                    display_name: "Claude Haiku 4.5",
                },
            ],
            AgentKind::Codex => &[ModelOption {
                id: "o4-mini",
                display_name: "o4-mini",
            }],
            AgentKind::Gemini => &[
                ModelOption {
                    id: "gemini-2.5-pro",
                    display_name: "Gemini 2.5 Pro",
                },
                ModelOption {
                    id: "gemini-2.5-flash",
                    display_name: "Gemini 2.5 Flash",
                },
            ],
            AgentKind::OpenCode => &[ModelOption {
                id: "default",
                display_name: "OpenCode default",
            }],
        }
    }

    /// Returns the CLI command string to launch this agent with the given model.
    pub fn cli_command(&self, model_id: &str) -> String {
        match self {
            AgentKind::Claude => format!("claude --model {}", model_id),
            AgentKind::Codex => format!("codex chat --model {}", model_id),
            AgentKind::Gemini => format!("gemini --model {}", model_id),
            AgentKind::OpenCode => format!("opencode --model {}", model_id),
        }
    }

    /// Returns the default agent kind (Claude) and its default model id.
    pub fn default_agent_and_model() -> (AgentKind, &'static str) {
        let agent = AgentKind::Claude;
        let model_id = agent.curated_models()[0].id;
        (agent, model_id)
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
