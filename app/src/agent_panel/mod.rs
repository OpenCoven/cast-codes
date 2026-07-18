//! The unified CastCodes agent panel — one list + transcript + composer over
//! both CLI-agent and Coven-daemon conversations. Feature-flagged behind
//! [`warp_core::features::FeatureFlag::UnifiedAgentPanel`]; the old `cli_chat`
//! and `ai_assistant` panels are retired in favor of this in a later sub-phase
//! (2d).
//!
//! 2b builds the view shell (merged list + transcript + composer shell). It
//! reuses the `cli_chat` model layer and the shared `agent_transcript`
//! renderer; per-backend composer routing + the live daemon source arrive in
//! 2c. The composer submits via the shared `WorkspaceAction::SubmitChatPrompt`
//! (live CLI only in 2b).

pub mod feature_flag;
#[allow(dead_code)]
pub mod strings;
pub mod view;

#[cfg(test)]
mod view_tests;

use warpui::elements::{CrossAxisAlignment, Element, Expanded, Flex, MainAxisSize, ParentElement};
use warpui::{AppContext, Entity, ModelHandle, SingletonEntity, View, ViewContext, ViewHandle};

use crate::cli_chat::model::{ChatModel, ChatModelEvent};
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use crate::workspace::WorkspaceAction;

/// The unified agent panel view. Holds a strong handle to the singleton
/// [`ChatModel`] and a [`SubmittableTextInput`] composer, and re-renders on any
/// [`ChatModelEvent`].
pub struct AgentPanelView {
    pub(crate) chat_model: ModelHandle<ChatModel>,
    pub(crate) composer_input: ViewHandle<SubmittableTextInput>,
}

impl AgentPanelView {
    /// Build an `AgentPanelView` bound to the app's singleton `ChatModel`.
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let chat_model = ChatModel::handle(ctx);
        ctx.subscribe_to_model(
            &chat_model,
            |_view, _model, _event: &ChatModelEvent, ctx| {
                ctx.notify();
            },
        );

        let composer_input = Self::create_composer(ctx);

        Self {
            chat_model,
            composer_input,
        }
    }

    /// Construct bound to an explicit `ChatModel` handle (tests / non-singleton
    /// callers).
    #[allow(dead_code)]
    pub fn with_model(chat_model: ModelHandle<ChatModel>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(
            &chat_model,
            |_view, _model, _event: &ChatModelEvent, ctx| {
                ctx.notify();
            },
        );

        let composer_input = Self::create_composer(ctx);

        Self {
            chat_model,
            composer_input,
        }
    }

    fn create_composer(ctx: &mut ViewContext<Self>) -> ViewHandle<SubmittableTextInput> {
        let input = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text(strings::COMPOSER_PLACEHOLDER_ACTIVE, ctx);
            input.set_outer_margins(0., 0., ctx);
            input
        });

        ctx.subscribe_to_view(&input, |_view, _, event, ctx| match event {
            SubmittableTextInputEvent::Submit(text) => {
                ctx.dispatch_typed_action(&WorkspaceAction::SubmitChatPrompt {
                    text: text.clone(),
                });
            }
            SubmittableTextInputEvent::Escape => {}
        });

        input
    }
}

impl Entity for AgentPanelView {
    type Event = ();
}

impl View for AgentPanelView {
    fn ui_name() -> &'static str {
        "AgentPanelView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let header = view::header::render(self, app);
        let list = view::conversation_list::render_list(self, app);
        let transcript = view::transcript::render_panel(self, app);
        let composer = view::composer::render_composer(self, app);

        // Right column: transcript (flex-expanded) + composer pinned at bottom.
        let right_column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Expanded::new(1.0, transcript).finish())
            .with_child(composer)
            .finish();

        // Main body: conversation list on the left, right column fills the rest.
        let body = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(list)
            .with_child(Expanded::new(1.0, right_column).finish())
            .finish();

        // Overall: header bar at top, body fills remaining space.
        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header)
            .with_child(Expanded::new(1.0, body).finish())
            .finish()
    }
}
