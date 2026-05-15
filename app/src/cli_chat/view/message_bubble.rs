//! Minimal v1 message-bubble primitives for the CastCodes chat panel.
//!
//! These render one row per `ChatEntry`. Styling is intentionally
//! sparse — Phase 7 (`empty / error / settings polish`) introduces
//! colors, avatars, borders, and richer typography. The goal in v1 is
//! structural correctness so events visibly land in the transcript.
//!
//! The element-builder APIs (`Container`, `Text`, `with_uniform_padding`)
//! mirror the patterns used by
//! `app/src/workspace/view/conversation_list/view.rs` and
//! `app/src/ai_assistant/panel.rs`, the closest panel-style sibling views
//! in this repo.

use warpui::elements::{Container, Element, Text};
use warpui::fonts::FamilyId;

/// Convenience: build a single padded text row with the given content.
fn text_row(text: impl Into<std::borrow::Cow<'static, str>>, family: FamilyId, size: f32, pad: f32) -> Box<dyn Element> {
    Container::new(Text::new(text, family, size).finish())
        .with_uniform_padding(pad)
        .finish()
}

pub fn user_bubble(text: &str, family: FamilyId, font_size: f32) -> Box<dyn Element> {
    // v1: prefix with "You: " so the role is visible without styling.
    text_row(format!("You: {}", text), family, font_size, 8.0)
}

pub fn assistant_bubble(text: &str, family: FamilyId, font_size: f32) -> Box<dyn Element> {
    text_row(format!("Assistant: {}", text), family, font_size, 8.0)
}

pub fn tool_placeholder(
    tool_name: &str,
    input_preview: Option<&str>,
    family: FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    let label = match input_preview {
        Some(p) => format!("[tool] {}({})", tool_name, p),
        None => format!("[tool] {}()", tool_name),
    };
    text_row(label, family, font_size, 6.0)
}

pub fn permission_placeholder(
    summary: &str,
    family: FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    text_row(format!("[permission] {}", summary), family, font_size, 6.0)
}

pub fn info_line(text: Option<&str>, family: FamilyId, font_size: f32) -> Box<dyn Element> {
    let label = text.unwrap_or("").to_string();
    text_row(label, family, font_size, 4.0)
}

pub fn stop_marker(family: FamilyId, font_size: f32) -> Box<dyn Element> {
    info_line(Some(crate::cli_chat::strings::TRANSCRIPT_TURN_COMPLETE), family, font_size)
}
