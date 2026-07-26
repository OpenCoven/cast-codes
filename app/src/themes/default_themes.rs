//! Bundled themes shipped with CastCodes.
//!
//! The set is deliberately small: the brand default plus a neutral pair and the
//! three most-used community palettes. Decorative background-image, gradient,
//! and referral-reward themes were retired — they conflict with the Phase 1
//! design contract (calm, dense, editor-grade, no decorative chrome), and the
//! referral themes could never unlock in an OSS build. Retired
//! [`super::theme::ThemeKind`] variants still deserialize and are remapped to
//! the nearest surviving theme, so saved settings keep working.
use pathfinder_color::ColorU;
use warp_core::ui::{
    color::OPAQUE,
    theme::{AnsiColor, AnsiColors, Details, Fill, TerminalColors, UiTokens, WarpTheme},
};

const DARK_MODE_NORMAL_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x616161FF),
    AnsiColor::from_u32(0xFF8272FF),
    AnsiColor::from_u32(0xB4FA72FF),
    AnsiColor::from_u32(0xFEFDC2FF),
    AnsiColor::from_u32(0xA5D5FEFF),
    AnsiColor::from_u32(0xFF8FFDFF),
    AnsiColor::from_u32(0xD0D1FEFF),
    AnsiColor::from_u32(0xF1F1F1FF),
);
const DARK_MODE_BRIGHT_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x8E8E8EFF),
    AnsiColor::from_u32(0xFFC4BDFF),
    AnsiColor::from_u32(0xD6FCB9FF),
    AnsiColor::from_u32(0xFEFDD5FF),
    AnsiColor::from_u32(0xC1E3FEFF),
    AnsiColor::from_u32(0xFFB1FEFF),
    AnsiColor::from_u32(0xE5E6FEFF),
    AnsiColor::from_u32(0xFEFFFFFF),
);

const CASTCODES_NORMAL_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x5A5A65FF),
    AnsiColor::from_u32(0xEF4444FF),
    AnsiColor::from_u32(0x22C55EFF),
    AnsiColor::from_u32(0xD4A84BFF),
    AnsiColor::from_u32(0x8E8E9AFF),
    AnsiColor::from_u32(0x7C3AEDFF),
    AnsiColor::from_u32(0xA78BFAFF),
    AnsiColor::from_u32(0xE8E8EDFF),
);
const CASTCODES_BRIGHT_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x8E8E9AFF),
    AnsiColor::from_u32(0xF87171FF),
    AnsiColor::from_u32(0x4ADE80FF),
    AnsiColor::from_u32(0xEBCB7AFF),
    AnsiColor::from_u32(0xB8B8C4FF),
    AnsiColor::from_u32(0xA78BFAFF),
    AnsiColor::from_u32(0xC4B5FDFF),
    AnsiColor::from_u32(0xFFFFFFFF),
);

const LIGHT_MODE_NORMAL_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x212121FF),
    AnsiColor::from_u32(0xC30771FF),
    AnsiColor::from_u32(0x10A778FF),
    AnsiColor::from_u32(0xA89C14FF),
    AnsiColor::from_u32(0x008EC4FF),
    AnsiColor::from_u32(0x523C79FF),
    AnsiColor::from_u32(0x20A5BAFF),
    AnsiColor::from_u32(0xE0E0E0FF),
);
const LIGHT_MODE_BRIGHT_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x212121FF),
    AnsiColor::from_u32(0xFB007AFF),
    AnsiColor::from_u32(0x5FD7AFFF),
    AnsiColor::from_u32(0xF3E430FF),
    AnsiColor::from_u32(0x20BBFCFF),
    AnsiColor::from_u32(0x6855DEFF),
    AnsiColor::from_u32(0x4FB8CCFF),
    AnsiColor::from_u32(0xF1F1F1FF),
);

const SOLARIZED_DARK_NORMAL_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x073642FF),
    AnsiColor::from_u32(0xDC322FFF),
    AnsiColor::from_u32(0x859900FF),
    AnsiColor::from_u32(0xB58900FF),
    AnsiColor::from_u32(0x268BD2FF),
    AnsiColor::from_u32(0xD33682FF),
    AnsiColor::from_u32(0x2AA198FF),
    AnsiColor::from_u32(0xEEE8D5FF),
);
const SOLARIZED_DARK_BRIGHT_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x002B36FF),
    AnsiColor::from_u32(0xCB4B16FF),
    AnsiColor::from_u32(0x586E75FF),
    AnsiColor::from_u32(0x657B83FF),
    AnsiColor::from_u32(0x839496FF),
    AnsiColor::from_u32(0x6C71C4FF),
    AnsiColor::from_u32(0x93A1A1FF),
    AnsiColor::from_u32(0xFDF6E3FF),
);

const DRACULA_NORMAL_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x000000FF),
    AnsiColor::from_u32(0xFF5555FF),
    AnsiColor::from_u32(0x50FA7BFF),
    AnsiColor::from_u32(0xF1FA8CFF),
    AnsiColor::from_u32(0xBD93F9FF),
    AnsiColor::from_u32(0xFF79C6FF),
    AnsiColor::from_u32(0x8BE9FDFF),
    AnsiColor::from_u32(0xBBBBBBFF),
);
const DRACULA_BRIGHT_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x555555FF),
    AnsiColor::from_u32(0xFF5555FF),
    AnsiColor::from_u32(0x50FA7BFF),
    AnsiColor::from_u32(0xF1FA8CFF),
    AnsiColor::from_u32(0xCAA9FAFF),
    AnsiColor::from_u32(0xFF79C6FF),
    AnsiColor::from_u32(0x8BE9FDFF),
    AnsiColor::from_u32(0xFFFFFFFF),
);

const GRUVBOX_DARK_NORMAL_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x282828FF),
    AnsiColor::from_u32(0xCC241DFF),
    AnsiColor::from_u32(0x98971AFF),
    AnsiColor::from_u32(0xD79921FF),
    AnsiColor::from_u32(0x458588FF),
    AnsiColor::from_u32(0xB16286FF),
    AnsiColor::from_u32(0x689D6AFF),
    AnsiColor::from_u32(0xA89984FF),
);
const GRUVBOX_DARK_BRIGHT_COLORS: AnsiColors = AnsiColors::new(
    AnsiColor::from_u32(0x928374FF),
    AnsiColor::from_u32(0xFB4934FF),
    AnsiColor::from_u32(0xB8BB26FF),
    AnsiColor::from_u32(0xFABD2FFF),
    AnsiColor::from_u32(0x83A598FF),
    AnsiColor::from_u32(0xD3869BFF),
    AnsiColor::from_u32(0x8EC07CFF),
    AnsiColor::from_u32(0xEBDBB2FF),
);

pub(super) fn light_mode_colors() -> TerminalColors {
    TerminalColors::new(LIGHT_MODE_NORMAL_COLORS, LIGHT_MODE_BRIGHT_COLORS)
}

pub(super) fn dark_mode_colors() -> TerminalColors {
    TerminalColors::new(DARK_MODE_NORMAL_COLORS, DARK_MODE_BRIGHT_COLORS)
}

pub(super) fn castcodes_terminal_colors() -> TerminalColors {
    TerminalColors::new(CASTCODES_NORMAL_COLORS, CASTCODES_BRIGHT_COLORS)
}

pub(super) fn solarized_dark_colors() -> TerminalColors {
    TerminalColors::new(SOLARIZED_DARK_NORMAL_COLORS, SOLARIZED_DARK_BRIGHT_COLORS)
}

pub(super) fn dracula_colors() -> TerminalColors {
    TerminalColors::new(DRACULA_NORMAL_COLORS, DRACULA_BRIGHT_COLORS)
}

pub(super) fn gruvbox_dark_colors() -> TerminalColors {
    TerminalColors::new(GRUVBOX_DARK_NORMAL_COLORS, GRUVBOX_DARK_BRIGHT_COLORS)
}

/// CastCodes dark theme — OpenCoven brand palette.
///
/// Brand slots mapped onto the available `WarpTheme` color slots:
/// - background: `#0f0f12`
/// - foreground (text primary): `#e8e8ed`
/// - accent (purple): `#7c3aed`
/// - terminal ANSI: CastCodes-specific dark palette derived from the Phase 1
///   semantic tokens instead of the generic dark theme.
///
/// The remaining brand slots (surface `#161619`, elevated surface `#1e1e22`,
/// borders, text secondary `#8e8e9a`, text muted `#5a5a65`, gold `#d4a84b`,
/// chrome `#0a0a0d`) are carried by the semantic `UiTokens` block below and
/// surfaced through the fallback-aware accessors in
/// `warp_core::ui::theme::color`; see `DESIGN-CHANGES.md`.
///
/// Border tokens are stored pre-blended to opaque over the `#0f0f12`
/// background (theme hex serialization is RGB-only): 8% white → `#222225`,
/// 12% → `#2c2c2e`, 4% → `#19191b`.
pub fn castcodes_dark() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x0F0F12FF)),
        ColorU::from_u32(0xE8E8EDFF),
        Fill::Solid(ColorU::from_u32(0x7C3AEDFF)),
        None,
        Some(Details::Darker),
        castcodes_terminal_colors(),
        None,
        Some("CastCodes Dark".to_string()),
    )
    .with_ui_tokens(castcodes_ui_tokens())
}

/// Semantic UI tokens for `castcodes_dark`, mirroring
/// `resources/design-tokens.css`.
pub(super) fn castcodes_ui_tokens() -> UiTokens {
    UiTokens {
        card: Some(ColorU::from_u32(0x161619FF)),
        card_foreground: Some(ColorU::from_u32(0xE8E8EDFF)),
        popover: Some(ColorU::from_u32(0x1E1E22FF)),
        popover_foreground: Some(ColorU::from_u32(0xE8E8EDFF)),
        primary: Some(ColorU::from_u32(0x7C3AEDFF)),
        primary_foreground: Some(ColorU::from_u32(0xFFFFFFFF)),
        secondary: Some(ColorU::from_u32(0x161619FF)),
        secondary_foreground: Some(ColorU::from_u32(0x8E8E9AFF)),
        muted: Some(ColorU::from_u32(0x161619FF)),
        muted_foreground: Some(ColorU::from_u32(0x5A5A65FF)),
        destructive: Some(ColorU::from_u32(0xEF4444FF)),
        border: Some(ColorU::from_u32(0x222225FF)),
        input: Some(ColorU::from_u32(0x222225FF)),
        ring: Some(ColorU::from_u32(0x7C3AEDFF)),
        sidebar: Some(ColorU::from_u32(0x161619FF)),
        sidebar_foreground: Some(ColorU::from_u32(0xE8E8EDFF)),
        chrome: Some(ColorU::from_u32(0x0A0A0DFF)),
        border_strong: Some(ColorU::from_u32(0x2C2C2EFF)),
        border_subtle: Some(ColorU::from_u32(0x19191BFF)),
        primary_hover: Some(ColorU::from_u32(0x6D28D9FF)),
        highlight: Some(ColorU::from_u32(0xD4A84BFF)),
    }
}

/// Default bundled themes
pub fn dark_theme() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x000000FF)),
        ColorU::from_u32(0xffffffff),
        Fill::Solid(ColorU::from_u32(0x19AAD8FF)),
        None,
        Some(Details::Darker),
        dark_mode_colors(),
        None,
        Some("Dark".to_string()),
    )
}

pub fn light_theme() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::white()),
        ColorU::new(17, 17, 17, OPAQUE),
        Fill::Solid(ColorU::from_u32(0x00c2ffff)),
        None,
        Some(Details::Lighter),
        light_mode_colors(),
        None,
        Some("Light".to_string()),
    )
}

pub(super) fn dracula() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x282A36FF)),
        ColorU::from_u32(0xF8F8F2FF),
        Fill::Solid(ColorU::from_u32(0xFF79C6FF)),
        None,
        Some(Details::Darker),
        dracula_colors(),
        None,
        Some("Dracula".to_string()),
    )
}

pub(super) fn solarized_dark() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x002B36FF)),
        ColorU::from_u32(0xF8F8F2FF),
        Fill::Solid(ColorU::from_u32(0xCB4B16FF)),
        None,
        Some(Details::Darker),
        solarized_dark_colors(),
        None,
        Some("Solarized Dark".to_string()),
    )
}

pub(super) fn gruvbox_dark() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x282828FF)),
        ColorU::from_u32(0xEBDBB2FF),
        Fill::Solid(ColorU::from_u32(0xFC802DFF)),
        None,
        Some(Details::Darker),
        gruvbox_dark_colors(),
        None,
        Some("Gruvbox Dark".to_string()),
    )
}
