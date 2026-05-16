use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};

// Per-user settings for the embedded browser pane.
define_settings_group!(BrowserSettings, settings: [
    devtools_enabled: DevtoolsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "browser.devtools_enabled",
        description: "Enables Web Inspector (DevTools) in the embedded browser pane. Off by default to reduce accidental data exposure.",
    },
    blocklist_enabled: BlocklistEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "browser.blocklist_enabled",
        description: "Blocks navigations to URLs matching the bundled tracker/ad blocklist. Subresource requests are NOT filtered (wry 0.38 limitation).",
    }
]);
