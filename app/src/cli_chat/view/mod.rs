//! CastCodes Chat Panel view layer (Phase 5 — transcript + conversation list + composer).
//!
//! The panel is a two-column layout:
//! - Left column (~200px): conversation list sidebar
//! - Right column (remaining): transcript of the bound conversation, plus the
//!   composer at the bottom
//!
//! Later phases add the model picker, settings, and polish.
//! See `specs/castcodes-chat-panel/PLAN.md`.

pub mod composer;
pub mod conversation_list;
pub mod empty_state; // stub for now; Phase 7
pub mod error_banner; // stub for now; Phase 7
pub mod info_bar; // stub for now
pub mod message_bubble;
pub mod model_picker; // stub for now; Phase 6
pub mod permission_card; // stub for now
pub mod settings_section; // stub for now; Phase 8
pub mod tool_call_card; // stub for now
pub mod transcript;

use warpui::elements::{CrossAxisAlignment, Element, Expanded, Flex, MainAxisSize, ParentElement};
use warpui::{AppContext, Entity, ModelHandle, SingletonEntity, View, ViewContext, ViewHandle};

use crate::cli_chat::model::{ChatModel, ChatModelEvent};
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};
use crate::workspace::WorkspaceAction;

/// The CastCodes chat panel view.
///
/// Holds a strong handle to the singleton [`ChatModel`], a
/// [`SubmittableTextInput`] for the composer, and re-renders whenever
/// a [`ChatModelEvent`] is emitted.
pub struct ChatPanelView {
    pub(crate) chat_model: ModelHandle<ChatModel>,
    pub(crate) composer_input: ViewHandle<SubmittableTextInput>,
}

impl ChatPanelView {
    /// Build a `ChatPanelView` bound to the app's singleton `ChatModel`.
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

    /// Construct a `ChatPanelView` bound to an explicit `ChatModel` handle.
    ///
    /// Useful in tests or callers that want to share a non-singleton
    /// `ChatModel` instance. The singleton path goes through [`Self::new`].
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

    /// Create the composer's [`SubmittableTextInput`] and subscribe to submit events.
    fn create_composer(ctx: &mut ViewContext<Self>) -> ViewHandle<SubmittableTextInput> {
        let input = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text(crate::cli_chat::strings::COMPOSER_PLACEHOLDER_ACTIVE, ctx);
            input.set_outer_margins(0., 0., ctx);
            input
        });

        ctx.subscribe_to_view(&input, |_view, _, event, ctx| match event {
            SubmittableTextInputEvent::Submit(text) => {
                ctx.dispatch_typed_action(&WorkspaceAction::SubmitChatPrompt {
                    text: text.clone(),
                });
            }
            SubmittableTextInputEvent::Escape => {
                // No-op for now; could blur focus in a later polish phase.
            }
        });

        input
    }
}

impl Entity for ChatPanelView {
    type Event = ();
}

impl View for ChatPanelView {
    fn ui_name() -> &'static str {
        "ChatPanelView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let header = model_picker::render(self, app);
        let list = conversation_list::render_list(self, app);
        let transcript = transcript::render_panel(self, app);
        let composer = composer::render_composer(self, app);

        // Right column: transcript (flex-expanded) + composer pinned at bottom.
        let right_column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Expanded::new(1.0, transcript).finish())
            .with_child(composer)
            .finish();

        // Main body: conversation list on the left, right column fills remaining space.
        let body = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(list)
            .with_child(Expanded::new(1.0, right_column).finish())
            .finish();

        // Overall layout: header bar at top, body fills remaining space.
        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header)
            .with_child(Expanded::new(1.0, body).finish())
            .finish()
    }
}
