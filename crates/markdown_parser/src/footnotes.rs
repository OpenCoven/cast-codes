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

/// Walk every inline fragment in `text` and rewrite occurrences of `[^id]` (for
/// `id` present in `defs`) into a hyperlink fragment numbered in first-reference order.
///
/// Returns the rewritten text and a `FootnoteContext` populated with the
/// definitions that were actually used (in numbered order).
pub(crate) fn rewrite_references(
    mut text: FormattedText,
    defs: HashMap<String, FootnoteDef>,
) -> (FormattedText, FootnoteContext) {
    let mut ctx = FootnoteContext {
        definitions: defs,
        numbers: HashMap::new(),
        used: Vec::new(),
    };

    for line in text.lines.iter_mut() {
        rewrite_line(line, &mut ctx);
    }

    (text, ctx)
}

fn rewrite_line(line: &mut FormattedTextLine, ctx: &mut FootnoteContext) {
    let fragments = match line {
        FormattedTextLine::Line(frags) => frags,
        FormattedTextLine::Heading(h) => &mut h.text,
        FormattedTextLine::OrderedList(list) => &mut list.indented_text.text,
        FormattedTextLine::UnorderedList(list) => &mut list.text,
        FormattedTextLine::TaskList(list) => &mut list.text,
        _ => return,
    };
    rewrite_fragments(fragments, ctx);
}

fn rewrite_fragments(
    fragments: &mut Vec<FormattedTextFragment>,
    ctx: &mut FootnoteContext,
) {
    let mut out: Vec<FormattedTextFragment> = Vec::with_capacity(fragments.len());
    for fragment in fragments.drain(..) {
        if !fragment.text.contains("[^") || fragment.styles.inline_code {
            out.push(fragment);
            continue;
        }
        let original_styles = fragment.styles.clone();
        let mut remaining = fragment.text.as_str();
        let mut buf = String::new();
        while let Some(at) = remaining.find("[^") {
            buf.push_str(&remaining[..at]);
            let after = &remaining[at + 2..];
            if let Some(close) = after.find(']') {
                let id = &after[..close];
                if !id.is_empty()
                    && !id.contains(char::is_whitespace)
                    && ctx.definitions.contains_key(id)
                {
                    if !buf.is_empty() {
                        out.push(FormattedTextFragment {
                            text: std::mem::take(&mut buf),
                            styles: original_styles.clone(),
                        });
                    }
                    let number = match ctx.numbers.get(id) {
                        Some(n) => *n,
                        None => {
                            let n = ctx.used.len() + 1;
                            ctx.numbers.insert(id.to_string(), n);
                            ctx.used.push(ctx.definitions[id].clone());
                            n
                        }
                    };
                    out.push(FormattedTextFragment {
                        text: number.to_string(),
                        styles: FormattedTextStyles {
                            italic: true,
                            hyperlink: Some(Hyperlink::Url(format!("#fn-{id}"))),
                            ..Default::default()
                        },
                    });
                    remaining = &after[close + 1..];
                    continue;
                }
            }
            // Not a defined reference — emit the [^ literally and keep scanning.
            buf.push_str("[^");
            remaining = after;
        }
        buf.push_str(remaining);
        if !buf.is_empty() {
            out.push(FormattedTextFragment {
                text: buf,
                styles: original_styles,
            });
        }
    }
    *fragments = out;
}

/// Append a footnotes section to `text` based on `ctx.used`.
pub(crate) fn append_section(text: &mut FormattedText, ctx: &FootnoteContext) {
    if ctx.used.is_empty() {
        return;
    }
    text.lines.push_back(FormattedTextLine::HorizontalRule);
    for (index, def) in ctx.used.iter().enumerate() {
        let number = index + 1;
        let body_fragments = crate::parse_inline_markdown(&def.content);
        let mut content_fragments: Vec<FormattedTextFragment> = body_fragments.into_iter().collect();
        content_fragments.push(FormattedTextFragment {
            text: " ↩".to_string(),
            styles: FormattedTextStyles {
                hyperlink: Some(Hyperlink::Url(format!("#fnref-{}", def.id))),
                ..Default::default()
            },
        });
        text.lines
            .push_back(FormattedTextLine::OrderedList(OrderedFormattedIndentTextInline {
                number: Some(number),
                indented_text: FormattedIndentTextInline {
                    indent_level: 0,
                    text: content_fragments,
                },
            }));
    }
}

#[cfg(test)]
#[path = "footnotes_tests.rs"]
mod tests;
