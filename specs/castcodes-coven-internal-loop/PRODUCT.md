# CastCodes ↔ Coven Internal Conversion Loop — Product Spec

## Summary

Turn CastCodes into the daily internal tool for Coven work. Define a single canonical "lane" state machine for a unit of Coven-backed work — open repo → launch lane → inspect output → review diff → verify → PR/merge/archive — and frame every repeatable workflow ("ritual") as a named path through that machine. End every lane in a structured proof packet stored locally, and derive weekly internal updates from packets rather than from abstract roadmap language.

The product story for one sentence: **"In CastCodes, every Coven-backed task is a lane that ends in a proof packet."**

## Background — what already exists

CastCodes already has substantial infrastructure this spec composes on rather than rebuilds:

- **`cast_agent` crate** (`crates/cast_agent/`) — the gateway client, substrate collector, and runtime that surface Coven state to the agent panel. Today it expects a TCP `/v1/*` HTTP gateway that does not exist on the running Coven daemon.
- **Coven Sessions section** in the agent panel (`app/src/ai_assistant/panel.rs::render_sessions_section`) — read-only list of sessions with name + status dot + last-active timestamp, refreshed on a 60 s loop.
- **Session click-through** — clicking a row dispatches `WorkspaceAction::OpenCovenSessionInNewTab { name, cwd }` and opens a terminal tab at the session's CWD. The plumbing for "lane lives in a CastCodes tab" already works.
- **Worktree manager spec in flight** (`specs/castcodes-worktree-manager/`) — provides the isolated-worktree-per-lane primitive this spec composes on. We do not duplicate or reinvent its work.
- **Chat panel spec already shipped** (`specs/castcodes-chat-panel/`) — handles AI coding CLI conversations via the OSC 777 event protocol. Distinct from the Coven panel; this spec does not modify it.
- **The running Coven daemon** (`@opencoven/cli` v0.0.29, `coven daemon serve`) exposes a Unix socket at `~/.coven/coven.sock` with a session/event/familiar API rooted at `/api/v1/*`. The daemon's actual model is "harnesses (codex / claude / future) run as PTY-backed sessions; daemon owns lifecycle + records every byte as events." It is **not** a chat-completion gateway.
- **Public framing exists** in `docs/COVEN-POWERED-CASTCODES.md` — names the six rituals (Start Coding, Review Stack, Release Check, Fix OpenClaw, Coven Dogfood Quest, Multi-Harness Review) as Phase 4 roadmap items, and names the end-of-task retrospective (worked / missing / should-become-issue) as Phase 3. This spec is the implementation lens on that framing.

## Why

CastCodes is positioned in `docs/COVEN-POWERED-CASTCODES.md` as "the singular public proof surface for Coven." It cannot credibly be that proof surface until it is the daily internal tool the team actually uses for Coven work. Three concrete pain points today:

- **No canonical flow.** The lane lifecycle (open → launch → inspect → review → verify → merge) is implicit across multiple UI surfaces and not written down anywhere code can align against. Different internal workflows reinvent the same steps.
- **Rituals exist only as names.** The six rituals are listed in `COVEN-POWERED-CASTCODES.md` as roadmap milestones with no executable surface. Internal team uses ad-hoc CLI invocations instead, producing no shared evidence.
- **Weekly updates are abstract.** Without per-session structured artifacts, internal updates default to roadmap-shaped prose ("we're working on X") rather than evidence-shaped prose ("here are five lanes that ran this week, three packets worth showing publicly, two issues filed against OpenClaw").

Closing this loop unblocks acceptance criterion #1 of the active goal ("a real Coven/CastCodes task can be done end-to-end from CastCodes") and unblocks criteria #2 and #3 by giving them the substrate they need.

## Goals

- **A single state machine** for a CastCodes lane, written down, with explicit states / transitions / invariants. Every Coven-backed workflow reduces to a path through it.
- **A proof packet schema** — structured JSON, one file per lane, stored locally — capturing what worked, what broke, what should become an issue, what can be shown publicly.
- **Six named rituals** as concrete commands in CastCodes that drive a lane through a defined path. PLAN-01 in this directory implements the first (Start Coding); the other five get their own PLAN files (PLAN-02 through PLAN-06).
- **A working end-to-end loop** — a user inside CastCodes can launch a Coven-backed lane, watch it run, review the diff, verify, and end the lane with a packet on disk. No degraded-mode placeholders.
- **A weekly-update pipeline (later)** that globs packets and emits a derivable internal update. Not implemented in v1; the schema is designed so it can be implemented as pure data transformation later.

## Non-goals (v1)

- We do **not** add chat-completion endpoints to the Coven daemon. The daemon stays a session/event store; chat-completion-style UX stays in the existing chat panel using its existing CLI-driven path.
- We do **not** modify the chat panel spec (`specs/castcodes-chat-panel/`) or its event sources. Lanes and chat are sibling surfaces.
- We do **not** auto-publish proof packets anywhere. Packets are local. Publishing decisions live in a separate human review step on the `can_be_shown_publicly` slice.
- We do **not** ship the weekly-update aggregator in v1. v1 ends with packets on disk; aggregation is a follow-up spec.
- We do **not** rebuild the worktree manager. Lane launches delegate worktree creation to whatever `specs/castcodes-worktree-manager/` ships.
- We do **not** call any Warp-owned hosted service. CastCodes' fork-local OSS boundary stays intact; everything in this spec talks to either the local Coven daemon (`unix:///Users/<u>/.coven/coven.sock`) or a local CLI subprocess.

## The canonical lane state machine

A **lane** is one unit of Coven-backed work: one project, one harness, one purpose, one terminal-tab-and-records-thereof. Lanes are CastCodes-side abstractions; the Coven daemon does not need to know about them. The daemon knows about *sessions* (the PTY recording); a lane wraps a session plus the CastCodes-side review/verify/packet workflow on top.

### States

| State | Meaning | How CastCodes knows it's in this state |
|---|---|---|
| `proposed` | User has named a lane (repo + harness + initial prompt + worktree intent) but not launched. | Lane exists in the panel form; no daemon session yet. |
| `launching` | CastCodes is spawning the `coven` CLI in a new terminal tab; daemon is registering the session. | Tab open, polling `/api/v1/sessions` for a matching `project_root` + recent `created_at`. |
| `running` | Harness is doing work; daemon is recording events. | Daemon reports session `status="running"`. |
| `reviewing` | Harness has halted; user is inspecting diff and event log. | Daemon reports `status ∈ {completed, killed, orphaned, idle}`. Auto-transition. |
| `verifying` | User has triggered verification (project-defined; `cargo check`, `script/check_rebrand`, tests). Optional. | CastCodes is running a subprocess defined in `.castcodes/verify.toml` (or a fallback). |
| `merged` | Terminal. Diff has landed on the target branch. | User clicks "Merge" after `reviewing` or `verifying`. |
| `pr_open` | Terminal. A PR exists pointing at the lane's diff; no merge yet. | User clicks "Open PR" — CastCodes runs `gh pr create` (or surfaces an existing PR if one exists). |
| `archived` | Terminal. User abandoned the lane without merging; diff is preserved for reference. | User clicks "Archive". |
| `failed` | Terminal. Lane could not progress (launch failed, harness errored, verification failed and user chose to record the failure). | Daemon reports `status="failed"` OR user clicks "Failed" after a verify-bust. |

### Transitions

```
proposed ── launch ──────────▶ launching
launching ── spawned ────────▶ running
launching ── fail ───────────▶ failed
running   ── halt ───────────▶ reviewing
reviewing ── start verify ───▶ verifying
reviewing ── merge ──────────▶ merged       (writes packet)
reviewing ── open pr ────────▶ pr_open      (writes packet)
reviewing ── archive ────────▶ archived     (writes packet)
reviewing ── mark failed ────▶ failed       (writes packet)
verifying ── pass + merge ───▶ merged       (writes packet)
verifying ── pass + open pr ─▶ pr_open      (writes packet)
verifying ── fail + retry ───▶ running
verifying ── fail + archive ─▶ archived     (writes packet)
verifying ── fail + record ──▶ failed       (writes packet)
{archived,failed} ── redo ───▶ proposed     (new lane; packet preserved on old one)
```

### Invariants

- A lane in any non-terminal state has at most one *active* daemon session (`status ∈ {running, idle}`). Halted sessions retained for replay.
- Every transition into a terminal state writes exactly one packet file. No silent terminations.
- The `running → reviewing` transition is **automatic** (CastCodes observes daemon status); all other transitions require explicit user action.
- A lane's `project_root` and `harness` are immutable after `launching`. Changing either creates a new lane.

## Proof packet schema

### Location

`~/.coven/proof-packets/<session-id>.json` — co-located with daemon state for natural backup/redaction/transport semantics.

### Schema (v1)

```jsonc
{
  "packet_version": 1,
  "session_id": "uuid",                   // matches daemon session
  "lane_id": "uuid",                      // CastCodes-side lane id (allows lane:packet:session three-way join)
  "project_root": "/abs/path",
  "harness": "codex" | "claude" | "...",
  "ritual": "start-coding" | "review-stack" | "release-check"
          | "fix-openclaw" | "coven-dogfood-quest" | "multi-harness-review"
          | null,                         // null = ad-hoc lane not driven by a ritual
  "ritual_extras": { },                   // ritual-specific extra fields (see ritual outlines)

  "started_at": "RFC3339",                // proposed → launching
  "launched_at": "RFC3339",               // launching → running
  "halted_at": "RFC3339" | null,          // running → reviewing
  "ended_at": "RFC3339",                  // → terminal state
  "terminal_state": "merged" | "pr_open" | "archived" | "failed",

  "worked":              ["string", ...], // what behaved as expected
  "broke":               ["string", ...], // bugs, friction, surprises
  "should_become_issue": [
    { "title": "...", "body": "...", "repo": "OpenCoven/..." }
  ],
  "can_be_shown_publicly": ["string", ...], // safe-to-paste highlights for weekly updates

  "links": {
    "pr_url": "https://github.com/..." | null,
    "diff_sha": "abcdef..." | null,         // git rev of the lane's terminal diff
    "branch": "string" | null,
    "worktree_path": "/abs/path" | null,
    "event_seq_range": [first_seq, last_seq] // pointer into daemon event log
  },

  "verification": {
    "ran":     true | false,
    "command": "string" | null,             // exact command(s) run
    "exit_code": 0 | null,
    "summary":   "string" | null            // human-written summary of what passed/failed
  },

  "redaction_notes": "string"             // what was scrubbed before saving, e.g. "API key on event seq 137 elided"
}
```

### Lifecycle

- Packet draft form is opened on first transition into `reviewing`. User edits `worked` / `broke` / `should_become_issue` / `can_be_shown_publicly` as the session progresses.
- Packet is **persisted to disk** on first transition into a terminal state. Re-entering a packet's lane after that is a read-only render; further edits require a new lane.
- Redaction is **author-driven** in v1. The packet form surfaces redaction prompts ("Did you paste any secrets into this lane?") but the human is the redaction authority. Automated redaction is a v2 concern.

### Why local-only

Packets reference event log byte ranges from the daemon's local store. They are not portable across machines without exporting the event log too. v1 keeps the loop local for trust + simplicity; cross-machine export is a follow-up spec.

## Six ritual outlines

Each ritual is a named entry into the state machine with: an entry shortcut (command palette or button), preset values for the lane (harness, prompt template, worktree strategy), a canonical path through the states, and a packet flavor describing what the terminal packet should emphasize.

### Start Coding (PLAN-01 — first to implement)

- **Entry:** Command palette → "Coven: Start Coding". Picks current workspace as `project_root` by default.
- **Form:** Backend harness picker (codex / claude / future entries — sourced from the daemon's observed `session.harness` values, **not** from `/api/v1/familiars` which is a separable personas catalog), prompt textarea, worktree toggle (default on).
- **Path:** `proposed → launching → running → reviewing → verifying → {merged | pr_open | archived}`.
- **Packet flavor:** Generic. `should_become_issue` populated with anything that surprised the user; `can_be_shown_publicly` with anything safe.

**Note on familiars vs. harnesses:** the daemon exposes both a `harness` field on sessions (one of `codex`, `claude`, `cockpit`, etc.; identifies the backend CLI process) AND a `/api/v1/familiars` catalog of personas (`nova`, `sage`, `cody`, etc.; persona-with-role identity for routing intent). V1 rituals pick backend harnesses directly. A future "familiar router" layer can map "ask Cody to do X" onto "spawn codex/claude with the right system prompt" — out of scope for v1.

### Review Stack

- **Entry:** Command palette → "Coven: Review Stack". Takes a list of refs (PRs by URL, commits by SHA, branch names).
- **Form:** Stack inputs + harness picker.
- **Path:** N parallel `proposed → launching → running` lanes (one per item in the stack), each auto-transitions to `reviewing`; user reviews together.
- **Packet flavor:** One packet per stack item with shared `ritual_extras.review_stack_group_id`.

### Release Check

- **Entry:** Command palette → "Coven: Release Check". Pulls the current repo + sibling repos in the OpenCoven org.
- **Form:** Target version, harness picker, cross-repo opt-in checkboxes.
- **Path:** `proposed → launching → running → reviewing → verifying` (verifying is the load-bearing state — runs `script/check_rebrand`, `cargo check`, integration tests as defined in `.castcodes/verify.toml`). Terminal state is `merged` (go) or `failed` (no-go).
- **Packet flavor:** `terminal_state` doubles as release-go/no-go signal; `can_be_shown_publicly` becomes the release notes draft. `ritual_extras.target_version`.

### Fix OpenClaw

- **Entry:** Command palette → "Coven: Fix OpenClaw". Hard-pinned to `OpenCoven/OpenClaw` (or whichever downstream project the team is shepherding that week — config in `.castcodes/rituals.toml`).
- **Form:** Bug title / link, harness picker preset to whichever has best results on OpenClaw historically.
- **Path:** Same as Start Coding but with `project_root` and harness defaults pre-filled.
- **Packet flavor:** `links.pr_url` is the expected terminal artifact; `ritual_extras.target_repo` records which downstream this lane attacked.

### Coven Dogfood Quest

- **Entry:** Command palette → "Coven: Dogfood Quest". `project_root` hard-pinned to a Coven repo (`coven/`, `cast-codes/`, etc.); user picks which.
- **Form:** Quest text (the bug or feature for Coven itself), harness picker.
- **Path:** Same as Start Coding; the *content* is what makes it the dogfood — Coven exercising itself.
- **Packet flavor:** **Highest-value packet** for weekly updates. `can_be_shown_publicly` directly answers "is Coven good enough for this?" `should_become_issue` items default to filing against the relevant Coven repo. `ritual_extras.quest_text`.

### Multi-Harness Review

- **Entry:** Command palette → "Coven: Multi-Harness Review".
- **Form:** Same prompt + worktree, but pick N harnesses (default 2 — codex + claude).
- **Path:** N parallel lanes, one per harness, launched simultaneously against fresh worktrees with the same prompt. User reviews diffs side by side and picks a winner.
- **Packet flavor:** N packets share a `ritual_extras.multi_harness_group_id`. The winning packet has `terminal_state ∈ {merged, pr_open}`; losers `archived`. Each packet's `worked`/`broke` is harness-specific.

## User flows

### Start Coding (canonical)

1. User in CastCodes presses ⌘⇧P (command palette) → "Coven: Start Coding".
2. Inline form opens in the agent panel: harness picker (familiars list), prompt textarea, worktree toggle (on). `project_root` defaults to current workspace.
3. User types prompt, clicks "Launch".
4. Lane transitions `proposed → launching`. CastCodes creates a worktree (delegated to worktree manager) and spawns the `coven` CLI in a new terminal tab inside it. Tab title prefix `Coven: <prompt-snippet>`.
5. CastCodes polls `/api/v1/sessions` until a session for the new worktree's `project_root` appears (new `created_at` within the last 30 s), captures the `session_id`, attaches it to the tab.
6. Lane transitions `launching → running`. Tab badge turns green. Lane row in the panel shows live event count (read via `/api/v1/events?sessionId=...`) and tail of recent output.
7. Harness finishes; daemon reports `status="completed"`. Lane auto-transitions to `reviewing`. Panel surfaces a "Review" affordance: diff view (from worktree git state), event log replay, and the packet draft form.
8. User reviews diff. If they want to verify, they click "Verify" — CastCodes runs the project-defined verify command and captures the result into the packet draft.
9. User picks a terminal action: "Merge" (runs `git push` to target branch), "Open PR" (runs `gh pr create`), "Archive" (cleans up worktree), or "Failed" (records the failure mode). Packet is written to `~/.coven/proof-packets/<session-id>.json`.
10. Lane row collapses into a finalized "Lane archive" entry in the panel, with a link to the packet file.

### Reviewing an old packet

1. Agent panel → "Past lanes" tab → list of finalized packets, sortable by recency.
2. Click a packet → opens a read-only render: packet contents + diff + event log replay (if event log still in daemon store).
3. "View packet JSON" button opens the file in the editor.

### Weekly update derivation (v2)

(Not implemented in v1; sketched for spec completeness.)

1. User runs a CLI: `castcodes weekly --since 2026-05-20`.
2. Tool globs `~/.coven/proof-packets/*.json` filtered by `ended_at >= 2026-05-20`.
3. Emits a Markdown digest: ritual counts, top items from `can_be_shown_publicly`, all `should_become_issue` entries grouped by repo.
4. User reviews + edits + pastes into internal weekly update channel.

## Settings

New settings introduced (file: `~/Library/Application Support/dev.castcodes.CastCodes/settings.json`, key: `coven.internal_loop`):

| Key | Type | Default | Meaning |
|---|---|---|---|
| `coven_socket_path` | path | `$HOME/.coven/coven.sock` | Override daemon socket location. |
| `coven_cli_path` | path \| `auto` | `auto` | Resolved via `$PATH` lookup by default. |
| `packet_dir` | path | `$HOME/.coven/proof-packets` | Where packets are written. |
| `rituals_config_path` | path | `<project_root>/.castcodes/rituals.toml` | Per-project ritual config (Fix OpenClaw target, harness defaults). |
| `verify_config_path` | path | `<project_root>/.castcodes/verify.toml` | Per-project verification commands. |

No CLI config is mandatory; sensible defaults match the running environment on the dev machine.

## Verification

- Public-surface strings and assets pass `./script/check_rebrand` (rebrand guard rule from `CASTCODES.md`).
- `cargo check -p cast_agent` clean after the wire changes in PLAN-01.
- `cargo check -p warp-app --bin cast-codes --features gui` clean.
- Manual smoke test (described in PLAN-01 Phase 5): launch CastCodes, run Start Coding against the `cast-codes` repo itself with a trivial prompt, confirm lane progresses through every state, confirm packet file exists.

## Open questions (resolved-but-flagged-for-review)

These were settled during brainstorming but flagged here so a future reader knows they were deliberate:

- **Why a state machine vs. a runbook?** State machine lets rituals compose declaratively (paths through the same graph) instead of duplicating procedure. Runbooks calcify and drift from code. See brainstorming transcript 2026-05-26.
- **Why local packet storage, not daemon-side event kind?** Packets are derived metadata over events; storing them as a separate file keeps the weekly aggregator a pure filesystem glob rather than a cross-session `/api/v1/events` traversal.
- **Why not extend the existing chat panel to host lanes?** Chat panel = one-shot completion. Lanes = recorded multi-step agentic sessions with diffs and packets. Different invariants; sibling surfaces, not the same surface.
- **Why JSON over a binary format?** Packets are read by humans during weekly update derivation. JSON keeps the file `jq`-able without a tool. Tradeoff accepted.

## Future work explicitly out of scope for v1

- Weekly-update aggregator CLI / UI.
- Cross-machine packet export.
- Automated redaction of secrets from packets.
- Daemon-side chat completion endpoints (would change the architecture; not in scope here).
- Multi-Harness Review UI side-by-side diff comparison polish (the lanes work in v1; the polish is its own spec).
- Packet attachment of binary artifacts (screenshots, profiles) — packet JSON references them by path only in v1.
