//! Cast Agent — Coven-native agent backend for CastCodes.
//!
//! Replaces the Warp Agent integration in `crates/ai` by talking to the
//! Coven Gateway (HTTP + WebSocket), collecting workspace substrate context,
//! and bridging Coven sessions and Comux panes.
//!
//! Public entry points:
//! - [`AgentBackend`] — trait the host (`crates/ai`) calls into.
//! - [`CastAgent`] — concrete implementation backed by the Coven Gateway.
//! - [`Substrate`], [`CovenSession`], [`ComuxPane`] — payload types.
//!
//! See `CAST-AGENT.md` at the repo root for architecture + configuration.

pub mod agent;
pub mod comux;
pub mod config;
pub mod gateway;
pub mod session;
pub mod substrate;

pub use agent::{AgentBackend, AgentMessage, AgentResponse, CastAgent};
pub use comux::ComuxPane;
pub use config::CastAgentConfig;
pub use session::CovenSession;
pub use substrate::{DiagnosticEntry, PaneInfo, Substrate};

/// Error type for cast_agent operations.
#[derive(Debug, thiserror::Error)]
pub enum CastAgentError {
    #[error("gateway unreachable: {0}")]
    GatewayUnreachable(String),

    #[error("gateway returned status {status}: {body}")]
    GatewayStatus { status: u16, body: String },

    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CastAgentError>;
