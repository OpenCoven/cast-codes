//! GFM-style HTML span lexing for the markdown parser.
//!
//! `try_lex_html_span` recognizes a single HTML span starting at the current
//! input position and returns it together with the remaining input. Tags are
//! classified against a hardcoded safe-list / strip-list / block-list.
//!
//! This module does NOT parse HTML structure — that work is delegated to
//! `html_parser::parse_html_inline_fragments` and
//! `html_parser::parse_html_block_lines`, which both use `html5ever`.

pub(crate) const PHRASING_SAFE_TAGS: &[&str] = &[
    "a", "b", "br", "code", "del", "em", "i", "ins", "kbd",
    "mark", "q", "s", "small", "span", "strong", "sub", "sup", "u",
];

pub(crate) const BLOCK_SAFE_TAGS: &[&str] = &[
    "blockquote", "caption", "dd", "details", "div", "dl", "dt",
    "h1", "h2", "h3", "h4", "h5", "h6",
    "hr", "img", "li", "ol", "p", "pre", "summary",
    "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul",
];

pub(crate) const STRIPPED_TAGS: &[&str] = &[
    "applet", "body", "button", "embed", "form", "frame", "frameset",
    "head", "html", "iframe", "input", "link", "meta", "noscript",
    "object", "script", "style", "title",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HtmlSpanKind {
    PhrasingSafe,
    BlockSafe,
    Stripped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HtmlSpan<'a> {
    /// The entire matched span, including `<tag …>` … `</tag>` (or just the open tag if
    /// self-closing or unmatched).
    pub(crate) raw: &'a str,
    /// Lowercased tag name (e.g. `"b"`, `"div"`). Empty string for HTML comments.
    pub(crate) tag: String,
    pub(crate) kind: HtmlSpanKind,
}

pub(crate) fn classify(tag: &str) -> HtmlSpanKind {
    let lower = tag.to_ascii_lowercase();
    if PHRASING_SAFE_TAGS.iter().any(|t| *t == lower) {
        HtmlSpanKind::PhrasingSafe
    } else if BLOCK_SAFE_TAGS.iter().any(|t| *t == lower) {
        HtmlSpanKind::BlockSafe
    } else if STRIPPED_TAGS.iter().any(|t| *t == lower) {
        HtmlSpanKind::Stripped
    } else {
        HtmlSpanKind::Unknown
    }
}

pub(crate) fn try_lex_html_span(input: &str) -> Option<(HtmlSpan<'_>, &str)> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'<') {
        return None;
    }
    // HTML comment: <!-- … -->
    if let Some(after_open) = input.strip_prefix("<!--") {
        let end = after_open
            .find("-->")
            .map(|i| input.len() - after_open.len() + i + 3)
            .unwrap_or(input.len());
        return Some((
            HtmlSpan { raw: &input[..end], tag: String::new(), kind: HtmlSpanKind::Stripped },
            &input[end..],
        ));
    }
    let mut idx = 1;
    let is_close = bytes.get(idx) == Some(&b'/');
    if is_close {
        idx += 1;
    }
    let tag_start = idx;
    while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'-') {
        idx += 1;
    }
    if idx == tag_start {
        return None; // not a tag
    }
    let tag = input[tag_start..idx].to_ascii_lowercase();
    // For closing tags we have no body to scan; just find '>'.
    if is_close {
        let close_offset = input[idx..].find('>')?;
        let after = idx + close_offset + 1;
        return Some((
            HtmlSpan { raw: &input[..after], tag: tag.clone(), kind: classify(&tag) },
            &input[after..],
        ));
    }
    // Skip the open-tag's attributes. We need to honor quoted attribute values
    // so we don't mistake `'>'` inside a quote for end-of-tag.
    let mut quote: Option<u8> = None;
    let mut self_closing = false;
    while idx < bytes.len() {
        let c = bytes[idx];
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, b'"') | (None, b'\'') => quote = Some(c),
            (None, b'/') if bytes.get(idx + 1) == Some(&b'>') => {
                self_closing = true;
                idx += 2;
                break;
            }
            (None, b'>') => {
                idx += 1;
                break;
            }
            _ => {}
        }
        idx += 1;
    }
    // Void / self-closing elements end here.
    let void_tags = ["br", "hr", "img", "input", "meta", "link"];
    if self_closing || void_tags.contains(&tag.as_str()) {
        return Some((
            HtmlSpan { raw: &input[..idx], tag: tag.clone(), kind: classify(&tag) },
            &input[idx..],
        ));
    }
    // Scan for matching close tag, honoring nested same-tag pairs.
    let mut depth = 1usize;
    let mut scan = idx;
    while scan < bytes.len() {
        let Some(rel) = input[scan..].find('<') else { break };
        scan += rel;
        if input[scan..].starts_with("<!--") {
            scan = match input[scan + 4..].find("-->") {
                Some(i) => scan + 4 + i + 3,
                None => bytes.len(),
            };
            continue;
        }
        let is_close_here = bytes.get(scan + 1) == Some(&b'/');
        let name_start = scan + if is_close_here { 2 } else { 1 };
        let mut name_end = name_start;
        while name_end < bytes.len()
            && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'-')
        {
            name_end += 1;
        }
        let same = input[name_start..name_end].eq_ignore_ascii_case(tag.as_str());
        let close_rel = match input[name_end..].find('>') {
            Some(i) => i + 1,
            None => bytes.len() - name_end,
        };
        let after_tag = name_end + close_rel;
        if same {
            if is_close_here {
                depth -= 1;
                if depth == 0 {
                    return Some((
                        HtmlSpan { raw: &input[..after_tag], tag: tag.clone(), kind: classify(&tag) },
                        &input[after_tag..],
                    ));
                }
            } else if !void_tags.contains(&tag.as_str()) {
                depth += 1;
            }
        }
        scan = after_tag;
    }
    // No matching close; treat the open tag alone as the span.
    Some((
        HtmlSpan { raw: &input[..idx], tag: tag.clone(), kind: classify(&tag) },
        &input[idx..],
    ))
}

#[cfg(test)]
#[path = "gfm_html_tests.rs"]
mod tests;
