use super::*;

/// Minimal `WarpTheme` with no `ui` block — used by accessor tests that
/// need a theme to exercise fallback paths without overriding behavior.
fn test_theme_without_ui() -> WarpTheme {
    WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x090300ff)), // background
        ColorU::from_u32(0xa5a2a2ff),              // foreground
        Fill::Solid(ColorU::from_u32(0x01a0e4ff)), // accent
        None,                                      // cursor
        Some(Details::Darker),                     // details
        mock_terminal_colors(),                    // terminal colors
        None,                                      // background_image
        None,                                      // name
    )
}

#[test]
fn muted_foreground_falls_back_to_opencoven_muted() {
    let theme = test_theme_without_ui();
    // OPENCOVEN_MUTED is pub(crate) in app crate, so re-derive its value:
    let expected = ColorU {
        r: 90,
        g: 90,
        b: 101,
        a: 255,
    };
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
    assert_eq!(
        theme.sidebar_bg(),
        Fill::Solid(ColorU::from_u32(0x0a0604ff))
    );
}

#[test]
fn ring_falls_back_to_accent() {
    let theme = test_theme_without_ui();
    assert_eq!(theme.ring(), theme.accent());
}

#[test]
fn ring_uses_ui_override_when_set() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        ring: Some(ColorU::from_u32(0x4ade80ff)),
        ..Default::default()
    });
    assert_eq!(theme.ring(), Fill::Solid(ColorU::from_u32(0x4ade80ff)));
}

#[test]
fn ui_sidebar_override_returns_none_without_ui() {
    let theme = test_theme_without_ui();
    assert!(theme.ui_sidebar_override().is_none());
}

#[test]
fn ui_sidebar_override_returns_some_when_ui_sidebar_set() {
    let mut theme = test_theme_without_ui();
    theme.ui = Some(UiTokens {
        sidebar: Some(ColorU::from_u32(0x0a0604ff)),
        ..Default::default()
    });
    assert_eq!(
        theme.ui_sidebar_override(),
        Some(Fill::Solid(ColorU::from_u32(0x0a0604ff)))
    );
}

#[test]
fn serialize_test() {
    let theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        ColorU::from_u32(0x20A5BAFF),
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        Some("test_theme".to_string()),
    );
    assert_eq!(
        r##"---
background: "#20a5ba"
accent: "#20a5ba"
foreground: "#20a5ba"
details: darker
terminal_colors:
  normal:
    black: "#616161"
    red: "#ff8272"
    green: "#b4fa72"
    yellow: "#fefdc2"
    blue: "#a5d5fe"
    magenta: "#ff8ffd"
    cyan: "#d0d1fe"
    white: "#f1f1f1"
  bright:
    black: "#8e8e8e"
    red: "#ffc4bd"
    green: "#d6fcb9"
    yellow: "#fefdd5"
    blue: "#c1e3fe"
    magenta: "#ffb1fe"
    cyan: "#e5e6fe"
    white: "#feffff"
name: test_theme
"##,
        serde_yaml::to_string(&theme).expect("Couldn't serialize")
    );
}

#[test]
fn deserialize_with_name_test() {
    let theme = serde_yaml::from_str::<WarpTheme>(
        r##"---
background: "#20a5ba"
accent: "#20a5ba"
foreground: "#20a5ba"
details: darker
terminal_colors:
  normal:
    black: "#616161"
    red: "#ff8272"
    green: "#b4fa72"
    yellow: "#fefdc2"
    blue: "#a5d5fe"
    magenta: "#ff8ffd"
    cyan: "#d0d1fe"
    white: "#f1f1f1"
  bright:
    black: "#8e8e8e"
    red: "#ffc4bd"
    green: "#d6fcb9"
    yellow: "#fefdd5"
    blue: "#c1e3fe"
    magenta: "#ffb1fe"
    cyan: "#e5e6fe"
    white: "#feffff"
name: test_theme
"##,
    )
    .expect("Couldn't deserialize");

    let expected_theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        ColorU::from_u32(0x20A5BAFF),
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        Some("test_theme".to_string()),
    );

    assert_eq!(expected_theme, theme);
}

#[test]
fn deserialize_without_name_test() {
    let theme = serde_yaml::from_str::<WarpTheme>(
        r##"---
background: "#20a5ba"
accent: "#20a5ba"
foreground: "#20a5ba"
details: darker
terminal_colors:
  normal:
    black: "#616161"
    red: "#ff8272"
    green: "#b4fa72"
    yellow: "#fefdc2"
    blue: "#a5d5fe"
    magenta: "#ff8ffd"
    cyan: "#d0d1fe"
    white: "#f1f1f1"
  bright:
    black: "#8e8e8e"
    red: "#ffc4bd"
    green: "#d6fcb9"
    yellow: "#fefdd5"
    blue: "#c1e3fe"
    magenta: "#ffb1fe"
    cyan: "#e5e6fe"
    white: "#feffff"
"##,
    )
    .expect("Couldn't deserialize");

    let expected_theme = WarpTheme::new(
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        ColorU::from_u32(0x20A5BAFF),
        Fill::Solid(ColorU::from_u32(0x20A5BAFF)),
        None,
        Some(Details::Darker),
        mock_terminal_colors(),
        None,
        None,
    );

    assert_eq!(expected_theme, theme);
}

#[test]
fn blend_gradient_test() {
    let (c1, c2, c3, c4) = (
        ColorU::from_u32(0x002b36ff),
        ColorU::from_u32(0xcb4b16ff),
        ColorU::from_u32(0xffffff19),
        ColorU::from_u32(0xffffff19),
    );
    let g1 = VerticalGradient::new(c1, c2);
    let g2 = VerticalGradient::new(c3, c4);

    assert_eq!(
        g1.blend(&g2),
        VerticalGradient::new(c1.blend(&c3), c2.blend(&c4))
    );
}

#[test]
fn blend_coloru_test() {
    let c1 = ColorU::from_u32(0x002b36ff);
    let c2 = ColorU::from_u32(0xF8F8F2FF);
    assert_eq!(
        c1.blend(&coloru_with_opacity(c2, 10)),
        ColorU::from_u32(0x183f48ff)
    );
    assert_eq!(
        ColorU::from_u32(0x000000ff).blend(&coloru_with_opacity(c2, 10)),
        ColorU::from_u32(0x181818ff)
    );
}

/// TODO(CORE-3626): write an equivalent test with Windows paths.
#[cfg(not(windows))]
#[test]
fn test_deserialize_image() {
    // Paths that start with `~` should expand to include the home dir.
    let a = "
    path: ~/warp.jpg
    opacity: 60
    ";
    let image: Image = serde_yaml::from_str(a).unwrap();
    assert_eq!(image.opacity, 60);
    assert_eq!(
        image.source,
        AssetSource::LocalFile {
            path: home_dir()
                .unwrap()
                .join("warp.jpg")
                .to_str()
                .unwrap_or_default()
                .to_owned()
        }
    );

    // Absolute paths should be unchanged.
    let b = "
    path: /warp.jpg
    opacity: 60
    ";
    let image: Image = serde_yaml::from_str(b).unwrap();
    assert_eq!(image.opacity, 60);
    assert_eq!(
        image.source,
        AssetSource::LocalFile {
            path: "/warp.jpg".to_owned()
        }
    );

    // Relative paths should expand to include the theme dir.
    let c = "
    path: warp.jpg
    opacity: 60
    ";
    let image: Image = serde_yaml::from_str(c).unwrap();
    assert_eq!(image.opacity, 60);
    assert_eq!(
        image.source,
        AssetSource::LocalFile {
            path: themes_dir()
                .join("warp.jpg")
                .to_str()
                .unwrap_or_default()
                .to_owned()
        }
    );

    // No opacity should become the default
    let d = "
    path: warp.jpg
    ";
    let image: Image = serde_yaml::from_str(d).unwrap();
    assert_eq!(image.opacity, default_image_opacity());
}

#[test]
fn ansi_color_deserializing_test() {
    let raw = r##"
        black: "#000000"
        red: "#ff0000"
        green: "#00ff00"
        yellow: "#00ffff"
        blue: "#0000ff"
        magenta: "#ff0000"
        cyan: "#0000ff"
        white: "#ffffff"
        "##;
    let ansi_colors: AnsiColors = serde_yaml::from_str(raw).expect("Couldn't deserialize");
    assert_eq!(ansi_colors.black, AnsiColor::from_u32(0x000000ff));
    assert_eq!(ansi_colors.red, AnsiColor::from_u32(0xff0000ff));
    assert_eq!(ansi_colors.green, AnsiColor::from_u32(0x00ff00ff));
    assert_eq!(ansi_colors.yellow, AnsiColor::from_u32(0x00ffffff));
    assert_eq!(ansi_colors.blue, AnsiColor::from_u32(0x0000ffff));
    assert_eq!(ansi_colors.magenta, AnsiColor::from_u32(0xff0000ff));
    assert_eq!(ansi_colors.cyan, AnsiColor::from_u32(0x0000ffff));
    assert_eq!(ansi_colors.white, AnsiColor::from_u32(0xffffffff));
}

#[test]
fn ansi_color_serializing_test() {
    let ansi_colors = AnsiColors::new(
        AnsiColor::from_u32(0x000000ff),
        AnsiColor::from_u32(0xff0000ff),
        AnsiColor::from_u32(0x00ff00ff),
        AnsiColor::from_u32(0x00ffffff),
        AnsiColor::from_u32(0x0000ffff),
        AnsiColor::from_u32(0xff0000ff),
        AnsiColor::from_u32(0x0000ffff),
        AnsiColor::from_u32(0xffffffff),
    );
    let serialized = serde_yaml::to_string(&ansi_colors).expect("Couldn't serialize");
    let raw = r##"---
black: "#000000"
red: "#ff0000"
green: "#00ff00"
yellow: "#00ffff"
blue: "#0000ff"
magenta: "#ff0000"
cyan: "#0000ff"
white: "#ffffff"
"##;
    assert_eq!(serialized, raw);

    let ansi_colors2: AnsiColors = serde_yaml::from_str(&serialized).expect("Couldn't deserialize");
    assert_eq!(ansi_colors2, ansi_colors);
}

#[test]
fn from_hex_negative_test() {
    assert_eq!(
        hex_color::coloru_from_hex_string("#0").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#00").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#00000").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#0000000").unwrap_err(),
        hex_color::HexColorError::InvalidLength
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("0000").unwrap_err(),
        hex_color::HexColorError::HashPrefix
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#ZXD").unwrap_err(),
        hex_color::HexColorError::InvalidValue
    );
}

#[test]
fn from_hex_positive_test() {
    assert_eq!(
        hex_color::coloru_from_hex_string("#000").unwrap(),
        ColorU::from_u32(0x000000ff)
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#000000").unwrap(),
        ColorU::from_u32(0x000000ff)
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#123").unwrap(),
        ColorU::from_u32(0x112233ff)
    );
    assert_eq!(
        hex_color::coloru_from_hex_string("#112233").unwrap(),
        ColorU::from_u32(0x112233ff)
    );
}

#[test]
fn infer_from_foreground_color_test() {
    assert_eq!(
        ColorScheme::infer_from_foreground_color(ColorU::white()),
        ColorScheme::LightOnDark
    );
    assert_eq!(
        ColorScheme::infer_from_foreground_color(ColorU::black()),
        ColorScheme::DarkOnLight
    );
}

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
    assert_eq!(
        tokens.muted_foreground.unwrap(),
        ColorU::from_u32(0x5a5a65ff)
    );
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

#[test]
fn ui_tokens_accepts_3_char_hex() {
    let yaml = "card: '#fff'\n";
    let tokens: UiTokens = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(tokens.card.unwrap(), ColorU::from_u32(0xffffffff));
}

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
    assert_eq!(
        theme.active_ui_text_color(),
        Fill::Solid(ColorU::from_u32(0xe8e6e3ff))
    );
}

#[test]
fn outline_unchanged_without_ui() {
    let theme = test_theme_without_ui();
    let derived = color::internal_colors::fg_overlay_2(&theme);
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

#[test]
fn surface_2_unchanged_without_ui() {
    let theme = test_theme_without_ui();
    let derived = Fill::Solid(color::internal_colors::neutral_2(&theme));
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
