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

    /// Inverse of [`AgentKind::as_protocol_str`]. `None` for unrecognized names.
    pub fn from_protocol_str(s: &str) -> Option<Self> {
        Some(match s {
            "claude" => AgentKind::Claude,
            "codex" => AgentKind::Codex,
            "gemini" => AgentKind::Gemini,
            "opencode" => AgentKind::OpenCode,
            _ => return None,
        })
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

/// The backend a conversation is driven by. Generalizes the panel over the
/// OSC-777 CLIs (which run in a terminal pane) and the Coven daemon (headless).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationBackend {
    /// An OSC-777 CLI agent running in a terminal pane.
    Cli(AgentKind),
    /// A Coven daemon lane (e.g. `coven-code`), driven headless via cast_agent.
    Daemon { harness: String },
}

impl ConversationBackend {
    /// Harness/kind name persisted in the `agent` column (`claude`, `codex`,
    /// `coven-code`, …).
    pub fn name(&self) -> String {
        match self {
            ConversationBackend::Cli(k) => k.as_protocol_str().to_string(),
            ConversationBackend::Daemon { harness } => harness.clone(),
        }
    }

    /// Discriminator persisted in the `backend` column.
    pub fn kind_str(&self) -> &'static str {
        match self {
            ConversationBackend::Cli(_) => "cli",
            ConversationBackend::Daemon { .. } => "daemon",
        }
    }

    /// Human-facing label for lists and headers. CLI agents use their curated
    /// display name; daemon lanes show the harness name verbatim.
    pub fn display_name(&self) -> String {
        match self {
            ConversationBackend::Cli(k) => k.display_name().to_string(),
            ConversationBackend::Daemon { harness } => harness.clone(),
        }
    }

    /// The CLI [`AgentKind`] backing this conversation, for CLI-only affordances
    /// like the model picker. Daemon lanes have no CLI kind, so they fall back
    /// to the default agent (`Claude`) — such lanes never surface the picker.
    pub fn agent_kind(&self) -> AgentKind {
        match self {
            ConversationBackend::Cli(k) => *k,
            ConversationBackend::Daemon { .. } => AgentKind::Claude,
        }
    }

    /// Reconstruct from the two persisted columns. Unknown `cli` agent names
    /// fall back to `Claude` (forward-compatible); any non-`cli` kind is a
    /// daemon lane carrying the name verbatim.
    pub fn from_persisted(name: &str, kind: &str) -> Self {
        if kind == "cli" {
            ConversationBackend::Cli(
                AgentKind::from_protocol_str(name).unwrap_or(AgentKind::Claude),
            )
        } else {
            ConversationBackend::Daemon {
                harness: name.to_string(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatConversation {
    pub session_id: String,
    pub backend: ConversationBackend,
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
    pub fn new(session_id: String, backend: ConversationBackend, now: DateTime<Utc>) -> Self {
        Self {
            session_id,
            backend,
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
    /// A Coven-daemon conversation selected for input. No terminal — sends go
    /// through cast_agent. Sendable whenever the daemon runtime is available.
    LiveDaemon {
        session_id: String,
    },
}
