# Unified Agent Panel — Phase 2d Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the unified agent panel the single agent entry point and retire the two surfaces it replaces — the standalone `cli_chat::ChatPanelView` (deleted) and the `ai_assistant` panel's coven-stream section (removed) — *staged*: flip the default first, delete only after parity is confirmed.

**Architecture:** Two reversible steps then two deletions. (1) Default-enable `FeatureFlag::UnifiedAgentPanel` and gate the old daemon surface off when it is on, so exactly one agent surface shows. (2) After a parity check, delete `cli_chat::view/*` and its panel wiring (keeping the whole `cli_chat` *model* layer, which `agent_panel` depends on), and remove the coven-stream rendering from `ai_assistant::panel.rs` (keeping `coven_entry.rs` + `coven_stream_persist.rs`, which `agent_panel` reuses). The `ai_assistant` "Familiar" panel itself stays — it has non-coven functions (editor, requests, input suggestions); only its Coven section goes.

**Tech Stack:** Rust, `warp_features::FeatureFlag`, Cargo feature gating (`enabled_features()` in `app/src/lib.rs`), warpui.

**Depends on:** 2a (#205), 2b (#206), 2c (#207) — the unified panel must be fully built and at parity before this deletes anything. Rebase on top of 2c.

**Key scoping facts (from code survey — cite in review):**
- **Deletable (view/wiring only):** `app/src/cli_chat/view/*` (11 files incl. `ChatPanelView`); `WorkspaceAction::ToggleCliChatPanel` (`workspace/action.rs:264`) + its handler (`workspace/view.rs:20741-20761`) + binding (`workspace/mod.rs:743-748`) + `CustomAction::ToggleCliChatPanel` (`util/bindings.rs:139,440-442`) + menu (`app_menus.rs:404-410`) + field `cli_chat_panel_view` (`workspace/view.rs:1029`) + state `is_cli_chat_panel_open` (`workspace/util.rs:125`); `WorkspaceAction::SubmitChatPrompt` (superseded by 2b's `SubmitAgentPrompt`).
- **MUST KEEP:** the entire `cli_chat` model layer (`model`, `store`, `store_schema`, `conversation`, `entry`, `history_migration`, `strings`, `paths`, `feature_flag`) — `agent_panel` depends on it; `WorkspaceAction::{OpenChatSession, CliChatNewChat}` — reused by `agent_panel` (list-row open + header new-chat); `ai_assistant::coven_entry::daemon_event_to_entry` + `coven_stream_persist` — reused by 2c; the `AIAssistantPanelView` struct + its non-coven areas.
- **Removed from `ai_assistant::panel.rs` (staged):** `coven_stream` field (`:254`), `CovenStreamState` (`:258-294`), `append_entry_coalescing` (`:308-325`), `buffer_coven_agent_event` (`:329-340`), the stream consumer (`:1098-1227`), `poll_coven_stream` (`:1427-1461`), `render_coven_stream_section` (`:1470-1607`), the coven branch of `render_gateway_status_pill` (`:1614-1663`), and the call site (`:2226`).
- **Flag default mechanism:** all flags init `false` (`warp_features/src/lib.rs:858`); `init_feature_flags()` (`app/src/lib.rs:2458`) enables everything `enabled_features()` (`:2466-2750`) returns, which is driven by compile-time `#[cfg(feature = "...")]`. Default-enabling a flag = add a Cargo feature + a `#[cfg(feature = "...")]` push in `enabled_features()`.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `crates/warp_features/src/lib.rs` | (`UnifiedAgentPanel` already added in 2b) | — |
| `app/Cargo.toml` | Add `unified_agent_panel` Cargo feature (in default set for dogfood channel) | Modify |
| `app/src/lib.rs` | Push `UnifiedAgentPanel` from `enabled_features()` under the feature | Modify (`:2466-2750`) |
| `app/src/ai_assistant/panel.rs` | Gate coven-stream render off when `UnifiedAgentPanel` on (Step A), then delete it (Step B) | Modify |
| `app/src/cli_chat/view/` | Delete the whole directory | Delete |
| `app/src/cli_chat/mod.rs` | Remove `pub mod view;` + `pub use view::ChatPanelView;` | Modify (`:34,36`) |
| `app/src/workspace/view.rs` | Remove `ChatPanelView` field/construct/render/import + `ToggleCliChatPanel` handler; keep `OpenChatSession`/`CliChatNewChat` | Modify |
| `app/src/workspace/util.rs` | Remove `is_cli_chat_panel_open` | Modify (`:125`) |
| `app/src/workspace/action.rs` | Remove `ToggleCliChatPanel` + `SubmitChatPrompt`; keep `OpenChatSession`/`CliChatNewChat` | Modify |
| `app/src/workspace/mod.rs` | Remove the cli_chat toggle binding | Modify (`:743-748`) |
| `app/src/util/bindings.rs` | Remove `CustomAction::ToggleCliChatPanel` + keystroke | Modify (`:139,440`) |
| `app/src/app_menus.rs` | Remove the cli_chat menu item | Modify (`:404-410`) |
| `specs/castcodes-unified-agent-panel/PARITY-2d.md` | Parity checklist artifact | Create |

---

## Task 1: Parity checklist — gate before any deletion

**Files:** Create `specs/castcodes-unified-agent-panel/PARITY-2d.md`

Deletion is irreversible for reviewers' trust; confirm the unified panel matches both old surfaces first. This task produces a checked artifact; do not proceed to Tasks 3–4 until every box is ticked (manually, with `./script/run`).

- [ ] **Step 1: Write the parity checklist.** `specs/castcodes-unified-agent-panel/PARITY-2d.md`:

```markdown
# Unified Agent Panel — Parity Checklist (gate for 2d deletion)

## From cli_chat::ChatPanelView
- [ ] Conversation list, newest-first, both backends, with backend badge
- [ ] Selecting a conversation binds it and shows its transcript
- [ ] Composer sends to a live CLI conversation (PTY) — text reaches the agent
- [ ] "New chat" starts a CLI agent (CliChatNewChat)
- [ ] Model/agent label in the header
- [ ] Transcript renders all ChatEntryKind variants via agent_transcript

## From ai_assistant coven-stream section
- [ ] Daemon (coven-code) conversations appear in the list
- [ ] Composer sends to a daemon conversation and streams the reply (2c)
- [ ] Gateway/daemon availability is reflected (status affordance)
- [ ] Historical stream-history.json conversations are present (2a migration)

## Non-regressions
- [ ] The Familiar panel (editor/requests/suggestions) still works after the
      coven-stream section is removed
- [ ] cli_chat model/store tests still pass; no daemon history is lost
```

- [ ] **Step 2: Dogfood behind the flag.** Build with the unified panel enabled and exercise each row: `cargo run` / `./script/run` with `UnifiedAgentPanel` on (Task 2 makes it default; before that, enable via the flag override). Tick each box; attach screenshots to the PR.

- [ ] **Step 3: Commit the checklist.**

```bash
git add specs/castcodes-unified-agent-panel/PARITY-2d.md
git commit -S -m "docs(agent_panel): 2d parity checklist"
```

---

## Task 2: Make the unified panel the default entry point (reversible)

**Files:** `app/Cargo.toml`, `app/src/lib.rs`, `app/src/ai_assistant/panel.rs`

Flip the default so the unified panel is the agent surface, and gate the old daemon surface off when it is on — no deletion yet, so this is fully reversible by toggling the flag.

- [ ] **Step 1: Add the Cargo feature.** In `app/Cargo.toml` `[features]`, add:

```toml
unified_agent_panel = []
```

  Include it in whatever feature set the dogfood/preview channel builds (grep how `cast-agent` is included; mirror it). Do **not** add it to the minimal/wasm sets.

- [ ] **Step 2: Enable the flag by default.** In `app/src/lib.rs` `enabled_features()` (near the other `#[cfg(feature = "...")]` pushes, `:2475-2750`):

```rust
    #[cfg(feature = "unified_agent_panel")]
    flags.push(FeatureFlag::UnifiedAgentPanel);
```

  (Match the exact push idiom used for neighboring flags — `flags.push(..)` vs a builder.)

- [ ] **Step 3: One daemon surface at a time.** In `app/src/ai_assistant/panel.rs`, at the coven-stream render call site (`:2226`), gate it off when the unified panel is enabled:

```rust
    #[cfg(feature = "cast-agent")]
    if !warp_core::features::FeatureFlag::UnifiedAgentPanel.is_enabled() {
        panel = panel.add_child(self.render_coven_stream_section(appearance));
    }
```

  This keeps the Familiar panel's coven section for non-unified builds and removes the double surface when unified is on. (The cli_chat panel is already flag-gated off by default via `CastCodesChatPanel`, so no change needed there for de-duplication.)

- [ ] **Step 4: Build both configurations.**

```bash
cargo check -p warp-app --bin cast-codes --features gui,cast-agent
cargo check -p warp-app --bin cast-codes --features gui,cast-agent,unified_agent_panel
```
Expected: both PASS.

- [ ] **Step 5: Commit.**

```bash
git add app/Cargo.toml app/src/lib.rs app/src/ai_assistant/panel.rs
git commit -S -m "feat(agent_panel): default-enable unified panel; single daemon surface"
```

**⛔ GATE: do not start Tasks 3–4 until Task 1's checklist is fully ticked.**

---

## Task 3: Delete `cli_chat::ChatPanelView` + its panel wiring

**Files:** `app/src/cli_chat/view/` (delete), `app/src/cli_chat/mod.rs`, `app/src/workspace/{view,util,action,mod}.rs`, `app/src/util/bindings.rs`, `app/src/app_menus.rs`

Delete the old CLI panel view and everything that only existed to host it. **Keep** the `cli_chat` model layer and the shared actions `OpenChatSession` / `CliChatNewChat`.

- [ ] **Step 1: Delete the view directory.**

```bash
git rm -r app/src/cli_chat/view
```

- [ ] **Step 2: Drop the module + re-export.** In `app/src/cli_chat/mod.rs`, remove `pub mod view;` (`:34`) and `pub use view::ChatPanelView;` (`:36`).

- [ ] **Step 3: Unwire the workspace panel.** In `app/src/workspace/view.rs`, remove: the `use crate::cli_chat::ChatPanelView` import (`:20`), the `cli_chat_panel_view` field (`:1029`) + its construction (`:1031`), the render block (`:1063`), and the `ToggleCliChatPanel` handler arm (`:20741-20761`). **Leave** the `OpenChatSession` (`:20766`), `SubmitChatPrompt`→ (see Step 6), and `CliChatNewChat` (`:20811`) handlers intact.

- [ ] **Step 4: Remove workspace state + action.** Delete `is_cli_chat_panel_open` (`app/src/workspace/util.rs:125`) and `WorkspaceAction::ToggleCliChatPanel` (`app/src/workspace/action.rs:264`). Remove the binding (`app/src/workspace/mod.rs:743-748`), `CustomAction::ToggleCliChatPanel` + its keystroke arm (`app/src/util/bindings.rs:139,440-442`), and the menu item (`app/src/app_menus.rs:404-410`).

- [ ] **Step 5: `TOGGLE_MENU_ITEM` string.** `cli_chat::strings::TOGGLE_MENU_ITEM` was used only by the removed binding — remove it if now unused (grep to confirm zero references), or leave `strings.rs` intact if `agent_panel` borrows from it.

- [ ] **Step 6: Remove `SubmitChatPrompt`.** 2b introduced `SubmitAgentPrompt` for the unified composer, so `WorkspaceAction::SubmitChatPrompt` + its handler are now dead. Grep for remaining references; if only the (now-deleted) cli_chat composer dispatched it, delete the variant (`action.rs`) and handler (`view.rs:20782-20808`). If `agent_panel` was wired to reuse `SubmitChatPrompt` instead of `SubmitAgentPrompt`, keep it and drop the unused one instead — reconcile with what 2b/2c actually landed.

- [ ] **Step 7: Build.**

```bash
cargo check -p warp-app --bin cast-codes --features gui,cast-agent,unified_agent_panel
```
Expected: PASS — the compiler flags any missed reference to the removed symbols; fix each.

- [ ] **Step 8: Model layer intact.**

```bash
cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat) or test(agent_panel)'
```
Expected: PASS — the `cli_chat` model/store/migration suites are untouched.

- [ ] **Step 9: Commit.**

```bash
git add -A
git commit -S -m "refactor(cli_chat): delete ChatPanelView; unified panel is the CLI surface"
```

---

## Task 4: Remove the `ai_assistant` coven-stream section

**Files:** `app/src/ai_assistant/panel.rs`

The unified panel now owns daemon conversations, so the Familiar panel's coven-stream rendering is redundant. Remove it; keep the panel's non-coven areas and the reused adapters (`coven_entry.rs`, `coven_stream_persist.rs`).

- [ ] **Step 1: Remove the render + call site.** Delete `render_coven_stream_section` (`:1470-1607`), the coven branch of `render_gateway_status_pill` (`:1614-1663` — keep any non-coven use, else remove), and the call at `:2226` (including the Task-2 flag gate that wrapped it).

- [ ] **Step 2: Remove the stream machinery.** Delete `coven_stream` field (`:254`), `CovenStreamState` (`:258-294`), `CovenStreamHistoryEntry` (`:298-303` — but see Step 4), `append_entry_coalescing` (`:308-325`), `buffer_coven_agent_event` (`:329-340`), `COVEN_STREAM_HISTORY_MAX` (`:342`), the field init in `new()` (`:502-510`), `reset_coven_stream`/`has_coven_stream_content` (`:933-997`), the stream consumer (`:1098-1227`), `surface_coven_offline_notice` (`:1370-1398`), `send_via_coven_gateway` (`:1401-1419`), and `poll_coven_stream` (`:1427-1461`). The compiler will list stragglers.

- [ ] **Step 3: Prune now-dead imports/events.** Remove any `use ::ai::cast_agent::*` / `CovenAgentEvent` imports and event subscriptions in `panel.rs` that only served the coven stream.

- [ ] **Step 4: Preserve the reused adapters.** Do **not** delete `app/src/ai_assistant/coven_entry.rs` (2c's `daemon_turn` re-exports `daemon_event_to_entry`) or `coven_stream_persist.rs` (2a's migration reads `stream-history.json`; the old `save()` is now uncalled but the reader/format stays). `CovenStreamHistoryEntry`'s on-disk shape is mirrored by 2a's `HistoryRecord` — if `CovenStreamHistoryEntry` lived in `panel.rs`, move it into `coven_stream_persist.rs` so the persist module still compiles after `panel.rs` loses it.

- [ ] **Step 5: Build + Familiar-panel non-regression.**

```bash
cargo check -p warp-app --bin cast-codes --features gui,cast-agent,unified_agent_panel
cargo nextest run -p warp-app --features cast-agent -E 'test(ai_assistant)'
```
Expected: PASS — the Familiar panel (editor/requests/suggestions) still builds and its tests pass.

- [ ] **Step 6: Commit.**

```bash
git add -A
git commit -S -m "refactor(ai_assistant): remove coven-stream section; unified panel owns daemon conversations"
```

---

## Task 5: Full gates

- [ ] **Step 1: Guards.**

```bash
./script/check_cli_chat_boundary   # cli_chat + agent_transcript + agent_panel
./script/check_ai_attribution
./script/check_rebrand
```
Expected: pass.

- [ ] **Step 2: Lint + fmt (both flag configs).**

```bash
cargo fmt -p warp-app
cargo clippy -p warp-app --features cast-agent --all-targets -- -D warnings
cargo clippy -p warp-app --features cast-agent,unified_agent_panel --all-targets -- -D warnings
```
Expected: clean in both — clippy must not flag dead code from the deletions.

- [ ] **Step 3: Full regression.**

```bash
cargo nextest run -p warp-app --features cast-agent,unified_agent_panel
```
Expected: PASS.

- [ ] **Step 4: Commit fmt-only changes.**

```bash
git add -A
git commit -S -m "chore(agent_panel): 2d gates + fmt"
```

---

## Done criteria (2d)

- `FeatureFlag::UnifiedAgentPanel` is default-enabled on the dogfood/preview channel; exactly one agent surface renders (the coven-stream section is gated off, then removed).
- `cli_chat::ChatPanelView` + all its panel wiring (field/construct/render/toggle action/binding/menu/state) are deleted; the `cli_chat` model layer and `OpenChatSession`/`CliChatNewChat` remain.
- The `ai_assistant` coven-stream section is removed; the Familiar panel's other areas still work; `coven_entry.rs` + `coven_stream_persist.rs` are preserved (reused by `agent_panel`).
- No daemon/CLI history is lost (sqlite store + `stream-history.json` migration intact); parity checklist fully ticked before any deletion; clippy `-D warnings` clean in both flag configs; guards pass; every commit signed.
- **Not** in 2d: routing CLIs through the daemon / retiring OSC-777 (Phase 3); deleting `coven_stream_persist`/`coven_entry` (still reused).

---

## Self-Review

**Spec coverage (DESIGN §"2d — Entry point + deprecate old panels"):** one entry point / default (Task 2) ✓; retire `cli_chat::ChatPanelView` (Task 3) ✓; retire the `ai_assistant` coven-stream section (Task 4) ✓; **staged — flag first, delete after parity** (Task 1 gate + Task 2 reversible flip precede Tasks 3–4) ✓.

**Placeholder scan:** the deletion steps enumerate exact symbols + line ranges from the code survey rather than "remove the old code." Task 3 §6 and Task 4 §4 explicitly say to *reconcile with what 2b/2c actually landed* (whether `agent_panel` reused `SubmitChatPrompt` or the new `SubmitAgentPrompt`, and where `CovenStreamHistoryEntry` lives) — these are genuine "confirm against the merged code" instructions, not vague TODOs, because the exact answer depends on 2b/2c merge details this plan cannot pin from `main`.

**Consistency:** "keep" vs "delete" is applied uniformly — model layer + `OpenChatSession`/`CliChatNewChat` + `coven_entry`/`coven_stream_persist` are kept everywhere they appear; `ChatPanelView`/`ToggleCliChatPanel`/`SubmitChatPrompt` + coven-stream render/machinery are deleted everywhere. The flag `UnifiedAgentPanel` (defined in 2b) is enabled in Task 2 and referenced in Task 4's gate.

**Risk flagged:** deletion PRs are high-blast-radius. Tasks 3–4 are gated behind Task 1 parity + the reversible Task 2 flip; if reviewers prefer, Tasks 3–4 can split into a separate follow-up "cleanup" PR after the default-flip soaks — the plan's staging supports either. The compiler is the safety net (every deletion step ends in a build that surfaces missed references).
