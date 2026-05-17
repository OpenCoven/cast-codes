# CastCodes Browser Panel — Plan 4: MCP Server

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans`. Read [`PRODUCT.md`](./PRODUCT.md), [`TECH.md`](./TECH.md), and [`PLAN-03-agent-surface.md`](./PLAN-03-agent-surface.md) first.
>
> **Branch base:** Stacked on PLAN-03. Branch from `feat/browser-panel-agent-surface` (PR #36).
>
> **Signing rule:** Every `git commit` MUST pass `-S`. Verify via `git log -1 --show-signature | head -3`.

**Goal:** Expose `BrowserAgent` (from PLAN-03) as MCP tools so any MCP-aware coding harness — Claude Code, Cursor, the cast-codes built-in agent — can drive the embedded browser pane via a standardized protocol.

**Architecture:** A new `app/src/browser/agent_mcp.rs` module spawns an MCP server on workspace startup. The server binds to a Unix domain socket under the CastCodes support directory and advertises its location + tool surface in `~/.cast-codes/mcp.json` so harnesses can auto-discover it. Each MCP tool maps mechanically to a `BrowserAgent` method.

**Auth model:** Unix file permissions (`0600` socket + `0600` discovery file). Owner-only. No tokens. No TCP exposure.

**Crate selection:** Reuse whatever MCP crate cast-codes already imports for plugin support (`grep -rn "rmcp\|mcp::" crates/`). If none exists yet, vendor `rmcp` 0.6+ at the workspace level — the de-facto Rust MCP SDK.

---

## Tool surface

| MCP tool | Maps to |
|---|---|
| `browser.list_tabs` | `BrowserAgent::list_tabs(ctx)` → JSON array of `TabInfo` |
| `browser.navigate` | `BrowserAgent::navigate(ctx, url)` (currently stub) |
| `browser.reload` | `BrowserAgent::reload(ctx)` (currently stub) |
| `browser.new_tab` | `BrowserAgent::new_tab(ctx, url)` (currently stub) |
| `browser.evaluate` | `BrowserAgent::evaluate_js(ctx, script).await` (currently stub) |

Tools that map to stubs return an MCP error indicating the capability is not yet wired. As PLAN-03 follow-ups land, the tool dispatchers light up automatically.

`browser.screenshot`, `browser.pick_element`, `browser.console.tail`, `browser.network.tail` ship in a later iteration alongside the wry upgrade.

---

## Files created or modified

**Created:**

| Path | Responsibility |
|---|---|
| `app/src/browser/agent_mcp.rs` | MCP server: socket binding, discovery file write, tool list, tool dispatch. |
| `app/src/browser/agent_mcp/discovery.rs` | Writes `~/.cast-codes/mcp.json`. Single-file overwrite (no merge), atomic temp+rename. |

**Modified:**

| Path | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `rmcp = { version = "0.6", features = ["server", "transport-io"] }` to `[workspace.dependencies]` unless an MCP crate is already declared. |
| `app/Cargo.toml` | `rmcp.workspace = true`. |
| `app/src/browser/mod.rs` | `pub(crate) mod agent_mcp;` |
| `app/src/lib.rs` | Spawn `agent_mcp::serve()` as a background warpui task during app init (mirror `cast_agent::runtime::boot`). |

---

## Phase 1 — Plan + skeleton

### Task 1.1: Commit this plan

```bash
git add specs/CASTCODES-BROWSER-PANEL/PLAN-04-mcp-server.md
git commit -S -m "docs(specs): PLAN-04 (MCP server)"
```

### Task 1.2: Module skeleton

Create `app/src/browser/agent_mcp.rs`:

```rust
//! MCP server exposing BrowserAgent over a Unix domain socket.
//!
//! Spawned on workspace startup. Writes the socket path + tool surface
//! version to ~/.cast-codes/mcp.json so MCP-aware harnesses
//! auto-discover it.

#[cfg(not(target_family = "wasm"))]
pub fn serve() {
    log::debug!("agent_mcp::serve invoked — TODO: implement server lifecycle");
}

#[cfg(target_family = "wasm")]
pub fn serve() {}
```

Wire in `app/src/browser/mod.rs`:
```rust
pub(crate) mod agent_mcp;
```

Commit:
```bash
git add app/src/browser/agent_mcp.rs app/src/browser/mod.rs
git commit -S -m "feat(browser): agent_mcp skeleton (no server yet)"
```

---

## Phase 2 — Socket + discovery

### Task 2.1: Add MCP crate to workspace

- [ ] Check whether an MCP crate is already declared:
```bash
grep -rn "^rmcp\|^mcp\|mcp_server" Cargo.toml crates/*/Cargo.toml
```

If not, add `rmcp = { version = "0.6", features = ["server", "transport-io"] }` to workspace deps and `rmcp.workspace = true` to `app/Cargo.toml`.

Commit the dep change separately.

### Task 2.2: Bind a Unix domain socket

`agent_mcp.rs`:

```rust
fn socket_path() -> Option<std::path::PathBuf> {
    let dir = warp_core::paths::warp_home_config_dir()?.join("mcp");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("browser.sock"))
}

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

    // TODO: spawn an accept loop on a tokio task.
    // Each accepted connection runs an rmcp Server reading JSON-RPC
    // from the stream and dispatching to BrowserAgent.
    log::debug!("agent_mcp: listening on {path:?}");
    let _ = listener;
}
```

### Task 2.3: Discovery file

`agent_mcp/discovery.rs`:

```rust
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct Discovery {
    transport: &'static str,
    socket: PathBuf,
    tools: Vec<&'static str>,
}

const TOOLS: &[&str] = &[
    "browser.list_tabs",
    "browser.navigate",
    "browser.reload",
    "browser.new_tab",
    "browser.evaluate",
];

pub fn write(socket: &Path) -> std::io::Result<()> {
    let Some(home) = warp_core::paths::warp_home_config_dir() else {
        return Ok(());
    };
    let dest = home.join("mcp.json");
    let tmp = dest.with_extension("json.tmp");
    let payload = Discovery {
        transport: "unix-socket",
        socket: socket.to_path_buf(),
        tools: TOOLS.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(&tmp, &bytes)?;
    fs::rename(&tmp, &dest)?;

    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o600));
    Ok(())
}
```

Call `discovery::write(&path)` from `serve()` after socket bind.

---

## Phase 3 — Tool dispatch

This is the substantive implementation work. Each connection accepts a JSON-RPC request, matches the `method` against the tool list, dispatches to the corresponding `BrowserAgent` method, and writes the JSON-encoded response back. Failures map to MCP error envelopes.

The dispatch loop runs on a tokio task per accepted connection. Each task needs read-only access to a warpui `AppContext` — that requires marshalling work back onto the main thread via a `Workspace` action or a dedicated mpsc channel polled in the warpui event loop. Sketch:

1. MCP task receives a request.
2. It sends `(method, params, reply_oneshot)` to a per-app `agent_mcp_rx`.
3. The warpui event loop (or a dedicated `Workspace` subscriber) drains `agent_mcp_rx`, calls the matching `BrowserAgent` method with `&mut AppContext`, and replies on the oneshot.
4. The MCP task awaits the oneshot, serializes the result, writes JSON-RPC.

Each request is short-lived so the channel doesn't grow.

This phase is the most code in PLAN-04 and depends on the PLAN-03 stubs being filled in (otherwise most tools return `not yet implemented`).

---

## Phase 4 — App init wiring

In `app/src/lib.rs`, find where `cast_agent::runtime::boot()` is called (or a similar app-startup hook). Add:

```rust
#[cfg(not(target_family = "wasm"))]
std::thread::spawn(|| {
    crate::browser::agent_mcp::serve();
});
```

Spawning on a dedicated thread avoids tying the MCP lifetime to the workspace lifecycle (the server lives for the app duration).

---

## Phase 5 — PR

- [ ] `cargo test -p warp-app browser::` passes.
- [ ] `./script/check_rebrand` passes.
- [ ] Manual: open the cast-codes app, then from a terminal:
```bash
cat ~/.cast-codes/mcp.json | jq
nc -U ~/.cast-codes/mcp/browser.sock < /dev/null && echo "connected"
```
Verify the discovery file lists tools and the socket accepts connections.

- [ ] Open PR against `feat/browser-panel-agent-surface`.

---

## Self-Review

**Spec coverage:**

| TECH spec § 9 requirement | Coverage |
|---|---|
| In-process MCP server | Phase 2 (socket bind) + Phase 4 (init wiring) |
| Unix domain socket discovery via `~/.cast-codes/mcp.json` | Phase 2.3 |
| Tools mapping to BrowserAgent | Phase 3 |
| Mode 0600 (owner-only) | Phase 2.2 + 2.3 |
| Reuse existing MCP crate if present | Phase 2.1 |

**Open questions deferred to implementation:**

- Whether to reuse `rmcp`, an existing local plugin MCP client, or hand-roll JSON-RPC. Decide during Phase 2.1.
- How to marshal requests onto the warpui main thread cleanly. Phase 3 sketches a channel-based approach; the concrete plumbing depends on how warpui exposes "schedule on main loop".

**Limitations:**

- Most tools delegate to PLAN-03 stubs and currently return `not yet implemented`. The PR makes the discovery surface real but the tool surface light; landing the PLAN-03 follow-ups makes them work.
- No Windows support: Unix domain sockets only. Named pipes for Windows are a separate effort.
