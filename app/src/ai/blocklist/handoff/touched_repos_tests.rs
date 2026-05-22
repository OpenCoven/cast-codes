//! Tests for `touched_repos.rs`.
//!
//! Only covers `find_git_root`, which actually walks the filesystem against a
//! temporary directory layout. The pure helpers (`parse_github_repo`,
//! `pick_handoff_overlap_env`) are exercised end-to-end by the handoff submit
//! path and don't get standalone tests — their correctness is enforced by
//! their call sites.

use super::*;
use crate::ai::agent::task::TaskId;
use crate::ai::agent::{
    AIAgentActionId, AIAgentExchange, AIAgentExchangeId, AIAgentInput, AIAgentOutput,
    AIAgentOutputMessage, AIAgentOutputStatus, FileEdit, FinishedAIAgentOutput, MessageId, Shared,
    UploadArtifactRequest,
};
use crate::ai::llms::LLMId;
use chrono::Local;
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::runtime::Runtime;

#[test]
fn find_git_root_walks_up_to_dot_git() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let nested = repo.join("src").join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(repo.join(".git")).unwrap();

    let file_in_repo = nested.join("foo.rs");
    fs::write(&file_in_repo, "").unwrap();

    let outside = tmp.path().join("not_a_repo").join("file.txt");
    fs::create_dir_all(outside.parent().unwrap()).unwrap();
    fs::write(&outside, "").unwrap();

    let rt = Runtime::new().unwrap();
    let (root_for_file, root_for_dir, root_for_outside) = rt.block_on(async {
        (
            find_git_root(&file_in_repo).await,
            find_git_root(&nested).await,
            find_git_root(&outside).await,
        )
    });

    assert_eq!(root_for_file.expect("root for file inside repo"), repo);
    assert_eq!(root_for_dir.expect("root for directory inside repo"), repo);
    assert!(root_for_outside.is_none());
}

#[test]
fn extract_paths_ignores_cancelled_or_failed_file_edit_results() {
    let cwd = "/repo";
    let cancelled = action_id("cancelled-edit");
    let failed = action_id("failed-edit");
    let successful = action_id("successful-edit");
    let exchange = exchange_with_actions(
        Some(cwd),
        vec![
            file_edit_action(cancelled.clone(), "cancelled.txt"),
            file_edit_action(failed.clone(), "failed.txt"),
            file_edit_action(successful.clone(), "written.txt"),
        ],
        vec![
            file_edit_result(cancelled, RequestFileEditsResult::Cancelled),
            file_edit_result(
                failed,
                RequestFileEditsResult::DiffApplicationFailed {
                    error: "patch failed".to_string(),
                },
            ),
            file_edit_result(successful, successful_file_edit_result()),
        ],
    );

    let paths = extract_paths_from_exchange_history(vec![&exchange]);

    assert_eq!(
        paths,
        vec![PathBuf::from(cwd), PathBuf::from("/repo/written.txt")]
    );
}

#[test]
fn extract_paths_ignores_failed_upload_artifacts() {
    let failed = action_id("failed-upload");
    let successful = action_id("successful-upload");
    let exchange = exchange_with_actions(
        Some("/repo/subdir"),
        vec![
            upload_action(failed.clone(), "../failed.log"),
            upload_action(successful.clone(), "../artifact.log"),
        ],
        vec![
            upload_result(
                failed,
                UploadArtifactResult::Error("upload failed".to_string()),
            ),
            upload_result(
                successful,
                UploadArtifactResult::Success {
                    artifact_uid: "artifact-uid".to_string(),
                    filepath: Some("../artifact.log".to_string()),
                    mime_type: "text/plain".to_string(),
                    description: None,
                    size_bytes: 128,
                },
            ),
        ],
    );

    let paths = extract_paths_from_exchange_history(vec![&exchange]);

    assert_eq!(
        paths,
        vec![
            PathBuf::from("/repo/subdir"),
            PathBuf::from("/repo/subdir/../artifact.log")
        ]
    );
}

fn action_id(value: &str) -> AIAgentActionId {
    value.to_string().into()
}

fn task_id() -> TaskId {
    TaskId::new("task".to_string())
}

fn file_edit_action(id: AIAgentActionId, file: &str) -> AIAgentAction {
    AIAgentAction {
        id,
        task_id: task_id(),
        action: AIAgentActionType::RequestFileEdits {
            file_edits: vec![FileEdit::Create {
                file: Some(file.to_string()),
                content: Some(String::new()),
            }],
            title: None,
        },
        requires_result: true,
    }
}

fn upload_action(id: AIAgentActionId, file_path: &str) -> AIAgentAction {
    AIAgentAction {
        id,
        task_id: task_id(),
        action: AIAgentActionType::UploadArtifact(UploadArtifactRequest {
            file_path: file_path.to_string(),
            description: None,
        }),
        requires_result: true,
    }
}

fn file_edit_result(id: AIAgentActionId, result: RequestFileEditsResult) -> AIAgentInput {
    AIAgentInput::ActionResult {
        result: AIAgentActionResult {
            id,
            task_id: task_id(),
            result: AIAgentActionResultType::RequestFileEdits(result),
        },
        context: Arc::from([]),
    }
}

fn upload_result(id: AIAgentActionId, result: UploadArtifactResult) -> AIAgentInput {
    AIAgentInput::ActionResult {
        result: AIAgentActionResult {
            id,
            task_id: task_id(),
            result: AIAgentActionResultType::UploadArtifact(result),
        },
        context: Arc::from([]),
    }
}

fn successful_file_edit_result() -> RequestFileEditsResult {
    RequestFileEditsResult::Success {
        diff: String::new(),
        updated_files: vec![],
        deleted_files: vec![],
        lines_added: 0,
        lines_removed: 0,
    }
}

fn exchange_with_actions(
    working_directory: Option<&str>,
    actions: Vec<AIAgentAction>,
    input: Vec<AIAgentInput>,
) -> AIAgentExchange {
    AIAgentExchange {
        id: AIAgentExchangeId::new(),
        input,
        output_status: AIAgentOutputStatus::Finished {
            finished_output: FinishedAIAgentOutput::Success {
                output: Shared::new(AIAgentOutput {
                    messages: actions
                        .into_iter()
                        .enumerate()
                        .map(|(index, action)| AIAgentOutputMessage {
                            id: MessageId::new(format!("message-{index}")),
                            message: AIAgentOutputMessageType::Action(action),
                            citations: vec![],
                        })
                        .collect(),
                    ..Default::default()
                }),
            },
        },
        added_message_ids: HashSet::new(),
        start_time: Local::now(),
        finish_time: None,
        time_to_first_token_ms: None,
        working_directory: working_directory.map(ToString::to_string),
        model_id: LLMId::from("test"),
        request_cost: None,
        coding_model_id: LLMId::from("test"),
        cli_agent_model_id: LLMId::from("test"),
        computer_use_model_id: LLMId::from("test"),
        response_initiator: None,
    }
}
