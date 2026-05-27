//! Model picker header bar for the CastCodes chat panel (Phase 6).
//!
//! Renders a horizontal bar at the top of the panel showing:
//! - The current agent name + model (or the default "Claude Opus 4.7")
//! - A "New chat" button that dispatches [`WorkspaceAction::CliChatNewChat`]
//!   with the default agent/model CLI command.
//!
//! Full dropdown selection of all agents/models is Phase 7+ polish.

use std::sync::{Arc, Mutex};

use warpui::elements::{
    Container, CrossAxisAlignment, Element, Flex, Hoverable, MainAxisSize, MouseState,
    ParentElement, Text,
};
use warpui::platform::Cursor;
use warpui::{AppContext, SingletonEntity};

use crate::appearance::Appearance;
use crate::cli_chat::conversation::{AgentKind, ConversationBinding};
use crate::cli_chat::strings;
use crate::cli_chat::view::ChatPanelView;
use crate::workspace::WorkspaceAction;

/// Height of the model picker header bar (in logical pixels).
const BAR_HEIGHT: f32 = 32.0;

/// Render the model-picker header bar.
///
/// Shows the current agent/model label on the left and a "New chat"
/// button on the right.
pub fn render(view: &ChatPanelView, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let font_family = appearance.ui_font_family();
    let font_size = appearance.ui_font_size();

    // Determine the label for the current agent/model.
    let chat = view.chat_model.as_ref(app);
    let label = current_agent_model_label(chat.binding(), chat, app);

    let label_element = Text::new(label, font_family, font_size).finish();

    // "New chat" button.
    let new_chat_button = render_new_chat_button(font_family, font_size);

    let bar = Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(Container::new(label_element).with_margin_left(8.0).finish())
        .with_child(
            // Push the button to the right by inserting a flexible spacer.
            // We use an empty container with flex weight.
            warpui::elements::Expanded::new(
                1.0,
                Container::new(warpui::elements::Empty::new().finish()).finish(),
            )
            .finish(),
        )
        .with_child(
            Container::new(new_chat_button)
                .with_margin_right(8.0)
                .finish(),
        )
        .finish();

    warpui::elements::ConstrainedBox::new(bar)
        .with_height(BAR_HEIGHT)
        .finish()
}

/// Build the display label for the current agent and model.
///
/// If a live or past conversation is bound, shows "Agent — Model".
/// Otherwise falls back to the default ("Claude Opus 4.7").
fn current_agent_model_label(
    binding: &ConversationBinding,
    chat: &crate::cli_chat::model::ChatModel,
    _app: &AppContext,
) -> String {
    let conv = match binding {
        ConversationBinding::Live { session_id, .. } | ConversationBinding::Past { session_id } => {
            chat.conversation(session_id)
        }
        ConversationBinding::None => None,
    };

    if let Some(conv) = conv {
        let agent_name = conv.agent.display_name();
        let model_name = conv
            .last_model
            .as_deref()
            .unwrap_or(conv.agent.curated_models()[0].display_name);
        format!("{} \u{2014} {}", agent_name, model_name)
    } else {
        let (agent, _) = AgentKind::default_agent_and_model();
        let default_model_display = agent.curated_models()[0].display_name;
        format!(
            "{} \u{2014} {}",
            agent.display_name(),
            default_model_display
        )
    }
}

/// Render the "New chat" button.
///
/// Dispatches `CliChatNewChat` with the default agent CLI command on click.
fn render_new_chat_button(
    font_family: warpui::fonts::FamilyId,
    font_size: f32,
) -> Box<dyn Element> {
    let (agent, model_id) = AgentKind::default_agent_and_model();
    let command = agent.cli_command(model_id);

    let mouse_state = Arc::new(Mutex::new(MouseState::default()));

    Hoverable::new(mouse_state, move |_| {
        Container::new(Text::new(strings::NEW_CHAT_BUTTON_LABEL, font_family, font_size).finish())
            .with_uniform_padding(4.0)
            .finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(WorkspaceAction::CliChatNewChat {
            command: command.clone(),
        });
    })
    .finish()
}
