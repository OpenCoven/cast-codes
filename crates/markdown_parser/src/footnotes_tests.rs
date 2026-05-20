use super::*;

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
