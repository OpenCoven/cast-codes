use clap::{Args, Subcommand};

use crate::{
    config_file::ConfigFileArgs,
    environment::{EnvironmentCreateArgs, EnvironmentUpdateArgs},
    mcp::MCPSpec,
    model::ModelArgs,
    scope::ObjectScope,
    skill::SkillSpec,
};

/// `ScheduleCommand` has a slightly unusual definition because we allow `oz schedule` as
// a shorthand for `oz schedule create`.
#[derive(Debug, Clone, Args)]
#[clap(args_conflicts_with_subcommands = true)]
pub struct ScheduleCommand {
    #[clap(subcommand)]
    subcommand: Option<ScheduleSubcommand>,

    #[clap(flatten)]
    create: Option<CreateScheduleArgs>,
}

impl ScheduleCommand {
    /// Get the specific scheduling subcommand. Returns `None` if using the `oz schedule` creation shorthand.
    pub fn subcommand(&self) -> Option<&ScheduleSubcommand> {
        self.subcommand.as_ref()
    }

    /// Convert into the specific scheduling subcommand to run.
    pub fn into_subcommand(self) -> ScheduleSubcommand {
        if let Some(create) = self.create {
            ScheduleSubcommand::Create(create)
        } else if let Some(cmd) = self.subcommand {
            cmd.into_runnable_subcommand()
        } else {
            panic!("Either subcommand or create args are required");
        }
    }
}

/// Schedule-related subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum ScheduleSubcommand {
    /// Create a scheduled Oz agent.
    Create(CreateScheduleArgs),
    /// Create a scheduled agent that turns OpenCoven Feedback into follow-up tasks.
    Feedback(FeedbackScheduleArgs),
    /// List scheduled Oz agents.
    List,
    /// Get a scheduled Oz agent's configuration.
    Get(GetScheduleArgs),
    /// Update a scheduled Oz agent.
    Update(UpdateScheduleArgs),
    /// Pause a scheduled Oz agent.
    ///
    /// A paused agent still exists, but will not run according to its schedule.
    Pause(PauseScheduleArgs),
    /// Unpause a scheduled Oz agent.
    ///
    /// The agent will resume executing on its previously-configured schedule.
    #[command(alias = "resume")]
    Unpause(UnpauseScheduleArgs),
    /// Delete a scheduled Oz agent.
    Delete(DeleteScheduleArgs),
}

impl ScheduleSubcommand {
    fn into_runnable_subcommand(self) -> Self {
        match self {
            ScheduleSubcommand::Feedback(args) => ScheduleSubcommand::Create(args.into_create()),
            subcommand => subcommand,
        }
    }
}

#[derive(Debug, Clone, Args)]
#[command(
    group(
        clap::ArgGroup::new("prompt_group")
            .required(true)
            .multiple(true)
            .args(["prompt", "skill"])
    )
)]
pub struct CreateScheduleArgs {
    /// Name of the scheduled agent.
    #[arg(long = "name")]
    pub name: String,

    /// Cron schedule expression (e.g., "0 9 * * 1" for 9 AM every Monday).
    #[arg(long = "cron")]
    pub cron: String,

    #[command(flatten)]
    pub model: ModelArgs,

    #[command(flatten)]
    pub environment: EnvironmentCreateArgs,

    #[command(flatten)]
    pub config_file: ConfigFileArgs,

    #[command(flatten)]
    pub scope: ObjectScope,

    /// MCP servers to configure for this schedule.
    ///
    /// Can be specified as:
    /// - A path to a JSON file containing MCP configuration
    /// - Inline JSON with MCP server configuration
    ///
    /// Can be specified multiple times to include multiple servers.
    #[arg(long = "mcp", value_name = "SPEC")]
    pub mcp_specs: Vec<MCPSpec>,

    /// Prompt for what the scheduled agent should do.
    #[arg(long = "prompt", short = 'p')]
    pub prompt: Option<String>,

    /// Automate a skill to run on a schedule.
    ///
    /// Format: `repo:skill_name` or `org/repo:skill_name`
    ///
    /// Skills are searched in `.agents/skills/`, `.warp/skills/`, `.claude/skills/`, and `.codex/skills/` directories.
    /// The skill is resolved at runtime in the agent's cloud environment.
    ///
    /// When used with --prompt, the skill provides the base context and the prompt is the user task.
    /// This is useful for running recurring workflows like code reviews, dependency updates, or reports.
    #[arg(long = "skill", value_name = "SPEC")]
    pub skill: Option<SkillSpec>,

    /// Where this job should be hosted.
    ///
    /// Setting "warp" (or omitting this flag) runs it on Warp's infrastructure.
    /// Any other value is treated as a self-hosted job and the value will be matched
    /// with the self-hosted worker's name.
    #[arg(long = "host", value_name = "WORKER_ID")]
    pub worker_host: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct FeedbackScheduleArgs {
    /// Cron schedule expression for checking feedback.
    #[arg(long = "cron")]
    pub cron: String,

    /// OpenCoven Feedback host or base URL, for example feedback.opencoven.dev.
    #[arg(long = "feedback-host", value_name = "HOST", value_parser = normalize_feedback_host)]
    pub feedback_host: String,

    /// Name of the scheduled feedback task.
    #[arg(long = "name", default_value = "Feedback task triage")]
    pub name: String,

    /// Product slug to triage.
    #[arg(long = "product", default_value = "cast-codes")]
    pub product: String,

    #[command(flatten)]
    pub model: ModelArgs,

    #[command(flatten)]
    pub environment: EnvironmentCreateArgs,

    #[command(flatten)]
    pub config_file: ConfigFileArgs,

    #[command(flatten)]
    pub scope: ObjectScope,

    /// Additional MCP servers to configure for this feedback task.
    #[arg(long = "mcp", value_name = "SPEC")]
    pub mcp_specs: Vec<MCPSpec>,

    /// Where this job should be hosted.
    #[arg(long = "host", value_name = "WORKER_ID")]
    pub worker_host: Option<String>,
}

impl FeedbackScheduleArgs {
    pub fn into_create(self) -> CreateScheduleArgs {
        let mut mcp_specs = vec![MCPSpec::Json(feedback_mcp_json(&self.feedback_host))];
        mcp_specs.extend(self.mcp_specs);

        CreateScheduleArgs {
            name: self.name,
            cron: self.cron,
            model: self.model,
            environment: self.environment,
            config_file: self.config_file,
            scope: self.scope,
            mcp_specs,
            prompt: Some(feedback_task_prompt(&self.product, &self.feedback_host)),
            skill: None,
            worker_host: self.worker_host,
        }
    }
}

fn normalize_feedback_host(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("feedback host cannot be empty".to_string());
    }

    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    let url = url::Url::parse(&candidate).map_err(|err| format!("invalid feedback host: {err}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("feedback host must use http or https".to_string());
    }
    if url.host_str().is_none() {
        return Err("feedback host must include a host name".to_string());
    }
    if !matches!(url.path(), "" | "/") {
        return Err("feedback host must not include a path".to_string());
    }

    let host = url.host_str().unwrap();
    let authority = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    Ok(format!("{}://{authority}", url.scheme()))
}

fn feedback_mcp_json(base_url: &str) -> String {
    format!(r#"{{"opencoven-feedback":{{"url":"{base_url}/api/mcp"}}}}"#)
}

fn feedback_task_prompt(product: &str, base_url: &str) -> String {
    format!(
        "Review OpenCoven Feedback for product `{product}` at {base_url}. \
         Create follow-up implementation tasks for new, high-signal, actionable feedback. \
         For each task include the public feedback link, concise problem statement, affected area, \
         priority, reproduction evidence when available, and suggested next step. \
         Deduplicate against existing tasks when visible. Do not publish, post comments, close feedback, \
         change integrations, or announce releases without explicit approval."
    )
}

#[derive(Debug, Clone, Args)]
pub struct PauseScheduleArgs {
    /// ID of the schedule to pause.
    pub schedule_id: String,
}

#[derive(Debug, Clone, Args)]
pub struct UnpauseScheduleArgs {
    /// ID of the schedule to unpause.
    pub schedule_id: String,
}

#[derive(Debug, Clone, Args)]
pub struct UpdateScheduleArgs {
    /// ID of the schedule to update.
    pub schedule_id: String,

    /// Update the scheduled agent name.
    #[arg(long = "name")]
    pub name: Option<String>,

    /// Update the cron schedule on which the agent is executed.
    #[arg(long = "cron")]
    pub cron: Option<String>,

    #[command(flatten)]
    pub model: ModelArgs,

    #[command(flatten)]
    pub environment: EnvironmentUpdateArgs,

    #[command(flatten)]
    pub config_file: ConfigFileArgs,

    /// MCP servers to configure for this schedule.
    ///
    /// Can be specified as:
    /// - A path to a JSON file containing MCP configuration
    /// - Inline JSON with MCP server configuration
    ///
    /// Can be specified multiple times to include multiple servers.
    #[arg(long = "mcp", value_name = "SPEC")]
    pub mcp_specs: Vec<MCPSpec>,

    /// Remove MCP servers from this schedule by server name.
    ///
    /// This removes the server entry whose key matches `SERVER_NAME`.
    #[arg(long = "remove-mcp", value_name = "SERVER_NAME")]
    pub remove_mcp: Vec<String>,

    /// Update the scheduled agent's prompt.
    #[arg(long = "prompt", short = 'p')]
    pub prompt: Option<String>,

    /// Update the skill used as the base prompt for the scheduled agent.
    ///
    /// Format: `skill_name`, `repo:skill_name`, or `org/repo:skill_name`
    ///
    /// Skills are searched in `.agents/skills/`, `.warp/skills/`, `.claude/skills/`, and `.codex/skills/` directories.
    /// The skill is resolved at runtime in the agent's cloud environment.
    #[arg(long = "skill", value_name = "SPEC", conflicts_with = "remove_skill")]
    pub skill: Option<SkillSpec>,

    /// Remove the skill from this scheduled agent.
    #[arg(long = "remove-skill", conflicts_with = "skill")]
    pub remove_skill: bool,

    /// Where this job should be hosted.
    ///
    /// Setting "warp" runs it on Warp's infrastructure.
    /// Any other value is treated as a self-hosted job and the value will be matched
    /// with the self-hosted worker's name.
    #[arg(long = "host", value_name = "WORKER_ID")]
    pub worker_host: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct DeleteScheduleArgs {
    /// ID of the schedule to delete.
    pub schedule_id: String,
}

#[derive(Debug, Clone, Args)]
pub struct GetScheduleArgs {
    /// ID of the schedule to get.
    pub schedule_id: String,
}
