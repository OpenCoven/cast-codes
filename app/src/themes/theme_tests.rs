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
            TerminalColors {
                normal: AnsiColors {
                    black: AnsiColor::from_u32(0x5A5A65FF),
                    red: AnsiColor::from_u32(0xEF4444FF),
                    green: AnsiColor::from_u32(0x22C55EFF),
                    yellow: AnsiColor::from_u32(0xD4A84BFF),
                    blue: AnsiColor::from_u32(0x8E8E9AFF),
                    magenta: AnsiColor::from_u32(0x7C3AEDFF),
                    cyan: AnsiColor::from_u32(0xA78BFAFF),
                    white: AnsiColor::from_u32(0xE8E8EDFF),
                },
                bright: AnsiColors {
                    black: AnsiColor::from_u32(0x8E8E9AFF),
                    red: AnsiColor::from_u32(0xF87171FF),
                    green: AnsiColor::from_u32(0x4ADE80FF),
                    yellow: AnsiColor::from_u32(0xEBCB7AFF),
                    blue: AnsiColor::from_u32(0xB8B8C4FF),
                    magenta: AnsiColor::from_u32(0xA78BFAFF),
                    cyan: AnsiColor::from_u32(0xC4B5FDFF),
                    white: AnsiColor::from_u32(0xFFFFFFFF),
                },
            },
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
