//! On-disk persistence for the Coven Gateway stream history.
//!
//! Stores up to `COVEN_STREAM_HISTORY_MAX` completed streams as
//! `~/.coven/stream-history.json` so the agent panel can restore them
//! across CastCodes restarts. Matches the same directory convention
//! cast_agent uses for its `config.toml` and `token` (`~/.coven/`,
//! resolved via [`dirs::home_dir`]).
//!
//! Path lives outside CastCodes' workspace serialization (`.cast-codes/`
//! per `CASTCODES.md`) deliberately — stream history is conversation
//! state that follows the user across workspaces, not workspace state.
//! Putting it in `~/.coven/` also keeps it next to the Coven Gateway
//! config that produced it.
//!
//! Errors are logged but never propagated. A missing file, a parse
//! failure, or a failed write all degrade gracefully to "no history"
//! — the panel just shows an empty history list and starts fresh.

#![cfg(feature = "cast-agent")]

use std::path::PathBuf;

use super::panel::CovenStreamHistoryEntry;

const HISTORY_FILENAME: &str = "stream-history.json";

fn history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".coven").join(HISTORY_FILENAME))
}

/// Read the persisted history. Returns `Vec::new()` on any error —
/// the panel treats absent history as "fresh session," not a fatal
/// startup problem.
pub fn load() -> Vec<CovenStreamHistoryEntry> {
    let Some(path) = history_path() else {
        log::debug!("cast_agent: stream history path unavailable (no home dir)");
        return Vec::new();
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(err) => {
            log::warn!(
                "cast_agent: stream history read failed at {}: {err}",
                path.display()
            );
            return Vec::new();
        }
    };
    match serde_json::from_str::<Vec<CovenStreamHistoryEntry>>(&raw) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!(
                "cast_agent: stream history parse failed at {}: {err} — discarding",
                path.display()
            );
            Vec::new()
        }
    }
}

/// Write the history to disk. Atomic-ish via write-to-temp +
/// rename so a crash partway through doesn't truncate the existing
/// file. Errors logged and dropped.
pub fn save(entries: &[CovenStreamHistoryEntry]) {
    let Some(path) = history_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            log::warn!(
                "cast_agent: stream history mkdir failed at {}: {err}",
                parent.display()
            );
            return;
        }
    }
    let payload = match serde_json::to_string_pretty(entries) {
        Ok(p) => p,
        Err(err) => {
            log::warn!("cast_agent: stream history serialize failed: {err}");
            return;
        }
    };
    let tmp = path.with_extension("json.tmp");
    if let Err(err) = std::fs::write(&tmp, &payload) {
        log::warn!(
            "cast_agent: stream history write failed at {}: {err}",
            tmp.display()
        );
        return;
    }
    if let Err(err) = std::fs::rename(&tmp, &path) {
        log::warn!(
            "cast_agent: stream history rename failed at {} -> {}: {err}",
            tmp.display(),
            path.display()
        );
    }
}
