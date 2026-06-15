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
// The Unix-socket transport + daemon chat helpers are only compiled on
// Unix-family targets — Windows has no `tokio::net::UnixStream`, and
// wasm doesn't run the daemon at all. The TCP/bridge transport in
// `gateway` is the cross-platform path.
#[cfg(unix)]
mod daemon_chat;
pub mod gateway;
pub mod runtime;
pub mod session;
pub mod substrate;
#[cfg(unix)]
mod unix_http;

pub use agent::{AgentBackend, AgentMessage, AgentResponse, CastAgent};
pub use comux::ComuxPane;
pub use config::CastAgentConfig;
pub use gateway::MessageChunk;
pub use runtime::{
    global, is_available, sessions, set_host_substrate, update_host_substrate, CastAgentRuntime,
};
pub use session::{CovenSession, SessionStatus};
pub use substrate::{DiagnosticEntry, DiagnosticSeverity, HostSubstrate, PaneInfo, Substrate};
