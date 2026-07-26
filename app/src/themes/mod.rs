pub mod default_themes;
pub mod theme;
pub mod theme_chooser;
pub mod theme_creator;
pub mod theme_creator_body;
pub mod theme_creator_modal;
pub mod theme_deletion_body;
pub mod theme_deletion_modal;
pub mod tweakcn_import;

use warp_core::ui::theme::WarpTheme;

/// Themes offered in the onboarding picker, drawn from the bundled set.
pub fn onboarding_theme_picker_themes() -> [WarpTheme; 4] {
    [
        default_themes::castcodes_dark(),
        default_themes::dark_theme(),
        default_themes::light_theme(),
        default_themes::dracula(),
    ]
}
