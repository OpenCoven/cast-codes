# Familiars Multi-Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the CastCodes agent panel select among multiple Familiars (daemon persona catalog) per conversation with a global default, degrading to the backend harness list when the daemon `/api/v1/familiars` catalog is absent.

**Architecture:** Add a daemon wire type (`DaemonFamiliar`) + a graceful `list_familiars()` gateway call + a pure `resolve()` selection layer (`RequestTarget`) in `cast_agent`, cached like `SessionStore`. The panel reads the cached catalog, stores a per-conversation selection, and feeds the resolved target through the existing `msg.body["harness"]` / new `msg.body["familiarId"]` request path. No daemon-chat wire change beyond adding the optional `familiarId` field.

**Tech Stack:** Rust, `cast_agent` crate (serde/serde_json/toml/tokio, unix-socket HTTP via `unix_http`), `warpui`/GPUI for the panel + settings UI. Re-exported to the app through `::ai::cast_agent::*`.

**Repo rules (apply to EVERY commit):**
- Sign every commit: `git commit -S`. **No `Co-Authored-By`/AI-attribution trailer** (repo hard rule; `./script/check_ai_attribution` scans commit messages).
- Before any push: `./script/check_rebrand` and `./script/check_ai_attribution` must pass.
- CI runs `cargo clippy --workspace --all-targets -- -D warnings`. Do not use `std::time::Instant` (banned; use `instant::Instant`).
- Work stays in worktree `/Users/buns/Documents/GitHub/OpenCoven/cast-codes/.worktrees/familiars` on branch `feat/familiars-multi-harness`. Rebase onto latest `origin/main` before opening the PR (concurrent `cast_agent` work is active).
- Build/test the cast_agent crate with `cargo test -p cast_agent`. Build the app with the feature: `cargo check -p warp --bin cast-codes --features gui,cast-agent` (the panel/settings code is behind the app-crate `cast-agent` feature).

---

## File Structure

- **Create** `crates/cast_agent/src/daemon_schema.rs` — `DaemonFamiliar` wire type (+ tolerant serde). One responsibility: daemon `/api/v1/familiars` shape.
- **Create** `crates/cast_agent/src/familiar.rs` — `SUPPORTED_HARNESSES`, `RequestTarget`, pure `resolve()`, and `FamiliarStore` cache. One responsibility: Familiar selection/resolution.
- **Modify** `crates/cast_agent/src/gateway.rs` — add `list_familiars()` + pure `parse_familiars(status, body)` helper.
- **Modify** `crates/cast_agent/src/config.rs` — add `default_familiar: Option<String>` (env + TOML).
- **Modify** `crates/cast_agent/src/lib.rs` — declare the two new modules.
- **Modify** `crates/cast_agent/src/agent.rs` + `runtime.rs` — expose `list_familiars()` / familiar snapshot on `CastAgent`/`CastAgentRuntime` (mirror the session methods).
- **Modify** `app/src/ai_assistant/panel.rs` — `CovenStreamState.selected_familiar`, picker chip, `coven_code_agent_message_body` takes a `RequestTarget`.
- **Modify** `app/src/settings_view/ai_page.rs` — a read-only Familiars section on the agent page.

---

## Task 1: `DaemonFamiliar` wire type

**Files:**
- Create: `crates/cast_agent/src/daemon_schema.rs`
- Modify: `crates/cast_agent/src/lib.rs`

- [ ] **Step 1: Declare the module.** In `crates/cast_agent/src/lib.rs`, add after the `pub mod config;` line:

```rust
pub mod daemon_schema;
```

- [ ] **Step 2: Write the failing test.** Create `crates/cast_agent/src/daemon_schema.rs` with the type and a parse test:

```rust
//! Wire types for the Coven daemon `/api/v1/*` endpoints that are specific
//! to the Familiars catalog. Shapes match the daemon contract described in
//! `specs/castcodes-coven-internal-loop/PLAN-01-start-coding.md`
//! (`DaemonFamiliar`). Deserialization is tolerant: unknown fields are
//! ignored and absent fields default, so daemon schema drift degrades
//! gracefully instead of hard-failing the client.

use serde::Deserialize;

/// A persona entry from `GET /api/v1/familiars`. A Familiar resolves to a
/// backend harness + system prompt daemon-side; the client only displays
/// and selects it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DaemonFamiliar {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub active_sessions: u32,
    #[serde(default)]
    pub memory_freshness: String,
}

impl DaemonFamiliar {
    /// Best label for the picker: `display_name` if set, else `name`, else `id`.
    pub fn label(&self) -> &str {
        if !self.display_name.is_empty() {
            &self.display_name
        } else if !self.name.is_empty() {
            &self.name
        } else {
            &self.id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_catalog_entry() {
        let json = r#"{
            "id": "nova", "name": "nova", "display_name": "Nova",
            "emoji": "🌟", "role": "builder", "description": "Ships features",
            "status": "online", "last_seen": "2026-07-17T00:00:00Z",
            "active_sessions": 2, "memory_freshness": "fresh"
        }"#;
        let f: DaemonFamiliar = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, "nova");
        assert_eq!(f.label(), "Nova");
        assert_eq!(f.active_sessions, 2);
    }

    #[test]
    fn tolerates_missing_and_unknown_fields() {
        let json = r#"{"id": "sage", "unexpected": "ignored"}"#;
        let f: DaemonFamiliar = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, "sage");
        assert_eq!(f.label(), "sage"); // falls back to id
        assert_eq!(f.status, ""); // defaulted
    }
}
```

- [ ] **Step 3: Run to verify it fails then passes.**

Run: `cargo test -p cast_agent daemon_schema`
Expected: compiles and both tests PASS (if the module wasn't declared it fails first — declare it in Step 1).

- [ ] **Step 4: Commit.**

```bash
git add crates/cast_agent/src/daemon_schema.rs crates/cast_agent/src/lib.rs
git commit -S -m "feat(cast_agent): add DaemonFamiliar wire type"
```

---

## Task 2: `list_familiars()` gateway call (graceful)

**Files:**
- Modify: `crates/cast_agent/src/gateway.rs`

- [ ] **Step 1: Write the failing test** for the pure parse helper. Add to the bottom of `crates/cast_agent/src/gateway.rs` (inside or add a `#[cfg(test)] mod familiars_tests`):

```rust
#[cfg(test)]
mod familiars_tests {
    use super::*;

    #[test]
    fn parses_2xx_catalog() {
        let body = br#"[{"id":"nova"},{"id":"sage"}]"#;
        let out = parse_familiars(200, body);
        assert_eq!(out.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["nova", "sage"]);
    }

    #[test]
    fn empty_on_404() {
        assert!(parse_familiars(404, b"not found").is_empty());
    }

    #[test]
    fn empty_on_malformed_2xx_body() {
        assert!(parse_familiars(200, b"{not json}").is_empty());
    }
}
```

- [ ] **Step 2: Run to verify it fails.**

Run: `cargo test -p cast_agent familiars_tests`
Expected: FAIL — `parse_familiars` not found.

- [ ] **Step 3: Implement the pure helper + async call.** Add near the other `impl GatewayClient` methods in `gateway.rs`. First the free helper (module scope):

```rust
/// Parse a `/api/v1/familiars` response into a catalog. Returns an empty
/// Vec for any non-2xx status or malformed body so the UI degrades to the
/// harness fallback instead of surfacing an error. Logs at debug on drop.
fn parse_familiars(status: u16, body: &[u8]) -> Vec<crate::daemon_schema::DaemonFamiliar> {
    if !(200..300).contains(&status) {
        log::debug!("cast_agent: /familiars returned HTTP {status}; treating as empty catalog");
        return Vec::new();
    }
    match serde_json::from_slice::<Vec<crate::daemon_schema::DaemonFamiliar>>(body) {
        Ok(list) => list,
        Err(err) => {
            log::debug!("cast_agent: /familiars body did not parse ({err}); empty catalog");
            Vec::new()
        }
    }
}
```

Then the method on `impl GatewayClient` (mirror `list_sessions`, but never error — always `Ok`):

```rust
    /// GET the Familiar persona catalog. Never errors: returns an empty
    /// catalog when the daemon does not implement the endpoint (404), is
    /// unreachable, or returns a malformed body. The TCP/bridge transport
    /// has no familiars endpoint, so it always yields an empty catalog.
    pub async fn list_familiars(&self) -> Vec<crate::daemon_schema::DaemonFamiliar> {
        match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => {
                match unix_http::request(
                    socket,
                    "GET",
                    "/api/v1/familiars",
                    None,
                    self.config.request_timeout,
                )
                .await
                {
                    Ok(resp) => parse_familiars(resp.status, &resp.body),
                    Err(err) => {
                        log::debug!("cast_agent: GET /api/v1/familiars failed: {err}; empty catalog");
                        Vec::new()
                    }
                }
            }
            Transport::Tcp { .. } => Vec::new(),
        }
    }
```

- [ ] **Step 4: Run to verify it passes.**

Run: `cargo test -p cast_agent familiars_tests`
Expected: PASS (all three).

- [ ] **Step 5: Commit.**

```bash
git add crates/cast_agent/src/gateway.rs
git commit -S -m "feat(cast_agent): add graceful list_familiars() gateway call"
```

---

## Task 3: `RequestTarget`, `SUPPORTED_HARNESSES`, pure `resolve()`

**Files:**
- Create: `crates/cast_agent/src/familiar.rs`
- Modify: `crates/cast_agent/src/lib.rs`

- [ ] **Step 1: Declare the module.** In `lib.rs`, add after `pub mod daemon_schema;`:

```rust
pub mod familiar;
```

- [ ] **Step 2: Write the failing test.** Create `crates/cast_agent/src/familiar.rs`:

```rust
//! Familiar selection: the pure resolution from a chosen Familiar id (or
//! the configured default) to the concrete `RequestTarget` sent on the
//! daemon session request, plus a cached view of the daemon catalog.

use std::sync::{Arc, RwLock};

use crate::daemon_schema::DaemonFamiliar;
use crate::gateway::GatewayClient;

/// Backend harnesses offered when the daemon persona catalog is empty.
/// Single source of truth for the fallback list; extend here.
pub const SUPPORTED_HARNESSES: &[&str] = &["coven-code", "codex", "claude"];

/// What actually goes on the daemon session request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTarget {
    /// A daemon-catalog Familiar id, sent as `familiarId`. The daemon maps
    /// it to a harness + system prompt (daemon-owned; future contract).
    Familiar(String),
    /// A concrete backend harness, sent as `harness`.
    Harness(String),
}

/// Resolve a selection to a request target.
/// Precedence: explicit `selected` (per-conversation) > `default` > the
/// first `SUPPORTED_HARNESSES` entry. A selection that matches a catalog id
/// becomes `Familiar`; otherwise it is treated as a raw harness string.
pub fn resolve(
    selected: Option<&str>,
    default: Option<&str>,
    catalog: &[DaemonFamiliar],
) -> RequestTarget {
    let choice = selected
        .filter(|s| !s.is_empty())
        .or(default.filter(|s| !s.is_empty()));
    match choice {
        Some(id) if catalog.iter().any(|f| f.id == id) => RequestTarget::Familiar(id.to_string()),
        Some(other) => RequestTarget::Harness(other.to_string()),
        None => RequestTarget::Harness(SUPPORTED_HARNESSES[0].to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fam(id: &str) -> DaemonFamiliar {
        DaemonFamiliar {
            id: id.into(),
            name: id.into(),
            display_name: String::new(),
            emoji: String::new(),
            role: String::new(),
            description: String::new(),
            status: String::new(),
            last_seen: String::new(),
            active_sessions: 0,
            memory_freshness: String::new(),
        }
    }

    #[test]
    fn selected_catalog_id_resolves_to_familiar() {
        let cat = vec![fam("nova")];
        assert_eq!(resolve(Some("nova"), None, &cat), RequestTarget::Familiar("nova".into()));
    }

    #[test]
    fn selected_wins_over_default() {
        let cat = vec![fam("nova"), fam("sage")];
        assert_eq!(resolve(Some("sage"), Some("nova"), &cat), RequestTarget::Familiar("sage".into()));
    }

    #[test]
    fn default_used_when_no_selection() {
        let cat = vec![fam("nova")];
        assert_eq!(resolve(None, Some("nova"), &cat), RequestTarget::Familiar("nova".into()));
    }

    #[test]
    fn empty_catalog_falls_back_to_harness() {
        assert_eq!(resolve(None, None, &[]), RequestTarget::Harness("coven-code".into()));
    }

    #[test]
    fn non_catalog_selection_is_raw_harness() {
        assert_eq!(resolve(Some("codex"), None, &[]), RequestTarget::Harness("codex".into()));
    }
}
```

- [ ] **Step 3: Run to verify it passes.**

Run: `cargo test -p cast_agent familiar::`
Expected: all five PASS.

- [ ] **Step 4: Commit.**

```bash
git add crates/cast_agent/src/familiar.rs crates/cast_agent/src/lib.rs
git commit -S -m "feat(cast_agent): add RequestTarget + resolve() selection logic"
```

---

## Task 4: `FamiliarStore` cache (mirror `SessionStore`)

**Files:**
- Modify: `crates/cast_agent/src/familiar.rs`

- [ ] **Step 1: Add the cache type** (append to `familiar.rs`, after `resolve`):

```rust
/// Cached view of the daemon Familiar catalog. Mirrors `SessionStore`:
/// `list()` refreshes from the gateway (never errors — the gateway call is
/// already graceful), `snapshot()` is a cheap sync read for the UI thread.
pub struct FamiliarStore {
    gateway: Arc<GatewayClient>,
    cache: RwLock<Vec<DaemonFamiliar>>,
}

impl FamiliarStore {
    pub fn new(gateway: Arc<GatewayClient>) -> Self {
        Self { gateway, cache: RwLock::new(Vec::new()) }
    }

    /// Refresh the cached catalog from the gateway and return it.
    pub async fn list(&self) -> Vec<DaemonFamiliar> {
        let fetched = self.gateway.list_familiars().await;
        let mut guard = self.cache.write().unwrap_or_else(|p| p.into_inner());
        *guard = fetched.clone();
        fetched
    }

    /// Sync snapshot of the cached catalog. Safe from the UI thread.
    pub fn snapshot(&self) -> Vec<DaemonFamiliar> {
        self.cache.read().unwrap_or_else(|p| p.into_inner()).clone()
    }
}
```

- [ ] **Step 2: Verify it compiles** (behavior is covered by Task 2 + Task 3; the store is thin glue).

Run: `cargo test -p cast_agent familiar::`
Expected: PASS (existing tests still pass; new struct compiles).

- [ ] **Step 3: Commit.**

```bash
git add crates/cast_agent/src/familiar.rs
git commit -S -m "feat(cast_agent): add FamiliarStore catalog cache"
```

---

## Task 5: `default_familiar` config

**Files:**
- Modify: `crates/cast_agent/src/config.rs`

- [ ] **Step 1: Add the field** to `CastAgentConfig` (after `socket_path`):

```rust
    /// Global default Familiar id used for new conversations when the user
    /// has not picked one. `None` falls back to the first supported harness.
    pub default_familiar: Option<String>,
```

And in the `Default` impl add:

```rust
            default_familiar: None,
```

- [ ] **Step 2: Add it to the file-config struct.** Find the internal file-config struct that `load_file()` deserializes (it has `gateway_url`, `token`, `socket_path` as `Option`s). Add:

```rust
    default_familiar: Option<String>,
```

- [ ] **Step 3: Wire env + file** in `load()`. After the socket block, before `cfg` is returned, add:

```rust
        // 5. Default Familiar — env > file. Absent leaves `None` (harness fallback).
        cfg.default_familiar = std::env::var("COVEN_DEFAULT_FAMILIAR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file_cfg.default_familiar.filter(|v| !v.trim().is_empty()));
```

- [ ] **Step 4: Write a test** proving TOML parses the field. Add to `config.rs` `#[cfg(test)] mod tests` (create if absent):

```rust
    #[test]
    fn file_config_parses_default_familiar() {
        // Uses the same deserialize path as load_file(); adjust the struct
        // name if load_file deserializes a differently-named type.
        let toml = r#"default_familiar = "nova""#;
        let parsed: FileConfig = toml::from_str(toml).unwrap();
        assert_eq!(parsed.default_familiar.as_deref(), Some("nova"));
    }
```

(If the file-config type is private and unnameable from the test module, instead assert via `CastAgentConfig::default().default_familiar.is_none()` and cover the TOML path in a manual check; note which you did.)

- [ ] **Step 5: Run.**

Run: `cargo test -p cast_agent config`
Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add crates/cast_agent/src/config.rs
git commit -S -m "feat(cast_agent): add default_familiar config (env + toml)"
```

---

## Task 6: Route `RequestTarget` into the daemon request body

**Files:**
- Modify: `crates/cast_agent/src/gateway.rs` (read `familiarId`)
- Modify: `app/src/ai_assistant/panel.rs` (`coven_code_agent_message_body`)

- [ ] **Step 1: Gateway — forward `familiarId` when present.** In `send_message_via_daemon`, right after the existing `harness` resolution block, add:

```rust
        let familiar_id = msg
            .body
            .get("familiarId")
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .map(String::from);
```

Then extend the `launch_body` json to include it conditionally (after building `launch_body`):

```rust
        let mut launch_body = launch_body;
        if let Some(fid) = &familiar_id {
            launch_body["familiarId"] = serde_json::Value::String(fid.clone());
        }
```

(Leave the existing `harness` field as-is; the daemon uses `familiarId` when present, else `harness`.)

- [ ] **Step 2: Panel — build the body from a `RequestTarget`.** Replace `coven_code_agent_message_body` in `panel.rs` with a version that takes the resolved target:

```rust
#[cfg(feature = "cast-agent")]
fn coven_code_agent_message_body(
    prompt: String,
    cwd: Option<&std::path::Path>,
    target: &::ai::cast_agent::RequestTarget,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "text": prompt,
        "title": COVEN_CODE_OPERATION_TITLE,
    });
    match target {
        ::ai::cast_agent::RequestTarget::Familiar(id) => {
            body["familiarId"] = serde_json::Value::String(id.clone());
            // Also send coven-code as the harness so a daemon that doesn't
            // yet route familiarId still spawns the native agent.
            body["harness"] = serde_json::Value::String(COVEN_CODE_HARNESS.to_string());
        }
        ::ai::cast_agent::RequestTarget::Harness(h) => {
            body["harness"] = serde_json::Value::String(h.clone());
        }
    }
    if let Some(cwd) = cwd {
        body["projectRoot"] = serde_json::Value::String(cwd.to_string_lossy().to_string());
    }
    body
}
```

- [ ] **Step 3: Re-export the types** so `::ai::cast_agent::RequestTarget` resolves. In `crates/ai/src/lib.rs` (the cast_agent facade re-export section), ensure `RequestTarget` (and `DaemonFamiliar`, `FamiliarStore`) are re-exported alongside the existing `AgentMessage`/`CastAgentConfig` re-exports. Add to the `pub use cast_agent::{...}` list:

```rust
    familiar::{FamiliarStore, RequestTarget, SUPPORTED_HARNESSES},
    daemon_schema::DaemonFamiliar,
```

- [ ] **Step 4: Update the single caller.** In `send_via_coven_gateway_with_prompt`, replace the `let body = coven_code_agent_message_body(prompt, cwd.as_deref());` line with the resolved-target version (selection wiring lands in Task 7; for now pass the default fallback):

```rust
        let target = ::ai::cast_agent::RequestTarget::Harness(COVEN_CODE_HARNESS.to_string());
        let body = coven_code_agent_message_body(prompt, cwd.as_deref(), &target);
```

- [ ] **Step 5: Build both crates.**

Run: `cargo test -p cast_agent && cargo check -p warp --bin cast-codes --features gui,cast-agent`
Expected: cast_agent tests PASS; app compiles.

- [ ] **Step 6: Commit.**

```bash
git add crates/cast_agent/src/gateway.rs crates/ai/src/lib.rs app/src/ai_assistant/panel.rs
git commit -S -m "feat(agent): route RequestTarget (harness/familiarId) into daemon request"
```

---

## Task 7: Panel per-conversation selection + picker chip

**Files:**
- Modify: `app/src/ai_assistant/panel.rs`
- Modify: `crates/cast_agent/src/agent.rs` + `crates/cast_agent/src/runtime.rs` (expose catalog snapshot)

- [ ] **Step 1: Expose the catalog on the runtime.** In `agent.rs`, add a `FamiliarStore` to `CastAgent` (mirror `sessions`): add a `familiars: Arc<FamiliarStore>` field, construct it in `CastAgent::new` (`Arc::new(FamiliarStore::new(gateway.clone()))`), and add:

```rust
    /// Refresh + return the Familiar catalog (never errors; empty when the
    /// daemon has no catalog).
    pub async fn refresh_familiars(&self) -> Vec<crate::daemon_schema::DaemonFamiliar> {
        self.familiars.list().await
    }

    /// Sync snapshot of the cached Familiar catalog for the UI thread.
    pub fn familiars_snapshot(&self) -> Vec<crate::daemon_schema::DaemonFamiliar> {
        self.familiars.snapshot()
    }
```

In `runtime.rs`, add pass-throughs mirroring the existing `sessions()` method:

```rust
    pub fn familiars(&self) -> Vec<cast_agent::daemon_schema::DaemonFamiliar> {
        self.agent.familiars_snapshot()
    }
```

(and refresh the catalog on the same background loop that refreshes sessions — add a `refresh_familiars().await` call next to the existing `refresh_sessions().await`.)

- [ ] **Step 2: Add per-conversation state.** In `CovenStreamState`, add a field:

```rust
    /// Familiar id selected for this conversation. `None` = use the global
    /// default (config `default_familiar`), else the first supported harness.
    selected_familiar: Option<String>,
```

- [ ] **Step 3: Feed selection into the request.** In `send_via_coven_gateway_with_prompt`, replace the Task-6 placeholder `target` with a resolved one:

```rust
        let catalog = runtime.familiars();
        let default_familiar = runtime.agent().config().default_familiar.clone();
        let target = ::ai::cast_agent::resolve(
            self.coven_stream.selected_familiar.as_deref(),
            default_familiar.as_deref(),
            &catalog,
        );
        let body = coven_code_agent_message_body(prompt, cwd.as_deref(), &target);
```

Re-export `resolve` from `crates/ai/src/lib.rs` (`familiar::resolve`). Confirm `config()` is public on `CastAgent` (it is, per agent.rs).

- [ ] **Step 4: Render the picker chip.** Add a `render_familiar_pill` method next to `render_gateway_status_pill`, and call it in `render_title_bar` next to the status pill. The chip shows the current label and dispatches a cycle action:

```rust
    #[cfg(feature = "cast-agent")]
    fn current_familiar_label(&self) -> String {
        let runtime = ::ai::cast_agent::global();
        let catalog = runtime.as_ref().map(|r| r.familiars()).unwrap_or_default();
        let default_familiar = runtime
            .as_ref()
            .map(|r| r.agent().config().default_familiar.clone())
            .unwrap_or_default();
        match ::ai::cast_agent::resolve(
            self.coven_stream.selected_familiar.as_deref(),
            default_familiar.as_deref(),
            &catalog,
        ) {
            ::ai::cast_agent::RequestTarget::Familiar(id) => catalog
                .iter()
                .find(|f| f.id == id)
                .map(|f| format!("{} {}", f.emoji, f.label()))
                .unwrap_or(id),
            ::ai::cast_agent::RequestTarget::Harness(h) => h,
        }
    }
```

Render it as a small labeled `Container` (mirror `render_gateway_status_pill`'s builder style) wrapped in a `Button`/click handler dispatching a new `CycleFamiliar` action. Exact edit points:

- **Add the action variant.** In the same enum as the existing panel actions `AIAssistantAction::{ClosePanel, FocusTerminalInput, ResetContext}` (used at `panel.rs` lines ~268–291), add `CycleFamiliar`.
- **Handle it.** In the panel's action handler — the same `match`/dispatch site that handles `AIAssistantAction::ResetContext` — add an arm for `CycleFamiliar` that calls a new helper `self.cycle_familiar(ctx)` then `ctx.notify()`.
- **Implement `cycle_familiar`** with exact logic:

```rust
    #[cfg(feature = "cast-agent")]
    fn cycle_familiar(&mut self, _ctx: &mut ViewContext<Self>) {
        // Build the option list: catalog ids when non-empty, else the
        // supported harness fallbacks.
        let catalog = ::ai::cast_agent::global().map(|r| r.familiars()).unwrap_or_default();
        let options: Vec<String> = if catalog.is_empty() {
            ::ai::cast_agent::SUPPORTED_HARNESSES.iter().map(|s| s.to_string()).collect()
        } else {
            catalog.iter().map(|f| f.id.clone()).collect()
        };
        if options.is_empty() {
            return;
        }
        let current = self.coven_stream.selected_familiar.clone();
        let next_idx = current
            .as_ref()
            .and_then(|c| options.iter().position(|o| o == c))
            .map(|i| (i + 1) % options.len())
            .unwrap_or(0);
        self.coven_stream.selected_familiar = Some(options[next_idx].clone());
    }
```

> **Note (UI verification):** GPUI/warpui widget code is verified by compiling + running, not unit tests. Keep the chip minimal (label + cycle-on-click). A dropdown can replace the cycle later; do not block v1 on it.

- [ ] **Step 5: Build.**

Run: `cargo check -p warp --bin cast-codes --features gui,cast-agent`
Expected: compiles.

- [ ] **Step 6: Manual verify.** Run the app (see castcodes-dev-loop / `/run`). With no daemon running: the chip shows `coven-code`; clicking cycles through `codex`/`claude`. Confirm sending a prompt still works (offline notice when daemon down is unchanged).

- [ ] **Step 7: Commit.**

```bash
git add crates/cast_agent/src/agent.rs crates/cast_agent/src/runtime.rs crates/ai/src/lib.rs app/src/ai_assistant/panel.rs
git commit -S -m "feat(agent): per-conversation Familiar picker in the agent panel"
```

---

## Task 8: Settings — read-only Familiars section

**Files:**
- Modify: `app/src/settings_view/ai_page.rs`

- [ ] **Step 1: Add a section renderer** on the agent page. Add `render_familiars_section` mirroring `render_next_command_section`'s `Flex::column()` shape. It lists the cached catalog (emoji + label + role + status) read from `::ai::cast_agent::global().map(|r| r.familiars())`, and — when empty — renders a single description line: `"No Familiars available. Start the Coven daemon to load the persona catalog."`. Include a short header via the existing description helper (`render_ai_setting_description` or the section-title helper used elsewhere on the page).

```rust
    #[cfg(feature = "cast-agent")]
    fn render_familiars_section(
        &self,
        app: &warpui::AppContext,
    ) -> Box<dyn warpui::Element> {
        let catalog = ::ai::cast_agent::global()
            .map(|r| r.familiars())
            .unwrap_or_default();
        let mut col = Flex::column();
        // Title + explanation (reuse the page's description helper).
        col = col.with_child(render_ai_setting_description(
            "Familiars are personas served by the Coven daemon. Selection happens per conversation in the agent panel.",
            true,
            app,
        ));
        if catalog.is_empty() {
            col = col.with_child(render_ai_setting_description(
                "No Familiars available. Start the Coven daemon to load the persona catalog.",
                true,
                app,
            ));
        } else {
            for f in &catalog {
                col = col.with_child(render_ai_setting_description(
                    &format!("{} {} — {} ({})", f.emoji, f.label(), f.role, f.status),
                    true,
                    app,
                ));
            }
        }
        col.finish()
    }
```

- [ ] **Step 2: Mount it** in the agent page render (the `AISubpage::WarpAgent` branch), appended after the existing sections with the same `.with_child(self.render_familiars_section(app))` pattern. Guard the call with `#[cfg(feature = "cast-agent")]` (and provide a no-op/skip on the non-feature build).

- [ ] **Step 3: Build.**

Run: `cargo check -p warp --bin cast-codes --features gui,cast-agent`
Expected: compiles.

- [ ] **Step 4: Manual verify.** Open Settings → the agent page; with no daemon it shows the "No Familiars available" line.

- [ ] **Step 5: Commit.**

```bash
git add app/src/settings_view/ai_page.rs
git commit -S -m "feat(settings): read-only Familiars catalog section on the agent page"
```

---

## Task 9: Final verification + PR

- [ ] **Step 1: Full crate tests.**

Run: `cargo test -p cast_agent`
Expected: all PASS.

- [ ] **Step 2: Clippy (matches CI).**

Run: `cargo clippy -p cast_agent -p warp --all-targets --features gui,cast-agent -- -D warnings`
Expected: no warnings. Fix any (watch for `instant::Instant` rule, collapsible-if, unused).

- [ ] **Step 3: Rebrand + attribution guards.**

Run: `./script/check_rebrand && ./script/check_ai_attribution`
Expected: both pass.

- [ ] **Step 4: Rebase onto latest main** (concurrent cast_agent work).

```bash
git fetch origin main
git rebase origin/main
# re-run Steps 1-3 after rebase; resolve any cast_agent conflicts
```

- [ ] **Step 5: Signed-commit sanity check + push.**

```bash
git log origin/main..HEAD --pretty='%H %G?' | awk '$2 != "G" {print "UNSIGNED:", $0}'   # must print nothing
git push -u origin feat/familiars-multi-harness
```

- [ ] **Step 6: Open PR** with the `create-pr` skill (title `feat(agent): Familiars multi-harness selection`), linking the design spec. Note in the body that rich personas are inert until the daemon ships `/api/v1/familiars`, and that the harness fallback keeps the picker useful meanwhile.

---

## Notes / Risks (carried from the spec)

- **Daemon dependency:** `list_familiars()` returns empty until the Coven daemon serves `/api/v1/familiars`; the picker shows `SUPPORTED_HARNESSES` and everything works as today. This is expected, not a bug.
- **PLAN-01 overlap:** `DaemonFamiliar` / `list_familiars()` / a centralized harness list also appear in `specs/castcodes-coven-internal-loop/PLAN-01-start-coding.md`. If that lands first, delete the duplicate definitions here and consume its `daemon_schema.rs`/`lane.rs` instead. Coordinate before Task 1.
- **Concurrent edits:** `gateway.rs`, `config.rs`, `panel.rs` are hot. Rebase before PR (Task 9 Step 4) and keep commits small so conflicts are cheap.
- **Per-conversation persistence deferred (v1 scope).** The spec's persistence section mentions writing `selected_familiar` alongside the conversation in stream-history. This plan keeps `selected_familiar` **in-memory** on `CovenStreamState` only — the panel has a single active conversation and the global `default_familiar` (config, persisted) covers restart behavior. Persisting per-conversation selection into `stream-history.json` is deliberately deferred; add it in a follow-up if history entries need to restore their Familiar. This is the one intentional divergence from the spec.
