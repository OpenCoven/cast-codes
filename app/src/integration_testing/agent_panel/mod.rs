//! Integration-test helpers for the unified agent panel.
//!
//! The panel's modules (`crate::agent_panel`, `crate::cli_chat`) are private
//! to the app crate, so integration tests drive them through these helpers.
//! Each helper goes through the production path (the singleton [`ChatModel`],
//! typed [`WorkspaceAction`] dispatch) rather than test-only backdoors.

use chrono::Utc;
use warpui::{App, SingletonEntity, WindowId};

use crate::agent_panel::view::composer;
use crate::cli_chat::conversation::ConversationBinding;
use crate::cli_chat::model::ChatModel;
use crate::integration_testing::view_getters::workspace_view;
use crate::workspace::WorkspaceAction;

/// The composer placeholder shown for a daemon conversation when the Coven
/// daemon is offline (or the cast-agent runtime is absent). Re-exported so
/// integration tests can assert on the exact user-visible copy.
pub use crate::agent_panel::strings::COMPOSER_PLACEHOLDER_DAEMON_OFFLINE;

/// Upsert a Coven-daemon conversation into the singleton [`ChatModel`], the
/// same way the live daemon source publishes sessions.
pub fn seed_daemon_conversation(app: &mut App, session_id: &str, title: &str) {
    ChatModel::handle(app).update(app, |model, ctx| {
        model.refresh_daemon_conversations(
            [(session_id.to_string(), title.to_string())],
            "coven-code",
            Utc::now(),
            ctx,
        );
    });
}

/// Dispatch [`WorkspaceAction::OpenChatSession`] for `session_id` — the
/// production routing that binds a daemon conversation (`LiveDaemon`) and
/// opens the unified agent panel.
pub fn open_chat_session(app: &mut App, window_id: WindowId, session_id: &str) {
    let workspace = workspace_view(app, window_id);
    let action = WorkspaceAction::OpenChatSession {
        session_id: session_id.to_string(),
    };
    app.update(|ctx| {
        ctx.dispatch_typed_action_for_view(window_id, workspace.id(), &action);
    });
}

/// Whether the unified agent panel is open in this window's workspace.
pub fn agent_panel_is_open(app: &App, window_id: WindowId) -> bool {
    workspace_view(app, window_id).read(app, |workspace, _| {
        workspace.current_workspace_state.is_agent_panel_open
    })
}

/// The daemon session id the chat model is currently bound to, if the
/// binding is `LiveDaemon`.
pub fn bound_daemon_session(app: &App) -> Option<String> {
    ChatModel::handle(app).read(app, |model, _| match model.binding() {
        ConversationBinding::LiveDaemon { session_id } => Some(session_id.clone()),
        _ => None,
    })
}

/// The placeholder copy an inactive composer renders for the current
/// binding + daemon handshake outcome — the exact selection
/// `render_composer` makes.
pub fn inactive_composer_placeholder(app: &App) -> &'static str {
    let link = composer::current_daemon_link();
    ChatModel::handle(app).read(app, |model, _| {
        composer::inactive_placeholder_for(model.binding(), link)
    })
}
