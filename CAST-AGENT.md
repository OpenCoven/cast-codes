# Cast Agent

Cast Agent is the Coven-native substrate manager and AI agent backend for
CastCodes. It is implemented as a standalone crate in
[`crates/cast_agent`](crates/cast_agent) and is intended to replace the Warp
Agent integration currently embedded in `crates/ai/src/agent/`.

## Status

- ✅ Crate skeleton (`crates/cast_agent`) — `cargo check -p cast_agent`.
- ✅ `crates/ai` facade — `ai::cast_agent::{global, is_available, ...}`,
  gated by the `cast-agent` feature (default-on).
- ✅ Dedicated tokio runtime on a background OS thread
  ([`crates/cast_agent/src/runtime.rs`](crates/cast_agent/src/runtime.rs)).
  Lazy `OnceLock<CastAgentRuntime>` so the UI thread reads `is_available()`
  as a cheap atomic. Periodic 30s health re-probe keeps the bit fresh.
- ✅ Eager runtime boot at app startup
  ([`app/src/lib.rs`](app/src/lib.rs) `run()`) so the first render is free
  of `OnceLock` init overhead.
- ✅ Gateway status pill — small 8px coloured dot in the agent panel
  header
  ([`app/src/ai_assistant/panel.rs`](app/src/ai_assistant/panel.rs)
  `render_gateway_status_pill`). Green when the gateway is reachable,
  amber otherwise; brand colours in
  [`app/src/ai/coven_brand.rs`](app/src/ai/coven_brand.rs)
  (`OPENCOVEN_SUCCESS`/`OPENCOVEN_WARNING`/`OPENCOVEN_MUTED`).
- ✅ Coven Sessions section — read-only list under the transcript in the
  agent panel
  ([`app/src/ai_assistant/panel.rs`](app/src/ai_assistant/panel.rs)
  `render_sessions_section`). Shows name, status dot, and last-active
  timestamp per session. Hidden until the gateway answers at least once.
  Cached snapshot is refreshed on a 60-second background loop on the
  cast_agent runtime; UI reads it sync via a `std::sync::RwLock`.
- ✅ Session click-through — clicking a session row dispatches
  `WorkspaceAction::OpenCovenSessionInNewTab { name, cwd }` which opens a
  new terminal tab whose shell starts at the session's CWD. Tab title is
  prefixed `Coven: <name>` so coven-spawned tabs are visually distinct.
  Rows whose `cwd` is `None` render the same but stay inert. Handler in
  [`app/src/workspace/view.rs`](app/src/workspace/view.rs)
  `add_new_coven_session_tab` bypasses `get_new_tab_startup_directory`
  because the click already specifies where to land.
- ✅ Streaming responses — `GatewayClient::stream_messages` opens a
  WebSocket against `/v1/messages/stream`, sends the initial
  `AgentMessage` as a JSON frame, and surfaces server frames as
  [`MessageChunk`](crates/cast_agent/src/gateway.rs) `Delta` / `Done` /
  `Error` items on a boxed `Stream`. Covered by an in-process stub
  WebSocket server in
  [`crates/cast_agent/tests/streaming.rs`](crates/cast_agent/tests/streaming.rs).
  No UI consumer yet — the agent panel still uses the existing
  non-streaming chat path.
- ✅ Per-call `warp-agent` gating audit — see
  [Feature-gating audit](#feature-gating-audit-warp-agent-vs-cast-agent)
  below. Verdict: of the seven warp_* deps the original scope listed, only
  `warp_multi_agent_api` is actually gateable, and even that needs a
  three-phase setup (extract wire types from public API → add
  cast_agent-native parallel types → optional-ify the dep). The other
  six are shared infrastructure that `crates/ai` requires regardless of
  agent backend. No code changes — this is a roadmap so the next agent
  can pick the right starting point.
- ✅ Host substrate bridge —
  [`CastAgentRuntime::set_host_substrate`](crates/cast_agent/src/runtime.rs)
  lets the host (`app/src`) push the editor-side slice of substrate
  (`active_file`, `open_panes`, `recent_errors`) into an
  `Arc<RwLock<HostSubstrate>>` owned by the runtime.
  [`CastAgentRuntime::build_substrate`](crates/cast_agent/src/runtime.rs)
  overlays it on top of the cast_agent-collected base (shell CWD, git
  branch, Comux panes) for gateway calls. Verified end-to-end by
  [`crates/cast_agent/tests/substrate.rs`](crates/cast_agent/tests/substrate.rs).
  Currently `app/src/lib.rs::run` only pushes a `HostSubstrate::default()`
  baseline; real data sources are landing per-field below.
- ✅ `active_file` publisher —
  [`app/src/code/active_file.rs::active_file_changed`](app/src/code/active_file.rs)
  mirrors every editor focus change into the cast_agent host substrate
  via
  [`update_host_substrate`](crates/cast_agent/src/runtime.rs).
  Patches just the `active_file` field so concurrent publishers (pane
  lifecycle, LSP) keep their slices when they land.
- ✅ `open_panes` publisher —
  [`Workspace::publish_open_panes_to_cast_agent`](app/src/workspace/view.rs)
  walks `self.tabs`, builds a `Vec<PaneInfo>` (id, title, cwd, active
  flag), and pushes it via `update_host_substrate`. Three converging
  refresh paths:
  - **Tab lifecycle** — `activate_tab_internal` (covers open + activate,
    since `add_tab_with_pane_layout` ends by activating) and
    `close_tabs` (covers last-tab-removed).
  - **Active-tab CWD updates** — `ctx.observe(&ActiveSession::handle(...))`
    re-publishes when the focused session's `path_if_local` changes
    (e.g. user `cd`s in the active tab).
  - **Background tabs** — `Workspace::tick_publish_open_panes` schedules
    a 10s `ctx.spawn` + `Timer::after` recursion that re-publishes
    unconditionally. Catches non-focused tabs whose CWD changes
    without flipping focus (e.g. a script `cd`s in a background pane).
  Per-pane `cwd` comes from
  [`PaneGroup::active_session_path`](app/src/pane_group/mod.rs); falls
  back to an empty `PathBuf` for non-local sessions (SSH).
- ✅ `recent_errors` publisher (per-editor) —
  [`LocalCodeEditorView::publish_diagnostics_to_cast_agent`](app/src/code/language_server_extension.rs)
  reads raw `lsp_types::Diagnostic`s from the LSP server for the
  editor's current file, filters to Error+Warning (Info/Hint are too
  noisy for the gateway), and pushes via `update_host_substrate` with
  path-scoped replacement: existing `recent_errors` entries for that
  path are dropped first, then the new ones appended. A 50-entry global
  cap trims the oldest. Hooked into `refresh_diagnostics` so every LSP
  `publishDiagnostics` event for an open code editor updates the
  gateway's view.
- ✅ Cross-server diagnostics collector —
  [`CastAgentDiagnosticsCollector`](app/src/code/cast_agent_diagnostics.rs)
  is a singleton model that subscribes to `LspManagerModel`'s
  `ServerStarted` events at app startup and chain-subscribes to every
  `LspServerModel`'s `LspEvent::DiagnosticsUpdated`. On each event it
  applies the same path-scoped replacement strategy as the per-editor
  publisher, so files **not currently open in a code editor** also
  contribute to `recent_errors`. Both publishers can fire for the same
  path; the calls are idempotent (second overwrites first with
  identical content). Closes the coverage gap left by the per-editor
  publisher.
- ✅ Streaming UI consumer with live rendering —
  [`AIAssistantAction::SendViaCovenGateway`](app/src/ai_assistant/panel.rs)
  reads the agent panel's editor buffer, builds an `AgentMessage`,
  drives a `stream_messages` call on the cast_agent runtime, and
  renders each `MessageChunk::Delta` into a `COVEN STREAM • LIVE`
  section below the transcript as chunks arrive. Bound to
  `cmd+shift+m`; skips when `is_available()` is `false`. Cross-thread
  plumbing: the cast_agent tokio task pushes chunks into a shared
  `Arc<std::sync::Mutex<CovenStreamState>>`; a UI-side poll loop
  drains the buffer every 100ms via `ctx.spawn` + `Timer::after`,
  appends to `text`, calls `ctx.notify()`, and reschedules itself
  while the stream is in flight. Concurrent invocations abort the
  previous tokio task via `JoinHandle::abort` and archive its text
  into a bounded `VecDeque` of up to 5 completed streams, rendered
  dimmed and newest-first above the live section. History persists
  across restarts at
  [`~/.coven/stream-history.json`](app/src/ai_assistant/coven_stream_persist.rs)
  — loaded on panel construction, saved (atomic temp + rename) on
  every archive. Lives outside CastCodes' workspace serialization so
  it follows the user across workspaces.
- 🟡 Phase A (paused, scope re-evaluated) — `LifecycleEventType` was
  internalized in PR #52 (ai-owned mirror at
  `crates/ai/src/agent/action/mod.rs` with bidirectional `From`s +
  `TryFrom<i32>` delegating to the wire enum). Attempting the same
  pattern for `FileContent` / `AnyFileContent` / `SkillReference`
  surfaced that the wire-type leak is **structural**, not concentrated
  in a few re-exports: 50+ `app/src/` files import
  `warp_multi_agent_api` directly (not via `crates/ai` re-exports),
  and the 7163-LOC `app/src/ai/agent/api/` conversion module is a
  symmetric wire-protocol translation layer that fundamentally exists
  to bridge ai's runtime types to/from wire types. Per-type
  internalization doesn't reduce the gate surface meaningfully. The
  Phase A → Phase B → Phase C decomposition from the original audit
  remains directionally correct, but Phase A as "internalize types one
  at a time" is the wrong unit of work. See the **revised roadmap**
  in [Feature-gating audit](#feature-gating-audit-warp-agent-vs-cast-agent)
  below.
- ⏳ Per-call `#[cfg(feature = "warp-agent")]` gating implementation
  (Phase C) — waits on the revised Phase A landing.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  crates/ai  (host)                                          │
│  ────────────                                               │
│  (will hold an Arc<CastAgent> behind the AgentBackend trait │
│   once the feature-flag wiring lands)                       │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│  cast_agent                                                 │
│  ───────────                                                │
│  agent.rs       — CastAgent, AgentBackend trait             │
│  substrate.rs   — Substrate, SubstrateCollector             │
│  gateway.rs     — Coven Gateway HTTP/WebSocket client       │
│  session.rs     — CovenSession + cached SessionStore        │
│  comux.rs       — Comux daemon Unix-socket bridge           │
│  config.rs      — env / ~/.coven/config.toml resolution     │
└────────────────────────────┬────────────────────────────────┘
                             │
                ┌────────────┴────────────┐
                ▼                         ▼
       Coven Gateway              Comux daemon
       (HTTP + WS)                (Unix socket)
```

## Configuration

Resolution order (highest priority first):

### Gateway URL

1. `COVEN_GATEWAY_URL` environment variable.
2. `gateway_url` key in `~/.coven/config.toml`.
3. `http://localhost:3000` (default).

### Token

1. `COVEN_TOKEN` environment variable.
2. `token` key in `~/.coven/config.toml`.
3. First non-empty line of `~/.coven/token`.
4. Unauthenticated (degraded mode).

Example `~/.coven/config.toml`:

```toml
gateway_url = "https://gateway.opencoven.dev"
token = "ck_live_..."
```

## Concept model

CastCodes talks to the Coven Gateway through five concepts. They are
separable on purpose — adding a discrete-job abstraction (Run) later does
not require collapsing the chat-thread abstraction (Conversation), and
neither touches the pane-lane abstraction (Session).

| Concept       | What it is                                                                                       | Identity                                  | Status today                                                |
|---------------|--------------------------------------------------------------------------------------------------|-------------------------------------------|-------------------------------------------------------------|
| Session       | A long-lived named agent **lane** the user sees in the agent panel. Pane-scoped CWD + status.    | `CovenSession.id` (gateway-assigned)      | ✅ Implemented (list/open/close + cached snapshot).         |
| Conversation  | A persistent **chat thread** — the unit of memory carried across turns.                          | `AgentMessage.conversation_id` (opaque)   | 🟡 Threaded through wire types; no list/get endpoint yet.   |
| Run           | A single discrete agent **job** — one user prompt → one streamed agent response, with lifecycle. | `run_id` (proposed; gateway-assigned)     | ⏳ Proposed. No endpoints today; see [API gaps](#api-gaps). |
| Message       | The **transport** carrying a turn within a run (sync POST or streamed WS frames).                | `(conversation_id, sequence)`             | ✅ Implemented (sync + streaming).                          |
| Substrate     | The **editor/workspace context** attached to messages (file, panes, branch, diagnostics, etc.).  | n/a — flows host → cast_agent → gateway   | 🟡 Collected & host-pushed; wire contract to gateway TBD.   |

### Why "Run" and not "Task"

`crates/ai` and the surrounding specs already use "task" (Oz conversation
state, `task.AgentConversationID`) and "run" (`oz run list`, multi-agent
orchestration). The Coven Gateway's discrete-job abstraction is at a
different layer — agent-panel-side, single-conversation, single-prompt —
and would collide with both if it also called itself "task". We use **Run**
inside cast_agent docs and pair it explicitly with `cast-agent-run-` /
`run_id` to disambiguate from `oz run` if the two ever surface side by side
in a UI.

### Session vs Conversation vs Run

The three are orthogonal and can coexist freely:

- A **Session** can host any number of **Conversations** (the lane is a
  shell-style pane; the chat thread is the agent's memory).
- A **Conversation** can be made up of multiple **Runs** (each turn is one
  Run; resume = a new Run with the same `conversation_id`).
- A **Run** carries one **Message** in, and one stream of **MessageChunks**
  out, with one **Substrate** snapshot attached.

For today's UI (chat-only, panel-scoped) Sessions + Conversations + Messages
are enough. Runs become necessary the moment the UI needs cancel, resume
after restart, artifacts, or "show me the agent's last 5 jobs".

## API contract

All endpoints are rooted at `CastAgentConfig::gateway_url` (see
[Configuration](#configuration)).

- **Auth.** `Authorization: Bearer <token>` when `CastAgentConfig::token`
  is set; omitted otherwise. The gateway is expected to accept
  unauthenticated requests in local-dev mode and reject them in hosted mode.
- **Content type.** `application/json` for HTTP bodies; UTF-8 JSON text
  frames for WebSocket.
- **Error envelope.** HTTP errors surface via `reqwest`'s
  `error_for_status` (4xx/5xx → `Err`). WebSocket errors arrive as
  `MessageChunk::Error { conversation_id, message }` followed by a clean
  server close.

Each subsection below tags status:

- ✅ **Implemented** — wired in `crates/cast_agent/src/gateway.rs` and
  exercised by tests / live UI.
- 🟡 **Partial** — types exist, wire format is documented, but consumer or
  producer is incomplete.
- ⏳ **Proposed** — not implemented; documented here as the contract the
  next implementation agent should target.

### Health

#### `GET /health` — ✅ Implemented

- **Request:** none.
- **Response:** any 2xx body is treated as healthy; body content is ignored.
- **Used by:** `GatewayClient::health_probe`, called at startup and on a
  30 s background loop. Drives `CastAgent::is_available()` and the panel's
  gateway status pill.

### Messages (chat transport)

These are the **transport** primitives. They carry one turn at a time and
are scoped by `conversation_id`. There is no session_id or run_id on the
wire today.

#### `POST /v1/messages` — ✅ Implemented (no UI consumer)

Single-shot chat round-trip.

```jsonc
// Request: AgentMessage
{
  "conversation_id": "string",
  "body": { /* provider-shaped, opaque to cast_agent */ }
}

// Response: AgentResponse
{
  "conversation_id": "string",
  "body": { /* provider-shaped, opaque to cast_agent */ }
}
```

- **Errors:** 4xx/5xx → `Err`. No structured error envelope today.
- **Substrate:** not attached on the wire yet (host pushes substrate
  into cast_agent locally, but cast_agent does not forward it). See
  [API gaps](#api-gaps).
- **Why no UI consumer:** the panel uses the WebSocket path
  (`stream_messages`) so deltas can render live. `POST /v1/messages` is
  retained for headless callers and tests.

#### `WS /v1/messages/stream` — ✅ Implemented (live in panel)

Streaming chat. Client opens a WebSocket, sends one `AgentMessage` JSON
text frame, then receives one `MessageChunk` per text frame until the
server cleanly closes.

```jsonc
// Server → Client frames (one per WS text message):
{ "type": "delta", "conversation_id": "...", "content": "partial text" }
{ "type": "done",  "conversation_id": "..." }
{ "type": "error", "conversation_id": "...", "message": "..." }
```

- **Tagging:** Serde-tagged enum on `type`, lowercased; see
  [`MessageChunk`](crates/cast_agent/src/gateway.rs).
- **End-of-stream:** a `done` frame OR an `error` frame OR a clean server
  close OR a transport failure. Callers must not assume `done` is always
  the last frame.
- **Ping/pong:** handled by `tokio-tungstenite`; binary frames are
  ignored.
- **Cancellation (today):** the client aborts the local
  `tokio::task::JoinHandle` that drives the read loop, then drops the
  socket. There is **no server-side cancel**: the server may keep working
  on the prompt and bill for it. See [API gaps](#api-gaps).
- **Resume:** posting another `AgentMessage` with the same
  `conversation_id` continues the thread. The gateway is responsible for
  reconstructing memory.

### Sessions (lane management)

A Session is a named lane the user sees in the panel. Pane-scoped, not
conversation-scoped — opening a session does **not** create a
conversation.

#### `GET /v1/sessions` — ✅ Implemented

Returns the array of active sessions. Used by the panel's "Coven
Sessions" list and the 60 s background refresh loop.

```jsonc
// Response: CovenSession[]
[
  {
    "id":          "string",
    "name":        "string",
    "status":      "active" | "idle" | "closed",
    "last_active": "RFC3339 timestamp" | null,
    "cwd":         "/abs/path" | null
  }
]
```

- `cwd: null` means the gateway did not provide a working directory (old
  gateway version, or session opened without one). The panel renders such
  rows but treats them as inert (no click-through).
- The client caches the last successful list and falls back to the cache
  on transport error.

#### `POST /v1/sessions` — ✅ Implemented (no UI consumer yet)

Open a session by name. The gateway creates one if missing and returns
the canonical record.

```jsonc
// Request:
{ "name": "string" }

// Response: CovenSession (same shape as the GET array element)
```

#### `DELETE /v1/sessions/:id` — ✅ Implemented (no UI consumer yet)

Close a session. Idempotent — repeated calls return success even if the
session no longer exists. Cached locally on success: the entry is dropped
from the in-memory list.

### Substrate (workspace context)

Substrate is the editor/workspace slice CastCodes attaches to agent calls
so the agent can reason about the user's current state. It is not a
remote resource: cast_agent **collects** it locally and would **send** it
to the gateway alongside messages — once the wire contract for that
attachment exists (see [API gaps](#api-gaps)).

`Substrate` shape (`crates/cast_agent/src/substrate.rs`):

```jsonc
{
  "active_file":   "/abs/path" | null,
  "open_panes": [
    { "id": "...", "title": "...", "cwd": "/abs/path", "active": true }
  ],
  "shell_cwd":     "/abs/path",
  "git_branch":    "main" | null,
  "recent_errors": [
    { "file": "/abs/path", "line": 12, "severity": "error" | "warning" | "info" | "hint", "message": "..." }
  ],
  "comux_panes": [
    { "id": "...", "cwd": "/abs/path", "title": "...", "active": true }
  ]
}
```

Producer split:

| Field           | Who fills it                                                          | Mechanism                                                       |
|-----------------|-----------------------------------------------------------------------|-----------------------------------------------------------------|
| `active_file`   | Host (`app/src/code/active_file.rs::active_file_changed`)             | `update_host_substrate` patches `active_file`.                  |
| `open_panes`    | Host (`Workspace::publish_open_panes_to_cast_agent`)                  | Three converging triggers: tab lifecycle, active-tab CWD, 10 s. |
| `recent_errors` | Host LSP (`LocalCodeEditorView` + `CastAgentDiagnosticsCollector`)    | Path-scoped replace; 50-entry global cap.                       |
| `shell_cwd`     | cast_agent (`SubstrateCollector::collect`)                            | `std::env::current_dir`.                                        |
| `git_branch`    | cast_agent (`detect_git_branch`)                                      | Walks `.git` upward; no shell-out.                              |
| `comux_panes`   | cast_agent (`ComuxBridge`)                                            | Unix-socket request to `/tmp/comux.sock`; empty when absent.    |

Today substrate does not cross the wire to the gateway. The gateway has
**no** `GET /v1/substrate` or substrate-attached-to-message endpoint.
The next implementation step is to define the wire surface — see
[API gaps](#api-gaps).

### Runs (discrete agent jobs) — ⏳ Proposed, not implemented

A Run models one agent **job**: one user prompt → one streamed response,
with explicit lifecycle, cancellation, status, and (eventually)
artifacts. It is the unit CastCodes needs once the UI grows past "live
stream + history of plain text" — i.e. when the user wants to leave the
panel and come back, see what the agent did, cancel a stalled run, or
recover the artifacts of a completed run after a restart.

This section documents the contract the next implementation agent should
target. **No code exists for any of it yet.** The shapes here are not a
commitment; they are a starting point informed by what the panel and the
existing `MessageChunk` stream already need.

#### Identity

```jsonc
{
  "run_id":          "cast-agent-run-<uuidv7>",
  "conversation_id": "...",         // parent conversation
  "session_id":      "..." | null,  // parent lane, if any
  "status":          "queued" | "running" | "succeeded" | "failed" | "cancelled",
  "created_at":      "RFC3339",
  "completed_at":    "RFC3339" | null
}
```

`run_id` is gateway-assigned. Clients must not invent it.

#### `POST /v1/runs` — start a run

Replaces (or wraps) `POST /v1/messages` once Runs land.

```jsonc
// Request
{
  "conversation_id": "...",          // required; client-supplied for resume, server-assigned for new conversations if omitted
  "session_id":      "..." | null,   // optional pane scoping
  "message":         { /* same body shape as AgentMessage.body */ },
  "substrate":       { /* Substrate snapshot, optional but recommended */ },
  "stream":          true | false    // default true; false returns the full response inline
}

// Response (stream: false)
{ "run_id": "...", "status": "succeeded", "result": { ... }, "artifacts": [ ... ] }

// Response (stream: true)
{ "run_id": "...", "events_url": "/v1/runs/<id>/events" }
```

#### `GET /v1/runs` — list runs

Filterable list. CastCodes needs at least:
`?conversation_id=`, `?session_id=`, `?status=running`,
`?limit=`, `?cursor=` (opaque pagination cursor).

#### `GET /v1/runs/:id` — get run detail

Status snapshot. Returns the full Run record plus a current `progress`
field if the run is still active. Should be cheap (no log replay).

#### `WS /v1/runs/:id/events` — stream events / logs

Same wire shape as `WS /v1/messages/stream` but scoped to a run, so
reconnecting after a panel close re-attaches to the existing run instead
of starting a new one. The server must support **mid-stream replay**:
when a client connects, the server replays buffered events since the run
started so the panel can rebuild state.

```jsonc
// Frames (extends MessageChunk; new variants are additive)
{ "type": "delta",    "run_id": "...", "content": "..." }
{ "type": "tool",     "run_id": "...", "tool": "...", "args": { ... } }      // tool calls
{ "type": "artifact", "run_id": "...", "artifact_id": "...", "kind": "..." } // links to artifact endpoint
{ "type": "log",      "run_id": "...", "level": "info", "message": "..." }
{ "type": "status",   "run_id": "...", "status": "running" | "succeeded" | "failed" | "cancelled" }
{ "type": "done",     "run_id": "..." }
{ "type": "error",    "run_id": "...", "message": "..." }
```

#### `POST /v1/runs/:id/cancel` — cancel a run

Server-side cancel. Idempotent. The gateway acknowledges via
`{ "status": "cancelling" | "cancelled" }`; the events stream emits a
`status: cancelled` frame and a `done` frame before closing.

This is the bit that today's `JoinHandle::abort` in the panel cannot
provide: closing the WS does not stop the gateway from continuing the
work.

#### `GET /v1/runs/:id/artifacts` — list artifacts

```jsonc
[
  {
    "artifact_id":   "...",
    "kind":          "file_diff" | "file_content" | "shell_command" | "...",
    "summary":       "...",
    "content_url":   "/v1/runs/<id>/artifacts/<artifact_id>",
    "created_at":    "RFC3339"
  }
]
```

`GET /v1/runs/:id/artifacts/:artifact_id` returns the body (likely
`application/json` for structured artifacts, `text/plain` or
`application/octet-stream` for raw content).

#### Resume / reopen

A panel that died mid-stream reopens by calling
`WS /v1/runs/:id/events` against the last `run_id` it knew about. If the
run completed in the meantime, the server replays the buffered events
and closes; if it is still running, the client picks up live.

Resuming a **conversation** (continuing a thread) is a different
operation: post a new run with the existing `conversation_id`.

### Conversations — 🟡 Partial / ⏳ Proposed

Today, `conversation_id` is the wire-level threading key on
`AgentMessage` / `AgentResponse` / `MessageChunk`. There is no list/get
endpoint, no history retrieval, and no documented persistence contract.

Once Runs land, the minimum useful conversation surface is:

- `GET /v1/conversations?session_id=&limit=&cursor=` — list of
  conversations the user can resume.
- `GET /v1/conversations/:id` — metadata (id, last_active, run count,
  parent session) + most recent N runs.

Conversation deletion / archival is intentionally out of scope until the
panel has a "history" view that needs it.

## Error and degraded-mode behaviour

| Symptom                                  | What the client does                                                            | What the UI shows                          |
|------------------------------------------|---------------------------------------------------------------------------------|--------------------------------------------|
| `GET /health` non-2xx or timeout         | `is_available()` flips to `false`; retried on 30 s loop.                        | Amber gateway pill; sessions list hidden.  |
| `GET /v1/sessions` transport error       | Returns the in-memory cache; logs at `warn`.                                    | Stale list rendered as-is; no flicker.     |
| `POST /v1/messages` 4xx/5xx              | `Err` propagates; no retry.                                                     | (no UI consumer today)                     |
| `WS /v1/messages/stream` connect failure | `stream_messages` returns `Err`.                                                | Stream section shows the error in-band.    |
| `WS /v1/messages/stream` mid-stream drop | The stream yields a final `Err`; the read loop ends.                            | Live section freezes on last received delta; user must re-send. |
| Comux socket absent / unresponsive       | `list_panes` returns `[]`; logs at `debug`.                                     | `comux_panes` is empty; everything else still works. |
| Runtime fails to boot                    | `cast_agent::global()` returns `None`; all sync helpers return defaults.        | Treated as offline; pill stays amber.      |

The runtime never panics on a gateway failure. **Degraded mode** = the
panel and substrate publishers keep working, the gateway pill is amber,
streamed messages stay in local history but cannot reach the gateway.

## Implemented vs proposed (at a glance)

| Surface                            | Status      | Notes                                                                                  |
|------------------------------------|-------------|----------------------------------------------------------------------------------------|
| `GET /health`                      | ✅          |                                                                                        |
| `POST /v1/messages`                | ✅          | No UI consumer; tests + headless callers only.                                         |
| `WS /v1/messages/stream`           | ✅          | Live in the agent panel; client-side abort only.                                       |
| `GET /v1/sessions`                 | ✅          | 60 s background refresh + sync snapshot for UI.                                        |
| `POST /v1/sessions`                | ✅          | Wired in `GatewayClient`, no UI consumer yet.                                          |
| `DELETE /v1/sessions/:id`          | ✅          | Wired in `GatewayClient`, no UI consumer yet.                                          |
| Substrate collection (client-side) | ✅          | `active_file`, `open_panes`, `recent_errors`, `shell_cwd`, `git_branch`, `comux_panes`.|
| Substrate → gateway wire contract  | ⏳          | No endpoint, no attached-to-message envelope.                                          |
| `POST /v1/runs`                    | ⏳          | See [Runs](#runs-discrete-agent-jobs---proposed-not-implemented).                      |
| `GET /v1/runs` / `GET /v1/runs/:id`| ⏳          |                                                                                        |
| `WS /v1/runs/:id/events`           | ⏳          |                                                                                        |
| `POST /v1/runs/:id/cancel`         | ⏳          | Today: client-side `JoinHandle::abort` only.                                           |
| `GET /v1/runs/:id/artifacts`       | ⏳          |                                                                                        |
| `GET /v1/conversations`            | ⏳          |                                                                                        |
| `GET /v1/conversations/:id`        | ⏳          |                                                                                        |

## API gaps

The brief asks four direct questions; answering them in order:

**1. What APIs are missing from the docs today?**

- A wire contract for **Substrate** crossing the gateway boundary.
  Substrate is collected and merged into a single struct in cast_agent,
  but neither `POST /v1/messages` nor `WS /v1/messages/stream` documents
  how to attach it. The likely shape is a top-level `substrate` field
  next to `body` on the request envelope, but neither side implements it.
- A wire contract for **Runs** (start / list / get / events / cancel /
  artifacts). Discussed above as proposed; nothing is implemented.
- A wire contract for **Conversations** (list / get / history).
- **Server-side cancellation** for in-flight work. Closing the WebSocket
  is not cancellation — the gateway can keep running the prompt.
- A **structured error envelope** for HTTP responses. Today errors come
  through as bare 4xx/5xx with no body shape, so the client cannot
  distinguish (e.g.) "rate limited" from "auth expired" without parsing
  free-text.
- **Pagination** on `GET /v1/sessions` (and on the proposed list
  endpoints). Trivially missing today because the list is small, but
  the contract should declare its shape (likely opaque `cursor` +
  `limit`) before clients grow to depend on the unpaginated form.

**2. Are tasks missing because they are not implemented yet, because
CastCodes calls them by another name, or because docs have not caught
up?**

Not implemented yet, and intentionally **not called "tasks"**. The
existing `crates/ai`-side `LifecycleEventType` (`Started`, `Idle`,
`InProgress`, `Succeeded`, `Failed`, `Cancelled`, …) is the
`warp_multi_agent_api` wire enum the upstream agent backend uses; the
specs use `task.AgentConversationID` and `oz run list` at the Oz
orchestration layer. Both are sibling concerns, not the cast_agent's
in-panel job lifecycle. Cast_agent currently has no equivalent
abstraction at all — only `conversation_id` threaded through messages,
and the WebSocket lifetime that wraps a single turn. The right move is
to introduce **Runs** (see above) rather than overload "task".

**3. What is the minimum API set needed for a good CastCodes agent UX?**

For the panel as it exists today plus the next two obvious UX upgrades
(resume after restart, cancel in-flight work), the minimum is:

1. ✅ `GET /health`
2. ✅ `WS /v1/messages/stream`
3. ✅ `GET /v1/sessions`
4. ⏳ A way to attach `Substrate` to outgoing messages (wire envelope
    on the existing message endpoints; no new endpoint required).
5. ⏳ `POST /v1/runs/:id/cancel` (or equivalent), so cancel is real.
6. ⏳ `WS /v1/runs/:id/events` with mid-stream replay, so a panel
    close-and-reopen reattaches instead of losing context.

Items 4–6 are the smallest credible "Runs" rollout: substrate on the
wire + cancel + reattach. Everything else (artifacts, lists,
conversation history) can defer.

**4. What should be deferred?**

- `GET /v1/conversations` and `GET /v1/conversations/:id` — only needed
  once the panel grows a history view.
- `GET /v1/runs/:id/artifacts` and artifact retrieval — only needed
  once the agent produces non-text output the panel can render.
- `POST /v1/sessions` and `DELETE /v1/sessions/:id` UI consumers — the
  client code exists; deferring means leaving them callable from tests
  and headless tools but not surfacing them in the panel.
- A structured HTTP error envelope — nice-to-have; today's bare 4xx/5xx
  is workable while the gateway is single-tenant.
- Pagination — defer until any list endpoint produces > ~50 entries.

## Comux bridge

Cast Agent looks for the Comux daemon at:

1. `$COMUX_SOCKET` (env override).
2. `/tmp/comux.sock` (default).

Request wire format (newline-delimited JSON):

```json
{"type":"list_panes"}
```

Response:

```json
{"panes":[{"id":"...","cwd":"...","title":"...","active":true}]}
```

If the socket is absent or the request fails, `list_panes()` returns an empty
`Vec` and logs at debug level. Comux is treated as optional context, never
a hard dependency.

## Open follow-ups

The brief asks for several integration steps that are deferred so they can
be done in a follow-up PR without partially-wiring the host crate:

1. **`crates/ai` feature-flag wiring.** Adding the
   `cast-agent` / `warp-agent` Cargo features requires unwinding the
   currently unconditional `warpui`, `warp_core`, `warp_terminal`,
   `warp_graphql`, `warp_multi_agent_api`, and `warp_util` dependencies in
   `crates/ai/Cargo.toml` and adding `#[cfg(feature = "...")]` at each
   construction site. Several of those crates are also used outside agent
   paths, so the gating has to be done call-by-call rather than wholesale.
   See [Feature-gating audit](#feature-gating-audit-warp-agent-vs-cast-agent)
   for the per-dep verdict and roadmap.

2. **TUI rebranding.** Replacing "Warp Agent" / "Warp AI" / "Warp Drive"
   strings and the agent panel header with Cast Agent branding and a live
   gateway status pill is straightforward, but currently lives across
   several `app/src/ai_assistant/` and `crates/ai/` view modules and is
   safest done as a separate pass alongside the integration above.

3. **Runs / Substrate wire surface.** The proposed Runs and on-the-wire
   Substrate contracts in [API contract](#api-contract) are the next
   integration step once panel parity is closer. See [API gaps](#api-gaps)
   for the minimum subset.

Session click-through (✅) and streaming responses (✅) — originally listed
here — landed and have moved up to the [Status](#status) section.

## Feature-gating audit (`warp-agent` vs `cast-agent`)

An earlier scope note proposed gating the upstream `warp_*` dependencies
in `crates/ai` behind a `warp-agent` Cargo feature so `cast-agent`-only
builds are leaner. After auditing every `warp_*` import in
`crates/ai/src/` against today's code
(`rg -l 'warp_' crates/ai/src/`), the picture is more constrained than
that proposal implied: most of the listed crates provide **shared infrastructure**
that `crates/ai` uses for non-agent purposes (telemetry, codebase
indexing, UI entity primitives, paths). Only one crate is plausibly
gateable today.

### Per-dep verdict

| Dep | Files / refs | Used for | Verdict |
|-----|--------------|----------|---------|
| `warp_core`            | 9 / 27 | Telemetry, `features::FeatureFlag`, `channel::ChannelState`, `command::ExitCode`, `paths::secure_state_dir`, `sync_queue`, `ui::Icon`, `safe_anyhow!`/`safe_warn!` macros. Used by `telemetry.rs`, `aws_credentials.rs`, the entire `index/full_source_code_embedding/` codebase-indexing tree, and the agent action paths. | **Shared infra — cannot gate.** Required for `crates/ai` to compile regardless of agent backend. |
| `warp_util`            | 6 / 19 | `StandardizedPath` (workspace-wide path normalization), used by codebase indexing tests + production paths. | **Shared infra — cannot gate.** Non-agent codebase indexing depends on it. |
| `warpui`               | 8 / 12 | `Entity`, `ModelContext`, `SingletonEntity`, `ModelHandle`, `App`, `AppContext`, `r#async::Timer`, `platform::OperatingSystem`. The GPUI-style entity system. Every model/view in `crates/ai` participates. | **Shared infra — cannot gate.** UI framework foundation; gating it removes `crates/ai` itself. |
| `warpui_extras`        | 1 / 1  | `secure_storage::AppContextExt` for `api_keys.rs`. | **Shared infra — cannot gate.** Used by non-agent api-key storage. |
| `warp_terminal`        | 5 / 6  | `shell::ShellLaunchData` (used by `paths.rs`, non-agent), `model::BlockId`, `model::escape_sequences` (used by agent action paths). | **Mixed.** Non-agent `paths.rs` use blocks wholesale gating. Could narrow the gate to the agent action paths only, but the dep stays unconditional. |
| `warp_graphql`         | 2 / 25 | `EmbeddingConfig`, `RepoMetadata`, `FragmentLocationInput` for codebase indexing GraphQL queries. **Not used by agent paths.** | **Shared infra — cannot gate.** Codebase-indexing wire types, not agent protocol. |
| `warp_multi_agent_api` | 10 / 19 | Agent protocol wire types: `LifecycleEventType`, `FileContent`, `AnyFileContent`, `SkillReference`, `message::tool_call::*` mode types, `apply_file_diffs_result::*`. Used in `agent/`, `skills/`, `api_keys.rs`, `aws_credentials.rs`. Currently re-exported as public API of `crates/ai` (`pub use warp_multi_agent_api::LifecycleEventType;` in `agent/action/mod.rs`). | **Gateable in principle — but blocked by the public-API leak.** See the roadmap below. |

### Roadmap to a real `warp-agent` gate

The brief assumed each warp_* dep is "an agent client construction site"
that can be wrapped with a `#[cfg(...)]` block. The actual shape of the
codebase is different: the only meaningful gating opportunity is
`warp_multi_agent_api`, and even that needs preparation before the
`#[cfg(...)]` can land safely.

**Phase A (original framing) — stop leaking wire types through
`crates/ai`'s public API.** Premise: define `crates/ai`-owned types
for anything that re-exports `warp_multi_agent_api::*`, with `From`
conversions kept inside the agent subtree. No behaviour change, no
feature flags.

**Phase B — `cast_agent`-native parallel types.** Introduce equivalent
types inside `cast_agent` (or a new `agent_wire_types` crate that both
backends depend on). Define `From<cast_agent::Foo> for
ai::Foo` and the reverse where needed. Keep the warp-agent
side gated behind `#[cfg(feature = "warp-agent")]`.

**Phase C — actually gate the dep.** Once Phase A and B land, the
`warp_multi_agent_api` import in `crates/ai/src/agent/` lives only
inside `#[cfg(feature = "warp-agent")]` blocks. Cargo can then
optional-ify the dep:

```toml
[dependencies]
warp_multi_agent_api = { workspace = true, optional = true }

[features]
warp-agent = ["dep:warp_multi_agent_api"]
```

`cast-agent`-only builds will then skip the protobuf compilation and
the dep entirely.

### Phase A revision (post-PR #52 finding)

PR #52 landed the `LifecycleEventType` internalization successfully:
small i32 enum, single re-export site, clean mirror with
bidirectional `From`s + `TryFrom<i32>`. Attempting the same pattern
for `FileContent` / `AnyFileContent` / `SkillReference` revealed that
the **wire-type leak is not concentrated in `crates/ai`'s public
re-exports** — it is spread across the host crate:

- 50+ `app/src/` files import `warp_multi_agent_api` **directly** (not
  via `crates/ai`). They use wire types for conversation persistence,
  streaming/orchestration events, code-review comment payloads,
  agent-mode integration tests, terminal/blocklist controllers, and
  more.
- `app/src/ai/agent/api/` (`convert_to.rs` + `convert_from.rs` +
  `convert_conversation.rs` + `impl.rs` and their test siblings —
  7163 LOC total) is a **symmetric wire-protocol translation layer**.
  Its entire purpose is to bridge ai's runtime types to/from
  `warp_multi_agent_api` types; per-type internalization in
  `crates/ai` doesn't reduce its dep on the wire crate at all.
- Persistence (`app/src/persistence/agent.rs`,
  `app/src/ai/agent/conversation_yaml.rs`) stores wire-typed conversation
  structures. Renaming `crates/ai::FileContent` to a mirror without
  also rewriting on-disk schema would silently desync persisted
  conversations.

**Implication:** internalizing wire types one at a time inside
`crates/ai` makes `crates/ai`'s public API marginally cleaner but
doesn't move the gating needle. The dep `warp_multi_agent_api` would
remain unconditional in `app/src/` until every file in the list above
is also migrated. **The original Phase A is a small grooming step,
not the gate prerequisite.**

### Revised Phase A — pick one of three strategies

**Strategy 1 — Wholesale module gating.** Treat
`app/src/ai/agent/api/` (the 7163 LOC conversion layer) plus the
~40 `app/src/ai/*` consumer files as a single unit. Gate the entire
subtree behind `#[cfg(feature = "warp-agent")]`. `cast-agent`-only
builds drop the whole agent-protocol translation layer. The
`cast-agent`-native path goes through `cast_agent::gateway` (the
streaming consumer already wired in PR #42) instead of the
`warp_multi_agent_api` conversion path. **One large PR**; high
review burden but unambiguous result. Requires the cast_agent panel
path to be feature-complete enough to be the *only* path on the
`cast-agent`-only build — currently it is log-only / streaming-only,
so this strategy waits on full panel parity.

**Strategy 2 — Accept the wire types as ai's protocol surface.**
Stop trying to hide `warp_multi_agent_api` behind ai-owned mirrors;
declare it the canonical agent-protocol crate. Re-export it
explicitly from `crates/ai` so `app/src/` consumers depend on
`crates/ai::wire::*` rather than `warp_multi_agent_api::*` directly.
A `warp-agent`-vs-`cast-agent` build still requires `cast_agent` to
implement compatible message shapes (which it largely does — see
`crates/cast_agent/src/gateway.rs`'s `MessageChunk` / `AgentMessage`),
but the host crate no longer pretends to own protocol-shaped types
it doesn't own. **One medium PR**; smallest behavioural change.

**Strategy 3 — Defer Phase A indefinitely; gate at runtime.** Skip
compile-time gating entirely. The agent backend is already selected
at runtime (`is_available()` on the cast_agent global, plus the
existing `warp_multi_agent_api`-backed path). Continue shipping both
backends in every build until cast_agent achieves panel parity, then
remove the warp-agent path entirely in a single delete-PR. **Zero
PRs needed now**; the warp-agent path remains the fallback.

### Recommendation

Strategy 3 for now (defer Phase A) — the runtime gate is already
serving the user-visible purpose ("show the gateway pill, route
panel input via cast_agent when available"), and the compile-time
gate's *only* benefit is build leanness, which is a second-order
concern while cast_agent's panel surface is still maturing
(streaming consumer landed in #42; gating, history, and persistence
landed in #46–#50; full message-send round-trip is still TBD).

Strategy 1 becomes the right move once the cast_agent panel can
service every request that the warp-agent path currently services —
including conversation persistence, orchestration events, code-review
threads, and agent-mode integration tests. That's a feature-parity
gate, not a refactoring gate.

Strategy 2 is a fallback if the "lean cast-agent-only build" goal turns
out to be load-bearing before parity lands.

### What this section is not

The per-dep verdict table above remains ground truth — six of seven
`warp_*` crates are non-agent shared infrastructure and will never be
gateable in `crates/ai`. This roadmap revision only concerns
`warp_multi_agent_api`, the one dep where gating is theoretically
possible. Future agents picking up Phase A should choose a strategy
explicitly before opening PRs; the original "internalize types one
at a time" framing produces zero gating benefit on its own.
