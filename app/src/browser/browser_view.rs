use std::{cell::RefCell, rc::Rc};
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use pathfinder_geometry::{
    rect::RectF,
    vector::{vec2f, Vector2F},
};
use warpui::{
    elements::{
        AfterLayoutContext, Border, ChildView, Clipped, ConstrainedBox, Container, CornerRadius,
        CrossAxisAlignment, Element, EventContext, Expanded, Flex, LayoutContext, MainAxisSize,
        MouseStateHandle, PaintContext, ParentElement as _, Point, Radius, SizeConstraint,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle, WindowId,
};

use crate::{
    appearance::Appearance,
    editor::{
        EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys, SingleLineEditorOptions,
        TextOptions,
    },
    pane_group::{
        focus_state::PaneFocusHandle,
        pane::view::{self, HeaderContent, StandardHeader, StandardHeaderOptions},
        BackingView, PaneConfiguration, PaneEvent,
    },
    ui_components::{blended_colors, buttons::icon_button_with_color, icons::Icon},
};

use super::BrowserModel;

const URL_BAR_HEIGHT: f32 = 32.0;
const TOOLBAR_HEIGHT: f32 = 48.0;
const TOOLBAR_HORIZONTAL_PADDING: f32 = 10.0;
const TOOLBAR_BUTTON_GAP: f32 = 6.0;
const URL_BAR_BORDER_RADIUS: f32 = 6.0;
const URL_BAR_PLACEHOLDER: &str = "Enter URL";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserViewEvent {
    Pane(PaneEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserViewAction {
    Back,
    Forward,
    Reload,
}

struct NativeBrowserWebView {
    #[cfg(not(target_family = "wasm"))]
    webview: Option<wry::WebView>,
    title_tx: async_channel::Sender<String>,
    pending_url: Option<String>,
    bounds: Option<RectF>,
    attach_error_logged: bool,
}

impl NativeBrowserWebView {
    fn new(initial_url: impl Into<String>, title_tx: async_channel::Sender<String>) -> Self {
        Self {
            #[cfg(not(target_family = "wasm"))]
            webview: None,
            title_tx,
            pending_url: Some(initial_url.into()),
            bounds: None,
            attach_error_logged: false,
        }
    }

    fn load_url(&mut self, url: &str) {
        self.pending_url = Some(url.to_string());

        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.load_url(url) {
                log::warn!("failed to load browser pane URL {url}: {err}");
            }
        }
    }

    fn go_back(&self) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.evaluate_script("history.back()") {
                log::warn!("failed to navigate browser pane back: {err}");
            }
        }
    }

    fn go_forward(&self) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.evaluate_script("history.forward()") {
                log::warn!("failed to navigate browser pane forward: {err}");
            }
        }
    }

    fn reload(&self) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.evaluate_script("location.reload()") {
                log::warn!("failed to reload browser pane: {err}");
            }
        }
    }

    fn set_bounds(&mut self, window_id: WindowId, bounds: RectF, app: &AppContext) {
        self.bounds = Some(bounds);
        self.attach_if_needed(window_id, bounds, app);

        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            let rect = Self::wry_rect(bounds);

            if let Err(err) = webview.set_bounds(rect) {
                log::warn!("failed to resize browser pane webview: {err}");
            }
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn wry_rect(bounds: RectF) -> wry::Rect {
        let size = bounds.size();
        wry::Rect {
            x: bounds.min_x().round() as i32,
            y: bounds.min_y().round() as i32,
            width: size.x().max(0.0).round() as u32,
            height: size.y().max(0.0).round() as u32,
        }
    }

    fn attach_if_needed(&mut self, window_id: WindowId, bounds: RectF, app: &AppContext) {
        #[cfg(target_os = "macos")]
        {
            if self.webview.is_some()
                || self.attach_error_logged
                || app.windows().active_window() != Some(window_id)
            {
                return;
            }

            let Some(parent) = active_appkit_view_handle() else {
                return;
            };

            let url = self.pending_url.clone().unwrap_or_default();
            let title_tx = self.title_tx.clone();
            match wry::WebViewBuilder::new_as_child(&parent)
                .with_url(url)
                .with_bounds(Self::wry_rect(bounds))
                .with_accept_first_mouse(true)
                .with_document_title_changed_handler(move |title| {
                    let _ = title_tx.try_send(title);
                })
                .build()
            {
                Ok(webview) => {
                    self.webview = Some(webview);
                }
                Err(err) => {
                    self.attach_error_logged = true;
                    log::warn!("failed to attach browser pane webview: {err}");
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        let _ = (window_id, bounds, app);
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct BorrowedAppKitView {
    native_view: NonNull<c_void>,
}

#[cfg(target_os = "macos")]
impl wry::raw_window_handle::HasWindowHandle for BorrowedAppKitView {
    fn window_handle(
        &self,
    ) -> Result<wry::raw_window_handle::WindowHandle<'_>, wry::raw_window_handle::HandleError> {
        let appkit_window_handle =
            wry::raw_window_handle::AppKitWindowHandle::new(self.native_view.cast());
        Ok(unsafe {
            wry::raw_window_handle::WindowHandle::borrow_raw(
                wry::raw_window_handle::RawWindowHandle::AppKit(appkit_window_handle),
            )
        })
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn active_appkit_view_handle() -> Option<BorrowedAppKitView> {
    use cocoa::{
        appkit::NSApp,
        base::{id, nil},
    };
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let app = NSApp();
        if app == nil {
            return None;
        }

        let window: id = msg_send![app, keyWindow];
        if window == nil {
            return None;
        }

        let native_view: id = msg_send![window, contentView];
        NonNull::new(native_view as *mut c_void)
            .map(|native_view| BorrowedAppKitView { native_view })
    }
}

struct NativeWebViewElement {
    webview: Rc<RefCell<NativeBrowserWebView>>,
    window_id: WindowId,
    size: Option<Vector2F>,
    origin: Option<Point>,
}

impl NativeWebViewElement {
    fn new(webview: Rc<RefCell<NativeBrowserWebView>>, window_id: WindowId) -> Self {
        Self {
            webview,
            window_id,
            size: None,
            origin: None,
        }
    }
}

impl Element for NativeWebViewElement {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        _ctx: &mut LayoutContext,
        _app: &AppContext,
    ) -> Vector2F {
        let max_constraint = constraint.max;
        let size = vec2f(
            if max_constraint.x().is_infinite() {
                constraint.min.x()
            } else {
                max_constraint.x()
            },
            if max_constraint.y().is_infinite() {
                constraint.min.y()
            } else {
                max_constraint.y()
            },
        );
        self.size = Some(size);
        size
    }

    fn after_layout(&mut self, _ctx: &mut AfterLayoutContext, _app: &AppContext) {}

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.origin = Some(Point::from_vec2f(origin, ctx.scene.z_index()));

        if let Some(size) = self.size {
            self.webview
                .borrow_mut()
                .set_bounds(self.window_id, RectF::new(origin, size), app);
        }
    }

    fn dispatch_event(
        &mut self,
        _event: &warpui::event::DispatchedEvent,
        _ctx: &mut EventContext,
        _app: &AppContext,
    ) -> bool {
        false
    }

    fn size(&self) -> Option<Vector2F> {
        self.size
    }

    fn origin(&self) -> Option<Point> {
        self.origin
    }
}

pub struct BrowserView {
    model: BrowserModel,
    window_id: WindowId,
    url_editor: ViewHandle<EditorView>,
    native_webview: Rc<RefCell<NativeBrowserWebView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    back_button_mouse_state: MouseStateHandle,
    forward_button_mouse_state: MouseStateHandle,
    reload_button_mouse_state: MouseStateHandle,
}

impl BrowserView {
    pub fn new(initial_url: Option<String>, ctx: &mut ViewContext<Self>) -> Self {
        let model = BrowserModel::new(initial_url.unwrap_or_default());
        let pane_configuration =
            ctx.add_model(|_ctx| PaneConfiguration::new(model.display_title()));
        let (title_tx, title_rx) = async_channel::unbounded();
        let native_webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
            model.current_url().to_string(),
            title_tx,
        )));
        let current_url = model.current_url().to_string();

        let url_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(12.0), appearance),
                    select_all_on_focus: true,
                    clear_selections_on_blur: true,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    ..Default::default()
                },
                ctx,
            );
            editor.set_placeholder_text(URL_BAR_PLACEHOLDER, ctx);
            editor.set_buffer_text_with_base_buffer(&current_url, ctx);
            editor
        });

        ctx.subscribe_to_view(&url_editor, move |view, _, event, ctx| {
            if matches!(event, EditorEvent::Enter) {
                view.navigate_to_editor_url(ctx);
            }
        });
        ctx.spawn_stream_local(title_rx, Self::handle_document_title, |_, _| {});

        Self {
            model,
            window_id: ctx.window_id(),
            url_editor,
            native_webview,
            pane_configuration,
            focus_handle: None,
            back_button_mouse_state: MouseStateHandle::default(),
            forward_button_mouse_state: MouseStateHandle::default(),
            reload_button_mouse_state: MouseStateHandle::default(),
        }
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    pub fn current_url(&self) -> &str {
        self.model.current_url()
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.url_editor);
    }

    fn navigate_to_editor_url(&mut self, ctx: &mut ViewContext<Self>) {
        let url = self.url_editor.as_ref(ctx).buffer_text(ctx);
        self.navigate(url, ctx);
    }

    fn navigate(&mut self, url: impl Into<String>, ctx: &mut ViewContext<Self>) {
        if let Some(url) = self.model.navigate(url) {
            self.url_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer(&url, ctx);
            });
            self.native_webview.borrow_mut().load_url(&url);
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn go_back(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(url) = self.model.go_back() {
            self.url_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer(&url, ctx);
            });
            self.native_webview.borrow().go_back();
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn go_forward(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(url) = self.model.go_forward() {
            self.url_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer(&url, ctx);
            });
            self.native_webview.borrow().go_forward();
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn reload(&mut self, ctx: &mut ViewContext<Self>) {
        self.model.reload();
        self.native_webview.borrow().reload();
        self.sync_pane_title(ctx);
        ctx.notify();
    }

    fn handle_document_title(&mut self, title: String, ctx: &mut ViewContext<Self>) {
        if self.model.set_title(title) {
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn sync_pane_title(&self, ctx: &mut ViewContext<Self>) {
        self.pane_configuration.update(ctx, |configuration, ctx| {
            configuration.set_title(self.model.display_title(), ctx);
            configuration.set_title_secondary(self.model.current_url(), ctx);
        });
    }

    fn render_toolbar_button(
        &self,
        icon: Icon,
        tooltip: &'static str,
        mouse_state: MouseStateHandle,
        active: bool,
        disabled: bool,
        action: BrowserViewAction,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder().clone();
        let color = if disabled {
            blended_colors::text_disabled(theme, theme.background()).into()
        } else {
            blended_colors::text_main(theme, theme.background()).into()
        };

        let mut button = icon_button_with_color(appearance, icon, active, mouse_state, color)
            .with_tooltip(move || ui_builder.tool_tip(tooltip.to_string()).build().finish());

        if disabled {
            button = button.disabled();
        }

        button
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(action.clone());
            })
            .finish()
    }

    fn render_toolbar(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let mut toolbar = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Max);

        toolbar.add_child(self.render_toolbar_button(
            Icon::ArrowLeft,
            "Back",
            self.back_button_mouse_state.clone(),
            false,
            !self.model.can_go_back(),
            BrowserViewAction::Back,
            app,
        ));
        toolbar.add_child(
            Container::new(self.render_toolbar_button(
                Icon::ArrowRight,
                "Forward",
                self.forward_button_mouse_state.clone(),
                false,
                !self.model.can_go_forward(),
                BrowserViewAction::Forward,
                app,
            ))
            .with_margin_left(TOOLBAR_BUTTON_GAP)
            .finish(),
        );
        toolbar.add_child(
            Container::new(self.render_toolbar_button(
                Icon::Refresh,
                "Reload",
                self.reload_button_mouse_state.clone(),
                self.model.is_loading(),
                false,
                BrowserViewAction::Reload,
                app,
            ))
            .with_margin_left(TOOLBAR_BUTTON_GAP)
            .finish(),
        );

        let editor = Container::new(
            ConstrainedBox::new(Clipped::new(ChildView::new(&self.url_editor).finish()).finish())
                .with_height(URL_BAR_HEIGHT)
                .finish(),
        )
        .with_horizontal_padding(10.0)
        .with_background(theme.surface_1())
        .with_border(Border::all(1.0).with_border_fill(theme.surface_3()))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
            URL_BAR_BORDER_RADIUS,
        )))
        .finish();

        toolbar.add_child(
            Expanded::new(
                1.0,
                Container::new(editor)
                    .with_margin_left(TOOLBAR_HORIZONTAL_PADDING)
                    .finish(),
            )
            .finish(),
        );

        ConstrainedBox::new(
            Container::new(toolbar.finish())
                .with_horizontal_padding(TOOLBAR_HORIZONTAL_PADDING)
                .with_background(theme.background())
                .finish(),
        )
        .with_height(TOOLBAR_HEIGHT)
        .finish()
    }
}

impl Entity for BrowserView {
    type Event = BrowserViewEvent;
}

impl View for BrowserView {
    fn ui_name() -> &'static str {
        "BrowserView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let theme = Appearance::as_ref(app).theme();
        let webview = Container::new(
            NativeWebViewElement::new(self.native_webview.clone(), self.window_id).finish(),
        )
        .with_background(theme.background())
        .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(self.render_toolbar(app))
            .with_child(Expanded::new(1.0, webview).finish())
            .finish()
    }
}

impl TypedActionView for BrowserView {
    type Action = BrowserViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            BrowserViewAction::Back => self.go_back(ctx),
            BrowserViewAction::Forward => self.go_forward(ctx),
            BrowserViewAction::Reload => self.reload(ctx),
        }
    }
}

impl BackingView for BrowserView {
    type PaneHeaderOverflowMenuAction = BrowserViewAction;
    type CustomAction = BrowserViewAction;
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(BrowserViewEvent::Pane(PaneEvent::Close));
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        self.focus(ctx);
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        app: &AppContext,
    ) -> HeaderContent {
        let theme = Appearance::as_ref(app).theme();
        HeaderContent::Standard(StandardHeader {
            title: self.model.display_title().to_string(),
            title_secondary: Some(self.model.current_url().to_string()),
            title_style: None,
            title_clip_config: warpui::text_layout::ClipConfig::start(),
            title_max_width: None,
            left_of_title: Some(Icon::Globe.to_warpui_icon(theme.foreground()).finish()),
            right_of_title: None,
            left_of_overflow: None,
            options: StandardHeaderOptions {
                always_show_icons: true,
                ..StandardHeaderOptions::default()
            },
        })
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
