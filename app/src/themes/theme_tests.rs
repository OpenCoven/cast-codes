use super::*;
use crate::util::color::OPAQUE;

#[test]
fn castcodes_dark_theme_uses_phase_1_palette() {
    assert_eq!(
        castcodes_dark(),
        WarpTheme::new(
            ColorU::from_u32(0x0F0F12FF).into(),
            ColorU::from_u32(0xE8E8EDFF),
            ColorU::from_u32(0x7C3AEDFF).into(),
            None,
            Some(Details::Darker),
            castcodes_terminal_colors(),
            None,
            Some("CastCodes Dark".to_string()),
        )
        .with_ui_tokens(castcodes_ui_tokens())
    );
}

#[test]
fn system_dark_theme_defaults_to_castcodes_dark() {
    assert_eq!(
        SelectedSystemThemes::default().dark,
        ThemeKind::CastCodesDark
    );
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn in_memory_theme_generation_test() {
    let mountains_bg_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "assets",
        "async",
        "jpg",
        "mountains.jpg",
    ]
    .iter()
    .collect();

    let mut in_memory_theme = warpui::r#async::block_on(InMemoryThemeOptions::new(
        "mountains".to_string(),
        mountains_bg_path.clone(),
    ))
    .unwrap();

    let mountains_bg_path_string = mountains_bg_path.to_str().unwrap_or_default().to_owned();
    assert_eq!(
        in_memory_theme.theme(),
        WarpTheme::new(
            // the theme defaults to the 0th bg color
            ColorU::new(35, 31, 44, OPAQUE).into(),
            // this background color makes it a "dark" theme, so the foreground is white
            ColorU::white(),
            // the most distinct accent color is 3rd one
            ColorU::new(238, 203, 111, OPAQUE).into(),
            None,
            Some(Details::Darker),
            dark_mode_colors(),
            Some(Image {
                source: AssetSource::LocalFile {
                    path: mountains_bg_path_string.clone()
                },
                opacity: 30,
            }),
            Some("mountains".to_string()),
        )
    );

    in_memory_theme.chosen_bg_color_index = 2;

    assert_eq!(
        in_memory_theme.theme(),
        WarpTheme::new(
            // now the background is the 2nd one
            ColorU::new(229, 142, 113, OPAQUE).into(),
            // changing the background color made this a light theme
            ColorU::black(),
            // now the 4th color is the most distinct color
            ColorU::new(193, 217, 212, OPAQUE).into(),
            None,
            Some(Details::Lighter),
            light_mode_colors(),
            Some(Image {
                source: AssetSource::LocalFile {
                    path: mountains_bg_path_string
                },
                opacity: 30,
            }),
            Some("mountains".to_string()),
        )
    );
}

/// Backward-compat pixel parity for the 23 non-CastCodes built-in themes.
///
/// Every legacy built-in theme must:
/// 1. Carry no `ui` block (tasks 1–6 guarantee `WarpTheme::new` sets `ui = None`).
/// 2. Return the same derived values from `surface_2()`, `outline()`, and
///    `active_ui_text_color()` as they would without the override path — i.e.
///    the shims must be transparent when `ui` is absent.
///
/// `castcodes_dark()` is intentionally excluded: it carries the brand
/// `UiTokens` block (see `castcodes_dark_carries_brand_ui_tokens`).
///
/// If any of these assertions fire it means a built-in theme was accidentally
/// given a `ui` block, or one of the accessor fallback paths drifted.
#[test]
fn builtin_themes_render_identically_without_ui_block() {
    let builtins: Vec<WarpTheme> = vec![
        dark_theme(),
        light_theme(),
        dracula(),
        solarized_light(),
        solarized_dark(),
        gruvbox_dark(),
        gruvbox_light(),
        cyber_wave(),
        willow_dream(),
        fancy_dracula(),
        phenomenon(),
        jellyfish(),
        koi(),
        leafy(),
        marble(),
        pink_city(),
        snowy(),
        red_rock(),
        dark_city(),
        sent_referral_reward(),
        solar_flare(),
        adeberry(),
        received_referral_reward(),
    ];

    assert_eq!(
        builtins.len(),
        23,
        "update this test when adding/removing built-in themes"
    );

    for theme in &builtins {
        let name = theme.name();

        // Invariant 1: no ui block on any built-in.
        assert!(
            theme.ui().is_none(),
            "built-in {:?} unexpectedly carries a ui block",
            name
        );

        // Invariant 2: surface_2() returns the same value as the derived path.
        let derived_surface_2 = Fill::Solid(color::internal_colors::neutral_2(theme));
        assert_eq!(
            theme.surface_2(),
            derived_surface_2,
            "surface_2 drift for {:?}",
            name
        );

        // Invariant 3: outline() returns the same value as the derived path.
        let derived_outline = color::internal_colors::fg_overlay_2(theme);
        assert_eq!(
            theme.outline(),
            derived_outline,
            "outline drift for {:?}",
            name
        );

        // Invariant 4: active_ui_text_color() returns the same value as the derived path.
        let derived_text = theme.main_text_color(theme.surface_2());
        assert_eq!(
            theme.active_ui_text_color(),
            derived_text,
            "active_ui_text_color drift for {:?}",
            name
        );
    }
}

/// `castcodes_dark` must carry the full brand `UiTokens` block and surface it
/// through the fallback-aware accessors (Phase 1 design contract; values
/// mirror `resources/design-tokens.css`).
#[test]
fn castcodes_dark_carries_brand_ui_tokens() {
    let theme = castcodes_dark();
    let ui = theme.ui().expect("castcodes_dark carries a ui block");

    // No import provenance — this is a built-in, not a tweakcn import.
    assert_eq!(theme.source(), None);

    // Surfaces.
    assert_eq!(
        theme.surface_1(),
        Fill::Solid(ColorU::from_u32(0x161619FF)),
        "surface_1 should be the brand surface"
    );
    assert_eq!(
        theme.surface_2(),
        Fill::Solid(ColorU::from_u32(0x161619FF)),
        "surface_2 should be the brand surface (card)"
    );
    assert_eq!(
        theme.surface_3(),
        Fill::Solid(ColorU::from_u32(0x1E1E22FF)),
        "surface_3 should be the elevated surface (popover)"
    );
    assert_eq!(
        theme.chrome_bg_override(),
        Some(Fill::Solid(ColorU::from_u32(0x0A0A0DFF))),
        "title/status chrome"
    );
    assert_eq!(
        theme.sidebar_bg(),
        Fill::Solid(ColorU::from_u32(0x161619FF))
    );

    // Borders (pre-blended opaque over #0f0f12).
    assert_eq!(theme.outline(), Fill::Solid(ColorU::from_u32(0x222225FF)));
    assert_eq!(
        theme.border_strong(),
        Fill::Solid(ColorU::from_u32(0x2C2C2EFF))
    );
    assert_eq!(
        theme.border_subtle(),
        Fill::Solid(ColorU::from_u32(0x19191BFF))
    );

    // Text hierarchy.
    assert_eq!(
        theme.sub_text_color(theme.background()),
        Fill::Solid(ColorU::from_u32(0x8E8E9AFF)),
        "secondary text"
    );
    assert_eq!(
        theme.hint_text_color(theme.background()),
        Fill::Solid(ColorU::from_u32(0x5A5A65FF)),
        "muted text"
    );
    assert_eq!(theme.muted_foreground(), ColorU::from_u32(0x5A5A65FF));

    // Accents.
    assert_eq!(theme.ring(), Fill::Solid(ColorU::from_u32(0x7C3AEDFF)));
    assert_eq!(
        theme.accent_hover(),
        Fill::Solid(ColorU::from_u32(0x6D28D9FF))
    );
    assert_eq!(
        theme.accent_hover_override(),
        Some(Fill::Solid(ColorU::from_u32(0x6D28D9FF)))
    );
    assert_eq!(
        theme.highlight_override(),
        Some(ColorU::from_u32(0xD4A84BFF))
    );
    assert_eq!(ui.primary, Some(ColorU::from_u32(0x7C3AEDFF)));
    assert_eq!(ui.destructive, Some(ColorU::from_u32(0xEF4444FF)));
}

/// The new CastCodes token accessors must fall back to today's derived values
/// for themes without a `ui` block, so every legacy theme stays
/// pixel-identical.
#[test]
fn new_brand_accessors_fall_back_without_ui_block() {
    let theme = dark_theme();
    assert!(theme.ui().is_none());

    assert_eq!(theme.chrome_bg_override(), None);
    assert_eq!(theme.highlight_override(), None);
    assert_eq!(theme.accent_hover_override(), None);
    assert_eq!(
        theme.border_strong(),
        color::internal_colors::fg_overlay_3(&theme)
    );
    assert_eq!(
        theme.border_subtle(),
        color::internal_colors::fg_overlay_1(&theme)
    );
    assert_eq!(
        theme.surface_1(),
        Fill::Solid(color::internal_colors::neutral_1(&theme))
    );
    assert_eq!(
        theme.surface_3(),
        Fill::Solid(color::internal_colors::neutral_3(&theme))
    );
    assert_eq!(
        theme.accent_hover(),
        color::internal_colors::accent_hover(&theme)
    );
    let bg = theme.background();
    assert_eq!(
        theme.sub_text_color(bg),
        Fill::from(color::internal_colors::text_sub(&theme, bg)),
    );
}
