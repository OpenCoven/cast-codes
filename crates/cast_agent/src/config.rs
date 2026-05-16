//! Cast Agent configuration. Resolved from (in order):
//! 1. Environment variables (`COVEN_GATEWAY_URL`, `COVEN_TOKEN`).
//! 2. `~/.coven/config.toml` keys `gateway_url`, `token`.
//! 3. `~/.coven/token` file (first line) for the token.
//! 4. Defaults: `http://localhost:3000`, no token.

use std::path::PathBuf;

const DEFAULT_GATEWAY_URL: &str = "http://localhost:3000";

#[derive(Debug, Clone)]
pub struct CastAgentConfig {
    pub gateway_url: String,
    pub token: Option<String>,
    pub request_timeout: std::time::Duration,
}

impl Default for CastAgentConfig {
    fn default() -> Self {
        Self {
            gateway_url: DEFAULT_GATEWAY_URL.to_string(),
            token: None,
            request_timeout: std::time::Duration::from_secs(10),
        }
    }
}

impl CastAgentConfig {
    /// Load using the documented resolution order. Never fails — falls back
    /// to defaults if every source is unavailable.
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let file_cfg = Self::load_file().unwrap_or_default();

        // 1. Gateway URL — env > file > default.
        cfg.gateway_url = std::env::var("COVEN_GATEWAY_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or(file_cfg.gateway_url)
            .unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string());

        // 2. Token — env > file > token-file.
        cfg.token = std::env::var("COVEN_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or(file_cfg.token)
            .or_else(Self::load_token_file);

        cfg
    }

    /// Path to `~/.coven/config.toml` if a home directory is resolvable.
    pub fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".coven").join("config.toml"))
    }

    /// Path to `~/.coven/token` (one-line plaintext token).
    pub fn token_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".coven").join("token"))
    }

    fn load_file() -> Option<FileConfig> {
        let path = Self::config_path()?;
        let raw = std::fs::read_to_string(&path).ok()?;
        toml::from_str::<FileConfig>(&raw).ok()
    }

    fn load_token_file() -> Option<String> {
        let path = Self::token_path()?;
        let raw = std::fs::read_to_string(&path).ok()?;
        let first = raw.lines().next()?.trim().to_string();
        if first.is_empty() {
            None
        } else {
            Some(first)
        }
    }
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
struct FileConfig {
    #[serde(default)]
    gateway_url: Option<String>,
    #[serde(default)]
    token: Option<String>,
}
