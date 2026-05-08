//! Conversion between diff state Rust types and proto-generated types.
//!
//! Rust types are canonical, proto types are the wire format.
//! Only the directions needed by the server are implemented here.
//!
//! This module lives in `app/` (rather than in the `remote_server` crate alongside
//! `repo_metadata_proto`) because it depends on app-level types
//! (`code_review::diff_state`, `util::git`) that are not available in the crate.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::proto;

use crate::code_review::diff_size_limits::DiffSize;
use crate::code_review::diff_state::{
    DiffHunk, DiffLine, DiffLineType, DiffMetadata, DiffMetadataAgainstBase, DiffMode, DiffState,
    DiffStats, FileDiff, FileDiffAndContent, FileStatusInfo, GitDiffData, GitDiffWithBaseContent,
    GitFileStatus,
};
use crate::util::git::{Commit, PrInfo};

// ── Proto → Rust (for incoming client messages) ────────────────────

/// Converts a proto `DiffMode` into a Rust `DiffMode`.
pub fn proto_to_diff_mode(proto_mode: &proto::DiffMode) -> DiffMode {
    match &proto_mode.mode {
        Some(proto::diff_mode::Mode::Head(_)) | None => DiffMode::Head,
        Some(proto::diff_mode::Mode::MainBranch(_)) => DiffMode::MainBranch,
        Some(proto::diff_mode::Mode::OtherBranch(ob)) => {
            DiffMode::OtherBranch(ob.branch_name.clone())
        }
    }
}

/// Converts a proto `GitFileStatus` into a Rust `GitFileStatus`.
pub fn proto_to_git_file_status(proto_status: &proto::GitFileStatus) -> GitFileStatus {
    match &proto_status.status {
        Some(proto::git_file_status::Status::NewFile(_)) => GitFileStatus::New,
        Some(proto::git_file_status::Status::Modified(_)) => GitFileStatus::Modified,
        Some(proto::git_file_status::Status::Deleted(_)) => GitFileStatus::Deleted,
        Some(proto::git_file_status::Status::Renamed(r)) => GitFileStatus::Renamed {
            old_path: r.old_path.clone(),
        },
        Some(proto::git_file_status::Status::Copied(c)) => GitFileStatus::Copied {
            old_path: c.old_path.clone(),
        },
        Some(proto::git_file_status::Status::Untracked(_)) => GitFileStatus::Untracked,
        Some(proto::git_file_status::Status::Conflicted(_)) => GitFileStatus::Conflicted,
        // Default to Modified for unrecognized/missing status.
        None => GitFileStatus::Modified,
    }
}

/// Converts a proto `FileStatusInfo` into a Rust `FileStatusInfo`.
pub fn proto_to_file_status_info(proto_info: &proto::FileStatusInfo) -> FileStatusInfo {
    FileStatusInfo {
        path: PathBuf::from(&proto_info.path),
        status: proto_info
            .status
            .as_ref()
            .map(proto_to_git_file_status)
            .unwrap_or(GitFileStatus::Modified),
    }
}

/// Converts a proto `DiffStats` into a Rust `DiffStats`.
pub fn proto_to_diff_stats(stats: &proto::DiffStats) -> DiffStats {
    DiffStats {
        files_changed: stats.files_changed as usize,
        total_additions: stats.total_additions as usize,
        total_deletions: stats.total_deletions as usize,
    }
}

fn proto_to_diff_metadata_against_base(
    base: &proto::DiffMetadataAgainstBase,
) -> DiffMetadataAgainstBase {
    DiffMetadataAgainstBase {
        aggregate_stats: base
            .aggregate_stats
            .as_ref()
            .map(proto_to_diff_stats)
            .unwrap_or_default(),
    }
}

fn proto_to_commit(commit: &proto::Commit) -> Commit {
    Commit {
        hash: commit.hash.clone(),
        subject: commit.subject.clone(),
        files_changed: commit.files_changed as usize,
        additions: commit.additions as usize,
        deletions: commit.deletions as usize,
    }
}

fn proto_to_pr_info(pr: &proto::PrInfo) -> PrInfo {
    PrInfo {
        number: pr.number,
        url: pr.url.clone(),
    }
}

/// Converts a proto `DiffMetadata` into a Rust `DiffMetadata`.
pub fn proto_to_diff_metadata(metadata: &proto::DiffMetadata) -> DiffMetadata {
    DiffMetadata {
        main_branch_name: metadata.main_branch_name.clone(),
        current_branch_name: metadata.current_branch_name.clone(),
        against_head: metadata
            .against_head
            .as_ref()
            .map(proto_to_diff_metadata_against_base)
            .unwrap_or_default(),
        against_base_branch: metadata
            .against_base_branch
            .as_ref()
            .map(proto_to_diff_metadata_against_base),
        has_head_commit: metadata.has_head_commit,
        unpushed_commits: metadata
            .unpushed_commits
            .iter()
            .map(proto_to_commit)
            .collect(),
        upstream_ref: metadata.upstream_ref.clone(),
        pr_info: metadata.pr_info.as_ref().map(proto_to_pr_info),
    }
}

fn proto_to_diff_line_type(line_type: i32) -> DiffLineType {
    match proto::DiffLineType::try_from(line_type) {
        Ok(proto::DiffLineType::Context) => DiffLineType::Context,
        Ok(proto::DiffLineType::Add) => DiffLineType::Add,
        Ok(proto::DiffLineType::Delete) => DiffLineType::Delete,
        Ok(proto::DiffLineType::HunkHeader) => DiffLineType::HunkHeader,
        _ => DiffLineType::Context,
    }
}

fn proto_to_diff_line(line: &proto::DiffLine) -> DiffLine {
    DiffLine {
        line_type: proto_to_diff_line_type(line.line_type),
        old_line_number: line.old_line_number.map(|n| n as usize),
        new_line_number: line.new_line_number.map(|n| n as usize),
        text: line.text.clone(),
        no_trailing_newline: line.no_trailing_newline,
    }
}

fn proto_to_diff_hunk(hunk: &proto::DiffHunk) -> DiffHunk {
    DiffHunk {
        old_start_line: hunk.old_start_line as usize,
        old_line_count: hunk.old_line_count as usize,
        new_start_line: hunk.new_start_line as usize,
        new_line_count: hunk.new_line_count as usize,
        lines: hunk.lines.iter().map(proto_to_diff_line).collect(),
        unified_diff_start: hunk.unified_diff_start as usize,
        unified_diff_end: hunk.unified_diff_end as usize,
    }
}

fn proto_to_diff_size(size: i32) -> DiffSize {
    match proto::DiffSize::try_from(size) {
        Ok(proto::DiffSize::Normal) => DiffSize::Normal,
        Ok(proto::DiffSize::Large) => DiffSize::Large,
        Ok(proto::DiffSize::Unrenderable) => DiffSize::Unrenderable,
        _ => DiffSize::Normal,
    }
}

/// Converts a proto `FileDiff` into a Rust `FileDiff`.
pub fn proto_to_file_diff(file: &proto::FileDiff) -> FileDiff {
    FileDiff {
        file_path: PathBuf::from(&file.file_path),
        status: file
            .status
            .as_ref()
            .map(proto_to_git_file_status)
            .unwrap_or(GitFileStatus::Modified),
        hunks: Arc::new(file.hunks.iter().map(proto_to_diff_hunk).collect()),
        is_binary: file.is_binary,
        is_autogenerated: file.is_autogenerated,
        max_line_number: file.max_line_number as usize,
        has_hidden_bidi_chars: file.has_hidden_bidi_chars,
        size: proto_to_diff_size(file.size),
    }
}

/// Converts a proto `GitDiffData` into a Rust `GitDiffData`.
pub fn proto_to_git_diff_data(data: &proto::GitDiffData) -> GitDiffData {
    GitDiffData {
        files: data.files.iter().map(proto_to_file_diff).collect(),
        total_additions: data.total_additions as usize,
        total_deletions: data.total_deletions as usize,
        files_changed: data.files_changed as usize,
    }
}

/// Converts a proto `DiffState` into a Rust `DiffState`.
pub fn proto_to_diff_state(state: Option<&proto::DiffState>) -> DiffState {
    let Some(state) = state else {
        return DiffState::Loading;
    };
    match &state.state {
        Some(proto::diff_state::State::NotInRepository(_)) => DiffState::NotInRepository,
        Some(proto::diff_state::State::Loading(_)) => DiffState::Loading,
        Some(proto::diff_state::State::Error(e)) => DiffState::Error(e.message.clone()),
        Some(proto::diff_state::State::Loaded(_)) => DiffState::Loaded,
        None => DiffState::Loading,
    }
}

/// Wraps a `GitDiffData` into a `GitDiffWithBaseContent` with `content_at_head: None`
/// for all files. Remote files don't have local base content.
pub fn git_diff_data_to_base_content(data: &GitDiffData) -> GitDiffWithBaseContent {
    GitDiffWithBaseContent {
        files: data
            .files
            .iter()
            .map(|f| FileDiffAndContent {
                file_diff: f.clone(),
                content_at_head: None,
            })
            .collect(),
        total_additions: data.total_additions,
        total_deletions: data.total_deletions,
        files_changed: data.files_changed,
    }
}

// ── Rust → Proto (for server pushes) ────────────────────────────────

pub fn diff_mode_to_proto(mode: &DiffMode) -> proto::DiffMode {
    let mode_oneof = match mode {
        DiffMode::Head => proto::diff_mode::Mode::Head(proto::DiffModeHead {}),
        DiffMode::MainBranch => proto::diff_mode::Mode::MainBranch(proto::DiffModeMainBranch {}),
        DiffMode::OtherBranch(branch) => {
            proto::diff_mode::Mode::OtherBranch(proto::DiffModeOtherBranch {
                branch_name: branch.clone(),
            })
        }
    };
    proto::DiffMode {
        mode: Some(mode_oneof),
    }
}

pub fn diff_stats_to_proto(stats: &DiffStats) -> proto::DiffStats {
    proto::DiffStats {
        files_changed: stats.files_changed as u64,
        total_additions: stats.total_additions as u64,
        total_deletions: stats.total_deletions as u64,
    }
}

fn diff_metadata_against_base_to_proto(
    m: &DiffMetadataAgainstBase,
) -> proto::DiffMetadataAgainstBase {
    proto::DiffMetadataAgainstBase {
        aggregate_stats: Some(diff_stats_to_proto(&m.aggregate_stats)),
    }
}

fn commit_to_proto(c: &Commit) -> proto::Commit {
    proto::Commit {
        hash: c.hash.clone(),
        subject: c.subject.clone(),
        files_changed: c.files_changed as u64,
        additions: c.additions as u64,
        deletions: c.deletions as u64,
    }
}

fn pr_info_to_proto(p: &PrInfo) -> proto::PrInfo {
    proto::PrInfo {
        number: p.number,
        url: p.url.clone(),
    }
}

pub fn diff_metadata_to_proto(m: &DiffMetadata) -> proto::DiffMetadata {
    proto::DiffMetadata {
        main_branch_name: m.main_branch_name.clone(),
        current_branch_name: m.current_branch_name.clone(),
        against_head: Some(diff_metadata_against_base_to_proto(&m.against_head)),
        against_base_branch: m
            .against_base_branch
            .as_ref()
            .map(diff_metadata_against_base_to_proto),
        has_head_commit: m.has_head_commit,
        unpushed_commits: m.unpushed_commits.iter().map(commit_to_proto).collect(),
        upstream_ref: m.upstream_ref.clone(),
        pr_info: m.pr_info.as_ref().map(pr_info_to_proto),
    }
}

fn git_file_status_to_proto(s: &GitFileStatus) -> proto::GitFileStatus {
    let status = match s {
        GitFileStatus::New => proto::git_file_status::Status::NewFile(proto::GitFileStatusNew {}),
        GitFileStatus::Modified => {
            proto::git_file_status::Status::Modified(proto::GitFileStatusModified {})
        }
        GitFileStatus::Deleted => {
            proto::git_file_status::Status::Deleted(proto::GitFileStatusDeleted {})
        }
        GitFileStatus::Renamed { old_path } => {
            proto::git_file_status::Status::Renamed(proto::GitFileStatusRenamed {
                old_path: old_path.clone(),
            })
        }
        GitFileStatus::Copied { old_path } => {
            proto::git_file_status::Status::Copied(proto::GitFileStatusCopied {
                old_path: old_path.clone(),
            })
        }
        GitFileStatus::Untracked => {
            proto::git_file_status::Status::Untracked(proto::GitFileStatusUntracked {})
        }
        GitFileStatus::Conflicted => {
            proto::git_file_status::Status::Conflicted(proto::GitFileStatusConflicted {})
        }
    };
    proto::GitFileStatus {
        status: Some(status),
    }
}

fn diff_line_type_to_proto(t: &DiffLineType) -> proto::DiffLineType {
    match t {
        DiffLineType::Context => proto::DiffLineType::Context,
        DiffLineType::Add => proto::DiffLineType::Add,
        DiffLineType::Delete => proto::DiffLineType::Delete,
        DiffLineType::HunkHeader => proto::DiffLineType::HunkHeader,
    }
}

fn diff_line_to_proto(l: &DiffLine) -> proto::DiffLine {
    proto::DiffLine {
        line_type: diff_line_type_to_proto(&l.line_type).into(),
        old_line_number: l.old_line_number.map(|n| n as u64),
        new_line_number: l.new_line_number.map(|n| n as u64),
        text: l.text.clone(),
        no_trailing_newline: l.no_trailing_newline,
    }
}

fn diff_hunk_to_proto(h: &DiffHunk) -> proto::DiffHunk {
    proto::DiffHunk {
        old_start_line: h.old_start_line as u64,
        old_line_count: h.old_line_count as u64,
        new_start_line: h.new_start_line as u64,
        new_line_count: h.new_line_count as u64,
        lines: h.lines.iter().map(diff_line_to_proto).collect(),
        unified_diff_start: h.unified_diff_start as u64,
        unified_diff_end: h.unified_diff_end as u64,
    }
}

fn diff_size_to_proto(s: &DiffSize) -> proto::DiffSize {
    match s {
        DiffSize::Normal => proto::DiffSize::Normal,
        DiffSize::Large => proto::DiffSize::Large,
        DiffSize::Unrenderable => proto::DiffSize::Unrenderable,
    }
}

pub fn file_diff_to_proto(f: &FileDiff, content_at_head: Option<&str>) -> proto::FileDiff {
    proto::FileDiff {
        file_path: f.file_path.to_string_lossy().to_string(),
        status: Some(git_file_status_to_proto(&f.status)),
        hunks: f.hunks.iter().map(diff_hunk_to_proto).collect(),
        is_binary: f.is_binary,
        is_autogenerated: f.is_autogenerated,
        max_line_number: f.max_line_number as u64,
        has_hidden_bidi_chars: f.has_hidden_bidi_chars,
        size: diff_size_to_proto(&f.size).into(),
        content_at_base: content_at_head.map(|s| s.to_string()),
    }
}

fn file_diff_and_content_to_proto(f: &FileDiffAndContent) -> proto::FileDiff {
    file_diff_to_proto(&f.file_diff, f.content_at_head.as_deref())
}

pub fn git_diff_data_to_proto(d: &GitDiffData) -> proto::GitDiffData {
    proto::GitDiffData {
        files: d
            .files
            .iter()
            .map(|f| file_diff_to_proto(f, None))
            .collect(),
        total_additions: d.total_additions as u64,
        total_deletions: d.total_deletions as u64,
        files_changed: d.files_changed as u64,
    }
}

pub fn git_diff_with_base_content_to_proto(d: &GitDiffWithBaseContent) -> proto::GitDiffData {
    proto::GitDiffData {
        files: d.files.iter().map(file_diff_and_content_to_proto).collect(),
        total_additions: d.total_additions as u64,
        total_deletions: d.total_deletions as u64,
        files_changed: d.files_changed as u64,
    }
}

fn diff_state_to_proto(state: &DiffState) -> proto::DiffState {
    let state_oneof = match state {
        DiffState::NotInRepository => {
            proto::diff_state::State::NotInRepository(proto::DiffStateNotInRepository {})
        }
        DiffState::Loading => proto::diff_state::State::Loading(proto::DiffStateLoading {}),
        DiffState::Error(msg) => proto::diff_state::State::Error(proto::DiffStateErrorValue {
            message: msg.clone(),
        }),
        DiffState::Loaded => proto::diff_state::State::Loaded(proto::DiffStateLoaded {}),
    };
    proto::DiffState {
        state: Some(state_oneof),
    }
}

// ── Higher-level message builders ───────────────────────────────────

/// Builds a `DiffStateSnapshot` proto message from the model's current state.
/// Uses `GitDiffData` (no `content_at_head`) — used for sync responses where
/// the model is already loaded and the full `GitDiffWithBaseContent` is no
/// longer available.
pub fn build_diff_state_snapshot(
    repo_path: &str,
    mode: &DiffMode,
    metadata: Option<&DiffMetadata>,
    state: &DiffState,
    diffs: Option<&GitDiffData>,
) -> proto::DiffStateSnapshot {
    proto::DiffStateSnapshot {
        repo_path: repo_path.to_string(),
        mode: Some(diff_mode_to_proto(mode)),
        metadata: metadata.map(diff_metadata_to_proto),
        state: Some(diff_state_to_proto(state)),
        diffs: diffs.map(git_diff_data_to_proto),
    }
}

/// Builds a `DiffStateSnapshot` with `content_at_head` populated per file.
/// Used for async responses (NewDiffsComputed) where the full
/// `GitDiffWithBaseContent` is available from the event.
pub fn build_diff_state_snapshot_with_content(
    repo_path: &str,
    mode: &DiffMode,
    metadata: Option<&DiffMetadata>,
    state: &DiffState,
    diffs: Option<&GitDiffWithBaseContent>,
) -> proto::DiffStateSnapshot {
    proto::DiffStateSnapshot {
        repo_path: repo_path.to_string(),
        mode: Some(diff_mode_to_proto(mode)),
        metadata: metadata.map(diff_metadata_to_proto),
        state: Some(diff_state_to_proto(state)),
        diffs: diffs.map(git_diff_with_base_content_to_proto),
    }
}

/// Builds a `DiffStateMetadataUpdate` proto message.
pub fn build_diff_state_metadata_update(
    repo_path: &str,
    mode: &DiffMode,
    metadata: &DiffMetadata,
) -> proto::DiffStateMetadataUpdate {
    proto::DiffStateMetadataUpdate {
        repo_path: repo_path.to_string(),
        mode: Some(diff_mode_to_proto(mode)),
        metadata: Some(diff_metadata_to_proto(metadata)),
    }
}

/// Builds a `DiffStateFileDelta` proto message.
pub fn build_diff_state_file_delta(
    repo_path: &str,
    mode: &DiffMode,
    file_path: &Path,
    diff: Option<&FileDiffAndContent>,
    metadata: Option<&DiffMetadata>,
) -> proto::DiffStateFileDelta {
    proto::DiffStateFileDelta {
        repo_path: repo_path.to_string(),
        mode: Some(diff_mode_to_proto(mode)),
        file_path: file_path.to_string_lossy().to_string(),
        diff: diff.map(file_diff_and_content_to_proto),
        metadata: metadata.map(diff_metadata_to_proto),
    }
}
