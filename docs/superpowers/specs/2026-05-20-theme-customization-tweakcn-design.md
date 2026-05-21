# Deeper theme customization with tweakcn import

**Date:** 2026-05-20
**Status:** Approved (brainstorming) — pending plan
**Owner:** Val Alexander

## Summary

Expand CastCodes' theme schema with an optional, tweakcn-aligned `ui:` token block so users can customize chat-panel backgrounds, text colors, borders, and other semantic UI surfaces that are currently either derived from a small handful of base colors or hardcoded as brand constants. Add an in-settings "Import theme…" flow that accepts pasted CSS or a dropped `.css` file from [tweakcn](https://tweakcn.com/), converts OKLCH→sRGB, writes a YAML theme to `~/.config/warp/themes/`, and selects it.

The change is purely additive — every existing built-in and user theme keeps working unchanged because missing `ui:` fields fall back to today's derived values.

## Goals

- Let users customize the AI chat panel background and text colors per theme.
- Accept tweakcn CSS exports as a first-class import format (paste or drag-drop).
- Preserve backward compatibility for all existing themes (built-in and user).
- Stay within the CastCodes fork-local OSS boundary: no network calls, no hosted-service dependency.

## Non-goals

- Live in-app theme editor / token picker UI.
- Export from CastCodes → tweakcn (round-trip).
- Per-pane theme overrides.
- Animated transitions between themes.
- ANSI palette generation from tweakcn (tweakcn has no ANSI; we inherit terminal_colors from the user's currently selected base theme).

## Background

Today's theme schema (YAML, parsed in `app/src/user_config/util.rs:144`, struct in `crates/warp_core/src/ui/theme/mod.rs:589`):

```yaml
accent: '#01a0e4'
background: '#090300'
foreground: '#a5a2a2'
details: darker            # Darker | Lighter | Custom
terminal_colors:
  normal:  { black, red, green, yellow, blue, magenta, cyan, white }
  bright:  { black, red, green, yellow, blue, magenta, cyan, white }
```

The chat panel (`app/src/ai_assistant/panel.rs`) does not have any dedicated color knobs:

- Panel background = `theme.surface_2()` / `surface_3()`, both derived from `background + details` opacity ramp (`crates/warp_core/src/ui/theme/color.rs:90`).
- Primary text = `theme.active_ui_text_color()`, derived from `foreground` against the surface.
- Secondary text = `OPENCOVEN_MUTED` — a hardcoded brand constant in `app/src/ai/coven_brand.rs:29`.
- Status text uses `OPENCOVEN_SUCCESS` / `OPENCOVEN_WARNING` — also hardcoded.
- Borders = `theme.outline()`, derived.

There is therefore no way to give the chat panel a distinct surface color or to override its muted text without forking the source. tweakcn's token vocabulary (`card`, `popover`, `muted_foreground`, `border`, `sidebar`, etc.) is a natural superset of what's needed; aligning to it solves the customization gap and the import story in one move.

## Design

### 1. Schema: optional `ui:` block

Add a single optional `ui:` block to the theme YAML. All fields are optional `ColorU` (sRGB hex on disk).

```yaml
# Existing fields — unchanged, still authoritative for terminal rendering
accent: '#01a0e4'
background: '#090300'
foreground: '#a5a2a2'
details: darker
terminal_colors: { ... }

# NEW — optional, tweakcn-aligned semantic UI tokens
ui:
  card: '#0f0905'                # chat panel + floating surface bg
  card_foreground: '#e8e6e3'     # text on `card`
  popover: '#13100c'             # tooltip / dropdown bg
  popover_foreground: '#e8e6e3'
  primary: '#01a0e4'             # interactive primary (send button, etc.)
  primary_foreground: '#ffffff'
  secondary: '#1a1410'           # tertiary buttons, chip bg
  secondary_foreground: '#a5a2a2'
  muted: '#1a1410'               # subtle bg fills
  muted_foreground: '#5a5a65'    # ← replaces hardcoded OPENCOVEN_MUTED
  destructive: '#db2d20'
  border: '#2a2520'              # chat dividers, panel outline
  input: '#1a1410'
  ring: '#01a0e4'                # focus ring
  sidebar: '#0a0604'             # sidebar bg
  sidebar_foreground: '#e8e6e3'

# Provenance — written by import flow; ignored by the runtime
source: tweakcn
source_imported_at: '2026-05-20T12:00:00Z'
```

Rust:

```rust
// crates/warp_core/src/ui/theme/mod.rs
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct UiTokens {
    pub card: Option<ColorU>,
    pub card_foreground: Option<ColorU>,
    pub popover: Option<ColorU>,
    pub popover_foreground: Option<ColorU>,
    pub primary: Option<ColorU>,
    pub primary_foreground: Option<ColorU>,
    pub secondary: Option<ColorU>,
    pub secondary_foreground: Option<ColorU>,
    pub muted: Option<ColorU>,
    pub muted_foreground: Option<ColorU>,
    pub destructive: Option<ColorU>,
    pub border: Option<ColorU>,
    pub input: Option<ColorU>,
    pub ring: Option<ColorU>,
    pub sidebar: Option<ColorU>,
    pub sidebar_foreground: Option<ColorU>,
}

pub struct WarpTheme {
    // … existing fields unchanged …
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiTokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_imported_at: Option<String>,
}
```

### 2. Accessor changes (`crates/warp_core/src/ui/theme/color.rs`)

Existing accessors return `Fill` (which can be solid or gradient). Each override path checks the `ui:` block first, wraps the `ColorU` in `Fill::Solid`, and falls back to today's derivation if absent. No callsite changes are needed for these.

| Accessor | Return | New behavior |
| --- | --- | --- |
| `surface_2() -> Fill` | `Fill` | `self.ui.as_ref().and_then(|u| u.card).map(Fill::Solid).unwrap_or_else(|| Fill::Solid(neutral_2(self)))` |
| `outline() -> Fill` | `Fill` | `... u.border ... .unwrap_or_else(|| fg_overlay_2(self))` |
| `active_ui_text_color() -> Fill` | `Fill` | `... u.card_foreground ... .unwrap_or_else(|| self.main_text_color(self.surface_2()))` |

`surface_1()` and `surface_3()` are **not** overridden by `ui.card` directly — they remain derived. This matters because chat-internal subsurfaces (e.g., the chat input row that uses `surface_3`) will continue to derive from `details + background`, not from `ui.card`. If a future iteration wants those overridable too, add `ui.subsurface` / `ui.surface_alt`. Out of scope for v1.

Three new accessors for tokens with no existing home:

```rust
impl WarpTheme {
    pub fn muted_foreground(&self) -> ColorU {
        self.ui.as_ref().and_then(|u| u.muted_foreground).unwrap_or(OPENCOVEN_MUTED)
    }
    pub fn sidebar_bg(&self) -> Fill {
        self.ui.as_ref().and_then(|u| u.sidebar).map(Fill::Solid).unwrap_or_else(|| self.surface_1())
    }
    pub fn ring(&self) -> Fill {
        self.ui.as_ref().and_then(|u| u.ring).map(Fill::Solid).unwrap_or_else(|| self.accent())
    }
}
```

**Sidebar wire-up note.** Today, sidebar callsites use `theme.surface_1()` directly. To make `ui.sidebar` take effect, the sidebar callsites must switch to `theme.sidebar_bg()`. Scope this PR to the obvious sidebar surface views (workspace left panel, tab strip background) — enumerated in §"File-by-file change list".

Note: `OPENCOVEN_MUTED`, `OPENCOVEN_SUCCESS`, `OPENCOVEN_WARNING` remain in `app/src/ai/coven_brand.rs` as the fallback defaults; they stop being load-bearing rendered values once a theme provides `ui.muted_foreground`.

### 3. Chat panel wire-up (`app/src/ai_assistant/panel.rs`)

Single, targeted replacement: every `OPENCOVEN_MUTED` reference becomes `theme.muted_foreground()`. Lines affected today: 1061, 1069, 1211, 1267 (and the `OPENCOVEN_MUTED` arm at 1225 for the closed-session pill). `OPENCOVEN_SUCCESS` / `OPENCOVEN_WARNING` are left as brand status indicators (semantically tied to gateway state, not theme-driven; they stay hardcoded for v1).

The panel's surface and outline already come from `theme.surface_2()`/`theme.surface_3()` and `theme.outline()`, so the schema's `ui.card`/`ui.border` automatically flow through with no additional edits.

### 4. tweakcn parser

New module `app/src/themes/tweakcn_import.rs`. No new crate dependencies — OKLCH conversion is ~50 LoC of vendored formulas.

```rust
pub enum ImportError {
    NoColorBlocksFound,
    InvalidOklch { var: String, raw: String },
    OutOfSrgbGamut { var: String, oklch: (f64, f64, f64) },
}

pub struct TweakCnTheme {
    pub name: Option<String>,    // from a leading CSS comment if present
    pub light: HashMap<String, ColorU>,
    pub dark: HashMap<String, ColorU>,
}

pub fn parse_tweakcn_css(input: &str) -> Result<TweakCnTheme, ImportError>;
pub fn to_warp_theme(t: &TweakCnTheme, mode: ThemeKind, inherit_terminal_from: &WarpTheme) -> WarpTheme;
```

Algorithm:

1. Regex-extract `:root { ... }` (light) and `.dark { ... }` (dark) blocks.
2. For each `--name: oklch(L C H);` declaration:
   - Parse L (0..1), C (0..0.4 typical), H (0..360 deg).
   - Convert OKLCH → linear sRGB via the published Björn Ottosson formulas.
   - Apply sRGB transfer function → 8-bit sRGB → hex.
   - If any channel is out of [0, 1] post-conversion: surface `OutOfSrgbGamut` with an option to clamp (settings modal offers a "clamp out-of-gamut colors" toggle, default on).
3. Map tweakcn → CastCodes:

   | tweakcn `--var` | CastCodes field |
   | --- | --- |
   | `--background` | top-level `background` |
   | `--foreground` | top-level `foreground` |
   | `--primary` | top-level `accent` (and `ui.primary`) |
   | `--card` | `ui.card` |
   | `--card-foreground` | `ui.card_foreground` |
   | `--popover` / `--popover-foreground` | `ui.popover` / `ui.popover_foreground` |
   | `--secondary` / `--secondary-foreground` | `ui.secondary` / `ui.secondary_foreground` |
   | `--muted` / `--muted-foreground` | `ui.muted` / `ui.muted_foreground` |
   | `--destructive` | `ui.destructive` |
   | `--border` | `ui.border` |
   | `--input` | `ui.input` |
   | `--ring` | `ui.ring` |
   | `--sidebar` / `--sidebar-foreground` | `ui.sidebar` / `ui.sidebar_foreground` |
   | `--primary-foreground` | `ui.primary_foreground` |

   tweakcn variables we ignore for v1: `--accent` / `--accent-foreground` (shadcn's "accent" is a hover surface, not our brand accent — we already mapped `--primary` to top-level `accent`), `--chart-1..5`, `--radius`, font and shadow variables. They are read into the parser but not written to YAML so we can revisit later without re-parsing.

4. `terminal_colors` is *not* derived from tweakcn (it has no ANSI palette). The new theme inherits `terminal_colors` from `inherit_terminal_from` (the user's currently selected base theme at import time). Users can edit by hand afterward.

### 5. Import UX (`app/src/settings_view/appearance_page.rs`)

Add an **"Import theme…"** button next to the theme picker on the Appearance page. Clicking opens a modal:

- **Paste box** for the raw CSS (multi-line `TextField`).
- **Drop zone** for `.css` files (uses existing drag-drop plumbing in the settings shell).
- **Live preview** row: four swatches (`background`, `card`, `accent`, `card_foreground`) computed from the paste content as the user types, with a 200ms debounce.
- **Name** input (prefilled from the CSS comment or filename, slug-validated `[a-z0-9-]`).
- **Light/Dark detected** badge — shows which blocks were found.
- **Out-of-gamut behavior** toggle (default: clamp).
- **Save** button — disabled until parse succeeds.

On save:

- Writes `~/.config/warp/themes/<slug>.yaml` for the dark block.
- Writes `~/.config/warp/themes/<slug>-light.yaml` for the light block if present.
- Re-runs the theme loader (`load_theme_configs()` in `app/src/user_config/native.rs:168`).
- Selects the new theme via `WorkspaceAction::ShowThemeChooser` → set-current.

No network, no permissions prompts, no hosted-service surface — fits the CastCodes fork-local OSS boundary ([[castcodes-fork-local-boundary]]).

### 6. Migration / backward compatibility

- **Built-in themes** in `app/src/themes/default_themes.rs`: untouched. All existing accessors fall back to today's derivation when `ui` is `None`.
- **Existing user YAMLs** in `~/.config/warp/themes/`: untouched. Same fallback.
- **Pixel parity test** (see §7) locks down that any theme without a `ui:` block renders identically to today.

### 7. Testing strategy

| Layer | Test |
| --- | --- |
| Unit | OKLCH→sRGB conversion against published reference values (e.g., Ottosson's test vectors). |
| Unit | Parser handles `:root { … }` only, `.dark { … }` only, both, and neither (error). |
| Unit | Out-of-gamut error vs clamp toggle. |
| Snapshot | Commit a real tweakcn export to `crates/integration/tests/data/tweakcn_sample.css`, assert resulting `WarpTheme` equals golden YAML at `crates/integration/tests/data/tweakcn_sample_dark.yaml`. |
| Backward-compat | For every built-in theme: assert `theme.surface_2()`, `theme.outline()`, `theme.active_ui_text_color()` return identical `ColorU` before vs after this PR (table test). |
| Integration | Load a `ui:`-block YAML, render the AI panel via Builder/TestStep ([[warp-integration-test]]), assert chat-panel bg pixel = `ui.card` and muted text pixel = `ui.muted_foreground`. |

### 8. Risk and mitigation

| Risk | Mitigation |
| --- | --- |
| OKLCH→sRGB conversion drift introduces wrong colors. | Pin a known formula version, add reference-value tests, expose a "clamp out-of-gamut" toggle. |
| `ui:` overrides break terminal readability (e.g., user pastes a theme where `card_foreground` ≈ `card`). | Pre-save contrast check: warn (don't block) if WCAG AA fails between any `*` / `*_foreground` pair. |
| tweakcn changes its CSS export format. | Parser is regex-driven on stable `--var: oklch(L C H);` lines; we document the supported format in the modal's help link. Versioned `source: tweakcn-v1` in YAML lets us evolve. |
| User pastes a theme that ships only light or only dark — they then toggle the missing mode. | Detect which blocks parsed, write only those files, fall back to the previously selected theme for the missing mode (current behavior). |

### 9. Open questions (to resolve during planning)

- Should we expose `--accent` / `--accent-foreground` from tweakcn (shadcn-semantics "hover surface") even though we map `--primary` to our brand accent? Likely no for v1; revisit if users ask.
- Should the contrast warning block save, or just warn? Current proposal: warn only.
- Should we store the imported CSS source alongside the YAML for future re-conversion? Adds disk weight; skip for v1.

## File-by-file change list

```
crates/warp_core/src/ui/theme/mod.rs                    # UiTokens struct, ui field on WarpTheme
crates/warp_core/src/ui/theme/color.rs                  # accessor shims + 3 new accessors
app/src/ai_assistant/panel.rs                           # OPENCOVEN_MUTED → theme.muted_foreground()
app/src/workspace/view/left_panel.rs                    # surface_1() → sidebar_bg() (sidebar wire-up)
app/src/workspace/view/tab_bar.rs (or equivalent)       # surface_1() → sidebar_bg() if applicable
app/src/themes/tweakcn_import.rs                        # NEW — OKLCH parser, mapping, writer
app/src/themes/mod.rs                                   # re-export tweakcn_import
app/src/settings_view/appearance_page.rs                # "Import theme…" button
app/src/settings_view/import_theme_modal.rs             # NEW — paste/drop modal
crates/integration/tests/data/tweakcn_sample.css        # NEW — fixture
crates/integration/tests/data/tweakcn_sample_dark.yaml  # NEW — golden
crates/integration/tests/data/tweakcn_sample_light.yaml # NEW — golden
```

Concrete sidebar callsites will be identified during planning (the planning phase should grep `surface_1()` in `app/src/workspace/` and decide which switch to `sidebar_bg()` — likely the outer panel bg only, not nested controls).

## Acceptance criteria

1. A theme YAML with a `ui:` block parses and applies; chat panel bg/text observably change.
2. A theme YAML without a `ui:` block renders pixel-identically to before this PR.
3. Pasting a tweakcn export into the new modal results in a selectable theme within ≤2s, no network calls.
4. Both light and dark blocks (when present) produce two paired YAML files.
5. Out-of-gamut colors are either clamped (default) or reported with the offending `--var` named.
6. `cargo test -p warp_core` and the new integration test pass; presubmit clean.
