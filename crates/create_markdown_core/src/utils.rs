//! Utility functions for span manipulation, escaping, and ID generation.
//!
//! Ported from `packages/core/src/core/utils.ts`. Implementations are
//! incomplete in the initial scaffold — see follow-up commits.

use crate::types::{Block, TextSpan};

#[cfg(test)]
use crate::types::BlockType;
use std::sync::atomic::{AtomicU64, Ordering};

/// Generates a unique block ID. Format mirrors the JS package
/// (`block_<timestamp>_<counter>`) but the implementation uses a monotonic
/// counter so IDs are deterministic across the process.
pub fn generate_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("block_{n:x}")
}

/// Deep-clones a single block. Implemented via the [`Clone`] impl, since
/// [`Block`] owns all its data.
pub fn deep_clone_block(block: &Block) -> Block {
    block.clone()
}

/// Deep-clones a slice of blocks.
pub fn deep_clone_blocks(blocks: &[Block]) -> Vec<Block> {
    blocks.to_vec()
}

/// Normalize CRLF line endings to LF.
pub fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Convert line endings to a target style.
pub fn convert_line_endings(s: &str, target: crate::types::LineEnding) -> String {
    let lf = normalize_line_endings(s);
    match target {
        crate::types::LineEnding::Lf => lf,
        crate::types::LineEnding::CrLf => lf.replace('\n', "\r\n"),
    }
}

/// Escape markdown special characters in a plain-text string.
///
/// Escapes the characters that markdown treats as syntax: `\`, `` ` ``, `*`,
/// `_`, `{`, `}`, `[`, `]`, `(`, `)`, `#`, `+`, `-`, `.`, `!`, `|`, `>`, `~`.
pub fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '('
                | ')'
                | '#'
                | '+'
                | '-'
                | '.'
                | '!'
                | '|'
                | '>'
                | '~'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Inverse of [`escape_markdown`]: unescape any backslash-escaped markdown
/// special characters.
pub fn unescape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if matches!(
                    next,
                    '\\' | '`'
                        | '*'
                        | '_'
                        | '{'
                        | '}'
                        | '['
                        | ']'
                        | '('
                        | ')'
                        | '#'
                        | '+'
                        | '-'
                        | '.'
                        | '!'
                        | '|'
                        | '>'
                        | '~'
                ) {
                    out.push(next);
                    chars.next();
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

/// Escape backticks inside fenced code blocks so the fence remains parseable.
/// Returns a fence string with enough backticks to wrap the content.
pub fn escape_code_block(code: &str) -> String {
    let mut max_run = 0usize;
    let mut current = 0usize;
    for c in code.chars() {
        if c == '`' {
            current += 1;
            if current > max_run {
                max_run = current;
            }
        } else {
            current = 0;
        }
    }
    "`".repeat(max_run.max(2) + 1)
}

/// Trim trailing whitespace from each line.
pub fn trim_trailing_whitespace(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse runs of 3+ blank lines into 2.
pub fn trim_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank_run = 0usize;
    for line in s.split('\n') {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(line);
            out.push('\n');
        }
    }
    // Drop the final extra '\n' to match JS .trim()-ish behavior.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Indent every line of `s` by `count` spaces.
pub fn indent(s: &str, count: usize) -> String {
    let pad = " ".repeat(count);
    s.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
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

/// Returns the plain-text content of a single span.
pub fn plain_span(span: &TextSpan) -> &str {
    span.text.as_str()
}

/// Returns the plain-text content of a block (its `content` joined).
pub fn plain_content(block: &Block) -> String {
    spans_to_plain_text(&block.content)
}

/// Returns `true` if the block has non-empty inline content.
pub fn has_content(block: &Block) -> bool {
    block.content.iter().any(|s| !s.text.is_empty())
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

/// Alias kept for parity with the JS export name `deepClone` (which the JS
/// package exposes alongside `deepCloneBlocks`).
pub fn deep_clone(block: &Block) -> Block {
    deep_clone_block(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_and_unescape_roundtrip() {
        let cases = ["plain", "has *asterisks*", "[link](url)", "back\\slash"];
        for case in cases {
            let escaped = escape_markdown(case);
            let back = unescape_markdown(&escaped);
            assert_eq!(back, case, "round-trip failed for {case:?}");
        }
    }

    #[test]
    fn normalize_line_endings_handles_crlf_and_cr() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\n"), "a\nb\nc\n");
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
