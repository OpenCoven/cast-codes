use super::*;

#[test]
fn lex_simple_open_close() {
    let (span, rest) = try_lex_html_span("<b>hi</b> rest").unwrap();
    assert_eq!(span.raw, "<b>hi</b>");
    assert_eq!(span.tag, "b");
    assert_eq!(span.kind, HtmlSpanKind::PhrasingSafe);
    assert_eq!(rest, " rest");
}

#[test]
fn lex_self_closing() {
    let (span, rest) = try_lex_html_span("<br/>after").unwrap();
    assert_eq!(span.raw, "<br/>");
    assert_eq!(rest, "after");
}

#[test]
fn lex_void_no_slash() {
    let (span, rest) = try_lex_html_span("<br>after").unwrap();
    assert_eq!(span.raw, "<br>");
    assert_eq!(rest, "after");
}

#[test]
fn lex_attribute_with_gt_in_quote() {
    let (span, rest) = try_lex_html_span(r#"<a title=">"href="x">link</a>!"#).unwrap();
    assert_eq!(span.raw, r#"<a title=">"href="x">link</a>"#);
    assert_eq!(rest, "!");
}

#[test]
fn lex_nested_same_tag() {
    let (span, rest) =
        try_lex_html_span("<details><details>x</details></details>tail").unwrap();
    assert_eq!(span.raw, "<details><details>x</details></details>");
    assert_eq!(rest, "tail");
}

#[test]
fn lex_missing_close_returns_open_only() {
    let (span, rest) = try_lex_html_span("<details>x and more").unwrap();
    assert_eq!(span.raw, "<details>");
    assert_eq!(rest, "x and more");
}

#[test]
fn lex_comment_stripped() {
    let (span, rest) = try_lex_html_span("<!-- secret -->after").unwrap();
    assert!(span.is_stripped());
    assert_eq!(rest, "after");
}

#[test]
fn lex_not_a_tag_returns_none() {
    assert!(try_lex_html_span("< not a tag").is_none());
    assert!(try_lex_html_span("a <b>").is_none());
}

#[test]
fn lex_classification() {
    assert_eq!(classify("kbd"), HtmlSpanKind::PhrasingSafe);
    assert_eq!(classify("details"), HtmlSpanKind::BlockSafe);
    assert_eq!(classify("script"), HtmlSpanKind::Stripped);
    assert_eq!(classify("foo"), HtmlSpanKind::Unknown);
    assert_eq!(classify("DETAILS"), HtmlSpanKind::BlockSafe, "case-insensitive");
}
