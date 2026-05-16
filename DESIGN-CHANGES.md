# DESIGN-CHANGES — CastCodes UI/UX Modernization

Tracks Phase 1 of [`CODY-BRIEF.md`](CODY-BRIEF.md) (OpenCoven brand identity).

## Scope reconciliation

`CODY-BRIEF.md` was written against a Zed-style codebase layout:
`crates/theme`, `crates/title_bar`, `crates/workspace/src/pane.rs`,
`crates/project_panel`, `pane::ToggleTabBar`, `cargo check -p zed`.

This repository is the **Warp fork** rebranded as CastCodes. The relevant
paths here are:

| Brief reference                          | This repo                                        |
|------------------------------------------|--------------------------------------------------|
| `crates/theme`                           | `app/src/themes/`                                |
| `crates/workspace/src/pane.rs`           | `app/src/workspace/`, `app/src/tab.rs` (1905 LoC)|
| `crates/title_bar`                       | rendered via `warpui` in `app/src` views         |
| `crates/workspace/src/status_bar.rs`     | `app/src/shell_indicator.rs` + tab decorations   |
| `crates/project_panel`, `crates/outline_panel` | not present (Warp has no project panel)    |
| `crates/editor`                          | `crates/editor` (already exists)                 |
| `cargo check -p zed`                     | `cargo check -p warp-app --bin cast-codes --features gui` |

`WarpTheme::new()` accepts seven concrete slots (background fill,
foreground color, accent fill, optional gradient, details preset, terminal
ANSI palette, optional background image, display name). Brand slots that
the brief defines but `WarpTheme` does not (surface, elevated surface,
border, text secondary, text muted, accent secondary gold, status bar bg,
title bar bg) have no direct theme slot to map onto — they are computed
downstream from `details` + `background` + `accent` in dependent UI
crates.

## Applied (this PR)

### Brand rebrand sweep

- `app/src/drive/index.rs:109` — `WARP_DRIVE_TITLE` literal updated from
  `"Warp Drive"` to `"Cast Drive"`.
- `app/src/drive/index.rs:3962` — header label now reads from the constant
  rather than a duplicated string literal.
- `app/src/search/data_source.rs:316` — `QueryFilter::Drive` display label
  updated to `"Cast Drive"`.
- Existing `app/src/settings_view/mod.rs:339-341` aliases
  (`"Cast Drive" | "WarpDrive" | "Warp Drive"`,
  `"Cast Agent" | "Warp Agent"`) preserve back-compat with persisted user
  settings per [`CASTCODES.md`](CASTCODES.md) naming rules.

### Design tokens (shadcn-style)

- New [`resources/design-tokens.css`](resources/design-tokens.css)
  captures the full brand palette as CSS custom properties using
  shadcn/ui semantic aliases (`--background`, `--card`, `--primary`,
  `--ring`, `--destructive`, etc.).
- Single source of truth for any web-side surface (docs site, marketing,
  future plugin host UI) so the same hex values stay in sync with the
  native `castcodes_dark` theme. The native theme is the authoritative
  consumer for the GPUI app; this file mirrors it.

### Feature flags

- `crates/ai/Cargo.toml` — added `default = ["cast-agent"]` plus the
  `cast-agent` and `warp-agent` features. `cast-agent` pulls in the new
  `crates/cast_agent` as an optional dep. `warp-agent` is declared
  without removing the existing unconditional `warp_*` deps because they
  are used outside agent paths; call-site `#[cfg(feature = "...")]`
  gating is the follow-up step. Verified with `cargo check -p ai`.
- `Cargo.toml` workspace deps table — registered
  `cast_agent = { path = "crates/cast_agent" }` so other crates can
  consume it via `cast_agent.workspace = true`.

### 1.1 Theme — `castcodes_dark`

- New theme function `castcodes_dark()` in
  [`app/src/themes/default_themes.rs`](app/src/themes/default_themes.rs)
  with the OpenCoven palette:
  - background: `#0F0F12`
  - foreground (text primary): `#E8E8ED`
  - accent (purple): `#7C3AED`
  - details: `Darker` (so derived surfaces darken correctly)
  - terminal ANSI: reuses the existing `dark_mode_colors()` palette.
- New `ThemeKind::CastCodesDark` variant in
  [`app/src/themes/theme.rs`](app/src/themes/theme.rs); marked
  `#[default]` so it is the first-launch theme. The previous default
  (`ThemeKind::Dark`) is preserved as a selectable theme.
- Registered in `WarpThemeConfig::new()` and used as the fallback for
  `WarpThemeConfig::theme()` when an unknown kind is requested.
- Added to the onboarding theme picker
  ([`app/src/themes/mod.rs`](app/src/themes/mod.rs)) in slot 0,
  replacing the prior plain "Dark" entry.

## Deferred (follow-up PRs)

The remainder of Phase 1 is deferred so this change stays additive and
reviewable. Each item is non-trivial in this codebase because Warp's UI
layer (warpui + `app/src/`) does not split surfaces into dedicated crates
the way Zed does.

### 1.2 Collapsible tab bar

The brief asks for `tab_bar_collapsed` on a `Pane` struct,
`pane::ToggleTabBar`, and a `cmd+shift+b` keybind. The closest equivalents
here are `app/src/tab.rs` (1905 LoC, tab rendering + drag/drop/lifecycle)
and `app/src/workspace/mod.rs` (1581 LoC). There is no `Pane` struct with
a tab bar field — tab rendering is interleaved with header/toolbar logic.
A collapsible mode would require a refactor of `tab.rs` to gate the tab
strip render path and a new global action wired through the existing
keybinding registry. Out of scope for this PR.

### 1.3 Title bar / 1.4 Status bar / 1.5 Sidebar / 1.6 Editor area / 1.7 Inputs / 1.8 Buttons

These styling changes are valid but require:

- Refactoring `WarpTheme` (in `crates/warp_core/src/ui/theme`) to expose
  semantic slots beyond the current bg/fg/accent triple, OR
- Updating each rendering site individually to read from `castcodes_dark`
  and pick derived colors locally.

Either approach is a multi-day pass that touches every panel render. The
new theme already provides the brand background and accent, so the
existing `details: Darker` derivation gives the dark, accent-purple flavor
the brief is asking for at a coarse level — finer surface separation
(surface vs. elevated surface, border at 8% white, etc.) is a follow-up.

### Build verification

- `cargo check -p cast_agent` ✅ (27.21s).
- `cargo check -p ai` ✅ (42.21s) with the new `default = ["cast-agent"]`.

The theme changes and rebrand string sweeps are syntactic edits to
existing files in the `app` crate; full `cargo check -p warp-app` on this
Warp fork is a multi-minute compile and was not run in this session.
Recommended verification before merge:

```bash
./script/check_rebrand
cargo check -p warp-app --bin cast-codes --features gui
```
