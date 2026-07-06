// Onboarding library crate

mod agent_onboarding_view;
pub mod callout;
mod model;
pub mod slides;

/// The user's intention selected during onboarding slides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingIntention {
    Terminal,
    AgentDrivenDevelopment,
}

impl std::fmt::Display for OnboardingIntention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardingIntention::AgentDrivenDevelopment => write!(f, "agent_driven"),
            OnboardingIntention::Terminal => write!(f, "terminal"),
        }
    }
}

pub use callout::{OnboardingCalloutView, OnboardingKeybindings};

/// User-facing names of the AI features enabled when the agent intention is selected.
/// Shared by the intention slide's agent card checklist and the login slide's
/// skip-login confirmation dialog so the two always stay in sync.
pub const AI_FEATURES: &[&str] = &[
    "Native Coven Code operations",
    "Next command predictions",
    "Prompt suggestions",
    "Codebase context",
    "Remote control with Claude Code, Codex, and other agents",
    "Agents over SSH",
];

/// User-facing names of the Warp Drive features enabled when the terminal
/// intention is selected with Warp Drive turned on. Shared by the login slide's
/// skip-login confirmation dialog so the list stays in sync with any future
/// surfaces that need it.
pub const WARP_DRIVE_FEATURES: &[&str] = &["Cast Drive", "Session Sharing"];

pub mod components;
mod visuals;

/// The default mode for new sessions, chosen during onboarding.
/// Mapped to `DefaultSessionMode` at the application boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionDefault {
    #[default]
    Agent,
    Terminal,
}

impl std::fmt::Display for SessionDefault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionDefault::Agent => write!(f, "agent"),
            SessionDefault::Terminal => write!(f, "terminal"),
        }
    }
}

pub use agent_onboarding_view::{AgentOnboardingAction, AgentOnboardingEvent, AgentOnboardingView};
pub use model::{OnboardingAuthState, SelectedSettings, UICustomizationSettings};
pub use slides::ProjectOnboardingSettings;

pub fn init(app: &mut warpui::AppContext) {
    agent_onboarding_view::init(app);
    callout::init(app);
}

#[cfg(test)]
mod copy_tests {
    const INTRO_SLIDE: &str = include_str!("slides/intro_slide.rs");
    const INTENTION_SLIDE: &str = include_str!("slides/intention_slide.rs");
    const PROJECT_SLIDE: &str = include_str!("slides/project_slide.rs");
    const THEME_PICKER_SLIDE: &str = include_str!("slides/theme_picker_slide.rs");
    const FREE_USER_NO_AI_SLIDE: &str = include_str!("slides/free_user_no_ai_slide.rs");
    const AGENT_CALLOUT_VIEW: &str = include_str!("callout/view.rs");
    const CALLOUT_EXAMPLE: &str = include_str!("../examples/callout.rs");

    #[test]
    fn intro_copy_uses_plain_castcodes_positioning() {
        assert!(INTRO_SLIDE.contains("\"CastCodes\""));
        assert!(!INTRO_SLIDE.contains("Cast Codes with OpenCoven"));
        assert!(!INTRO_SLIDE.contains("sovereign"));
        assert!(!INTRO_SLIDE.contains("conjuring"));
    }

    #[test]
    fn onboarding_completion_ctas_are_plain() {
        for source in [
            INTENTION_SLIDE,
            PROJECT_SLIDE,
            THEME_PICKER_SLIDE,
            FREE_USER_NO_AI_SLIDE,
        ] {
            assert!(!source.contains("Get Casting"));
        }
    }

    #[test]
    fn intention_copy_is_concrete_and_local_first() {
        assert!(INTENTION_SLIDE.contains("native Coven Code"));
        assert!(INTENTION_SLIDE.contains("backed by Coven Code"));
        assert!(!INTENTION_SLIDE.contains("best in class"));
        assert!(!INTENTION_SLIDE.contains("agent driven development AI features"));
    }

    #[test]
    fn onboarding_copy_uses_plain_castcodes_possessive() {
        let awkward_possessive = format!("{}'s", "CastCodes");
        let expected_callout_title = ["CastCodes", "agent experience"].join(" ");

        for source in [
            THEME_PICKER_SLIDE,
            FREE_USER_NO_AI_SLIDE,
            AGENT_CALLOUT_VIEW,
        ] {
            assert!(!source.contains(&awkward_possessive));
        }

        assert!(AGENT_CALLOUT_VIEW.contains(&expected_callout_title));
    }

    #[test]
    fn examples_use_castcodes_terms() {
        assert!(CALLOUT_EXAMPLE.contains("Meet your CastCodes input"));
        assert!(!CALLOUT_EXAMPLE.contains("Meet your Warp input"));
    }
}
