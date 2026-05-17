# create_markdown_core

Block-based markdown parsing and serialization. Rust sister port of
[`@create-markdown/core`](https://www.npmjs.com/package/@create-markdown/core)
(v2.0.3) — same block model, same JSON shape, idiomatic Rust API.

## Status

Tracking `@create-markdown/core@2.0.3`:

- [x] Block types and inline `TextSpan`/styles
- [x] Utilities (escape, ID, span helpers)
- [x] Block factories (`paragraph`, `h1`..`h6`, lists, code, table, callouts)
- [x] Document operations (insert, append, move, find, update)
- [x] Markdown serializer (blocks → markdown)
- [x] Line-based tokenizer
- [x] Inline parser (bold/italic/code/strikethrough/highlight/links/images/escapes)
- [x] Block parser (headings, paragraphs, lists, code fences, tables, callouts, dividers)
- [ ] Full vitest → `#[test]` test-suite parity (hand-written suite covers the surface; the JS package's vitest fixtures aren't yet ported)

118 unit tests cover every surface and pass on CI.

## Quick Start

```rust
use create_markdown_core::{
    blocks_to_markdown, bullet_list, h1, paragraph, MarkdownSerializeOptions,
};

let doc = vec![
    h1("Hello, world!"),
    paragraph("Block-based markdown in Rust."),
    bullet_list(["one", "two", "three"]),
];

let md = blocks_to_markdown(&doc, &MarkdownSerializeOptions::default());
println!("{md}");
// # Hello, world!
//
// Block-based markdown in Rust.
//
// - one
// - two
// - three
```

Build documents, mutate them, and serialize:

```rust
use create_markdown_core::{
    append_block, document_to_markdown, empty_document, h1, paragraph, DocumentOptions,
    MarkdownSerializeOptions,
};

let doc = empty_document(DocumentOptions::default());
let doc = append_block(&doc, &h1("Title"));
let doc = append_block(&doc, &paragraph("body text"));
println!("{}", document_to_markdown(&doc, &MarkdownSerializeOptions::default()));
```

## Naming

The Rust API is snake_case. The JS→Rust map for the public surface:

| JS                         | Rust                              |
| -------------------------- | --------------------------------- |
| `parse(md)`                | `parse(md)`                       |
| `markdownToBlocks`         | `markdown_to_blocks`              |
| `markdownToDocument`       | `markdown_to_document`            |
| `stringify`                | `stringify`                       |
| `blocksToMarkdown`         | `blocks_to_markdown`              |
| `documentToMarkdown`       | `document_to_markdown`            |
| `createDocument`           | `create_document`                 |
| `appendBlock` / `insertBlock` / `removeBlock` / `moveBlock` / `findBlock` | same, snake_case |
| `escapeMarkdown` / `unescapeMarkdown` | `escape_markdown` / `unescape_markdown` |
| `generateId`               | `generate_id`                     |
| `fromMarkdown` / `toMarkdown` | `from_markdown` / `to_markdown` |
| `h1`..`h6`                 | `h1`..`h6`                        |
| `bulletList`               | `bullet_list`                     |
| `numberedList`             | `numbered_list`                   |
| `checkList` / `checkListItem` | `check_list` / `check_list_item` |
| `codeBlock`                | `code_block`                      |
| `blockquote`               | `block_quote`                     |
| `infoCallout` / `warningCallout` / `tipCallout` / `dangerCallout` / `noteCallout` | same, snake_case |

`BlockType`, `CalloutType`, and other enum variants use PascalCase variant
names but serialize to the JS string literals (`paragraph`, `heading`,
`bulletList`, …) so JSON output is wire-compatible with the npm package.

### Block nesting

The block model nests lists via the **item's `children`** field, not by
making one list a sibling-child of another. To produce

```markdown
- parent
  - child a
  - child b
```

build it as a parent paragraph whose `children` is the nested list:

```rust
use create_markdown_core::{bullet_list, paragraph, ListItem};

let nested = bullet_list(["child a", "child b"]);
let mut parent = paragraph("parent");
parent.children = vec![nested];
let outer = bullet_list(vec![ListItem::Block(parent)]);
```

## License

MIT, same as `@create-markdown/core`.
