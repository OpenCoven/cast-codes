# Worktree Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Dogfood-flagged Worktree Manager feature that lets a CastCodes user create, open, and remove additional git worktrees from the command palette, with a per-pane `<worktree> · <branch>` indicator, per `specs/castcodes-worktree-manager/PRODUCT.md` and `TECH.md`.

**Architecture:** New `app/src/util/worktree.rs` wraps `git worktree`; new `WorkspaceAction` variants and palette entries call it; `TerminalView::current_git_label` extends the existing branch-display path; one new `FeatureFlag` and one settings group gate everything.

**Tech Stack:** Rust, async (existing `run_git_command` plumbing in `app/src/util/git.rs`), CastCodes' typed-action framework, `define_settings_group!` macro, `crates/integration` Builder test framework.

---

## Conventions for this plan

- **Branch:** every task lands on `feat/worktree-manager` (already created and tracking `specs/castcodes-worktree-manager/`). Sub-agent branches per TECH.md §Parallelization rebase onto it.
- **Commits:** every commit signed (`git commit -S …`) per global rule. Verify with `git log -1 --show-signature` before pushing.
- **Tests first:** every task that produces code starts with a failing test, then the minimal code to pass.
- **Cargo target:** unless otherwise specified, build/test with `cargo test -p app` (or `-p warp_features`, `-p settings`, `-p integration` for crate-specific tasks). The Warp dev loop uses these aliases.
- **`local_fs` gating:** every new fn that shells out goes behind `#[cfg(feature = "local_fs")]`, with a `#[cfg(not(feature = "local_fs"))]` stub that returns `Err(anyhow!("Not supported on wasm"))`. Mirror `app/src/util/git.rs`.

## File structure

**New files:**

| Path | Responsibility |
| --- | --- |
| `app/src/util/worktree.rs` | Pure-function helpers + async `git worktree` wrappers. No UI/state. |
| `app/src/util/worktree_tests.rs` | Unit tests for `worktree.rs`. |
| `app/src/util/worktree_test_fixtures/single_main.txt` | `git worktree list --porcelain` capture: main only. |
| `app/src/util/worktree_test_fixtures/main_plus_one.txt` | Capture: main + one feature worktree. |
| `app/src/util/worktree_test_fixtures/locked.txt` | Capture: locked worktree. |
| `app/src/util/worktree_test_fixtures/prunable.txt` | Capture: prunable worktree. |
| `app/src/util/worktree_test_fixtures/detached.txt` | Capture: detached HEAD worktree. |
| `app/src/util/worktree_test_fixtures/bare.txt` | Capture: bare worktree. |
| `app/src/settings/worktree_manager.rs` | `WorktreeManagerSettings` group via `define_settings_group!`. |
| `app/src/workspace/actions/worktree_manager/mod.rs` | Module root + shared `BranchTarget`, `WorktreeOpenTarget` types. |
| `app/src/workspace/actions/worktree_manager/new_worktree.rs` | `NewWorktreeFromBranch` handler. |
| `app/src/workspace/actions/worktree_manager/open_worktree.rs` | `OpenWorktreeInTab` + `PruneWorktree` handlers. |
| `app/src/workspace/actions/worktree_manager/remove_worktree.rs` | `RemoveWorktree` handler + confirmation flow. |
| `app/src/workspace/actions/worktree_manager/pickers.rs` | Branch picker + worktree picker model. |
| `crates/integration/src/test/worktree_manager.rs` | Builder-style end-to-end tests. |

**Modified files:**

| Path | Change |
| --- | --- |
| `app/src/util/mod.rs` | `pub mod worktree;` |
| `app/src/settings/mod.rs` (or sibling registration site) | Register `WorktreeManagerSettings`. |
| `crates/warp_features/src/lib.rs` | Add `WorktreeManager` variant to `FeatureFlag` enum. |
| `app/src/workspace/action.rs` | Add 4 enum variants + supporting types. |
| `app/src/workspace/action_tests.rs` | Round-trip + `should_save_app_state_on_action` tests for new variants. |
| `app/src/workspace/actions.rs` (or wherever typed handlers register) | Register the new handler module. |
| `app/src/terminal/view/tab_metadata.rs` | Add `current_git_label`; keep `current_git_branch` as wrapper. |
| `app/src/tab.rs` | Call `current_git_label` in title/tooltip path. |
| `app/src/workspace/view/vertical_tabs.rs` | Same call-site swap as `tab.rs`. |
| `app/src/command_palette.rs` | Register 3 worktree palette entries. |
| `crates/integration/src/test/mod.rs` | `mod worktree_manager;` |

---

## Phase 1 — independent foundations (Agents A, C, D in parallel per TECH.md)

### Task 1: Add `FeatureFlag::WorktreeManager`

**Files:**
- Modify: `crates/warp_features/src/lib.rs`
- Test: `crates/warp_features/src/lib.rs` (existing in-file tests)

- [ ] **Step 1: Read current FeatureFlag enum**

Run: `rg -n "enum FeatureFlag" crates/warp_features/src/lib.rs`
Expected: hit on one line; note nearby variants like `VerticalTabs` for the pattern.

- [ ] **Step 2: Write the failing test**

Add to the existing test module in `crates/warp_features/src/lib.rs`:

```rust
#[test]
fn worktree_manager_flag_round_trip() {
    let flag = FeatureFlag::WorktreeManager;
    flag.set_enabled(true);
    assert!(flag.is_enabled());
    flag.set_enabled(false);
    assert!(!flag.is_enabled());
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p warp_features worktree_manager_flag_round_trip`
Expected: compile error — `WorktreeManager` not a variant.

- [ ] **Step 4: Add the variant**

In `crates/warp_features/src/lib.rs`, add to the `FeatureFlag` enum near `VerticalTabs`:

```rust
WorktreeManager,
```

If the enum has any match-exhaustiveness consumers in this crate, add a branch matching the existing pattern for `VerticalTabs` (search for `FeatureFlag::VerticalTabs` in this crate only — call sites in `app/` come later).

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p warp_features worktree_manager_flag_round_trip`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/warp_features/src/lib.rs
git commit -S -m "feat(warp_features): add WorktreeManager flag"
git log -1 --show-signature | head -2
```
Verify output contains `Good "..." signature`.

---

### Task 2: Declare `WorktreeManagerSettings`

**Files:**
- Create: `app/src/settings/worktree_manager.rs`
- Modify: `app/src/settings/mod.rs` (or wherever sibling groups are exported — discover via `rg -n "define_settings_group!" app/src`)
- Test: `app/src/settings/worktree_manager.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Discover settings registration site**

Run: `rg -n "define_settings_group!" app/src/`
Expected: at least one match; note the file. That's the conventional registration site; also note how the macro's output is re-exported.

- [ ] **Step 2: Write the failing test**

Create `app/src/settings/worktree_manager.rs`:

```rust
use crate::settings::WorktreeManagerSettings;

#[cfg(test)]
mod tests {
    use super::*;
    use settings::SettingsManager;

    #[test]
    fn defaults() {
        let mgr = SettingsManager::new_for_tests();
        let s = WorktreeManagerSettings::as_ref(&mgr);
        assert_eq!(s.staging_directory, None);
        assert_eq!(s.prune_on_remove, false);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p app settings::worktree_manager`
Expected: compile error — `WorktreeManagerSettings` not defined.

- [ ] **Step 4: Implement settings group**

Replace the file body (above the `#[cfg(test)]` block) with:

```rust
use settings::define_settings_group;

define_settings_group! {
    pub group WorktreeManagerSettings {
        /// Optional path template for new worktrees.
        /// Placeholders: `<repo-root>`, `<branch-slug>`.
        /// When `None` or empty, defaults to `<repo-root>/.castcodes/worktrees/<branch-slug>`.
        pub staging_directory: Option<String> = None,
        /// When `true`, force-removing a worktree also deletes any residual files.
        pub prune_on_remove: bool = false,
    }
}
```

(Adjust the import path for `define_settings_group` if the macro is re-exported elsewhere — see Step 1 result.)

- [ ] **Step 5: Register the module**

In `app/src/settings/mod.rs` (or the equivalent), add:
```rust
pub mod worktree_manager;
pub use worktree_manager::WorktreeManagerSettings;
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p app settings::worktree_manager::tests::defaults`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src/settings/worktree_manager.rs app/src/settings/mod.rs
git commit -S -m "feat(settings): add WorktreeManagerSettings group"
git log -1 --show-signature | head -2
```

---

### Task 3: `worktree.rs` skeleton + `slugify_branch`

**Files:**
- Create: `app/src/util/worktree.rs`
- Create: `app/src/util/worktree_tests.rs`
- Modify: `app/src/util/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `app/src/util/worktree_tests.rs`:

```rust
use super::worktree::slugify_branch;

#[test]
fn slugify_simple() {
    assert_eq!(slugify_branch("main"), "main");
}

#[test]
fn slugify_with_slash() {
    assert_eq!(slugify_branch("feature/foo"), "feature-foo");
}

#[test]
fn slugify_with_whitespace_and_punct() {
    assert_eq!(slugify_branch("feat/Foo Bar.baz"), "feat-foo-bar.baz");
}

#[test]
fn slugify_collapses_runs() {
    assert_eq!(slugify_branch("a///b___c"), "a-b-c");
}

#[test]
fn slugify_trims_edges() {
    assert_eq!(slugify_branch("/feature/foo/"), "feature-foo");
}

#[test]
fn slugify_empty_falls_back() {
    assert_eq!(slugify_branch("///"), "worktree");
    assert_eq!(slugify_branch(""), "worktree");
}

#[test]
fn slugify_lowercases() {
    assert_eq!(slugify_branch("Feature/UPPER"), "feature-upper");
}

#[test]
fn slugify_unicode_strips() {
    assert_eq!(slugify_branch("feat/✨sparkle"), "feat-sparkle");
}
```

- [ ] **Step 2: Create the worktree module skeleton**

Create `app/src/util/worktree.rs`:

```rust
//! Wraps `git worktree` and related path helpers.
//!
//! Mirrors `app/src/util/git.rs`: all fns that shell out are behind
//! `#[cfg(feature = "local_fs")]` with a WASM stub.

use std::path::{Path, PathBuf};

/// Convert a branch name to a filesystem-safe slug.
///
/// Rules (PRODUCT.md 6): lowercase, replace any `[^a-z0-9._-]` with `-`,
/// collapse runs of `-`, trim leading/trailing `-`. Empty result falls
/// back to `"worktree"`.
pub fn slugify_branch(branch: &str) -> String {
    let mut out = String::with_capacity(branch.len());
    let mut prev_dash = true; // start "in dash run" so leading dashes are trimmed
    for ch in branch.chars() {
        let mapped = match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '-',
        };
        if mapped == '-' {
            if !prev_dash {
                out.push('-');
                prev_dash = true;
            }
        } else {
            out.push(mapped);
            prev_dash = false;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "worktree".to_string()
    } else {
        out
    }
}

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;
```

- [ ] **Step 3: Register the module**

Add to `app/src/util/mod.rs`:
```rust
pub mod worktree;
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p app util::worktree::tests::slugify`
Expected: 8 passes.

- [ ] **Step 5: Commit**

```bash
git add app/src/util/worktree.rs app/src/util/worktree_tests.rs app/src/util/mod.rs
git commit -S -m "feat(util): add worktree module with slugify_branch"
git log -1 --show-signature | head -2
```

---

### Task 4: `default_staging_dir` + `unique_path`

**Files:**
- Modify: `app/src/util/worktree.rs`
- Modify: `app/src/util/worktree_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `app/src/util/worktree_tests.rs`:

```rust
use super::worktree::{default_staging_dir, unique_path};
use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;

#[test]
fn default_staging_dir_no_override() {
    let repo = Path::new("/work/myrepo");
    let dir = default_staging_dir(repo, "feature-a", None);
    assert_eq!(dir, PathBuf::from("/work/myrepo/.castcodes/worktrees/feature-a"));
}

#[test]
fn default_staging_dir_override_with_repo_root_placeholder() {
    let repo = Path::new("/work/myrepo");
    let dir = default_staging_dir(repo, "feature-a", Some("<repo-root>/tmp/<branch-slug>"));
    assert_eq!(dir, PathBuf::from("/work/myrepo/tmp/feature-a"));
}

#[test]
fn default_staging_dir_absolute_override() {
    let repo = Path::new("/work/myrepo");
    let dir = default_staging_dir(repo, "feature-a", Some("/scratch/<branch-slug>"));
    assert_eq!(dir, PathBuf::from("/scratch/feature-a"));
}

#[test]
fn default_staging_dir_relative_override() {
    let repo = Path::new("/work/myrepo");
    let dir = default_staging_dir(repo, "feature-a", Some("trees/<branch-slug>"));
    assert_eq!(dir, PathBuf::from("/work/myrepo/trees/feature-a"));
}

#[test]
fn default_staging_dir_empty_override_falls_back() {
    let repo = Path::new("/work/myrepo");
    let dir = default_staging_dir(repo, "feature-a", Some(""));
    assert_eq!(dir, PathBuf::from("/work/myrepo/.castcodes/worktrees/feature-a"));
}

#[test]
fn unique_path_returns_base_when_free() {
    let td = TempDir::new().unwrap();
    let base = td.path().join("feature-a");
    assert_eq!(unique_path(&base), base);
}

#[test]
fn unique_path_suffixes_on_collision() {
    let td = TempDir::new().unwrap();
    let base = td.path().join("feature-a");
    fs::create_dir(&base).unwrap();
    let got = unique_path(&base);
    assert_eq!(got, td.path().join("feature-a-2"));
}

#[test]
fn unique_path_skips_existing_suffixes() {
    let td = TempDir::new().unwrap();
    let base = td.path().join("feature-a");
    fs::create_dir(&base).unwrap();
    fs::create_dir(td.path().join("feature-a-2")).unwrap();
    fs::create_dir(td.path().join("feature-a-3")).unwrap();
    let got = unique_path(&base);
    assert_eq!(got, td.path().join("feature-a-4"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p app util::worktree::tests::default_staging_dir`
Expected: compile error — `default_staging_dir`/`unique_path` not found.

- [ ] **Step 3: Implement the helpers**

Append to `app/src/util/worktree.rs`:

```rust
/// Resolve the on-disk path a new worktree will be created at.
///
/// `override_tmpl` follows PRODUCT.md (22): supports `<repo-root>` and
/// `<branch-slug>` placeholders, absolute or relative, with `None` /
/// empty falling back to the default `<repo>/.castcodes/worktrees/<slug>`.
pub fn default_staging_dir(repo_root: &Path, slug: &str, override_tmpl: Option<&str>) -> PathBuf {
    let tmpl = override_tmpl.filter(|s| !s.is_empty());
    let resolved = match tmpl {
        None => return repo_root.join(".castcodes/worktrees").join(slug),
        Some(t) => t
            .replace("<repo-root>", &repo_root.display().to_string())
            .replace("<branch-slug>", slug),
    };
    let p = PathBuf::from(&resolved);
    if p.is_absolute() {
        p
    } else {
        repo_root.join(p)
    }
}

/// Append `-2`, `-3`, … to `base` until the path does not exist.
pub fn unique_path(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_name().unwrap_or_default().to_string_lossy().to_string();
    let mut i = 2usize;
    loop {
        let candidate = parent.join(format!("{stem}-{i}"));
        if !candidate.exists() {
            return candidate;
        }
        i += 1;
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` of `app/Cargo.toml` if not already present:

```bash
rg -n "tempfile" app/Cargo.toml || echo "MISSING — add tempfile to [dev-dependencies]"
```

If missing, add under `[dev-dependencies]`:
```toml
tempfile = "3"
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p app util::worktree::tests`
Expected: all slugify + staging-dir + unique-path tests pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/util/worktree.rs app/src/util/worktree_tests.rs app/Cargo.toml
git commit -S -m "feat(util): worktree default_staging_dir + unique_path"
git log -1 --show-signature | head -2
```

---

### Task 5: `WorktreeInfo` + porcelain parser

**Files:**
- Modify: `app/src/util/worktree.rs`
- Modify: `app/src/util/worktree_tests.rs`
- Create: `app/src/util/worktree_test_fixtures/single_main.txt`
- Create: `app/src/util/worktree_test_fixtures/main_plus_one.txt`
- Create: `app/src/util/worktree_test_fixtures/locked.txt`
- Create: `app/src/util/worktree_test_fixtures/prunable.txt`
- Create: `app/src/util/worktree_test_fixtures/detached.txt`
- Create: `app/src/util/worktree_test_fixtures/bare.txt`

- [ ] **Step 1: Capture porcelain fixtures**

Each fixture file holds the literal output of `git worktree list --porcelain` for one scenario. Create them with the contents below.

`app/src/util/worktree_test_fixtures/single_main.txt`:
```
worktree /work/myrepo
HEAD abcdef0123456789abcdef0123456789abcdef01
branch refs/heads/main

```

`app/src/util/worktree_test_fixtures/main_plus_one.txt`:
```
worktree /work/myrepo
HEAD abcdef0123456789abcdef0123456789abcdef01
branch refs/heads/main

worktree /work/myrepo/.castcodes/worktrees/feature-a
HEAD 1234567890123456789012345678901234567890
branch refs/heads/feature/a

```

`app/src/util/worktree_test_fixtures/locked.txt`:
```
worktree /work/myrepo
HEAD abcdef0123456789abcdef0123456789abcdef01
branch refs/heads/main

worktree /work/myrepo/.castcodes/worktrees/locked
HEAD 1111111111111111111111111111111111111111
branch refs/heads/locked-branch
locked release branch

```

`app/src/util/worktree_test_fixtures/prunable.txt`:
```
worktree /work/myrepo
HEAD abcdef0123456789abcdef0123456789abcdef01
branch refs/heads/main

worktree /work/myrepo/.castcodes/worktrees/gone
HEAD 2222222222222222222222222222222222222222
branch refs/heads/gone-branch
prunable gitdir file points to non-existent location

```

`app/src/util/worktree_test_fixtures/detached.txt`:
```
worktree /work/myrepo
HEAD abcdef0123456789abcdef0123456789abcdef01
branch refs/heads/main

worktree /work/myrepo/.castcodes/worktrees/detached
HEAD 3333333333333333333333333333333333333333
detached

```

`app/src/util/worktree_test_fixtures/bare.txt`:
```
worktree /work/myrepo.git
bare

worktree /work/myrepo/.castcodes/worktrees/feature-a
HEAD 1234567890123456789012345678901234567890
branch refs/heads/feature/a

```

- [ ] **Step 2: Write the failing tests**

Append to `app/src/util/worktree_tests.rs`:

```rust
use super::worktree::{parse_worktree_list_porcelain, WorktreeInfo};

const F_SINGLE: &str = include_str!("worktree_test_fixtures/single_main.txt");
const F_MAIN_PLUS_ONE: &str = include_str!("worktree_test_fixtures/main_plus_one.txt");
const F_LOCKED: &str = include_str!("worktree_test_fixtures/locked.txt");
const F_PRUNABLE: &str = include_str!("worktree_test_fixtures/prunable.txt");
const F_DETACHED: &str = include_str!("worktree_test_fixtures/detached.txt");
const F_BARE: &str = include_str!("worktree_test_fixtures/bare.txt");

#[test]
fn parse_single_main() {
    let r = parse_worktree_list_porcelain(F_SINGLE);
    assert_eq!(r.len(), 1);
    assert!(r[0].is_main);
    assert_eq!(r[0].branch.as_deref(), Some("main"));
    assert_eq!(r[0].head, "abcdef0");
    assert_eq!(r[0].path, PathBuf::from("/work/myrepo"));
}

#[test]
fn parse_main_plus_one() {
    let r = parse_worktree_list_porcelain(F_MAIN_PLUS_ONE);
    assert_eq!(r.len(), 2);
    assert!(r[0].is_main);
    assert!(!r[1].is_main);
    assert_eq!(r[1].branch.as_deref(), Some("feature/a"));
}

#[test]
fn parse_locked_flag() {
    let r = parse_worktree_list_porcelain(F_LOCKED);
    assert!(r.iter().any(|w| w.is_locked && w.branch.as_deref() == Some("locked-branch")));
}

#[test]
fn parse_prunable_flag() {
    let r = parse_worktree_list_porcelain(F_PRUNABLE);
    assert!(r.iter().any(|w| w.is_prunable && w.branch.as_deref() == Some("gone-branch")));
}

#[test]
fn parse_detached_head() {
    let r = parse_worktree_list_porcelain(F_DETACHED);
    let detached = r.iter().find(|w| w.path.ends_with("detached")).unwrap();
    assert_eq!(detached.branch, None);
    assert_eq!(detached.head, "3333333");
}

#[test]
fn parse_bare_flag() {
    let r = parse_worktree_list_porcelain(F_BARE);
    assert!(r[0].is_bare);
    assert!(r[0].branch.is_none());
}

#[test]
fn parse_main_is_first_entry() {
    let r = parse_worktree_list_porcelain(F_MAIN_PLUS_ONE);
    assert!(r[0].is_main, "first entry of `git worktree list` is always main");
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p app util::worktree::tests::parse_`
Expected: compile error — `parse_worktree_list_porcelain` and `WorktreeInfo` not defined.

- [ ] **Step 4: Implement the parser**

Append to `app/src/util/worktree.rs`:

```rust
/// Parsed entry from `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,  // refs/heads/X → X; None when detached or bare
    pub head: String,            // short SHA (first 7 chars)
    pub is_main: bool,           // true for the first entry (porcelain guarantee)
    pub is_locked: bool,
    pub is_prunable: bool,
    pub is_bare: bool,
}

/// Parse `git worktree list --porcelain` output.
///
/// The porcelain format groups entries with blank-line separators. Each
/// group starts with `worktree <path>` and may include any of:
///   HEAD <sha>
///   branch refs/heads/<name>
///   detached
///   bare
///   locked [reason...]
///   prunable [reason...]
///
/// Unknown lines are ignored defensively so a future git version that
/// adds a token cannot break listing.
pub fn parse_worktree_list_porcelain(input: &str) -> Vec<WorktreeInfo> {
    let mut out = Vec::new();
    let mut first = true;
    let mut current: Option<WorktreeInfo> = None;

    let flush = |cur: &mut Option<WorktreeInfo>, out: &mut Vec<WorktreeInfo>| {
        if let Some(w) = cur.take() {
            out.push(w);
        }
    };

    for line in input.lines() {
        if line.is_empty() {
            flush(&mut current, &mut out);
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush(&mut current, &mut out);
            current = Some(WorktreeInfo {
                path: PathBuf::from(rest),
                branch: None,
                head: String::new(),
                is_main: first,
                is_locked: false,
                is_prunable: false,
                is_bare: false,
            });
            first = false;
            continue;
        }
        let Some(w) = current.as_mut() else { continue };
        if let Some(sha) = line.strip_prefix("HEAD ") {
            w.head = sha.chars().take(7).collect();
        } else if let Some(refname) = line.strip_prefix("branch ") {
            w.branch = refname.strip_prefix("refs/heads/").map(str::to_string)
                .or_else(|| Some(refname.to_string()));
        } else if line == "detached" {
            w.branch = None;
        } else if line == "bare" {
            w.is_bare = true;
        } else if line == "locked" || line.starts_with("locked ") {
            w.is_locked = true;
        } else if line == "prunable" || line.starts_with("prunable ") {
            w.is_prunable = true;
        }
        // unknown leading tokens are ignored
    }
    flush(&mut current, &mut out);
    out
}
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test -p app util::worktree::tests::parse_`
Expected: 7 passes.

- [ ] **Step 6: Commit**

```bash
git add app/src/util/worktree.rs app/src/util/worktree_tests.rs app/src/util/worktree_test_fixtures/
git commit -S -m "feat(util): worktree porcelain parser + fixtures"
git log -1 --show-signature | head -2
```

---

### Task 6: `list_worktrees` async wrapper

**Files:**
- Modify: `app/src/util/worktree.rs`

- [ ] **Step 1: Add doc-test (compile-only)**

Append the function and a doc snippet to `app/src/util/worktree.rs`:

```rust
use anyhow::Result;

#[cfg(feature = "local_fs")]
pub async fn list_worktrees(repo: &Path) -> Result<Vec<WorktreeInfo>> {
    let out = crate::util::git::run_git_command(repo, &["worktree", "list", "--porcelain"]).await?;
    Ok(parse_worktree_list_porcelain(&out))
}

#[cfg(not(feature = "local_fs"))]
pub async fn list_worktrees(_repo: &Path) -> Result<Vec<WorktreeInfo>> {
    Err(anyhow::anyhow!("Not supported on wasm"))
}
```

- [ ] **Step 2: Build verify**

Run: `cargo check -p app --features local_fs && cargo check -p app --no-default-features`
Expected: both succeed.

- [ ] **Step 3: Add an integration-style test against a real repo**

Append to `app/src/util/worktree_tests.rs`:

```rust
#[cfg(feature = "local_fs")]
#[tokio::test]
async fn list_worktrees_round_trip_on_temp_repo() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        let s = Command::new("git").args(args).current_dir(repo).output().unwrap();
        assert!(s.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&s.stderr));
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);
    run(&["worktree", "add", "-q", ".castcodes/worktrees/feature-a", "feature/a"]);

    let list = super::worktree::list_worktrees(repo).await.unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0].is_main);
    assert!(list.iter().any(|w| w.branch.as_deref() == Some("feature/a")));
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p app --features local_fs util::worktree::tests::list_worktrees_round_trip_on_temp_repo -- --nocapture`
Expected: PASS. (Skip on machines without `git` on PATH; document in CI matrix that this is a `local_fs`-only test.)

- [ ] **Step 5: Commit**

```bash
git add app/src/util/worktree.rs app/src/util/worktree_tests.rs
git commit -S -m "feat(util): worktree list_worktrees async wrapper"
git log -1 --show-signature | head -2
```

---

### Task 7: `add_worktree`

**Files:**
- Modify: `app/src/util/worktree.rs`
- Modify: `app/src/util/worktree_tests.rs`

- [ ] **Step 1: Write the failing test**

Append to `app/src/util/worktree_tests.rs`:

```rust
#[cfg(feature = "local_fs")]
#[tokio::test]
async fn add_worktree_existing_branch() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git").args(args).current_dir(repo).status().unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);

    let target = repo.join(".castcodes/worktrees/feature-a");
    super::worktree::add_worktree(repo, &target, "feature/a", false).await.unwrap();
    assert!(target.join(".git").exists() || target.join(".git").is_file());
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn add_worktree_creates_new_branch() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git").args(args).current_dir(repo).status().unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);

    let target = repo.join(".castcodes/worktrees/brand-new");
    super::worktree::add_worktree(repo, &target, "brand-new", true).await.unwrap();
    let branches = Command::new("git").args(["branch", "--list", "brand-new"])
        .current_dir(repo).output().unwrap();
    assert!(String::from_utf8_lossy(&branches.stdout).contains("brand-new"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p app --features local_fs util::worktree::tests::add_worktree`
Expected: compile error — `add_worktree` not defined.

- [ ] **Step 3: Implement `add_worktree`**

Append to `app/src/util/worktree.rs`:

```rust
#[cfg(feature = "local_fs")]
pub async fn add_worktree(repo: &Path, target: &Path, branch: &str, create: bool) -> Result<()> {
    let target_str = target.to_string_lossy().to_string();
    let mut args: Vec<&str> = vec!["worktree", "add"];
    if create {
        args.extend_from_slice(&["-b", branch]);
        args.push(&target_str);
    } else {
        args.push(&target_str);
        args.push(branch);
    }
    crate::util::git::run_git_command(repo, &args).await?;
    Ok(())
}

#[cfg(not(feature = "local_fs"))]
pub async fn add_worktree(_repo: &Path, _target: &Path, _branch: &str, _create: bool) -> Result<()> {
    Err(anyhow::anyhow!("Not supported on wasm"))
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p app --features local_fs util::worktree::tests::add_worktree`
Expected: 2 passes.

- [ ] **Step 5: Commit**

```bash
git add app/src/util/worktree.rs app/src/util/worktree_tests.rs
git commit -S -m "feat(util): worktree add_worktree"
git log -1 --show-signature | head -2
```

---

### Task 8: `remove_worktree` + `prune_worktrees`

**Files:**
- Modify: `app/src/util/worktree.rs`
- Modify: `app/src/util/worktree_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `app/src/util/worktree_tests.rs`:

```rust
#[cfg(feature = "local_fs")]
#[tokio::test]
async fn remove_worktree_clean() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git").args(args).current_dir(repo).status().unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);
    run(&["worktree", "add", "-q", ".castcodes/worktrees/feature-a", "feature/a"]);
    let target = repo.join(".castcodes/worktrees/feature-a");

    super::worktree::remove_worktree(repo, &target, false).await.unwrap();
    assert!(!target.exists());
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn remove_worktree_dirty_requires_force() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git").args(args).current_dir(repo).status().unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);
    run(&["worktree", "add", "-q", ".castcodes/worktrees/feature-a", "feature/a"]);
    let target = repo.join(".castcodes/worktrees/feature-a");
    std::fs::write(target.join("dirty.txt"), "dirty").unwrap();

    let res = super::worktree::remove_worktree(repo, &target, false).await;
    assert!(res.is_err(), "non-force remove of dirty worktree should fail");
    super::worktree::remove_worktree(repo, &target, true).await.unwrap();
    assert!(!target.exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p app --features local_fs util::worktree::tests::remove_worktree`
Expected: compile error.

- [ ] **Step 3: Implement**

Append to `app/src/util/worktree.rs`:

```rust
#[cfg(feature = "local_fs")]
pub async fn remove_worktree(repo: &Path, target: &Path, force: bool) -> Result<()> {
    let target_str = target.to_string_lossy().to_string();
    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&target_str);
    crate::util::git::run_git_command(repo, &args).await?;
    Ok(())
}

#[cfg(not(feature = "local_fs"))]
pub async fn remove_worktree(_repo: &Path, _target: &Path, _force: bool) -> Result<()> {
    Err(anyhow::anyhow!("Not supported on wasm"))
}

#[cfg(feature = "local_fs")]
pub async fn prune_worktrees(repo: &Path) -> Result<()> {
    crate::util::git::run_git_command(repo, &["worktree", "prune"]).await?;
    Ok(())
}

#[cfg(not(feature = "local_fs"))]
pub async fn prune_worktrees(_repo: &Path) -> Result<()> {
    Err(anyhow::anyhow!("Not supported on wasm"))
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p app --features local_fs util::worktree::tests::remove_worktree`
Expected: 2 passes.

- [ ] **Step 5: Commit**

```bash
git add app/src/util/worktree.rs app/src/util/worktree_tests.rs
git commit -S -m "feat(util): worktree remove + prune"
git log -1 --show-signature | head -2
```

---

### Task 9: `GitLabel` + `current_git_label` on TerminalView

**Files:**
- Modify: `app/src/terminal/view/tab_metadata.rs`
- Test: `app/src/terminal/view/tab_metadata.rs` (inline)

- [ ] **Step 1: Read the existing function for context**

Run: `sed -n '1,80p' app/src/terminal/view/tab_metadata.rs`
Expected: see existing `display_working_directory` (around line 15) and `current_git_branch` (around line 36). Note imports and `AppContext` use.

- [ ] **Step 2: Write the failing tests**

Append to `app/src/terminal/view/tab_metadata.rs` (inside an existing `#[cfg(test)] mod tests`, or create one if absent):

```rust
#[cfg(test)]
mod tab_metadata_tests {
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
    fn label_detached_uses_sha() {
        let label = compute_git_label_from_paths(
            &PathBuf::from("/repo/.castcodes/worktrees/detached"),
            None,
            &PathBuf::from("/repo/.git/worktrees/detached"),
            &PathBuf::from("/repo/.git"),
            true,
        );
        assert_eq!(label.worktree_slug.as_deref(), Some("detached"));
        // sha-or-fallback handled by callers; the function returns empty when both None
    }

    #[test]
    fn label_missing_worktree_sets_flag() {
        let label = compute_git_label_from_paths(
            &PathBuf::from("/repo/.castcodes/worktrees/gone"),
            Some("gone".to_string()),
            &PathBuf::from("/repo/.git/worktrees/gone"),
            &PathBuf::from("/repo/.git"),
            false, // path does not exist
        );
        assert!(label.missing);
        assert_eq!(label.worktree_slug.as_deref(), Some("gone"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p app terminal::view::tab_metadata::tab_metadata_tests`
Expected: compile error — `GitLabel`, `compute_git_label_from_paths` not defined.

- [ ] **Step 4: Implement `GitLabel` + pure-helper + `current_git_label`**

Append/insert into `app/src/terminal/view/tab_metadata.rs`:

```rust
/// What the tab indicator displays for the pane's git state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLabel {
    /// `Some(slug)` only when CWD is in a non-main worktree.
    pub worktree_slug: Option<String>,
    /// Branch name (or short SHA when detached).
    pub branch_or_sha: String,
    /// True when the worktree path no longer exists on disk (PRODUCT.md 19).
    pub missing: bool,
}

/// Pure helper for unit-testing the label computation.
///
/// `git_dir`: the per-worktree gitdir (e.g. `/repo/.git/worktrees/feature-a`)
/// `common_dir`: the shared gitdir (e.g. `/repo/.git`)
/// If they're equal, the CWD is in the main worktree.
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
    /// Tab indicator label (PRODUCT.md 16–19). Returns `None` when the pane
    /// has no CWD or is outside any git repository.
    pub fn current_git_label(&self, ctx: &AppContext) -> Option<GitLabel> {
        let cwd = self.display_working_directory(ctx)?;
        let cwd_path = std::path::PathBuf::from(&cwd);
        // Resolve git_dir + common_dir via the existing git utility.
        // (Implementer note: read `app/src/util/git.rs` for the canonical
        // `detect_git_dirs(repo) -> (git_dir, common_dir)` helper. If it
        // doesn't exist yet, add it using `git rev-parse --git-dir
        // --git-common-dir` parsed line-by-line — small and worth its
        // own commit.)
        let (git_dir, common_dir) = crate::util::git::detect_git_dirs_sync(&cwd_path)?;
        let branch = self.current_git_branch(ctx);
        let exists = cwd_path.exists();
        Some(compute_git_label_from_paths(&cwd_path, branch, &git_dir, &common_dir, exists))
    }
}
```

If `detect_git_dirs_sync` does not exist in `app/src/util/git.rs`, add it (sync wrapper acceptable here because the tab render path is sync):

```rust
// In app/src/util/git.rs:
#[cfg(feature = "local_fs")]
pub fn detect_git_dirs_sync(cwd: &Path) -> Option<(PathBuf, PathBuf)> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir", "--git-common-dir"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let mut lines = s.lines();
    let git_dir = PathBuf::from(lines.next()?);
    let common_dir = PathBuf::from(lines.next()?);
    let resolve = |p: PathBuf| if p.is_absolute() { p } else { cwd.join(p) };
    Some((resolve(git_dir), resolve(common_dir)))
}
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test -p app terminal::view::tab_metadata::tab_metadata_tests`
Expected: 4 passes.

- [ ] **Step 6: Commit**

```bash
git add app/src/terminal/view/tab_metadata.rs app/src/util/git.rs
git commit -S -m "feat(terminal): add GitLabel + current_git_label"
git log -1 --show-signature | head -2
```

---

### Task 10: Wire `GitLabel` into tab + vertical_tabs render

**Files:**
- Modify: `app/src/tab.rs`
- Modify: `app/src/workspace/view/vertical_tabs.rs`

- [ ] **Step 1: Locate call sites**

Run: `rg -n "current_git_branch" app/src/tab.rs app/src/workspace/view/vertical_tabs.rs`
Expected: each call site identified.

- [ ] **Step 2: Add a render-format helper (so both call sites stay DRY)**

Add to `app/src/terminal/view/tab_metadata.rs`:

```rust
impl GitLabel {
    /// PRODUCT.md (16, 17, 18, 19): formats the indicator string.
    pub fn render(&self) -> String {
        let body = match (self.worktree_slug.as_deref(), self.branch_or_sha.as_str()) {
            (None, b) => b.to_string(),
            (Some(slug), b) if !b.is_empty() => format!("{slug} · {b}"),
            (Some(slug), _) => slug.to_string(),
        };
        if self.missing {
            // Caller decides on visual treatment; we just signal it via a marker
            // prefix. Tab renderer maps this to muted/strikethrough styling.
            format!("⌫ {body}")
        } else {
            body
        }
    }
}
```

(The marker is a placeholder — the tab renderer in the next step replaces it with proper styling.)

- [ ] **Step 3: Swap the call site in `app/src/tab.rs`**

In the title-composition area (around line 600 per the Explore agent), where `terminal_view.current_git_branch(ctx)` is read into the displayed title, replace with:

```rust
let git_indicator: Option<String> = terminal_view
    .current_git_label(ctx)
    .filter(|l| !l.branch_or_sha.is_empty() || l.worktree_slug.is_some())
    .map(|l| {
        if l.missing {
            // TODO when the design system has a strikethrough token, switch to that.
            format!("(missing) {}", l.worktree_slug.as_deref().unwrap_or(""))
        } else {
            l.render()
        }
    });
```

Then use `git_indicator` everywhere `current_git_branch(ctx).map(...)` was used. Keep behavior identical when `worktree_slug` is `None` (PRODUCT.md 16).

- [ ] **Step 4: Same swap in `app/src/workspace/view/vertical_tabs.rs`**

Run: `rg -n "current_git_branch" app/src/workspace/view/vertical_tabs.rs`

For each match, replace with the `current_git_label` pattern from Step 3.

- [ ] **Step 5: Build check**

Run: `cargo check -p app --features local_fs`
Expected: success.

- [ ] **Step 6: Visual smoke test (manual)**

Build and run the app (`./scripts/run-castcodes.sh` or the equivalent from `castcodes-dev-loop`). In a normal repo pane, confirm the tab shows the branch as before. Then:
```bash
git worktree add /tmp/wt-smoke -b smoke
cd /tmp/wt-smoke
```
in a new tab and confirm the indicator shows `wt-smoke · smoke`.

- [ ] **Step 7: Commit**

```bash
git add app/src/tab.rs app/src/workspace/view/vertical_tabs.rs app/src/terminal/view/tab_metadata.rs
git commit -S -m "feat(tab): show worktree slug alongside branch"
git log -1 --show-signature | head -2
```

---

## Phase 2 — actions, handlers, palette wiring (Agent B, after Phase 1)

### Task 11: `WorkspaceAction` variants + support types

**Files:**
- Modify: `app/src/workspace/action.rs`
- Modify: `app/src/workspace/action_tests.rs`
- Create: `app/src/workspace/actions/worktree_manager/mod.rs`

- [ ] **Step 1: Write the failing round-trip test**

Append to `app/src/workspace/action_tests.rs`:

```rust
#[test]
fn new_worktree_round_trip() {
    use crate::workspace::action::{BranchTarget, WorktreeOpenTarget};
    let action = WorkspaceAction::NewWorktreeFromBranch {
        branch: BranchTarget::Existing("feature/a".to_string()),
        open_in: WorktreeOpenTarget::NewTab,
    };
    let json = serde_json::to_string(&action).unwrap();
    let back: WorkspaceAction = serde_json::from_str(&json).unwrap();
    assert_eq!(action, back);
    assert!(action.should_save_app_state_on_action());
}

#[test]
fn remove_worktree_round_trip() {
    let action = WorkspaceAction::RemoveWorktree {
        worktree_path: std::path::PathBuf::from("/tmp/feature-a"),
        force: false,
    };
    let json = serde_json::to_string(&action).unwrap();
    let back: WorkspaceAction = serde_json::from_str(&json).unwrap();
    assert_eq!(action, back);
}

#[test]
fn open_and_prune_round_trip() {
    let open = WorkspaceAction::OpenWorktreeInTab {
        worktree_path: std::path::PathBuf::from("/tmp/feature-a"),
    };
    let prune = WorkspaceAction::PruneWorktree {
        worktree_path: std::path::PathBuf::from("/tmp/gone"),
    };
    let _ = serde_json::to_string(&open).unwrap();
    let _ = serde_json::to_string(&prune).unwrap();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p app workspace::action_tests`
Expected: compile error — variants not defined.

- [ ] **Step 3: Add the variants**

In `app/src/workspace/action.rs`, add to `WorkspaceAction` enum (preserve existing `#[derive(...)]`, `#[serde(...)]`, and any `#[non_exhaustive]` patterns):

```rust
NewWorktreeFromBranch {
    branch: BranchTarget,
    open_in: WorktreeOpenTarget,
},
OpenWorktreeInTab {
    worktree_path: std::path::PathBuf,
},
RemoveWorktree {
    worktree_path: std::path::PathBuf,
    force: bool,
},
PruneWorktree {
    worktree_path: std::path::PathBuf,
},
```

In the same file, define support types:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BranchTarget {
    Existing(String),
    CreateFromHead(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WorktreeOpenTarget {
    NewTab,
}
```

In `should_save_app_state_on_action` (or equivalent match arm), return `true` for all four new variants.

- [ ] **Step 4: Create the handler module skeleton**

Create `app/src/workspace/actions/worktree_manager/mod.rs`:

```rust
//! Worktree Manager action handlers.
//!
//! Each handler is a no-op when `FeatureFlag::WorktreeManager` is off
//! (PRODUCT.md 1), so replay during session restore is well-defined
//! even after the flag is disabled.

pub mod new_worktree;
pub mod open_worktree;
pub mod remove_worktree;
pub mod pickers;
```

Register the module from wherever other handler modules are listed (search: `rg -n "pub mod" app/src/workspace/actions.rs`).

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test -p app workspace::action_tests::new_worktree_round_trip workspace::action_tests::remove_worktree_round_trip workspace::action_tests::open_and_prune_round_trip`
Expected: 3 passes.

- [ ] **Step 6: Commit**

```bash
git add app/src/workspace/action.rs app/src/workspace/action_tests.rs app/src/workspace/actions/worktree_manager/
git commit -S -m "feat(action): add Worktree Manager action variants"
git log -1 --show-signature | head -2
```

---

### Task 12: Branch + worktree picker model

**Files:**
- Create: `app/src/workspace/actions/worktree_manager/pickers.rs`

- [ ] **Step 1: Write the failing test**

Create `app/src/workspace/actions/worktree_manager/pickers.rs`:

```rust
//! In-memory picker rows for the branch and worktree sub-pickers.
//!
//! These are pure functions over data fetched from the worktree util;
//! the UI binding lives in palette.rs.

use crate::util::worktree::WorktreeInfo;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRow {
    pub name: String,
    pub remote_only: bool,
    pub in_use_at: Option<PathBuf>,  // PRODUCT.md 5 "in use" tag
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRow {
    pub path: PathBuf,
    pub label: String,           // "main" or slug
    pub branch_or_sha: String,
    pub status: Vec<&'static str>, // ["locked","prunable","detached","bare"]
}

/// Build branch rows from `(branch, is_local)` pairs and the current
/// worktree list. Local branches sorted first (PRODUCT.md 5).
pub fn build_branch_rows(
    branches: &[(String, bool)],
    worktrees: &[WorktreeInfo],
) -> Vec<BranchRow> {
    let mut rows: Vec<BranchRow> = branches
        .iter()
        .map(|(name, is_local)| {
            let in_use_at = worktrees
                .iter()
                .find(|w| w.branch.as_deref() == Some(name.as_str()))
                .map(|w| w.path.clone());
            BranchRow {
                name: name.clone(),
                remote_only: !is_local,
                in_use_at,
            }
        })
        .collect();
    rows.sort_by_key(|r| (r.remote_only, r.name.clone()));
    rows
}

/// PRODUCT.md 11: main first, then by path. Status tags derived from flags.
pub fn build_worktree_rows(worktrees: &[WorktreeInfo]) -> Vec<WorktreeRow> {
    let mut rows: Vec<WorktreeRow> = worktrees.iter().map(|w| {
        let mut status = Vec::new();
        if w.is_locked { status.push("locked"); }
        if w.is_prunable { status.push("prunable"); }
        if w.is_bare { status.push("bare"); }
        if w.branch.is_none() && !w.is_bare { status.push("detached"); }
        let label = if w.is_main {
            "main".to_string()
        } else {
            w.path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
        };
        WorktreeRow {
            path: w.path.clone(),
            label,
            branch_or_sha: w.branch.clone().unwrap_or_else(|| w.head.clone()),
            status,
        }
    }).collect();
    rows.sort_by_key(|r| (!worktrees.iter().any(|w| w.path == r.path && w.is_main), r.path.clone()));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::worktree::WorktreeInfo;

    fn wi(path: &str, branch: Option<&str>, main: bool) -> WorktreeInfo {
        WorktreeInfo {
            path: PathBuf::from(path),
            branch: branch.map(str::to_string),
            head: "abcdef0".into(),
            is_main: main,
            is_locked: false,
            is_prunable: false,
            is_bare: false,
        }
    }

    #[test]
    fn branch_rows_mark_in_use() {
        let branches = vec![("main".into(), true), ("feature/a".into(), true)];
        let wts = vec![
            wi("/repo", Some("main"), true),
            wi("/repo/.castcodes/worktrees/feature-a", Some("feature/a"), false),
        ];
        let rows = build_branch_rows(&branches, &wts);
        assert_eq!(rows[0].name, "feature/a");
        assert!(rows[0].in_use_at.is_some());
        assert!(rows.iter().find(|r| r.name == "main").unwrap().in_use_at.is_some());
    }

    #[test]
    fn branch_rows_remote_after_local() {
        let branches = vec![
            ("origin/feature/c".into(), false),
            ("feature/b".into(), true),
        ];
        let rows = build_branch_rows(&branches, &[]);
        assert_eq!(rows[0].name, "feature/b");
        assert_eq!(rows[1].name, "origin/feature/c");
    }

    #[test]
    fn worktree_rows_main_first() {
        let wts = vec![
            wi("/repo/.castcodes/worktrees/feature-a", Some("feature/a"), false),
            wi("/repo", Some("main"), true),
        ];
        let rows = build_worktree_rows(&wts);
        assert_eq!(rows[0].label, "main");
        assert_eq!(rows[1].label, "feature-a");
    }

    #[test]
    fn worktree_rows_status_tags() {
        let mut w = wi("/repo/.castcodes/worktrees/locked", Some("locked"), false);
        w.is_locked = true;
        let rows = build_worktree_rows(&[w]);
        assert!(rows[0].status.contains(&"locked"));
    }

    #[test]
    fn worktree_rows_detached_tag() {
        let w = wi("/repo/.castcodes/worktrees/detached", None, false);
        let rows = build_worktree_rows(&[w]);
        assert!(rows[0].status.contains(&"detached"));
    }
}
```

- [ ] **Step 2: Run tests to verify pass**

Run: `cargo test -p app workspace::actions::worktree_manager::pickers::tests`
Expected: 5 passes.

- [ ] **Step 3: Commit**

```bash
git add app/src/workspace/actions/worktree_manager/pickers.rs
git commit -S -m "feat(worktree): branch + worktree picker row builders"
git log -1 --show-signature | head -2
```

---

### Task 13: `NewWorktreeFromBranch` handler

**Files:**
- Create: `app/src/workspace/actions/worktree_manager/new_worktree.rs`

- [ ] **Step 1: Read a sibling handler for the dispatch pattern**

Run: `rg -n "WorkspaceAction::RenamePane" app/src/workspace/`
Expected: hit on the handler call site; read 40 lines around it to learn how a typed handler is registered and receives `&mut AppContext`.

- [ ] **Step 2: Write the failing test**

Create `app/src/workspace/actions/worktree_manager/new_worktree.rs`:

```rust
//! Handler for `WorkspaceAction::NewWorktreeFromBranch`.
//!
//! Resolves the active pane's repo, creates the worktree on disk,
//! then dispatches a tab-spawn with the new CWD (PRODUCT.md 7).
//! Errors surface as toasts via the existing toast infra (PRODUCT.md 8, 10).

use crate::settings::WorktreeManagerSettings;
use crate::util::worktree::{add_worktree, default_staging_dir, slugify_branch, unique_path};
use crate::workspace::action::{BranchTarget, WorktreeOpenTarget};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct NewWorktreeRequest {
    pub repo_root: PathBuf,
    pub branch: BranchTarget,
    pub open_in: WorktreeOpenTarget,
    pub staging_override: Option<String>,
}

pub struct NewWorktreeOutcome {
    pub created_at: PathBuf,
    pub branch_displayed: String,
}

/// Pure-logic core, testable without a real `AppContext`.
pub async fn new_worktree_core(req: NewWorktreeRequest) -> Result<NewWorktreeOutcome> {
    let (branch_name, create) = match &req.branch {
        BranchTarget::Existing(b) => (b.clone(), false),
        BranchTarget::CreateFromHead(b) => (b.clone(), true),
    };
    let slug = slugify_branch(&branch_name);
    let base = default_staging_dir(&req.repo_root, &slug, req.staging_override.as_deref());
    let target = unique_path(&base);
    add_worktree(&req.repo_root, &target, &branch_name, create)
        .await
        .with_context(|| format!("git worktree add {}", target.display()))?;
    Ok(NewWorktreeOutcome { created_at: target, branch_displayed: branch_name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo(repo: &Path) {
        let run = |args: &[&str]| {
            Command::new("git").args(args).current_dir(repo).status().unwrap();
        };
        run(&["init", "--initial-branch=main", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(repo.join("a"), "a").unwrap();
        run(&["add", "a"]);
        run(&["commit", "-q", "-m", "init"]);
        run(&["branch", "feature/a"]);
    }

    #[cfg(feature = "local_fs")]
    #[tokio::test]
    async fn core_creates_at_default_path() {
        let td = TempDir::new().unwrap();
        init_repo(td.path());
        let out = new_worktree_core(NewWorktreeRequest {
            repo_root: td.path().to_path_buf(),
            branch: BranchTarget::Existing("feature/a".into()),
            open_in: WorktreeOpenTarget::NewTab,
            staging_override: None,
        }).await.unwrap();
        assert_eq!(out.created_at, td.path().join(".castcodes/worktrees/feature-a"));
        assert_eq!(out.branch_displayed, "feature/a");
    }

    #[cfg(feature = "local_fs")]
    #[tokio::test]
    async fn core_suffixes_on_collision() {
        let td = TempDir::new().unwrap();
        init_repo(td.path());
        let first = new_worktree_core(NewWorktreeRequest {
            repo_root: td.path().to_path_buf(),
            branch: BranchTarget::Existing("feature/a".into()),
            open_in: WorktreeOpenTarget::NewTab,
            staging_override: None,
        }).await.unwrap();
        // Recreate the same directory after `git worktree remove` would normally clean it.
        // Simulate by leaving it in place and trying a *new branch* with the same slug.
        std::fs::create_dir_all(td.path().join(".castcodes/worktrees/feature-a-2")).ok();
        Command::new("git").args(["branch", "feature/a2"]).current_dir(td.path()).status().unwrap();
        let second = new_worktree_core(NewWorktreeRequest {
            repo_root: td.path().to_path_buf(),
            branch: BranchTarget::Existing("feature/a2".into()),
            open_in: WorktreeOpenTarget::NewTab,
            // override produces same slug to force collision
            staging_override: Some("<repo-root>/.castcodes/worktrees/feature-a".into()),
        }).await.unwrap();
        assert_ne!(first.created_at, second.created_at);
    }
}
```

- [ ] **Step 3: Run tests to verify fail then pass**

Run: `cargo test -p app --features local_fs workspace::actions::worktree_manager::new_worktree::tests`
Expected: compile error first (no `new_worktree_core` yet) → after Step 2's file is saved, both tests pass.

- [ ] **Step 4: Wire the handler to the dispatcher**

In the typed-action dispatcher (the same place `RenamePane` and friends are routed — found in Step 1), add an arm for `WorkspaceAction::NewWorktreeFromBranch`:

```rust
WorkspaceAction::NewWorktreeFromBranch { branch, open_in } => {
    if !warp_features::FeatureFlag::WorktreeManager.is_enabled() {
        return; // PRODUCT.md 1
    }
    let Some(repo_root) = crate::util::git::detect_repo_root_for_active_pane(ctx) else {
        crate::ui::toast::error(ctx, "No git repository in this pane");
        return;
    };
    let staging_override = crate::settings::WorktreeManagerSettings::as_ref(ctx)
        .staging_directory.clone();
    let req = crate::workspace::actions::worktree_manager::new_worktree::NewWorktreeRequest {
        repo_root: repo_root.clone(),
        branch: branch.clone(),
        open_in: *open_in,
        staging_override,
    };
    let task_ctx = ctx.weak();
    ctx.spawn(async move {
        crate::ui::toast::pending(&task_ctx, "Creating worktree…");
        match crate::workspace::actions::worktree_manager::new_worktree::new_worktree_core(req).await {
            Ok(out) => {
                crate::ui::toast::success(
                    &task_ctx,
                    format!("Worktree {} ready", out.created_at.display()),
                );
                crate::pane_group::spawn_tab_at(&task_ctx, &out.created_at);
            }
            Err(e) => crate::ui::toast::error(&task_ctx, format!("Worktree failed: {e:#}")),
        }
    });
}
```

(Implementer: the exact APIs for `toast`, `ctx.spawn`, `ctx.weak`, and `spawn_tab_at` need to match what's already in the repo. Use the patterns from a sibling handler that creates a toast and spawns a tab — the "+" button handler is the closest match. Search: `rg -n "spawn_tab\|new_tab\|spawn_terminal" app/src/pane_group/mod.rs`.)

- [ ] **Step 5: Build check**

Run: `cargo check -p app --features local_fs`
Expected: success.

- [ ] **Step 6: Commit**

```bash
git add app/src/workspace/actions/worktree_manager/new_worktree.rs app/src/workspace/action.rs
git commit -S -m "feat(action): NewWorktreeFromBranch handler"
git log -1 --show-signature | head -2
```

---

### Task 14: `OpenWorktreeInTab` + `PruneWorktree` handlers

**Files:**
- Create: `app/src/workspace/actions/worktree_manager/open_worktree.rs`

- [ ] **Step 1: Write the file**

Create `app/src/workspace/actions/worktree_manager/open_worktree.rs`:

```rust
//! Handlers for `OpenWorktreeInTab` and `PruneWorktree`.

use crate::util::worktree::prune_worktrees;
use anyhow::Result;
use std::path::Path;

/// Pure logic: validate the path exists and is a worktree dir, then return Ok.
/// Tab spawning is done by the dispatch wrapper, not the core, so this stays
/// trivially testable.
pub fn validate_open_path(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Worktree path no longer exists: {}", path.display());
    }
    Ok(())
}

#[cfg(feature = "local_fs")]
pub async fn prune_core(repo_root: &Path) -> Result<()> {
    prune_worktrees(repo_root).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_open_path_missing() {
        let res = validate_open_path(Path::new("/definitely/not/there/xyz"));
        assert!(res.is_err());
    }

    #[test]
    fn validate_open_path_existing() {
        let td = TempDir::new().unwrap();
        validate_open_path(td.path()).unwrap();
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p app workspace::actions::worktree_manager::open_worktree::tests`
Expected: 2 passes.

- [ ] **Step 3: Wire dispatcher arms**

In the same dispatcher file edited in Task 13, add:

```rust
WorkspaceAction::OpenWorktreeInTab { worktree_path } => {
    if !warp_features::FeatureFlag::WorktreeManager.is_enabled() { return; }
    match crate::workspace::actions::worktree_manager::open_worktree::validate_open_path(worktree_path) {
        Ok(()) => crate::pane_group::spawn_tab_at(ctx, worktree_path),
        Err(e) => crate::ui::toast::error(ctx, format!("{e}")),
    }
}
WorkspaceAction::PruneWorktree { worktree_path: _ } => {
    if !warp_features::FeatureFlag::WorktreeManager.is_enabled() { return; }
    let Some(repo_root) = crate::util::git::detect_repo_root_for_active_pane(ctx) else { return; };
    let task_ctx = ctx.weak();
    ctx.spawn(async move {
        match crate::workspace::actions::worktree_manager::open_worktree::prune_core(&repo_root).await {
            Ok(()) => crate::ui::toast::success(&task_ctx, "Pruned stale worktrees"),
            Err(e) => crate::ui::toast::error(&task_ctx, format!("Prune failed: {e:#}")),
        }
    });
}
```

(Note: `PruneWorktree` here prunes all stale entries for the active pane's repo, not a specific path. PRODUCT.md 13 says "removes the stale administrative entry … equivalent to `git worktree prune` filtered to that path" — `git worktree prune` doesn't take a path filter, so we prune the whole repo's stale entries; the user-visible effect is identical for the specific entry the user just selected.)

- [ ] **Step 4: Build check**

Run: `cargo check -p app --features local_fs`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/actions/worktree_manager/open_worktree.rs app/src/workspace/action.rs
git commit -S -m "feat(action): OpenWorktreeInTab + PruneWorktree handlers"
git log -1 --show-signature | head -2
```

---

### Task 15: `RemoveWorktree` handler

**Files:**
- Create: `app/src/workspace/actions/worktree_manager/remove_worktree.rs`

- [ ] **Step 1: Write file with test**

Create `app/src/workspace/actions/worktree_manager/remove_worktree.rs`:

```rust
//! `RemoveWorktree` handler.
//!
//! PRODUCT.md 15: confirmation flow with a Force escalation on dirty.

use crate::util::worktree::remove_worktree;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct RemoveRequest {
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub force: bool,
}

#[cfg(feature = "local_fs")]
pub async fn remove_core(req: RemoveRequest) -> Result<()> {
    remove_worktree(&req.repo_root, &req.worktree_path, req.force).await
}

/// PRODUCT.md 15 forbids removing the main worktree via this UI.
pub fn is_main_worktree(path: &Path, repo_root: &Path) -> bool {
    path == repo_root
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    #[cfg(feature = "local_fs")]
    #[tokio::test]
    async fn remove_dirty_then_force() {
        let td = TempDir::new().unwrap();
        let repo = td.path();
        let run = |args: &[&str]| {
            Command::new("git").args(args).current_dir(repo).status().unwrap();
        };
        run(&["init", "--initial-branch=main", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(repo.join("a"), "a").unwrap();
        run(&["add", "a"]);
        run(&["commit", "-q", "-m", "init"]);
        run(&["branch", "feature/a"]);
        run(&["worktree", "add", "-q", ".castcodes/worktrees/feature-a", "feature/a"]);
        let target = repo.join(".castcodes/worktrees/feature-a");
        std::fs::write(target.join("dirty.txt"), "dirty").unwrap();

        let res = remove_core(RemoveRequest {
            repo_root: repo.to_path_buf(),
            worktree_path: target.clone(),
            force: false,
        }).await;
        assert!(res.is_err());
        remove_core(RemoveRequest {
            repo_root: repo.to_path_buf(),
            worktree_path: target.clone(),
            force: true,
        }).await.unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn main_worktree_detection() {
        assert!(is_main_worktree(Path::new("/r"), Path::new("/r")));
        assert!(!is_main_worktree(Path::new("/r/.castcodes/worktrees/a"), Path::new("/r")));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p app --features local_fs workspace::actions::worktree_manager::remove_worktree::tests`
Expected: 2 passes.

- [ ] **Step 3: Wire dispatcher arm with confirmation modal**

In the dispatcher:

```rust
WorkspaceAction::RemoveWorktree { worktree_path, force } => {
    if !warp_features::FeatureFlag::WorktreeManager.is_enabled() { return; }
    let Some(repo_root) = crate::util::git::detect_repo_root_for_active_pane(ctx) else { return; };
    if crate::workspace::actions::worktree_manager::remove_worktree::is_main_worktree(worktree_path, &repo_root) {
        crate::ui::toast::error(ctx, "Cannot remove the main worktree");
        return;
    }
    // PRODUCT.md 15: confirmation modal. Use the existing destructive-confirm modal.
    let path = worktree_path.clone();
    let repo = repo_root.clone();
    let forced = *force;
    let task_ctx = ctx.weak();
    crate::ui::modal::confirm_destructive(
        ctx,
        format!("Remove worktree {}?", path.display()),
        move |confirmed| if confirmed {
            let task_ctx = task_ctx.clone();
            let req = crate::workspace::actions::worktree_manager::remove_worktree::RemoveRequest {
                repo_root: repo.clone(),
                worktree_path: path.clone(),
                force: forced,
            };
            tokio::spawn(async move {
                match crate::workspace::actions::worktree_manager::remove_worktree::remove_core(req).await {
                    Ok(()) => crate::ui::toast::success(&task_ctx, "Worktree removed"),
                    Err(e) => {
                        // PRODUCT.md 15 bullet 1: re-show with Force button on dirty.
                        crate::ui::toast::error(&task_ctx, format!("Remove failed: {e:#}"));
                    }
                }
            });
        },
    );
}
```

(Implementer: `confirm_destructive` API name may differ — read the rename-tab or delete-pane flows for the actual modal API and adapt. The two-step "non-force fails → re-confirm with Force" loop is implemented by dispatching a second `RemoveWorktree { force: true }` action from the error-toast's action button if the error string contains "use --force" or similar; if the toast infra doesn't support action buttons easily, surface the error and let the user re-trigger from the palette.)

- [ ] **Step 4: Build check**

Run: `cargo check -p app --features local_fs`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/actions/worktree_manager/remove_worktree.rs app/src/workspace/action.rs
git commit -S -m "feat(action): RemoveWorktree handler with confirm flow"
git log -1 --show-signature | head -2
```

---

### Task 16: Register palette entries

**Files:**
- Modify: `app/src/command_palette.rs`

- [ ] **Step 1: Read existing palette entry registration**

Run: `rg -n "PaletteEntry\|register_palette_entry\|PRIORITIZED_KEYBINDINGS" app/src/command_palette.rs`
Expected: locate the convention. Read 100 lines around the first match.

- [ ] **Step 2: Add three entries**

In `app/src/command_palette.rs`, add three new palette entries near the other "feature flag gated" entries. The exact API is whatever the file uses; the entries are:

```rust
// PRODUCT.md 2, 3: three Worktree Manager entries, visible only when the
// WorktreeManager flag is on AND the active pane has a CWD; disabled (with
// subtitle) when CWD is not in a git repository.
{
    title: "New worktree from branch…",
    subtitle: "Open a fresh worktree of a chosen branch in a new tab",
    keywords: &["worktree", "branch", "git"],
    visible_when: |ctx| warp_features::FeatureFlag::WorktreeManager.is_enabled()
        && crate::util::git::active_pane_has_cwd(ctx),
    enabled_when: |ctx| crate::util::git::detect_repo_root_for_active_pane(ctx).is_some(),
    disabled_subtitle: Some("No git repository in this pane"),
    on_select: |ctx| crate::workspace::actions::worktree_manager::pickers::open_branch_picker(ctx),
},
{
    title: "Open worktree in new tab…",
    subtitle: "Choose an existing worktree of this repository",
    keywords: &["worktree", "open", "git"],
    visible_when: |ctx| warp_features::FeatureFlag::WorktreeManager.is_enabled()
        && crate::util::git::active_pane_has_cwd(ctx),
    enabled_when: |ctx| crate::util::git::detect_repo_root_for_active_pane(ctx).is_some(),
    disabled_subtitle: Some("No git repository in this pane"),
    on_select: |ctx| crate::workspace::actions::worktree_manager::pickers::open_worktree_picker(ctx, /*for_remove=*/ false),
},
{
    title: "Remove worktree…",
    subtitle: "Remove an existing non-main worktree of this repository",
    keywords: &["worktree", "remove", "delete", "git"],
    visible_when: |ctx| warp_features::FeatureFlag::WorktreeManager.is_enabled()
        && crate::util::git::active_pane_has_cwd(ctx),
    enabled_when: |ctx| crate::util::git::detect_repo_root_for_active_pane(ctx).is_some(),
    disabled_subtitle: Some("No git repository in this pane"),
    on_select: |ctx| crate::workspace::actions::worktree_manager::pickers::open_worktree_picker(ctx, /*for_remove=*/ true),
},
```

Adapt fields/closures to the actual `PaletteEntry` struct/function in use.

- [ ] **Step 3: Implement `open_branch_picker` / `open_worktree_picker` stubs**

In `app/src/workspace/actions/worktree_manager/pickers.rs`, append:

```rust
#[cfg(feature = "local_fs")]
pub fn open_branch_picker(ctx: &mut crate::AppContext) {
    let Some(repo) = crate::util::git::detect_repo_root_for_active_pane(ctx) else { return; };
    let task_ctx = ctx.weak();
    ctx.spawn(async move {
        let branches = crate::util::git::get_all_branches(&repo, 200, true).await.unwrap_or_default();
        let wts = crate::util::worktree::list_worktrees(&repo).await.unwrap_or_default();
        let rows = build_branch_rows(&branches, &wts);
        // Reuse the existing sub-picker infra; pattern matches the Files picker.
        crate::ui::picker::show(
            &task_ctx,
            "Pick a branch",
            rows.iter().map(|r| crate::ui::picker::Row {
                title: r.name.clone(),
                subtitle: r.in_use_at.as_ref()
                    .map(|p| format!("in use at {}", p.display()))
                    .unwrap_or_else(|| if r.remote_only { "remote".into() } else { "local".into() }),
                meta: r.name.clone(),
            }).collect(),
            move |ctx, selection| {
                let branch = match selection {
                    crate::ui::picker::Selection::Existing(meta) => crate::workspace::action::BranchTarget::Existing(meta),
                    crate::ui::picker::Selection::Typed(name) => crate::workspace::action::BranchTarget::CreateFromHead(name),
                };
                ctx.dispatch(crate::workspace::action::WorkspaceAction::NewWorktreeFromBranch {
                    branch,
                    open_in: crate::workspace::action::WorktreeOpenTarget::NewTab,
                });
            },
        );
    });
}

#[cfg(feature = "local_fs")]
pub fn open_worktree_picker(ctx: &mut crate::AppContext, for_remove: bool) {
    let Some(repo) = crate::util::git::detect_repo_root_for_active_pane(ctx) else { return; };
    let task_ctx = ctx.weak();
    ctx.spawn(async move {
        let wts = crate::util::worktree::list_worktrees(&repo).await.unwrap_or_default();
        let mut rows = build_worktree_rows(&wts);
        if for_remove {
            // PRODUCT.md 15: main worktree filtered out.
            rows.retain(|r| r.label != "main");
        }
        crate::ui::picker::show(
            &task_ctx,
            if for_remove { "Remove worktree" } else { "Open worktree" },
            rows.iter().map(|r| crate::ui::picker::Row {
                title: r.label.clone(),
                subtitle: format!("{} {}", r.branch_or_sha, r.status.join(" ")),
                meta: r.path.display().to_string(),
            }).collect(),
            move |ctx, selection| {
                let crate::ui::picker::Selection::Existing(meta) = selection else { return; };
                let path = std::path::PathBuf::from(meta);
                let action = if for_remove {
                    crate::workspace::action::WorkspaceAction::RemoveWorktree { worktree_path: path, force: false }
                } else {
                    // PRODUCT.md 13: prunable rows dispatch Prune, not Open.
                    // The row's `status` is lost here; if the picker UI returns
                    // additional flags, branch on them. Otherwise the open handler
                    // falls back to a graceful error when the path is missing.
                    crate::workspace::action::WorkspaceAction::OpenWorktreeInTab { worktree_path: path }
                };
                ctx.dispatch(action);
            },
        );
    });
}
```

(The `ui::picker` / `ui::toast` / `ui::modal` namespaces are placeholders for whatever the repo actually uses. Read `app/src/ui/` or grep `pub mod picker\|pub mod toast\|pub mod modal` and substitute.)

- [ ] **Step 4: Build check**

Run: `cargo check -p app --features local_fs`
Expected: any compile errors here come from mismatched UI API names; fix by adapting to the actual symbols.

- [ ] **Step 5: Commit**

```bash
git add app/src/command_palette.rs app/src/workspace/actions/worktree_manager/pickers.rs
git commit -S -m "feat(palette): register Worktree Manager entries"
git log -1 --show-signature | head -2
```

---

### Task 17: Phase 2 gate — full app build + clippy

- [ ] **Step 1: Full build**

Run: `cargo build -p app --features local_fs`
Expected: success.

- [ ] **Step 2: Clippy**

Run: `cargo clippy -p app --features local_fs --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: All unit tests**

Run: `cargo test -p app --features local_fs`
Expected: green.

- [ ] **Step 4: Fix anything that broke; commit fixes**

```bash
git add -A
git commit -S -m "chore(worktree): clippy/build fixups"
```
(Skip if nothing changed.)

---

## Phase 3 — integration tests (Agent E)

### Task 18: Integration test scaffolding

**Files:**
- Create: `crates/integration/src/test/worktree_manager.rs`
- Modify: `crates/integration/src/test/mod.rs`

- [ ] **Step 1: Read sibling integration test**

Run: `cat crates/integration/src/test/pane_restoration.rs`
Expected: understand `Builder`, `new_builder()`, `with_step`, and how the test framework asserts pane/tab state.

- [ ] **Step 2: Add module declaration**

In `crates/integration/src/test/mod.rs`, add:
```rust
pub mod worktree_manager;
```

- [ ] **Step 3: Create the test file skeleton**

Create `crates/integration/src/test/worktree_manager.rs`:

```rust
//! End-to-end tests for the Worktree Manager feature.
//!
//! Each test enables `FeatureFlag::WorktreeManager` at the top and uses
//! the `Builder` framework to drive palette + sub-picker + assertion
//! sequences. Mirrors `pane_restoration.rs`.

use crate::test_framework::{
    new_builder, new_step_with_default_assertions,
    wait_until_bootstrapped_single_pane_for_tab, Builder,
};
use warp_features::FeatureFlag;

fn with_flag() -> Builder {
    FeatureFlag::WorktreeManager.set_enabled(true);
    new_builder().with_step(wait_until_bootstrapped_single_pane_for_tab(0))
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/integration/src/test/worktree_manager.rs crates/integration/src/test/mod.rs
git commit -S -m "test(integration): scaffold worktree_manager suite"
git log -1 --show-signature | head -2
```

---

### Task 19: Test — flag-off hides palette entries (PRODUCT.md 1)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_flag_off_hides_entries() -> Builder {
    FeatureFlag::WorktreeManager.set_enabled(false);
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(new_step_with_default_assertions("open palette")
            .with_keystrokes(crate::keys::CMD_P))
        .with_step(new_step_with_default_assertions("assert no worktree entries")
            .assert(|ui| {
                let titles = ui.palette_visible_entry_titles();
                assert!(!titles.iter().any(|t| t.contains("worktree")),
                    "no worktree entries when flag off; got {titles:?}");
            }))
}
```

- [ ] **Step 2: Wire into runner**

If the runner uses a registry function (see `pane_restoration.rs` end), add to it:
```rust
runner.add(test_flag_off_hides_entries);
```

- [ ] **Step 3: Run**

Run: `cargo test -p integration --features local_fs worktree_manager::test_flag_off_hides_entries`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): flag-off hides palette entries"
git log -1 --show-signature | head -2
```

---

### Task 20: Test — palette disabled outside repo (PRODUCT.md 3)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_palette_disabled_outside_repo() -> Builder {
    with_flag()
        .with_step(new_step_with_default_assertions("cd /tmp")
            .with_keystrokes(crate::keys::type_text("cd /tmp\n")))
        .with_step(new_step_with_default_assertions("open palette")
            .with_keystrokes(crate::keys::CMD_P))
        .with_step(new_step_with_default_assertions("assert entries are disabled")
            .assert(|ui| {
                for title in ["New worktree from branch…", "Open worktree in new tab…", "Remove worktree…"] {
                    let entry = ui.palette_entry(title).expect("entry visible");
                    assert!(!entry.enabled, "{title} should be disabled outside a repo");
                    assert_eq!(entry.disabled_subtitle.as_deref(), Some("No git repository in this pane"));
                }
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_palette_disabled_outside_repo
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): palette disabled outside repo"
git log -1 --show-signature | head -2
```

---

### Task 21: Test — create worktree from existing branch (PRODUCT.md 5–8, 17)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_new_worktree_from_branch() -> Builder {
    with_flag()
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main", "feature/a"]))
        .with_step(new_step_with_default_assertions("open palette")
            .with_keystrokes(crate::keys::CMD_P))
        .with_step(new_step_with_default_assertions("select New worktree from branch")
            .with_keystrokes(crate::keys::type_text("new worktree from branch"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("pick feature/a")
            .with_keystrokes(crate::keys::type_text("feature/a"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("assert tab opened in new worktree")
            .assert(|ui| {
                let tabs = ui.tabs();
                assert_eq!(tabs.len(), 2, "should have opened a new tab");
                let new_tab = &tabs[1];
                assert!(new_tab.cwd.ends_with(".castcodes/worktrees/feature-a"),
                    "new tab CWD: {:?}", new_tab.cwd);
                assert!(new_tab.title.contains("feature-a · feature/a"),
                    "tab title: {}", new_tab.title);
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_new_worktree_from_branch
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): create worktree from existing branch"
git log -1 --show-signature | head -2
```

---

### Task 22: Test — collision suffixing (PRODUCT.md 6)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_new_worktree_slug_collision() -> Builder {
    with_flag()
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main", "feature/a"])
            .with_pre_created_path(".castcodes/worktrees/feature-a"))
        .with_step(new_step_with_default_assertions("open palette + select + pick branch")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("new worktree from branch"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("feature/a"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("assert -2 suffix")
            .assert(|ui| {
                let tabs = ui.tabs();
                let new_tab = tabs.last().unwrap();
                assert!(new_tab.cwd.ends_with(".castcodes/worktrees/feature-a-2"),
                    "expected -2 suffix, got {:?}", new_tab.cwd);
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_new_worktree_slug_collision
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): collision suffix on create"
git log -1 --show-signature | head -2
```

---

### Task 23: Test — open existing worktree (PRODUCT.md 11, 12)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_open_worktree_lists_existing() -> Builder {
    with_flag()
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main", "feature/a"])
            .with_pre_created_worktree("feature/a", ".castcodes/worktrees/feature-a"))
        .with_step(new_step_with_default_assertions("open palette + Open worktree")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("open worktree in new tab"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("assert picker lists main + feature-a")
            .assert(|ui| {
                let rows = ui.picker_visible_row_titles();
                assert!(rows.contains(&"main".to_string()));
                assert!(rows.contains(&"feature-a".to_string()));
            }))
        .with_step(new_step_with_default_assertions("pick feature-a")
            .with_keystrokes(crate::keys::type_text("feature-a"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("assert tab opened")
            .assert(|ui| {
                let last = ui.tabs().last().unwrap();
                assert!(last.cwd.ends_with(".castcodes/worktrees/feature-a"));
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_open_worktree_lists_existing
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): open existing worktree"
git log -1 --show-signature | head -2
```

---

### Task 24: Test — remove keeps pane, marks missing (PRODUCT.md 15, 19)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_remove_worktree_keeps_pane() -> Builder {
    with_flag()
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main", "feature/a"])
            .with_pre_created_worktree("feature/a", ".castcodes/worktrees/feature-a"))
        .with_step(new_step_with_default_assertions("open the worktree in a new tab")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("open worktree in new tab"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("feature-a"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("remove the worktree")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("remove worktree"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("feature-a"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::ENTER)) // confirm modal
        .with_step(new_step_with_default_assertions("tab persists, indicator missing")
            .assert(|ui| {
                let tabs = ui.tabs();
                assert_eq!(tabs.len(), 2, "tab should still exist");
                let last = tabs.last().unwrap();
                assert!(last.title.contains("(missing)") || last.title.contains("⌫"),
                    "indicator should show missing; got {}", last.title);
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_remove_worktree_keeps_pane
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): remove preserves pane, marks missing"
git log -1 --show-signature | head -2
```

---

### Task 25: Test — staging directory override (PRODUCT.md 22)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_setting_overrides_staging_dir() -> Builder {
    with_flag()
        .with_setting("worktree_manager.staging_directory", "<repo-root>/wts/<branch-slug>")
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main", "feature/a"]))
        .with_step(new_step_with_default_assertions("create worktree")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("new worktree from branch"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("feature/a"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("path uses override template")
            .assert(|ui| {
                let last = ui.tabs().last().unwrap();
                assert!(last.cwd.ends_with("wts/feature-a"),
                    "expected override path, got {:?}", last.cwd);
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_setting_overrides_staging_dir
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): staging dir override setting"
git log -1 --show-signature | head -2
```

---

### Task 26: Test — type a name to create new branch (PRODUCT.md 5 bullet 3, 6 bullet 3)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_create_new_branch_path() -> Builder {
    with_flag()
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main"]))
        .with_step(new_step_with_default_assertions("type new branch name in picker")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("new worktree from branch"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("brand-new"))
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("branch created from HEAD, tab opened")
            .assert(|ui| {
                let last = ui.tabs().last().unwrap();
                assert!(last.cwd.ends_with(".castcodes/worktrees/brand-new"));
                let branches = ui.git_branches_in_repo();
                assert!(branches.iter().any(|b| b == "brand-new"));
            }))
}
```

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_create_new_branch_path
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): create new branch from picker"
git log -1 --show-signature | head -2
```

---

### Task 27: Test — force-remove on dirty (PRODUCT.md 10, 15 bullet 1)

**Files:**
- Modify: `crates/integration/src/test/worktree_manager.rs`

- [ ] **Step 1: Append test**

```rust
pub fn test_remove_force_path() -> Builder {
    with_flag()
        .with_repo(crate::test_framework::FixtureRepo::with_branches(&["main", "feature/a"])
            .with_pre_created_worktree("feature/a", ".castcodes/worktrees/feature-a")
            .with_dirty_file_in_worktree(".castcodes/worktrees/feature-a", "dirty.txt", "dirty"))
        .with_step(new_step_with_default_assertions("attempt non-force remove")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("remove worktree"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("feature-a"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::ENTER))
        .with_step(new_step_with_default_assertions("error toast surfaced with git stderr")
            .assert(|ui| {
                let toasts = ui.recent_toasts();
                assert!(toasts.iter().any(|t| t.kind == "error" && t.body.contains("force")),
                    "expected toast hinting at --force; got {toasts:?}");
            }))
        .with_step(new_step_with_default_assertions("retry with Force")
            .with_keystrokes(crate::keys::CMD_P)
            .with_keystrokes(crate::keys::type_text("remove worktree"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::type_text("feature-a"))
            .with_keystrokes(crate::keys::ENTER)
            .with_keystrokes(crate::keys::FORCE_REMOVE_SHORTCUT))
        .with_step(new_step_with_default_assertions("worktree removed")
            .assert(|ui| {
                assert!(!ui.repo_path_exists(".castcodes/worktrees/feature-a"));
            }))
}
```

(The `FORCE_REMOVE_SHORTCUT` keystroke is whatever the destructive-confirm modal accepts as the "force" affirmative — likely Shift+Enter or a labeled button hit by typing its label and pressing Enter. Adapt to the actual modal API.)

- [ ] **Step 2: Wire + run + commit**

```bash
cargo test -p integration --features local_fs worktree_manager::test_remove_force_path
git add crates/integration/src/test/worktree_manager.rs
git commit -S -m "test(worktree): force-remove path on dirty worktree"
git log -1 --show-signature | head -2
```

---

### Task 28: Full integration suite + presubmit

- [ ] **Step 1: Run the full integration suite**

Run: `cargo test -p integration --features local_fs worktree_manager`
Expected: all 9 tests pass.

- [ ] **Step 2: Run repo presubmit**

Run: the repo's `presubmit` alias (per the `castcodes-dev-loop` skill — `cargo run --bin presubmit` or `./scripts/presubmit.sh`; pick whichever the skill specifies).
Expected: green.

- [ ] **Step 3: Manual verification matrix (TECH.md §Testing)**

Walk the four manual checks from TECH.md §Testing → Manual verification:
- [ ] Indicator live-update on `cd` between worktrees
- [ ] Two panes, same worktree, both indicators update on remove
- [ ] Two panes, two different worktrees of same repo, focus switch does not change CWDs
- [ ] WASM build: palette entries absent, branch-only indicator preserved

Record the outcome of each in the PR description.

- [ ] **Step 4: Final commit if anything changed**

```bash
git add -A
git commit -S -m "chore(worktree): final cleanups from manual verification" || true
```

---

## Self-review checklist (run before opening PR)

- [ ] **Spec coverage:** every numbered invariant in `PRODUCT.md` maps to either a unit test (Tasks 3–9, 11–15) or an integration test (Tasks 19–27) or an explicit manual check (Task 28).
- [ ] **Placeholder scan:** no `TODO`, `TBD`, `unimplemented!()`, or "Add appropriate error handling" left in committed code.
- [ ] **Signing:** `git log origin/main..HEAD --pretty='%H %G?' | awk '$2 != "G" {print "UNSIGNED:", $0}'` prints nothing.
- [ ] **Flag default:** `FeatureFlag::WorktreeManager` defaults off in production (Dogfood). Verify with the rollout enum in `crates/warp_features/src/lib.rs`.
- [ ] **`.gitignore`:** the feature does NOT auto-edit `.gitignore` (PRODUCT.md 28). Note in PR description that users should add `.castcodes/worktrees/` themselves.

---

## Execution Handoff

Plan complete and saved to `specs/castcodes-worktree-manager/PLAN.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per phase, review between phases, fast iteration. With the 5-agent parallelization plan in TECH.md, Phase 1 (Tasks 1–10) fans out across three sub-agent worktrees and converges before Phase 2 (Tasks 11–16, sequential, single agent), then Phase 3 (Tasks 18–28, sequential, single agent).

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
