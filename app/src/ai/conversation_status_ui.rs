use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::coloru_with_opacity;
use warp_core::ui::theme::{Fill, WarpTheme};
use warpui::color::ColorU;
use warpui::elements::{
    ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Flex, MainAxisSize, Padding,
    ParentElement, Radius, Text,
};
use warpui::Element;

use crate::ai::agent::conversation::ConversationStatus;
use crate::ai::agent_conversations_model::AgentRunDisplayStatus;
use crate::ui_components::design;
use crate::ui_components::icons::Icon;

/// Padding around the status icon
pub const STATUS_ELEMENT_PADDING: f32 = design::space::HAIRLINE;

pub trait StatusElementStyle {
    fn status_icon_and_color(&self, theme: &WarpTheme) -> (Icon, ColorU);
}

impl StatusElementStyle for ConversationStatus {
    fn status_icon_and_color(&self, theme: &WarpTheme) -> (Icon, ColorU) {
        ConversationStatus::status_icon_and_color(self, theme)
    }
}

impl StatusElementStyle for AgentRunDisplayStatus {
    fn status_icon_and_color(&self, theme: &WarpTheme) -> (Icon, ColorU) {
        AgentRunDisplayStatus::status_icon_and_color(self, theme)
    }
}

/// Render the status element used by agent and conversation views.
pub fn render_status_element(
    status: &impl StatusElementStyle,
    icon_size: f32,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let (icon, color) = status.status_icon_and_color(theme);

    Container::new(
        ConstrainedBox::new(icon.to_warpui_icon(Fill::from(color)).finish())
            .with_width(icon_size)
            .with_height(icon_size)
            .finish(),
    )
    .with_uniform_padding(STATUS_ELEMENT_PADDING)
    .with_background(coloru_with_opacity(color, 10))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
        design::radius::COMPACT,
    )))
    .finish()
}

/// Render a compact status pill with the status icon and a short text label.
/// Shared by pane chrome surfaces (vertical tab detail rows, pane headers) so
/// each status reads identically everywhere it appears with a label.
pub fn render_status_element_with_label(
    status: &impl StatusElementStyle,
    label: impl Into<String>,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let (icon, color) = status.status_icon_and_color(theme);

    Container::new(
        Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.)
            .with_child(
                ConstrainedBox::new(icon.to_warpui_icon(Fill::from(color)).finish())
                    .with_width(12.)
                    .with_height(12.)
                    .finish(),
            )
            .with_child(
                Text::new_inline(
                    label.into(),
                    appearance.ui_font_family(),
                    design::font_size::MICRO,
                )
                .with_color(Fill::from(color).into())
                .finish(),
            )
            .finish(),
    )
    .with_padding(
        Padding::uniform(design::space::HAIRLINE)
            .with_left(design::space::XS)
            .with_right(design::space::XS),
    )
    .with_background(coloru_with_opacity(color, 10))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
        design::radius::COMPACT,
    )))
    .finish()
}
