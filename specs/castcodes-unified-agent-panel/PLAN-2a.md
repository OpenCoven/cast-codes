# Phase 2a Implementation Plan — Shared conversation core (`ConversationBackend` + persistence migration)

> **For agentic workers:** Implement task-by-task with TDD. Each step is one small action. Commit at each task boundary (`git commit -S`). Steps use checkbox (`- [ ]`) syntax. Verify each command's expected result before moving on.

**Goal:** Generalize `cli_chat`'s conversation model + sqlite store over a `ConversationBackend` enum (`Cli(AgentKind) | Daemon{harness}`) and migrate the daemon panel's `stream-history.json` into sqlite — a behavior-preserving foundation the unified panel (2b–2d) builds on.

**Architecture:** Replace `ChatConversation.agent: AgentKind` with `backend: ConversationBackend`. Persist it via the existing `agent` column (harness/kind name) plus a new `backend` column (`'cli'`/`'daemon'`) added by a schema Version 2 migration. A one-time, non-destructive `stream-history.json → sqlite` data migration folds the daemon panel's text history in.

**Tech Stack:** Rust, `rusqlite`, `serde`, the `agent_transcript::ChatEntry` model (Phase 1).

**Spec:** `specs/castcodes-unified-agent-panel/DESIGN.md`. **Scope:** 2a only; 2b (unified view), 2c (composer routing + daemon source), 2d (entry point + deprecation) get their own plans after 2a lands.

**Pre-flight (every task):** feature branch; commits signed; before pushing run `./script/check_ai_attribution`, `./script/check_rebrand`, `./script/check_cli_chat_boundary`, `cargo clippy -p warp-app --features cast-agent -- -D warnings`.

---

## File structure

| Path | Responsibility | Change |
|---|---|---|
| `app/src/cli_chat/conversation.rs` | `ConversationBackend` enum; `ChatConversation.backend` | Modify |
| `app/src/cli_chat/store_schema.rs` | Version 2 migration: `backend` column | Modify |
| `app/src/cli_chat/store.rs` | Write/read `backend`; keep `agent` as the name column | Modify |
| `app/src/cli_chat/history_migration.rs` | One-time `stream-history.json → sqlite` migration | Create |
| `app/src/cli_chat/model.rs` + call sites | `conv.agent` → `conv.backend` | Modify |
| `app/src/cli_chat/*_tests.rs` | Update fixtures to `backend` | Modify |

`ConversationBackend` lives in `cli_chat::conversation` for 2a (2b promotes the core into `agent_panel`; keeping it here now minimizes churn and keeps `cli_chat`'s tests as the behavior gate).

---

## Task 1: `ConversationBackend` enum

**Files:** Modify `app/src/cli_chat/conversation.rs`; Test: `app/src/cli_chat/conversation_tests.rs`.

- [ ] **Step 1: Write the failing test.** Add to `conversation_tests.rs`:

```rust
#[test]
fn backend_round_trips_name_and_kind() {
    use crate::cli_chat::conversation::{AgentKind, ConversationBackend};
    let cli = ConversationBackend::Cli(AgentKind::Codex);
    assert_eq!(cli.name(), "codex");
    assert_eq!(cli.kind_str(), "cli");
    assert_eq!(ConversationBackend::from_persisted("codex", "cli"), cli);

    let daemon = ConversationBackend::Daemon { harness: "coven-code".into() };
    assert_eq!(daemon.name(), "coven-code");
    assert_eq!(daemon.kind_str(), "daemon");
    assert_eq!(ConversationBackend::from_persisted("coven-code", "daemon"), daemon);
}

#[test]
fn from_persisted_unknown_cli_agent_defaults_to_claude() {
    use crate::cli_chat::conversation::{AgentKind, ConversationBackend};
    // Forward-compatible: an unrecognized cli agent name falls back rather than panicking.
    assert_eq!(
        ConversationBackend::from_persisted("nope", "cli"),
        ConversationBackend::Cli(AgentKind::Claude)
    );
}
```

- [ ] **Step 2: Run — verify fail.** `cargo test -p warp-app --features cast-agent --lib cli_chat::conversation_tests::backend_round_trips 2>&1 | head` → FAIL (`ConversationBackend` not found).

- [ ] **Step 3: Implement the enum** in `conversation.rs` (below `AgentKind`):

```rust
/// The backend a conversation is driven by. Generalizes the panel over the
/// OSC-777 CLIs (which run in a terminal pane) and the Coven daemon (headless).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationBackend {
    /// An OSC-777 CLI agent running in a terminal pane.
    Cli(AgentKind),
    /// A Coven daemon lane (e.g. `coven-code`), driven headless via cast_agent.
    Daemon { harness: String },
}

impl ConversationBackend {
    /// Harness/kind name persisted in the `agent` column (`claude`, `codex`,
    /// `coven-code`, …).
    pub fn name(&self) -> String {
        match self {
            ConversationBackend::Cli(k) => k.as_protocol_str().to_string(),
            ConversationBackend::Daemon { harness } => harness.clone(),
        }
    }

    /// Discriminator persisted in the `backend` column.
    pub fn kind_str(&self) -> &'static str {
        match self {
            ConversationBackend::Cli(_) => "cli",
            ConversationBackend::Daemon { .. } => "daemon",
        }
    }

    /// Reconstruct from the two persisted columns. Unknown `cli` agent names
    /// fall back to `Claude` (forward-compatible); any non-`cli` kind is a
    /// daemon lane carrying the name verbatim.
    pub fn from_persisted(name: &str, kind: &str) -> Self {
        if kind == "cli" {
            ConversationBackend::Cli(AgentKind::from_protocol_str(name).unwrap_or(AgentKind::Claude))
        } else {
            ConversationBackend::Daemon { harness: name.to_string() }
        }
    }
}
```

- [ ] **Step 4: Add `AgentKind::from_protocol_str`** (inverse of `as_protocol_str`), in the `impl AgentKind` block:

```rust
    pub fn from_protocol_str(s: &str) -> Option<Self> {
        Some(match s {
            "claude" => AgentKind::Claude,
            "codex" => AgentKind::Codex,
            "gemini" => AgentKind::Gemini,
            "opencode" => AgentKind::OpenCode,
            _ => return None,
        })
    }
```

- [ ] **Step 5: Run — verify pass.** `cargo test -p warp-app --features cast-agent --lib cli_chat::conversation_tests::backend 2>&1 | tail` → PASS.

- [ ] **Step 6: Commit.**

```bash
git add app/src/cli_chat/conversation.rs app/src/cli_chat/conversation_tests.rs
git commit -S -m "feat(cli_chat): add ConversationBackend enum (Cli | Daemon)"
```

---

## Task 2: Generalize `ChatConversation` over the backend

**Files:** Modify `app/src/cli_chat/conversation.rs`, `app/src/cli_chat/model.rs`, and every `conv.agent` reader (compiler-driven).

- [ ] **Step 1: Change the field.** In `ChatConversation` (currently `pub agent: AgentKind`) →

```rust
    pub backend: ConversationBackend,
```
  and update `ChatConversation::new`:

```rust
    pub fn new(session_id: String, backend: ConversationBackend, now: DateTime<Utc>) -> Self {
        Self {
            session_id,
            backend,
            // ...existing fields unchanged (title, cwd, project, created_at,
            //    updated_at: now, status, entries: Vec::new(), last_model, ...)
        }
    }
```

- [ ] **Step 2: Compile to enumerate call sites.** Run `cargo check -p warp-app --features cast-agent 2>&1 | grep -E "error\[|no field \`agent\`|expected .AgentKind" | head -30`. Expect errors at each `conv.agent` / `ChatConversation::new(.., AgentKind::X, ..)` site (in `model.rs`, `store.rs`, `conversation.rs` helpers, tests).

- [ ] **Step 3: Fix `model.rs` conversation creation.** Where `ChatModel` builds a conversation from a CLI session (search `ChatConversation::new(` and `AgentKind::from_cli_agent`), wrap the resolved `AgentKind` in `ConversationBackend::Cli(...)`:

```rust
    let backend = ConversationBackend::Cli(agent_kind);
    let conv = ChatConversation::new(session_id, backend, now);
```
  and change any `conv.agent` reads to derive from `conv.backend` (e.g. a `model_picker` that needs the `AgentKind` matches `ConversationBackend::Cli(k) => …`, and treats `Daemon` as no CLI model picker).

- [ ] **Step 4: Update the tests** (`conversation_tests.rs`, `model_tests.rs`, `store_tests.rs`) that construct conversations: replace `AgentKind::X` arguments with `ConversationBackend::Cli(AgentKind::X)`, and `conv.agent` reads with a `match conv.backend`.

- [ ] **Step 5: Compile clean.** `cargo check -p warp-app --features cast-agent 2>&1 | grep -E "error|warning:" | head` → empty (store.rs is handled in Task 3–4; if store.rs errors remain, they're expected until then — scope this step to `conversation.rs` + `model.rs` + non-store tests, or land Tasks 2–4 together before this check).

- [ ] **Step 6: Commit** (fold with Tasks 3–4 if the store won't compile standalone):

```bash
git add app/src/cli_chat/conversation.rs app/src/cli_chat/model.rs app/src/cli_chat/*_tests.rs
git commit -S -m "refactor(cli_chat): ChatConversation carries ConversationBackend"
```

---

## Task 3: Schema Version 2 — `backend` column

**Files:** Modify `app/src/cli_chat/store_schema.rs`; Test: `app/src/cli_chat/store_schema_tests.rs`.

- [ ] **Step 1: Write the failing test.** Add to `store_schema_tests.rs`:

```rust
#[test]
fn migrate_adds_backend_column_defaulting_to_cli() {
    use rusqlite::Connection;
    let conn = Connection::open_in_memory().unwrap();
    // Apply only version 1, insert a legacy row, then migrate to current.
    conn.execute_batch(super::super::store_schema::V1_FOR_TEST).unwrap();
    conn.execute(
        "INSERT INTO chat_conversation (session_id, agent, title, created_at, updated_at, status)
         VALUES ('s1','codex','t',0,0,'idle')",
        [],
    ).unwrap();
    super::super::store_schema::migrate(&conn).unwrap();
    let backend: String = conn
        .query_row("SELECT backend FROM chat_conversation WHERE session_id='s1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(backend, "cli", "existing rows default to cli");
}
```
  (If exposing `V1_FOR_TEST` is undesirable, instead assert the column exists via `PRAGMA table_info(chat_conversation)` after a fresh `migrate`.)

- [ ] **Step 2: Run — verify fail.** `cargo test -p warp-app --features cast-agent --lib cli_chat::store_schema_tests::migrate_adds_backend 2>&1 | head` → FAIL.

- [ ] **Step 3: Add the Version 2 migration.** In `store_schema.rs`: bump `pub const CURRENT_VERSION: i32 = 2;` and append a second element to `MIGRATIONS`:

```rust
    // Version 2 — add the backend discriminator ('cli' | 'daemon'). Existing
    // rows are OSC-777 CLI conversations, so they default to 'cli'.
    &[
        "ALTER TABLE chat_conversation ADD COLUMN backend TEXT NOT NULL DEFAULT 'cli'",
    ],
```
  The existing `migrate()` applies it for any db at version 1. Fresh dbs run V1 then V2. (`ALTER TABLE ADD COLUMN` is idempotent across processes because the version guard skips already-applied migrations.)

- [ ] **Step 4: Run — verify pass** + existing schema tests still pass:

`cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat::store_schema)'` → PASS.

- [ ] **Step 5: Commit.**

```bash
git add app/src/cli_chat/store_schema.rs app/src/cli_chat/store_schema_tests.rs
git commit -S -m "feat(cli_chat): schema v2 adds chat_conversation.backend column"
```

---

## Task 4: Store reads/writes the backend

**Files:** Modify `app/src/cli_chat/store.rs`; Test: `app/src/cli_chat/store_tests.rs`.

- [ ] **Step 1: Write the failing test.** Add to `store_tests.rs`:

```rust
#[test]
fn daemon_conversation_round_trips_backend() {
    use crate::cli_chat::conversation::{ChatConversation, ConversationBackend};
    let store = ChatStore::open_in_memory().unwrap();
    let now = chrono::Utc::now();
    let conv = ChatConversation::new(
        "sess-daemon".into(),
        ConversationBackend::Daemon { harness: "coven-code".into() },
        now,
    );
    store.upsert_conversation(&conv).unwrap();
    let loaded = store.load_conversation("sess-daemon").unwrap().unwrap();
    assert_eq!(loaded.backend, ConversationBackend::Daemon { harness: "coven-code".into() });
}
```
  (Use whatever in-memory constructor the store exposes; `store.rs` has `open_in_memory`-style helpers — mirror the existing `round_trip_conversation_and_entries` test's setup.)

- [ ] **Step 2: Run — verify fail** (`.backend` mismatch or write drops the kind).

- [ ] **Step 3: Update `upsert_conversation`** (currently writes `conv.agent.as_protocol_str()` into `agent`): write both columns:

```rust
    "INSERT INTO chat_conversation
        (session_id, agent, backend, title, cwd, project, created_at, updated_at, status, last_model)
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
     ON CONFLICT(session_id) DO UPDATE SET
        agent=excluded.agent, backend=excluded.backend, title=excluded.title,
        updated_at=excluded.updated_at, status=excluded.status, last_model=excluded.last_model",
    // params: session_id, conv.backend.name(), conv.backend.kind_str(), title, cwd, project,
    //         created_at, updated_at, status, last_model
```
  (Keep the exact existing ON CONFLICT / column set; only add `backend` to the column list, the values tuple, and the update set.)

- [ ] **Step 4: Update `row_to_conversation`** (currently `agent: agent_from_str(&agent_str)`): read the new column and build the backend:

```rust
    let agent_str: String = row.get(1)?;       // agent name column
    let backend_str: String = row.get(/* new backend column index */)?;
    // ...
    backend: ConversationBackend::from_persisted(&agent_str, &backend_str),
```
  Update the two `SELECT` lists (`load_conversation`, `list_conversations`) to include `backend`, and adjust column indices. `agent_from_str` can be deleted (superseded by `from_persisted`).

- [ ] **Step 5: Run — verify pass + full store suite.** `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat::store)'` → PASS (existing CLI round-trips + the new daemon one).

- [ ] **Step 6: Commit.**

```bash
git add app/src/cli_chat/store.rs app/src/cli_chat/store_tests.rs
git commit -S -m "feat(cli_chat): persist conversation backend (cli/daemon) in the store"
```

---

## Task 5: `stream-history.json → sqlite` migration

**Files:** Create `app/src/cli_chat/history_migration.rs`; modify `app/src/cli_chat/mod.rs` (register + call site is 2b, but the pure function + tests land here).

The daemon panel persisted `Vec<{ conversation_id: String, text: String }>` at `~/.coven/stream-history.json` (see `ai_assistant::coven_stream_persist`). Fold each into a `coven-code` conversation with one `AssistantResponse` entry. Non-destructive (rename to `.migrated`), idempotent (skip if already migrated).

- [ ] **Step 1: Write the failing test.** Create `history_migration.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_chat::store::ChatStore;

    #[test]
    fn migrates_text_history_into_daemon_conversations() {
        let store = ChatStore::open_in_memory().unwrap();
        let history = vec![
            HistoryRecord { conversation_id: "c1".into(), text: "hello from coven".into() },
        ];
        let migrated = migrate_history_records(&store, &history, chrono::Utc::now()).unwrap();
        assert_eq!(migrated, 1);
        let conv = store.load_conversation("c1").unwrap().unwrap();
        assert!(matches!(
            conv.backend,
            crate::cli_chat::conversation::ConversationBackend::Daemon { .. }
        ));
        assert!(matches!(
            conv.entries.first().map(|e| &e.kind),
            Some(crate::agent_transcript::entry::ChatEntryKind::AssistantResponse { text }) if text == "hello from coven"
        ));
    }

    #[test]
    fn re_running_is_idempotent() {
        let store = ChatStore::open_in_memory().unwrap();
        let h = vec![HistoryRecord { conversation_id: "c1".into(), text: "x".into() }];
        let now = chrono::Utc::now();
        assert_eq!(migrate_history_records(&store, &h, now).unwrap(), 1);
        // Second run inserts no duplicate entries (upsert conversation, entry unique per sequence).
        assert_eq!(migrate_history_records(&store, &h, now).unwrap(), 1);
        let conv = store.load_conversation("c1").unwrap().unwrap();
        assert_eq!(conv.entries.len(), 1, "no duplicate entry on re-run");
    }
}
```

- [ ] **Step 2: Run — verify fail.**

- [ ] **Step 3: Implement the pure migration.**

```rust
//! One-time, non-destructive migration of the daemon panel's plain-text
//! `~/.coven/stream-history.json` into the unified sqlite store as coven-code
//! `Daemon` conversations. Idempotent; a parse failure is logged and skipped.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind};
use crate::cli_chat::conversation::{ChatConversation, ConversationBackend};
use crate::cli_chat::store::ChatStore;

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryRecord {
    pub conversation_id: String,
    pub text: String,
}

/// Insert each history record as a coven-code Daemon conversation with a single
/// AssistantResponse entry. Returns the number of conversations processed.
/// Idempotent: conversations upsert by id, and the single entry is written at a
/// fixed `sequence = 0` (UNIQUE(session_id, sequence) makes a re-insert a no-op
/// via `insert_entry`'s existing INSERT OR IGNORE semantics).
pub fn migrate_history_records(
    store: &ChatStore,
    records: &[HistoryRecord],
    now: DateTime<Utc>,
) -> rusqlite::Result<usize> {
    for rec in records {
        let conv = ChatConversation::new(
            rec.conversation_id.clone(),
            ConversationBackend::Daemon { harness: "coven-code".into() },
            now,
        );
        store.upsert_conversation(&conv)?;
        let entry = ChatEntry {
            sequence: 0,
            created_at: now,
            kind: ChatEntryKind::AssistantResponse { text: rec.text.clone() },
        };
        store.insert_entry(&rec.conversation_id, &entry)?;
    }
    Ok(records.len())
}

/// Read + migrate `~/.coven/stream-history.json` if present and not yet
/// migrated, then rename it `.migrated` (never delete). Safe to call on every
/// panel startup. The 2b panel bootstrap calls this once.
pub fn migrate_stream_history_file(store: &ChatStore, now: DateTime<Utc>) {
    let Some(path) = dirs::home_dir().map(|h| h.join(".coven").join("stream-history.json")) else {
        return;
    };
    if !path.exists() {
        return;
    }
    let records: Vec<HistoryRecord> = match std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(r) => r,
        None => {
            tracing::warn!("stream-history.json unreadable/malformed; skipping migration");
            return;
        }
    };
    if let Err(e) = migrate_history_records(store, &records, now) {
        tracing::warn!("stream-history migration failed: {e}; leaving file in place");
        return;
    }
    let _ = std::fs::rename(&path, path.with_extension("json.migrated"));
}
```
  Confirm `ChatStore::insert_entry` uses `INSERT OR IGNORE` (or add `OR IGNORE`) so re-inserting `sequence = 0` is a no-op — verify against `store.rs` line ~72 (`insert_entry`); if it's a plain `INSERT`, change to `INSERT OR IGNORE` for idempotency.

- [ ] **Step 4: Register the module.** `app/src/cli_chat/mod.rs`: add `pub mod history_migration;`.

- [ ] **Step 5: Run — verify pass.** `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat::history_migration)'` → PASS.

- [ ] **Step 6: Commit.**

```bash
git add app/src/cli_chat/history_migration.rs app/src/cli_chat/mod.rs app/src/cli_chat/store.rs
git commit -S -m "feat(cli_chat): non-destructive stream-history.json -> sqlite migration"
```

---

## Task 6: Full regression + gates

- [ ] **Step 1: Behavior-preserving check.** `cargo nextest run -p warp-app --features cast-agent -E 'test(cli_chat)'` → all pre-existing CLI conversation/store/model tests PASS unchanged (proves the generalization didn't regress the CLI path), plus the new backend/migration tests.
- [ ] **Step 2: Lint + fmt.** `cargo clippy -p warp-app --features cast-agent -- -D warnings` → clean; `cargo fmt -p warp-app`.
- [ ] **Step 3: Guards.** `./script/check_ai_attribution`, `./script/check_rebrand`, `./script/check_cli_chat_boundary` → pass.
- [ ] **Step 4: Commit** any fmt-only changes.

## Done criteria (2a)

- `ChatConversation` carries `ConversationBackend`; sqlite persists `cli`/`daemon` conversations and round-trips both.
- `stream-history.json` migrates once, non-destructively, idempotently, into coven-code `Daemon` conversations.
- `cli_chat`'s full existing suite passes unchanged (behavior-preserving); gates green; every commit signed.
- **Not** in 2a: the unified panel view, composer routing, daemon conversation source, entry point, deprecation — those are 2b–2d (separate plans).
