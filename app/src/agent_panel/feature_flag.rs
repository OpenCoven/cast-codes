//! Convenience accessor for the `UnifiedAgentPanel` feature flag. Mirrors
//! `crate::cli_chat::feature_flag`.

use warp_core::features::FeatureFlag;

pub fn is_enabled() -> bool {
    FeatureFlag::UnifiedAgentPanel.is_enabled()
}
