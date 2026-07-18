# Unified Agent Panel — Phase 2c Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the unified agent panel's composer *route by backend* — CLI conversations submit to the terminal PTY (already wired in 2b), Coven-daemon conversations submit to `cast_agent` and stream structured events back into the same transcript — and populate the list with *live* daemon sessions.

**Architecture:** Three pieces. (1) Generalize `ConversationBinding` with a daemon variant so a selected daemon conversation is "sendable" without a terminal. (2) A direct `ChatModel::append_entry` path (daemon events aren't `CLIAgentEvent`s, so they can't go through `apply_event`). (3) A daemon send/stream consumer on `AgentPanelView` that mirrors the shipped `ai_assistant` panel: build an `AgentMessage`, launch the daemon turn via `CastAgent::stream_agent_events` (which already delivers the prompt in the `POST /api/v1/sessions` launch body with `launchMode:"stream"` — the Phase-1 "prompt-delivery detail" is thus already resolved), buffer `CovenAgentEvent`s, and drain them through `daemon_event_to_entry` → `ChatModel::append_entry`.

**Tech Stack:** Rust, warpui (`View`/`ModelHandle`/async spawn+buffer+drain), `cast_agent` (`stream_agent_events`, `AgentMessage`, `CovenAgentEvent`, `::ai::cast_agent::{global, sessions}`), the shared `agent_transcript` render model, rusqlite via `ChatStore`.

**Depends on:** 2a (#205 — `ConversationBackend`, `backend` column, migration) and 2b (#206 — `agent_panel::AgentPanelView`, `WorkspaceAction::{ToggleUnifiedAgentPanel, SubmitAgentPrompt}`, the composer shell, `migrate_stream_history_file` at bootstrap). Rebase this on top of 2b before starting.

**Key facts established by code survey (cite in review):**
- **Prompt delivery (RESOLVED):** `GatewayClient::launch_daemon_session` (`crates/cast_agent/src/gateway.rs:298-393`) extracts the prompt via `daemon_chat::extract_prompt(&msg.body)` and POSTs `{"projectRoot","harness","prompt","launchMode":"stream","title"}` to `/api/v1/sessions`. **No stdin user-frame is needed; the launch-body prompt is the delivery mechanism.** Each submit is one launch (session-per-turn); multi-turn daemon *context continuity* is a harness/daemon concern and stays out of 2c.
- **Binding liveness:** `ChatModel::bind_live` has no production caller; the panel auto-binds on the first CLI `EventReceived` in `apply_event` (`app/src/cli_chat/model.rs:394-399`). Daemon conversations produce no `CLIAgentEvent`, so 2c introduces an explicit daemon binding.
- **Reference implementation:** `app/src/ai_assistant/panel.rs:1233-1262` (stream consumer) + `:330-340` (`buffer_coven_agent_event`) + `app/src/ai_assistant/coven_entry.rs:14-52` (`daemon_event_to_entry`). 2c mirrors this exactly, targeting `ChatModel` instead of `CovenStreamState`.

**Out of scope:** deleting the old panels / making the unified panel default (2d); routing CLIs through the daemon or retiring OSC-777 (Phase 3); multi-turn daemon context continuity.

---

## File Structure

| File | Responsibility | Create/Modify |
|---|---|---|
| `app/src/cli_chat/conversation.rs` | Add `ConversationBinding::LiveDaemon { session_id }` | Modify (`:213-222`) |
| `app/src/cli_chat/model.rs` | `bind_daemon`, `append_entry`, `refresh_daemon_conversations` | Modify |
| `app/src/cli_chat/model_tests.rs` | Tests for the three new methods | Modify |
| `app/src/cli_chat/conversation_tests.rs` | (if binding has unit coverage) | Modify |
| `app/src/agent_panel/mod.rs` | Daemon-stream buffer field on `AgentPanelView` + drain | Modify |
| `app/src/agent_panel/daemon_turn.rs` | Build `AgentMessage`, spawn stream consumer, buffer events | Create |
| `app/src/agent_panel/view/composer.rs` | Enable composer for `LiveDaemon` too | Modify |
| `app/src/agent_panel/daemon_turn_tests.rs` | Unit tests: message build + event→entry drain | Create |
| `app/src/workspace/view.rs` | `SubmitAgentPrompt` daemon branch; `OpenChatSession` backend routing | Modify (`:20766`, `:20782` areas) |

**Reference (read, do not modify): `app/src/ai_assistant/panel.rs` (stream consumer + spawn/buffer/drain machinery), `app/src/ai_assistant/coven_entry.rs` (`daemon_event_to_entry`), `crates/cast_agent/src/gateway.rs:298-393` (`launch_daemon_session`), `crates/cast_agent/tests/daemon_streaming.rs` (in-process daemon stub for integration coverage).**

---

## Task 1: A daemon-aware conversation binding

**Files:**
- Modify: `app/src/cli_chat/conversation.rs:213-222`, `app/src/cli_chat/model.rs`
- Test: `app/src/cli_chat/model_tests.rs`

Today `ConversationBinding` is `None | Live { session_id, terminal_view_id } | Past { session_id }`. `Live` is terminal-bound (CLI only). Add a daemon variant that carries only a `session_id` — a selected daemon conversation is sendable without a terminal.

- [ ] **Step 1: Write the failing test.** In `model_tests.rs`:

```rust
#[test]
fn selecting_a_daemon_conversation_binds_it_live_daemon() {
    use crate::cli_chat::conversation::{ConversationBackend, ConversationBinding};

    let mut model = ChatModel::for_test(); // existing/test seam (see Task note)
    // Seed a daemon conversation directly.
    model.upsert_daemon_conversation_for_test("d1", "coven-code");

    model.bind_daemon("d1".into());

    assert!(matches!(
        model.binding(),
        ConversationBinding::LiveDaemon { session_id } if session_id == "d1"
    ));
}
```

  Use whatever in-memory model constructor the existing model tests use (grep `ChatModel::` in `model_tests.rs`); if there is no non-`ctx` seam, add a minimal `#[cfg(test)] fn for_test()` and a `#[cfg(test)] fn upsert_daemon_conversation_for_test` that inserts a `ChatConversation::new(id, ConversationBackend::Daemon { harness }, now)` into `self.conversations`.

- [ ] **Step 2: Run — verify fail.**

Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat::model_tests::selecting_a_daemon)'`
Expected: FAIL (no `LiveDaemon` / no `bind_daemon`).

- [ ] **Step 3: Add the variant.** In `conversation.rs`:

```rust
pub enum ConversationBinding {
    None,
    Live {
        session_id: String,
        terminal_view_id: warpui::EntityId,
    },
    Past {
        session_id: String,
    },
    /// A Coven-daemon conversation selected for input. No terminal — sends go
    /// through cast_agent. Sendable whenever the daemon runtime is available.
    LiveDaemon {
        session_id: String,
    },
}
```

- [ ] **Step 4: Add `bind_daemon`.** In `model.rs`, next to `bind_past` (grep it):

```rust
    /// Bind a daemon conversation for input. Emits `BindingChanged`.
    pub fn bind_daemon(&mut self, session_id: String, ctx: &mut ModelContext<Self>) {
        self.binding = ConversationBinding::LiveDaemon { session_id };
        ctx.emit(ChatModelEvent::BindingChanged);
    }
```

  (The test calls a `ctx`-free form; if the test seam has no `ctx`, split into `fn set_binding(&mut self, ConversationBinding)` used by both, or have the test assert after a `ctx`-driven call using the existing model-test harness. Match the pattern the existing `bind_past` tests use.)

- [ ] **Step 5: Handle the new variant everywhere `ConversationBinding` is matched.** Grep `ConversationBinding::` across `app/src` and add arms (transcript lookup treats `LiveDaemon { session_id }` like `Live`/`Past` — resolve the conversation by `session_id`). The compiler enumerates the non-exhaustive matches; fix each.

- [ ] **Step 6: Run — verify pass** + full binding regression.

Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat)'`
Expected: PASS (new test + all pre-existing).

- [ ] **Step 7: Commit.**

```bash
git add app/src/cli_chat/conversation.rs app/src/cli_chat/model.rs app/src/cli_chat/model_tests.rs
git commit -S -m "feat(cli_chat): add LiveDaemon binding for daemon conversations"
```

---

## Task 2: Direct `ChatModel::append_entry` path

**Files:**
- Modify: `app/src/cli_chat/model.rs`
- Test: `app/src/cli_chat/model_tests.rs`

Daemon events are `CovenAgentEvent`s, not `CLIAgentEvent`s, so they cannot go through `apply_event` (`model.rs:268-403`). Add a public method that appends a ready-made `ChatEntry` to a conversation, assigning the next sequence, persisting, and emitting — factoring the append/persist/emit tail of `apply_event` so both share it.

- [ ] **Step 1: Write the failing test.** In `model_tests.rs`:

```rust
#[test]
fn append_entry_adds_persists_and_sequences() {
    use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind};

    let mut model = ChatModel::for_test();
    model.upsert_daemon_conversation_for_test("d1", "coven-code");

    let e0 = ChatEntry {
        sequence: 0, // append_entry overrides with the model's next seq
        created_at: chrono::Utc::now(),
        kind: ChatEntryKind::UserPrompt { text: "hello".into() },
    };
    model.append_entry_for_test("d1", e0);
    let e1 = ChatEntry {
        sequence: 0,
        created_at: chrono::Utc::now(),
        kind: ChatEntryKind::AssistantResponse { text: "hi".into() },
    };
    model.append_entry_for_test("d1", e1);

    let conv = model.conversation("d1").unwrap();
    assert_eq!(conv.entries.len(), 2);
    assert_eq!(conv.entries[0].sequence, 0);
    assert_eq!(conv.entries[1].sequence, 1, "sequence auto-assigned");
}
```

- [ ] **Step 2: Run — verify fail.**

- [ ] **Step 3: Implement.** In `model.rs`:

```rust
    /// Append a ready-made entry to `session_id`, assigning the next sequence,
    /// persisting to the store, and emitting `ConversationUpdated` +
    /// `ConversationListChanged`. Used by the daemon source (events that are
    /// not `CLIAgentEvent`s). No-op if the conversation is unknown.
    pub fn append_entry(
        &mut self,
        session_id: &str,
        mut entry: ChatEntry,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(conv) = self.conversations.get_mut(session_id) else {
            return;
        };
        let seq = self.next_sequence.entry(session_id.to_string()).or_insert(0);
        entry.sequence = *seq;
        *seq += 1;
        conv.updated_at = entry.created_at;
        conv.entries.push(entry.clone());

        if let Some(store) = self.store.as_ref() {
            let _ = store.upsert_conversation(self.conversations.get(session_id).unwrap());
            let _ = store.insert_entry(session_id, &entry);
        }
        ctx.emit(ChatModelEvent::ConversationUpdated { session_id: session_id.to_string() });
        ctx.emit(ChatModelEvent::ConversationListChanged);
    }
```

  Refactor: if `apply_event`'s tail (`model.rs:371-388`) duplicates this persist/emit logic, extract a private `persist_and_emit` both call — keep `apply_event`'s behavior byte-identical (its tests gate it). Add the `for_test` shims used by the test (ctx-free wrappers or drive through the existing model-test harness).

- [ ] **Step 4: Run — verify pass** (new + all pre-existing model tests).

- [ ] **Step 5: Commit.**

```bash
git add app/src/cli_chat/model.rs app/src/cli_chat/model_tests.rs
git commit -S -m "feat(cli_chat): ChatModel::append_entry direct entry path"
```

---

## Task 3: Live daemon conversation source

**Files:**
- Modify: `app/src/cli_chat/model.rs`
- Test: `app/src/cli_chat/model_tests.rs`

2b's migration seeds *historical* daemon conversations. This adds *live* ones from the running daemon: upsert a `Daemon` conversation per `CovenSession` so active daemon sessions appear in the merged list. The panel calls this on refresh (Task 4 wiring); the model method is pure + testable.

- [ ] **Step 1: Write the failing test.** In `model_tests.rs`:

```rust
#[test]
fn refresh_daemon_conversations_upserts_sessions() {
    use crate::cli_chat::conversation::ConversationBackend;

    let mut model = ChatModel::for_test();
    // Minimal session descriptors (id, name/title, harness).
    model.refresh_daemon_conversations_for_test(&[
        ("s1".to_string(), "Fix auth".to_string(), "coven-code".to_string()),
    ]);

    let conv = model.conversation("s1").expect("session upserted");
    assert!(matches!(conv.backend, ConversationBackend::Daemon { .. }));
    assert_eq!(conv.title, "Fix auth");

    // Idempotent: a second refresh with the same id keeps one conversation.
    model.refresh_daemon_conversations_for_test(&[
        ("s1".to_string(), "Fix auth".to_string(), "coven-code".to_string()),
    ]);
    assert_eq!(model.conversations_sorted_by_recency().iter().filter(|c| c.session_id == "s1").count(), 1);
}
```

- [ ] **Step 2: Run — verify fail.**

- [ ] **Step 3: Implement.** In `model.rs`, take the real `CovenSession` list in the production method and a tuple slice in the test shim:

```rust
    /// Upsert a `Daemon` conversation for each live daemon session. Preserves
    /// existing entries (only refreshes list membership + title). Idempotent.
    pub fn refresh_daemon_conversations(
        &mut self,
        sessions: &[::ai::cast_agent::CovenSession],
        harness: &str,
        now: DateTime<Utc>,
        ctx: &mut ModelContext<Self>,
    ) {
        let mut changed = false;
        for s in sessions {
            let conv = self.conversations.entry(s.id.clone()).or_insert_with(|| {
                changed = true;
                ChatConversation::new(
                    s.id.clone(),
                    ConversationBackend::Daemon { harness: harness.to_string() },
                    now,
                )
            });
            if conv.title != s.name {
                conv.title = s.name.clone();
                changed = true;
            }
        }
        if changed {
            ctx.emit(ChatModelEvent::ConversationListChanged);
        }
    }
```

  Add the `_for_test` shim mapping `(id, name, harness)` tuples to the same upsert logic without `ctx`. `harness` is `"coven-code"` for now (the only stream-capable lane); a future multi-harness source can pass per-session harness.

- [ ] **Step 4: Run — verify pass.**

- [ ] **Step 5: Commit.**

```bash
git add app/src/cli_chat/model.rs app/src/cli_chat/model_tests.rs
git commit -S -m "feat(cli_chat): live daemon conversation source (refresh from sessions)"
```

---

## Task 4: Composer routing — daemon send + stream consumer

**Files:**
- Create: `app/src/agent_panel/daemon_turn.rs`, `app/src/agent_panel/daemon_turn_tests.rs`
- Modify: `app/src/agent_panel/mod.rs`, `app/src/workspace/view.rs`

The `SubmitAgentPrompt` handler (2b) routes CLI. Add the daemon branch: build an `AgentMessage`, append the user prompt immediately, launch `stream_agent_events`, buffer `CovenAgentEvent`s, and drain them into `ChatModel::append_entry` via `daemon_event_to_entry`. **Mirror `ai_assistant::panel.rs` exactly for the spawn/buffer/drain machinery** — that is the proven pattern for bridging the async cast_agent stream into a warpui model without cross-thread model mutation.

- [ ] **Step 1: Write the failing unit tests** (pure logic; the live stream is covered by the existing `crates/cast_agent/tests/daemon_streaming.rs` stub). `app/src/agent_panel/daemon_turn_tests.rs`:

```rust
use crate::agent_panel::daemon_turn::{build_daemon_message, drain_event_to_entry};

#[test]
fn build_daemon_message_puts_prompt_in_body() {
    let msg = build_daemon_message("conv-1", "do the thing");
    assert_eq!(msg.conversation_id, "conv-1");
    assert_eq!(msg.body.get("prompt").and_then(|v| v.as_str()), Some("do the thing"));
}

#[test]
fn drain_maps_assistant_delta_to_entry() {
    use ::ai::cast_agent::CovenAgentEvent;
    let entry = drain_event_to_entry(
        CovenAgentEvent::AssistantDelta { text: "hi".into() },
        0,
        chrono::Utc::now(),
    );
    assert!(matches!(
        entry.map(|e| e.kind),
        Some(crate::agent_transcript::entry::ChatEntryKind::AssistantResponse { text }) if text == "hi"
    ));
}

#[test]
fn drain_ignores_ignored_events() {
    use ::ai::cast_agent::CovenAgentEvent;
    assert!(drain_event_to_entry(CovenAgentEvent::Ignored, 0, chrono::Utc::now()).is_none());
}
```

- [ ] **Step 2: Run — verify fail.**

- [ ] **Step 3: Implement the pure helpers.** `app/src/agent_panel/daemon_turn.rs`:

```rust
//! Daemon-turn plumbing for the unified agent panel: build the cast_agent
//! message for a composer submit, and map streamed daemon events into
//! transcript entries. The live stream consumer (spawn + buffer + drain) is
//! wired on `AgentPanelView` and mirrors `ai_assistant::panel.rs`.

use chrono::{DateTime, Utc};

use crate::agent_transcript::entry::ChatEntry;

/// Build the `AgentMessage` for a daemon turn. The prompt goes in `body.prompt`
/// — `GatewayClient::launch_daemon_session` reads it via `extract_prompt` and
/// delivers it in the `POST /api/v1/sessions` launch body (launchMode:"stream").
pub fn build_daemon_message(conversation_id: &str, text: &str) -> ::ai::cast_agent::AgentMessage {
    ::ai::cast_agent::AgentMessage {
        conversation_id: conversation_id.to_string(),
        body: serde_json::json!({ "prompt": text }),
    }
}

/// Map one streamed daemon event to a transcript entry (delegates to the
/// shared `ai_assistant` converter so both panels agree).
pub fn drain_event_to_entry(
    event: ::ai::cast_agent::CovenAgentEvent,
    sequence: u64,
    now: DateTime<Utc>,
) -> Option<ChatEntry> {
    crate::ai_assistant::coven_entry::daemon_event_to_entry(event, sequence, now)
}
```

  Confirm `daemon_event_to_entry` is reachable from `agent_panel` (it's `pub` in `ai_assistant::coven_entry`). If module visibility blocks it, re-export it as `pub use` from a neutral spot, or move it to `agent_transcript` (it depends only on `CovenAgentEvent` + `ChatEntry` — a clean home). Prefer re-export to keep 2c small.

- [ ] **Step 4: Run — verify pass** (the three unit tests).

- [ ] **Step 5: Add the stream consumer to `AgentPanelView`.** In `app/src/agent_panel/mod.rs`, add a buffer field and a `start_daemon_turn` method that mirrors `ai_assistant::panel.rs:1233-1262` + its surrounding spawn/buffer machinery (read those lines and replicate the spawn primitive, the `Arc<Mutex<..>>` buffer, and the terminal-event break). Sketch:

```rust
use std::sync::{Arc, Mutex};

// field on AgentPanelView:
//   daemon_buffer: Arc<Mutex<Vec<::ai::cast_agent::CovenAgentEvent>>>,
// initialized to Arc::new(Mutex::new(Vec::new())) in new()/with_model().

impl AgentPanelView {
    /// Launch a daemon turn and stream events into `daemon_buffer`. The drain
    /// (Step 6) moves them into the model on the UI thread.
    #[cfg(unix)]
    pub(crate) fn start_daemon_turn(&self, conversation_id: String, text: String) {
        let Some(rt) = ::ai::cast_agent::global() else { return };
        let msg = crate::agent_panel::daemon_turn::build_daemon_message(&conversation_id, &text);
        let buffer = self.daemon_buffer.clone();
        // Spawn on cast_agent's runtime exactly as ai_assistant/panel.rs does:
        //   rt.spawn(async move {
        //       if let Ok(mut s) = agent.stream_agent_events(msg).await {
        //           while let Some(ev) = s.next().await {
        //               let terminal = matches!(ev, Done | Error{..});
        //               buffer.lock()..push(ev);
        //               if terminal { break; }
        //           }
        //       }
        //   });
        // Use the SAME runtime handle / spawn call the ai_assistant panel uses.
        let _ = (rt, msg, buffer);
        unimplemented!("mirror ai_assistant/panel.rs:1233-1262 spawn+stream loop")
    }
}
```

- [ ] **Step 6: Drain buffered events into the model.** On render (or a periodic notify, mirroring how `ai_assistant` drains `pending_entries`), move buffered events into the model:

```rust
    /// Drain streamed daemon events into the bound daemon conversation. Called
    /// from render/tick, mirroring ai_assistant's pending-entries drain.
    fn drain_daemon_buffer(&self, ctx: &mut ViewContext<Self>) {
        let drained: Vec<_> = {
            let mut buf = self.daemon_buffer.lock().unwrap_or_else(|p| p.into_inner());
            std::mem::take(&mut *buf)
        };
        if drained.is_empty() { return; }
        let session_id = match self.chat_model.as_ref(ctx).binding() {
            crate::cli_chat::conversation::ConversationBinding::LiveDaemon { session_id } => session_id.clone(),
            _ => return,
        };
        self.chat_model.update(ctx, |model, ctx| {
            for ev in drained {
                if let Some(entry) = crate::agent_panel::daemon_turn::drain_event_to_entry(
                    ev, 0, chrono::Utc::now(),
                ) {
                    model.append_entry(&session_id, entry, ctx);
                }
            }
        });
    }
```

  Call `drain_daemon_buffer` at the top of `View::render` (like `ai_assistant` does) so streamed entries appear. Match the exact drain-trigger the reference panel uses.

- [ ] **Step 7: Wire the `SubmitAgentPrompt` daemon branch.** In `app/src/workspace/view.rs`, extend the `SubmitAgentPrompt` handler (added in 2b) so that when the binding is `LiveDaemon { session_id }`:
  1. Append the user's prompt immediately as a `ChatEntryKind::UserPrompt { text }` via `ChatModel::append_entry`.
  2. Call `agent_panel_view.read(ctx, |v, _| v.start_daemon_turn(session_id, text))` (or `update`) to launch the stream.

  For `Live { .. }` (CLI) keep the 2b PTY path. Grep the 2b handler and add the daemon arm; the compiler confirms exhaustiveness.

- [ ] **Step 8: Route `OpenChatSession` by backend.** In the `OpenChatSession` handler (`view.rs:20766-20779`), branch on the selected conversation's backend: `Daemon { .. }` → `model.bind_daemon(session_id)`; `Cli(_)` → existing `bind_past` behavior. This makes selecting a daemon conversation enable the composer.

- [ ] **Step 9: Build + run.**

```bash
cargo check -p warp-app --bin cast-codes --features gui,cast-agent
cargo nextest run -p warp-app --features cast-agent -E 'test(agent_panel::daemon_turn_tests)'
```
Expected: PASS.

- [ ] **Step 10: Commit.**

```bash
git add app/src/agent_panel/ app/src/workspace/view.rs
git commit -S -m "feat(agent_panel): route composer by backend — daemon send + stream consumer"
```

---

## Task 5: Enable the composer for daemon conversations

**Files:**
- Modify: `app/src/agent_panel/view/composer.rs`
- Test: `app/src/agent_panel/view_tests.rs`

2b's composer is active only for a live *CLI* binding. Extend "active" to a `LiveDaemon` binding when the daemon runtime is available.

- [ ] **Step 1: Write the failing test.** Add to `view_tests.rs` a pure predicate test — factor the active check into a testable function `composer_is_active(binding, backend_lookup, daemon_available) -> bool`:

```rust
#[test]
fn composer_active_for_live_daemon_when_runtime_up() {
    use crate::agent_panel::view::composer::composer_is_active_for;
    use crate::cli_chat::conversation::ConversationBinding;

    let b = ConversationBinding::LiveDaemon { session_id: "d1".into() };
    assert!(composer_is_active_for(&b, /* daemon_available */ true));
    assert!(!composer_is_active_for(&b, /* daemon_available */ false));
}
```

- [ ] **Step 2: Run — verify fail.**

- [ ] **Step 3: Implement.** In `composer.rs`, factor the predicate and use it in `render_composer`:

```rust
/// Whether the composer accepts input for this binding.
pub fn composer_is_active_for(binding: &ConversationBinding, daemon_available: bool) -> bool {
    match binding {
        ConversationBinding::Live { .. } => true, // live CLI (terminal present)
        ConversationBinding::LiveDaemon { .. } => daemon_available,
        _ => false,
    }
}
```

  In `render_composer`, compute `daemon_available` via `::ai::cast_agent::is_available()` and select active vs. placeholder using `composer_is_active_for(chat.binding(), daemon_available)` (keep the CLI-terminal-presence nuance from 2b for the `Live` arm if it checks the terminal exists).

- [ ] **Step 4: Run — verify pass** + layout test still green.

- [ ] **Step 5: Commit.**

```bash
git add app/src/agent_panel/view/composer.rs app/src/agent_panel/view_tests.rs
git commit -S -m "feat(agent_panel): enable composer for live daemon conversations"
```

---

## Task 6: Full gates + prompt-delivery confirmation note

**Files:**
- Modify: `specs/castcodes-unified-agent-panel/DESIGN.md` (or a short `NOTES` in the panel module) — record that the Phase-1 prompt-delivery question is resolved.

- [ ] **Step 1: Record the resolution.** Add one line to the DESIGN (Phase 2c section) or a `//!` note in `daemon_turn.rs`: *"Stream-mode prompt delivery: the prompt is carried in the `POST /api/v1/sessions` launch body (`launchMode:"stream"`) via `launch_daemon_session`; no stdin user-frame is required. Each submit is one launch (session-per-turn)."* (Already stated in `daemon_turn.rs` doc comment — ensure it's present.)

- [ ] **Step 2: Guards.**

```bash
./script/check_cli_chat_boundary   # now includes agent_panel (2b); daemon_turn imports only cast_agent facade + agent_transcript
./script/check_ai_attribution
./script/check_rebrand
```
Expected: pass. Note: `daemon_turn.rs` references `::ai::cast_agent` (the facade) and `crate::ai_assistant::coven_entry` — neither is Warp-owned infra, but confirm the boundary guard's pattern doesn't false-positive; if `ai_assistant` is off-limits for the guarded dirs, move `daemon_event_to_entry` into `agent_transcript` (Step 3 of Task 4's re-export note) so `agent_panel` depends only on `agent_transcript` + the `cast_agent` facade.

- [ ] **Step 3: Lint + fmt.**

```bash
cargo fmt -p warp-app
cargo clippy -p warp-app --features cast-agent --all-targets -- -D warnings
```
Expected: clean.

- [ ] **Step 4: Full regression.**

```bash
cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat) or test(agent_panel) or test(agent_transcript)'
cargo nextest run -p cast_agent   # daemon_streaming stub still green (stream consumer contract)
```
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add -A
git commit -S -m "chore(agent_panel): 2c gates + prompt-delivery note"
```

---

## Done criteria (2c)

- Selecting a daemon conversation binds it `LiveDaemon` and enables the composer; submitting sends via `cast_agent` and streams `CovenAgentEvent`s back into the same `agent_transcript` transcript through `ChatModel::append_entry`.
- CLI conversations still submit to the terminal PTY (2b path unchanged).
- Live daemon sessions from `::ai::cast_agent::sessions()` populate the merged list alongside migrated + CLI conversations.
- The stream-mode prompt-delivery detail is confirmed resolved (launch-body prompt) and documented.
- Full `cli_chat`/`agent_panel`/`agent_transcript` suites pass; `cast_agent` daemon-stream stub passes; clippy `-D warnings` clean; guards pass; every commit signed.
- **Not** in 2c: deleting the old panels / default entry point (2d); multi-turn daemon context continuity; OSC-777 retirement (Phase 3).

---

## Self-Review

**Spec coverage (DESIGN §"2c — Composer routing + daemon source wiring"):** route submit by backend (Task 4 §7) ✓; wire the Coven-daemon source — sessions (Task 3) + `stream_agent_events` (Task 4 §5) ✓; resolve stream-mode prompt-delivery (Task 6 §1 — resolved: launch-body prompt) ✓; DESIGN Risk 2 "cli_chat may not be bound to live sessions" — surfaced explicitly: CLI auto-binds on first event, daemon uses the new `LiveDaemon` binding (Task 1) ✓; `daemon_event_to_entry` reuse (Task 4) ✓.

**Placeholder scan:** the two `unimplemented!`/sketch blocks (Task 4 §5 spawn loop) are deliberate anchors to `ai_assistant/panel.rs:1233-1262` — the exact reference lines to replicate — because the warpui async-spawn primitive must match the codebase's existing, working pattern rather than a guessed API. Every pure/testable unit (message build, event→entry, binding, append, refresh, active predicate) has complete code + tests.

**Type consistency:** `ConversationBinding::LiveDaemon { session_id }` defined in Task 1, matched in Tasks 4/5. `ChatModel::{bind_daemon, append_entry, refresh_daemon_conversations}` defined in Tasks 1–3, called in Task 4. `build_daemon_message`/`drain_event_to_entry` defined + tested in Task 4, used in Task 4 §5/6. `AgentMessage { conversation_id, body }` and `CovenAgentEvent` variants match the cast_agent survey. `composer_is_active_for` defined + tested in Task 5.

**Risks flagged for the implementer:** (1) the async spawn/buffer/drain must mirror `ai_assistant` exactly — do not invent a warpui async primitive (Task 4 §5/6 say so). (2) `daemon_event_to_entry` visibility from `agent_panel` — if the fork-local boundary guard or module privacy objects, move it into `agent_transcript` (a clean home; depends only on `CovenAgentEvent` + `ChatEntry`) and re-point both panels. (3) session-per-turn: each submit launches a new daemon session; multi-turn context continuity is explicitly out of scope and should be called out in the 2c PR so reviewers don't expect conversational memory yet.
