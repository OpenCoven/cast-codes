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
