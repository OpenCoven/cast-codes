//! Type definitions for the block-based document model.
//!
//! Ported from `packages/core/src/types/index.ts`. JSON shape is preserved so
//! documents round-trip between this crate and `@create-markdown/core`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// All supported block types in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    #[serde(rename = "paragraph")]
    Paragraph,
    #[serde(rename = "heading")]
    Heading,
    #[serde(rename = "bulletList")]
    BulletList,
    #[serde(rename = "numberedList")]
    NumberedList,
    #[serde(rename = "checkList")]
    CheckList,
    #[serde(rename = "codeBlock")]
    CodeBlock,
    #[serde(rename = "blockquote")]
    Blockquote,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "divider")]
    Divider,
    #[serde(rename = "callout")]
    Callout,
}

impl BlockType {
    /// String form used in serialized output (the JS `BlockType` literal).
    pub const fn as_str(self) -> &'static str {
        match self {
            BlockType::Paragraph => "paragraph",
            BlockType::Heading => "heading",
            BlockType::BulletList => "bulletList",
            BlockType::NumberedList => "numberedList",
            BlockType::CheckList => "checkList",
            BlockType::CodeBlock => "codeBlock",
            BlockType::Blockquote => "blockquote",
            BlockType::Table => "table",
            BlockType::Image => "image",
            BlockType::Divider => "divider",
            BlockType::Callout => "callout",
        }
    }
}

/// Callout variants for styled callout blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalloutType {
    Info,
    Warning,
    Tip,
    Danger,
    Note,
}

impl CalloutType {
    pub const fn as_str(self) -> &'static str {
        match self {
            CalloutType::Info => "info",
            CalloutType::Warning => "warning",
            CalloutType::Tip => "tip",
            CalloutType::Danger => "danger",
            CalloutType::Note => "note",
        }
    }
}

/// Link data for inline links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LinkData {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
}

/// Inline styles that can be applied to text spans.
///
/// JSON shape matches the JS `InlineStyle` interface: every flag is optional
/// and omitted when `false`/`None`, so e.g. plain text serializes as `{}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InlineStyle {
    #[serde(skip_serializing_if = "is_false", default)]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub underline: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub strikethrough: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub code: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub highlight: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub link: Option<LinkData>,
}

#[inline]
fn is_false(b: &bool) -> bool {
    !*b
}

impl InlineStyle {
    /// Returns true if no styles are set (the span is plain text).
    pub fn is_plain(&self) -> bool {
        !self.bold
            && !self.italic
            && !self.underline
            && !self.strikethrough
            && !self.code
            && !self.highlight
            && self.link.is_none()
    }
}

/// A span of text with optional inline styles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextSpan {
    pub text: String,
    pub styles: InlineStyle,
}

impl TextSpan {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            styles: InlineStyle::default(),
        }
    }
}

/// Properties for heading blocks. `level` is 1..=6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadingProps {
    pub level: u8,
}

/// Properties for code blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CodeBlockProps {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
}

/// Properties for checklist items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CheckListProps {
    pub checked: bool,
}

/// Properties for image blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ImageProps {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub height: Option<u32>,
}

/// Properties for callout blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalloutProps {
    #[serde(rename = "type")]
    pub callout_type: CalloutType,
}

/// Column alignment for table cells. `None` (serialized as `null`) is left
/// alignment by default — matches the JS `('left' | 'center' | 'right' | null)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableAlignment {
    Left,
    Center,
    Right,
}

/// Properties for table blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TableProps {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alignments: Option<Vec<Option<TableAlignment>>>,
}

/// Type-specific properties for a [`Block`].
///
/// One variant per [`BlockType`]. Variants for blocks with no props
/// (paragraph, lists, blockquote, divider) are unit variants and serialize
/// as the empty object `{}` to match the JS `EmptyProps`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockProps {
    Heading(HeadingProps),
    CodeBlock(CodeBlockProps),
    CheckList(CheckListProps),
    Image(ImageProps),
    Callout(CalloutProps),
    Table(TableProps),
    Empty(EmptyProps),
}

/// Empty props (paragraph, bulletList, numberedList, blockquote, divider).
/// Serializes as `{}` so the JSON shape matches the JS `EmptyProps`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EmptyProps {}

impl BlockProps {
    pub fn empty() -> Self {
        BlockProps::Empty(EmptyProps {})
    }
}

/// A block in the document.
///
/// JSON shape matches the JS `Block` interface: `{ id, type, content,
/// children, props }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Unique identifier for the block.
    pub id: String,
    /// The type of block.
    #[serde(rename = "type")]
    pub block_type: BlockType,
    /// Inline content (text spans with styles).
    pub content: Vec<TextSpan>,
    /// Nested child blocks (for lists, etc.).
    pub children: Vec<Block>,
    /// Type-specific properties.
    pub props: BlockProps,
}

// JS-style type aliases. In Rust these are just `Block` — the variant is
// distinguished by the `block_type` field, not the type parameter — but the
// aliases are re-exported so that documentation reads like the JS surface.
pub type ParagraphBlock = Block;
pub type HeadingBlock = Block;
pub type BulletListBlock = Block;
pub type NumberedListBlock = Block;
pub type CheckListBlock = Block;
pub type CodeBlockBlock = Block;
pub type BlockquoteBlock = Block;
pub type TableBlock = Block;
pub type ImageBlock = Block;
pub type DividerBlock = Block;
pub type CalloutBlock = Block;

/// Document metadata.
///
/// Mirrors the JS `DocumentMeta`: known fields are named, plus an `extras`
/// catch-all matching the JS `[key: string]: unknown` index signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DocumentMeta {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub author: Option<String>,
    /// ISO-8601 timestamp string (matches what `JSON.stringify(Date)` produces).
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<String>,
    /// ISO-8601 timestamp string.
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<String>,
    /// Custom metadata fields (the JS `[key: string]: unknown`). Sorted by key
    /// for deterministic JSON output.
    #[serde(flatten)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

/// Complete markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Document version for migrations.
    pub version: u32,
    /// Array of blocks in the document.
    pub blocks: Vec<Block>,
    /// Document metadata.
    pub meta: DocumentMeta,
}

/// Options for markdown serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSerializeOptions {
    /// Line ending character(s). Defaults to `"\n"`.
    pub line_ending: LineEnding,
    /// Number of spaces for list indentation. Defaults to `2`.
    pub list_indent: usize,
    /// Heading style. Defaults to ATX (`# heading`).
    pub heading_style: HeadingStyle,
    /// Code block style. Defaults to fenced (` ``` `).
    pub code_block_style: CodeBlockStyle,
    /// Bullet character for unordered lists. Defaults to `-`.
    pub bullet_char: BulletChar,
    /// Emphasis character for bold/italic. Defaults to `*`.
    pub emphasis_char: EmphasisChar,
}

impl Default for MarkdownSerializeOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::Lf,
            list_indent: 2,
            heading_style: HeadingStyle::Atx,
            code_block_style: CodeBlockStyle::Fenced,
            bullet_char: BulletChar::Dash,
            emphasis_char: EmphasisChar::Asterisk,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// `"\n"`
    Lf,
    /// `"\r\n"`
    CrLf,
}

impl LineEnding {
    pub const fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingStyle {
    /// `# Heading`
    Atx,
    /// Underline-style (only valid for levels 1 and 2).
    Setext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockStyle {
    Fenced,
    Indented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulletChar {
    Dash,
    Asterisk,
    Plus,
}

impl BulletChar {
    pub const fn as_char(self) -> char {
        match self {
            BulletChar::Dash => '-',
            BulletChar::Asterisk => '*',
            BulletChar::Plus => '+',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmphasisChar {
    Asterisk,
    Underscore,
}

impl EmphasisChar {
    pub const fn as_char(self) -> char {
        match self {
            EmphasisChar::Asterisk => '*',
            EmphasisChar::Underscore => '_',
        }
    }
}

/// Boxed ID-generator callback. Type alias to keep the option structs
/// readable.
pub type IdGenerator = std::sync::Arc<dyn Fn() -> String + Send + Sync>;

/// Options for markdown parsing.
#[derive(Clone, Default)]
pub struct MarkdownParseOptions {
    /// Custom ID generator. If `None`, uses [`crate::utils::generate_id`].
    pub generate_id: Option<IdGenerator>,
    /// Enable strict parsing mode.
    pub strict: bool,
}

impl std::fmt::Debug for MarkdownParseOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownParseOptions")
            .field("generate_id", &self.generate_id.as_ref().map(|_| "<fn>"))
            .field("strict", &self.strict)
            .finish()
    }
}

impl PartialEq for MarkdownParseOptions {
    fn eq(&self, other: &Self) -> bool {
        self.generate_id.is_some() == other.generate_id.is_some() && self.strict == other.strict
    }
}

/// Options for document creation.
#[derive(Clone, Default)]
pub struct DocumentOptions {
    pub meta: Option<DocumentMeta>,
    pub generate_id: Option<IdGenerator>,
}

impl std::fmt::Debug for DocumentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentOptions")
            .field("meta", &self.meta)
            .field("generate_id", &self.generate_id.as_ref().map(|_| "<fn>"))
            .finish()
    }
}
