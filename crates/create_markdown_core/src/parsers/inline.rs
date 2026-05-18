//! Inline markdown parser — converts a single line of markdown text to a
//! `Vec<TextSpan>` with inline styles applied.
//!
//! Ported from `packages/core/src/parsers/inline.ts`. Recognizes inline code
//! (`` `…` ``), links (`[text](url)`) and images (`![alt](url)`), emphasis
//! (`*…*`, `**…**`, `***…***`, `_…_`, `__…__`, `___…___`), strikethrough
//! (`~~…~~`), highlight (`==…==`), and `\` escapes.

use crate::types::{InlineStyle, LinkData, TextSpan};

// ============================================================================
// Internal Token Tree
// ============================================================================

#[derive(Debug, Clone)]
enum InlineToken {
    Text(String),
    Bold(Vec<InlineToken>),
    Italic(Vec<InlineToken>),
    Code(String),
    Strikethrough(Vec<InlineToken>),
    Highlight(Vec<InlineToken>),
    Link {
        url: String,
        title: Option<String>,
        children: Vec<InlineToken>,
    },
    Image {
        alt: String,
        url: String,
        title: Option<String>,
    },
}

// ============================================================================
// Public Entry Points
// ============================================================================

/// Parses inline markdown into a vec of [`TextSpan`]s with inline styles
/// applied. Empty input returns an empty vec.
pub fn parse_inline(text: &str) -> Vec<TextSpan> {
    if text.is_empty() {
        return Vec::new();
    }
    let tokens = parse_inline_tokens(text);
    let spans = tokens_to_spans(&tokens);
    merge_adjacent_spans(spans)
}

/// Strips inline formatting and returns the plain-text concatenation.
pub fn extract_plain_text(text: &str) -> String {
    parse_inline(text).into_iter().map(|s| s.text).collect()
}

// ============================================================================
// Tokenizer
// ============================================================================

fn parse_inline_tokens(text: &str) -> Vec<InlineToken> {
    let bytes = text.as_bytes();
    let mut tokens: Vec<InlineToken> = Vec::new();
    let mut buffer = String::new();
    let mut i = 0;

    let flush = |buffer: &mut String, tokens: &mut Vec<InlineToken>| {
        if !buffer.is_empty() {
            tokens.push(InlineToken::Text(std::mem::take(buffer)));
        }
    };

    while i < bytes.len() {
        let b = bytes[i];

        if b == b'\\' && i + 1 < bytes.len() {
            let next_char_len = char_len_at(bytes, i + 1);
            buffer.push_str(&text[i + 1..i + 1 + next_char_len]);
            i += 1 + next_char_len;
            continue;
        }

        if b == b'`'
            && let Some((tok, end)) = match_code(bytes, text, i) {
                flush(&mut buffer, &mut tokens);
                tokens.push(tok);
                i = end;
                continue;
            }

        if b == b'!' && i + 1 < bytes.len() && bytes[i + 1] == b'['
            && let Some((tok, end)) = match_link_or_image(bytes, text, i + 1, true) {
                flush(&mut buffer, &mut tokens);
                tokens.push(tok);
                i = end;
                continue;
            }

        if b == b'['
            && let Some((tok, end)) = match_link_or_image(bytes, text, i, false) {
                flush(&mut buffer, &mut tokens);
                tokens.push(tok);
                i = end;
                continue;
            }

        if (b == b'*' || b == b'_')
            && let Some((tok, end)) = match_emphasis(bytes, text, i) {
                flush(&mut buffer, &mut tokens);
                tokens.push(tok);
                i = end;
                continue;
            }

        if b == b'~' && i + 1 < bytes.len() && bytes[i + 1] == b'~'
            && let Some((tok, end)) = match_strikethrough(bytes, text, i) {
                flush(&mut buffer, &mut tokens);
                tokens.push(tok);
                i = end;
                continue;
            }

        if b == b'=' && i + 1 < bytes.len() && bytes[i + 1] == b'='
            && let Some((tok, end)) = match_highlight(bytes, text, i) {
                flush(&mut buffer, &mut tokens);
                tokens.push(tok);
                i = end;
                continue;
            }

        let len = char_len_at(bytes, i);
        buffer.push_str(&text[i..i + len]);
        i += len;
    }

    flush(&mut buffer, &mut tokens);
    tokens
}

fn char_len_at(bytes: &[u8], i: usize) -> usize {
    let b = bytes[i];
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

// ============================================================================
// Pattern Matchers
// ============================================================================

fn match_code(bytes: &[u8], text: &str, start: usize) -> Option<(InlineToken, usize)> {
    let mut backticks = 0usize;
    let mut i = start;
    while i < bytes.len() && bytes[i] == b'`' {
        backticks += 1;
        i += 1;
    }
    if backticks == 0 {
        return None;
    }
    let mut search = i;
    while search < bytes.len() {
        if bytes[search] != b'`' {
            search += 1;
            continue;
        }
        let mut run = 0usize;
        let mut j = search;
        while j < bytes.len() && bytes[j] == b'`' {
            run += 1;
            j += 1;
        }
        if run == backticks {
            let content = text[i..search].trim().to_string();
            return Some((InlineToken::Code(content), j));
        }
        search = j;
    }
    None
}

fn match_link_or_image(
    bytes: &[u8],
    text: &str,
    bracket_pos: usize,
    is_image: bool,
) -> Option<(InlineToken, usize)> {
    if bytes.get(bracket_pos) != Some(&b'[') {
        return None;
    }
    let mut depth = 1usize;
    let mut i = bracket_pos + 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => depth -= 1,
            b'\\' => i += 1,
            _ => {}
        }
        if depth == 0 {
            break;
        }
        i += 1;
    }
    if depth != 0 || i >= bytes.len() {
        return None;
    }
    let close_bracket = i;
    let after_bracket = close_bracket + 1;
    if bytes.get(after_bracket) != Some(&b'(') {
        return None;
    }

    let url_start = after_bracket + 1;
    let mut url_end = url_start;
    let mut paren_depth = 1usize;
    while url_end < bytes.len() && paren_depth > 0 {
        match bytes[url_end] {
            b'(' => paren_depth += 1,
            b')' => paren_depth -= 1,
            b'\\' => url_end += 1,
            _ => {}
        }
        if paren_depth == 0 {
            break;
        }
        url_end += 1;
    }
    if paren_depth != 0 {
        return None;
    }
    let inner = text[url_start..url_end].trim();
    let (url, title) = parse_url_and_title(inner);
    let bracket_text = &text[bracket_pos + 1..close_bracket];

    let token = if is_image {
        InlineToken::Image {
            alt: bracket_text.to_string(),
            url,
            title,
        }
    } else {
        InlineToken::Link {
            url,
            title,
            children: parse_inline_tokens(bracket_text),
        }
    };
    Some((token, url_end + 1))
}

fn parse_url_and_title(inner: &str) -> (String, Option<String>) {
    let bytes = inner.as_bytes();
    if bytes.len() < 4 {
        return (inner.to_string(), None);
    }
    let last = bytes[bytes.len() - 1];
    if last != b'"' && last != b'\'' {
        return (inner.to_string(), None);
    }
    let quote = last;
    let mut i = bytes.len() - 2;
    while i > 0 {
        if bytes[i] == quote {
            let before = bytes[i - 1];
            if before == b' ' || before == b'\t' {
                let url = inner[..i].trim_end();
                let title = &inner[i + 1..bytes.len() - 1];
                return (url.to_string(), Some(title.to_string()));
            }
        }
        i -= 1;
    }
    (inner.to_string(), None)
}

fn match_emphasis(bytes: &[u8], text: &str, start: usize) -> Option<(InlineToken, usize)> {
    let marker = bytes[start];
    if marker != b'*' && marker != b'_' {
        return None;
    }
    let mut count = 0usize;
    let mut i = start;
    while i < bytes.len() && bytes[i] == marker && count < 3 {
        count += 1;
        i += 1;
    }
    if count == 0 {
        return None;
    }
    let mut search = i;
    while search < bytes.len() {
        if bytes[search] != marker {
            search += 1;
            continue;
        }
        let mut run = 0usize;
        let mut j = search;
        while j < bytes.len() && bytes[j] == marker {
            run += 1;
            j += 1;
        }
        if run < count {
            search = j;
            continue;
        }
        let close_at = j - count;
        if close_at == 0 || bytes[close_at - 1] == b' ' {
            search = j;
            continue;
        }
        let content = &text[i..close_at];
        if content.trim().is_empty() {
            search = j;
            continue;
        }
        let children = parse_inline_tokens(content);
        let token = match count {
            3 => InlineToken::Bold(vec![InlineToken::Italic(children)]),
            2 => InlineToken::Bold(children),
            _ => InlineToken::Italic(children),
        };
        return Some((token, close_at + count));
    }
    None
}

fn match_strikethrough(bytes: &[u8], text: &str, start: usize) -> Option<(InlineToken, usize)> {
    if start + 1 >= bytes.len() || bytes[start] != b'~' || bytes[start + 1] != b'~' {
        return None;
    }
    let inner_start = start + 2;
    let mut i = inner_start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'~' && bytes[i + 1] == b'~' {
            let content = &text[inner_start..i];
            if content.trim().is_empty() {
                i += 1;
                continue;
            }
            return Some((
                InlineToken::Strikethrough(parse_inline_tokens(content)),
                i + 2,
            ));
        }
        i += 1;
    }
    None
}

fn match_highlight(bytes: &[u8], text: &str, start: usize) -> Option<(InlineToken, usize)> {
    if start + 1 >= bytes.len() || bytes[start] != b'=' || bytes[start + 1] != b'=' {
        return None;
    }
    let inner_start = start + 2;
    let mut i = inner_start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'=' && bytes[i + 1] == b'=' {
            let content = &text[inner_start..i];
            if content.trim().is_empty() {
                i += 1;
                continue;
            }
            return Some((
                InlineToken::Highlight(parse_inline_tokens(content)),
                i + 2,
            ));
        }
        i += 1;
    }
    None
}

// ============================================================================
// Token Tree → TextSpans
// ============================================================================

fn tokens_to_spans(tokens: &[InlineToken]) -> Vec<TextSpan> {
    let mut out = Vec::new();
    for tok in tokens {
        push_token_spans(tok, &InlineStyle::default(), &mut out);
    }
    out
}

fn push_token_spans(token: &InlineToken, inherited: &InlineStyle, out: &mut Vec<TextSpan>) {
    match token {
        InlineToken::Text(text) => out.push(TextSpan {
            text: text.clone(),
            styles: inherited.clone(),
        }),
        InlineToken::Bold(children) => {
            let mut styles = inherited.clone();
            styles.bold = true;
            for c in children {
                push_token_spans(c, &styles, out);
            }
        }
        InlineToken::Italic(children) => {
            let mut styles = inherited.clone();
            styles.italic = true;
            for c in children {
                push_token_spans(c, &styles, out);
            }
        }
        InlineToken::Code(text) => {
            let mut styles = inherited.clone();
            styles.code = true;
            out.push(TextSpan {
                text: text.clone(),
                styles,
            });
        }
        InlineToken::Strikethrough(children) => {
            let mut styles = inherited.clone();
            styles.strikethrough = true;
            for c in children {
                push_token_spans(c, &styles, out);
            }
        }
        InlineToken::Highlight(children) => {
            let mut styles = inherited.clone();
            styles.highlight = true;
            for c in children {
                push_token_spans(c, &styles, out);
            }
        }
        InlineToken::Link {
            url,
            title,
            children,
        } => {
            let mut styles = inherited.clone();
            styles.link = Some(LinkData {
                url: url.clone(),
                title: title.clone(),
            });
            for c in children {
                push_token_spans(c, &styles, out);
            }
        }
        InlineToken::Image { alt, .. } => {
            out.push(TextSpan {
                text: format!("[image: {alt}]"),
                styles: inherited.clone(),
            });
        }
    }
}

fn merge_adjacent_spans(spans: Vec<TextSpan>) -> Vec<TextSpan> {
    let mut merged: Vec<TextSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(prev) = merged.last_mut()
            && styles_equal(&prev.styles, &span.styles) {
                prev.text.push_str(&span.text);
                continue;
            }
        merged.push(span);
    }
    merged
}

fn styles_equal(a: &InlineStyle, b: &InlineStyle) -> bool {
    a.bold == b.bold
        && a.italic == b.italic
        && a.underline == b.underline
        && a.strikethrough == b.strikethrough
        && a.code == b.code
        && a.highlight == b.highlight
        && a.link == b.link
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<TextSpan> {
        parse_inline(text)
    }

    #[test]
    fn empty_returns_empty() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn plain_text_passthrough() {
        let s = parse("hello world");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "hello world");
        assert!(s[0].styles.is_plain());
    }

    #[test]
    fn bold_with_asterisks() {
        let s = parse("**loud**");
        assert_eq!(s.len(), 1);
        assert!(s[0].styles.bold);
    }

    #[test]
    fn bold_with_underscores() {
        let s = parse("__loud__");
        assert!(s[0].styles.bold);
    }

    #[test]
    fn italic_with_asterisks() {
        let s = parse("*soft*");
        assert!(s[0].styles.italic);
    }

    #[test]
    fn bold_italic_triple_marker() {
        let s = parse("***both***");
        assert!(s[0].styles.bold);
        assert!(s[0].styles.italic);
    }

    #[test]
    fn inline_code_preserves_text() {
        let s = parse("here is `code` inline");
        assert_eq!(s.len(), 3);
        assert_eq!(s[1].text, "code");
        assert!(s[1].styles.code);
    }

    #[test]
    fn strikethrough() {
        let s = parse("~~gone~~");
        assert!(s[0].styles.strikethrough);
    }

    #[test]
    fn highlight() {
        let s = parse("==mark==");
        assert!(s[0].styles.highlight);
    }

    #[test]
    fn link_basic() {
        let s = parse("[Anthropic](https://anthropic.com)");
        let link = s[0].styles.link.as_ref().unwrap();
        assert_eq!(link.url, "https://anthropic.com");
    }

    #[test]
    fn link_with_title() {
        let s = parse("[A](https://a \"home\")");
        let link = s[0].styles.link.as_ref().unwrap();
        assert_eq!(link.url, "https://a");
        assert_eq!(link.title.as_deref(), Some("home"));
    }

    #[test]
    fn image_renders_placeholder() {
        let s = parse("![alt](https://x)");
        assert_eq!(s[0].text, "[image: alt]");
    }

    #[test]
    fn nested_bold_italic_via_children() {
        let s = parse("**bold _and italic_**");
        assert!(s.iter().any(|sp| sp.styles.bold && !sp.styles.italic));
        assert!(s.iter().any(|sp| sp.styles.bold && sp.styles.italic));
    }

    #[test]
    fn escape_backslash_treats_next_char_literally() {
        let s = parse(r"\*not bold\*");
        assert_eq!(s[0].text, "*not bold*");
    }

    #[test]
    fn unclosed_marker_treated_as_literal() {
        let s = parse("*just a star");
        assert_eq!(s[0].text, "*just a star");
    }

    #[test]
    fn merge_unit_collapses_same_style_neighbors() {
        let style = InlineStyle {
            bold: true,
            ..Default::default()
        };
        let merged = merge_adjacent_spans(vec![
            TextSpan {
                text: "a".into(),
                styles: style.clone(),
            },
            TextSpan {
                text: "b".into(),
                styles: style.clone(),
            },
            TextSpan {
                text: "c".into(),
                styles: InlineStyle::default(),
            },
        ]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].text, "ab");
    }

    #[test]
    fn extract_plain_text_strips_styles() {
        assert_eq!(extract_plain_text("**bold** and `code`"), "bold and code");
    }

    #[test]
    fn utf8_text_preserved() {
        let s = parse("héllo *wörld*");
        assert!(s.iter().any(|sp| sp.text == "wörld" && sp.styles.italic));
    }
}
