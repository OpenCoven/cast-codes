//! Rust sister port of [`@create-markdown/core`](https://www.npmjs.com/package/@create-markdown/core).
//!
//! Block-based markdown parsing and serialization. Mirrors the JS package's
//! behavior and JSON shape (so a [`Document`] round-trips between Rust and
//! TypeScript) but exposes a Rust-idiomatic snake_case API. See the README
//! for the JS→Rust naming map.
//!
//! Tracks `@create-markdown/core@2.0.3`. This is the scaffold commit — types,
//! utils, and the public version constant are complete; block factories,
//! document operations, parser, and serializer are filled in by subsequent
//! commits.

pub mod types;
pub mod utils;

pub use types::{
    Block, BlockProps, BlockType, BlockquoteBlock, BulletChar, BulletListBlock, CalloutBlock,
    CalloutProps, CalloutType, CheckListBlock, CheckListProps, CodeBlockBlock, CodeBlockProps,
    CodeBlockStyle, DividerBlock, Document, DocumentMeta, DocumentOptions, EmphasisChar,
    EmptyProps, HeadingBlock, HeadingProps, HeadingStyle, ImageBlock, ImageProps, InlineStyle,
    LineEnding, LinkData, MarkdownParseOptions, MarkdownSerializeOptions, NumberedListBlock,
    ParagraphBlock, TableAlignment, TableBlock, TableProps, TextSpan,
};
pub use utils::{
    convert_line_endings, deep_clone, deep_clone_block, deep_clone_blocks, escape_code_block,
    escape_markdown, generate_id, has_children, has_content, indent, is_valid_block_type,
    is_valid_heading_level, normalize_line_endings, plain_content, plain_span, spans_to_plain_text,
    trim_blank_lines, trim_trailing_whitespace, unescape_markdown,
};

/// Package version. Mirrors the npm package's exported `VERSION`.
pub const VERSION: &str = "2.0.3";

/// Document schema version used by [`Document::version`]. Mirrors the JS
/// `DOCUMENT_VERSION` constant.
pub const DOCUMENT_VERSION: u32 = 1;
