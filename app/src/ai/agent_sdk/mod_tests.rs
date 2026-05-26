use super::{
    command_requires_auth, command_requires_cloud_services, command_requires_hosted_auth,
    command_to_telemetry_event, reconcile_task_harness, should_create_hosted_task_for_local_run,
};
use clap::Parser;
use serde_json::json;
use warp_cli::{
    agent::AgentCommand,
    agent::Harness,
    artifact::{ArtifactCommand, DownloadArtifactArgs, GetArtifactArgs, UploadArtifactArgs},
    task::{MessageCommand, MessageSendArgs, MessageWatchArgs, TaskCommand},
    Args, CliCommand, Command,
};
use warp_core::channel::ChannelState;
use warp_core::telemetry::TelemetryEvent;

const TASK_ID: &str = "00000000-0000-0000-0000-000000000001";

#[test]
fn logout_does_not_require_auth() {
    assert!(!command_requires_auth(&CliCommand::Logout));
}

#[test]
fn login_does_not_require_auth() {
    assert!(!command_requires_auth(&CliCommand::Login));
}

#[test]
fn artifact_download_requires_auth() {
    assert!(command_requires_hosted_auth(&CliCommand::Artifact(
        ArtifactCommand::Download(DownloadArtifactArgs {
            artifact_uid: "artifact-123".to_string(),
            out: None,
        },)
    )));
}

#[test]
fn run_message_send_requires_auth() {
    assert!(command_requires_hosted_auth(&CliCommand::Run(
        TaskCommand::Message(MessageCommand::Send(MessageSendArgs {
            to: vec!["run-456".to_string()],
            subject: "subject".to_string(),
            body: "body".to_string(),
            sender_run_id: "run-123".to_string(),
        }),)
    )));
}

#[test]
fn artifact_get_requires_auth() {
    assert!(command_requires_hosted_auth(&CliCommand::Artifact(
        ArtifactCommand::Get(GetArtifactArgs {
            artifact_uid: "artifact-123".to_string(),
        },)
    )));
}

#[test]
fn artifact_upload_requires_auth() {
    assert!(command_requires_hosted_auth(&CliCommand::Artifact(
        ArtifactCommand::Upload(UploadArtifactArgs {
            path: "artifact.txt".into(),
            run_id: Some("run-123".to_string()),
            conversation_id: None,
            description: None,
        },)
    )));
}

#[test]
fn oss_cli_agent_run_does_not_require_login() {
    assert!(!ChannelState::cloud_services_available());

    let command = parse_cli_command(["cast-codes", "agent", "run", "--prompt", "hello"]);

    assert!(matches!(command, CliCommand::Agent(AgentCommand::Run(_))));
    assert!(!command_requires_auth(&command));
}

#[test]
fn oss_cli_agent_run_is_local_only_by_default() {
    assert!(!ChannelState::cloud_services_available());

    let command = parse_cli_command(["cast-codes", "agent", "run", "--prompt", "hello"]);

    assert!(!command_requires_cloud_services(&command));
    assert!(!should_create_hosted_task_for_local_run());
}

#[test]
fn oss_cli_rejects_account_and_cloud_commands() {
    assert!(!ChannelState::cloud_services_available());

    let login = parse_cli_command(["cast-codes", "login"]);
    let run_cloud = parse_cli_command(["cast-codes", "agent", "run-cloud", "--prompt", "hello"]);
    let task_run = parse_cli_command(["cast-codes", "run", "list"]);

    assert!(command_requires_cloud_services(&login));
    assert!(command_requires_cloud_services(&run_cloud));
    assert!(command_requires_cloud_services(&task_run));
}

#[test]
fn oss_cli_rejects_cloud_options_on_local_agent_run() {
    assert!(!ChannelState::cloud_services_available());

    let task_backed_run = parse_cli_command(["cast-codes", "agent", "run", "--task-id", TASK_ID]);
    let shared_run =
        parse_cli_command(["cast-codes", "agent", "run", "--prompt", "hello", "--share"]);

    assert!(command_requires_cloud_services(&task_backed_run));
    assert!(command_requires_cloud_services(&shared_run));
}

fn parse_cli_command<const N: usize>(args: [&str; N]) -> CliCommand {
    let parsed = Args::try_parse_from(args).unwrap();
    match parsed.command().expect("expected command") {
        Command::CommandLine(command) => command.as_ref().clone(),
        command => panic!("expected CLI command, got {command:?}"),
    }
}

#[test]
#[serial_test::serial]
fn run_message_send_telemetry_uses_canonical_harness_from_env() {
    std::env::set_var("OZ_HARNESS", "  CLAUDE  ");
    let event = command_to_telemetry_event(&CliCommand::Run(TaskCommand::Message(
        MessageCommand::Send(MessageSendArgs {
            to: vec!["run-456".to_string()],
            subject: "subject".to_string(),
            body: "body".to_string(),
            sender_run_id: "run-123".to_string(),
        }),
    )));
    std::env::remove_var("OZ_HARNESS");

    assert_eq!(event.payload(), Some(json!({ "harness": "claude" })));
}

#[test]
#[serial_test::serial]
fn run_message_send_telemetry_supports_claude_code_alias() {
    std::env::set_var("OZ_HARNESS", "CLAUDE_CODE");
    let event = command_to_telemetry_event(&CliCommand::Run(TaskCommand::Message(
        MessageCommand::Send(MessageSendArgs {
            to: vec!["run-456".to_string()],
            subject: "subject".to_string(),
            body: "body".to_string(),
            sender_run_id: "run-123".to_string(),
        }),
    )));
    std::env::remove_var("OZ_HARNESS");

    assert_eq!(event.payload(), Some(json!({ "harness": "claude" })));
}

#[test]
#[serial_test::serial]
fn run_message_send_telemetry_supports_opencode_harness() {
    std::env::set_var("OZ_HARNESS", "opencode");
    let event = command_to_telemetry_event(&CliCommand::Run(TaskCommand::Message(
        MessageCommand::Send(MessageSendArgs {
            to: vec!["run-456".to_string()],
            subject: "subject".to_string(),
            body: "body".to_string(),
            sender_run_id: "run-123".to_string(),
        }),
    )));
    std::env::remove_var("OZ_HARNESS");

    assert_eq!(event.payload(), Some(json!({ "harness": "opencode" })));
}

#[test]
#[serial_test::serial]
fn run_message_send_telemetry_defaults_to_unknown_harness() {
    std::env::remove_var("OZ_HARNESS");
    let event = command_to_telemetry_event(&CliCommand::Run(TaskCommand::Message(
        MessageCommand::Send(MessageSendArgs {
            to: vec!["run-456".to_string()],
            subject: "subject".to_string(),
            body: "body".to_string(),
            sender_run_id: "run-123".to_string(),
        }),
    )));

    assert_eq!(event.payload(), Some(json!({ "harness": "unknown" })));
}

#[test]
fn reconcile_task_harness_adopts_task_harness_when_cli_uses_default() {
    let mut selected_harness = Harness::Oz;
    let harness = reconcile_task_harness(TASK_ID, &mut selected_harness, Harness::Claude)
        .expect("default harness should adopt task harness");

    assert_eq!(selected_harness, Harness::Claude);
    assert_eq!(harness.harness(), Harness::Claude);
}

#[test]
fn reconcile_task_harness_allows_matching_explicit_harness() {
    let mut selected_harness = Harness::Claude;
    let harness = reconcile_task_harness(TASK_ID, &mut selected_harness, Harness::Claude)
        .expect("matching harness should succeed");

    assert_eq!(selected_harness, Harness::Claude);
    assert_eq!(harness.harness(), Harness::Claude);
}

#[test]
fn reconcile_task_harness_rejects_explicit_mismatch() {
    let mut selected_harness = Harness::Gemini;
    let err = reconcile_task_harness(TASK_ID, &mut selected_harness, Harness::Claude)
        .expect_err("mismatched harness should fail");

    assert_eq!(selected_harness, Harness::Gemini);
    assert!(err.to_string().contains("Task"));
    assert!(err.to_string().contains("--harness gemini"));
    assert!(err.to_string().contains("claude"));
}

#[test]
#[serial_test::serial]
fn run_message_watch_telemetry_defaults_to_unknown_harness() {
    std::env::remove_var("OZ_HARNESS");
    let event = command_to_telemetry_event(&CliCommand::Run(TaskCommand::Message(
        MessageCommand::Watch(MessageWatchArgs {
            run_id: "run-123".to_string(),
            since_sequence: 0,
        }),
    )));

    assert_eq!(event.payload(), Some(json!({ "harness": "unknown" })));
}
