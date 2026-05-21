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
