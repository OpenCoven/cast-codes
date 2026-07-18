# Retire the Familiar Agent Panel — Revised 2d Task 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the `ai_assistant` "Familiar" agent panel (`AIAssistantPanelView`) and its dead hosted-AI backend, now that the unified `agent_panel` owns agent conversations. Keep everything else in `ai_assistant/` that the rest of the app depends on.

**Why this supersedes PLAN-2d Task 4:** the original Task 4 assumed the coven-stream was a removable *section* of a still-functional Familiar panel. It isn't. Under `#[cfg(feature = "cast-agent")]`, `AIAssistantPanelView::issue_primary_request` (`panel.rs:872-898`) routes **every** prompt through `send_via_coven_gateway` — the coven-stream *is* the panel's agent path. The non-coven parts (`requests_model`, `transcript_view`) are the old Warp-hosted path, dead in this fork (`requests.rs:1`: *"TODO: Delete all of this once agent mode fully replaces the AI assistant panel"*). So this is a **whole-panel retirement**, not a section trim.

**Architecture:** Delete `panel.rs` + the dead `requests.rs`/`transcript.rs` + the orphaned `coven_stream_persist.rs`, and unwire the panel from the workspace (field, construct, toggle, handler, render, state, action, keybinding, tests). **Preserve** the shared `ai_assistant` surface that the wider app uses (`AskAIType`, `execution_context::WarpAiExecutionContext`, the `AI_ASSISTANT_*` constants, `coven_entry::daemon_event_to_entry`).

**Tech Stack:** Rust, warpui, `warp_features::FeatureFlag`, the `cast-agent` + `unified_agent_panel` Cargo features.

---

## ⛔ Prerequisites & the gating decision (read first)

This plan must **not** start until all of these hold:

1. **#214 (cli_chat panel retirement) is merged.** This plan is the second half of the deprecation.
2. **`PARITY-2d.md` is dogfooded and ticked** — the unified panel must demonstrably cover the daemon flow the Familiar panel provided.
3. **The default-surface decision is made (load-bearing).** The Familiar panel's toggle is gated `.with_enabled(|| !FeatureFlag::AgentMode.is_enabled())` (`workspace/mod.rs:1247`). Retiring it removes the agent surface for users who have **neither** Agent Mode **nor** `unified_agent_panel` enabled. So one of these must be true *before* this lands:
   - **(Recommended)** `unified_agent_panel` is promoted to a **default-on** feature (flip 2d stage-1's opt-in feature into the default/preview set), so every build that had the Familiar panel now gets the unified panel; **or**
   - the Familiar retirement is itself gated so the panel only disappears where the unified panel (or Agent Mode) is present.

   **Do not delete the panel while the default build would be left with no agent surface.** Resolve this with the maintainer and record the choice at the top of the retirement PR.

If any prerequisite is unmet, stop and surface it.

---

## File Structure

| File | Action | Notes |
|---|---|---|
| `app/src/ai_assistant/panel.rs` | **Delete** | The whole `AIAssistantPanelView` (~2300 lines) |
| `app/src/ai_assistant/requests.rs` | **Delete** | Dead hosted-AI backend (verify no external users) |
| `app/src/ai_assistant/transcript.rs` | **Delete** | Renders `requests_model`; dead in cast-agent (verify) |
| `app/src/ai_assistant/coven_stream_persist.rs` | **Delete** | Orphaned once `panel.rs` goes (no external callers) |
| `app/src/ai_assistant/mod.rs` | **Modify** | Drop `panel`/`requests`/`transcript`/`coven_stream_persist` mods + panel-only constants; **keep** `AskAIType`, `execution_context`, `coven_entry`, `utils`(verify), and app-wide constants |
| `app/src/ai_assistant/coven_entry.rs` | **Keep** | `daemon_event_to_entry` — used by `agent_panel::daemon_turn` |
| `app/src/ai_assistant/execution_context.rs` | **Keep** | `WarpAiExecutionContext` used across terminal/ai/pane_group |
| `app/src/workspace/view.rs` | **Modify** | Remove field (`:969`), `build_ai_assistant_panel_view` (`:1523`), `handle_ai_assistant_panel_event` (`:1528`), `toggle_ai_assistant_panel` (`:4214`), render branch (`:19247`), `is_ai_assistant_panel_open` state, `ASK_AI_ASSISTANT_KEYBINDING_NAME` (`:569`) |
| `app/src/workspace/action.rs` | **Modify** | Remove `ToggleAIAssistant` variant (`:228`) + its exhaustiveness arm (`:882`) |
| `app/src/workspace/mod.rs` | **Modify** | Remove the `!AgentMode` Familiar keybinding (`:1242-1253`); keep the `AgentMode` `NewPaneInAgentMode` binding on the same ID |
| `app/src/integration_testing/view_getters.rs` | **Modify** | Remove `ai_assistant_panel_view()` fixture (`:11,:217`) |
| `app/src/workspace/view_tests.rs` | **Modify** | Remove/refactor Familiar focus tests (`:1175-1203`) |
| `app/src/lib.rs` | **Modify** | Remove `ai_assistant::panel` re-export if present |

**Reference (do not delete): `AskAIType`, `AI_ASSISTANT_FEATURE_NAME`, `AI_ASSISTANT_LOGO_COLOR`, `ASK_AI_ASSISTANT_TEXT` are consumed by `terminal/view.rs`, `pane_group/mod.rs`, `workspace/view.rs`; `execution_context::WarpAiExecutionContext` by `ai/agent`, `ai/predict`, `terminal`, `active_session`.**

---

## Task 1: Confirm the dead/shared boundary (no code changes)

**Files:** none — this is a verification gate that pins the delete-vs-keep set for the compiler-driven tasks.

- [ ] **Step 1: Confirm `requests.rs` / `transcript.rs` have no external users.**

Run:
```bash
grep -rn "ai_assistant::requests\|ai_assistant::transcript\|Requests\b\|Transcript::new" app/src \
  | grep -v "app/src/ai_assistant/"
```
Expected: only `panel.rs`-internal usage (which is being deleted). If anything outside `ai_assistant/` uses them, note it — those call sites must be handled before deletion.

- [ ] **Step 2: Confirm `coven_stream_persist` is orphaned.**

Run:
```bash
grep -rn "coven_stream_persist" app/src | grep -v "coven_stream_persist.rs\|panel.rs\|mod.rs"
```
Expected: **none** (only `panel.rs` calls `save`/`load`). `CovenStreamHistoryEntry` lives in `panel.rs`; the 2a migration (`cli_chat/history_migration.rs`) has its **own** reader (`HistoryRecord`) and only references the wire format in a comment — so deleting `coven_stream_persist` does not break the migration. If a non-`panel.rs` caller exists, keep `coven_stream_persist` (moving `CovenStreamHistoryEntry` into it) instead of deleting.

- [ ] **Step 3: Confirm the keep-set is used app-wide.**

Run:
```bash
grep -rn "AskAIType\|WarpAiExecutionContext\|AI_ASSISTANT_FEATURE_NAME" app/src | grep -v "app/src/ai_assistant/" | wc -l
```
Expected: many (terminal, pane_group, ai/*). These stay in `ai_assistant/mod.rs` + `execution_context.rs`.

- [ ] **Step 4: Record the delete-set vs keep-set** in the PR description from the results above. No commit.

---

## Task 2: Remove the workspace wiring (compiler-driven)

**Files:** `app/src/workspace/{view.rs, action.rs, mod.rs}`, `app/src/lib.rs`

Do the *wiring* removal first so the panel becomes unreferenced before the files are deleted — the compiler then confirms `panel.rs` has no remaining consumers.

- [ ] **Step 1: Remove the action + keybinding.** In `workspace/action.rs` delete the `ToggleAIAssistant` variant (`:228`) and its arm in the "needs save" match (`:882-883`). In `workspace/mod.rs` delete the Familiar `EditableBinding` gated on `!AgentMode` (`:1242-1253`) — **keep** the sibling `NewPaneInAgentMode` binding on the same `workspace:toggle_ai_assistant` id (it's Agent Mode's, gated on `AgentMode.is_enabled()`).

- [ ] **Step 2: Remove the panel field, construction, event handler, toggle, render, state.** In `workspace/view.rs` remove: the `ai_assistant_panel` field (`:969`), the struct-literal init, `build_ai_assistant_panel_view()` (`:1523-1525`) + its `subscribe_to_view` (`:1527-1529`), `handle_ai_assistant_panel_event()` (`:1528`), `toggle_ai_assistant_panel()` (`:4214-4257`), the `is_ai_assistant_panel_open` render branch (`:19247-19252`) and the state field, and `ASK_AI_ASSISTANT_KEYBINDING_NAME` (`:569`) if now unused. Keep the `use crate::ai_assistant::{AskAIType, AI_ASSISTANT_FEATURE_NAME, ...}` imports that other workspace code still uses (the compiler says which).

- [ ] **Step 3: Remove any `pub use ai_assistant::panel;`** from `lib.rs` (grep to confirm).

- [ ] **Step 4: Build.**

Run: `cargo check -p warp-app --bin cast-codes --features gui,cast-agent`
Expected: fails only with unresolved `AIAssistantPanelView`/`ToggleAIAssistant`/`ai_assistant::panel` references — fix each until it reports only that `panel`/`requests`/`transcript`/`coven_stream_persist` modules are now unused (Task 3 deletes them). Iterate until the only remaining errors are the module deletions.

- [ ] **Step 5: Commit.**

```bash
git add -A
git commit -S -m "refactor(ai_assistant): unwire the Familiar panel from the workspace"
```

---

## Task 3: Delete the panel + dead backend + orphaned persist

**Files:** delete `app/src/ai_assistant/{panel.rs, requests.rs, transcript.rs, coven_stream_persist.rs}`; modify `app/src/ai_assistant/mod.rs`

- [ ] **Step 1: Delete the files.**

```bash
git rm app/src/ai_assistant/panel.rs \
       app/src/ai_assistant/requests.rs \
       app/src/ai_assistant/transcript.rs \
       app/src/ai_assistant/coven_stream_persist.rs
```
(If Task 1 Step 2 found an external `coven_stream_persist` user, keep that file — move `CovenStreamHistoryEntry` into it from `panel.rs` — and drop it from this `git rm`.)

- [ ] **Step 2: Update `ai_assistant/mod.rs`.** Remove the `mod`/`pub mod` lines for `panel`, `requests`, `transcript`, `coven_stream_persist`, and any panel-only constants (`AI_ASSISTANT_SVG_PATH`, `AI_ASSISTANT_LOGO_COLOR`, `ASK_AI_ASSISTANT_TEXT`, `PROMPT_CHARACTER_LIMIT`) **only if** a repo-wide grep shows no non-deleted user. **Keep** `coven_entry` (cast-agent), `execution_context`, `utils` (verify), `AskAIType`, and `AI_ASSISTANT_FEATURE_NAME` (still used by `terminal`/`pane_group`/`workspace`).

- [ ] **Step 3: Verify `utils` + `AI_ASSISTANT_*` keep/drop.** For each symbol you consider dropping:
```bash
grep -rn "<symbol>" app/src | grep -v "app/src/ai_assistant/"
```
Keep anything with an external user; delete only the truly panel-local ones.

- [ ] **Step 4: Build.**

Run: `cargo check -p warp-app --bin cast-codes --features gui,cast-agent`
Expected: PASS. Fix any straggler `mod.rs` re-export or dead constant the compiler flags.

- [ ] **Step 5: Commit.**

```bash
git add -A
git commit -S -m "refactor(ai_assistant): delete the Familiar panel + dead hosted-AI backend"
```

---

## Task 4: Fix tests + integration fixtures

**Files:** `app/src/integration_testing/view_getters.rs`, `app/src/workspace/view_tests.rs`

- [ ] **Step 1: Remove the panel test fixture.** Delete `ai_assistant_panel_view()` (`view_getters.rs:11,:217`) and any imports it needed.

- [ ] **Step 2: Remove/refactor the Familiar focus tests.** In `workspace/view_tests.rs` delete the assertions referencing `ai_assistant_panel` (`:1175-1203`). If a test's *intent* (panel focus behavior) still matters for the unified panel, port it to `agent_panel`; otherwise delete it and note the removal in the PR.

- [ ] **Step 3: Run the touched suites.**

```bash
cargo nextest run -p warp-app --features cast-agent -E 'test(workspace) or test(ai_assistant) or test(agent_panel)'
```
Expected: PASS (no references to the deleted panel).

- [ ] **Step 4: Commit.**

```bash
git add -A
git commit -S -m "test: drop Familiar-panel fixtures + focus tests"
```

---

## Task 5: Full gates (both flag configs)

- [ ] **Step 1: Guards.**

```bash
./script/check_cli_chat_boundary
./script/check_ai_attribution
./script/check_rebrand
```

- [ ] **Step 2: Lint + fmt in both configs.**

```bash
cargo fmt -p warp-app
cargo clippy -p warp-app --features cast-agent --all-targets -- -D warnings
cargo clippy -p warp-app --features cast-agent,unified_agent_panel --all-targets -- -D warnings
```
Expected: clean in both — no dead code left behind by the deletion.

- [ ] **Step 3: Full regression.**

```bash
cargo nextest run -p warp-app --features cast-agent,unified_agent_panel
```
Expected: PASS.

- [ ] **Step 4: Commit fmt-only changes.**

```bash
git add -A
git commit -S -m "chore(ai_assistant): Familiar-retirement gates + fmt"
```

---

## Done criteria

- `AIAssistantPanelView` and its dead hosted-AI backend (`requests.rs`, `transcript.rs`) and orphaned `coven_stream_persist.rs` are deleted; the panel is fully unwired from the workspace (field/construct/toggle/handler/render/state/action/keybinding/tests).
- The shared `ai_assistant` surface the rest of the app uses (`AskAIType`, `execution_context::WarpAiExecutionContext`, `coven_entry::daemon_event_to_entry`, `AI_ASSISTANT_FEATURE_NAME`) is preserved and still builds.
- The default-surface decision (prerequisite 3) is resolved and recorded — no build is left without an agent surface.
- Both flag configs clippy `-D warnings` clean; guards pass; full suite passes; every commit signed.
- Agent Mode is unaffected (separate system; its `NewPaneInAgentMode` binding on the shared id remains).

---

## Self-Review

**Spec coverage:** whole-panel retirement (Tasks 2-3) ✓; dead-backend cleanup `requests`/`transcript` (Task 3) ✓; orphaned `coven_stream_persist` (Task 1 §2 + Task 3) ✓; preserve app-wide `ai_assistant` surface (Task 1 §3, Task 3 §2-3) ✓; tests/fixtures (Task 4) ✓; the **gating decision** that the original plan missed (prerequisite 3) ✓; Agent Mode non-impact (Done criteria) ✓.

**Placeholder scan:** the delete/keep boundary is pinned by grep gates (Task 1) rather than assumed; `<symbol>`/`<external user>` in Task 3 §2-3 are explicit per-symbol verification steps, not vague TODOs.

**Consistency:** `coven_entry` + `execution_context` + `AskAIType` are in the keep-set everywhere they appear; `panel`/`requests`/`transcript`/`coven_stream_persist` in the delete-set everywhere. The compiler-driven ordering (unwire → delete) means every task ends in a build that proves the boundary.

**Risks flagged:** (1) **prerequisite 3 is the real risk** — retiring the Familiar panel without defaulting the unified panel strands non-AgentMode users; the plan blocks on resolving it. (2) `coven_stream_persist`/`CovenStreamHistoryEntry` ordering — verified orphaned, but Task 1 §2 keeps the fallback (move the struct, keep the file) if a caller is found. (3) Panel-local constants in `mod.rs` — Task 3 §3 greps each before dropping, since several `AI_ASSISTANT_*`/`AskAIType` names are used far outside the panel.
