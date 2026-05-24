//! Wraps `git worktree` and related path helpers.
//!
//! Mirrors `app/src/util/git.rs`: fns that shell out will go behind
//! `#[cfg(feature = "local_fs")]` with a WASM stub. This file currently
//! only contains pure-function helpers; async git wrappers land in
//! subsequent tasks.

// Path and PathBuf are unused now but will be consumed by subsequent tasks
// that add async git-worktree wrappers.
#[allow(unused_imports)]
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

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;
