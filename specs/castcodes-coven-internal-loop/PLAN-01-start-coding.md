# CastCodes ↔ Coven Internal Conversion Loop — Plan 1: Start Coding (wire foundation + first ritual)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Read [`PRODUCT.md`](./PRODUCT.md) before starting; this plan assumes it has been read.
>
> **Signing rule:** Every `git commit` in this plan MUST pass `-S`. The repo's local `commit.gpgsign` is false; without `-S` commits land unsigned. After each commit, run `git log -1 --show-signature` and confirm a `Good "<algorithm>" signature` line. If signing failed, STOP and surface to the user.
>
> **AI attribution rule:** Commit messages and any file added by this plan MUST contain no AI-attribution markers. Do NOT add `Co-Authored-By:` lines naming an AI tool, model, or harness. Do NOT add "Generated with …" footers. Run `./script/check_ai_attribution` before every commit if available; otherwise verify by grep.
>
> **Branch verification rule:** Before each commit run `git branch --show-current`. If the branch is not `cast/coven-internal-loop` (or whatever branch you started this work on), STOP — a parallel call may have switched HEAD.
>
> **Uncommitted-work guard:** Before starting Phase 0, confirm the working tree contains the user's pre-existing uncommitted changes in `app/src/tab_configs/branch_picker.rs`, `app/src/tab_configs/worktree_picker.rs`, `app/src/terminal/view/tab_metadata.rs`, and the untracked `specs/castcodes-session-replay/` directory. DO NOT touch those files in this plan. If they are missing, STOP and ask the user — they may have been stashed or branched elsewhere.

**Goal:** Land the wire foundation (CastCodes ↔ Coven daemon over Unix socket + `/api/v1/*`) and the first ritual (Start Coding) end to end. Acceptance: a user inside CastCodes can run the Start Coding ritual against the `cast-codes` repo itself with a trivial prompt, watch the lane progress through every state from `proposed` to a terminal state, and end with a proof packet file on disk at `~/.coven/proof-packets/<session-id>.json`.

**Architecture:** Five focused subsystems landing in dependency order. (1) Replace the cast_agent gateway's TCP `/v1` transport with a Unix-socket `/api/v1` transport + schema adapter; gate the now-inert chat endpoints behind a feature flag. (2) Add a CastCodes-side lane model + state machine driver living in a new `crates/cast_agent/src/lane.rs` module. (3) Grow `panel.rs::render_sessions_section` into a viewer + launcher with a lane row per active lane and a "Start Coding" form. (4) Implement the launcher: spawn `coven` CLI in a worktree, attach to the resulting session, drive state transitions on daemon status events. (5) Implement the terminal-action handlers (Merge / Open PR / Archive / Failed) and the packet writer.

**Tech Stack:** Rust 2024 edition (workspace default), `hyper` + `hyperlocal` (new dep — minimal, well-maintained, workspace-friendly) for Unix-socket HTTP, `tokio` (existing), `serde` / `serde_json` (existing), `gpui`-style entity system from `warpui` (existing). No new dependencies for the panel or lane modules beyond what cast_agent already pulls in.

---

## Files created or modified

**Created:**

| Path | Responsibility |
|---|---|
| `crates/cast_agent/src/unix_http.rs` | Thin Unix-socket HTTP client built on `hyperlocal` + `hyper`. Surface: `UnixHttpClient::get(path) -> Response`, `delete(path) -> Response`, `post_json(path, body) -> Response`. |
| `crates/cast_agent/src/daemon_schema.rs` | Daemon-side types matching `/api/v1/*` exact wire shape (`DaemonSession`, `DaemonEvent`, `DaemonFamiliar`). Internal to cast_agent — these are wire types, not the public API. |
| `crates/cast_agent/src/adapter.rs` | Bidirectional adapters `daemon_schema::DaemonSession → session::CovenSession` and (where applicable) event mapping. Keeps the public CovenSession shape stable. |
| `crates/cast_agent/src/lane.rs` | `Lane` model + `LaneState` enum + `LaneStateMachine` driver. Maps daemon `status` → `LaneState` transitions; emits transition events for the UI. |
| `crates/cast_agent/src/packet.rs` | `ProofPacket` struct (v1 schema from PRODUCT.md), serde, write_packet helper, atomic temp-and-rename write to `~/.coven/proof-packets/<session-id>.json`. |
| `crates/cast_agent/tests/unix_http.rs` | Async test bringing up a `hyper` server bound to a temp Unix socket; exercises `get`/`post`/`delete`. |
| `crates/cast_agent/tests/lane_state.rs` | Pure-state-machine tests: every transition listed in PRODUCT.md is unit-tested for accept / reject. |
| `crates/cast_agent/tests/packet_roundtrip.rs` | Write packet → read back → verify shape; tests the v1 schema explicitly. |
| `app/src/ai_assistant/coven_panel.rs` | New panel subview owning the lanes UI: lane row, lane state badge, Start Coding form, review actions, packet draft form. Split from `panel.rs` to keep that file focused on the existing chat composer. |
| `app/src/ai_assistant/coven_panel_tests.rs` | View-model tests for lane row rendering + state badge resolution + form validation. |

**Modified:**

| Path | Change |
|---|---|
| `crates/cast_agent/Cargo.toml` | Add `hyper = { version = "1", features = ["client", "http1"] }` (if not already present), `hyperlocal = "0.9"`, `hyper-util = { version = "0.1", features = ["client-legacy", "tokio"] }`. Wrap legacy chat code in a new `legacy-gateway-chat` feature gate. |
| `crates/cast_agent/src/config.rs` | Default `gateway_url` becomes `unix:///<HOME>/.coven/coven.sock`. Continue accepting `http://...` and `unix://...` via env override. Emit deprecation warning for `http://localhost:3000`. Add `packet_dir` config (default `~/.coven/proof-packets`). |
| `crates/cast_agent/src/gateway.rs` | Remove TCP-only assumption from `health_probe`, `list_sessions`, `get_session` (new), `list_events` (new), `list_familiars` (new). Internally dispatch to `UnixHttpClient` when scheme is `unix://`, `reqwest::Client` otherwise. Path prefix swapped from `/v1` to `/api/v1`. Adapter applied so the public `CovenSession` shape stays stable. Gate `send_message`, `stream_messages`, `open_session`, `close_session` behind `#[cfg(feature = "legacy-gateway-chat")]`. Update module docs. |
| `crates/cast_agent/src/session.rs` | Add fields needed by lanes: `project_root`, `harness`, `title`, `exit_code`, `archived_at`, `created_at`, `updated_at`, `conversation_id` (matching daemon shape). Keep `id`, `name` (mapped from `title`), `status`, `last_active` (mapped from `updated_at`), `cwd` (mapped from `project_root`). |
| `crates/cast_agent/src/runtime.rs` | Boot a `LaneStateMachine` instance on the cast_agent runtime. Periodic 5-second `list_sessions()` refresh drives auto-transitions (`running → reviewing`). Expose `lanes()` accessor for the panel. |
| `crates/cast_agent/src/lib.rs` | `pub mod lane; pub mod packet; pub mod unix_http; pub(crate) mod daemon_schema; pub(crate) mod adapter;`. |
| `app/src/ai_assistant/panel.rs` | Remove inline `render_sessions_section` body, delegate to `coven_panel::render`. Header `render_gateway_status_pill` semantics unchanged. |
| `app/src/ai_assistant/mod.rs` | `mod coven_panel;`. |
| `app/src/workspace/view.rs` | `add_new_coven_session_tab` grows a sibling `spawn_coven_lane_in_worktree(harness, prompt, project_root) -> WorkspaceAction` that creates a worktree (delegating to existing worktree manager), opens a new terminal tab inside it, runs `coven <harness> --prompt "..."` (or the harness's CLI invocation) as the tab's startup command, and returns the tab id for lane attachment. |
| `CAST-AGENT.md` | New section "Lanes" describing the lane model and pointing to this spec. Keep existing Status section authoritative for the gateway shape — add a note about the transport switch. |
| `DESIGN-CHANGES.md` | Append entry noting the Coven Panel viewer + Start Coding ritual + Unix-socket gateway transport. |

---

## Phase 0 — Environment + branch checks

### Task 0.1: Confirm working environment

**Files:** none.

- [ ] **Step 1: Confirm branch and uncommitted-work guard**

Run:
```bash
git branch --show-current
git status
```
Expected: branch `cast/coven-internal-loop` (create from `main` if not present: `git checkout -b cast/coven-internal-loop`). The status MUST show the user's pre-existing changes in `app/src/tab_configs/branch_picker.rs`, `app/src/tab_configs/worktree_picker.rs`, `app/src/terminal/view/tab_metadata.rs` (modified) and `specs/castcodes-session-replay/` (untracked). DO NOT include those in any commit during this plan.

- [ ] **Step 2: Confirm signing key is set**

```bash
git config --get user.signingkey
git config --get gpg.format
```
Both MUST return a non-empty value. If empty, STOP and surface to the user (per global rule in `~/.claude/CLAUDE.md`).

- [ ] **Step 3: Confirm Coven daemon is running and reachable**

```bash
curl -s --unix-socket ~/.coven/coven.sock http://localhost/api/v1/health | jq .ok
```
Expected: `true`. If unreachable, start it: `coven daemon serve &` and re-run.

- [ ] **Step 4: Confirm cargo check baseline is clean**

```bash
cargo check -p cast_agent
cargo check -p warp-app --bin cast-codes --features gui
```
Both MUST succeed before any code change. If failures exist, STOP and surface to the user — they are pre-existing and not in scope for this plan.

---

## Phase 1 — Wire foundation: Unix-socket transport + /api/v1 + adapter

### Task 1.1: Add Unix-socket HTTP dep + module skeleton

**Files:** `crates/cast_agent/Cargo.toml`, `crates/cast_agent/src/lib.rs`, `crates/cast_agent/src/unix_http.rs`.

- [ ] **Step 1: Add dependencies**

In `crates/cast_agent/Cargo.toml`, add under `[dependencies]`:
```toml
hyper = { version = "1", features = ["client", "http1"] }
hyperlocal = "0.9"
hyper-util = { version = "0.1", features = ["client-legacy", "tokio"] }
http-body-util = "0.1"
```
Verify versions resolve in the workspace lockfile (`cargo check -p cast_agent`); if the workspace pins different majors, defer to the workspace pin.

- [ ] **Step 2: Create `unix_http.rs` skeleton**

Implement:
```rust
pub struct UnixHttpClient { socket: PathBuf, client: Client<UnixConnector, Full<Bytes>> }
impl UnixHttpClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self;
    pub async fn get(&self, path: &str) -> anyhow::Result<Response>;
    pub async fn delete(&self, path: &str) -> anyhow::Result<Response>;
    pub async fn post_json<T: Serialize>(&self, path: &str, body: &T) -> anyhow::Result<Response>;
}
pub struct Response { pub status: u16, pub body: Bytes }
impl Response { pub fn json<T: DeserializeOwned>(&self) -> anyhow::Result<T>; }
```
Use `hyperlocal::Uri::new(socket, path)` to construct request URIs. Surface non-2xx as `Err` with status code preserved in the error.

- [ ] **Step 3: Wire `mod unix_http;` into `lib.rs`**

Add `pub mod unix_http;` (pub because tests use it).

- [ ] **Step 4: Commit**

```bash
git add crates/cast_agent/Cargo.toml crates/cast_agent/src/unix_http.rs crates/cast_agent/src/lib.rs
git commit -S -m "feat(cast_agent): add Unix-socket HTTP client skeleton"
git log -1 --show-signature   # confirm signed
```

### Task 1.2: Unix-socket HTTP integration test

**Files:** `crates/cast_agent/tests/unix_http.rs`.

- [ ] **Step 1: Write the test**

Spin up a `hyper::server::conn::http1::Builder` bound to a `tokio::net::UnixListener` at a temp path. Serve three routes: `GET /api/v1/health` returning `{"ok": true}`, `POST /api/v1/echo` returning the request body, `DELETE /api/v1/thing/:id` returning 204. The test then constructs `UnixHttpClient` against the temp socket, exercises each route, asserts on status and body.

- [ ] **Step 2: Run + commit**

```bash
cargo test -p cast_agent --test unix_http
git add crates/cast_agent/tests/unix_http.rs
git commit -S -m "test(cast_agent): integration test for Unix-socket HTTP client"
git log -1 --show-signature
```

### Task 1.3: Daemon-schema types

**Files:** `crates/cast_agent/src/daemon_schema.rs`, `crates/cast_agent/src/lib.rs`.

- [ ] **Step 1: Define wire types**

Mirror the exact daemon JSON shapes observed in PRODUCT.md "Background":
```rust
pub struct DaemonSession {
    pub id: String,
    pub project_root: Option<String>,
    pub harness: Option<String>,
    pub title: Option<String>,
    pub status: DaemonSessionStatus,
    pub exit_code: Option<i32>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub conversation_id: Option<String>,
}
pub enum DaemonSessionStatus { Running, Completed, Killed, Failed, Orphaned, Idle, Cockpit }
pub struct DaemonEvent { pub seq: u64, pub id: String, pub session_id: String, pub kind: String, pub payload_json: String, pub created_at: DateTime<Utc> }
pub struct DaemonFamiliar { pub id: String, pub name: String, pub display_name: String, pub emoji: String, pub role: String, pub description: String, pub status: String, pub last_seen: String, pub active_sessions: u32, pub memory_freshness: String }
```
All `#[derive(Debug, Clone, Deserialize)]`. Status enum `#[serde(rename_all = "lowercase")]`.

- [ ] **Step 2: Wire `pub(crate) mod daemon_schema;` into `lib.rs`. Commit.**

### Task 1.4: Adapter from daemon schema to public CovenSession

**Files:** `crates/cast_agent/src/adapter.rs`, `crates/cast_agent/src/session.rs`, `crates/cast_agent/src/lib.rs`.

- [ ] **Step 1: Extend `CovenSession`**

Add the fields enumerated in the table above. Make `name` derived from `title` (or fall back to `format!("{}@{}", harness, project_root)` if title is empty). Make `last_active` derived from `updated_at`. Make `cwd` derived from `project_root`.

- [ ] **Step 2: Implement adapter**

```rust
pub(crate) fn daemon_to_public(d: DaemonSession) -> CovenSession { ... }
```
Define a `CovenSessionStatus` enum that aliases over the daemon status with a `to_public_label` for the panel pill.

- [ ] **Step 3: Unit test adapter**

In `adapter.rs` `#[cfg(test)] mod tests`, cover the empty-title fallback, null `project_root`, and each status variant.

- [ ] **Step 4: Commit**

### Task 1.5: Repoint GatewayClient at Unix socket + /api/v1

**Files:** `crates/cast_agent/src/config.rs`, `crates/cast_agent/src/gateway.rs`.

- [ ] **Step 1: Config default change**

In `config.rs`, change `gateway_url` default to `format!("unix://{}/.coven/coven.sock", env!("HOME"))` (or runtime-resolved). Continue accepting any string via env override. Add a `parse_gateway_scheme(&str) -> {Unix(PathBuf) | Http(String)}` helper.

Emit a `log::warn!` if the resolved scheme is HTTP, with text: "cast_agent: HTTP gateway URL is deprecated; default is unix:///.../coven.sock — see PRODUCT.md".

Add `packet_dir: PathBuf` config (default `~/.coven/proof-packets`).

- [ ] **Step 2: Gateway internal dispatch**

In `gateway.rs`, internalise an enum `Transport { Unix(UnixHttpClient), Tcp(reqwest::Client) }`. Build it from the config at `GatewayClient::new`. Replace every `self.http.get(...)` / `post(...)` / `delete(...)` with a transport-aware helper:

```rust
async fn get_json<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T>;
```

Path prefix changes from `/v1` to `/api/v1`. Update `health_probe` to use `/api/v1/health` (also try `/health` as fallback — the daemon supports both — for resilience).

- [ ] **Step 3: list_sessions returns adapted CovenSession**

`list_sessions` now: `GET /api/v1/sessions` → `Vec<DaemonSession>` → `Vec<CovenSession>` via the adapter. Same authoring style for new methods:

```rust
pub async fn get_session(&self, id: &str) -> anyhow::Result<CovenSession>;
pub async fn list_events(&self, session_id: &str, since: Option<u64>) -> anyhow::Result<Vec<DaemonEvent>>;
pub async fn list_familiars(&self) -> anyhow::Result<Vec<DaemonFamiliar>>;
```

- [ ] **Step 4: Gate legacy chat endpoints**

Wrap `send_message`, `stream_messages`, `open_session`, `close_session` (and their tests) in `#[cfg(feature = "legacy-gateway-chat")]`. Add to `Cargo.toml`:
```toml
[features]
default = []
legacy-gateway-chat = []
```
These remain compilable on `cargo test --features legacy-gateway-chat` but are unreachable from the default build.

- [ ] **Step 5: Run + commit**

```bash
cargo check -p cast_agent
cargo test -p cast_agent
cargo test -p cast_agent --features legacy-gateway-chat
git add -p crates/cast_agent
git commit -S -m "feat(cast_agent): switch gateway transport to Unix socket + /api/v1"
git log -1 --show-signature
```

### Task 1.6: Smoke-check end-to-end against the running daemon

**Files:** none (integration check).

- [ ] **Step 1: Launch a smoke binary**

```bash
cargo run -p cast_agent --example list_sessions   # add a small example file if not present
```
(If the cast_agent crate has no examples dir, write a tiny `examples/list_sessions.rs` that boots `GatewayClient`, calls `list_sessions()`, prints results. Add as part of this task.)

Expected: prints the same sessions visible from `curl --unix-socket ~/.coven/coven.sock http://localhost/api/v1/sessions`. Confirms the wire is real before moving on.

---

## Phase 2 — Lane model + state machine driver

### Task 2.1: Lane state + transitions

**Files:** `crates/cast_agent/src/lane.rs`, `crates/cast_agent/src/lib.rs`, `crates/cast_agent/tests/lane_state.rs`.

- [ ] **Step 1: Define types**

```rust
pub enum LaneState { Proposed, Launching, Running, Reviewing, Verifying, Merged, PrOpen, Archived, Failed }
pub enum LaneEvent { Launch, Spawned, Halt, StartVerify, VerifyPass, VerifyFail, Merge, OpenPr, Archive, MarkFailed, Redo }
pub struct Lane {
    pub id: Uuid,
    pub session_id: Option<String>,        // None until launching → spawned
    pub project_root: PathBuf,
    pub harness: String,
    pub prompt: String,
    pub worktree_path: Option<PathBuf>,
    pub state: LaneState,
    pub started_at: DateTime<Utc>,
    pub launched_at: Option<DateTime<Utc>>,
    pub halted_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub ritual: Option<String>,
    pub ritual_extras: serde_json::Value,
    pub packet_draft: PacketDraft,
}
```

- [ ] **Step 2: Implement transitions**

`impl Lane { pub fn apply(&mut self, event: LaneEvent) -> Result<(), TransitionError>; }` — enumerate every legal transition from PRODUCT.md. Reject illegal transitions explicitly.

- [ ] **Step 3: Unit-test every transition**

In `tests/lane_state.rs`, for each row in PRODUCT.md's transitions table, write one accept-test and (for each disallowed pair) one reject-test. Property-style: the test enumerates `(state, event)` pairs and asserts the legal/illegal set matches the spec.

- [ ] **Step 4: Commit**

### Task 2.2: LaneStateMachine driver (cast_agent runtime hookup)

**Files:** `crates/cast_agent/src/lane.rs`, `crates/cast_agent/src/runtime.rs`.

- [ ] **Step 1: Driver model**

```rust
pub struct LaneStateMachine { lanes: RwLock<HashMap<Uuid, Lane>> }
impl LaneStateMachine {
    pub fn create_proposed(&self, params: NewLaneParams) -> Uuid;
    pub fn launch(&self, id: Uuid, session_id: String, worktree: PathBuf) -> Result<()>;
    pub fn observe_daemon_status(&self, session_id: &str, status: DaemonSessionStatus);
    pub fn snapshot(&self) -> Vec<Lane>;     // for UI
    pub fn apply(&self, id: Uuid, event: LaneEvent) -> Result<()>;
}
```
`observe_daemon_status` translates daemon status → `LaneEvent::Halt` for completed/killed/orphaned/idle, no-op for running.

- [ ] **Step 2: Wire into runtime**

In `CastAgentRuntime`, hold an `Arc<LaneStateMachine>`. Add a periodic 5-second loop that calls `list_sessions()` and, for each known lane with a `session_id`, invokes `observe_daemon_status`.

- [ ] **Step 3: Sync accessor for the UI**

`CastAgentRuntime::lanes_snapshot() -> Vec<Lane>` reads via `RwLock::read`. UI consumes this synchronously on every frame.

- [ ] **Step 4: Commit**

---

## Phase 3 — Coven Panel UI: viewer + Start Coding launcher

### Task 3.1: Split panel.rs → coven_panel.rs

**Files:** `app/src/ai_assistant/panel.rs`, `app/src/ai_assistant/coven_panel.rs`, `app/src/ai_assistant/mod.rs`.

- [ ] **Step 1: Extract current `render_sessions_section`**

Move the body into `coven_panel::render_sessions` verbatim. Replace call site in `panel.rs` with `coven_panel::render_sessions(ctx, cast_agent)`. Compile cleanly with no behavior change.

- [ ] **Step 2: Commit (refactor-only intermediate)**

### Task 3.2: Lane row + state badge

**Files:** `app/src/ai_assistant/coven_panel.rs`.

- [ ] **Step 1: Replace passive session row with lane row**

For every lane returned by `cast_agent.lanes_snapshot()`, render a row with:
- Harness emoji (from familiars lookup)
- Truncated prompt or session title
- State badge (small pill, color per state — green for `running`, amber for `reviewing`/`verifying`, neutral for terminal states)
- Tail of recent event count + last-output snippet (if running) — read via `list_events` with `since=last_seen_seq`
- Click target: open the session's worktree as the focused tab (existing `OpenCovenSessionInNewTab` action, with `cwd` derived from `worktree_path`)

Sessions that have no associated lane (e.g., spawned outside CastCodes) still render as before — a passive row with name + status dot, no review actions.

- [ ] **Step 2: View-model test**

`coven_panel_tests.rs`: assert state-badge color for each `LaneState`, assert empty-prompt fallback, assert harness emoji resolution.

- [ ] **Step 3: Commit**

### Task 3.3: Start Coding form

**Files:** `app/src/ai_assistant/coven_panel.rs`, `app/src/workspace/view.rs`.

- [ ] **Step 1: Form rendering**

A collapsible section above the lane list titled "Start Coding". Fields:
- `project_root`: defaults to current workspace, read-only chip with click-to-change (opens a small picker over open workspaces).
- `harness`: dropdown sourced from a **hardcoded** list of backend harnesses for v1: `["codex", "claude"]`. Do **not** source from `cast_agent.list_familiars()` — familiars are a separable persona catalog (see PRODUCT.md "Note on familiars vs. harnesses"). The hardcoded list lives in `crates/cast_agent/src/lane.rs` as `pub const SUPPORTED_HARNESSES: &[&str] = &["codex", "claude"];` so future PLANs can extend it in one place.
- `prompt`: multi-line text input.
- `worktree`: checkbox, default checked.
- Submit button: "Launch lane".

- [ ] **Step 2: Form submit handler**

On submit:
1. Call `cast_agent.lanes().create_proposed(NewLaneParams { ritual: Some("start-coding"), ... })` → returns `lane_id`.
2. Dispatch `WorkspaceAction::SpawnCovenLane { lane_id, harness, prompt, project_root, use_worktree }`.

- [ ] **Step 3: Implement `SpawnCovenLane`**

In `workspace/view.rs`, the action handler:
1. If `use_worktree`, create one via the worktree manager primitive (delegate; do not re-implement).
2. Open a new terminal tab inside `worktree_path` (or `project_root`), tab title `Coven: <prompt-snippet truncated to 40 chars>`.
3. Run the harness CLI in the tab. **The exact `coven` CLI invocation MUST be verified against `coven --help` before this step is implemented** — the running daemon is `@opencoven/cli` v0.0.29 and its CLI surface should be confirmed, not guessed. Likely candidates (in order of probability based on observed session shapes): `coven session start --project "<root>" --harness "<harness>" --prompt "<escaped prompt>"`, `coven run --harness <harness> --prompt "<prompt>"`, or `coven {harness} --prompt "<prompt>"`. The implementing agent runs `coven --help` and the relevant subcommand `--help` to pick the right invocation, then records it as the v1 command template at the top of `lane.rs`:
   ```rust
   pub fn spawn_command(harness: &str, project_root: &Path, prompt: &str) -> Vec<String> { /* verified template */ }
   ```
4. Asynchronously poll `cast_agent.list_sessions()` every 1 s for up to 30 s looking for a session whose `project_root == worktree_path && created_at >= dispatch_time`. On match, call `cast_agent.lanes().launch(lane_id, session_id, worktree_path)`. On timeout, call `cast_agent.lanes().apply(lane_id, LaneEvent::MarkFailed)` and surface a notification.

- [ ] **Step 4: Commit**

### Task 3.4: Review actions

**Files:** `app/src/ai_assistant/coven_panel.rs`.

- [ ] **Step 1: Per-lane review actions**

For lanes in `Reviewing` or `Verifying` states, render action buttons in the row:
- "Verify" (only if `Reviewing` → triggers Phase 4 verification)
- "Merge" / "Open PR" / "Archive" / "Failed"

Clicking dispatches the corresponding `LaneEvent` via `cast_agent.lanes().apply(...)`. For "Merge" and "Open PR", these are placeholders that just record the intent in the packet draft for v1 (running `git push` or `gh pr create` is a follow-up in TECH.md — out of scope for PLAN-01 keep-it-small).

- [ ] **Step 2: Packet draft form**

When a lane enters `Reviewing`, expose an inline form with four textareas (`worked`, `broke`, `should_become_issue`, `can_be_shown_publicly`) bound to `lane.packet_draft`. Edits persist in-memory; written to disk on terminal transition.

- [ ] **Step 3: Commit**

---

## Phase 4 — Verification + packet writer

### Task 4.1: Verification subprocess

**Files:** `crates/cast_agent/src/lane.rs` (extend), `app/src/ai_assistant/coven_panel.rs`.

- [ ] **Step 1: Resolve verify command**

Read `<project_root>/.castcodes/verify.toml` if present:
```toml
verify = ["cargo check"]
```
If not present, fall back to:
- `cargo check` if `Cargo.toml` is in `project_root`
- `npm test` if `package.json` is in `project_root`
- echo "No verify command configured" otherwise

- [ ] **Step 2: Run verify**

`Lane::run_verify(&self) -> tokio::process::Child` — spawn via `tokio::process::Command`, capture exit code into `lane.packet_draft.verification`. On exit, transition: `LaneEvent::VerifyPass` (exit 0) or `LaneEvent::VerifyFail` (non-zero).

- [ ] **Step 3: Commit**

### Task 4.2: Proof packet writer

**Files:** `crates/cast_agent/src/packet.rs`, `crates/cast_agent/tests/packet_roundtrip.rs`.

- [ ] **Step 1: Define `ProofPacket` struct**

Match the v1 schema in PRODUCT.md exactly. `#[derive(Debug, Clone, Serialize, Deserialize)]`. `serde(rename_all = "snake_case")` where needed to match the JSON shape.

- [ ] **Step 2: `write_packet`**

```rust
pub fn write_packet(packet: &ProofPacket, packet_dir: &Path) -> anyhow::Result<PathBuf>;
```
Atomic-write pattern: write to `<dir>/.<session-id>.json.tmp`, fsync, rename to `<dir>/<session-id>.json`. Return final path.

- [ ] **Step 3: Hook into terminal transitions**

In `Lane::apply`, when transitioning into `{Merged, PrOpen, Archived, Failed}`, compose a `ProofPacket` from the lane state + draft, call `write_packet`, log the path. If write fails, log error but allow the transition (packet recovery is a v2 problem).

- [ ] **Step 4: Roundtrip test**

`tests/packet_roundtrip.rs`: build a packet with non-trivial values in every field, write to temp dir, read back via `serde_json::from_str`, assert equality.

- [ ] **Step 5: Commit**

---

## Phase 5 — End-to-end smoke test (dogfood-on-cast-codes)

### Task 5.1: Build and launch CastCodes

**Files:** none.

- [ ] **Step 1: Release build**

```bash
cargo build -p warp-app --bin cast-codes --features gui --release
```
Should complete cleanly.

- [ ] **Step 2: Launch the built app**

```bash
./target/release/cast-codes
```
Or — if the user prefers the installed app — `open /Applications/CastCodes.app` (the installed app does NOT have this plan's changes; for the smoke test, use the cargo-built binary).

### Task 5.2: Run Start Coding against the cast-codes repo

**Files:** none (manual test).

- [ ] **Step 1: Open the cast-codes workspace**

In the launched CastCodes, open `/Users/buns/Documents/GitHub/OpenCoven/cast-codes` as the workspace.

- [ ] **Step 2: Verify the gateway pill is green**

Top of the agent panel should show the green Coven status pill. If amber, check `~/.coven/coven.sock` permissions and `coven daemon serve` is running.

- [ ] **Step 3: Expand "Coven Sessions" / "Coven Lanes" section**

You should see the live sessions list pulled from `/api/v1/sessions`. Confirm at least one familiar session appears (e.g., the prior "logs prune" session from PRODUCT.md examples).

- [ ] **Step 4: Use Start Coding**

Click "Start Coding". Form opens. Pick `codex` (or `claude` — either supported v1 harness) from the harness picker, prompt: `"Add a one-sentence module-level doc comment to crates/cast_agent/src/lane.rs explaining the lane state machine"`, leave worktree checked. Click "Launch lane".

- [ ] **Step 5: Watch the lane progress**

Expected sequence visible in the panel within ~60 s:
- Lane state badge: `proposed` → `launching` → `running`
- A new terminal tab opens titled `Coven: Add a one-sentence module...`
- A new session appears in the sessions list whose `project_root` is the worktree path
- Event count climbs as harness runs
- When harness halts, badge auto-transitions to `reviewing`

- [ ] **Step 6: Review + verify + terminal**

- Click "Verify" on the lane row. Expected: `cargo check` runs in subprocess; exit code captured.
- Open the packet draft form. Fill in: `worked = ["lane progressed through all states automatically"]`, `broke = []` (or whatever you actually observed), `should_become_issue = []`, `can_be_shown_publicly = ["first end-to-end Start Coding ritual succeeded with <chosen-harness> on cast-codes"]`.
- Click "Open PR" (or "Archive" — the action doesn't actually run `gh pr create` in v1, just records intent).

- [ ] **Step 7: Confirm packet on disk**

```bash
ls -la ~/.coven/proof-packets/
cat ~/.coven/proof-packets/<session-id>.json | jq .
```
Confirm a file exists with `packet_version: 1`, the correct `session_id`, `harness: "cody"`, `ritual: "start-coding"`, `terminal_state` matches the action you clicked, and your `worked`/`broke`/`can_be_shown_publicly` content is preserved.

- [ ] **Step 8: Record the smoke result**

If anything in Steps 5–7 fails, document in `~/.coven/proof-packets/<session-id>.json` under `broke[]` and STOP — file an issue in this spec's `CHECKLIST.md` (create the file if needed), and surface to the user before proceeding.

If everything passes, this is the **first real Coven/CastCodes task end-to-end from CastCodes** — acceptance criterion #1 of the active goal is met. The packet itself is acceptance criterion #3's first data point. Acceptance criterion #2 is partially met: the Start Coding ritual exists as a CastCodes command; PLAN-02 through PLAN-06 cover the remaining five.

---

## Final verification

- [ ] `cargo check -p cast_agent` clean
- [ ] `cargo check -p warp-app --bin cast-codes --features gui` clean
- [ ] `cargo test -p cast_agent` passes
- [ ] `cargo test -p cast_agent --features legacy-gateway-chat` passes (legacy chat code remains compilable)
- [ ] `./script/check_rebrand` passes (no rebrand surface regressions)
- [ ] The smoke proof packet at `~/.coven/proof-packets/<session-id>.json` exists and is well-formed JSON
- [ ] All commits on `cast/coven-internal-loop` are signed (`git log origin/main..HEAD --pretty='%H %G?' | awk '$2 != "G" {print "UNSIGNED:", $0}'` prints nothing)
- [ ] No commit on this branch contains AI-attribution markers (`git log origin/main..HEAD --grep="Co-Authored-By.*[Aa][Ii]\|Generated with" --oneline` prints nothing)
- [ ] CAST-AGENT.md mentions the transport switch
- [ ] DESIGN-CHANGES.md has the new entry

---

## Follow-up PLANs (out of scope here, scoped for context)

- `PLAN-02-review-stack.md` — Review Stack ritual: N-lane creation, shared `ritual_extras.review_stack_group_id`.
- `PLAN-03-release-check.md` — Release Check: rich verification gate, target-version handling, release-notes draft.
- `PLAN-04-fix-openclaw.md` — Fix OpenClaw: per-ritual `.castcodes/rituals.toml`, cross-repo lane launch.
- `PLAN-05-coven-dogfood-quest.md` — Dogfood Quest: pinned Coven-repo project_root, quest text storage.
- `PLAN-06-multi-harness-review.md` — Multi-Harness Review: N parallel lanes, group-id, side-by-side diff UI.
- `PLAN-07-weekly-aggregator.md` — Weekly update aggregator CLI/UI: globs packets, emits Markdown digest.
- `TECH.md` — only authored when one of the above warrants it (e.g., Multi-Harness Review's side-by-side diff UI). PLAN-01 is intentionally TECH.md-free per the spec-driven-implementation skill ("write TECH.md when warranted").
