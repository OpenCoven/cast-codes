# CastCodes Chat Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Read [`PRODUCT.md`](./PRODUCT.md) and [`TECH.md`](./TECH.md) before starting; this plan assumes both have been read.

**Goal:** Add a CastCodes Chat Panel that renders the transcript of an in-terminal CLI agent session (claude / codex / gemini / opencode) as a chat UI, persists events locally, and lets users continue past conversations — all without violating the fork-local OSS boundary.

**Architecture:** New `app/src/cli_chat/` module that subscribes to the existing `CLIAgentSessionsModel`, converts incoming `CLIAgentEvent`s into typed `ChatEntry`s, persists them to a local sqlite (`rusqlite`), and renders them in a right-panel chat view. The composer reuses the existing rich-input flow. No subprocess spawning, no stream-JSON parsing — both already exist in `cli_agent_sessions/`.

**Tech Stack:** Rust, warpui (in-house UI framework), `rusqlite` (new workspace dep), `chrono`, `serde`/`serde_json`, `uuid`, `tokio`.

---

## Files created or modified

**Created (new module `app/src/cli_chat/`):**

| Path | Responsibility |
|---|---|
| `app/src/cli_chat/mod.rs` | Module wiring + public re-exports |
| `app/src/cli_chat/model.rs` | `ChatModel` (warpui Entity) that subscribes to `CLIAgentSessionsModel` |
| `app/src/cli_chat/entry.rs` | `ChatEntry`, `ChatEntryKind`, conversion from `CLIAgentEvent` |
| `app/src/cli_chat/conversation.rs` | `ChatConversation`, `AgentKind`, binding state |
| `app/src/cli_chat/store.rs` | `ChatStore`: rusqlite wrapper, insert/load |
| `app/src/cli_chat/store_schema.rs` | `CREATE TABLE` statements + migration runner |
| `app/src/cli_chat/paths.rs` | Local data-dir resolution for `cli_chat.sqlite` |
| `app/src/cli_chat/view.rs` | `ChatPanelView` (warpui View) |
| `app/src/cli_chat/view/transcript.rs` | Transcript list renderer |
| `app/src/cli_chat/view/message_bubble.rs` | User/assistant message rendering |
| `app/src/cli_chat/view/tool_call_card.rs` | Collapsible tool-call card |
| `app/src/cli_chat/view/permission_card.rs` | Permission-request rendering |
| `app/src/cli_chat/view/info_bar.rs` | idle_prompt / question_asked thin bar |
| `app/src/cli_chat/view/composer.rs` | Input composer that delegates to rich input |
| `app/src/cli_chat/view/conversation_list.rs` | Sidebar list of past sessions |
| `app/src/cli_chat/view/empty_state.rs` | No-CLI / no-plugin / no-history states |
| `app/src/cli_chat/view/error_banner.rs` | Protocol parse error notice |
| `app/src/cli_chat/view/model_picker.rs` | CLI + model dropdown + "New chat" |
| `app/src/cli_chat/view/settings_section.rs` | Settings page rows |
| `app/src/cli_chat/strings.rs` | All user-visible strings (rebrand audit point) |
| `app/src/cli_chat/feature_flag.rs` | Wraps `FeatureFlag::CastCodesChatPanel` checks |
| `app/src/cli_chat/tests/fixtures/*.json` | Captured `CLIAgentEvent` payload fixtures |
| `specs/castcodes-chat-panel/CHECKLIST.md` | Manual verification checklist |

**Modified:**

| Path | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `rusqlite = { version = "0.32", features = ["bundled"] }` under `[workspace.dependencies]` |
| `app/Cargo.toml` | Add `rusqlite.workspace = true`, `uuid.workspace = true` if not already present |
| `app/src/lib.rs` | `pub mod cli_chat;` |
| `app/src/workspace/view.rs` | Register right-panel slot for `ChatPanelView` |
| `app/src/workspace/action.rs` | Add `ToggleCliChatPanel` action |
| `app/src/app_menus.rs` | Add "Toggle Chat Panel" menu item + keybinding |
| `app/src/settings_view/ai_page.rs` | Call into `cli_chat::view::settings_section` to register rows |
| `crates/warp_features/src/lib.rs` | Add `FeatureFlag::CastCodesChatPanel` variant; add to default-on list for OSS |

---

## Phase 1 — Skeleton

### Task 1.1: Add `rusqlite` to the workspace

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `app/Cargo.toml`

- [ ] **Step 1: Add to workspace deps**

In root `Cargo.toml`, locate the `[workspace.dependencies]` section (search for `diesel = { version = "2.3.8"`) and add a new line below it:

```toml
rusqlite = { version = "0.32", features = ["bundled", "chrono", "serde_json"] }
```

- [ ] **Step 2: Add to `app/Cargo.toml`**

In `app/Cargo.toml` under `[dependencies]`, add:

```toml
rusqlite.workspace = true
```

Confirm `uuid.workspace = true` and `chrono.workspace = true` are already present; if not, add them.

- [ ] **Step 3: Verify build**

Run: `cargo check -p warp-app --bin cast-codes --features gui`
Expected: PASS (no new code yet; this is just a dependency addition check).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml app/Cargo.toml Cargo.lock
git commit -m "deps: add rusqlite to workspace for cli_chat module"
```

### Task 1.2: Add `CastCodesChatPanel` feature flag

**Files:**
- Modify: `crates/warp_features/src/lib.rs`

- [ ] **Step 1: Read the existing `FeatureFlag` enum**

Read `crates/warp_features/src/lib.rs` lines 1–120 to confirm the enum layout and the registration arrays (`DEBUG_FLAGS`, `DOGFOOD_FLAGS`, etc.). Identify where the cardinality is computed (the `cardinality::<FeatureFlag>()` callers).

- [ ] **Step 2: Add the variant**

Add `CastCodesChatPanel` to `pub enum FeatureFlag { ... }` immediately above the closing brace. Match the casing convention used by neighboring variants.

- [ ] **Step 3: Add to default-on list for OSS**

The repo has channel-specific default-on lists (search for `STABLE_FLAGS` or the equivalent — read around `DOGFOOD_FLAGS` at line 893 for the pattern). Add `FeatureFlag::CastCodesChatPanel` to whichever default-on array covers the OSS / `cast-codes` channel. If unclear, leave it out of all of them in this commit (flag defaults to off) and revisit in Phase 8 after the panel renders.

- [ ] **Step 4: Verify build**

Run: `cargo check -p warp_features`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/warp_features/src/lib.rs
git commit -m "feat(features): add CastCodesChatPanel feature flag"
```

### Task 1.3: Scaffold the `cli_chat` module

**Files:**
- Create: `app/src/cli_chat/mod.rs`
- Create: `app/src/cli_chat/feature_flag.rs`
- Create: `app/src/cli_chat/strings.rs`
- Modify: `app/src/lib.rs`

- [ ] **Step 1: Create `app/src/cli_chat/mod.rs`**

```rust
//! CastCodes Chat Panel.
//!
//! Renders a chat-style transcript of in-terminal CLI agent sessions
//! (claude, codex, gemini, opencode). Subscribes to the existing
//! `CLIAgentSessionsModel`; persists events to a local sqlite store.
//!
//! See `specs/castcodes-chat-panel/PRODUCT.md` and `TECH.md`.

pub mod conversation;
pub mod entry;
pub mod feature_flag;
pub mod model;
pub mod paths;
pub mod store;
pub mod store_schema;
pub mod strings;
pub mod view;

pub use conversation::{AgentKind, ChatConversation, ConversationBinding};
pub use entry::{ChatEntry, ChatEntryKind};
pub use model::{ChatModel, ChatModelEvent};
pub use view::ChatPanelView;
```

Note: subsequent tasks create each submodule. This file will not compile until the submodules exist, so we will not run `cargo check` until Step 5.

- [ ] **Step 2: Create `app/src/cli_chat/feature_flag.rs`**

```rust
use crate::FeatureFlag;
use warpui::AppContext;

pub fn is_enabled(app: &AppContext) -> bool {
    FeatureFlag::CastCodesChatPanel.is_enabled(app)
}
```

If `FeatureFlag::is_enabled(app)` is not the actual method signature, read `crates/warp_features/src/lib.rs` to find the canonical accessor (look for callers of `FeatureFlag::AgentModeWorkflows`, `FeatureFlag::MultiWorkspace`, etc., to mirror their usage exactly).

- [ ] **Step 3: Create `app/src/cli_chat/strings.rs`**

```rust
//! User-visible strings for the chat panel.
//! Centralized so `./script/check_rebrand` only needs to audit one file.

pub const PANEL_TITLE: &str = "Chat";
pub const TOGGLE_MENU_ITEM: &str = "Toggle Chat Panel";

pub const EMPTY_NO_CLI_TITLE: &str = "No supported CLI detected";
pub const EMPTY_NO_CLI_BODY: &str =
    "Install one of the supported CLIs to start chatting. \
     The chat panel renders the conversation from a CLI session running in any CastCodes terminal.";

pub const EMPTY_NO_PLUGIN_TITLE: &str = "Plugin required";
pub const EMPTY_NO_PLUGIN_BODY: &str =
    "The chat panel renders structured events emitted by the vendor plugin for this CLI. \
     See the vendor's documentation to install the plugin.";

pub const EMPTY_NO_HISTORY_TITLE: &str = "No conversations yet";
pub const EMPTY_NO_HISTORY_BODY: &str =
    "Run a supported CLI in a terminal to start a conversation. It will appear here automatically.";

pub const COMPOSER_PLACEHOLDER_ACTIVE: &str = "Message the running CLI agent…";
pub const COMPOSER_PLACEHOLDER_INACTIVE: &str = "Run a CLI agent in a terminal to start chatting.";

pub const TRANSCRIPT_TURN_COMPLETE: &str = "Turn complete";
pub const TRANSCRIPT_SESSION_ENDED: &str = "Session ended";

pub const ERROR_INCOMPATIBLE_PLUGIN: &str =
    "Plugin version may be incompatible — some events were skipped.";
```

- [ ] **Step 4: Register the module in `app/src/lib.rs`**

Find an existing `pub mod` line near other top-level modules (e.g., `pub mod ai;`, `pub mod terminal;`) and add `pub mod cli_chat;` in alphabetical order.

- [ ] **Step 5: Create stub submodule files so the crate compiles**

For each of `conversation`, `entry`, `model`, `paths`, `store`, `store_schema`, `view`, create an empty file with just a single line containing `// stub` so the `mod.rs` references resolve. These are fleshed out in subsequent tasks.

```bash
for f in conversation entry model paths store store_schema view; do
  echo "// stub" > app/src/cli_chat/$f.rs
done
```

Wait — `view` is a directory, not a file. Replace the loop with:

```bash
for f in conversation entry model paths store store_schema; do
  echo "// stub" > app/src/cli_chat/$f.rs
done
mkdir -p app/src/cli_chat/view
echo "// stub" > app/src/cli_chat/view/mod.rs
```

And update `app/src/cli_chat/mod.rs` so its `pub mod view;` line resolves to `view/mod.rs` (Rust does this automatically).

The stub `view/mod.rs` should declare a minimal placeholder type to satisfy the `pub use view::ChatPanelView;` re-export:

```rust
// stub — replaced in Phase 2

pub struct ChatPanelView;
```

For the other stubs, the `mod.rs` re-exports demand actual types. Add minimal placeholders in each so the crate compiles. For `conversation.rs`:

```rust
// stub — replaced in Task 1.5

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind { Claude, Codex, Gemini, OpenCode }

#[derive(Debug, Clone)]
pub struct ChatConversation;

#[derive(Debug, Clone, Copy)]
pub enum ConversationBinding { None }
```

For `entry.rs`:

```rust
// stub — replaced in Task 2.1

#[derive(Debug, Clone)]
pub struct ChatEntry;

#[derive(Debug, Clone)]
pub enum ChatEntryKind { Placeholder }
```

For `model.rs`:

```rust
// stub — replaced in Task 2.2

#[derive(Debug, Clone)]
pub struct ChatModel;

#[derive(Debug, Clone)]
pub enum ChatModelEvent { Placeholder }
```

Other stubs (`paths.rs`, `store.rs`, `store_schema.rs`) can stay as `// stub` because nothing in `mod.rs` re-exports from them yet.

- [ ] **Step 6: Verify build**

Run: `cargo check -p warp-app --bin cast-codes --features gui`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src/cli_chat/ app/src/lib.rs
git commit -m "feat(cli_chat): scaffold module with stub submodules"
```

### Task 1.4: Wire workspace action + menu item

**Files:**
- Modify: `app/src/workspace/action.rs`
- Modify: `app/src/app_menus.rs`

- [ ] **Step 1: Read the existing action definitions**

Read `app/src/workspace/action.rs` lines 1–80 to find the `WorkspaceAction` enum (or equivalent action registry). Identify a similar binary-toggle action (e.g., something like `ToggleVerticalTabs` or `ToggleSidebar`) to mirror.

- [ ] **Step 2: Add the action variant**

Add `ToggleCliChatPanel` to the action enum. If the enum has associated metadata (e.g., a `display_name` method, a keybinding default), add the corresponding entries by mirroring an existing variant.

- [ ] **Step 3: Add the menu item**

Read `app/src/app_menus.rs` to find where existing toggle items are registered (search for `Toggle` in the file). Add a "Toggle Chat Panel" entry using `cli_chat::strings::TOGGLE_MENU_ITEM` as the label, bound to `WorkspaceAction::ToggleCliChatPanel`.

- [ ] **Step 4: Add the default keybinding**

Search the keymap files (`crates/warpui/.../keymap*` or `app/src/util/bindings.rs` or wherever keybindings are declared in this codebase — search for `Cmd+Shift+J` to confirm availability) and add a default binding for `ToggleCliChatPanel`. If `Cmd+Shift+J` is taken, pick the next free combo and update `specs/castcodes-chat-panel/PRODUCT.md` settings section accordingly.

- [ ] **Step 5: Verify build**

Run: `cargo check -p warp-app --bin cast-codes --features gui`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src/workspace/action.rs app/src/app_menus.rs
git commit -m "feat(cli_chat): add ToggleCliChatPanel action + menu item"
```

### Task 1.5: Define domain types

**Files:**
- Modify: `app/src/cli_chat/conversation.rs` (replace stub)
- Test: `app/src/cli_chat/conversation_tests.rs` (new, inline-style — see step 1 for convention check)

- [ ] **Step 1: Confirm the codebase's test-file convention**

Search the repo for files like `*_tests.rs`. The codebase uses sibling `_tests.rs` files (e.g., `state_tests.rs`, `mod_tests.rs`) rather than `#[cfg(test)] mod tests`. Mirror this convention. If a module already has a `_tests.rs`, your tests go there; otherwise create one.

- [ ] **Step 2: Write `conversation.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;
use crate::terminal::CLIAgent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
    OpenCode,
}

impl AgentKind {
    pub fn from_cli_agent(agent: &CLIAgent) -> Option<Self> {
        // CLIAgent is defined in app/src/terminal/mod.rs.
        // Read that file to find the variant names and update this match.
        // The four supported agents map 1:1; any other variant returns None.
        use crate::terminal::CLIAgent::*;
        Some(match agent {
            Claude => AgentKind::Claude,
            Codex => AgentKind::Codex,
            Gemini => AgentKind::Gemini,
            OpenCode => AgentKind::OpenCode,
            _ => return None,
        })
    }

    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
            AgentKind::Gemini => "gemini",
            AgentKind::OpenCode => "opencode",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentKind::Claude => "Claude",
            AgentKind::Codex => "Codex",
            AgentKind::Gemini => "Gemini",
            AgentKind::OpenCode => "OpenCode",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatConversation {
    pub session_id: String,
    pub agent: AgentKind,
    pub title: String,
    pub cwd: Option<String>,
    pub project: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: CLIAgentSessionStatus,
    pub last_model: Option<String>,
    pub entries: Vec<crate::cli_chat::entry::ChatEntry>,
}

impl ChatConversation {
    pub fn new(session_id: String, agent: AgentKind, now: DateTime<Utc>) -> Self {
        Self {
            session_id,
            agent,
            title: String::new(),
            cwd: None,
            project: None,
            created_at: now,
            updated_at: now,
            status: CLIAgentSessionStatus::InProgress,
            last_model: None,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConversationBinding {
    None,
    Live { session_id: String, terminal_view_id: warpui::EntityId },
    Past { session_id: String },
}
```

If `CLIAgent`'s variant names don't match (Claude / Codex / Gemini / OpenCode), update the `from_cli_agent` match arms accordingly. Read `app/src/terminal/mod.rs` to confirm.

- [ ] **Step 3: Write a unit test**

Create `app/src/cli_chat/conversation_tests.rs`:

```rust
use chrono::Utc;
use super::conversation::{AgentKind, ChatConversation};
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

#[test]
fn agent_kind_protocol_strings_round_trip() {
    let kinds = [AgentKind::Claude, AgentKind::Codex, AgentKind::Gemini, AgentKind::OpenCode];
    let strs = ["claude", "codex", "gemini", "opencode"];
    for (kind, s) in kinds.iter().zip(strs.iter()) {
        assert_eq!(kind.as_protocol_str(), *s);
    }
}

#[test]
fn new_conversation_defaults_are_in_progress() {
    let conv = ChatConversation::new("abc".into(), AgentKind::Claude, Utc::now());
    assert_eq!(conv.session_id, "abc");
    assert_eq!(conv.entries.len(), 0);
    assert!(matches!(conv.status, CLIAgentSessionStatus::InProgress));
}
```

Register the test module in `mod.rs`:

```rust
#[cfg(test)]
mod conversation_tests;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p warp-app cli_chat::conversation_tests --features gui`
Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/conversation.rs app/src/cli_chat/conversation_tests.rs app/src/cli_chat/mod.rs
git commit -m "feat(cli_chat): define ChatConversation + AgentKind types"
```

---

## Phase 2 — Event subscription + transcript (in-memory only)

### Task 2.1: Implement `ChatEntry` and event-to-entry conversion

**Files:**
- Modify: `app/src/cli_chat/entry.rs` (replace stub)
- Create: `app/src/cli_chat/entry_tests.rs`
- Create: `app/src/cli_chat/tests/fixtures/claude_prompt_submit.json`
- Create: `app/src/cli_chat/tests/fixtures/claude_tool_complete.json`
- Create: `app/src/cli_chat/tests/fixtures/claude_permission_request.json`
- Create: `app/src/cli_chat/tests/fixtures/claude_stop_with_response.json`
- Create: `app/src/cli_chat/tests/fixtures/claude_idle_prompt.json`

- [ ] **Step 1: Capture fixture payloads**

Copy verbatim from `app/src/terminal/cli_agent_sessions/mod_tests.rs` (already in the repo). Each fixture is the JSON body the listener sees. Example (`claude_prompt_submit.json`):

```json
{"v":1,"agent":"claude","event":"prompt_submit","session_id":"abc","cwd":"/tmp/proj","project":"proj","query":"fix the bug"}
```

Add five files matching the existing `mod_tests.rs` examples (prompt_submit, tool_complete, permission_request, stop with response, idle_prompt). The tests embed these via `include_str!`.

- [ ] **Step 2: Write `entry.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::terminal::cli_agent_sessions::event::{
    CLIAgentEvent, CLIAgentEventType,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
    pub kind: ChatEntryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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
    PermissionReplied {
        approved: bool,
        summary: Option<String>,
    },
    Info {
        info_kind: InfoKind,
        summary: Option<String>,
    },
    Stop {
        reason: StopReason,
        response: Option<String>,
    },
    Raw {
        event_type: String,
        payload_json: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoKind {
    IdlePrompt,
    QuestionAsked,
    SessionStart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Normal,
    Cancelled,
    Errored,
    Unknown,
}

impl ChatEntry {
    /// Build a ChatEntry from a parsed CLIAgentEvent. Returns None for event
    /// types that do not produce a transcript entry (e.g., PermissionReplied
    /// may be folded into the prior PermissionRequest entry — see callers).
    pub fn from_event(event: &CLIAgentEvent, sequence: u64, now: DateTime<Utc>) -> Option<Self> {
        let kind = match &event.event {
            CLIAgentEventType::SessionStart => ChatEntryKind::Info {
                info_kind: InfoKind::SessionStart,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::PromptSubmit => {
                let text = event.payload.query.clone()?;
                ChatEntryKind::UserPrompt { text }
            }
            CLIAgentEventType::ToolComplete => ChatEntryKind::ToolCall {
                tool_name: event.payload.tool_name.clone().unwrap_or_default(),
                input_preview: event.payload.tool_input_preview.clone(),
            },
            CLIAgentEventType::PermissionRequest => ChatEntryKind::PermissionRequest {
                summary: event.payload.summary.clone().unwrap_or_default(),
                tool_name: event.payload.tool_name.clone(),
                tool_input_preview: event.payload.tool_input_preview.clone(),
            },
            CLIAgentEventType::PermissionReplied => ChatEntryKind::PermissionReplied {
                approved: true,  // event payload may or may not carry this; refine on real data
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::QuestionAsked => ChatEntryKind::Info {
                info_kind: InfoKind::QuestionAsked,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::IdlePrompt => ChatEntryKind::Info {
                info_kind: InfoKind::IdlePrompt,
                summary: event.payload.summary.clone(),
            },
            CLIAgentEventType::Stop => {
                let response = event.payload.response.clone().filter(|s| !s.is_empty());
                // If a response was emitted, also surface it as an AssistantResponse
                // entry. Callers handle this by inserting both entries; here we
                // produce just the Stop entry. See ChatModel for the second insert.
                ChatEntryKind::Stop {
                    reason: StopReason::Normal,
                    response,
                }
            }
            CLIAgentEventType::Unknown(s) => ChatEntryKind::Raw {
                event_type: s.clone(),
                payload_json: serde_json::to_string(&serde_json::json!({
                    "query": event.payload.query,
                    "response": event.payload.response,
                    "summary": event.payload.summary,
                    "tool_name": event.payload.tool_name,
                })).unwrap_or_default(),
            },
        };
        Some(Self {
            sequence,
            created_at: now,
            kind,
        })
    }
}
```

- [ ] **Step 3: Write tests**

Create `app/src/cli_chat/entry_tests.rs`:

```rust
use chrono::Utc;
use super::entry::{ChatEntry, ChatEntryKind, InfoKind};
use crate::terminal::cli_agent_sessions::event::parse_event;

fn parse_fixture(body: &str) -> crate::terminal::cli_agent_sessions::event::CLIAgentEvent {
    parse_event(Some("warp://cli-agent"), body).expect("fixture parses")
}

#[test]
fn prompt_submit_becomes_user_prompt() {
    let body = include_str!("tests/fixtures/claude_prompt_submit.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    match entry.kind {
        ChatEntryKind::UserPrompt { text } => assert_eq!(text, "fix the bug"),
        other => panic!("expected UserPrompt, got {:?}", other),
    }
}

#[test]
fn tool_complete_becomes_tool_call() {
    let body = include_str!("tests/fixtures/claude_tool_complete.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    assert!(matches!(entry.kind, ChatEntryKind::ToolCall { .. }));
}

#[test]
fn stop_with_response_carries_response() {
    let body = include_str!("tests/fixtures/claude_stop_with_response.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    match entry.kind {
        ChatEntryKind::Stop { response: Some(r), .. } => assert!(!r.is_empty()),
        other => panic!("expected Stop with response, got {:?}", other),
    }
}

#[test]
fn idle_prompt_becomes_info() {
    let body = include_str!("tests/fixtures/claude_idle_prompt.json");
    let event = parse_fixture(body);
    let entry = ChatEntry::from_event(&event, 0, Utc::now()).expect("entry produced");
    assert!(matches!(
        entry.kind,
        ChatEntryKind::Info { info_kind: InfoKind::IdlePrompt, .. }
    ));
}
```

Register in `mod.rs`:

```rust
#[cfg(test)]
mod entry_tests;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p warp-app cli_chat::entry_tests --features gui`
Expected: PASS, 4 tests.

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/entry.rs app/src/cli_chat/entry_tests.rs app/src/cli_chat/tests/ app/src/cli_chat/mod.rs
git commit -m "feat(cli_chat): convert CLIAgentEvent to typed ChatEntry"
```

### Task 2.2: Implement `ChatModel` subscriber

**Files:**
- Modify: `app/src/cli_chat/model.rs` (replace stub)
- Create: `app/src/cli_chat/model_tests.rs`

- [ ] **Step 1: Read the existing sessions model events API**

Read `app/src/terminal/cli_agent_sessions/mod.rs` lines 234–290 (the `CLIAgentSessionsModelEvent` enum and `CLIAgentSessionsModel` struct) to identify which event variant is emitted when a `CLIAgentEvent` is forwarded. Likely there's a `SessionUpdated { terminal_view_id, event }` or similar — confirm and use that exact variant name.

- [ ] **Step 2: Write `model.rs`**

```rust
use std::collections::HashMap;

use chrono::Utc;
use warpui::{Entity, EntityId, ModelContext, ModelHandle, SingletonEntity};

use crate::cli_chat::conversation::{AgentKind, ChatConversation, ConversationBinding};
use crate::cli_chat::entry::{ChatEntry, ChatEntryKind};
use crate::terminal::cli_agent_sessions::{
    event::CLIAgentEvent, CLIAgentSessionsModel, CLIAgentSessionsModelEvent,
};

#[derive(Debug, Clone)]
pub enum ChatModelEvent {
    ConversationUpdated { session_id: String },
    ConversationListChanged,
    BindingChanged,
    ProtocolIncompatibilityDetected,
}

pub struct ChatModel {
    conversations: HashMap<String, ChatConversation>,
    next_sequence: HashMap<String, u64>,
    binding: ConversationBinding,
}

impl ChatModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        // Subscribe to CLIAgentSessionsModel events.
        // The exact subscription API depends on warpui — read existing callers
        // of `CLIAgentSessionsModel::observe` or similar in app/src/ to mirror
        // the pattern. Typical shape:
        let sessions = CLIAgentSessionsModel::as_handle(ctx.app());
        ctx.observe(&sessions, |this, sessions, event, ctx| {
            this.handle_sessions_event(event, &sessions, ctx);
        });
        Self {
            conversations: HashMap::new(),
            next_sequence: HashMap::new(),
            binding: ConversationBinding::None,
        }
    }

    pub fn binding(&self) -> &ConversationBinding {
        &self.binding
    }

    pub fn conversation(&self, session_id: &str) -> Option<&ChatConversation> {
        self.conversations.get(session_id)
    }

    pub fn conversations_sorted_by_recency(&self) -> Vec<&ChatConversation> {
        let mut v: Vec<_> = self.conversations.values().collect();
        v.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        v
    }

    pub fn bind_live(&mut self, session_id: String, terminal_view_id: EntityId, ctx: &mut ModelContext<Self>) {
        self.binding = ConversationBinding::Live { session_id, terminal_view_id };
        ctx.emit(ChatModelEvent::BindingChanged);
    }

    pub fn bind_past(&mut self, session_id: String, ctx: &mut ModelContext<Self>) {
        self.binding = ConversationBinding::Past { session_id };
        ctx.emit(ChatModelEvent::BindingChanged);
    }

    pub fn unbind(&mut self, ctx: &mut ModelContext<Self>) {
        self.binding = ConversationBinding::None;
        ctx.emit(ChatModelEvent::BindingChanged);
    }

    fn handle_sessions_event(
        &mut self,
        event: &CLIAgentSessionsModelEvent,
        sessions: &ModelHandle<CLIAgentSessionsModel>,
        ctx: &mut ModelContext<Self>,
    ) {
        // The exact match arms depend on the real CLIAgentSessionsModelEvent enum.
        // Read app/src/terminal/cli_agent_sessions/mod.rs and adjust.
        match event {
            CLIAgentSessionsModelEvent::EventReceived { terminal_view_id, event } => {
                self.apply_event(event, *terminal_view_id, ctx);
            }
            _ => {}
        }
    }

    fn apply_event(&mut self, event: &CLIAgentEvent, terminal_view_id: EntityId, ctx: &mut ModelContext<Self>) {
        let Some(session_id) = event.session_id.clone() else { return };
        let agent = match AgentKind::from_cli_agent(&event.agent) {
            Some(a) => a,
            None => return,
        };
        let now = Utc::now();

        let conv = self.conversations.entry(session_id.clone()).or_insert_with(|| {
            let mut c = ChatConversation::new(session_id.clone(), agent, now);
            c.cwd = event.cwd.clone();
            c.project = event.project.clone();
            c
        });
        conv.updated_at = now;

        let sequence = self.next_sequence.entry(session_id.clone()).or_insert(0);
        if let Some(mut entry) = ChatEntry::from_event(event, *sequence, now) {
            // Auto-derive title from the first user prompt.
            if conv.title.is_empty() {
                if let ChatEntryKind::UserPrompt { text } = &entry.kind {
                    conv.title = text.chars().take(80).collect();
                }
            }
            conv.entries.push(entry.clone());
            *sequence += 1;

            // Special case: Stop event with response also produces an
            // AssistantResponse entry immediately before the Stop entry, so
            // the transcript reads naturally.
            if let ChatEntryKind::Stop { response: Some(text), .. } = &entry.kind {
                let prev_seq = *sequence;
                let response_entry = ChatEntry {
                    sequence: prev_seq,
                    created_at: now,
                    kind: ChatEntryKind::AssistantResponse { text: text.clone() },
                };
                // Insert the assistant response just before the Stop entry.
                let last_idx = conv.entries.len() - 1;
                conv.entries.insert(last_idx, response_entry);
                *sequence += 1;
            }
        }

        // Auto-bind on first event for a new session if nothing is currently bound.
        if matches!(self.binding, ConversationBinding::None) {
            self.binding = ConversationBinding::Live {
                session_id: session_id.clone(),
                terminal_view_id,
            };
            ctx.emit(ChatModelEvent::BindingChanged);
        }

        ctx.emit(ChatModelEvent::ConversationUpdated { session_id });
        ctx.emit(ChatModelEvent::ConversationListChanged);
    }
}
```

If `CLIAgentSessionsModel::as_handle` / `observe` / `as_ref` are not the exact APIs (look for `SingletonEntity` impls and callers throughout the app — `app/src/ai/agent_management/` is a good reference for the subscription pattern), adjust to match. The data flow is what matters: subscribe to the sessions model, intercept event forwarding, build conversations.

- [ ] **Step 3: Write a model test**

Create `app/src/cli_chat/model_tests.rs`. The model needs a warpui app context for `ModelContext`. Mirror the testing pattern from existing models — search for `model_tests.rs` files in `app/src/ai/` (e.g., `request_usage_model_tests.rs`) to find the harness.

```rust
// Pattern (refine using a real model_tests example as reference):
// 1. Build a warpui test app.
// 2. Insert a CLIAgentSessionsModel.
// 3. Insert a ChatModel that subscribes.
// 4. Programmatically fire a CLIAgentEvent through the sessions model
//    (using whatever public test hook exists — likely
//    sessions.update_from_event(...)).
// 5. Assert ChatModel.conversation(session_id) contains the expected entry.
```

Write at least two tests:

```rust
#[test]
fn first_prompt_creates_conversation_and_binds_live() {
    // ... set up warpui test app, insert sessions model, insert chat model ...
    // fire prompt_submit, then:
    // assert chat_model.conversation("abc").is_some();
    // assert matches!(chat_model.binding(), ConversationBinding::Live { .. });
}

#[test]
fn stop_event_with_response_splits_into_assistant_then_stop() {
    // ... fire stop event with response: "Memory is safe" ...
    // assert last two entries are AssistantResponse then Stop.
}
```

The full test bodies require the warpui app-context harness; the executing agent should copy the harness setup from a sibling `*_tests.rs` in this codebase rather than inventing it.

- [ ] **Step 4: Run tests**

Run: `cargo test -p warp-app cli_chat::model_tests --features gui`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/model.rs app/src/cli_chat/model_tests.rs app/src/cli_chat/mod.rs
git commit -m "feat(cli_chat): ChatModel subscribes to CLIAgentSessionsModel"
```

### Task 2.3: Implement minimal `ChatPanelView` (transcript-only)

**Files:**
- Modify: `app/src/cli_chat/view/mod.rs` (replace stub)
- Create: `app/src/cli_chat/view/transcript.rs`
- Create: `app/src/cli_chat/view/message_bubble.rs`

- [ ] **Step 1: Pick a reference view to mirror**

Read `app/src/workspace/view/conversation_list/view.rs` (the inherited conversation-list view) lines 1–150 to identify the warpui `View` + `ViewContext` + `Render` pattern in this codebase. Mirror the import block and the `impl View for X` structure.

- [ ] **Step 2: Write `view/mod.rs`**

```rust
pub mod composer;          // stub for now; Phase 5
pub mod conversation_list; // stub for now; Phase 4
pub mod empty_state;       // stub for now; Phase 7
pub mod error_banner;      // stub for now; Phase 7
pub mod info_bar;          // stub for now
pub mod message_bubble;
pub mod model_picker;      // stub for now; Phase 6
pub mod permission_card;   // stub for now
pub mod settings_section;  // stub for now; Phase 8
pub mod tool_call_card;    // stub for now
pub mod transcript;

use warpui::{AppContext, Entity, ModelContext, ModelHandle, View, ViewContext};

use crate::cli_chat::model::{ChatModel, ChatModelEvent};

pub struct ChatPanelView {
    chat_model: ModelHandle<ChatModel>,
}

impl ChatPanelView {
    pub fn new(chat_model: ModelHandle<ChatModel>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.observe(&chat_model, |_view, _model, _event: &ChatModelEvent, ctx| {
            ctx.notify();
        });
        Self { chat_model }
    }
}

impl View for ChatPanelView {
    fn render(&mut self, ctx: &mut ViewContext<Self>) -> Box<dyn warpui::elements::Element> {
        transcript::render_panel(self, ctx)
    }
}
```

Create stub files for the other submodules so `mod.rs` compiles:

```bash
for f in composer conversation_list empty_state error_banner info_bar model_picker permission_card settings_section tool_call_card; do
  echo "// stub — see PLAN.md" > app/src/cli_chat/view/$f.rs
done
```

- [ ] **Step 3: Write `view/transcript.rs`**

```rust
use warpui::elements::{Element, Flex, ParentElement, MainAxisAlignment, MainAxisSize};

use crate::cli_chat::conversation::{ChatConversation, ConversationBinding};
use crate::cli_chat::entry::{ChatEntry, ChatEntryKind};
use crate::cli_chat::view::ChatPanelView;
use crate::cli_chat::view::message_bubble;

pub fn render_panel(view: &ChatPanelView, ctx: &mut warpui::ViewContext<ChatPanelView>) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(ctx.app());

    let conversation = match chat.binding() {
        ConversationBinding::Live { session_id, .. } | ConversationBinding::Past { session_id } => {
            chat.conversation(session_id)
        }
        ConversationBinding::None => None,
    };

    let body: Box<dyn Element> = match conversation {
        Some(conv) => render_transcript(conv),
        None => render_empty_placeholder(),
    };

    Flex::column()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::Start)
        .with_child(body)
        .finish()
}

fn render_transcript(conv: &ChatConversation) -> Box<dyn Element> {
    let mut col = Flex::column().with_main_axis_size(MainAxisSize::Min);
    for entry in &conv.entries {
        col = col.with_child(render_entry(entry));
    }
    col.finish()
}

fn render_entry(entry: &ChatEntry) -> Box<dyn Element> {
    match &entry.kind {
        ChatEntryKind::UserPrompt { text } => message_bubble::user_bubble(text),
        ChatEntryKind::AssistantResponse { text } => message_bubble::assistant_bubble(text),
        ChatEntryKind::ToolCall { tool_name, input_preview } => {
            message_bubble::tool_placeholder(tool_name, input_preview.as_deref())
        }
        ChatEntryKind::PermissionRequest { summary, .. } => {
            message_bubble::permission_placeholder(summary)
        }
        ChatEntryKind::Info { summary, .. } => message_bubble::info_line(summary.as_deref()),
        ChatEntryKind::Stop { .. } => message_bubble::stop_marker(),
        ChatEntryKind::PermissionReplied { .. } | ChatEntryKind::Raw { .. } => {
            message_bubble::info_line(Some("(internal event)"))
        }
    }
}

fn render_empty_placeholder() -> Box<dyn Element> {
    use crate::cli_chat::strings::EMPTY_NO_HISTORY_TITLE;
    message_bubble::info_line(Some(EMPTY_NO_HISTORY_TITLE))
}
```

If the exact warpui `Flex` / `Element` builder API differs, mirror it from `conversation_list/view.rs`.

- [ ] **Step 4: Write `view/message_bubble.rs`**

```rust
use warpui::elements::{Element, Container, Padding, Text, ParentElement};

pub fn user_bubble(text: &str) -> Box<dyn Element> {
    Container::new(Text::new(text).finish())
        .with_padding(Padding::all(8.0))
        .finish()
}

pub fn assistant_bubble(text: &str) -> Box<dyn Element> {
    Container::new(Text::new(text).finish())
        .with_padding(Padding::all(8.0))
        .finish()
}

pub fn tool_placeholder(tool_name: &str, input_preview: Option<&str>) -> Box<dyn Element> {
    let label = match input_preview {
        Some(p) => format!("[tool] {}({})", tool_name, p),
        None => format!("[tool] {}()", tool_name),
    };
    Container::new(Text::new(&label).finish())
        .with_padding(Padding::all(6.0))
        .finish()
}

pub fn permission_placeholder(summary: &str) -> Box<dyn Element> {
    Container::new(Text::new(&format!("[permission] {}", summary)).finish())
        .with_padding(Padding::all(6.0))
        .finish()
}

pub fn info_line(text: Option<&str>) -> Box<dyn Element> {
    let label = text.unwrap_or("");
    Container::new(Text::new(label).finish())
        .with_padding(Padding::all(4.0))
        .finish()
}

pub fn stop_marker() -> Box<dyn Element> {
    info_line(Some(crate::cli_chat::strings::TRANSCRIPT_TURN_COMPLETE))
}
```

Styling (colors, borders, fonts) is intentionally minimal in v1 — Phase 7 polishes. The goal here is structural correctness.

- [ ] **Step 5: Wire panel into workspace**

Edit `app/src/workspace/view.rs` to register `ChatPanelView` in the right-panel slot. Read `app/src/workspace/view/right_panel.rs` to learn the registration pattern. Mirror it.

- [ ] **Step 6: Verify build + manual render**

Run: `cargo check -p warp-app --bin cast-codes --features gui`
Expected: PASS.

Run: `./script/run`
Manually: Toggle the chat panel via the new menu item. With no events, the panel should show the empty-history info line.

- [ ] **Step 7: Commit**

```bash
git add app/src/cli_chat/view/ app/src/workspace/view.rs
git commit -m "feat(cli_chat): minimal ChatPanelView renders transcript"
```

---

## Phase 3 — Persistence (sqlite)

### Task 3.1: Write schema + migration runner

**Files:**
- Modify: `app/src/cli_chat/store_schema.rs` (replace stub)
- Create: `app/src/cli_chat/store_schema_tests.rs`

- [ ] **Step 1: Write `store_schema.rs`**

```rust
use rusqlite::{Connection, Result};

pub const CURRENT_VERSION: i32 = 1;

const MIGRATIONS: &[&[&str]] = &[
    // Version 1: initial schema.
    &[
        r#"CREATE TABLE IF NOT EXISTS chat_conversation (
            session_id        TEXT PRIMARY KEY,
            agent             TEXT NOT NULL,
            title             TEXT NOT NULL,
            cwd               TEXT,
            project           TEXT,
            created_at        INTEGER NOT NULL,
            updated_at        INTEGER NOT NULL,
            status            TEXT NOT NULL,
            last_model        TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS chat_entry (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id        TEXT NOT NULL REFERENCES chat_conversation(session_id) ON DELETE CASCADE,
            sequence          INTEGER NOT NULL,
            created_at        INTEGER NOT NULL,
            kind              TEXT NOT NULL,
            payload_json      TEXT NOT NULL,
            UNIQUE (session_id, sequence)
        )"#,
        r#"CREATE INDEX IF NOT EXISTS idx_chat_entry_session ON chat_entry(session_id, sequence)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_chat_conv_updated ON chat_conversation(updated_at DESC)"#,
        r#"CREATE TABLE IF NOT EXISTS chat_schema_version (
            version INTEGER PRIMARY KEY
        )"#,
        r#"INSERT OR IGNORE INTO chat_schema_version (version) VALUES (1)"#,
    ],
];

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS chat_schema_version (version INTEGER PRIMARY KEY)")?;
    let current: i32 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM chat_schema_version", [], |r| r.get(0))
        .unwrap_or(0);

    if current > CURRENT_VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }

    for v in (current as usize)..MIGRATIONS.len() {
        for stmt in MIGRATIONS[v] {
            conn.execute(stmt, [])?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO chat_schema_version (version) VALUES (?1)",
            [v as i32 + 1],
        )?;
    }
    Ok(())
}
```

- [ ] **Step 2: Write tests**

Create `app/src/cli_chat/store_schema_tests.rs`:

```rust
use rusqlite::Connection;
use super::store_schema::{migrate, CURRENT_VERSION};

#[test]
fn migrate_from_empty_creates_tables() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_conversation'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn migrate_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    migrate(&conn).unwrap();
    let v: i32 = conn.query_row("SELECT MAX(version) FROM chat_schema_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, CURRENT_VERSION);
}

#[test]
fn migrate_rejects_future_version() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE chat_schema_version (version INTEGER PRIMARY KEY)", []).unwrap();
    conn.execute("INSERT INTO chat_schema_version VALUES (99)", []).unwrap();
    assert!(migrate(&conn).is_err());
}
```

Register in `mod.rs`:

```rust
#[cfg(test)]
mod store_schema_tests;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p warp-app cli_chat::store_schema_tests --features gui`
Expected: PASS, 3 tests.

- [ ] **Step 4: Commit**

```bash
git add app/src/cli_chat/store_schema.rs app/src/cli_chat/store_schema_tests.rs app/src/cli_chat/mod.rs
git commit -m "feat(cli_chat): sqlite schema + migration runner"
```

### Task 3.2: Implement `ChatStore`

**Files:**
- Modify: `app/src/cli_chat/store.rs` (replace stub)
- Modify: `app/src/cli_chat/paths.rs` (replace stub)
- Create: `app/src/cli_chat/store_tests.rs`

- [ ] **Step 1: Write `paths.rs`**

Read how the rest of the codebase resolves the CastCodes data directory. Look for callers of `cast_codes` config helpers (search for `.cast-codes` in `crates/warp_files/` or `app/src/`). Mirror the exact API.

```rust
use std::path::PathBuf;
use anyhow::Result;

/// Returns the absolute path to the cli_chat sqlite database, ensuring the
/// parent directory exists.
pub fn database_path() -> Result<PathBuf> {
    // Mirror the data-dir resolution used elsewhere in the codebase.
    // Likely something like:
    //   let data_dir = crate::cast_codes_data_dir()?;
    //   std::fs::create_dir_all(&data_dir)?;
    //   Ok(data_dir.join("cli_chat.sqlite"))
    //
    // Read app/src/lib.rs and crates/warp_files/ to find the canonical helper
    // and replace this stub.
    todo!("wire up to cast_codes_data_dir helper")
}
```

Once the executing agent identifies the helper, replace the `todo!()` with the call.

- [ ] **Step 2: Write `store.rs`**

```rust
use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection};

use crate::cli_chat::conversation::{AgentKind, ChatConversation};
use crate::cli_chat::entry::{ChatEntry, ChatEntryKind};
use crate::cli_chat::store_schema;
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

pub struct ChatStore {
    conn: Connection,
}

impl ChatStore {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        store_schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        store_schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn upsert_conversation(&self, conv: &ChatConversation) -> rusqlite::Result<()> {
        self.conn.execute(
            r#"INSERT INTO chat_conversation
                (session_id, agent, title, cwd, project, created_at, updated_at, status, last_model)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(session_id) DO UPDATE SET
                    agent = excluded.agent,
                    title = excluded.title,
                    cwd = excluded.cwd,
                    project = excluded.project,
                    updated_at = excluded.updated_at,
                    status = excluded.status,
                    last_model = excluded.last_model"#,
            params![
                conv.session_id,
                conv.agent.as_protocol_str(),
                conv.title,
                conv.cwd,
                conv.project,
                conv.created_at.timestamp_millis(),
                conv.updated_at.timestamp_millis(),
                status_to_str(&conv.status),
                conv.last_model,
            ],
        )?;
        Ok(())
    }

    pub fn insert_entry(&self, session_id: &str, entry: &ChatEntry) -> rusqlite::Result<()> {
        let (kind, payload) = serialize_entry(&entry.kind);
        self.conn.execute(
            r#"INSERT INTO chat_entry
                (session_id, sequence, created_at, kind, payload_json)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(session_id, sequence) DO NOTHING"#,
            params![session_id, entry.sequence as i64, entry.created_at.timestamp_millis(), kind, payload],
        )?;
        Ok(())
    }

    pub fn load_conversation(&self, session_id: &str) -> rusqlite::Result<Option<ChatConversation>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT session_id, agent, title, cwd, project, created_at, updated_at, status, last_model
                 FROM chat_conversation WHERE session_id = ?1"#,
        )?;
        let mut rows = stmt.query(params![session_id])?;
        let Some(row) = rows.next()? else { return Ok(None) };
        let agent = agent_from_str(row.get::<_, String>(1)?.as_str())
            .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
        let mut conv = ChatConversation {
            session_id: row.get(0)?,
            agent,
            title: row.get(2)?,
            cwd: row.get(3)?,
            project: row.get(4)?,
            created_at: Utc.timestamp_millis_opt(row.get(5)?).unwrap(),
            updated_at: Utc.timestamp_millis_opt(row.get(6)?).unwrap(),
            status: str_to_status(row.get::<_, String>(7)?.as_str()),
            last_model: row.get(8)?,
            entries: vec![],
        };
        conv.entries = self.load_entries(session_id)?;
        Ok(Some(conv))
    }

    pub fn load_entries(&self, session_id: &str) -> rusqlite::Result<Vec<ChatEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT sequence, created_at, kind, payload_json FROM chat_entry WHERE session_id = ?1 ORDER BY sequence",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            let sequence: i64 = row.get(0)?;
            let ts: i64 = row.get(1)?;
            let kind: String = row.get(2)?;
            let payload: String = row.get(3)?;
            Ok(ChatEntry {
                sequence: sequence as u64,
                created_at: Utc.timestamp_millis_opt(ts).unwrap(),
                kind: deserialize_entry_kind(&kind, &payload),
            })
        })?;
        rows.collect()
    }

    pub fn list_conversations(&self) -> rusqlite::Result<Vec<ChatConversation>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT session_id FROM chat_conversation ORDER BY updated_at DESC"#,
        )?;
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(c) = self.load_conversation(&id)? {
                out.push(c);
            }
        }
        Ok(out)
    }
}

fn status_to_str(s: &CLIAgentSessionStatus) -> &'static str {
    match s {
        CLIAgentSessionStatus::InProgress => "in_progress",
        CLIAgentSessionStatus::Success => "success",
        CLIAgentSessionStatus::Blocked { .. } => "blocked",
    }
}

fn str_to_status(s: &str) -> CLIAgentSessionStatus {
    match s {
        "success" => CLIAgentSessionStatus::Success,
        "blocked" => CLIAgentSessionStatus::Blocked { message: None },
        _ => CLIAgentSessionStatus::InProgress,
    }
}

fn agent_from_str(s: &str) -> Option<AgentKind> {
    Some(match s {
        "claude" => AgentKind::Claude,
        "codex" => AgentKind::Codex,
        "gemini" => AgentKind::Gemini,
        "opencode" => AgentKind::OpenCode,
        _ => return None,
    })
}

fn serialize_entry(kind: &ChatEntryKind) -> (&'static str, String) {
    let kind_str = match kind {
        ChatEntryKind::UserPrompt { .. } => "user",
        ChatEntryKind::AssistantResponse { .. } => "assistant",
        ChatEntryKind::ToolCall { .. } => "tool",
        ChatEntryKind::PermissionRequest { .. } => "permission",
        ChatEntryKind::PermissionReplied { .. } => "permission_replied",
        ChatEntryKind::Info { .. } => "info",
        ChatEntryKind::Stop { .. } => "stop",
        ChatEntryKind::Raw { .. } => "raw",
    };
    let payload = serde_json::to_string(kind).unwrap_or_default();
    (kind_str, payload)
}

fn deserialize_entry_kind(_kind: &str, payload_json: &str) -> ChatEntryKind {
    serde_json::from_str(payload_json).unwrap_or(ChatEntryKind::Raw {
        event_type: "deserialize_failed".into(),
        payload_json: payload_json.into(),
    })
}
```

- [ ] **Step 3: Write tests**

Create `app/src/cli_chat/store_tests.rs`:

```rust
use chrono::Utc;
use super::conversation::{AgentKind, ChatConversation};
use super::entry::{ChatEntry, ChatEntryKind};
use super::store::ChatStore;
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

#[test]
fn round_trip_conversation_and_entries() {
    let store = ChatStore::open_in_memory().unwrap();
    let now = Utc::now();
    let conv = ChatConversation {
        session_id: "abc".into(),
        agent: AgentKind::Claude,
        title: "hello".into(),
        cwd: Some("/tmp".into()),
        project: Some("proj".into()),
        created_at: now,
        updated_at: now,
        status: CLIAgentSessionStatus::InProgress,
        last_model: Some("claude-opus-4-7".into()),
        entries: vec![],
    };
    store.upsert_conversation(&conv).unwrap();

    let entry = ChatEntry {
        sequence: 0,
        created_at: now,
        kind: ChatEntryKind::UserPrompt { text: "fix the bug".into() },
    };
    store.insert_entry("abc", &entry).unwrap();

    let loaded = store.load_conversation("abc").unwrap().expect("conv exists");
    assert_eq!(loaded.title, "hello");
    assert_eq!(loaded.entries.len(), 1);
    match &loaded.entries[0].kind {
        ChatEntryKind::UserPrompt { text } => assert_eq!(text, "fix the bug"),
        other => panic!("unexpected entry kind: {:?}", other),
    }
}

#[test]
fn list_conversations_returns_in_recency_order() {
    let store = ChatStore::open_in_memory().unwrap();
    let now = Utc::now();
    let mut a = ChatConversation {
        session_id: "a".into(),
        agent: AgentKind::Claude,
        title: "older".into(),
        cwd: None, project: None,
        created_at: now, updated_at: now,
        status: CLIAgentSessionStatus::InProgress,
        last_model: None, entries: vec![],
    };
    let mut b = a.clone();
    b.session_id = "b".into();
    b.title = "newer".into();
    b.updated_at = now + chrono::Duration::seconds(10);
    store.upsert_conversation(&a).unwrap();
    store.upsert_conversation(&b).unwrap();
    let list = store.list_conversations().unwrap();
    assert_eq!(list[0].session_id, "b");
    assert_eq!(list[1].session_id, "a");
}
```

Register in `mod.rs`:

```rust
#[cfg(test)]
mod store_tests;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p warp-app cli_chat::store_tests --features gui`
Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/store.rs app/src/cli_chat/store_tests.rs app/src/cli_chat/paths.rs app/src/cli_chat/mod.rs
git commit -m "feat(cli_chat): rusqlite-backed ChatStore with round-trip tests"
```

### Task 3.3: Persist events as they arrive; load history on startup

**Files:**
- Modify: `app/src/cli_chat/model.rs`

- [ ] **Step 1: Add a `ChatStore` field to `ChatModel`**

Update `ChatModel::new` to open `ChatStore::open(&paths::database_path()?)` and store the handle. If open fails (e.g., readonly volume), log via `tracing::warn!` and continue with `Option<ChatStore>` set to None so the model still works in memory.

```rust
pub struct ChatModel {
    conversations: HashMap<String, ChatConversation>,
    next_sequence: HashMap<String, u64>,
    binding: ConversationBinding,
    store: Option<ChatStore>,
}

impl ChatModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let store = match crate::cli_chat::paths::database_path()
            .and_then(|p| ChatStore::open(&p).map_err(Into::into))
        {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!("cli_chat: opening sqlite store failed: {}", e);
                None
            }
        };
        let mut this = Self {
            conversations: HashMap::new(),
            next_sequence: HashMap::new(),
            binding: ConversationBinding::None,
            store,
        };
        this.load_existing_history();
        // ... subscribe to sessions model as before ...
        this
    }

    fn load_existing_history(&mut self) {
        let Some(store) = &self.store else { return };
        match store.list_conversations() {
            Ok(list) => {
                for conv in list {
                    let session_id = conv.session_id.clone();
                    let next_seq = conv.entries.iter().map(|e| e.sequence).max().map(|m| m + 1).unwrap_or(0);
                    self.next_sequence.insert(session_id.clone(), next_seq);
                    self.conversations.insert(session_id, conv);
                }
            }
            Err(e) => tracing::warn!("cli_chat: loading history failed: {}", e),
        }
    }
}
```

- [ ] **Step 2: Persist on event**

Inside `apply_event` (Task 2.2), after the entry is appended to the conversation:

```rust
if let Some(store) = &self.store {
    let _ = store.upsert_conversation(conv);
    if let Some(last) = conv.entries.last() {
        let _ = store.insert_entry(&session_id, last);
    }
    // Also insert the synthetic AssistantResponse entry if present.
}
```

The synthetic `AssistantResponse` insertion already happens before the Stop entry (Task 2.2 inserts at `entries.len() - 1`). After insertion you can iterate the last two entries and persist both.

- [ ] **Step 3: Add a model test for persistence**

Add to `app/src/cli_chat/model_tests.rs`:

```rust
#[test]
fn events_persist_to_store() {
    // Build a ChatModel pointed at an in-memory store (refactor `new` to accept
    // an optional store override for testability, or extract a smaller
    // constructor `ChatModel::with_store_for_testing(store)`).
    // Fire a prompt_submit. Assert the in-memory store now contains the row.
}
```

If `ChatModel::new` is hard to substitute the store in, add `ChatModel::with_store_for_testing(store: ChatStore) -> Self` that skips the path resolution.

- [ ] **Step 4: Run tests**

Run: `cargo test -p warp-app cli_chat:: --features gui`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/model.rs app/src/cli_chat/model_tests.rs
git commit -m "feat(cli_chat): persist events to sqlite + restore history on launch"
```

---

## Phase 4 — Conversation list / past sessions

### Task 4.1: Implement the conversation-list sidebar

**Files:**
- Modify: `app/src/cli_chat/view/conversation_list.rs` (replace stub)
- Modify: `app/src/cli_chat/view/mod.rs` (compose into panel layout)

- [ ] **Step 1: Write `conversation_list.rs`**

```rust
use warpui::elements::{Element, Flex, Container, Text, EventHandler, ParentElement, MainAxisSize};

use crate::cli_chat::conversation::ChatConversation;
use crate::cli_chat::view::ChatPanelView;

pub fn render(view: &ChatPanelView, ctx: &mut warpui::ViewContext<ChatPanelView>) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(ctx.app());
    let conversations = chat.conversations_sorted_by_recency();
    let mut col = Flex::column().with_main_axis_size(MainAxisSize::Min);
    for conv in conversations {
        col = col.with_child(render_item(conv));
    }
    col.finish()
}

fn render_item(conv: &ChatConversation) -> Box<dyn Element> {
    let session_id = conv.session_id.clone();
    let title = if conv.title.is_empty() {
        format!("(untitled) — {}", conv.agent.display_name())
    } else {
        format!("{} — {}", conv.title, conv.agent.display_name())
    };
    let body = Container::new(Text::new(&title).finish())
        .with_padding(warpui::elements::Padding::all(8.0))
        .finish();
    EventHandler::new(body)
        .on_left_mouse_down(move |ctx, _, _| {
            // Dispatch an action to bind this session.
            ctx.dispatch_typed_action(crate::workspace::action::WorkspaceAction::OpenChatSession {
                session_id: session_id.clone(),
            });
            warpui::elements::DispatchEventResult::StopPropagation
        })
        .finish()
}
```

If `WorkspaceAction::OpenChatSession` doesn't exist, add it (see Task 4.2).

- [ ] **Step 2: Compose the sidebar into the panel layout**

Edit `app/src/cli_chat/view/mod.rs` so the panel is a horizontal split: list on the left (~30% width), transcript on the right. Mirror the split pattern from `app/src/workspace/view/conversation_list/view.rs` if a similar two-pane layout is there.

```rust
impl View for ChatPanelView {
    fn render(&mut self, ctx: &mut ViewContext<Self>) -> Box<dyn warpui::elements::Element> {
        use warpui::elements::{Flex, ParentElement, MainAxisSize};
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(conversation_list::render(self, ctx))
            .with_child(transcript::render_panel(self, ctx))
            .finish()
    }
}
```

- [ ] **Step 3: Run + manual check**

Run: `./script/run`. After running `claude` in a terminal long enough to record events, open the panel. The list should show the conversation.

- [ ] **Step 4: Commit**

```bash
git add app/src/cli_chat/view/
git commit -m "feat(cli_chat): conversation-list sidebar"
```

### Task 4.2: Add `OpenChatSession` action and binding

**Files:**
- Modify: `app/src/workspace/action.rs`
- Modify: `app/src/cli_chat/model.rs`
- Modify: `app/src/cli_chat/view/mod.rs`

- [ ] **Step 1: Add the action**

In `app/src/workspace/action.rs`, add to the action enum:

```rust
OpenChatSession { session_id: String },
```

If the enum derives a fixed set of traits, mirror an existing parameterized action's derives.

- [ ] **Step 2: Add a handler in `ChatPanelView`**

In `app/src/cli_chat/view/mod.rs`, register an action observer that, on `OpenChatSession`, calls `chat_model.update(|m, ctx| m.bind_past(session_id, ctx))`. The exact action-handler registration API depends on warpui — read existing callers of `register_action` or `on_action` to mirror.

- [ ] **Step 3: Test binding flow**

In `model_tests.rs`, add:

```rust
#[test]
fn bind_past_switches_binding_state() {
    // Insert a conversation in the store, build ChatModel pointing at that store,
    // call model.bind_past("abc", ctx), assert binding is Past.
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p warp-app cli_chat:: --features gui`

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/action.rs app/src/cli_chat/
git commit -m "feat(cli_chat): bind past sessions via OpenChatSession action"
```

---

## Phase 5 — Composer

### Task 5.1: Implement composer delegating to rich input

**Files:**
- Modify: `app/src/cli_chat/view/composer.rs` (replace stub)
- Modify: `app/src/cli_chat/view/mod.rs` (compose composer below transcript)

- [ ] **Step 1: Read the existing rich-input submit path**

Read `app/src/terminal/input/cli_agent.rs` and `app/src/terminal/cli_agent_sessions/mod.rs` (specifically the `open_input`, `close_input`, `set_draft`, `take_draft` methods around lines 430–540) to understand the submit path. The composer must integrate with this — not duplicate it.

- [ ] **Step 2: Write `composer.rs`**

```rust
use warpui::elements::{Element, Container, EventHandler, Padding, Text, ParentElement};
use warpui::AppContext;

use crate::cli_chat::conversation::ConversationBinding;
use crate::cli_chat::view::ChatPanelView;
use crate::terminal::cli_agent_sessions::{CLIAgentInputEntrypoint, CLIAgentSessionsModel};

pub fn render(view: &ChatPanelView, ctx: &mut warpui::ViewContext<ChatPanelView>) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(ctx.app());
    let placeholder = match chat.binding() {
        ConversationBinding::Live { .. } => crate::cli_chat::strings::COMPOSER_PLACEHOLDER_ACTIVE,
        _ => crate::cli_chat::strings::COMPOSER_PLACEHOLDER_INACTIVE,
    };
    // Minimal single-line editor for v1. Mirror the editor construction used
    // elsewhere (e.g., the existing SingleLineEditor in conversation_list/view.rs).
    Container::new(Text::new(placeholder).finish())
        .with_padding(Padding::all(8.0))
        .finish()
}

pub fn submit(view: &mut ChatPanelView, text: String, app: &mut AppContext) {
    let chat = view.chat_model.as_ref(app);
    let ConversationBinding::Live { terminal_view_id, .. } = chat.binding().clone() else { return };
    let sessions = CLIAgentSessionsModel::as_handle(app);
    sessions.update(app, |m, ctx| {
        m.set_draft(terminal_view_id, text);
        m.open_input(terminal_view_id, CLIAgentInputEntrypoint::FooterButton, ctx);
        // Then trigger the same submit action the rich input footer button uses.
        // Read app/src/terminal/input/cli_agent.rs to find the action name and
        // dispatch it here.
    });
}
```

Replace the placeholder `Text` rendering with a real `EditorView` (mirror the editor instantiation pattern from `app/src/workspace/view/conversation_list/view.rs`) so the composer can actually accept input. Bind the editor's submit-on-Enter handler to call `submit(view, text, app)`.

- [ ] **Step 3: Mount the composer in the panel layout**

In `view/mod.rs`:

```rust
Flex::column()
    .with_child(/* horizontal row of list + transcript */)
    .with_child(composer::render(self, ctx))
    .finish()
```

- [ ] **Step 4: Manual verification**

Run `./script/run`. Run `claude` in a terminal. Open the chat panel. Type into the composer and press Enter. Confirm the prompt appears in the terminal (delegated through rich input).

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/view/composer.rs app/src/cli_chat/view/mod.rs
git commit -m "feat(cli_chat): composer wired to rich-input submit"
```

---

## Phase 6 — Model picker + new chat

### Task 6.1: Implement model picker

**Files:**
- Modify: `app/src/cli_chat/view/model_picker.rs` (replace stub)
- Modify: `app/src/cli_chat/view/mod.rs`

- [ ] **Step 1: Write a per-CLI curated model list**

Add to `app/src/cli_chat/conversation.rs` (or a new `app/src/cli_chat/models.rs`):

```rust
pub struct ModelOption {
    pub id: &'static str,
    pub display_name: &'static str,
}

impl AgentKind {
    pub fn curated_models(&self) -> &'static [ModelOption] {
        match self {
            AgentKind::Claude => &[
                ModelOption { id: "claude-opus-4-7", display_name: "Claude Opus 4.7" },
                ModelOption { id: "claude-sonnet-4-6", display_name: "Claude Sonnet 4.6" },
                ModelOption { id: "claude-haiku-4-5-20251001", display_name: "Claude Haiku 4.5" },
            ],
            AgentKind::Codex => &[
                ModelOption { id: "gpt-5-codex", display_name: "GPT-5 Codex" },
                ModelOption { id: "o4-mini", display_name: "o4-mini" },
            ],
            AgentKind::Gemini => &[
                ModelOption { id: "gemini-2.5-pro", display_name: "Gemini 2.5 Pro" },
                ModelOption { id: "gemini-2.5-flash", display_name: "Gemini 2.5 Flash" },
            ],
            AgentKind::OpenCode => &[
                ModelOption { id: "default", display_name: "OpenCode default" },
            ],
        }
    }
}
```

- [ ] **Step 2: Write `model_picker.rs`**

```rust
use warpui::elements::{Element, Container, Flex, Text, EventHandler, ParentElement, Padding};

use crate::cli_chat::conversation::{AgentKind, ModelOption};
use crate::cli_chat::view::ChatPanelView;

pub fn render(_view: &ChatPanelView, _ctx: &mut warpui::ViewContext<ChatPanelView>) -> Box<dyn Element> {
    // Render: [Agent dropdown] [Model dropdown] [New Chat button]
    // Each dropdown uses the existing Menu pattern in this codebase.
    // Read app/src/menu.rs or wherever Menu is defined and mirror.
    let agent_label = Container::new(Text::new("Claude").finish())
        .with_padding(Padding::all(6.0))
        .finish();
    let model_label = Container::new(Text::new("Opus 4.7").finish())
        .with_padding(Padding::all(6.0))
        .finish();
    let new_chat = EventHandler::new(
        Container::new(Text::new("New chat").finish())
            .with_padding(Padding::all(6.0))
            .finish(),
    )
    .on_left_mouse_down(|ctx, _, _| {
        ctx.dispatch_typed_action(crate::workspace::action::WorkspaceAction::CliChatNewChat {
            agent: AgentKind::Claude,
            model: "claude-opus-4-7".to_string(),
        });
        warpui::elements::DispatchEventResult::StopPropagation
    })
    .finish();

    Flex::row()
        .with_child(agent_label)
        .with_child(model_label)
        .with_child(new_chat)
        .finish()
}
```

This is a minimal version. Real dropdowns come in a polish pass — for v1 the picker reads from settings (default agent + default model) and "New chat" launches the default combo. Add real dropdowns as a polish task in Phase 7 if time permits.

- [ ] **Step 3: Add `CliChatNewChat` action**

In `app/src/workspace/action.rs`:

```rust
CliChatNewChat { agent: AgentKind, model: String },
```

- [ ] **Step 4: Implement the action handler**

The handler must open a new terminal with the command `<agent_bin> --model <model>`. Read `app/src/workspace/` and `app/src/terminal/` for the existing "open new terminal pane with command" API (search for `open_terminal_with_command` or similar). Use that API; do not spawn the subprocess directly.

```rust
fn handle_cli_chat_new_chat(workspace: &mut Workspace, agent: AgentKind, model: String, ctx: &mut WorkspaceContext) {
    let cmd = match agent {
        AgentKind::Claude => format!("claude --model {} ", model),
        AgentKind::Codex => format!("codex chat --model {} ", model),
        AgentKind::Gemini => format!("gemini --model {} ", model),
        AgentKind::OpenCode => format!("opencode --model {} ", model),
    };
    workspace.open_new_terminal_with_command(cmd, ctx);
}
```

If the API has a different name, find it via grep (`grep -rn "open.*terminal.*command" app/src/`) and use the correct one.

- [ ] **Step 5: Mount the picker in the panel header**

In `view/mod.rs`, add a header row above the list+transcript:

```rust
Flex::column()
    .with_child(model_picker::render(self, ctx))
    .with_child(/* list + transcript row */)
    .with_child(composer::render(self, ctx))
    .finish()
```

- [ ] **Step 6: Manual verification**

Run `./script/run`. Open the chat panel. Click "New chat". A new terminal opens running `claude --model claude-opus-4-7`. Once the plugin emits `session_start`, the panel auto-binds.

- [ ] **Step 7: Commit**

```bash
git add app/src/cli_chat/view/model_picker.rs app/src/cli_chat/conversation.rs app/src/workspace/action.rs app/src/cli_chat/view/mod.rs
git commit -m "feat(cli_chat): model picker + 'New chat' opens terminal with CLI"
```

---

## Phase 7 — Empty / error / polish

### Task 7.1: Empty states

**Files:**
- Modify: `app/src/cli_chat/view/empty_state.rs` (replace stub)
- Modify: `app/src/cli_chat/view/mod.rs`

- [ ] **Step 1: Detect installation status**

Add to `app/src/cli_chat/model.rs` (or a new `app/src/cli_chat/detect.rs`):

```rust
use crate::cli_chat::conversation::AgentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAvailability {
    Ready,            // CLI on PATH and plugin installed
    PluginMissing,    // CLI on PATH but plugin not detected
    NotInstalled,     // CLI not on PATH
    Unknown,
}

pub fn detect(agent: AgentKind) -> AgentAvailability {
    // For each agent, the existing plugin_manager has a (safe) `is_installed`
    // function that does a filesystem check. Call it WITHOUT touching any
    // marketplace-URL helpers.
    //
    // For PATH detection, use `which` or replicate the logic in
    // `crates/command/src/blocking.rs::CommandExt::find_in_path`.
    //
    // Returning Unknown is acceptable in v1 if implementation is non-trivial.
    AgentAvailability::Unknown
}
```

Note: PATH detection that calls into `plugin_manager/*.rs`'s `is_installed()` is allowed (it does only local FS checks). Calling `install()` or `update()` is forbidden in OSS builds (Phase 8 adds a CI guard).

- [ ] **Step 2: Write `empty_state.rs`**

```rust
use warpui::elements::{Element, Flex, Container, Text, Padding, ParentElement, MainAxisSize};

use crate::cli_chat::conversation::AgentKind;
use crate::cli_chat::model::AgentAvailability;
use crate::cli_chat::strings;

pub enum EmptyKind {
    NoHistoryAndNoCli,
    NoHistoryButCliReady,
    PluginMissing { agent: AgentKind },
}

pub fn render(kind: EmptyKind) -> Box<dyn Element> {
    let (title, body) = match kind {
        EmptyKind::NoHistoryAndNoCli => (strings::EMPTY_NO_CLI_TITLE, strings::EMPTY_NO_CLI_BODY),
        EmptyKind::NoHistoryButCliReady => (strings::EMPTY_NO_HISTORY_TITLE, strings::EMPTY_NO_HISTORY_BODY),
        EmptyKind::PluginMissing { .. } => (strings::EMPTY_NO_PLUGIN_TITLE, strings::EMPTY_NO_PLUGIN_BODY),
    };
    Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_child(Container::new(Text::new(title).finish()).with_padding(Padding::all(8.0)).finish())
        .with_child(Container::new(Text::new(body).finish()).with_padding(Padding::all(8.0)).finish())
        .finish()
}
```

- [ ] **Step 3: Use empty state in panel render**

In `transcript.rs::render_empty_placeholder`, replace the placeholder with a call to `empty_state::render(EmptyKind::NoHistoryAndNoCli)` or the appropriate variant based on detection.

- [ ] **Step 4: Commit**

```bash
git add app/src/cli_chat/view/empty_state.rs app/src/cli_chat/model.rs app/src/cli_chat/view/transcript.rs
git commit -m "feat(cli_chat): empty states for no-history / no-CLI / no-plugin"
```

### Task 7.2: Error banner for malformed events

**Files:**
- Modify: `app/src/cli_chat/view/error_banner.rs` (replace stub)
- Modify: `app/src/cli_chat/model.rs`

- [ ] **Step 1: Track skipped events count in `ChatModel`**

Add `skipped_event_count: u64` field. Increment in `apply_event` whenever `ChatEntry::from_event` returns None or when serde fails. Once >= 3, emit `ChatModelEvent::ProtocolIncompatibilityDetected`.

- [ ] **Step 2: Render the banner above the transcript when the flag is set**

```rust
pub fn render(message: &str) -> Box<dyn Element> {
    use warpui::elements::{Element, Container, Text, Padding, ParentElement};
    Container::new(Text::new(message).finish())
        .with_padding(Padding::all(8.0))
        .finish()
}
```

In `view/mod.rs`, prepend the banner to the transcript column when `chat.skipped_event_count() >= 3`.

- [ ] **Step 3: Commit**

```bash
git add app/src/cli_chat/view/error_banner.rs app/src/cli_chat/model.rs app/src/cli_chat/view/mod.rs
git commit -m "feat(cli_chat): error banner for plugin protocol incompatibility"
```

### Task 7.3: Settings section

**Files:**
- Modify: `app/src/cli_chat/view/settings_section.rs` (replace stub)
- Modify: `app/src/settings_view/ai_page.rs`

- [ ] **Step 1: Read the existing settings registration pattern**

Read `app/src/settings_view/ai_page.rs` lines 1–200 to find the section/row registration API. Mirror it.

- [ ] **Step 2: Write `settings_section.rs`**

```rust
//! Registers the "Chat Panel" settings section under the AI settings page.
//! No auto-installer rows here — install actions are vendor-doc links only.

use warpui::AppContext;

pub fn register(app: &mut AppContext) {
    // Sections:
    // - Detected CLIs (read-only list with version + installed/missing badge).
    // - Default agent (dropdown over AgentKind::iter()).
    // - Default model per agent (dropdown over AgentKind::curated_models()).
    // - "Show permission cards in transcript" toggle.
    // - "Open chat panel automatically on session start" toggle (default off).
    //
    // Implementation: mirror existing settings-row registration in
    // app/src/settings_view/ai_page.rs.
}
```

The full implementation requires reading the existing settings page code to mirror its row-builder API. The executing agent fills this in by reference.

- [ ] **Step 3: Hook into ai_page**

Call `cli_chat::view::settings_section::register(app)` from the appropriate point in `ai_page.rs` registration.

- [ ] **Step 4: Verify build + open settings UI**

Run `./script/run`. Navigate to AI settings. Confirm the new "Chat Panel" section appears.

- [ ] **Step 5: Commit**

```bash
git add app/src/cli_chat/view/settings_section.rs app/src/settings_view/ai_page.rs
git commit -m "feat(cli_chat): add Chat Panel settings section"
```

---

## Phase 8 — CI guards, rebrand, manual checklist

### Task 8.1: CI grep guards

**Files:**
- Create: `script/check_cli_chat_boundary` (new shell script)
- Modify: `.github/workflows/ci.yml` (add a step that runs the new script)

- [ ] **Step 1: Write the guard script**

```bash
#!/usr/bin/env bash
# Verifies the cli_chat module does not couple to hosted Warp infrastructure.
set -euo pipefail

PATTERN='warp_multi_agent_api|warp_server_client|app\.warp\.dev|api\.warp\.dev|MARKETPLACE_REPO|PLATFORM_MARKETPLACE_REPO|EXTENSION_REPO|warpdotdev/'
TARGET='app/src/cli_chat'

if ! [ -d "$TARGET" ]; then
    echo "cli_chat module not present; skipping guard."
    exit 0
fi

if grep -rnE "$PATTERN" "$TARGET" --include='*.rs'; then
    echo
    echo "ERROR: cli_chat must not reference Warp-owned infrastructure."
    echo "       See specs/castcodes-chat-panel/TECH.md for the boundary policy."
    exit 1
fi

echo "cli_chat boundary check: OK"
```

Make it executable: `chmod +x script/check_cli_chat_boundary`.

- [ ] **Step 2: Wire into CI**

In `.github/workflows/ci.yml`, find the existing lint/check step (probably runs `./script/check_rebrand`). Add a step running `./script/check_cli_chat_boundary` in the same job.

- [ ] **Step 3: Verify it catches a violation**

Manually add `// warpdotdev/test` as a comment in any `cli_chat/*.rs`. Run `./script/check_cli_chat_boundary`. Expected: FAIL. Remove the comment. Run again: PASS.

- [ ] **Step 4: Commit**

```bash
git add script/check_cli_chat_boundary .github/workflows/ci.yml
git commit -m "ci(cli_chat): enforce fork-local boundary with grep guard"
```

### Task 8.2: Rebrand guard sweep

**Files:** None (verification only)

- [ ] **Step 1: Run the rebrand guard**

Run: `./script/check_rebrand`
Expected: PASS. If failures point at our new strings, audit `app/src/cli_chat/strings.rs` and any inline strings in `view/*.rs`. Move offending text into `strings.rs` and replace with CastCodes-on-brand wording.

- [ ] **Step 2: Re-run until PASS**

If a string is genuinely about an upstream concept (e.g., "Claude Code plugin"), and the guard rejects it, follow `castcodes-rebrand-surface` skill guidance to either exclude the string (with justification) or rephrase.

- [ ] **Step 3: Commit any string adjustments**

```bash
git add app/src/cli_chat/
git commit -m "chore(cli_chat): rebrand-guard pass"
```

### Task 8.3: Manual verification checklist

**Files:**
- Create: `specs/castcodes-chat-panel/CHECKLIST.md`

- [ ] **Step 1: Author the checklist**

```markdown
# CastCodes Chat Panel — Manual Verification Checklist

Run through this before marking the feature ready for review.

## Empty states
- [ ] Fresh install, no `claude`/`codex`/`gemini`/`opencode` on PATH: panel shows "No supported CLI detected" with install commands.
- [ ] `claude` on PATH but plugin not installed (rename plugin dir or remove): panel shows "Plugin required" with vendor docs hint, no auto-install button.
- [ ] No prior conversations: list area shows "No conversations yet".

## Live transcript
- [ ] Run `claude` in a terminal, log in, send a prompt that elicits a tool call. Open panel: see user prompt, tool-call card, assistant response, stop marker in order.
- [ ] Run `codex chat`, send a prompt. Verify the transcript renders (codex uses the agent-specific notification format; minimal entries acceptable).
- [ ] Send a prompt that requires permission approval. Permission card appears in the transcript while the terminal shows its own prompt.

## Composer
- [ ] With a live session bound, type into composer + Enter. Verify the prompt appears in the terminal as input.
- [ ] With no live session bound, composer is disabled and shows the inactive placeholder.

## Persistence and restore
- [ ] Quit CastCodes mid-conversation. Restart. Open panel. Prior conversation appears in the list. Click it. Transcript reproduces from disk.
- [ ] Past session view is read-only — composer is disabled with a note.

## Model picker / new chat
- [ ] Click "New chat" with default agent/model. A new terminal pane opens with the appropriate CLI command. Once `session_start` arrives, the panel auto-binds.

## Error handling
- [ ] Force a malformed event (edit a fixture in dev, or send a synthetic malformed payload via a test helper). After ≥3 skipped events, the error banner appears.
- [ ] Close a terminal mid-session. Panel transcript shows "Session ended"; conversation moves to past-list.

## Boundary and rebrand
- [ ] `./script/check_cli_chat_boundary` passes.
- [ ] `./script/check_rebrand` passes.
- [ ] `cargo check -p warp-app --bin cast-codes --features gui` passes clean.
- [ ] `cargo test -p warp-app cli_chat:: --features gui` passes (unit + persistence).

## Misc
- [ ] Toggle Chat Panel keybinding works (default `Cmd+Shift+J` or whatever was chosen).
- [ ] Feature flag `CastCodesChatPanel` set to false hides the panel and disables the menu item.
- [ ] No new `warp.dev` host appears in network logs while panel is in use.
```

- [ ] **Step 2: Commit**

```bash
git add specs/castcodes-chat-panel/CHECKLIST.md
git commit -m "docs(cli_chat): add manual verification checklist"
```

### Task 8.4: Final build + workspace verification

**Files:** None.

- [ ] **Step 1: Full build**

Run: `cargo check -p warp-app --bin cast-codes --features gui`
Expected: PASS.

- [ ] **Step 2: Full test run**

Run: `cargo test -p warp-app cli_chat:: --features gui`
Expected: PASS.

- [ ] **Step 3: Clippy**

Run: `cargo clippy -p warp-app --bin cast-codes --features gui -- -D warnings`
Expected: PASS. Fix any clippy issues that originate in `app/src/cli_chat/`; ignore upstream pre-existing warnings outside the module.

- [ ] **Step 4: All guards**

```bash
./script/check_rebrand
./script/check_cli_chat_boundary
```

Both: PASS.

- [ ] **Step 5: Manual checklist**

Walk through `specs/castcodes-chat-panel/CHECKLIST.md`. Tick everything off.

- [ ] **Step 6: Final commit if any fixes**

```bash
git add -A
git commit -m "fix(cli_chat): final clippy + guard cleanup"
```

---

## Self-review

Spec coverage check — every requirement in PRODUCT.md / TECH.md maps to a task:

- Live transcript rendering → Phase 2 (Tasks 2.1–2.3).
- Persistence + restore → Phase 3 (Tasks 3.1–3.3).
- Conversation list + past-session view → Phase 4.
- Composer wired to rich input → Phase 5.
- Model picker + new-chat flow → Phase 6.
- Empty / error / settings → Phase 7.
- Fork-local boundary CI guard → Phase 8 (Task 8.1).
- Rebrand → Phase 8 (Task 8.2).
- Manual verification → Phase 8 (Task 8.3, CHECKLIST.md).
- Feature flag → Phase 1 (Task 1.2).
- Workspace toggle/keybinding → Phase 1 (Task 1.4).

Type consistency check — `ChatEntry`, `ChatEntryKind`, `ChatConversation`, `AgentKind`, `ConversationBinding`, `ChatModel`, `ChatModelEvent` are defined in Phases 1–2 and referenced consistently afterward.

Outstanding ambiguities deliberately left for the executing agent to resolve by reading the codebase (not placeholders — concrete pointers to the file to consult):

- Exact warpui `View` / `ViewContext` / `Render` / observer API: mirror `app/src/workspace/view/conversation_list/view.rs`.
- Exact `CLIAgentSessionsModel` event-emit variant name: read `app/src/terminal/cli_agent_sessions/mod.rs` lines 234–290.
- Exact "open new terminal with command" helper: grep `app/src/workspace/` for `open.*terminal.*command`.
- CastCodes data-dir helper name: grep `crates/warp_files/` and `app/src/` for `.cast-codes` and `cast_codes_data_dir`.
- Settings-row registration API: mirror existing rows in `app/src/settings_view/ai_page.rs`.

These are research-and-mirror steps, not placeholder text. Each task that needs them gives the file and the symbol to look for.
