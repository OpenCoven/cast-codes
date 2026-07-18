//! Merged conversation-list sidebar for the unified agent panel.
//!
//! Renders every [`ChatConversation`] (both CLI and daemon backends) sorted by
//! recency, each row carrying a backend badge. Clicking a row dispatches
//! [`WorkspaceAction::OpenChatSession`] (shared with the legacy panel).

use std::sync::{Arc, Mutex};

use warpui::elements::{
    ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Hoverable, MainAxisSize,
    MouseState, ParentElement, Text,
};
use warpui::fonts::FamilyId;
use warpui::platform::Cursor;
use warpui::{AppContext, SingletonEntity};

use crate::agent_panel::strings;
use crate::agent_panel::AgentPanelView;
use crate::appearance::Appearance;
use crate::cli_chat::conversation::{ChatConversation, ConversationBackend};
use crate::workspace::WorkspaceAction;

/// Width of the conversation-list sidebar (in logical pixels).
const LIST_WIDTH: f32 = 200.0;

/// Short badge label for a conversation's backend.
pub fn backend_badge(backend: &ConversationBackend) -> &'static str {
    match backend {
        ConversationBackend::Cli(_) => strings::BADGE_CLI,
        ConversationBackend::Daemon { .. } => strings::BADGE_DAEMON,
    }
}

/// Render the merged conversation-list sidebar column.
pub fn render_list(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    let chat = view.chat_model.as_ref(app);
    let conversations = chat.conversations_sorted_by_recency();

    let mut col = Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch);

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

/// Render a single conversation row: backend badge + title + backend name. The
/// whole row is clickable and dispatches `WorkspaceAction::OpenChatSession`.
fn render_conversation_item(
    conv: &ChatConversation,
    font_family: FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    let badge = backend_badge(&conv.backend).to_string();
    let title = if conv.title.is_empty() {
        format!("(untitled) \u{2014} {}", conv.backend.display_name())
    } else {
        conv.title.clone()
    };
    let subtitle = conv.backend.display_name();

    let session_id = conv.session_id.clone();
    let mouse_state = Arc::new(Mutex::new(MouseState::default()));

    Hoverable::new(mouse_state, move |_| {
        let title_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(Text::new(badge.clone(), font_family, font_size - 2.0).finish())
                    .with_margin_right(6.0)
                    .finish(),
            )
            .with_child(Text::new(title.clone(), font_family, font_size).finish())
            .finish();

        let row = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_child(title_row)
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
