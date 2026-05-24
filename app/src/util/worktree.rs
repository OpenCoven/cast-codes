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

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;
