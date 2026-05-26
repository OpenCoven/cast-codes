//! Per-session WebKit data directory resolution.
//!
//! Returns `<warp_home_config_dir>/browser/data/<session_id>/`. Each
//! workspace tab owns its own browser pane and its own session id (the
//! pane group's [`EntityId`]), so cookies, localStorage, IndexedDB, and
//! cache are isolated per workspace tab. Sessions remain scoped to the
//! CastCodes app (i.e., separate from any system browser).
//!
//! Session ids are runtime [`EntityId`]s — stable within a single app
//! launch but not across restarts. That is acceptable for Layer B: a
//! fresh app launch yields fresh tabs with fresh contexts. Persistence
//! across restarts (Layer C) will need a stable per-tab UUID stored on
//! `TabData`; when that lands, the same directory layout still applies
//! and only the id source changes.
//!
//! ## Platform reality check (wry 0.38)
//!
//! - **macOS**: `wry::WebView` ignores the `WebContext` (see
//!   `wkwebview/mod.rs:95` in wry 0.38 where the parameter is named
//!   `_web_context` and never assigned). All WKWebView instances use
//!   `WKWebsiteDataStore.defaultDataStore`, which is per-app-sandbox — so
//!   data is already isolated from Safari, but cannot be redirected to a
//!   custom path without a wry upgrade or a raw objc bridge. The per-
//!   session directory is still created so other subsystems (downloads,
//!   future service workers, etc.) have a stable home, and so the per-
//!   tab isolation lights up automatically once wry exposes the macOS
//!   `WKWebsiteDataStore` plumbing.
//! - **Linux / webkit2gtk**: `WebContext` is honored; per-session
//!   isolation works as expected.
//! - **Windows / WebView2**: Honored via the same path.
//!
//! [`EntityId`]: warpui_core::EntityId

use std::path::PathBuf;

/// Resolves the per-session WebKit data directory, creating it on disk
/// if missing. `session_id` is typically the owning pane group's
/// [`EntityId`] rendered via `Display` — any short, filesystem-safe
/// identifier works.
///
/// Returns `None` if the CastCodes home dir is unavailable or the
/// directory could not be created (filesystem error logged at WARN
/// level).
///
/// [`EntityId`]: warpui_core::EntityId
pub fn browser_data_dir(session_id: &str) -> Option<PathBuf> {
    let mut dir = warp_core::paths::warp_home_config_dir()?;
    dir.push("browser");
    dir.push("data");
    dir.push(session_id);
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::warn!("failed to create browser data dir {dir:?}: {err}");
        return None;
    }
    Some(dir)
}
