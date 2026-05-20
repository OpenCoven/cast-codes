# Tech Spec: GitHub-Flavored Markdown Preview

See `specs/castcodes-md-gfm-html/PRODUCT.md` for user-visible behavior and success criteria.

## 1. Where the change lands

All work lives in `crates/markdown_parser/`. Downstream consumers (notebook editor, AI blocks, embedded items, code view, conversation rendering) are untouched. The change is the parser learning two new constructs (HTML spans, footnotes) and emitting them as existing `FormattedTextLine` / `FormattedTextFragment` variants.

```
crates/markdown_parser/src/
  lib.rs                    (no changes to public types)
  markdown_parser.rs        EDIT: integrate HTML lexing + footnote pre/post pass
  html_parser.rs            EDIT: extract two pub-crate helpers reusable from markdown_parser
  html_parser_tests.rs      EDIT: cover the new safe-list behavior in the existing test surface
  markdown_parser_tests.rs  EDIT: add HTML + footnote cases
  gfm_html.rs               NEW: safe-list + tag-filter constants and HTML span lexer
  footnotes.rs              NEW: footnote definition collection + reference resolution
```

`html_parser.rs` today owns its `parse_phrasing_content` and per-element block routing. v1 refactors these into pub-crate helpers (`parse_html_inline_fragments`, `parse_html_block_lines`) that `gfm_html.rs` can call. `parse_html` (the existing top-level entry point used for HTML-paste) becomes a thin wrapper over those helpers — no behavior change for existing callers.

## 2. HTML lexing (`gfm_html.rs`)

### Safe-list

Two `const &[&str]` arrays, sourced from the GFM tag-filter spec:

```rust
pub(crate) const PHRASING_SAFE_TAGS: &[&str] = &[
    "a", "b", "br", "code", "del", "em", "i", "ins", "kbd",
    "mark", "q", "s", "small", "span", "strong", "sub", "sup", "u",
];

pub(crate) const BLOCK_SAFE_TAGS: &[&str] = &[
    "details", "summary", "div", "p", "blockquote", "pre",
    "ul", "ol", "li", "dl", "dt", "dd",
    "table", "thead", "tbody", "tr", "th", "td", "caption",
    "h1", "h2", "h3", "h4", "h5", "h6",
    "hr", "img",
];

pub(crate) const STRIPPED_TAGS: &[&str] = &[
    "script", "style", "iframe", "object", "embed",
    "form", "button", "link", "meta", "title", "noscript",
    "applet", "frame", "frameset", "head", "html", "body",
];
```

Tags absent from all three lists pass through as their literal text (matches GitHub's behavior).

### Lexer

```rust
pub(crate) struct HtmlSpan<'a> {
    pub(crate) raw: &'a str,        // the entire matched <tag>...</tag>
    pub(crate) tag: &'a str,        // lowercase tag name
    pub(crate) is_block: bool,      // tag classified as block per BLOCK_SAFE_TAGS
    pub(crate) is_stripped: bool,   // tag classified for strip
}

pub(crate) fn try_lex_html_span(input: &str) -> Option<(HtmlSpan, &str /* rest */)>;
```

Algorithm:

1. Require `input.starts_with('<')` and that `<` is followed by an ASCII letter, `/`, or `!--`. (`<!--` comments are stripped silently.)
2. Read tag name (ASCII letters, digits, `-`); lowercase it.
3. Skip attributes by scanning to the first unquoted `>`. Quoted attribute values (`"..."` or `'...'`) are skipped without matching their inner `>`.
4. If the open tag ended `/>`, the span is the open tag alone. Otherwise, scan forward to find the matching closing `</tag>`, handling nested same-tag pairs by depth counting. If no close tag is found within the remaining input, the span is just the open tag (treated as a void element, matching how browsers handle stray opens).
5. Classify the tag against the three constants.

The lexer is character-driven, not regex-based. It runs in linear time over the captured span.

### Inline vs block dispatch

The markdown parser already routes per-line. The integration:

- **Block context** (called from `parse_markdown_internal` between block parsers): if `try_lex_html_span` matches and the tag is in `BLOCK_SAFE_TAGS`, consume the span and emit `parse_html_block_lines(span.raw)` (re-uses the `html5ever` tree walker from `html_parser.rs`). If the tag is in `STRIPPED_TAGS`, consume and emit nothing. If the tag is in `PHRASING_SAFE_TAGS`, do *not* consume here — let the paragraph parser handle it inline.
- **Inline context** (called from `parse_markdown_line` while building a paragraph): if `try_lex_html_span` matches and the tag is in `PHRASING_SAFE_TAGS`, consume the span and emit `parse_html_inline_fragments(span.raw)` into the current paragraph. Block tags here are *not* consumed inline — they end the current paragraph and reroute to the block path on the next pass. Stripped tags emit no fragments.

### `<details>`/`<summary>` rendering

`<details>` is treated as a block. The dispatch in `html_parser.rs` recognizes `details` specifically and emits, in order:

1. A `FormattedTextLine::Line` whose first fragment is `"▾ "` and whose remainder is the inline content of the `<summary>` child (parsed via `parse_html_inline_fragments`). If no `<summary>` is present, the line is `"▾ Details"`.
2. The remaining children of `<details>` parsed via `parse_html_block_lines` and appended.

No interaction wiring. The line carries no special variant — it's a regular line that happens to start with a glyph.

### Inline phrasing styling for new tags

`html_parser.rs::parse_phrasing_content` already maps `b/strong → bold`, `i/em → italic`, `u/ins → underline`, `s → strikethrough`, `code → inline_code`. v1 adds:

- `kbd` → `inline_code` flag (visually treats key labels as code chips, the closest existing affordance).
- `sub`, `sup` → `italic` flag plus a comment noting that subscript/superscript vertical shift isn't carried by `FormattedTextStyles` and is a deliberate v1 omission. (Adding a real shift requires a new style flag and renderer support; out of scope.)
- `mark`, `small`, `q`, `span` → pass through children as plain phrasing (no style change for v1).
- `del` → `strikethrough` (alias of `s`).
- `a` is already handled by the existing `href` attribute walker.
- `br` is already emitted as a line break.

### `<img>` block

`<img src=… alt=…>` in a block context emits `FormattedTextLine::Image(FormattedImage { alt_text, source, title: None })` — reusing the existing image block. `width` / `height` attributes are read but discarded for v1 (the existing `ImageBlockConfig` is layout-time, not parse-time).

### Raw `<table>`

When the markdown parser sees a block-context `<table>`, the existing `html_parser.rs` table walker (currently in `TOP_LEVEL_ELEMENT_TAGS_TO_SKIP`) is moved into a dedicated branch that builds a `FormattedTable` from the `<thead>`/`<tbody>`/`<tr>`/`<th>`/`<td>` tree and emits `FormattedTextLine::Table(table)`. The existing GFM pipe-table parser is unchanged; raw-HTML tables and pipe tables converge on the same render model.

## 3. Footnotes (`footnotes.rs`)

Two-pass implementation, called from `parse_markdown_impl` around the existing parser:

### Pass 1 — extract definitions

Scan `markdown` line-by-line for lines matching `^\[\^([^\]]+)\]:\s*(.*)$`. Collect into:

```rust
struct FootnoteDef {
    id: String,
    content: String,           // raw markdown of the definition body
    number: usize,             // 1-based, assigned at *reference* time, not definition time
}
```

Remove these lines from the input handed to the main parser. Continuation lines (indented under a footnote definition) are absorbed into `content`.

### Pass 2 — rewrite references

After the main parser produces `FormattedText`, walk every `FormattedTextLine`'s inline fragments and rewrite occurrences of `[^id]` (where `id` is a registered definition) into a hyperlink fragment:

```rust
FormattedTextFragment {
    text: format!("{}", number),   // "1", "2", …
    styles: FormattedTextStyles {
        italic: true,              // visual stand-in for superscript in v1
        hyperlink: Some(Hyperlink::Url(format!("#fn-{}", id))),
        ..Default::default()
    },
}
```

Number assignment: each footnote id gets its number from the order of *first reference* in the document. Re-references to the same id reuse the number. Unused definitions are dropped from the appended section.

### Pass 3 — append footnotes section

If at least one reference was rewritten, append:

- A `FormattedTextLine::HorizontalRule`.
- An `OrderedList` line per used definition. The list line content is `parse_inline_markdown(content)` + a trailing fragment `" ↩"` styled as a hyperlink whose URL is `#fnref-{id}`. (`#fnref-{id}` is a synthetic anchor; CodeView's hyperlink handler treats unknown anchor URLs as no-ops, which is acceptable for v1 — visual fidelity matters more than navigation working.)

The synthetic anchor links (`#fn-id`, `#fnref-id`) are valid hyperlinks per the `Hyperlink::Url` variant; the CodeView click handler simply does nothing for them today. If a follow-up wants in-document scroll-to-anchor behavior, that's a separate piece of work in CodeView's hyperlink dispatch.

## 4. Public API

No change. `parse_markdown` and `parse_markdown_with_gfm_tables` keep their signatures. The two new helpers in `html_parser.rs` are pub-crate. `gfm_html.rs` and `footnotes.rs` are private modules of `markdown_parser`.

## 5. Feature flag

None. The change is a refinement of existing default-on markdown rendering. Adding a flag adds churn without a real "off" use case — the previous behavior (HTML as literal text, footnotes as `[^id]` literal text) was a bug, not a feature.

If a regression in a downstream consumer (notebook auto-format roundtrip, AI block streaming) shows up during integration, the right gate is whichever code path the regression is in, not a parser-level kill switch.

## 6. Tests

### Unit (`markdown_parser_tests.rs`)

- `test_parse_html_details_with_summary` — `<details><summary>X</summary>Y</details>` → line "▾ X" then "Y".
- `test_parse_html_details_without_summary` — line "▾ Details" then content.
- `test_parse_html_kbd` — `Press <kbd>X</kbd>` → "Press " + inline-code "X".
- `test_parse_html_sub_sup` — `H<sub>2</sub>O` → "H" + italic "2" + "O".
- `test_parse_html_underline` — `<u>x</u>` and `<ins>x</ins>` → underline fragment "x".
- `test_parse_html_strikethrough_alias` — `<del>x</del>` → strikethrough fragment "x".
- `test_parse_html_br` — `a<br>b` → "a" + linebreak + "b".
- `test_parse_html_img_block` — block `<img src=… alt=…>` → `FormattedTextLine::Image`.
- `test_parse_html_raw_table` — block `<table>…</table>` → `FormattedTextLine::Table` with matching headers/rows.
- `test_parse_html_script_stripped` — `<script>alert(1)</script>` → emits nothing.
- `test_parse_html_unknown_tag_passes_through` — `<foo>x</foo>` → literal "<foo>x</foo>".
- `test_parse_html_malformed_open_no_close` — `<details>x` (no `</details>`) → renders the open tag as a void element + "x" as the next paragraph.
- `test_parse_footnote_single` — `claim[^1]\n\n[^1]: defn` → "claim¹" + hr + ordered-list "1. defn ↩".
- `test_parse_footnote_repeated_reference` — two `[^x]` references share number 1.
- `test_parse_footnote_unused_definition_dropped` — `[^x]: never used` produces no footnote section.
- `test_parse_footnote_undefined_reference_passthrough` — `claim[^missing]` with no definition → literal "claim[^missing]".

### Integration (smoke test, performed manually before merging)

Add `crates/markdown_parser/test-fixtures/gfm-smoketest.md` (a fixture file, not a runnable test) containing the safe-list tags + footnote + mermaid + GFM tables + task lists. Open it in CodeView, switch to Rendered, visually confirm:

- Each block tag renders with the matching visual treatment.
- The mermaid block renders as an SVG (verifies the existing wiring; no code expected to change).
- The footnote reference and back-reference render as styled hyperlinks.
- No leftover angle-bracket text or panics.

Failure of the mermaid step expands the scope of this PR with a follow-up commit; passing it closes that question.

### Existing tests

All existing `markdown_parser_tests.rs` cases must continue to pass unchanged. In particular, the HTML-paste tests in `html_parser_tests.rs` must be unaffected by the extraction of helpers.

## 7. Risk and rollback

- **Risk**: the refactor of `html_parser.rs` accidentally changes HTML-paste behavior (paste from Notion, GDocs, Confluence). Mitigation: the extracted helpers are called from `parse_html` via the same code path; the existing `html_parser_tests.rs` suite is the regression net.
- **Risk**: footnote rewriting touches every fragment of every line, which could be quadratic on documents with many footnote references. Mitigation: build a `HashMap<String, usize>` of definitions once, and short-circuit when the line text contains no `[^`.
- **Risk**: `<details>` opens that contain block content the markdown parser would normally treat as paragraphs (e.g. lists inside `<details>`) get parsed by `html5ever`, which doesn't run markdown inside HTML blocks. GitHub *does* render markdown inside HTML blocks (with a blank line separator). v1 documents this as a known limitation: markdown inside `<details>` is rendered as HTML only. If users hit this, a follow-up can add a "re-enter markdown parser when an HTML block contains a blank-line-separated markdown island" pass.
- **Rollback**: revert the parser PR. No downstream consumers are touched; no DB migration; no settings persistence; no feature flag to clean up.

## 8. Sequencing

1. Extract `parse_html_inline_fragments` and `parse_html_block_lines` from `html_parser.rs`; confirm existing tests pass.
2. Add `gfm_html.rs` with constants + `try_lex_html_span`. Unit-test the lexer.
3. Wire HTML dispatch into `parse_markdown_internal` (block path) and `parse_markdown_line` (inline path). Add the HTML test cases.
4. Add `footnotes.rs` with the two-pass implementation. Wire into `parse_markdown_impl`. Add the footnote test cases.
5. Hand-run the smoke fixture in CodeView. Capture screenshots in the PR description.
