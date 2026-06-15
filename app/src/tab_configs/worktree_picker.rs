use std::path::PathBuf;

use warpui::{
    elements::{ChildView, Element},
    r#async::SpawnedFutureHandle,
    ui_components::components::UiComponentStyles,
    AppContext, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::{
    tab_configs::PickerStyle,
    util::worktree::{list_worktrees, WorktreeInfo},
    view_components::{DropdownItem, FilterableDropdown},
};

const DEFAULT_DROPDOWN_WIDTH: f32 = 380.;
/// Placeholder shown while the async `git worktree list` is in flight.
const LOADING_PLACEHOLDER: &str = "Fetching worktrees\u{2026}";
/// Placeholder shown when the repo has no listable worktrees (or the fetch failed).
const EMPTY_PLACEHOLDER: &str = "No worktrees found";

/// Action emitted by the inner `FilterableDropdown` and consumed by
/// [`WorktreePicker::handle_action`].
///
/// `PathBuf` is carried directly so the parent (Workspace) does not need to
/// re-resolve a row-label-to-path mapping on selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreePickerAction {
    Select(PathBuf),
}

/// Events emitted by [`WorktreePicker`] for the owning workspace to observe.
pub enum WorktreePickerEvent {
    /// The user picked a worktree row. Workspace should dispatch
    /// `OpenWorktreeInRepo { repo_path }` and close the surrounding modal.
    Confirm { worktree_path: PathBuf },
}

/// A filterable dropdown listing the worktrees of a given git repository.
///
/// Items come from `crate::util::worktree::list_worktrees`. Each row's title
/// is the directory slug (or `"main"` for the main worktree); the surrounding
/// label includes the branch (or short SHA when detached) and any status tags
/// (`locked`, `prunable`, `detached`, `bare`).
///
/// On select, emits [`WorktreePickerEvent::Confirm`] with the chosen
/// worktree's filesystem path.
pub struct WorktreePicker {
    dropdown: ViewHandle<FilterableDropdown<WorktreePickerAction>>,
    /// Repository root the current items belong to. `None` until
    /// `set_repo_root` is called the first time.
    repo_root: Option<PathBuf>,
    /// Monotonically increasing counter incremented on every fetch. The
    /// spawn callback compares against the captured epoch and discards
    /// stale results, mirroring `BranchPicker::fetch_branches`.
    fetch_epoch: u64,
    /// True while an async fetch is in flight; the dropdown is disabled
    /// during this window so the user cannot interact with the placeholder.
    is_loading: bool,
    /// Handle for the active worktree fetch, if any. Replaced fetches are
    /// aborted so rapid repo changes do not leave stale `git worktree` work running.
    fetch_handle: Option<SpawnedFutureHandle>,
}

impl WorktreePicker {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        Self::new_with_style(None, ctx)
    }

    pub fn new_with_style(style: Option<PickerStyle>, ctx: &mut ViewContext<Self>) -> Self {
        let width = style.as_ref().map_or(DEFAULT_DROPDOWN_WIDTH, |s| s.width);
        let bg = style.and_then(|s| s.background);
        let dropdown = ctx.add_typed_action_view(|ctx| {
            let mut dropdown = FilterableDropdown::new(ctx);
            dropdown.set_top_bar_max_width(width);
            dropdown.set_menu_width(width, ctx);
            if let Some(bg) = bg {
                dropdown.set_style(UiComponentStyles {
                    background: Some(bg.into()),
                    ..Default::default()
                });
            }
            dropdown
        });

        Self {
            dropdown,
            repo_root: None,
            fetch_epoch: 0,
            is_loading: false,
            fetch_handle: None,
        }
    }

    /// Sets (or replaces) the repository whose worktrees are listed. Kicks
    /// off an async fetch and immediately shows a loading placeholder so
    /// the dropdown is never blank.
    pub fn set_repo_root(&mut self, repo_root: PathBuf, ctx: &mut ViewContext<Self>) {
        self.repo_root = Some(repo_root.clone());
        self.refresh(ctx);
    }

    /// Re-runs the worktree fetch against the currently-set `repo_root`.
    ///
    /// No-op if `set_repo_root` has never been called.
    pub fn refresh(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            return;
        };

        self.is_loading = true;
        self.dropdown.update(ctx, |dropdown, ctx| {
            dropdown.set_disabled(ctx);
            // Use a sentinel `PathBuf` for the placeholder. It is never
            // dispatched because we treat the loading state separately and
            // the user cannot interact with a disabled dropdown.
            let placeholder = DropdownItem::new(
                LOADING_PLACEHOLDER.to_string(),
                WorktreePickerAction::Select(PathBuf::new()),
            );
            dropdown.set_items(vec![placeholder], ctx);
            dropdown.set_selected_by_name(LOADING_PLACEHOLDER, ctx);
        });

        if let Some(handle) = self.fetch_handle.take() {
            handle.abort();
        }

        self.fetch_epoch = self.fetch_epoch.wrapping_add(1);
        let epoch = self.fetch_epoch;

        self.fetch_handle = Some(ctx.spawn(
            async move { list_worktrees(&repo_root).await },
            move |me, result, ctx| {
                // Discard stale results (a newer fetch has been started).
                if me.fetch_epoch != epoch {
                    return;
                }

                me.fetch_handle = None;
                me.is_loading = false;
                me.dropdown.update(ctx, |dropdown, ctx| {
                    dropdown.set_enabled(ctx);
                });

                let infos = match result {
                    Ok(infos) => infos,
                    Err(err) => {
                        log::warn!("WorktreePicker: failed to list worktrees: {err}");
                        Vec::new()
                    }
                };

                let rows = format_worktree_rows(&infos);
                let items: Vec<DropdownItem<WorktreePickerAction>> = rows
                    .into_iter()
                    .map(|(label, path)| {
                        DropdownItem::new(label, WorktreePickerAction::Select(path))
                    })
                    .collect();

                me.dropdown.update(ctx, |dropdown, ctx| {
                    if items.is_empty() {
                        // Show an explicit empty-state row so the user gets
                        // feedback instead of a silent blank menu.
                        let placeholder = DropdownItem::new(
                            EMPTY_PLACEHOLDER.to_string(),
                            WorktreePickerAction::Select(PathBuf::new()),
                        );
                        dropdown.set_items(vec![placeholder], ctx);
                        dropdown.set_selected_by_name(EMPTY_PLACEHOLDER, ctx);
                        dropdown.set_disabled(ctx);
                    } else {
                        dropdown.set_items(items, ctx);
                    }
                });

                ctx.notify();
            },
        ));
    }

    pub fn toggle_dropdown(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        self.dropdown.update(ctx, |dropdown, ctx| {
            dropdown.toggle_expanded(ctx);
        });
        self.dropdown.as_ref(ctx).is_expanded()
    }

    /// True while an async `list_worktrees` fetch is in flight.
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }
}

impl Entity for WorktreePicker {
    type Event = WorktreePickerEvent;
}

impl View for WorktreePicker {
    fn ui_name() -> &'static str {
        "WorktreePicker"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.dropdown).finish()
    }
}

impl TypedActionView for WorktreePicker {
    type Action = WorktreePickerAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            WorktreePickerAction::Select(path) => {
                // Defensive: ignore the synthetic placeholder rows.
                if path.as_os_str().is_empty() {
                    return;
                }
                ctx.emit(WorktreePickerEvent::Confirm {
                    worktree_path: path.clone(),
                });
            }
        }
    }
}

/// Returns the display slug for a single worktree: `"main"` for the main
/// worktree (matching git's terminology), otherwise the path's final
/// component. Falls back to the full path string if there is no filename.
fn worktree_slug(info: &WorktreeInfo) -> String {
    if info.is_main {
        return "main".to_string();
    }
    info.path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| info.path.display().to_string())
}

/// Returns the status tags for a worktree, in display order.
fn worktree_status_tags(info: &WorktreeInfo) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if info.is_locked {
        tags.push("locked");
    }
    if info.is_prunable {
        tags.push("prunable");
    }
    if info.branch.is_none() && !info.is_bare {
        tags.push("detached");
    }
    if info.is_bare {
        tags.push("bare");
    }
    tags
}

/// Build a single human-readable label for a worktree row.
///
/// Format: `"<slug> — <branch-or-sha> [tag tag …]"`. The em-dash and bracket
/// segments are only added when the corresponding data exists.
fn format_worktree_label(info: &WorktreeInfo) -> String {
    let slug = worktree_slug(info);
    let detail = info.branch.clone().or_else(|| {
        let head = info.head.trim();
        (!head.is_empty()).then(|| head.to_string())
    });
    let tags = worktree_status_tags(info);

    let mut label = slug;
    if let Some(detail) = detail {
        label.push_str(" \u{2014} ");
        label.push_str(&detail);
    }
    if !tags.is_empty() {
        label.push_str(" [");
        label.push_str(&tags.join(" "));
        label.push(']');
    }
    label
}

/// Format a slice of `WorktreeInfo`s into `(label, path)` rows suitable for
/// populating a `FilterableDropdown`.
///
/// Extracted as a free function so the formatting is unit-testable without
/// constructing a `View`.
pub fn format_worktree_rows(infos: &[WorktreeInfo]) -> Vec<(String, PathBuf)> {
    infos
        .iter()
        .map(|info| (format_worktree_label(info), info.path.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(
        path: &str,
        branch: Option<&str>,
        head: &str,
        is_main: bool,
        is_locked: bool,
        is_prunable: bool,
        is_bare: bool,
    ) -> WorktreeInfo {
        WorktreeInfo {
            path: PathBuf::from(path),
            branch: branch.map(str::to_string),
            head: head.to_string(),
            is_main,
            is_locked,
            is_prunable,
            is_bare,
        }
    }

    #[test]
    fn main_worktree_uses_main_slug_and_branch_label() {
        let i = info(
            "/repos/myrepo",
            Some("main"),
            "abc1234",
            true,
            false,
            false,
            false,
        );
        assert_eq!(format_worktree_label(&i), "main \u{2014} main");
    }

    #[test]
    fn secondary_worktree_uses_dirname_slug_with_branch() {
        let i = info(
            "/repos/myrepo/.castcodes/worktrees/feat-x",
            Some("feat-x"),
            "deadbee",
            false,
            false,
            false,
            false,
        );
        assert_eq!(format_worktree_label(&i), "feat-x \u{2014} feat-x");
    }

    #[test]
    fn detached_worktree_shows_short_sha_and_detached_tag() {
        let i = info(
            "/repos/myrepo/.castcodes/worktrees/hotfix",
            None,
            "abc1234",
            false,
            false,
            false,
            false,
        );
        assert_eq!(
            format_worktree_label(&i),
            "hotfix \u{2014} abc1234 [detached]"
        );
    }

    #[test]
    fn locked_and_prunable_tags_appear_in_brackets() {
        let i = info(
            "/repos/myrepo/.castcodes/worktrees/old",
            Some("old"),
            "1111111",
            false,
            true,
            true,
            false,
        );
        assert_eq!(
            format_worktree_label(&i),
            "old \u{2014} old [locked prunable]"
        );
    }

    #[test]
    fn bare_worktree_shows_bare_tag_without_detached() {
        let i = info("/repos/bare-mirror", None, "", true, false, false, true);
        assert_eq!(format_worktree_label(&i), "main [bare]");
    }

    #[test]
    fn format_worktree_rows_pairs_label_and_path() {
        let infos = vec![
            info(
                "/repos/myrepo",
                Some("main"),
                "abc1234",
                true,
                false,
                false,
                false,
            ),
            info(
                "/repos/myrepo/.castcodes/worktrees/feat-x",
                Some("feat-x"),
                "deadbee",
                false,
                false,
                false,
                false,
            ),
        ];
        let rows = format_worktree_rows(&infos);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "main \u{2014} main");
        assert_eq!(rows[0].1, PathBuf::from("/repos/myrepo"));
        assert_eq!(rows[1].0, "feat-x \u{2014} feat-x");
        assert_eq!(
            rows[1].1,
            PathBuf::from("/repos/myrepo/.castcodes/worktrees/feat-x")
        );
    }

    #[test]
    fn empty_input_produces_empty_rows() {
        let rows = format_worktree_rows(&[]);
        assert!(rows.is_empty());
    }

    #[test]
    fn slug_falls_back_to_full_path_when_no_filename() {
        let i = info("/", Some("main"), "abc1234", false, false, false, false);
        // file_name() returns None for "/", so we fall back to the path display.
        let slug = worktree_slug(&i);
        assert_eq!(slug, "/");
    }
}
