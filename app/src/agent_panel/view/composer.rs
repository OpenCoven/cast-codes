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

pub fn render_composer(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(app);
    #[cfg(feature = "cast-agent")]
    let daemon_available = ::ai::cast_agent::is_available();
    #[cfg(not(feature = "cast-agent"))]
    let daemon_available = false;

    if composer_is_active_for(chat.binding(), daemon_available) {
        render_active_composer(view)
    } else {
        render_inactive_placeholder(app)
    }
}

fn render_active_composer(view: &AgentPanelView) -> Box<dyn Element> {
    Container::new(ChildView::new(&view.composer_input).finish())
        .with_uniform_padding(COMPOSER_PADDING)
        .finish()
}

fn render_inactive_placeholder(app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    Container::new(
        Text::new(
            strings::COMPOSER_PLACEHOLDER_INACTIVE,
            font_family,
            font_size,
        )
        .finish(),
    )
    .with_uniform_padding(COMPOSER_PADDING)
    .finish()
}
