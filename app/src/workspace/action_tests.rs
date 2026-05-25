use super::WorkspaceAction;
use crate::pane_group::TerminalPaneId;
use crate::workspace::tab_settings::{
    VerticalTabsDisplayGranularity, VerticalTabsPrimaryInfo, VerticalTabsTabItemMode,
    VerticalTabsViewMode,
};
use crate::workspace::PaneViewLocator;
use warpui::EntityId;

#[test]
fn vertical_tabs_view_mode_change_does_not_save_workspace_state() {
    assert!(
        !WorkspaceAction::SetVerticalTabsViewMode(VerticalTabsViewMode::Compact)
            .should_save_app_state_on_action()
    );
}

#[test]
fn vertical_tabs_panel_toggle_still_saves_workspace_state() {
    assert!(WorkspaceAction::ToggleVerticalTabsPanel.should_save_app_state_on_action());
}

#[test]
fn settings_popup_toggle_does_not_save_workspace_state() {
    assert!(!WorkspaceAction::ToggleVerticalTabsSettingsPopup.should_save_app_state_on_action());
}

#[test]
fn display_granularity_change_does_not_save_workspace_state() {
    assert!(!WorkspaceAction::SetVerticalTabsDisplayGranularity(
        VerticalTabsDisplayGranularity::Panes
    )
    .should_save_app_state_on_action());
    assert!(!WorkspaceAction::SetVerticalTabsDisplayGranularity(
        VerticalTabsDisplayGranularity::Tabs
    )
    .should_save_app_state_on_action());
}

#[test]
fn tab_item_mode_change_does_not_save_workspace_state() {
    assert!(
        !WorkspaceAction::SetVerticalTabsTabItemMode(VerticalTabsTabItemMode::FocusedSession)
            .should_save_app_state_on_action()
    );
    assert!(
        !WorkspaceAction::SetVerticalTabsTabItemMode(VerticalTabsTabItemMode::Summary)
            .should_save_app_state_on_action()
    );
}

#[test]
fn primary_info_change_does_not_save_workspace_state() {
    assert!(
        !WorkspaceAction::SetVerticalTabsPrimaryInfo(VerticalTabsPrimaryInfo::Command)
            .should_save_app_state_on_action()
    );
    assert!(!WorkspaceAction::SetVerticalTabsPrimaryInfo(
        VerticalTabsPrimaryInfo::WorkingDirectory
    )
    .should_save_app_state_on_action());
    assert!(
        !WorkspaceAction::SetVerticalTabsPrimaryInfo(VerticalTabsPrimaryInfo::Branch)
            .should_save_app_state_on_action()
    );
}

#[test]
fn pane_name_actions_save_workspace_state() {
    let locator = PaneViewLocator {
        pane_group_id: EntityId::new(),
        pane_id: TerminalPaneId::dummy_terminal_pane_id().into(),
    };

    assert!(WorkspaceAction::RenamePane(locator).should_save_app_state_on_action());
    assert!(WorkspaceAction::ResetPaneName(locator).should_save_app_state_on_action());
    // GH-9351: the keyboard-bindable variant must persist app state on the
    // same conditions as the locator-based one, since both ultimately drive
    // `rename_pane` which mutates `pane_configuration`.
    assert!(WorkspaceAction::RenameActivePane.should_save_app_state_on_action());
}

// --- Worktree Manager action tests ---

#[test]
fn remove_worktree_round_trip() {
    // Verify save-state opt-in for RemoveWorktree and PruneWorktree.
    assert!(WorkspaceAction::RemoveWorktree {
        worktree_path: std::path::PathBuf::from("/tmp/feature-a"),
        force: false,
    }
    .should_save_app_state_on_action());

    assert!(WorkspaceAction::PruneWorktree {
        worktree_path: std::path::PathBuf::from("/tmp/gone"),
    }
    .should_save_app_state_on_action());
}

#[test]
fn sentinel_variants_do_not_save_workspace_state() {
    // OpenWorktreePicker and OpenWorktreeRemoveStub are UI-only triggers —
    // they must NOT cause a workspace-state save.
    assert!(!WorkspaceAction::OpenWorktreePicker.should_save_app_state_on_action());
    assert!(!WorkspaceAction::OpenWorktreeRemoveStub.should_save_app_state_on_action());
}
