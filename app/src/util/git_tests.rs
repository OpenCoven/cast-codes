use std::path::Path;

use command::r#async::Command;
use command::Stdio;
use tempfile::TempDir;

use super::{detect_current_branch, detect_current_branch_display, detect_repo_root_sync};

/// Helper: run a git command inside the given repo directory.
async fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("failed to run git");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Creates a temp git repo with one commit and returns `(dir_handle, repo_path)`.
async fn init_repo() -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = dir.path().to_path_buf();

    git(&path, &["init", "-b", "main"]).await;
    git(&path, &["config", "user.email", "test@test.com"]).await;
    git(&path, &["config", "user.name", "Test"]).await;
    git(&path, &["commit", "--allow-empty", "-m", "initial"]).await;

    (dir, path)
}

#[tokio::test]
async fn on_normal_branch_returns_branch_name() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["checkout", "-b", "feature-xyz"]).await;

    assert_eq!(detect_current_branch(&repo).await.unwrap(), "feature-xyz");
    assert_eq!(
        detect_current_branch_display(&repo).await.unwrap(),
        "feature-xyz"
    );
}

#[tokio::test]
async fn detached_head_raw_returns_head() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["checkout", "--detach", "HEAD"]).await;

    assert_eq!(detect_current_branch(&repo).await.unwrap(), "HEAD");
}

#[tokio::test]
async fn detached_head_display_returns_short_sha() {
    let (_dir, repo) = init_repo().await;
    let full_sha = git(&repo, &["rev-parse", "HEAD"]).await;
    git(&repo, &["checkout", "--detach", "HEAD"]).await;

    let result = detect_current_branch_display(&repo).await.unwrap();

    assert_ne!(
        result, "HEAD",
        "display variant should not return literal HEAD"
    );
    assert!(
        full_sha.starts_with(&result),
        "expected {full_sha} to start with {result}"
    );
}

#[tokio::test]
async fn detached_tag_display_returns_short_sha() {
    let (_dir, repo) = init_repo().await;
    git(&repo, &["tag", "v1.0"]).await;
    git(&repo, &["checkout", "v1.0"]).await;

    let full_sha = git(&repo, &["rev-parse", "HEAD"]).await;
    let result = detect_current_branch_display(&repo).await.unwrap();

    assert_ne!(result, "HEAD");
    assert!(
        full_sha.starts_with(&result),
        "expected {full_sha} to start with {result}"
    );
}

// --- detect_repo_root_sync tests ---

#[cfg(feature = "local_fs")]
#[test]
fn detect_repo_root_sync_returns_repo_root() {
    use std::process::Command as StdCommand;

    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let repo = dir.path();

    let run = |args: &[&str]| {
        StdCommand::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git command failed");
    };
    run(&["init", "-b", "main", "-q"]);
    run(&["config", "user.email", "test@test.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["commit", "--allow-empty", "-m", "initial"]);

    let result = detect_repo_root_sync(repo);
    let expected = repo.canonicalize().expect("canonicalize repo path");
    // git --show-toplevel also returns a canonical (real) path.
    let result_canonical = result
        .as_ref()
        .expect("expected Some from detect_repo_root_sync inside a git repo")
        .canonicalize()
        .expect("canonicalize result path");
    assert_eq!(result_canonical, expected);
}

#[cfg(feature = "local_fs")]
#[test]
fn detect_repo_root_sync_returns_none_outside_repo() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    // A plain tempdir — not a git repo.
    let result = detect_repo_root_sync(dir.path());
    assert!(
        result.is_none(),
        "expected None outside a git repo, got {result:?}"
    );
}
