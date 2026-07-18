//! Composer shell for the unified agent panel.
//!
//! In 2b the composer is active only for a live CLI conversation (submit routes
//! to the terminal PTY via the shared `WorkspaceAction::SubmitChatPrompt`);
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

pub fn render_composer(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(app);
    // In 2b, a `Live` binding is always a live CLI session (daemon conversations
    // use the `LiveDaemon` binding introduced in 2c). So `Live` == sendable CLI.
    let is_live_cli = matches!(chat.binding(), ConversationBinding::Live { .. });

    if is_live_cli {
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
