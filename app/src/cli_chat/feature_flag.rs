//! Convenience accessor for the `CastCodesChatPanel` feature flag.
//!
//! Mirrors the canonical pattern used elsewhere in this crate (e.g.,
//! `app/src/settings/ai.rs`, `app/src/ai/agent/conversation.rs`): the
//! flag is checked via the global thread-safe state with no caller
//! context required.

use warp_core::features::FeatureFlag;

pub fn is_enabled() -> bool {
    FeatureFlag::CastCodesChatPanel.is_enabled()
}
