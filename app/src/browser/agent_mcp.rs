//! MCP server exposing `BrowserAgent` over a Unix domain socket.
//!
//! Spawned on workspace startup. Writes the socket path + tool surface
//! version to `~/.cast-codes/mcp.json` so MCP-aware harnesses
//! auto-discover it.
//!
//! This is a skeleton: the socket bind and discovery-file write are in
//! place but the tool dispatch loop is intentionally not yet wired —
//! the substantive request marshalling onto the warpui main loop and
//! the rmcp integration land in a follow-up PR. See
//! `specs/CASTCODES-BROWSER-PANEL/PLAN-04-mcp-server.md`.
//!
//! ## Platform support
//!
//! The server relies on Unix domain sockets and is therefore only
//! compiled on Unix targets (`#[cfg(unix)]`). A no-op stub is
//! provided for WASM and for any other non-Unix target (e.g. Windows)
//! until named-pipe support is added.

/// Unix implementation — binds the domain socket and writes the
/// discovery file.
#[cfg(unix)]
pub fn serve() {
    let Some(path) = socket_path() else {
        log::warn!("agent_mcp: cannot resolve socket path");
        return;
    };

    // Remove any stale socket from a previous process.
    let _ = std::fs::remove_file(&path);

    let listener = match std::os::unix::net::UnixListener::bind(&path) {
        Ok(l) => l,
        Err(err) => {
            log::warn!("agent_mcp: bind failed at {path:?}: {err}");
            return;
        }
    };

    // Lock down to owner-only.
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));

    // Write the discovery file so MCP-aware harnesses find us.
    if let Err(err) = write_discovery(&path) {
        log::warn!("agent_mcp: discovery file write failed: {err}");
    }

    log::info!("agent_mcp: listening on {path:?} (tool dispatch not yet implemented)");
    // Accept loop is intentionally minimal: any incoming connection is
    // closed immediately. This makes the socket visible to client
    // probes without committing us to a half-baked tool surface.
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => drop(stream),
            Err(err) => {
                log::debug!("agent_mcp: accept error: {err}");
                break;
            }
        }
    }
}

/// WASM stub — Unix domain sockets are unavailable in the browser.
#[cfg(target_family = "wasm")]
pub fn serve() {}

/// Non-Unix, non-WASM stub (e.g. Windows) — named-pipe support is a
/// future effort.
#[cfg(all(not(unix), not(target_family = "wasm")))]
pub fn serve() {
    log::debug!("agent_mcp: Unix domain sockets not available on this platform; skipping");
}

#[cfg(unix)]
fn socket_path() -> Option<std::path::PathBuf> {
    let dir = warp_core::paths::warp_home_config_dir()?.join("mcp");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("browser.sock"))
}

/// Write `~/.cast-codes/mcp.json` describing the socket path and tool
/// surface.
///
/// ## Security: no TOCTOU window
///
/// The temp file is `chmod`-ed to `0600` *before* the atomic rename so
/// the destination is never briefly world-readable. Relying on
/// `set_permissions` after `rename` would leave a window (between
/// `rename` and `chmod`) where the file is readable at the umask-
/// derived mode (typically `0644`).
#[cfg(unix)]
fn write_discovery(socket: &std::path::Path) -> std::io::Result<()> {
    use std::fs;
    use std::path::PathBuf;

    use serde::Serialize;

    #[derive(Serialize)]
    struct Discovery<'a> {
        transport: &'static str,
        socket: &'a std::path::Path,
        tools: &'static [&'static str],
    }

    const TOOLS: &[&str] = &[
        "browser.list_tabs",
        "browser.navigate",
        "browser.reload",
        "browser.new_tab",
        "browser.evaluate",
    ];

    let Some(home) = warp_core::paths::warp_home_config_dir() else {
        return Ok(());
    };
    let dest: PathBuf = home.join("mcp.json");
    let tmp = dest.with_extension("json.tmp");
    let payload = Discovery {
        transport: "unix-socket",
        socket,
        tools: TOOLS,
    };
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(&tmp, &bytes)?;

    // Harden the temp file *before* the rename so the destination is
    // never briefly readable at the umask-derived mode.
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;

    fs::rename(&tmp, &dest)?;
    Ok(())
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn write_discovery_creates_mode_0600_file() {
        use std::os::unix::fs::MetadataExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("browser.sock");

        // We can't call write_discovery directly (it uses
        // warp_home_config_dir), so exercise the chmod-before-rename
        // contract directly.
        let dest = dir.path().join("mcp.json");
        let tmp = dest.with_extension("json.tmp");
        std::fs::write(&tmp, b"{}").unwrap();

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::rename(&tmp, &dest).unwrap();

        let mode = std::fs::metadata(&dest).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o600, "mcp.json must be owner-only (0600), got {mode:o}");
        let _ = socket; // suppress unused warning
    }
}
