use std::path::Path;
use std::{collections::HashSet, future::Future, path::PathBuf};

#[cfg(test)]
use virtual_fs::{Stub, VirtualFS};
use warp_util::standardized_path::StandardizedPath;
#[cfg(test)]
use warpui::r#async::FutureId;
use warpui::{AppContext, Entity, ModelContext, ModelHandle};

use crate::DirectoryWatcher;
use crate::Repository;
use futures::future::{ready, Either};
use warpui::SingletonEntity;

/// Indicates why a repository detection event was emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoDetectionSource {
    /// User actively navigated to this repo in a terminal (via cd/pwd change).
    TerminalNavigation,
    /// Repo was detected during project rules indexing.
    ProjectRulesIndexing,
    /// Repo was detected for code review/diff state initialization.
    CodeReviewInitialization,
    /// Repo was cloned or discovered during cloud agent environment preparation.
    CloudEnvironmentPrep,
}

pub enum DetectedRepositoriesEvent {
    DetectedGitRepo {
        repository: ModelHandle<Repository>,
        source: RepoDetectionSource,
    },
}

/// Tracks the detected _git_ repositories during the lifetime of the application. This should be the canonical source of truth for repository information.
#[derive(Default)]
pub struct DetectedRepositories {
    repository_roots: HashSet<StandardizedPath>,
    child_repo_scan_roots_in_flight: HashSet<StandardizedPath>,
    #[cfg(test)]
    /// List of spawned background tasks, for testing.
    spawned_futures: Vec<FutureId>,
}

impl DetectedRepositories {
    /// Given the active directory pwd, kick off a background job to detect the git project root and emit an event
    /// to interested listeners.
    #[cfg_attr(not(feature = "local_fs"), allow(unused_variables))]
    pub fn detect_possible_git_repo(
        &mut self,
        active_directory: &str,
        source: RepoDetectionSource,
        ctx: &mut ModelContext<Self>,
    ) -> impl Future<Output = Option<PathBuf>> {
        #[cfg(feature = "local_fs")]
        {
            use futures::channel::oneshot;

            let Ok(path) = StandardizedPath::from_local_canonicalized(Path::new(active_directory))
            else {
                return Either::Right(ready(None));
            };

            if let Some(repository) = self.repository_roots.get(&path) {
                if let Some(local_path) = repository.to_local_path() {
                    if let Some(repository) =
                        DirectoryWatcher::as_ref(ctx).get_watched_directory_for_path(&local_path)
                    {
                        ctx.emit(DetectedRepositoriesEvent::DetectedGitRepo {
                            repository: repository.clone(),
                            source,
                        });
                    }
                }
                return Either::Right(ready(repository.to_local_path()));
            };

            let local_path_for_search = path.to_local_path();
            let child_repo_scan_root = matches!(source, RepoDetectionSource::TerminalNavigation)
                .then(|| path.clone())
                .filter(|path| self.child_repo_scan_roots_in_flight.insert(path.clone()));
            let should_detect_child_repos = child_repo_scan_root.is_some();
            let (tx, rx) = oneshot::channel::<Option<PathBuf>>();
            let spawned_handle = ctx.spawn(
                async move {
                    if let Some(local_path) = local_path_for_search {
                        let containing_repo = find_git_repo(&local_path).await;
                        let child_repos = if should_detect_child_repos && containing_repo.is_none()
                        {
                            find_child_git_repos(&local_path).await
                        } else {
                            Vec::new()
                        };
                        GitRepoDetectionResult {
                            containing_repo,
                            child_repos,
                        }
                    } else {
                        GitRepoDetectionResult {
                            containing_repo: None,
                            child_repos: Vec::new(),
                        }
                    }
                },
                move |me, res, ctx| {
                    if let Some(path) = child_repo_scan_root {
                        me.child_repo_scan_roots_in_flight.remove(&path);
                    }

                    let containing_repo_path = res
                        .containing_repo
                        .and_then(|info| me.register_git_repo_info(info, source, ctx));

                    for child_repo in res.child_repos {
                        me.register_git_repo_info(child_repo, source, ctx);
                    }

                    let _ = tx.send(containing_repo_path);
                },
            );

            #[cfg(not(test))]
            let _ = spawned_handle;

            #[cfg(test)]
            self.spawned_futures.push(spawned_handle.future_id());

            Either::Left(async move { rx.await.unwrap_or(None) })
        }

        #[cfg(not(feature = "local_fs"))]
        {
            use futures::future::Ready;
            Either::<Ready<Option<PathBuf>>, Ready<Option<PathBuf>>>::Left(ready(None))
        }
    }

    #[cfg(test)]
    pub fn spawned_futures(&self) -> &[FutureId] {
        &self.spawned_futures
    }

    /// Given a path, return its corresdponding watched repository, if any.
    pub fn get_watched_repo_for_path(
        &self,
        path: &Path,
        ctx: &AppContext,
    ) -> Option<ModelHandle<Repository>> {
        let root = self.get_root_for_path(path)?;
        DirectoryWatcher::as_ref(ctx).get_watched_directory_for_path(&root)
    }

    /// Given a path, return its corresponding repo root. Note that this does not run the check
    /// against the actual file system. Instead it checks against our cached path to root mapping.
    pub fn get_root_for_path(&self, path: &Path) -> Option<PathBuf> {
        let std_path = StandardizedPath::from_local_canonicalized(path).ok()?;
        let repo = self.find_repository_root(&std_path)?;
        repo.to_local_path()
    }

    /// Given a path, return repository roots already detected below that path.
    ///
    /// This is used for directory-style workspaces where the active terminal is
    /// in a non-git parent directory that directly contains multiple repositories.
    /// Detection is still explicit and cached: this method does not scan the
    /// filesystem.
    pub fn get_descendant_roots_for_path(&self, path: &Path) -> Vec<PathBuf> {
        let Ok(std_path) = StandardizedPath::from_local_canonicalized(path) else {
            return Vec::new();
        };

        let mut roots = self
            .repository_roots
            .iter()
            .filter(|repo| *repo != &std_path && repo.starts_with(&std_path))
            .filter_map(StandardizedPath::to_local_path)
            .filter(|repo| repo.exists())
            .collect::<Vec<_>>();
        roots.sort();
        roots
    }

    /// Find the repository that contains the given path, if any.
    fn find_repository_root(&self, path: &StandardizedPath) -> Option<StandardizedPath> {
        let mut current = Some(path.clone());
        while let Some(ancestor) = current {
            if let Some(repo) = self.repository_roots.get(&ancestor) {
                return Some(repo.clone());
            }
            current = ancestor.parent();
        }
        None
    }

    #[cfg(feature = "local_fs")]
    fn register_git_repo_info(
        &mut self,
        info: GitRepoInfo,
        source: RepoDetectionSource,
        ctx: &mut ModelContext<Self>,
    ) -> Option<PathBuf> {
        let repo_root_path = info
            .working_tree_path
            .as_ref()
            .and_then(|path| StandardizedPath::from_local_canonicalized(path).ok())?;

        self.repository_roots.insert(repo_root_path.clone());

        let external_git_dir =
            StandardizedPath::from_local_canonicalized(info.git_dir_path.as_path())
                .ok()
                // Only treat as external if it's outside the working tree.
                .filter(|p| !p.starts_with(&repo_root_path));

        let repository = DirectoryWatcher::handle(ctx).update(ctx, |watcher, ctx| {
            watcher
                .add_directory_with_git_dir(repo_root_path, external_git_dir, ctx)
                .ok()
        })?;

        let repo_path = repository.as_ref(ctx).root_dir().to_local_path();
        ctx.emit(DetectedRepositoriesEvent::DetectedGitRepo { repository, source });
        repo_path
    }
}

impl Entity for DetectedRepositories {
    type Event = DetectedRepositoriesEvent;
}

impl SingletonEntity for DetectedRepositories {}

/// Test helpers: direct mutation of internal state.
#[cfg(any(test, feature = "test-util"))]
impl DetectedRepositories {
    /// Insert a repository root path directly, bypassing git detection.
    pub fn insert_test_repo_root(&mut self, path: StandardizedPath) {
        self.repository_roots.insert(path);
    }
}

/// Information about a discovered Git repository.
#[cfg(feature = "local_fs")]
#[derive(Debug, Clone)]
struct GitRepoInfo {
    /// Path to the working tree, if present. None for bare repositories.
    working_tree_path: Option<PathBuf>,
    /// Path to the git directory (contains objects, refs, and index).
    /// We can watch the HEAD file for branch changes, but currently don't do so.
    git_dir_path: PathBuf,
}

#[cfg(feature = "local_fs")]
struct GitRepoDetectionResult {
    containing_repo: Option<GitRepoInfo>,
    child_repos: Vec<GitRepoInfo>,
}

/// Finds the Git repository containing the given path, if any.
///
/// Supports:
/// - A .git directory containing a HEAD file: parent directory is the working tree, .git is the git dir
/// - A <project>.git directory containing a HEAD file (bare repo): that directory is the git dir and there is no working tree
/// - A .git file containing "gitdir: <path>": working tree is the parent directory; git dir is the parsed path (resolved if relative)
///
/// Traverses up to the user's $HOME directory; if no repo is found by that point, returns `None`.
#[cfg(feature = "local_fs")]
async fn find_git_repo(path: &Path) -> Option<GitRepoInfo> {
    let home_dir = dirs::home_dir()?;
    let mut current = path.to_owned();

    loop {
        if current == home_dir {
            return None;
        }

        if let Some(info) = find_git_repo_at_path(&current).await {
            return Some(info);
        }

        if !current.pop() {
            return None;
        }
    }
}

/// Finds a Git repository whose working tree is exactly `path`.
#[cfg(feature = "local_fs")]
async fn find_git_repo_at_path(path: &Path) -> Option<GitRepoInfo> {
    // First, check if the current directory is a bare git repository.
    if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
        if dir_name.ends_with(".git") && is_valid_git_dir(path).await {
            return Some(GitRepoInfo {
                working_tree_path: None,
                git_dir_path: path.to_path_buf(),
            });
        }
    }

    // Check for a .git directory.
    let dot_git_path = path.join(".git");
    if let Ok(dot_git_type) = async_fs::symlink_metadata(&dot_git_path)
        .await
        .map(|m| m.file_type())
    {
        if dot_git_type.is_dir() {
            // A standard repository with a .git directory.
            if is_valid_git_dir(&dot_git_path).await {
                return Some(GitRepoInfo {
                    working_tree_path: Some(path.to_path_buf()),
                    git_dir_path: dot_git_path,
                });
            }
        } else if dot_git_type.is_file() {
            // A potential gitfile, used by worktrees and submodules.
            if let Ok(contents) = async_fs::read_to_string(&dot_git_path).await {
                // Typical format: "gitdir: <path>\n"
                if let Some(rest) = contents.trim().strip_prefix("gitdir:") {
                    let gitdir_path = PathBuf::from(rest.trim());
                    let resolved_gitdir = if gitdir_path.is_absolute() {
                        gitdir_path
                    } else {
                        path.join(gitdir_path)
                    };
                    if is_valid_git_dir(&resolved_gitdir).await {
                        return Some(GitRepoInfo {
                            working_tree_path: Some(path.to_path_buf()),
                            git_dir_path: resolved_gitdir,
                        });
                    }
                }
            }
        }
    }

    None
}

/// Finds git repositories in the direct children of `path`.
#[cfg(feature = "local_fs")]
async fn find_child_git_repos(path: &Path) -> Vec<GitRepoInfo> {
    use futures::TryStreamExt;

    let Ok(mut entries) = async_fs::read_dir(path).await else {
        return Vec::new();
    };

    let mut child_dirs = Vec::new();
    while let Ok(Some(entry)) = entries.try_next().await {
        let child_path = entry.path();
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        if child_path.file_name().and_then(|name| name.to_str()) == Some(".git") {
            continue;
        }
        child_dirs.push(child_path);
    }

    child_dirs.sort();

    let mut repos = Vec::new();
    for child_dir in child_dirs {
        if let Some(info) = find_git_repo_at_path(&child_dir).await {
            if info.working_tree_path.as_deref() == Some(child_dir.as_path()) {
                repos.push(info);
            }
        }
    }
    repos
}

/// Checks whether the given directory is a valid Git directory by verifying it contains a HEAD file.
#[cfg(feature = "local_fs")]
async fn is_valid_git_dir(dir: &Path) -> bool {
    async_fs::metadata(dir.join("HEAD"))
        .await
        .map(|m| m.is_file())
        .unwrap_or(false)
}

/// Helper function to stub a git repository in a VirtualFS with the given repository directory name.
#[cfg(test)]
pub(crate) fn stub_git_repository(vfs: &mut VirtualFS, repo_name: &str) {
    let objects_dir = format!("{repo_name}/.git/objects");
    vfs.mkdir(&objects_dir);

    let head_path = format!("{repo_name}/.git/HEAD");
    let config_path = format!("{repo_name}/.git/config");
    vfs.with_files(vec![
        Stub::FileWithContent(&head_path, "ref: refs/heads/main"),
        Stub::FileWithContent(&config_path, "[core]\n\trepositoryformatversion = 0"),
    ]);
}

#[cfg(test)]
#[path = "repositories_tests.rs"]
mod tests;
