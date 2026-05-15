//! Conversation-list sidebar for the CastCodes chat panel (Phase 4).
//!
//! Renders a vertical list of all [`ChatConversation`]s, sorted by recency.
//! Clicking a row dispatches [`WorkspaceAction::OpenChatSession`] to bind
//! the transcript pane to that past session.

use std::sync::{Arc, Mutex};

use warpui::elements::{
    ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Hoverable, MainAxisSize,
    MouseState, ParentElement, Text,
};
use warpui::fonts::FamilyId;
use warpui::platform::Cursor;
use warpui::{AppContext, SingletonEntity};

use crate::appearance::Appearance;
use crate::cli_chat::conversation::ChatConversation;
use crate::cli_chat::view::ChatPanelView;
use crate::workspace::WorkspaceAction;

/// Width of the conversation-list sidebar (in logical pixels).
const LIST_WIDTH: f32 = 200.0;

/// Render the conversation-list sidebar column.
///
/// The caller embeds the returned element in the left column of the
/// chat panel's `Flex::row` layout (see `ChatPanelView::render`).
pub fn render_list(view: &ChatPanelView, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    let chat = view.chat_model.as_ref(app);
    let conversations = chat.conversations_sorted_by_recency();

    let mut col = Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch);

    // Header
    col = col.with_child(
        Container::new(Text::new("Conversations", font_family, font_size + 1.0).finish())
            .with_uniform_padding(6.0)
            .finish(),
    );

    if conversations.is_empty() {
        col = col.with_child(
            Container::new(
                Text::new("No conversations yet", font_family, font_size - 1.0).finish(),
            )
            .with_uniform_padding(6.0)
            .finish(),
        );
    } else {
        for conv in &conversations {
            col = col.with_child(render_conversation_item(conv, font_family, font_size));
        }
    }

    ConstrainedBox::new(col.finish())
        .with_width(LIST_WIDTH)
        .finish()
}

/// Render a single conversation row.
///
/// Shows the title (or a fallback) and the agent display name. The
/// entire row is clickable and dispatches `WorkspaceAction::OpenChatSession`.
fn render_conversation_item(
    conv: &ChatConversation,
    font_family: FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    let title = if conv.title.is_empty() {
        format!("(untitled) \u{2014} {}", conv.agent.display_name())
    } else {
        conv.title.clone()
    };

    let subtitle = conv.agent.display_name().to_string();

    let session_id = conv.session_id.clone();
    let mouse_state = Arc::new(Mutex::new(MouseState::default()));

    Hoverable::new(mouse_state, move |_| {
        let row = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_child(Text::new(title.clone(), font_family, font_size).finish())
            .with_child(Text::new(subtitle.clone(), font_family, font_size - 2.0).finish())
            .finish();

        Container::new(row).with_uniform_padding(6.0).finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(WorkspaceAction::OpenChatSession {
            session_id: session_id.clone(),
        });
    })
    .finish()
}
