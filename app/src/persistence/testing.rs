//! Module with integration test-only util methods setting up sqlite.

use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};

use super::{schema, sqlite::init_db};
/// Updates the 'user' and 'host' columns for stored blocks to the given values.
///
/// This is used at runtime to update the user and host values to real values based on the running
/// machine in integration tests that rely on accuracy of these values.
pub fn set_user_and_hostname_for_blocks(user: String, hostname: String) {
    let mut conn = init_db().expect("Should be able to establish sqlite connection.");

    // Update the 'user' and 'host' columns to their real values (based on the machine on which this test is running)
    // for blocks that were stored with the placeholder 'local:user' and 'local:host' values.
    //
    // This allows us to use real (rather than mocked out) logic for matching restored
    // blocks to the appropriate session based on session hostnamebased on system hostname.
    diesel::update(schema::blocks::dsl::blocks.filter(schema::blocks::user.eq("local:user")))
        .set((
            schema::blocks::user.eq(user),
            schema::blocks::host.eq(hostname),
        ))
        .execute(&mut conn)
        .expect("Failed to update user and hostname for restored blocks.");
}

pub fn set_user_and_hostname_for_commands(user: String, hostname: String) {
    let mut conn = init_db().expect("Should be able to establish sqlite connection.");

    // Update the 'user' and 'host' columns to their real values (based on the machine on which
    // this test is running) for commands that were stored with the placeholder 'local:user' and
    // 'local:host' values.
    //
    // This allows us to use real (rather than mocked out) logic for matching history commands to
    // the appropriate session based on session hostnamebased on system hostname.
    diesel::update(
        schema::commands::dsl::commands.filter(schema::commands::username.eq("local:user")),
    )
    .set((
        schema::commands::username.eq(user),
        schema::commands::hostname.eq(hostname),
    ))
    .execute(&mut conn)
    .expect("Failed to update user and hostname for persisted commands.");
}

/// Returns the number of tabs stored in the persisted app-state snapshot.
///
/// This is used by integration tests to verify that a session snapshot actually reached the
/// sqlite database (e.g. via the shutdown save hook).
pub fn count_persisted_tabs() -> i64 {
    let mut conn = init_db().expect("Should be able to establish sqlite connection.");

    schema::tabs::dsl::tabs
        .count()
        .get_result(&mut conn)
        .expect("Failed to count persisted tabs.")
}

/// Deletes the persisted app-state snapshot (windows, tabs, panes, etc.).
///
/// This lets integration tests wipe state written by ambient saves (window
/// events, tab actions) so they can verify that a later save — e.g. the
/// shutdown hook — persists a fresh snapshot on its own.
pub fn clear_persisted_app_state() {
    let mut conn = init_db().expect("Should be able to establish sqlite connection.");

    conn.transaction(super::sqlite::delete_app_state)
        .expect("Failed to clear persisted app state.");
}
