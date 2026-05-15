# CastCodes Chat Panel — Technical Spec

Companion to [PRODUCT.md](./PRODUCT.md). Describes how the chat panel is built on top of the existing `cli_agent_sessions/` infrastructure.

## What we are building on

A pre-implementation audit of `app/src/terminal/cli_agent_sessions/` and related modules established that CastCodes already has:

- A versioned OSC 777 event protocol (`crate::terminal::cli_agent_sessions::event`) with parsers for `SessionStart`, `PromptSubmit`, `ToolComplete`, `Stop`, `PermissionRequest`, `PermissionReplied`, `QuestionAsked`, `IdlePrompt`, and an `Unknown(String)` fallback.
- Per-CLI session handlers (`cli_agent_sessions::listener::CLIAgentSessionHandler`) that convert raw notifications into `CLIAgentEvent` and forward to the sessions model.
- A singleton `CLIAgentSessionsModel` keyed by `terminal_view_id` that tracks status, context, rich-input state, and per-session drafts. Public API includes `session(...)`, `register_listener(...)`, `update_from_event(...)`, `open_input(...)`, `close_input(...)`, `set_session(...)`, `set_draft(...)`, etc.
- A rich-input editor wired into the terminal Input view (`app/src/terminal/input/cli_agent.rs`) for sending follow-up prompts to a running CLI agent. Triggered via Ctrl-G, footer button, or auto-show.
- Vendor plugin installers for `claude`, `codex`, `gemini`, `opencode` in `cli_agent_sessions/plugin_manager/` — **with Warp-owned marketplace repo references that the chat panel must not consult**.

The chat panel is purely additive on top of this. It does not modify any of the above. It subscribes, persists, and renders.

## Module layout

No new top-level crate. The chat panel lives entirely under `app/src/cli_chat/`, with a thin internal persistence helper. The crate-level dependencies for the work are `chrono`, `serde`, `serde_json`, `uuid`, and `warpui` — all already in the workspace — plus `rusqlite`, which is **added to the workspace as part of this feature** (one new line in the root `Cargo.toml` and one new line in `app/Cargo.toml`). We use `rusqlite` rather than the workspace's existing `diesel`/`persistence` setup because `crates/persistence/Cargo.toml` itself depends on `warp_multi_agent_api`, which would violate the fork-local boundary if pulled into the new module.

```
app/src/cli_chat/
  mod.rs                          # public re-exports + module wiring
  model.rs                        # ChatModel: subscriber + state machine
  store.rs                        # sqlite-backed conversation persistence (rusqlite)
  store_schema.rs                 # CREATE TABLE statements + migration runner
  view.rs                         # ChatPanelView (Render impl)
  view/
    transcript.rs                 # transcript list rendering
    message_bubble.rs             # user/assistant text bubble
    tool_call_card.rs             # collapsible tool-call card
    permission_card.rs            # permission_request rendering
    info_bar.rs                   # idle_prompt / question_asked rendering
    composer.rs                   # input composer (delegates to rich input)
    conversation_list.rs          # sidebar list of past sessions
    empty_state.rs                # no-CLI / no-plugin / no-history states
    error_banner.rs               # protocol parse error notice
    model_picker.rs               # CLI + model dropdown + "New chat" button
    settings_section.rs           # registers settings rows
  paths.rs                        # database path resolution
  feature_flag.rs                 # CastCodesChatPanel feature flag wiring
```

Touchpoints in existing code:

- `app/src/lib.rs` — register the new module.
- `app/src/workspace/view.rs` — register the panel slot in the workspace shell (sidebar or split, decision below).
- `app/src/app_menus.rs` — add a "Toggle Chat Panel" menu item and a keybinding.
- `app/src/settings_view/ai_page.rs` — register the new settings section.
- `crates/warp_features/src/lib.rs` — add `CastCodesChatPanel` variant to the `FeatureFlag` enum.

## Architecture

### Data flow

```
terminal stdout
   │
   ▼  (OSC 777 with sentinel "warp://cli-agent")
CLIAgentSessionListener  ──parse──▶  CLIAgentEvent
   │
   ▼  (existing forwarding)
CLIAgentSessionsModel  ──emits──▶  CLIAgentSessionsModelEvent
   │
   ▼  (new: ChatModel subscribes)
ChatModel
   │
   ├──▶ ChatStore.insert_event(...)  (sqlite write)
   │
   └──▶ notify ChatPanelView         (re-render)
```

`ChatModel` is a `warpui::Entity` that observes `CLIAgentSessionsModel` events. For each forwarded event:

1. Resolve or create the local `ChatConversation` record keyed by `session_id`.
2. Append a typed `ChatEntry` (user message, assistant response, tool call, permission, info, stop) to the conversation's entry list.
3. Persist the event to sqlite via `ChatStore`.
4. Emit a `ChatModelEvent::ConversationUpdated` so views can re-render.

`ChatPanelView` renders the currently-bound `ChatConversation` (live or past). Composer input is forwarded into the existing rich-input flow rather than touching the terminal directly.

### Session binding

A panel is bound to one of:

- **Live**: A `session_id` that maps to an active `CLIAgentSession`. New events stream in.
- **Past**: A `session_id` whose CLIAgentSession is no longer active. Read-only view from sqlite.
- **None**: No conversation selected (initial state, empty-history state, panel just opened).

State transitions:

```
None ──first event for session X──▶ Live(X)
None ──user opens past──▶ Past(Y)
Live(X) ──CLI session ends──▶ Past(X)
Live(X) ──user opens past──▶ Past(Y)
Past(Y) ──user opens live──▶ Live(X)
Past(Y) ──user clicks "Continue in new terminal"──▶ (opens new terminal, becomes Live when session_start arrives for new session)
```

### Persistence

A single sqlite database at `${data_dir}/cli_chat.sqlite` (path resolved via the same helper that already places CastCodes-local config under `~/.cast-codes/`). One database, additive schema only.

```sql
CREATE TABLE IF NOT EXISTS chat_conversation (
    session_id        TEXT PRIMARY KEY,
    agent             TEXT NOT NULL,      -- "claude" | "codex" | "gemini" | "opencode"
    title             TEXT NOT NULL,
    cwd               TEXT,
    project           TEXT,
    created_at        INTEGER NOT NULL,   -- unix millis
    updated_at        INTEGER NOT NULL,
    status            TEXT NOT NULL,      -- "in_progress" | "success" | "blocked"
    last_model        TEXT
);

CREATE TABLE IF NOT EXISTS chat_entry (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id        TEXT NOT NULL REFERENCES chat_conversation(session_id) ON DELETE CASCADE,
    sequence          INTEGER NOT NULL,
    created_at        INTEGER NOT NULL,
    kind              TEXT NOT NULL,      -- "user" | "assistant" | "tool" | "permission" | "info" | "stop" | "raw"
    payload_json      TEXT NOT NULL,      -- serialized ChatEntry payload
    UNIQUE (session_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_chat_entry_session ON chat_entry(session_id, sequence);
CREATE INDEX IF NOT EXISTS idx_chat_conv_updated  ON chat_conversation(updated_at DESC);

CREATE TABLE IF NOT EXISTS chat_schema_version (
    version           INTEGER PRIMARY KEY
);
INSERT OR IGNORE INTO chat_schema_version (version) VALUES (1);
```

`payload_json` carries the typed entry data (text, tool name + input preview, permission summary, etc.) so the schema is forward-compatible: adding new entry kinds adds new variants, no schema migration.

Migrations are managed by a tiny home-grown runner in `store_schema.rs` that compares `chat_schema_version.version` against a compile-time constant and applies `MIGRATIONS[version..]` statements in order. We do not pull in `diesel_migrations` because the existing `crates/persistence/` is `warp_multi_agent_api`-coupled and we are explicitly not depending on it.

### Domain types

```rust
pub enum AgentKind { Claude, Codex, Gemini, OpenCode }

pub struct ChatConversation {
    pub session_id: String,
    pub agent: AgentKind,
    pub title: String,
    pub cwd: Option<String>,
    pub project: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: CLIAgentSessionStatus,    // re-exported from cli_agent_sessions
    pub last_model: Option<String>,
    pub entries: Vec<ChatEntry>,
}

pub struct ChatEntry {
    pub id: i64,
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
    pub kind: ChatEntryKind,
}

pub enum ChatEntryKind {
    UserPrompt { text: String },
    AssistantResponse { text: String },
    ToolCall {
        tool_name: String,
        input_preview: Option<String>,
    },
    PermissionRequest {
        summary: String,
        tool_name: Option<String>,
        tool_input_preview: Option<String>,
    },
    PermissionReplied { approved: bool, summary: Option<String> },
    Info { kind: InfoKind, summary: Option<String> },     // idle_prompt, question_asked
    Stop { reason: StopReason },
    Raw { event_type: String, payload_json: String },     // forward-compat for Unknown
}
```

Mapping `CLIAgentEvent → ChatEntryKind` lives in `model.rs` and is unit-tested with fixtures captured from real events.

### Composer wiring

The composer in `view/composer.rs` reuses, but does not duplicate, the existing rich-input flow:

```rust
// On submit:
let model = CLIAgentSessionsModel::as_ref(app);
let Some(terminal_view_id) = self.conversation.bound_terminal_view_id else { return };
model.update(app, |m, ctx| {
    m.open_input(terminal_view_id, CLIAgentInputEntrypoint::FooterButton, ctx);
    m.set_draft(terminal_view_id, self.composer_text.clone());
});
// Then dispatch the existing "submit rich input" action.
```

Reuse here is deliberate: it routes the panel's submit through the same path as Ctrl-G submits, which means stdin write semantics, draft-clearing, and telemetry-suppression are all consistent.

### Model picker / "New chat"

Launching a new CLI session is shell-out via the existing terminal infrastructure, not subprocess. Pseudocode:

```rust
let command = match agent {
    AgentKind::Claude => format!("claude --model {} ", model_id),
    AgentKind::Codex  => format!("codex chat --model {} ", model_id),
    AgentKind::Gemini => format!("gemini --model {} ", model_id),
    AgentKind::OpenCode => format!("opencode --model {} ", model_id),
};
workspace.open_new_terminal_with_command(command, ctx);
```

The new terminal pane is opened with the constructed command. Once the vendor plugin emits `session_start` for that terminal, the panel auto-binds (or, more conservatively, the panel surfaces a "Bind" affordance — decision noted below).

The curated model list per CLI lives in `app/src/cli_chat/model.rs` as a `const`-style table per agent. Initial set:

- Claude: `claude-opus-4-7`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`.
- Codex: `gpt-5-codex`, `o4-mini`, latest model the codex CLI advertises (refreshable; see Risks).
- Gemini: `gemini-2.5-pro`, `gemini-2.5-flash`.
- OpenCode: defaults pinned to whatever opencode's documented set is.

Model lists are static in v1 and refreshed by hand-editing this file. A follow-up can introduce `claude --list-models` style detection.

### Empty / error states

| State | UI |
|---|---|
| No supported CLI on PATH | `empty_state.rs` lists each supported CLI with the canonical install command and a vendor docs link. Composer disabled. |
| CLI on PATH but plugin missing | `empty_state.rs` variant: "To stream events into the chat panel, install the vendor plugin for `<agent>`. See vendor documentation." No auto-install button in OSS. Composer disabled. |
| CLI session active, but plugin emits malformed events | `error_banner.rs` displays "Plugin version may be incompatible — events were skipped." Bounded counter so one bad event doesn't spam. |
| Bound session's terminal closed | Transcript stays visible, marked "Session ended". Composer disabled. "Continue in new terminal" button enabled if `--resume` is supported. |
| Database open fails | Panel disables persistence-dependent features (conversation list, restore on open) and shows a one-line banner. Live transcript still works. |

### Plugin install handling — fork-local boundary

The inherited `plugin_manager/{claude,codex,gemini,opencode}.rs` modules contain code paths that consult Warp-owned marketplace URLs (`warpdotdev/claude-code-warp`, `warpdotdev/claude-code-warp-internal`, `warpdotdev/gemini-cli-warp`). The new chat panel does **not** call any function in those modules that touches those URLs.

Two safe entry points exist:

1. **`is_installed(&self) -> bool`** — local filesystem check (e.g., presence of `~/.claude/plugins/...`). Calling these from chat panel code is safe: no network or upstream URL.
2. **Version detection** — purely local. Safe.

We do not call `install()`, `update()`, or any function that consults `MARKETPLACE_REPO`. The empty-state UI explicitly points users to vendor documentation rather than executing an installer.

A CI grep check enforces this:

```bash
! grep -rn "MARKETPLACE_REPO\|PLATFORM_MARKETPLACE_REPO\|EXTENSION_REPO\|warpdotdev/" \
    app/src/cli_chat/
```

### Workspace placement

Two candidates considered:

- **Right-side panel slot** (mirroring `app/src/workspace/view/right_panel.rs` patterns). This is the chosen placement: it lives next to the active terminal pane, the user can keep both visible, and it does not contend with the existing left-side vertical-tabs sidebar where session chips already live.
- Top-tab in the pane group: rejected. Tabs there are for editors/terminals; a chat panel does not fit that mental model and would hide the terminal.

The panel is registered in `app/src/workspace/view.rs` similarly to how other right-panel content is registered. Showing/hiding is controlled by a workspace action (`ToggleCliChatPanel`) bound to a default keybinding of `Cmd+Shift+H` on macOS / `Ctrl+Shift+H` on Linux/Windows. (`Cmd+Shift+J`, the original placeholder in this doc, is already bound to `TerminalAction::ToggleQueueNextPrompt`.)

## Feature gating

Add a new `FeatureFlag` variant in `crates/warp_features/src/lib.rs`:

```rust
pub enum FeatureFlag {
    ...
    CastCodesChatPanel,
    ...
}
```

Default: on in OSS (`cast-codes` binary). The flag exists so we can roll back quickly without unwinding the worktree; longer-term it can be removed once stable.

No new cargo features. All code is unconditionally compiled.

## Testing strategy

### Unit

- `model.rs`: event-to-entry mapping tests for each `CLIAgentEventType` variant. Fixtures are JSON payloads matching what the existing `cli_agent_sessions/mod_tests.rs` already uses, so we share captured-event payloads where helpful.
- `store.rs`: round-trip tests on an in-memory sqlite (`":memory:"`). Insert conversation + entries, query, verify ordering, verify cascading delete.
- `store_schema.rs`: migration runner from empty DB → current version; idempotency (running twice is a no-op); detection of a DB at an unknown future version returns an explicit error (we don't downgrade).

### Integration

- `app/src/cli_chat/`: warpui-level tests with a fixture `CLIAgentSessionsModel` driven by scripted events. Assert the transcript view renders expected message types in order. Pattern modeled on existing `app/src/integration_testing/agent_mode/`.
- Composer submit test: posting via the composer results in a `set_draft` + `open_input` call on the sessions model; verified with a mock.
- Past-session view test: load a fixture conversation into a temp sqlite, open in panel, assert read-only state and disabled composer.

### Manual

A `specs/castcodes-chat-panel/CHECKLIST.md` captures the end-to-end manual verification: live transcript, restart, model picker new-chat, empty state, plugin-missing state, malformed event handling, rebrand guard pass.

## Sequencing (high-level — full plan in PLAN.md)

1. **Skeleton**: feature flag, panel module, empty state renders, workspace wiring.
2. **Subscribe to events**: `ChatModel` observes `CLIAgentSessionsModel`; events convert to in-memory entries; transcript renders live.
3. **Persistence**: sqlite store + schema + writes-on-event + reads-on-open. Round-trip tested.
4. **Conversation list**: sidebar lists past sessions; selecting opens read-only view; switching back to live works.
5. **Composer**: wired into existing rich-input submit flow.
6. **Model picker + new chat**: opens a new terminal with the chosen model.
7. **Empty / plugin-missing / error polish**.
8. **Settings section + rebrand + CI guards + manual checklist**.

## Risks

- **Event protocol drift**: vendor plugins control the OSC 777 payload shape. Mitigation: forward-compatible `Unknown(String)` handling already exists; the chat store has a `Raw` entry kind that preserves the JSON for forensic display.
- **`response` field absence**: the v1 protocol's `stop` event carries `response`, but it's optional. When absent, the transcript can only show the tool sequence; we surface "Turn complete" rather than fabricate an assistant message.
- **Multiple terminals running the same CLI**: `session_id` is the join key and is per-agent-session unique. Two terminals running `claude` simultaneously each get distinct sessions; the panel switches between them via the conversation list.
- **Codex's non-JSON notification format**: the listener has an agent-specific parser override for codex. The chat model treats whatever the listener forwards as authoritative; if a `tool_complete` arrives without rich fields, the entry is rendered minimally.
- **Database growth**: chat history is unbounded. Mitigation: include a settings toggle "Auto-delete chats older than 90 days" disabled by default in v1; add a "Delete all" affordance in settings. v1 is acceptable without pruning; this is documented.
- **Fork-local-boundary in the inherited plugin manager**: addressed above with a CI grep guard and an explicit no-go list of functions the panel may not call.
- **Keybinding collisions**: the default is `Cmd+Shift+H` / `Ctrl+Shift+H` (after `Cmd+Shift+J` was found to collide with `TerminalAction::ToggleQueueNextPrompt`); run the keybindings audit again before shipping to confirm no further collisions.
- **Inherited `ai/` types coupling**: `CLIAgentSessionStatus::to_conversation_status` references `crate::ai::agent::conversation::ConversationStatus`. We use the source enum directly (not the inherited mapping) and do not pull anything else from `app/src/ai/`.

## Out-of-scope follow-ups

- Direct-provider BYOK (Anthropic / OpenAI / Google / OpenRouter SDKs) as a second backend.
- OAuth flows for Claude Pro/Max etc.
- Inline-in-block chat presentation.
- Multi-session split view in the same panel.
- Auto-detection of plugin install status via running `claude --version` / `claude plugin list` (we currently inspect filesystem markers; vendor-CLI invocation is a later refinement).
- A non-Warp-pointing plugin distribution path (CastCodes-owned plugin fork).
- Pruning / archival policy for old conversations.
