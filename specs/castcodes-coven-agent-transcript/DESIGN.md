# One Coven Agent Surface — Phase 1 Design

- **Date:** 2026-07-17
- **Status:** Design approved; ready for implementation planning
- **Scope:** Phase 1 of a phased roadmap (Phases 2–3 sketched for context only)

## Problem

CastCodes has **two fully-built, parallel AI-conversation surfaces**:

| | Agent panel (`app/src/ai_assistant/`) | Chat panel (`app/src/cli_chat/`) |
|---|---|---|
| Backend | Coven daemon session — `coven-code` harness via `~/.coven/coven.sock` | AI CLI (claude/codex/gemini/opencode) running in a terminal pane, observed via OSC-777 events |
| Rendering | Plain-text stream (`MessageChunk::Delta`) | Rich: message bubbles, tool-call cards, permission cards, info bars |
| Persistence | `~/.coven/stream-history.json` | sqlite (`ChatStore`) |
| Composer | Editor → daemon `POST /api/v1/sessions` | rich-input → terminal PTY stdin |
| Entry point | agent-panel toggle | `Cmd/Ctrl+Shift+H`, runtime `FeatureFlag::CastCodesChatPanel` |
| Extras | — | conversation list, model picker |

Two surfaces that both show "an AI conversation" with different backends, rendering, and persistence is incoherent for a product whose story is "Coven Code is the native agent." The concrete, user-visible gap: **coven-code renders as plain text** while the chat panel already has the rich rendering coven-code should have.

## Roadmap (context — only Phase 1 is specified here)

1. **Phase 1 — Shared transcript model (this doc).** Extract `cli_chat`'s render model + rich views into a backend-agnostic `agent_transcript` module behind a `TranscriptSource` seam, and add a Coven-daemon adapter so **coven-code renders richly** in the existing agent panel. Backend-agnostic; no panel merge.
2. **Phase 2 — Unified panel, entry point & persistence.** One surface: one keybinding, one conversation list spanning daemon sessions + CLI sessions, one composer routing to the right backend, one sqlite store (migrate `stream-history.json`). Deprecate the redundant panel.
3. **Phase 3 — Backend consolidation (deferred decision).** Whether to route claude/codex through daemon lanes and retire the OSC-777 terminal-CLI path — decided *after* Phases 1–2 show how daemon-lane parity lands.

The OSC-777 retirement question is **explicitly deferred**. Phase 1 is designed to hold regardless of that outcome: it unifies only the presentation layer, over whichever backends exist.

## Phase 1 goal

`coven-code` sessions render with the same rich UI the chat panel already has — tool-call cards, permission cards, assistant/user bubbles, stop markers — by sharing one entry model and one set of views between the two backends. The change is **strictly additive**: if structured events are unavailable or malformed, coven-code still renders plain text exactly as today.

## Non-goals (Phase 1)

- No panel merge, single entry point, or unified conversation list → **Phase 2**.
- No persistence unification — coven-code keeps `~/.coven/stream-history.json` for now → **Phase 2**.
- No routing claude/codex through the daemon → **Phase 3**.
- Not building stream-json *emission* inside the Coven daemon if it is absent — that is an upstream Coven concern; the spike (below) surfaces it as a dependency/risk.

## Architecture

`ChatEntry` is the neutral contract. Both backends produce it; one set of views renders it.

```
Coven path (NEW):
  coven-code session (daemon) ──stream-json──▶ cast_agent CovenAgentEvent stream
     ──▶ daemon_event_to_entry ──▶ ChatEntry ──▶ agent_transcript views  (ai_assistant panel)

CLI path (UNCHANGED behavior):
  CLI in terminal ──OSC-777──▶ CLIAgentEvent ──▶ ChatEntry::from_event ──▶ agent_transcript views  (cli_chat panel)
```

### Components

**1. `app/src/agent_transcript/` — new neutral module (extracted from `cli_chat`)**
- `ChatEntry` + `ChatEntryKind` (`UserPrompt`, `AssistantResponse`, `ToolCall`, `PermissionRequest`, `Info`, `Stop`, `PermissionReplied`, `Raw`) — moved from `cli_chat/entry.rs`.
- Rich views moved from `cli_chat/view/`: `message_bubble`, `tool_call_card`, `permission_card`, `info_bar`, and the `transcript` renderer.
- A `TranscriptSource` trait: yields `ChatEntry`s + a coarse status for a conversation. No backend, persistence, or panel knowledge.
- Depends on nothing in `cli_chat` or `cast_agent` (leaf module), so both consumers can depend on it without cycles.

**2. `app/src/cli_chat/` — refactored, behavior-preserving**
- Keeps `ChatModel`, sqlite `store`, `conversation_list`, `composer`, `model_picker`, `empty_state`, `error_banner`.
- Now imports `ChatEntry` + views from `agent_transcript`; `ChatEntry::from_event(CLIAgentEvent)` becomes the CLI `TranscriptSource` impl.
- Its existing tests must pass unchanged (proof the extraction preserves behavior).

**3. `crates/cast_agent` — extended with a structured event stream**
- New `CovenAgentEvent` enum: `AssistantDelta { text }`, `ToolCall { name, args }`, `ToolResult { name, output }`, `PermissionRequest { summary }`, `Done`, `Error { message }`.
- Sourced from the coven-code session's **stream-json** (`system.init / user / assistant / tool_result / result`), exposed as a `Stream<CovenAgentEvent>` alongside the existing plain `MessageChunk` path (which remains the fallback).

**4. `app/src/ai_assistant/panel.rs` — rewired rendering**
- New `daemon_event_to_entry(CovenAgentEvent) -> Option<ChatEntry>` adapter. It lives on the **backend side** (`ai_assistant` or `cast_agent`), never in `agent_transcript` — the neutral module must not know about `CovenAgentEvent`, exactly as it must not know about `CLIAgentEvent`. Both adapters (`from_event`, `daemon_event_to_entry`) depend on `agent_transcript`, not the reverse.
- The plain-text "COVEN STREAM" block is replaced by an `agent_transcript` transcript view fed by the mapped entries. The offline notice and Phase-1 gating (`CovenDispatch`, `COVEN_CODE_OFFLINE_NOTICE`) are unchanged.

**5. Structured-event source — ✅ RESOLVED by spike (see `SPIKE-daemon-structured-events.md`)**
The daemon supports **`launchMode: "stream"`**: it runs the harness with its adapter `stream_args` and forwards the harness's JSONL (`system.init / user / assistant / tool_result / result`). `coven-code` declares `"stream": true` + `stream_args` (`--print --input-format stream-json --output-format stream-json`), as do `codex`/`claude`/`copilot`. **No upstream Coven change is needed** — `cast_agent` changes the launch from `nonInteractive` to `stream` and parses the JSONL `output` events into `CovenAgentEvent`s. One wire detail to confirm early in the plan: how the prompt is delivered in stream mode (positional prompt is ignored; the harness expects a stream-json user frame on stdin — long-lived, one turn per frame).

## Data flow details

- `cast_agent` opens the coven-code session (existing launch path) and, per the spike outcome, reads structured events, decoding each JSONL line into a `CovenAgentEvent`.
- The panel's stream consumer (already present from Phase 2 of the streaming work) maps each `CovenAgentEvent` to a `ChatEntry` via `daemon_event_to_entry`, appends it to an in-memory transcript, and the `agent_transcript` view renders it.
- Ordering + partial assistant deltas: consecutive `AssistantDelta`s coalesce into one growing `AssistantResponse` bubble (same live-append behavior as today's stream), while `ToolCall`/`ToolResult`/`PermissionRequest` produce discrete cards.

## Error handling (graceful degradation)

- **Unmappable/unknown structured event** → skip and increment a counter; reuse `cli_chat`'s existing skipped-event → error-banner mechanism (moved to `agent_transcript` or shared).
- **stream-json parse failure** (malformed/partial line, or structured events absent) → fall back to rendering the raw chunk text as an `AssistantResponse` bubble. Coven-code is then **no worse than today's plain text** — the feature is purely additive.
- **Daemon offline** → unchanged Phase-1 in-band offline notice.

## Testing

- **Unit — `daemon_event_to_entry`:** stream-json fixtures (`assistant`, `tool_result`, permission) → expected `ChatEntry` kinds; mirrors `cli_chat`'s existing `from_event` unit tests.
- **Unit — fallback:** malformed/absent structured events → `AssistantResponse` fallback (never panics, never empties).
- **Layout:** each moved `agent_transcript` view gets a "renders without panic" test (the repo's `View` layout-safety convention).
- **Integration:** extend the in-process Unix-daemon stub (`crates/cast_agent/tests/daemon_streaming.rs`) to emit stream-json → assert rich `CovenAgentEvent`s / entries.
- **Regression:** `cli_chat`'s full existing test suite passes unchanged after the extraction.

## Risks & open questions

1. **~~(Top risk) Structured events from the daemon~~ — ✅ RESOLVED** (`SPIKE-daemon-structured-events.md`). The daemon's `launchMode: "stream"` forwards harness stream-json; `coven-code` supports it natively; no upstream change needed. Residual: confirm the stream-mode prompt-delivery wire detail early in the plan.
2. **`agent_transcript` extraction churn** — moving types/views out of `cli_chat` touches many imports. Mitigation: pure move + re-export shim initially; behavior-preserving; gated by `cli_chat`'s unchanged tests.
3. **Fork-local boundary** — `agent_transcript` must not pull Warp-owned infra (`cli_chat` has `script/check_cli_chat_boundary`). Mitigation: keep `agent_transcript` a leaf render module; extend the boundary guard to cover it.
4. **Feature gating** — coven-code rich rendering should ride the `cast-agent` build feature (already used by the panel); optionally a runtime flag for staged rollout.

## Rollout

- Land behind the existing `cast-agent` feature; the plain-text fallback means no regression if structured events are unavailable.
- Verify with `./script/check_ai_attribution`, `./script/check_rebrand`, `./script/check_cli_chat_boundary` (extended), and the test matrix above.
- Phase 2 (unified panel/persistence) is a separate spec that builds on the `agent_transcript` seam this phase creates.
