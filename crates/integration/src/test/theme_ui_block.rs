//! Integration test: a theme YAML that contains a `ui:` block drives the
//! colour accessors that the chat panel reads.
//!
//! The test writes a sentinel theme (card = #ff00ff, muted_foreground =
//! #ffff00) to the hermetic themes directory, activates it via the
//! user-preferences JSON file, boots the app, and then asserts that
//! `Appearance::theme()` returns the exact sentinel colours from
//! `surface_2()` and `muted_foreground()`.
//!
//! This is an end-to-end plumbing test exercising the full path:
//!   YAML on disk → WarpTheme deserialisation → UiTokens accessors
//!
//! The test is marked `#[ignore]` because activating a custom theme at
//! startup only works on the real-display path.  Run it manually with:
//!
//! ```bash
//! WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS=1 \
//!   cargo run -p integration --bin integration -- \
//!   test_ui_block_theme_drives_colour_accessors
//! ```

use warp::appearance::Appearance;
use warp::integration_testing::create_file_from_assets;
use warp::integration_testing::terminal::wait_until_bootstrapped_single_pane_for_tab;
use warp::integration_testing::themes::themes_dir;
use warp::integration_testing::view_getters::workspace_view;
use warp_core::ui::theme::Fill;
use warpui::{async_assert_eq, color::ColorU, integration::TestStep, SingletonEntity};

use super::{new_builder, Builder, TEST_ONLY_ASSETS};

/// Name of the fixture YAML embedded in `tests/data/`.
const THEME_FILE_NAME: &str = "ui_block_theme.yaml";

/// Sentinel `ui.card` value from the fixture — must match the YAML.
const SENTINEL_CARD: u32 = 0xff00ffff;

/// Sentinel `ui.muted_foreground` value from the fixture — must match the YAML.
const SENTINEL_MUTED_FG: u32 = 0xffff00ff;

/// End-to-end test: a YAML theme that contains a `ui:` block with sentinel
/// colours is loaded at startup and the `Appearance` singleton exposes those
/// exact colours through `surface_2()` and `muted_foreground()`.
///
/// # Activation mechanism
///
/// The user-preferences JSON written in `with_setup` sets `"Theme"` to a
/// serialised `ThemeKind::Custom { name, path }` that points to the YAML we
/// wrote into `themes_dir()`.  Both paths are resolved inside `with_setup`,
/// which runs after `$HOME` has been redirected to the hermetic test sandbox.
///
/// # Display requirement
///
/// Marked `#[ignore]` because the `Appearance` singleton is only fully
/// initialised on the real-display path.  See module-level docs above.
pub fn test_ui_block_theme_drives_colour_accessors() -> Builder {
    new_builder()
        .with_setup(|_utils| {
            // 1. Write the sentinel theme YAML into the hermetic themes dir.
            //    HOME is already redirected to the test sandbox at this point.
            let dest_dir = themes_dir();
            std::fs::create_dir_all(&dest_dir).expect("should create hermetic themes dir");

            let dest_path = dest_dir.join(THEME_FILE_NAME);
            create_file_from_assets(TEST_ONLY_ASSETS, THEME_FILE_NAME, &dest_path);

            // 2. Build a user-preferences JSON that activates the theme.
            //
            //    The storage key for `theme_kind` in `ThemeSettings` is
            //    "Theme" (derived by `stringify!` from the setting identifier
            //    in `define_settings_group!`; no explicit `storage_key:`
            //    override is present in the macro invocation).
            //
            //    `ThemeKind::Custom` serialises (via serde) as:
            //      {"Custom":{"name":"<display-name>","path":"<abs-path>"}}
            //
            //    The outer user-preferences file stores each value as a
            //    JSON-encoded string, so the inner JSON is a plain string.
            let theme_kind_json = serde_json::json!({
                "Custom": {
                    "name": "ui-block-sentinel",
                    "path": dest_path,
                }
            })
            .to_string();

            let prefs_path = warp::settings::user_preferences_file_path();
            if let Some(parent) = prefs_path.parent() {
                std::fs::create_dir_all(parent).expect("should create config local dir");
            }
            // The user-preferences JSON maps storage_key → JSON-encoded value.
            let prefs_json = serde_json::json!({ "Theme": theme_kind_json }).to_string();
            std::fs::write(&prefs_path, prefs_json).expect("should write user preferences");
        })
        // Wait for the app to boot and the terminal to reach a stable state.
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        // Assert that the Appearance singleton loaded our sentinel theme.
        .with_step(
            TestStep::new("ui: block sentinel colours reach Appearance")
                .add_named_assertion("surface_2() == Fill::Solid(#ff00ff)", |app, window_id| {
                    // Access Appearance through any live ViewContext.
                    // ViewContext<T> derefs to AppContext, so
                    // Appearance::as_ref(ctx) resolves correctly.
                    workspace_view(app, window_id).read(app, |_view, ctx| {
                        let theme = Appearance::as_ref(ctx).theme();
                        async_assert_eq!(
                            theme.surface_2(),
                            Fill::Solid(ColorU::from_u32(SENTINEL_CARD)),
                            "surface_2() should equal sentinel card colour #ff00ff"
                        )
                    })
                })
                .add_named_assertion("muted_foreground() == #ffff00", |app, window_id| {
                    workspace_view(app, window_id).read(app, |_view, ctx| {
                        let theme = Appearance::as_ref(ctx).theme();
                        async_assert_eq!(
                            theme.muted_foreground(),
                            ColorU::from_u32(SENTINEL_MUTED_FG),
                            "muted_foreground() should equal sentinel colour #ffff00"
                        )
                    })
                }),
        )
}
