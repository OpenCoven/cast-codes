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

/// Backward-compat pixel parity for all 24 built-in themes.
///
/// Every built-in theme must:
/// 1. Carry no `ui` block (tasks 1–6 guarantee `WarpTheme::new` sets `ui = None`).
/// 2. Return the same derived values from `surface_2()`, `outline()`, and
///    `active_ui_text_color()` as they would without the override path — i.e.
///    the shims must be transparent when `ui` is absent.
///
/// If any of these assertions fire it means a built-in theme was accidentally
/// given a `ui` block, or one of the accessor fallback paths drifted.
#[test]
fn builtin_themes_render_identically_without_ui_block() {
    let builtins: Vec<WarpTheme> = vec![
        castcodes_dark(),
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
        24,
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
