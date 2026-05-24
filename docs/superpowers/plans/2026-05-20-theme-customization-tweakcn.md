# Theme Customization with tweakcn Import — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `ui:` token block to CastCodes themes plus an in-settings tweakcn CSS import flow, so users can customize chat-panel background, text colors, borders, and sidebar surface without forking source.

**Architecture:** Additive Approach A from spec — `WarpTheme` gains an optional `UiTokens` struct. Three existing accessors (`surface_2`, `outline`, `active_ui_text_color`) check `ui:` first and fall back to today's derived values. Three new accessors (`muted_foreground`, `sidebar_bg`, `ring`) cover tokens with no current home. tweakcn import is a paste/drop modal that parses OKLCH CSS, converts to sRGB hex, writes YAML(s) to `~/.config/warp/themes/`, and refreshes the theme loader. No new crates, no network.

**Tech Stack:** Rust, serde YAML, `warpui` widget layer, `pathfinder_color::ColorU`. Vendored OKLCH→sRGB conversion (~50 LoC). Existing `hex_color` serde adapter for `ColorU` (see `crates/warp_core/src/ui/theme/mod.rs:160`).

**Spec:** `docs/superpowers/specs/2026-05-20-theme-customization-tweakcn-design.md` (committed as `e2e003d4`).

**Commit policy:** Every commit MUST be signed (`git commit -S`). Verify with `git log -1 --show-signature` and grep for `Good "<algo>" signature`.

---

## File Structure

**Modified files**

| Path | Purpose |
| --- | --- |
| `crates/warp_core/src/ui/theme/mod.rs` | Add `UiTokens` struct; add `ui`, `source`, `source_imported_at` fields to `WarpTheme` |
| `crates/warp_core/src/ui/theme/color.rs` | Shim `surface_2`/`outline`/`active_ui_text_color`; add `muted_foreground`/`sidebar_bg`/`ring` |
| `crates/warp_core/src/ui/theme/theme_tests.rs` | Unit tests for schema + accessors + backward compat |
| `app/src/ai_assistant/panel.rs` | Replace 4 `OPENCOVEN_MUTED` callsites with `theme.muted_foreground()` |
| `app/src/workspace/view/left_panel.rs` | Switch outer-panel `surface_1()` to `sidebar_bg()` |
| `app/src/themes/mod.rs` | Re-export new `tweakcn_import` module |
| `app/src/settings_view/appearance_page.rs` | Add "Import theme…" button next to theme picker |

**New files**

| Path | Purpose |
| --- | --- |
| `app/src/themes/tweakcn_import.rs` | OKLCH→sRGB conversion, CSS parser, `to_warp_theme` mapper, writer |
| `app/src/settings_view/import_theme_modal.rs` | Paste/drop modal UI |
| `crates/integration/tests/data/tweakcn_sample.css` | Real tweakcn export fixture |
| `crates/integration/tests/data/tweakcn_sample_dark.yaml` | Golden YAML for dark block |
| `crates/integration/tests/data/tweakcn_sample_light.yaml` | Golden YAML for light block |
| `crates/integration/tests/theme_ui_block.rs` (or extend an existing test) | Integration: load `ui:`-block theme, render chat panel, assert pixels |

**Boundary notes**
- `OPENCOVEN_MUTED` / `OPENCOVEN_SUCCESS` / `OPENCOVEN_WARNING` constants stay in `app/src/ai/coven_brand.rs`. `OPENCOVEN_MUTED` becomes the *fallback* default when `ui.muted_foreground` is absent. The other two remain hardcoded brand semantics (gateway state, not theme-driven) per spec §3.
- Built-in themes in `app/src/themes/default_themes.rs` are **untouched**.

---

## Task 1: Define `UiTokens` struct + serde plumbing

**Files:**
- Modify: `crates/warp_core/src/ui/theme/mod.rs` (around line 580, near `TerminalColors`)
- Test: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Write failing test for empty/missing `ui:` block**

Add to `theme_tests.rs`:

```rust
#[test]
fn ui_tokens_default_all_none() {
    let tokens = UiTokens::default();
    assert!(tokens.card.is_none());
    assert!(tokens.card_foreground.is_none());
    assert!(tokens.muted_foreground.is_none());
    assert!(tokens.border.is_none());
    assert!(tokens.sidebar.is_none());
}

#[test]
fn ui_tokens_deserialize_partial() {
    let yaml = r#"
card: '#0f0905'
muted_foreground: '#5a5a65'
"#;
    let tokens: UiTokens = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(tokens.card.unwrap(), ColorU::from_u32(0x0f0905ff));
    assert_eq!(tokens.muted_foreground.unwrap(), ColorU::from_u32(0x5a5a65ff));
    assert!(tokens.popover.is_none());
}

#[test]
fn ui_tokens_serialize_skips_none() {
    let tokens = UiTokens {
        card: Some(ColorU::from_u32(0x0f0905ff)),
        ..Default::default()
    };
    let out = serde_yaml::to_string(&tokens).unwrap();
    assert!(out.contains("card"));
    assert!(!out.contains("popover"));
}
```

Use existing `ColorU::from_u32` constructor (search `crates/warp_core/src/ui/theme/mod.rs` for usage; it's used in `mock_terminal_colors`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p warp_core ui_tokens`
Expected: FAIL — `UiTokens` does not exist.

- [ ] **Step 3: Add `UiTokens` struct**

In `crates/warp_core/src/ui/theme/mod.rs`, after the `TerminalColors` struct (~line 586), add:

```rust
/// Optional, tweakcn-aligned semantic UI tokens. Every field is optional;
/// missing fields fall back to today's derived values in `color.rs`.
/// Field names mirror tweakcn (`--card`, `--card-foreground`, etc.) snake_cased.
#[derive(Serialize, Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct UiTokens {
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub card: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub card_foreground: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub popover: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub popover_foreground: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub primary: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub primary_foreground: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub secondary: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub secondary_foreground: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub muted: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub muted_foreground: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub destructive: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub border: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub input: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub ring: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub sidebar: Option<ColorU>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_color")]
    pub sidebar_foreground: Option<ColorU>,
}
```

- [ ] **Step 4: Add `opt_hex_color` serde adapter for `Option<ColorU>`**

The existing `hex_color` module (imported at `mod.rs:12` as `use super::color::{hex_color, ...}`) handles bare `ColorU`. For `Option<ColorU>` we need a wrapper. Add this small module inside `mod.rs` near the top (right after the `use` imports):

```rust
mod opt_hex_color {
    use super::ColorU;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(value: &Option<ColorU>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(c) => {
                let hex = format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
                hex.serialize(s)
            }
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ColorU>, D::Error> {
        let opt: Option<String> = Option::deserialize(d)?;
        match opt {
            None => Ok(None),
            Some(s) => {
                let s = s.trim_start_matches('#');
                if s.len() != 6 {
                    return Err(serde::de::Error::custom(format!(
                        "expected 6-digit hex, got '{}'",
                        s
                    )));
                }
                let r = u8::from_str_radix(&s[0..2], 16).map_err(serde::de::Error::custom)?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(serde::de::Error::custom)?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(serde::de::Error::custom)?;
                Ok(Some(ColorU { r, g, b, a: 255 }))
            }
        }
    }
}
```

Before writing, verify the existing `hex_color` module's signatures by reading `crates/warp_core/src/ui/color.rs` (the file `hex_color` lives in — pattern: `pub fn serialize(...)` / `pub fn deserialize(...)`). If the existing module supports `Option<ColorU>` already (e.g., via a wrapping `Option<String>` pattern), reuse it instead — search `with = "hex_color"` for `Option<ColorU>` usages first. If none exist, create `opt_hex_color` as above.

- [ ] **Step 5: Run tests, verify pass**

Run: `cargo test -p warp_core ui_tokens`
Expected: PASS (3 tests).

- [ ] **Step 6: Run full warp_core test suite — no regressions**

Run: `cargo test -p warp_core`
Expected: all pre-existing tests pass.

- [ ] **Step 7: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/mod.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(theme): add UiTokens struct for tweakcn-aligned UI tokens

Optional semantic tokens (card, popover, muted_foreground, border,
sidebar, etc.) that future accessor shims will consult. Adds opt_hex_color
serde adapter so hex strings round-trip cleanly for Option<ColorU>.
No runtime callsites yet — pure schema addition.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 2: Add `ui`, `source`, `source_imported_at` fields to `WarpTheme`

**Files:**
- Modify: `crates/warp_core/src/ui/theme/mod.rs` (lines 588-605, struct definition)
- Test: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Write failing test for `WarpTheme` deserialization with `ui:` block**

Add to `theme_tests.rs`:

```rust
#[test]
fn warp_theme_with_ui_block() {
    let yaml = r#"
accent: '#01a0e4'
background: '#090300'
foreground: '#a5a2a2'
details: darker
terminal_colors:
  normal: { black: '#000000', red: '#db2d20', green: '#01a252', yellow: '#fded02', blue: '#01a0e4', magenta: '#a16a94', cyan: '#b5e4f4', white: '#a5a2a2' }
  bright: { black: '#5c5855', red: '#e8bbd0', green: '#3a3432', yellow: '#4a4543', blue: '#807d7c', magenta: '#d6d5d4', cyan: '#cdab53', white: '#f7f7f7' }
ui:
  card: '#0f0905'
  card_foreground: '#e8e6e3'
  muted_foreground: '#5a5a65'
source: tweakcn
source_imported_at: '2026-05-20T12:00:00Z'
"#;
    let theme: WarpTheme = serde_yaml::from_str(yaml).unwrap();
    let ui = theme.ui.as_ref().expect("ui block present");
    assert_eq!(ui.card.unwrap(), ColorU::from_u32(0x0f0905ff));
    assert_eq!(ui.muted_foreground.unwrap(), ColorU::from_u32(0x5a5a65ff));
    assert_eq!(theme.source.as_deref(), Some("tweakcn"));
}

#[test]
fn warp_theme_without_ui_block_still_parses() {
    let yaml = r#"
accent: '#01a0e4'
background: '#090300'
foreground: '#a5a2a2'
details: darker
terminal_colors:
  normal: { black: '#000000', red: '#db2d20', green: '#01a252', yellow: '#fded02', blue: '#01a0e4', magenta: '#a16a94', cyan: '#b5e4f4', white: '#a5a2a2' }
  bright: { black: '#5c5855', red: '#e8bbd0', green: '#3a3432', yellow: '#4a4543', blue: '#807d7c', magenta: '#d6d5d4', cyan: '#cdab53', white: '#f7f7f7' }
"#;
    let theme: WarpTheme = serde_yaml::from_str(yaml).unwrap();
    assert!(theme.ui.is_none());
    assert!(theme.source.is_none());
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p warp_core warp_theme_with_ui_block warp_theme_without_ui_block`
Expected: FAIL — `ui` field doesn't exist on `WarpTheme`.

- [ ] **Step 3: Add fields to `WarpTheme` struct**

In `crates/warp_core/src/ui/theme/mod.rs`, modify the struct around line 588:

```rust
#[derive(Serialize, Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WarpTheme {
    background: Fill,
    accent: Fill,
    #[serde(with = "hex_color")]
    foreground: ColorU,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    cursor: Option<Fill>,

    #[serde(skip_serializing_if = "Option::is_none")]
    background_image: Option<Image>,

    details: Details,
    terminal_colors: TerminalColors,
    // If name is None, we construct the name by processing the theme .yaml file name
    name: Option<String>,

    /// Optional tweakcn-aligned UI tokens. See `UiTokens` for the schema.
    /// When absent, all accessors fall back to today's derived values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) ui: Option<UiTokens>,

    /// Provenance — set by the import flow; ignored at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,

    /// Provenance — set by the import flow; ignored at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source_imported_at: Option<String>,
}
```

Update `WarpTheme::new` (around line 608) to keep the same signature but default-initialize the three new fields:

```rust
impl WarpTheme {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bg: Fill,
        foreground: ColorU,
        accent: Fill,
        cursor: Option<Fill>,
        details: Option<Details>,
        terminal_colors: TerminalColors,
        background_image: Option<Image>,
        name: Option<String>,
    ) -> Self {
        WarpTheme {
            background: bg,
            foreground,
            accent,
            cursor,
            details: details.unwrap_or_else(|| Details::Custom(CustomDetails::default())),
            terminal_colors,
            background_image,
            name,
            ui: None,
            source: None,
            source_imported_at: None,
        }
    }
    // … rest unchanged …
}
```

Add a `with_ui` builder used only by the import flow (place after `set_name`):

```rust
    pub fn with_ui(mut self, ui: UiTokens, source: impl Into<String>, imported_at: impl Into<String>) -> Self {
        self.ui = Some(ui);
        self.source = Some(source.into());
        self.source_imported_at = Some(imported_at.into());
        self
    }
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p warp_core warp_theme_with_ui_block warp_theme_without_ui_block`
Expected: PASS.

- [ ] **Step 5: Full warp_core regression**

Run: `cargo test -p warp_core`
Expected: all pass. If any pre-existing built-in theme fails to round-trip, surface the diff and fix the new fields' serde defaults before continuing.

- [ ] **Step 6: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/mod.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(theme): add optional ui block + provenance fields to WarpTheme

Three new optional fields (ui, source, source_imported_at) all defaulting
to None so existing themes round-trip unchanged. Adds with_ui() builder
for the upcoming tweakcn import flow.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 3: Add `muted_foreground`, `sidebar_bg`, `ring` accessors

**Files:**
- Modify: `crates/warp_core/src/ui/theme/color.rs` (after the existing `cursor()` accessor around line 134)
- Test: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Write failing tests for the 3 new accessors**

```rust
#[test]
fn muted_foreground_falls_back_to_opencoven_muted() {
    let theme = test_theme_without_ui();
    // OPENCOVEN_MUTED is pub(crate) in app crate, so re-derive its value:
    let expected = ColorU { r: 90, g: 90, b: 101, a: 255 };
    assert_eq!(theme.muted_foreground(), expected);
}

#[test]
fn muted_foreground_uses_ui_override_when_set() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        muted_foreground: Some(ColorU::from_u32(0xabcdefff)),
        ..Default::default()
    });
    assert_eq!(theme.muted_foreground(), ColorU::from_u32(0xabcdefff));
}

#[test]
fn sidebar_bg_falls_back_to_surface_1() {
    let theme = test_theme_without_ui();
    assert_eq!(theme.sidebar_bg(), theme.surface_1());
}

#[test]
fn sidebar_bg_uses_ui_override_when_set() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        sidebar: Some(ColorU::from_u32(0x0a0604ff)),
        ..Default::default()
    });
    assert_eq!(theme.sidebar_bg(), Fill::Solid(ColorU::from_u32(0x0a0604ff)));
}

#[test]
fn ring_falls_back_to_accent() {
    let theme = test_theme_without_ui();
    assert_eq!(theme.ring(), theme.accent());
}
```

Use the existing test helper (search `theme_tests.rs` for `mock_warp_theme` or similar; if none exists, build a minimal `WarpTheme` via `WarpTheme::new(...)` with `mock_terminal_colors()` from `mod.rs:657`). Name the helper `test_theme_without_ui()` for clarity.

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p warp_core muted_foreground sidebar_bg ring`
Expected: FAIL — methods don't exist.

- [ ] **Step 3: Define `MUTED_FOREGROUND_FALLBACK` constant in color.rs**

The fallback `(r=90, g=90, b=101)` matches `OPENCOVEN_MUTED` in `app/src/ai/coven_brand.rs:29`. Mirror it in `color.rs` so `warp_core` is self-contained (it can't depend on the `app` crate). Add near the top, after the `BLOCK_SELECTION_OPACITY` constant (~line 22):

```rust
/// Fallback for `WarpTheme::muted_foreground()` when no `ui.muted_foreground` is set.
/// Matches `OPENCOVEN_MUTED` (#5A5A65) in `app/src/ai/coven_brand.rs` — keep in sync.
const MUTED_FOREGROUND_FALLBACK: ColorU = ColorU { r: 90, g: 90, b: 101, a: 255 };
```

- [ ] **Step 4: Add 3 accessors to `impl WarpTheme`**

Place after the `cursor()` method (around line 136) in `color.rs`:

```rust
    pub fn muted_foreground(&self) -> ColorU {
        self.ui
            .as_ref()
            .and_then(|u| u.muted_foreground)
            .unwrap_or(MUTED_FOREGROUND_FALLBACK)
    }

    pub fn sidebar_bg(&self) -> Fill {
        self.ui
            .as_ref()
            .and_then(|u| u.sidebar)
            .map(Fill::Solid)
            .unwrap_or_else(|| self.surface_1())
    }

    pub fn ring(&self) -> Fill {
        self.ui
            .as_ref()
            .and_then(|u| u.ring)
            .map(Fill::Solid)
            .unwrap_or_else(|| self.accent())
    }
```

Note: `self.ui` is the field added in Task 2 with `pub(crate)` visibility; `color.rs` is inside the same crate so direct access works.

- [ ] **Step 5: Run accessor tests, verify pass**

Run: `cargo test -p warp_core muted_foreground sidebar_bg ring`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/color.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(theme): add muted_foreground/sidebar_bg/ring accessors

New accessors for tokens with no existing home. All fall back to today's
derivation (OPENCOVEN_MUTED constant, surface_1, accent) when ui block
absent. Callsites switch to these in subsequent commits.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 4: Shim `surface_2()` with `ui.card` override

**Files:**
- Modify: `crates/warp_core/src/ui/theme/color.rs:122-124`
- Test: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Backward-compat test for unchanged behavior**

```rust
#[test]
fn surface_2_unchanged_without_ui() {
    let theme = test_theme_without_ui();
    let derived = Fill::Solid(internal_colors::neutral_2(&theme));
    assert_eq!(theme.surface_2(), derived);
}

#[test]
fn surface_2_uses_ui_card_override() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        card: Some(ColorU::from_u32(0x0f0905ff)),
        ..Default::default()
    });
    assert_eq!(theme.surface_2(), Fill::Solid(ColorU::from_u32(0x0f0905ff)));
}
```

`internal_colors::neutral_2` is `pub(super)` in `color.rs`; if the test file can't access it directly, expose a `pub(crate) fn derived_surface_2(theme: &WarpTheme) -> Fill { Fill::Solid(neutral_2(theme)) }` helper inside `color.rs` and call that from the test.

- [ ] **Step 2: Run, verify fail (override case will fail; backward-compat will pass)**

Run: `cargo test -p warp_core surface_2_`
Expected: `surface_2_unchanged_without_ui` PASS, `surface_2_uses_ui_card_override` FAIL.

- [ ] **Step 3: Update `surface_2()` in `color.rs:122`**

Replace:

```rust
    pub fn surface_2(&self) -> Fill {
        Fill::Solid(neutral_2(self))
    }
```

with:

```rust
    pub fn surface_2(&self) -> Fill {
        self.ui
            .as_ref()
            .and_then(|u| u.card)
            .map(Fill::Solid)
            .unwrap_or_else(|| Fill::Solid(neutral_2(self)))
    }
```

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p warp_core surface_2_`
Expected: both PASS.

- [ ] **Step 5: Run full warp_core suite — guard against any test relying on derived value**

Run: `cargo test -p warp_core`
Expected: all pass. If a test fails because it constructed a theme expecting derived `surface_2` and got an override, the test's theme construction needs no change because `test_theme_without_ui()` has `ui = None`.

- [ ] **Step 6: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/color.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(theme): surface_2() consults ui.card before deriving

Themes without a ui block still produce identical Fill::Solid(neutral_2)
output. With ui.card set, every callsite that reads surface_2 (chat panel
bg, settings rows, popovers) picks up the override automatically.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 5: Shim `outline()` with `ui.border` override

**Files:**
- Modify: `crates/warp_core/src/ui/theme/color.rs:154-156`
- Test: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Tests**

```rust
#[test]
fn outline_unchanged_without_ui() {
    let theme = test_theme_without_ui();
    let derived = Fill::Solid(internal_colors::fg_overlay_2(&theme));
    assert_eq!(theme.outline(), derived);
}

#[test]
fn outline_uses_ui_border_override() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        border: Some(ColorU::from_u32(0x2a2520ff)),
        ..Default::default()
    });
    assert_eq!(theme.outline(), Fill::Solid(ColorU::from_u32(0x2a2520ff)));
}
```

(Same access caveat as Task 4 for `fg_overlay_2`.)

- [ ] **Step 2: Run, verify mixed pass/fail**

Run: `cargo test -p warp_core outline_`
Expected: backward-compat PASS, override FAIL.

- [ ] **Step 3: Update `outline()` in `color.rs:154`**

Replace:

```rust
    pub fn outline(&self) -> Fill {
        fg_overlay_2(self)
    }
```

with:

```rust
    pub fn outline(&self) -> Fill {
        self.ui
            .as_ref()
            .and_then(|u| u.border)
            .map(Fill::Solid)
            .unwrap_or_else(|| fg_overlay_2(self))
    }
```

**Important:** `fg_overlay_2(self)` already returns `Fill`, not `ColorU` — confirm by reading `internal_colors::fg_overlay_2` signature. If it returns `Fill`, drop the `Fill::Solid(...)` wrap from the `unwrap_or_else` branch.

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p warp_core outline_`
Expected: both PASS.

- [ ] **Step 5: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/color.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(theme): outline() consults ui.border before deriving

Chat-panel dividers, panel outlines, and every other outline() caller
pick up ui.border automatically when the theme provides one.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 6: Shim `active_ui_text_color()` with `ui.card_foreground` override

**Files:**
- Modify: `crates/warp_core/src/ui/theme/color.rs:186-188`
- Test: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Tests**

```rust
#[test]
fn active_ui_text_color_unchanged_without_ui() {
    let theme = test_theme_without_ui();
    let derived = theme.main_text_color(theme.surface_2());
    assert_eq!(theme.active_ui_text_color(), derived);
}

#[test]
fn active_ui_text_color_uses_ui_card_foreground_override() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        card_foreground: Some(ColorU::from_u32(0xe8e6e3ff)),
        ..Default::default()
    });
    assert_eq!(theme.active_ui_text_color(), Fill::Solid(ColorU::from_u32(0xe8e6e3ff)));
}
```

- [ ] **Step 2: Run, verify mixed**

Run: `cargo test -p warp_core active_ui_text_color_`
Expected: backward-compat PASS, override FAIL.

- [ ] **Step 3: Update `active_ui_text_color()` in `color.rs:186`**

Replace:

```rust
    pub fn active_ui_text_color(&self) -> Fill {
        self.main_text_color(self.surface_2())
    }
```

with:

```rust
    pub fn active_ui_text_color(&self) -> Fill {
        self.ui
            .as_ref()
            .and_then(|u| u.card_foreground)
            .map(Fill::Solid)
            .unwrap_or_else(|| self.main_text_color(self.surface_2()))
    }
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p warp_core active_ui_text_color_`
Expected: both PASS.

- [ ] **Step 5: Full warp_core test pass**

Run: `cargo test -p warp_core`
Expected: all pass.

- [ ] **Step 6: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/color.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(theme): active_ui_text_color() consults ui.card_foreground

Closes the trio of accessor shims. Chat panel primary text, settings
text, and every other active_ui_text_color() caller now honors
ui.card_foreground when set.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 7: Chat panel — replace `OPENCOVEN_MUTED` with `theme.muted_foreground()`

**Files:**
- Modify: `app/src/ai_assistant/panel.rs:1061, 1069, 1211, 1267, 1225`

- [ ] **Step 1: Inspect each callsite**

Read `app/src/ai_assistant/panel.rs:1050-1280` to confirm each occurrence's surrounding `theme` binding. The pattern at each site is `let theme = appearance.theme();` is already in scope (`panel.rs:1055`, `1196`).

- [ ] **Step 2: Make the replacements**

At lines 1061, 1069, 1211, 1267, and 1225 in `app/src/ai_assistant/panel.rs`, replace each `OPENCOVEN_MUTED` with `theme.muted_foreground()`.

Concretely (each replacement is `OPENCOVEN_MUTED` → `theme.muted_foreground()`):

- Line 1061: `let body_color = if dim { OPENCOVEN_MUTED } else { primary };` → `let body_color = if dim { theme.muted_foreground() } else { primary };`
- Line 1069: `font_color: Some(OPENCOVEN_MUTED),` → `font_color: Some(theme.muted_foreground()),`
- Line 1211: `font_color: Some(OPENCOVEN_MUTED),` → `font_color: Some(theme.muted_foreground()),`
- Line 1225: `::ai::cast_agent::SessionStatus::Closed => OPENCOVEN_MUTED,` → `::ai::cast_agent::SessionStatus::Closed => theme.muted_foreground(),`
- Line 1267: `font_color: Some(OPENCOVEN_MUTED),` → `font_color: Some(theme.muted_foreground()),`

After all five replacements, the import at `panel.rs:26` (`use crate::ai::coven_brand::{OPENCOVEN_MUTED, OPENCOVEN_SUCCESS, OPENCOVEN_WARNING};`) becomes `use crate::ai::coven_brand::{OPENCOVEN_SUCCESS, OPENCOVEN_WARNING};` because `OPENCOVEN_MUTED` is no longer referenced. Update the import accordingly.

Do **not** delete the `OPENCOVEN_MUTED` const itself — it remains as the in-`color.rs` fallback target. (It is still `pub(crate)` in `coven_brand.rs`; that's fine even if currently unused — Rust won't warn for `pub(crate)`.)

- [ ] **Step 3: Verify it builds**

Run: `cargo check -p app`
Expected: clean. If a "unused import" warning appears for `OPENCOVEN_MUTED`, you missed a callsite — grep `OPENCOVEN_MUTED` in `app/src/ai_assistant/panel.rs` and confirm zero references.

- [ ] **Step 4: Quick smoke test**

Run any existing chat panel test:
```bash
cargo test -p app ai_assistant
```
Expected: all pre-existing tests pass.

- [ ] **Step 5: Commit (signed)**

```bash
git add app/src/ai_assistant/panel.rs
git commit -S -m "$(cat <<'EOF'
feat(chat): route panel muted text through theme.muted_foreground()

All 5 OPENCOVEN_MUTED callsites in the AI assistant panel now read from
the theme. Default behavior is unchanged (fallback constant in color.rs
matches the brand value); themes with ui.muted_foreground take over.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 8: Sidebar wire-up — switch outer-panel `surface_1()` to `sidebar_bg()`

**Files:**
- Modify: `app/src/workspace/view/left_panel.rs`

- [ ] **Step 1: Identify the outer panel background callsite**

Run:
```bash
grep -n "surface_1" app/src/workspace/view/left_panel.rs
```
Expected: one or more matches. The **outer** panel background — typically the topmost `Container::new(...).with_background(theme.surface_1())` for the left panel root — is the one to switch. Nested controls (rows, hover states) stay on `surface_1()` (this matches spec §"Sidebar wire-up note": outer panel bg only).

If multiple matches, read 3 lines of context around each and pick the one that is the **root** container of the left panel (its parent is the workspace root, not another panel sub-component).

- [ ] **Step 2: Replace outer panel `theme.surface_1()` with `theme.sidebar_bg()`**

Single-line edit. Keep all other `surface_1()` usages as-is.

- [ ] **Step 3: Verify it builds**

Run: `cargo check -p app`
Expected: clean.

- [ ] **Step 4: Test backward-compat — sidebar renders identically without `ui.sidebar`**

There is no automated UI-pixel test for left_panel yet (the integration test in Task 17 will cover the chat panel; sidebar parity is verified manually via the dev-loop in Task 19). Add a brief inline test in `crates/warp_core/src/ui/theme/theme_tests.rs`:

```rust
#[test]
fn sidebar_bg_matches_surface_1_when_ui_absent() {
    let theme = test_theme_without_ui();
    assert_eq!(theme.sidebar_bg(), theme.surface_1());
}
```

(This may duplicate the test from Task 3 — if it already exists there verbatim, skip.)

- [ ] **Step 5: Commit (signed)**

```bash
git add app/src/workspace/view/left_panel.rs crates/warp_core/src/ui/theme/theme_tests.rs
git commit -S -m "$(cat <<'EOF'
feat(workspace): left panel root reads sidebar_bg() not surface_1()

Outer panel bg now honors ui.sidebar when a theme provides it. Nested
controls inside the panel keep using surface_1() so they continue to
derive against background+details. Default behavior unchanged.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 9: OKLCH → sRGB hex conversion (vendored Ottosson formulas)

**Files:**
- Create: `app/src/themes/tweakcn_import.rs`
- Modify: `app/src/themes/mod.rs` (add `pub mod tweakcn_import;`)

- [ ] **Step 1: Write failing reference-value tests**

Create `app/src/themes/tweakcn_import.rs`:

```rust
//! Convert tweakcn CSS exports (OKLCH colors in shadcn token format) into
//! CastCodes `WarpTheme` YAMLs. No new crates — Ottosson's OKLCH → linear
//! sRGB formulas are short enough to vendor.

use pathfinder_color::ColorU;

/// Convert OKLCH (L: 0..1, C: 0..0.4 typical, H: 0..360 degrees) to
/// 8-bit sRGB. Returns `Err((r, g, b))` if any channel was out of the
/// [0,1] linear-sRGB gamut before clamping; `Ok` if inside.
///
/// Algorithm: Björn Ottosson's OKLab → linear sRGB (§"Converting from
/// OKLab" in the published Oklab post) plus the standard linear-sRGB →
/// sRGB transfer function.
pub(crate) fn oklch_to_srgb_u8(l: f64, c: f64, h_deg: f64) -> Result<ColorU, ColorU> {
    let h = h_deg.to_radians();
    let a = c * h.cos();
    let b_ = c * h.sin();

    let l_ = l + 0.3963377774 * a + 0.2158037573 * b_;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b_;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b_;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r_lin =  4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    let g_lin = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    let b_lin = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;

    let in_gamut = (0.0..=1.0).contains(&r_lin)
        && (0.0..=1.0).contains(&g_lin)
        && (0.0..=1.0).contains(&b_lin);

    let to_srgb = |c: f64| {
        let c = c.clamp(0.0, 1.0);
        if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
    };

    let r = (to_srgb(r_lin) * 255.0).round() as u8;
    let g = (to_srgb(g_lin) * 255.0).round() as u8;
    let b = (to_srgb(b_lin) * 255.0).round() as u8;
    let color = ColorU { r, g, b, a: 255 };

    if in_gamut { Ok(color) } else { Err(color) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference: oklch(0 0 0) → black (#000000).
    #[test]
    fn pure_black() {
        let c = oklch_to_srgb_u8(0.0, 0.0, 0.0).unwrap();
        assert_eq!((c.r, c.g, c.b), (0, 0, 0));
    }

    /// Reference: oklch(1 0 0) → white (#FFFFFF).
    #[test]
    fn pure_white() {
        let c = oklch_to_srgb_u8(1.0, 0.0, 0.0).unwrap();
        assert_eq!((c.r, c.g, c.b), (255, 255, 255));
    }

    /// Reference (tweakcn default-dark `--background`): oklch(0.145 0 0)
    /// → ~#252525 (allow ±1 per channel for rounding).
    #[test]
    fn tweakcn_default_dark_background() {
        let c = oklch_to_srgb_u8(0.145, 0.0, 0.0).unwrap();
        let dr = (c.r as i32 - 0x25).abs();
        let dg = (c.g as i32 - 0x25).abs();
        let db = (c.b as i32 - 0x25).abs();
        assert!(dr <= 1 && dg <= 1 && db <= 1, "got #{:02x}{:02x}{:02x}", c.r, c.g, c.b);
    }

    /// Out-of-gamut OKLCH (very saturated red) should return Err but still
    /// produce a clamped representable color.
    #[test]
    fn out_of_gamut_returns_err_but_clamps() {
        // oklch(0.5 0.4 30) — chroma 0.4 is at/beyond sRGB gamut.
        let result = oklch_to_srgb_u8(0.5, 0.4, 30.0);
        assert!(result.is_err(), "expected out-of-gamut");
        let clamped = result.unwrap_err();
        // All channels clamped into [0, 255].
        let _ = clamped; // representable, no further assertion
    }
}
```

- [ ] **Step 2: Register the module**

Edit `app/src/themes/mod.rs` — add a line:

```rust
pub mod tweakcn_import;
```

If `themes/mod.rs` already has `pub mod theme;` etc., follow alphabetical order or end-of-list as the existing convention dictates.

- [ ] **Step 3: Run tests, verify pass**

Run: `cargo test -p app tweakcn_import`
Expected: 4 tests PASS.

- [ ] **Step 4: Commit (signed)**

```bash
git add app/src/themes/tweakcn_import.rs app/src/themes/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(themes): vendor OKLCH to sRGB conversion for tweakcn import

Adds oklch_to_srgb_u8() using Ottosson's published OKLab → linear sRGB
matrix plus the standard sRGB transfer function. Returns Err with a
clamped color when the input is out of gamut so the import flow can
warn the user. No new crates.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 10: tweakcn CSS parser — block extraction

**Files:**
- Modify: `app/src/themes/tweakcn_import.rs`

- [ ] **Step 1: Failing test for block extraction**

Append to `tweakcn_import.rs`:

```rust
#[derive(Debug, PartialEq)]
pub enum ImportError {
    NoColorBlocksFound,
    InvalidOklch { var: String, raw: String },
    OutOfSrgbGamut { var: String, srgb: ColorU },
}

#[derive(Debug, Default, PartialEq)]
pub struct ParsedBlocks {
    pub light: std::collections::HashMap<String, (f64, f64, f64)>, // var → (L, C, H_deg)
    pub dark: std::collections::HashMap<String, (f64, f64, f64)>,
    pub name_comment: Option<String>,
}

/// Pull `:root { ... }` and `.dark { ... }` blocks out of a tweakcn CSS
/// export. Parses each `--var: oklch(L C H);` line into a (L,C,H) triple.
/// `oklch()` is the only color function supported — anything else is
/// silently skipped (tweakcn occasionally emits raw hex for transparency
/// values like shadow color).
pub fn parse_blocks(css: &str) -> Result<ParsedBlocks, ImportError> {
    // implementation in Step 3
    todo!()
}
```

And tests:

```rust
#[cfg(test)]
mod parse_block_tests {
    use super::*;

    const SAMPLE: &str = r#"
/* tweakcn theme: midnight-ember */
:root {
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
}
.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.205 0 0);
}
"#;

    #[test]
    fn extracts_both_blocks() {
        let blocks = parse_blocks(SAMPLE).unwrap();
        assert_eq!(blocks.light.len(), 2);
        assert_eq!(blocks.dark.len(), 3);
    }

    #[test]
    fn block_values_are_parsed() {
        let blocks = parse_blocks(SAMPLE).unwrap();
        let (l, c, h) = blocks.dark["card"];
        assert!((l - 0.205).abs() < 1e-9);
        assert_eq!(c, 0.0);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn name_comment_extracted() {
        let blocks = parse_blocks(SAMPLE).unwrap();
        assert_eq!(blocks.name_comment.as_deref(), Some("midnight-ember"));
    }

    #[test]
    fn no_blocks_errors() {
        let result = parse_blocks("body { color: red; }");
        assert!(matches!(result, Err(ImportError::NoColorBlocksFound)));
    }

    #[test]
    fn only_dark_block_ok() {
        let css = ".dark { --background: oklch(0 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        assert!(blocks.light.is_empty());
        assert_eq!(blocks.dark.len(), 1);
    }
}
```

- [ ] **Step 2: Run, verify fail (todo!() panics)**

Run: `cargo test -p app parse_block_tests`
Expected: all 5 FAIL with panic from `todo!()`.

- [ ] **Step 3: Implement `parse_blocks`**

Replace the `todo!()` body:

```rust
pub fn parse_blocks(css: &str) -> Result<ParsedBlocks, ImportError> {
    let mut blocks = ParsedBlocks::default();

    // Strip CSS comments first; capture the first inline comment as a name hint.
    let mut name_hint = None;
    let mut cleaned = String::with_capacity(css.len());
    let mut i = 0;
    let bytes = css.as_bytes();
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Find closing */
            let start = i + 2;
            let end = css[start..].find("*/").map(|j| start + j).unwrap_or(bytes.len());
            let comment = css[start..end].trim();
            if name_hint.is_none() {
                // Look for "tweakcn theme: <slug>" or just take the comment if it's a single word.
                if let Some(rest) = comment.strip_prefix("tweakcn theme:") {
                    name_hint = Some(rest.trim().to_string());
                } else if !comment.contains(' ') && !comment.is_empty() {
                    name_hint = Some(comment.to_string());
                }
            }
            i = if end < bytes.len() { end + 2 } else { bytes.len() };
        } else {
            cleaned.push(bytes[i] as char);
            i += 1;
        }
    }
    blocks.name_comment = name_hint;

    fn extract_block<'a>(haystack: &'a str, selector: &str) -> Option<&'a str> {
        let needle = format!("{}", selector);
        let start = haystack.find(&needle)?;
        let body_start = haystack[start + needle.len()..].find('{')? + start + needle.len() + 1;
        let mut depth = 1;
        let mut end = body_start;
        for (idx, ch) in haystack[body_start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = body_start + idx;
                        break;
                    }
                }
                _ => {}
            }
        }
        Some(&haystack[body_start..end])
    }

    let parse_decls = |body: &str, target: &mut std::collections::HashMap<String, (f64, f64, f64)>| {
        for decl in body.split(';') {
            let decl = decl.trim();
            if !decl.starts_with("--") { continue; }
            let Some((name, value)) = decl.split_once(':') else { continue };
            let name = name.trim().trim_start_matches("--").to_string();
            let value = value.trim();
            // Only `oklch(L C H[ / a])` is supported; anything else is silently skipped.
            let Some(args) = value.strip_prefix("oklch(").and_then(|s| s.strip_suffix(')')) else {
                continue;
            };
            let triple: Vec<&str> = args.split_whitespace().take(3).collect();
            if triple.len() < 3 { continue; }
            let l: f64 = triple[0].trim_end_matches('%').parse().unwrap_or(f64::NAN);
            // tweakcn emits L as 0..1 (no `%`), but tolerate `%` style:
            let l = if triple[0].ends_with('%') { l / 100.0 } else { l };
            let c: f64 = triple[1].parse().unwrap_or(f64::NAN);
            let h: f64 = triple[2].trim_end_matches("deg").parse().unwrap_or(f64::NAN);
            if l.is_finite() && c.is_finite() && h.is_finite() {
                target.insert(name, (l, c, h));
            }
        }
    };

    if let Some(body) = extract_block(&cleaned, ":root") {
        parse_decls(body, &mut blocks.light);
    }
    if let Some(body) = extract_block(&cleaned, ".dark") {
        parse_decls(body, &mut blocks.dark);
    }

    if blocks.light.is_empty() && blocks.dark.is_empty() {
        return Err(ImportError::NoColorBlocksFound);
    }
    Ok(blocks)
}
```

- [ ] **Step 4: Run, verify pass**

Run: `cargo test -p app parse_block_tests`
Expected: 5 PASS.

- [ ] **Step 5: Commit (signed)**

```bash
git add app/src/themes/tweakcn_import.rs
git commit -S -m "$(cat <<'EOF'
feat(themes): parse :root and .dark CSS blocks into OKLCH triples

Hand-rolled CSS parser pulls --var: oklch(L C H); declarations out of
each block. Tolerates leading comments (used as a theme name hint),
ignores non-oklch values (e.g. hex shadow colors). NoColorBlocksFound
when neither selector is present.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 11: tweakcn → CastCodes mapping (`to_warp_theme`)

**Files:**
- Modify: `app/src/themes/tweakcn_import.rs`

- [ ] **Step 1: Failing test**

```rust
#[cfg(test)]
mod map_tests {
    use super::*;
    use crate::themes::tweakcn_import::*;
    // Reference theme to inherit terminal_colors from.
    fn inherit_from() -> warp_core::ui::theme::WarpTheme {
        // Use a built-in or mock theme exposed via warp_core's test-util feature.
        warp_core::ui::theme::test_util::mock_warp_theme()
    }

    #[test]
    fn maps_dark_block_into_warp_theme() {
        let css = r#"
:root { --background: oklch(1 0 0); --foreground: oklch(0.1 0 0); }
.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --primary: oklch(0.6 0.18 250);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --border: oklch(1 0 0 / 10%);
  --sidebar: oklch(0.205 0 0);
}
"#;
        let blocks = parse_blocks(css).unwrap();
        let base = inherit_from();
        let theme = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Clamp).unwrap();
        let ui = theme.ui.as_ref().expect("ui block set");
        assert!(ui.card.is_some());
        assert!(ui.card_foreground.is_some());
        assert!(ui.muted_foreground.is_some());
        assert!(ui.sidebar.is_some());
        // Provenance written
        assert_eq!(theme.source.as_deref(), Some("tweakcn"));
        // Terminal colors inherited
        assert_eq!(theme.terminal_colors(), base.terminal_colors());
    }

    #[test]
    fn light_mode_uses_root_block() {
        let css = ":root { --background: oklch(1 0 0); --card: oklch(0.95 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = inherit_from();
        let theme = to_warp_theme(&blocks, ThemeMode::Light, &base, GamutPolicy::Clamp).unwrap();
        assert!(theme.ui.as_ref().unwrap().card.is_some());
    }

    #[test]
    fn missing_mode_errors() {
        let css = ":root { --background: oklch(1 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = inherit_from();
        let err = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Clamp).unwrap_err();
        assert_eq!(err, ImportError::NoColorBlocksFound);
    }

    #[test]
    fn out_of_gamut_errors_when_policy_is_strict() {
        let css = ".dark { --background: oklch(0.5 0.4 30); }";
        let blocks = parse_blocks(css).unwrap();
        let base = inherit_from();
        let err = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Strict).unwrap_err();
        assert!(matches!(err, ImportError::OutOfSrgbGamut { .. }));
    }
}
```

Note: this test imports `warp_core::ui::theme::test_util::mock_warp_theme`. If no such helper exists today, expose one in `crates/warp_core/src/ui/theme/mod.rs` behind the existing `cfg(any(test, feature = "test-util"))` block (`mod.rs:656` already has `mock_terminal_colors` under that flag — add a sibling `pub fn mock_warp_theme() -> WarpTheme { … }` that calls `WarpTheme::new(...)` with mock_terminal_colors). Commit that helper in a small companion change inside Task 11 if needed.

- [ ] **Step 2: Implement types + function**

Append to `tweakcn_import.rs`:

```rust
use warp_core::ui::theme::{UiTokens, WarpTheme, TerminalColors};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode { Light, Dark }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamutPolicy { Clamp, Strict }

const TWEAKCN_TOP_LEVEL: &[(&str, TopLevelTarget)] = &[
    ("background", TopLevelTarget::Background),
    ("foreground", TopLevelTarget::Foreground),
    ("primary", TopLevelTarget::Accent),
];

enum TopLevelTarget { Background, Foreground, Accent }

fn convert(
    triple: (f64, f64, f64),
    var: &str,
    policy: GamutPolicy,
) -> Result<ColorU, ImportError> {
    match oklch_to_srgb_u8(triple.0, triple.1, triple.2) {
        Ok(c) => Ok(c),
        Err(clamped) => match policy {
            GamutPolicy::Clamp => Ok(clamped),
            GamutPolicy::Strict => Err(ImportError::OutOfSrgbGamut {
                var: var.to_string(),
                srgb: clamped,
            }),
        },
    }
}

pub fn to_warp_theme(
    blocks: &ParsedBlocks,
    mode: ThemeMode,
    inherit_terminal_from: &WarpTheme,
    policy: GamutPolicy,
) -> Result<WarpTheme, ImportError> {
    let block = match mode {
        ThemeMode::Dark => &blocks.dark,
        ThemeMode::Light => &blocks.light,
    };
    if block.is_empty() {
        return Err(ImportError::NoColorBlocksFound);
    }

    // Pull required top-level fields, falling back to inherit_from if absent.
    let lookup = |key: &str| -> Result<Option<ColorU>, ImportError> {
        match block.get(key) {
            Some(&t) => convert(t, key, policy).map(Some),
            None => Ok(None),
        }
    };

    let background = lookup("background")?
        .map(|c| Fill::Solid(c))
        .unwrap_or_else(|| inherit_terminal_from.background());
    let foreground = lookup("foreground")?.unwrap_or_else(|| inherit_terminal_from.foreground_color());
    let accent = lookup("primary")?
        .map(|c| Fill::Solid(c))
        .unwrap_or_else(|| inherit_terminal_from.accent());

    let mut ui = UiTokens::default();
    for (css_name, set) in tweakcn_ui_mapping() {
        if let Some(&triple) = block.get(css_name) {
            let c = convert(triple, css_name, policy)?;
            set(&mut ui, c);
        }
    }

    let theme = WarpTheme::new(
        background,
        foreground,
        accent,
        None,
        None, // Details: default to derived (Darker)
        inherit_terminal_from.terminal_colors().clone(),
        None,
        blocks.name_comment.clone(),
    );
    let now = chrono::Utc::now().to_rfc3339();
    Ok(theme.with_ui(ui, "tweakcn", now))
}

fn tweakcn_ui_mapping() -> &'static [(&'static str, fn(&mut UiTokens, ColorU))] {
    &[
        ("card",                |u, c| u.card = Some(c)),
        ("card-foreground",     |u, c| u.card_foreground = Some(c)),
        ("popover",             |u, c| u.popover = Some(c)),
        ("popover-foreground",  |u, c| u.popover_foreground = Some(c)),
        ("primary",             |u, c| u.primary = Some(c)),
        ("primary-foreground",  |u, c| u.primary_foreground = Some(c)),
        ("secondary",           |u, c| u.secondary = Some(c)),
        ("secondary-foreground",|u, c| u.secondary_foreground = Some(c)),
        ("muted",               |u, c| u.muted = Some(c)),
        ("muted-foreground",    |u, c| u.muted_foreground = Some(c)),
        ("destructive",         |u, c| u.destructive = Some(c)),
        ("border",              |u, c| u.border = Some(c)),
        ("input",               |u, c| u.input = Some(c)),
        ("ring",                |u, c| u.ring = Some(c)),
        ("sidebar",             |u, c| u.sidebar = Some(c)),
        ("sidebar-foreground",  |u, c| u.sidebar_foreground = Some(c)),
    ]
}
```

Notes:
- `WarpTheme::background()` returns `Fill`; `foreground_color()` returns `ColorU`. If those getter names differ in the current code (e.g., `foreground()` returns `Fill`), adjust the expressions to read the right getter — read `crates/warp_core/src/ui/theme/color.rs:92-108` to confirm.
- `chrono` is already a transitive dep of the app crate via `warp_core` — verify with `cargo tree -p app -i chrono`. If absent, swap for `std::time::SystemTime::now()` formatted as RFC 3339 by hand (or skip the timestamp and just store the date).

- [ ] **Step 3: Run mapping tests**

Run: `cargo test -p app map_tests`
Expected: 4 PASS. If `mock_warp_theme` was needed, also re-run `cargo test -p warp_core` to confirm the test-util addition doesn't break anything.

- [ ] **Step 4: Commit (signed)**

```bash
git add app/src/themes/tweakcn_import.rs crates/warp_core/src/ui/theme/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(themes): map parsed tweakcn blocks to WarpTheme

to_warp_theme() applies tweakcn → CastCodes naming, converts each OKLCH
triple through the vendored Ottosson formula, and inherits terminal_colors
from the user's currently-selected base theme (tweakcn has no ANSI palette).
Supports per-call GamutPolicy::Clamp (default) and Strict (warning UI).
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 12: YAML writer for imported themes

**Files:**
- Modify: `app/src/themes/tweakcn_import.rs`

- [ ] **Step 1: Failing test for writer**

```rust
#[cfg(test)]
mod writer_tests {
    use super::*;
    use std::path::PathBuf;

    fn tmpdir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("cc-tweakcn-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn writes_dark_only_when_no_light_block() {
        let dir = tmpdir();
        let css = ".dark { --background: oklch(0.145 0 0); --card: oklch(0.2 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::test_util::mock_warp_theme();
        let written = write_imported(
            &blocks,
            "my-theme",
            &base,
            GamutPolicy::Clamp,
            &dir,
        ).unwrap();
        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with("my-theme.yaml"));
        assert!(std::fs::read_to_string(&written[0]).unwrap().contains("ui:"));
    }

    #[test]
    fn writes_both_when_both_blocks_present() {
        let dir = tmpdir();
        let css = ":root { --background: oklch(1 0 0); } .dark { --background: oklch(0 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::test_util::mock_warp_theme();
        let written = write_imported(
            &blocks,
            "duo",
            &base,
            GamutPolicy::Clamp,
            &dir,
        ).unwrap();
        let names: Vec<String> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"duo.yaml".to_string()));
        assert!(names.contains(&"duo-light.yaml".to_string()));
    }
}
```

Note: this test uses `uuid` — if not a dev-dep already, replace the `tmpdir()` body with `tempfile::TempDir::new().unwrap().into_path()` (the `tempfile` crate is almost certainly already in the workspace).

- [ ] **Step 2: Implement `write_imported`**

Append to `tweakcn_import.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn write_imported(
    blocks: &ParsedBlocks,
    slug: &str,
    inherit_terminal_from: &WarpTheme,
    policy: GamutPolicy,
    themes_dir: &Path,
) -> Result<Vec<PathBuf>, ImportError> {
    let mut written = Vec::new();
    let slug = sanitize_slug(slug);
    let primary_path = themes_dir.join(format!("{}.yaml", slug));
    let light_path = themes_dir.join(format!("{}-light.yaml", slug));

    if !blocks.dark.is_empty() {
        let theme = to_warp_theme(blocks, ThemeMode::Dark, inherit_terminal_from, policy)?;
        let yaml = serde_yaml::to_string(&theme).expect("serialize theme");
        std::fs::write(&primary_path, yaml).map_err(io_to_import)?;
        written.push(primary_path);
    }
    if !blocks.light.is_empty() {
        let theme = to_warp_theme(blocks, ThemeMode::Light, inherit_terminal_from, policy)?;
        let yaml = serde_yaml::to_string(&theme).expect("serialize theme");
        std::fs::write(&light_path, yaml).map_err(io_to_import)?;
        written.push(light_path);
    }
    Ok(written)
}

fn sanitize_slug(s: &str) -> String {
    let cleaned: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse runs of '-' and trim leading/trailing.
    let mut out = String::new();
    let mut prev_dash = true;
    for c in cleaned.chars() {
        if c == '-' {
            if !prev_dash { out.push(c); }
            prev_dash = true;
        } else {
            out.push(c);
            prev_dash = false;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "imported-theme".to_string() } else { out }
}

fn io_to_import(_: std::io::Error) -> ImportError {
    // Map filesystem errors into a generic import error for now. Surface as
    // a banner in the modal; details flow through the panic in serde_yaml
    // are unlikely to happen for in-process strings.
    ImportError::NoColorBlocksFound
}
```

Better: add a dedicated `ImportError::Io(String)` variant rather than collapsing into `NoColorBlocksFound`. Update the enum at the top of the file:

```rust
#[derive(Debug, PartialEq)]
pub enum ImportError {
    NoColorBlocksFound,
    InvalidOklch { var: String, raw: String },
    OutOfSrgbGamut { var: String, srgb: ColorU },
    Io(String),
}
```

…and `io_to_import` returns `ImportError::Io(e.to_string())`.

- [ ] **Step 3: Run writer tests, verify pass**

Run: `cargo test -p app writer_tests`
Expected: 2 PASS.

- [ ] **Step 4: Commit (signed)**

```bash
git add app/src/themes/tweakcn_import.rs
git commit -S -m "$(cat <<'EOF'
feat(themes): write_imported() persists imported themes to disk

Slug-sanitizes the name, writes <slug>.yaml for the dark block and
<slug>-light.yaml for the light block (whichever are present). Returns
the list of written paths so callers can refresh the loader and select
the new theme.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 13: Fixture + golden YAML for snapshot testing

**Files:**
- Create: `crates/integration/tests/data/tweakcn_sample.css`
- Create: `crates/integration/tests/data/tweakcn_sample_dark.yaml`
- Create: `crates/integration/tests/data/tweakcn_sample_light.yaml`

- [ ] **Step 1: Add CSS fixture**

Save to `crates/integration/tests/data/tweakcn_sample.css`:

```css
/* tweakcn theme: cast-codes-sample */
:root {
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.145 0 0);
  --secondary: oklch(0.97 0 0);
  --secondary-foreground: oklch(0.205 0 0);
  --muted: oklch(0.97 0 0);
  --muted-foreground: oklch(0.556 0 0);
  --destructive: oklch(0.577 0.245 27.325);
  --border: oklch(0.922 0 0);
  --input: oklch(0.922 0 0);
  --ring: oklch(0.708 0 0);
  --sidebar: oklch(0.985 0 0);
  --sidebar-foreground: oklch(0.145 0 0);
}

.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --primary: oklch(0.922 0 0);
  --primary-foreground: oklch(0.205 0 0);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.269 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --secondary: oklch(0.269 0 0);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --destructive: oklch(0.704 0.191 22.216);
  --border: oklch(0.269 0 0);
  --input: oklch(0.371 0 0);
  --ring: oklch(0.439 0 0);
  --sidebar: oklch(0.205 0 0);
  --sidebar-foreground: oklch(0.985 0 0);
}
```

- [ ] **Step 2: Generate golden YAMLs by running the parser**

Write a throwaway Rust binary in `crates/integration/tests/data/gen_golden.rs` (or use a `#[test]` with `--ignored`) that calls `parse_blocks` + `to_warp_theme` + `serde_yaml::to_string` for both modes and prints the YAML. Run it once, copy the output into the two golden files, then delete the throwaway. Alternative: drive it through a regular `#[test]` that writes to the golden path on first run; subsequent runs assert equality.

Recommended: do it inline as a test in `app/src/themes/tweakcn_import.rs`:

```rust
#[test]
#[ignore] // Run with `cargo test -- --ignored regenerate_golden` to refresh fixtures.
fn regenerate_golden() {
    let css = include_str!("../../../crates/integration/tests/data/tweakcn_sample.css");
    let blocks = parse_blocks(css).unwrap();
    let base = warp_core::ui::theme::test_util::mock_warp_theme();
    let dark = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Clamp).unwrap();
    let light = to_warp_theme(&blocks, ThemeMode::Light, &base, GamutPolicy::Clamp).unwrap();
    std::fs::write(
        "../crates/integration/tests/data/tweakcn_sample_dark.yaml",
        serde_yaml::to_string(&dark).unwrap(),
    ).unwrap();
    std::fs::write(
        "../crates/integration/tests/data/tweakcn_sample_light.yaml",
        serde_yaml::to_string(&light).unwrap(),
    ).unwrap();
}
```

Run once:

```bash
cargo test -p app regenerate_golden -- --ignored
```

Hand-verify the resulting YAML against the spec's expected field naming (`card`, `card_foreground`, etc.). Commit only the YAML files; the `#[test]` stays for future regenerations.

- [ ] **Step 3: Snapshot assertion test (live, non-ignored)**

Add to `tweakcn_import.rs`:

```rust
#[test]
fn dark_block_matches_golden() {
    let css = include_str!("../../../crates/integration/tests/data/tweakcn_sample.css");
    let golden = include_str!("../../../crates/integration/tests/data/tweakcn_sample_dark.yaml");
    let blocks = parse_blocks(css).unwrap();
    let base = warp_core::ui::theme::test_util::mock_warp_theme();
    let theme = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Clamp).unwrap();
    let actual = serde_yaml::to_string(&theme).unwrap();
    // Strip the `source_imported_at` line because it embeds wall-clock time.
    let strip = |s: &str| s.lines().filter(|l| !l.contains("source_imported_at")).collect::<Vec<_>>().join("\n");
    assert_eq!(strip(&actual), strip(golden));
}
```

(Repeat for the light block.)

- [ ] **Step 4: Run snapshot tests**

Run: `cargo test -p app dark_block_matches_golden light_block_matches_golden`
Expected: PASS.

- [ ] **Step 5: Commit (signed)**

```bash
git add crates/integration/tests/data/tweakcn_sample.css crates/integration/tests/data/tweakcn_sample_dark.yaml crates/integration/tests/data/tweakcn_sample_light.yaml app/src/themes/tweakcn_import.rs
git commit -S -m "$(cat <<'EOF'
test(themes): snapshot golden YAMLs for tweakcn sample export

Fixture is a real tweakcn export (cast-codes-sample). Two golden YAMLs
(dark + light) lock in the OKLCH → sRGB conversion + tweakcn → CastCodes
mapping. Regeneration test is ignored by default; run with --ignored.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 14: "Import theme…" button on the Appearance page

**Files:**
- Modify: `app/src/settings_view/appearance_page.rs`

- [ ] **Step 1: Locate the theme picker row in `appearance_page.rs`**

Search for `ThemeChooserMode` (used at `appearance_page.rs:63`) and find the rendered "Theme" row (likely near the top of `fn render` body, line 691+). The button should sit next to the existing theme picker.

```bash
grep -n "Theme\|theme_chooser\|ShowThemeChooser" app/src/settings_view/appearance_page.rs | head -10
```

- [ ] **Step 2: Add an `ImportTheme` variant to `AppearancePageAction`**

Find the existing `AppearancePageAction` enum (used by `tab_close_button_position_dropdown` etc. — search the file). Add a variant:

```rust
ShowImportThemeModal,
```

Wire it in the `on_action` / dispatch match (whatever pattern the file already uses) to call a new method `self.show_import_theme_modal(ctx)` defined in Step 3.

- [ ] **Step 3: Implement the action handler**

Add to `impl AppearancePage` (or whatever the type is):

```rust
fn show_import_theme_modal(&mut self, ctx: &mut ViewContext<Self>) {
    // Dispatch a WorkspaceAction that the workspace will translate into a
    // modal mount, mirroring the existing theme_creator_modal flow.
    ctx.dispatch_action(WorkspaceAction::ShowImportThemeModal);
}
```

Add `WorkspaceAction::ShowImportThemeModal` to the workspace action enum (search the codebase for `WorkspaceAction::ShowThemeChooser` to find the file — likely `app/src/workspace/action.rs` or similar — and add a sibling variant).

- [ ] **Step 4: Render the button next to the theme picker**

In the existing render block where the "Theme" row is built, add a sibling button using `ActionButton` (already imported at `appearance_page.rs:112`). Pattern:

```rust
.with_child(
    ActionButton::new(
        ButtonSize::Medium,
        NakedTheme::Default,
        ui_builder.button(...)
    )
    .with_label("Import theme…")
    .on_click(|ctx| ctx.dispatch_action(AppearancePageAction::ShowImportThemeModal))
    .finish(),
)
```

The exact shape depends on the surrounding row builder. Read 20 lines around the existing theme-chooser button and mirror its construction style.

- [ ] **Step 5: Build, no UI test yet**

Run: `cargo check -p app`
Expected: clean. UI smoke test happens in the dev-loop (Task 19).

- [ ] **Step 6: Commit (signed)**

```bash
git add app/src/settings_view/appearance_page.rs app/src/workspace/action.rs  # adjust path
git commit -S -m "$(cat <<'EOF'
feat(settings): add 'Import theme…' button on Appearance page

Button sits next to the existing theme chooser and dispatches
WorkspaceAction::ShowImportThemeModal. Modal implementation lands in
the next commit.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 15: Import-theme modal — paste box + parse + save

**Files:**
- Create: `app/src/settings_view/import_theme_modal.rs`
- Modify: `app/src/settings_view/mod.rs` (re-export)
- Modify: workspace modal mount point (file TBD — find via `theme_creator_modal` pattern)

- [ ] **Step 1: Skim the existing `theme_creator_modal.rs` for the modal pattern**

```bash
sed -n '1,80p' app/src/themes/theme_creator_modal.rs
```

Mirror its structure: `pub struct ImportThemeModal { ... }`, `impl View for ImportThemeModal { fn render(...) ... }`, plus a constructor `new(ctx) -> Self`. Match the same `ModalContent` / `Modal` chrome.

- [ ] **Step 2: Define the modal state**

`app/src/settings_view/import_theme_modal.rs`:

```rust
use std::path::PathBuf;
use warpui::view::{View, ViewContext, ViewHandle};
use warpui::ui_components::button::ButtonVariant;
use crate::themes::tweakcn_import::{
    parse_blocks, write_imported, GamutPolicy, ImportError, ParsedBlocks,
};
use crate::workspace::Appearance;

pub struct ImportThemeModal {
    css_text: String,
    name: String,
    parse_result: Option<Result<ParsedBlocks, ImportError>>,
    clamp_out_of_gamut: bool,
}

impl ImportThemeModal {
    pub fn new() -> Self {
        Self {
            css_text: String::new(),
            name: String::new(),
            parse_result: None,
            clamp_out_of_gamut: true,
        }
    }

    fn on_paste_changed(&mut self, new_text: String) {
        self.css_text = new_text;
        self.parse_result = Some(parse_blocks(&self.css_text));
        if let Some(Ok(blocks)) = &self.parse_result {
            if self.name.is_empty() {
                if let Some(hint) = &blocks.name_comment {
                    self.name = hint.clone();
                }
            }
        }
    }

    fn can_save(&self) -> bool {
        matches!(&self.parse_result, Some(Ok(blocks)) if !blocks.light.is_empty() || !blocks.dark.is_empty())
            && !self.name.trim().is_empty()
    }

    fn save(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(Ok(blocks)) = self.parse_result.as_ref() else { return };
        let appearance = ctx.app::<Appearance>(); // see existing modal for the right accessor
        let base = appearance.theme();
        let themes_dir = crate::user_config::native::user_themes_dir();
        let policy = if self.clamp_out_of_gamut { GamutPolicy::Clamp } else { GamutPolicy::Strict };

        match write_imported(blocks, &self.name, &base, policy, &themes_dir) {
            Ok(paths) => {
                // Refresh the theme loader so the new YAML is discoverable.
                ctx.dispatch_action(crate::workspace::action::WorkspaceAction::ReloadThemes);
                // Best-effort: select the first (dark) theme written.
                if let Some(path) = paths.first() {
                    let slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    ctx.dispatch_action(crate::workspace::action::WorkspaceAction::SelectTheme(slug.to_string()));
                }
                ctx.dispatch_action(crate::workspace::action::WorkspaceAction::CloseModal);
            }
            Err(e) => {
                // Surface in a status row on the modal — keep modal open.
                // Push the error into self for next render.
                self.show_error = Some(format!("{:?}", e));
                ctx.notify(); // re-render
            }
        }
    }
}

impl View for ImportThemeModal {
    fn render(&self, app: &warpui::view::AppContext) -> Box<dyn warpui::elements::Element> {
        // TODO: build modal UI here — heading "Import theme from tweakcn",
        // multi-line text field bound to on_paste_changed, name input,
        // "Clamp out-of-gamut colors" checkbox, preview row (4 swatches),
        // Save / Cancel buttons. See `theme_creator_modal.rs` for the
        // construction style.
        todo!("modal render — mirror theme_creator_modal layout")
    }
}
```

Critical caveats:
- `Appearance`, `user_config::native::user_themes_dir`, `WorkspaceAction::ReloadThemes`/`SelectTheme`/`CloseModal` are all placeholders for the actual names in this repo. Resolve each by reading: `app/src/themes/theme_creator_modal.rs` (modal close + theme select pattern), `app/src/user_config/native.rs:168` (themes dir), `app/src/workspace/action.rs` (action variants). Add new variants where needed and wire them in workspace dispatch.
- The "TODO" inside `render` is **the only TODO allowed in this plan** because the visual layout closely mirrors `theme_creator_modal.rs` and the easiest way to write it correctly is to read that file first and translate row-by-row. The implementer should expand the TODO into a concrete element tree before the commit.

- [ ] **Step 3: Render the modal — concrete layout**

Read `app/src/themes/theme_creator_modal.rs` end-to-end. Then replace the `todo!` body with:

1. `Container::new(Flex::column()...)` as the outer modal frame, padded 16px, background `theme.surface_2()`.
2. Heading: `"Import theme from tweakcn"` at 16px/Semibold.
3. Multi-line text field: bind to `self.css_text`; on every change call `self.on_paste_changed(new)`. Use the existing `TextField` widget (search `appearance_page.rs` for how multi-line input is created).
4. Name `TextField`: single-line, slug-validated on submit (`sanitize_slug` from `tweakcn_import.rs`).
5. "Light/Dark detected" badge row that displays which blocks parsed.
6. "Clamp out-of-gamut colors" checkbox bound to `self.clamp_out_of_gamut`.
7. Save button (`ButtonVariant::Accent`) disabled when `!self.can_save()`; calls `self.save(ctx)`.
8. Cancel button (`ButtonVariant::Text`) dispatching `CloseModal`.
9. Error banner area showing `self.show_error` if set.

(No need to enumerate the exact `Flex::row().with_child(...).with_child(...)` boilerplate here — the implementer mirrors `theme_creator_modal.rs` element-for-element.)

- [ ] **Step 4: Register the modal in the workspace mount table**

Find the file that translates `WorkspaceAction::ShowThemeChooser` into a mounted modal (`app/src/themes/theme_chooser.rs` or `app/src/workspace/...`). Add a sibling arm for `WorkspaceAction::ShowImportThemeModal` that constructs `ImportThemeModal::new()` and mounts it. Mirror the close pattern for `WorkspaceAction::CloseModal`.

- [ ] **Step 5: Verify it builds**

Run: `cargo check -p app`
Expected: clean.

- [ ] **Step 6: Commit (signed)**

```bash
git add app/src/settings_view/import_theme_modal.rs app/src/settings_view/mod.rs app/src/workspace/  # adjust as needed
git commit -S -m "$(cat <<'EOF'
feat(settings): import-theme modal for tweakcn CSS exports

Paste-in or drop-in modal that parses tweakcn CSS, previews the four
key swatches, validates the slug, and writes the resulting YAML(s) to
~/.config/warp/themes/. Refreshes the theme loader and selects the new
theme on save. No network calls.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 16: Drag-and-drop CSS file support in the modal

**Files:**
- Modify: `app/src/settings_view/import_theme_modal.rs`

- [ ] **Step 1: Find an existing drop-zone pattern**

```bash
grep -rn "drag\|drop_target\|DropTarget\|FileDrop" app/src/ | head -20
```

Mirror whatever pattern the codebase uses (likely a `DropTarget` widget or an event handler on a container).

- [ ] **Step 2: Wire a `.css`-only drop handler**

In `ImportThemeModal::render`, wrap the paste TextField in a drop-target container. On file drop:

```rust
fn on_file_dropped(&mut self, path: PathBuf, ctx: &mut ViewContext<Self>) {
    if path.extension().and_then(|s| s.to_str()) != Some("css") {
        self.show_error = Some("Only .css files are supported.".to_string());
        ctx.notify();
        return;
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            // Use the filename stem as a default slug if the modal name is empty.
            if self.name.is_empty() {
                self.name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported-theme")
                    .to_string();
            }
            self.on_paste_changed(contents);
        }
        Err(e) => {
            self.show_error = Some(format!("Read failed: {}", e));
        }
    }
    ctx.notify();
}
```

- [ ] **Step 3: Verify it builds**

Run: `cargo check -p app`
Expected: clean.

- [ ] **Step 4: Commit (signed)**

```bash
git add app/src/settings_view/import_theme_modal.rs
git commit -S -m "$(cat <<'EOF'
feat(settings): accept dropped .css files in the import modal

Drag-and-drop a tweakcn export onto the modal to populate the paste box.
Rejects non-.css files with an inline error. Uses the filename stem as
the default slug.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 17: Integration test — `ui:`-block theme drives chat panel pixels

**Files:**
- Create: `crates/integration/tests/data/ui_block_theme.yaml` (a hand-written theme using a unique `ui.card` color)
- Create or modify: `crates/integration/tests/theme_ui_block.rs` (or wherever the existing Builder/TestStep tests live)

- [ ] **Step 1: Consult the integration-test skill**

This task should be performed with the `warp-integration-test` skill loaded. The skill knows the Builder/TestStep idioms — invoke it before writing the test.

- [ ] **Step 2: Hand-author the fixture theme**

`crates/integration/tests/data/ui_block_theme.yaml`:

```yaml
accent: '#01a0e4'
background: '#090300'
foreground: '#a5a2a2'
details: darker
terminal_colors:
  normal: { black: '#000000', red: '#db2d20', green: '#01a252', yellow: '#fded02', blue: '#01a0e4', magenta: '#a16a94', cyan: '#b5e4f4', white: '#a5a2a2' }
  bright: { black: '#5c5855', red: '#e8bbd0', green: '#3a3432', yellow: '#4a4543', blue: '#807d7c', magenta: '#d6d5d4', cyan: '#cdab53', white: '#f7f7f7' }
ui:
  card: '#ff00ff'              # SENTINEL — unmistakable in pixel inspection
  card_foreground: '#00ff00'
  muted_foreground: '#ffff00'
  border: '#00ffff'
```

The four sentinel colors are easy to assert against rendered pixels.

- [ ] **Step 3: Write the test**

Using the Builder/TestStep idiom:

```rust
// crates/integration/tests/theme_ui_block.rs
#[test]
fn ui_block_drives_chat_panel_pixels() {
    let theme_path = "tests/data/ui_block_theme.yaml";
    integration::Builder::new()
        .with_theme_from_yaml(theme_path)
        .open_ai_panel()
        .render_frame()
        .assert_pixel_at_chat_panel_bg(ColorU::from_u32(0xff00ffff))
        .assert_pixel_at_muted_secondary_text(ColorU::from_u32(0xffff00ff))
        .run();
}
```

The exact method names depend on what the Builder framework exposes; consult the skill output for the right combinators.

- [ ] **Step 4: Run the integration test**

Run: `cargo test -p integration ui_block_drives_chat_panel_pixels --release`
Expected: PASS. (Use `--release` if the existing integration tests do; check the suite's CI config.)

- [ ] **Step 5: Commit (signed)**

```bash
git add crates/integration/tests/data/ui_block_theme.yaml crates/integration/tests/theme_ui_block.rs
git commit -S -m "$(cat <<'EOF'
test(integration): ui block overrides chat panel pixels

Loads a theme with sentinel #ff00ff/#ffff00 in the ui block and asserts
those exact colors appear in the rendered chat-panel background and
muted secondary text. Locks in the end-to-end flow from YAML to pixel.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 18: Backward-compat pixel parity for built-in themes

**Files:**
- Modify: `crates/warp_core/src/ui/theme/theme_tests.rs`

- [ ] **Step 1: Table test across all built-ins**

```rust
#[test]
fn builtin_themes_render_identically_without_ui_block() {
    // Pull every built-in theme via the existing accessor on default_themes.
    let builtins = crate::ui::theme::test_util::builtin_themes_for_test();
    for theme in builtins {
        assert!(theme.ui.is_none(), "built-in {:?} unexpectedly carries a ui block", theme.name());
        // Sanity: all three accessors return their derived values.
        let derived_surface_2 = Fill::Solid(internal_colors::neutral_2(&theme));
        let derived_outline = internal_colors::fg_overlay_2(&theme); // adjust type wrap if Fill
        let derived_text = theme.main_text_color(theme.surface_2());

        assert_eq!(theme.surface_2(), derived_surface_2, "surface_2 drift for {:?}", theme.name());
        assert_eq!(theme.outline(), derived_outline, "outline drift for {:?}", theme.name());
        assert_eq!(theme.active_ui_text_color(), derived_text, "text drift for {:?}", theme.name());
    }
}
```

`test_util::builtin_themes_for_test()` likely needs adding alongside the `mock_warp_theme` helper from Task 11 — it should iterate over `default_themes::ALL_THEMES` (or whatever the export looks like) and return owned `WarpTheme`s. Read `app/src/themes/default_themes.rs` to find the right way to list every built-in.

- [ ] **Step 2: Run, verify pass**

Run: `cargo test -p warp_core builtin_themes_render_identically`
Expected: PASS.

- [ ] **Step 3: Commit (signed)**

```bash
git add crates/warp_core/src/ui/theme/theme_tests.rs crates/warp_core/src/ui/theme/mod.rs
git commit -S -m "$(cat <<'EOF'
test(theme): backward-compat pixel parity for all built-in themes

Table test asserting surface_2(), outline(), and active_ui_text_color()
return the same derived values as before this PR for every built-in.
Closes the backward-compatibility loop required by the spec.
EOF
)"
git log -1 --show-signature | grep -q 'Good .* signature' || (echo "UNSIGNED COMMIT — STOP" && exit 1)
```

---

## Task 19: Presubmit + manual smoke test

**Files:** none (CI / dev-loop)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --workspace`
Expected: all green.

- [ ] **Step 2: Run presubmit / lints**

Use the `castcodes-dev-loop` skill or run the equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 3: Manual smoke**

1. Build and launch the app: `cargo run --bin warp --release` (or whatever the public-OSS binary entry is — confirm via `castcodes-dev-loop`).
2. Open Settings → Appearance.
3. Click "Import theme…".
4. Paste the contents of `crates/integration/tests/data/tweakcn_sample.css`.
5. Verify: name input prefills to `cast-codes-sample`; both light/dark badges show; Save is enabled.
6. Click Save.
7. Confirm the theme appears in the picker and is selected. Chat panel bg should visibly change.
8. Open `~/.config/warp/themes/cast-codes-sample.yaml` — verify the `ui:` block was written.
9. Restart the app — confirm the imported theme reloads from disk.
10. Revert to a built-in theme; confirm pixel-for-pixel it matches what shipped before.

- [ ] **Step 4: No commit — this task is verification only**

Manual smoke produces no artifacts. If a regression surfaces, file an issue or reopen a task above with the failure.

---

## Final commit policy reminder

Before pushing any branch:

```bash
git log origin/main..HEAD --pretty='%H %G?' | awk '$2 != "G" {print "UNSIGNED:", $0}'
```

If anything prints, **do not push**. Sign the missing commits (rebase + sign, or amend with `-S`) before pushing.

---

## Spec coverage check

| Spec section | Tasks |
| --- | --- |
| §1 Schema | Task 1 (`UiTokens`), Task 2 (fields on `WarpTheme`) |
| §2 Accessor changes | Task 3 (new accessors), Tasks 4-6 (shims) |
| §3 Chat panel wire-up | Task 7 |
| §4 tweakcn parser | Tasks 9, 10, 11, 12 |
| §5 Import UX | Tasks 14, 15, 16 |
| §6 Migration / backward-compat | Tasks 4-6 (in-line tests), Task 18 (full built-in parity) |
| §7 Testing strategy | Tasks 9 (OKLCH unit), 10 (parser unit), 13 (snapshot), 17 (integration), 18 (backward-compat) |
| §8 Risk: out-of-gamut | Task 11 (`GamutPolicy`), Task 15 (modal toggle) |
| §8 Risk: contrast | Out of scope for v1 — spec calls it "warn only" + non-blocking; revisit in a follow-up |
| Acceptance criterion 1 (ui block applies) | Task 17 |
| Acceptance criterion 2 (no-ui-block pixel parity) | Task 18 |
| Acceptance criterion 3 (paste → ≤2s, no network) | Tasks 9-16 (no network anywhere); Task 19 (manual timing) |
| Acceptance criterion 4 (light + dark paired YAMLs) | Task 12 |
| Acceptance criterion 5 (out-of-gamut clamped/named) | Tasks 11, 15 |
| Acceptance criterion 6 (`cargo test` + presubmit clean) | Task 19 |

WCAG contrast warning (spec §8) is intentionally deferred — note it as a follow-up issue.
