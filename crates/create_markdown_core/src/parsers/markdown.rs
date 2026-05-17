//! Block-level markdown parser — converts a token stream to a `Vec<Block>`.
//!
//! Stub for now; filled in by a subsequent commit.

use crate::types::{Block, Document, MarkdownParseOptions};

/// Parses markdown to a flat vec of blocks. Stub — currently returns an
/// empty vec; the full implementation lands with the inline parser.
pub fn markdown_to_blocks(_markdown: &str, _options: &MarkdownParseOptions) -> Vec<Block> {
    Vec::new()
}

/// Parses markdown to a [`Document`]. Stub.
pub fn markdown_to_document(markdown: &str, options: &MarkdownParseOptions) -> Document {
    let blocks = markdown_to_blocks(markdown, options);
    crate::document::create_document(&blocks, crate::types::DocumentOptions::default())
}

/// Convenience alias for [`markdown_to_blocks`]. Stub.
pub fn parse(markdown: &str) -> Vec<Block> {
    markdown_to_blocks(markdown, &MarkdownParseOptions::default())
}

/// Convenience alias for [`crate::parsers::inline::parse_inline`].
pub fn parse_inline_content(text: &str) -> Vec<crate::types::TextSpan> {
    crate::parsers::inline::parse_inline(text)
}
