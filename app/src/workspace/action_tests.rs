use super::WorkspaceAction;
use crate::pane_group::TerminalPaneId;
use crate::workspace::action::{BranchTarget, WorktreeOpenTarget};
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

/// WorkspaceAction itself is not Serialize/Deserialize, so round-trip tests
/// exercise the supporting types (BranchTarget, WorktreeOpenTarget) directly
/// and verify should_save_app_state_on_action for all four new variants.

#[test]
fn new_worktree_round_trip() {
    // Round-trip the supporting types.
    let branch = BranchTarget::Existing("feature/a".to_string());
    let json = serde_json::to_string(&branch).expect("serialize BranchTarget");
    let back: BranchTarget = serde_json::from_str(&json).expect("deserialize BranchTarget");
    assert_eq!(branch, back);

    let target = WorktreeOpenTarget::NewTab;
    let json = serde_json::to_string(&target).expect("serialize WorktreeOpenTarget");
    let back: WorktreeOpenTarget = serde_json::from_str(&json).expect("deserialize WorktreeOpenTarget");
    assert_eq!(target, back);

    // Verify save-state opt-in for NewWorktreeFromBranch.
    assert!(WorkspaceAction::NewWorktreeFromBranch {
        branch: BranchTarget::Existing("feature/a".to_string()),
        open_in: WorktreeOpenTarget::NewTab,
    }
    .should_save_app_state_on_action());
}

#[test]
fn remove_worktree_round_trip() {
    // Round-trip BranchTarget::CreateFromHead variant.
    let branch = BranchTarget::CreateFromHead("new-branch".to_string());
    let json = serde_json::to_string(&branch).expect("serialize BranchTarget::CreateFromHead");
    let back: BranchTarget = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(branch, back);

    // Verify save-state opt-in for RemoveWorktree.
    assert!(WorkspaceAction::RemoveWorktree {
        worktree_path: std::path::PathBuf::from("/tmp/feature-a"),
        force: false,
    }
    .should_save_app_state_on_action());
}

#[test]
fn open_and_prune_round_trip() {
    // Verify save-state opt-in for OpenWorktreeInTab and PruneWorktree.
    assert!(WorkspaceAction::OpenWorktreeInTab {
        worktree_path: std::path::PathBuf::from("/tmp/feature-a"),
    }
    .should_save_app_state_on_action());

    assert!(WorkspaceAction::PruneWorktree {
        worktree_path: std::path::PathBuf::from("/tmp/gone"),
    }
    .should_save_app_state_on_action());
}
