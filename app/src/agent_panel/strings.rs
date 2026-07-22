//! User-facing strings for the unified agent panel. Fork-local; no Warp
//! naming (guarded by check_rebrand).

pub const PANEL_TITLE: &str = "Agent";
pub const TOGGLE_MENU_ITEM: &str = "Toggle Agent Panel";
pub const NEW_CHAT_LABEL: &str = "New chat";

pub const COMPOSER_PLACEHOLDER_ACTIVE: &str = "Message the running agent…";
pub const COMPOSER_PLACEHOLDER_INACTIVE: &str =
    "Select a live CLI conversation to send input, or run a CLI agent in a terminal.";
/// Daemon conversation selected but the handshake reports the daemon down.
/// Names the real CLI verb (`coven daemon start`) so the fix is one copy-paste.
pub const COMPOSER_PLACEHOLDER_DAEMON_OFFLINE: &str =
    "Coven daemon is offline. Start it with `coven daemon start`.";
/// Daemon answered the handshake but not with this build's API contract —
/// a different version, a missing `apiVersion`, or an unparseable health
/// body. In every case one side needs an update.
pub const COMPOSER_PLACEHOLDER_DAEMON_INCOMPATIBLE: &str =
    "Coven daemon isn't compatible with this version of CastCodes. Update Coven or CastCodes.";

/// Row badge labels distinguishing the two backends in the merged list.
pub const BADGE_CLI: &str = "CLI";
pub const BADGE_DAEMON: &str = "Coven";
