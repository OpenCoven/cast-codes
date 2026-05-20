//! Footnote pre/post-processing for the markdown parser.
//!
//! GFM footnotes have two parts:
//! - Definitions of the form `[^id]: text` (block-level, may continue on
//!   subsequent indented lines).
//! - References of the form `[^id]` (inline).
//!
//! This module pre-extracts definitions from the raw markdown (returning the
//! source minus the definition lines), and post-rewrites references in the
//! parsed `FormattedText`, appending a footnotes section if any references
//! were resolved.

use std::collections::HashMap;

use crate::{
    FormattedIndentTextInline, FormattedText, FormattedTextFragment, FormattedTextLine,
    FormattedTextStyles, Hyperlink, OrderedFormattedIndentTextInline,
};

#[derive(Debug, Clone)]
pub(crate) struct FootnoteDef {
    pub(crate) id: String,
    pub(crate) content: String,
}

pub(crate) struct FootnoteContext {
    pub(crate) definitions: HashMap<String, FootnoteDef>,
    /// Resolved id → assigned number (1-based, in order of first reference).
    pub(crate) numbers: HashMap<String, usize>,
    /// Definitions used at least once, in number order.
    pub(crate) used: Vec<FootnoteDef>,
}

impl FootnoteContext {
    pub(crate) fn empty() -> Self {
        Self {
            definitions: HashMap::new(),
            numbers: HashMap::new(),
            used: Vec::new(),
        }
    }
}

/// Strip footnote definitions from the source. Returns the source minus the
/// definition lines and the collected definition map.
pub(crate) fn extract_definitions(source: &str) -> (String, HashMap<String, FootnoteDef>) {
    let mut definitions: HashMap<String, FootnoteDef> = HashMap::new();
    let mut output = String::with_capacity(source.len());
    let mut lines = source.split_inclusive('\n').peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some((id, first)) = parse_definition_line(trimmed) {
            let mut content = first.to_string();
            // Absorb indented continuation lines (4 spaces or a tab).
            while let Some(peek) = lines.peek() {
                let peek_no_eol = peek.trim_end_matches(['\r', '\n']);
                if peek_no_eol.starts_with("    ") || peek_no_eol.starts_with('\t') {
                    let cont = lines.next().unwrap();
                    let cont_trimmed = cont
                        .trim_end_matches(['\r', '\n'])
                        .trim_start_matches(['\t'])
                        .trim_start_matches("    ");
                    content.push(' ');
                    content.push_str(cont_trimmed);
                } else {
                    break;
                }
            }
            definitions.insert(id.clone(), FootnoteDef { id, content });
            continue;
        }
        output.push_str(line);
    }

    (output, definitions)
}

fn parse_definition_line(line: &str) -> Option<(String, &str)> {
    let rest = line.strip_prefix("[^")?;
    let close = rest.find(']')?;
    let id = &rest[..close];
    if id.is_empty() || id.contains(char::is_whitespace) {
        return None;
    }
    let after = &rest[close + 1..];
    let body = after.strip_prefix(':')?.trim_start();
    Some((id.to_string(), body))
}

#[cfg(test)]
#[path = "footnotes_tests.rs"]
mod tests;
