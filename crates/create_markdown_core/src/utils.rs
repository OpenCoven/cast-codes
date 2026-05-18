//! Utility functions for ID generation, deep cloning, escaping, and text/span helpers.
//!
//! Ported from `packages/core/src/core/utils.ts`. Function semantics match the
//! JS upstream — escaping uses the same character set (``[\\`*_{}[\]()#+\-.!|]``),
//! IDs are 8-character random alphanumeric strings, `deep_clone` regenerates
//! block IDs by default.

use crate::types::{Block, BlockProps, InlineStyle, LineEnding, TextSpan};
use std::cell::Cell;

#[cfg(test)]
use crate::types::BlockType;

/// Characters used for ID generation (URL-safe, matches the JS upstream).
const ID_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

const DEFAULT_ID_LENGTH: usize = 8;

thread_local! {
    /// Per-thread PRNG state. Seeded from a per-process counter on first use.
    static RNG_STATE: Cell<u64> = const { Cell::new(0) };
}

fn seed_rng() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED_COUNTER: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    SEED_COUNTER
        .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed)
        .wrapping_add(nanos)
        .wrapping_add(1)
}

fn next_random_u64() -> u64 {
    RNG_STATE.with(|state| {
        let mut x = state.get();
        if x == 0 {
            x = seed_rng();
        }
        // xorshift64
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        state.set(x);
        x
    })
}

/// Generates a unique 8-character URL-safe block ID.
///
/// Matches the JS upstream's nanoid-style format: alphanumeric, 8 chars by
/// default. Use [`generate_id_with_length`] for a custom length.
pub fn generate_id() -> String {
    generate_id_with_length(DEFAULT_ID_LENGTH)
}

/// Generates a random URL-safe ID of the given length.
pub fn generate_id_with_length(length: usize) -> String {
    let mut out = String::with_capacity(length);
    let chars_len = ID_CHARS.len() as u64;
    for _ in 0..length {
        let n = next_random_u64();
        let idx = (n % chars_len) as usize;
        out.push(ID_CHARS[idx] as char);
    }
    out
}

/// Deep-clones a block (and all its children), regenerating IDs.
///
/// Mirrors the JS `deepClone(block, regenerateIds = true)`.
pub fn deep_clone(block: &Block) -> Block {
    deep_clone_block_with(block, true)
}

/// Deep-clones a block. If `regenerate_ids` is `false`, IDs are preserved.
pub fn deep_clone_block_with(block: &Block, regenerate_ids: bool) -> Block {
    Block {
        id: if regenerate_ids {
            generate_id()
        } else {
            block.id.clone()
        },
        block_type: block.block_type,
        content: block
            .content
            .iter()
            .map(|s| TextSpan {
                text: s.text.clone(),
                styles: s.styles.clone(),
            })
            .collect(),
        children: block
            .children
            .iter()
            .map(|c| deep_clone_block_with(c, regenerate_ids))
            .collect(),
        props: block.props.clone(),
    }
}

/// Deep-clones a block (alias for [`deep_clone`] kept for clarity at call
/// sites that explicitly want the "single block" variant).
pub fn deep_clone_block(block: &Block) -> Block {
    deep_clone(block)
}

/// Deep-clones a slice of blocks, regenerating IDs.
pub fn deep_clone_blocks(blocks: &[Block]) -> Vec<Block> {
    blocks.iter().map(deep_clone).collect()
}

/// Normalize CRLF and bare CR line endings to LF.
pub fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Convert line endings to a target style.
pub fn convert_line_endings(s: &str, target: LineEnding) -> String {
    let lf = normalize_line_endings(s);
    match target {
        LineEnding::Lf => lf,
        LineEnding::CrLf => lf.replace('\n', "\r\n"),
    }
}

/// Characters escaped by [`escape_markdown`] — matches the JS regex
/// `[\\`*_{}[\]()#+\-.!|]`.
const MARKDOWN_ESCAPE_CHARS: &[char] = &[
    '\\', '`', '*', '_', '{', '}', '[', ']', '(', ')', '#', '+', '-', '.', '!', '|',
];

/// Escapes special markdown characters in plain text so it can be embedded
/// in a markdown document without being interpreted as syntax.
pub fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if MARKDOWN_ESCAPE_CHARS.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Inverse of [`escape_markdown`]: unescapes any backslash-escaped markdown
/// special characters. Backslashes before non-special characters are left
/// alone.
pub fn unescape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&next) = chars.peek()
                && MARKDOWN_ESCAPE_CHARS.contains(&next) {
                    out.push(next);
                    chars.next();
                    continue;
                }
        out.push(c);
    }
    out
}

/// Escapes ``` sequences inside code-block content so they can't close the
/// fence. Matches the JS `escapeCodeBlock`.
pub fn escape_code_block(s: &str) -> String {
    s.replace("```", "\\`\\`\\`")
}

/// Trim trailing whitespace from each line.
pub fn trim_trailing_whitespace(s: &str) -> String {
    s.split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove leading and trailing blank lines.
pub fn trim_blank_lines(s: &str) -> String {
    let lines: Vec<&str> = s.split('\n').collect();
    let mut start = 0usize;
    while start < lines.len() && lines[start].trim().is_empty() {
        start += 1;
    }
    let mut end = lines.len();
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[start..end].join("\n")
}

/// Indent every non-blank line of `s` by `count` spaces.
pub fn indent(s: &str, count: usize) -> String {
    let pad = " ".repeat(count);
    s.split('\n')
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Concatenate the plain text of a slice of spans (styles dropped).
pub fn spans_to_plain_text(spans: &[TextSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Creates a single text span with no styles.
///
/// Mirrors the JS `plainSpan(text)`.
pub fn plain_span(text: impl Into<String>) -> TextSpan {
    TextSpan {
        text: text.into(),
        styles: InlineStyle::default(),
    }
}

/// Creates an inline-content array containing a single plain text span.
///
/// Mirrors the JS `plainContent(text)`.
pub fn plain_content(text: impl Into<String>) -> Vec<TextSpan> {
    vec![plain_span(text)]
}

/// Returns the concatenated plain text of a block's inline content.
///
/// (The JS package's `plainContent` only takes strings — block text
/// extraction is `spansToPlainText(block.content)`. This is a Rust-only
/// convenience alias.)
pub fn block_plain_text(block: &Block) -> String {
    spans_to_plain_text(&block.content)
}

/// Returns `true` if the block has any non-empty inline content.
pub fn has_content(block: &Block) -> bool {
    !block.content.is_empty() && block.content.iter().any(|s| !s.text.is_empty())
}

/// Returns `true` if the block has child blocks.
pub fn has_children(block: &Block) -> bool {
    !block.children.is_empty()
}

/// Returns `true` if `level` is a valid heading level (1..=6).
pub fn is_valid_heading_level(level: u8) -> bool {
    (1..=6).contains(&level)
}

/// Returns `true` if `s` is a valid block type string.
pub fn is_valid_block_type(s: &str) -> bool {
    matches!(
        s,
        "paragraph"
            | "heading"
            | "bulletList"
            | "numberedList"
            | "checkList"
            | "codeBlock"
            | "blockquote"
            | "table"
            | "image"
            | "divider"
            | "callout"
    )
}

// `BlockProps` is referenced by `deep_clone_block_with` via the `block.props`
// field; the import keeps the type in scope for clippy's variable inference.
#[allow(dead_code)]
fn _ensure_block_props_in_scope(_p: BlockProps) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_id_is_8_chars_alphanumeric() {
        let id = generate_id();
        assert_eq!(id.len(), DEFAULT_ID_LENGTH);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_id_is_distinct_within_a_burst() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..256 {
            assert!(seen.insert(generate_id()));
        }
    }

    #[test]
    fn escape_and_unescape_roundtrip() {
        let cases = [
            "plain",
            "has *asterisks* and _underscores_",
            "[link](url)",
            "back\\slash",
            "many|pipes|here",
        ];
        for case in cases {
            let escaped = escape_markdown(case);
            let back = unescape_markdown(&escaped);
            assert_eq!(back, case, "round-trip failed for {case:?}");
        }
    }

    #[test]
    fn escape_markdown_escapes_js_charset_only() {
        // The JS package does NOT escape '>' or '~'.
        let s = "tilde~angle>";
        assert_eq!(escape_markdown(s), "tilde~angle>");
    }

    #[test]
    fn normalize_line_endings_handles_crlf_and_cr() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\n"), "a\nb\nc\n");
    }

    #[test]
    fn escape_code_block_replaces_triple_backticks() {
        assert_eq!(escape_code_block("```js"), "\\`\\`\\`js");
    }

    #[test]
    fn trim_blank_lines_strips_leading_and_trailing() {
        assert_eq!(trim_blank_lines("\n\n  \nhi\nthere\n\n"), "hi\nthere");
    }

    #[test]
    fn indent_skips_blank_lines() {
        assert_eq!(indent("a\n\nb", 2), "  a\n\n  b");
    }

    #[test]
    fn plain_span_round_trips() {
        let s = plain_span("hi");
        assert_eq!(s.text, "hi");
        assert!(s.styles.is_plain());
    }

    #[test]
    fn plain_content_is_single_span() {
        let c = plain_content("hi");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "hi");
    }

    #[test]
    fn is_valid_heading_level_bounds() {
        assert!(is_valid_heading_level(1));
        assert!(is_valid_heading_level(6));
        assert!(!is_valid_heading_level(0));
        assert!(!is_valid_heading_level(7));
    }

    #[test]
    fn is_valid_block_type_known_strings() {
        for t in [
            "paragraph",
            "heading",
            "bulletList",
            "numberedList",
            "checkList",
            "codeBlock",
            "blockquote",
            "table",
            "image",
            "divider",
            "callout",
        ] {
            assert!(is_valid_block_type(t));
        }
        assert!(!is_valid_block_type("unknown"));
        assert!(!is_valid_block_type(""));
    }

    #[test]
    fn block_type_str_matches() {
        for bt in [
            BlockType::Paragraph,
            BlockType::Heading,
            BlockType::BulletList,
            BlockType::NumberedList,
            BlockType::CheckList,
            BlockType::CodeBlock,
            BlockType::Blockquote,
            BlockType::Table,
            BlockType::Image,
            BlockType::Divider,
            BlockType::Callout,
        ] {
            assert!(is_valid_block_type(bt.as_str()));
        }
    }
}
