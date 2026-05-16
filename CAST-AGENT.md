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
  below. Verdict: of the seven warp_* deps the brief listed, only
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
- ⏳ Per-call `#[cfg(feature = "warp-agent")]` gating implementation,
  agent panel switch to `stream_messages` for actual chat — see
  "Open follow-ups" below.

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

## Endpoints used

| Method | Path                     | Purpose                          |
|--------|--------------------------|----------------------------------|
| GET    | `/health`                | Startup probe → `is_available()` |
| POST   | `/v1/messages`           | Send a chat message              |
| GET    | `/v1/sessions`           | List active Coven sessions       |
| POST   | `/v1/sessions`           | Open a session by name           |
| DELETE | `/v1/sessions/:id`       | Close a session (idempotent)     |

Auth header: `Authorization: Bearer <token>` when configured.

## Degradation

- If `/health` returns non-200 or times out, the agent stays usable but
  `is_available()` returns `false`. The UI should render an amber pill.
- `list_sessions()` falls back to its in-memory cache on transport error.
- `get_substrate()` returns the local CWD + git branch even with no gateway,
  and Comux pane data is folded in only when the daemon is reachable.

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

2. **TUI rebranding.** Replacing "Warp Agent" / "Warp AI" / "Warp Drive"
   strings and the agent panel header with Cast Agent branding and a live
   gateway status pill is straightforward, but currently lives across
   several `app/src/ai_assistant/` and `crates/ai/` view modules and is
   safest done as a separate pass alongside the integration above.

3. **Session click-through.** Clicking a Coven session in the agent panel
   should open a new terminal pane with the right CWD — this needs the
   workspace pane API (`app/src/workspace`) and is part of the TUI work.

4. **Streaming responses.** `GatewayClient::send_message` currently does a
   single round-trip. A `stream_messages` method using `tokio-tungstenite`
   against `/v1/messages/stream` should be added when the host wires its
   streaming UI through.

## Feature-gating audit (`warp-agent` vs `cast-agent`)

[`CODY-BRIEF.md`](CODY-BRIEF.md) §2.5 calls for gating the upstream
`warp_*` dependencies in `crates/ai` behind a `warp-agent` Cargo feature
so `cast-agent`-only builds are leaner. After auditing every `warp_*`
import in `crates/ai/src/` against today's code (`grep -rln warp_*
crates/ai/src/`), the picture is more constrained than the brief
implied: most of the listed crates provide **shared infrastructure**
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

**Phase A — stop leaking wire types through `crates/ai`'s public API.**
Today `crates/ai/src/agent/action/mod.rs` does
`pub use warp_multi_agent_api::LifecycleEventType;`, and several
`From<warp_multi_agent_api::FileContent> for FileContext` impls live in
`agent/action/convert.rs`. Downstream `app/src/` consumers depend on
these. Step one is to keep all `warp_multi_agent_api` references
*internal* to `crates/ai`: define `crates/ai`-owned types for anything
that's re-exported, with `From` conversions kept inside the agent
subtree. No behaviour change, no feature flags yet.

**Phase B — `cast_agent`-native parallel types.** Introduce equivalent
types inside `cast_agent` (or a new `agent_wire_types` crate that both
backends depend on). Define `From<cast_agent::Foo> for
crates::ai::Foo` and the reverse where needed. Keep the warp-agent
side gated behind `#[cfg(feature = "warp-agent")]`.

**Phase C — actually gate the dep.** Once Phase A and B land, the
`warp_multi_agent_api` import in `crates/ai/src/agent/` lives only
inside `#[cfg(feature = "warp-agent")]` blocks. Cargo can then
optional-ify the dep:

```toml
warp_multi_agent_api = { workspace = true, optional = true }
warp-agent = ["dep:warp_multi_agent_api", ...]
```

`cast-agent`-only builds will then skip the protobuf compilation and
the dep entirely.

### What this PR is not

This PR ships **no code changes** beyond the documentation update —
explicitly per scope choice. The work above is multi-PR; landing Phase
A alone would touch every `agent/` and `skills/` consumer. Future
agents should treat the per-dep verdict table as ground truth before
attempting `#[cfg(feature = "warp-agent")]` blocks. Most warp_* deps
will never be gateable in `crates/ai` because their use is not
agent-related — and that's a real architectural answer, not a TODO.
