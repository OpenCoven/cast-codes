mod hoa_onboarding_flow;
mod tab_config_step;
mod welcome_banner;

pub use hoa_onboarding_flow::{init, HoaOnboardingFlow, HoaOnboardingFlowEvent, HoaOnboardingStep};

use warpui::AppContext;

use warp_core::user_preferences::GetUserPreferences;

const HAS_COMPLETED_HOA_ONBOARDING_KEY: &str = "HasCompletedHOAOnboarding";

pub fn has_completed_hoa_onboarding(ctx: &AppContext) -> bool {
    ctx.private_user_preferences()
        .read_value(HAS_COMPLETED_HOA_ONBOARDING_KEY)
        .unwrap_or_default()
        .and_then(|s| serde_json::from_str::<bool>(&s).ok())
        .unwrap_or(false)
}

pub fn mark_hoa_onboarding_completed(ctx: &AppContext) {
    let _ = ctx.private_user_preferences().write_value(
        HAS_COMPLETED_HOA_ONBOARDING_KEY,
        serde_json::to_string(&true).expect("bool serializes to JSON"),
    );
}

#[cfg(test)]
mod copy_tests {
    const WELCOME_BANNER_SOURCE: &str = include_str!("welcome_banner.rs");
    const HOA_FLOW_SOURCE: &str = include_str!("hoa_onboarding_flow.rs");

    #[test]
    fn hoa_onboarding_copy_is_castcodes_owned_and_plain() {
        let awkward_possessive = format!("{}'s", "CastCodes");
        let promotional_phrase = ["level", "up"].join(" ");
        let upstream_docs_host = ["docs", "warp", "dev"].join(".");
        let expected_title = ["Use any coding agent with", "CastCodes"].join(" ");
        let expected_doc_target = ["OpenCoven", "cast-codes"].join("/");

        for source in [WELCOME_BANNER_SOURCE, HOA_FLOW_SOURCE] {
            assert!(!source.contains(&awkward_possessive));
            assert!(!source.contains(&promotional_phrase));
            assert!(!source.contains(&upstream_docs_host));
        }

        assert!(WELCOME_BANNER_SOURCE.contains(&expected_title));
        assert!(HOA_FLOW_SOURCE.contains(&expected_doc_target));
    }
}
