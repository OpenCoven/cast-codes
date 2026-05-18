//! Block-level markdown parser — converts the token stream from
//! [`crate::parsers::tokenizer`] into a `Vec<Block>`.
//!
//! Ported from `packages/core/src/parsers/markdown.ts`. Honors
//! [`MarkdownParseOptions::generate_id`] for custom block-ID generation.

use crate::blocks::create_block;
use crate::parsers::inline::parse_inline;
use crate::parsers::tokenizer::{tokenize, Token, TokenType};
use crate::types::{
    Block, BlockProps, BlockType, CalloutProps, CalloutType, CheckListProps, CodeBlockProps,
    Document, DocumentOptions, HeadingProps, ImageProps, MarkdownParseOptions, TableAlignment,
    TableProps, TextSpan,
};
use crate::utils::{generate_id as default_generate_id, plain_content};

// ============================================================================
// Public Entry Points
// ============================================================================

/// Parses a markdown string into a flat vec of top-level blocks.
pub fn markdown_to_blocks(markdown: &str, options: &MarkdownParseOptions) -> Vec<Block> {
    let tokens = tokenize(markdown);
    parse_tokens(&tokens, options)
}

/// Parses a markdown string into a [`Document`].
pub fn markdown_to_document(markdown: &str, options: &MarkdownParseOptions) -> Document {
    let blocks = markdown_to_blocks(markdown, options);
    crate::document::create_document(&blocks, DocumentOptions::default())
}

/// Convenience: parse with default options.
pub fn parse(markdown: &str) -> Vec<Block> {
    markdown_to_blocks(markdown, &MarkdownParseOptions::default())
}

/// Convenience: parse a single line of inline markdown to `Vec<TextSpan>`.
pub fn parse_inline_content(text: &str) -> Vec<TextSpan> {
    parse_inline(text)
}

// ============================================================================
// Token Stream Cursor
// ============================================================================

struct Cursor<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    fn done(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn advance(&mut self) {
        self.index += 1;
    }
}

fn new_id(options: &MarkdownParseOptions) -> String {
    match &options.generate_id {
        Some(generator) => generator(),
        None => default_generate_id(),
    }
}

fn block_with_id(
    options: &MarkdownParseOptions,
    block_type: BlockType,
    content: Vec<TextSpan>,
    props: BlockProps,
    children: Vec<Block>,
) -> Block {
    let mut b = create_block(block_type, content, props, children);
    b.id = new_id(options);
    b
}

// ============================================================================
// Block Parsers
// ============================================================================

fn parse_tokens(tokens: &[Token], options: &MarkdownParseOptions) -> Vec<Block> {
    let mut cur = Cursor::new(tokens);
    let mut blocks = Vec::new();

    while !cur.done() {
        let Some(tok) = cur.peek() else { break };
        if tok.token_type == TokenType::Blank {
            cur.advance();
            continue;
        }
        if let Some(block) = parse_block(&mut cur, options) {
            blocks.push(block);
        }
    }

    blocks
}

fn parse_block(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Option<Block> {
    let tok = cur.peek()?;
    match tok.token_type {
        TokenType::Heading => Some(parse_heading(cur, options)),
        TokenType::Paragraph => Some(parse_paragraph(cur, options)),
        TokenType::BulletListItem => Some(parse_bullet_list(cur, options)),
        TokenType::NumberedListItem => Some(parse_numbered_list(cur, options)),
        TokenType::CheckListItem => Some(parse_check_list_item(cur, options)),
        TokenType::CodeFenceStart => Some(parse_code_block(cur, options)),
        TokenType::Blockquote => Some(parse_blockquote(cur, options)),
        TokenType::Callout => Some(parse_callout(cur, options)),
        TokenType::Divider => Some(parse_divider(cur, options)),
        TokenType::TableRow => Some(parse_table(cur, options)),
        TokenType::Image => Some(parse_image(cur, options)),
        TokenType::Blank
        | TokenType::CodeContent
        | TokenType::CodeFenceEnd
        | TokenType::TableSeparator => {
            cur.advance();
            None
        }
    }
}

fn parse_heading(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let tok = cur.peek().expect("checked");
    let level = tok.meta.level.unwrap_or(1).clamp(1, 6);
    let content = parse_inline(&tok.content);
    cur.advance();
    block_with_id(
        options,
        BlockType::Heading,
        content,
        BlockProps::Heading(HeadingProps { level }),
        Vec::new(),
    )
}

fn parse_paragraph(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let mut lines: Vec<String> = Vec::new();
    while let Some(t) = cur.peek() {
        if t.token_type != TokenType::Paragraph {
            break;
        }
        lines.push(t.content.clone());
        cur.advance();
    }
    let content = parse_inline(&lines.join(" "));
    block_with_id(
        options,
        BlockType::Paragraph,
        content,
        BlockProps::empty(),
        Vec::new(),
    )
}

fn parse_bullet_list(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let base_indent = cur.peek().expect("checked").indent;
    let mut children: Vec<Block> = Vec::new();

    while let Some(t) = cur.peek() {
        if t.token_type != TokenType::BulletListItem || t.indent < base_indent {
            break;
        }
        if t.indent > base_indent {
            let Some(last_item) = children.last_mut() else {
                break;
            };
            let nested_list = parse_bullet_list(cur, options);
            last_item.children.push(nested_list);
            continue;
        }
        let content = parse_inline(&t.content);
        children.push(block_with_id(
            options,
            BlockType::Paragraph,
            content,
            BlockProps::empty(),
            Vec::new(),
        ));
        cur.advance();
    }

    block_with_id(
        options,
        BlockType::BulletList,
        Vec::new(),
        BlockProps::empty(),
        children,
    )
}

fn parse_numbered_list(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let base_indent = cur.peek().expect("checked").indent;
    let mut children: Vec<Block> = Vec::new();

    while let Some(t) = cur.peek() {
        if t.token_type != TokenType::NumberedListItem || t.indent < base_indent {
            break;
        }
        if t.indent > base_indent {
            let Some(last_item) = children.last_mut() else {
                break;
            };
            let nested_list = parse_numbered_list(cur, options);
            last_item.children.push(nested_list);
            continue;
        }
        let content = parse_inline(&t.content);
        children.push(block_with_id(
            options,
            BlockType::Paragraph,
            content,
            BlockProps::empty(),
            Vec::new(),
        ));
        cur.advance();
    }

    block_with_id(
        options,
        BlockType::NumberedList,
        Vec::new(),
        BlockProps::empty(),
        children,
    )
}

fn parse_check_list_item(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let tok = cur.peek().expect("checked");
    let checked = tok.meta.checked.unwrap_or(false);
    let content = parse_inline(&tok.content);
    cur.advance();
    block_with_id(
        options,
        BlockType::CheckList,
        content,
        BlockProps::CheckList(CheckListProps { checked }),
        Vec::new(),
    )
}

fn parse_code_block(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let start = cur.peek().expect("checked");
    let language = start.meta.language.clone().filter(|s| !s.is_empty());
    cur.advance();

    let mut code_lines: Vec<String> = Vec::new();
    while let Some(t) = cur.peek() {
        if t.token_type != TokenType::CodeContent {
            break;
        }
        code_lines.push(t.content.clone());
        cur.advance();
    }

    if let Some(t) = cur.peek()
        && t.token_type == TokenType::CodeFenceEnd {
            cur.advance();
        }

    block_with_id(
        options,
        BlockType::CodeBlock,
        plain_content(code_lines.join("\n")),
        BlockProps::CodeBlock(CodeBlockProps { language }),
        Vec::new(),
    )
}

fn parse_blockquote(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let mut lines: Vec<String> = Vec::new();
    while let Some(t) = cur.peek() {
        if t.token_type != TokenType::Blockquote {
            break;
        }
        lines.push(t.content.clone());
        cur.advance();
    }
    let content = parse_inline(&lines.join("\n"));
    block_with_id(
        options,
        BlockType::Blockquote,
        content,
        BlockProps::empty(),
        Vec::new(),
    )
}

fn parse_callout(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let callout_kind = cur
        .peek()
        .and_then(|t| t.meta.callout_type.clone())
        .as_deref()
        .map(callout_type_from_str)
        .unwrap_or(CalloutType::Note);
    cur.advance();

    let mut lines: Vec<String> = Vec::new();
    while let Some(t) = cur.peek() {
        if t.token_type != TokenType::Blockquote {
            break;
        }
        lines.push(t.content.clone());
        cur.advance();
    }
    let content = parse_inline(&lines.join("\n"));
    block_with_id(
        options,
        BlockType::Callout,
        content,
        BlockProps::Callout(CalloutProps {
            callout_type: callout_kind,
        }),
        Vec::new(),
    )
}

fn callout_type_from_str(s: &str) -> CalloutType {
    match s.to_ascii_lowercase().as_str() {
        "info" => CalloutType::Info,
        "warning" => CalloutType::Warning,
        "tip" => CalloutType::Tip,
        "danger" => CalloutType::Danger,
        _ => CalloutType::Note,
    }
}

fn parse_divider(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    cur.advance();
    block_with_id(
        options,
        BlockType::Divider,
        Vec::new(),
        BlockProps::empty(),
        Vec::new(),
    )
}

fn parse_table(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut alignments: Vec<Option<TableAlignment>> = Vec::new();
    let mut is_first = true;
    let mut has_separator = false;

    while let Some(t) = cur.peek() {
        match t.token_type {
            TokenType::TableSeparator => {
                alignments = parse_table_alignments(&t.content);
                has_separator = true;
                cur.advance();
            }
            TokenType::TableRow => {
                let row = t
                    .content
                    .trim()
                    .trim_start_matches('|')
                    .trim_end_matches('|');
                // Split on '|' and trim whitespace but *keep* empty cells so
                // that intentional blank columns (e.g. `| a |  | b |`) are
                // preserved and the table can round-trip correctly.
                let cells: Vec<String> = row.split('|').map(|c| c.trim().to_string()).collect();
                if is_first && !has_separator {
                    headers = cells;
                    is_first = false;
                } else if has_separator {
                    rows.push(cells);
                }
                cur.advance();
            }
            _ => break,
        }
    }

    if !has_separator && !headers.is_empty() {
        rows.insert(0, headers);
        headers = Vec::new();
    }

    block_with_id(
        options,
        BlockType::Table,
        Vec::new(),
        BlockProps::Table(TableProps {
            headers,
            rows,
            alignments: if alignments.is_empty() {
                None
            } else {
                Some(alignments)
            },
        }),
        Vec::new(),
    )
}

fn parse_table_alignments(separator: &str) -> Vec<Option<TableAlignment>> {
    separator
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|cell| {
            let left = cell.starts_with(':');
            let right = cell.ends_with(':');
            match (left, right) {
                (true, true) => Some(TableAlignment::Center),
                (true, false) => Some(TableAlignment::Left),
                (false, true) => Some(TableAlignment::Right),
                _ => None,
            }
        })
        .collect()
}

fn parse_image(cur: &mut Cursor<'_>, options: &MarkdownParseOptions) -> Block {
    let tok = cur.peek().expect("checked");
    let url = tok.content.clone();
    let alt = tok.meta.image_alt.clone().filter(|s| !s.is_empty());
    let title = tok.meta.image_title.clone();
    cur.advance();
    block_with_id(
        options,
        BlockType::Image,
        Vec::new(),
        BlockProps::Image(ImageProps {
            url,
            alt,
            title,
            width: None,
            height: None,
        }),
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_md(input: &str) -> Vec<Block> {
        parse(input)
    }

    #[test]
    fn empty_input() {
        assert!(parse_md("").is_empty());
    }

    #[test]
    fn heading_levels() {
        for level in 1..=6u8 {
            let line = format!("{} title", "#".repeat(level as usize));
            let blocks = parse_md(&line);
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].block_type, BlockType::Heading);
            match blocks[0].props {
                BlockProps::Heading(p) => assert_eq!(p.level, level),
                _ => panic!(),
            }
            assert_eq!(blocks[0].content[0].text, "title");
        }
    }

    #[test]
    fn paragraph_collects_multi_line() {
        let blocks = parse_md("line one\nline two");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
        assert_eq!(blocks[0].content[0].text, "line one line two");
    }

    #[test]
    fn paragraph_inline_styles() {
        let blocks = parse_md("plain **bold** and *italic*");
        let bold = blocks[0].content.iter().any(|s| s.styles.bold);
        let italic = blocks[0].content.iter().any(|s| s.styles.italic);
        assert!(bold && italic);
    }

    #[test]
    fn bullet_list_with_three_items() {
        let blocks = parse_md("- one\n- two\n- three");
        assert_eq!(blocks[0].block_type, BlockType::BulletList);
        assert_eq!(blocks[0].children.len(), 3);
        assert_eq!(blocks[0].children[0].content[0].text, "one");
    }

    #[test]
    fn numbered_list_with_two_items() {
        let blocks = parse_md("1. first\n2. second");
        assert_eq!(blocks[0].block_type, BlockType::NumberedList);
        assert_eq!(blocks[0].children.len(), 2);
    }

    #[test]
    fn check_list_item_carries_checked() {
        let blocks = parse_md("- [x] done");
        match blocks[0].props {
            BlockProps::CheckList(p) => assert!(p.checked),
            _ => panic!(),
        }
    }

    #[test]
    fn fenced_code_block() {
        let blocks = parse_md("```rust\nlet x = 1;\nlet y = 2;\n```");
        assert_eq!(blocks[0].block_type, BlockType::CodeBlock);
        match &blocks[0].props {
            BlockProps::CodeBlock(p) => assert_eq!(p.language.as_deref(), Some("rust")),
            _ => panic!(),
        }
        assert_eq!(blocks[0].content[0].text, "let x = 1;\nlet y = 2;");
    }

    #[test]
    fn blockquote_collects_lines() {
        let blocks = parse_md("> one\n> two");
        assert_eq!(blocks[0].block_type, BlockType::Blockquote);
    }

    #[test]
    fn callout_with_body() {
        let blocks = parse_md("> [!INFO]\n> be careful");
        assert_eq!(blocks[0].block_type, BlockType::Callout);
        match blocks[0].props {
            BlockProps::Callout(p) => assert_eq!(p.callout_type, CalloutType::Info),
            _ => panic!(),
        }
        assert_eq!(blocks[0].content[0].text, "be careful");
    }

    #[test]
    fn divider() {
        let blocks = parse_md("---");
        assert_eq!(blocks[0].block_type, BlockType::Divider);
    }

    #[test]
    fn table_with_alignments() {
        let blocks = parse_md("| a | b |\n| :-- | --: |\n| 1 | 2 |");
        assert_eq!(blocks[0].block_type, BlockType::Table);
        match &blocks[0].props {
            BlockProps::Table(p) => {
                assert_eq!(p.headers, vec!["a", "b"]);
                assert_eq!(p.rows, vec![vec!["1".to_string(), "2".to_string()]]);
                let aligns = p.alignments.as_ref().unwrap();
                assert_eq!(aligns[0], Some(TableAlignment::Left));
                assert_eq!(aligns[1], Some(TableAlignment::Right));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn standalone_image() {
        let blocks = parse_md("![alt](https://x)");
        match &blocks[0].props {
            BlockProps::Image(p) => {
                assert_eq!(p.url, "https://x");
                assert_eq!(p.alt.as_deref(), Some("alt"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn document_round_trip_through_serializer() {
        let md = "# Hello\n\nworld";
        let blocks = parse(md);
        let back = crate::serializers::markdown::blocks_to_markdown(
            &blocks,
            &crate::types::MarkdownSerializeOptions::default(),
        );
        assert_eq!(back, md);
    }

    #[test]
    fn convenience_aliases_work() {
        let blocks = parse("paragraph");
        assert_eq!(blocks.len(), 1);
        let spans = parse_inline_content("**bold**");
        assert!(spans[0].styles.bold);
        let doc = markdown_to_document("# title", &MarkdownParseOptions::default());
        assert_eq!(doc.blocks.len(), 1);
    }
}

    // ── new tests for fixed behaviours ─────────────────────────────────────

    #[test]
    fn image_with_title_round_trips() {
        let blocks = parse(r#"![logo](https://example.com/logo.png "Brand Logo")"#);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].props {
            BlockProps::Image(p) => {
                assert_eq!(p.url, "https://example.com/logo.png");
                assert_eq!(p.alt.as_deref(), Some("logo"));
                assert_eq!(p.title.as_deref(), Some("Brand Logo"));
            }
            _ => panic!("expected Image block"),
        }
        // Serializer should emit the title back out.
        let back = crate::serializers::markdown::blocks_to_markdown(
            &blocks,
            &crate::types::MarkdownSerializeOptions::default(),
        );
        assert!(back.contains("\"Brand Logo\""), "serialized: {back}");
    }

    #[test]
    fn table_preserves_empty_cells() {
        let md = "| a |  | c |\n|---|---|---|\n| 1 |  | 3 |";
        let blocks = parse(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].props {
            BlockProps::Table(t) => {
                assert_eq!(t.headers.len(), 3, "headers: {:?}", t.headers);
                assert_eq!(t.rows[0].len(), 3, "row: {:?}", t.rows[0]);
                assert_eq!(t.headers[1], "", "middle header should be empty");
                assert_eq!(t.rows[0][1], "", "middle cell should be empty");
            }
            _ => panic!("expected Table block"),
        }
    }

    #[test]
    fn nested_bullet_list_items_are_children_not_dropped() {
        let md = "- a\n  - b\n  - c\n- d";
        let blocks = parse(md);
        assert_eq!(blocks.len(), 1);
        let list = &blocks[0];
        // top-level: items for "a" and "d"
        assert_eq!(list.children.len(), 2, "top-level items: {}", list.children.len());
        // "a" item should have a nested list child
        let item_a = &list.children[0];
        assert_eq!(item_a.children.len(), 1, "item_a should have 1 nested list child");
    }

    #[test]
    fn nested_numbered_list_items_are_children_not_dropped() {
        let md = "1. a\n   1. b\n1. c";
        let blocks = parse(md);
        assert_eq!(blocks.len(), 1);
        let list = &blocks[0];
        assert_eq!(list.children.len(), 2, "top-level items: {}", list.children.len());
        let item_a = &list.children[0];
        assert_eq!(item_a.children.len(), 1, "item_a should have 1 nested list child");
    }
