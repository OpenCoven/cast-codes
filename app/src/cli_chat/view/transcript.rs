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

use crate::agent_transcript::view::message_bubble;
use crate::agent_transcript::view::transcript::render_entries;
use crate::appearance::Appearance;
use crate::cli_chat::conversation::{ChatConversation, ConversationBinding};
use crate::cli_chat::view::empty_state::{self, EmptyKind};
use crate::cli_chat::view::error_banner;
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
    // Neutral rendering lives in `agent_transcript`; the panel just supplies
    // the bound conversation's entries.
    render_entries(&conv.entries, font_family, font_size)
}

fn render_empty_placeholder(
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    use crate::cli_chat::strings::EMPTY_NO_HISTORY_TITLE;
    message_bubble::info_line(Some(EMPTY_NO_HISTORY_TITLE), font_family, font_size)
}
