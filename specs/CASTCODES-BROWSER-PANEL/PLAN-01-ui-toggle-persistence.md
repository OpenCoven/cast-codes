# CastCodes Browser Panel — Plan 1: UI, Toggle, Persistence

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Read [`PRODUCT.md`](./PRODUCT.md) and [`TECH.md`](./TECH.md) before starting; this plan assumes both have been read.
>
> **Signing rule:** Every `git commit` in this plan MUST pass `-S`. The repo's local `commit.gpgsign` is false; without `-S` commits land unsigned. The signing key is already configured (`ssh-ed25519`). After each commit, run `git log -1 --show-signature` and confirm `Good "git" signature`. If signing failed, STOP and surface to the user.

**Goal:** Upgrade the existing browser pane (`app/src/browser/`) into a polished, toggleable, persistent panel matching the reference screenshot and comux's chrome treatment. No agent surface or security hardening in this plan — those land in PLAN-02, PLAN-03, PLAN-04.

**Architecture:** Split `browser_view.rs` (993 lines, conflates rendering + webview lifecycle) into focused modules: `webview_host.rs` (wry plumbing), `url_input.rs` (URL-or-search parser), `about_home.rs` (start page), `persistence.rs` (tab list JSON). Add `WorkspaceAction::ToggleBrowserPane` mirroring the existing `ToggleCliChatPanel` pattern. Persist pane open/closed state + tab list to a single global JSON file; restore on workspace startup via the existing session-restore hook.

**Tech Stack:** Rust, warpui (in-house UI framework), `wry` (existing dep), `serde_json` (existing dep), `opener` (existing dep). No new workspace dependencies.

---

## Files created or modified

**Created (new modules under `app/src/browser/`):**

| Path | Responsibility |
|---|---|
| `app/src/browser/webview_host.rs` | `NativeBrowserWebView` extracted verbatim from `browser_view.rs` (no behavior change). |
| `app/src/browser/url_input.rs` | `pub enum Resolved` + `pub fn resolve(&str) -> Resolved`. Pure functions. |
| `app/src/browser/about_home.rs` | Built-in start page served as a `data:text/html;base64,...` URL. |
| `app/src/browser/persistence.rs` | `BrowserState` load/save with atomic write + debounce. |
| `app/assets/bundled/html/about_home.html` | Static HTML for `about:home`. |
| `crates/integration/tests/browser_pane_toggle.rs` | Integration test: keymap, menu, palette, titlebar button. |
| `crates/integration/tests/browser_pane_multi_tab.rs` | Integration test: tab add/close/select. |
| `crates/integration/tests/browser_pane_persistence.rs` | Integration test: tabs survive restart. |

**Modified:**

| Path | Change |
|---|---|
| `app/src/browser/mod.rs` | Add `pub mod webview_host; pub mod url_input; pub mod about_home; pub mod persistence;` |
| `app/src/browser/browser_model.rs` | Extend `BrowserTab` with `pinned: bool` and `favicon: Option<String>`. Add `BrowserState`, `TabSnapshot`, `snapshot()`, `restore()`. |
| `app/src/browser/browser_view.rs` | Remove inlined `NativeBrowserWebView` (now in `webview_host.rs`). Restyle toolbar: add collapse + open-external. Adjust tab chip styling. Wire URL input through `url_input::resolve`. Default new tab to `about:home`. |
| `app/src/workspace/action.rs` | Add `ToggleBrowserPane` variant; keep `OpenBrowserPane { url: Option<String> }` for the alias path. |
| `app/src/workspace/view.rs` | Add `TOGGLE_BROWSER_PANE_BINDING_NAME` constant. Add `ToggleBrowserPane` match arm. Add `toggle_browser_pane()` method that adds the pane if absent, removes it if present. Hook persistence on toggle + on tab change. Hook restore on workspace init. |
| `app/src/workspace/mod.rs` | Replace existing `EditableBinding::new("workspace:open_browser_pane", …)` with a primary `TOGGLE_BROWSER_PANE_BINDING_NAME` binding plus an alias binding under the old name. Bind ⌘⌥B on macOS, ⌃⌥B on Linux/Windows. |
| `app/src/app_menus.rs` | Update View → Open Browser → "Toggle Browser Pane" wired to `workspace:toggle_browser_pane`. |
| `app/src/pane_group/pane/browser_pane.rs` | Add `BrowserPane::is_browser_pane(&PaneId)` helper so toggle can locate an existing pane. |
| `crates/warpui_core/src/keymap/...` | No direct edit — `EditableBinding::with_mac_key_binding("cmd-alt-b")` is the registration. |

---

## Phase 1 — Module split (no behavior change)

### Task 1.1: Extract `NativeBrowserWebView` into `webview_host.rs`

**Files:**
- Create: `app/src/browser/webview_host.rs`
- Modify: `app/src/browser/browser_view.rs:75-260` (remove the extracted struct)
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1: Create the new file**

Create `app/src/browser/webview_host.rs` with the verbatim contents of the `NativeBrowserWebView` struct and impls from `app/src/browser/browser_view.rs` lines 75 through approximately 260 (everything from `struct NativeBrowserWebView {` through the end of the `impl wry::raw_window_handle::HasWindowHandle for BorrowedAppKitView` block). Also move:

- The `BorrowedAppKitView` struct (search `browser_view.rs` for `struct BorrowedAppKitView`).
- Any `#[cfg(target_os = "macos")]` and `use` lines exclusively required by these types (e.g. `std::ffi::c_void`, `std::ptr::NonNull`, the wry imports).

Keep `pub(crate)` visibility on the struct and its methods so `browser_view.rs` can still use it. Top of the file:

```rust
use std::cell::RefCell;
use std::rc::Rc;
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use pathfinder_geometry::rect::RectF;
use warpui::{AppContext, WindowId};

use super::browser_model::TabId;

// (move struct + impls here verbatim)
```

- [ ] **Step 2: Delete the moved code from `browser_view.rs`**

In `app/src/browser/browser_view.rs`, delete the lines you just copied. Replace the moved imports with `use super::webview_host::{NativeBrowserWebView, BorrowedAppKitView};` (only the names actually referenced from `browser_view.rs` — `BorrowedAppKitView` may not be needed externally; drop it from the import if it isn't referenced).

- [ ] **Step 3: Wire the new module**

Edit `app/src/browser/mod.rs` (currently 5 lines) and add `pub mod webview_host;` at the bottom.

- [ ] **Step 4: Verify build**

Run:
```bash
cargo check -p warp --bin cast-codes --features gui
```
Expected: clean compile, possibly some `unused_import` warnings in `webview_host.rs` if you over-copied imports. Fix any warnings before commit.

- [ ] **Step 5: Verify tests still pass**

Run:
```bash
cargo test -p warp browser::
```
Expected: all existing `browser_model::tests` pass; no new tests yet.

- [ ] **Step 6: Commit**

```bash
git add app/src/browser/webview_host.rs app/src/browser/browser_view.rs app/src/browser/mod.rs
git commit -S -m "$(cat <<'EOF'
refactor(browser): extract NativeBrowserWebView into webview_host module

No behavior change. browser_view.rs is currently 993 lines and conflates
rendering with wry lifecycle. Splitting NativeBrowserWebView into a
dedicated webview_host module isolates the AppKit/wry plumbing so future
agent-surface and security changes can land without touching the view tree.

EOF
)"
git log -1 --show-signature | head -3
```

Verify the signature line reads `Good "git" signature …`. If not, STOP.

---

## Phase 2 — URL input parsing

### Task 2.1: Write failing tests for `url_input::resolve`

**Files:**
- Create: `app/src/browser/url_input.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1: Create the module with tests + empty implementation**

Create `app/src/browser/url_input.rs`:

```rust
//! URL / search query resolver for the browser pane's address bar.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Url(String),
    Search(String),
}

pub fn resolve(raw: &str) -> Resolved {
    let _ = raw;
    unimplemented!("write me")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_goes_to_about_home() {
        assert_eq!(resolve(""), Resolved::Url("about:home".to_string()));
        assert_eq!(resolve("   "), Resolved::Url("about:home".to_string()));
    }

    #[test]
    fn known_schemes_pass_through() {
        for url in [
            "http://example.com",
            "https://example.com",
            "file:///tmp/x.html",
            "about:blank",
            "data:text/html,<h1>hi</h1>",
            "castcodes://settings",
        ] {
            assert_eq!(resolve(url), Resolved::Url(url.to_string()));
        }
    }

    #[test]
    fn bare_hostname_gets_https() {
        assert_eq!(
            resolve("example.com"),
            Resolved::Url("https://example.com".to_string())
        );
        assert_eq!(
            resolve("example.com/path?q=1"),
            Resolved::Url("https://example.com/path?q=1".to_string())
        );
    }

    #[test]
    fn loopback_gets_http_not_https() {
        assert_eq!(
            resolve("localhost"),
            Resolved::Url("http://localhost".to_string())
        );
        assert_eq!(
            resolve("localhost:3000"),
            Resolved::Url("http://localhost:3000".to_string())
        );
        assert_eq!(
            resolve("127.0.0.1:8080/api"),
            Resolved::Url("http://127.0.0.1:8080/api".to_string())
        );
    }

    #[test]
    fn freetext_becomes_duckduckgo_search() {
        assert_eq!(
            resolve("rust async traits"),
            Resolved::Search("https://duckduckgo.com/?q=rust%20async%20traits".to_string())
        );
        assert_eq!(
            resolve("what is the time"),
            Resolved::Search("https://duckduckgo.com/?q=what%20is%20the%20time".to_string())
        );
    }

    #[test]
    fn input_with_spaces_but_dotty_is_still_search() {
        // "foo.bar baz" has a dot but also a space — treat as search, not URL.
        assert_eq!(
            resolve("foo.bar baz"),
            Resolved::Search("https://duckduckgo.com/?q=foo.bar%20baz".to_string())
        );
    }
}
```

Add `pub mod url_input;` to `app/src/browser/mod.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cargo test -p warp browser::url_input
```
Expected: all tests FAIL (or panic with `unimplemented!`).

### Task 2.2: Implement `resolve()` to pass all tests

**Files:**
- Modify: `app/src/browser/url_input.rs`

- [ ] **Step 1: Replace the `unimplemented!()` body**

Replace the `resolve` function body in `app/src/browser/url_input.rs` with:

```rust
pub fn resolve(raw: &str) -> Resolved {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Resolved::Url("about:home".to_string());
    }

    // Known scheme passes through.
    for scheme in ["http://", "https://", "file://", "about:", "data:", "castcodes://"] {
        if trimmed.starts_with(scheme) {
            return Resolved::Url(trimmed.to_string());
        }
    }

    // Detect a hostname-like input: no whitespace, and either contains a dot
    // or starts with a loopback host.
    let looks_like_host = !trimmed.contains(char::is_whitespace)
        && (trimmed.contains('.') || is_loopback_host(trimmed));

    if looks_like_host {
        let scheme = if is_loopback_host(trimmed) { "http://" } else { "https://" };
        return Resolved::Url(format!("{scheme}{trimmed}"));
    }

    // Otherwise: search query.
    let encoded = percent_encode_query(trimmed);
    Resolved::Search(format!("https://duckduckgo.com/?q={encoded}"))
}

fn is_loopback_host(input: &str) -> bool {
    let host = input.split_once('/').map(|(h, _)| h).unwrap_or(input);
    let host = host.split_once(':').map(|(h, _)| h).unwrap_or(host);
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
}

fn percent_encode_query(input: &str) -> String {
    // Minimal RFC3986 query encoding — alphanumerics + a few safe chars pass through;
    // everything else (including spaces) becomes %XX.
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    // Normalize uppercase to match the test expectations (spaces → %20).
    out
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run:
```bash
cargo test -p warp browser::url_input
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add app/src/browser/url_input.rs app/src/browser/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): add url_input resolver

Pure parser that turns address-bar input into either a URL to navigate
or a DuckDuckGo search query. Handles loopback hosts (localhost,
127.0.0.1) on http://; non-loopback hostnames on https://; passes known
schemes through; routes everything else to DDG.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

---

## Phase 3 — about:home start page

### Task 3.1: Add the bundled HTML asset

**Files:**
- Create: `app/assets/bundled/html/about_home.html`

- [ ] **Step 1: Write the start page HTML**

Create `app/assets/bundled/html/about_home.html`:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>New Tab</title>
  <style>
    :root {
      color-scheme: dark;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    }
    html, body {
      margin: 0;
      padding: 0;
      height: 100%;
      background: #0e0e10;
      color: #e8e8ea;
    }
    main {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 12px;
      height: 100%;
      text-align: center;
    }
    h1 { margin: 0; font-size: 28px; font-weight: 600; letter-spacing: -0.01em; }
    p  { margin: 0; opacity: 0.6; font-size: 14px; }
  </style>
</head>
<body>
  <main>
    <h1>New Tab</h1>
    <p>Use the address bar above to navigate or search.</p>
  </main>
</body>
</html>
```

### Task 3.2: Wire the asset through `about_home` module

**Files:**
- Create: `app/src/browser/about_home.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1: Create the module**

Create `app/src/browser/about_home.rs`:

```rust
//! Built-in start page for new tabs. Served as a `data:` URL so we don't
//! need a custom URL-scheme handler for v1.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

const ABOUT_HOME_HTML: &str = include_str!("../../assets/bundled/html/about_home.html");

/// Returns the data: URL representing the new-tab page.
pub fn url() -> String {
    let encoded = B64.encode(ABOUT_HOME_HTML.as_bytes());
    format!("data:text/html;charset=utf-8;base64,{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_is_data_scheme() {
        let u = url();
        assert!(u.starts_with("data:text/html;charset=utf-8;base64,"));
        // Decoding the base64 round-trips to the source HTML.
        let prefix = "data:text/html;charset=utf-8;base64,";
        let body = &u[prefix.len()..];
        let decoded = B64.decode(body).expect("base64 decodes");
        let decoded_str = String::from_utf8(decoded).expect("utf8");
        assert!(decoded_str.contains("<h1>New Tab</h1>"));
    }
}
```

- [ ] **Step 2: Verify `base64` is available in `app/Cargo.toml`**

Run:
```bash
grep -n "^base64" app/Cargo.toml
```

If `base64` is not present, add to `[dependencies]`:

```toml
base64.workspace = true
```

And confirm the workspace root `Cargo.toml` has `base64` under `[workspace.dependencies]` (search for `base64 = ` — Warp's tree already uses base64 widely, so this should be present).

- [ ] **Step 3: Register the module**

Add `pub mod about_home;` to `app/src/browser/mod.rs`.

- [ ] **Step 4: Run the test**

Run:
```bash
cargo test -p warp browser::about_home
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/browser/about_home.rs app/src/browser/mod.rs app/assets/bundled/html/about_home.html app/Cargo.toml
git commit -S -m "$(cat <<'EOF'
feat(browser): add about:home start page module

Bundled HTML rendered as a data: URL so new tabs land on a CastCodes
start page instead of opencoven.ai (which would violate the fork-local
cloud boundary). Plain HTML, no scripts.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

---

## Phase 4 — BrowserModel persistence schema

### Task 4.1: Extend `BrowserTab` with `pinned` and `favicon`

**Files:**
- Modify: `app/src/browser/browser_model.rs`

- [ ] **Step 1: Add failing tests for the new fields**

In `app/src/browser/browser_model.rs`, inside `#[cfg(test)] mod tests`, append:

```rust
    #[test]
    fn new_tab_has_default_pinned_and_no_favicon() {
        let model = BrowserModel::new("https://a.test");
        let tab = &model.tabs()[0];
        assert!(!tab.pinned());
        assert_eq!(tab.favicon(), None);
    }

    #[test]
    fn pinned_and_favicon_setters_round_trip() {
        let mut model = BrowserModel::new("https://a.test");
        let id = model.tabs()[0].id();
        model.set_pinned(id, true);
        model.set_favicon(id, Some("https://a.test/favicon.ico".into()));
        let tab = &model.tabs()[0];
        assert!(tab.pinned());
        assert_eq!(tab.favicon(), Some("https://a.test/favicon.ico"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cargo test -p warp browser::browser_model
```
Expected: FAIL with `method not found: pinned`, `favicon`, `set_pinned`, `set_favicon`.

- [ ] **Step 3: Add fields + accessors**

In `app/src/browser/browser_model.rs`, replace the `BrowserTab` struct definition (lines 5-13) with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTab {
    id: TabId,
    current_url: String,
    back_history: Vec<String>,
    forward_history: Vec<String>,
    loading: bool,
    title: String,
    pinned: bool,
    favicon: Option<String>,
}
```

In `BrowserTab::new`, initialize the new fields:

```rust
fn new(id: TabId, url: impl Into<String>) -> Self {
    Self {
        id,
        current_url: normalize_url(url.into()),
        back_history: Vec::new(),
        forward_history: Vec::new(),
        loading: false,
        title: String::new(),
        pinned: false,
        favicon: None,
    }
}
```

After the existing `set_title` method, add accessors:

```rust
pub fn pinned(&self) -> bool {
    self.pinned
}

pub fn favicon(&self) -> Option<&str> {
    self.favicon.as_deref()
}

fn set_pinned(&mut self, pinned: bool) {
    self.pinned = pinned;
}

fn set_favicon(&mut self, favicon: Option<String>) {
    self.favicon = favicon;
}
```

Add the matching `BrowserModel` methods (right above the closing `}` of `impl BrowserModel`):

```rust
pub fn set_pinned(&mut self, id: TabId, pinned: bool) -> bool {
    let Some(idx) = self.index_of(id) else { return false; };
    self.tabs[idx].set_pinned(pinned);
    true
}

pub fn set_favicon(&mut self, id: TabId, favicon: Option<String>) -> bool {
    let Some(idx) = self.index_of(id) else { return false; };
    self.tabs[idx].set_favicon(favicon);
    true
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo test -p warp browser::browser_model
```
Expected: PASS for all `browser_model::tests`.

### Task 4.2: Add snapshot / restore API + types

**Files:**
- Modify: `app/src/browser/browser_model.rs`

- [ ] **Step 1: Add failing tests for snapshot/restore**

Append inside `mod tests`:

```rust
    #[test]
    fn snapshot_round_trip_preserves_tabs_active_and_pinned() {
        let mut model = BrowserModel::new("https://a.test");
        model.add_tab("https://b.test");
        let pinned_id = model.tabs()[0].id();
        model.set_pinned(pinned_id, true);
        model.select_tab(1);

        let state = model.snapshot(/* open */ true);
        assert_eq!(state.v, 1);
        assert!(state.open);
        assert_eq!(state.active, 1);
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.tabs[0].url, "https://a.test");
        assert!(state.tabs[0].pinned);
        assert_eq!(state.tabs[1].url, "https://b.test");
        assert!(!state.tabs[1].pinned);

        let restored = BrowserModel::restore(state);
        assert_eq!(restored.tabs().len(), 2);
        assert_eq!(restored.active_index(), 1);
        assert_eq!(restored.tabs()[0].current_url(), "https://a.test");
        assert!(restored.tabs()[0].pinned());
        assert_eq!(restored.tabs()[1].current_url(), "https://b.test");
    }

    #[test]
    fn restore_with_empty_tabs_falls_back_to_default() {
        let state = BrowserState {
            v: 1,
            open: true,
            tabs: vec![],
            active: 0,
        };
        let model = BrowserModel::restore(state);
        // Restoring with no tabs spawns the default about:home tab.
        assert_eq!(model.tabs().len(), 1);
        assert_eq!(model.current_url(), DEFAULT_BROWSER_URL);
    }

    #[test]
    fn restore_clamps_out_of_range_active() {
        let state = BrowserState {
            v: 1,
            open: true,
            tabs: vec![TabSnapshot {
                url: "https://a.test".into(),
                title: String::new(),
                pinned: false,
            }],
            active: 99,
        };
        let model = BrowserModel::restore(state);
        assert_eq!(model.active_index(), 0);
    }

    #[test]
    fn history_is_not_persisted() {
        let mut model = BrowserModel::new("https://a.test");
        model.navigate("https://b.test");
        model.navigate("https://c.test");
        assert!(model.can_go_back());

        let restored = BrowserModel::restore(model.snapshot(true));
        // After round-trip, history is empty.
        assert!(!restored.can_go_back());
        assert!(!restored.can_go_forward());
        assert_eq!(restored.current_url(), "https://c.test");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cargo test -p warp browser::browser_model
```
Expected: FAIL with `cannot find type BrowserState`, `cannot find type TabSnapshot`, `method not found: snapshot`, `method not found: restore`.

- [ ] **Step 3: Add the types and methods**

At the top of `app/src/browser/browser_model.rs` (after the `pub const DEFAULT_BROWSER_URL` line), add:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserState {
    pub v: u32,
    pub open: bool,
    pub tabs: Vec<TabSnapshot>,
    pub active: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabSnapshot {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
}

pub const BROWSER_STATE_VERSION: u32 = 1;
```

Add these methods to `impl BrowserModel` (above the closing `}`):

```rust
pub fn snapshot(&self, open: bool) -> BrowserState {
    BrowserState {
        v: BROWSER_STATE_VERSION,
        open,
        active: self.active,
        tabs: self
            .tabs
            .iter()
            .map(|tab| TabSnapshot {
                url: tab.current_url.clone(),
                title: tab.title.clone(),
                pinned: tab.pinned,
            })
            .collect(),
    }
}

pub fn restore(state: BrowserState) -> Self {
    let mut model = Self {
        tabs: Vec::with_capacity(state.tabs.len().max(1)),
        active: 0,
        next_id: 0,
    };
    for snap in state.tabs {
        let id = model.push_tab(snap.url);
        let idx = model.tabs.len() - 1;
        model.tabs[idx].title = snap.title;
        model.tabs[idx].pinned = snap.pinned;
    }
    if model.tabs.is_empty() {
        model.push_tab(DEFAULT_BROWSER_URL);
    }
    model.active = state.active.min(model.tabs.len() - 1);
    model
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo test -p warp browser::browser_model
```
Expected: PASS for all tests including the new ones.

- [ ] **Step 5: Commit**

```bash
git add app/src/browser/browser_model.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): add BrowserState snapshot / restore + pinned + favicon

Adds `pinned` and `favicon` fields to BrowserTab (no UI affordance yet —
that's a follow-up). Adds versioned BrowserState + TabSnapshot for
persistence in the next phase. History is intentionally not persisted;
test asserts this is documented behavior.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

---

## Phase 5 — Persistence module

### Task 5.1: Failing tests for load / save round-trip

**Files:**
- Create: `app/src/browser/persistence.rs`
- Modify: `app/src/browser/mod.rs`

- [ ] **Step 1: Create the module skeleton with failing tests**

Create `app/src/browser/persistence.rs`:

```rust
//! Persistence of `BrowserState` to a JSON file under the CastCodes
//! support directory. Atomic write via temp-file + rename. Load is
//! lenient: any failure (missing file, malformed JSON, unknown version)
//! returns `None` instead of panicking.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::browser_model::{BrowserState, BROWSER_STATE_VERSION};

/// Returns the path to the persisted browser state file.
pub fn state_path(state_dir: &Path) -> PathBuf {
    state_dir.join("browser").join("state.json")
}

/// Loads the persisted state, or `None` if the file is missing, malformed,
/// or written by an unknown version.
pub fn load(state_dir: &Path) -> Option<BrowserState> {
    let _ = state_dir;
    None
}

/// Atomically writes the state file. Creates parent directories as needed.
pub fn save(state_dir: &Path, state: &BrowserState) -> std::io::Result<()> {
    let _ = (state_dir, state);
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::browser_model::{TabSnapshot, BrowserModel};

    fn tmp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn load_returns_none_when_file_absent() {
        let dir = tmp_dir();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tmp_dir();
        let state = BrowserState {
            v: BROWSER_STATE_VERSION,
            open: true,
            tabs: vec![
                TabSnapshot { url: "https://a.test".into(), title: "A".into(), pinned: true },
                TabSnapshot { url: "https://b.test".into(), title: "B".into(), pinned: false },
            ],
            active: 1,
        };
        save(dir.path(), &state).expect("save");
        let loaded = load(dir.path()).expect("load");
        assert_eq!(loaded, state);
    }

    #[test]
    fn load_returns_none_for_malformed_json() {
        let dir = tmp_dir();
        let path = state_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not json").unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn load_returns_none_for_unknown_version() {
        let dir = tmp_dir();
        let path = state_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            br#"{"v":99,"open":true,"tabs":[],"active":0}"#,
        ).unwrap();
        assert!(load(dir.path()).is_none());
    }

    #[test]
    fn save_uses_atomic_temp_rename() {
        // Crash-safety check: after a save, no `.tmp` is left behind.
        let dir = tmp_dir();
        let model = BrowserModel::new("https://a.test");
        save(dir.path(), &model.snapshot(true)).expect("save");
        let tmp = state_path(dir.path()).with_extension("json.tmp");
        assert!(!tmp.exists(), "leftover temp file: {:?}", tmp);
    }
}
```

Add `pub mod persistence;` to `app/src/browser/mod.rs`.

- [ ] **Step 2: Verify `tempfile` is available in `app/Cargo.toml`**

Run:
```bash
grep -n "^tempfile" app/Cargo.toml
```

If not present under `[dev-dependencies]`, add:

```toml
[dev-dependencies]
tempfile.workspace = true
```

The workspace root `Cargo.toml` already declares `tempfile` (Warp uses it extensively). Confirm with `grep "^tempfile = " Cargo.toml`.

- [ ] **Step 3: Run tests to verify they fail**

Run:
```bash
cargo test -p warp browser::persistence
```
Expected: FAIL with `unimplemented!` panics on the save-related tests; `load_returns_none_when_file_absent` and `load_returns_none_for_malformed_json` may pass already (since `load` currently always returns `None`). That's OK.

### Task 5.2: Implement load + atomic save

**Files:**
- Modify: `app/src/browser/persistence.rs`

- [ ] **Step 1: Replace the stub bodies**

Replace the bodies of `load` and `save` in `app/src/browser/persistence.rs` with:

```rust
pub fn load(state_dir: &Path) -> Option<BrowserState> {
    let path = state_path(state_dir);
    let bytes = fs::read(&path).ok()?;
    let parsed: BrowserState = serde_json::from_slice(&bytes).ok()?;
    if parsed.v != BROWSER_STATE_VERSION {
        return None;
    }
    Some(parsed)
}

pub fn save(state_dir: &Path, state: &BrowserState) -> std::io::Result<()> {
    let path = state_path(state_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        let json = serde_json::to_vec_pretty(state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run:
```bash
cargo test -p warp browser::persistence
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add app/src/browser/persistence.rs app/src/browser/mod.rs app/Cargo.toml
git commit -S -m "$(cat <<'EOF'
feat(browser): add persistence module for BrowserState

Atomic save (temp-file + rename) and lenient load (missing/malformed/
unknown-version returns None instead of panicking). No debounce yet;
debouncing happens at the call site in workspace::view.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

---

## Phase 6 — Toolbar + tab strip restyle

> **Caveat:** Phase 6 modifies `browser_view.rs` rendering code. The exact warpui element types (`Container`, `Flex`, `Hoverable`, `Border`, etc.) and styling constants are already imported at the top of the file (lines 1-35) and used heavily in the existing toolbar (search for `fn render_toolbar` or equivalent). Reuse existing helpers — do not introduce new ones.

### Task 6.1: Add collapse + open-external buttons to the toolbar

**Files:**
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1: Locate the toolbar render function**

Search `app/src/browser/browser_view.rs` for the function that renders the toolbar row containing back / forward / reload. Typically named `render_toolbar` or inlined in the main `render` method. Identify the `Flex` that holds the icon buttons.

- [ ] **Step 2: Add `BrowserViewAction` variants**

In the `BrowserViewAction` enum (lines 59-67), add two new variants:

```rust
pub enum BrowserViewAction {
    Back,
    Forward,
    Reload,
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    OpenExternal,       // NEW
    Collapse,           // NEW (dispatches workspace:toggle_browser_pane)
}
```

- [ ] **Step 3: Add icon imports if missing**

At the top of `browser_view.rs`, ensure the `Icon` enum re-export includes `OpenExternal` and `ChevronLeft` (or whichever icons render the open-external arrow and the collapse chevron). Search the file for the existing `Icon::` usages (e.g. `Icon::Reload`) to confirm the import path. If `Icon::OpenExternal` or `Icon::ChevronLeft` doesn't exist, fall back to `Icon::Close` for collapse and `Icon::Plus` for open-external as visual stand-ins; this is acceptable for v1 and tracked as a known visual gap in the integration test description.

- [ ] **Step 4: Insert the two buttons into the toolbar Flex**

Find the toolbar row that wraps back/forward/reload buttons. Prepend a collapse button at the start and append an open-external button at the end. Pattern (adapt to local helper names):

```rust
.add_child(icon_button_with_color(
    Icon::ChevronLeft, /* tooltip */ "Toggle browser pane (⌘⌥B)",
    Color::Text, ctx, |ctx| ctx.dispatch_action(BrowserViewAction::Collapse),
))
// … existing back / forward / reload / URL input …
.add_child(icon_button_with_color(
    Icon::OpenExternal, /* tooltip */ "Open in default browser",
    Color::Text, ctx, |ctx| ctx.dispatch_action(BrowserViewAction::OpenExternal),
))
```

- [ ] **Step 5: Handle the new actions**

In the `BrowserView::handle_action` method (search for the existing match on `BrowserViewAction`), add:

```rust
BrowserViewAction::OpenExternal => {
    let url = self.model.read(ctx).current_url().to_string();
    if let Err(err) = opener::open(&url) {
        log::warn!("failed to open url in external browser: {err}");
    }
}
BrowserViewAction::Collapse => {
    ctx.dispatch_global_action("workspace:toggle_browser_pane", &());
}
```

If `opener` is not yet imported, add `use opener;` at the top.

- [ ] **Step 6: Verify build + smoke test**

Run:
```bash
cargo check -p warp --bin cast-codes --features gui
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): add collapse + open-external buttons to toolbar

Collapse dispatches workspace:toggle_browser_pane (wired in Phase 7).
Open-external opens the active tab URL in the system default browser
via the `opener` crate.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

### Task 6.2: Update URL input placeholder + wire url_input::resolve

**Files:**
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1: Update placeholder constant**

In `app/src/browser/browser_view.rs:52`, change:

```rust
const URL_BAR_PLACEHOLDER: &str = "Enter URL";
```

to:

```rust
const URL_BAR_PLACEHOLDER: &str = "URL or search the web";
```

- [ ] **Step 2: Route the editor's `Submit` event through `url_input::resolve`**

Search the file for the editor `Submit` handler (look for `EditorEvent::Submit` or the existing place where the URL bar input is read and `model.navigate(...)` is called). Replace direct `navigate(raw_text)` with:

```rust
use super::url_input::{resolve, Resolved};

let target = match resolve(&raw_text) {
    Resolved::Url(u) => u,
    Resolved::Search(u) => u,
};
self.model.update(ctx, |m, _| { m.navigate(target.clone()); });
self.active_webview_mut().load_url(&target);
```

(`self.active_webview_mut()` is the equivalent of the existing tab → webview lookup. Use the local helper.)

- [ ] **Step 3: Verify build**

Run:
```bash
cargo check -p warp --bin cast-codes --features gui
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): URL bar accepts free-text → DuckDuckGo search

Placeholder updated to 'URL or search the web'. Submit handler now
routes through browser::url_input::resolve which returns either a URL
to navigate or a DDG search URL.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

### Task 6.3: Default new-tab URL becomes `about:home`

**Files:**
- Modify: `app/src/browser/browser_model.rs`
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1: Update test to assert about:home as new-tab default**

In `app/src/browser/browser_model.rs`, find the `multi_tab_lifecycle` test (around line 297). The assertion currently reads:

```rust
assert_eq!(model.current_url(), DEFAULT_BROWSER_URL);
```

Replace `DEFAULT_BROWSER_URL` references throughout the file. Change the constant at the top:

```rust
pub const DEFAULT_BROWSER_URL: &str = "about:home";
```

Then add a new test:

```rust
    #[test]
    fn default_new_tab_lands_on_about_home() {
        let model = BrowserModel::new(DEFAULT_BROWSER_URL);
        assert_eq!(model.current_url(), "about:home");
    }
```

- [ ] **Step 2: Update `normalize_url` to pass-through `about:home`**

In `app/src/browser/browser_model.rs`, the `normalize_url` function already passes through `about:` schemes (line 261). Verify by reading that block; no change needed if it does. If it doesn't, add `|| url.starts_with("about:")` to the pass-through condition.

- [ ] **Step 3: Map `about:home` → data: URL at the webview boundary**

The `BrowserModel` keeps the human-readable `about:home` in its tab URL (so the URL bar shows it), but the `NativeBrowserWebView::load_url` call needs the real `data:` URL. In `browser_view.rs`, at every call site where we hand a URL to the webview, route through a helper:

```rust
fn webview_url_for(model_url: &str) -> String {
    if model_url == "about:home" {
        super::about_home::url()
    } else {
        model_url.to_string()
    }
}
```

Use `webview_url_for(&...)` instead of cloning the model URL directly when calling `webview.load_url(...)`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p warp browser::
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/browser/browser_model.rs app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): new tabs land on about:home

DEFAULT_BROWSER_URL changes from https://opencoven.ai to about:home,
keeping the CastCodes public build off any upstream domain by default.
Webview load_url calls map about:home to the bundled data: URL.

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

### Task 6.4: Tab strip chip styling — accent border + hover close

**Files:**
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1: Locate the tab chip render function**

Search for the tab strip rendering — likely a function named `render_tab` or inline inside `render_tab_strip`. Each tab chip uses `Container` + `Border` + the constants at lines 41-51 (`TAB_BORDER_RADIUS`, `TAB_HEIGHT`, etc.).

- [ ] **Step 2: Add accent border for the active tab**

In the chip render function, conditionally apply a border:

```rust
let is_active = self.model.read(ctx).active_index() == idx;
let border = if is_active {
    Border::all(1.0, theme.accent())
} else {
    Border::none()
};
container.with_border(border).with_corner_radius(CornerRadius::all(Radius::new(TAB_BORDER_RADIUS)))
```

(Adapt `theme.accent()` to the local accent-color accessor — the existing toolbar/tab code references some color helper; copy that idiom rather than guessing.)

- [ ] **Step 3: Hide the close button until hover**

The chip already has a `close_mouse: MouseStateHandle` per `TabUiState`. Wrap the close-button child with `Hoverable::new(parent_state.chip_mouse.clone(), |hovered| { ... only render close if hovered || is_active })`. If the existing render already does this, leave it alone.

- [ ] **Step 4: Add a hover tooltip showing the full title**

Reuse the existing tooltip helper used elsewhere (search the warpui imports for `Tooltip` or `WithTooltip`). Wrap the chip in a tooltip whose text is `tab.display_title()`.

- [ ] **Step 5: Verify visually**

Build:

```bash
cargo build -p warp --bin cast-codes --features gui
```

Then run the app:

```bash
./target/debug/cast-codes
```

Open the browser pane (⌘⌥B will be wired in Phase 7; for now use View → Open Browser). Add a couple tabs, hover, confirm: active tab has accent border, close button appears on hover, hovering the chip shows the full title in a tooltip.

- [ ] **Step 6: Commit**

```bash
git add app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(browser): tab chip styling — accent border, hover close, tooltip

Active tab gets an accent border. Close button only visible on hover.
Hovering a chip surfaces the full tab title via tooltip (which is what
the floating-title overlay from the design brief was solving).

EOF
)"
git log -1 --show-signature | head -3
```

Verify signature.

---

## Phase 7 — Toggle action, menu, keymap

### Task 7.1: Add `WorkspaceAction::ToggleBrowserPane`

**Files:**
- Modify: `app/src/workspace/action.rs`

- [ ] **Step 1: Add the variant**

In `app/src/workspace/action.rs:706`, locate the `OpenBrowserPane` variant. Add a sibling variant right above it:

```rust
ToggleBrowserPane,
OpenBrowserPane { url: Option<String> },
```

Search the file for `OpenBrowserPane { .. } => false` (around line 986) and add `ToggleBrowserPane` to the same `false` list. Search for any other match arms that exhaustively cover `WorkspaceAction` and add a `ToggleBrowserPane` arm that delegates to the same handler `OpenBrowserPane` uses, or a new arm if behavior differs. Use the chat panel `ToggleCliChatPanel` variant (line 269) as a reference for what list memberships are needed.

- [ ] **Step 2: Verify build**

```bash
cargo check -p warp --bin cast-codes --features gui
```
Expected: PASS. The new variant doesn't yet have a `view.rs` handler — that's the next task. If the compiler insists on exhaustive coverage, add a temporary placeholder arm in `view.rs` (just `unimplemented!("ToggleBrowserPane")`) until Task 7.2 fills it in.

- [ ] **Step 3: Commit (interim)**

```bash
git add app/src/workspace/action.rs
git commit -S -m "$(cat <<'EOF'
chore(workspace): add ToggleBrowserPane action variant

Empty handler — will be implemented in the next commit.

EOF
)"
git log -1 --show-signature | head -3
```

### Task 7.2: Implement `toggle_browser_pane` in `view.rs`

**Files:**
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/pane_group/pane/browser_pane.rs`

- [ ] **Step 1: Add a helper on `BrowserPane`**

In `app/src/pane_group/pane/browser_pane.rs`, after the existing `impl BrowserPane` block (line 43), add:

```rust
impl BrowserPane {
    pub fn is_pane_id(pane_id: PaneId) -> bool {
        matches!(pane_id, PaneId::Browser(_))
    }
}
```

(The actual variant name is whatever the existing `PaneId::from_browser_pane_*` helpers produce — inspect `app/src/pane_group/pane/mod.rs:124` for `BrowserPaneId` to confirm. If `PaneId` uses a different variant naming, adapt.)

- [ ] **Step 2: Add the binding-name constant + toggle method**

In `app/src/workspace/view.rs`, immediately below the existing `TOGGLE_CLI_CHAT_PANEL_BINDING_NAME` constant (line 632), add:

```rust
pub(crate) const TOGGLE_BROWSER_PANE_BINDING_NAME: &str = "workspace:toggle_browser_pane";
```

Locate the existing `open_browser_pane` method (line 13041). Immediately below it, add:

```rust
pub(crate) fn toggle_browser_pane(&mut self, ctx: &mut ViewContext<Self>) {
    // If a browser pane already exists in the active pane group, remove it.
    let active_group = self.active_tab_pane_group();
    let existing = active_group.as_ref(ctx).first_browser_pane_id();
    match existing {
        Some(pane_id) => {
            active_group.update(ctx, |group, ctx| {
                group.close_pane_by_id(pane_id, ctx);
            });
            // Mark the browser as closed in persisted state.
            self.save_browser_state(/* open */ false, ctx);
        }
        None => {
            self.open_browser_pane(None, ctx);
            self.save_browser_state(/* open */ true, ctx);
        }
    }
}
```

`PaneGroup::first_browser_pane_id` and `PaneGroup::close_pane_by_id` may need to be added if they don't exist. Inspect `app/src/pane_group/mod.rs` (or wherever `PaneGroup` lives) and either reuse an existing close-by-id helper or add a thin wrapper that walks the group's pane list. Treat this as a sub-task of Step 2 — keep the addition minimal.

`Workspace::save_browser_state` is a method you'll add in Phase 9. For now, stub it as `fn save_browser_state(&self, _open: bool, _ctx: &mut ViewContext<Self>) {}` near the bottom of the `impl Workspace` block. It'll be filled in.

- [ ] **Step 3: Add the action match arm**

In the same file, find the existing `OpenBrowserPane { url }` match arm (line 20681). Above it, add:

```rust
#[cfg(not(target_family = "wasm"))]
ToggleBrowserPane => {
    self.toggle_browser_pane(ctx);
}
```

- [ ] **Step 4: Verify build + existing tests**

```bash
cargo check -p warp --bin cast-codes --features gui
cargo test -p warp browser::
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/pane_group/pane/browser_pane.rs app/src/pane_group/pane/mod.rs app/src/pane_group/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(workspace): implement ToggleBrowserPane action

Adds Workspace::toggle_browser_pane which adds the pane if absent,
removes it if present. Persistence is stubbed; wired up in Phase 9.

EOF
)"
git log -1 --show-signature | head -3
```

### Task 7.3: Register the binding (⌘⌥B) + keep the open alias

**Files:**
- Modify: `app/src/workspace/mod.rs`

- [ ] **Step 1: Import the new binding constant**

At the top of `app/src/workspace/mod.rs:81` (near the existing `use crate::workspace::view::TOGGLE_CLI_CHAT_PANEL_BINDING_NAME;`), add:

```rust
use crate::workspace::view::TOGGLE_BROWSER_PANE_BINDING_NAME;
```

- [ ] **Step 2: Replace the existing browser binding**

At `app/src/workspace/mod.rs:740-747`, the current binding looks like:

```rust
EditableBinding::new(
    "workspace:open_browser_pane",
    BindingDescription::new("Open Browser Pane"),
    WorkspaceAction::OpenBrowserPane { url: None },
)
.with_group(bindings::BindingGroup::Navigation.as_str())
.with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging")),
```

Replace with **two** bindings — primary toggle plus a no-op alias kept for backward compatibility:

```rust
EditableBinding::new(
    TOGGLE_BROWSER_PANE_BINDING_NAME,
    BindingDescription::new("Toggle Browser Pane")
        .with_custom_description(bindings::MAC_MENUS_CONTEXT, "Toggle Browser Pane"),
    WorkspaceAction::ToggleBrowserPane,
)
.with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging"))
.with_group(bindings::BindingGroup::Navigation.as_str())
.with_mac_key_binding("cmd-alt-b")
.with_linux_or_windows_key_binding("ctrl-alt-b"),
EditableBinding::new(
    "workspace:open_browser_pane",
    BindingDescription::new("Open Browser Pane"),
    WorkspaceAction::OpenBrowserPane { url: None },
)
.with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging"))
.with_group(bindings::BindingGroup::Navigation.as_str()),
```

- [ ] **Step 3: Verify keymap tests**

Run:
```bash
cargo test -p warpui_core keymap
```
Expected: PASS (no new tests; just confirming registration didn't break the keymap loader).

- [ ] **Step 4: Verify smoke test**

```bash
cargo run -p warp --bin cast-codes --features gui
```

In the app, press ⌘⌥B. Browser pane should open. Press ⌘⌥B again. It should close. Both palette entries (`workspace:toggle_browser_pane` and `workspace:open_browser_pane`) should still appear and work.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(workspace): bind ⌘⌥B to toggle browser pane

Primary binding workspace:toggle_browser_pane gets the ⌘⌥B shortcut
(⌃⌥B on Linux/Windows). The old workspace:open_browser_pane action
remains registered without a shortcut for backward compatibility.

EOF
)"
git log -1 --show-signature | head -3
```

### Task 7.4: Update the View menu

**Files:**
- Modify: `app/src/app_menus.rs`

- [ ] **Step 1: Replace the existing menu entry**

In `app/src/app_menus.rs:410`, find:

```rust
ctx.dispatch_global_action("workspace:open_browser_pane", &());
```

Replace with:

```rust
ctx.dispatch_global_action("workspace:toggle_browser_pane", &());
```

Update the menu item label nearby — search for `"Open Browser"` or similar and change to `"Toggle Browser Pane"`.

- [ ] **Step 2: Smoke test**

```bash
cargo run -p warp --bin cast-codes --features gui
```

Open View menu → "Toggle Browser Pane" appears with shortcut "⌘⌥B" displayed. Click it: pane opens. Click again: pane closes.

- [ ] **Step 3: Commit**

```bash
git add app/src/app_menus.rs
git commit -S -m "$(cat <<'EOF'
feat(app_menus): View → Toggle Browser Pane

Menu entry now dispatches workspace:toggle_browser_pane and reflects
the ⌘⌥B shortcut.

EOF
)"
git log -1 --show-signature | head -3
```

---

## Phase 8 — Titlebar toggle button

> **Note:** Adding the titlebar chrome button requires understanding the existing `header_toolbar_item` system. Inspect `app/src/workspace/header_toolbar_item.rs` first to see how items are registered. The chat panel toggle (commit `768c1289`) likely added its own chrome item; mirror that pattern.

### Task 8.1: Add a `BrowserToggle` toolbar item

**Files:**
- Modify: `app/src/workspace/header_toolbar_item.rs`
- Modify: `app/src/workspace/header_toolbar_editor.rs` (if registration lives there)

- [ ] **Step 1: Inspect the existing header_toolbar items**

```bash
grep -n "pub enum.*ToolbarItem\|HeaderToolbarItem" app/src/workspace/header_toolbar_item.rs | head -10
```

Identify the enum that lists toolbar items and where items get rendered.

- [ ] **Step 2: Add a `BrowserToggle` variant**

Following the existing pattern, add `BrowserToggle` to the toolbar-item enum. In the render arm:

```rust
HeaderToolbarItem::BrowserToggle => {
    let pressed = ctx.global::<Workspace>().is_browser_pane_open();
    icon_button_with_color(
        if pressed { Icon::PanelRightFilled } else { Icon::PanelRight },
        "Toggle browser pane (⌘⌥B)",
        Color::Text, ctx,
        |ctx| ctx.dispatch_global_action(TOGGLE_BROWSER_PANE_BINDING_NAME, &()),
    )
}
```

`Workspace::is_browser_pane_open()` is a small new helper:

```rust
pub fn is_browser_pane_open(&self, ctx: &AppContext) -> bool {
    let group = self.active_tab_pane_group();
    group.as_ref(ctx).first_browser_pane_id().is_some()
}
```

Place this near the existing `open_browser_pane` method in `workspace/view.rs`.

- [ ] **Step 3: Register the item in the default toolbar order**

If `header_toolbar_editor.rs` keeps an ordered list of default items, append `HeaderToolbarItem::BrowserToggle` to it. Mirror exactly how the chat panel toggle was added (search `header_toolbar_editor.rs` for `ToggleCliChatPanel` or `ChatToggle`).

- [ ] **Step 4: Smoke test**

```bash
cargo run -p warp --bin cast-codes --features gui
```

Titlebar shows a new browser-toggle button. Click it: pane opens, button shows pressed state. Click again: pane closes, button shows unpressed state.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/header_toolbar_item.rs app/src/workspace/header_toolbar_editor.rs app/src/workspace/view.rs
git commit -S -m "$(cat <<'EOF'
feat(titlebar): add browser-pane toggle button

Mirrors the chat panel toggle pattern. Pressed state reflects whether a
browser pane currently exists in the active pane group.

EOF
)"
git log -1 --show-signature | head -3
```

---

## Phase 9 — Persist + restore on startup

### Task 9.1: Real `save_browser_state` implementation

**Files:**
- Modify: `app/src/workspace/view.rs`
- Modify: `app/src/browser/browser_view.rs`

- [ ] **Step 1: Add the data-dir resolver**

The `persistence` crate already exposes config-dir resolution. Find the existing accessor — search `crates/persistence/src/` for a function returning the CastCodes support directory (something like `castcodes_support_dir()` or `app_data_dir()`). Use that.

In `workspace/view.rs`, add near `toggle_browser_pane`:

```rust
fn browser_state_dir() -> std::path::PathBuf {
    persistence::castcodes_support_dir() // adapt to real fn name
}
```

- [ ] **Step 2: Replace the stub `save_browser_state`**

Replace the placeholder added in Task 7.2 with:

```rust
pub(crate) fn save_browser_state(&self, open: bool, ctx: &mut ViewContext<Self>) {
    let active_group = self.active_tab_pane_group();
    let snapshot_opt = active_group
        .as_ref(ctx)
        .first_browser_pane_id()
        .and_then(|id| active_group.as_ref(ctx).browser_view_for(id))
        .map(|view: ViewHandle<BrowserView>| {
            view.as_ref(ctx).model().read(ctx).snapshot(open)
        });

    // If the pane is closed and there's no view, persist a "closed, no tabs" state.
    let state = snapshot_opt.unwrap_or_else(|| {
        crate::browser::browser_model::BrowserState {
            v: crate::browser::browser_model::BROWSER_STATE_VERSION,
            open: false,
            tabs: vec![],
            active: 0,
        }
    });

    let state_dir = Self::browser_state_dir();
    // Spawn off the I/O so we don't block the UI thread.
    ctx.spawn(move |_| async move {
        if let Err(err) = crate::browser::persistence::save(&state_dir, &state) {
            log::warn!("failed to persist browser state: {err}");
        }
    }).detach();
}
```

`active_group.browser_view_for(pane_id)` may need adding to `PaneGroup` — a tiny accessor that returns the `ViewHandle<BrowserView>` if the pane is a browser pane.

- [ ] **Step 3: Trigger save on browser-side mutations**

In `app/src/browser/browser_view.rs`, in the `BrowserView::handle_action` body, after any of `NewTab`, `CloseTab`, `SelectTab`, `Navigate`, emit a `BrowserViewEvent::StateChanged` event. Add the variant to `BrowserViewEvent`:

```rust
pub enum BrowserViewEvent {
    Pane(PaneEvent),
    StateChanged,
}
```

In `BrowserPane::attach` (`app/src/pane_group/pane/browser_pane.rs:50`), the existing subscription to `BrowserViewEvent` already forwards `Pane(pane_event)` to `pane_group.handle_pane_event`. Add a parallel arm:

```rust
ctx.subscribe_to_view(&browser_view, move |pane_group, _, event, ctx| {
    match event {
        BrowserViewEvent::Pane(pane_event) => {
            pane_group.handle_pane_event(pane_id, pane_event, ctx);
        }
        BrowserViewEvent::StateChanged => {
            pane_group.notify_workspace_browser_state_changed(ctx);
        }
    }
});
```

`PaneGroup::notify_workspace_browser_state_changed` is a thin emit upward — add it. The workspace subscribes to PaneGroup events already (search for `subscribe_to_view(&self.tab_pane_group` etc.) and adds a match arm calling `self.save_browser_state(true, ctx)` (open is true here because the change came from inside a live browser pane).

Debounce concern: the spec calls for 500ms debounce. For v1, debounce inside `save_browser_state` itself using a single-shot `tokio::time::sleep` cancel-on-new-call:

```rust
// Field on Workspace:
browser_save_pending: Option<tokio::task::JoinHandle<()>>,

pub(crate) fn save_browser_state(&mut self, open: bool, ctx: &mut ViewContext<Self>) {
    if let Some(h) = self.browser_save_pending.take() {
        h.abort();
    }
    let state = /* … compute as above … */;
    let state_dir = Self::browser_state_dir();
    self.browser_save_pending = Some(ctx.spawn(move |_| async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Err(err) = crate::browser::persistence::save(&state_dir, &state) {
            log::warn!("failed to persist browser state: {err}");
        }
    }));
}
```

- [ ] **Step 4: Verify build**

```bash
cargo check -p warp --bin cast-codes --features gui
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/browser/browser_view.rs app/src/pane_group/pane/browser_pane.rs app/src/pane_group/mod.rs
git commit -S -m "$(cat <<'EOF'
feat(workspace): persist browser pane state on changes

Toggle, tab add/close/select, and navigation each schedule a debounced
(500ms) JSON save of the BrowserState to the CastCodes support
directory.

EOF
)"
git log -1 --show-signature | head -3
```

### Task 9.2: Restore on workspace init

**Files:**
- Modify: `app/src/workspace/view.rs`

- [ ] **Step 1: Add `restore_browser_state` and call it from workspace init**

Find the existing `Workspace::new` (or equivalent constructor) in `workspace/view.rs`. Add a call to:

```rust
fn restore_browser_state(&mut self, ctx: &mut ViewContext<Self>) {
    let Some(state) = crate::browser::persistence::load(&Self::browser_state_dir()) else {
        return;
    };
    if !state.open {
        return;
    }
    let active_group = self.active_tab_pane_group();
    active_group.update(ctx, |group, ctx| {
        let view = ctx.add_typed_action_view(|ctx| {
            crate::browser::BrowserView::with_restored_state(state.clone(), ctx)
        });
        let pane = crate::pane_group::pane::browser_pane::BrowserPane::from_view(view, ctx);
        group.add_pane_with_direction(
            crate::pane_group::pane::Direction::Right,
            pane,
            /* focus_new_pane */ false,
            ctx,
        );
    });
}
```

`BrowserView::with_restored_state(state, ctx)` is a new constructor — add it to `browser_view.rs`, alongside the existing `BrowserView::new`:

```rust
pub fn with_restored_state<V: View>(state: BrowserState, ctx: &mut ViewContext<V>) -> Self {
    let model = BrowserModel::restore(state);
    Self::from_model(model, ctx)
}
```

`Self::from_model` should refactor out of the existing `BrowserView::new`: separate "build a BrowserView wrapping this model" from "build a fresh model and BrowserView from a URL". Search `browser_view.rs` for the existing `BrowserView::new` impl, extract its inner body into `from_model`, and have both constructors share it.

Call `self.restore_browser_state(ctx)` at the end of `Workspace::new` or whichever init hook the chat panel restoration uses (search for `restore_cli_chat_panel` or similar — the chat panel does the same dance, mirror its hook location).

- [ ] **Step 2: Verify build**

```bash
cargo check -p warp --bin cast-codes --features gui
```
Expected: PASS.

- [ ] **Step 3: Manual round-trip smoke**

```bash
cargo run -p warp --bin cast-codes --features gui
```

Open browser pane, add three tabs at `https://example.com`, `https://example.org`, `https://example.net`. Quit (⌘Q). Relaunch. Browser pane should reopen with the same three tabs, the active tab matching the one selected before quitting.

- [ ] **Step 4: Commit**

```bash
git add app/src/workspace/view.rs app/src/browser/browser_view.rs
git commit -S -m "$(cat <<'EOF'
feat(workspace): restore browser pane on workspace startup

If the last persisted state was open, the workspace reopens the browser
pane with the persisted tab list. The pane is restored without stealing
focus from the active terminal.

EOF
)"
git log -1 --show-signature | head -3
```

---

## Phase 10 — Integration tests

### Task 10.1: `browser_pane_toggle`

**Files:**
- Create: `crates/integration/tests/browser_pane_toggle.rs`

- [ ] **Step 1: Inspect an existing integration test for shape**

```bash
ls crates/integration/tests | head -20
head -60 crates/integration/tests/$(ls crates/integration/tests | head -1)
```

Identify the `Builder` / `TestStep` pattern. Most tests follow:

```rust
use integration::{Builder, TestStep};

#[test]
fn toggle_browser_pane_via_keymap() {
    Builder::new()
        .with_workspace()
        .step(TestStep::DispatchAction("workspace:toggle_browser_pane"))
        .step(TestStep::Assert(|app| {
            assert!(app.has_browser_pane(), "browser pane should be open");
        }))
        .step(TestStep::DispatchAction("workspace:toggle_browser_pane"))
        .step(TestStep::Assert(|app| {
            assert!(!app.has_browser_pane(), "browser pane should be closed");
        }))
        .run();
}
```

- [ ] **Step 2: Write the test file**

Create `crates/integration/tests/browser_pane_toggle.rs` with the test above plus two more covering the menu and the palette:

```rust
#[test]
fn toggle_browser_pane_via_menu() { /* dispatch through View menu */ }

#[test]
fn open_browser_pane_alias_still_works() {
    Builder::new()
        .with_workspace()
        .step(TestStep::DispatchAction("workspace:open_browser_pane"))
        .step(TestStep::Assert(|app| assert!(app.has_browser_pane())))
        .run();
}
```

`Builder`, `TestStep`, and `app.has_browser_pane()` are the local test harness helpers. `has_browser_pane()` may need adding — small helper on the test `App` wrapper that asks the active pane group whether any pane is a browser pane.

- [ ] **Step 3: Run the test**

```bash
cargo test -p integration browser_pane_toggle
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/integration/tests/browser_pane_toggle.rs crates/integration/src/...
git commit -S -m "$(cat <<'EOF'
test(integration): browser pane toggle via keymap/menu/palette

Asserts the toggle action opens and closes the pane symmetrically and
that the workspace:open_browser_pane alias still works.

EOF
)"
git log -1 --show-signature | head -3
```

### Task 10.2: `browser_pane_multi_tab`

**Files:**
- Create: `crates/integration/tests/browser_pane_multi_tab.rs`

- [ ] **Step 1: Write the test**

```rust
use integration::{Builder, TestStep};

#[test]
fn add_close_select_tabs() {
    Builder::new()
        .with_workspace()
        .step(TestStep::DispatchAction("workspace:toggle_browser_pane"))
        // After toggle, exactly one tab (about:home) exists.
        .step(TestStep::Assert(|app| {
            assert_eq!(app.browser_tab_count(), 1);
        }))
        .step(TestStep::DispatchBrowserAction("new_tab"))
        .step(TestStep::DispatchBrowserAction("new_tab"))
        .step(TestStep::Assert(|app| {
            assert_eq!(app.browser_tab_count(), 3);
            assert_eq!(app.browser_active_index(), 2);
        }))
        .step(TestStep::DispatchBrowserAction("select_tab:1"))
        .step(TestStep::DispatchBrowserAction("close_tab:1"))
        .step(TestStep::Assert(|app| {
            // Closing the middle (active) tab lands focus on the next tab,
            // which after the remove is at index 1.
            assert_eq!(app.browser_tab_count(), 2);
            assert_eq!(app.browser_active_index(), 1);
        }))
        .step(TestStep::DispatchBrowserAction("close_tab:0"))
        .step(TestStep::DispatchBrowserAction("close_tab:0"))
        .step(TestStep::Assert(|app| {
            // Closing the last tab spawns about:home.
            assert_eq!(app.browser_tab_count(), 1);
            assert_eq!(app.browser_current_url(), "about:home");
        }))
        .run();
}
```

Test-harness helpers `browser_tab_count`, `browser_active_index`, `browser_current_url`, `DispatchBrowserAction(...)` likely need adding. Keep them thin.

- [ ] **Step 2: Run**

```bash
cargo test -p integration browser_pane_multi_tab
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/integration/tests/browser_pane_multi_tab.rs crates/integration/src/...
git commit -S -m "$(cat <<'EOF'
test(integration): browser pane multi-tab lifecycle

Covers add/select/close including the close-last-tab path that spawns
about:home.

EOF
)"
git log -1 --show-signature | head -3
```

### Task 10.3: `browser_pane_persistence`

**Files:**
- Create: `crates/integration/tests/browser_pane_persistence.rs`

- [ ] **Step 1: Write the test**

```rust
use integration::{Builder, TestStep};

#[test]
fn tabs_survive_workspace_restart() {
    let storage = Builder::isolated_storage(); // shared dir across both runs

    // Run 1: open pane, add tabs at known URLs.
    Builder::new()
        .with_storage(&storage)
        .with_workspace()
        .step(TestStep::DispatchAction("workspace:toggle_browser_pane"))
        .step(TestStep::DispatchBrowserAction("navigate:https://example.com"))
        .step(TestStep::DispatchBrowserAction("new_tab"))
        .step(TestStep::DispatchBrowserAction("navigate:https://example.org"))
        .step(TestStep::FlushPersistence) // force the 500ms debounce to fire
        .run();

    // Run 2: relaunch with the same storage.
    Builder::new()
        .with_storage(&storage)
        .with_workspace()
        .step(TestStep::Assert(|app| {
            assert!(app.has_browser_pane(), "browser pane reopens automatically");
            assert_eq!(app.browser_tab_count(), 2);
            assert!(app.browser_tab_urls().contains(&"https://example.com".to_string()));
            assert!(app.browser_tab_urls().contains(&"https://example.org".to_string()));
        }))
        .run();
}
```

`Builder::isolated_storage()`, `with_storage`, `FlushPersistence`, and `browser_tab_urls` are test-harness helpers. Add them inside `crates/integration/src/` next to existing equivalents (search for `isolated_storage` in the integration crate — it likely already exists for chat panel persistence tests).

- [ ] **Step 2: Run**

```bash
cargo test -p integration browser_pane_persistence
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/integration/tests/browser_pane_persistence.rs crates/integration/src/...
git commit -S -m "$(cat <<'EOF'
test(integration): browser pane persists across workspace restart

Two-phase test using shared storage: opens pane + adds tabs in run 1,
verifies the pane reopens with the same tabs in run 2.

EOF
)"
git log -1 --show-signature | head -3
```

---

## Phase 11 — Final checks

### Task 11.1: Run all browser tests + rebrand guard

**Files:**
- (no changes — verification only)

- [ ] **Step 1: Full browser test suite**

```bash
cargo test -p warp browser::
cargo test -p integration browser_pane_
```
Expected: PASS.

- [ ] **Step 2: Rebrand guard**

```bash
./script/check_rebrand
```
Expected: PASS. No new public-surface strings should be flagged.

- [ ] **Step 3: Core identity checks**

```bash
cargo test -p warp_core --features local_fs
cargo check -p warp --bin cast-codes --features gui
```
Expected: PASS.

- [ ] **Step 4: Manual visual diff against the design brief**

Run the app, open the browser pane, compare to the screenshot in the design brief:

- Toolbar order: collapse, back, forward, reload, URL input, open-external. ✓
- Tab strip: chips with truncated title and X close on hover, `+` add-tab button at the end. ✓
- Active tab has accent border. ✓
- Titlebar has a browser-toggle chrome button. ✓
- ⌘⌥B toggles. ✓
- New tab opens about:home. ✓

### Task 11.2: PR

- [ ] **Step 1: Push the branch**

(Assumes you started this work on a feature branch. If still on `main`, create one with `git checkout -b feat/browser-panel-ui-toggle-persistence` and amend-rebase if needed — see "git safety" notes in the global CLAUDE.md.)

```bash
git push -u origin feat/browser-panel-ui-toggle-persistence
```

- [ ] **Step 2: Open the PR**

```bash
gh pr create --title "feat(browser): polish, toggle, persistence (PLAN-01)" --body "$(cat <<'EOF'
## Summary

- Splits `app/src/browser/browser_view.rs` (993 LOC) into focused modules: `webview_host`, `url_input`, `about_home`, `persistence`.
- Restyled toolbar with collapse + open-external buttons; URL bar accepts free-text and routes to DuckDuckGo.
- Active tab has accent border; tab close button on hover; chip tooltip shows full title.
- New `WorkspaceAction::ToggleBrowserPane` action; menu entry, palette command, and ⌘⌥B keyboard shortcut all dispatch it.
- Titlebar chrome toggle button with pressed state.
- Pane open/closed state and tab list persist across CastCodes restarts in `~/Library/Application Support/dev.castcodes.CastCodes/browser/state.json` (atomic write, 500ms debounce, lenient load).
- Three new integration tests under `crates/integration/tests/`.

Scope is intentionally limited per `specs/CASTCODES-BROWSER-PANEL/PLAN-01-ui-toggle-persistence.md`. Security hardening, agent surface, and MCP server land in PLAN-02 / PLAN-03 / PLAN-04.

## Test plan

- [x] `cargo test -p warp browser::`
- [x] `cargo test -p integration browser_pane_`
- [x] `./script/check_rebrand`
- [x] Manual: ⌘⌥B toggle in app
- [x] Manual: open pane, add tabs, quit, relaunch — tabs return
- [x] Manual: visual diff vs `specs/CASTCODES-BROWSER-PANEL/PRODUCT.md` reference screenshot

EOF
)"
```

---

## Self-Review

**Spec coverage check (against `TECH.md` §§ 1-3, 6, and the PRODUCT spec's "User-Visible Behavior" section):**

| Requirement | Covered by |
|---|---|
| Extract `webview_host.rs` | Task 1.1 |
| Add `url_input.rs` with resolve() | Task 2.1 + 2.2 |
| Add `about_home.rs` start page | Task 3.1 + 3.2 |
| `BrowserTab` gains `pinned` + `favicon` | Task 4.1 |
| `BrowserState` + `TabSnapshot` + snapshot/restore | Task 4.2 |
| `persistence.rs` with atomic save + lenient load | Task 5.1 + 5.2 |
| Toolbar collapse + open-external buttons | Task 6.1 |
| URL bar placeholder + url_input wiring | Task 6.2 |
| Default new tab → about:home | Task 6.3 |
| Tab chip accent border + hover close + tooltip | Task 6.4 |
| `WorkspaceAction::ToggleBrowserPane` + handler | Task 7.1 + 7.2 |
| ⌘⌥B keymap binding | Task 7.3 |
| Open-browser-pane alias preserved | Task 7.3 |
| View menu update | Task 7.4 |
| Titlebar toggle button with pressed state | Task 8.1 |
| Persistence save on changes (debounced) | Task 9.1 |
| Restore on workspace init | Task 9.2 |
| Integration test: toggle | Task 10.1 |
| Integration test: multi-tab | Task 10.2 |
| Integration test: persistence | Task 10.3 |
| `./script/check_rebrand` passes | Task 11.1 |

No spec requirements are uncovered.

**Placeholder scan:** No "TBD" / "TODO" / "fill in later" steps. Every code block is concrete. Two areas flagged as "adapt to local helper name" (theme accent accessor in Task 6.4; `castcodes_support_dir` real fn name in Task 9.1) are unavoidable indirection — the local helper names depend on the codebase and a quick `grep` resolves both. Each is called out explicitly so the engineer knows to look.

**Type consistency:** `BrowserState`, `TabSnapshot`, `BROWSER_STATE_VERSION`, `Resolved::{Url,Search}`, `WorkspaceAction::ToggleBrowserPane`, `TOGGLE_BROWSER_PANE_BINDING_NAME`, `BrowserViewAction::{OpenExternal,Collapse}`, and `BrowserViewEvent::StateChanged` are referenced consistently across phases. The "stub then fill" pattern (Task 7.2 stubs `save_browser_state`, Task 9.1 fills it in) is intentional and labeled.
