use rusqlite::Connection;

use super::store_schema::{migrate, CURRENT_VERSION};

#[test]
fn migrate_from_empty_creates_tables() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_conversation'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_entry'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn migrate_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    migrate(&conn).unwrap();

    let v: i32 = conn
        .query_row("SELECT MAX(version) FROM chat_schema_version", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(v, CURRENT_VERSION);
}

#[test]
fn migrate_adds_backend_column_defaulting_to_cli() {
    let conn = Connection::open_in_memory().unwrap();
    // Reconstruct a version-1 database by hand: the pre-backend
    // chat_conversation shape, a legacy CLI row, and version pinned to 1.
    conn.execute_batch(
        "CREATE TABLE chat_conversation (
            session_id  TEXT PRIMARY KEY,
            agent       TEXT NOT NULL,
            title       TEXT NOT NULL,
            cwd         TEXT,
            project     TEXT,
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL,
            status      TEXT NOT NULL,
            last_model  TEXT
        );
        CREATE TABLE chat_schema_version (version INTEGER PRIMARY KEY);
        INSERT INTO chat_schema_version (version) VALUES (1);
        INSERT INTO chat_conversation
            (session_id, agent, title, created_at, updated_at, status)
            VALUES ('s1', 'codex', 't', 0, 0, 'idle');",
    )
    .unwrap();

    // Upgrading to the current version must add `backend` and backfill 'cli'.
    migrate(&conn).unwrap();

    let backend: String = conn
        .query_row(
            "SELECT backend FROM chat_conversation WHERE session_id = 's1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(backend, "cli", "existing rows default to cli");

    let v: i32 = conn
        .query_row("SELECT MAX(version) FROM chat_schema_version", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(v, CURRENT_VERSION);
}

#[test]
fn migrate_rejects_future_version() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE chat_schema_version (version INTEGER PRIMARY KEY)",
        [],
    )
    .unwrap();
    conn.execute("INSERT INTO chat_schema_version VALUES (99)", [])
        .unwrap();
    assert!(migrate(&conn).is_err());
}
