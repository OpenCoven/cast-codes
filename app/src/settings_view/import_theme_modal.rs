//! Import-theme modal for tweakcn share links.
//!
//! Opens when the user presses "Import theme…" in Appearance settings.
//! The user pastes a tweakcn share link (e.g. `https://tweakcn.com/themes/<id>`),
//! the modal extracts the theme id, fetches the registry JSON, and on Save
//! writes YAML(s) to disk plus dispatches a theme-reload+select event so the
//! new theme is immediately active.

use std::time::Duration;

use crate::appearance::Appearance;
use crate::editor::{EditorView, Event as EditorEvent, SingleLineEditorOptions};
use crate::modal::Modal;
#[cfg(feature = "local_fs")]
use crate::themes::theme::CustomTheme;
use crate::themes::theme::ThemeKind;
use crate::themes::tweakcn_import::{extract_theme_id, fetch_share_url, ImportError, ParsedBlocks};
#[cfg(feature = "local_fs")]
use crate::themes::tweakcn_import::{write_imported, GamutPolicy};
#[cfg(feature = "local_fs")]
use crate::user_config;
use warpui::elements::{
    Container, CornerRadius, CrossAxisAlignment, Fill, Flex, MainAxisSize, ParentElement, Radius,
    Shrinkable, Text,
};
use warpui::fonts::Weight;
use warpui::presenter::ChildView;
use warpui::r#async::Timer;
use warpui::ui_components::button::ButtonVariant;
use warpui::ui_components::components::{Coords, UiComponent as _, UiComponentStyles};
use warpui::ui_components::text_input::TextInput;
use warpui::ViewHandle;
use warpui::{
    AppContext, Element, Entity, SingletonEntity as _, TypedActionView, View, ViewContext,
};

const MODAL_HEADER: &str = "Import theme from tweakcn";
const MODAL_WIDTH: f32 = 560.;
const MODAL_HEIGHT: f32 = 360.;
const FETCH_DEBOUNCE: Duration = Duration::from_millis(300);

// ─── Fetch state ──────────────────────────────────────────────────────────

enum FetchState {
    Idle,
    Fetching,
    Fetched(ParsedBlocks),
    Error(String),
}

// ─── ImportThemeBody ─────────────────────────────────────────────────────────

pub struct ImportThemeBody {
    url_editor: ViewHandle<EditorView>,
    url_text: String,
    state: FetchState,
    pub(crate) show_error: Option<String>,
    pending_token: u64,
}

#[derive(Debug)]
pub enum ImportThemeBodyAction {
    Save,
    Cancel,
}

pub enum ImportThemeBodyEvent {
    Close,
    ThemeSaved { theme: ThemeKind },
    ShowError { message: String },
}

impl ImportThemeBody {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let url_editor = {
            let editor = ctx.add_typed_action_view(|ctx| {
                EditorView::single_line(SingleLineEditorOptions::default(), ctx)
            });
            ctx.subscribe_to_view(&editor, move |me, _, event, ctx| {
                me.handle_url_editor_event(event, ctx);
            });
            editor
        };

        Self {
            url_editor,
            url_text: String::new(),
            state: FetchState::Idle,
            show_error: None,
            pending_token: 0,
        }
    }

    fn handle_url_editor_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        if let EditorEvent::Edited(_) = event {
            let text = self
                .url_editor
                .read(ctx, |editor, app| editor.buffer_text(app));
            self.on_url_changed(text, ctx);
        }
    }

    fn on_url_changed(&mut self, new_text: String, ctx: &mut ViewContext<Self>) {
        self.url_text = new_text;
        self.show_error = None;
        self.pending_token = self.pending_token.wrapping_add(1);

        if self.url_text.trim().is_empty() {
            self.state = FetchState::Idle;
            ctx.notify();
            return;
        }

        // Validate URL synchronously before debouncing the network call so
        // typos surface immediately.
        if let Err(e) = extract_theme_id(&self.url_text) {
            self.state = FetchState::Error(format_error(&e));
            ctx.notify();
            return;
        }

        let token = self.pending_token;
        let _ = ctx.spawn(
            Timer::after(FETCH_DEBOUNCE),
            move |me: &mut Self, _, ctx| {
                if me.pending_token == token {
                    me.run_fetch(ctx);
                }
            },
        );
        ctx.notify();
    }

    fn run_fetch(&mut self, ctx: &mut ViewContext<Self>) {
        let input = self.url_text.clone();
        self.state = FetchState::Fetching;
        let token = self.pending_token;
        ctx.notify();

        ctx.spawn(
            async move { fetch_share_url(&input).await },
            move |me, result, ctx| {
                if me.pending_token != token {
                    return;
                }
                me.state = match result {
                    Ok(blocks) => FetchState::Fetched(blocks),
                    Err(e) => FetchState::Error(format_error(&e)),
                };
                ctx.notify();
            },
        );
    }

    pub fn can_save(&self) -> bool {
        matches!(&self.state, FetchState::Fetched(blocks) if !blocks.dark.is_empty() || !blocks.light.is_empty())
    }

    pub fn save(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.can_save() {
            return;
        }

        #[cfg(feature = "local_fs")]
        {
            let blocks = match &self.state {
                FetchState::Fetched(b) => b,
                _ => return,
            };

            // Prefer the registry JSON `name`, fall back to the share-link id.
            let slug = blocks
                .name_comment
                .clone()
                .or_else(|| extract_theme_id(&self.url_text).ok())
                .unwrap_or_else(|| "imported-theme".to_string());

            let base_theme = Appearance::as_ref(ctx).theme().clone();
            let themes_dir = user_config::themes_dir();

            match write_imported(blocks, &slug, &base_theme, GamutPolicy::Clamp, &themes_dir) {
                Ok(paths) if !paths.is_empty() => {
                    let path = paths.into_iter().next().unwrap();
                    let display_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&slug)
                        .to_string();
                    let theme = ThemeKind::Custom(CustomTheme::new(display_name, path));
                    ctx.emit(ImportThemeBodyEvent::ThemeSaved { theme });
                    ctx.emit(ImportThemeBodyEvent::Close);
                }
                Ok(_) => {
                    self.show_error = Some("No color blocks were found in the theme.".to_string());
                    ctx.notify();
                }
                Err(e) => {
                    self.show_error = Some(format!("Write failed: {}", format_error(&e)));
                    ctx.emit(ImportThemeBodyEvent::ShowError {
                        message: self.show_error.clone().unwrap(),
                    });
                    ctx.notify();
                }
            }
        }

        #[cfg(not(feature = "local_fs"))]
        {
            self.show_error = Some(
                "Theme import requires a local filesystem, not available in web mode.".to_string(),
            );
            ctx.notify();
        }
    }

    pub fn cancel(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(ImportThemeBodyEvent::Close);
    }
}

fn format_error(e: &ImportError) -> String {
    match e {
        ImportError::InvalidShareUrl(msg) => msg.clone(),
        ImportError::Fetch(msg) => msg.clone(),
        ImportError::NoColorBlocksFound => "Theme has no color blocks.".to_string(),
        ImportError::InvalidOklch { var, raw } => format!("Invalid oklch() for --{var}: {raw}"),
        ImportError::OutOfSrgbGamut { var, .. } => format!("--{var} is outside the sRGB gamut"),
        ImportError::Io(msg) => msg.clone(),
    }
}

impl Entity for ImportThemeBody {
    type Event = ImportThemeBodyEvent;
}

impl View for ImportThemeBody {
    fn ui_name() -> &'static str {
        "ImportThemeBody"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let input_style = UiComponentStyles::default()
            .set_border_color(theme.outline().into())
            .set_font_family_id(appearance.header_font_family())
            .set_font_size(13.)
            .set_background(Fill::None)
            .set_border_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .set_padding(Coords::uniform(8.).top(6.).bottom(6.))
            .set_border_width(1.);

        let button_base = UiComponentStyles {
            font_size: Some(13.),
            font_family_id: Some(appearance.ui_font_family()),
            font_weight: Some(Weight::Bold),
            border_radius: Some(CornerRadius::with_all(Radius::Pixels(4.))),
            padding: Some(Coords::uniform(10.)),
            ..Default::default()
        };

        let save_button_style = UiComponentStyles {
            background: Some(theme.accent().into()),
            border_color: Some(theme.accent().into()),
            font_color: Some(theme.main_text_color(theme.accent()).into()),
            ..button_base
        };

        let cancel_button_style = UiComponentStyles {
            background: Some(theme.surface_1().into()),
            border_color: Some(theme.outline().into()),
            font_color: Some(theme.active_ui_text_color().into()),
            ..button_base
        };

        let disabled_style = UiComponentStyles {
            background: Some(theme.surface_3().into()),
            border_color: Some(theme.outline().into()),
            font_color: Some(theme.disabled_ui_text_color().into()),
            ..button_base
        };

        // ── Status line ───────────────────────────────────────────────────
        let (status_text, status_color, has_light, has_dark) = match &self.state {
            FetchState::Idle => (String::new(), theme.disabled_ui_text_color(), false, false),
            FetchState::Fetching => (
                "Fetching theme…".to_string(),
                theme.disabled_ui_text_color(),
                false,
                false,
            ),
            FetchState::Fetched(blocks) => {
                let name = blocks
                    .name_comment
                    .clone()
                    .unwrap_or_else(|| "Imported theme".to_string());
                (
                    format!("Loaded: {name}"),
                    theme.accent(),
                    !blocks.light.is_empty(),
                    !blocks.dark.is_empty(),
                )
            }
            FetchState::Error(msg) => (
                msg.clone(),
                warp_core::ui::theme::Fill::Solid(pathfinder_color::ColorU {
                    r: 220,
                    g: 50,
                    b: 50,
                    a: 255,
                }),
                false,
                false,
            ),
        };

        let light_text = if has_light {
            "Light: ✓"
        } else {
            "Light: –"
        };
        let dark_text = if has_dark { "Dark: ✓" } else { "Dark: –" };

        // ── Save / Cancel buttons ──────────────────────────────────────────
        let save_button = if self.can_save() {
            appearance
                .ui_builder()
                .button(ButtonVariant::Accent, Default::default())
                .with_style(save_button_style)
                .with_centered_text_label("Save".into())
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(ImportThemeBodyAction::Save);
                })
                .finish()
        } else {
            appearance
                .ui_builder()
                .button(ButtonVariant::Basic, Default::default())
                .with_style(disabled_style)
                .disabled()
                .with_centered_text_label("Save".into())
                .build()
                .finish()
        };

        let cancel_button = appearance
            .ui_builder()
            .button(ButtonVariant::Basic, Default::default())
            .with_style(cancel_button_style)
            .with_centered_text_label("Cancel".into())
            .build()
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(ImportThemeBodyAction::Cancel);
            })
            .finish();

        // ── URL input ─────────────────────────────────────────────────────
        let url_input = Container::new(
            TextInput::new(self.url_editor.clone(), input_style)
                .build()
                .finish(),
        )
        .with_margin_top(6.)
        .finish();

        // ── Status row ────────────────────────────────────────────────────
        let status_row: Box<dyn Element> = if status_text.is_empty() {
            Container::new(
                Text::new_inline("", appearance.ui_font_family(), 12.)
                    .with_color(theme.disabled_ui_text_color().into())
                    .finish(),
            )
            .with_margin_top(10.)
            .finish()
        } else {
            Container::new(
                Text::new_inline(status_text, appearance.ui_font_family(), 12.)
                    .with_color(status_color.into())
                    .finish(),
            )
            .with_margin_top(10.)
            .finish()
        };

        // ── Badge row ─────────────────────────────────────────────────────
        let badge_row = Flex::row()
            .with_child(
                Text::new_inline(light_text, appearance.ui_font_family(), 12.)
                    .with_color(if has_light {
                        theme.accent().into()
                    } else {
                        theme.disabled_ui_text_color().into()
                    })
                    .finish(),
            )
            .with_child(
                Container::new(
                    Text::new_inline(dark_text, appearance.ui_font_family(), 12.)
                        .with_color(if has_dark {
                            theme.accent().into()
                        } else {
                            theme.disabled_ui_text_color().into()
                        })
                        .finish(),
                )
                .with_margin_left(16.)
                .finish(),
            )
            .finish();

        // ── Error banner ──────────────────────────────────────────────────
        let maybe_error: Option<Box<dyn Element>> = self.show_error.as_ref().map(|msg| {
            let error_msg = msg.clone();
            Container::new(
                Text::new_inline(error_msg, appearance.ui_font_family(), 12.)
                    .with_color(pathfinder_color::ColorU {
                        r: 220,
                        g: 50,
                        b: 50,
                        a: 255,
                    })
                    .finish(),
            )
            .with_margin_top(8.)
            .finish() as Box<dyn Element>
        });

        // ── Button row ────────────────────────────────────────────────────
        let button_row = Container::new(
            Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_child(
                    Shrinkable::new(
                        0.5,
                        Container::new(cancel_button).with_margin_right(8.).finish(),
                    )
                    .finish(),
                )
                .with_child(Shrinkable::new(0.5, save_button).finish())
                .finish(),
        )
        .with_margin_top(16.)
        .finish();

        // ── Layout ───────────────────────────────────────────────────────
        let mut layout = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

        layout.add_child(
            Text::new_inline(
                "Paste a tweakcn share link (e.g. https://tweakcn.com/themes/…)",
                appearance.ui_font_family(),
                12.,
            )
            .with_color(theme.active_ui_text_color().into())
            .finish(),
        );
        layout.add_child(url_input);

        layout.add_child(status_row);
        layout.add_child(Container::new(badge_row).with_margin_top(8.).finish());

        if let Some(error_element) = maybe_error {
            layout.add_child(error_element);
        }

        layout.add_child(button_row);

        layout.finish()
    }
}

impl TypedActionView for ImportThemeBody {
    type Action = ImportThemeBodyAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ImportThemeBodyAction::Save => self.save(ctx),
            ImportThemeBodyAction::Cancel => self.cancel(ctx),
        }
    }
}

// ─── ImportThemeModal (outer shell) ──────────────────────────────────────────

pub struct ImportThemeModal {
    modal: ViewHandle<Modal<ImportThemeBody>>,
}

#[derive(Debug)]
pub enum ImportThemeModalAction {
    Cancel,
}

pub enum ImportThemeModalEvent {
    Close,
    ThemeSaved { theme: ThemeKind },
    ShowErrorToast { message: String },
}

pub fn init(app: &mut warpui::AppContext) {
    use warpui::keymap::macros::*;
    use warpui::keymap::FixedBinding;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        ImportThemeModalAction::Cancel,
        id!("ImportThemeModal"),
    )]);
}

impl ImportThemeModal {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let body = ctx.add_typed_action_view(ImportThemeBody::new);

        ctx.subscribe_to_view(&body, move |me, _, event, ctx| {
            me.handle_body_event(event, ctx);
        });

        let modal = ctx.add_typed_action_view(|ctx| {
            Modal::new(Some(MODAL_HEADER.to_string()), body, ctx)
                .with_modal_style(UiComponentStyles {
                    width: Some(MODAL_WIDTH),
                    height: Some(MODAL_HEIGHT),
                    ..Default::default()
                })
                .with_header_style(UiComponentStyles {
                    padding: Some(Coords {
                        top: 24.,
                        bottom: 0.,
                        left: 24.,
                        right: 24.,
                    }),
                    font_size: Some(16.),
                    font_weight: Some(Weight::Bold),
                    ..Default::default()
                })
                .with_body_style(UiComponentStyles {
                    padding: Some(Coords {
                        top: 16.,
                        bottom: 24.,
                        left: 24.,
                        right: 24.,
                    }),
                    height: Some(0.),
                    ..Default::default()
                })
                .with_background_opacity(100)
                .with_dismiss_on_click()
                .close_modal_button_disabled()
        });

        Self { modal }
    }

    pub fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(ImportThemeModalEvent::Close);
    }

    pub fn cancel(&mut self, ctx: &mut ViewContext<Self>) {
        self.modal.update(ctx, |modal, ctx| {
            modal.body().update(ctx, |body, ctx| {
                body.cancel(ctx);
            });
        });
    }

    fn handle_body_event(&mut self, event: &ImportThemeBodyEvent, ctx: &mut ViewContext<Self>) {
        match event {
            ImportThemeBodyEvent::Close => {
                self.close(ctx);
            }
            ImportThemeBodyEvent::ThemeSaved { theme } => {
                ctx.emit(ImportThemeModalEvent::ThemeSaved {
                    theme: theme.clone(),
                });
            }
            ImportThemeBodyEvent::ShowError { message } => {
                ctx.emit(ImportThemeModalEvent::ShowErrorToast {
                    message: message.clone(),
                });
            }
        }
    }
}

impl Entity for ImportThemeModal {
    type Event = ImportThemeModalEvent;
}

impl View for ImportThemeModal {
    fn ui_name() -> &'static str {
        "ImportThemeModal"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.modal).finish()
    }
}

impl TypedActionView for ImportThemeModal {
    type Action = ImportThemeModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ImportThemeModalAction::Cancel => self.cancel(ctx),
        }
    }
}
