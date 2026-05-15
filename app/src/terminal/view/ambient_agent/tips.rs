//! Tips for agent loading screen.

use crate::ai::agent_tips::AITip;
use warpui::keymap::Keystroke;
use warpui::AppContext;

/// An agent tip with text and optional link.
#[derive(Clone, Debug)]
pub struct CloudModeTip {
    text: String,
    link: Option<String>,
}

impl CloudModeTip {
    pub fn new(text: impl Into<String>, link: Option<impl Into<String>>) -> Self {
        Self {
            text: text.into(),
            link: link.map(|l| l.into()),
        }
    }
}

impl AITip for CloudModeTip {
    fn keystroke(&self, _app: &AppContext) -> Option<Keystroke> {
        None
    }

    fn link(&self) -> Option<String> {
        self.link.clone()
    }

    fn description(&self) -> &str {
        &self.text
    }

    // Uses the default implementation which adds "Tip: " prefix and parses backticks as inline code
}

/// Returns a collection of tips for the agent loading screen.
pub fn get_cloud_mode_tips() -> Vec<CloudModeTip> {
    vec![
        CloudModeTip::new(
            "Keep agent tasks scoped to one clear repository change when possible.",
            None::<String>,
        ),
        CloudModeTip::new(
            "Use Skills to make repeatable local workflows easy to invoke.",
            None::<String>,
        ),
        CloudModeTip::new(
            "Review agent edits before applying broader refactors.",
            None::<String>,
        ),
        CloudModeTip::new(
            "Run focused tests after an agent changes code.",
            None::<String>,
        ),
        CloudModeTip::new(
            "Keep secrets in your local environment or configured credential store.",
            None::<String>,
        ),
    ]
}
