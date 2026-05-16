# Product Spec: CastCodes Browser Panel

## Problem

CastCodes engineers use the app as a coding harness for agent-driven work. When the agent builds a web experience — UI, layout, an API integration — there is no first-class way for the engineer or the agent to *see* and *interact with* the result inside CastCodes. Today's workflow is: alt-tab to a separate browser, screenshot manually, paste into the chat. That breaks the agent loop and loses fidelity.

A browser panel already exists at `app/src/browser/` (added in commit `390efd42`) with multi-tab support, wry-based WebKit child views, and a View-menu entry. This spec defines the upgrade that turns that pane into a professional design/UI/UX layer agents can drive directly.

## Goals

- The browser panel is a normal CastCodes pane that opens and closes via the titlebar, the View menu, the command palette, or a keyboard shortcut.
- Visually matches the reference screenshot and comux's tab strip / browser bar treatment.
- Embedded webview is secure and isolated by default — no cookie bleed, no popups, no unguarded DevTools, no third-party trackers.
- Agents (Claude Code, Cursor, the built-in CastCodes agent, anything speaking MCP) can drive the browser: navigate, screenshot, evaluate JS, pick elements, tail console + network.
- Tab list and pane open/closed state persist across CastCodes restarts.

## Non-Goals (v1)

- Favicon fetching, bookmarks, history UI, find-in-page, zoom controls.
- Private-browsing mode (per-tab ephemeral data).
- Form auto-fill, recording-to-test, multi-tab orchestration in a single MCP call.
- Windows support. (macOS first, Linux best-effort.)
- Per-workspace tab persistence. (Global to the CastCodes app for v1.)

## User-Visible Behavior

### Toggling the panel

- **Titlebar button** — a chrome button next to existing titlebar items, showing pressed state when the pane is open. Tooltip: "Toggle browser pane (⌘⌥B)".
- **View menu** — `View → Open Browser` (existing) becomes `View → Toggle Browser` and reflects toggle semantics.
- **Command palette** — `workspace:open_browser_pane` (existing) is renamed to `workspace:toggle_browser_pane`; an alias keeps the old name working.
- **Keyboard shortcut** — ⌘⌥B. Confirmed free in cast-codes' keymap.
- **Persistence** — the pane remembers whether it was open at last shutdown and reopens in that state on launch.

### Browser bar (top row of the pane)

Single fixed-height row containing, left to right:

1. **Collapse** — hides the pane (same as titlebar toggle).
2. **Back / Forward / Reload** — wired to the active tab's history. Disabled-state styling when no history.
3. **URL / search input** — full-width text field. On Enter:
   - Parses as URL → navigate.
   - Parses as bare hostname (e.g. `example.com`, `localhost:3000`) → prepend `https://` (or `http://` if loopback) and navigate.
   - Anything else → `https://duckduckgo.com/?q=<encoded query>`. DuckDuckGo chosen because it respects the CastCodes cloud boundary.
   - Placeholder: *"URL or search the web"*.
4. **Open-external** — opens the active tab's URL in the system default browser.

### Tab strip (second row)

- Each tab is a chip showing: a small status dot (spinner while loading, otherwise static), truncated title, and a close (✕) button visible on hover.
- Hover tooltip on a chip shows the tab's full title.
- Active tab is visually elevated; inactive chips are muted.
- A `+` button at the right end of the strip adds a new tab.
- New tabs open at `about:home` — a static built-in start page. Not `opencoven.ai`, per the CastCodes fork-local boundary.

### Webview content

- Fills the remaining pane area.
- Each tab owns its own wry WebView; switching tabs swaps which child view is visible, none are torn down.
- New-window requests (`target="_blank"`, `window.open`) become new tabs in the pane instead of native windows. `mailto:`, `tel:`, and `castcodes:` schemes hand off to the OS.

### Security defaults (every new tab)

- **App-private storage** — cookies, localStorage, IndexedDB live in a CastCodes-private data directory under the app's support folder; never shared with Safari/Chrome. Single data dir per app install for v1; per-workspace isolation is deferred to v2.
- **Popups blocked** — pages can't spawn OS windows; `_blank` links open as tabs in the pane.
- **DevTools off** — disabled by default and gated behind `Settings → Browser → Enable DevTools`. When enabled, a `⋮ → Inspect` overflow item opens devtools for the active tab.
- **Third-party tracker / ad blocking** — built-in EasyList-derived filter applied at the request layer. User can disable via Settings.

## Agent Capabilities

The pane exposes four primary capabilities to coding agents:

1. **Screenshot the active tab.** Viewport or full-page (capped at 8000px tall). PNG bytes returned to the agent.
2. **DOM / element picker.** The agent triggers picker mode; the user hovers and clicks an element; the agent receives the CSS selector, bounding box, computed styles, and outer HTML.
3. **Console + network capture.** The agent subscribes to a tab's events and receives console messages and network requests/responses. Sensitive headers (`Authorization`, `Cookie`, `Set-Cookie`) are redacted at the source.
4. **Navigation + reload API.** `navigate(url)`, `reload()`, `new_tab(url)`, `evaluate(js)`, `list_tabs()`.

Capabilities are exposed two ways:

- **Internally** via warpui actions — the cast-codes UI uses these directly.
- **Externally** via an in-process MCP server bound to a per-workspace Unix domain socket. The socket path is advertised in `~/.cast-codes/mcp.json` so any MCP-aware harness (Claude Code, Cursor, etc.) auto-discovers `browser.*` tools without manual configuration.

The socket is mode `0600` — Unix file permissions are the authentication boundary. No TCP exposure.

## Success Criteria

- The screenshot in the design brief is reproducible in the running app (tab strip, browser bar, panel toggle in titlebar, dark theme matches).
- ⌘⌥B toggles the pane from any focus context.
- Tabs persist across restart: open three tabs at known URLs, quit, relaunch — same three tabs come back.
- A coding agent in Claude Code can call `browser.screenshot` and receive a valid PNG of the active CastCodes browser tab.
- A page in the pane cannot read a cookie set in Safari (verified manually).
- `./script/check_rebrand` passes after the change.

## Validation Loop

1. Manual smoke against `https://example.com`, `http://localhost:3000`, `about:home`.
2. Integration tests in `crates/integration` covering toggle, multi-tab lifecycle, and persistence.
3. MCP server in-process tests exercising every tool.
4. Visual diff against the reference screenshot — no automated tool; the engineer reviewing the PR confirms the chrome matches.
