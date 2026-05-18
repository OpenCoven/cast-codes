//! Markdown serializer — converts blocks back to a markdown string.
//!
//! Ported from `packages/core/src/serializers/markdown.ts`. Honors
//! [`MarkdownSerializeOptions`] for line endings, list indent, heading
//! style, code-fence style, bullet character, and emphasis character.

use crate::types::{
    Block, BlockProps, BlockType, BulletChar, CalloutType, CodeBlockStyle, Document, EmphasisChar,
    HeadingStyle, InlineStyle, LineEnding, MarkdownSerializeOptions, TableAlignment, TextSpan,
};
use crate::utils::spans_to_plain_text;

// ============================================================================
// Main Serializer
// ============================================================================

/// Converts a slice of blocks to a markdown string.
///
/// Top-level blocks are joined by **two** copies of `options.line_ending` to
/// produce the standard paragraph break.
pub fn blocks_to_markdown(blocks: &[Block], options: &MarkdownSerializeOptions) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .enumerate()
        .map(|(idx, block)| serialize_block(block, 0, options, idx, blocks))
        .collect();
    let separator = format!("{}{}", options.line_ending.as_str(), options.line_ending.as_str());
    parts.join(&separator)
}

/// Converts a document to a markdown string. Equivalent to
/// `blocks_to_markdown(&doc.blocks, options)`.
pub fn document_to_markdown(doc: &Document, options: &MarkdownSerializeOptions) -> String {
    blocks_to_markdown(&doc.blocks, options)
}

/// Quick stringify: convert blocks to markdown using the default options.
pub fn stringify(blocks: &[Block]) -> String {
    blocks_to_markdown(blocks, &MarkdownSerializeOptions::default())
}

// ============================================================================
// Block Serialization
// ============================================================================

/// Serializes a single block. `depth` controls list nesting indentation;
/// `index` and `siblings` are passed through for context (currently unused
/// by all block types but kept for parity with the JS signature).
pub fn serialize_block(
    block: &Block,
    depth: usize,
    options: &MarkdownSerializeOptions,
    _index: usize,
    _siblings: &[Block],
) -> String {
    match block.block_type {
        BlockType::Paragraph => serialize_inline_content(&block.content, options),
        BlockType::Heading => serialize_heading(block, options),
        BlockType::BulletList => serialize_bullet_list(block, depth, options),
        BlockType::NumberedList => serialize_numbered_list(block, depth, options),
        BlockType::CheckList => serialize_check_list(block, depth, options),
        BlockType::CodeBlock => serialize_code_block(block, options),
        BlockType::Blockquote => serialize_blockquote(block, options),
        BlockType::Table => serialize_table(block, options),
        BlockType::Image => serialize_image(block),
        BlockType::Divider => "---".to_string(),
        BlockType::Callout => serialize_callout(block, options),
    }
}

fn serialize_heading(block: &Block, options: &MarkdownSerializeOptions) -> String {
    let level = match &block.props {
        BlockProps::Heading(p) => p.level,
        _ => 1,
    };
    let content = serialize_inline_content(&block.content, options);

    if options.heading_style == HeadingStyle::Setext && (level == 1 || level == 2) {
        let underline = if level == 1 { '=' } else { '-' };
        let bar: String = std::iter::repeat_n(underline, content.chars().count()).collect();
        return format!("{content}{}{bar}", options.line_ending.as_str());
    }

    let hashes = "#".repeat(level as usize);
    format!("{hashes} {content}")
}

fn serialize_bullet_list(block: &Block, depth: usize, options: &MarkdownSerializeOptions) -> String {
    let pad = " ".repeat(depth * options.list_indent);
    let bullet = options.bullet_char.as_char();
    block
        .children
        .iter()
        .map(|child| {
            let content = serialize_list_item_content(child, depth, options);
            format!("{pad}{bullet} {content}")
        })
        .collect::<Vec<_>>()
        .join(options.line_ending.as_str())
}

fn serialize_numbered_list(
    block: &Block,
    depth: usize,
    options: &MarkdownSerializeOptions,
) -> String {
    let pad = " ".repeat(depth * options.list_indent);
    block
        .children
        .iter()
        .enumerate()
        .map(|(idx, child)| {
            let n = idx + 1;
            let content = serialize_list_item_content(child, depth, options);
            format!("{pad}{n}. {content}")
        })
        .collect::<Vec<_>>()
        .join(options.line_ending.as_str())
}

fn serialize_check_list(block: &Block, depth: usize, options: &MarkdownSerializeOptions) -> String {
    let pad = " ".repeat(depth * options.list_indent);
    let checked = match &block.props {
        BlockProps::CheckList(p) if p.checked => "x",
        _ => " ",
    };
    let content = serialize_inline_content(&block.content, options);
    format!("{pad}- [{checked}] {content}")
}

fn serialize_list_item_content(
    item: &Block,
    depth: usize,
    options: &MarkdownSerializeOptions,
) -> String {
    if !item.children.is_empty() {
        let content = serialize_inline_content(&item.content, options);
        let nested: Vec<String> = item
            .children
            .iter()
            .enumerate()
            .map(|(idx, child)| serialize_block(child, depth + 1, options, idx, &item.children))
            .collect();
        format!(
            "{content}{}{}",
            options.line_ending.as_str(),
            nested.join(options.line_ending.as_str())
        )
    } else {
        serialize_inline_content(&item.content, options)
    }
}

fn serialize_code_block(block: &Block, options: &MarkdownSerializeOptions) -> String {
    let code = spans_to_plain_text(&block.content);
    let language = match &block.props {
        BlockProps::CodeBlock(p) => p.language.clone().unwrap_or_default(),
        _ => String::new(),
    };

    if options.code_block_style == CodeBlockStyle::Indented {
        return code
            .split('\n')
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join(options.line_ending.as_str());
    }

    let le = options.line_ending.as_str();
    // Escape any ``` sequences inside the code so the fence isn't prematurely
    // closed. Uses the existing `escape_code_block` utility.
    let escaped_code = crate::utils::escape_code_block(&code);
    format!("```{language}{le}{escaped_code}{le}```")
}

fn serialize_blockquote(block: &Block, options: &MarkdownSerializeOptions) -> String {
    let content = serialize_inline_content(&block.content, options);
    content
        .split('\n')
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join(options.line_ending.as_str())
}

fn serialize_image(block: &Block) -> String {
    let (url, alt, title) = match &block.props {
        BlockProps::Image(p) => (
            p.url.clone(),
            p.alt.clone().unwrap_or_default(),
            p.title.clone(),
        ),
        _ => (String::new(), String::new(), None),
    };
    match title {
        Some(t) => format!("![{alt}]({url} \"{t}\")"),
        None => format!("![{alt}]({url})"),
    }
}

fn serialize_table(block: &Block, options: &MarkdownSerializeOptions) -> String {
    let (headers, rows, alignments) = match &block.props {
        BlockProps::Table(p) => (
            p.headers.clone(),
            p.rows.clone(),
            p.alignments.clone(),
        ),
        _ => (Vec::new(), Vec::new(), None),
    };
    let le = options.line_ending.as_str();

    // Column widths: max(header, max(row)), floored at 3.
    let column_widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, header)| {
            let header_len = header.chars().count();
            let row_max = rows
                .iter()
                .map(|row| row.get(i).map(|c| c.chars().count()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            header_len.max(row_max).max(3)
        })
        .collect();

    let header_row = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad_right(h, column_widths[i]))
        .collect::<Vec<_>>()
        .join(" | ");

    let separator_row = (0..headers.len())
        .map(|i| {
            let width = column_widths[i];
            let alignment = alignments.as_ref().and_then(|a| a.get(i).copied().flatten());
            match alignment {
                Some(TableAlignment::Left) => format!(":{}", "-".repeat(width.saturating_sub(1))),
                Some(TableAlignment::Right) => format!("{}:", "-".repeat(width.saturating_sub(1))),
                Some(TableAlignment::Center) => format!(
                    ":{}:",
                    "-".repeat(width.saturating_sub(2))
                ),
                None => "-".repeat(width),
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");

    let data_rows: Vec<String> = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, cell)| pad_right(cell, *column_widths.get(i).unwrap_or(&3)))
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect();

    let mut lines = Vec::with_capacity(2 + data_rows.len());
    lines.push(format!("| {header_row} |"));
    lines.push(format!("| {separator_row} |"));
    for row in data_rows {
        lines.push(format!("| {row} |"));
    }
    lines.join(le)
}

fn pad_right(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if count >= width {
        s.to_string()
    } else {
        let mut out = String::with_capacity(s.len() + (width - count));
        out.push_str(s);
        for _ in 0..(width - count) {
            out.push(' ');
        }
        out
    }
}

fn serialize_callout(block: &Block, options: &MarkdownSerializeOptions) -> String {
    let kind = match &block.props {
        BlockProps::Callout(p) => callout_label(p.callout_type),
        _ => "INFO",
    };
    let content = serialize_inline_content(&block.content, options);
    let le = options.line_ending.as_str();
    let body = content
        .split('\n')
        .collect::<Vec<_>>()
        .join(&format!("{le}> "));
    format!("> [!{kind}]{le}> {body}")
}

fn callout_label(callout: CalloutType) -> &'static str {
    match callout {
        CalloutType::Info => "INFO",
        CalloutType::Warning => "WARNING",
        CalloutType::Tip => "TIP",
        CalloutType::Danger => "DANGER",
        CalloutType::Note => "NOTE",
    }
}

// ============================================================================
// Inline Content Serialization
// ============================================================================

/// Serialize a slice of [`TextSpan`]s to markdown.
pub fn serialize_inline_content(spans: &[TextSpan], options: &MarkdownSerializeOptions) -> String {
    let mut out = String::new();
    for span in spans {
        out.push_str(&serialize_span(span, options));
    }
    out
}

/// Serialize a single [`TextSpan`] with its styles. Styles are applied
/// inside-out (code innermost, link outermost) matching the JS upstream.
pub fn serialize_span(span: &TextSpan, options: &MarkdownSerializeOptions) -> String {
    let mut text = span.text.clone();
    let styles = &span.styles;

    if !has_styles(styles) {
        return text;
    }

    if styles.code {
        // Choose a delimiter longer than the longest backtick run inside the
        // text so the inline-code span is always valid CommonMark.
        let max_run = {
            let mut max = 0usize;
            let mut cur = 0usize;
            for ch in text.chars() {
                if ch == '`' {
                    cur += 1;
                    max = max.max(cur);
                } else {
                    cur = 0;
                }
            }
            max
        };
        let delim = "`".repeat(max_run + 1);
        // Add a surrounding space when the content starts or ends with a
        // backtick, per CommonMark § 6.1.
        let (pre, post) = if text.starts_with('`') || text.ends_with('`') {
            (" ", " ")
        } else {
            ("", "")
        };
        text = format!("{delim}{pre}{text}{post}{delim}");
    }

    if styles.highlight {
        text = format!("=={text}==");
    }

    if styles.strikethrough {
        text = format!("~~{text}~~");
    }

    let ch = options.emphasis_char.as_char();
    if styles.bold && styles.italic {
        text = format!("{ch}{ch}{ch}{text}{ch}{ch}{ch}");
    } else if styles.italic {
        text = format!("{ch}{text}{ch}");
    } else if styles.bold {
        // (JS upstream applies bold even alongside italic separately when only
        // one is set; the combined case is handled above.)
    }

    if styles.bold && !styles.italic {
        text = format!("{ch}{ch}{text}{ch}{ch}");
    }

    if styles.underline {
        text = format!("<u>{text}</u>");
    }

    if let Some(link) = &styles.link {
        text = match &link.title {
            Some(title) => format!("[{text}]({} \"{title}\")", link.url),
            None => format!("[{text}]({})", link.url),
        };
    }

    text
}

fn has_styles(styles: &InlineStyle) -> bool {
    styles.bold
        || styles.italic
        || styles.underline
        || styles.strikethrough
        || styles.code
        || styles.highlight
        || styles.link.is_some()
}

// Silence unused-import warnings if a refactor removes the LineEnding usage.
#[allow(dead_code)]
fn _le_in_scope(_le: LineEnding) {}
#[allow(dead_code)]
fn _bc_in_scope(_b: BulletChar) {}
#[allow(dead_code)]
fn _ec_in_scope(_e: EmphasisChar) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{
        block_quote, bold, bullet_list, callout, check_list, code, code_block, divider, h1, h2,
        heading, image, info_callout, italic, link, numbered_list, paragraph, table, text,
        ImageOptions,
    };
    use crate::types::{CalloutType, MarkdownSerializeOptions};

    fn opts() -> MarkdownSerializeOptions {
        MarkdownSerializeOptions::default()
    }

    #[test]
    fn paragraph_serializes_inline_only() {
        let p = paragraph("hello");
        assert_eq!(blocks_to_markdown(&[p], &opts()), "hello");
    }

    #[test]
    fn heading_atx_default() {
        assert_eq!(blocks_to_markdown(&[h1("Title")], &opts()), "# Title");
        assert_eq!(blocks_to_markdown(&[h2("Sub")], &opts()), "## Sub");
        assert_eq!(
            blocks_to_markdown(&[heading(6, "Deep")], &opts()),
            "###### Deep"
        );
    }

    #[test]
    fn heading_setext_for_levels_1_and_2() {
        let mut o = opts();
        o.heading_style = HeadingStyle::Setext;
        assert_eq!(blocks_to_markdown(&[h1("Title")], &o), "Title\n=====");
        assert_eq!(blocks_to_markdown(&[h2("Sub")], &o), "Sub\n---");
        // Level 3+ falls back to ATX
        assert_eq!(blocks_to_markdown(&[heading(3, "x")], &o), "### x");
    }

    #[test]
    fn blocks_joined_by_blank_line() {
        let md = blocks_to_markdown(&[h1("T"), paragraph("body")], &opts());
        assert_eq!(md, "# T\n\nbody");
    }

    #[test]
    fn bullet_list_basic() {
        let list = bullet_list(["a", "b", "c"]);
        assert_eq!(blocks_to_markdown(&[list], &opts()), "- a\n- b\n- c");
    }

    #[test]
    fn numbered_list_basic() {
        let list = numbered_list(["a", "b"]);
        assert_eq!(blocks_to_markdown(&[list], &opts()), "1. a\n2. b");
    }

    #[test]
    fn check_list_renders_x_and_space() {
        let items = check_list([("done", true), ("todo", false)]);
        let md = blocks_to_markdown(&items, &opts());
        assert_eq!(md, "- [x] done\n\n- [ ] todo");
    }

    #[test]
    fn code_block_fenced() {
        let cb = code_block("println!", Some("rust".into()));
        assert_eq!(blocks_to_markdown(&[cb], &opts()), "```rust\nprintln!\n```");
    }

    #[test]
    fn code_block_indented() {
        let mut o = opts();
        o.code_block_style = CodeBlockStyle::Indented;
        let cb = code_block("a\nb", None);
        assert_eq!(blocks_to_markdown(&[cb], &o), "    a\n    b");
    }

    #[test]
    fn blockquote_prefixes_each_line() {
        let bq = block_quote("one");
        assert_eq!(blocks_to_markdown(&[bq], &opts()), "> one");
    }

    #[test]
    fn divider_renders_three_dashes() {
        assert_eq!(blocks_to_markdown(&[divider()], &opts()), "---");
    }

    #[test]
    fn image_with_and_without_title() {
        let no_title = image(
            "https://x",
            Some("alt".into()),
            ImageOptions::default(),
        );
        assert_eq!(blocks_to_markdown(&[no_title], &opts()), "![alt](https://x)");
        let with_title = image(
            "https://x",
            Some("alt".into()),
            ImageOptions {
                title: Some("the title".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            blocks_to_markdown(&[with_title], &opts()),
            "![alt](https://x \"the title\")"
        );
    }

    #[test]
    fn callout_gfm_alert_syntax() {
        let c = info_callout("be careful");
        assert_eq!(blocks_to_markdown(&[c], &opts()), "> [!INFO]\n> be careful");
        let warning = callout(CalloutType::Warning, "danger");
        assert_eq!(
            blocks_to_markdown(&[warning], &opts()),
            "> [!WARNING]\n> danger"
        );
    }

    #[test]
    fn table_with_alignments() {
        let t = table(
            vec!["A".into(), "B".into()],
            vec![vec!["1".into(), "2".into()], vec!["3".into(), "4".into()]],
            Some(vec![Some(TableAlignment::Left), Some(TableAlignment::Right)]),
        );
        let md = blocks_to_markdown(&[t], &opts());
        assert!(md.starts_with("| A   | B   |"));
        assert!(md.contains("| :-- | --: |"));
        assert!(md.contains("| 1   | 2   |"));
    }

    #[test]
    fn span_bold_italic_combined() {
        let p = paragraph(vec![TextSpan {
            text: "both".into(),
            styles: InlineStyle {
                bold: true,
                italic: true,
                ..Default::default()
            },
        }]);
        assert_eq!(blocks_to_markdown(&[p], &opts()), "***both***");
    }

    #[test]
    fn span_bold_only() {
        let p = paragraph(vec![bold("loud")]);
        assert_eq!(blocks_to_markdown(&[p], &opts()), "**loud**");
    }

    #[test]
    fn span_italic_only() {
        let p = paragraph(vec![italic("soft")]);
        assert_eq!(blocks_to_markdown(&[p], &opts()), "*soft*");
    }

    #[test]
    fn span_underscore_emphasis() {
        let mut o = opts();
        o.emphasis_char = EmphasisChar::Underscore;
        let p = paragraph(vec![bold("loud")]);
        assert_eq!(blocks_to_markdown(&[p], &o), "__loud__");
    }

    #[test]
    fn span_code_link_strike_highlight() {
        let p = paragraph(vec![code("x"), text(" + "), link("Anth", "https://a", None)]);
        assert_eq!(
            blocks_to_markdown(&[p], &opts()),
            "`x` + [Anth](https://a)"
        );
    }

    #[test]
    fn nested_bullet_list_indents_via_item_children() {
        // The @create-markdown nesting model: a list item is a block with
        // `content` (its line) and `children` (its nested blocks). A nested
        // list lives in the item's `children`, not as a sibling list.
        let inner = bullet_list(["x", "y"]);
        let mut parent_item = paragraph("parent");
        parent_item.children = vec![inner];
        let outer = bullet_list(vec![crate::ListItem::Block(parent_item)]);
        let md = blocks_to_markdown(&[outer], &opts());
        assert!(
            md.contains("- parent"),
            "expected outer bullet item; got: {md}"
        );
        assert!(md.contains("  - x"), "expected nested bullet x; got: {md}");
        assert!(md.contains("  - y"), "expected nested bullet y; got: {md}");
    }
}

    #[test]
    fn code_block_with_backtick_fence_is_escaped() {
        use crate::blocks::code_block as mkcode;
        let block = mkcode("```js\nconsole.log(1)\n```", Some("md".to_string()));
        let siblings = [block.clone()];
        let md = serialize_block(&block, 0, &MarkdownSerializeOptions::default(), 0, &siblings);
        assert!(!md.contains("```js\nconsole.log(1)\n```"), "raw fence not escaped: {md}");
    }

    #[test]
    fn inline_code_with_backtick_uses_double_delimiter() {
        use crate::blocks::paragraph as para;
        use crate::types::{InlineStyle, TextSpan};
        let span = TextSpan {
            text: "foo`bar".to_string(),
            styles: InlineStyle { code: true, ..Default::default() },
        };
        let block = {
            let mut b = para("");
            b.content = vec![span];
            b
        };
        let siblings = [block.clone()];
        let md = serialize_block(&block, 0, &MarkdownSerializeOptions::default(), 0, &siblings);
        assert!(md.contains("``foo`bar``"), "expected double-backtick delimiter; got: {md}");
    }
