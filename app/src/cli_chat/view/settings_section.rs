//! Settings section for the CastCodes chat panel.
//!
//! Currently a placeholder — full settings integration with the inherited
//! `ai_page.rs` settings infrastructure is deferred to a follow-up.

// TODO(Phase 8): Wire into app/src/settings_view/ai_page.rs

/// Register any settings UI elements for the chat panel.
///
/// No-op for v1. A follow-up will integrate with the settings page to
/// expose CLI agent preferences (default model, auto-scroll, etc.).
pub fn register() {}
