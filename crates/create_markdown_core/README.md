# create_markdown_core

Block-based markdown parsing and serialization. Rust sister port of
[`@create-markdown/core`](https://www.npmjs.com/package/@create-markdown/core)
(v2.0.3) — same block model, same JSON shape, idiomatic Rust API.

## Status

Tracking `@create-markdown/core@2.0.3`:

- [x] Block types and inline `TextSpan`/styles
- [x] Utilities (escape, ID, span helpers)
- [ ] Block factories (`paragraph`, `h1`..`h6`, lists, code, table, callouts)
- [ ] Document operations (insert, append, move, find, update)
- [ ] Tokenizer
- [ ] Markdown parser (markdown → blocks)
- [ ] Markdown serializer (blocks → markdown)
- [ ] Full test parity with the JS package

## Quick Start

```rust
use create_markdown_core::{Block, Document, DocumentMeta, BlockType, TextSpan};

let span = TextSpan::plain("Hello, world!");
assert_eq!(span.text, "Hello, world!");
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

## License

MIT, same as `@create-markdown/core`.
