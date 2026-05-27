//! Empty-state views for the CastCodes chat panel.
//!
//! When no conversation is bound and the conversation list is empty, the
//! panel shows a contextual empty state instead of a blank transcript.
//!
//! For v1, we only check whether the conversation list is empty:
//! - Empty list → "No conversations yet" with a hint to run a CLI.
//! - Full CLI / plugin detection is deferred to a follow-up.

use warpui::elements::{Container, Element, Flex, MainAxisSize, ParentElement, Text};
use warpui::fonts::FamilyId;

use crate::cli_chat::strings;

/// Which empty state to display.
#[allow(clippy::enum_variant_names)]
pub(crate) enum EmptyKind {
    /// No conversations at all — invite the user to run a CLI.
    NoHistory,
    /// A CLI is on PATH but the vendor plugin is missing.
    #[allow(dead_code)]
    NoPlugin,
    /// No supported CLI detected on PATH.
    #[allow(dead_code)]
    NoCli,
}

/// Render the appropriate empty-state card.
pub fn render(kind: EmptyKind, font_family: FamilyId, font_size: f32) -> Box<dyn Element> {
    let (title, body) = match kind {
        EmptyKind::NoHistory => (
            strings::EMPTY_NO_HISTORY_TITLE,
            strings::EMPTY_NO_HISTORY_BODY,
        ),
        EmptyKind::NoPlugin => (
            strings::EMPTY_NO_PLUGIN_TITLE,
            strings::EMPTY_NO_PLUGIN_BODY,
        ),
        EmptyKind::NoCli => (strings::EMPTY_NO_CLI_TITLE, strings::EMPTY_NO_CLI_BODY),
    };

    let title_el = Container::new(Text::new(title, font_family, font_size + 2.0).finish())
        .with_uniform_padding(8.0)
        .finish();

    let body_el = Container::new(Text::new(body, font_family, font_size).finish())
        .with_uniform_padding(8.0)
        .finish();

    Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_child(title_el)
        .with_child(body_el)
        .finish()
}
