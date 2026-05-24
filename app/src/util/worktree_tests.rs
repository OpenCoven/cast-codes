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
    assert_eq!(
        dir,
        PathBuf::from("/work/myrepo/.castcodes/worktrees/feature-a")
    );
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
    assert_eq!(
        dir,
        PathBuf::from("/work/myrepo/.castcodes/worktrees/feature-a")
    );
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
    assert!(r
        .iter()
        .any(|w| w.is_locked && w.branch.as_deref() == Some("locked-branch")));
}

#[test]
fn parse_prunable_flag() {
    let r = parse_worktree_list_porcelain(F_PRUNABLE);
    assert!(r
        .iter()
        .any(|w| w.is_prunable && w.branch.as_deref() == Some("gone-branch")));
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
    assert!(
        r[0].is_main,
        "first entry of `git worktree list` is always main"
    );
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn list_worktrees_round_trip_on_temp_repo() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        let s = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            s.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&s.stderr)
        );
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);
    run(&[
        "worktree",
        "add",
        "-q",
        ".castcodes/worktrees/feature-a",
        "feature/a",
    ]);

    let list = super::list_worktrees(repo).await.unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0].is_main);
    assert!(list
        .iter()
        .any(|w| w.branch.as_deref() == Some("feature/a")));
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn add_worktree_existing_branch() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);

    let target = repo.join(".castcodes/worktrees/feature-a");
    super::add_worktree(repo, &target, "feature/a", false)
        .await
        .unwrap();
    assert!(target.join(".git").exists() || target.join(".git").is_file());
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn add_worktree_creates_new_branch() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);

    let target = repo.join(".castcodes/worktrees/brand-new");
    super::add_worktree(repo, &target, "brand-new", true)
        .await
        .unwrap();
    let branches = Command::new("git")
        .args(["branch", "--list", "brand-new"])
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&branches.stdout).contains("brand-new"));
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn remove_worktree_clean() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);
    run(&[
        "worktree",
        "add",
        "-q",
        ".castcodes/worktrees/feature-a",
        "feature/a",
    ]);
    let target = repo.join(".castcodes/worktrees/feature-a");

    super::remove_worktree(repo, &target, false).await.unwrap();
    assert!(!target.exists());
}

#[cfg(feature = "local_fs")]
#[tokio::test]
async fn remove_worktree_dirty_requires_force() {
    use std::process::Command;
    let td = TempDir::new().unwrap();
    let repo = td.path();
    let run = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .unwrap();
    };
    run(&["init", "--initial-branch=main", "-q"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("a"), "a").unwrap();
    run(&["add", "a"]);
    run(&["commit", "-q", "-m", "init"]);
    run(&["branch", "feature/a"]);
    run(&[
        "worktree",
        "add",
        "-q",
        ".castcodes/worktrees/feature-a",
        "feature/a",
    ]);
    let target = repo.join(".castcodes/worktrees/feature-a");
    std::fs::write(target.join("dirty.txt"), "dirty").unwrap();

    let res = super::remove_worktree(repo, &target, false).await;
    assert!(
        res.is_err(),
        "non-force remove of dirty worktree should fail"
    );
    super::remove_worktree(repo, &target, true).await.unwrap();
    assert!(!target.exists());
}
