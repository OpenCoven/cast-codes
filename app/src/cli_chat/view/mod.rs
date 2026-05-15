//! CastCodes Chat Panel view layer (Phase 2 — transcript-only).
//!
//! The panel is intentionally minimal in v1: a column of `ChatEntry`
//! rows derived from the bound [`ChatConversation`]. Later phases add
//! the composer, conversation list, model picker, settings, and polish.
//! See `specs/castcodes-chat-panel/PLAN.md`.

pub mod composer; // stub for now; Phase 5
pub mod conversation_list; // stub for now; Phase 4
pub mod empty_state; // stub for now; Phase 7
pub mod error_banner; // stub for now; Phase 7
pub mod info_bar; // stub for now
pub mod message_bubble;
pub mod model_picker; // stub for now; Phase 6
pub mod permission_card; // stub for now
pub mod settings_section; // stub for now; Phase 8
pub mod tool_call_card; // stub for now
pub mod transcript;

use warpui::elements::Element;
use warpui::{AppContext, Entity, ModelHandle, SingletonEntity, View, ViewContext};

use crate::cli_chat::model::{ChatModel, ChatModelEvent};

/// The minimal Phase-2 CastCodes chat panel view.
///
/// Holds a strong handle to the singleton [`ChatModel`] and re-renders
/// whenever a [`ChatModelEvent`] is emitted. All structural rendering
/// lives in [`transcript::render_panel`].
pub struct ChatPanelView {
    pub(crate) chat_model: ModelHandle<ChatModel>,
}

impl ChatPanelView {
    /// Build a `ChatPanelView` bound to the app's singleton `ChatModel`.
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let chat_model = ChatModel::handle(ctx);
        ctx.subscribe_to_model(&chat_model, |_view, _model, _event: &ChatModelEvent, ctx| {
            ctx.notify();
        });
        Self { chat_model }
    }

    /// Construct a `ChatPanelView` bound to an explicit `ChatModel` handle.
    ///
    /// Useful in tests or callers that want to share a non-singleton
    /// `ChatModel` instance. The singleton path goes through [`Self::new`].
    #[allow(dead_code)]
    pub fn with_model(chat_model: ModelHandle<ChatModel>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&chat_model, |_view, _model, _event: &ChatModelEvent, ctx| {
            ctx.notify();
        });
        Self { chat_model }
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
        transcript::render_panel(self, app)
    }
}
