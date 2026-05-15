use std::path::PathBuf;

use anyhow::{Context, Result};

const DB_FILENAME: &str = "cli_chat.sqlite";

/// Returns the path to the CLI chat sqlite database, creating the parent
/// directory if it does not already exist.
///
/// The database lives inside the platform state directory alongside other
/// non-portable application data (see [`warp_core::paths::state_dir`]).
pub fn cli_chat_db_path() -> Result<PathBuf> {
    let dir = warp_core::paths::state_dir();
    anyhow::ensure!(
        !dir.as_os_str().is_empty(),
        "could not determine application state directory"
    );
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create state dir: {}", dir.display()))?;
    Ok(dir.join(DB_FILENAME))
}
