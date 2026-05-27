use std::path::Path;

use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection};

use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

use super::conversation::{AgentKind, ChatConversation};
use super::entry::{ChatEntry, ChatEntryKind};
use super::store_schema;

/// Persistent store for CLI chat conversations and transcript entries.
///
/// Backed by a single rusqlite [`Connection`]. All operations are synchronous
/// and intended to be called from a background thread or async task.
pub struct ChatStore {
    conn: Connection,
}

impl ChatStore {
    /// Open (or create) the database at the given filesystem path and run
    /// migrations.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        store_schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database — useful for tests.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        store_schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Insert or update a conversation row.
    ///
    /// On conflict (same `session_id`) the mutable metadata columns are
    /// updated while `created_at` is preserved.
    pub fn upsert_conversation(&self, conv: &ChatConversation) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO chat_conversation
                (session_id, agent, title, cwd, project, created_at, updated_at, status, last_model)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(session_id) DO UPDATE SET
                title      = excluded.title,
                cwd        = excluded.cwd,
                project    = excluded.project,
                updated_at = excluded.updated_at,
                status     = excluded.status,
                last_model = excluded.last_model",
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

    /// Insert a single transcript entry. Duplicate `(session_id, sequence)`
    /// pairs are silently ignored (idempotent).
    pub fn insert_entry(&self, session_id: &str, entry: &ChatEntry) -> rusqlite::Result<()> {
        let (kind_tag, payload_json) = serialize_entry(&entry.kind);
        self.conn.execute(
            "INSERT INTO chat_entry
                (session_id, sequence, created_at, kind, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(session_id, sequence) DO NOTHING",
            params![
                session_id,
                entry.sequence as i64,
                entry.created_at.timestamp_millis(),
                kind_tag,
                payload_json,
            ],
        )?;
        Ok(())
    }

    /// Load a single conversation by `session_id`, including all of its
    /// entries. Returns `None` if the conversation does not exist.
    pub fn load_conversation(
        &self,
        session_id: &str,
    ) -> rusqlite::Result<Option<ChatConversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, agent, title, cwd, project, created_at, updated_at, status, last_model
             FROM chat_conversation
             WHERE session_id = ?1",
        )?;

        let mut rows = stmt.query(params![session_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        let mut conv = row_to_conversation(row)?;
        conv.entries = self.load_entries(session_id)?;
        Ok(Some(conv))
    }

    /// Load all entries for a conversation, ordered by sequence.
    pub fn load_entries(&self, session_id: &str) -> rusqlite::Result<Vec<ChatEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT sequence, created_at, kind, payload_json
             FROM chat_entry
             WHERE session_id = ?1
             ORDER BY sequence ASC",
        )?;

        let entries = stmt
            .query_map(params![session_id], |row| {
                let seq: i64 = row.get(0)?;
                let ts_millis: i64 = row.get(1)?;
                let kind_tag: String = row.get(2)?;
                let payload_json: String = row.get(3)?;
                Ok(ChatEntry {
                    sequence: seq as u64,
                    created_at: millis_to_dt(ts_millis),
                    kind: deserialize_entry_kind(&kind_tag, &payload_json),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(entries)
    }

    /// List all conversations ordered by `updated_at DESC`.
    ///
    /// Each conversation's entries are loaded eagerly.
    pub fn list_conversations(&self) -> rusqlite::Result<Vec<ChatConversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, agent, title, cwd, project, created_at, updated_at, status, last_model
             FROM chat_conversation
             ORDER BY updated_at DESC",
        )?;

        let convs = stmt
            .query_map([], row_to_conversation)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut result = Vec::with_capacity(convs.len());
        for mut conv in convs {
            conv.entries = self.load_entries(&conv.session_id)?;
            result.push(conv);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatConversation> {
    let session_id: String = row.get(0)?;
    let agent_str: String = row.get(1)?;
    let title: String = row.get(2)?;
    let cwd: Option<String> = row.get(3)?;
    let project: Option<String> = row.get(4)?;
    let created_millis: i64 = row.get(5)?;
    let updated_millis: i64 = row.get(6)?;
    let status_str: String = row.get(7)?;
    let last_model: Option<String> = row.get(8)?;

    Ok(ChatConversation {
        session_id,
        agent: agent_from_str(&agent_str),
        title,
        cwd,
        project,
        created_at: millis_to_dt(created_millis),
        updated_at: millis_to_dt(updated_millis),
        status: str_to_status(&status_str),
        last_model,
        entries: Vec::new(),
    })
}

/// Convert milliseconds since epoch to a `DateTime<Utc>`, falling back to
/// the Unix epoch on invalid values.
fn millis_to_dt(ms: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_millis_opt(ms).single().unwrap_or_default()
}

// -- Status serialisation ---------------------------------------------------

fn status_to_str(status: &CLIAgentSessionStatus) -> String {
    match status {
        CLIAgentSessionStatus::InProgress => "in_progress".to_owned(),
        CLIAgentSessionStatus::Success => "success".to_owned(),
        CLIAgentSessionStatus::Blocked { message } => match message {
            Some(msg) => format!("blocked:{msg}"),
            None => "blocked".to_owned(),
        },
    }
}

fn str_to_status(s: &str) -> CLIAgentSessionStatus {
    match s {
        "in_progress" => CLIAgentSessionStatus::InProgress,
        "success" => CLIAgentSessionStatus::Success,
        other => {
            if let Some(msg) = other.strip_prefix("blocked:") {
                CLIAgentSessionStatus::Blocked {
                    message: Some(msg.to_owned()),
                }
            } else {
                // "blocked" with no message, or any unknown value
                CLIAgentSessionStatus::Blocked { message: None }
            }
        }
    }
}

// -- Agent serialisation ----------------------------------------------------

fn agent_from_str(s: &str) -> AgentKind {
    match s {
        "claude" => AgentKind::Claude,
        "codex" => AgentKind::Codex,
        "gemini" => AgentKind::Gemini,
        "opencode" => AgentKind::OpenCode,
        // Fallback — default to Claude so we never panic on unknown data.
        _ => AgentKind::Claude,
    }
}

// -- Entry kind serialisation -----------------------------------------------

/// Serialize a `ChatEntryKind` into a `(kind_tag, payload_json)` pair suitable
/// for the `kind` and `payload_json` columns.
fn serialize_entry(kind: &ChatEntryKind) -> (String, String) {
    // `ChatEntryKind` uses `#[serde(tag = "kind")]`, so serde_json will
    // produce `{"kind": "<tag>", ...}`. We store the tag separately for
    // easy querying and the full JSON blob for the payload.
    let json = serde_json::to_string(kind).unwrap_or_default();
    let tag = serde_json::from_str::<serde_json::Value>(&json)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(String::from))
        .unwrap_or_else(|| "raw".to_owned());
    (tag, json)
}

/// Reconstruct a `ChatEntryKind` from the persisted kind tag and JSON blob.
///
/// Falls back to `ChatEntryKind::Raw` on any deserialization failure so that
/// forward-incompatible entries never cause a hard error.
fn deserialize_entry_kind(_kind_tag: &str, payload_json: &str) -> ChatEntryKind {
    serde_json::from_str::<ChatEntryKind>(payload_json).unwrap_or_else(|_| ChatEntryKind::Raw {
        event_type: _kind_tag.to_owned(),
        payload_json: payload_json.to_owned(),
    })
}
