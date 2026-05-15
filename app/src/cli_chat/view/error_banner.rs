//! Error banner rendered above the transcript when too many events are
//! skipped (likely due to a plugin version mismatch).
//!
//! The banner appears once `ChatModel::skipped_event_count()` reaches
//! [`SKIPPED_THRESHOLD`].

use warpui::elements::{Container, Element, Text};
use warpui::fonts::FamilyId;

use crate::cli_chat::strings;

/// Minimum number of skipped events before showing the banner.
pub const SKIPPED_THRESHOLD: u64 = 3;

/// Render the incompatible-plugin warning banner.
pub fn render(font_family: FamilyId, font_size: f32) -> Box<dyn Element> {
    Container::new(
        Text::new(strings::ERROR_INCOMPATIBLE_PLUGIN, font_family, font_size).finish(),
    )
    .with_uniform_padding(8.0)
    .finish()
}
