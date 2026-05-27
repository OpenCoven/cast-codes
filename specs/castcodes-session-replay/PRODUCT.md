# CastCodes Session Replay — PRODUCT

**Status:** Draft v1 · 2026-05-26
**Owner:** CastCodes app
**Coven contracts consumed:** `coven-session-artifacts`, `coven-handoff-packet`, `coven-gateway-wire`, `coven-trust-layer`
**Acceptance target:** "CastCodes can show useful session artifacts without exposing unsafe details."

## Problem

Today CastCodes' `AIAssistantPanelView` shows a live list of Coven sessions and a streaming chat pane (`COVEN STREAM • LIVE`). There's no way to open a past session, see what an agent actually did, or hand off context to another harness. With the Coven-side trust + artifact specs landing, CastCodes can finally render full sessions safely — but it needs an explicit UI contract for what it shows and what it deliberately hides.

This spec defines that contract.

## Scope

A new **Session Replay** view, reachable from:

- The existing sessions list in `AIAssistantPanelView::render_sessions_section()` (click a session → open replay).
- A new "Recent Sessions" entry point on the workspace's command palette.
- Deep link: `castcodes://sessions/:id` (future; not blocking for v1).

The view is **read-only**. No actions that mutate Coven state are taken from inside Replay (no resume, no delete, no decrypt) in v1.

## Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  Session Replay                                                  ✕  │
├──────────────────────────────────────────────────────────────────────┤
│  ●  cargo refactor                              Claude → Codex      │
│  Started 2h ago · 7 files changed · 3 commands · 1 verification      │
│  Verdict: completed                                                  │
├──────────────────────────────────────────────────────────────────────┤
│  Filter:  [all] [transcript] [command] [changed file] [verification] │
│           [handoff] [summary]                                         │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ●─ Claude (run abc-123) ─────────────────────                       │
│   │                                                                  │
│   ├─ 14:02:03  transcript · user                                     │
│   │   > refactor store.rs to add provenance columns                  │
│   │                                                                  │
│   ├─ 14:02:11  transcript · assistant                                │
│   │   I'll add four columns: producer_harness, ...                   │
│   │                                                                  │
│   ├─ 14:02:41  command   cargo check        exit 0 · 2.4s            │
│   ├─ 14:03:12  changed_file  crates/.../store.rs   +42 / -3          │
│   ├─ 14:03:30  verification cargo test     ✓ pass  · 12.1s           │
│   └─ 14:03:45  handoff   → Codex                                     │
│                                                                      │
│  ●─ Codex (run def-456) ─────────────────────                        │
│   │                                                                  │
│   ├─ 14:04:01  transcript · assistant                                │
│   │   Continuing from handoff: I'll wire the manifest endpoint...    │
│   ...                                                                │
└──────────────────────────────────────────────────────────────────────┘
```

## Header section

The header shows session-level summary derived from the manifest in one fetch:

- **Title** (`sessions.title`, falling back to first transcript line's first 60 chars).
- **Status pill** — `running`, `complete`, `crashed`, `archived`.
- **Provenance summary** — `Claude → Codex` rendered from the manifest's `provenance` list, in order.
- **Counts** — files changed, commands, verifications. Pulled from `manifest.artifacts.*.count`.
- **Final verdict** — from `summary.verdict` if present, else "no summary."

If a handoff exists but no summary, the header shows `In progress · waiting for next harness`.

## Timeline section

Chronological list of artifact entries, grouped visually by `producer_harness` + `producer_run_id`. Each entry is a single line plus an expand-on-click body. Filter chips at the top let the user narrow to one kind.

Per-kind rendering:

- **`transcript`** — role icon (user/assistant/system), expandable to full text. Text is the already-redacted form from Coven; CastCodes does **not** apply additional redaction. (If Coven sent it, it's safe to show.)
- **`event`** — single line: `event · session_start`, no expand body for known subtypes; expandable for `custom:*`.
- **`command`** — single line: `command  <argv[0]>  exit <N> · <duration>`. Expandable to show full argv, stdout, stderr (truncated to first 8 KiB; "view full output" link is disabled in v1 because decrypt is Unix-socket-only).
- **`changed_file`** — line: `changed_file  <path>  +<bytes> / -<bytes>` with action icon (`+` created, `~` modified, `−` deleted, `→` renamed). Expand shows pre/post sha256 hashes (first 12 chars), the artifact ref ids, and a disabled "view diff" affordance with tooltip "open this session in the Coven CLI to view file contents."
- **`verification`** — line: `verification  <tool>  ✓ pass | ✗ fail | ⊘ skip · <duration>`. Expand shows summary + inline output (already truncated at write time).
- **`handoff`** — line: `handoff  →  <to.harness>`. Expand renders the **handoff packet card** (next section).
- **`summary`** — line: `summary · verdict: <verdict>`. Expand shows the prose + structured fields. Always rendered, even when filter is set, since the summary is the closing event.

## Handoff packet card

A dedicated render for `coven.handoff.v1` packets. Six labelled sections in fixed order matching `coven-handoff-packet`:

```
┌─ HANDOFF ────────────────────────────────────────────────────┐
│  From:  Claude (abc-123)        → To:  Codex                 │
│  Trigger: harness_initiated     Created: 14:03:45            │
├──────────────────────────────────────────────────────────────┤
│  TASK CONTEXT                                                 │
│    Goal: <task_context.original_goal>                         │
│    Constraints:                                               │
│      • <task_context.constraints[0]>                          │
│      • <task_context.constraints[1]>                          │
│                                                               │
│  CURRENT STATE                                                │
│    Last action: <current_state.last_action>                   │
│    Open questions: ...                                        │
│                                                               │
│  FILES TOUCHED  (3)                                           │
│    • crates/.../store.rs       (jump to event)                │
│    • crates/.../privacy.rs     (jump to event)                │
│                                                               │
│  RISKS  (1 blocking)                                          │
│    ⚠ incomplete_edit          [blocking]                      │
│      Cookie handling in privacy.rs is partial.                │
│                                                               │
│  VERIFICATION                                                 │
│    ✓ cargo test    pass at 14:03:30        ●  stale: no       │
│                                                               │
│  NEXT ACTION                                                  │
│    Wire the manifest endpoint to use the new provenance       │
│    columns and add a contract test.                           │
│    Do not:                                                    │
│      • re-run cargo test before the wire change               │
│    Expected outcome: manifest contains provenance entries.    │
└──────────────────────────────────────────────────────────────┘
```

The card is the **only** UI in CastCodes that interprets a handoff packet. Every field is rendered exactly as Coven returned it; CastCodes does not invent or omit fields.

## What CastCodes deliberately does NOT show in v1

These are the boundaries the trust-layer spec assumes Coven enforces; CastCodes never tries to work around them:

- **Artifact bodies** (raw file snapshots, full command output past 64 KiB). The expand UI shows a disabled "decrypt requires the Coven CLI" affordance.
- **Decrypted contents**, even if a CastCodes user with daemon access could fetch them via Unix socket. CastCodes only speaks `/v1/*` over TCP.
- **Tokens or keys**. The bearer token CastCodes uses is read from `~/.coven/token` and never displayed.
- **Provenance for events that lack it**. If a backfilled old event has NULL `producer_harness`, the timeline shows `unknown harness` rather than guessing.
- **Sub-agent attribution chains beyond `producer_harness` + `producer_run_id`**. v1 does not synthesize a "this came from a sub-spawned agent" view; that's a future spec.

## Empty / error states

- **Coven gateway offline:** show the same gateway-status pill the panel already shows, plus "Open a Coven daemon to view session history."
- **Session not found:** "This session is not available. It may have been archived to disk or pruned."
- **Session has no events:** "Session was started but no events were recorded."
- **Pruned partial:** if some events for the session are missing because of retention pruning, show a banner: "Some events from this session were pruned on <date>. The timeline below is incomplete."
- **Unknown artifact kind:** if Coven serves a kind CastCodes doesn't recognize (forward-compatibility), show it as `unknown(<kind>)` line, no expand body, no crash.

## Performance

- Single manifest fetch on open. Manifest is bounded in size (count + per-event summary fields, no bodies). Acceptable to fetch synchronously.
- Full event bodies on expand: lazy fetch from `GET /v1/sessions/:id/events?after_seq=<N>&limit=1`.
- Live updates: if the session is `running`, subscribe to `/v1/events` cursor and append new events to the timeline.

## Acceptance for v1

Session Replay is "v1 done" when:

1. Clicking a session in the existing sessions list opens the Replay view.
2. A representative multi-harness session (Claude → Codex with at least one command, one changed file, one verification, one handoff, one summary) renders end-to-end with no decrypt and no errors.
3. The handoff card renders all six packet sections.
4. Filter chips correctly subset the timeline.
5. Pruned/partial sessions show the appropriate banner instead of failing.
6. No code path in Replay calls a `/api/v1/*` endpoint or a privileged `/v1/*` endpoint (Unix socket is not used; privileged endpoints return 404 over TCP and CastCodes doesn't ask).

## Out of scope for v1

- Resuming a session from CastCodes (would require a Unix-socket call).
- Editing or annotating handoff packets in-app.
- Cross-session search ("show me all sessions where verification failed").
- Diff view for changed files (decrypt is Unix-socket-only).
- Exporting a session as a portable archive (future).
- Mobile/web replay surface (future; see `CASTCODES-BROWSER-PANEL` spec).
