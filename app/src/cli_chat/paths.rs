use std::path::PathBuf;

use anyhow::{Context, Result};

const DB_FILENAME: &str = "cli_chat.sqlite";

/// Returns the path to the CLI chat sqlite database, creating the parent
/// directory if it does not already exist.
///
/// The database lives inside the CastCodes home config directory
/// (`~/.cast-codes*`) alongside other fork-local user data.
pub fn cli_chat_db_path() -> Result<PathBuf> {
    let dir = warp_core::paths::warp_home_config_dir()
        .context("could not determine CastCodes home config directory")?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create CastCodes config dir: {}", dir.display()))?;
    Ok(dir.join(DB_FILENAME))
}
