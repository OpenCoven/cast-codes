/// Common functionality used across different AI Assistant components.
use pathfinder_color::ColorU;
use warpui::{
    elements::{
        ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Flex, Icon, MainAxisAlignment,
        MainAxisSize, MouseStateHandle, ParentElement, Radius, Text,
    },
    platform::Cursor,
    ui_components::{
        button::ButtonVariant,
        components::{Coords, UiComponent, UiComponentStyles},
    },
    AppContext, Element, ModelHandle,
};

use crate::{appearance::Appearance, ui_components::blended_colors};

// The AI-dialogue data types now live in `dialogue_types` (UI-free, shared with
// the server API / auth). Re-export them here for the panel UI that still
// consumes them via `utils`.
pub use super::dialogue_types::{
    markdown_segments_from_text, AssistantTranscriptPart, CodeBlockIndex, FormattedTranscriptMessage,
    MarkdownSegment, TranscriptPart, TranscriptPartSubType,
};
use super::{panel::AIAssistantAction, requests::Requests};

const PREPARED_RESPONSE_FONT_SIZE: f32 = 11.;
const REQUEST_LIMIT_INFO_FONT_SIZE: f32 = 11.;

const SQUARE_ALERT_SVG_PATH: &str = "bundled/svg/alert-square.svg";
const TRIANGLE_ALERT_SVG_PATH: &str = "bundled/svg/alert-triangle.svg";


pub fn render_prepared_response_button(
    appearance: &Appearance,
    mouse_state_handle: MouseStateHandle,
    width: Option<f32>,
    right_left_padding: Option<f32>,
    prompt: &'static str,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let default_button_styles = UiComponentStyles {
        width,
        font_size: Some(PREPARED_RESPONSE_FONT_SIZE),
        font_family_id: Some(appearance.ui_font_family()),
        font_color: Some(
            appearance
                .theme()
                .main_text_color(appearance.theme().background())
                .into(),
        ),
        border_radius: Some(CornerRadius::with_all(Radius::Percentage(50.))),
        border_color: Some(theme.accent().into()),
        border_width: Some(1.),
        padding: Some(Coords {
            top: 5.,
            bottom: 5.,
            left: right_left_padding.unwrap_or(0.),
            right: right_left_padding.unwrap_or(0.),
        }),
        ..Default::default()
    };
    let hovered_and_clicked_styles = UiComponentStyles {
        background: Some(theme.accent().into()),
        font_color: Some(theme.background().into()),
        ..default_button_styles
    };
    appearance
        .ui_builder()
        .button_with_custom_styles(
            ButtonVariant::Text,
            mouse_state_handle,
            default_button_styles,
            Some(hovered_and_clicked_styles),
            Some(hovered_and_clicked_styles),
            Some(hovered_and_clicked_styles),
        )
        .with_centered_text_label(prompt.to_string())
        .build()
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(AIAssistantAction::PreparedPrompt(prompt))
        })
        .finish()
}

pub fn render_request_limit_info(
    request_model: &ModelHandle<Requests>,
    app: &AppContext,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let text_color: ColorU =
        blended_colors::text_sub(appearance.theme(), appearance.theme().background());

    let num_requests_used = request_model.as_ref(app).num_requests_used();
    let num_requests_remaining = request_model.as_ref(app).num_remaining_reqs();
    let request_limit = request_model.as_ref(app).request_limit();
    let next_refresh_time = request_model.as_ref(app).serialized_time_until_refresh();

    // Always show the remaining requests count.
    let mut row = Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::Center)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            Text::new_inline(
                format!("Credits used: {num_requests_used} / {request_limit}.",),
                appearance.ui_font_family(),
                REQUEST_LIMIT_INFO_FONT_SIZE,
            )
            .with_color(text_color)
            .finish(),
        );

    // Add the warning icon if necessary.
    let icon = if num_requests_remaining == 0 {
        Some(Icon::new(
            TRIANGLE_ALERT_SVG_PATH,
            appearance.theme().ui_error_color(),
        ))
    } else if num_requests_remaining <= 10 {
        Some(Icon::new(
            SQUARE_ALERT_SVG_PATH,
            appearance.theme().ui_warning_color(),
        ))
    } else {
        None
    };

    if let Some(icon) = icon {
        row.add_child(
            Container::new(
                ConstrainedBox::new(icon.finish())
                    .with_height(16.)
                    .with_width(16.)
                    .finish(),
            )
            .with_margin_left(5.)
            .finish(),
        );
    }

    // Show the next refresh time if it's valid.
    if let Some(next_refresh_time) = next_refresh_time {
        row.add_child(
            Container::new(
                Text::new_inline(
                    format!("{next_refresh_time} until refresh."),
                    appearance.ui_font_family(),
                    REQUEST_LIMIT_INFO_FONT_SIZE,
                )
                .with_color(text_color)
                .finish(),
            )
            .with_margin_left(5.)
            .finish(),
        );
    }

    row.finish()
}

pub fn code_block_position_id(code_block_index: CodeBlockIndex) -> String {
    format!("code_block_id_{}", code_block_index.as_id_str(),)
}

pub fn save_as_workflow_position_id(code_block_index: CodeBlockIndex) -> String {
    format!(
        "{}_save_as_workflow",
        code_block_position_id(code_block_index)
    )
}
