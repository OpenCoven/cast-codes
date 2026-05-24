use crate::context_chips::display_chip::GitLineChanges;
use crate::context_chips::{git_line_changes_from_chips, ContextChipKind};
use crate::terminal::TerminalView;
use warpui::AppContext;

/// Tab indicator label that knows about worktrees (PRODUCT.md 16–19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLabel {
    /// `Some(slug)` only when CWD is in a non-main worktree.
    pub worktree_slug: Option<String>,
    /// Branch name or short SHA when detached. May be empty.
    pub branch_or_sha: String,
    /// True when the worktree path no longer exists on disk (PRODUCT.md 19).
    pub missing: bool,
}

/// Pure helper for unit-testing the label computation.
///
/// `git_dir` is the per-worktree gitdir (e.g. `/repo/.git/worktrees/feature-a`).
/// `common_dir` is the shared gitdir (e.g. `/repo/.git`).
/// When they're equal, the CWD is in the main worktree.
fn compute_git_label_from_paths(
    cwd: &std::path::Path,
    branch: Option<String>,
    git_dir: &std::path::Path,
    common_dir: &std::path::Path,
    cwd_exists: bool,
) -> GitLabel {
    let is_main = git_dir == common_dir;
    let slug = if is_main {
        None
    } else {
        cwd.file_name().map(|s| s.to_string_lossy().to_string())
    };
    GitLabel {
        worktree_slug: slug,
        branch_or_sha: branch.unwrap_or_default(),
        missing: !cwd_exists,
    }
}

impl TerminalView {
    fn prompt_chip_value(&self, chip_kind: &ContextChipKind, ctx: &AppContext) -> Option<String> {
        self.current_prompt
            .as_ref(ctx)
            .latest_chip_value(chip_kind, ctx)
            .map(|v| v.to_string())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn display_working_directory(&self, ctx: &AppContext) -> Option<String> {
        let raw = self
            .prompt_chip_value(&ContextChipKind::WorkingDirectory, ctx)
            .or_else(|| self.pwd())?;
        let home_dir = self
            .active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
            .and_then(|session| session.home_dir().map(str::to_owned));
        Some(warp_util::path::user_friendly_path(&raw, home_dir.as_deref()).to_string())
    }

    pub fn terminal_title_from_shell(&self) -> String {
        let model = self.model.lock();
        let fallback_title = model.shell_launch_state().display_name().to_owned();
        model
            .terminal_title()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or(fallback_title)
    }

    #[cfg_attr(not(feature = "local_fs"), allow(clippy::unnecessary_lazy_evaluations))]
    pub fn current_git_branch(&self, ctx: &AppContext) -> Option<String> {
        self.prompt_chip_value(&ContextChipKind::ShellGitBranch, ctx)
            .or_else(|| {
                #[cfg(feature = "local_fs")]
                {
                    self.git_status_metadata(ctx)
                        .map(|metadata| metadata.current_branch_name.clone())
                        .filter(|branch| !branch.trim().is_empty())
                }
                #[cfg(not(feature = "local_fs"))]
                {
                    None
                }
            })
    }

    pub fn last_completed_command_text(&self) -> Option<String> {
        let model = self.model.lock();
        model.block_list().blocks().iter().rev().find_map(|block| {
            if block.finished()
                && !block.is_background()
                && !block.is_static()
                && (block.bootstrap_stage().is_done() || block.is_restored())
            {
                let cmd = block.command_to_string();
                if cmd.trim().is_empty() {
                    None
                } else {
                    Some(cmd)
                }
            } else {
                None
            }
        })
    }

    pub fn terminal_title_text(&self) -> String {
        if !self.terminal_title.trim().is_empty() {
            return self.terminal_title.clone();
        }
        self.terminal_title_from_shell()
    }

    pub fn current_pull_request_url(&self, ctx: &AppContext) -> Option<String> {
        self.current_prompt
            .as_ref(ctx)
            .latest_chip_value(&ContextChipKind::GithubPullRequest, ctx)
            .map(|v| v.to_string())
            .filter(|value| !value.trim().is_empty())
    }

    /// Tab indicator label (PRODUCT.md 16–19). Returns `None` when the pane
    /// has no CWD or is outside any git repository.
    ///
    /// Uses `pwd()` for the raw filesystem path (avoids `~`-expansion issues
    /// that `display_working_directory` would introduce).
    #[cfg_attr(not(feature = "local_fs"), allow(unused_variables))]
    pub fn current_git_label(&self, ctx: &AppContext) -> Option<GitLabel> {
        let cwd_str = self.pwd()?;
        let cwd_path = std::path::PathBuf::from(&cwd_str);
        #[cfg(feature = "local_fs")]
        {
            let (git_dir, common_dir) =
                crate::util::git::detect_git_dirs_sync(&cwd_path)?;
            let branch = self.current_git_branch(ctx);
            let exists = cwd_path.exists();
            Some(compute_git_label_from_paths(
                &cwd_path,
                branch,
                &git_dir,
                &common_dir,
                exists,
            ))
        }
        #[cfg(not(feature = "local_fs"))]
        {
            None
        }
    }

    #[cfg_attr(not(feature = "local_fs"), allow(clippy::unnecessary_lazy_evaluations))]
    pub fn current_diff_line_changes(&self, ctx: &AppContext) -> Option<GitLineChanges> {
        // Prefer the filesystem-event-based GitRepoStatusModel (which includes
        // untracked files) over parsing the raw shell chip output. This matches
        // the preference order used by the prompt chip display (display.rs) and
        // agent footer (chips.rs).
        #[cfg(feature = "local_fs")]
        let from_model = self
            .git_status_metadata(ctx)
            .map(|metadata| GitLineChanges::from_diff_stats(&metadata.stats_against_head));
        #[cfg(not(feature = "local_fs"))]
        let from_model: Option<GitLineChanges> = None;

        from_model
            .or_else(|| {
                git_line_changes_from_chips(&self.current_prompt.as_ref(ctx).agent_view_chips(ctx))
            })
            .filter(|line_changes| {
                line_changes.files_changed > 0
                    || line_changes.lines_added > 0
                    || line_changes.lines_removed > 0
            })
    }
}

#[cfg(test)]
mod git_label_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn label_main_worktree_has_no_slug() {
        let label = compute_git_label_from_paths(
            &PathBuf::from("/repo"),
            Some("main".to_string()),
            &PathBuf::from("/repo/.git"),
            &PathBuf::from("/repo/.git"),
            true,
        );
        assert_eq!(label.worktree_slug, None);
        assert_eq!(label.branch_or_sha, "main");
        assert!(!label.missing);
    }

    #[test]
    fn label_non_main_worktree_has_slug() {
        let label = compute_git_label_from_paths(
            &PathBuf::from("/repo/.castcodes/worktrees/feature-a"),
            Some("feature/a".to_string()),
            &PathBuf::from("/repo/.git/worktrees/feature-a"),
            &PathBuf::from("/repo/.git"),
            true,
        );
        assert_eq!(label.worktree_slug.as_deref(), Some("feature-a"));
        assert_eq!(label.branch_or_sha, "feature/a");
        assert!(!label.missing);
    }

    #[test]
    fn label_detached_no_branch_returns_empty_branch_field() {
        let label = compute_git_label_from_paths(
            &PathBuf::from("/repo/.castcodes/worktrees/detached"),
            None,
            &PathBuf::from("/repo/.git/worktrees/detached"),
            &PathBuf::from("/repo/.git"),
            true,
        );
        assert_eq!(label.worktree_slug.as_deref(), Some("detached"));
        assert!(label.branch_or_sha.is_empty());
    }

    #[test]
    fn label_missing_worktree_sets_flag() {
        let label = compute_git_label_from_paths(
            &PathBuf::from("/repo/.castcodes/worktrees/gone"),
            Some("gone".to_string()),
            &PathBuf::from("/repo/.git/worktrees/gone"),
            &PathBuf::from("/repo/.git"),
            false,
        );
        assert!(label.missing);
        assert_eq!(label.worktree_slug.as_deref(), Some("gone"));
    }
}
