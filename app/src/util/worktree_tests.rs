use super::{default_staging_dir, parse_worktree_list_porcelain, slugify_branch, unique_path};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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
