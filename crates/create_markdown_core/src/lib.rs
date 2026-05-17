//! Rust sister port of [`@create-markdown/core`](https://www.npmjs.com/package/@create-markdown/core).
//!
//! Block-based markdown parsing and serialization. Mirrors the JS package's
//! behavior and JSON shape (so a [`Document`] round-trips between Rust and
//! TypeScript) but exposes a Rust-idiomatic snake_case API. See the README
//! for the JS→Rust naming map.
//!
//! Tracks `@create-markdown/core@2.0.3`. Block types, utils, and factories
//! are complete; document operations, parser, and serializer follow.

pub mod blocks;
pub mod document;
pub mod parsers;
pub mod serializers;
pub mod types;
pub mod utils;

pub use parsers::inline::{extract_plain_text, parse_inline};
pub use parsers::markdown::{
    markdown_to_blocks, markdown_to_document, parse, parse_inline_content,
};
pub use parsers::tokenizer::{
    group_tokens, is_code_token, is_list_token, tokenize, Token, TokenMeta, TokenType,
};

/// Convenience: parse markdown into a [`Document`]. Mirrors the JS `fromMarkdown`.
pub fn from_markdown(markdown: &str) -> Document {
    markdown_to_document(markdown, &MarkdownParseOptions::default())
}

/// Convenience: serialize blocks to a markdown string. Mirrors the JS `toMarkdown`.
pub fn to_markdown(blocks: &[Block]) -> String {
    blocks_to_markdown(blocks, &MarkdownSerializeOptions::default())
}

pub use serializers::markdown::{
    blocks_to_markdown, document_to_markdown, serialize_block, serialize_inline_content,
    serialize_span, stringify,
};

pub use document::{
    append_block, append_block_content, clear_blocks, clone_document, create_document,
    empty_document, filter_blocks, find_block, find_blocks_by_type, get_block_at, get_block_count,
    get_block_index, get_first_block, get_last_block, get_meta_field, has_block, insert_block,
    insert_blocks, is_empty, map_blocks, move_block, prepend_block, remove_block, remove_blocks,
    replace_block, set_block_content, set_blocks, set_meta_field, swap_blocks, update_block,
    update_meta, BlockUpdate,
};
pub use blocks::{
    add_children, append_content, block_quote, bold, bullet_list, callout, check_list,
    check_list_item, code, code_block, create_block, danger_callout, divider, h1, h2, h3, h4, h5,
    h6, heading, highlight, image, info_callout, italic, link, note_callout, numbered_list,
    paragraph, prepend_content, set_content, spans, strikethrough, styled, table, text,
    tip_callout, underline, update_props, warning_callout, CheckItem, ImageOptions, IntoContent,
    ListItem,
};
pub use types::{
    Block, BlockProps, BlockType, BlockquoteBlock, BulletChar, BulletListBlock, CalloutBlock,
    CalloutProps, CalloutType, CheckListBlock, CheckListProps, CodeBlockBlock, CodeBlockProps,
    CodeBlockStyle, DividerBlock, Document, DocumentMeta, DocumentOptions, EmphasisChar,
    EmptyProps, HeadingBlock, HeadingProps, HeadingStyle, IdGenerator, ImageBlock, ImageProps,
    InlineStyle, LineEnding, LinkData, MarkdownParseOptions, MarkdownSerializeOptions,
    NumberedListBlock, ParagraphBlock, TableAlignment, TableBlock, TableProps, TextSpan,
};
pub use utils::{
    block_plain_text, convert_line_endings, deep_clone, deep_clone_block, deep_clone_block_with,
    deep_clone_blocks, escape_code_block, escape_markdown, generate_id, generate_id_with_length,
    has_children, has_content, indent, is_valid_block_type, is_valid_heading_level,
    normalize_line_endings, plain_content, plain_span, spans_to_plain_text, trim_blank_lines,
    trim_trailing_whitespace, unescape_markdown,
};

/// Package version. Mirrors the npm package's exported `VERSION`.
pub const VERSION: &str = "2.0.3";

/// Document schema version used by [`Document::version`]. Mirrors the JS
/// `DOCUMENT_VERSION` constant.
pub const DOCUMENT_VERSION: u32 = 1;
