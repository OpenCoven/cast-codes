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

/// Subset of [`Substrate`] that only the host (`crates/ai` / `app/src`) can
/// know about — editor focus, open terminal panes, and recent LSP
/// diagnostics. [`SubstrateCollector::collect`] fills in everything else
/// (shell CWD, git branch, Comux panes) and merges the latest
/// `HostSubstrate` snapshot in on top.
///
/// The host pushes updates via [`crate::runtime::set_host_substrate`]
/// whenever its state changes (focus event, tab open/close, LSP diagnostic
/// arrival). cast_agent doesn't reach into the host — pushing keeps the
/// async boundary clean and means cast_agent never blocks the UI thread.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HostSubstrate {
    pub active_file: Option<PathBuf>,
    pub open_panes: Vec<PaneInfo>,
    pub recent_errors: Vec<DiagnosticEntry>,
}

impl Substrate {
    /// Overwrite the host-owned fields on `self` with values from
    /// `host`. Fields cast_agent populates itself (`shell_cwd`,
    /// `git_branch`, `comux_panes`) are preserved.
    pub fn apply_host(&mut self, host: HostSubstrate) {
        self.active_file = host.active_file;
        self.open_panes = host.open_panes;
        self.recent_errors = host.recent_errors;
    }
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

    /// Collect a minimal substrate snapshot (shell CWD + git branch).
    /// The host (`crates/ai`) is expected to merge in `active_file`,
    /// `open_panes`, and `recent_errors` from its own editor state before
    /// sending; this collector does not yet expose a dedicated merge helper.
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

/// Read the current branch by walking up from `cwd` looking for `.git`,
/// handling both a `.git` directory (regular repo) and a `.git` file
/// containing `gitdir: <path>` (worktrees, submodules). Returns `None` for
/// detached HEAD or non-git dirs. Never shells out to git.
fn detect_git_branch(cwd: &std::path::Path) -> Option<String> {
    let mut dir = cwd.to_path_buf();
    loop {
        let git_entry = dir.join(".git");
        if git_entry.is_dir() {
            return read_branch_from_head(&git_entry.join("HEAD"));
        }
        if git_entry.is_file() {
            let content = std::fs::read_to_string(&git_entry).ok()?;
            let gitdir = content
                .lines()
                .find_map(|l| l.strip_prefix("gitdir:"))?
                .trim();
            let gitdir_path = std::path::PathBuf::from(gitdir);
            let resolved = if gitdir_path.is_absolute() {
                gitdir_path
            } else {
                dir.join(gitdir_path)
            };
            return read_branch_from_head(&resolved.join("HEAD"));
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn read_branch_from_head(head: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(head).ok()?;
    let trimmed = content.trim();
    trimmed
        .strip_prefix("ref: refs/heads/")
        .map(|s| s.to_string())
}
