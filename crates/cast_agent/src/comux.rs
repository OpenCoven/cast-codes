//! Comux bridge — discovers a running Comux daemon via Unix domain socket
//! and asks it for active panes. Used to enrich [`crate::substrate::Substrate`]
//! so the agent knows about terminal panes outside CastCodes itself.
//!
//! Wire protocol (BunsDev/comux `src/daemon/protocol.ts`):
//! - Request:  `{ "type": "list_panes" }\n`
//! - Response: `{ "panes": [{ "id", "cwd", "title", "active" }] }\n`
//!
//! If the socket isn't present or the request fails, this returns an empty
//! list and logs at debug level — the bridge is best-effort.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComuxPane {
    pub id: String,
    pub cwd: PathBuf,
    pub title: String,
    pub active: bool,
}

#[derive(serde::Serialize)]
struct ListPanesRequest {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(serde::Deserialize)]
struct ListPanesResponse {
    panes: Vec<ComuxPane>,
}

/// Resolves the Comux socket path: `$COMUX_SOCKET` env var, then `/tmp/comux.sock`.
pub fn resolve_socket_path() -> PathBuf {
    if let Ok(env_path) = std::env::var("COMUX_SOCKET") {
        if !env_path.trim().is_empty() {
            return PathBuf::from(env_path);
        }
    }
    PathBuf::from("/tmp/comux.sock")
}

pub struct ComuxBridge {
    socket_path: PathBuf,
}

impl ComuxBridge {
    pub fn new() -> Self {
        Self {
            socket_path: resolve_socket_path(),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Returns an empty `Vec` if the socket is absent or the request fails.
    pub async fn list_panes(&self) -> anyhow::Result<Vec<ComuxPane>> {
        #[cfg(unix)]
        {
            if !self.socket_path.exists() {
                log::debug!(
                    "cast_agent: Comux socket {:?} not present, skipping pane discovery",
                    self.socket_path
                );
                return Ok(Vec::new());
            }
            match unix_request(&self.socket_path).await {
                Ok(panes) => Ok(panes),
                Err(err) => {
                    log::debug!(
                        "cast_agent: Comux list_panes failed at {:?}: {err}",
                        self.socket_path
                    );
                    Ok(Vec::new())
                }
            }
        }

        #[cfg(not(unix))]
        {
            // Comux is Unix-only; on other platforms always degrade quietly.
            log::debug!("cast_agent: Comux bridge unsupported on this platform");
            Ok(Vec::new())
        }
    }
}

impl Default for ComuxBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(unix)]
async fn unix_request(path: &Path) -> anyhow::Result<Vec<ComuxPane>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let mut stream = UnixStream::connect(path).await?;
    let req = serde_json::to_string(&ListPanesRequest { kind: "list_panes" })?;
    stream.write_all(req.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let parsed: ListPanesResponse = serde_json::from_str(line.trim())?;
    Ok(parsed.panes)
}
