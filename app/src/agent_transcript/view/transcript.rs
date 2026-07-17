//! Neutral transcript rendering: turn a list of [`ChatEntry`]s into a column
//! of rows via `message_bubble`. Backend-agnostic — both the CLI panel and the
//! Coven agent panel render through here. Callers that own a
//! backend-specific conversation type pass its entry slice.

use warpui::elements::{Element, Flex, MainAxisSize, ParentElement};
use warpui::fonts::FamilyId;

use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind};
use crate::agent_transcript::view::message_bubble;

/// Render an ordered slice of entries as a column of transcript rows.
pub fn render_entries(
    entries: &[ChatEntry],
    font_family: FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    let mut col = Flex::column().with_main_axis_size(MainAxisSize::Min);
    for entry in entries {
        col = col.with_child(render_entry(entry, font_family, font_size));
    }
    col.finish()
}

/// Render one entry to a row element. Exhaustive over `ChatEntryKind`.
pub fn render_entry(entry: &ChatEntry, font_family: FamilyId, font_size: f32) -> Box<dyn Element> {
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
        ChatEntryKind::ToolResult {
            tool_name,
            output_preview,
        } => message_bubble::tool_placeholder(
            tool_name,
            output_preview.as_deref(),
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
