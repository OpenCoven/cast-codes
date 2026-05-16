# Tech Spec: CastCodes Browser Panel

See `specs/CASTCODES-BROWSER-PANEL/PRODUCT.md` for product behavior and success criteria.

## 1. Module Layout

`app/src/browser/`

```
mod.rs                 (existing — re-export new modules)
browser_model.rs       (existing — extend per §2)
browser_view.rs        (existing — restyle + slim per §3, target < 600 lines)
webview_host.rs        NEW: NativeBrowserWebView + AppKit child-view plumbing
url_input.rs           NEW: URL-or-search parser (pure functions)
agent_api.rs           NEW: screenshot / evaluate / pick / capture implementations
agent_mcp.rs           NEW: in-process MCP server wrapping agent_api
persistence.rs         NEW: tab list serialization
about_home.rs          NEW: built-in start page (data: URL or static asset)
```

`browser_view.rs` today is 993 lines and conflates rendering, webview lifecycle, AppKit handle plumbing, action dispatch, and the title channel. Splitting `webview_host.rs` and `agent_api.rs` out is part of this work — required, not optional, because v1 cannot land cleanly otherwise.

Boundaries:

- `browser_model` — pure state; no `wry`, no UI imports.
- `webview_host` — owns one `wry::WebView`; exposes `attach`, `set_bounds`, `navigate`, `eval`, `screenshot`, `set_visible`.
- `browser_view` — orchestrates the warpui tree; holds `BrowserModel` and `Vec<NativeBrowserWebView>`.
- `agent_api` — operates on `&webview_host::NativeBrowserWebView`; returns `BrowserAgentResponse`.
- `agent_mcp` — protocol adapter only; no business logic.

## 2. Tab Model Extensions

`browser_model.rs`

Extend `BrowserTab`:

```rust
pub struct BrowserTab {
    id: TabId,
    current_url: String,
    back_history: Vec<String>,
    forward_history: Vec<String>,
    loading: bool,
    title: String,
    favicon: Option<String>,   // NEW — URL string; None until page reports one
    pinned: bool,              // NEW — survives "close all"; v1 has no UI gesture yet
}
```

Add serialization types:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BrowserState {
    pub v: u32,
    pub open: bool,
    pub tabs: Vec<TabSnapshot>,
    pub active: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TabSnapshot {
    pub url: String,
    pub title: String,
    pub pinned: bool,
}

impl BrowserModel {
    pub fn snapshot(&self, open: bool) -> BrowserState { … }
    pub fn restore(state: BrowserState) -> Self { … } // unknown v → fresh
}
```

History is intentionally not persisted; spec calls this out as documented behavior.

## 3. View Layer Changes

`browser_view.rs`

### Toolbar restyle

- Single 48px row: `[collapse] [◀] [▶] [⟳] [URL input ……………] [↗]`.
- Reuse `icon_button_with_color` from `ui_components/buttons`.
- URL input is the existing single-line editor (`SingleLineEditorOptions`). Add placeholder `URL or search the web`. On submit, run the new `url_input::resolve(&input)` and dispatch `BrowserViewAction::Navigate { tab: None, url }`.
- Open-external button calls `opener::open(active.current_url())`.

### Tab strip

- Existing tab strip stays. Adjust chip styling to match the reference: 4px corner radius, accent border on active, hover-only close button, hover tooltip showing full title.
- Loading dot animation: hand off to `webview_host`'s `is_loading` signal (already plumbed via the title channel).
- `+` button at the right end dispatches `BrowserViewAction::NewTab { url: None }` which defaults to `about:home`.

### Titlebar toggle button

Follow the chat panel toggle pattern from commit `768c1289` (touches `app/src/app_menus.rs`, view layer adds a toggle action mirrored to a menu entry). Implementation steps:

1. Add `BrowserViewAction::TogglePane` (or reuse the existing `workspace:open_browser_pane` dispatch made idempotent).
2. Register a titlebar chrome button via the existing `header_toolbar_item` system (`app/src/workspace/header_toolbar_item.rs`).
3. Mirror the action in `app/src/app_menus.rs` under `View →`.
4. Pressed state derived from `BrowserModel`'s open/closed flag.

### Keymap

Add `cmd-alt-b` → `workspace:toggle_browser_pane` in `crates/warpui_core/src/keymap/`. Rename the existing `workspace:open_browser_pane` action to `workspace:toggle_browser_pane`; add an alias for the old name.

### Action surface

```rust
pub enum BrowserViewAction {
    // existing
    Back, Forward, Reload,
    NewTab,                         // → NewTab { url: None }
    CloseTab(usize), SelectTab(usize),

    // extended / new
    Navigate { tab: Option<TabId>, url: String },
    NewTabWithUrl { url: String },
    ReloadTab { tab: Option<TabId> },
    Screenshot { tab: Option<TabId>, full_page: bool },
    EvaluateJs { tab: Option<TabId>, script: String },
    CaptureConsole { tab: Option<TabId>, subscribe: bool },
    CaptureNetwork { tab: Option<TabId>, subscribe: bool },
    PickElement { tab: Option<TabId> },
    ListTabs,
}
```

Each agent-facing action returns a `BrowserAgentResponse` via a oneshot channel embedded in the action's metadata so the action handler can await wry's async response.

## 4. WebView Host

`webview_host.rs`

Move the current `NativeBrowserWebView` struct here. Add:

```rust
impl NativeBrowserWebView {
    fn screenshot(&self, full_page: bool) -> Result<Vec<u8>, BrowserError> { … }
    fn evaluate_script_with_result(&self, script: &str) -> Result<serde_json::Value, BrowserError> { … }
    fn install_console_capture(&self) -> async_channel::Receiver<ConsoleEvent> { … }
    fn install_network_capture(&self) -> async_channel::Receiver<NetworkEvent> { … }
    fn install_picker(&self) -> async_channel::Receiver<PickedElement> { … }
    fn set_devtools_enabled(&mut self, enabled: bool) { … }
}
```

Builder configuration applied at construction:

```rust
WebViewBuilder::new_as_child(&parent)
    .with_data_directory(per_workspace_data_dir())
    .with_url(initial_url)
    .with_devtools(settings.devtools_enabled)
    .with_new_window_req_handler(|req| {
        if matches!(scheme(&req.uri), "mailto" | "tel" | "castcodes") {
            opener::open(req.uri); ResponseAction::Deny
        } else {
            dispatch(BrowserViewAction::NewTabWithUrl { url: req.uri });
            ResponseAction::Deny
        }
    })
    .with_web_resource_request_handler(blocklist::matcher())
    .build()?
```

If `wry::WebView::screenshot()` isn't available in the cast-codes lockfile's wry version, raise a `chore(deps): bump wry` PR before this work lands.

## 5. URL Input Parsing

`url_input.rs`

```rust
pub enum Resolved {
    Url(String),
    Search(String),
}

pub fn resolve(raw: &str) -> Resolved { … }
```

Rules (tested):

1. Empty → return `Url("about:home")`.
2. Starts with known scheme (`http`, `https`, `file`, `about`, `data`, `castcodes`) → `Url(raw)`.
3. Looks like a hostname (no spaces, contains `.` or matches `localhost(:port)?`) → prepend `http://` for loopback, `https://` otherwise.
4. Anything else → `Search("https://duckduckgo.com/?q={percent_encoded(raw)}")`.

Pure functions, no I/O, golden-table unit tests.

## 6. Persistence

`persistence.rs`

Path resolution via the existing `persistence` crate (do not duplicate config-dir logic):

- macOS: `~/Library/Application Support/dev.castcodes.CastCodes/browser/state.json`
- Linux: `$XDG_DATA_HOME/cast-codes/browser/state.json`

Write pattern: write to `state.json.tmp`, `fsync`, rename. Debounced 500ms after any of: tab added, tab closed, active changed, navigation completed, pane toggled.

Read pattern: at workspace startup. On parse error or unknown `v`, return `None` and the pane opens fresh — never panic, never block startup.

```rust
pub fn load() -> Option<BrowserState> { … }
pub fn save(state: &BrowserState) { /* spawn debounced task */ }
```

## 7. Security

### App-wide data directory

All `wry::WebView` instances share `state_dir.join("browser/data")` as their `with_data_directory(...)`. This isolates CastCodes from Safari/Chrome cookies but does *not* isolate one workspace from another within CastCodes — v1 ships a single shared bucket. Per-workspace isolation is deferred until cast-codes' `Workspace` struct exposes a stable identity that survives restart (not verified to exist as of this spec).

### Popup policy

Implemented in the `with_new_window_req_handler` shown in §4. All `_blank` and `window.open()` traffic stays in-pane.

### DevTools gating

New setting `browser.devtools_enabled` (boolean, default `false`). Settings UI panel: `Settings → Browser`. Reading the setting at WebView construction is sufficient — toggling at runtime requires rebuilding the WebView for the active tab, which the toggle handler does explicitly.

### Tracker / ad blocklist

- Bundled static blocklist: a curated EasyList network-rules subset (no cosmetic rules), vendored at `app/assets/bundled/blocklists/easylist-network.txt` from a pinned snapshot of `https://easylist.to/easylist/easylist.txt` (network rules only, stripped of cosmetic and element-hiding rules at vendor time). Included via `include_str!`. Target size < 200 KB compiled.
- Parser: `adblock-rust` crate (pure-Rust, no JS engine dep). Build the matcher once at app startup, share via `Arc<Engine>`.
- Application: `with_web_resource_request_handler` calls `engine.check_network_request(...)`; cancel matching subresource requests.
- Refresh policy: manual quarterly. Document `script/refresh_blocklist` shell wrapper for the engineer doing the refresh. Do not auto-download (would violate the CastCodes cloud boundary).
- User override: `Settings → Browser → Block trackers and ads` (default on). When off, the matcher is bypassed.

### Injected scripts

The only script injected by CastCodes runs on `PageLoadEvent::Finished` and handles:

1. Title + favicon reporting.
2. Keyboard shortcut bridge (⌘T → new-tab event).
3. Capture/picker overlays — installed on demand when an agent action requests them, removed when subscription is torn down.

Script body is a string constant. No user-controlled interpolation. Capture shims (`console`, `fetch`) live in the same script.

### Header redaction

Network capture shim drops these header values before emitting:

- `Authorization`, `Cookie`, `Set-Cookie`, `Proxy-Authorization`, `X-Api-Key`, `X-Auth-Token`.

Replaced with `<redacted>` so agents see the shape but not the secret.

## 8. Agent API — In-Process

`agent_api.rs`

Each capability is a free function taking `&webview_host::NativeBrowserWebView` plus parameters, returning `Result<BrowserAgentResponse, BrowserError>`:

```rust
pub enum BrowserAgentResponse {
    Screenshot { png: Vec<u8>, truncated: bool },
    EvalResult(serde_json::Value),
    Picked(PickedElement),
    Stream(StreamHandle),
    Tabs(Vec<TabSnapshot>),
    Ok,
}

pub struct PickedElement {
    pub selector: String,
    pub bbox: (f32, f32, f32, f32),
    pub computed_styles: serde_json::Value,
    pub outer_html: String,
}
```

### Screenshot

- Viewport-only: `wry::WebView::screenshot()` (requires wry 0.46+).
- Full-page: get `document.documentElement.scrollHeight` via eval; capped at 8000px; if exceeded, set `truncated: true` and capture up to the cap. Stitch tiles via `image` crate.

### Element picker

- Inject an overlay listener that listens for the next `mouseover` (highlights) and `click` (commits).
- Generate selector via a small JS function in the same injected script (`finder.js`-style algorithm, vendored as a string constant — small and well-understood).
- Commit returns `{ selector, bbox, computed_styles, html }` over the title channel pattern. Picker removes itself after commit or after a 60s timeout.

### Console + network tail

- Pull model (not streaming) — agent calls `browser.console.tail(tab?, since?)` and gets new events since the cursor. Simpler client integration than long-lived streams.
- Buffer per tab is bounded (e.g. last 500 events); older events drop. Document the bound.

## 9. Agent API — MCP Server

`agent_mcp.rs`

- Spawn as a warpui background task at workspace startup. Stop on workspace shutdown.
- Transport: Unix domain socket. Path:
  - macOS: `~/Library/Application Support/dev.castcodes.CastCodes/mcp/browser.sock`
  - Linux: `$XDG_RUNTIME_DIR/cast-codes/browser.sock`
- File mode `0600` — owner-only.
- Server writes the socket path + tool surface version to `~/.cast-codes/mcp.json`. Standard MCP discovery hook. (When per-workspace isolation lands, the discovery file grows to map workspace identity → socket path; not in v1.)
- Reuse the MCP crate cast-codes already uses for plugins. (Confirm during implementation — likely `rmcp` based on Rust ecosystem; pick whichever crate the plugin host already imports.)

### Tool surface

| Tool | Maps to action |
|---|---|
| `browser.list_tabs` | `ListTabs` |
| `browser.navigate(url, tab?)` | `Navigate` |
| `browser.new_tab(url?)` | `NewTab` / `NewTabWithUrl` |
| `browser.reload(tab?)` | `ReloadTab` |
| `browser.screenshot(tab?, full_page?)` | `Screenshot` → base64 PNG content block |
| `browser.evaluate(script, tab?)` | `EvaluateJs` → JSON |
| `browser.pick_element(tab?)` | `PickElement` → JSON |
| `browser.console.tail(tab?, since?)` | `CaptureConsole(subscribe=true)` + cursor read |
| `browser.network.tail(tab?, since?)` | `CaptureNetwork(subscribe=true)` + cursor read |

Each tool handler builds the action, awaits the oneshot response, serializes to MCP `content` blocks. No business logic in this layer.

## 10. Testing

### Pure-Rust unit tests

- `browser_model::tests` — extend existing tests with `snapshot()`/`restore()` round-trip, pinned-tab survival, version-mismatch handling, history-not-persisted assertion.
- `url_input::tests` — golden table covering URL/hostname/loopback/search.
- `persistence::tests` — temp-file rename atomicity; malformed JSON yields `None`; unknown `v` yields `None`.
- `blocklist::tests` — fixture of URLs known to be in the bundled list; assert blocked.

### Integration tests

Use the Warp Builder/TestStep harness in `crates/integration`:

- `crates/integration/tests/browser_pane_toggle.rs` — keymap, titlebar button pressed state, View menu, command palette mirror, persistence of open/closed.
- `crates/integration/tests/browser_pane_multi_tab.rs` — three-tab add/close/select; close-last spawns `about:home`.
- `crates/integration/tests/browser_pane_persistence.rs` — three tabs at known URLs, quit, relaunch, same three tabs reload.

### MCP server tests

- In-process: spin up the server in a test, connect via in-memory transport, call each tool, assert MCP response shape.

### Manual smoke

- `https://example.com`, `http://localhost:3000`, `about:home`.
- DevTools toggle on → ⋮ → Inspect opens devtools. Off → no inspect entry.
- MCP screenshot: connect with Claude Code, call `browser.screenshot`, verify PNG decodes.
- Cookie isolation: set a cookie in Safari at example.com, open the same URL in the CastCodes pane, verify the cookie is absent.
- Rebrand guard: `./script/check_rebrand` passes.

## 11. Risks

1. **wry version skew.** `wry::WebView::screenshot()` requires 0.46+. If lockfile is older, raise a `chore(deps): bump wry` PR first. Verify before starting implementation.
2. **Blocklist staleness.** Bundled list goes stale without refresh. Mitigation: quarterly refresh process documented; opt-out setting; log block-hit count.
3. **Full-page screenshot OOM.** 8000px cap; `truncated: true` flag surfaces the limit to agents.
4. **MCP socket discovery for non-MCP-aware agents.** Documented limitation; tooling that doesn't speak MCP must integrate via warpui actions.
5. **Platform support.** macOS first-class; Linux best-effort; Windows out of scope for v1.
6. **CastCodes cloud-boundary regression.** Mitigations: `about:home` is the new-tab default (not opencoven.ai); blocklist does not auto-download; MCP socket is local-only; `./script/check_rebrand` runs in CI.

## 12. Open Questions Deferred to Implementation Plan

- Exact wry API for request interception on Linux (webkit2gtk `WebContext::register_uri_scheme`) vs macOS (`WKURLSchemeHandler`) — likely a thin platform-specific module.
- Whether `about:home` is served as a `data:` URL or registered as a custom scheme handler. Both work; pick whichever is simpler given the platform plumbing chosen above.
- Specific MCP crate. Reuse whatever the plugin host imports; confirmed during implementation.
- Settings UI layout for `Settings → Browser`. Existing `settings_view` patterns dictate this; not a design choice.
