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
  (`OPENCOVEN_SUCCESS`/`OPENCOVEN_WARNING`).
- ⏳ Session list / click-through, streaming responses, per-call
  `#[cfg(feature = "warp-agent")]` gating — see "Open follow-ups" below.

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
