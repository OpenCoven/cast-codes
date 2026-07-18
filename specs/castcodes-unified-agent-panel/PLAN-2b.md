# Unified Agent Panel — Phase 2b Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a new, feature-flagged **unified agent panel** (`app/src/agent_panel/`) — a merged conversation list (both CLI and daemon backends) + `agent_transcript` transcript + composer shell — reachable by one keybinding, without touching the two existing panels.

**Architecture:** A new `AgentPanelView` mirrors `cli_chat::ChatPanelView`'s structure (holds the singleton `ModelHandle<ChatModel>` + a `SubmittableTextInput` composer) but renders through its own `agent_panel::view` helpers generalized over `ConversationBackend` (landed in 2a). The merged list is genuinely cross-backend because 2a's `stream-history.json → sqlite` migration is wired into `ChatModel` bootstrap here, so migrated `coven-code` `Daemon` conversations appear alongside CLI ones. The composer is a *shell*: enabled for live CLI conversations (reusing the PTY submit path), disabled with a placeholder for daemon/unbound conversations. **Per-backend composer routing and the live daemon conversation source are explicitly deferred to 2c.** The old `cli_chat` and `ai_assistant` panels are left byte-for-byte untouched (deleted wholesale in 2d after parity); the temporary view-helper duplication is intentional and short-lived.

**Tech Stack:** Rust, warpui (upstream GPUI-derived UI framework — `View`/`Entity`/`Element`/`Flex`), `warp_features::FeatureFlag`, rusqlite (via 2a's `ChatStore`), the `cast-agent` Cargo feature.

**Depends on:** 2a (PR #205) — `ConversationBackend`, `ChatConversation.backend`, schema v2 `backend` column, `history_migration::migrate_stream_history_file`. Rebase this on top of 2a before starting.

**Out of scope (later sub-phases):**
- **2c** — per-backend composer routing (daemon send via `stream_agent_events`), the live Coven-daemon conversation source, and the stream-mode prompt-delivery detail.
- **2d** — deleting `cli_chat::ChatPanelView` + the `ai_assistant` coven-stream section; making the unified panel the default.

---

## File Structure

| File | Responsibility | Create/Modify |
|---|---|---|
| `crates/warp_features/src/lib.rs` | Add `FeatureFlag::UnifiedAgentPanel` variant | Modify (`:855` area) |
| `app/src/agent_panel/mod.rs` | Module root: `AgentPanelView` struct, `Entity`/`View` impls, constructors, composer wiring | Create |
| `app/src/agent_panel/feature_flag.rs` | `is_enabled()` accessor mirroring `cli_chat::feature_flag` | Create |
| `app/src/agent_panel/strings.rs` | User-facing strings (panel title, toggle label, composer placeholders, badge labels) | Create |
| `app/src/agent_panel/view/mod.rs` | `pub mod` re-exports for the render helpers | Create |
| `app/src/agent_panel/view/header.rs` | Header bar (label + New chat), generalized | Create |
| `app/src/agent_panel/view/conversation_list.rs` | Merged list with per-row backend badge | Create |
| `app/src/agent_panel/view/transcript.rs` | Transcript pane via `agent_transcript::render_entries` | Create |
| `app/src/agent_panel/view/composer.rs` | Composer shell (active for live CLI, placeholder otherwise) | Create |
| `app/src/agent_panel/view_tests.rs` | Layout-safety + badge unit tests | Create |
| `app/src/cli_chat/model.rs` | Call `migrate_stream_history_file` in `ChatModel` bootstrap | Modify (`:73` `new`) |
| `app/src/cli_chat/model_tests.rs` | Test: bootstrap surfaces migrated daemon conversations | Modify |
| `app/src/lib.rs` (or the module-declaring root) | `pub mod agent_panel;` (gated `#[cfg(not(target_family = "wasm"))]`) | Modify |
| `app/src/workspace/action.rs` | `WorkspaceAction::ToggleUnifiedAgentPanel` + `SubmitAgentPrompt { text }` | Modify (`:268` area) |
| `app/src/workspace/view.rs` | Panel field, flag-gated construction, render path, action handlers | Modify (`:1030`, `:2753`, `:19260`, `:20740` areas) |
| `app/src/workspace/mod.rs` | Keybinding registration | Modify (`:735` area) |
| `app/src/util/bindings.rs` | Keystroke for the new `CustomAction` | Modify (`:440` area) |
| `app/src/app_menus.rs` | Menu item (flag-gated) | Modify (`:402` area) |
| `script/check_cli_chat_boundary` | Add `app/src/agent_panel` to `TARGETS` | Modify (`:7`) |

**Reference — the existing panel to mirror (do not modify in 2b):**
- `app/src/cli_chat/view/mod.rs:29-142` — `ChatPanelView` struct + `View::render` composition.
- `app/src/cli_chat/view/{conversation_list,composer,transcript,model_picker}.rs` — the render free-functions taking `&ChatPanelView` + `&AppContext`.
- Registration: `app/src/workspace/view.rs:1030` (field), `:2753-2760` (construction), `:19260-19276` (render), `:20740-20823` (action handlers); `app/src/workspace/mod.rs:735-750` (keybinding); `app/src/util/bindings.rs:440-446` (keystroke); `app/src/app_menus.rs:402-411` (menu).

---

## Task 1: New `UnifiedAgentPanel` feature flag

**Files:**
- Modify: `crates/warp_features/src/lib.rs:855`
- Create: `app/src/agent_panel/feature_flag.rs`

The flag gates the whole panel; `FeatureFlag` variants index positional arrays (`FLAG_STATES[*self as usize]`), so appending a variant is safe and requires no other array edits.

- [ ] **Step 1: Add the enum variant.** In `crates/warp_features/src/lib.rs`, immediately after `CastCodesChatPanel,` (line 855):

```rust
    /// Gates the unified CastCodes agent panel: one surface merging the
    /// CLI-agent chat and the Coven-daemon agent conversations behind a
    /// single list + transcript + composer. CastCodes-only; never enabled
    /// in upstream Warp builds.
    UnifiedAgentPanel,
```

- [ ] **Step 2: Create the accessor.** `app/src/agent_panel/feature_flag.rs`:

```rust
//! Convenience accessor for the `UnifiedAgentPanel` feature flag. Mirrors
//! `crate::cli_chat::feature_flag`.

use warp_core::features::FeatureFlag;

pub fn is_enabled() -> bool {
    FeatureFlag::UnifiedAgentPanel.is_enabled()
}
```

- [ ] **Step 3: Build the features crate.**

Run: `cargo check -p warp_features`
Expected: PASS (new variant compiles; positional arrays auto-size).

- [ ] **Step 4: Commit.**

```bash
git add crates/warp_features/src/lib.rs app/src/agent_panel/feature_flag.rs
git commit -S -m "feat(agent_panel): add UnifiedAgentPanel feature flag"
```

---

## Task 2: Module scaffold + strings

**Files:**
- Create: `app/src/agent_panel/mod.rs`, `app/src/agent_panel/strings.rs`, `app/src/agent_panel/view/mod.rs`
- Modify: `app/src/lib.rs` (module declaration)

- [ ] **Step 1: Create the strings module.** `app/src/agent_panel/strings.rs`:

```rust
//! User-facing strings for the unified agent panel. Fork-local; no Warp
//! naming (guarded by check_rebrand).

pub const PANEL_TITLE: &str = "Agent";
pub const TOGGLE_MENU_ITEM: &str = "Toggle Agent Panel";
pub const NEW_CHAT_LABEL: &str = "New chat";

pub const COMPOSER_PLACEHOLDER_ACTIVE: &str = "Message the running agent…";
pub const COMPOSER_PLACEHOLDER_INACTIVE: &str =
    "Select a live CLI conversation to send input, or run a CLI agent in a terminal.";

/// Row badge labels distinguishing the two backends in the merged list.
pub const BADGE_CLI: &str = "CLI";
pub const BADGE_DAEMON: &str = "Coven";
```

- [ ] **Step 2: Create the view submodule root.** `app/src/agent_panel/view/mod.rs`:

```rust
pub mod composer;
pub mod conversation_list;
pub mod header;
pub mod transcript;
```

- [ ] **Step 3: Create the module root (skeleton — filled in Task 3).** `app/src/agent_panel/mod.rs`:

```rust
//! The unified CastCodes agent panel — one list + transcript + composer over
//! both CLI-agent and Coven-daemon conversations. Feature-flagged behind
//! [`FeatureFlag::UnifiedAgentPanel`]; the old `cli_chat` and `ai_assistant`
//! panels are retired in favor of this in a later sub-phase (2d).

pub mod feature_flag;
#[allow(dead_code)]
pub mod strings;
#[allow(dead_code)]
pub mod view;

#[cfg(test)]
mod view_tests;

pub use view::mod_view::AgentPanelView;
```

  Note: the `pub use` line is provisional; Task 3 defines `AgentPanelView` in `mod.rs` directly and this line becomes `// (defined below)`. Keep the module list.

- [ ] **Step 4: Declare the module.** In `app/src/lib.rs`, next to the `cli_chat` module declaration (grep `pub mod cli_chat`), add — matching the same `#[cfg]` gating `cli_chat` uses (the panel is non-wasm; confirm the exact cfg on the `cli_chat` line and copy it):

```rust
#[cfg(not(target_family = "wasm"))]
pub mod agent_panel;
```

- [ ] **Step 5: Build.**

Run: `cargo check -p warp-app --features cast-agent`
Expected: PASS (empty scaffold; `dead_code` allowed).

- [ ] **Step 6: Commit.**

```bash
git add app/src/agent_panel/ app/src/lib.rs
git commit -S -m "feat(agent_panel): module scaffold + strings"
```

---

## Task 3: `AgentPanelView` + generalized render helpers

**Files:**
- Modify: `app/src/agent_panel/mod.rs`
- Create: `app/src/agent_panel/view/{header,conversation_list,transcript,composer}.rs`

The render helpers are free functions taking `&AgentPanelView` + `&AppContext`, exactly like `cli_chat`'s. Bodies are adapted from the referenced `cli_chat` files, changing the view type and reading `view.chat_model.as_ref(app)` accessors that already exist on the 2a-generalized `ChatModel`.

- [ ] **Step 1: Define the view.** Replace the provisional `pub use` in `app/src/agent_panel/mod.rs` with the struct + impls (mirror `cli_chat/view/mod.rs:29-142`):

```rust
use warpui::{
    AppContext, Element, Entity, ModelHandle, View, ViewContext, ViewHandle,
    // + Flex, MainAxisSize, CrossAxisAlignment, Expanded — copy the exact
    //   import list from cli_chat/view/mod.rs.
};

use crate::cli_chat::model::{ChatModel, ChatModelEvent};
use crate::view_components::submittable_text_input::{
    SubmittableTextInput, SubmittableTextInputEvent,
};
use crate::workspace::action::WorkspaceAction;

pub struct AgentPanelView {
    pub(crate) chat_model: ModelHandle<ChatModel>,
    pub(crate) composer_input: ViewHandle<SubmittableTextInput>,
}

impl AgentPanelView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let chat_model = ChatModel::handle(ctx);
        ctx.subscribe_to_model(&chat_model, |_me, _model, _event: &ChatModelEvent, ctx| {
            ctx.notify(); // re-render on any model change
        });
        let composer_input = Self::create_composer(ctx);
        Self { chat_model, composer_input }
    }

    /// Test/explicit-model constructor (mirrors `ChatPanelView::with_model`).
    pub fn with_model(chat_model: ModelHandle<ChatModel>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&chat_model, |_me, _model, _event: &ChatModelEvent, ctx| {
            ctx.notify();
        });
        let composer_input = Self::create_composer(ctx);
        Self { chat_model, composer_input }
    }

    fn create_composer(ctx: &mut ViewContext<Self>) -> ViewHandle<SubmittableTextInput> {
        let composer = ctx.add_typed_action_view(SubmittableTextInput::new);
        composer.update(ctx, |input, ctx| {
            input.set_placeholder_text(strings::COMPOSER_PLACEHOLDER_ACTIVE, ctx);
        });
        ctx.subscribe_to_view(&composer, |_me, _input, event, ctx| match event {
            SubmittableTextInputEvent::Submit(text) => {
                ctx.dispatch_action(WorkspaceAction::SubmitAgentPrompt { text: text.clone() });
            }
            SubmittableTextInputEvent::Escape => {}
        });
        composer
    }
}

impl Entity for AgentPanelView {
    type Event = ();
}

impl View for AgentPanelView {
    fn ui_name() -> &'static str {
        "AgentPanelView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let header = view::header::render(self, app);
        let list = view::conversation_list::render_list(self, app);
        let transcript = view::transcript::render_panel(self, app);
        let composer = view::composer::render_composer(self, app);

        // Right column: transcript fills, composer pinned at bottom.
        let right_column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Expanded::new(1.0, transcript).finish())
            .with_child(composer)
            .finish();

        let body = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(list)
            .with_child(Expanded::new(1.0, right_column).finish())
            .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header)
            .with_child(Expanded::new(1.0, body).finish())
            .finish()
    }
}
```

  Copy the exact `warpui` import paths and any `SubmittableTextInput::new` construction nuance (`add_typed_action_view` vs `add_view`) from `cli_chat/view/mod.rs:41-100` — that file is the ground truth for the framework calls; only the dispatched action (`SubmitAgentPrompt`) and placeholder string differ.

- [ ] **Step 2: Header helper.** `app/src/agent_panel/view/header.rs` — adapt `cli_chat/view/model_picker.rs:32-129`:

```rust
use warpui::{AppContext, Element};

use super::super::strings;
use super::super::AgentPanelView;

/// Header bar: active-conversation label on the left, "New chat" on the right.
/// 32px tall (match `cli_chat` model_picker BAR_HEIGHT).
pub fn render(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    // Body: copy model_picker::render (lines 32-129), replacing
    //   `conv.agent.display_name()` → `conv.backend.display_name()` (already
    //   done in 2a) and the New-chat action with the agent-panel equivalent.
    // The label uses conv.backend.display_name() + conv.last_model.
    unimplemented!("adapt from cli_chat/view/model_picker.rs:32-129")
}
```

  Replace `unimplemented!` with the adapted body. The "New chat" button may keep dispatching `WorkspaceAction::CliChatNewChat { command }` for now (CLI-only new chats); daemon new-conversation creation is 2c. Do not introduce a new action for it in 2b.

- [ ] **Step 3: Conversation list with backend badge.** `app/src/agent_panel/view/conversation_list.rs` — adapt `cli_chat/view/conversation_list.rs:29-103`:

```rust
use warpui::{AppContext, Element};

use super::super::strings;
use super::super::AgentPanelView;
use crate::cli_chat::conversation::{ChatConversation, ConversationBackend};

pub fn render_list(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    // Copy conversation_list::render_list (lines 29-65): 200px ConstrainedBox,
    // Flex::column of rows from
    //   view.chat_model.as_ref(app).conversations_sorted_by_recency().
    // Each row: render_conversation_item(conv, font_family, font_size).
    // Row click still dispatches WorkspaceAction::OpenChatSession { session_id }.
    unimplemented!("adapt from cli_chat/view/conversation_list.rs:29-65")
}

/// Short badge label for a conversation's backend.
pub fn backend_badge(backend: &ConversationBackend) -> &'static str {
    match backend {
        ConversationBackend::Cli(_) => strings::BADGE_CLI,
        ConversationBackend::Daemon { .. } => strings::BADGE_DAEMON,
    }
}

fn render_conversation_item(
    conv: &ChatConversation,
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    // Copy the body from conversation_list.rs:71-103, and prepend a small
    // badge element (a Container with backend_badge(&conv.backend) text) to
    // the row so CLI vs Coven conversations are visually distinguished.
    unimplemented!("adapt from cli_chat/view/conversation_list.rs:71-103 + badge")
}
```

- [ ] **Step 4: Transcript helper.** `app/src/agent_panel/view/transcript.rs` — adapt `cli_chat/view/transcript.rs:26-83`:

```rust
use warpui::{AppContext, Element};

use super::super::AgentPanelView;
use crate::agent_transcript::view::transcript::render_entries;
use crate::cli_chat::conversation::ChatConversation;

pub fn render_panel(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    // Copy transcript::render_panel (lines 26-83): resolve binding via
    //   view.chat_model.as_ref(app).binding(), look up chat.conversation(id),
    //   and render_entries(&conv.entries, font_family, font_size).
    // Keep the skipped-event error-banner behavior. Empty-state when unbound.
    unimplemented!("adapt from cli_chat/view/transcript.rs:26-83")
}
```

- [ ] **Step 5: Composer shell.** `app/src/agent_panel/view/composer.rs` — adapt `cli_chat/view/composer.rs:29-63`, but gate "active" on a **live CLI** binding (daemon input is 2c):

```rust
use warpui::{AppContext, ChildView, Container, Element};

use super::super::strings;
use super::super::AgentPanelView;
use crate::cli_chat::conversation::{ConversationBackend, ConversationBinding};

pub fn render_composer(view: &AgentPanelView, app: &AppContext) -> Box<dyn Element> {
    let chat = view.chat_model.as_ref(app);
    let active_cli = matches!(chat.binding(), ConversationBinding::Live { session_id, .. }
        if matches!(
            chat.conversation(session_id).map(|c| &c.backend),
            Some(ConversationBackend::Cli(_))
        ));

    if active_cli {
        // Active: render the real text input (copy render_active_composer,
        // composer.rs:34-46).
        Container::new(ChildView::new(&view.composer_input).finish()).finish()
    } else {
        // Shell: disabled placeholder (copy render_inactive_placeholder,
        // composer.rs:48-63) using strings::COMPOSER_PLACEHOLDER_INACTIVE.
        render_inactive_placeholder(app)
    }
}

fn render_inactive_placeholder(app: &AppContext) -> Box<dyn Element> {
    unimplemented!("adapt from cli_chat/view/composer.rs:48-63")
}
```

- [ ] **Step 6: Build.**

Run: `cargo check -p warp-app --features cast-agent`
Expected: PASS after the `unimplemented!` bodies are filled. (Fill them, then re-run — `unimplemented!` compiles but Task 4's layout test would panic, so complete the bodies here.)

- [ ] **Step 7: Commit.**

```bash
git add app/src/agent_panel/mod.rs app/src/agent_panel/view/
git commit -S -m "feat(agent_panel): AgentPanelView + generalized list/transcript/composer helpers"
```

---

## Task 4: Layout-safety + badge tests

**Files:**
- Create: `app/src/agent_panel/view_tests.rs`

This adds the `View` layout-safety coverage that Phase 1 deferred (per the create-pr skill's "UI components need layout validation tests").

- [ ] **Step 1: Write the failing tests.** `app/src/agent_panel/view_tests.rs`:

```rust
use crate::agent_panel::view::conversation_list::backend_badge;
use crate::agent_panel::AgentPanelView;
use crate::cli_chat::conversation::{AgentKind, ConversationBackend};

#[test]
fn backend_badge_distinguishes_cli_and_daemon() {
    assert_eq!(backend_badge(&ConversationBackend::Cli(AgentKind::Claude)), "CLI");
    assert_eq!(
        backend_badge(&ConversationBackend::Daemon { harness: "coven-code".into() }),
        "Coven"
    );
}

#[test]
fn agent_panel_view_lays_out_without_panicking() {
    use warpui::App;
    use warp::test_util::{terminal::initialize_app_for_terminal_view, add_window_with_terminal};

    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let term = add_window_with_terminal(&mut app, None);

        // Construct the panel with the singleton model and render it — must
        // not panic with an empty (unbound) model.
        term.update(&mut app, |_view, ctx| {
            let panel = ctx.add_view(AgentPanelView::new);
            let _ = panel; // constructing + subscribing must not panic
        });
    })
}
```

  If `AgentPanelView::new` needs a window/view context that `add_view` doesn't give in this harness, follow the exact pattern the codebase already uses for panel layout tests (grep `add_window_with_terminal` for a `View` that is constructed and rendered in a test), and render via a `ChildView` to force a layout pass. Match whatever the existing `cli_chat`/`ai_assistant` view tests do; do not invent a new harness.

- [ ] **Step 2: Register the test module.** Confirm `mod.rs` has `#[cfg(test)] mod view_tests;` (added in Task 2).

- [ ] **Step 3: Run — verify pass.**

Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(agent_panel::view_tests)'`
Expected: PASS (both tests).

- [ ] **Step 4: Commit.**

```bash
git add app/src/agent_panel/view_tests.rs app/src/agent_panel/mod.rs
git commit -S -m "test(agent_panel): backend badge + layout-safety coverage"
```

---

## Task 5: Wire the `stream-history.json` migration into `ChatModel` bootstrap

**Files:**
- Modify: `app/src/cli_chat/model.rs:73` (`ChatModel::new`)
- Modify: `app/src/cli_chat/model_tests.rs`

2a's `migrate_stream_history_file` exists but has no caller. Call it once at model bootstrap, *before* `load_existing_history()`, so migrated `coven-code` `Daemon` conversations are read into the in-memory list — this is what makes the 2b list genuinely merged.

- [ ] **Step 1: Write the failing test.** Add to `app/src/cli_chat/model_tests.rs` (use whatever store-injection the existing model tests use; if `ChatModel::new` always opens the real store, add a test-only constructor `ChatModel::with_store(store, ctx)` that the migration + load path share, and route `new` through it):

```rust
#[test]
fn bootstrap_surfaces_migrated_daemon_conversations() {
    use crate::cli_chat::conversation::ConversationBackend;
    use crate::cli_chat::history_migration::{migrate_history_records, HistoryRecord};
    use crate::cli_chat::store::ChatStore;

    // A store already carrying a migrated daemon conversation.
    let store = ChatStore::open_in_memory().unwrap();
    migrate_history_records(
        &store,
        &[HistoryRecord { conversation_id: "d1".into(), text: "hi from coven".into() }],
        chrono::Utc::now(),
    )
    .unwrap();

    // Building a model over this store must load the daemon conversation into
    // the merged list.
    let model = ChatModel::from_store_for_test(store);
    let convs = model.conversations_sorted_by_recency();
    let daemon = convs.iter().find(|c| c.session_id == "d1").expect("migrated convo present");
    assert!(matches!(daemon.backend, ConversationBackend::Daemon { .. }));
}
```

  Adapt the constructor name to the codebase's existing test seam. If none exists, the minimal seam is a private `fn load_from_store(&mut self, store)` that both `new` and the test drive; keep it behavior-preserving for the existing model tests.

- [ ] **Step 2: Run — verify fail.**

Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat::model_tests::bootstrap_surfaces_migrated)'`
Expected: FAIL (migration not called at bootstrap / no test seam).

- [ ] **Step 3: Wire the migration.** In `ChatModel::new` (`model.rs:73`), after the store is opened and before `load_existing_history()`:

```rust
        if let Some(store) = self.store.as_ref() {
            crate::cli_chat::history_migration::migrate_stream_history_file(store, chrono::Utc::now());
        }
```

  (Order matters: migrate → then `load_existing_history()` reads the newly-inserted rows. If `new` loads inline, insert the migration call immediately before that load. `migrate_stream_history_file` is a no-op when the file is absent or already `.migrated`, so it is safe on every startup.)

- [ ] **Step 4: Run — verify pass.**

Run: `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat::model_tests)'`
Expected: PASS (new test + all pre-existing model tests unchanged).

- [ ] **Step 5: Commit.**

```bash
git add app/src/cli_chat/model.rs app/src/cli_chat/model_tests.rs
git commit -S -m "feat(cli_chat): migrate stream-history into the model at bootstrap"
```

---

## Task 6: Workspace registration — field, actions, render, keybinding

**Files:**
- Modify: `app/src/workspace/action.rs`, `app/src/workspace/view.rs`, `app/src/workspace/mod.rs`, `app/src/util/bindings.rs`, `app/src/app_menus.rs`

Mirror the `cli_chat` registration exactly (references in the File Structure table). All additions are `#[cfg(not(target_family = "wasm"))]` like their `cli_chat` counterparts.

- [ ] **Step 1: Actions.** In `app/src/workspace/action.rs` (near the `ToggleCliChatPanel` block, ~line 268):

```rust
    #[cfg(not(target_family = "wasm"))]
    ToggleUnifiedAgentPanel,

    #[cfg(not(target_family = "wasm"))]
    SubmitAgentPrompt {
        text: String,
    },
```

- [ ] **Step 2: Panel field.** In `app/src/workspace/view.rs` (near `:1030`):

```rust
    #[cfg(not(target_family = "wasm"))]
    agent_panel_view: Option<ViewHandle<crate::agent_panel::AgentPanelView>>,
```

  And a workspace-state flag `is_agent_panel_open: bool` alongside `is_cli_chat_panel_open` (grep that field's declaration in the workspace-state struct and add the sibling).

- [ ] **Step 3: Construction.** In the workspace constructor (near `:2753`), gated on the new flag:

```rust
    #[cfg(not(target_family = "wasm"))]
    let agent_panel_view = if crate::agent_panel::feature_flag::is_enabled() {
        Some(ctx.add_view(crate::agent_panel::AgentPanelView::new))
    } else {
        None
    };
```

  Assign it into the struct literal next to `cli_chat_panel_view`.

- [ ] **Step 4: Render path.** In the panels render block (near `:19260`), mirror the `cli_chat` block; render at the right dock, 360px:

```rust
    #[cfg(not(target_family = "wasm"))]
    {
        if self.current_workspace_state.is_agent_panel_open {
            if let Some(panel) = &self.agent_panel_view {
                let content = self.render_panel(
                    app,
                    ChildView::new(panel).finish(),
                    &PanelPosition::Right,
                );
                panels_view = panels_view.with_child(
                    ConstrainedBox::new(content).with_width(360.0).finish(),
                );
            }
        }
    }
```

- [ ] **Step 5: Action handlers.** In the `WorkspaceAction` dispatch (near `:20740`), add handlers mirroring `ToggleCliChatPanel`/`SubmitChatPrompt`:
  - `ToggleUnifiedAgentPanel` → flip `is_agent_panel_open`, focus the panel when opening (copy `ToggleCliChatPanel`, `:20740-20763`).
  - `SubmitAgentPrompt { text }` → **2b routing = CLI-only.** Reuse the exact CLI path `SubmitChatPrompt` uses (`:20782-20808`): look up the live binding on `ChatModel`, and if it is a live `Cli` conversation, call `submit_text_to_cli_agent_pty(...)`. For a daemon/None binding, log a debug line and no-op (daemon send is 2c). Extract the shared PTY-submit into a helper both handlers call, or duplicate the few lines with a `// 2c: route Daemon here` marker.

- [ ] **Step 6: Keybinding.** In `app/src/workspace/mod.rs` (near `:735`), add an `EditableBinding` mirroring the cli_chat one, with a distinct action and keystroke so both panels are reachable during dogfood:

```rust
    #[cfg(not(target_family = "wasm"))]
    EditableBinding::new(
        "workspace:toggle_unified_agent_panel",
        BindingDescription::new(crate::agent_panel::strings::TOGGLE_MENU_ITEM),
        WorkspaceAction::ToggleUnifiedAgentPanel,
    )
    .with_context_predicate(id!("Workspace"))
    .with_group(bindings::BindingGroup::Navigation.as_str())
    .with_enabled(crate::agent_panel::feature_flag::is_enabled)
    .with_custom_action(CustomAction::ToggleUnifiedAgentPanel)
    .with_mac_key_binding("cmd-shift-U")
    .with_linux_or_windows_key_binding("ctrl-shift-U"),
```

  Add `CustomAction::ToggleUnifiedAgentPanel` to the `CustomAction` enum (grep its definition) and its keystroke arm in `app/src/util/bindings.rs` (near `:440`):

```rust
    CustomAction::ToggleUnifiedAgentPanel => {
        if OperatingSystem::get().is_mac() {
            Keystroke::parse("cmd-shift-U").ok()
        } else {
            Keystroke::parse("ctrl-shift-U").ok()
        }
    }
```

- [ ] **Step 7: Menu item.** In `app/src/app_menus.rs` (near `:402`), add a flag-gated item mirroring the cli_chat menu block, using `CustomAction::ToggleUnifiedAgentPanel`.

- [ ] **Step 8: Build + smoke.**

Run: `cargo check -p warp-app --bin cast-codes --features gui,cast-agent`
Expected: PASS. (The full app binary must link — action/keybinding/menu wiring is exercised at compile time.)

- [ ] **Step 9: Commit.**

```bash
git add app/src/workspace/ app/src/util/bindings.rs app/src/app_menus.rs
git commit -S -m "feat(agent_panel): register unified panel — field, actions, keybinding, menu"
```

---

## Task 7: Fork-local boundary guard + full gates

**Files:**
- Modify: `script/check_cli_chat_boundary:7`

- [ ] **Step 1: Extend the boundary guard.** In `script/check_cli_chat_boundary`, add `app/src/agent_panel` to `TARGETS`:

```bash
TARGETS='app/src/cli_chat app/src/agent_transcript app/src/agent_panel'
```

- [ ] **Step 2: Run the guards.**

```bash
./script/check_cli_chat_boundary
./script/check_ai_attribution
./script/check_rebrand
```
Expected: all pass (`agent_panel` references no Warp-owned infra; `strings.rs` uses fork-local naming).

- [ ] **Step 3: Lint + fmt.**

```bash
cargo fmt -p warp-app
cargo clippy -p warp-app --features cast-agent --all-targets -- -D warnings
```
Expected: clean.

- [ ] **Step 4: Full behavior + layout regression.**

```bash
cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat) or test(agent_panel) or test(agent_transcript)'
```
Expected: PASS — the entire pre-existing `cli_chat`/`agent_transcript` suite unchanged (proves 2b didn't regress the old panel), plus the new `agent_panel` tests.

- [ ] **Step 5: Commit any fmt-only changes.**

```bash
git add -A
git commit -S -m "chore(agent_panel): boundary guard + fmt"
```

---

## Done criteria (2b)

- A new `FeatureFlag::UnifiedAgentPanel` gates `app/src/agent_panel/AgentPanelView`, reachable via `cmd-shift-U` / `ctrl-shift-U` and the menu, rendered at the right dock (360px) — the two old panels untouched and still reachable.
- The panel shows a **merged conversation list** (CLI + migrated daemon conversations, each with a backend badge), the shared `agent_transcript` transcript, and a composer shell (active for live CLI conversations, placeholder otherwise).
- `stream-history.json` is migrated into the model at bootstrap, so daemon conversations appear in the list.
- `AgentPanelView` has layout-safety coverage; the full `cli_chat`/`agent_transcript` suite passes unchanged; clippy `-D warnings` clean; `check_cli_chat_boundary` (now covering `agent_panel`), `check_ai_attribution`, `check_rebrand` pass; every commit signed.
- **Not** in 2b: per-backend composer routing, the live daemon conversation source, daemon send / stream-mode prompt delivery (2c); deletion of the old panels / making this the default (2d).

---

## Self-Review

**Spec coverage (DESIGN §"2b — Unified panel view + merged list"):** new `agent_panel` view (Tasks 2–3) ✓; conversation list spanning both backends (Task 3 badge + Task 5 migration wiring) ✓; `agent_transcript` transcript (Task 3) ✓; composer shell (Task 3) ✓; feature flag (Task 1) + keybinding (Task 6) ✓; boundary guard extension called out in DESIGN §Error-handling (Task 7) ✓; layout-safety test that DESIGN §Testing flags as the Phase-1 deferral (Task 4) ✓. Deferrals (routing, daemon source) are explicitly fenced to 2c.

**Placeholder scan:** the `unimplemented!("adapt from …:LINES")` markers in Task 3 are deliberate copy-anchors to exact source ranges, not open-ended TODOs — each names the precise file+lines to port and the one substitution to make. Every other step has concrete code or an exact command.

**Type consistency:** `AgentPanelView { chat_model: ModelHandle<ChatModel>, composer_input: ViewHandle<SubmittableTextInput> }` is used identically across Tasks 3, 4, 6. `WorkspaceAction::{ToggleUnifiedAgentPanel, SubmitAgentPrompt}` and `CustomAction::ToggleUnifiedAgentPanel` are defined (Task 6 steps 1/6) before use. `backend_badge(&ConversationBackend) -> &'static str` is defined in Task 3 and tested in Task 4. `ChatModel` accessors (`binding`, `conversation`, `conversations_sorted_by_recency`, `handle`) match the 2a-generalized model surfaced by the Explore map.

**Risk flagged for the implementer:** the `ChatModel::new` test seam (Task 5 Step 1) and the exact `warpui` view-test harness (Task 4 Step 1) must match existing codebase patterns — both steps say to mirror existing tests rather than invent a seam. If `cli_chat` is not yet bound to a *live* `CLIAgentSessionsModel` (DESIGN Risk 2), the CLI composer's "active" branch may not light up outside an integration context; that wiring is confirmed/completed in 2c and does not block the 2b shell.
