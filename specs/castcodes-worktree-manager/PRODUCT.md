# Worktree Manager

## Summary

A first-class way to create, open, and remove multiple git worktrees of the same repository from inside CastCodes, plus a per-pane indicator so the user always knows which worktree a pane is running in. Gives the user fully concurrent multi-branch workflows in one window without leaving the terminal, in the spirit of how the Codex agent stages each task in its own worktree.

## MVP scope (what ships behind the flag in this PR)

Discovery during implementation revealed that CastCodes already has `WorkspaceAction::OpenNewWorktreeModal` (a working "create worktree" UI) and `WorkspaceAction::OpenWorktreeInRepo { repo_path }` (a working "open existing worktree" handler). To avoid duplication, this PR ships:

- The per-pane `<slug> · <branch>` tab indicator for any non-main worktree, regardless of how it was created (invariants 16–21, 23). **Already implemented in Phase 1.**
- The `WorktreeManagerSettings` group with `staging_directory` and `prune_on_remove` knobs (22).
- Three palette entries (2 and 4):
  - **"New worktree…"** dispatches the existing `OpenNewWorktreeModal`. Defers to whatever default branch / staging convention that modal already uses; our `staging_directory` setting (22) is **not honored by it in this PR**.
  - **"Open worktree in repo…"** opens a small `FilterableDropdown` modal listing all worktrees of the active pane's repo (rows from `list_worktrees`), then dispatches the existing `OpenWorktreeInRepo` with the picked path. Matches invariants 11, 12.
  - **"Remove worktree…"** shows an error toast "Coming in a follow-up" when invoked, so it's discoverable but inert. Real removal flow (15) is deferred.
- The feature flag `WorktreeManager` gates all three entries (1).

Deferred to a follow-up PR:
- `NewWorktreeFromBranch` action variant and branch sub-picker (5–10). Until we have a real reason to bypass `OpenNewWorktreeModal`, we don't ship a parallel create flow.
- `RemoveWorktree` real flow + force-escalation modal + `RemoveWorktreeConfirmationDialog` (15). The action variant exists in the enum but its handler only toasts the deferral message.
- `PruneWorktree` real flow (13). Variant exists; handler toasts.
- "Disabled outside a git repo" palette subtitle (3). MVP shows the entries unconditionally; the picker handler shows the error toast if the active pane is not in a repo.
- Integration tests (Tasks 19–28 in PLAN.md).
- Staging-directory setting wiring into the create flow (23). The setting parses correctly and is unit-tested; no path actually consumes it in this PR.

Everything below this section describes the **target** behavior. Anything not in the MVP-scope list above is target-only for this PR and tracked as follow-up work.

## Figma

Figma: none provided.

## Behavior

### Feature gating

1. The feature is gated behind a runtime feature flag (`WorktreeManager`). When the flag is off, none of the palette entries, settings, or tab indicators described below appear, and no new files or directories are created under the user's repositories. Turning the flag off after use does not delete worktrees that were already created.

### Command palette entries

2. When the flag is on and the active pane's current working directory is inside a git repository, the command palette lists three entries in Command mode:
   - **New worktree from branch…**
   - **Open worktree in new tab…**
   - **Remove worktree…**

3. When the active pane is not inside any git repository, the same three entries are still visible in the palette but show a subtitle "No git repository in this pane" and are not selectable. They do not silently disappear, so the user can tell the feature exists.

4. The palette entries are discoverable by fuzzy-search for the words "worktree", "branch", "git", or the entry titles themselves.

### "New worktree from branch…" flow

5. Selecting **New worktree from branch…** opens a sub-picker listing branches in the current pane's repository:
   - Local branches first, then remote-tracking branches not already local.
   - The currently checked-out branch of any existing worktree is shown with a small "in use" tag and is selectable but leads to the conflict path described in (10).
   - The user may also type a name that doesn't match an existing branch; this is treated as "create a new branch from current HEAD with this name".
   - Sorted by last commit date, descending.

6. After the user picks a branch (or types a new name), a worktree directory is created at `<repo-root>/.castcodes/worktrees/<branch-slug>/` and the chosen branch is checked out into it.
   - `<branch-slug>` is derived from the branch name by replacing `/`, whitespace, and other shell-unsafe characters with `-`, collapsing runs of `-`, and trimming leading/trailing `-`. For example, `feature/foo bar` → `feature-foo-bar`.
   - If a directory at the computed path already exists, the slug is suffixed with `-2`, `-3`, etc. until a free path is found. The slug shown in the tab indicator reflects the suffixed name.
   - For a brand-new branch name, the branch is created at the current HEAD of the pane's repository.

7. On success a new tab is opened with its initial working directory set to the new worktree path, and that tab is focused. The tab's title reflects the worktree (see (16)–(18)).

8. While the worktree is being created the palette closes immediately and a short pending toast appears ("Creating worktree `<slug>`…"). On success the toast is replaced with a success toast; on failure the toast is replaced with an error toast that contains the verbatim stderr from `git`.

9. If creation fails after the toast is shown, no new tab is opened and no leftover directory is left under `.castcodes/worktrees/`.

10. **Conflicts and errors** are surfaced to the user rather than silently swallowed:
    - Branch already checked out in another worktree → error toast names the other worktree path.
    - Target directory not writable or disk full → error toast with `git` output.
    - Target directory is non-empty (e.g. user manually created the path) → error toast with `git` output.

### "Open worktree in new tab…" flow

11. Selecting **Open worktree in new tab…** opens a sub-picker listing every worktree of the current repository, including the main worktree. Each row shows: worktree slug (or "main"), checked-out branch or detached HEAD short SHA, and a status tag of `locked`, `prunable`, `detached`, or `bare` when applicable.

12. Selecting a non-prunable row opens a new tab with its initial working directory set to that worktree path, and focuses it. Selecting the main worktree opens a new tab at the repository root.

13. Selecting a row tagged `prunable` does not open a tab; it instead asks the user to confirm pruning. Confirming removes the stale administrative entry (equivalent to `git worktree prune` filtered to that path) and refreshes the list. Cancelling leaves the entry as-is.

14. The picker reflects the state of `git worktree list` at the moment it is opened. A worktree created from another CastCodes window or from an external shell appears the next time the picker is opened.

### "Remove worktree…" flow

15. Selecting **Remove worktree…** opens a sub-picker listing every worktree of the current repository **except** the main worktree (which cannot be removed via this UI). After picking, a confirmation modal shows the worktree path, branch, and whether removal will be forced.
    - The default attempts a non-forced remove. If `git worktree remove` reports a dirty tree, the modal re-appears with the message and a **Force remove** button.
    - Removal does not close or kill any pane whose CWD is inside that worktree. After removal, the still-open pane's shell remains at the now-orphaned path; the tab indicator shows a strikethrough or "missing" treatment (see (19)).
    - When the `prune_on_remove` setting is true, removal also deletes any residual files in the worktree directory (the directory itself is removed by `git worktree remove`; this setting only matters if a forced remove left files behind).

### Per-pane worktree indicator

16. When a pane's current working directory is inside the **main** worktree of a repository, the tab indicator shows the branch name only — identical to today's behavior.

17. When a pane's current working directory is inside a **non-main** worktree of a repository, the tab indicator shows `<worktree-slug> · <branch>`. The slug is the directory name of the worktree under `.castcodes/worktrees/` (or whatever the worktree's directory name is, if it was created outside `.castcodes/worktrees/`).

18. When a pane is on a detached HEAD inside a worktree, the indicator shows `<worktree-slug> · <short-sha>`.

19. If a pane's CWD points at a worktree that has been removed underneath it, the indicator shows the last known slug with a visual "missing" treatment (e.g. strikethrough or muted color) until the user `cd`s elsewhere.

20. The indicator updates whenever the pane's CWD changes (the existing CWD-detection path), without the user having to refocus the tab.

21. When a pane is outside any git repository, no worktree indicator is shown — identical to today's behavior.

### Settings

22. Two user-overridable settings exist under a `worktree_manager` group:
    - `staging_directory`: path template for new worktrees. Defaults to `<repo-root>/.castcodes/worktrees`. Accepts `<repo-root>` and `<branch-slug>` placeholders. May be absolute or relative to repo root. If unset or empty, the default applies.
    - `prune_on_remove`: boolean, defaults to false. See (15).

23. Changing `staging_directory` does not move existing worktrees. Newly created worktrees use the new value; the indicator (17) still derives its slug from the actual directory name on disk.

### Keyboard, focus, and accessibility

24. All three palette entries, both sub-pickers, and the confirmation modal are keyboard-operable end-to-end: arrow keys to navigate, Enter to confirm, Esc to cancel.

25. The branch picker (5) and worktree picker (11, 15) support fuzzy search over the visible row text.

### Cross-pane and cross-instance behavior

26. Two panes may have their CWD inside the same worktree at the same time. Each pane independently shows the same indicator. Removing that worktree affects both panes per (19).

27. Two panes in the same window may simultaneously have their CWD inside two different worktrees of the same repository, with no interaction between them. Switching focus between panes updates which tab is highlighted; it does not change any pane's underlying CWD.

28. The feature does not depend on or modify any global git config; `.castcodes/worktrees/` directories are not added to the user's global gitignore. If the repository does not already ignore that path, the user is free to add it themselves; the feature does not silently edit the repository's `.gitignore`.

### Platforms

29. On the web/WASM build (where local filesystem access is unavailable), the feature flag has no effect: palette entries do not appear, and the tab indicator falls back to today's branch-only behavior. The setting keys are still accepted in config files for parity but are inert.

### Out of scope (this iteration)

30. Auto-isolating AI agent runs into per-task worktrees (the Codex auto-isolation pattern) is intentionally not part of this feature. The primitives added here can be reused by a later agent-isolation feature without modification.

31. Persisting which pane was bound to which worktree across app restarts via the session model is intentionally not part of this feature. A new tab opened via this feature is a normal tab whose CWD happens to be a worktree path; it persists through session restore exactly as any other tab does.

32. The feature does not replace, hide, or wrap the user's existing ability to run `git worktree` commands directly in the shell. Those continue to work, and worktrees created that way are listed by the picker in (11) and removed by (15) just like UI-created ones.
