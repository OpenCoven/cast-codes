# Phase 1 Implementation Plan — Shared `agent_transcript` + rich coven-code rendering

> **For agentic workers:** Implement task-by-task with TDD. Each step is one small action. Commit at each task boundary. Steps use checkbox (`- [ ]`) syntax. Verify each `cargo` command's expected result before moving on.

**Goal:** Make `coven-code` daemon sessions render with the chat panel's rich UI (tool cards, permission cards, assistant/user bubbles) by sharing one entry model + one set of views between the OSC-777 CLI backend and the Coven daemon backend — strictly additive, with a plain-text fallback.

**Architecture:** Extract `cli_chat`'s neutral render types + views into a new leaf module `app/src/agent_transcript/`. `cli_chat` keeps its `CLIAgentEvent → ChatEntry` adapter. `cast_agent` gains a structured `CovenAgentEvent` stream by launching the daemon session with `launchMode: "stream"` and parsing the harness stream-json. A new `daemon_event_to_entry` adapter maps `CovenAgentEvent → ChatEntry`; the `ai_assistant` panel renders coven-code through the shared views.

**Tech Stack:** Rust, `warpui` entity/view framework, `cargo nextest`, `serde_json`, the Coven daemon Unix-socket API (`~/.coven/coven.sock`, `/api/v1/*`).

**Spec:** `specs/castcodes-coven-agent-transcript/DESIGN.md` + `SPIKE-daemon-structured-events.md`.

**Pre-flight (every task):** run on a feature branch; commits signed (`git commit -S`); before pushing run `./script/check_ai_attribution`, `./script/check_rebrand`, `./script/check_cli_chat_boundary`, and `cargo clippy -p warp-app --features cast-agent -- -D warnings`.

---

## File structure

| Path | Responsibility | Change |
|---|---|---|
| `app/src/agent_transcript/mod.rs` | Neutral module root; re-exports | Create |
| `app/src/agent_transcript/entry.rs` | `ChatEntry`, `ChatEntryKind`, `InfoKind`, `StopReason` (no backend deps) | Create (moved from `cli_chat/entry.rs`) |
| `app/src/agent_transcript/source.rs` | `TranscriptSource` trait | Create |
| `app/src/agent_transcript/view/` | `message_bubble`, `tool_call_card`, `permission_card`, `info_bar`, `transcript` | Create (moved from `cli_chat/view/`) |
| `app/src/cli_chat/entry.rs` | CLI adapter: `impl ChatEntry { from_event }` over the moved type | Modify (types removed, adapter kept) |
| `app/src/cli_chat/**` | Import the moved types/views from `agent_transcript` | Modify (import fixups only) |
| `crates/cast_agent/src/stream_json.rs` | `CovenAgentEvent` + stream-json line parser (pure) | Create |
| `crates/cast_agent/src/gateway.rs` | `launchMode: "stream"` launch + structured stream method | Modify |
| `crates/cast_agent/src/lib.rs` | Export `CovenAgentEvent`, parser | Modify |
| `app/src/ai_assistant/coven_entry.rs` | `daemon_event_to_entry(CovenAgentEvent) -> Option<ChatEntry>` | Create |
| `app/src/ai_assistant/panel.rs` | Render coven-code stream via `agent_transcript` views; plain-text fallback | Modify |
| `script/check_cli_chat_boundary` | Extend to also cover `app/src/agent_transcript` | Modify |

---

## Task 1: Extract neutral render types into `agent_transcript`

**Files:**
- Create: `app/src/agent_transcript/mod.rs`, `app/src/agent_transcript/entry.rs`
- Modify: `app/src/cli_chat/entry.rs`, `app/src/lib.rs` (register `mod agent_transcript;`), `app/src/cli_chat/mod.rs`
- Test: existing `app/src/cli_chat/entry_tests.rs` must pass unchanged

- [ ] **Step 1: Create the module and move the neutral types.**
  Create `app/src/agent_transcript/entry.rs` containing **exactly** the `ChatEntry`, `ChatEntryKind`, `InfoKind`, and `StopReason` definitions currently in `app/src/cli_chat/entry.rs` (the `struct ChatEntry`, the `enum ChatEntryKind`, `enum InfoKind`, `enum StopReason` — the derive attributes and serde tags verbatim). Do **not** move the `impl ChatEntry { fn from_event }` block or the `use crate::terminal::cli_agent_sessions::…` import — those stay in `cli_chat`. Drop the now-unused `use chrono::{DateTime, Utc};`? Keep it — `ChatEntry` uses `DateTime<Utc>`.

- [ ] **Step 2: Create `agent_transcript/mod.rs`.**

```rust
//! Backend-agnostic agent transcript: the render model (`ChatEntry`) and the
//! rich views that display it. Fed by adapters that live on the backend side
//! (`cli_chat` for OSC-777 CLI agents; `ai_assistant`/`cast_agent` for the
//! Coven daemon). This module must not depend on any backend or transport —
//! it is a leaf render module (enforced by `script/check_cli_chat_boundary`).

pub mod entry;
pub mod source;
pub mod view;

pub use entry::{ChatEntry, ChatEntryKind, InfoKind, StopReason};
```

- [ ] **Step 3: Register the module.** In `app/src/lib.rs`, add `mod agent_transcript;` next to the existing `mod cli_chat;` declaration.

- [ ] **Step 4: Repoint `cli_chat`'s adapter at the moved types.** In `app/src/cli_chat/entry.rs`, delete the moved type definitions, keep the `use crate::terminal::cli_agent_sessions::event::{CLIAgentEvent, CLIAgentEventType};` import, add `use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind, InfoKind, StopReason};`, and keep the `impl ChatEntry { pub fn from_event(…) }` block as-is (inherent impls on a same-crate type from another module are legal). Re-export for existing callers: add `pub use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind, InfoKind, StopReason};` in `cli_chat/mod.rs` so `crate::cli_chat::entry::ChatEntry` paths keep resolving.

- [ ] **Step 5: Run cli_chat's tests — they must pass unchanged (proves behavior-preserving).**

Run: `cargo nextest run -p warp-app -E 'test(cli_chat::entry)' --no-fail-fast`
Expected: PASS (same tests as before the move).

- [ ] **Step 6: Compile-check the feature build.**

Run: `cargo check -p warp-app --features cast-agent`
Expected: `Finished` with no errors.

- [ ] **Step 7: Commit.**

```bash
git add app/src/agent_transcript/ app/src/cli_chat/entry.rs app/src/cli_chat/mod.rs app/src/lib.rs
git commit -S -m "refactor(agent_transcript): extract neutral ChatEntry types from cli_chat"
```

---

## Task 2: Move the rich view components into `agent_transcript`

**Files:**
- Create: `app/src/agent_transcript/view/{mod.rs,message_bubble.rs,tool_call_card.rs,permission_card.rs,info_bar.rs,transcript.rs}`
- Modify: `app/src/cli_chat/view/mod.rs` (re-export moved pieces), and any `cli_chat` importers

Only the **backend-neutral** views move: `message_bubble`, `tool_call_card`, `permission_card`, `info_bar`, and the pure `transcript` renderer. The panel-specific views (`composer`, `conversation_list`, `model_picker`, `empty_state`, `error_banner`, `settings_section`) stay in `cli_chat` — they know about CLI sessions/PTY.

- [ ] **Step 1: Move the neutral view files.**

```bash
git mv app/src/cli_chat/view/message_bubble.rs app/src/agent_transcript/view/message_bubble.rs
git mv app/src/cli_chat/view/tool_call_card.rs app/src/agent_transcript/view/tool_call_card.rs
git mv app/src/cli_chat/view/permission_card.rs app/src/agent_transcript/view/permission_card.rs
git mv app/src/cli_chat/view/info_bar.rs app/src/agent_transcript/view/info_bar.rs
```

- [ ] **Step 2: Move the transcript renderer, but keep the `cli_chat`-specific binding wrapper in `cli_chat`.** `cli_chat/view/transcript.rs` has two parts: `render_transcript(conv, …)`/`render_entry(entry, …)` (neutral — renders a `ChatEntry` list) and the `ConversationBinding`-aware wrapper (`ConversationBinding::Live/Past/None` → pick conversation). Move the neutral `render_transcript`/`render_entry` functions to `app/src/agent_transcript/view/transcript.rs`; leave the `ConversationBinding` wrapper in `cli_chat/view/transcript.rs` calling `agent_transcript::view::transcript::render_transcript`.

- [ ] **Step 3: Create `app/src/agent_transcript/view/mod.rs`.**

```rust
//! Rich transcript views over `ChatEntry`. No backend/session knowledge.
pub mod info_bar;
pub mod message_bubble;
pub mod permission_card;
pub mod tool_call_card;
pub mod transcript;
```

- [ ] **Step 4: Fix imports in the moved files.** In each moved view file, change `use crate::cli_chat::entry::…` → `use crate::agent_transcript::entry::…`, and `use super::…`/`use crate::cli_chat::view::…` cross-references to `crate::agent_transcript::view::…`. Remove any `use crate::cli_chat::…` that referenced CLI-session types — the neutral views only need `ChatEntry`/`ChatEntryKind`/`InfoKind`/`StopReason` and `warpui`/`Appearance` render types.

- [ ] **Step 5: Re-export from `cli_chat/view/mod.rs`** so existing `cli_chat` code keeps compiling: add `pub use crate::agent_transcript::view::{message_bubble, tool_call_card, permission_card, info_bar};` and update `cli_chat/view/mod.rs` to drop the moved `mod` lines.

- [ ] **Step 6: Compile + run the full cli_chat suite (regression gate).**

Run: `cargo nextest run -p warp-app -E 'test(cli_chat)' --no-fail-fast`
Expected: PASS — all pre-existing `cli_chat` tests green (behavior-preserving move).

- [ ] **Step 7: Add a layout-safety test for the moved views.**
  Create `app/src/agent_transcript/view/view_tests.rs` with one "renders without panic" test per view, following the repo pattern (see `warp-ui-guidelines`):

```rust
#[test]
fn tool_call_card_lays_out() {
    use warpui::App;
    use warp::test_util::{terminal::initialize_app_for_terminal_view, add_window_with_terminal};
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let term = add_window_with_terminal(&mut app, None);
        term.update(&mut app, |_view, _ctx| {
            let entry = crate::agent_transcript::entry::ChatEntry {
                sequence: 0,
                created_at: chrono::Utc::now(),
                kind: crate::agent_transcript::entry::ChatEntryKind::ToolCall {
                    tool_name: "Read".into(),
                    input_preview: Some("file.rs".into()),
                },
            };
            // Build the element; layout happens on render. Must not panic.
            let _ = crate::agent_transcript::view::transcript::render_entry(&entry, /* font_family */ Default::default(), 13.0);
        });
    })
}
```
  Register `mod view_tests;` under `#[cfg(test)]` in `agent_transcript/view/mod.rs`. Add analogous tests for `message_bubble` (UserPrompt + AssistantResponse), `permission_card`, `info_bar`, and `Stop`.

- [ ] **Step 8: Run the layout tests.**

Run: `cargo nextest run -p warp-app -E 'test(agent_transcript::view)'`
Expected: PASS.

- [ ] **Step 9: Commit.**

```bash
git add app/src/agent_transcript/ app/src/cli_chat/view/
git commit -S -m "refactor(agent_transcript): move rich transcript views out of cli_chat"
```

---

## Task 3: Define the `TranscriptSource` seam + the CLI impl

**Files:** Create `app/src/agent_transcript/source.rs`; modify `app/src/cli_chat/model.rs`.

- [ ] **Step 1: Write the trait.**

```rust
//! `TranscriptSource`: a backend-agnostic producer of `ChatEntry`s for a
//! conversation. Phase 1 defines the seam and the CLI impl; Phase 2's unified
//! model consumes it. Adapters live on the backend side and depend on this
//! module, never the reverse.

use crate::agent_transcript::entry::ChatEntry;

/// Coarse lifecycle status a source reports for a conversation, independent of
/// any specific backend's status vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptStatus {
    InProgress,
    Idle,
    Blocked,
    Done,
    Failed,
}

/// A source of transcript entries for one conversation.
pub trait TranscriptSource {
    /// Stable identifier for the conversation this source feeds.
    fn conversation_id(&self) -> &str;
    /// Entries produced so far, in order.
    fn entries(&self) -> &[ChatEntry];
    /// Current coarse status.
    fn status(&self) -> TranscriptStatus;
}
```

- [ ] **Step 2: Register `pub mod source;`** in `agent_transcript/mod.rs` (already added in Task 1 Step 2) and `pub use source::{TranscriptSource, TranscriptStatus};`.

- [ ] **Step 3: Implement `TranscriptSource` for `cli_chat`'s `ChatConversation`.** In `cli_chat/conversation.rs`, add:

```rust
impl crate::agent_transcript::source::TranscriptSource for ChatConversation {
    fn conversation_id(&self) -> &str { &self.session_id }
    fn entries(&self) -> &[crate::agent_transcript::entry::ChatEntry] { &self.entries }
    fn status(&self) -> crate::agent_transcript::source::TranscriptStatus {
        use crate::agent_transcript::source::TranscriptStatus as T;
        match self.status {
            CLIAgentSessionStatus::InProgress => T::InProgress,
            CLIAgentSessionStatus::Blocked { .. } => T::Blocked,
            CLIAgentSessionStatus::Idle | CLIAgentSessionStatus::WaitingPermission => T::Idle,
            CLIAgentSessionStatus::Success => T::Done,
            CLIAgentSessionStatus::Stopped | CLIAgentSessionStatus::Failed => T::Failed,
        }
    }
}
```
  (Adjust the match arms to the actual `CLIAgentSessionStatus` variants; verify with `grep -n 'enum CLIAgentSessionStatus' -A12 app/src/terminal/cli_agent_sessions/mod.rs`.)

- [ ] **Step 4: Compile-check.** Run: `cargo check -p warp-app --features cast-agent` → `Finished`.

- [ ] **Step 5: Commit.**

```bash
git add app/src/agent_transcript/source.rs app/src/agent_transcript/mod.rs app/src/cli_chat/conversation.rs
git commit -S -m "feat(agent_transcript): add TranscriptSource seam + cli_chat impl"
```

---

## Task 4: `cast_agent` — `CovenAgentEvent` + stream-json parser (pure, TDD)

**Files:** Create `crates/cast_agent/src/stream_json.rs`; modify `crates/cast_agent/src/lib.rs`.

The daemon's stream-json lines are JSON objects tagged by `type` (`system.init`, `user`, `assistant`, `tool_result`, `result`). We parse each line into a `CovenAgentEvent`. Unknown/parse-failures are surfaced as a fallback text event so the panel never loses output.

- [ ] **Step 1: Write the failing test.** Create `crates/cast_agent/src/stream_json.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_assistant_text() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}"#;
        assert_eq!(
            parse_stream_json_line(line),
            CovenAgentEvent::AssistantDelta { text: "Hello".into() }
        );
    }

    #[test]
    fn parses_tool_call_from_assistant_tool_use() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file":"a.rs"}}]}}"#;
        match parse_stream_json_line(line) {
            CovenAgentEvent::ToolCall { name, .. } => assert_eq!(name, "Read"),
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn parses_tool_result() {
        let line = r#"{"type":"tool_result","name":"Read","output":"contents"}"#;
        match parse_stream_json_line(line) {
            CovenAgentEvent::ToolResult { name, output } => {
                assert_eq!(name, "Read");
                assert_eq!(output, "contents");
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn parses_result_as_done() {
        assert_eq!(
            parse_stream_json_line(r#"{"type":"result","subtype":"success"}"#),
            CovenAgentEvent::Done
        );
    }

    #[test]
    fn unparseable_line_becomes_fallback_text() {
        // Not JSON, or unknown shape → never dropped; surfaced as text.
        assert_eq!(
            parse_stream_json_line("not json"),
            CovenAgentEvent::AssistantDelta { text: "not json".into() }
        );
    }

    #[test]
    fn system_init_is_ignored() {
        assert_eq!(parse_stream_json_line(r#"{"type":"system","subtype":"init"}"#), CovenAgentEvent::Ignored);
    }
}
```

- [ ] **Step 2: Run it — verify it fails to compile (types absent).**

Run: `cargo test -p cast_agent --lib stream_json 2>&1 | head`
Expected: FAIL — `cannot find type CovenAgentEvent` / `function parse_stream_json_line`.

- [ ] **Step 3: Implement the enum + parser above the test module.**

```rust
//! Parser for the harness stream-json emitted by a daemon session launched
//! with `launchMode: "stream"`. Lines are JSON objects tagged by `type`
//! (`system.init` / `user` / `assistant` / `tool_result` / `result`). Any
//! line we can't map is surfaced as assistant text so no output is lost.

use serde_json::Value;

/// A structured event decoded from one stream-json line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CovenAgentEvent {
    /// A chunk of assistant text (coalesced downstream into one bubble).
    AssistantDelta { text: String },
    /// The harness invoked a tool.
    ToolCall { name: String, input_preview: Option<String> },
    /// A tool returned output.
    ToolResult { name: String, output: String },
    /// The harness is requesting permission for an action.
    PermissionRequest { summary: String },
    /// Terminal success/stop.
    Done,
    /// Terminal failure with a message.
    Error { message: String },
    /// A line we intentionally drop (e.g. `system.init`).
    Ignored,
}

/// Parse a single stream-json line. Never returns an error — unparseable or
/// unknown-shaped lines become `AssistantDelta` with the raw text, so the
/// panel degrades to plain text rather than dropping output.
pub fn parse_stream_json_line(line: &str) -> CovenAgentEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return CovenAgentEvent::Ignored;
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return CovenAgentEvent::AssistantDelta { text: line.to_string() };
    };
    match v.get("type").and_then(Value::as_str) {
        Some("system") => CovenAgentEvent::Ignored,
        Some("user") => CovenAgentEvent::Ignored, // echoed input; the panel already shows the user's prompt
        Some("assistant") => assistant_event(&v),
        Some("tool_result") => CovenAgentEvent::ToolResult {
            name: v.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
            output: content_text(&v).unwrap_or_default(),
        },
        Some("result") => match v.get("subtype").and_then(Value::as_str) {
            Some("error") | Some("error_max_turns") => CovenAgentEvent::Error {
                message: v.get("error").and_then(Value::as_str).unwrap_or("run failed").to_string(),
            },
            _ => CovenAgentEvent::Done,
        },
        _ => CovenAgentEvent::AssistantDelta { text: line.to_string() },
    }
}

/// An `assistant` line carries a `message.content` array whose items are
/// `{type:"text",text}` or `{type:"tool_use",name,input}`. We surface the
/// first actionable item; multi-item lines are rare and the fallback keeps
/// text visible.
fn assistant_event(v: &Value) -> CovenAgentEvent {
    let content = v.get("message").and_then(|m| m.get("content")).and_then(Value::as_array);
    if let Some(items) = content {
        for item in items {
            match item.get("type").and_then(Value::as_str) {
                Some("tool_use") => {
                    return CovenAgentEvent::ToolCall {
                        name: item.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
                        input_preview: item.get("input").map(|i| {
                            let s = i.to_string();
                            s.chars().take(200).collect()
                        }),
                    };
                }
                Some("text") => {
                    if let Some(t) = item.get("text").and_then(Value::as_str) {
                        return CovenAgentEvent::AssistantDelta { text: t.to_string() };
                    }
                }
                _ => {}
            }
        }
    }
    // Some harnesses put text directly on `text`.
    if let Some(t) = v.get("text").and_then(Value::as_str) {
        return CovenAgentEvent::AssistantDelta { text: t.to_string() };
    }
    CovenAgentEvent::Ignored
}

fn content_text(v: &Value) -> Option<String> {
    if let Some(s) = v.get("output").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    if let Some(s) = v.get("content").and_then(Value::as_str) {
        return Some(s.to_string());
    }
    None
}
```

- [ ] **Step 4: Run the tests — verify they pass.**

Run: `cargo test -p cast_agent --lib stream_json`
Expected: PASS (7 tests). Adjust the exact JSON field names if the first live-session test in Task 5 reveals a different wire shape — the parser's job is documented by these tests.

- [ ] **Step 5: Export.** In `crates/cast_agent/src/lib.rs`: `mod stream_json;` and `pub use stream_json::{parse_stream_json_line, CovenAgentEvent};`.

- [ ] **Step 6: Commit.**

```bash
git add crates/cast_agent/src/stream_json.rs crates/cast_agent/src/lib.rs
git commit -S -m "feat(cast_agent): stream-json line parser -> CovenAgentEvent"
```

---

## Task 5: `cast_agent` — launch with `launchMode: "stream"` and stream structured events

**Files:** Modify `crates/cast_agent/src/gateway.rs`.

The spike's one open detail — how the prompt is delivered in stream mode — is resolved here first, with a controlled probe, then implemented.

- [ ] **Step 1: Resolve the prompt-delivery wire detail (one controlled probe).** Against the live daemon on the dev machine, launch a trivial stream-mode coven-code session and observe how the prompt must be delivered and how output events arrive:

```bash
# Trivial, low-cost prompt. Observe whether the positional prompt is honored,
# or whether a stream-json user frame on stdin is required, and what the
# recorded /api/v1/events look like (JSONL vs PTY).
coven run coven-code --stream-json --archive "reply with the single word: pong"
```
  Record in `SPIKE-daemon-structured-events.md` the confirmed launch body: whether `POST /api/v1/sessions` with `launchMode:"stream"` + `prompt` injects the first user frame, or whether a follow-up `POST /api/v1/sessions/:id/input` (stream-json user frame) is required. **Everything below assumes the former (prompt injected at launch); if the probe shows the latter, add the input POST in Step 3.**

- [ ] **Step 2: Add a structured launch path.** In `gateway.rs`, add a sibling to `launch_daemon_session` that sends `launchMode: "stream"`:

```rust
/// Launch a coven-code session in stream mode so the harness emits
/// structured stream-json. Reuses the prompt/projectRoot/harness resolution
/// of `launch_daemon_session` but sets `launchMode: "stream"`.
#[cfg(unix)]
async fn launch_daemon_session_stream(
    &self,
    socket: &std::path::Path,
    msg: &AgentMessage,
) -> anyhow::Result<LaunchedDaemonSession> {
    // Identical resolution to launch_daemon_session (extract a shared
    // `resolve_launch_params(&msg) -> (prompt, project_root, harness, title)`
    // helper and call it from both to stay DRY), then:
    let launch_body = serde_json::json!({
        "projectRoot": project_root,
        "harness": harness,
        "prompt": prompt,
        "launchMode": "stream",
        "title": title,
    });
    // POST /api/v1/sessions exactly as launch_daemon_session does, returning
    // LaunchedDaemonSession { session_id, harness, project_root }.
}
```
  Refactor the shared param resolution out of `launch_daemon_session` into `resolve_launch_params` so both launchers use it (DRY).

- [ ] **Step 3: Add `stream_agent_events` that yields `CovenAgentEvent`s.** Model it on the existing `stream_messages_via_daemon` `unfold`, but decode each `output` event's `data` line-by-line via `parse_stream_json_line`, yielding `CovenAgentEvent`s instead of `MessageChunk`:

```rust
/// Stream structured `CovenAgentEvent`s from a stream-mode coven-code session.
/// Same incremental /api/v1/events poll loop as `stream_messages_via_daemon`,
/// but each output line is parsed as stream-json. Emits `Done` on terminal
/// status. On any transport error the stream ends with `Error`.
#[cfg(unix)]
pub async fn stream_agent_events(
    &self,
    msg: AgentMessage,
) -> anyhow::Result<Pin<Box<dyn Stream<Item = CovenAgentEvent> + Send>>> {
    let Transport::Unix { socket } = &self.transport else {
        anyhow::bail!("stream_agent_events requires the Unix daemon transport");
    };
    let launched = self.launch_daemon_session_stream(socket, &msg).await?;
    // Reuse drain_output_deltas' event fetch, but instead of building Delta
    // chunks, split each output `data` on '\n' and parse each line:
    //   for line in data.split('\n') { events.push(parse_stream_json_line(line)) }
    // Filter out CovenAgentEvent::Ignored. Coalesce is done in the panel.
    // ... unfold state identical to DaemonStreamState; yield CovenAgentEvent ...
}
```
  Reuse `drain_output_deltas`' HTTP fetch by extracting a `fetch_output_lines(socket, session_id, after_seq, timeout) -> Result<(Vec<String>, u64)>` helper shared by both the plain and structured paths (DRY).

- [ ] **Step 4: Extend the daemon stub test** in `crates/cast_agent/tests/daemon_streaming.rs` (new test fn) to emit stream-json `output` events and assert the structured stream:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn streams_structured_coven_agent_events() {
    // Stub emits, as `output` event `data`, these JSONL lines:
    //   {"type":"system","subtype":"init"}
    //   {"type":"assistant","message":{"content":[{"type":"text","text":"Hi "}]}}
    //   {"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"f":"a"}}]}}
    //   {"type":"tool_result","name":"Read","output":"ok"}
    //   {"type":"result","subtype":"success"}
    // Assert collected events (minus Ignored) == [AssistantDelta("Hi "), ToolCall{Read}, ToolResult{Read,"ok"}, Done].
}
```
  Build the stub the same way as `streams_daemon_output_incrementally_until_done` (drain the POST body — see the fix in that file), returning the JSONL as `output` `data`.

- [ ] **Step 5: Run the cast_agent suite.**

Run: `cargo nextest run -p cast_agent`
Expected: PASS (existing + the two new tests). Then `cargo clippy -p cast_agent --all-targets -- -D warnings` → clean; `cargo fmt -p cast_agent`.

- [ ] **Step 6: Commit.**

```bash
git add crates/cast_agent/src/gateway.rs crates/cast_agent/tests/daemon_streaming.rs
git commit -S -m "feat(cast_agent): stream structured CovenAgentEvents via launchMode:stream"
```

---

## Task 6: `ai_assistant` — `daemon_event_to_entry` adapter (TDD)

**Files:** Create `app/src/ai_assistant/coven_entry.rs`; modify `app/src/ai_assistant/mod.rs`.

- [ ] **Step 1: Write the failing test.** Create `app/src/ai_assistant/coven_entry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ::ai::cast_agent::CovenAgentEvent;
    use crate::agent_transcript::entry::ChatEntryKind;

    fn kind(ev: CovenAgentEvent) -> Option<ChatEntryKind> {
        daemon_event_to_entry(ev, 0, chrono::Utc::now()).map(|e| e.kind)
    }

    #[test]
    fn assistant_delta_becomes_assistant_response() {
        assert!(matches!(
            kind(CovenAgentEvent::AssistantDelta { text: "hi".into() }),
            Some(ChatEntryKind::AssistantResponse { text }) if text == "hi"
        ));
    }

    #[test]
    fn tool_call_becomes_tool_card() {
        assert!(matches!(
            kind(CovenAgentEvent::ToolCall { name: "Read".into(), input_preview: Some("a".into()) }),
            Some(ChatEntryKind::ToolCall { tool_name, .. }) if tool_name == "Read"
        ));
    }

    #[test]
    fn done_and_ignored_produce_no_entry() {
        assert!(kind(CovenAgentEvent::Done).is_some_or_stop()); // Stop entry; see impl
        assert!(kind(CovenAgentEvent::Ignored).is_none());
    }
}
```
  (Replace `is_some_or_stop()` with a concrete assertion once the impl decides `Done → ChatEntryKind::Stop`.)

- [ ] **Step 2: Run — verify fail.** Run: `cargo test -p warp-app --features cast-agent --lib ai_assistant::coven_entry 2>&1 | head` → FAIL (function absent).

- [ ] **Step 3: Implement the adapter.**

```rust
//! Maps `cast_agent::CovenAgentEvent`s to the neutral `ChatEntry` render model,
//! so coven-code daemon sessions render through the shared `agent_transcript`
//! views. This adapter lives on the backend side; `agent_transcript` never
//! depends on `CovenAgentEvent`.

use chrono::{DateTime, Utc};

use ::ai::cast_agent::CovenAgentEvent;
use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind, StopReason};

/// Convert one `CovenAgentEvent` into a `ChatEntry`. Returns `None` for events
/// that carry no displayable entry (`Ignored`).
pub fn daemon_event_to_entry(
    event: CovenAgentEvent,
    sequence: u64,
    now: DateTime<Utc>,
) -> Option<ChatEntry> {
    let kind = match event {
        CovenAgentEvent::AssistantDelta { text } => ChatEntryKind::AssistantResponse { text },
        CovenAgentEvent::ToolCall { name, input_preview } => ChatEntryKind::ToolCall {
            tool_name: name,
            input_preview,
        },
        CovenAgentEvent::ToolResult { name, output } => ChatEntryKind::ToolCall {
            // Rendered as a resolved tool card; Phase 2 may split call/result.
            tool_name: name,
            input_preview: Some(output.chars().take(200).collect()),
        },
        CovenAgentEvent::PermissionRequest { summary } => ChatEntryKind::PermissionRequest {
            summary,
            tool_name: None,
            tool_input_preview: None,
        },
        CovenAgentEvent::Done => ChatEntryKind::Stop { reason: StopReason::Normal, response: None },
        CovenAgentEvent::Error { message } => ChatEntryKind::Stop {
            reason: StopReason::Errored,
            response: Some(message),
        },
        CovenAgentEvent::Ignored => return None,
    };
    Some(ChatEntry { sequence, created_at: now, kind })
}
```
  Fix the Task 6 Step 1 `Done` assertion to `matches!(kind(CovenAgentEvent::Done), Some(ChatEntryKind::Stop { .. }))`.

- [ ] **Step 4: Register + run.** Add `#[cfg(feature = "cast-agent")] pub mod coven_entry;` in `ai_assistant/mod.rs`.
Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(ai_assistant::coven_entry)'` → PASS.

- [ ] **Step 5: Commit.**

```bash
git add app/src/ai_assistant/coven_entry.rs app/src/ai_assistant/mod.rs
git commit -S -m "feat(ai_assistant): daemon_event_to_entry adapter (CovenAgentEvent -> ChatEntry)"
```

---

## Task 7: `ai_assistant` panel — render coven-code richly via `agent_transcript`

**Files:** Modify `app/src/ai_assistant/panel.rs`.

Replace the plain-text `CovenStreamState.text` accumulation + `render_coven_stream_section` with a `Vec<ChatEntry>` fed by the structured stream, rendered via `agent_transcript::view::transcript::render_transcript`. Keep the plain-text path as the fallback when the structured stream errors early.

- [ ] **Step 1: Add an entries buffer to the stream state.** In `CovenStreamState`, add `entries: Vec<ChatEntry>` alongside the existing `text` (keep `text` for the fallback). Import `use crate::agent_transcript::entry::ChatEntry;` and `daemon_event_to_entry`.

- [ ] **Step 2: Route the primary request through the structured stream.** In `send_via_coven_gateway_with_prompt`, call `runtime.agent().stream_agent_events(msg)` (Task 5) instead of `stream_messages`. In the tokio task, for each `CovenAgentEvent`:
  - coalesce consecutive `AssistantDelta`s into the trailing `AssistantResponse` entry (if the last pending entry is `AssistantResponse`, append text; else push a new one);
  - otherwise push `daemon_event_to_entry(ev, seq, now)` into `pending_entries`.
  Drain into `state.entries` in `poll_coven_stream` (mirror the existing chunk-drain loop), calling `ctx.notify()`.

- [ ] **Step 3: Fallback.** If `stream_agent_events` returns `Err` (or the first event is `Error` before any `AssistantDelta`), fall back to the existing `send_message` plain-text path and render into `state.text` exactly as today. This preserves the strictly-additive guarantee.

- [ ] **Step 4: Render entries.** In `render_coven_stream_section`, when `state.entries` is non-empty render via `agent_transcript::view::transcript::render_transcript` over the entries; else fall back to the existing plain-`text` rendering. Preserve the `COVEN STREAM • LIVE` header and the history ring.

- [ ] **Step 5: Reset.** In `reset_coven_stream`, clear `entries` alongside `text`.

- [ ] **Step 6: Compile + clippy + a panel layout smoke test.**

Run: `cargo check -p warp-app --features cast-agent` → `Finished`
Run: `cargo clippy -p warp-app --features cast-agent -- -D warnings` → clean.
Add/extend the panel's existing layout test to push one `AssistantResponse` + one `ToolCall` entry and assert it renders without panic.

- [ ] **Step 7: Commit.**

```bash
git add app/src/ai_assistant/panel.rs
git commit -S -m "feat(ai_assistant): render coven-code richly via agent_transcript views"
```

---

## Task 8: Boundary guard, gates, docs

**Files:** Modify `script/check_cli_chat_boundary`; update `specs/castcodes-coven-agent-transcript/DESIGN.md` status; `AGENTS.md`/`CAST-AGENT.md` note if warranted.

- [ ] **Step 1: Extend the fork-boundary guard** to also scan the new neutral module. In `script/check_cli_chat_boundary`, change `TARGET='app/src/cli_chat'` to check both:

```bash
for TARGET in app/src/cli_chat app/src/agent_transcript; do
  [ -d "$TARGET" ] || continue
  if grep -rnE "$PATTERN" "$TARGET" --include='*.rs'; then
    echo "ERROR: $TARGET must not reference Warp-owned infrastructure."; exit 1
  fi
done
echo "cli_chat/agent_transcript boundary check: OK"
```

- [ ] **Step 2: Run all gates.**

Run: `./script/check_cli_chat_boundary` → OK
Run: `./script/check_ai_attribution` → passed
Run: `./script/check_rebrand` → passed
Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(agent_transcript) or test(cli_chat) or test(ai_assistant::coven_entry)'` → PASS
Run: `cargo nextest run -p cast_agent` → PASS

- [ ] **Step 3: Update the design doc status** to "Phase 1 implemented" and note the resolved prompt-delivery wire detail from Task 5 Step 1.

- [ ] **Step 4: Commit.**

```bash
git add script/check_cli_chat_boundary specs/castcodes-coven-agent-transcript/
git commit -S -m "chore(agent_transcript): extend fork-boundary guard + record Phase 1 completion"
```

---

## Done criteria

- Coven-code sessions render tool-call cards / permission cards / bubbles in the agent panel (verified by launching a stream-mode session against the local daemon, or by the daemon-stub integration test when no live daemon).
- `cli_chat` behaves identically (its full test suite passes unchanged).
- Non-macOS/non-`cast-agent` builds unaffected (the coven path is `#[cfg(feature = "cast-agent")]`; `agent_transcript` is backend-neutral and compiles everywhere).
- All gates green; every commit signed.
- No panel merge, persistence change, or backend routing — those are Phases 2–3.
