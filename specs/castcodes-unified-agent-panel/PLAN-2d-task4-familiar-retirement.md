# Retire the Familiar Agent Panel — Implementation Plan (v2, corrected)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the `ai_assistant` "Familiar" agent panel (`AIAssistantPanelView`) now that the unified `agent_panel` is the default agent surface — *without* breaking the AI server-API data layer that shares code with the panel.

> ## ⚠️ v2 correction — read this first
> An execution attempt (2026-07-18) found the v1 delete-set (`panel.rs` + `requests.rs` + `transcript.rs` + `utils.rs` + `coven_stream_persist.rs`) is **wrong**. The Familiar panel is **not** a cleanly-separable cluster:
>
> 1. **It's a circular UI knot.** `panel.rs ↔ transcript.rs ↔ utils.rs` are mutually dependent:
>    - `transcript.rs` imports `panel::{HEADER_HEIGHT, HEXAGON_ALERT_SVG_PATH}`
>    - `panel.rs` holds `transcript_view: ViewHandle<Transcript>` and builds it (`panel.rs:470 Transcript::new`)
>    - `utils.rs` imports `panel::AIAssistantAction` and dispatches it (`utils.rs:308`)
>    - `render_prepared_response_button` / `render_request_limit_info` live in `utils.rs` but are called by **both** `panel.rs` **and** `transcript.rs`
> 2. **The UI knot is fused to the AI server-API data layer (must survive):**
>    - `app/src/server/server_api/ai.rs` imports `ai_assistant::utils::TranscriptPart` (6 sites) and `ai_assistant::requests::GenerateDialogueResult`
>    - `app/src/auth/mod.rs` imports `ai_assistant::requests::REQUEST_LIMIT_INFO_CACHE_KEY`
>
> So retiring the panel is a **data-vs-UI extraction refactor**, not a deletion. And it **touches the `server_api` compile** — a critical layer.
>
> **This is NOT release-blocking.** The unified panel is already the default agent surface (#216) and the Familiar coven-stream is suppressed by default (the 2d stage-1 gate, #213). Physically deleting the panel is cosmetic cleanup. Do it as its own dogfood-backed effort — ideally alongside retiring the legacy hosted-AI `server_api` dialogue path, since they're entangled.

---

## Prerequisites

1. **Dogfood the unified panel** (`PARITY-2d.md`) — confirm it fully covers the daemon flow before removing the fallback UI.
2. **Decide the scope of the legacy hosted-AI path.** `requests.rs`/`transcript.rs`/`utils.rs` exist to serve the old Warp-hosted AI dialogue (stubbed `501` in this fork; `requests.rs` header: *"TODO: Delete all of this once agent mode fully replaces the AI assistant panel"*). Two viable scopes:
   - **(A) Panel-only retirement** — delete the panel UI, extract the data types that `server_api`/`auth` need into a surviving module, keep `requests.rs` (data). Leaves the hosted-AI *server-API* types in place.
   - **(B) Full hosted-AI retirement** — also remove the `server_api` dialogue path (`GenerateDialogueResult`, the `generate_dialogue_answer` endpoint, the `TranscriptPart` graphql conversion) so the whole legacy AI-dialogue subsystem goes. Larger; verify no other consumer.
   Pick one and record it at the top of the retirement PR. **This plan details (A);** (B) additionally deletes the `server_api` dialogue surface.

---

## Dependency map (ground truth for the extraction)

**KEEP (used app-wide, unrelated to the panel):**
- `ai_assistant::execution_context::WarpAiExecutionContext` — 11 files (terminal, ai/agent, ai/predict, active_session, pane_group).
- `ai_assistant::AskAIType` + `AI_ASSISTANT_FEATURE_NAME`/`AI_ASSISTANT_LOGO_COLOR`/`ASK_AI_ASSISTANT_TEXT` — terminal/view.rs, pane_group/mod.rs, workspace/view.rs.
- `ai_assistant::coven_entry::daemon_event_to_entry` — `agent_panel::daemon_turn` (the unified panel).

**KEEP as data (server-API / auth consumers), EXTRACT out of the UI knot:**
- `utils::TranscriptPart` (+ `FormattedTranscriptMessage`, `MarkdownSegment`, `CodeBlockIndex`, `TranscriptPartSubType`, `AssistantTranscriptPart`, `markdown_segments_from_text`) → move into a new UI-free module, e.g. `ai_assistant/dialogue_types.rs`. Verify each moved item does **not** transitively pull `panel`/`transcript` UI types (`CodeBlockMouseStateHandles` etc. must NOT come along).
- `requests::GenerateDialogueResult` + `requests::REQUEST_LIMIT_INFO_CACHE_KEY` → keep in `requests.rs` **iff** `requests.rs` compiles without the panel; otherwise split its data types out too.

**DELETE (the UI knot):**
- `panel.rs` (the `AIAssistantPanelView`) + `coven_stream_persist.rs` (orphaned once `panel.rs` goes — verified no external caller).
- `transcript.rs` (the `Transcript` view — only `panel.rs` + `transcript_tests.rs` construct it).
- The **UI half** of `utils.rs`: `render_prepared_response_button`, `render_request_limit_info`, `AIAssistantAction` (relocate/delete), and anything importing `panel`/`transcript` UI types.

---

## Tasks (scope A)

### Task 1: Prove the data/UI split compiles in isolation
- [ ] Create `ai_assistant/dialogue_types.rs`; move `TranscriptPart` + its supporting types + `markdown_segments_from_text` there. Update `server_api/ai.rs` + any `ai_assistant`-internal users to the new path.
- [ ] Confirm the moved types do **not** reference `panel`/`transcript`/`AIAssistantAction`/mouse-handle UI types. If they do, extract those leaf types too or stop and reassess.
- [ ] `cargo check -p warp-app --features gui,cast-agent` — `server_api` should now depend only on `dialogue_types`, not the UI knot (the panel/transcript UI still compiles for now).
- [ ] Commit.

### Task 2: Unwire the panel from the workspace (compiler-driven)
- [ ] `workspace/action.rs`: remove `ToggleAIAssistant` (`:228`) + `ClickedAIAssistantIcon` (`:229`) + their exhaustiveness arms (`:868-869`).
- [ ] `workspace/mod.rs`: remove the Familiar keybinding (the `ToggleAIAssistant` `EditableBinding` gated `!AgentMode`, ~`:1225`); **keep** the `NewPaneInAgentMode` binding on the shared `workspace:toggle_ai_assistant` id.
- [ ] `lib.rs`: remove `ai_assistant::panel::init(ctx)` (`:1638`).
- [ ] `workspace/view.rs`: remove the field (`:967`), type import (`:365`), `build_ai_assistant_panel_view` (`:1515`), the construct + struct-init (`:2816`, `:3104`), `handle_ai_assistant_panel_event` + the `AIAssistantPanelEvent` import (`:365`, `:15969`), `toggle_ai_assistant_panel` (`:4201`), the `ClickedAIAssistantIcon` dispatch + handler (`:18313`, `:21093`), `ASK_AI_ASSISTANT_KEYBINDING_NAME` (`:569`,`:619`), and **all ~25 `self.ai_assistant_panel` / `is_ai_assistant_panel_open` focus/state sites** (treat the panel as permanently closed — drop those branches). Keep `AskAIType`/`AI_ASSISTANT_FEATURE_NAME` imports still used elsewhere.
- [ ] `workspace/util.rs`: remove `is_ai_assistant_panel_open`.
- [ ] Build iteratively until the only remaining errors are the to-be-deleted files.
- [ ] Commit.

### Task 3: Delete the UI knot
- [ ] Relocate/delete `AIAssistantAction` (only `utils.rs` uses `PreparedPrompt`; if the prepared-prompt button is deleted, the enum goes entirely).
- [ ] `git rm app/src/ai_assistant/{panel.rs, transcript.rs, coven_stream_persist.rs}` and delete the UI fns from `utils.rs` (`render_prepared_response_button`, `render_request_limit_info`, the `panel`/`transcript` imports). Update `ai_assistant/mod.rs` (`pub mod` list + panel-only constants — grep each before dropping).
- [ ] Delete `transcript_tests.rs`; port any still-relevant coverage.
- [ ] Build until clean.
- [ ] Commit.

### Task 4: Tests + fixtures
- [ ] `integration_testing/view_getters.rs`: remove `ai_assistant_panel_view()` (`:11`,`:217`).
- [ ] `workspace/view_tests.rs`: remove the Familiar focus tests (`:1175-1203`).
- [ ] Run `cargo nextest run -p warp-app --features cast-agent -E 'test(workspace) or test(ai_assistant) or test(agent_panel) or test(server)'`.
- [ ] Commit.

### Task 5: Gates (both flag configs)
- [ ] `./script/check_cli_chat_boundary`, `check_ai_attribution`, `check_rebrand`.
- [ ] `cargo fmt`; `cargo clippy … --features cast-agent` and `…,unified_agent_panel` (all-targets, `-D warnings`) — clean, **including `server_api`**.
- [ ] `cargo nextest run -p warp-app --features cast-agent,unified_agent_panel`.
- [ ] Commit.

---

## Done criteria (scope A)
- The Familiar panel UI (`panel.rs` + `transcript.rs` + the UI half of `utils.rs` + `coven_stream_persist.rs`) is deleted and unwired from the workspace (~25 focus sites, actions, keybinding, init, event handler, state, field, fixtures).
- The AI-dialogue **data types** (`TranscriptPart` & friends, `GenerateDialogueResult`, the cache key) survive in a UI-free module; **`server_api` and `auth` still compile**.
- App-wide `ai_assistant` surface (`AskAIType`, `execution_context`, `coven_entry`) untouched; Agent Mode unaffected.
- Both flag configs clippy `-D warnings` clean; full suite passes; every commit signed.

## Risks
1. **`server_api` compile** — the extraction (Task 1) is the crux; if `TranscriptPart` transitively needs UI types, the split is harder. Task 1 stops-and-reassesses in that case.
2. **~25 workspace focus sites** — behavior-preserving only if the panel is truly always-closed; **needs dogfood verification** of workspace focus after the change (an agent cannot verify this).
3. **Scope creep to (B)** — if the surviving data module drags in most of the hosted-AI path, consider doing (B) instead (retire the whole hosted-AI dialogue subsystem) as a cleaner cut.

---

## History
- **v1** (superseded): assumed `panel`+`requests`+`transcript`+`utils` were a cleanly-deletable cluster with a functional Familiar panel minus a "coven-stream section." Both premises were wrong — see the v2 correction above.
