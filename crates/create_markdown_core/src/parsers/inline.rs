//! Inline markdown parser — converts a single line of markdown text to a
//! `Vec<TextSpan>` with inline styles applied.
//!
//! Stub for now; filled in by a subsequent commit.

use crate::types::TextSpan;
use crate::utils::plain_content;

/// Parses inline markdown into a vec of [`TextSpan`]s. The stub treats the
/// input as plain text — call sites get something reasonable until the full
/// inline parser lands.
pub fn parse_inline(text: &str) -> Vec<TextSpan> {
    plain_content(text)
}
