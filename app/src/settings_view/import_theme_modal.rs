//! Import-theme modal for tweakcn CSS exports.
//!
//! Opens when the user presses "Import theme…" in Appearance settings.
//! The user pastes tweakcn CSS, optionally edits the theme name, and clicks
//! Save. The modal calls `write_imported` to write YAML(s) to disk, then
//! dispatches a theme-reload+select event so the new theme is immediately
//! active.
//!
//! ## Drag-and-drop
//! The modal body is wrapped in a `FileDropZone` element (inner module) that
//! intercepts `Event::DragAndDropFiles` from the OS and dispatches a
//! `ImportThemeBodyAction::FileDropped` action.  Only `.css` files are
//! accepted; anything else is rejected with an inline error.

use std::any::Any;
use std::time::Duration;

use crate::appearance::Appearance;
use crate::editor::{EditorOptions, EditorView, Event as EditorEvent, SingleLineEditorOptions};
use crate::modal::Modal;
use crate::themes::theme::{CustomTheme, ThemeKind};
use crate::themes::tweakcn_import::{parse_blocks, write_imported, GamutPolicy, ParsedBlocks};
#[cfg(feature = "local_fs")]
use crate::user_config;
use warpui::elements::Point;
use warpui::elements::{
    ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Fill, Flex, MainAxisSize,
    ParentElement, Radius, Shrinkable, Text,
};
use warpui::event::DispatchedEvent;
use warpui::fonts::Weight;
use warpui::geometry::vector::Vector2F;
use warpui::presenter::ChildView;
use warpui::r#async::Timer;
use warpui::ui_components::button::ButtonVariant;
use warpui::ui_components::components::{Coords, UiComponent as _, UiComponentStyles};
use warpui::ui_components::text_input::TextInput;
use warpui::ViewHandle;
use warpui::{
    AfterLayoutContext, AppContext, Element, Entity, Event, EventContext, LayoutContext,
    PaintContext, SingletonEntity as _, SizeConstraint, TypedActionView, View, ViewContext,
};

// ─── FileDropZone ─────────────────────────────────────────────────────────────
//
// A transparent element wrapper that sits over any child and intercepts OS-level
// DragAndDropFiles events, forwarding them as a typed action to the view.

struct FileDropZone {
    child: Box<dyn Element>,
}

impl FileDropZone {
    fn new(child: Box<dyn Element>) -> Self {
        Self { child }
    }
}

impl Element for FileDropZone {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        self.child.layout(constraint, ctx, app)
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.child.paint(origin, ctx, app);
    }

    fn size(&self) -> Option<Vector2F> {
        self.child.size()
    }

    fn origin(&self) -> Option<Point> {
        self.child.origin()
    }

    fn parent_data(&self) -> Option<&dyn Any> {
        self.child.parent_data()
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        if let Some(z_index) = self.z_index() {
            if let Some(inner) = event.at_z_index(z_index, ctx) {
                if let Event::DragAndDropFiles { paths, location } = inner {
                    if self.bounds().map_or(false, |b| b.contains_point(*location))
                        && !paths.is_empty()
                    {
                        let paths: Vec<String> = paths.iter().map(ToOwned::to_owned).collect();
                        ctx.dispatch_typed_action(ImportThemeBodyAction::FileDropped(paths));
                        return true;
                    }
                }
            }
        }

        self.child.dispatch_event(event, ctx, app)
    }
}

const MODAL_HEADER: &str = "Import theme from tweakcn";
const MODAL_WIDTH: f32 = 560.;
const MODAL_HEIGHT: f32 = 520.;
const CSS_EDITOR_MAX_HEIGHT: f32 = 240.;
const PARSE_DEBOUNCE: Duration = Duration::from_millis(200);

// ─── ImportThemeBody ─────────────────────────────────────────────────────────

pub struct ImportThemeBody {
    /// The multi-line editor used for pasting CSS.
    css_editor: ViewHandle<EditorView>,
    /// The single-line editor used for the theme name/slug.
    name_editor: ViewHandle<EditorView>,
    /// Current raw CSS (mirrors the css_editor buffer).
    css_text: String,
    /// Current theme name (mirrors the name_editor buffer).
    name: String,
    /// Last parse result (re-computed whenever css_text changes).
    parse_result: Option<Result<ParsedBlocks, String>>,
    /// Whether to clamp out-of-gamut colors (default `true`).
    clamp_out_of_gamut: bool,
    /// Last save/write error to display in the UI.
    pub(crate) show_error: Option<String>,
    pending_parse_token: u64,
}

#[derive(Debug)]
pub enum ImportThemeBodyAction {
    Save,
    Cancel,
    ToggleClamp,
    /// OS-level file-drop: the vec contains the absolute path strings of the
    /// dropped items (may be multiple; only the first `.css` one is used).
    FileDropped(Vec<String>),
}

pub enum ImportThemeBodyEvent {
    Close,
    ThemeSaved { theme: ThemeKind },
    ShowError { message: String },
}

impl ImportThemeBody {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let css_editor = {
            let editor = ctx.add_typed_action_view(|ctx| {
                EditorView::new(
                    EditorOptions {
                        soft_wrap: true,
                        ..Default::default()
                    },
                    ctx,
                )
            });
            ctx.subscribe_to_view(&editor, move |me, _, event, ctx| {
                me.handle_css_editor_event(event, ctx);
            });
            editor
        };

        let name_editor = {
            let editor = ctx.add_typed_action_view(|ctx| {
                EditorView::single_line(SingleLineEditorOptions::default(), ctx)
            });
            ctx.subscribe_to_view(&editor, move |me, _, event, ctx| {
                me.handle_name_editor_event(event, ctx);
            });
            editor
        };

        Self {
            css_editor,
            name_editor,
            css_text: String::new(),
            name: String::new(),
            parse_result: None,
            clamp_out_of_gamut: true,
            show_error: None,
            pending_parse_token: 0,
        }
    }

    fn handle_css_editor_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        if let EditorEvent::Edited(_) = event {
            let text = self
                .css_editor
                .read(ctx, |editor, app| editor.buffer_text(app));
            self.on_css_changed(text, ctx);
        }
    }

    fn handle_name_editor_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        if let EditorEvent::Edited(_) = event {
            let text = self
                .name_editor
                .read(ctx, |editor, app| editor.buffer_text(app));
            self.name = text;
            ctx.notify();
        }
    }

    fn on_css_changed(&mut self, new_text: String, ctx: &mut ViewContext<Self>) {
        self.css_text = new_text;
        self.show_error = None;

        if self.css_text.trim().is_empty() {
            self.parse_result = None;
            self.pending_parse_token = self.pending_parse_token.wrapping_add(1);
            ctx.notify();
            return;
        }

        self.pending_parse_token = self.pending_parse_token.wrapping_add(1);
        let token = self.pending_parse_token;
        let _ = ctx.spawn(
            Timer::after(PARSE_DEBOUNCE),
            move |me: &mut Self, _, ctx| {
                if me.pending_parse_token == token {
                    me.run_parse(ctx);
                }
            },
        );

        ctx.notify();
    }

    fn run_parse(&mut self, ctx: &mut ViewContext<Self>) {
        if self.css_text.trim().is_empty() {
            self.parse_result = None;
            ctx.notify();
            return;
        }

        match parse_blocks(&self.css_text) {
            Ok(blocks) => {
                // Auto-fill name from CSS comment hint if the name field is still empty.
                if self.name.is_empty() {
                    if let Some(hint) = blocks.name_comment.as_deref() {
                        self.name = hint.to_string();
                        let name_clone = self.name.clone();
                        self.name_editor.update(ctx, |editor, ctx| {
                            editor.set_buffer_text(&name_clone, ctx);
                        });
                    }
                }
                self.parse_result = Some(Ok(blocks));
            }
            Err(e) => {
                self.parse_result = Some(Err(format!("{e:?}")));
            }
        }
        ctx.notify();
    }

    pub fn can_save(&self) -> bool {
        if self.name.trim().is_empty() {
            return false;
        }
        match &self.parse_result {
            Some(Ok(blocks)) => !blocks.dark.is_empty() || !blocks.light.is_empty(),
            _ => false,
        }
    }

    pub fn save(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.can_save() {
            return;
        }

        #[cfg(feature = "local_fs")]
        {
            let blocks = match &self.parse_result {
                Some(Ok(b)) => b,
                _ => return,
            };

            let slug = self.name.trim().to_string();
            let policy = if self.clamp_out_of_gamut {
                GamutPolicy::Clamp
            } else {
                GamutPolicy::Strict
            };

            let base_theme = Appearance::as_ref(ctx).theme().clone();
            let themes_dir = user_config::themes_dir();

            match write_imported(&blocks, &slug, &base_theme, policy, &themes_dir) {
                Ok(paths) if !paths.is_empty() => {
                    // Use the first written path (dark variant, or light if only light).
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
                    self.show_error =
                        Some("No color blocks were written — check your CSS.".to_string());
                    ctx.notify();
                }
                Err(e) => {
                    self.show_error = Some(format!("Write failed: {e:?}"));
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

    /// Handle a file dropped onto the modal (OS DragAndDropFiles event).
    ///
    /// Accepts the first `.css` file found in `paths`.  Non-`.css` files (or
    /// an empty list) show an inline error and leave the paste box untouched.
    ///
    /// Gated on `local_fs` because the fallback (web) has no filesystem access
    /// and the event is not reachable there anyway.
    #[cfg(feature = "local_fs")]
    pub fn on_file_dropped(&mut self, paths: Vec<String>, ctx: &mut ViewContext<Self>) {
        use std::path::Path;

        let css_path = paths
            .iter()
            .map(Path::new)
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("css"));

        let path = match css_path {
            Some(p) => p,
            None => {
                self.show_error = Some("Only .css files are supported.".to_string());
                ctx.notify();
                return;
            }
        };

        match std::fs::read_to_string(path) {
            Ok(contents) => {
                // Use the filename stem as a default slug if the modal name is empty.
                if self.name.is_empty() {
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("imported-theme")
                        .to_string();
                    self.name = stem.clone();
                    self.name_editor.update(ctx, |editor, ctx| {
                        editor.set_buffer_text(&stem, ctx);
                    });
                }
                // Populate the CSS editor buffer so the user can see what was loaded.
                let contents_clone = contents.clone();
                self.css_editor.update(ctx, |editor, ctx| {
                    editor.set_buffer_text(&contents_clone, ctx);
                });
                self.css_text = contents;
                self.pending_parse_token = self.pending_parse_token.wrapping_add(1);
                self.run_parse(ctx);
            }
            Err(e) => {
                self.show_error = Some(format!("Read failed: {e}"));
                ctx.notify();
            }
        }
    }

    #[cfg(not(feature = "local_fs"))]
    pub fn on_file_dropped(&mut self, _paths: Vec<String>, ctx: &mut ViewContext<Self>) {
        self.show_error =
            Some("File drop requires a local filesystem, not available in web mode.".to_string());
        ctx.notify();
    }

    pub fn toggle_clamp(&mut self, ctx: &mut ViewContext<Self>) {
        self.clamp_out_of_gamut = !self.clamp_out_of_gamut;
        ctx.notify();
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

        // ── Shared styles ──────────────────────────────────────────────────
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

        // ── Parse status badges ────────────────────────────────────────────
        let (has_light, has_dark) = match &self.parse_result {
            Some(Ok(blocks)) => (!blocks.light.is_empty(), !blocks.dark.is_empty()),
            _ => (false, false),
        };

        let light_text = if has_light {
            "Light: ✓"
        } else {
            "Light: –"
        };
        let dark_text = if has_dark { "Dark: ✓" } else { "Dark: –" };

        // ── Clamp toggle text ──────────────────────────────────────────────
        let clamp_indicator = if self.clamp_out_of_gamut {
            "☑"
        } else {
            "☐"
        };
        let clamp_label = format!("{clamp_indicator} Clamp out-of-gamut colors");

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

        // ── CSS editor ────────────────────────────────────────────────────
        let css_input = Container::new(
            ConstrainedBox::new(
                TextInput::new(self.css_editor.clone(), input_style)
                    .build()
                    .finish(),
            )
            .with_max_height(CSS_EDITOR_MAX_HEIGHT)
            .finish(),
        )
        .with_margin_top(6.)
        .finish();

        // ── Name editor ───────────────────────────────────────────────────
        let name_input = Container::new(
            TextInput::new(
                self.name_editor.clone(),
                UiComponentStyles::default()
                    .set_border_color(theme.outline().into())
                    .set_font_family_id(appearance.header_font_family())
                    .set_font_size(13.)
                    .set_background(Fill::None)
                    .set_border_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                    .set_padding(Coords::uniform(8.).top(6.).bottom(6.))
                    .set_border_width(1.),
            )
            .build()
            .finish(),
        )
        .with_margin_top(6.)
        .finish();

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

        // ── Clamp toggle ──────────────────────────────────────────────────
        let clamp_row = warpui::elements::EventHandler::new(
            Text::new_inline(clamp_label, appearance.ui_font_family(), 12.)
                .with_color(theme.active_ui_text_color().into())
                .finish(),
        )
        .on_left_mouse_down(|ctx, _, _| {
            ctx.dispatch_typed_action(ImportThemeBodyAction::ToggleClamp);
            warpui::elements::DispatchEventResult::StopPropagation
        })
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

        // CSS paste label + field
        layout.add_child(
            Text::new_inline("Paste tweakcn CSS", appearance.ui_font_family(), 12.)
                .with_color(theme.active_ui_text_color().into())
                .finish(),
        );
        layout.add_child(css_input);

        // Name label + field
        layout.add_child(
            Container::new(
                Text::new_inline("Theme name (slug)", appearance.ui_font_family(), 12.)
                    .with_color(theme.active_ui_text_color().into())
                    .finish(),
            )
            .with_margin_top(12.)
            .finish(),
        );
        layout.add_child(name_input);

        // Detected blocks row
        layout.add_child(Container::new(badge_row).with_margin_top(10.).finish());

        // Clamp toggle
        layout.add_child(Container::new(clamp_row).with_margin_top(8.).finish());

        // Error (conditional)
        if let Some(error_element) = maybe_error {
            layout.add_child(error_element);
        }

        // Button row
        layout.add_child(button_row);

        // Wrap the whole layout in a FileDropZone so OS-level .css file drops
        // are captured and dispatched as ImportThemeBodyAction::FileDropped.
        Box::new(FileDropZone::new(layout.finish()))
    }
}

impl TypedActionView for ImportThemeBody {
    type Action = ImportThemeBodyAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ImportThemeBodyAction::Save => self.save(ctx),
            ImportThemeBodyAction::Cancel => self.cancel(ctx),
            ImportThemeBodyAction::ToggleClamp => self.toggle_clamp(ctx),
            ImportThemeBodyAction::FileDropped(paths) => {
                self.on_file_dropped(paths.clone(), ctx);
            }
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
