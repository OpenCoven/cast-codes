//! Convenience accessor for the `UnifiedAgentPanel` feature flag. Mirrors the `cli_chat` feature-flag accessor pattern.

use warp_core::features::FeatureFlag;

pub fn is_enabled() -> bool {
    FeatureFlag::UnifiedAgentPanel.is_enabled()
}
