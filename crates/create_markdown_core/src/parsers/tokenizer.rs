//! Line-based markdown tokenizer.
//!
//! Ported from `packages/core/src/parsers/tokenizer.ts`. Hand-rolled
//! pattern matchers (no `regex` dependency) mirror the JS upstream's
//! regex shapes; behavior is preserved.

// ============================================================================
// Token Types
// ============================================================================

/// One line's worth of recognized markdown syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Heading,
    Paragraph,
    BulletListItem,
    NumberedListItem,
    CheckListItem,
    CodeFenceStart,
    CodeFenceEnd,
    CodeContent,
    Blockquote,
    Divider,
    TableRow,
    TableSeparator,
    Image,
    Callout,
    Blank,
}

/// A tokenized markdown line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    /// Raw line as it appeared in the input.
    pub raw: String,
    /// Cleaned content for the token (e.g. heading text without `#`s).
    pub content: String,
    /// Number of leading whitespace characters before the syntax marker.
    pub indent: usize,
    /// 1-based line number.
    pub line: usize,
    /// Type-specific metadata.
    pub meta: TokenMeta,
}

/// Optional metadata carried by some tokens.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenMeta {
    pub level: Option<u8>,
    pub list_marker: Option<char>,
    pub list_index: Option<u32>,
    pub checked: Option<bool>,
    pub language: Option<String>,
    pub callout_type: Option<String>,
    /// For standalone images, the alt text.
    pub image_alt: Option<String>,
    /// For standalone images, the optional title from `![alt](url "title")`.
    pub image_title: Option<String>,
}

// ============================================================================
// Pattern Helpers (hand-rolled to avoid a regex dependency)
// ============================================================================

fn leading_whitespace_count(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

fn trim_trailing_whitespace(s: &str) -> &str {
    s.trim_end_matches([' ', '\t'])
}

fn is_blank(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace())
}

/// `^(#{1,6})\s+(.*)$` — ATX heading.
fn match_atx_heading(line: &str) -> Option<(u8, &str)> {
    let bytes = line.as_bytes();
    let mut level = 0usize;
    while level < bytes.len() && bytes[level] == b'#' {
        level += 1;
    }
    if !(1..=6).contains(&level) || level == bytes.len() {
        return None;
    }
    let rest = &line[level..];
    // require at least one whitespace
    let mut rest_chars = rest.chars();
    let first = rest_chars.next()?;
    if !first.is_whitespace() {
        return None;
    }
    Some((level as u8, rest.trim_start()))
}

/// `^=+\s*$` — Setext H1 underline.
fn is_setext_h1(line: &str) -> bool {
    let trimmed = trim_trailing_whitespace(line);
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '=')
}

/// `^-+\s*$` — Setext H2 underline OR thematic break (`---`).
fn is_setext_h2(line: &str) -> bool {
    let trimmed = trim_trailing_whitespace(line);
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '-')
}

/// `^(\s*)([-*+])\s+(.*)$` — bullet list item. Returns (indent, marker, rest).
fn match_bullet_list(line: &str) -> Option<(usize, char, &str)> {
    let indent = leading_whitespace_count(line);
    let after = &line[indent..];
    let mut chars = after.chars();
    let marker = chars.next()?;
    if !matches!(marker, '-' | '*' | '+') {
        return None;
    }
    let next = chars.next()?;
    if !next.is_whitespace() {
        return None;
    }
    let rest_start = indent + marker.len_utf8() + next.len_utf8();
    Some((indent, marker, line[rest_start..].trim_start()))
}

/// `^(\s*)(\d+)\.\s+(.*)$` — numbered list item.
fn match_numbered_list(line: &str) -> Option<(usize, u32, &str)> {
    let indent = leading_whitespace_count(line);
    let after = &line[indent..];
    let bytes = after.as_bytes();
    let mut digit_count = 0usize;
    while digit_count < bytes.len() && bytes[digit_count].is_ascii_digit() {
        digit_count += 1;
    }
    if digit_count == 0 || digit_count >= bytes.len() {
        return None;
    }
    if bytes[digit_count] != b'.' {
        return None;
    }
    let after_dot = &after[digit_count + 1..];
    let mut chars = after_dot.chars();
    let next = chars.next()?;
    if !next.is_whitespace() {
        return None;
    }
    let number: u32 = after[..digit_count].parse().ok()?;
    Some((
        indent,
        number,
        after_dot[next.len_utf8()..].trim_start(),
    ))
}

/// `^(\s*)[-*+]\s+\[([ xX])\]\s+(.*)$` — check list item.
fn match_check_list(line: &str) -> Option<(usize, bool, &str)> {
    let (indent, _marker, rest) = match_bullet_list(line)?;
    let bytes = rest.as_bytes();
    if bytes.len() < 4 || bytes[0] != b'[' || bytes[2] != b']' {
        return None;
    }
    let mark = bytes[1];
    let checked = match mark {
        b'x' | b'X' => true,
        b' ' => false,
        _ => return None,
    };
    let after = &rest[3..];
    let mut chars = after.chars();
    let next = chars.next()?;
    if !next.is_whitespace() {
        return None;
    }
    Some((indent, checked, after[next.len_utf8()..].trim_start()))
}

/// `^(\s*)(`{3,}|~{3,})(\w*)?\s*$` — fenced code block start/end.
fn match_code_fence(line: &str) -> Option<(usize, char, usize, String)> {
    let indent = leading_whitespace_count(line);
    let after = &line[indent..];
    let mut chars = after.chars();
    let fence_char = chars.next()?;
    if fence_char != '`' && fence_char != '~' {
        return None;
    }
    let mut fence_len = 1usize;
    for c in after[fence_char.len_utf8()..].chars() {
        if c == fence_char {
            fence_len += 1;
        } else {
            break;
        }
    }
    if fence_len < 3 {
        return None;
    }
    let after_fence = &after[fence_len * fence_char.len_utf8()..];
    let trimmed = after_fence.trim_end();
    // Language is the leading run of \w chars; the rest of the line must be
    // whitespace (or empty) to match the JS regex.
    let lang: String = trimmed.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    let after_lang = &trimmed[lang.len()..];
    if !after_lang.chars().all(|c| c.is_whitespace()) {
        return None;
    }
    Some((indent, fence_char, fence_len, lang))
}

/// `^>\s?(.*)$` — blockquote. Returns the content after `>` and an
/// optional single whitespace.
fn match_blockquote(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.is_empty() || bytes[0] != b'>' {
        return None;
    }
    let after = &line[1..];
    let first = after.chars().next();
    match first {
        Some(c) if c.is_whitespace() => Some(&after[c.len_utf8()..]),
        _ => Some(after),
    }
}

/// `^>\s*\[!(\w+)\]\s*$` — GitHub-style callout alert header.
fn match_callout(line: &str) -> Option<&str> {
    let after = match_blockquote(line)?;
    let trimmed = after.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() < 4 || bytes[0] != b'[' || bytes[1] != b'!' {
        return None;
    }
    let close = trimmed.find(']')?;
    if close <= 2 {
        return None;
    }
    let kind = &trimmed[2..close];
    if kind.is_empty() || !kind.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let after_close = trimmed[close + 1..].trim();
    if !after_close.is_empty() {
        return None;
    }
    Some(kind)
}

/// `^(\s*)(-{3,}|\*{3,}|_{3,})\s*$` — thematic break / divider.
fn match_divider(line: &str) -> Option<usize> {
    let indent = leading_whitespace_count(line);
    let after = trim_trailing_whitespace(&line[indent..]);
    if after.len() < 3 {
        return None;
    }
    let first = after.chars().next()?;
    if !matches!(first, '-' | '*' | '_') {
        return None;
    }
    if !after.chars().all(|c| c == first) {
        return None;
    }
    Some(indent)
}

/// `^\|(.+)\|$` — table data row. Returns the inner content.
fn match_table_row(line: &str) -> Option<&str> {
    let trimmed = line.trim_end();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 3 {
        return None;
    }
    Some(&trimmed[1..trimmed.len() - 1])
}

/// `^\|[\s\-:|]+\|$` — table separator row.
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim_end();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') || trimmed.len() < 3 {
        return false;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c == ' ' || c == '\t' || c == '-' || c == ':' || c == '|')
}

/// `^!\[([^\]]*)\]\(([^)]+)\)$` — standalone image. Returns (alt, url).
fn match_image(line: &str) -> Option<(&str, &str, Option<&str>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("![") || !trimmed.ends_with(')') {
        return None;
    }
    let alt_end = trimmed.find(']')?;
    if alt_end <= 1 {
        return None;
    }
    let alt = &trimmed[2..alt_end];
    let after = &trimmed[alt_end + 1..];
    if !after.starts_with('(') {
        return None;
    }
    // Content between the outer parens (the closing ')' has been stripped).
    let paren_inner = &after[1..after.len() - 1];
    if paren_inner.is_empty() {
        return None;
    }
    // Detect an optional title: `url "title"` or `url 'title'`.
    // A title is present only when paren_inner ends with a quoted string
    // AND there is a space between the URL and the opening quote.
    let (url, title) = parse_url_and_title(paren_inner);
    if url.is_empty() {
        return None;
    }
    Some((alt, url, title))
}

/// Split `url "title"` or `url 'title'` into `(url, Some(title))`.
/// Returns `(inner, None)` when no title is present.
fn parse_url_and_title(inner: &str) -> (&str, Option<&str>) {
    // A title must be wrapped in matching quotes and preceded by whitespace.
    for &q in &['"', '\''] {
        if inner.ends_with(q) {
            // Find the *opening* quote of the title, which must be preceded
            // by at least one ASCII space.
            if let Some(open_pos) = inner[..inner.len() - 1].rfind(q) {
                // Verify the character just before the opening quote is whitespace.
                if open_pos > 0 {
                    let before = inner.as_bytes()[open_pos - 1];
                    if before == b' ' || before == b'\t' {
                        let url_part = inner[..open_pos].trim_end();
                        let title_part = &inner[open_pos + 1..inner.len() - 1];
                        if !url_part.is_empty() {
                            return (url_part, Some(title_part));
                        }
                    }
                }
            }
        }
    }
    (inner, None)
}

// ============================================================================
// Tokenizer Public API
// ============================================================================

struct State {
    in_code_block: bool,
    code_block_fence_char: char,
    code_block_fence_len: usize,
}

/// Tokenize a markdown string into a vec of [`Token`]s, one per non-folded
/// line. CRLF and CR line endings are normalized to LF before splitting.
pub fn tokenize(markdown: &str) -> Vec<Token> {
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let mut tokens = Vec::new();
    let mut state = State {
        in_code_block: false,
        code_block_fence_char: '`',
        code_block_fence_len: 0,
    };

    for (i, line) in lines.iter().enumerate() {
        if let Some(tok) = tokenize_line(line, &mut state, &lines, i) {
            tokens.push(tok);
        }
    }
    tokens
}

fn tokenize_line(
    line: &str,
    state: &mut State,
    all_lines: &[&str],
    line_index: usize,
) -> Option<Token> {
    let line_number = line_index + 1;

    // Inside a fenced code block — only check for a closing fence of the
    // same kind, otherwise treat as code content.
    if state.in_code_block {
        if let Some((_, ch, len, _)) = match_code_fence(line)
            && ch == state.code_block_fence_char && len >= state.code_block_fence_len {
                state.in_code_block = false;
                state.code_block_fence_len = 0;
                return Some(Token {
                    token_type: TokenType::CodeFenceEnd,
                    raw: line.to_string(),
                    content: String::new(),
                    indent: 0,
                    line: line_number,
                    meta: TokenMeta::default(),
                });
            }
        return Some(Token {
            token_type: TokenType::CodeContent,
            raw: line.to_string(),
            content: line.to_string(),
            indent: 0,
            line: line_number,
            meta: TokenMeta::default(),
        });
    }

    // Code fence start
    if let Some((indent, ch, len, lang)) = match_code_fence(line) {
        state.in_code_block = true;
        state.code_block_fence_char = ch;
        state.code_block_fence_len = len;
        return Some(Token {
            token_type: TokenType::CodeFenceStart,
            raw: line.to_string(),
            content: String::new(),
            indent,
            line: line_number,
            meta: TokenMeta {
                language: Some(lang),
                ..Default::default()
            },
        });
    }

    // Blank
    if is_blank(line) {
        return Some(Token {
            token_type: TokenType::Blank,
            raw: line.to_string(),
            content: String::new(),
            indent: 0,
            line: line_number,
            meta: TokenMeta::default(),
        });
    }

    // Divider (before setext / paragraph)
    if let Some(indent) = match_divider(line) {
        let prev_line = if line_index > 0 { all_lines[line_index - 1] } else { "" };
        let prev_nonblank = !prev_line.trim().is_empty();
        // If the previous non-blank line is paragraph text, this `---` is
        // a setext H2 underline — defer to the setext branch below.
        if !(prev_nonblank && is_setext_h2(line)) {
            return Some(Token {
                token_type: TokenType::Divider,
                raw: line.to_string(),
                content: String::new(),
                indent,
                line: line_number,
                meta: TokenMeta::default(),
            });
        }
    }

    // ATX heading
    if let Some((level, content)) = match_atx_heading(line) {
        return Some(Token {
            token_type: TokenType::Heading,
            raw: line.to_string(),
            content: content.trim().to_string(),
            indent: 0,
            line: line_number,
            meta: TokenMeta {
                level: Some(level),
                ..Default::default()
            },
        });
    }

    // Callout (must come before generic blockquote)
    if let Some(kind) = match_callout(line) {
        return Some(Token {
            token_type: TokenType::Callout,
            raw: line.to_string(),
            content: String::new(),
            indent: 0,
            line: line_number,
            meta: TokenMeta {
                callout_type: Some(kind.to_ascii_lowercase()),
                ..Default::default()
            },
        });
    }

    // Blockquote
    if let Some(content) = match_blockquote(line) {
        return Some(Token {
            token_type: TokenType::Blockquote,
            raw: line.to_string(),
            content: content.to_string(),
            indent: 0,
            line: line_number,
            meta: TokenMeta::default(),
        });
    }

    // Check list (before bullet list)
    if let Some((indent, checked, content)) = match_check_list(line) {
        return Some(Token {
            token_type: TokenType::CheckListItem,
            raw: line.to_string(),
            content: content.to_string(),
            indent,
            line: line_number,
            meta: TokenMeta {
                checked: Some(checked),
                ..Default::default()
            },
        });
    }

    // Bullet list
    if let Some((indent, marker, content)) = match_bullet_list(line) {
        return Some(Token {
            token_type: TokenType::BulletListItem,
            raw: line.to_string(),
            content: content.to_string(),
            indent,
            line: line_number,
            meta: TokenMeta {
                list_marker: Some(marker),
                ..Default::default()
            },
        });
    }

    // Numbered list
    if let Some((indent, number, content)) = match_numbered_list(line) {
        return Some(Token {
            token_type: TokenType::NumberedListItem,
            raw: line.to_string(),
            content: content.to_string(),
            indent,
            line: line_number,
            meta: TokenMeta {
                list_index: Some(number),
                ..Default::default()
            },
        });
    }

    // Table separator (before table row)
    if is_table_separator(line) {
        return Some(Token {
            token_type: TokenType::TableSeparator,
            raw: line.to_string(),
            content: line.to_string(),
            indent: 0,
            line: line_number,
            meta: TokenMeta::default(),
        });
    }

    // Table row
    if let Some(inner) = match_table_row(line) {
        return Some(Token {
            token_type: TokenType::TableRow,
            raw: line.to_string(),
            content: inner.to_string(),
            indent: 0,
            line: line_number,
            meta: TokenMeta::default(),
        });
    }

    // Standalone image
    if let Some((alt, url, title)) = match_image(line) {
        return Some(Token {
            token_type: TokenType::Image,
            raw: line.to_string(),
            content: url.to_string(),
            indent: 0,
            line: line_number,
            meta: TokenMeta {
                image_alt: Some(alt.to_string()),
                image_title: title.map(str::to_string),
                ..Default::default()
            },
        });
    }

    // Setext heading underline (paragraph → heading promotion happens in
    // the block parser; the tokenizer just records the underline as a
    // Heading token whose content holds the *previous* line's text).
    if is_setext_h1(line) || is_setext_h2(line) {
        let prev_line = if line_index > 0 { all_lines[line_index - 1] } else { "" };
        if !prev_line.trim().is_empty() {
            return Some(Token {
                token_type: TokenType::Heading,
                raw: line.to_string(),
                content: prev_line.trim().to_string(),
                indent: 0,
                line: line_number,
                meta: TokenMeta {
                    level: Some(if is_setext_h1(line) { 1 } else { 2 }),
                    ..Default::default()
                },
            });
        }
        return Some(Token {
            token_type: TokenType::Divider,
            raw: line.to_string(),
            content: String::new(),
            indent: 0,
            line: line_number,
            meta: TokenMeta::default(),
        });
    }

    // Default: paragraph
    let indent = leading_whitespace_count(line);
    Some(Token {
        token_type: TokenType::Paragraph,
        raw: line.to_string(),
        content: line.trim().to_string(),
        indent,
        line: line_number,
        meta: TokenMeta::default(),
    })
}

// ============================================================================
// Token Utilities
// ============================================================================

/// Groups consecutive tokens of the same type, splitting on blank lines.
/// Mirrors the JS `groupTokens`.
pub fn group_tokens(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut groups: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    let mut last_type: Option<TokenType> = None;

    for tok in tokens {
        if tok.token_type == TokenType::Blank {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            last_type = None;
            continue;
        }

        if last_type == Some(tok.token_type) {
            current.push(tok.clone());
        } else {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            current.push(tok.clone());
            last_type = Some(tok.token_type);
        }
    }

    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

/// Returns `true` for any list-item token type.
pub fn is_list_token(token: &Token) -> bool {
    matches!(
        token.token_type,
        TokenType::BulletListItem | TokenType::NumberedListItem | TokenType::CheckListItem
    )
}

/// Returns `true` for any code-block-related token type.
pub fn is_code_token(token: &Token) -> bool {
    matches!(
        token.token_type,
        TokenType::CodeFenceStart | TokenType::CodeFenceEnd | TokenType::CodeContent
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(tokens: &[Token]) -> Vec<TokenType> {
        tokens.iter().map(|t| t.token_type).collect()
    }

    #[test]
    fn atx_heading_levels_1_through_6() {
        for level in 1..=6u8 {
            let line = format!("{} title", "#".repeat(level as usize));
            let toks = tokenize(&line);
            assert_eq!(toks.len(), 1);
            assert_eq!(toks[0].token_type, TokenType::Heading);
            assert_eq!(toks[0].meta.level, Some(level));
            assert_eq!(toks[0].content, "title");
        }
    }

    #[test]
    fn atx_seven_hashes_is_paragraph() {
        let toks = tokenize("####### too deep");
        assert_eq!(toks[0].token_type, TokenType::Paragraph);
    }

    #[test]
    fn paragraph_defaults() {
        let toks = tokenize("just text");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].token_type, TokenType::Paragraph);
        assert_eq!(toks[0].content, "just text");
    }

    #[test]
    fn blank_line_token() {
        let toks = tokenize("\n");
        assert_eq!(types(&toks), vec![TokenType::Blank, TokenType::Blank]);
    }

    #[test]
    fn bullet_list_items_with_indent() {
        let toks = tokenize("- one\n  - two");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].token_type, TokenType::BulletListItem);
        assert_eq!(toks[0].indent, 0);
        assert_eq!(toks[0].content, "one");
        assert_eq!(toks[1].token_type, TokenType::BulletListItem);
        assert_eq!(toks[1].indent, 2);
    }

    #[test]
    fn numbered_list_extracts_number() {
        let toks = tokenize("1. first\n2. second");
        assert_eq!(toks[0].token_type, TokenType::NumberedListItem);
        assert_eq!(toks[0].meta.list_index, Some(1));
        assert_eq!(toks[1].meta.list_index, Some(2));
    }

    #[test]
    fn check_list_extracts_checked() {
        let toks = tokenize("- [x] done\n- [ ] todo");
        assert_eq!(toks[0].token_type, TokenType::CheckListItem);
        assert_eq!(toks[0].meta.checked, Some(true));
        assert_eq!(toks[1].meta.checked, Some(false));
    }

    #[test]
    fn code_fence_round_trip() {
        let toks = tokenize("```rust\nlet x = 1;\n```");
        assert_eq!(toks.len(), 3);
        assert_eq!(toks[0].token_type, TokenType::CodeFenceStart);
        assert_eq!(toks[0].meta.language.as_deref(), Some("rust"));
        assert_eq!(toks[1].token_type, TokenType::CodeContent);
        assert_eq!(toks[1].content, "let x = 1;");
        assert_eq!(toks[2].token_type, TokenType::CodeFenceEnd);
    }

    #[test]
    fn callout_before_blockquote() {
        let toks = tokenize("> [!INFO]\n> body");
        assert_eq!(toks[0].token_type, TokenType::Callout);
        assert_eq!(toks[0].meta.callout_type.as_deref(), Some("info"));
        assert_eq!(toks[1].token_type, TokenType::Blockquote);
        assert_eq!(toks[1].content, "body");
    }

    #[test]
    fn divider_three_dashes() {
        let toks = tokenize("---");
        assert_eq!(toks[0].token_type, TokenType::Divider);
    }

    #[test]
    fn setext_h1_promotes_previous_line() {
        let toks = tokenize("Title\n=====");
        // First line is paragraph; second line is heading whose content
        // points back at "Title".
        assert_eq!(toks[0].token_type, TokenType::Paragraph);
        assert_eq!(toks[1].token_type, TokenType::Heading);
        assert_eq!(toks[1].meta.level, Some(1));
        assert_eq!(toks[1].content, "Title");
    }

    #[test]
    fn setext_h2_after_paragraph() {
        let toks = tokenize("Sub\n---");
        assert_eq!(toks[1].token_type, TokenType::Heading);
        assert_eq!(toks[1].meta.level, Some(2));
    }

    #[test]
    fn table_row_and_separator() {
        let toks = tokenize("| a | b |\n| - | - |\n| 1 | 2 |");
        assert_eq!(toks[0].token_type, TokenType::TableRow);
        assert_eq!(toks[1].token_type, TokenType::TableSeparator);
        assert_eq!(toks[2].token_type, TokenType::TableRow);
    }

    #[test]
    fn standalone_image() {
        let toks = tokenize("![alt](https://x)");
        assert_eq!(toks[0].token_type, TokenType::Image);
        assert_eq!(toks[0].content, "https://x");
        assert_eq!(toks[0].meta.image_alt.as_deref(), Some("alt"));
    }

    #[test]
    fn crlf_normalized() {
        let toks = tokenize("a\r\nb");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].content, "a");
        assert_eq!(toks[1].content, "b");
    }

    #[test]
    fn group_tokens_splits_on_blank() {
        let toks = tokenize("a\nb\n\nc");
        let groups = group_tokens(&toks);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[1].len(), 1);
    }

    #[test]
    fn is_list_and_code_classifiers() {
        let toks = tokenize("- x\n```\nfoo\n```");
        assert!(is_list_token(&toks[0]));
        assert!(is_code_token(&toks[1]));
        assert!(is_code_token(&toks[2]));
        assert!(is_code_token(&toks[3]));
    }
}
