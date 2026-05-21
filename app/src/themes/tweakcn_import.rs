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

    let r_lin = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    let g_lin = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    let b_lin = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;

    let in_gamut = (0.0..=1.0).contains(&r_lin)
        && (0.0..=1.0).contains(&g_lin)
        && (0.0..=1.0).contains(&b_lin);

    let to_srgb = |c: f64| {
        let c = c.clamp(0.0, 1.0);
        if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    };

    let r = (to_srgb(r_lin) * 255.0).round() as u8;
    let g = (to_srgb(g_lin) * 255.0).round() as u8;
    let b = (to_srgb(b_lin) * 255.0).round() as u8;
    let color = ColorU { r, g, b, a: 255 };

    if in_gamut {
        Ok(color)
    } else {
        Err(color)
    }
}

#[derive(Debug, PartialEq)]
pub enum ImportError {
    NoColorBlocksFound,
    InvalidOklch { var: String, raw: String },
    OutOfSrgbGamut { var: String, srgb: ColorU },
    Io(String),
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
    let mut blocks = ParsedBlocks::default();

    // Strip CSS comments first; capture the first inline comment as a name hint.
    let mut name_hint = None;
    let mut cleaned = String::with_capacity(css.len());
    let mut i = 0;
    while i < css.len() {
        let rest = &css[i..];
        if rest.starts_with("/*") {
            // Find closing */
            let start = i + 2;
            let end = css[start..]
                .find("*/")
                .map(|j| start + j)
                .unwrap_or(css.len());
            let comment = css[start..end].trim();
            if name_hint.is_none() {
                // Look for "tweakcn theme: <slug>" or just take the comment if it's a single word.
                if let Some(rest) = comment.strip_prefix("tweakcn theme:") {
                    name_hint = Some(rest.trim().to_string());
                } else if !comment.contains(' ') && !comment.is_empty() {
                    name_hint = Some(comment.to_string());
                }
            }
            i = if end < css.len() { end + 2 } else { css.len() };
        } else {
            let ch = rest
                .chars()
                .next()
                .expect("non-empty string slice has a char");
            cleaned.push(ch);
            i += ch.len_utf8();
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

    let parse_decls = |body: &str,
                       target: &mut std::collections::HashMap<String, (f64, f64, f64)>|
     -> Result<(), ImportError> {
        for decl in body.split(';') {
            let decl = decl.trim();
            if !decl.starts_with("--") {
                continue;
            }
            let Some((name, value)) = decl.split_once(':') else {
                continue;
            };
            let name = name.trim().trim_start_matches("--").to_string();
            let value = value.trim();
            // Only `oklch(L C H[ / a])` is supported; anything else is silently skipped.
            let Some(args) = value
                .strip_prefix("oklch(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                continue;
            };
            let triple: Vec<&str> = args.split_whitespace().take(3).collect();
            if triple.len() < 3 {
                return Err(ImportError::InvalidOklch {
                    var: name,
                    raw: value.to_string(),
                });
            }
            let l: f64 = triple[0].trim_end_matches('%').parse().unwrap_or(f64::NAN);
            // tweakcn emits L as 0..1 (no `%`), but tolerate `%` style:
            let l = if triple[0].ends_with('%') {
                l / 100.0
            } else {
                l
            };
            let c: f64 = triple[1].parse().unwrap_or(f64::NAN);
            let h: f64 = triple[2]
                .trim_end_matches("deg")
                .parse()
                .unwrap_or(f64::NAN);
            if l.is_finite() && c.is_finite() && h.is_finite() {
                target.insert(name, (l, c, h));
            } else {
                return Err(ImportError::InvalidOklch {
                    var: name,
                    raw: value.to_string(),
                });
            }
        }
        Ok(())
    };

    if let Some(body) = extract_block(&cleaned, ":root") {
        parse_decls(body, &mut blocks.light)?;
    }
    if let Some(body) = extract_block(&cleaned, ".dark") {
        parse_decls(body, &mut blocks.dark)?;
    }

    if blocks.light.is_empty() && blocks.dark.is_empty() {
        return Err(ImportError::NoColorBlocksFound);
    }
    Ok(blocks)
}

// ─── Mapper: ParsedBlocks → WarpTheme ──────────────────────────────────────

use warp_core::ui::theme::{Fill, UiTokens, WarpTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamutPolicy {
    Clamp,
    Strict,
}

fn convert(triple: (f64, f64, f64), var: &str, policy: GamutPolicy) -> Result<ColorU, ImportError> {
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

/// Map a [`ParsedBlocks`] (from [`parse_blocks`]) into a complete [`WarpTheme`].
///
/// `inherit_terminal_from` is used as the source of terminal colors (ANSI
/// palette), background/foreground/accent fallbacks when the CSS block omits
/// them, and as the structural template.  `mode` selects which block
/// (`:root` → Light, `.dark` → Dark) to read from.
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

    let lookup = |key: &str| -> Result<Option<ColorU>, ImportError> {
        match block.get(key) {
            Some(&t) => convert(t, key, policy).map(Some),
            None => Ok(None),
        }
    };

    let background: Fill = match lookup("background")? {
        Some(c) => Fill::Solid(c),
        None => inherit_terminal_from.background(),
    };
    let foreground: ColorU =
        lookup("foreground")?.unwrap_or_else(|| inherit_terminal_from.foreground_color());
    let accent: Fill = match lookup("primary")? {
        Some(c) => Fill::Solid(c),
        None => inherit_terminal_from.accent(),
    };

    let mut ui = UiTokens::default();
    for &(css_name, setter) in tweakcn_ui_mapping() {
        if let Some(&triple) = block.get(css_name) {
            let c = convert(triple, css_name, policy)?;
            setter(&mut ui, c);
        }
    }

    let now = {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| format!("ts-{}", d.as_secs()))
            .unwrap_or_else(|_| "ts-unknown".to_string())
    };

    let theme = WarpTheme::new(
        background,
        foreground,
        accent,
        None,
        None,
        inherit_terminal_from.terminal_colors().clone(),
        None,
        blocks.name_comment.clone(),
    );
    Ok(theme.with_ui(ui, "tweakcn", now))
}

use std::path::{Path, PathBuf};

/// Write a tweakcn-imported theme to disk.
///
/// `slug` is sanitized into a filesystem-safe name.  The dark block (if any)
/// is written to `<themes_dir>/<slug>.yaml`; the light block (if any) to
/// `<themes_dir>/<slug>-light.yaml`.  Returns the list of paths actually
/// written.
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
        let yaml =
            serde_yaml::to_string(&theme).map_err(|e| io_to_import(std::io::Error::other(e)))?;
        std::fs::write(&primary_path, yaml).map_err(io_to_import)?;
        written.push(primary_path);
    }
    if !blocks.light.is_empty() {
        let theme = to_warp_theme(blocks, ThemeMode::Light, inherit_terminal_from, policy)?;
        let yaml =
            serde_yaml::to_string(&theme).map_err(|e| io_to_import(std::io::Error::other(e)))?;
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
            if !prev_dash {
                out.push(c);
            }
            prev_dash = true;
        } else {
            out.push(c);
            prev_dash = false;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "imported-theme".to_string()
    } else {
        out
    }
}

fn io_to_import(e: std::io::Error) -> ImportError {
    ImportError::Io(e.to_string())
}

fn tweakcn_ui_mapping() -> &'static [(&'static str, fn(&mut UiTokens, ColorU))] {
    &[
        ("card", |u, c| u.card = Some(c)),
        ("card-foreground", |u, c| u.card_foreground = Some(c)),
        ("popover", |u, c| u.popover = Some(c)),
        ("popover-foreground", |u, c| u.popover_foreground = Some(c)),
        ("primary", |u, c| u.primary = Some(c)),
        ("primary-foreground", |u, c| u.primary_foreground = Some(c)),
        ("secondary", |u, c| u.secondary = Some(c)),
        ("secondary-foreground", |u, c| {
            u.secondary_foreground = Some(c)
        }),
        ("muted", |u, c| u.muted = Some(c)),
        ("muted-foreground", |u, c| u.muted_foreground = Some(c)),
        ("destructive", |u, c| u.destructive = Some(c)),
        ("border", |u, c| u.border = Some(c)),
        ("input", |u, c| u.input = Some(c)),
        ("ring", |u, c| u.ring = Some(c)),
        ("sidebar", |u, c| u.sidebar = Some(c)),
        ("sidebar-foreground", |u, c| u.sidebar_foreground = Some(c)),
    ]
}

// ─── map_tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod map_tests {
    use super::*;

    fn inherit_from() -> WarpTheme {
        warp_core::ui::theme::mock_warp_theme()
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
        let ui = theme.ui().expect("ui block set");
        assert!(ui.card.is_some());
        assert!(ui.card_foreground.is_some());
        assert!(ui.muted_foreground.is_some());
        assert!(ui.sidebar.is_some());
        // Provenance written
        assert_eq!(theme.source(), Some("tweakcn"));
        // Terminal colors inherited
        assert_eq!(theme.terminal_colors(), base.terminal_colors());
    }

    #[test]
    fn light_mode_uses_root_block() {
        let css = ":root { --background: oklch(1 0 0); --card: oklch(0.95 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = inherit_from();
        let theme = to_warp_theme(&blocks, ThemeMode::Light, &base, GamutPolicy::Clamp).unwrap();
        assert!(theme.ui().unwrap().card.is_some());
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
    /// → ~#0a0a0a (allow ±1 per channel for rounding).
    ///
    /// Note: the Ottosson formula gives L=0.145 as linear ≈0.00305 →
    /// sRGB ≈10/255 (#0a). The plan spec comment of "#252525" was based
    /// on a different L scale; the math and CSS Color 4 spec agree on #0a0a0a.
    #[test]
    fn tweakcn_default_dark_background() {
        let c = oklch_to_srgb_u8(0.145, 0.0, 0.0).unwrap();
        let dr = (c.r as i32 - 0x0a).abs();
        let dg = (c.g as i32 - 0x0a).abs();
        let db = (c.b as i32 - 0x0a).abs();
        assert!(
            dr <= 1 && dg <= 1 && db <= 1,
            "got #{:02x}{:02x}{:02x}",
            c.r,
            c.g,
            c.b
        );
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
    fn non_ascii_css_is_preserved_while_stripping_comments() {
        let css = r#"
/* tweakcn theme: café */
:root {
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
}
"#;
        let blocks = parse_blocks(css).unwrap();
        assert_eq!(blocks.name_comment.as_deref(), Some("café"));
        assert_eq!(blocks.light.len(), 2);
    }

    #[test]
    fn invalid_oklch_declaration_reports_the_variable() {
        let css = ":root { --background: oklch(not-a-number 0 0); }";
        let result = parse_blocks(css);
        assert!(matches!(
            result,
            Err(ImportError::InvalidOklch { var, .. }) if var == "background"
        ));
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

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    #[ignore] // Run with `cargo test -- --ignored regenerate_golden` to refresh fixtures.
    fn regenerate_golden() {
        // include_str! is relative to the source file's location: app/src/themes/
        let css = include_str!("../../../crates/integration/tests/data/tweakcn_sample.css");
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::mock_warp_theme();
        let dark = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Clamp).unwrap();
        let light = to_warp_theme(&blocks, ThemeMode::Light, &base, GamutPolicy::Clamp).unwrap();
        // Note: path is relative to the crate root (app/), so go up to workspace root first.
        std::fs::write(
            "../crates/integration/tests/data/tweakcn_sample_dark.yaml",
            serde_yaml::to_string(&dark).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "../crates/integration/tests/data/tweakcn_sample_light.yaml",
            serde_yaml::to_string(&light).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn dark_block_matches_golden() {
        let css = include_str!("../../../crates/integration/tests/data/tweakcn_sample.css");
        let golden =
            include_str!("../../../crates/integration/tests/data/tweakcn_sample_dark.yaml");
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::mock_warp_theme();
        let theme = to_warp_theme(&blocks, ThemeMode::Dark, &base, GamutPolicy::Clamp).unwrap();
        let actual = serde_yaml::to_string(&theme).unwrap();
        // Strip the `source_imported_at` line because it embeds wall-clock time.
        let strip = |s: &str| {
            s.lines()
                .filter(|l| !l.contains("source_imported_at"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(strip(&actual), strip(golden));
    }

    #[test]
    fn light_block_matches_golden() {
        let css = include_str!("../../../crates/integration/tests/data/tweakcn_sample.css");
        let golden =
            include_str!("../../../crates/integration/tests/data/tweakcn_sample_light.yaml");
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::mock_warp_theme();
        let theme = to_warp_theme(&blocks, ThemeMode::Light, &base, GamutPolicy::Clamp).unwrap();
        let actual = serde_yaml::to_string(&theme).unwrap();
        let strip = |s: &str| {
            s.lines()
                .filter(|l| !l.contains("source_imported_at"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(strip(&actual), strip(golden));
    }
}

#[cfg(test)]
mod writer_tests {
    use super::*;

    #[test]
    fn writes_dark_only_when_no_light_block() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        let css = ".dark { --background: oklch(0.145 0 0); --card: oklch(0.2 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::mock_warp_theme();
        let written = write_imported(&blocks, "my-theme", &base, GamutPolicy::Clamp, &dir).unwrap();
        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with("my-theme.yaml"));
        assert!(std::fs::read_to_string(&written[0])
            .unwrap()
            .contains("ui:"));
    }

    #[test]
    fn writes_both_when_both_blocks_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        let css = ":root { --background: oklch(1 0 0); } .dark { --background: oklch(0 0 0); }";
        let blocks = parse_blocks(css).unwrap();
        let base = warp_core::ui::theme::mock_warp_theme();
        let written = write_imported(&blocks, "duo", &base, GamutPolicy::Clamp, &dir).unwrap();
        let names: Vec<String> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"duo.yaml".to_string()));
        assert!(names.contains(&"duo-light.yaml".to_string()));
    }
}
