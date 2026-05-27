// Persistence is only invoked from the native (non-wasm) browser pane
// path. On wasm the helpers exist but are never called.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

//! Persistence of `BrowserState` to a JSON file under the CastCodes
//! support directory. Atomic write via temp-file + rename. Load is
//! lenient: any failure (missing file, malformed JSON, unknown version)
//! returns `None` instead of panicking.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::browser_model::{BrowserState, BROWSER_STATE_VERSION};

/// Returns the path to the persisted browser state file.
pub fn state_path(state_dir: &Path) -> PathBuf {
    state_dir.join("browser").join("state.json")
}

/// Loads the persisted state, or `None` if the file is missing, malformed,
/// or written by an unknown version.
pub fn load(state_dir: &Path) -> Option<BrowserState> {
    let path = state_path(state_dir);
    let bytes = fs::read(&path).ok()?;
    let parsed: BrowserState = serde_json::from_slice(&bytes).ok()?;
    if parsed.v != BROWSER_STATE_VERSION {
        return None;
    }
    Some(parsed)
}

/// Atomically writes the state file. Creates parent directories as needed.
pub fn save(state_dir: &Path, state: &BrowserState) -> std::io::Result<()> {
    let path = state_path(state_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        let json = serde_json::to_vec_pretty(state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
pub fn save_to_default_dir(state: &BrowserState) -> std::io::Result<()> {
    let dir = warp_core::paths::warp_home_config_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "CastCodes home dir unavailable",
        )
    })?;
    save(&dir, state)
}

#[cfg(test)]
mod tests {
    use super::super::browser_model::{BrowserModel, TabSnapshot};
    use super::*;

    fn tmp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn load_returns_none_when_file_absent() {
        let dir = tmp_dir();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tmp_dir();
        let state = BrowserState {
            v: BROWSER_STATE_VERSION,
            open: true,
            tabs: vec![
                TabSnapshot {
                    url: "https://a.test".into(),
                    title: "A".into(),
                    pinned: true,
                },
                TabSnapshot {
                    url: "https://b.test".into(),
                    title: "B".into(),
                    pinned: false,
                },
            ],
            active: 1,
        };
        save(dir.path(), &state).expect("save");
        let loaded = load(dir.path()).expect("load");
        assert_eq!(loaded, state);
    }

    #[test]
    fn load_returns_none_for_malformed_json() {
        let dir = tmp_dir();
        let path = state_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not json").unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn load_returns_none_for_unknown_version() {
        let dir = tmp_dir();
        let path = state_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, br#"{"v":99,"open":true,"tabs":[],"active":0}"#).unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_uses_atomic_temp_rename() {
        let dir = tmp_dir();
        let model = BrowserModel::new("https://a.test");
        save(dir.path(), &model.snapshot(true)).expect("save");
        let tmp = state_path(dir.path()).with_extension("json.tmp");
        assert!(!tmp.exists(), "leftover temp file: {:?}", tmp);
    }
}
