//! Composer bar for the CastCodes chat panel (Phase 5).
//!
//! When the panel is bound to a live CLI agent session, renders an
//! enabled [`SubmittableTextInput`] — the user types a message and
//! presses Enter (or clicks the send button) to submit it. The submit
//! event is handled in [`super::ChatPanelView`] and dispatched as
//! [`WorkspaceAction::SubmitChatPrompt`], which writes the text to
//! the terminal's PTY via `submit_text_to_cli_agent_pty`.
//!
//! When no live session is bound, the composer renders a static
//! placeholder label so the user knows they need a running CLI agent.

use warpui::elements::{Container, Element, Text};
use warpui::presenter::ChildView;
use warpui::{AppContext, SingletonEntity};

use crate::appearance::Appearance;
use crate::cli_chat::conversation::ConversationBinding;
use crate::cli_chat::strings;
use crate::cli_chat::view::ChatPanelView;

/// Padding around the composer area.
const COMPOSER_PADDING: f32 = 8.0;

/// Render the composer bar at the bottom of the right column.
///
/// When bound to a live session, shows the [`SubmittableTextInput`].
/// Otherwise, shows a disabled placeholder message.
pub fn render_composer(view: &ChatPanelView, app: &AppContext) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(app);
    let is_live = matches!(chat.binding(), ConversationBinding::Live { .. });

    if is_live {
        render_active_composer(view, app)
    } else {
        render_inactive_placeholder(app)
    }
}

/// Renders the live composer with the embedded [`SubmittableTextInput`].
fn render_active_composer(view: &ChatPanelView, _app: &AppContext) -> Box<dyn Element> {
    Container::new(ChildView::new(&view.composer_input).finish())
        .with_uniform_padding(COMPOSER_PADDING)
        .finish()
}

/// Renders a disabled placeholder when no live session is bound.
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
