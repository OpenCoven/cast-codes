//! User-visible strings for the chat panel.
//! Centralized so `./script/check_rebrand` only needs to audit one file.

pub const PANEL_TITLE: &str = "Chat";
pub const TOGGLE_MENU_ITEM: &str = "Toggle Chat Panel";

pub const EMPTY_NO_CLI_TITLE: &str = "No supported CLI detected";
pub const EMPTY_NO_CLI_BODY: &str =
    "Install one of the supported CLIs to start chatting. \
     The chat panel renders the conversation from a CLI session running in any CastCodes terminal.";

pub const EMPTY_NO_PLUGIN_TITLE: &str = "Plugin required";
pub const EMPTY_NO_PLUGIN_BODY: &str =
    "The chat panel renders structured events emitted by the vendor plugin for this CLI. \
     See the vendor's documentation to install the plugin.";

pub const EMPTY_NO_HISTORY_TITLE: &str = "No conversations yet";
pub const EMPTY_NO_HISTORY_BODY: &str =
    "Run a supported CLI in a terminal to start a conversation. It will appear here automatically.";

pub const COMPOSER_PLACEHOLDER_ACTIVE: &str = "Message the running CLI agent…";
pub const COMPOSER_PLACEHOLDER_INACTIVE: &str = "Run a CLI agent in a terminal to start chatting.";

pub const TRANSCRIPT_TURN_COMPLETE: &str = "Turn complete";
pub const TRANSCRIPT_SESSION_ENDED: &str = "Session ended";

pub const ERROR_INCOMPATIBLE_PLUGIN: &str =
    "Plugin version may be incompatible — some events were skipped.";
