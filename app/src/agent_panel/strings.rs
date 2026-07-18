//! User-facing strings for the unified agent panel. Fork-local; no Warp
//! naming (guarded by check_rebrand).

pub const PANEL_TITLE: &str = "Agent";
pub const TOGGLE_MENU_ITEM: &str = "Toggle Agent Panel";
pub const NEW_CHAT_LABEL: &str = "New chat";

pub const COMPOSER_PLACEHOLDER_ACTIVE: &str = "Message the running agent…";
pub const COMPOSER_PLACEHOLDER_INACTIVE: &str =
    "Select a live CLI conversation to send input, or run a CLI agent in a terminal.";

/// Row badge labels distinguishing the two backends in the merged list.
pub const BADGE_CLI: &str = "CLI";
pub const BADGE_DAEMON: &str = "Coven";
