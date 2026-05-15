# CastCodes Chat Panel — Technical Spec

Companion to [PRODUCT.md](./PRODUCT.md). This document describes how the chat panel is built and integrated into the CastCodes codebase.

## Shape recap

From brainstorming: **Shape A** — the CLI owns the agent loop. CastCodes is a UI and orchestration layer over `claude` / `codex` subprocesses. The inherited Warp agent code in `crates/ai/` and `app/src/ai/` is not modified; it stays dormant in the OSS build. The chat panel is new code in a new crate plus a new app module.

## Crate layout

Add one new crate and one new app module. No edits to existing AI crates.

```
crates/
  cli_chat/                       # new
    Cargo.toml
    src/
      lib.rs                      # public re-exports
      model.rs                    # domain types (Conversation, ChatMessage, ToolCall, FileEdit, CliKind, Model)
      session.rs                  # CliSession trait, SessionEvent enum, state machine
      backend/
        mod.rs                    # backend registry / detection
        claude.rs                 # claude CLI adapter
        codex.rs                  # codex CLI adapter
        protocol.rs               # shared stream-JSON parsing helpers
      detect.rs                   # PATH and version detection per CLI
      persistence.rs              # sqlite schema + load/save
      summarize.rs                # one-shot summarization for carry-forward

app/src/
  cli_chat/                       # new
    mod.rs
    panel.rs                      # main panel view
    composer.rs                   # input editor
    transcript.rs                 # message list rendering
    message_bubble.rs             # assistant / user message rendering
    tool_call_card.rs             # collapsible tool-call/result card
    file_edit_card.rs             # diff preview rendering
    model_picker.rs               # CLI + model dropdowns
    session_list.rs               # sidebar of past conversations
    empty_state.rs                # no-CLI-installed view
    error_banner.rs               # inline auth / crash banners
    settings_section.rs           # registers settings rows
    integration.rs                # wires panel into workspace view
```

### Why a new crate (`crates/cli_chat`) instead of extending `crates/ai/`

`crates/ai/` is densely coupled to `warp_multi_agent_api` protobuf types (`api::FileContent`, `api::SkillReference`, `api::LifecycleEventType`, etc.). Reusing it would either (a) require dragging those types through new code, or (b) introduce a parallel set of types inside the same crate, which guarantees naming collisions and rebase pain. A new crate gives a clean compile boundary, an independent dependency footprint (notably no `warp_multi_agent_api`), and keeps the inherited agent code uncontaminated for eventual upstream merges.

## Domain model

```rust
pub enum CliKind { Claude, Codex }

pub struct Model {
    pub id: String,                 // e.g. "claude-opus-4-7"
    pub display_name: String,       // e.g. "Claude Opus 4.7"
    pub cli: CliKind,
    pub supports_tools: bool,
}

pub struct Conversation {
    pub id: ConversationId,         // uuid
    pub title: String,              // auto-generated from first user message
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_cli: CliKind,
    pub last_model: String,
    pub resume_token: Option<String>, // CLI-specific (claude: session uuid)
    pub messages: Vec<ChatMessage>,
}

pub struct ChatMessage {
    pub id: MessageId,
    pub role: Role,                 // User | Assistant | System
    pub content: Vec<MessageBlock>, // text, tool_call, tool_result, file_edit, error
    pub created_at: DateTime<Utc>,
}

pub enum MessageBlock {
    Text(String),
    ToolCall { id: String, name: String, input_summary: String, status: ToolStatus },
    ToolResult { call_id: String, kind: ToolResultKind, content: String, is_error: bool },
    FileEdit { path: PathBuf, diff: String, applied: bool },
    Error { kind: SessionErrorKind, message: String },
}
```

## Backend abstraction

```rust
pub trait CliSession: Send + 'static {
    /// Send a user message. Idempotent only when state is Idle.
    fn send(&mut self, user_text: &str) -> Result<()>;

    /// Cancel the current turn. No-op if not streaming.
    fn cancel(&mut self) -> Result<()>;

    /// Close the session, killing the subprocess if needed.
    fn close(self) -> Result<()>;

    fn supports_resume(&self) -> bool;
    fn resume_token(&self) -> Option<String>;
    fn model(&self) -> &str;
    fn cli_kind(&self) -> CliKind;
}

pub enum SessionEvent {
    Started { session_id: String, model: String },
    AssistantDelta { text: String },
    AssistantTurnEnd { stop_reason: StopReason },
    ToolCall { id: String, name: String, input_summary: String },
    ToolResult { call_id: String, kind: ToolResultKind, content: String, is_error: bool },
    FileEdit { path: PathBuf, diff: String, applied: bool },
    Error { kind: SessionErrorKind, message: String },
    Closed { exit_code: Option<i32> },
}
```

Sessions emit `SessionEvent` over a `tokio::sync::mpsc::UnboundedReceiver<SessionEvent>` (or a `flume::Receiver`, matching whatever async channel primitives the codebase already uses). The panel polls the receiver inside warpui's existing `ModelContext` task pattern.

### `ClaudeSession`

Spawn:

```
claude
  --output-format stream-json
  --input-format stream-json
  --verbose
  --model <model-id>
  [--resume <session-id>]
  [--cwd <workspace-root>]
```

Stdin receives newline-delimited JSON of the form `{"type":"user","message":{"role":"user","content":"..."}}`. Stdout emits newline-delimited JSON events: `system` (init), `assistant` (deltas with content blocks), `user` (tool results), `result` (turn complete, with stop reason and usage).

Parsing strategy: a small enum mirrors the documented stream-json event shape. Unknown event types are logged and ignored (forward-compatible). The CLI's reported `session_id` is stored as the resume token; on the next send to the same conversation, `--resume <id>` is appended.

Cancellation: write `\n` followed by closing stdin's write half, or use the documented `claude /cancel` slash command if the running CLI version supports it. Detection at startup chooses the strategy.

### `CodexSession`

Codex CLI's streaming protocol is less stable than claude's. Implementation captures:
- Invocation flags (`codex chat --json` or whatever the version-detected variant is).
- A pluggable line parser that handles the version-specific event names.
- A failure path that surfaces "Unsupported codex version: <version>" rather than silently mis-parsing.

If codex's protocol is too unstable for v1, the codex backend lands behind a `cli_chat_codex` cargo feature flag (default off) and only claude ships in the first PR. PRODUCT.md notes this as a possible scope adjustment.

## Session state machine

```
Idle ──send──▶ Spawning ──ready──▶ Streaming ──turn-end──▶ Idle
                  │                    │
                  │                    ├──cancel──▶ Cancelling ──▶ Idle
                  │                    │
                  │                    └──crash───▶ Errored
                  │
                  └──spawn-fail──▶ Errored

Errored ──restart──▶ Spawning
Errored ──close────▶ Closed
Idle    ──close────▶ Closed
```

The panel guards user actions against the current state: `send` only from Idle, `cancel` only from Streaming, etc.

## Persistence

Reuse the existing `crates/persistence/` crate's sqlite plumbing. New tables, additive migration:

```sql
CREATE TABLE cli_chat_conversation (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    last_cli        TEXT NOT NULL,
    last_model      TEXT NOT NULL,
    resume_token    TEXT
);

CREATE TABLE cli_chat_message (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES cli_chat_conversation(id) ON DELETE CASCADE,
    role            TEXT NOT NULL,                        -- user | assistant | system
    content_json    TEXT NOT NULL,                        -- serialized Vec<MessageBlock>
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_cli_chat_message_conv ON cli_chat_message(conversation_id, created_at);
```

`MessageBlock` is serialized as JSON to avoid table proliferation. Tool calls, file edits, and errors all live inside the content JSON.

Storage path: `${cast_codes_data_dir}/cli_chat.sqlite`. The data dir resolution uses the same logic as the rest of the app (see `crates/warp_files/` and the public `.cast-codes` config dir rule from CASTCODES.md).

## UI integration

### Where it lives

A dedicated sidebar tab next to the existing terminal pane group, mirroring (but separate from) the inherited `app/src/workspace/view/conversation_list/`. Justification: the inherited surface is dormant in OSS; building a parallel sidebar avoids tangled coupling and keeps the rebrand-guard scope tight.

Integration touchpoints in existing code (small, additive):

- `app/src/workspace/view.rs` — register the new sidebar slot.
- `app/src/app_menus.rs` — add a "Toggle CLI Chat panel" menu item and keybinding.
- `app/src/settings_view/` — register the settings section.

### Rendering

warpui (the existing UI framework in `crates/warpui/`) is the rendering layer. The panel views follow the same `Entity` / `ModelContext` / `Render` patterns used throughout `app/src/`. No new UI framework. The `warp-ui-guidelines` skill is consulted before each view file is written.

### Streaming updates

Each `SessionEvent` received from the CLI session is dispatched to the conversation's `Entity<Conversation>` in warpui, which updates the in-memory `Vec<MessageBlock>` for the current assistant message and emits a notify. The transcript view subscribes and re-renders the affected message bubble. Assistant text deltas append to the last `MessageBlock::Text`; tool calls insert new blocks.

## Cross-cutting concerns

### Feature gating

A new cargo feature `cli_chat` on the `warp` binary crate gates the new module. Default-on for OSS builds. The inherited hosted agent UI stays unfeatured (always compiled) for now; a follow-up may add an opposing feature to gate it out of OSS.

### Rebrand guard

Every user-visible string is added to the rebrand allowlist or written to pass `./script/check_rebrand` from the start. Strings live in a `cli_chat/strings.rs` constants module to make audit trivial.

### Fork-local boundary verification

No new dependency on `warp_multi_agent_api`, `warp_server_client`, or any module under the cloud-services path. CI grep guard added as part of the PR:

```bash
! grep -rn "warp_multi_agent_api\|warp_server_client\|app\.warp\.dev\|api\.warp\.dev" \
    crates/cli_chat/src/ app/src/cli_chat/
```

### Telemetry

None emitted from `cli_chat` code. Local `tracing`/`log` debug spans only. Verified by the same CI grep guard pattern (`! grep -rn "rudderstack\|sentry\|telemetry::emit" crates/cli_chat/src/ app/src/cli_chat/`).

## Error handling

| Failure | UX | Code path |
|---|---|---|
| CLI binary not on PATH | Empty state with install commands | `detect.rs` returns `Status::NotInstalled` |
| CLI binary present, version unsupported | Empty state with "Unsupported version" + detected version | `detect.rs` returns `Status::UnsupportedVersion` |
| Spawn fails (permission, EBUSY, etc.) | Error banner; "Retry" button | `Spawning ──spawn-fail──▶ Errored` |
| CLI reports auth error on first message | Inline error message with CLI's stderr + login hint | `SessionEvent::Error { kind: NotAuthenticated }` |
| Subprocess exits mid-turn | Transcript notes "Session ended unexpectedly"; "Restart session" button | `SessionEvent::Closed` with non-zero exit |
| Protocol parse error | Skip event, log; if >N consecutive errors, surface "CLI version may be incompatible" | `protocol.rs` parser with bounded error counter |
| Resume token rejected | Fall back to summarize-and-continue flow | `summarize.rs` |

## Testing

### Unit

- `protocol.rs`: golden-file tests of stream-JSON parsing using captured `claude --print --output-format stream-json` fixtures committed under `crates/cli_chat/tests/fixtures/claude/`. Captured fixtures rather than mocked because the protocol is the source of truth and we want regressions to surface.
- `session.rs`: state-machine transition tests with a mock `Subprocess`.
- `persistence.rs`: round-trip tests for `Conversation` save/load on a tmp sqlite db.

### Integration

- `app/src/cli_chat/`: warpui-level tests with a `MockCliSession` driver pushing scripted `SessionEvent` streams and asserting the rendered transcript matches expectations. Pattern mirrors existing `app/src/integration_testing/agent_mode/`.
- One end-to-end test that spawns the real `claude` CLI with a synthetic prompt, asserting at least one `AssistantDelta` arrives. Gated behind `#[cfg_attr(not(claude_cli_available), ignore)]`; CI sets the cfg when the binary is on PATH.

### Manual verification checklist

Captured in a CHECKLIST.md alongside this spec before merge:

- Fresh install, no `claude` on PATH: panel shows empty state correctly.
- Logged-out `claude`: send fails with auth message + hint.
- Logged-in `claude`, send "echo hello" type prompt: streaming text renders.
- Prompt that triggers a tool call (file read): tool card appears.
- Prompt that triggers a file edit: diff card appears with correct path and patch.
- Stop button mid-stream: cancellation visible in transcript; composer re-enables.
- App restart with prior conversation: conversation listed; opens with transcript intact; next send resumes via `--resume`.
- Model switch with carry-forward: new session preamble contains summary of prior turn.
- `./script/check_rebrand` passes.
- `cargo check -p warp --bin cast-codes --features gui,cli_chat` passes.

## Sequencing (high-level phases — full plan in writing-plans output)

1. **Skeleton**: new crate, `CliSession` trait, no-op backend, panel renders empty state. Compiles, tests pass.
2. **Claude backend, no persistence**: send + receive streaming text only (no tool calls yet) end-to-end against a real `claude` CLI.
3. **Tool calls and file edits rendering**: parse tool_use / tool_result / edit events, render cards.
4. **Persistence**: sqlite tables, conversation list, restore-on-open.
5. **Resume + model switching**: `--resume`, carry-forward summary, model picker behavior.
6. **Codex backend** (or behind a feature flag if protocol stability is poor).
7. **Empty / error / unauthenticated state polish + settings page integration.**
8. **Rebrand guard pass, CI grep guards, manual checklist sweep.**

## Out of scope follow-ups (Shape C and beyond)

- Direct-provider BYOK backend (Anthropic / OpenAI / Google / OpenRouter HTTPS) using the same `CliSession` trait renamed to `ChatBackend`. The trait is named with this rename in mind from day one — internally we still call it `CliSession` in v1 to avoid pretending we have more abstraction than we do.
- OAuth backend (Claude Pro/Max direct OAuth, like Claude Code uses).
- Terminal block context attachments.
- Voice input / output.
- Multi-tab / parallel sessions.
- Gemini, aider, opencode backends.
- Compile-gating the inherited hosted-agent UI out of OSS builds.

## Risks

- **Stream-JSON protocol drift**: Claude Code's stream-json is documented but explicitly marked as evolving. Mitigation: version detection, captured-fixture tests, forward-compatible "skip unknown" parsing.
- **Codex protocol instability**: Mitigated by feature-flagging codex if needed.
- **Windows subprocess + PTY**: tokio's process API works on Windows but CLI tools may have differing behavior. Mitigation: end-to-end test runs on Windows CI before claiming Windows support; ship with macOS/Linux only if Windows is rough.
- **Cargo build time**: new crate adds compile time. Mitigation: keep dependency footprint small (no heavy async runtimes beyond what the workspace already uses).
- **Inherited `ai/` crate still references things we may need to touch later**: out of v1 scope, but worth a brief audit after step 1 to confirm nothing in the new code accidentally pulls it.
- **Model capability metadata staleness**: hand-curated per-CLI model list will drift. Mitigation: fetch from `claude --list-models` if such a flag exists in the detected version; otherwise pin to a small known-good set and note the curation point in code.
