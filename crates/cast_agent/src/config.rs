//! Cast Agent configuration. Resolved from (in order):
//! 1. Environment variables (`COVEN_SOCKET`, `COVEN_GATEWAY_URL`, `COVEN_TOKEN`).
//! 2. `~/.coven/config.toml` keys `socket_path`, `gateway_url`, `token`.
//! 3. `~/.coven/token` file (first line) for the token.
//! 4. Defaults: TCP `http://localhost:3999` (the CastCodes ↔ Coven daemon
//!    gateway bridge), no socket, no token.
//!
//! Transport selection: if `socket_path` is `Some(_)` after resolution
//! (env or file — there is no auto-detect; choosing Unix transport must
//! be explicit), the gateway uses the Unix transport and talks
//! `/api/v1/*` directly to the daemon. Otherwise it uses TCP and talks
//! `/v1/*` to `gateway_url` — by default the local bridge at
//! `127.0.0.1:3999` (see `~/.coven/castcodes-gateway-bridge.mjs`).
//!
//! Why TCP-by-default with the bridge in front: the bridge speaks plain
//! HTTP, so it works from any local consumer (CastCodes desktop, future
//! browser tools, ad-hoc curl) without each needing Unix-socket support.
//! Direct-Unix is faster (one less hop) but only useful when the caller
//! can speak Unix HTTP and wants to skip schema translation — set
//! `COVEN_SOCKET=$HOME/.coven/coven.sock` to opt in.

use std::path::PathBuf;

const DEFAULT_GATEWAY_URL: &str = "http://localhost:3999";

#[derive(Debug, Clone)]
pub struct CastAgentConfig {
    pub gateway_url: String,
    pub token: Option<String>,
    pub request_timeout: std::time::Duration,
    /// Path to the Coven daemon's Unix socket (typically
    /// `~/.coven/coven.sock`). When `Some`, the gateway client uses this
    /// instead of `gateway_url` and talks `/api/v1/*` to the daemon.
    pub socket_path: Option<PathBuf>,
}

impl Default for CastAgentConfig {
    fn default() -> Self {
        Self {
            gateway_url: DEFAULT_GATEWAY_URL.to_string(),
            token: None,
            request_timeout: std::time::Duration::from_secs(10),
            socket_path: None,
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
            .or_else(|| file_cfg.gateway_url.filter(|v| !v.trim().is_empty()))
            .unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string());

        // 2. Token — env > file > token-file.
        cfg.token = std::env::var("COVEN_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.token.filter(|v| !v.trim().is_empty()))
            .or_else(Self::load_token_file);

        // 3. Socket path — env > file. NO auto-detect: choosing Unix
        // transport must be explicit, so the default behaviour is the
        // bridge TCP path even on machines where the daemon socket
        // happens to exist.
        cfg.socket_path = std::env::var("COVEN_SOCKET")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(|v| PathBuf::from(v.trim()))
            .or_else(|| file_cfg.socket_path.filter(|v| !v.as_os_str().is_empty()));

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

    /// Default Unix-socket location used by the OpenCoven daemon.
    /// Returned for callers that want to opt in (e.g. set `COVEN_SOCKET`
    /// programmatically) — `load()` does NOT auto-detect.
    pub fn default_socket_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".coven").join("coven.sock"))
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
    #[serde(default)]
    socket_path: Option<PathBuf>,
}
