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
