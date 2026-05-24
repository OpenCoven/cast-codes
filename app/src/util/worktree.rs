//! Wraps `git worktree` and related path helpers.
//!
//! Mirrors `app/src/util/git.rs`: fns that shell out will go behind
//! `#[cfg(feature = "local_fs")]` with a WASM stub. This file currently
//! only contains pure-function helpers; async git wrappers land in
//! subsequent tasks.

use std::path::{Path, PathBuf};

/// Convert a branch name to a filesystem-safe slug.
///
/// Rules: lowercase, replace any `[^a-z0-9._-]` with `-`, collapse runs
/// of `-`, trim leading/trailing `-`. Empty result falls back to
/// `"worktree"`.
pub fn slugify_branch(branch: &str) -> String {
    let mut out = String::with_capacity(branch.len());
    let mut prev_dash = true; // start "in dash run" so leading dashes are trimmed
    for ch in branch.chars() {
        let mapped = match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '.' | '-' => ch,
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

/// Resolve the on-disk path a new worktree will be created at.
///
/// `override_tmpl` supports `<repo-root>` and `<branch-slug>` placeholders,
/// absolute or relative. `None` / empty falls back to the default
/// `<repo>/.castcodes/worktrees/<slug>`.
pub fn default_staging_dir(repo_root: &Path, slug: &str, override_tmpl: Option<&str>) -> PathBuf {
    let tmpl = override_tmpl.filter(|s| !s.is_empty());
    let Some(t) = tmpl else {
        return repo_root.join(".castcodes/worktrees").join(slug);
    };
    let resolved = t
        .replace("<repo-root>", &repo_root.display().to_string())
        .replace("<branch-slug>", slug);
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
    let mut i: usize = 2;
    loop {
        let candidate = parent.join(format!("{stem}-{i}"));
        if !candidate.exists() {
            return candidate;
        }
        i += 1;
    }
}

/// Parsed entry from `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>, // refs/heads/X → X; None when detached or bare
    pub head: String,           // short SHA (first 7 chars)
    pub is_main: bool,          // true for the first entry
    pub is_locked: bool,
    pub is_prunable: bool,
    pub is_bare: bool,
}

/// Parse `git worktree list --porcelain` output.
///
/// Each entry starts with `worktree <path>` and ends at a blank line. Unknown
/// lines are ignored defensively so a future git version cannot break listing.
pub fn parse_worktree_list_porcelain(input: &str) -> Vec<WorktreeInfo> {
    let mut out = Vec::new();
    let mut first = true;
    let mut current: Option<WorktreeInfo> = None;

    fn flush(cur: &mut Option<WorktreeInfo>, out: &mut Vec<WorktreeInfo>) {
        if let Some(w) = cur.take() {
            out.push(w);
        }
    }

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
            w.branch = refname
                .strip_prefix("refs/heads/")
                .map(str::to_string)
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

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;
