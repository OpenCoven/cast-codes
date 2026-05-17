//! Block factory functions and content/props mutators.
//!
//! Ported from `packages/core/src/core/blocks.ts`. Each function constructs
//! a [`Block`] with a fresh ID via [`crate::utils::generate_id`], matching
//! the JS upstream's defaults.

use crate::types::{
    Block, BlockProps, BlockType, CalloutProps, CalloutType, CheckListProps, CodeBlockProps,
    EmptyProps, HeadingProps, ImageProps, InlineStyle, LinkData, TableAlignment, TableProps,
    TextSpan,
};
use crate::utils::{generate_id, plain_content, plain_span};

// ============================================================================
// Generic Block Creator
// ============================================================================

/// Creates a block with the given type, content, props, and children.
///
/// Mirrors the JS `createBlock(type, content, props, children)`.
pub fn create_block(
    block_type: BlockType,
    content: Vec<TextSpan>,
    props: BlockProps,
    children: Vec<Block>,
) -> Block {
    Block {
        id: generate_id(),
        block_type,
        content,
        children,
        props,
    }
}

// ============================================================================
// Inline Content Helpers (Text Spans)
// ============================================================================

/// Accepts either a `&str` or `Vec<TextSpan>` for ergonomic block factories.
///
/// JS passes `string | TextSpan[]` to every block factory; this trait gives
/// Rust the same flexibility without sacrificing type safety.
pub trait IntoContent {
    fn into_content(self) -> Vec<TextSpan>;
}

impl IntoContent for &str {
    fn into_content(self) -> Vec<TextSpan> {
        plain_content(self)
    }
}

impl IntoContent for String {
    fn into_content(self) -> Vec<TextSpan> {
        plain_content(self)
    }
}

impl IntoContent for Vec<TextSpan> {
    fn into_content(self) -> Vec<TextSpan> {
        self
    }
}

impl<const N: usize> IntoContent for [TextSpan; N] {
    fn into_content(self) -> Vec<TextSpan> {
        self.into_iter().collect()
    }
}

/// Creates a plain-text span with no styles.
pub fn text(content: impl Into<String>) -> TextSpan {
    plain_span(content)
}

/// Creates a bold text span.
pub fn bold(content: impl Into<String>) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            bold: true,
            ..Default::default()
        },
    }
}

/// Creates an italic text span.
pub fn italic(content: impl Into<String>) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            italic: true,
            ..Default::default()
        },
    }
}

/// Creates an inline-code text span.
pub fn code(content: impl Into<String>) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            code: true,
            ..Default::default()
        },
    }
}

/// Creates a strikethrough text span.
pub fn strikethrough(content: impl Into<String>) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            strikethrough: true,
            ..Default::default()
        },
    }
}

/// Creates an underlined text span.
pub fn underline(content: impl Into<String>) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            underline: true,
            ..Default::default()
        },
    }
}

/// Creates a highlighted text span.
pub fn highlight(content: impl Into<String>) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            highlight: true,
            ..Default::default()
        },
    }
}

/// Creates a link text span.
///
/// `title` is optional and matches the JS upstream's third positional arg.
pub fn link(
    content: impl Into<String>,
    url: impl Into<String>,
    title: Option<String>,
) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles: InlineStyle {
            link: Some(LinkData {
                url: url.into(),
                title,
            }),
            ..Default::default()
        },
    }
}

/// Creates a text span with arbitrary inline styles.
pub fn styled(content: impl Into<String>, styles: InlineStyle) -> TextSpan {
    TextSpan {
        text: content.into(),
        styles,
    }
}

/// Combine multiple text spans into a content array.
///
/// Equivalent to the JS spread-form `spans(a, b, c)` — in Rust, prefer
/// `vec![a, b, c]` directly. This helper exists for parity.
pub fn spans(items: impl IntoIterator<Item = TextSpan>) -> Vec<TextSpan> {
    items.into_iter().collect()
}

// ============================================================================
// Block Factory Functions
// ============================================================================

/// Creates a paragraph block.
pub fn paragraph(content: impl IntoContent) -> Block {
    create_block(
        BlockType::Paragraph,
        content.into_content(),
        BlockProps::empty(),
        Vec::new(),
    )
}

/// Creates a heading block. `level` must be 1..=6.
///
/// Panics if `level` is outside 1..=6.
pub fn heading(level: u8, content: impl IntoContent) -> Block {
    assert!(
        (1..=6).contains(&level),
        "heading level must be 1..=6, got {level}"
    );
    create_block(
        BlockType::Heading,
        content.into_content(),
        BlockProps::Heading(HeadingProps { level }),
        Vec::new(),
    )
}

/// Creates an H1 heading.
pub fn h1(content: impl IntoContent) -> Block {
    heading(1, content)
}
/// Creates an H2 heading.
pub fn h2(content: impl IntoContent) -> Block {
    heading(2, content)
}
/// Creates an H3 heading.
pub fn h3(content: impl IntoContent) -> Block {
    heading(3, content)
}
/// Creates an H4 heading.
pub fn h4(content: impl IntoContent) -> Block {
    heading(4, content)
}
/// Creates an H5 heading.
pub fn h5(content: impl IntoContent) -> Block {
    heading(5, content)
}
/// Creates an H6 heading.
pub fn h6(content: impl IntoContent) -> Block {
    heading(6, content)
}

/// Item value accepted by [`bullet_list`] and [`numbered_list`].
///
/// Mirrors the JS `string | TextSpan[] | Block` union.
pub enum ListItem {
    Text(String),
    Spans(Vec<TextSpan>),
    Block(Block),
}

impl From<&str> for ListItem {
    fn from(s: &str) -> Self {
        ListItem::Text(s.to_string())
    }
}

impl From<String> for ListItem {
    fn from(s: String) -> Self {
        ListItem::Text(s)
    }
}

impl From<Vec<TextSpan>> for ListItem {
    fn from(spans: Vec<TextSpan>) -> Self {
        ListItem::Spans(spans)
    }
}

impl From<Block> for ListItem {
    fn from(block: Block) -> Self {
        ListItem::Block(block)
    }
}

fn list_item_to_block(item: ListItem) -> Block {
    match item {
        ListItem::Text(s) => paragraph(s.as_str()),
        ListItem::Spans(spans) => paragraph(spans),
        ListItem::Block(b) => b,
    }
}

/// Creates a bullet (unordered) list block whose children are the given items.
///
/// String/span items are wrapped in paragraphs; pre-built blocks are kept as
/// children verbatim.
pub fn bullet_list<I, V>(items: I) -> Block
where
    I: IntoIterator<Item = V>,
    V: Into<ListItem>,
{
    let children = items.into_iter().map(|i| list_item_to_block(i.into())).collect();
    create_block(
        BlockType::BulletList,
        Vec::new(),
        BlockProps::empty(),
        children,
    )
}

/// Creates a numbered (ordered) list block whose children are the given items.
pub fn numbered_list<I, V>(items: I) -> Block
where
    I: IntoIterator<Item = V>,
    V: Into<ListItem>,
{
    let children = items.into_iter().map(|i| list_item_to_block(i.into())).collect();
    create_block(
        BlockType::NumberedList,
        Vec::new(),
        BlockProps::empty(),
        children,
    )
}

/// Creates a single checklist item.
pub fn check_list_item(content: impl IntoContent, checked: bool) -> Block {
    create_block(
        BlockType::CheckList,
        content.into_content(),
        BlockProps::CheckList(CheckListProps { checked }),
        Vec::new(),
    )
}

/// Item value accepted by [`check_list`].
pub struct CheckItem {
    pub content: Vec<TextSpan>,
    pub checked: bool,
}

impl CheckItem {
    pub fn new(content: impl IntoContent, checked: bool) -> Self {
        Self {
            content: content.into_content(),
            checked,
        }
    }
}

impl From<&str> for CheckItem {
    fn from(s: &str) -> Self {
        Self::new(s, false)
    }
}

impl From<(&str, bool)> for CheckItem {
    fn from((s, checked): (&str, bool)) -> Self {
        Self::new(s, checked)
    }
}

impl From<(String, bool)> for CheckItem {
    fn from((s, checked): (String, bool)) -> Self {
        Self::new(s.as_str(), checked)
    }
}

/// Creates a checklist as a flat `Vec<Block>` of `checkList` items.
///
/// The JS upstream returns `Block[]` here rather than wrapping items in a
/// container — this matches that shape.
pub fn check_list<I, V>(items: I) -> Vec<Block>
where
    I: IntoIterator<Item = V>,
    V: Into<CheckItem>,
{
    items
        .into_iter()
        .map(|item| {
            let CheckItem { content, checked } = item.into();
            check_list_item(content, checked)
        })
        .collect()
}

/// Creates a code block. `language` is optional.
pub fn code_block(code: impl Into<String>, language: Option<String>) -> Block {
    create_block(
        BlockType::CodeBlock,
        plain_content(code),
        BlockProps::CodeBlock(CodeBlockProps { language }),
        Vec::new(),
    )
}

/// Creates a blockquote.
///
/// Named `block_quote` in Rust (matching idiomatic Rust naming) and re-exported
/// at the crate root.
pub fn block_quote(content: impl IntoContent) -> Block {
    create_block(
        BlockType::Blockquote,
        content.into_content(),
        BlockProps::empty(),
        Vec::new(),
    )
}

/// Creates a horizontal divider.
pub fn divider() -> Block {
    create_block(
        BlockType::Divider,
        Vec::new(),
        BlockProps::empty(),
        Vec::new(),
    )
}

/// Options for [`image`].
#[derive(Debug, Clone, Default)]
pub struct ImageOptions {
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Creates an image block.
pub fn image(url: impl Into<String>, alt: Option<String>, options: ImageOptions) -> Block {
    create_block(
        BlockType::Image,
        Vec::new(),
        BlockProps::Image(ImageProps {
            url: url.into(),
            alt,
            title: options.title,
            width: options.width,
            height: options.height,
        }),
        Vec::new(),
    )
}

/// Creates a callout block.
pub fn callout(callout_type: CalloutType, content: impl IntoContent) -> Block {
    create_block(
        BlockType::Callout,
        content.into_content(),
        BlockProps::Callout(CalloutProps { callout_type }),
        Vec::new(),
    )
}

/// Creates an info callout.
pub fn info_callout(content: impl IntoContent) -> Block {
    callout(CalloutType::Info, content)
}
/// Creates a warning callout.
pub fn warning_callout(content: impl IntoContent) -> Block {
    callout(CalloutType::Warning, content)
}
/// Creates a tip callout.
pub fn tip_callout(content: impl IntoContent) -> Block {
    callout(CalloutType::Tip, content)
}
/// Creates a danger callout.
pub fn danger_callout(content: impl IntoContent) -> Block {
    callout(CalloutType::Danger, content)
}
/// Creates a note callout.
pub fn note_callout(content: impl IntoContent) -> Block {
    callout(CalloutType::Note, content)
}

/// Creates a table block.
///
/// `alignments`, if present, should have one entry per column. `None`
/// entries indicate left alignment.
pub fn table(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    alignments: Option<Vec<Option<TableAlignment>>>,
) -> Block {
    create_block(
        BlockType::Table,
        Vec::new(),
        BlockProps::Table(TableProps {
            headers,
            rows,
            alignments,
        }),
        Vec::new(),
    )
}

// ============================================================================
// Block Modification Helpers
// ============================================================================

/// Returns a new block with `new_spans` appended to its content.
pub fn append_content(mut block: Block, new_spans: impl IntoIterator<Item = TextSpan>) -> Block {
    block.content.extend(new_spans);
    block
}

/// Returns a new block with `new_spans` prepended to its content.
pub fn prepend_content(mut block: Block, new_spans: impl IntoIterator<Item = TextSpan>) -> Block {
    let mut combined: Vec<TextSpan> = new_spans.into_iter().collect();
    combined.append(&mut block.content);
    block.content = combined;
    block
}

/// Returns a new block with its content replaced by `content`.
pub fn set_content(mut block: Block, content: Vec<TextSpan>) -> Block {
    block.content = content;
    block
}

/// Returns a new block with `new_children` appended to its children.
pub fn add_children(mut block: Block, new_children: impl IntoIterator<Item = Block>) -> Block {
    block.children.extend(new_children);
    block
}

/// Returns a new block with its props replaced.
///
/// JS's `updateProps(block, partialProps)` does a shallow merge; in Rust the
/// per-variant props enum makes that awkward, so this takes a complete new
/// [`BlockProps`]. The caller is responsible for merging if a partial update
/// is desired.
pub fn update_props(mut block: Block, props: BlockProps) -> Block {
    block.props = props;
    block
}

// Suppress an unused-import warning on EmptyProps — exposed for users so it's
// kept in the public surface even though this module only constructs it
// through `BlockProps::empty()`.
#[allow(dead_code)]
fn _ensure_empty_props_in_scope(_: EmptyProps) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_from_str() {
        let p = paragraph("hello");
        assert_eq!(p.block_type, BlockType::Paragraph);
        assert_eq!(p.content.len(), 1);
        assert_eq!(p.content[0].text, "hello");
        assert!(p.content[0].styles.is_plain());
        assert!(p.children.is_empty());
        assert!(matches!(p.props, BlockProps::Empty(_)));
    }

    #[test]
    fn paragraph_from_spans() {
        let p = paragraph(vec![bold("bold"), text(" rest")]);
        assert_eq!(p.content.len(), 2);
        assert!(p.content[0].styles.bold);
        assert!(p.content[1].styles.is_plain());
    }

    #[test]
    fn heading_levels_round_trip() {
        for level in 1..=6u8 {
            let h = heading(level, "h");
            assert_eq!(h.block_type, BlockType::Heading);
            match h.props {
                BlockProps::Heading(props) => assert_eq!(props.level, level),
                _ => panic!("expected heading props"),
            }
        }
    }

    #[test]
    fn h1_through_h6_match_explicit_heading() {
        assert!(matches!(h1("a").props, BlockProps::Heading(HeadingProps { level: 1 })));
        assert!(matches!(h2("a").props, BlockProps::Heading(HeadingProps { level: 2 })));
        assert!(matches!(h3("a").props, BlockProps::Heading(HeadingProps { level: 3 })));
        assert!(matches!(h4("a").props, BlockProps::Heading(HeadingProps { level: 4 })));
        assert!(matches!(h5("a").props, BlockProps::Heading(HeadingProps { level: 5 })));
        assert!(matches!(h6("a").props, BlockProps::Heading(HeadingProps { level: 6 })));
    }

    #[test]
    #[should_panic(expected = "heading level must be 1..=6")]
    fn heading_level_zero_panics() {
        let _ = heading(0, "x");
    }

    #[test]
    #[should_panic(expected = "heading level must be 1..=6")]
    fn heading_level_seven_panics() {
        let _ = heading(7, "x");
    }

    #[test]
    fn bullet_list_wraps_strings_in_paragraphs() {
        let list = bullet_list(["one", "two", "three"]);
        assert_eq!(list.block_type, BlockType::BulletList);
        assert_eq!(list.children.len(), 3);
        for child in &list.children {
            assert_eq!(child.block_type, BlockType::Paragraph);
        }
    }

    #[test]
    fn numbered_list_accepts_pre_built_blocks() {
        let list = numbered_list(vec![ListItem::Block(h1("nested heading"))]);
        assert_eq!(list.children.len(), 1);
        assert_eq!(list.children[0].block_type, BlockType::Heading);
    }

    #[test]
    fn check_list_item_carries_checked_flag() {
        let item = check_list_item("buy milk", true);
        match item.props {
            BlockProps::CheckList(props) => assert!(props.checked),
            _ => panic!("expected check list props"),
        }
    }

    #[test]
    fn check_list_returns_flat_blocks() {
        let items = check_list([("a", true), ("b", false)]);
        assert_eq!(items.len(), 2);
        match &items[0].props {
            BlockProps::CheckList(p) => assert!(p.checked),
            _ => panic!(),
        }
        match &items[1].props {
            BlockProps::CheckList(p) => assert!(!p.checked),
            _ => panic!(),
        }
    }

    #[test]
    fn code_block_with_and_without_language() {
        let b = code_block("println!", Some("rust".into()));
        match b.props {
            BlockProps::CodeBlock(p) => assert_eq!(p.language.as_deref(), Some("rust")),
            _ => panic!(),
        }
        let none = code_block("x", None);
        match none.props {
            BlockProps::CodeBlock(p) => assert!(p.language.is_none()),
            _ => panic!(),
        }
    }

    #[test]
    fn callout_variants() {
        assert!(matches!(
            info_callout("hi").props,
            BlockProps::Callout(CalloutProps {
                callout_type: CalloutType::Info
            })
        ));
        assert!(matches!(
            warning_callout("hi").props,
            BlockProps::Callout(CalloutProps {
                callout_type: CalloutType::Warning
            })
        ));
        assert!(matches!(
            tip_callout("hi").props,
            BlockProps::Callout(CalloutProps {
                callout_type: CalloutType::Tip
            })
        ));
        assert!(matches!(
            danger_callout("hi").props,
            BlockProps::Callout(CalloutProps {
                callout_type: CalloutType::Danger
            })
        ));
        assert!(matches!(
            note_callout("hi").props,
            BlockProps::Callout(CalloutProps {
                callout_type: CalloutType::Note
            })
        ));
    }

    #[test]
    fn table_carries_headers_and_rows() {
        let t = table(
            vec!["A".into(), "B".into()],
            vec![vec!["1".into(), "2".into()]],
            None,
        );
        match t.props {
            BlockProps::Table(p) => {
                assert_eq!(p.headers, vec!["A", "B"]);
                assert_eq!(p.rows, vec![vec!["1".to_string(), "2".to_string()]]);
                assert!(p.alignments.is_none());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn image_with_options() {
        let img = image(
            "u",
            Some("alt".into()),
            ImageOptions {
                title: Some("t".into()),
                width: Some(100),
                height: Some(200),
            },
        );
        match img.props {
            BlockProps::Image(p) => {
                assert_eq!(p.url, "u");
                assert_eq!(p.alt.as_deref(), Some("alt"));
                assert_eq!(p.title.as_deref(), Some("t"));
                assert_eq!(p.width, Some(100));
                assert_eq!(p.height, Some(200));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn append_prepend_set_content() {
        let p = paragraph("middle");
        let after = append_content(p.clone(), vec![text(" end")]);
        assert_eq!(after.content.len(), 2);
        assert_eq!(after.content[1].text, " end");

        let before = prepend_content(p.clone(), vec![text("start ")]);
        assert_eq!(before.content[0].text, "start ");
        assert_eq!(before.content[1].text, "middle");

        let replaced = set_content(p, vec![text("new")]);
        assert_eq!(replaced.content.len(), 1);
        assert_eq!(replaced.content[0].text, "new");
    }

    #[test]
    fn add_children_appends() {
        let list = bullet_list(["one"]);
        let extra = paragraph("two");
        let bigger = add_children(list, vec![extra]);
        assert_eq!(bigger.children.len(), 2);
    }

    #[test]
    fn link_carries_url_and_optional_title() {
        let l = link("Anthropic", "https://anthropic.com", Some("home".into()));
        assert_eq!(l.text, "Anthropic");
        let data = l.styles.link.unwrap();
        assert_eq!(data.url, "https://anthropic.com");
        assert_eq!(data.title.as_deref(), Some("home"));
    }

    #[test]
    fn divider_is_empty() {
        let d = divider();
        assert_eq!(d.block_type, BlockType::Divider);
        assert!(d.content.is_empty());
        assert!(d.children.is_empty());
    }
}
