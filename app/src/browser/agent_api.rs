//! Typed Rust interface for driving the embedded browser pane.
//!
//! The in-process CastCodes agent uses these methods directly; PLAN-04's
//! MCP server wraps the same surface for out-of-process agents.
//!
//! ## Scope (v1)
//!
//! - `list_tabs` — read-only snapshot of every tab in the active pane.
//!
//! ## Deferred
//!
//! - `navigate` / `reload` / `new_tab` — wired in a follow-up PR. The
//!   action variants exist; the agent_api wrappers need an active-pane
//!   lookup + view update plumbed through.
//! - `evaluate_js` — wry 0.38 has `evaluate_script_with_callback`, so
//!   this can land once the active-pane plumbing does.
//! - `screenshot`, `pick_element`, `capture_console`, `capture_network` —
//!   blocked on wry upgrade.

use serde::Serialize;
use warpui::{AppContext, SingletonEntity as _};

use super::browser_model::TabId;
use crate::workspace::WorkspaceRegistry;

#[derive(Debug, Clone, Serialize)]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub active: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserAgentError {
    #[error("no browser pane is currently open")]
    NoPaneOpen,
    #[error("requested tab id {0} does not exist")]
    UnknownTab(TabId),
    #[error("evaluate_script failed: {0}")]
    EvalFailed(String),
}

pub struct BrowserAgent;

impl BrowserAgent {
    /// Returns one snapshot per open tab in the active browser pane.
    /// Empty list if no pane is open.
    pub fn list_tabs(ctx: &AppContext) -> Vec<TabInfo> {
        let Some(window_id) = ctx.windows().active_window() else {
            return vec![];
        };
        let Some(workspace) = WorkspaceRegistry::as_ref(ctx).get(window_id, ctx) else {
            return vec![];
        };
        workspace.read(ctx, |ws, ctx| ws.list_browser_tabs(ctx))
    }

    /// Stub. Implementation deferred to a follow-up that exposes a
    /// `Workspace::navigate_browser` helper.
    pub fn navigate(_ctx: &mut AppContext, _url: String) -> Result<(), BrowserAgentError> {
        Err(BrowserAgentError::EvalFailed(
            "navigate not yet implemented".into(),
        ))
    }

    /// Stub.
    pub fn reload(_ctx: &mut AppContext) -> Result<(), BrowserAgentError> {
        Err(BrowserAgentError::EvalFailed(
            "reload not yet implemented".into(),
        ))
    }

    /// Stub.
    pub fn new_tab(_ctx: &mut AppContext, _url: String) -> Result<TabId, BrowserAgentError> {
        Err(BrowserAgentError::EvalFailed(
            "new_tab not yet implemented".into(),
        ))
    }

    /// Stub.
    pub async fn evaluate_js(
        _ctx: &AppContext,
        _script: String,
    ) -> Result<serde_json::Value, BrowserAgentError> {
        Err(BrowserAgentError::EvalFailed(
            "evaluate_js not yet implemented".into(),
        ))
    }
}
