//! One-time, non-destructive migration of the daemon panel's plain-text
//! `~/.coven/stream-history.json` into the unified sqlite store as coven-code
//! `Daemon` conversations. Idempotent; a parse failure is logged and skipped.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::agent_transcript::entry::{ChatEntry, ChatEntryKind};
use crate::cli_chat::conversation::{ChatConversation, ConversationBackend};
use crate::cli_chat::store::ChatStore;

/// One persisted daemon-panel stream. Mirrors
/// `ai_assistant::panel::CovenStreamHistoryEntry`'s on-disk shape so the two
/// stay wire-compatible.
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryRecord {
    pub conversation_id: String,
    pub text: String,
}

/// Insert each history record as a coven-code Daemon conversation with a single
/// AssistantResponse entry. Returns the number of conversations processed.
///
/// Idempotent: conversations upsert by id, and the single entry is written at a
/// fixed `sequence = 0` (`UNIQUE(session_id, sequence)` makes a re-insert a
/// no-op via `insert_entry`'s `ON CONFLICT ... DO NOTHING` semantics).
pub fn migrate_history_records(
    store: &ChatStore,
    records: &[HistoryRecord],
    now: DateTime<Utc>,
) -> rusqlite::Result<usize> {
    for rec in records {
        let conv = ChatConversation::new(
            rec.conversation_id.clone(),
            ConversationBackend::Daemon {
                harness: "coven-code".into(),
            },
            now,
        );
        store.upsert_conversation(&conv)?;
        let entry = ChatEntry {
            sequence: 0,
            created_at: now,
            kind: ChatEntryKind::AssistantResponse {
                text: rec.text.clone(),
            },
        };
        store.insert_entry(&rec.conversation_id, &entry)?;
    }
    Ok(records.len())
}

/// Read + migrate `~/.coven/stream-history.json` if present and not yet
/// migrated, then rename it `.migrated` (never delete). Safe to call on every
/// panel startup; the 2b panel bootstrap calls this once.
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
            log::warn!("cast_agent: stream-history.json unreadable/malformed; skipping migration");
            return;
        }
    };
    if let Err(e) = migrate_history_records(store, &records, now) {
        log::warn!("cast_agent: stream-history migration failed: {e}; leaving file in place");
        return;
    }
    let _ = std::fs::rename(&path, path.with_extension("json.migrated"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_chat::store::ChatStore;

    #[test]
    fn migrates_text_history_into_daemon_conversations() {
        let store = ChatStore::open_in_memory().unwrap();
        let history = vec![HistoryRecord {
            conversation_id: "c1".into(),
            text: "hello from coven".into(),
        }];
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
        let h = vec![HistoryRecord {
            conversation_id: "c1".into(),
            text: "x".into(),
        }];
        let now = chrono::Utc::now();
        assert_eq!(migrate_history_records(&store, &h, now).unwrap(), 1);
        // Second run inserts no duplicate entries (upsert conversation, entry
        // unique per sequence).
        assert_eq!(migrate_history_records(&store, &h, now).unwrap(), 1);
        let conv = store.load_conversation("c1").unwrap().unwrap();
        assert_eq!(conv.entries.len(), 1, "no duplicate entry on re-run");
    }
}
