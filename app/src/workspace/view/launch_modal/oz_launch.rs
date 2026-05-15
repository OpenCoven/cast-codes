use super::{CTAButton, CheckboxConfig, LaunchModalEvent, Slide};
use crate::terminal::view::OnboardingIntention;
use crate::ui_components::icons::Icon;
use crate::workspace::action::WorkspaceAction;
use crate::workspace::view::OnboardingTutorial;
use asset_macro::bundled_or_fetched_asset;
use markdown_parser::{FormattedTextFragment, FormattedTextLine};
use warpui::assets::asset_cache::AssetSource;
use warpui::AppContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OzLaunchSlide {
    CloudAgents,
    AgentAutomations,
    AgentManagement,
    LaunchCredits,
}

impl Slide for OzLaunchSlide {
    fn modal_title(&self) -> String {
        "Agent orchestration".to_string()
    }

    fn modal_subtext_paragraphs(&self) -> Vec<FormattedTextLine> {
        vec![FormattedTextLine::Line(vec![
            FormattedTextFragment::plain_text(
                "Coordinate local coding agents directly inside CastCodes.",
            ),
        ])]
    }

    fn first() -> Self {
        OzLaunchSlide::CloudAgents
    }

    fn next(&self) -> Option<Self> {
        match self {
            OzLaunchSlide::CloudAgents => Some(OzLaunchSlide::AgentAutomations),
            OzLaunchSlide::AgentAutomations => Some(OzLaunchSlide::AgentManagement),
            OzLaunchSlide::AgentManagement => Some(OzLaunchSlide::LaunchCredits),
            OzLaunchSlide::LaunchCredits => None,
        }
    }

    fn prev(&self) -> Option<Self> {
        match self {
            OzLaunchSlide::CloudAgents => None,
            OzLaunchSlide::AgentAutomations => Some(OzLaunchSlide::CloudAgents),
            OzLaunchSlide::AgentManagement => Some(OzLaunchSlide::AgentAutomations),
            OzLaunchSlide::LaunchCredits => Some(OzLaunchSlide::AgentManagement),
        }
    }

    fn display_text(&self) -> Option<&'static str> {
        Some(match self {
            OzLaunchSlide::CloudAgents => "Local agents",
            OzLaunchSlide::AgentAutomations => "Agent automations",
            OzLaunchSlide::AgentManagement => "Agent management",
            OzLaunchSlide::LaunchCredits => "Local setup",
        })
    }

    fn short_label(&self) -> &'static str {
        match self {
            OzLaunchSlide::CloudAgents => "Local agents",
            OzLaunchSlide::AgentAutomations => "Agent automations",
            OzLaunchSlide::AgentManagement => "Agent management",
            OzLaunchSlide::LaunchCredits => "Local setup",
        }
    }

    fn title(&self) -> &'static str {
        match self {
            OzLaunchSlide::CloudAgents => "Run coding agents locally",
            OzLaunchSlide::AgentAutomations => {
                "Orchestrate agents, turning Skills into automations"
            }
            OzLaunchSlide::AgentManagement => "Track local agent sessions",
            OzLaunchSlide::LaunchCredits => "Start from the current workspace context",
        }
    }

    fn title_icon(&self) -> Option<Icon> {
        None
    }

    fn content(&self) -> &'static str {
        match self {
            OzLaunchSlide::CloudAgents => {
                "Use local agents to work from the files, commands, and terminal context already open in CastCodes."
            }
            OzLaunchSlide::AgentAutomations => {
                "Agents can use the standard Skills format to keep repeatable workflows close to the repository."
            }
            OzLaunchSlide::AgentManagement => {
                "View active agent sessions in CastCodes, inspect their work, and steer them without leaving the terminal."
            }
            OzLaunchSlide::LaunchCredits => {
                "Start a local agent from the current workspace and keep the whole workflow on this machine."
            }
        }
    }

    fn image(&self) -> AssetSource {
        // TODO: Replace with new images once provided.
        match self {
            OzLaunchSlide::CloudAgents => {
                bundled_or_fetched_asset!("png/oz_cloud_agents.png")
            }
            OzLaunchSlide::AgentAutomations => {
                bundled_or_fetched_asset!("png/oz_agent_automations.png")
            }
            OzLaunchSlide::AgentManagement => {
                bundled_or_fetched_asset!("png/oz_agent_management.png")
            }
            OzLaunchSlide::LaunchCredits => {
                bundled_or_fetched_asset!("png/oz_launch_credits.png")
            }
        }
    }

    fn all() -> Vec<Self> {
        vec![
            OzLaunchSlide::CloudAgents,
            OzLaunchSlide::AgentAutomations,
            OzLaunchSlide::AgentManagement,
            OzLaunchSlide::LaunchCredits,
        ]
    }

    fn cta_button(&self) -> CTAButton<Self> {
        match self {
            OzLaunchSlide::CloudAgents
            | OzLaunchSlide::AgentAutomations
            | OzLaunchSlide::AgentManagement => {
                let next = self.next().expect("Non-final slides should have a next");
                CTAButton::next_slide(next, format!("Next: {}", next.short_label()))
            }
            OzLaunchSlide::LaunchCredits => CTAButton::custom("Start locally", |ctx| {
                ctx.emit(LaunchModalEvent::Close);
                ctx.dispatch_typed_action(&WorkspaceAction::StartAgentOnboardingTutorial(
                    OnboardingTutorial::NoProject {
                        intention: OnboardingIntention::AgentDrivenDevelopment,
                    },
                ));
                ctx.dispatch_typed_action(&WorkspaceAction::AddAmbientAgentTab);
            }),
        }
    }

    fn secondary_cta_button(&self) -> Option<CTAButton<Self>> {
        match self {
            OzLaunchSlide::LaunchCredits => Some(CTAButton::close("Skip for now")),
            OzLaunchSlide::CloudAgents
            | OzLaunchSlide::AgentAutomations
            | OzLaunchSlide::AgentManagement => None,
        }
    }

    fn checkbox_config(&self) -> Option<CheckboxConfig> {
        None
    }

    fn should_show_checkbox(&self, _app: &AppContext) -> bool {
        false
    }

    fn on_close(&self, ctx: &mut warpui::ViewContext<super::LaunchModal<Self>>) {
        ctx.dispatch_typed_action(&WorkspaceAction::StartAgentOnboardingTutorial(
            OnboardingTutorial::NoProject {
                intention: OnboardingIntention::AgentDrivenDevelopment,
            },
        ));
    }
}

pub fn init(app: &mut warpui::AppContext) {
    super::init::<OzLaunchSlide>(app);
}
