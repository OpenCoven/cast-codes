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
                normal: AnsiColor {
                    black: ColorU::from_u32(0x0F0F12FF),
                    red: ColorU::from_u32(0xF7768EFF),
                    green: ColorU::from_u32(0x9ECE6AFF),
                    yellow: ColorU::from_u32(0xE0AF68FF),
                    blue: ColorU::from_u32(0x7AA2F7FF),
                    magenta: ColorU::from_u32(0xBB9AFAFF),
                    cyan: ColorU::from_u32(0x7DCFFFFF),
                    white: ColorU::from_u32(0xA9B1D6FF),
                },
                bright: AnsiColor {
                    black: ColorU::from_u32(0x414868FF),
                    red: ColorU::from_u32(0xF7768EFF),
                    green: ColorU::from_u32(0x9ECE6AFF),
                    yellow: ColorU::from_u32(0xE0AF68FF),
                    blue: ColorU::from_u32(0x7AA2F7FF),
                    magenta: ColorU::from_u32(0xBB9AFAFF),
                    cyan: ColorU::from_u32(0x7DCFFFFF),
                    white: ColorU::from_u32(0xC0CAF5FF),
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
