# DESIGN-CHANGES — CastCodes UI/UX Modernization

Tracks Phase 1 of the OpenCoven brand-identity rebrand.

## Repo-level guardrails

- New root [`AGENTS.md`](AGENTS.md) defines the CastCodes-specific agent rules:
  no AI attribution, staged rebrand discipline, Phase 1 design constraints, and
  verification expectations.
- New [`script/check_ai_attribution`](script/check_ai_attribution) blocks
  generated-by/model-credit artifacts while still allowing AI/model names in
  real product behavior, compatibility notes, specs, and tests.
- [`README.md`](README.md) now includes the attribution guard alongside the
  rebrand guard so public-surface changes run both checks together.

## Scope reconciliation

The original Phase 1 scope was written against a Zed-style codebase layout
(`crates/theme`, `crates/title_bar`, `crates/workspace/src/pane.rs`,
`crates/project_panel`, `pane::ToggleTabBar`, `cargo check -p zed`).

This repository is the **Warp fork** rebranded as CastCodes. The relevant
paths here are:

| Zed-style path                           | This repo                                        |
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
ANSI palette, optional background image, display name). Additional brand
slots from the original scope but not present on `WarpTheme` (surface,
elevated surface, border, text secondary, text muted, accent secondary
gold, status bar bg, title bar bg) have no direct theme slot to map
onto — they are computed downstream from `details` + `background` +
`accent` in dependent UI crates.

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
- The token contract is intentionally minimalist: 4px compact radius, 6px
  controls, 8px maximum large-surface radius, 100-150ms motion, title/status
  chrome at `#0a0a0d`, and gold used only as a sparse highlight.

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
  - terminal ANSI: uses a CastCodes-specific dark palette derived from the
    Phase 1 brand tokens rather than the generic dark theme palette.
- New `ThemeKind::CastCodesDark` variant in
  [`app/src/themes/theme.rs`](app/src/themes/theme.rs); marked
  `#[default]` so it is the first-launch theme. The previous default
  (`ThemeKind::Dark`) is preserved as a selectable theme.
- Registered in `WarpThemeConfig::new()` and used as the fallback for
  `WarpThemeConfig::theme()` when an unknown kind is requested.
- Added to the onboarding theme picker
  ([`app/src/themes/mod.rs`](app/src/themes/mod.rs)) in slot 0,
  replacing the prior plain "Dark" entry.
- `SelectedSystemThemes::default()` now also uses `CastCodesDark` for the
  dark-mode branch, so enabling system-theme matching does not silently revert
  CastCodes back to the generic upstream dark theme.
- `app/src/settings/initializer.rs` now treats `CastCodesDark` as the new-user
  default when the legacy Adeberry feature-flag override is enabled.

### 1.2 Collapsible tab bar

- Added `WorkspaceAction::ToggleTabBar` and the editable
  `workspace:toggle_tab_bar` binding.
- Bound horizontal tab-bar collapse to `cmd+shift+b` / `ctrl+shift+b`.
  The existing vertical-tabs-panel binding keeps the same keystroke only
  when vertical tabs are active, so the two actions stay context-separated.
- Added an in-session `Workspace::tab_bar_collapsed` flag. Collapsing the
  tab bar closes tab-bar popups, hides the tab strip, and leaves a 2px
  accent-purple reveal line with an expand chevron.
- Hovering the reveal area temporarily shows the full tab bar again; the
  expand chevron toggles the persistent collapsed state off.
- Covered the manual collapse mode resolver with targeted unit tests.

## Deferred (follow-up PRs)

The remainder of Phase 1 is deferred so this change stays additive and
reviewable. Each item is non-trivial in this codebase because Warp's UI
layer (warpui + `app/src/`) does not split surfaces into dedicated crates
the way Zed does.

### 1.3 Title bar / 1.4 Status bar / 1.5 Sidebar / 1.6 Editor area / 1.7 Inputs / 1.8 Buttons

These styling changes are valid but require:

- Refactoring `WarpTheme` (in `crates/warp_core/src/ui/theme`) to expose
  semantic slots beyond the current bg/fg/accent triple, OR
- Updating each rendering site individually to read from `castcodes_dark`
  and pick derived colors locally.

Either approach is a multi-day pass that touches every panel render. The
new theme already provides the brand background and accent, so the
existing `details: Darker` derivation gives the dark, accent-purple flavor
the brand spec is asking for at a coarse level — finer surface separation
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
