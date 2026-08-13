#[cfg(feature = "local_fs")]
pub use crate::persistence::database_file_path;

use warpui::integration::TestStep;

/// Replays the persistence portion of the app's `on_will_terminate` callback (see
/// `app_callbacks` in `lib.rs`): enqueue a final session snapshot, then synchronously
/// terminate the sqlite writer thread.
///
/// Integration tests cannot assert anything after the real termination callback runs, so
/// this helper runs the same shutdown sequence mid-test, letting a later step verify that
/// the snapshot reached the database.
pub fn run_shutdown_persistence_hooks() -> TestStep {
    TestStep::new("Run the shutdown persistence hooks").with_action(|app, _, _data| {
        app.update(|ctx| {
            crate::workspace::run_shutdown_persistence(ctx);
        });
    })
}
