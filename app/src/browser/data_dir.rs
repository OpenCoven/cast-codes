//! Per-session WebKit data directory resolution.
//!
//! Returns `<warp_home_config_dir>/browser/data/<session_id>/`, where
//! `session_id` is a UUID v4 string persisted with the browser pane
//! snapshot. Each browser pane owns its own session id, so cookies,
//! localStorage, IndexedDB, and cache are isolated per pane while staying
//! scoped to the CastCodes app (i.e., separate from any system browser).
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
use std::path::PathBuf;

use uuid::Uuid;

fn parse_session_uuid(session_id: &str) -> Option<Uuid> {
    let uuid = Uuid::parse_str(session_id).ok()?;
    (uuid.get_version_num() == 4).then_some(uuid)
}

/// Returns a canonical UUID session id, replacing invalid persisted values
/// with a fresh UUID v4 before they can become path components.
pub fn normalize_session_id(session_id: &str) -> String {
    match parse_session_uuid(session_id) {
        Some(uuid) => uuid.to_string(),
        None => {
            log::warn!("invalid browser session id; generated a replacement");
            Uuid::new_v4().to_string()
        }
    }
}

/// Resolves the per-session WebKit data directory, creating it on disk
/// if missing. `session_id` must be a UUID string; callers restoring
/// persisted state should call [`normalize_session_id`] before storing
/// the id on live browser views.
///
/// Returns `None` if the CastCodes home dir is unavailable or the
/// session id is invalid, or if the directory could not be created
/// (filesystem error logged at WARN level).
pub fn browser_data_dir(session_id: &str) -> Option<PathBuf> {
    let session_uuid = match parse_session_uuid(session_id) {
        Some(uuid) => uuid,
        None => {
            log::warn!("refusing browser data dir for invalid session id");
            return None;
        }
    };

    let mut dir = warp_core::paths::warp_home_config_dir()?;
    dir.push("browser");
    dir.push("data");
    dir.push(session_uuid.to_string());
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::warn!("failed to create browser data dir {dir:?}: {err}");
        return None;
    }
    Some(dir)
}

/// Best-effort one-shot cleanup of the pre-Layer-C
/// `<warp_home_config_dir>/browser/state.json` file. That file was the
/// single-pane JSON store; the SQLite app-state path now persists every
/// browser pane, so the legacy file is orphaned. Safe to call repeatedly
/// — missing file is silent, other errors log at WARN.
pub fn delete_legacy_state_file() {
    let Some(mut path) = warp_core::paths::warp_home_config_dir() else {
        return;
    };
    path.push("browser");
    path.push("state.json");
    match std::fs::remove_file(&path) {
        Ok(()) => log::info!("removed legacy browser state file {path:?}"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => log::warn!("failed to remove legacy browser state file {path:?}: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_session_id_preserves_valid_uuid() {
        let id = "5f2b3ce0-94f3-49ac-bab5-4f229a159f50";
        assert_eq!(normalize_session_id(id), id);
    }

    #[test]
    fn normalize_session_id_canonicalizes_uuid() {
        assert_eq!(
            normalize_session_id("5F2B3CE0-94F3-49AC-BAB5-4F229A159F50"),
            "5f2b3ce0-94f3-49ac-bab5-4f229a159f50"
        );
    }

    #[test]
    fn normalize_session_id_replaces_invalid_values() {
        let id = normalize_session_id("../state.json");
        assert_ne!(id, "../state.json");
        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn normalize_session_id_replaces_non_v4_uuid() {
        let id = normalize_session_id("550e8400-e29b-11d4-a716-446655440000");
        assert_ne!(id, "550e8400-e29b-11d4-a716-446655440000");
        assert_eq!(Uuid::parse_str(&id).unwrap().get_version_num(), 4);
    }

    #[test]
    fn browser_data_dir_rejects_invalid_session_ids() {
        assert!(browser_data_dir("../state.json").is_none());
        assert!(browser_data_dir("550e8400-e29b-11d4-a716-446655440000").is_none());
    }
}
