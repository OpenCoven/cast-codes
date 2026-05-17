//! Resolves the WebKit data directory for the embedded browser pane.
//!
//! Cookies, localStorage, IndexedDB, and service-worker storage all live
//! under this directory. Sharing this path across every pane in the app
//! gives consistent SSO behavior but isolates CastCodes from the user's
//! system browser (Safari/Chrome cookies are not visible).
//!
//! Per-workspace isolation is intentionally NOT done here — that requires
//! a stable workspace identity that survives restart, which cast-codes
//! does not currently expose. Future work.

use std::path::PathBuf;

fn path_for_base(base: PathBuf) -> PathBuf {
    base.join("browser").join("data")
}

/// Returns the WebKit data directory path. Creates the directory if it
/// does not already exist. `None` if the CastCodes home dir cannot be
/// resolved (e.g. headless CI without `HOME`).
pub fn path() -> Option<PathBuf> {
    let base = warp_core::paths::warp_home_config_dir()?;
    let dir = path_for_base(base);
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::warn!("failed to create browser data dir at {dir:?}: {err}");
        return None;
    }
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_includes_browser_segment() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = path_for_base(dir.path().to_path_buf());
        assert_eq!(path, dir.path().join("browser").join("data"));
    }
}
