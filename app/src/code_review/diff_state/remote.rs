//! Remote diff state model.
//!
//! Client-side model for a single `(host_id, repo_path, mode)` diff state
//! received from the remote server. Presents the same read API as
//! `LocalDiffStateModel` and emits the same `DiffStateModelEvent` variants.
//! Mode is immutable — maps 1:1 to a pinned server-side model.

use std::path::PathBuf;
use std::sync::Arc;

use remote_server::manager::{RemoteServerManager, RemoteServerManagerEvent};
use remote_server::HostId;
use warp_util::remote_path::RemotePath;
use warp_util::standardized_path::StandardizedPath;
use warpui::{ModelContext, SingletonEntity};

use crate::remote_server::diff_state_proto;
use crate::util::git::{Commit, PrInfo};

use super::{
    DiffMetadata, DiffMode, DiffState, DiffStateModelEvent, DiffStats, FileDiffAndContent,
    GitDiffData, GitFileStatus,
};

// ── Internal state ───────────────────────────────────────────────────

#[derive(Default)]
enum InternalRemoteDiffState {
    #[default]
    Loading,
    NotInRepository,
    Loaded(GitDiffData),
    Error(String),
}

// ── Model ────────────────────────────────────────────────────────────

pub struct RemoteDiffStateModel {
    host_id: HostId,
    repo_path: StandardizedPath,
    mode: DiffMode,
    state: InternalRemoteDiffState,
    metadata: Option<DiffMetadata>,
}

impl warpui::Entity for RemoteDiffStateModel {
    type Event = DiffStateModelEvent;
}

impl RemoteDiffStateModel {
    /// Creates a new remote diff state model and initiates the `GetDiffState`
    /// request. The model starts in `Loading` state.
    pub fn new(
        host_id: HostId,
        repo_path: StandardizedPath,
        mode: DiffMode,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        // Subscribe to RemoteServerManager push events and filter by our
        // (host_id, repo_path, mode) triple.
        let mgr_handle = RemoteServerManager::handle(ctx);
        ctx.subscribe_to_model(&mgr_handle, Self::handle_manager_event);

        // Send the initial GetDiffState request through the manager.
        // Session is resolved at call time from the host_id — no need to
        // store it, which also handles reconnects transparently.
        let session_id = Self::find_connected_session(&host_id, ctx);
        if let Some(session_id) = session_id {
            let proto_mode = diff_state_proto::diff_mode_to_proto(&mode);
            mgr_handle.update(ctx, |mgr, ctx| {
                mgr.get_diff_state(session_id, repo_path.to_string(), proto_mode, ctx);
            });
        } else {
            log::warn!(
                "RemoteDiffStateModel: no connected session for host={host_id:?}, \
                 will wait for push events"
            );
        }

        Self {
            host_id,
            repo_path,
            mode,
            state: InternalRemoteDiffState::Loading,
            metadata: None,
        }
    }

    /// Resolves a connected session for `host_id` at call time.
    /// Returns `None` if no session is currently connected.
    fn find_connected_session(
        host_id: &HostId,
        ctx: &ModelContext<Self>,
    ) -> Option<warp_core::SessionId> {
        let mgr = RemoteServerManager::handle(ctx);
        let mgr_ref = mgr.as_ref(ctx);
        mgr_ref.sessions_for_host(host_id).and_then(|sessions| {
            sessions
                .iter()
                .copied()
                .find(|sid| mgr_ref.client_for_session(*sid).is_some())
        })
    }

    // ── Event handler ────────────────────────────────────────────────

    fn handle_manager_event(
        &mut self,
        event: &RemoteServerManagerEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        match event {
            RemoteServerManagerEvent::DiffStateSnapshotReceived {
                host_id,
                repo_path,
                mode,
                snapshot,
            } if self.matches(host_id, repo_path, mode) => {
                self.apply_snapshot(snapshot, ctx);
            }
            RemoteServerManagerEvent::DiffStateMetadataUpdateReceived {
                host_id,
                repo_path,
                mode,
                update,
            } if self.matches(host_id, repo_path, mode) => {
                if let Some(metadata) = &update.metadata {
                    self.apply_metadata_update(metadata, ctx);
                }
            }
            RemoteServerManagerEvent::DiffStateFileDeltaReceived {
                host_id,
                repo_path,
                mode,
                delta,
            } if self.matches(host_id, repo_path, mode) => {
                self.apply_file_delta(delta, ctx);
            }
            _ => {}
        }
    }

    /// Returns true if the event is for this model's (host_id, repo_path, mode).
    fn matches(
        &self,
        host_id: &HostId,
        repo_path: &StandardizedPath,
        mode: &remote_server::proto::DiffMode,
    ) -> bool {
        &self.host_id == host_id
            && &self.repo_path == repo_path
            && diff_state_proto::diff_mode_to_proto(&self.mode) == *mode
    }

    // ── Apply methods (proto → domain conversion + event emission) ────

    fn apply_snapshot(
        &mut self,
        snapshot: &remote_server::proto::DiffStateSnapshot,
        ctx: &mut ModelContext<Self>,
    ) {
        // Update metadata, detecting branch changes.
        if let Some(proto_meta) = &snapshot.metadata {
            let previous_branch = self
                .metadata
                .as_ref()
                .map(|m| m.current_branch_name.clone());
            let domain_meta = diff_state_proto::proto_to_diff_metadata(proto_meta);
            let current_branch = Some(domain_meta.current_branch_name.clone());
            self.metadata = Some(domain_meta.clone());

            if previous_branch != current_branch {
                ctx.emit(DiffStateModelEvent::CurrentBranchChanged);
            }
            ctx.emit(DiffStateModelEvent::MetadataRefreshed(domain_meta));
        }

        // Update state.
        let state = diff_state_proto::proto_to_diff_state(snapshot.state.as_ref());
        match state {
            DiffState::NotInRepository => {
                self.state = InternalRemoteDiffState::NotInRepository;
                ctx.emit(DiffStateModelEvent::NewDiffsComputed(None));
            }
            DiffState::Loading => {
                self.state = InternalRemoteDiffState::Loading;
                ctx.emit(DiffStateModelEvent::NewDiffsComputed(None));
            }
            DiffState::Error(msg) => {
                self.state = InternalRemoteDiffState::Error(msg);
                ctx.emit(DiffStateModelEvent::NewDiffsComputed(None));
            }
            DiffState::Loaded => {
                let domain_diffs = match &snapshot.diffs {
                    Some(proto_diffs) => diff_state_proto::proto_to_git_diff_data(proto_diffs),
                    None => GitDiffData {
                        files: vec![],
                        total_additions: 0,
                        total_deletions: 0,
                        files_changed: 0,
                    },
                };
                let base_content = diff_state_proto::git_diff_data_to_base_content(&domain_diffs);
                self.state = InternalRemoteDiffState::Loaded(domain_diffs);
                ctx.emit(DiffStateModelEvent::NewDiffsComputed(Some(Arc::new(
                    base_content,
                ))));
            }
        }
    }

    fn apply_metadata_update(
        &mut self,
        proto_meta: &remote_server::proto::DiffMetadata,
        ctx: &mut ModelContext<Self>,
    ) {
        let previous_branch = self
            .metadata
            .as_ref()
            .map(|m| m.current_branch_name.clone());
        let domain_meta = diff_state_proto::proto_to_diff_metadata(proto_meta);
        let current_branch = Some(domain_meta.current_branch_name.clone());
        self.metadata = Some(domain_meta.clone());

        if previous_branch != current_branch {
            ctx.emit(DiffStateModelEvent::CurrentBranchChanged);
        }
        ctx.emit(DiffStateModelEvent::MetadataRefreshed(domain_meta));
    }

    fn apply_file_delta(
        &mut self,
        delta: &remote_server::proto::DiffStateFileDelta,
        ctx: &mut ModelContext<Self>,
    ) {
        if let Some(proto_meta) = &delta.metadata {
            self.apply_metadata_update(proto_meta, ctx);
        }

        let InternalRemoteDiffState::Loaded(ref mut diffs) = self.state else {
            // Ignore file deltas until the initial snapshot has loaded.
            return;
        };

        let file_path = PathBuf::from(&delta.file_path);
        let domain_diff = delta
            .diff
            .as_ref()
            .map(diff_state_proto::proto_to_file_diff);

        if let Some(ref new_diff) = domain_diff {
            if let Some(pos) = diffs.files.iter().position(|f| f.file_path == file_path) {
                diffs.files[pos] = new_diff.clone();
            } else {
                diffs.files.push(new_diff.clone());
            }
        } else {
            diffs.files.retain(|f| f.file_path != file_path);
        }
        diffs.total_additions = diffs.files.iter().map(|f| f.additions()).sum();
        diffs.total_deletions = diffs.files.iter().map(|f| f.deletions()).sum();
        diffs.files_changed = diffs.files.len();

        let arc_diff = domain_diff.map(|fd| {
            Arc::new(FileDiffAndContent {
                file_diff: fd,
                content_at_head: None,
            })
        });
        ctx.emit(DiffStateModelEvent::SingleFileUpdated {
            path: file_path,
            diff: arc_diff,
        });
    }

    // ── Cleanup ──────────────────────────────────────────────────────

    /// Sends `UnsubscribeDiffState` to the server. Call before dropping the
    /// model (the wrapper calls it during mode switch / pane close).
    pub fn unsubscribe(&self, ctx: &mut ModelContext<Self>) {
        let Some(session_id) = Self::find_connected_session(&self.host_id, ctx) else {
            log::debug!(
                "RemoteDiffStateModel::unsubscribe: no connected session for host={:?}",
                self.host_id
            );
            return;
        };
        let proto_mode = diff_state_proto::diff_mode_to_proto(&self.mode);
        RemoteServerManager::handle(ctx)
            .as_ref(ctx)
            .unsubscribe_diff_state(session_id, self.repo_path.to_string(), proto_mode);
    }

    // ── Read API (matching LocalDiffStateModel interface) ────────────

    pub fn get(&self) -> DiffState {
        match &self.state {
            InternalRemoteDiffState::NotInRepository => DiffState::NotInRepository,
            InternalRemoteDiffState::Loading => DiffState::Loading,
            InternalRemoteDiffState::Loaded(_) => DiffState::Loaded,
            InternalRemoteDiffState::Error(msg) => DiffState::Error(msg.clone()),
        }
    }

    pub fn diff_mode(&self) -> DiffMode {
        self.mode.clone()
    }

    pub fn get_uncommitted_stats(&self) -> Option<DiffStats> {
        self.metadata
            .as_ref()
            .map(|m| m.against_head.aggregate_stats)
    }

    pub fn get_main_branch_name(&self) -> Option<String> {
        self.metadata
            .as_ref()
            .map(|m| m.main_branch_name.clone())
            .filter(|s| !s.is_empty())
    }

    pub fn get_current_branch_name(&self) -> Option<String> {
        self.metadata
            .as_ref()
            .map(|m| m.current_branch_name.clone())
            .filter(|s| !s.is_empty())
    }

    pub fn is_on_main_branch(&self) -> bool {
        self.metadata.as_ref().is_some_and(|m| {
            !m.current_branch_name.is_empty() && m.current_branch_name == m.main_branch_name
        })
    }

    pub fn unpushed_commits(&self) -> &[Commit] {
        self.metadata
            .as_ref()
            .map(|m| m.unpushed_commits.as_slice())
            .unwrap_or(&[])
    }

    pub fn upstream_ref(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.upstream_ref.as_deref())
    }

    pub fn upstream_differs_from_main(&self) -> bool {
        match (self.upstream_ref(), self.get_main_branch_name().as_deref()) {
            (Some(upstream), Some(main)) => upstream != main,
            _ => false,
        }
    }

    pub fn pr_info(&self) -> Option<&PrInfo> {
        self.metadata.as_ref().and_then(|m| m.pr_info.as_ref())
    }

    pub fn is_pr_info_refreshing(&self) -> bool {
        false
    }

    pub fn is_git_operation_blocked(&self, _ctx: &warpui::AppContext) -> bool {
        false
    }

    pub fn has_head(&self) -> bool {
        self.metadata.as_ref().is_some_and(|m| m.has_head_commit)
    }

    pub fn remote_path(&self) -> RemotePath {
        RemotePath::new(self.host_id.clone(), self.repo_path.clone())
    }

    // ── Write API ────────────────────────────────────────────────────

    /// Sends a `DiscardFiles` request to the remote server. The server's
    /// watcher will push updated diff snapshots on success.
    pub fn discard_files(
        &self,
        file_infos: Vec<super::FileStatusInfo>,
        should_stash: bool,
        branch_name: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(session_id) = Self::find_connected_session(&self.host_id, ctx) else {
            log::warn!(
                "RemoteDiffStateModel::discard_files: no connected session for host={:?}",
                self.host_id
            );
            return;
        };

        let proto_files: Vec<_> = file_infos
            .iter()
            .map(domain_file_status_info_to_proto)
            .collect();

        let repo_path = self.repo_path.to_string();
        RemoteServerManager::handle(ctx).update(ctx, |mgr, ctx| {
            mgr.discard_files(
                session_id,
                repo_path,
                proto_files,
                should_stash,
                branch_name,
                ctx,
            );
        });
    }
}

// ── Domain → Proto conversion helpers ─────────────────────────────────

fn domain_file_status_info_to_proto(
    info: &super::FileStatusInfo,
) -> remote_server::proto::FileStatusInfo {
    remote_server::proto::FileStatusInfo {
        path: info.path.to_string_lossy().to_string(),
        status: Some(domain_git_file_status_to_proto(&info.status)),
    }
}

fn domain_git_file_status_to_proto(status: &GitFileStatus) -> remote_server::proto::GitFileStatus {
    use remote_server::proto::git_file_status::Status;
    remote_server::proto::GitFileStatus {
        status: Some(match status {
            GitFileStatus::New => Status::NewFile(remote_server::proto::GitFileStatusNew {}),
            GitFileStatus::Modified => {
                Status::Modified(remote_server::proto::GitFileStatusModified {})
            }
            GitFileStatus::Deleted => {
                Status::Deleted(remote_server::proto::GitFileStatusDeleted {})
            }
            GitFileStatus::Renamed { old_path } => {
                Status::Renamed(remote_server::proto::GitFileStatusRenamed {
                    old_path: old_path.clone(),
                })
            }
            GitFileStatus::Copied { old_path } => {
                Status::Copied(remote_server::proto::GitFileStatusCopied {
                    old_path: old_path.clone(),
                })
            }
            GitFileStatus::Untracked => {
                Status::Untracked(remote_server::proto::GitFileStatusUntracked {})
            }
            GitFileStatus::Conflicted => {
                Status::Conflicted(remote_server::proto::GitFileStatusConflicted {})
            }
        }),
    }
}
