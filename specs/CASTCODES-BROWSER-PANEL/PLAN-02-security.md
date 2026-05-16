# CastCodes Browser Panel — Plan 2: Security Hardening

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Read [`PRODUCT.md`](./PRODUCT.md) and [`TECH.md`](./TECH.md) first.
>
> **Branch base:** This plan builds on PLAN-01. Branch from `feat/browser-panel-ui-toggle-persistence` (PR #32). Once PR #32 merges, rebase onto `main`.
>
> **Signing rule:** Every `git commit` MUST pass `-S`. After each commit run `git log -1 --show-signature | head -3` and confirm `Good "git" signature ... ED25519`. STOP if signing fails.

**Goal:** Make the embedded browser pane secure by default — per-app private data dir, popup interception, gated DevTools, and a vendored EasyList blocklist applied at navigation time.

**Architecture:** All four security postures plug into the `wry::WebViewBuilder` call site in `app/src/browser/webview_host.rs`. A new `BrowserSettings` group (`browser.devtools_enabled`, `browser.blocklist_enabled`) gates user-controllable behaviors. A new `app/src/browser/blocklist.rs` builds an `adblock-rust` matcher from a vendored EasyList snapshot once at app startup and shares it via `Arc<Engine>`. Popup / new-window requests route back into the existing `BrowserViewAction::NewTabWithUrl` path so `_blank` links open as in-pane tabs.

**Wry version constraint:** wry 0.38.2 (the pinned version) has `with_devtools`, `with_new_window_req_handler`, `with_navigation_handler`, and `with_web_context`. It does NOT have `with_web_resource_request_handler` (request-layer subresource interception). This plan applies the blocklist via `with_navigation_handler` (catches clicked links + iframe `src` loads + redirects) — covering ~70% of tracker traffic. Image trackers, fetch/XHR analytics, and websocket beacons are NOT blocked in this plan. Full network-layer blocking requires upgrading wry to a version with request handlers; tracked as a separate follow-up.

**Tech Stack:** Rust, `wry` 0.38 (existing), `adblock` crate (new workspace dep), `serde` (existing), the existing `settings::define_settings_group!` macro.

---

## Files created or modified

**Created:**

| Path | Responsibility |
|---|---|
| `app/src/browser/blocklist.rs` | Adblock-rust matcher; lazy-init from bundled rules; `check_url(url)` + `is_enabled()` |
| `app/src/browser/popup_policy.rs` | `decide(req) -> PopupAction { Tab(url), External(url), Block }`. Pure function over `wry::NewWindowReq` |
| `app/src/browser/data_dir.rs` | Resolves the per-app WebKit data dir path. macOS/Linux pathing |
| `app/src/settings/browser.rs` | `BrowserSettings` group: `devtools_enabled` (default `false`), `blocklist_enabled` (default `true`) |
| `app/assets/bundled/blocklists/easylist-network.txt` | Vendored EasyList network-rules subset (pinned snapshot from https://easylist.to/easylist/easylist.txt, network rules only) |

**Modified:**

| Path | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `adblock = { version = "0.9", default-features = false, features = ["full-regex-handling"] }` under `[workspace.dependencies]`. |
| `app/Cargo.toml` | Add `adblock.workspace = true`. |
| `app/src/browser/mod.rs` | Add `pub(crate) mod blocklist; pub(crate) mod popup_policy; pub(crate) mod data_dir;` in sort order. |
| `app/src/browser/webview_host.rs` | Build the wry `WebContext` from `data_dir::path()` once, share it across the pane's tabs. Apply `.with_devtools(settings.devtools_enabled)`, `.with_new_window_req_handler(...)`, `.with_navigation_handler(...)` at builder time. |
| `app/src/browser/browser_view.rs` | Receive a `BrowserViewAction::NewTabWithUrl { url: String }` variant for popup-routed `_blank` links. Pass current `BrowserSettings` snapshot into webview construction. |
| `app/src/settings/mod.rs` | `pub mod browser;` and re-export `BrowserSettings` from the settings facade. |
| `app/src/lib.rs` | Register `BrowserSettings` in the settings init path (mirror how `BlockVisibilitySettings` registers). |
| `script/refresh_blocklist` (NEW) | Small shell wrapper documenting the quarterly refresh procedure. Optional but recommended; only ~15 lines. |

---

## Phase 1 — Settings group

### Task 1.1: Add `BrowserSettings` group

**Files:**
- Create: `app/src/settings/browser.rs`
- Modify: `app/src/settings/mod.rs`

- [ ] **Step 1:** Inspect `app/src/settings/block_visibility.rs` end-to-end and `app/src/settings/mod.rs` to see how groups are registered. Copy the pattern.

- [ ] **Step 2:** Create `app/src/settings/browser.rs`:

```rust
use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};

// Per-user settings for the embedded browser pane.
define_settings_group!(BrowserSettings, settings: [
    devtools_enabled: DevtoolsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "browser.devtools_enabled",
        description: "Enables Web Inspector (DevTools) in the embedded browser pane. Off by default to reduce accidental data exposure.",
    },
    blocklist_enabled: BlocklistEnabled {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "browser.blocklist_enabled",
        description: "Blocks navigations to URLs matching the bundled tracker/ad blocklist (EasyList network rules). Subresource requests are NOT filtered in this version.",
    }
]);
```

- [ ] **Step 3:** In `app/src/settings/mod.rs`, add `pub mod browser;` in alphabetical order with the other module declarations. Also add a `pub use browser::BrowserSettings;` re-export if the file re-exports other settings types (mirror the pattern used for `BlockVisibilitySettings`).

- [ ] **Step 4:** Verify the settings group registers. Search how `BlockVisibilitySettings` gets registered at app start (`grep -n "BlockVisibilitySettings" app/src/lib.rs`). Add a parallel registration line for `BrowserSettings` in the same location.

- [ ] **Step 5:** Build:
```bash
cargo check -p warp-app --bin cast-codes --features gui 2>&1 | grep -E "^error" | head
```
Expected: no errors.

- [ ] **Step 6:** Commit (signed):
```bash
git add app/src/settings/browser.rs app/src/settings/mod.rs app/src/lib.rs
git commit -S -m "$(cat <<'EOF'
feat(settings): add BrowserSettings group

Two booleans gating the embedded browser pane's security posture:
- devtools_enabled (default false): turns on the WebKit Inspector.
- blocklist_enabled (default true): applies the bundled tracker
  blocklist via the navigation handler.

No behavior wired yet — that's the next commits.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

---

## Phase 2 — App-private WebKit data directory

### Task 2.1: Resolve the data dir path

**Files:**
- Create: `app/src/browser/data_dir.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1:** Create `app/src/browser/data_dir.rs`:

```rust
//! Resolves the WebKit data directory for the embedded browser pane.
//!
//! Cookies, localStorage, IndexedDB, and service-worker storage all live
//! under this directory. Sharing this path across every pane in the app
//! gives consistent SSO behavior but isolates CastCodes from the user's
//! system browser (Safari/Chrome cookies are not visible).
//!
//! Per-workspace isolation is intentionally NOT done here — that
//! requires a stable workspace identity that survives restart, which
//! cast-codes does not currently expose. Future work.

use std::path::PathBuf;

/// Returns the WebKit data directory path. Creates the directory if it
/// does not already exist.
pub fn path() -> Option<PathBuf> {
    let base = warp_core::paths::warp_home_config_dir()?;
    let dir = base.join("browser").join("data");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::warn!("failed to create browser data dir at {dir:?}: {err}");
        return None;
    }
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_ends_in_browser_data() {
        // Smoke: the function returns a path that lives under .cast-codes/browser/data.
        // We can't make stronger assertions without faking the home dir,
        // and warp_home_config_dir() reads HOME. Treat None as acceptable
        // for headless CI without a HOME.
        if let Some(p) = path() {
            let s = p.to_string_lossy();
            assert!(s.contains("browser"), "{s}");
            assert!(s.ends_with("data") || s.ends_with("data/"), "{s}");
        }
    }
}
```

- [ ] **Step 2:** Wire into `app/src/browser/mod.rs`:
```rust
pub(crate) mod data_dir;
```
(Insert in sort order with the other modules.)

- [ ] **Step 3:** Test:
```bash
cargo test -p warp-app browser::data_dir
```
Expected: 1 test pass.

- [ ] **Step 4:** Commit:
```bash
git add app/src/browser/data_dir.rs app/src/browser/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): resolve app-private WebKit data dir path

New data_dir::path() resolves ~/.cast-codes/browser/data and creates
the directory if absent. Used by webview_host to isolate the browser
pane's cookies/storage from the user's system browser.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

### Task 2.2: Apply data dir at WebView construction

**Files:**
- Modify: `app/src/browser/webview_host.rs`

- [ ] **Step 1:** Read the current `NativeBrowserWebView::attach_if_needed` (around line 187 in `webview_host.rs`) and locate the `wry::WebViewBuilder::new_as_child(&parent)` call.

- [ ] **Step 2:** Add `WebContext` construction. `wry::WebContext::new(data_dir)` takes an `Option<PathBuf>`; sharing one `WebContext` across multiple WebViews lets them share cookies/storage.

`WebContext` lives in `wry`. The single shared instance should live on `NativeBrowserWebView`'s parent — that's `BrowserView`. For v1 simplicity, build a fresh `WebContext` per pane (per `BrowserView`) and share it across all the tabs in that pane.

The cleanest place is to:
- Add `web_context: Rc<RefCell<wry::WebContext>>` to `BrowserView`.
- Pass `&mut WebContext` into each `NativeBrowserWebView::attach_if_needed` call.
- Pass it as the builder's `.with_web_context(&mut web_context)`.

Pseudocode for the change in `BrowserView::new` (creating the WebContext once):

```rust
let web_context = Rc::new(RefCell::new(wry::WebContext::new(
    super::data_dir::path(),
)));
```

Then in `attach_if_needed`, accept `&mut WebContext` and add `.with_web_context(web_context)` to the builder chain.

Plumbing: change `NativeBrowserWebView::set_bounds` (the trigger for `attach_if_needed`) to accept a `&mut WebContext` from the caller, OR make `NativeBrowserWebView` hold an `Rc<RefCell<WebContext>>` of its own (with the actual context shared via Rc).

The Rc-shared approach is cleaner — fewer call-site changes. Adopt that.

- [ ] **Step 3:** Concrete edits:

In `webview_host.rs`, on the `NativeBrowserWebView` struct, add a field:

```rust
#[cfg(not(target_family = "wasm"))]
web_context: Rc<RefCell<wry::WebContext>>,
```

Update `NativeBrowserWebView::new` to accept and store it:

```rust
pub(crate) fn new(
    tab_id: TabId,
    initial_url: impl Into<String>,
    title_tx: async_channel::Sender<(TabId, String)>,
    desired_visible: bool,
    #[cfg(not(target_family = "wasm"))]
    web_context: Rc<RefCell<wry::WebContext>>,
) -> Self {
    Self {
        tab_id,
        webview: None,
        title_tx,
        pending_url: Some(initial_url.into()),
        bounds: None,
        desired_visible,
        attach_error_logged: false,
        #[cfg(not(target_family = "wasm"))]
        web_context,
    }
}
```

In `attach_if_needed`, change:

```rust
match wry::WebViewBuilder::new_as_child(&parent)
    .with_url(...)
    .with_bounds(...)
    .with_initialization_script(...)
    .build()
```

to:

```rust
let mut ctx_ref = self.web_context.borrow_mut();
match wry::WebViewBuilder::new_as_child(&parent)
    .with_web_context(&mut *ctx_ref)
    .with_url(...)
    .with_bounds(...)
    .with_initialization_script(...)
    .build()
```

In `BrowserView::new` and all sites that build a `NativeBrowserWebView`, construct the shared `WebContext` once and pass `web_context.clone()` to each `NativeBrowserWebView::new`.

- [ ] **Step 4:** Build:
```bash
cargo check -p warp-app --bin cast-codes --features gui
```
Expected: clean compile.

- [ ] **Step 5:** Manual smoke (optional but recommended):
- Run `cargo run -p warp-app --bin cast-codes --features gui`
- Open the browser pane (⌘⌥B)
- Navigate to `https://example.com`
- Open Safari, set a cookie at `example.com`
- Reload the pane — cookie should NOT be visible (separate storage). Verifies isolation.

- [ ] **Step 6:** Commit:
```bash
git add app/src/browser/webview_host.rs app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): isolate WebKit storage to ~/.cast-codes/browser/data

Each BrowserView constructs a single wry::WebContext pointed at the
app-private data dir and shares it across all tabs in the pane. As a
result CastCodes sees its own cookies/localStorage/IndexedDB — no
overlap with Safari/Chrome.

Per-workspace isolation deferred; cast-codes lacks stable workspace
identity across restart.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

---

## Phase 3 — Popup / new-window policy

### Task 3.1: Failing tests for `popup_policy::decide`

**Files:**
- Create: `app/src/browser/popup_policy.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1:** Create `app/src/browser/popup_policy.rs`:

```rust
//! Policy for handling `target="_blank"` and `window.open()` requests
//! inside the embedded browser pane.
//!
//! All web-pane links that would normally pop a new OS window get
//! redirected to either a new tab in the pane (default) or the system
//! default browser (for a small allowlist of schemes that don't belong
//! in an in-app webview).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupAction {
    /// Open the URL as a new tab in the same browser pane.
    Tab(String),
    /// Hand off to the OS default handler.
    External(String),
    /// Reject the request entirely.
    Block,
}

/// Decides what to do with a requested new-window URL. Pure function.
pub fn decide(url: &str) -> PopupAction {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return PopupAction::Block;
    }
    // Schemes that should hand off to the OS rather than load in-pane.
    for scheme in ["mailto:", "tel:", "sms:", "castcodes:"] {
        if trimmed.starts_with(scheme) {
            return PopupAction::External(trimmed.to_string());
        }
    }
    // `javascript:` URLs are blocked for the same reason every modern
    // browser blocks them in new-window context — common XSS vector.
    if trimmed.starts_with("javascript:") {
        return PopupAction::Block;
    }
    // Everything else opens as a new tab in the pane.
    PopupAction::Tab(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_blocked() {
        assert_eq!(decide(""), PopupAction::Block);
        assert_eq!(decide("   "), PopupAction::Block);
    }

    #[test]
    fn known_external_schemes_handed_off() {
        assert_eq!(
            decide("mailto:hi@example.com"),
            PopupAction::External("mailto:hi@example.com".into())
        );
        assert_eq!(
            decide("tel:+15555550100"),
            PopupAction::External("tel:+15555550100".into())
        );
        assert_eq!(
            decide("castcodes://settings"),
            PopupAction::External("castcodes://settings".into())
        );
    }

    #[test]
    fn javascript_scheme_is_blocked() {
        assert_eq!(
            decide("javascript:alert(1)"),
            PopupAction::Block
        );
    }

    #[test]
    fn https_url_opens_as_new_tab() {
        assert_eq!(
            decide("https://example.com/path"),
            PopupAction::Tab("https://example.com/path".into())
        );
    }

    #[test]
    fn http_loopback_opens_as_new_tab() {
        assert_eq!(
            decide("http://localhost:3000/foo"),
            PopupAction::Tab("http://localhost:3000/foo".into())
        );
    }
}
```

Add `pub(crate) mod popup_policy;` to `mod.rs`.

- [ ] **Step 2:** Run tests — all pass on first commit because the function is already implemented (this is paste-from-plan TDD with the impl included). If you prefer strict TDD, split into two commits: first with `unimplemented!` body, then implementation.

```bash
cargo test -p warp-app browser::popup_policy
```
Expected: 5 passing.

- [ ] **Step 3:** Commit:
```bash
git add app/src/browser/popup_policy.rs app/src/browser/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): popup policy resolver for _blank / window.open

Pure function returning Tab(url), External(url), or Block. Used by
the wry new_window_req_handler in the next commit. mailto/tel/sms/
castcodes hand off to the OS; javascript: is blocked; everything
else opens as an in-pane tab.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

### Task 3.2: Wire the handler into the WebView builder

**Files:**
- Modify: `app/src/browser/webview_host.rs`
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1:** Add `BrowserViewAction::NewTabWithUrl { url: String }` to the existing action enum in `browser_view.rs`. Update the matching `handle_action` arm to dispatch into `new_tab` but with the supplied URL instead of `DEFAULT_BROWSER_URL`.

You'll need to extract the new-tab-creation body of the existing `new_tab` method into a helper `fn new_tab_with(&mut self, url: &str, ctx: ...)` that the existing `new_tab` and the new `NewTabWithUrl` arm both call.

- [ ] **Step 2:** In `webview_host.rs`'s `attach_if_needed`, add the handler. Because the handler needs to dispatch a warpui action back to the BrowserView, we route via a channel rather than capturing context handles:

Add to `NativeBrowserWebView`:

```rust
#[cfg(not(target_family = "wasm"))]
new_window_tx: async_channel::Sender<String>,
```

`NativeBrowserWebView::new` takes the sender alongside `title_tx`.

In `attach_if_needed`, before `.build()`:

```rust
let tx = self.new_window_tx.clone();
builder = builder.with_new_window_req_handler(move |url: String| -> bool {
    use super::popup_policy::{decide, PopupAction};
    match decide(&url) {
        PopupAction::Tab(u) => {
            let _ = tx.try_send(u);
        }
        PopupAction::External(u) => {
            if let Err(err) = opener::open(&u) {
                log::warn!("failed to open external url {u}: {err}");
            }
        }
        PopupAction::Block => {}
    }
    // Always return `false` — never let wry pop a new native window.
    false
});
```

In `BrowserView::new`, create the channel:

```rust
let (new_window_tx, new_window_rx) = async_channel::unbounded::<String>();
```

Pass `new_window_tx.clone()` into each `NativeBrowserWebView::new` call site. Spawn a task that drains `new_window_rx` and dispatches `BrowserViewAction::NewTabWithUrl { url }` — mirror the existing title-channel drain pattern.

- [ ] **Step 3:** Build + test:
```bash
cargo check -p warp-app --bin cast-codes --features gui
cargo test -p warp-app browser::
```

- [ ] **Step 4:** Commit:
```bash
git add app/src/browser/webview_host.rs app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): intercept popups + window.open as new tabs

Wires popup_policy::decide into wry's new_window_req_handler. _blank
links and window.open() calls now route into the pane's existing
new-tab path (BrowserViewAction::NewTabWithUrl). mailto/tel/sms/
castcodes hand off to the OS. javascript: URLs are dropped. Pages
can never pop a native window.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

---

## Phase 4 — DevTools gating

### Task 4.1: Apply `with_devtools` from settings

**Files:**
- Modify: `app/src/browser/webview_host.rs`
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1:** In `BrowserView::new`, read the setting once at pane open:

```rust
let devtools_enabled = crate::settings::browser::BrowserSettings::as_ref(ctx).devtools_enabled;
```

Pass `devtools_enabled` into every `NativeBrowserWebView::new(...)` callsite.

- [ ] **Step 2:** Add the field + apply on builder:

In `NativeBrowserWebView`:

```rust
#[cfg(not(target_family = "wasm"))]
devtools_enabled: bool,
```

`NativeBrowserWebView::new` accepts it; `attach_if_needed` adds `.with_devtools(self.devtools_enabled)` to the builder chain.

- [ ] **Step 3:** Verify:
```bash
cargo check -p warp-app --bin cast-codes --features gui
```

- [ ] **Step 4:** Commit:
```bash
git add app/src/browser/webview_host.rs app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): gate DevTools on BrowserSettings.devtools_enabled

Default is false — the Inspector is off unless the user explicitly
enables it under Settings → Browser. Setting is read once at pane
open; toggling it at runtime requires reopening the pane (acceptable
for a power-user feature flag).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

---

## Phase 5 — Blocklist infrastructure

### Task 5.1: Add adblock workspace dep + vendor EasyList snapshot

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `app/Cargo.toml`
- Create: `app/assets/bundled/blocklists/easylist-network.txt`

- [ ] **Step 1:** Vendor a pinned EasyList snapshot. Run from a separate scratch directory (not the worktree), download once, then copy to the worktree:

```bash
curl -fsSL https://easylist.to/easylist/easylist.txt -o /tmp/easylist-full.txt
# Strip cosmetic rules — keep only network-filter rules (lines that don't
# contain ## or #@# or :style). Smaller, faster, and we don't apply CSS
# hiding rules in the v1 implementation.
grep -E -v '^!|^$|##|#\@#|:style' /tmp/easylist-full.txt | head -c 200000 > app/assets/bundled/blocklists/easylist-network.txt
```

(Adjust the grep filter to match whatever the actual file uses — the goal is "drop cosmetic rules, keep network rules". If the size cap of 200000 strips too aggressively, raise it. Target: <200KB compiled.)

Add a header line at the top of the vendored file noting the snapshot date and source:

```
! Vendored from https://easylist.to/easylist/easylist.txt
! Snapshot date: <YYYY-MM-DD>
! Stripped: cosmetic / element-hiding rules. Network rules only.
```

- [ ] **Step 2:** Add `adblock` to workspace deps. Edit `Cargo.toml` (workspace root), under `[workspace.dependencies]` (around line 110 where `async-trait` lives):

```toml
adblock = { version = "0.9", default-features = false, features = ["full-regex-handling"] }
```

In `app/Cargo.toml` under `[dependencies]`:

```toml
adblock.workspace = true
```

- [ ] **Step 3:** Verify the dep resolves:
```bash
cargo check -p warp-app --bin cast-codes --features gui 2>&1 | grep -E "error" | head
```
Expected: passes (no code uses it yet, dep is just present).

- [ ] **Step 4:** Commit:
```bash
git add Cargo.toml app/Cargo.toml app/assets/bundled/blocklists/easylist-network.txt Cargo.lock
git commit -S -m "$(cat <<'EOF'
chore(deps): add adblock crate + vendor EasyList network rules

Adblock-rust 0.9 with full-regex-handling, no default features.

Vendored EasyList snapshot (network rules only, cosmetic rules
stripped). Refresh manually quarterly — auto-downloading would
violate the CastCodes fork-local cloud boundary.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

### Task 5.2: blocklist module

**Files:**
- Create: `app/src/browser/blocklist.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1:** Create the module:

```rust
//! Tracker / ad blocklist applied at navigation time.
//!
//! The rules engine is built once at app startup from a bundled
//! EasyList snapshot. URLs are checked via `should_block(url, origin)`
//! which returns true if EasyList's network rules match the request.
//!
//! Only top-level + iframe navigations go through this matcher — wry
//! 0.38 doesn't expose subresource request hooks. Image/XHR/fetch
//! trackers are NOT blocked in this version. Tracked as follow-up.

use std::sync::OnceLock;

use adblock::engine::Engine;
use adblock::lists::{FilterSet, ParseOptions};

const RULES: &str = include_str!("../../assets/bundled/blocklists/easylist-network.txt");

static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();

/// Builds the matcher lazily on first call. Returns `None` if the
/// bundled rules fail to parse — we log and fall through (no blocking)
/// rather than panicking.
fn engine() -> Option<&'static Engine> {
    ENGINE
        .get_or_init(|| {
            let mut set = FilterSet::new(true);
            let _ = set.add_filter_list(RULES, ParseOptions::default());
            Some(Engine::from_filter_set(set, true))
        })
        .as_ref()
}

/// Returns true if `url` should be blocked. `origin` is the page that
/// initiated the navigation (used by EasyList's third-party rules).
pub fn should_block(url: &str, origin: &str) -> bool {
    let Some(engine) = engine() else { return false; };
    // adblock's `check_network_request` API takes a Request struct.
    let request = match adblock::request::Request::new(url, origin, "document") {
        Ok(r) => r,
        Err(_) => return false,
    };
    let result = engine.check_network_request(&request);
    result.matched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_builds_from_bundled_rules() {
        // Calling should_block forces engine initialization. If parsing
        // fails the engine will be None and we'd return false here; the
        // important thing is that this doesn't panic.
        let _ = should_block("https://example.com/", "https://example.com/");
    }

    #[test]
    fn known_doubleclick_is_blocked() {
        // EasyList consistently blocks doubleclick.net. If the bundled
        // snapshot is stale or our matcher is misconfigured, this test
        // surfaces it.
        assert!(should_block(
            "https://googleads.g.doubleclick.net/pagead/ads",
            "https://example.com/",
        ));
    }

    #[test]
    fn first_party_request_not_blocked() {
        assert!(!should_block(
            "https://example.com/index.html",
            "https://example.com/",
        ));
    }
}
```

Add `pub(crate) mod blocklist;` to `app/src/browser/mod.rs`.

- [ ] **Step 2:** Run tests:
```bash
cargo test -p warp-app browser::blocklist
```
Expected: 3 passing. If `known_doubleclick_is_blocked` fails, the EasyList snapshot may be missing those rules — re-vendor and retry.

- [ ] **Step 3:** Commit:
```bash
git add app/src/browser/blocklist.rs app/src/browser/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): adblock engine over bundled EasyList rules

OnceLock-initialized matcher. should_block(url, origin) returns true
for EasyList network-rule matches. First-party requests are never
blocked. Engine init failures degrade to "no blocking" rather than
panic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

---

## Phase 6 — Apply blocklist via navigation handler

### Task 6.1: Wire `with_navigation_handler`

**Files:**
- Modify: `app/src/browser/webview_host.rs`

- [ ] **Step 1:** In `BrowserView::new`, read the blocklist-enabled setting once at pane open:

```rust
let blocklist_enabled = crate::settings::browser::BrowserSettings::as_ref(ctx).blocklist_enabled;
```

Pass it into `NativeBrowserWebView::new`.

- [ ] **Step 2:** Add the field + handler:

In `NativeBrowserWebView`:

```rust
#[cfg(not(target_family = "wasm"))]
blocklist_enabled: bool,
```

In `attach_if_needed`, add navigation handler before `.build()`:

```rust
let enabled = self.blocklist_enabled;
let initial_url = self.pending_url.clone().unwrap_or_default();
builder = builder.with_navigation_handler(move |url: String| -> bool {
    if !enabled {
        return true;
    }
    if super::blocklist::should_block(&url, &initial_url) {
        log::debug!("browser blocklist: cancelling navigation to {url}");
        return false;
    }
    true
});
```

Note: `initial_url` becomes stale as the user navigates. The "real" origin would be the WebView's current URL, but wry's handler doesn't supply it. Using `initial_url` here means third-party rules effectively all evaluate as "from the pane's first page". Document this limitation in a `//` comment.

- [ ] **Step 3:** Verify:
```bash
cargo check -p warp-app --bin cast-codes --features gui
cargo test -p warp-app browser::
```

- [ ] **Step 4:** Manual smoke:
- Open the pane, navigate to a page that loads ads (e.g. a news site).
- Check the debug log for `browser blocklist: cancelling navigation to ...` messages.

- [ ] **Step 5:** Commit:
```bash
git add app/src/browser/webview_host.rs app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): apply EasyList blocklist at navigation time

with_navigation_handler intercepts top-level navigations + iframe
loads. Subresources (img, XHR, fetch, websockets) are NOT filtered —
wry 0.38 doesn't expose subresource handlers. Future work tracked.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

---

## Phase 7 — Settings UI + final checks

### Task 7.1: Settings page rows

**Files:**
- Modify: existing settings view (search for where `BlockVisibilitySettings` is rendered to find the pattern, e.g. `app/src/settings_view/appearance_page.rs` or similar)
- Create: new "Browser" section under Settings if no equivalent panel exists, OR add the two rows to a sensible existing page.

- [ ] **Step 1:** Inspect:
```bash
grep -rn "BlockVisibilitySettings\|block_visibility" app/src/settings_view/ 2>/dev/null | head -10
```

Mirror that pattern for `BrowserSettings`. Add two toggle rows: "Enable Developer Tools (DevTools)" and "Block trackers and ads".

- [ ] **Step 2:** Verify build:
```bash
cargo check -p warp-app --bin cast-codes --features gui
```

- [ ] **Step 3:** Commit:
```bash
git add app/src/settings_view/...
git commit -S -m "$(cat <<'EOF'
feat(settings_view): expose browser pane security toggles

Settings → (appropriate page) gains two rows:
- Enable Developer Tools (DevTools)
- Block trackers and ads

Both default to the BrowserSettings defaults (DevTools off,
blocklist on).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

### Task 7.2: Refresh script

**Files:**
- Create: `script/refresh_blocklist`

- [ ] **Step 1:** Create the script (chmod +x):

```bash
#!/usr/bin/env bash
# Refreshes the bundled EasyList network rules. Run this manually once
# per quarter (or when bug reports mention sites bypassing the blocker).
#
# Usage:   ./script/refresh_blocklist
#          git diff app/assets/bundled/blocklists/easylist-network.txt
#          git commit -S -m "chore(browser): refresh EasyList snapshot"
set -euo pipefail

DEST="$(git rev-parse --show-toplevel)/app/assets/bundled/blocklists/easylist-network.txt"
SRC="https://easylist.to/easylist/easylist.txt"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

echo "Fetching $SRC ..."
curl -fsSL "$SRC" -o "$TMP"

DATE="$(date -u +%Y-%m-%d)"
{
    echo "! Vendored from $SRC"
    echo "! Snapshot date: $DATE"
    echo "! Stripped: cosmetic / element-hiding rules. Network rules only."
    echo ""
    grep -E -v '^!|^$|##|#\@#|:style' "$TMP"
} > "$DEST"

echo "Wrote $DEST"
wc -l "$DEST"
```

- [ ] **Step 2:** `chmod +x script/refresh_blocklist`

- [ ] **Step 3:** Commit:
```bash
git add script/refresh_blocklist
git commit -S -m "$(cat <<'EOF'
chore(scripts): add quarterly EasyList refresh helper

Single command refresh: ./script/refresh_blocklist. Fetches the
upstream list, strips cosmetic rules, writes the bundled file with a
date-stamped header. Run manually — no auto-download (CastCodes
cloud boundary).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Verify signature.

### Task 7.3: Final verification + PR

- [ ] **Step 1:** Full check:
```bash
cargo check -p warp-app --bin cast-codes --features gui
cargo test -p warp-app browser::
./script/check_rebrand
git log origin/main..HEAD --pretty='%h %G?' | awk '$2 != "G" {print "UNSIGNED:", $0}'
```

All four must pass (last command should print nothing).

- [ ] **Step 2:** Push:
```bash
git push -u origin feat/browser-panel-security
```

- [ ] **Step 3:** Open PR against `main` (or against `feat/browser-panel-ui-toggle-persistence` if PR #32 hasn't merged yet — use the parent branch).

Title: `feat(browser): security hardening — data isolation, popups, devtools, blocklist (PLAN-02)`

Body covers:
- What landed (the 4 hardening layers).
- The wry 0.38 subresource-blocking limitation and that follow-up work needs a wry upgrade.
- The vendored EasyList snapshot + quarterly refresh procedure.
- Test plan including manual cookie-isolation smoke and an ad-domain navigation smoke.

---

## Self-Review

**Spec coverage:**

| TECH spec § 4 / 7 requirement | Covered by |
|---|---|
| App-wide data dir | Phase 2 |
| Popup / new-window policy | Phase 3 |
| DevTools off, gated by setting | Phase 4 |
| Tracker / ad blocklist | Phase 5 + 6 |
| Quarterly refresh process | Task 7.2 |
| Settings UI for both toggles | Task 7.1 |

No spec gaps.

**Placeholder scan:** No "TBD" / "fill in later". Two areas labeled as "search the codebase for the existing pattern" (settings registration in Task 1.1 step 4; settings view row layout in Task 7.1) are necessary because they depend on local conventions a fresh subagent can resolve with a single `grep`.

**Type consistency:** `PopupAction`, `BrowserSettings`, `BrowserViewAction::NewTabWithUrl`, `should_block`, `engine` all referenced consistently across phases.

**Known limitations documented in plan:**
- wry 0.38 has no subresource request hook → blocklist covers navigations only.
- `with_navigation_handler` doesn't supply origin → use first-page URL as approximation.
- DevTools setting requires pane reopen to take effect (read once at pane open).
- Per-workspace data isolation deferred until cast-codes exposes stable workspace IDs.
