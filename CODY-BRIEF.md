# cast-codes Agent Brief
**Version:** 1.0 — 2026-05-16
**Repo:** `~/Documents/GitHub/OpenCoven/cast-codes`
**Agent:** Claude Opus 4.7 (Claude Code CLI) or Codex
**Author:** Valentina / OpenCoven

---

## Mission

Two parallel goals, executed in order:

1. **UI/UX Modernization** — Make cast-codes look like a first-class OpenCoven product: sleek, dark, branded, professional
2. **Cast Agent Integration** — Replace Warp Agent with Cast Agent, a Coven-native agent that talks to the Coven Gateway and bridges Coven CLI, Coven sessions, and Comux panes

Do not conflate the two phases. Complete Phase 1 first, verify the build, then proceed to Phase 2.

---

## Phase 1: UI/UX Modernization

### Brand Identity

OpenCoven is dark-first, refined, and purposeful. The aesthetic is closer to a professional tool than a consumer app.

**Color palette:**

| Role | Value |
|------|-------|
| Background | `#0f0f12` |
| Surface | `#161619` |
| Elevated surface | `#1e1e22` |
| Border | `rgba(255,255,255,0.08)` |
| Text primary | `#e8e8ed` |
| Text secondary | `#8e8e9a` |
| Text muted | `#5a5a65` |
| Accent (purple) | `#7c3aed` |
| Accent hover | `#6d28d9` |
| Accent secondary (gold) | `#d4a84b` — use sparingly, highlights only |
| Success | `#22c55e` |
| Warning | `#f59e0b` |
| Error | `#ef4444` |
| Tab bar bg | Background (`#0f0f12`) |
| Active tab bg | Elevated surface |
| Status bar bg | `#0a0a0d` |
| Title bar bg | `#0a0a0d` |

**Motion:** 100–150ms ease-in-out. Fast and clean, not bouncy.
**Radius:** 6–8px interactive elements, 4px small items.
**Reference aesthetic:** tweakcn.com — semantic variable-driven theming, clean hierarchy, no decorative chrome.

---

### 1.1 Theme system (crates/theme)

- Create or rename the primary dark theme to `castcodes-dark`
- Map all brand colors above to GPUI theme color slots
- Make `castcodes-dark` the default theme on first launch
- Preserve all existing theme switching infrastructure — do not remove other themes

---

### 1.2 Tab bar (crates/workspace/src/pane.rs)

- Height: 34px (down from 36px)
- Active tab: elevated surface background + 2px bottom border in `#7c3aed`
- Inactive tabs: no border, text.secondary color, surface hover state
- Remove heavy dividers between tabs — spacing only
- Close button: hidden by default, 12px icon, appears on tab hover only

**Collapsible tab bar (required):**
- Small pill/chevron toggle at the far-left of the tab bar
- Toggle collapses the entire tab bar: height → 0 with overflow hidden, replaced by a 2px `#7c3aed` accent line
- On hover near the accent line, a small floating pill re-appear to expand
- Keybind: `cmd+shift+b` → action `pane::ToggleTabBar`
- State: `tab_bar_collapsed: bool` field on `Pane` struct, initialized `false`
- Persist state in-session (no cross-session persistence required for v1)

---

### 1.3 Title bar (crates/title_bar)

- Background: `#0a0a0d`
- Remove or soften heavy borders — use `rgba(255,255,255,0.06)` max
- Window controls: 4px left margin, vertically centered
- Project name: text.secondary color (not primary — it's context, not focus)
- Branch indicator: pill badge, border-only (no fill), accent purple text

---

### 1.4 Status bar (crates/workspace/src/status_bar.rs)

- Height: 22px
- All items: text.muted default, text.secondary on hover
- Separators: border color at 60% opacity
- Language mode indicator: right-aligned, slightly smaller font size

---

### 1.5 Sidebar (crates/project_panel, crates/outline_panel)

- Background: surface color (not background)
- File tree indent guides: border color at 40% opacity
- Active file: 2px accent purple left bar + elevated surface background
- Hovered file: 4% opacity elevated overlay
- Folder chevrons: 120ms rotation transition
- Section headers: text.muted, UPPERCASE, letter-spacing 0.06em, 10px font size

---

### 1.6 Editor area (crates/editor)

- Gutter background: matches surface (not a distinct color)
- Line numbers: text.muted
- Current line highlight: elevated surface at 60% opacity
- Selection: accent purple at 25% opacity
- Cursor: accent purple, 2px width
- Scrollbar: track transparent, thumb at border color, only visible on hover/scroll

---

### 1.7 Input / Search fields

- Background: surface
- Default border: border color (8% white)
- Focus border: accent purple at 60% opacity
- Placeholder: text.muted
- Border radius: 6px

---

### 1.8 Buttons & interactive elements

| Type | Style |
|------|-------|
| Primary | Accent purple fill, white text, 6px radius |
| Secondary | Surface fill, text.secondary, subtle border |
| Ghost / icon | No background until hover (surface on hover) |
| Destructive | Error red, same shape as primary |

---

### Phase 1 deliverables

- [ ] `castcodes-dark` theme applied with all brand colors
- [ ] Collapsible tab bar implemented and keybind working
- [ ] Title bar, status bar, sidebar, editor gutter all updated
- [ ] `cargo check -p zed` passes
- [ ] `DESIGN-CHANGES.md` at repo root: what changed and why

---

## Phase 2: Cast Agent Integration

### Background

`crates/ai` currently depends on Warp's agent infrastructure:
- `warpui`, `warp_core`, `warp_terminal`, `warp_util`
- `warp_graphql`, `warp_multi_agent_api`

These must be replaced by **Cast Agent** — a Coven-native agent that connects to the Coven Gateway.

**Warp Agent must not be deleted.** Gate it behind a `warp-agent` Cargo feature. The new default is `cast-agent`.

---

### What Cast Agent does

Cast Agent is the substrate manager and AI agent backend for cast-codes. It:

1. **Connects to the Coven Gateway** — the same gateway used by Coven CLI and Comux
2. **Replaces Warp Agent** at all existing call sites in `crates/ai/src/agent/`
3. **Collects substrate context** — the current workspace state sent to the gateway on each agent invocation
4. **Bridges Coven sessions** — can list, open, and close Coven sessions from inside cast-codes
5. **Bridges Comux panes** — if Comux is running, discovers active pane names and CWDs and includes them in context
6. **Degrades gracefully** — if the gateway is unreachable, shows an offline indicator and continues working without crashing

---

### 2.1 Read before writing

Before touching any code, read and understand:

```
crates/ai/Cargo.toml                   — see all warp_* deps
crates/ai/src/agent/                   — existing Warp agent implementation
crates/ai/src/                         — full ai crate structure
```

Also fetch these reference files from the Coven repos:
```
https://raw.githubusercontent.com/OpenCoven/coven/main/src/runtime.ts
https://raw.githubusercontent.com/OpenCoven/coven/main/src/client.ts
```

And check this repo for any `.agents/` skills or docs:
```
.agents/
AGENTS.md
```

---

### 2.2 New crate: crates/cast_agent

Create this crate from scratch:

```
crates/cast_agent/
  Cargo.toml
  src/
    lib.rs          — public API, re-exports AgentBackend trait + CastAgent
    agent.rs        — CastAgent struct, top-level entry point
    substrate.rs    — SubstrateCollector: gathers workspace context
    gateway.rs      — HTTP + WebSocket client for Coven Gateway
    session.rs      — CovenSession: list / open / close via gateway API
    comux.rs        — Comux bridge: Unix socket discovery + pane listing
    config.rs       — reads COVEN_GATEWAY_URL, COVEN_TOKEN, ~/.coven/config.toml
```

**Cargo.toml dependencies:**
```toml
[dependencies]
anyhow.workspace = true
tokio = { workspace = true, features = ["rt-multi-thread", "net"] }
serde.workspace = true
serde_json.workspace = true
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-tungstenite = "0.24"
futures.workspace = true
log.workspace = true
thiserror.workspace = true
dirs.workspace = true
uuid.workspace = true
```

---

### 2.3 AgentBackend trait

Define a trait that the existing Warp call sites can be swapped to. Discover the exact interface by reading `crates/ai/src/agent/` first, then define:

```rust
pub trait AgentBackend: Send + Sync {
    async fn send_message(&self, msg: AgentMessage) -> anyhow::Result<AgentResponse>;
    async fn get_substrate(&self) -> anyhow::Result<Substrate>;
    async fn list_sessions(&self) -> anyhow::Result<Vec<CovenSession>>;
    async fn open_session(&self, name: &str) -> anyhow::Result<CovenSession>;
    async fn close_session(&self, id: &str) -> anyhow::Result<()>;
    fn is_available(&self) -> bool;
    fn agent_name(&self) -> &'static str;
}
```

**Substrate type:**
```rust
pub struct Substrate {
    pub active_file: Option<PathBuf>,
    pub open_panes: Vec<PaneInfo>,
    pub shell_cwd: PathBuf,
    pub git_branch: Option<String>,
    pub recent_errors: Vec<DiagnosticEntry>,
    pub comux_panes: Vec<ComuxPane>,   // empty if Comux not running
}
```

---

### 2.4 Coven Gateway connection

**Config resolution order:**
1. `COVEN_GATEWAY_URL` env var
2. `~/.coven/config.toml` key `gateway_url`
3. Default: `http://localhost:3000`

**Auth resolution order:**
1. `COVEN_TOKEN` env var
2. `~/.coven/token` file (first line)
3. Unauthenticated (degraded mode)

**Health check:** `GET /health` on startup. If non-200 or timeout, set `is_available()` → false and log a warning. Never panic.

---

### 2.5 Feature flag wiring in crates/ai

```toml
[features]
default = ["cast-agent"]
cast-agent = ["dep:cast_agent"]
warp-agent = ["dep:warpui", "dep:warp_core", "dep:warp_graphql",
               "dep:warp_multi_agent_api", "dep:warp_terminal", "dep:warp_util"]
```

In `crates/ai/src/agent/`:
- Find where Warp Agent is constructed
- Wrap with `#[cfg(feature = "cast-agent")]` / `#[cfg(feature = "warp-agent")]`
- Default path instantiates `CastAgent::new()`

---

### 2.6 Comux bridge (crates/cast_agent/src/comux.rs)

- Check for socket at `$COMUX_SOCKET` env var, then `/tmp/comux.sock`
- If found, connect via Unix domain socket
- Send a pane list request following the Comux daemon protocol:
  - See `BunsDev/comux` → `src/daemon/protocol.ts` for message format
  - Request: `{ "type": "list_panes" }`
  - Response: `{ "panes": [{ "id": "...", "cwd": "...", "title": "...", "active": bool }] }`
- If socket not found or connection fails: return empty `Vec<ComuxPane>`, log debug
- Include pane data in substrate context on every agent invocation

---

### 2.7 TUI changes

In the cast-codes agent panel / sidebar wherever the agent UI renders:

- Header: **"Cast Agent"** (not "Warp Agent")
- Connection status pill: green = gateway reachable, amber = offline/degraded
- Sessions section: list of active Coven sessions (name, status, last active timestamp)
- Clicking a session: opens it in a new terminal pane
- Remove or gate all Warp-branded UI labels, icons, and `warp.dev` URLs

**Branding substitutions:**

| Remove | Replace with |
|--------|-------------|
| "Warp Agent" | "Cast Agent" |
| "Warp AI" | "Coven AI" |
| "Warp Drive" | "Coven Gateway" |
| Warp logo/icon | CastCodes icon or Coven `✦` mark |
| `warp.dev` URLs | `opencoven.dev` or remove |

---

### Phase 2 deliverables

- [ ] `crates/cast_agent/` fully implemented (all 6 source files)
- [ ] `crates/ai` defaults to `cast-agent` feature, `warp-agent` gated
- [ ] TUI shows "Cast Agent" branding + live gateway connection status
- [ ] Comux bridge wired (graceful if not running)
- [ ] `cargo check -p zed` passes clean
- [ ] `CAST-AGENT.md` at repo root: architecture overview, config instructions, Comux bridge docs

---

## General constraints (both phases)

- **Do not break the build.** Run `cargo check -p zed` after each significant change.
- **Do not delete existing code** unless it is dead code gated behind the replaced feature.
- **Preserve all existing functionality** — this is additive, not destructive.
- **Leave TODO comments** for anything GPUI can't support natively (e.g. `// TODO(visual): blur not supported in GPUI`).
- **Commit per phase** — at minimum two commits: one for Phase 1, one for Phase 2.
- **If Coven Gateway or Comux is unreachable**, degrade gracefully with a UI indicator. No panics.

---

## Completion messages

After Phase 1:
```
openclaw message send --channel telegram --target '823292124' \
  --message '🎨 Phase 1 complete: cast-codes UI/UX modernization done. OpenCoven brand applied, collapsible tab bar added. cargo check passes. See DESIGN-CHANGES.md.'
```

After Phase 2:
```
openclaw message send --channel telegram --target '823292124' \
  --message '🪄 Phase 2 complete: Cast Agent integrated. Warp Agent replaced, Coven Gateway connected, Comux bridge wired. cargo check passes. See CAST-AGENT.md.'
```

---

*This document is the authoritative brief. Do not ask for clarification — use the information here and the referenced source files to make decisions. If something is genuinely ambiguous, choose the simpler/safer option and leave an inline comment explaining the choice.*
