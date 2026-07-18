# Unified Agent Panel — Phase 2 Design

- **Date:** 2026-07-18
- **Status:** Proposed — under review
- **Roadmap:** Phase 2 of the Coven agent-surface reconciliation. Phase 1 (rich rendering via shared `agent_transcript`) merged in #201; Phase 3 (backend consolidation / OSC-777 retirement) stays deferred.

## Problem

After Phase 1, CastCodes still has **two AI-conversation panels**:

| | `ai_assistant` (Familiar) panel | `cli_chat` panel |
|---|---|---|
| Status | Shipped default (`cast-agent` feature) | Feature-flagged (`CastCodesChatPanel`), `Cmd+Shift+H` |
| Backend | Coven daemon (`coven-code`), headless, streamed in-panel | OSC-777 CLI agents (claude/codex/gemini/opencode) running in a terminal pane |
| List | Read-only "Coven Sessions" (`render_sessions_section`) | Full conversation list |
| Persistence | `~/.coven/stream-history.json` (plain text) | sqlite (`chat_conversation` / `chat_entry`, versioned) |
| Composer | daemon API | terminal PTY stdin |
| Rendering | `agent_transcript` (Phase 1) | `agent_transcript` (Phase 1) |
| Extras | gateway status pill, Familiar picker | model picker |

Rendering is already unified. Two separate surfaces — different lists, persistence, composers, and entry points — is still incoherent for "one native agent."

## Goal

**One unified agent panel**: one entry point, one conversation list spanning **both** daemon sessions and CLI sessions, one composer that routes to the correct backend, and one sqlite store (migrating the daemon panel's `stream-history.json`). Both existing panels are deprecated behind it.

## Decisions (settled during design)

- **A new unified panel** (`app/src/agent_panel/`), not a takeover of either existing panel — cleanest boundaries; both old panels fold into it.
- **Backend picker + native mechanics.** The panel unifies the list / transcript / history / persistence; each backend keeps its native launch + interaction — the daemon backend stays **headless, streamed in-panel**; the terminal CLIs keep running in a **terminal pane** observed via OSC-777. This is the smallest change consistent with deferring backend consolidation to Phase 3.

## Non-goals (Phase 2)

- No routing claude/codex/etc. through the daemon, and no retiring the OSC-777 terminal-CLI path → **Phase 3**.
- No making the CLIs headless/panel-managed — they keep their terminal pane.
- The old panels' code is not deleted in the same PR that lands the unified panel; removal is a staged follow-up once parity is confirmed behind the flag.

## Architecture

The load-bearing idea: a shared conversation core generalized over a backend enum, with one model / list / store / composer around the already-shared `agent_transcript` rendering.

```rust
enum ConversationBackend {
    Cli(AgentKind),      // claude/codex/gemini/opencode — OSC-777, terminal pane, composer -> PTY stdin
    Daemon { harness: String }, // coven-code (+ future) — headless daemon lane, composer -> cast_agent
}
```

### Components

1. **`app/src/agent_panel/` (new)** — the unified panel view: a two-column **conversation list + `agent_transcript` transcript + composer**, behind one feature flag and one keybinding. Assembled from `cli_chat`'s existing `conversation_list`, `model_picker`, and `composer` views, generalized for both backends.

2. **Unified `ConversationModel`** — `cli_chat`'s `ChatModel` generalized to carry `ConversationBackend` on each `ChatConversation`, fed by **two sources**:
   - **CLI source** (existing): subscribes to `CLIAgentSessionsModel`; `CLIAgentEvent → ChatEntry::from_event`.
   - **Coven-daemon source** (new): populates `Daemon` conversations from the `cast_agent` session list; for the *active* daemon conversation, drives `stream_agent_events → daemon_event_to_entry → ChatEntry` (Phase 1 pieces).

3. **Persistence** — the existing sqlite `ChatStore`, extended:
   - Schema migration (`chat_schema_version` bump): add a `backend` column to `chat_conversation` (`'cli'` for existing rows, `'daemon'` for coven-code). `agent` continues to carry the harness/kind name.
   - One-time, **non-destructive, idempotent** data migration: read `~/.coven/stream-history.json` (`{conversation_id, text}` records), insert each as a `coven-code` `Daemon` conversation with a single `AssistantResponse` entry, then rename the file `.migrated` (never delete user data). Guarded by a migration marker so it runs once.
   - Daemon conversations persist as structured `ChatEntry`s from then on, like CLI ones.

4. **Composer routing** — one composer; on submit it looks up the active conversation's `ConversationBackend` and routes:
   - `Cli(kind)` → `submit_text_to_cli_agent_pty` (today's terminal path).
   - `Daemon` → the `cast_agent` send path (this is where the Phase-1-flagged stream-mode prompt-delivery detail — positional prompt vs. stdin user frame — is resolved and implemented).
   The composer is enabled only when the active backend can accept input (e.g. a CLI needs a live terminal session).

5. **Deprecation & rollout (staged)** — the standalone `cli_chat::ChatPanelView` and the `ai_assistant` coven-stream panel section both retire in favor of the unified panel. Land the unified panel behind a feature flag with one keybinding; keep the old panels' code until parity is confirmed behind the flag (dogfood), then delete them in a follow-up cleanup PR (mirrors the repo's "keep the fallback until parity" pattern used for the inherited Warp agent).

## Data flow

```
CLI conversation (unchanged mechanics):
  CLI in terminal ──OSC-777──▶ CLIAgentSessionsModel ──▶ ChatEntry::from_event
     ──▶ ConversationModel { backend: Cli(kind) } ──▶ agent_transcript::render_entries

Daemon conversation (new):
  cast_agent sessions ──▶ ConversationModel Daemon conversations (list)
  active daemon convo: composer ──▶ cast_agent send ──▶ stream_agent_events
     ──▶ daemon_event_to_entry ──▶ ChatEntry ──▶ same render path
```

The list merges both conversation sets by recency, each row carrying a backend badge. Selecting a conversation binds the transcript and points the composer at that backend.

## Error handling

- **Daemon offline** → daemon conversations render their persisted history read-only; new daemon sends surface the existing in-band offline notice; CLI conversations are unaffected.
- **Migration** → non-destructive + idempotent; a parse failure on `stream-history.json` logs and skips (never blocks the panel or loses the sqlite store).
- **Backend can't accept input** (no live terminal for a CLI conversation) → composer disabled with an explanatory placeholder, mirroring `cli_chat` today.
- **Fork-local boundary** → the promoted core stays free of Warp-owned infra; extend `check_cli_chat_boundary` to cover `agent_panel`.

## Testing

- **Unit** — `ConversationBackend` routing (which backend a conversation submits to); the sqlite schema migration (adds `backend`, idempotent) and the `stream-history.json → sqlite` data migration (idempotent, non-destructive, parse-failure-tolerant).
- **Model** — the unified `ConversationModel` merges both sources; the daemon source appends entries from `CovenAgentEvent`s.
- **Store** — round-trip both `Cli` and `Daemon` conversations; migration marker prevents re-migration.
- **Layout** — the unified panel renders without panic (add the `View` layout-safety harness deferred in Phase 1).
- **Integration** — reuse the in-process daemon stub (`cast_agent`) to feed a daemon conversation into the unified model end-to-end.

## Implementation decomposition (one spec → phased plan)

- **2a — Shared core + backend enum + migration.** Generalize `ChatModel`/`ChatStore`/`ChatConversation` over `ConversationBackend`; add the `backend` schema column + the `stream-history.json` data migration. Behavior-preserving for CLI conversations (existing `cli_chat` tests gate it).
- **2b — Unified panel view + merged list.** New `agent_panel` view: conversation list (both backends) + `agent_transcript` transcript + composer shell, behind a feature flag + keybinding.
- **2c — Composer routing + daemon source wiring.** Route submit by backend; wire the Coven-daemon source (sessions + `stream_agent_events`); resolve the stream-mode prompt-delivery detail.
- **2d — Entry point + deprecate old panels.** One keybinding; retire `cli_chat::ChatPanelView` + the `ai_assistant` coven-stream section (staged: flag first, delete after parity).

## Risks

1. **Two backend lifecycles in one model** (CLI terminal-bound vs. daemon headless) — isolated by `ConversationBackend` + per-backend composer routing; the model itself only deals in `ChatConversation` + `ChatEntry`.
2. **`cli_chat` may not be wired to *live* CLI sessions** — its own note says "no caller binds them to a live `CLIAgentSessionsModel` yet." 2c must confirm/complete that wiring for the CLI backend to work live, or the CLI side inherits `cli_chat`'s current (possibly-unwired) state.
3. **Migration correctness** — non-destructive + idempotent + parse-tolerant, verified by unit tests before it touches a real `stream-history.json`.
4. **Deprecating the shipped agent panel** — staged behind a flag; delete only after parity, so there's no window without a working agent surface.
