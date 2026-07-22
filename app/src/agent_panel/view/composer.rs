//! Composer shell for the unified agent panel.
//!
//! In 2b the composer is active only for a live CLI conversation (submit routes
//! to the terminal PTY via `WorkspaceAction::SubmitAgentPrompt`);
//! daemon and unbound conversations render a disabled placeholder. Per-backend
//! routing (daemon send) arrives in 2c.

use warpui::elements::{Container, Element, Text};
use warpui::presenter::ChildView;
use warpui::{AppContext, SingletonEntity};

use crate::agent_panel::strings;
use crate::agent_panel::AgentPanelView;
use crate::appearance::Appearance;
use crate::cli_chat::conversation::ConversationBinding;

/// Padding around the composer area.
const COMPOSER_PADDING: f32 = 8.0;

/// Whether the composer accepts input for this binding. A live CLI conversation
/// always can (its terminal is present); a daemon conversation can when the
/// cast_agent runtime is reachable.
pub fn composer_is_active_for(binding: &ConversationBinding, daemon_available: bool) -> bool {
    match binding {
        ConversationBinding::Live { .. } => true,
        ConversationBinding::LiveDaemon { .. } => daemon_available,
        ConversationBinding::Past { .. } | ConversationBinding::None => false,
    }
}

/// Feature-independent projection of the cast_agent handshake outcome, so
/// placeholder selection stays pure and testable without a daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonLink {
    /// Handshake succeeded — requests may proceed.
    Ready,
    /// Daemon unreachable (or the cast-agent feature/runtime is absent).
    Offline,
    /// Daemon answered but speaks a different `coven.daemon.v1` version.
    /// Only reachable when the cast-agent handshake runs.
    #[cfg_attr(not(feature = "cast-agent"), allow(dead_code))]
    Incompatible,
}

/// Crate-visible so integration-test helpers can read the same projection
/// the composer renders from.
pub(crate) fn current_daemon_link() -> DaemonLink {
    #[cfg(feature = "cast-agent")]
    {
        use ::ai::cast_agent::ConnectionState;
        match ::ai::cast_agent::connection_state() {
            ConnectionState::Ready(_) => DaemonLink::Ready,
            ConnectionState::Incompatible { .. } => DaemonLink::Incompatible,
            // No probe yet reads the same as down: input must stay closed
            // and "start the daemon" is still the right user action.
            ConnectionState::Unknown | ConnectionState::Unreachable => DaemonLink::Offline,
        }
    }
    #[cfg(not(feature = "cast-agent"))]
    {
        DaemonLink::Offline
    }
}

/// Placeholder text for an inactive composer. Daemon conversations get an
/// action-oriented hint keyed off the handshake outcome; everything else
/// keeps the generic "select a conversation" copy.
pub fn inactive_placeholder_for(binding: &ConversationBinding, link: DaemonLink) -> &'static str {
    match (binding, link) {
        (ConversationBinding::LiveDaemon { .. }, DaemonLink::Offline) => {
            strings::COMPOSER_PLACEHOLDER_DAEMON_OFFLINE
        }
        (ConversationBinding::LiveDaemon { .. }, DaemonLink::Incompatible) => {
            strings::COMPOSER_PLACEHOLDER_DAEMON_INCOMPATIBLE
        }
        _ => strings::COMPOSER_PLACEHOLDER_INACTIVE,
    }
}

pub fn render_composer(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(app);
    let link = current_daemon_link();

    if composer_is_active_for(chat.binding(), link == DaemonLink::Ready) {
        render_active_composer(view)
    } else {
        let placeholder = inactive_placeholder_for(chat.binding(), link);
        render_inactive_placeholder(app, placeholder)
    }
}

fn render_active_composer(view: &AgentPanelView) -> Box<dyn Element> {
    Container::new(ChildView::new(&view.composer_input).finish())
        .with_uniform_padding(COMPOSER_PADDING)
        .finish()
}

fn render_inactive_placeholder(app: &AppContext, placeholder: &'static str) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    Container::new(Text::new(placeholder, font_family, font_size).finish())
        .with_uniform_padding(COMPOSER_PADDING)
        .finish()
}
