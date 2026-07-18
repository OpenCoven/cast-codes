# Unified Agent Panel — Parity Checklist (gate for 2d deletion)

> **Purpose:** 2d retires the two old AI-conversation surfaces (the standalone
> `cli_chat::ChatPanelView` and the `ai_assistant` panel's coven-stream section)
> in favor of the unified `agent_panel`. Deletion is **irreversible for
> reviewers' trust** — this checklist must be fully ticked (dogfooded via
> `./script/run` with the `UnifiedAgentPanel` flag on, `cmd-shift-U` /
> `ctrl-shift-U`) **before** the deletion tasks (PLAN-2d Tasks 3–4) begin.

## From `cli_chat::ChatPanelView`

- [ ] Conversation list, newest-first, spanning both backends, with a backend badge
- [ ] Selecting a conversation binds it and shows its transcript
- [ ] Composer sends to a live CLI conversation (PTY) — text reaches the agent
- [ ] "New chat" starts a CLI agent (`CliChatNewChat`)
- [ ] Agent/model label in the header
- [ ] Transcript renders all `ChatEntryKind` variants via `agent_transcript`
- [ ] Error banner / empty states behave sensibly

## From the `ai_assistant` coven-stream section

- [ ] Daemon (coven-code) conversations appear in the list (live sessions + migrated history)
- [ ] Composer sends to a daemon conversation and streams the reply (2c)
- [ ] Daemon availability is reflected (composer disabled when the runtime is down)
- [ ] Historical `stream-history.json` conversations are present (2a migration)

## Non-regressions

- [ ] The Familiar panel (editor / requests / input suggestions) still works after
      the coven-stream section is gated off / removed
- [ ] `cli_chat` model/store tests still pass; no daemon or CLI history is lost
- [ ] Exactly one agent surface is reachable when `UnifiedAgentPanel` is enabled
      (no double daemon rendering)

---

## Status

**Not yet dogfooded.** The remaining 2d work is gated on this checklist:

1. **Task 2 (reversible)** — default-enable `UnifiedAgentPanel` on the dogfood
   channel + gate the `ai_assistant` coven-stream render off when the flag is on.
   Ships nothing irreversible; enables the dogfood needed to tick this list.
2. **Tasks 3–4 (irreversible)** — delete `cli_chat::view/*` + `ToggleCliChatPanel`
   / `SubmitChatPrompt` wiring, and remove the `ai_assistant` coven-stream
   section. **Blocked until every box above is ticked.**

Per the DESIGN's staged rollout, Tasks 3–4 may also land as a separate
follow-up "cleanup" PR after the default-flip soaks.
