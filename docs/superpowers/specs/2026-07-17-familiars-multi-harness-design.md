# Familiars — Multi-Harness Selection for the CastCodes Agent

**Status:** Design (pending implementation plan)
**Date:** 2026-07-17
**Base:** `main` @ `0e493ff1` (post PR #190 "Familiar" rename, post PR #191 native Coven Code)

## Summary

Let the user summon and select among multiple **Familiars** — persona identities
served by the Coven daemon's `/api/v1/familiars` catalog (`nova`, `sage`, `cody`,
each with an emoji, role, and description) — instead of the agent panel always
talking to the single hardcoded `coven-code` harness.

Selection is **per-conversation** with a configurable **global default**. When the
daemon catalog is unavailable (the current reality — the endpoint is not shipped
yet), the picker **degrades gracefully** to the known backend harnesses so the
panel keeps working exactly as it does today.

## Concept model (from `specs/castcodes-coven-internal-loop/PRODUCT.md`)

The product deliberately separates two things; this design honors that split:

- **Harness** — the backend CLI engine that runs (`coven-code`, `codex`,
  `claude`, `cockpit`, …). This is the `harness` field already carried on the
  daemon `POST /api/v1/sessions` request body.
- **Familiar** — a *persona* identity with a role/system-prompt, served by the
  daemon's `/api/v1/familiars` catalog. A Familiar resolves to a harness (plus a
  system prompt) **daemon-side**; the client does not invent that mapping.

This design adds the client-side Familiar **selection + display** layer. It does
NOT implement the daemon-side "familiar router" (persona → harness + prompt),
which `PRODUCT.md` marks out of scope for v1 and which lives in the daemon.

## Goals

- A Familiar picker in the agent panel, per-conversation, with a global default.
- Consume the daemon `/api/v1/familiars` catalog as the source of truth for the
  Familiar list (rich personas: id, name, emoji, role, description, status).
- Inject the selected Familiar into the existing per-request `harness`/`familiarId`
  path so the daemon spawns the right backend for that conversation.
- Graceful degradation to the backend harness list when the catalog is absent, so
  nothing regresses while the daemon endpoint is pending.
- Persist the global default; persist per-conversation selection with the
  conversation.

## Non-goals

- Implementing the daemon `/api/v1/familiars` endpoint (external Coven daemon).
- The familiar-router (persona → harness + system prompt mapping). Daemon-side, post-v1.
- Editing/creating Familiars from CastCodes (the catalog is daemon-owned; the
  client is read + select only).
- Multi-Familiar-per-conversation / orchestration (Oz). Out of scope.

## Architecture

New and touched units, each with one clear responsibility:

### 1. `crates/cast_agent/src/daemon_schema.rs` (new)
Wire types matching the daemon `/api/v1/*` shape, aligned with
`specs/castcodes-coven-internal-loop/PLAN-01-start-coding.md` so the in-flight
internal-loop work can reuse them rather than duplicate:

```rust
pub struct DaemonFamiliar {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub emoji: String,
    pub role: String,
    pub description: String,
    pub status: String,          // e.g. "online" | "offline"
    pub last_seen: String,
    pub active_sessions: u32,
    pub memory_freshness: String,
}
```
Deserialization is tolerant (unknown fields ignored; missing optional-ish fields
default) so daemon schema drift does not hard-fail the client.

### 2. `crates/cast_agent/src/gateway.rs` — `list_familiars()`
```rust
/// GET /api/v1/familiars. Returns an empty Vec (not an error) when the daemon
/// does not implement the endpoint (404) or is unreachable, so the UI degrades
/// instead of erroring.
pub async fn list_familiars(&self) -> anyhow::Result<Vec<DaemonFamiliar>>;
```
Dispatches over the existing `UnixHttpClient` (unix socket) / `reqwest` split.

### 3. `crates/cast_agent/src/familiar.rs` (new) — selection service
- Fetches + caches the catalog (background refresh, sync snapshot for the render
  thread — mirrors the existing `SessionStore` pattern in `session.rs`).
- Holds the **global default** familiar id (from config).
- `resolve(selected: Option<&str>) -> RequestTarget` where:
  ```rust
  pub enum RequestTarget {
      /// A daemon-catalog Familiar. Sent as `familiarId`; the daemon routes it
      /// to a harness + system prompt (daemon-owned mapping, future contract).
      Familiar(String),
      /// A plain backend harness (fallback when the catalog is unavailable, or
      /// when the user picks a raw harness). Sent as `harness`.
      Harness(String),
  }
  ```
  The client never invents the persona→harness mapping: a Familiar selection sends
  the opaque `familiarId`; only the fallback path sends a concrete `harness`.
- `SUPPORTED_HARNESSES: &[&str]` constant (fallback list, e.g.
  `["coven-code", "codex", "claude"]`) — single source of truth for the harness
  fallback, matching PLAN-01's intent to centralize this.

### 4. `crates/cast_agent/src/config.rs` — default persistence
Add `default_familiar: Option<String>` to `CastAgentConfig`, loaded from
`COVEN_DEFAULT_FAMILIAR` env and `~/.coven/config.toml` (`default_familiar = "..."`),
consistent with the existing gateway_url/token/socket loading.

### 5. `app/src/ai_assistant/panel.rs` — picker + per-conversation state
- Add `selected_familiar: Option<String>` to `CovenStreamState`; new conversations
  seed it from the service's default.
- Render a Familiar chip in the panel header next to the existing gateway status
  pill (`render_gateway_status_pill`). Clicking opens a dropdown of catalog
  personas (emoji + display_name + role); when the catalog is empty it lists
  `SUPPORTED_HARNESSES` instead.
- When assembling the request, `resolve()` produces a `RequestTarget`:
  `Harness(name)` populates the existing `msg.body["harness"]` field (no wire
  change — `gateway.rs` already reads it; we just stop hardcoding
  `COVEN_CODE_HARNESS`); `Familiar(id)` populates a new `msg.body["familiarId"]`
  field that `gateway.rs::launch_daemon_session()` forwards when present. Today,
  with no catalog, only the `Harness` branch is exercised end-to-end.

### 6. `app/src/settings_view/ai_page.rs` — Familiars subsection
A read-only "Familiars" section: lists the catalog (emoji, name, role, status)
and a dropdown to pick the global default (writes `default_familiar`). No
create/edit.

## Data flow

```
daemon GET /api/v1/familiars ──▶ gateway.list_familiars() ──▶ FamiliarService (cache)
                                                                     │
panel header chip ◀── snapshot ──────────────────────────────────────┤
   user picks Familiar ──▶ CovenStreamState.selected_familiar         │
                                                                     ▼
send prompt ──▶ resolve(selected|default) ──▶ RequestTarget
                    Harness(name) → msg.body["harness"]  ──┐
                    Familiar(id)  → msg.body["familiarId"]─┴─▶ POST /api/v1/sessions
```

## Degradation (today's reality: no daemon endpoint)

- `list_familiars()` returns `[]` on 404/unreachable.
- The picker shows `SUPPORTED_HARNESSES`; default resolves to `coven-code`.
- Behavior is identical to today's single-harness panel — zero regression. The
  personas simply appear once the daemon ships `/api/v1/familiars`.

## Persistence

- **Global default:** `~/.coven/config.toml` `default_familiar` (+ env override).
- **Per-conversation:** `selected_familiar` stored on `CovenStreamState` and
  written alongside the conversation in the existing stream-history persistence.

## Testing

Unit tests (cast_agent):
- `list_familiars()` parses a well-formed catalog; returns `[]` on 404 and on
  malformed/empty body (no panic).
- `FamiliarService.resolve()`: catalog present → familiar id resolves; catalog
  empty → falls back to `coven-code`; explicit per-conversation selection wins
  over default; default wins over `coven-code`.
- `config.rs`: `default_familiar` loads from env and TOML; absent → `None`.

Panel-level:
- Extend `castcodes_public_surface_tests` / panel tests to assert the picker
  renders the fallback harness list when the catalog is empty, and the selected
  familiar flows into the request body.

## Risks & dependencies

- **Daemon dependency (accepted):** rich personas are inert until the Coven
  daemon serves `/api/v1/familiars`. This design ships the client "ready to light
  up" with a harness fallback so it is useful in the meantime.
- **Overlap with `castcodes-coven-internal-loop` PLAN-01:** that spec also
  introduces `daemon_schema.rs`, `DaemonFamiliar`, `list_familiars()`, and a
  centralized harness list. This design intentionally uses the same names/shapes;
  if PLAN-01 lands first, this feature consumes its types instead of adding them.
  Coordinate before implementing to avoid duplicate definitions.
- **Concurrent `cast_agent` edits:** streaming/crash-recovery work is active on
  the crate. Implement in the isolated `feat/familiars-multi-harness` worktree and
  rebase onto the latest `main` before opening the PR.

## Rebrand / attribution notes

- User-visible strings use **"Familiar"/"Familiars"** (per the merged rename);
  internal identifiers (`cast_agent` crate, `CastAgent`) stay.
- Commits in this repo carry **no AI-attribution trailer** (repo hard rule,
  enforced by `check_ai_attribution`). Run `check_rebrand` + `check_ai_attribution`
  before pushing.
