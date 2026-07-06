use super::*;
use settings_page::MatchData;

const ABOUT_PAGE_SOURCE: &str = include_str!("about_page.rs");
const AI_DOCUMENT_MODEL_SOURCE: &str = include_str!("../ai/document/ai_document_model.rs");
const AI_DOCUMENT_VIEW_SOURCE: &str = include_str!("../ai/ai_document_view.rs");
const AI_PAGE_SOURCE: &str = include_str!("ai_page.rs");
const AI_SETTINGS_SOURCE: &str = include_str!("../settings/ai.rs");
const APPEARANCE_PAGE_SOURCE: &str = include_str!("appearance_page.rs");
const APP_SERVICES_WINDOWS_SOURCE: &str = include_str!("../app_services/windows/mod.rs");
const APP_SERVICES_WINDOWS_SINGLE_INSTANCE_SOURCE: &str =
    include_str!("../app_services/windows/single_instance_manager.rs");
const APP_SERVICES_LINUX_SOURCE: &str = include_str!("../app_services/linux/mod.rs");
const AUTOUPDATE_CHANNEL_VERSIONS_SOURCE: &str = include_str!("../autoupdate/channel_versions.rs");
const CODE_PAGE_SOURCE: &str = include_str!("code_page.rs");
const DEBUG_DUMP_SOURCE: &str = include_str!("../debug_dump.rs");
const DEFAULT_TERMINAL_SOURCE: &str = include_str!("../default_terminal/mod.rs");
const ENVIRONMENTS_PAGE_SOURCE: &str = include_str!("environments_page.rs");
const EXTERNAL_EDITOR_SOURCE: &str = include_str!("features/external_editor.rs");
const FEATURES_PAGE_SOURCE: &str = include_str!("features_page.rs");
const GENERAL_SETTINGS_SOURCE: &str = include_str!("../terminal/general_settings.rs");
const LOCAL_WORKFLOWS_SOURCE: &str = include_str!("../workflows/local_workflows.rs");
const MAIN_PAGE_SOURCE: &str = include_str!("main_page.rs");
const MCP_SERVERS_LIST_PAGE_SOURCE: &str = include_str!("mcp_servers/list_page.rs");
const PANE_GROUP_SOURCE: &str = include_str!("../pane_group/mod.rs");
const PLATFORM_PAGE_SOURCE: &str = include_str!("platform_page.rs");
const PRIVACY_PAGE_SOURCE: &str = include_str!("privacy_page.rs");
const PRIVACY_SETTINGS_SOURCE: &str = include_str!("../settings/privacy.rs");
const REFERRALS_PAGE_SOURCE: &str = include_str!("referrals_page.rs");
const SHOW_BLOCKS_VIEW_SOURCE: &str = include_str!("show_blocks_view.rs");
const SETTINGS_VIEW_SOURCE: &str = include_str!("mod.rs");
const SESSION_SETTINGS_SOURCE: &str = include_str!("../terminal/session_settings.rs");
const SERVER_API_SOURCE: &str = include_str!("../server/server_api.rs");
const SSH_ERROR_SOURCE: &str = include_str!("../terminal/ssh/error.rs");
const SSH_INSTALL_TMUX_SOURCE: &str = include_str!("../terminal/ssh/install_tmux.rs");
const SSH_WARPIFY_SOURCE: &str = include_str!("../terminal/ssh/warpify.rs");
const TERMINAL_VIEW_SOURCE: &str = include_str!("../terminal/view.rs");
const TERMINAL_INIT_PROJECT_SOURCE: &str = include_str!("../terminal/view/init_project/mod.rs");
const TERMINAL_AMBIENT_AGENT_HARNESS_SELECTOR_SOURCE: &str =
    include_str!("../terminal/view/ambient_agent/harness_selector.rs");
const TERMINAL_AMBIENT_AGENT_HOST_SELECTOR_SOURCE: &str =
    include_str!("../terminal/view/ambient_agent/host_selector.rs");
const TERMINAL_INLINE_AGENT_MODE_SETUP_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/agent_mode_setup.rs");
const TERMINAL_INLINE_ALIAS_EXPANSION_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/alias_expansion.rs");
const TERMINAL_INLINE_AWS_BEDROCK_LOGIN_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/aws_bedrock_login.rs");
const TERMINAL_INLINE_NOTIFICATIONS_DISCOVERY_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/notifications_discovery.rs");
const TERMINAL_INLINE_OPEN_IN_WARP_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/open_in_warp.rs");
const TERMINAL_INLINE_SHELL_PROCESS_TERMINATED_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/shell_process_terminated.rs");
const TERMINAL_INLINE_SSH_SOURCE: &str = include_str!("../terminal/view/inline_banner/ssh.rs");
const TERMINAL_INLINE_VIM_MODE_SOURCE: &str =
    include_str!("../terminal/view/inline_banner/vim_mode.rs");
const TERMINAL_ONBOARDING_AGENTIC_SUGGESTIONS_SOURCE: &str =
    include_str!("../terminal/view/block_onboarding/onboarding_agentic_suggestions_block.rs");
const TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE: &str =
    include_str!("../terminal/view/block_onboarding/onboarding_prompt_block.rs");
const TERMINAL_OPEN_IN_WARP_SOURCE: &str = include_str!("../terminal/view/open_in_warp.rs");
const TERMINAL_PROMPT_RENDER_HELPER_SOURCE: &str =
    include_str!("../terminal/prompt_render_helper.rs");
const TERMINAL_PROFILE_MODEL_SELECTOR_SOURCE: &str =
    include_str!("../terminal/profile_model_selector.rs");
const TERMINAL_SHELL_TERMINATED_BANNER_SOURCE: &str =
    include_str!("../terminal/view/shell_terminated_banner.rs");
const TERMINAL_SSH_REMOTE_SERVER_CHOICE_SOURCE: &str =
    include_str!("../terminal/view/ssh_remote_server_choice_view.rs");
const TERMINAL_SSH_REMOTE_SERVER_FAILED_BANNER_SOURCE: &str =
    include_str!("../terminal/view/ssh_remote_server_failed_banner.rs");
const TERMINAL_TOOLTIPS_SOURCE: &str = include_str!("../terminal/view/tooltips.rs");
const TERMINAL_VIEW_INIT_SOURCE: &str = include_str!("../terminal/view/init.rs");
const TERMINAL_USE_AGENT_FOOTER_SOURCE: &str =
    include_str!("../terminal/view/use_agent_footer/mod.rs");
const TERMINAL_WARPIFY_FOOTER_SOURCE: &str =
    include_str!("../terminal/view/use_agent_footer/warpify_footer.rs");
const TERMINAL_MODEL_SPEC_SCORES_SOURCE: &str =
    include_str!("../terminal/input/models/model_spec_scores.rs");
const TERMINAL_WARPIFY_BLOCK_BANNER_SOURCE: &str =
    include_str!("../terminal/view/block_banner/warpify.rs");
const TERMINAL_WARPIFY_RENDER_SOURCE: &str = include_str!("../terminal/warpify/render.rs");
const TERMINAL_WARPIFY_SETTINGS_SOURCE: &str = include_str!("../terminal/warpify/settings.rs");
const TERMINAL_WARPIFY_SUCCESS_BLOCK_SOURCE: &str =
    include_str!("../terminal/warpify/success_block.rs");
const THEME_PICKER_SLIDE_SOURCE: &str =
    include_str!("../../../crates/onboarding/src/slides/theme_picker_slide.rs");
const WARP_DRIVE_PAGE_SOURCE: &str = include_str!("warp_drive_page.rs");
const WARPIFY_PAGE_SOURCE: &str = include_str!("warpify_page.rs");
const WORKSPACE_ONE_TIME_MODAL_MODEL_SOURCE: &str =
    include_str!("../workspace/one_time_modal_model.rs");
const WORKSPACE_MOD_SOURCE: &str = include_str!("../workspace/mod.rs");
const WORKSPACE_VIEW_SOURCE: &str = include_str!("../workspace/view.rs");

// ── SettingsSection classification ──────────────────────────────────────────

#[test]
fn ai_subpages_are_identified() {
    assert!(SettingsSection::WarpAgent.is_ai_subpage());
    assert!(SettingsSection::AgentProfiles.is_ai_subpage());
    assert!(SettingsSection::AgentMCPServers.is_ai_subpage());
    assert!(SettingsSection::Knowledge.is_ai_subpage());
    assert!(SettingsSection::ThirdPartyCLIAgents.is_ai_subpage());

    assert!(!SettingsSection::AI.is_ai_subpage());
    assert!(!SettingsSection::Account.is_ai_subpage());
    assert!(!SettingsSection::CodeIndexing.is_ai_subpage());
}

#[test]
fn code_subpages_are_identified() {
    assert!(SettingsSection::CodeIndexing.is_code_subpage());
    assert!(SettingsSection::EditorAndCodeReview.is_code_subpage());

    assert!(!SettingsSection::Code.is_code_subpage());
    assert!(!SettingsSection::WarpAgent.is_code_subpage());
}

#[test]
fn cloud_platform_subpages_are_identified() {
    assert!(SettingsSection::CloudEnvironments.is_cloud_platform_subpage());
    assert!(SettingsSection::OzCloudAPIKeys.is_cloud_platform_subpage());

    assert!(!SettingsSection::Account.is_cloud_platform_subpage());
    assert!(!SettingsSection::WarpAgent.is_cloud_platform_subpage());
}

#[test]
fn is_subpage_covers_all_umbrella_types() {
    // All subpages under any umbrella should return true.
    for section in SettingsSection::ai_subpages() {
        assert!(section.is_subpage(), "{section:?} should be a subpage");
    }
    assert!(SettingsSection::CodeIndexing.is_subpage());
    assert!(SettingsSection::EditorAndCodeReview.is_subpage());
    assert!(SettingsSection::CloudEnvironments.is_subpage());
    assert!(SettingsSection::OzCloudAPIKeys.is_subpage());

    // Top-level pages should not be subpages.
    assert!(!SettingsSection::Account.is_subpage());
    assert!(!SettingsSection::AI.is_subpage());
    assert!(!SettingsSection::Code.is_subpage());
    assert!(!SettingsSection::Privacy.is_subpage());
}

// ── parent_page_section mapping ─────────────────────────────────────────────

#[test]
fn ai_subpages_map_to_ai_backing_page() {
    assert_eq!(
        SettingsSection::WarpAgent.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::AgentProfiles.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::Knowledge.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::ThirdPartyCLIAgents.parent_page_section(),
        SettingsSection::AI
    );
}

#[test]
fn agent_mcp_servers_maps_to_mcp_servers_page() {
    // AgentMCPServers renders the standalone MCPServers page, not the AI page.
    assert_eq!(
        SettingsSection::AgentMCPServers.parent_page_section(),
        SettingsSection::MCPServers
    );
}

#[test]
fn code_subpages_map_to_code_backing_page() {
    assert_eq!(
        SettingsSection::CodeIndexing.parent_page_section(),
        SettingsSection::Code
    );
    assert_eq!(
        SettingsSection::EditorAndCodeReview.parent_page_section(),
        SettingsSection::Code
    );
}

#[test]
fn cloud_platform_subpages_map_to_their_backing_pages() {
    assert_eq!(
        SettingsSection::CloudEnvironments.parent_page_section(),
        SettingsSection::CloudEnvironments
    );
    assert_eq!(
        SettingsSection::OzCloudAPIKeys.parent_page_section(),
        SettingsSection::OzCloudAPIKeys
    );
}

#[test]
fn non_subpage_sections_map_to_themselves() {
    assert_eq!(
        SettingsSection::Account.parent_page_section(),
        SettingsSection::Account
    );
    assert_eq!(
        SettingsSection::AI.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::Privacy.parent_page_section(),
        SettingsSection::Privacy
    );
}

// ── ai_subpages list ────────────────────────────────────────────────────────

#[test]
fn ai_subpages_list_contains_all_ai_subpage_variants() {
    let subpages = SettingsSection::ai_subpages();
    assert!(subpages.contains(&SettingsSection::WarpAgent));
    assert!(subpages.contains(&SettingsSection::AgentProfiles));
    assert!(subpages.contains(&SettingsSection::AgentMCPServers));
    assert!(subpages.contains(&SettingsSection::Knowledge));
    assert!(subpages.contains(&SettingsSection::ThirdPartyCLIAgents));
}

#[test]
fn ai_subpages_list_does_not_contain_non_subpages() {
    let subpages = SettingsSection::ai_subpages();
    assert!(!subpages.contains(&SettingsSection::AI));
    assert!(!subpages.contains(&SettingsSection::Account));
    assert!(!subpages.contains(&SettingsSection::Code));
}

// ── MatchData behavior ──────────────────────────────────────────────────────

#[test]
fn match_data_uncounted_true_is_truthy() {
    assert!(MatchData::Uncounted(true).is_truthy());
}

#[test]
fn match_data_uncounted_false_is_not_truthy() {
    assert!(!MatchData::Uncounted(false).is_truthy());
}

#[test]
fn match_data_countable_nonzero_is_truthy() {
    assert!(MatchData::Countable(3).is_truthy());
    assert!(MatchData::Countable(1).is_truthy());
}

#[test]
fn match_data_countable_zero_is_not_truthy() {
    assert!(!MatchData::Countable(0).is_truthy());
}

// ── Display / FromStr round-trip ────────────────────────────────────────────

#[test]
fn subpage_display_names_are_correct() {
    assert_eq!(SettingsSection::WarpAgent.to_string(), "Cast Agent");
    assert_eq!(SettingsSection::AgentProfiles.to_string(), "Profiles");
    assert_eq!(SettingsSection::AgentMCPServers.to_string(), "MCP servers");
    assert_eq!(SettingsSection::Knowledge.to_string(), "Knowledge");
    assert_eq!(
        SettingsSection::ThirdPartyCLIAgents.to_string(),
        "Third party CLI agents"
    );
    assert_eq!(
        SettingsSection::CodeIndexing.to_string(),
        "Indexing and projects"
    );
    assert_eq!(
        SettingsSection::EditorAndCodeReview.to_string(),
        "Editor and Code Review"
    );
    assert_eq!(
        SettingsSection::CloudEnvironments.to_string(),
        "Environments"
    );
    assert_eq!(
        SettingsSection::OzCloudAPIKeys.to_string(),
        "Agent API Keys"
    );
    assert_eq!(SettingsSection::Warpify.to_string(), "Shell integration");
}

#[test]
fn account_nav_label_matches_channel_capabilities() {
    assert_eq!(
        SettingsSection::Account.nav_label_for_channel(true),
        "Account"
    );
    assert_eq!(
        SettingsSection::Account.nav_label_for_channel(false),
        "About"
    );
    assert_eq!(
        SettingsSection::Appearance.nav_label_for_channel(false),
        "Appearance"
    );
}

#[test]
fn mcp_description_copy_matches_channel_capabilities() {
    let cloud_copy = mcp_servers::list_page::mcp_description_text_for_channel(true);
    assert!(cloud_copy.contains("team servers"));
    assert!(cloud_copy.contains("shared with you"));

    let local_copy = mcp_servers::list_page::mcp_description_text_for_channel(false);
    assert!(!local_copy.contains("team servers"));
    assert!(!local_copy.contains("shared with you"));
    assert!(local_copy.contains("custom server"));
    assert!(local_copy.contains("presets"));
}

#[test]
fn mcp_section_titles_match_channel_capabilities() {
    assert_eq!(
        mcp_servers::list_page::mcp_gallery_section_title_for_channel(true),
        "Shared from CastCodes"
    );
    assert_eq!(
        mcp_servers::list_page::mcp_gallery_section_title_for_channel(false),
        "MCP presets"
    );
    assert_eq!(
        mcp_servers::list_page::mcp_shared_section_title_for_channel(true, Some("Team")),
        "Shared by CastCodes and Team"
    );
    assert_eq!(
        mcp_servers::list_page::mcp_shared_section_title_for_channel(true, None),
        "Shared by CastCodes and from other devices"
    );
    assert_eq!(
        mcp_servers::list_page::mcp_shared_section_title_for_channel(false, Some("Team")),
        "Available MCPs"
    );
    assert_eq!(
        mcp_servers::list_page::mcp_shared_section_title_for_channel(false, None),
        "Available MCPs"
    );
}

#[test]
fn mcp_share_controls_are_cloud_service_only() {
    let expected_gate =
        "show_share_icon_button: ChannelState::cloud_services_available() && is_shareable";

    assert_eq!(
        MCP_SERVERS_LIST_PAGE_SOURCE.matches(expected_gate).count(),
        2,
        "template and installation MCP cards should hide share controls when cloud services are unavailable"
    );
}

#[test]
fn ai_usage_widget_is_cloud_service_only() {
    assert!(AI_PAGE_SOURCE
        .contains("let cloud_services_available = ChannelState::cloud_services_available();"));
    assert!(AI_PAGE_SOURCE.contains(
        "if cloud_services_available && !FeatureFlag::UsageBasedPricing.is_enabled() {\n                    widgets.push(Box::new(UsageWidget::default()));\n                }"
    ));
    assert!(!AI_PAGE_SOURCE.contains(
        "if !FeatureFlag::UsageBasedPricing.is_enabled() {\n                    widgets.push(Box::new(UsageWidget::default()));\n                }"
    ));
}

#[test]
fn cloud_agent_computer_use_widget_is_cloud_service_only() {
    assert!(AI_PAGE_SOURCE.contains(
        "if cloud_services_available && FeatureFlag::AgentModeComputerUse.is_enabled() {\n                    widgets.push(Box::new(CloudAgentComputerUseWidget::default()));\n                }"
    ));
    assert!(!AI_PAGE_SOURCE.contains(
        "if FeatureFlag::AgentModeComputerUse.is_enabled() {\n                    widgets.push(Box::new(CloudAgentComputerUseWidget::default()));\n                }"
    ));
}

#[test]
fn api_key_inputs_are_available_in_local_only_builds() {
    assert!(ai_page::api_keys_enabled_for_channel(true, true));
    assert!(!ai_page::api_keys_enabled_for_channel(false, true));
    assert!(ai_page::api_keys_enabled_for_channel(true, false));
    assert!(ai_page::api_keys_enabled_for_channel(false, false));

    assert_eq!(
        AI_PAGE_SOURCE
            .matches("api_keys_enabled_for_channel(is_byo_enabled, ChannelState::cloud_services_available())")
            .count()
            + AI_PAGE_SOURCE
                .matches("let is_byo_enabled = api_keys_enabled_for_channel(")
                .count(),
        3,
        "API key setup, workspace refresh, and render paths should use channel-aware BYOK availability"
    );
}

#[test]
fn api_key_upgrade_cta_is_cloud_service_only() {
    assert!(
        AI_PAGE_SOURCE.contains("if !is_byo_enabled && ChannelState::cloud_services_available() {")
    );
}

#[test]
fn api_key_hosted_credit_fallback_copy_is_clear() {
    assert!(AI_PAGE_SOURCE.contains("\"Hosted credit fallback\""));
    assert!(!AI_PAGE_SOURCE.contains("\"Cast credit fallback\""));
}

#[test]
fn castcodes_about_page_does_not_render_warp_logo_assets() {
    assert!(ABOUT_PAGE_SOURCE.contains("\"about castcodes version\""));
    assert!(!ABOUT_PAGE_SOURCE.contains("warp-logo-with-light-title.svg"));
    assert!(!ABOUT_PAGE_SOURCE.contains("warp-logo-with-dark-title.svg"));
}

#[test]
fn cloud_only_settings_pages_are_gated_by_channel_services() {
    assert!(ENVIRONMENTS_PAGE_SOURCE.contains("ChannelState::cloud_services_available()"));
    assert!(PLATFORM_PAGE_SOURCE.contains("ChannelState::cloud_services_available()"));
    assert!(PRIVACY_PAGE_SOURCE.contains("ChannelState::cloud_services_available()"));
    assert!(SHOW_BLOCKS_VIEW_SOURCE.contains("ChannelState::cloud_services_available()"));
    assert!(WARP_DRIVE_PAGE_SOURCE.contains("ChannelState::cloud_services_available()"));
    assert!(SETTINGS_VIEW_SOURCE.contains(
        "if cloud_services_available {\n            settings_pages.push(SettingsPage::new(platform_page_handle));\n            settings_pages.push(SettingsPage::new(referrals_page_handle));\n            settings_pages.push(SettingsPage::new(warp_drive_page_handle));\n            settings_pages.push(SettingsPage::new(show_blocks_view_handle));\n            settings_pages.push(SettingsPage::new(environments_page_handle.clone()));\n        }"
    ));
    assert!(SETTINGS_VIEW_SOURCE
        .contains("settings_pages.push(SettingsPage::new(warp_drive_page_handle))"));
    assert!(SETTINGS_VIEW_SOURCE.contains(
        "if cloud_services_available {\n            nav_items.insert(\n                3,"
    ));
    assert!(SETTINGS_VIEW_SOURCE
        .contains("nav_items.insert(9, SettingsNavItem::Page(SettingsSection::SharedBlocks))"));
    assert!(SETTINGS_VIEW_SOURCE
        .contains("nav_items.insert(10, SettingsNavItem::Page(SettingsSection::WarpDrive))"));
}

fn editable_binding_block<'a>(source: &'a str, binding_id: &str) -> &'a str {
    let quoted_binding_id = format!("\"{binding_id}\"");
    let marker = source
        .match_indices(&quoted_binding_id)
        .map(|(index, _)| index)
        .chain(source.match_indices(binding_id).map(|(index, _)| index))
        .find(|marker| source[..*marker].rfind("EditableBinding::new(").is_some())
        .unwrap_or_else(|| panic!("missing binding {binding_id}"));
    let start = source[..marker]
        .rfind("EditableBinding::new(")
        .expect("binding marker should be inside an EditableBinding block");
    let rest = &source[start..];
    let end = rest[1..]
        .find("EditableBinding::new(")
        .map(|index| index + 1)
        .unwrap_or(rest.len());
    &rest[..end]
}

fn editable_binding_blocks_containing<'a>(source: &'a str, pattern: &str) -> Vec<&'a str> {
    let mut blocks = Vec::new();
    let mut search_start = 0;

    while let Some(offset) = source[search_start..].find("EditableBinding::new(") {
        let start = search_start + offset;
        let rest = &source[start..];
        let end = rest[1..]
            .find("EditableBinding::new(")
            .map(|index| index + 1)
            .unwrap_or(rest.len());
        let block = &rest[..end];

        if block.contains(pattern) {
            blocks.push(block);
        }

        search_start = start + end;
    }

    blocks
}

#[test]
fn all_cast_drive_bindings_are_gated_by_channel_services() {
    let blocks = editable_binding_blocks_containing(WORKSPACE_MOD_SOURCE, "ENABLE_WARP_DRIVE");
    assert!(
        blocks.len() >= 15,
        "expected the full Cast Drive command binding set to be audited"
    );

    for block in blocks {
        assert!(
            block.contains(".with_enabled(|| ChannelState::cloud_services_available())"),
            "Cast Drive binding should be hidden when hosted cloud services are unavailable: {block}"
        );
    }
}

#[test]
fn cloud_only_command_palette_bindings_are_gated_by_channel_services() {
    for binding_id in [
        "workspace:show_settings_shared_blocks_page",
        "workspace:show_settings_referrals_page",
        "workspace:show_settings_environments_page",
        "workspace:show_invite_modal",
        "workspace:log_out",
    ] {
        assert!(
            editable_binding_block(WORKSPACE_MOD_SOURCE, binding_id)
                .contains(".with_enabled(|| ChannelState::cloud_services_available())"),
            "{binding_id} should be hidden when hosted cloud services are unavailable"
        );
    }
}

#[test]
fn cast_drive_command_palette_bindings_are_gated_by_channel_services() {
    for binding_id in [
        "workspace:create_team_notebook",
        "workspace:create_personal_notebook",
        "workspace:create_team_workflow",
        "workspace:create_personal_workflow",
        "workspace:create_team_folder",
        "workspace:create_personal_folder",
        "LEFT_PANEL_WARP_DRIVE_BINDING_NAME",
        "TOGGLE_WARP_DRIVE_BINDING_NAME",
        "workspace:search_drive",
        "workspace:export_all_warp_drive_objects",
    ] {
        assert!(
            editable_binding_block(WORKSPACE_MOD_SOURCE, binding_id)
                .contains(".with_enabled(|| ChannelState::cloud_services_available())"),
            "{binding_id} should be hidden when hosted cloud services are unavailable"
        );
    }
}

#[test]
fn cast_drive_auxiliary_bindings_are_gated_by_channel_services() {
    for binding_id in [
        "workspace:create_team_env_vars",
        "workspace:create_personal_env_vars",
        "workspace:create_personal_ai_prompt",
        "workspace:create_team_ai_prompt",
        "workspace:import_to_personal_drive",
        "workspace:import_to_team_drive",
    ] {
        assert!(
            editable_binding_block(WORKSPACE_MOD_SOURCE, binding_id)
                .contains(".with_enabled(|| ChannelState::cloud_services_available())"),
            "{binding_id} should be hidden when hosted cloud services are unavailable"
        );
    }
}

#[test]
fn privacy_settings_copy_is_castcodes_branded() {
    assert!(PRIVACY_PAGE_SOURCE.contains("CastCodes scans terminal blocks"));
    assert!(PRIVACY_PAGE_SOURCE.contains("delete your CastCodes account permanently"));
    assert!(!PRIVACY_PAGE_SOURCE.contains("Warp will scan blocks"));
    assert!(!PRIVACY_PAGE_SOURCE.contains("delete your Warp account permanently"));
}

#[test]
fn visible_settings_copy_uses_castcodes_terms_for_shell_and_shared_surfaces() {
    assert!(SETTINGS_VIEW_SOURCE.contains("\"Shell integration\""));
    assert!(CODE_PAGE_SOURCE.contains(".castcodesindexingignore"));
    assert!(WARPIFY_PAGE_SOURCE.contains("\"Shell integration\""));
    assert!(WARPIFY_PAGE_SOURCE.contains("Configure how CastCodes adds block support"));
    assert!(WARPIFY_PAGE_SOURCE.contains("Use shell integration for interactive SSH sessions."));
    assert!(WARPIFY_PAGE_SOURCE.contains("\"Enable SSH shell integration\""));
    assert!(WARPIFY_PAGE_SOURCE.contains("Use Tmux shell integration"));
    assert!(SHOW_BLOCKS_VIEW_SOURCE.contains("deleted from hosted servers"));

    assert!(!CODE_PAGE_SOURCE.contains(".warpindexingignore file"));
    assert!(!WARPIFY_PAGE_SOURCE.contains("render_page_title(\"Castify\""));
    assert!(!WARPIFY_PAGE_SOURCE.contains("Castify your interactive SSH sessions."));
    assert!(!WARPIFY_PAGE_SOURCE.contains("CastCodes adds support for blocks"));
    assert!(!WARPIFY_PAGE_SOURCE.contains("\"Castify SSH Sessions\""));
    assert!(!WARPIFY_PAGE_SOURCE.contains("Warp attempts"));
    assert!(!WARPIFY_PAGE_SOURCE.contains("Warpification\""));
    assert!(!SHOW_BLOCKS_VIEW_SOURCE.contains("Warp servers"));
}

#[test]
fn visible_settings_copy_uses_clean_castcodes_possessive() {
    let settings_sources = [
        EXTERNAL_EDITOR_SOURCE,
        PRIVACY_PAGE_SOURCE,
        REFERRALS_PAGE_SOURCE,
    ];

    for source in settings_sources {
        assert!(!source.contains("CastCodes's"));
    }

    assert!(EXTERNAL_EDITOR_SOURCE
        .contains("Open Markdown files in the CastCodes Markdown Viewer by default"));
    assert!(PRIVACY_PAGE_SOURCE.contains("Read the CastCodes privacy policy"));
    assert!(
        REFERRALS_PAGE_SOURCE.contains("Sign up to participate in the CastCodes referral program")
    );
}

#[test]
fn terminal_user_facing_copy_uses_castcodes_terms() {
    assert!(SSH_ERROR_SOURCE.contains("Shell integration hit a timeout."));
    assert!(SSH_ERROR_SOURCE.contains("Error setting up shell integration"));
    assert!(SSH_ERROR_SOURCE.contains("Use shell integration without TMUX"));
    assert!(SSH_ERROR_SOURCE.contains("Continue without shell integration"));
    assert!(SSH_WARPIFY_SOURCE.contains("Setting up SSH shell integration"));
    assert!(TERMINAL_INIT_PROJECT_SOURCE.contains("No code is stored on hosted servers."));
    assert!(
        TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE.contains("CastCodes has a custom prompt builder")
    );
    assert!(TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE
        .contains("CastCodes works with many custom prompts"));
    assert!(TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE.contains("CastCodes prompt"));
    assert!(TERMINAL_ONBOARDING_AGENTIC_SUGGESTIONS_SOURCE.contains("Welcome to CastCodes!"));
    assert!(TERMINAL_ONBOARDING_AGENTIC_SUGGESTIONS_SOURCE
        .contains("matrix theme for my CastCodes terminal"));
    assert!(TERMINAL_INLINE_OPEN_IN_WARP_SOURCE
        .contains("CastCodes can directly display Markdown files"));
    assert!(TERMINAL_INLINE_OPEN_IN_WARP_SOURCE.contains("View in CastCodes"));
    assert!(TERMINAL_INLINE_OPEN_IN_WARP_SOURCE.contains("Edit in CastCodes"));
    assert!(TERMINAL_OPEN_IN_WARP_SOURCE.contains("Open {} in CastCodes"));
    assert!(TERMINAL_TOOLTIPS_SOURCE.contains("\"Open in CastCodes\""));
    assert!(TERMINAL_VIEW_SOURCE.contains("MenuItemFields::new(\"Open in CastCodes\")"));
    assert!(TERMINAL_VIEW_SOURCE.contains("Opened shell integration settings"));
    assert!(SSH_INSTALL_TMUX_SOURCE.contains("enable shell integration for your SSH session"));
    assert!(TERMINAL_SSH_REMOTE_SERVER_CHOICE_SOURCE.contains("Install CastCodes' SSH extension"));
    assert!(TERMINAL_SSH_REMOTE_SERVER_CHOICE_SOURCE.contains("Manage shell integration settings"));
    assert!(TERMINAL_SSH_REMOTE_SERVER_FAILED_BANNER_SOURCE
        .contains("Couldn't connect to the CastCodes SSH extension"));
    assert!(TERMINAL_PROMPT_RENDER_HELPER_SOURCE.contains("Installing CastCodes SSH Extension"));
    assert!(TERMINAL_VIEW_SOURCE.contains("MenuItemFields::new(\"Ask AI\")"));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("\"Ask AI about Selection\""));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("\"Ask AI about last block\""));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("\"Enable shell integration\""));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("\"Enable SSH shell integration\""));
    assert!(TERMINAL_VIEW_SOURCE.contains("Powerlevel10k now supports CastCodes!"));
    assert!(TERMINAL_WARPIFY_SUCCESS_BLOCK_SOURCE.contains("Shell integration enabled"));
    assert!(
        TERMINAL_WARPIFY_SUCCESS_BLOCK_SOURCE.contains("automatically enable shell integration")
    );
    assert!(TERMINAL_WARPIFY_RENDER_SOURCE.contains("Never use shell integration for this host"));
    assert!(TERMINAL_VIEW_SOURCE.contains("incompatible with CastCodes"));
    assert!(TERMINAL_VIEW_SOURCE.contains("CastCodes notifications"));
    assert!(TERMINAL_INIT_PROJECT_SOURCE.contains("CastCodes can create one for you"));
    assert!(TERMINAL_INLINE_ALIAS_EXPANSION_SOURCE.contains("CastCodes can auto-expand aliases"));
    assert!(
        TERMINAL_INLINE_NOTIFICATIONS_DISCOVERY_SOURCE.contains("CastCodes was denied permissions")
    );
    assert!(TERMINAL_INLINE_SHELL_PROCESS_TERMINATED_SOURCE
        .contains("CastCodes' initialization script"));
    assert!(TERMINAL_INLINE_VIM_MODE_SOURCE.contains("Enable CastCodes' Vim keybindings"));
    assert!(
        TERMINAL_INLINE_AGENT_MODE_SETUP_SOURCE.contains("Optimize CastCodes for this codebase")
    );
    assert!(TERMINAL_INLINE_AWS_BEDROCK_LOGIN_SOURCE
        .contains("Your administrator has enabled AWS Bedrock"));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("Onboarding Callout: Universal Input - Terminal"));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("Onboarding Callout: Universal Input - Project"));
    assert!(TERMINAL_VIEW_INIT_SOURCE.contains("Onboarding Callout: Universal Input - No Project"));
    assert!(TERMINAL_USE_AGENT_FOOTER_SOURCE.contains("Ask the CastCodes agent"));
    assert!(TERMINAL_WARPIFY_FOOTER_SOURCE.contains("Enable shell integration"));
    assert!(TERMINAL_WARPIFY_BLOCK_BANNER_SOURCE.contains("Enable shell integration"));
    assert!(TERMINAL_INLINE_SSH_SOURCE.contains("SSH shell integration enabled"));
    assert!(TERMINAL_INLINE_SSH_SOURCE.contains("SSH shell integration disabled"));
    assert!(TERMINAL_AMBIENT_AGENT_HOST_SELECTOR_SOURCE.contains("Host::Warp => \"Hosted\""));
    assert!(TERMINAL_AMBIENT_AGENT_HARNESS_SELECTOR_SOURCE.contains("with the hosted agent"));
    assert!(TERMINAL_MODEL_SPEC_SCORES_SOURCE.contains("CastCodes benchmarks"));
    assert!(TERMINAL_PROFILE_MODEL_SELECTOR_SOURCE.contains("CastCodes benchmarks"));
    assert!(TERMINAL_SHELL_TERMINATED_BANNER_SOURCE
        .contains("Shell integration script output is displayed here"));
    assert!(PANE_GROUP_SOURCE.contains("CastCodes doesn't currently support your default shell"));
    assert!(DEBUG_DUMP_SOURCE.contains("CastCodes version"));
    assert!(DEFAULT_TERMINAL_SOURCE.contains("CastCodes as default terminal"));
    assert!(LOCAL_WORKFLOWS_SOURCE.contains("author: Some(\"CastCodes\".into())"));
    assert!(LOCAL_WORKFLOWS_SOURCE.contains("\"castcodes\".into()"));
    assert!(APP_SERVICES_WINDOWS_SOURCE.contains("there is no other instance of CastCodes"));
    assert!(APP_SERVICES_WINDOWS_SINGLE_INSTANCE_SOURCE.contains("CastCodes{:?}_URI_CHANNEL"));
    assert!(APP_SERVICES_WINDOWS_SINGLE_INSTANCE_SOURCE
        .contains("Local\\\\CastCodes{:?}_SingleInstance"));
    assert!(APP_SERVICES_LINUX_SOURCE.contains("dev.castcodes.CastCodes"));
    assert!(APP_SERVICES_LINUX_SOURCE.contains("/dev/castcodes/CastCodes"));
    assert!(AI_DOCUMENT_VIEW_SOURCE.contains("Failed to create Cast Drive notebook"));
    assert!(AI_DOCUMENT_MODEL_SOURCE.contains("saving AI Document to Cast Drive"));
    assert!(PRIVACY_SETTINGS_SOURCE.contains("Cast Drive privacy preferences are set"));
    assert!(PRIVACY_SETTINGS_SOURCE.contains("Cast Drive privacy preferences are not set"));
    assert!(AI_SETTINGS_SOURCE.contains("CastCodes-managed config files"));
    assert!(AI_PAGE_SOURCE.contains("models from providers without API keys are unavailable"));
    assert!(AI_PAGE_SOURCE.contains("cloud_services_available()"));
    assert!(AI_PAGE_SOURCE.contains("hosted credits"));
    assert!(SERVER_API_SOURCE.contains("channel versions and changelogs from configured server"));
    assert!(SERVER_API_SOURCE.contains("Received channel versions from configured server"));
    assert!(AUTOUPDATE_CHANNEL_VERSIONS_SOURCE.contains("channel versions from configured server"));
    assert!(WORKSPACE_VIEW_SOURCE.contains("CastCodes launch modal state"));
    assert!(WORKSPACE_ONE_TIME_MODAL_MODEL_SOURCE.contains("mark CastCodes launch modal"));

    assert!(!SSH_ERROR_SOURCE.contains("Warpifying the session hit a timeout."));
    assert!(!SSH_ERROR_SOURCE.contains("Error Warpifying session"));
    assert!(!SSH_ERROR_SOURCE.contains("Warpify without TMUX"));
    assert!(!SSH_ERROR_SOURCE.contains("Continue without Warpification"));
    assert!(!SSH_WARPIFY_SOURCE.contains("Warpifying SSH Session"));
    assert!(!TERMINAL_INIT_PROJECT_SOURCE.contains("Warp servers"));
    assert!(!TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE.contains("Warp has a custom prompt builder"));
    assert!(!TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE.contains("Warp prompt"));
    assert!(!TERMINAL_ONBOARDING_AGENTIC_SUGGESTIONS_SOURCE.contains("Welcome to Warp!"));
    assert!(!TERMINAL_INLINE_OPEN_IN_WARP_SOURCE.contains("View in Warp"));
    assert!(!TERMINAL_INLINE_OPEN_IN_WARP_SOURCE.contains("Edit in Warp"));
    assert!(!TERMINAL_TOOLTIPS_SOURCE.contains("\"Open in Warp\""));
    assert!(!TERMINAL_SSH_REMOTE_SERVER_CHOICE_SOURCE.contains("Install Warp's SSH extension"));
    assert!(!TERMINAL_SSH_REMOTE_SERVER_CHOICE_SOURCE.contains("Manage Warpify settings"));
    assert!(!TERMINAL_SSH_REMOTE_SERVER_FAILED_BANNER_SOURCE
        .contains("Couldn't connect to the Warp SSH extension"));
    assert!(!TERMINAL_PROMPT_RENDER_HELPER_SOURCE.contains("Warp SSH Extension"));
    assert!(!TERMINAL_VIEW_SOURCE.contains("MenuItemFields::new(\"Ask Warp AI\")"));
    assert!(!TERMINAL_VIEW_SOURCE.contains("Opened Warpify Settings"));
    assert!(!TERMINAL_VIEW_INIT_SOURCE.contains("\"Ask Warp AI"));
    assert!(!TERMINAL_VIEW_INIT_SOURCE.contains("Onboarding Callout: WarpInput"));
    assert!(!TERMINAL_VIEW_INIT_SOURCE.contains("\"Warpify subshell\""));
    assert!(!TERMINAL_VIEW_INIT_SOURCE.contains("\"Warpify ssh session\""));
    assert!(!TERMINAL_VIEW_SOURCE.contains("Powerlevel10k now supports Warp!"));
    assert!(!TERMINAL_WARPIFY_SUCCESS_BLOCK_SOURCE.contains("Session Warpified"));
    assert!(!TERMINAL_WARPIFY_SUCCESS_BLOCK_SOURCE.contains("automatically Warpify"));
    assert!(!TERMINAL_WARPIFY_RENDER_SOURCE.contains("Never Warpify this host"));
    assert!(!TERMINAL_VIEW_SOURCE.contains("incompatible with Warp"));
    assert!(!TERMINAL_VIEW_SOURCE.contains("Warp notifications"));
    assert!(!TERMINAL_INIT_PROJECT_SOURCE.contains("Warp can create one for you"));
    assert!(!TERMINAL_INLINE_ALIAS_EXPANSION_SOURCE.contains("Warp can auto-expand aliases"));
    assert!(!TERMINAL_INLINE_NOTIFICATIONS_DISCOVERY_SOURCE.contains("Warp was denied permissions"));
    assert!(
        !TERMINAL_INLINE_SHELL_PROCESS_TERMINATED_SOURCE.contains("Warp's initialization script")
    );
    assert!(!TERMINAL_INLINE_VIM_MODE_SOURCE.contains("Enable Warp's Vim keybindings"));
    assert!(!TERMINAL_INLINE_AGENT_MODE_SETUP_SOURCE.contains("Optimize Warp for this codebase"));
    assert!(!TERMINAL_INLINE_AWS_BEDROCK_LOGIN_SOURCE.contains("Your Warp admin"));
    assert!(!TERMINAL_USE_AGENT_FOOTER_SOURCE.contains("Ask the Warp agent"));
    assert!(!TERMINAL_WARPIFY_FOOTER_SOURCE.contains("Warpify subshell"));
    assert!(!TERMINAL_WARPIFY_BLOCK_BANNER_SOURCE.contains("Warpify subshell"));
    assert!(!TERMINAL_INLINE_SSH_SOURCE.contains("Warp SSH wrapper"));
    assert!(!TERMINAL_AMBIENT_AGENT_HOST_SELECTOR_SOURCE.contains("Host::Warp => \"Warp\""));
    assert!(!TERMINAL_AMBIENT_AGENT_HARNESS_SELECTOR_SOURCE.contains("with the Warp Agent"));
    assert!(!TERMINAL_MODEL_SPEC_SCORES_SOURCE.contains("Warp's benchmarks"));
    assert!(!TERMINAL_PROFILE_MODEL_SELECTOR_SOURCE.contains("Warp’s benchmarks"));
    assert!(!PANE_GROUP_SOURCE.contains("Warp doesn't currently support your default shell"));
    assert!(!DEBUG_DUMP_SOURCE.contains("Warp version"));
    assert!(!DEFAULT_TERMINAL_SOURCE.contains("Warp as default terminal"));
    assert!(!LOCAL_WORKFLOWS_SOURCE.contains("author: Some(\"Warp\".into())"));
    assert!(!LOCAL_WORKFLOWS_SOURCE.contains("\"warp\".into()"));
    assert!(!APP_SERVICES_WINDOWS_SOURCE.contains("there is no other instance of Warp"));
    assert!(!APP_SERVICES_WINDOWS_SINGLE_INSTANCE_SOURCE.contains("Warp{:?}_URI_CHANNEL"));
    assert!(
        !APP_SERVICES_WINDOWS_SINGLE_INSTANCE_SOURCE.contains("Local\\\\Warp{:?}_SingleInstance")
    );
    assert!(!APP_SERVICES_LINUX_SOURCE.contains("dev.warp.WarpLocal"));
    assert!(!APP_SERVICES_LINUX_SOURCE.contains("/dev/warp/WarpLocal"));
    assert!(!AI_DOCUMENT_VIEW_SOURCE.contains("Failed to create Warp Drive notebook"));
    assert!(!AI_DOCUMENT_MODEL_SOURCE.contains("saving AI Document to Warp Drive"));
    assert!(!PRIVACY_SETTINGS_SOURCE.contains("Warp Drive privacy preferences"));
    assert!(!AI_SETTINGS_SOURCE.contains("Warp-native config files"));
    assert!(!AI_PAGE_SOURCE.contains("CastCodes credits"));
    assert!(!AI_PAGE_SOURCE.contains("CastCodes's provided models"));
    assert!(!SERVER_API_SOURCE.contains("Warp server"));
    assert!(!AUTOUPDATE_CHANNEL_VERSIONS_SOURCE.contains("Warp server"));
    assert!(!WORKSPACE_VIEW_SOURCE.contains("OpenWarp launch modal state"));
    assert!(!WORKSPACE_ONE_TIME_MODAL_MODEL_SOURCE.contains("mark OpenWarp launch modal"));
}

#[test]
fn settings_schema_descriptions_use_castcodes_terms() {
    let sources = [
        GENERAL_SETTINGS_SOURCE,
        SESSION_SETTINGS_SOURCE,
        TERMINAL_WARPIFY_SETTINGS_SOURCE,
    ];
    let forbidden = [
        "description: \"Whether to show a warning dialog before quitting Warp.",
        "description: \"Whether to quit Warp when the last window is closed.",
        "description: \"Whether to restore the previous session when Warp starts up.",
        "description: \"Whether to launch Warp automatically when you log in.",
        "description: \"The shell to use when Warp starts up.",
        "description: \"Whether to use your shell's PS1 prompt instead of the Warp prompt.",
        "description: \"Commands that should not trigger the subshell warpification prompt.",
        "description: \"SSH hosts that should not trigger the warpification prompt.",
        "description: \"Whether to enable Warp features in SSH sessions.",
        "description: \"Whether to use a tmux-based wrapper for SSH warpification.",
    ];

    assert!(GENERAL_SETTINGS_SOURCE.contains("before quitting CastCodes"));
    assert!(SESSION_SETTINGS_SOURCE.contains("CastCodes prompt"));
    assert!(TERMINAL_WARPIFY_SETTINGS_SOURCE.contains("SSH shell integration"));
    assert!(AI_SETTINGS_SOURCE.contains("hosted credits"));
    assert!(!AI_SETTINGS_SOURCE.contains("CastCodes credits can be used"));

    for source in sources {
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "settings schema description should not contain {needle}"
            );
        }
    }
}

#[test]
fn settings_search_terms_use_castcodes_terms() {
    assert!(FEATURES_PAGE_SOURCE.contains("\"castcodes default terminal application\""));
    assert!(APPEARANCE_PAGE_SOURCE.contains(
        "\"left tools panel open closed across tabs file tree project explorer global search cast drive conversation list\""
    ));
    assert!(APPEARANCE_PAGE_SOURCE.contains(
        "\"input type castcodes universal classic style prompt terminal ai developer mode interface shell chips ps1\""
    ));
    assert!(APPEARANCE_PAGE_SOURCE.contains("\"prompt ps1 terminal castcodes shell custom\""));
    assert!(ENVIRONMENTS_PAGE_SOURCE
        .contains("\"environments environment ambient agents github castcodes assisted manual configuration\""));
    assert!(WARPIFY_PAGE_SOURCE.contains("\"ssh shell integration subshell session\""));
    assert!(WARPIFY_PAGE_SOURCE.contains("\"shell integration subshell\""));
    assert!(WARPIFY_PAGE_SOURCE.contains("\"shell integration ssh\""));

    assert!(!FEATURES_PAGE_SOURCE.contains("\"warp default terminal application\""));
    assert!(!APPEARANCE_PAGE_SOURCE.contains(
        "\"left tools panel open closed across tabs file tree project explorer global search warp drive conversation list\""
    ));
    assert!(!APPEARANCE_PAGE_SOURCE.contains(
        "\"input type warp universal classic style prompt terminal ai developer mode interface shell chips ps1\""
    ));
    assert!(!APPEARANCE_PAGE_SOURCE.contains("\"prompt ps1 terminal warp shell custom\""));
    assert!(!ENVIRONMENTS_PAGE_SOURCE.contains(
        "\"environments environment ambient agents github warp assisted manual configuration\""
    ));
    assert!(!WARPIFY_PAGE_SOURCE.contains("\"ssh subshell warpify session\""));
    assert!(!WARPIFY_PAGE_SOURCE.contains("\"warpify subshell\""));
    assert!(!WARPIFY_PAGE_SOURCE.contains("\"warpify ssh\""));
}

#[test]
fn settings_sources_do_not_link_to_warp_owned_support_surfaces() {
    let upstream_docs_host = ["docs", "warp", "dev"].join(".");
    let upstream_support = ["support", "warp.dev"].join("@");
    let upstream_sales = ["sales", "warp.dev"].join("@");
    let upstream_referrals = ["referrals", "warp.dev"].join("@");
    let upstream_typeform = ["warpdotdev", "typeform", "com"].join(".");
    let upstream_marketing_host = ["www", "warp", "dev"].join(".");
    let forbidden = [
        upstream_docs_host.as_str(),
        upstream_support.as_str(),
        upstream_sales.as_str(),
        upstream_referrals.as_str(),
        upstream_typeform.as_str(),
        upstream_marketing_host.as_str(),
    ];
    let sources = [
        ABOUT_PAGE_SOURCE,
        AI_PAGE_SOURCE,
        APPEARANCE_PAGE_SOURCE,
        EXTERNAL_EDITOR_SOURCE,
        FEATURES_PAGE_SOURCE,
        MAIN_PAGE_SOURCE,
        MCP_SERVERS_LIST_PAGE_SOURCE,
        PLATFORM_PAGE_SOURCE,
        PRIVACY_PAGE_SOURCE,
        REFERRALS_PAGE_SOURCE,
        SHOW_BLOCKS_VIEW_SOURCE,
        SSH_INSTALL_TMUX_SOURCE,
        TERMINAL_INLINE_OPEN_IN_WARP_SOURCE,
        TERMINAL_ONBOARDING_AGENTIC_SUGGESTIONS_SOURCE,
        TERMINAL_ONBOARDING_PROMPT_BLOCK_SOURCE,
        TERMINAL_OPEN_IN_WARP_SOURCE,
        TERMINAL_SSH_REMOTE_SERVER_CHOICE_SOURCE,
        TERMINAL_SSH_REMOTE_SERVER_FAILED_BANNER_SOURCE,
        TERMINAL_VIEW_SOURCE,
        TERMINAL_WARPIFY_RENDER_SOURCE,
        THEME_PICKER_SLIDE_SOURCE,
        WARP_DRIVE_PAGE_SOURCE,
        WARPIFY_PAGE_SOURCE,
    ];

    for source in sources {
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "settings source should not link to {needle}"
            );
        }
    }
}

#[test]
fn subpage_from_str_parses_display_names() {
    // The legacy "Warp Agent" name plus the new "Cast Agent" display name must
    // resolve to SettingsSection::WarpAgent so existing deep links, persisted
    // telemetry strings, and external callers continue to work after the
    // user-facing rename (see specs/GH1063/product.md, Behavior #8).
    assert_eq!(
        SettingsSection::from_str("Warp Agent"),
        Ok(SettingsSection::WarpAgent)
    );
    assert_eq!(
        SettingsSection::from_str("Profiles"),
        Ok(SettingsSection::AgentProfiles)
    );
    assert_eq!(
        SettingsSection::from_str("Knowledge"),
        Ok(SettingsSection::Knowledge)
    );
    assert_eq!(
        SettingsSection::from_str("Indexing and projects"),
        Ok(SettingsSection::CodeIndexing)
    );
    assert_eq!(
        SettingsSection::from_str("Editor and Code Review"),
        Ok(SettingsSection::EditorAndCodeReview)
    );
    assert_eq!(
        SettingsSection::from_str("Agent API Keys"),
        Ok(SettingsSection::OzCloudAPIKeys)
    );
    assert_eq!(
        SettingsSection::from_str("Shell integration"),
        Ok(SettingsSection::Warpify)
    );
}

// ── Subpage search filter simulation ────────────────────────────────────────
// These tests simulate the per-subpage search filtering logic used in
// handle_search_editor_event: each subpage should only be visible if its
// own widgets' search terms match, not if a sibling subpage's terms match.

/// Helper: given a map of subpage→MatchData, returns which subpages are visible.
fn visible_subpages(
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    subpages: &[SettingsSection],
) -> Vec<SettingsSection> {
    subpages
        .iter()
        .filter(|s| {
            subpage_filter
                .get(s)
                .map(|md| md.is_truthy())
                .unwrap_or(false)
        })
        .copied()
        .collect()
}

#[test]
fn search_knowledge_shows_only_knowledge_subpage() {
    // Simulate: searching "knowledge" matched the Knowledge subpage but not others.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    assert_eq!(visible, vec![SettingsSection::Knowledge]);
}

#[test]
fn search_agent_shows_profiles_and_cli_agents() {
    // "agent" appears in both AgentProfiles and ThirdPartyCLIAgents search terms.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(2));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(1),
    );

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    assert!(visible.contains(&SettingsSection::AgentProfiles));
    assert!(visible.contains(&SettingsSection::ThirdPartyCLIAgents));
    assert!(!visible.contains(&SettingsSection::WarpAgent));
    assert!(!visible.contains(&SettingsSection::Knowledge));
}

#[test]
fn empty_search_shows_no_subpages_in_filter() {
    // When search is cleared, subpage_filter is empty — all subpages fall back
    // to their backing page visibility (Uncounted(true) by default).
    let filter: HashMap<SettingsSection, MatchData> = HashMap::new();

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    // No entries in filter means no subpage-specific filtering; all return false
    // from the filter map. The actual rendering code falls back to the backing
    // page's pages_filter which defaults to Uncounted(true).
    assert!(visible.is_empty());
}

#[test]
fn search_with_no_matches_hides_all_subpages() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    assert!(visible.is_empty());
}

/// Helper: check if an umbrella should be visible given a subpage filter.
fn umbrella_visible(
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    umbrella_subpages: &[SettingsSection],
) -> bool {
    umbrella_subpages.iter().any(|s| {
        subpage_filter
            .get(s)
            .map(|md| md.is_truthy())
            .unwrap_or(false)
    })
}

#[test]
fn umbrella_hidden_when_no_subpages_match() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    assert!(!umbrella_visible(&filter, SettingsSection::ai_subpages()));
}

// ── cycle_pages search filter ────────────────────────────────────────────────
// These tests validate the logic added to cycle_pages() to ensure arrow key
// navigation respects the active search filter.

/// Mirrors the filter predicate used in cycle_pages() when search is active.
fn section_passes_nav_filter(
    section: SettingsSection,
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    pages_filter: &[(SettingsSection, MatchData)],
) -> bool {
    if let Some(md) = subpage_filter.get(&section) {
        md.is_truthy()
    } else {
        let backing = section.parent_page_section();
        pages_filter
            .iter()
            .any(|(s, md)| *s == backing && md.is_truthy())
    }
}

#[test]
fn nav_filter_includes_matching_subpage_and_excludes_others() {
    let mut subpage_filter = HashMap::new();
    subpage_filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    subpage_filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    subpage_filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    subpage_filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    // No page-level filter entries needed since all AI subpages have subpage_filter entries.
    let pages_filter: Vec<(SettingsSection, MatchData)> = vec![];

    assert!(!section_passes_nav_filter(
        SettingsSection::WarpAgent,
        &subpage_filter,
        &pages_filter
    ));
    assert!(!section_passes_nav_filter(
        SettingsSection::AgentProfiles,
        &subpage_filter,
        &pages_filter
    ));
    assert!(section_passes_nav_filter(
        SettingsSection::Knowledge,
        &subpage_filter,
        &pages_filter
    ));
    assert!(!section_passes_nav_filter(
        SettingsSection::ThirdPartyCLIAgents,
        &subpage_filter,
        &pages_filter
    ));
}

#[test]
fn nav_filter_falls_back_to_pages_filter_for_top_level_pages() {
    // Top-level pages (Account, Appearance, etc.) have no subpage_filter entry.
    // They fall back to pages_filter using parent_page_section() == themselves.
    let subpage_filter: HashMap<SettingsSection, MatchData> = HashMap::new();
    let pages_filter = vec![
        (SettingsSection::Account, MatchData::Uncounted(true)),
        (SettingsSection::Appearance, MatchData::Countable(0)),
        (SettingsSection::Features, MatchData::Uncounted(true)),
    ];

    assert!(section_passes_nav_filter(
        SettingsSection::Account,
        &subpage_filter,
        &pages_filter
    ));
    assert!(!section_passes_nav_filter(
        SettingsSection::Appearance,
        &subpage_filter,
        &pages_filter
    ));
    assert!(section_passes_nav_filter(
        SettingsSection::Features,
        &subpage_filter,
        &pages_filter
    ));
}

#[test]
fn umbrella_visible_when_any_subpage_matches() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    assert!(umbrella_visible(&filter, SettingsSection::ai_subpages()));
}

// ── Search auto-select simulation ───────────────────────────────────────────
// These tests simulate the auto-select logic in handle_search_editor_event:
// when the current subpage is filtered out by search, the view should jump
// to the first visible subpage or page.

/// Simulates the "is current still visible" check from the search handler.
/// Returns true if `current` is still visible given the subpage_filter and
/// a list of (backing_section, is_truthy) pairs for pages_filter.
fn is_current_visible(
    current: SettingsSection,
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    pages_visible: &[(SettingsSection, bool)],
) -> bool {
    if let Some(md) = subpage_filter.get(&current) {
        return md.is_truthy();
    }
    let backing = current.parent_page_section();
    pages_visible
        .iter()
        .any(|(section, visible)| *section == backing && *visible)
}

/// Simulates finding the first visible section from the nav_items order.
fn first_visible_section(
    nav_order: &[SettingsSection],
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    pages_visible: &[(SettingsSection, bool)],
) -> Option<SettingsSection> {
    nav_order.iter().copied().find(|section| {
        if let Some(md) = subpage_filter.get(section) {
            md.is_truthy()
        } else {
            let backing = section.parent_page_section();
            pages_visible
                .iter()
                .any(|(s, visible)| *s == backing && *visible)
        }
    })
}

#[test]
fn auto_select_jumps_away_from_filtered_out_subpage() {
    // User is on Knowledge, searches "agent" which matches Profiles but not Knowledge.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(2));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(1),
    );

    let current = SettingsSection::Knowledge;
    assert!(
        !is_current_visible(current, &filter, &[]),
        "Knowledge should not be visible when it has 0 matches"
    );

    // The nav order: Agent, Profiles, ..., Knowledge, ThirdPartyCLI
    let nav_order = SettingsSection::ai_subpages();
    let first = first_visible_section(nav_order, &filter, &[]);
    assert_eq!(
        first,
        Some(SettingsSection::AgentProfiles),
        "Should auto-select Profiles as the first visible subpage"
    );
}

#[test]
fn auto_select_stays_on_current_when_it_matches() {
    // User is on Knowledge, searches "knowledge" which matches Knowledge.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let current = SettingsSection::Knowledge;
    assert!(
        is_current_visible(current, &filter, &[]),
        "Knowledge should remain visible when it has matches"
    );
}

#[test]
fn auto_select_falls_back_to_top_level_page_when_no_subpages_match() {
    // All AI subpages filtered out, but Account (top-level) is still visible.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let pages_visible = vec![
        (SettingsSection::Account, true),
        (SettingsSection::AI, false),
    ];

    // Nav order includes top-level Account before the AI subpages.
    let nav_order = vec![
        SettingsSection::Account,
        SettingsSection::WarpAgent,
        SettingsSection::AgentProfiles,
        SettingsSection::Knowledge,
        SettingsSection::ThirdPartyCLIAgents,
    ];

    let first = first_visible_section(&nav_order, &filter, &pages_visible);
    assert_eq!(
        first,
        Some(SettingsSection::Account),
        "Should fall back to Account when no subpages match"
    );
}

#[test]
fn auto_select_handles_standalone_subpage_via_backing_page() {
    // AgentMCPServers has its own backing page (MCPServers), not in subpage_filter.
    // It should be visible if its backing page is visible.
    let filter = HashMap::new(); // no per-subpage entries for AgentMCPServers

    let pages_visible = vec![
        (SettingsSection::MCPServers, true),
        (SettingsSection::AI, false),
    ];

    let current = SettingsSection::AgentMCPServers;
    assert!(
        is_current_visible(current, &filter, &pages_visible),
        "AgentMCPServers should be visible via its MCPServers backing page"
    );
}

#[test]
fn auto_select_with_no_matches_anywhere() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));

    let pages_visible = vec![
        (SettingsSection::Account, false),
        (SettingsSection::AI, false),
    ];

    let nav_order = vec![
        SettingsSection::Account,
        SettingsSection::WarpAgent,
        SettingsSection::AgentProfiles,
    ];

    let first = first_visible_section(&nav_order, &filter, &pages_visible);
    assert_eq!(
        first, None,
        "No section should be selected when nothing matches"
    );
}

// ── Backward compatibility ──────────────────────────────────────────────────

#[test]
fn legacy_ai_section_maps_to_agent_default() {
    // SettingsSection::AI should be treated as backward-compat and map to the agent page
    // via the code in set_and_refresh_current_page_internal.
    // Here we just verify the parent_page_section is still AI (for page lookup).
    assert_eq!(
        SettingsSection::AI.parent_page_section(),
        SettingsSection::AI
    );
    // And that AI is NOT itself a subpage.
    assert!(!SettingsSection::AI.is_subpage());
}

// ── Collapsed umbrella nav-stop behavior ────────────────────────────────────
// Verify that arrow-key navigation lands on a collapsed umbrella as a single
// stop (and activates it by jumping to the first subpage, which auto-expands
// the umbrella) instead of silently skipping over it.

use nav::{SettingsNavItem, SettingsUmbrella};

/// Builds the nav-items layout used by `SettingsView::new`, matching the real
/// sidebar ordering so tests exercise realistic nav orders.
fn realistic_nav_items_for_channel(cloud_services_available: bool) -> Vec<SettingsNavItem> {
    let mut nav_items = vec![
        SettingsNavItem::Page(SettingsSection::Account),
        SettingsNavItem::Umbrella(SettingsUmbrella::new(
            "Agents",
            SettingsSection::ai_subpages().to_vec(),
        )),
        SettingsNavItem::Page(SettingsSection::Appearance),
        SettingsNavItem::Umbrella(SettingsUmbrella::new(
            "Code",
            SettingsSection::code_subpages().to_vec(),
        )),
        SettingsNavItem::Page(SettingsSection::Features),
    ];
    if cloud_services_available {
        nav_items.insert(
            4,
            SettingsNavItem::Umbrella(SettingsUmbrella::new(
                "Platform",
                SettingsSection::cloud_platform_subpages().to_vec(),
            )),
        );
    }
    nav_items
}

fn realistic_nav_items() -> Vec<SettingsNavItem> {
    realistic_nav_items_for_channel(true)
}

/// Mutably flips an umbrella's `expanded` flag at `nav_index`.
fn set_expanded(nav_items: &mut [SettingsNavItem], nav_index: usize, expanded: bool) {
    if let Some(SettingsNavItem::Umbrella(u)) = nav_items.get_mut(nav_index) {
        u.expanded = expanded;
    } else {
        panic!("nav_items[{nav_index}] is not an Umbrella");
    }
}

#[test]
fn collapsed_umbrella_is_a_single_nav_stop() {
    let nav_items = realistic_nav_items();
    // All umbrellas default to collapsed.
    let stops = build_nav_stops(&nav_items, |_| true);

    // Expect: Account, <Agents umbrella>, Appearance, <Code umbrella>,
    // <Platform umbrella>, Features.
    assert_eq!(stops.len(), 6);
    assert!(matches!(
        stops[0],
        NavStop::Section(SettingsSection::Account)
    ));
    assert!(matches!(
        stops[1],
        NavStop::CollapsedUmbrella {
            nav_index: 1,
            first_subpage: SettingsSection::WarpAgent,
            last_subpage: SettingsSection::ThirdPartyCLIAgents,
        }
    ));
    assert!(matches!(
        stops[2],
        NavStop::Section(SettingsSection::Appearance)
    ));
    assert!(matches!(
        stops[3],
        NavStop::CollapsedUmbrella {
            nav_index: 3,
            first_subpage: SettingsSection::CodeIndexing,
            last_subpage: SettingsSection::EditorAndCodeReview,
        }
    ));
    assert!(matches!(
        stops[4],
        NavStop::CollapsedUmbrella {
            nav_index: 4,
            first_subpage: SettingsSection::CloudEnvironments,
            last_subpage: SettingsSection::OzCloudAPIKeys,
        }
    ));
    assert!(matches!(
        stops[5],
        NavStop::Section(SettingsSection::Features)
    ));
}

#[test]
fn expanded_umbrella_produces_section_stop_per_subpage() {
    let mut nav_items = realistic_nav_items();
    // Expand the Agents umbrella so each of its subpages becomes a nav stop.
    set_expanded(&mut nav_items, 1, true);

    let stops = build_nav_stops(&nav_items, |_| true);

    // Expect: Account, WarpAgent, AgentProfiles, AgentMCPServers, Knowledge,
    // ThirdPartyCLIAgents, Appearance, <Code umbrella>,
    // <Platform umbrella>, Features.
    let sections: Vec<_> = stops
        .iter()
        .map(|s| match s {
            NavStop::Section(section) => format!("{section:?}"),
            NavStop::CollapsedUmbrella { nav_index, .. } => format!("Umbrella@{nav_index}"),
        })
        .collect();
    assert_eq!(
        sections,
        vec![
            "Account",
            "WarpAgent",
            "AgentProfiles",
            "AgentMCPServers",
            "Knowledge",
            "ThirdPartyCLIAgents",
            "Appearance",
            "Umbrella@3",
            "Umbrella@4",
            "Features",
        ]
    );
}

#[test]
fn collapsed_umbrella_with_filtered_subpages_uses_first_visible_subpage() {
    // When a search filter hides the first subpage, activating the collapsed
    // umbrella should land on the *next* visible subpage (still auto-expanding).
    let nav_items = realistic_nav_items();

    let stops = build_nav_stops(&nav_items, |section| {
        // Hide WarpAgent (first AI subpage); keep the rest.
        section != SettingsSection::WarpAgent
    });

    let agents_stop = stops
        .iter()
        .find(|s| matches!(s, NavStop::CollapsedUmbrella { nav_index: 1, .. }))
        .expect("Agents umbrella should still be a collapsed stop");

    match agents_stop {
        NavStop::CollapsedUmbrella {
            first_subpage,
            last_subpage,
            ..
        } => {
            assert_eq!(
                *first_subpage,
                SettingsSection::AgentProfiles,
                "WarpAgent is hidden by the filter, so the first visible subpage is AgentProfiles"
            );
            assert_eq!(
                *last_subpage,
                SettingsSection::ThirdPartyCLIAgents,
                "last_subpage is unaffected by hiding WarpAgent and should remain the last visible subpage"
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn umbrella_with_no_visible_subpages_is_skipped_entirely() {
    let nav_items = realistic_nav_items();

    let stops = build_nav_stops(&nav_items, |section| !section.is_ai_subpage());

    // The Agents umbrella's subpages are all AI subpages, so the entire
    // umbrella should be absent from the nav order.
    assert!(
        stops
            .iter()
            .all(|s| !matches!(s, NavStop::CollapsedUmbrella { nav_index: 1, .. })),
        "Agents umbrella should not appear when none of its subpages are visible"
    );
    // The still-visible Code / Platform umbrellas remain as stops.
    assert!(stops
        .iter()
        .any(|s| matches!(s, NavStop::CollapsedUmbrella { nav_index: 3, .. })));
    assert!(stops
        .iter()
        .any(|s| matches!(s, NavStop::CollapsedUmbrella { nav_index: 4, .. })));
}

#[test]
fn oss_nav_order_excludes_cloud_platform_umbrella() {
    let nav_items = realistic_nav_items_for_channel(false);
    let stops = build_nav_stops(&nav_items, |_| true);

    assert!(stops.iter().all(|s| !matches!(
        s,
        NavStop::CollapsedUmbrella {
            first_subpage: SettingsSection::CloudEnvironments,
            last_subpage: SettingsSection::OzCloudAPIKeys,
            ..
        }
    )));
    assert!(stops
        .iter()
        .all(|s| !matches!(s, NavStop::Section(SettingsSection::CloudEnvironments))));
    assert!(stops
        .iter()
        .all(|s| !matches!(s, NavStop::Section(SettingsSection::OzCloudAPIKeys))));
    assert!(stops
        .iter()
        .all(|s| !matches!(s, NavStop::Section(SettingsSection::SharedBlocks))));
}

#[test]
fn filtered_out_top_level_page_is_skipped() {
    let nav_items = realistic_nav_items();

    let stops = build_nav_stops(&nav_items, |section| section != SettingsSection::Features);

    assert!(
        !stops
            .iter()
            .any(|s| matches!(s, NavStop::Section(SettingsSection::Features))),
        "Features should be filtered out entirely"
    );
    // But other pages remain.
    assert!(stops
        .iter()
        .any(|s| matches!(s, NavStop::Section(SettingsSection::Account))));
}

// ── current_stop_index ──────────────────────────────────────────────────────

#[test]
fn current_stop_index_matches_section_stop() {
    let nav_items = realistic_nav_items();
    let stops = build_nav_stops(&nav_items, |_| true);

    let idx = current_stop_index(&stops, &nav_items, SettingsSection::Appearance);
    assert_eq!(idx, Some(2));
}

#[test]
fn current_stop_index_maps_subpage_to_collapsed_umbrella() {
    // Edge case: the user manually collapsed the Agents umbrella while still
    // on one of its subpages. The collapsed umbrella should match as the
    // current stop so arrow-key cycling continues from the umbrella's position.
    let nav_items = realistic_nav_items();
    let stops = build_nav_stops(&nav_items, |_| true);

    let idx = current_stop_index(&stops, &nav_items, SettingsSection::Knowledge);
    assert_eq!(
        idx,
        Some(1),
        "Knowledge is under the collapsed Agents umbrella at nav_index 1"
    );
}

#[test]
fn current_stop_index_returns_none_when_section_is_not_present() {
    let nav_items = realistic_nav_items();
    // Filter out all AI subpages (and therefore the Agents umbrella) entirely.
    let stops = build_nav_stops(&nav_items, |section| !section.is_ai_subpage());

    // Knowledge isn't directly in stops, and no remaining collapsed umbrella
    // contains it, so current_stop_index should return None.
    assert_eq!(
        current_stop_index(&stops, &nav_items, SettingsSection::Knowledge),
        None
    );
}

// ── next_stop_index wrapping ────────────────────────────────────────────────

#[test]
fn next_stop_index_wraps_at_ends() {
    assert_eq!(next_stop_index(0, 3, CycleDirection::Up), 2);
    assert_eq!(next_stop_index(2, 3, CycleDirection::Down), 0);
    assert_eq!(next_stop_index(1, 3, CycleDirection::Up), 0);
    assert_eq!(next_stop_index(1, 3, CycleDirection::Down), 2);
}

#[test]
fn next_stop_index_handles_single_stop() {
    assert_eq!(next_stop_index(0, 1, CycleDirection::Up), 0);
    assert_eq!(next_stop_index(0, 1, CycleDirection::Down), 0);
}

// ── End-to-end cycling (no search) ──────────────────────────────────────────
// These tests simulate the sequence of nav-stop activations that would result
// from repeatedly pressing Down/Up, ensuring a collapsed umbrella is never
// skipped over.

/// Computes the section that would become active after applying the direction
/// once, starting from `current`. Mirrors the final target-resolution step in
/// `cycle_pages`.
fn simulate_cycle(
    nav_items: &[SettingsNavItem],
    stops: &[NavStop],
    current: SettingsSection,
    direction: CycleDirection,
) -> SettingsSection {
    let active = current_stop_index(stops, nav_items, current)
        .expect("current should exist in stops in these tests");
    let next = next_stop_index(active, stops.len(), direction);
    match stops[next] {
        NavStop::Section(section) => section,
        NavStop::CollapsedUmbrella {
            first_subpage,
            last_subpage,
            ..
        } => match direction {
            CycleDirection::Up => last_subpage,
            CycleDirection::Down => first_subpage,
        },
    }
}

#[test]
fn arrow_down_from_account_with_collapsed_agents_lands_on_first_subpage() {
    let nav_items = realistic_nav_items();
    let stops = build_nav_stops(&nav_items, |_| true);

    // Pressing Down from Account should auto-expand Agents and select WarpAgent,
    // not skip over to Appearance.
    let next = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::Account,
        CycleDirection::Down,
    );
    assert_eq!(next, SettingsSection::WarpAgent);
}

#[test]
fn arrow_up_from_appearance_with_collapsed_agents_lands_on_last_subpage() {
    let nav_items = realistic_nav_items();
    let stops = build_nav_stops(&nav_items, |_| true);

    // Pressing Up from Appearance should land on the collapsed Agents
    // umbrella, which resolves to ThirdPartyCLIAgents (last visible subpage)
    // so the user continues moving in natural reading order rather than being
    // jumped back to the top of the umbrella.
    let next = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::Appearance,
        CycleDirection::Up,
    );
    assert_eq!(next, SettingsSection::ThirdPartyCLIAgents);
}

#[test]
fn arrow_up_into_collapsed_umbrella_respects_search_filter_for_last_subpage() {
    let nav_items = realistic_nav_items();
    // Hide the last two AI subpages; the last *visible* subpage of the
    // still-collapsed Agents umbrella should be AgentMCPServers.
    let is_visible = |section: SettingsSection| {
        !matches!(
            section,
            SettingsSection::Knowledge | SettingsSection::ThirdPartyCLIAgents
        )
    };
    let stops = build_nav_stops(&nav_items, is_visible);

    // From Appearance, Up should land on the last *visible* AI subpage
    // (AgentMCPServers), not on the filtered-out Knowledge/ThirdPartyCLIAgents
    // or on the first subpage WarpAgent.
    let next = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::Appearance,
        CycleDirection::Up,
    );
    assert_eq!(next, SettingsSection::AgentMCPServers);
}

#[test]
fn arrow_down_from_expanded_last_subpage_leaves_umbrella() {
    let mut nav_items = realistic_nav_items();
    set_expanded(&mut nav_items, 1, true); // expand Agents
    let stops = build_nav_stops(&nav_items, |_| true);

    // ThirdPartyCLIAgents is the last Agents subpage; Down should move to
    // Appearance (the next top-level page in the nav order).
    let next = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::ThirdPartyCLIAgents,
        CycleDirection::Down,
    );
    assert_eq!(next, SettingsSection::Appearance);
}

#[test]
fn arrow_down_across_adjacent_collapsed_umbrellas() {
    let nav_items = realistic_nav_items();
    // Both Code and Platform umbrellas are collapsed.
    let stops = build_nav_stops(&nav_items, |_| true);

    // From Appearance, Down should land on the first Code subpage
    // (Code umbrella auto-expands).
    let next_after_billing = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::Appearance,
        CycleDirection::Down,
    );
    assert_eq!(next_after_billing, SettingsSection::CodeIndexing);

    // From the Code umbrella stop (i.e. the user is "on" CodeIndexing which
    // maps back to the collapsed umbrella), pressing Down again should land
    // on the Platform umbrella's first subpage.
    let next_after_code = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::CodeIndexing,
        CycleDirection::Down,
    );
    assert_eq!(next_after_code, SettingsSection::CloudEnvironments);
}

#[test]
fn arrow_down_collapsed_umbrella_respects_search_filter() {
    let nav_items = realistic_nav_items();
    // Search filter hides WarpAgent and AgentProfiles so the first visible AI
    // subpage is AgentMCPServers.
    let is_visible = |section: SettingsSection| {
        !matches!(
            section,
            SettingsSection::WarpAgent | SettingsSection::AgentProfiles
        )
    };
    let stops = build_nav_stops(&nav_items, is_visible);

    // From Account, Down should land on AgentMCPServers (first visible
    // subpage of the still-collapsed Agents umbrella), not on WarpAgent /
    // AgentProfiles.
    let next = simulate_cycle(
        &nav_items,
        &stops,
        SettingsSection::Account,
        CycleDirection::Down,
    );
    assert_eq!(next, SettingsSection::AgentMCPServers);
}
