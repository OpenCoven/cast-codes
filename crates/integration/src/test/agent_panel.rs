//! Unified agent panel: the Coven-daemon composer placeholder flow.
//!
//! Covers the user-visible outcome of the `coven.daemon.v1` handshake: with
//! no daemon reachable (the integration build runs without the `cast-agent`
//! feature, which reads the same as an offline daemon), opening a daemon
//! conversation must open the panel, bind it sendable-side (`LiveDaemon`),
//! and show the action-oriented placeholder naming the real fix —
//! `coven daemon start` — instead of the generic "select a conversation"
//! copy.

use warp::integration_testing::agent_panel::{
    agent_panel_is_open, bound_daemon_session, inactive_composer_placeholder, open_chat_session,
    seed_daemon_conversation, COMPOSER_PLACEHOLDER_DAEMON_OFFLINE,
};
use warp::integration_testing::step::new_step_with_default_assertions;
use warp::integration_testing::terminal::wait_until_bootstrapped_single_pane_for_tab;
use warpui::async_assert;

use super::{new_builder, Builder};

const SESSION_ID: &str = "daemon-session-1";

/// Opening a Coven-daemon conversation while the daemon is offline shows
/// the composer hint that names the recovery command.
pub fn test_daemon_conversation_composer_names_daemon_fix() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            new_step_with_default_assertions("Seed and open a Coven-daemon conversation")
                .with_action(|app, window_id, _| {
                    seed_daemon_conversation(app, SESSION_ID, "Fix the flaky tests");
                    open_chat_session(app, window_id, SESSION_ID);
                })
                .add_named_assertion("agent panel opens with the daemon conversation bound", {
                    |app, window_id| {
                        async_assert!(
                            agent_panel_is_open(app, window_id)
                                && bound_daemon_session(app).as_deref() == Some(SESSION_ID),
                            "OpenChatSession must open the unified agent panel and bind \
                             the daemon conversation as LiveDaemon"
                        )
                    }
                })
                .add_named_assertion("composer placeholder names `coven daemon start`", {
                    |app, _| {
                        let placeholder = inactive_composer_placeholder(app);
                        async_assert!(
                            placeholder == COMPOSER_PLACEHOLDER_DAEMON_OFFLINE
                                && placeholder.contains("coven daemon start"),
                            "an offline daemon must surface the actionable placeholder, \
                             got {placeholder:?}"
                        )
                    }
                }),
        )
}
