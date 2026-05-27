use rusqlite::{Connection, Result};

pub const CURRENT_VERSION: i32 = 1;

const MIGRATIONS: &[&[&str]] = &[
    // Version 1
    &[
        "CREATE TABLE IF NOT EXISTS chat_conversation (
            session_id        TEXT PRIMARY KEY,
            agent             TEXT NOT NULL,
            title             TEXT NOT NULL,
            cwd               TEXT,
            project           TEXT,
            created_at        INTEGER NOT NULL,
            updated_at        INTEGER NOT NULL,
            status            TEXT NOT NULL,
            last_model        TEXT
        )",
        "CREATE TABLE IF NOT EXISTS chat_entry (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id        TEXT NOT NULL REFERENCES chat_conversation(session_id) ON DELETE CASCADE,
            sequence          INTEGER NOT NULL,
            created_at        INTEGER NOT NULL,
            kind              TEXT NOT NULL,
            payload_json      TEXT NOT NULL,
            UNIQUE (session_id, sequence)
        )",
        "CREATE INDEX IF NOT EXISTS idx_chat_entry_session ON chat_entry(session_id, sequence)",
        "CREATE INDEX IF NOT EXISTS idx_chat_conv_updated  ON chat_conversation(updated_at DESC)",
        "CREATE TABLE IF NOT EXISTS chat_schema_version (version INTEGER PRIMARY KEY)",
        "INSERT OR IGNORE INTO chat_schema_version (version) VALUES (1)",
    ],
];

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS chat_schema_version (version INTEGER PRIMARY KEY)",
    )?;

    let current: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM chat_schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if current > CURRENT_VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }

    for (v, migration) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        for stmt in *migration {
            conn.execute(stmt, [])?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO chat_schema_version (version) VALUES (?1)",
            [v as i32 + 1],
        )?;
    }

    Ok(())
}
