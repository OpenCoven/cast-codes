//! Transcript pane for the unified agent panel. Renders the bound
//! conversation's entries via the shared `agent_transcript` renderer. When no
//! conversation is bound, shows a brief info line so the pane is never blank.

use warpui::elements::{Element, Flex, MainAxisSize, ParentElement};
use warpui::{AppContext, SingletonEntity};

use crate::agent_panel::AgentPanelView;
use crate::agent_transcript::view::message_bubble;
use crate::agent_transcript::view::transcript::render_entries;
use crate::appearance::Appearance;
use crate::cli_chat::conversation::{ChatConversation, ConversationBinding};

/// Number of skipped events after which we surface a plugin-incompatibility
/// hint above the transcript (mirrors the legacy panel's threshold).
const SKIPPED_THRESHOLD: u64 = 3;

pub fn render_panel(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    let chat = view.chat_model.as_ref(app);

    let conversation = match chat.binding() {
        ConversationBinding::Live { session_id, .. } | ConversationBinding::Past { session_id }
        | ConversationBinding::LiveDaemon { session_id } => {
            chat.conversation(session_id)
        }
        ConversationBinding::None => None,
    };

    let mut col = Flex::column().with_main_axis_size(MainAxisSize::Max);

    if chat.skipped_event_count() >= SKIPPED_THRESHOLD {
        col = col.with_child(message_bubble::info_line(
            Some("Some agent events could not be parsed — the CLI plugin may be out of date."),
            font_family,
            font_size,
        ));
    }

    let body: Box<dyn Element> = match conversation {
        Some(conv) => render_transcript(conv, font_family, font_size),
        None => {
            let label = if chat.conversations_sorted_by_recency().is_empty() {
                "No conversations yet"
            } else {
                "Select a conversation to view its transcript."
            };
            message_bubble::info_line(Some(label), font_family, font_size)
        }
    };

    col.with_child(body).finish()
}

fn render_transcript(
    conv: &ChatConversation,
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    render_entries(&conv.entries, font_family, font_size)
}
