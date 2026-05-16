# CastCodes Browser Panel — Plan 3: Agent Surface (in-process)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans`. Read [`PRODUCT.md`](./PRODUCT.md) and [`TECH.md`](./TECH.md) first.
>
> **Branch base:** Stacked on PLAN-02. Branch from `feat/browser-panel-security` (PR #35). Once #35 merges, rebase onto `main`.
>
> **Signing rule:** Every `git commit` MUST pass `-S`. Verify with `git log -1 --show-signature | head -3`.

**Goal:** Give the in-process CastCodes agent (and PLAN-04's MCP server) a typed Rust interface for driving the embedded browser pane — navigate, reload, list tabs, evaluate JS in the active tab. Foundation for the richer capabilities (screenshot, element picker, console + network capture) that need a wry upgrade.

**Architecture:** A new `app/src/browser/agent_api.rs` module exposes a small `BrowserAgent` handle. Each method takes a `&AppContext`, looks up the active `BrowserView` (via the workspace's active pane group), and either reads from the model or dispatches a `BrowserViewAction`. JS evaluation uses wry's `evaluate_script_with_callback` plumbed through a oneshot channel so callers see a `Future<Output = serde_json::Value>`.

**Wry 0.38 constraint:** Screenshot, request-level network interception, and clean element-pick injection are unavailable or fragile in this version. Documented as deferred work below.

**Tech Stack:** Rust, `wry` 0.38, `serde_json`, `async_channel` / `futures::channel::oneshot` (existing deps), `warpui` action dispatch.

---

## Files created or modified

**Created:**

| Path | Responsibility |
|---|---|
| `app/src/browser/agent_api.rs` | `BrowserAgent` handle + typed methods. |

**Modified:**

| Path | Change |
|---|---|
| `app/src/browser/mod.rs` | Add `pub(crate) mod agent_api;` and `pub use agent_api::BrowserAgent;` |
| `app/src/browser/browser_view.rs` | Add `BrowserViewAction::Navigate { url: String }`, `Reload`-with-target, and `EvaluateJs { script: String, reply_tx: oneshot::Sender<serde_json::Value> }`. Methods to look up the active webview by `TabId` for targeted ops. Public `model_snapshot_tabs()` helper used by `list_tabs`. |
| `app/src/browser/webview_host.rs` | Add `evaluate_js_async(script, reply_tx)` using wry's `evaluate_script_with_callback`. Replies are JSON-decoded; non-JSON returns are wrapped as `serde_json::Value::String`. |

---

## Phase 1 — Plan + module skeleton

### Task 1.1: Commit PLAN-03 doc

- [ ] Commit this file:
```bash
git add specs/CASTCODES-BROWSER-PANEL/PLAN-03-agent-surface.md
git commit -S -m "docs(specs): PLAN-03 (agent surface)"
git log -1 --show-signature | head -3
```

### Task 1.2: Create `agent_api.rs` skeleton

**Files:**
- Create: `app/src/browser/agent_api.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] Create the module with the public API surface — methods stubbed `unimplemented!()`. Compile + commit so subsequent phases can fill in bodies without each one rebuilding the public interface.

```rust
//! Typed Rust interface for driving the embedded browser pane.
//!
//! The in-process CastCodes agent uses this directly; PLAN-04's MCP
//! server wraps the same surface for out-of-process agents.

use serde::Serialize;
use warpui::AppContext;

use super::browser_model::TabId;

#[derive(Debug, Clone, Serialize)]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub active: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserAgentError {
    #[error("no browser pane is currently open")]
    NoPaneOpen,
    #[error("requested tab id {0} does not exist")]
    UnknownTab(TabId),
    #[error("evaluate_script failed: {0}")]
    EvalFailed(String),
}

pub struct BrowserAgent;

impl BrowserAgent {
    /// Returns one snapshot per open tab in the active browser pane.
    /// Empty list if no pane is open.
    pub fn list_tabs(_ctx: &AppContext) -> Vec<TabInfo> {
        unimplemented!()
    }

    /// Navigates the active tab to `url`. Returns `NoPaneOpen` if no
    /// pane exists; opens a fresh pane is the caller's responsibility.
    pub fn navigate(_ctx: &mut AppContext, _url: String) -> Result<(), BrowserAgentError> {
        unimplemented!()
    }

    /// Reloads the active tab.
    pub fn reload(_ctx: &mut AppContext) -> Result<(), BrowserAgentError> {
        unimplemented!()
    }

    /// Opens a new tab at the supplied URL and makes it active.
    pub fn new_tab(_ctx: &mut AppContext, _url: String) -> Result<TabId, BrowserAgentError> {
        unimplemented!()
    }

    /// Evaluates `script` in the active tab. Returns the JSON-decoded
    /// result; non-JSON returns are wrapped as a `Value::String`.
    pub async fn evaluate_js(
        _ctx: &AppContext,
        _script: String,
    ) -> Result<serde_json::Value, BrowserAgentError> {
        unimplemented!()
    }
}
```

Add `pub(crate) mod agent_api; pub use agent_api::{BrowserAgent, BrowserAgentError, TabInfo};` to `mod.rs`.

Build + commit:
```bash
cargo check -p warp-app --bin cast-codes --features gui
git add app/src/browser/agent_api.rs app/src/browser/mod.rs
git commit -S -m "feat(browser): agent_api skeleton — types + stubbed methods"
```

---

## Phase 2 — list_tabs (read-only)

### Task 2.1: Implement `BrowserAgent::list_tabs`

**Files:**
- Modify: `app/src/browser/agent_api.rs`
- Possibly: `app/src/workspace/view.rs` (add `Workspace::active_browser_view()` accessor if missing)

- [ ] Find the active `BrowserView` from the workspace's active pane group. Reuse the pattern already in `Workspace::save_browser_state` (`PaneGroup::pane_ids().find(|id| id.is_browser_pane())` → `downcast_pane_by_id::<BrowserPane>` → `.browser_view(ctx)`).

- [ ] Implement:

```rust
pub fn list_tabs(ctx: &AppContext) -> Vec<TabInfo> {
    let Some(view) = active_browser_view(ctx) else { return vec![]; };
    let view_ref = view.as_ref(ctx);
    let model = view_ref.model();
    model.tabs().iter().enumerate().map(|(idx, tab)| TabInfo {
        id: tab.id(),
        url: tab.current_url().to_string(),
        title: tab.display_title().to_string(),
        loading: tab.is_loading(),
        active: idx == model.active_index(),
    }).collect()
}

fn active_browser_view(ctx: &AppContext) -> Option<warpui::ViewHandle<super::BrowserView>> {
    let workspaces = warpui_extras::singleton::WorkspaceRegistry::get(ctx)?;
    // ... resolve active workspace handle, lookup browser pane, return browser_view
    None
}
```

The exact accessor for the active workspace handle from a `&AppContext` is local — search `Workspace::current(ctx)` or `WorkspaceRegistry::active(ctx)`. Use whichever the codebase already uses for global-context workspace lookups (e.g. how the chat panel finds its workspace from a global action handler).

Unit test:

```rust
#[cfg(test)]
mod tests {
    // List-tabs is exercised end-to-end by integration tests in
    // PLAN-01's Phase 10 follow-up; pure-Rust testing of this function
    // requires either mocking the workspace or running the warpui test
    // harness. Skip for v1.
}
```

Commit:
```bash
git add app/src/browser/agent_api.rs
git commit -S -m "feat(agent_api): implement list_tabs"
```

### Task 2.2: navigate / reload / new_tab

Same shape as `list_tabs` but instead of reading from the model, dispatch the existing `BrowserViewAction` for the matching operation via the view handle's `update(ctx, |v, ctx| v.handle_action(...))` or a direct method call.

```rust
pub fn navigate(ctx: &mut AppContext, url: String) -> Result<(), BrowserAgentError> {
    let view = active_browser_view(ctx).ok_or(BrowserAgentError::NoPaneOpen)?;
    view.update(ctx, |v, ctx| v.navigate(url, ctx));
    Ok(())
}
```

`BrowserView::navigate` is already `pub(crate)` from PLAN-01. Same for `reload`.

For `new_tab`:

```rust
pub fn new_tab(ctx: &mut AppContext, url: String) -> Result<TabId, BrowserAgentError> {
    let view = active_browser_view(ctx).ok_or(BrowserAgentError::NoPaneOpen)?;
    let id = view.update(ctx, |v, ctx| {
        v.new_tab_at(url, ctx);
        v.model().active_tab().id()
    });
    Ok(id)
}
```

Build + commit:
```bash
cargo check -p warp-app --bin cast-codes --features gui
git add app/src/browser/agent_api.rs app/src/browser/browser_view.rs
git commit -S -m "feat(agent_api): navigate / reload / new_tab"
```

---

## Phase 3 — evaluate_js with async result

### Task 3.1: Plumb `evaluate_script_with_callback`

**Files:**
- Modify: `app/src/browser/webview_host.rs`
- Modify: `app/src/browser/browser_view.rs`
- Modify: `app/src/browser/agent_api.rs`

- [ ] In `webview_host.rs`, add a method that wraps wry:

```rust
pub(crate) fn evaluate_js_async(
    &self,
    script: &str,
    reply: futures::channel::oneshot::Sender<Result<serde_json::Value, String>>,
) {
    #[cfg(not(target_family = "wasm"))]
    if let Some(webview) = &self.webview {
        let cell = std::cell::RefCell::new(Some(reply));
        if let Err(err) = webview.evaluate_script_with_callback(script, move |raw: String| {
            let val = serde_json::from_str::<serde_json::Value>(&raw)
                .unwrap_or_else(|_| serde_json::Value::String(raw));
            if let Some(tx) = cell.borrow_mut().take() {
                let _ = tx.send(Ok(val));
            }
        }) {
            // Best-effort: the closure may never fire if the call itself failed.
            log::warn!("evaluate_script_with_callback returned error: {err}");
        }
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = (script,);
        let _ = reply.send(Err("not supported on wasm".to_string()));
    }
}
```

- [ ] In `browser_view.rs`, add a public method that finds the active webview and forwards to `evaluate_js_async`:

```rust
pub(crate) fn evaluate_js_on_active_tab(
    &self,
    script: &str,
    reply: futures::channel::oneshot::Sender<Result<serde_json::Value, String>>,
) {
    if let Some(webview) = self.active_webview() {
        webview.borrow().evaluate_js_async(script, reply);
    } else {
        let _ = reply.send(Err("no active webview".into()));
    }
}
```

- [ ] In `agent_api.rs`, build the async wrapper:

```rust
pub async fn evaluate_js(
    ctx: &AppContext,
    script: String,
) -> Result<serde_json::Value, BrowserAgentError> {
    let view = active_browser_view(ctx).ok_or(BrowserAgentError::NoPaneOpen)?;
    let (tx, rx) = futures::channel::oneshot::channel();
    view.as_ref(ctx).evaluate_js_on_active_tab(&script, tx);
    match rx.await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(BrowserAgentError::EvalFailed(err)),
        Err(_canceled) => Err(BrowserAgentError::EvalFailed("dropped".into())),
    }
}
```

Build + commit:
```bash
cargo check -p warp-app --bin cast-codes --features gui
git add app/src/browser/agent_api.rs app/src/browser/browser_view.rs app/src/browser/webview_host.rs
git commit -S -m "feat(agent_api): async evaluate_js via evaluate_script_with_callback"
```

---

## Phase 4 — PR

- [ ] `cargo test -p warp-app browser::` — all passing.
- [ ] `./script/check_rebrand` — passes.
- [ ] `git log origin/main..HEAD --pretty='%h %G?' | awk '$2 != "G" {print "UNSIGNED:", $0}'` — empty.
- [ ] Push + open PR against `feat/browser-panel-security`.

PR description must call out the deferred capabilities and link the wry-upgrade follow-up.

---

## Self-Review

**Spec coverage:**

| TECH spec § 8 capability | Plan coverage |
|---|---|
| `list_tabs` | Phase 2.1 ✅ |
| `navigate` / `reload` / `new_tab` | Phase 2.2 ✅ |
| `evaluate_js` | Phase 3.1 ✅ |
| `screenshot` | DEFERRED — wry 0.38 has no screenshot API. Follow-up paired with wry upgrade. |
| `pick_element` | DEFERRED — JS overlay injection + selector generation is moderate complexity; needs careful design to avoid conflicting with page scripts. |
| `capture_console` (subscribe + tail) | DEFERRED — JS `console.*` monkey-patching + per-tab ring buffer; ships with element picker. |
| `capture_network` (subscribe + tail) | DEFERRED — JS `fetch`/`XHR` wrapping; partial coverage anyway since wry doesn't expose subresources. |

**Placeholder scan:** No "TBD" — every code block is concrete. Two areas marked as "search the codebase for the existing pattern" (active workspace accessor in Task 2.1; `BrowserView::navigate` visibility in Task 2.2) are unavoidable; both are single-grep resolvable.

**Type consistency:** `BrowserAgent`, `BrowserAgentError`, `TabInfo`, `TabId` are referenced consistently. `active_browser_view(ctx)` is the single internal helper used by every public method.
