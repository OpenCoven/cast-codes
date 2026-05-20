use super::*;
use crate::{parse_markdown};

#[test]
fn rewrite_single_reference_appends_section() {
    let parsed = parse_markdown("Some claim[^x].\n\n[^x]: Because reasons.\n")
        .expect("parse");
    // Structural assertions:
    assert!(parsed.lines.iter().any(|l| matches!(l, FormattedTextLine::HorizontalRule)));
    let has_back_ref = parsed
        .lines
        .iter()
        .filter_map(|line| match line {
            FormattedTextLine::OrderedList(list) => Some(&list.indented_text.text),
            _ => None,
        })
        .flatten()
        .any(|f| f.text == " ↩" && matches!(&f.styles.hyperlink, Some(Hyperlink::Url(u)) if u.contains("fnref")));
    assert!(has_back_ref, "expected back-reference fragment");
    // The reference itself is now a hyperlink "1"
    let reference_hyperlink_present = parsed
        .lines
        .iter()
        .filter_map(|line| match line {
            FormattedTextLine::Line(frags) => Some(frags),
            _ => None,
        })
        .flatten()
        .any(|f| f.text == "1" && matches!(&f.styles.hyperlink, Some(Hyperlink::Url(u)) if u == "#fn-x"));
    assert!(reference_hyperlink_present, "expected #fn-x reference");
}

#[test]
fn unused_definition_dropped() {
    let parsed = parse_markdown("Plain text.\n\n[^never]: Unused.\n").expect("parse");
    assert!(parsed.lines.iter().all(|l| !matches!(l, FormattedTextLine::HorizontalRule)));
}

#[test]
fn undefined_reference_passes_through() {
    let parsed = parse_markdown("Some claim[^missing] here.\n").expect("parse");
    let joined: String = parsed
        .lines
        .iter()
        .filter_map(|line| match line {
            FormattedTextLine::Line(frags) => Some(frags.iter().map(|f| f.text.clone()).collect::<String>()),
            _ => None,
        })
        .collect();
    assert!(joined.contains("[^missing]"), "expected literal pass-through: {joined:?}");
}

#[test]
fn repeated_references_share_number() {
    let parsed = parse_markdown("A[^x] B[^x]\n\n[^x]: D\n").expect("parse");
    let count = parsed
        .lines
        .iter()
        .filter_map(|line| match line {
            FormattedTextLine::Line(frags) => Some(frags),
            _ => None,
        })
        .flatten()
        .filter(|f| f.text == "1")
        .count();
    assert_eq!(count, 2, "both references should be number 1");
}

#[test]
fn extract_single_definition() {
    let source = "claim[^x]\n\n[^x]: defn\n";
    let (out, defs) = extract_definitions(source);
    assert_eq!(out, "claim[^x]\n\n");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs.get("x").unwrap().content, "defn");
}

#[test]
fn extract_no_definitions_passthrough() {
    let source = "plain text\n";
    let (out, defs) = extract_definitions(source);
    assert_eq!(out, "plain text\n");
    assert!(defs.is_empty());
}

#[test]
fn extract_continuation_line() {
    let source = "[^x]: first\n    continued\n";
    let (out, defs) = extract_definitions(source);
    assert_eq!(out, "");
    assert_eq!(defs.get("x").unwrap().content, "first continued");
}

#[test]
fn extract_id_with_space_skipped() {
    // GFM ids don't allow whitespace; the line falls through as a regular paragraph.
    let source = "[^bad id]: defn\n";
    let (out, defs) = extract_definitions(source);
    assert_eq!(out, source);
    assert!(defs.is_empty());
}
