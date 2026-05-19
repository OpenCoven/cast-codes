// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use anyhow::Result;
use warp_core::{
    brand,
    channel::{Channel, ChannelConfig, ChannelState, OzConfig, WarpServerConfig as ServerConfig},
    AppId,
};

// Simple wrapper around warp::run() for public CastCodes builds.
fn main() -> Result<()> {
    let (qualifier, organization, application) = brand::public_app_id_parts();
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new(qualifier, organization, application),
            logfile_name: brand::LOG_FILE_NAME.into(),
            server_config: ServerConfig::unavailable(),
            oz_config: OzConfig::unavailable(),
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    }
    // OSS builds are login-free: bypass the Firebase anonymous-user gate so
    // users land directly in the workspace without a hosted account.
    state = state
        .with_additional_features(&[warp_core::features::FeatureFlag::SkipFirebaseAnonymousUser]);
    ChannelState::set(state);

    warp::run()
}

// If we're not using an external plist, embed the following as the Info.plist.
#[cfg(all(not(feature = "extern_plist"), target_os = "macos"))]
embed_plist::embed_info_plist_bytes!(r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleDisplayName</key>
    <string>CastCodes</string>
    <key>CFBundleExecutable</key>
    <string>cast-codes</string>
    <key>CFBundleIdentifier</key>
    <string>dev.castcodes.CastCodes</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>CastCodes</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>UIDesignRequiresCompatibility</key>
    <true/>
    <key>CFBundleURLTypes</key>
    <array><dict><key>CFBundleURLName</key><string>CastCodes</string><key>CFBundleURLSchemes</key><array><string>castcodes</string></array></dict></array>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026, CastCodes Maintainers</string>
    </dict>
    </plist>
"#.as_bytes());
