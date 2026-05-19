//! Transcript layout for the CastCodes chat panel.
//!
//! Walks the bound [`ChatConversation`] and produces one row per
//! [`ChatEntry`] via `message_bubble`. When no conversation is bound
//! (the panel just opened, or no CLI session has emitted events yet)
//! we render the appropriate empty state so the panel is never blank.
//!
//! When the model's `skipped_event_count` reaches the threshold, an
//! error banner is rendered above the transcript (see `error_banner`).
//!
//! The element-builder pattern (`Flex::column`, `MainAxisSize`) is
//! mirrored from `app/src/workspace/view/conversation_list/view.rs`
//! and `app/src/ai_assistant/panel.rs`.

use warpui::elements::{Element, Flex, MainAxisSize, ParentElement};
use warpui::{AppContext, SingletonEntity};

use crate::appearance::Appearance;
use crate::cli_chat::conversation::{ChatConversation, ConversationBinding};
use crate::cli_chat::entry::{ChatEntry, ChatEntryKind};
use crate::cli_chat::view::empty_state::{self, EmptyKind};
use crate::cli_chat::view::error_banner;
use crate::cli_chat::view::message_bubble;
use crate::cli_chat::view::ChatPanelView;

pub fn render_panel(view: &ChatPanelView, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    let chat = view.chat_model.as_ref(app);

    let conversation = match chat.binding() {
        ConversationBinding::Live { session_id, .. } | ConversationBinding::Past { session_id } => {
            chat.conversation(session_id)
        }
        ConversationBinding::None => None,
    };

    let mut col = Flex::column().with_main_axis_size(MainAxisSize::Max);

    // Show an error banner when enough events have been skipped to suggest
    // a plugin incompatibility.
    if chat.skipped_event_count() >= error_banner::SKIPPED_THRESHOLD {
        col = col.with_child(error_banner::render(font_family, font_size));
    }

    let body: Box<dyn Element> = match conversation {
        Some(conv) => render_transcript(conv, font_family, font_size),
        None => {
            let has_conversations = !chat.conversations_sorted_by_recency().is_empty();
            if has_conversations {
                // Conversations exist but none is bound — show a brief
                // placeholder. The user can select one from the sidebar.
                render_empty_placeholder(font_family, font_size)
            } else {
                // No conversations at all — render the full empty state.
                empty_state::render(EmptyKind::NoHistory, font_family, font_size)
            }
        }
    };

    col.with_child(body).finish()
}

fn render_transcript(
    conv: &ChatConversation,
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    let mut col = Flex::column().with_main_axis_size(MainAxisSize::Min);
    for entry in &conv.entries {
        col = col.with_child(render_entry(entry, font_family, font_size));
    }
    col.finish()
}

fn render_entry(
    entry: &ChatEntry,
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    match &entry.kind {
        ChatEntryKind::UserPrompt { text } => {
            message_bubble::user_bubble(text, font_family, font_size)
        }
        ChatEntryKind::AssistantResponse { text } => {
            message_bubble::assistant_bubble(text, font_family, font_size)
        }
        ChatEntryKind::ToolCall {
            tool_name,
            input_preview,
        } => message_bubble::tool_placeholder(
            tool_name,
            input_preview.as_deref(),
            font_family,
            font_size,
        ),
        ChatEntryKind::PermissionRequest { summary, .. } => {
            message_bubble::permission_placeholder(summary, font_family, font_size)
        }
        ChatEntryKind::Info { summary, .. } => {
            message_bubble::info_line(summary.as_deref(), font_family, font_size)
        }
        ChatEntryKind::Stop { .. } => message_bubble::stop_marker(font_family, font_size),
        ChatEntryKind::PermissionReplied { .. } | ChatEntryKind::Raw { .. } => {
            message_bubble::info_line(Some("(internal event)"), font_family, font_size)
        }
    }
}

fn render_empty_placeholder(
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    use crate::cli_chat::strings::EMPTY_NO_HISTORY_TITLE;
    message_bubble::info_line(Some(EMPTY_NO_HISTORY_TITLE), font_family, font_size)
}
