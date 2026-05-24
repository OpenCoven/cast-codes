//! Per-pane WebKit data directory resolution.
//!
//! Returns `<warp_home_config_dir>/browser/data`. The intent is that every
//! browser pane shares one persistent data store so cookies, localStorage,
//! and IndexedDB survive across pane open/close cycles, while remaining
//! scoped to the CastCodes app (i.e., separate from any system browser).
//!
//! ## Platform reality check (wry 0.38)
//!
//! - **macOS**: `wry::WebView` ignores the `WebContext` (see
//!   `wkwebview/mod.rs:95` in wry 0.38 where the parameter is named
//!   `_web_context` and never assigned). All WKWebView instances use
//!   `WKWebsiteDataStore.defaultDataStore`, which is per-app-sandbox — so
//!   data is already isolated from Safari, but cannot be redirected to a
//!   custom path without a wry upgrade or a raw objc bridge. The directory
//!   resolved here is still created so other subsystems (downloads, future
//!   service workers, etc.) have a stable home.
//! - **Linux / webkit2gtk**: `WebContext` is honored; passing a data
//!   directory to `WebViewBuilder::with_web_context` works as expected.
//! - **Windows / WebView2**: Honored via the same path.
//!
//! When wry adds macOS `WKWebsiteDataStore` plumbing, the wiring in
//! `webview_host.rs` will start using this directory without further
//! changes here.

use std::path::PathBuf;

/// Resolves the per-pane WebKit data directory, creating it on disk if
/// missing. Returns `None` if the CastCodes home dir is unavailable or the
/// directory could not be created (filesystem error logged at WARN level).
pub fn browser_data_dir() -> Option<PathBuf> {
    let mut dir = warp_core::paths::warp_home_config_dir()?;
    dir.push("browser");
    dir.push("data");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::warn!("failed to create browser data dir {dir:?}: {err}");
        return None;
    }
    Some(dir)
}
