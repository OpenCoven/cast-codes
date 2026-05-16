//! Substrate — the current workspace context sent to the Coven Gateway
//! on each agent invocation. The collector here gathers only what's safe
//! to determine without a full `crates/ai` -> `cast_agent` integration.
//!
//! Host integration (in `crates/ai`) is expected to populate `active_file`,
//! `open_panes`, `recent_errors`, and `git_branch` from its own state and
//! pass them in; this collector only fills in shell CWD and Comux panes.

use std::path::PathBuf;

use crate::comux::ComuxPane;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Substrate {
    /// The file currently focused in the editor, if any.
    pub active_file: Option<PathBuf>,
    /// Panes open in the host (CastCodes terminal panes).
    pub open_panes: Vec<PaneInfo>,
    /// CWD of the shell that owns the focused pane.
    pub shell_cwd: PathBuf,
    /// Git branch resolved from `shell_cwd`.
    pub git_branch: Option<String>,
    /// Recent diagnostics (errors/warnings) from the language servers.
    pub recent_errors: Vec<DiagnosticEntry>,
    /// Comux panes discovered via the Unix socket bridge — empty if not running.
    pub comux_panes: Vec<ComuxPane>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaneInfo {
    pub id: String,
    pub title: String,
    pub cwd: PathBuf,
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticEntry {
    pub file: PathBuf,
    pub line: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

pub struct SubstrateCollector {
    // Reserved for caches (e.g. last git branch lookup) — empty for v1.
}

impl SubstrateCollector {
    pub fn new() -> Self {
        Self {}
    }

    /// Collect a minimal substrate snapshot. The host is expected to fill in
    /// editor/pane state via [`Self::with_host_state`] before sending.
    pub async fn collect(&self) -> anyhow::Result<Substrate> {
        let shell_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let git_branch = detect_git_branch(&shell_cwd);
        Ok(Substrate {
            active_file: None,
            open_panes: Vec::new(),
            shell_cwd,
            git_branch,
            recent_errors: Vec::new(),
            comux_panes: Vec::new(),
        })
    }
}

impl Default for SubstrateCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Read the current branch from `<cwd>/.git/HEAD` if present, without
/// shelling out to git. Returns `None` for detached HEAD or non-git dirs.
fn detect_git_branch(cwd: &std::path::Path) -> Option<String> {
    let mut dir = cwd.to_path_buf();
    loop {
        let head = dir.join(".git").join("HEAD");
        if head.exists() {
            let content = std::fs::read_to_string(&head).ok()?;
            let trimmed = content.trim();
            return trimmed
                .strip_prefix("ref: refs/heads/")
                .map(|s| s.to_string());
        }
        if !dir.pop() {
            return None;
        }
    }
}
