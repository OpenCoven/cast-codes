const AUTH_VIEW_BODY_SOURCE: &str = include_str!("auth/auth_view_body.rs");
const AUTH_MOD_SOURCE: &str = include_str!("auth/mod.rs");
const AUTH_VIEW_SHARED_HELPERS_SOURCE: &str = include_str!("auth/auth_view_shared_helpers.rs");
const AI_AGENT_SDK_AMBIENT_SOURCE: &str = include_str!("ai/agent_sdk/ambient.rs");
const AI_AGENT_SDK_DRIVER_SOURCE: &str = include_str!("ai/agent_sdk/driver.rs");
const AI_AGENT_TIPS_SOURCE: &str = include_str!("ai/agent_tips.rs");
const AI_ASSISTANT_PANEL_SOURCE: &str = include_str!("ai_assistant/panel.rs");
const AI_BLOCK_COMMON_SOURCE: &str = include_str!("ai/blocklist/block/view_impl/common.rs");
const AI_BLOCK_SOURCE: &str = include_str!("ai/blocklist/block.rs");
const AI_BLOCK_STATUS_BAR_SOURCE: &str = include_str!("ai/blocklist/block/status_bar.rs");
const AI_FACT_RULE_SOURCE: &str = include_str!("ai/facts/view/rule.rs");
const AI_TELEMETRY_BANNER_SOURCE: &str = include_str!("ai/blocklist/telemetry_banner.rs");
const AGENT_VIEW_ZERO_STATE_BLOCK_SOURCE: &str =
    include_str!("ai/blocklist/agent_view/zero_state_block.rs");
const CLI_BLOCK_SOURCE: &str = include_str!("ai/blocklist/block/cli.rs");
const CLI_AGENT_EVENT_SOURCE: &str = include_str!("terminal/cli_agent_sessions/event/mod.rs");
const CLI_AGENT_PLUGIN_MANAGER_SOURCE: &str =
    include_str!("terminal/cli_agent_sessions/plugin_manager/mod.rs");
const CODE_REVIEW_VIEW_SOURCE: &str = include_str!("code_review/code_review_view.rs");
const COMMAND_SEARCH_WARP_AI_SOURCE: &str = include_str!("search/command_search/warp_ai.rs");
const COMMAND_SEARCH_SEARCHER_SOURCE: &str = include_str!("search/command_search/searcher.rs");
const DRIVE_IMPORT_MODAL_BODY_SOURCE: &str = include_str!("drive/import/modal_body.rs");
const DRIVE_INDEX_SOURCE: &str = include_str!("drive/index.rs");
const DRIVE_MOD_SOURCE: &str = include_str!("drive/mod.rs");
const DRIVE_SETTINGS_SOURCE: &str = include_str!("drive/settings.rs");
const DRIVE_WORKFLOWS_MODAL_SOURCE: &str = include_str!("drive/workflows/modal.rs");
const LINUX_AUTOUPDATE_SOURCE: &str = include_str!("autoupdate/linux.rs");
const LAUNCH_CONFIG_SAVE_MODAL_SOURCE: &str = include_str!("launch_configs/save_modal.rs");
const LOGIN_FAILURE_NOTIFICATION_SOURCE: &str = include_str!("auth/login_failure_notification.rs");
const LOGIN_SLIDE_SOURCE: &str = include_str!("auth/login_slide.rs");
const NOTEBOOK_EDITOR_VIEW_SOURCE: &str = include_str!("notebooks/editor/view.rs");
const PANE_GROUP_SOURCE: &str = include_str!("pane_group/mod.rs");
const PROMPT_EDITOR_MODAL_SOURCE: &str = include_str!("prompt/editor_modal.rs");
const PROMPT_ALERT_SOURCE: &str = include_str!("ai/blocklist/prompt/prompt_alert.rs");
const REWARD_VIEW_SOURCE: &str = include_str!("reward_view.rs");
const RESOURCE_CENTER_CHANGELOG_SOURCE: &str =
    include_str!("resource_center/section_views/changelog_section.rs");
const RESOURCE_CENTER_KEYBINDINGS_SOURCE: &str =
    include_str!("resource_center/keybindings_page.rs");
const RESOURCE_CENTER_UTILS_SOURCE: &str = include_str!("resource_center/utils.rs");
const SLASH_STATIC_COMMANDS_SOURCE: &str =
    include_str!("search/slash_command_menu/static_commands/commands.rs");
const TAB_CONFIG_SESSION_CONFIG_MODAL_SOURCE: &str =
    include_str!("tab_configs/session_config_modal.rs");
const TERMINAL_DOCKER_SANDBOX_SOURCE: &str = include_str!("terminal/view/docker_sandbox/mod.rs");
const TERMINAL_SECRETS_SOURCE: &str = include_str!("terminal/model/secrets.rs");
const USER_CONFIG_SOURCE: &str = include_str!("user_config/mod.rs");
const WASM_NUX_DIALOG_SOURCE: &str = include_str!("wasm_nux_dialog.rs");
const WORKFLOW_ALIASES_SOURCE: &str = include_str!("workflows/aliases.rs");
const WORKFLOW_CATEGORIES_SOURCE: &str = include_str!("workflows/categories.rs");
const WORKFLOW_VIEW_SOURCE: &str = include_str!("workflows/workflow_view.rs");
const WORKSPACE_MOD_SOURCE: &str = include_str!("workspace/mod.rs");
const WORKSPACE_VIEW_SOURCE: &str = include_str!("workspace/view.rs");

#[test]
fn public_app_surfaces_use_castcodes_links_and_labels() {
    assert!(LOGIN_SLIDE_SOURCE.contains("PRIVACY_POLICY_URL"));
    assert!(AUTH_MOD_SOURCE.contains("unsynced Cast Drive"));
    assert!(AUTH_VIEW_BODY_SOURCE.contains("PRIVACY_POLICY_URL"));
    assert!(AUTH_VIEW_SHARED_HELPERS_SOURCE.contains("PRIVACY_POLICY_URL"));
    assert!(LOGIN_FAILURE_NOTIFICATION_SOURCE
        .contains("LOGIN_TROUBLESHOOTING_DOCS_URL: &str = USER_DOCS_URL"));
    assert!(LAUNCH_CONFIG_SAVE_MODAL_SOURCE.contains("Some(USER_DOCS_URL.to_string())"));
    assert!(WASM_NUX_DIALOG_SOURCE.contains("ctx.open_url(USER_DOCS_URL)"));
    assert!(WORKSPACE_VIEW_SOURCE.contains("ctx.open_url(links::USER_DOCS_URL)"));
    assert!(WORKSPACE_VIEW_SOURCE.contains("links::USER_DOCS_URL.to_string()"));
    assert!(NOTEBOOK_EDITOR_VIEW_SOURCE.contains("\"Open in CastCodes\""));
    assert!(WORKFLOW_VIEW_SOURCE.contains("Run in CastCodes"));
    assert!(WORKFLOW_VIEW_SOURCE.contains("Familiar"));
    assert!(WORKFLOW_CATEGORIES_SOURCE.contains("crate::util::links::USER_DOCS_URL"));
    assert!(REWARD_VIEW_SOURCE.contains("exclusive CastCodes theme"));
    assert!(AI_AGENT_TIPS_SOURCE.contains("USER_DOCS_URL"));
    assert!(AI_AGENT_TIPS_SOURCE.contains("AGENTS.md"));
    assert!(AI_AGENT_TIPS_SOURCE.contains("Enable shell integration"));
    assert!(DRIVE_IMPORT_MODAL_BODY_SOURCE.contains("FILE_TYPE_DOCS_URL: &str = USER_DOCS_URL"));
    assert!(DRIVE_INDEX_SOURCE.contains("Syncing Cast Drive"));
    assert!(DRIVE_INDEX_SOURCE.contains("CastCodes issue"));
    assert!(DRIVE_INDEX_SOURCE.contains("teammates already on CastCodes"));
    assert!(DRIVE_WORKFLOWS_MODAL_SOURCE.contains("with your Familiar"));
    assert!(DRIVE_SETTINGS_SOURCE.contains("Cast Drive"));
    assert!(DRIVE_MOD_SOURCE.contains("Sort order for Cast Drive items."));
    assert!(LINUX_AUTOUPDATE_SOURCE.contains("Channel::Oss => warp_core::brand::PRODUCT_SLUG"));
    assert!(LINUX_AUTOUPDATE_SOURCE.contains("return warp_core::brand::ORG_ID.to_string();"));
    assert!(LINUX_AUTOUPDATE_SOURCE.contains("Detected that CastCodes was installed"));
    assert!(LINUX_AUTOUPDATE_SOURCE.contains("Relaunching CastCodes for update"));
    assert!(LINUX_AUTOUPDATE_SOURCE.contains("fn pacman_repo_config_command"));
    assert!(LINUX_AUTOUPDATE_SOURCE.contains("fn pacman_signing_key_command"));
    assert!(COMMAND_SEARCH_WARP_AI_SOURCE.contains("Familiar"));
    assert!(COMMAND_SEARCH_SEARCHER_SOURCE.contains("using the Familiar"));
    assert!(AI_BLOCK_SOURCE.contains("GITHUB_ISSUES_URL"));
    assert!(CLI_BLOCK_SOURCE.contains("GITHUB_ISSUES_URL"));
    assert!(AI_TELEMETRY_BANNER_SOURCE.contains("PRIVACY_POLICY_URL"));
    assert!(PROMPT_ALERT_SOURCE.contains("GITHUB_ISSUES_URL"));
    assert!(AGENT_VIEW_ZERO_STATE_BLOCK_SOURCE.contains("USER_DOCS_URL"));
    assert!(AI_FACT_RULE_SOURCE.contains("AGENTS.md"));
    assert!(AI_AGENT_SDK_DRIVER_SOURCE.contains("USER_DOCS_URL"));
    assert!(AI_AGENT_SDK_AMBIENT_SOURCE.contains("USER_DOCS_URL"));
    assert!(AI_ASSISTANT_PANEL_SOURCE.contains("const COVEN_CODE_HARNESS: &str = \"coven-code\""));
    assert!(AI_ASSISTANT_PANEL_SOURCE.contains("Run native Coven Code operation"));
    assert!(AI_ASSISTANT_PANEL_SOURCE.contains("fn issue_primary_request"));
    assert!(AI_ASSISTANT_PANEL_SOURCE.contains("send_via_coven_gateway_with_prompt"));
    assert!(AI_BLOCK_COMMON_SOURCE.contains("Internal CastCodes error."));
    assert!(AI_BLOCK_COMMON_SOURCE.contains("Working..."));
    assert!(AI_BLOCK_STATUS_BAR_SOURCE.contains("Working with {name}."));
    assert!(AI_BLOCK_STATUS_BAR_SOURCE.contains("Working with another model."));
    assert!(CODE_REVIEW_VIEW_SOURCE.contains("codebase indexing and AGENTS.md"));
    assert!(PANE_GROUP_SOURCE.contains("crate::util::links::USER_DOCS_URL"));
    assert!(PROMPT_EDITOR_MODAL_SOURCE.contains("CastCodes terminal prompt"));
    assert!(RESOURCE_CENTER_CHANGELOG_SOURCE.contains("USER_DOCS_URL"));
    assert!(RESOURCE_CENTER_KEYBINDINGS_SOURCE.contains("CastCodes keyboard shortcut"));
    assert!(RESOURCE_CENTER_UTILS_SOURCE.contains("CastCodes keyboard shortcut"));
    assert!(USER_CONFIG_SOURCE.contains("# CastCodes Launch Configuration"));
    assert!(USER_CONFIG_SOURCE.contains("github.com/OpenCoven/cast-codes"));
    assert!(WORKFLOW_ALIASES_SOURCE.contains("Cast Drive workflow"));
    assert!(SLASH_STATIC_COMMANDS_SOURCE.contains("CastCodes' built-in editor"));
    assert!(SLASH_STATIC_COMMANDS_SOURCE.contains("CastCodes' code editor"));
    assert!(TAB_CONFIG_SESSION_CONFIG_MODAL_SOURCE.contains("Start session"));
    assert!(TERMINAL_DOCKER_SANDBOX_SOURCE.contains("Cast Drive to sync for docker sandbox"));
    assert!(TERMINAL_SECRETS_SOURCE.contains("CastCodes API Key"));
    assert!(CLI_AGENT_EVENT_SOURCE.contains("CastCodes may need to be updated"));
    assert!(CLI_AGENT_PLUGIN_MANAGER_SOURCE.contains("ChannelState::cloud_services_available()"));
    assert!(CLI_AGENT_PLUGIN_MANAGER_SOURCE
        .contains("CLIAgent::Claude if ChannelState::cloud_services_available()"));
    assert!(CLI_AGENT_PLUGIN_MANAGER_SOURCE
        .contains("CLIAgent::OpenCode\n            if ChannelState::cloud_services_available()"));
    assert!(WORKSPACE_MOD_SOURCE.contains("Open Settings: Shell integration"));
    assert!(WORKSPACE_MOD_SOURCE.contains("Configure shell integration..."));
    assert!(WORKSPACE_MOD_SOURCE.contains("Open CastCodes Launch Modal"));
    assert!(WORKSPACE_MOD_SOURCE.contains("Reset CastCodes Launch Modal State"));

    let forbidden = [
        "https://www.warp.dev/terms-of-service",
        "https://warp.dev/privacy",
        "https://docs.warp.dev/support-and-community/troubleshooting-and-support/troubleshooting-login-issues",
        "https://docs.warp.dev/terminal/sessions/launch-configurations",
        "https://app.warp.dev/get_warp",
        "https://www.warp.dev",
        "https://docs.warp.dev/reference/cli",
        "https://docs.warp.dev/support-and-community/troubleshooting-and-support/updating-warp",
        "https://docs.warp.dev/getting-started/supported-shells",
        "https://docs.warp.dev/getting-started/keyboard-shortcuts",
        "https://docs.warp.dev/changelog",
        "https://warp.dev/download",
        "https://docs.warp.dev/terminal/more-features/linux#native-wayland",
        "Open in Warp",
        "Run in Warp",
        "Generate a title, descriptions, or parameters with Warp AI",
        "https://docs.warp.dev/knowledge-and-collaboration/warp-drive/workflows",
        "exclusive Warp theme",
        "Warpify a remote",
        "Once you generate a WARP.md rules file",
        "Syncing Warp Drive",
        "Ask Warp AI",
        "using Warp AI",
        "Internal Warp error.",
        "Warping...",
        "Warping with {name}.",
        "Warping with another model.",
        "A shortcut alias for a Warp Drive workflow.",
        "Open a skill's markdown file in Warp's built-in editor",
        "Open a file in Warp's code editor",
        "Timed out waiting for Warp Drive to sync for docker sandbox",
        "teammates already on Warp.",
        "Warp may need to be updated.",
        "unsynced Warp Drive",
        "# Warp Launch Configuration",
        "Enables codebase indexing and WARP.md",
        "Get Warping",
        "Warp terminal prompt",
        "Warp API Key",
        "Open Settings: Warpify",
        "Configure Warpify...",
        "Open OpenWarp Launch Modal",
        "Reset OpenWarp Launch Modal State",
        "Channel::Oss => \"warp-oss\"",
        "Detected that Warp was installed",
        "Relaunching warp for update",
    ];
    let sources = [
        AUTH_MOD_SOURCE,
        AUTH_VIEW_BODY_SOURCE,
        AUTH_VIEW_SHARED_HELPERS_SOURCE,
        AI_AGENT_SDK_AMBIENT_SOURCE,
        AI_AGENT_SDK_DRIVER_SOURCE,
        AI_ASSISTANT_PANEL_SOURCE,
        AI_AGENT_TIPS_SOURCE,
        AI_BLOCK_COMMON_SOURCE,
        AI_BLOCK_SOURCE,
        AI_BLOCK_STATUS_BAR_SOURCE,
        AI_FACT_RULE_SOURCE,
        AI_TELEMETRY_BANNER_SOURCE,
        AGENT_VIEW_ZERO_STATE_BLOCK_SOURCE,
        CLI_BLOCK_SOURCE,
        CLI_AGENT_EVENT_SOURCE,
        CODE_REVIEW_VIEW_SOURCE,
        COMMAND_SEARCH_SEARCHER_SOURCE,
        COMMAND_SEARCH_WARP_AI_SOURCE,
        DRIVE_IMPORT_MODAL_BODY_SOURCE,
        DRIVE_INDEX_SOURCE,
        DRIVE_MOD_SOURCE,
        DRIVE_SETTINGS_SOURCE,
        DRIVE_WORKFLOWS_MODAL_SOURCE,
        LINUX_AUTOUPDATE_SOURCE,
        LAUNCH_CONFIG_SAVE_MODAL_SOURCE,
        LOGIN_FAILURE_NOTIFICATION_SOURCE,
        LOGIN_SLIDE_SOURCE,
        NOTEBOOK_EDITOR_VIEW_SOURCE,
        PANE_GROUP_SOURCE,
        PROMPT_EDITOR_MODAL_SOURCE,
        PROMPT_ALERT_SOURCE,
        REWARD_VIEW_SOURCE,
        RESOURCE_CENTER_CHANGELOG_SOURCE,
        RESOURCE_CENTER_KEYBINDINGS_SOURCE,
        RESOURCE_CENTER_UTILS_SOURCE,
        SLASH_STATIC_COMMANDS_SOURCE,
        TAB_CONFIG_SESSION_CONFIG_MODAL_SOURCE,
        TERMINAL_DOCKER_SANDBOX_SOURCE,
        TERMINAL_SECRETS_SOURCE,
        USER_CONFIG_SOURCE,
        WASM_NUX_DIALOG_SOURCE,
        WORKFLOW_ALIASES_SOURCE,
        WORKFLOW_CATEGORIES_SOURCE,
        WORKFLOW_VIEW_SOURCE,
        WORKSPACE_MOD_SOURCE,
        WORKSPACE_VIEW_SOURCE,
    ];

    for source in sources {
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "public app surface should not contain {needle}"
            );
        }
    }
}
