use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

use pathfinder_geometry::{
    rect::RectF,
    vector::{vec2f, Vector2F},
};
use warpui::{
    elements::{
        AfterLayoutContext, Align, Border, ChildView, Clipped, ConstrainedBox, Container,
        CornerRadius, CrossAxisAlignment, Element, EventContext, Expanded, Flex, Hoverable,
        LayoutContext, MainAxisSize, MouseStateHandle, PaintContext, ParentElement as _, Point,
        Radius, SizeConstraint, Text,
    },
    text_layout::ClipConfig,
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

use super::about_home;
use super::browser_model::{BrowserModel, TabId, DEFAULT_BROWSER_URL};
use super::persistence;
use super::url_input::{resolve_with_engine, Resolved};
use super::webview_host::NativeBrowserWebView;
use crate::terminal::general_settings::GeneralSettings;

/// Map a model-side URL to the URL the webview should actually load.
/// `about:home` is rendered from a bundled HTML page served as a `data:` URL;
/// every other URL is loaded verbatim.
fn webview_url_for(model_url: &str) -> String {
    if model_url == "about:home" {
        about_home::url()
    } else {
        model_url.to_string()
    }
}

const URL_BAR_HEIGHT: f32 = 32.0;
const URL_BAR_MIN_WIDTH: f32 = 160.0;
// Toolbar = URL bar height + 4pt total vertical padding (2pt each side). The
// previous 48pt left ~16pt of dead space around a 32pt input and made the
// browser chrome look bulky relative to neighboring panes.
const TOOLBAR_HEIGHT: f32 = 36.0;
const TAB_STRIP_HEIGHT: f32 = 32.0;
const TAB_MAX_WIDTH: f32 = 200.0;
const TAB_MIN_WIDTH: f32 = 80.0;
const TAB_HEIGHT: f32 = 26.0;
const TAB_CHIP_PADDING: f32 = 8.0;
const TAB_CLOSE_BUTTON_SIZE: f32 = 16.0;
const TOOLBAR_HORIZONTAL_PADDING: f32 = 10.0;
const TOOLBAR_BUTTON_GAP: f32 = 6.0;
const TAB_GAP: f32 = 2.0;
const URL_BAR_BORDER_RADIUS: f32 = 6.0;
const TAB_BORDER_RADIUS: f32 = 4.0;
const URL_BAR_PLACEHOLDER: &str = "URL or search the web";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserViewEvent {
    Pane(PaneEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserViewAction {
    Back,
    Forward,
    Reload,
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    OpenExternal,
    Collapse,
}

#[derive(Default, Clone)]
struct TabUiState {
    chip_mouse: MouseStateHandle,
    close_mouse: MouseStateHandle,
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
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    /// Per-tab native webviews, aligned by index with `model.tabs()`.
    webviews: Vec<Rc<RefCell<NativeBrowserWebView>>>,
    /// Channel for tab-tagged document title updates from all webviews.
    title_tx: async_channel::Sender<(TabId, String)>,
    /// Per-tab UI mouse states keyed by stable [`TabId`] so they survive tab
    /// closures (which shift indices).
    tab_ui_states: HashMap<TabId, TabUiState>,
    back_button_mouse_state: MouseStateHandle,
    forward_button_mouse_state: MouseStateHandle,
    reload_button_mouse_state: MouseStateHandle,
    new_tab_button_mouse_state: MouseStateHandle,
    collapse_button_mouse_state: MouseStateHandle,
    open_external_button_mouse_state: MouseStateHandle,
}

impl BrowserView {
    /// Read-only access to the underlying model. Used by the workspace to
    /// snapshot tab state for persistence.
    pub(crate) fn model(&self) -> &BrowserModel {
        &self.model
    }
}

impl BrowserView {
    pub fn new(initial_url: Option<String>, ctx: &mut ViewContext<Self>) -> Self {
        let model = BrowserModel::new(initial_url.unwrap_or_default());
        let pane_configuration =
            ctx.add_model(|_ctx| PaneConfiguration::new(model.display_title()));
        let (title_tx, title_rx) = async_channel::unbounded::<(TabId, String)>();

        let initial_tab_id = model.active_tab().id();
        let native_webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
            initial_tab_id,
            webview_url_for(model.current_url()),
            title_tx.clone(),
            true,
        )));

        let mut tab_ui_states = HashMap::new();
        tab_ui_states.insert(initial_tab_id, TabUiState::default());

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
            pane_configuration,
            focus_handle: None,
            webviews: vec![native_webview],
            title_tx,
            tab_ui_states,
            back_button_mouse_state: MouseStateHandle::default(),
            forward_button_mouse_state: MouseStateHandle::default(),
            reload_button_mouse_state: MouseStateHandle::default(),
            new_tab_button_mouse_state: MouseStateHandle::default(),
            collapse_button_mouse_state: MouseStateHandle::default(),
            open_external_button_mouse_state: MouseStateHandle::default(),
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

    fn active_webview(&self) -> Option<&Rc<RefCell<NativeBrowserWebView>>> {
        self.webviews.get(self.model.active_index())
    }

    fn navigate_to_editor_url(&mut self, ctx: &mut ViewContext<Self>) {
        let raw_text = self.url_editor.as_ref(ctx).buffer_text(ctx);
        let engine = *GeneralSettings::as_ref(ctx).default_search_engine;
        let target = match resolve_with_engine(&raw_text, engine) {
            Resolved::Url(u) | Resolved::Search(u) => u,
        };
        self.navigate(target, ctx);
    }

    /// Navigate the active tab to `url`. Exposed to the workspace so external
    /// callers (e.g. terminal-link clicks) can populate the open browser pane
    /// instead of spawning a system-browser tab.
    pub(crate) fn navigate(&mut self, url: impl Into<String>, ctx: &mut ViewContext<Self>) {
        if let Some(url) = self.model.navigate(url) {
            self.url_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer(&url, ctx);
            });
            if let Some(webview) = self.active_webview() {
                webview.borrow_mut().load_url(&webview_url_for(&url));
            }
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn go_back(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(url) = self.model.go_back() {
            self.url_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer(&url, ctx);
            });
            if let Some(webview) = self.active_webview() {
                webview.borrow().go_back();
            }
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn go_forward(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(url) = self.model.go_forward() {
            self.url_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer(&url, ctx);
            });
            if let Some(webview) = self.active_webview() {
                webview.borrow().go_forward();
            }
            self.sync_pane_title(ctx);
            ctx.notify();
        }
    }

    fn reload(&mut self, ctx: &mut ViewContext<Self>) {
        self.model.reload();
        if let Some(webview) = self.active_webview() {
            webview.borrow().reload();
        }
        self.sync_pane_title(ctx);
        ctx.notify();
    }

    fn new_tab(&mut self, ctx: &mut ViewContext<Self>) {
        // Hide the currently active tab before adding the new one.
        if let Some(prev_active) = self.webviews.get(self.model.active_index()) {
            prev_active.borrow_mut().set_visibility(false);
        }

        let (tab_id, _idx) = self.model.add_tab(DEFAULT_BROWSER_URL);
        let webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
            tab_id,
            webview_url_for(DEFAULT_BROWSER_URL),
            self.title_tx.clone(),
            true,
        )));
        self.webviews.push(webview);
        self.tab_ui_states.insert(tab_id, TabUiState::default());

        self.sync_active_tab_into_editor(ctx);
        self.sync_pane_title(ctx);
        ctx.notify();
    }

    fn close_tab(&mut self, idx: usize, ctx: &mut ViewContext<Self>) {
        let prior_active_idx = self.model.active_index();
        let Some(result) = self.model.close_tab(idx) else {
            return;
        };

        // Drop the removed tab's webview (this detaches the native view).
        if result.removed_index < self.webviews.len() {
            let removed = self.webviews.remove(result.removed_index);
            removed.borrow_mut().set_visibility(false);
            // Removed is dropped here, which destroys the wry::WebView.
            drop(removed);
        }

        // Also clean up its UI state.
        if let Some(removed_tab_id) = self.tab_ui_states_remove_for_index(result.removed_index) {
            let _ = removed_tab_id;
        }

        // If we replaced the last tab with a fresh default tab, create a matching webview.
        if let Some(new_tab_id) = result.new_tab_id {
            let webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
                new_tab_id,
                webview_url_for(DEFAULT_BROWSER_URL),
                self.title_tx.clone(),
                true,
            )));
            self.webviews.push(webview);
            self.tab_ui_states.insert(new_tab_id, TabUiState::default());
        }

        // If the active tab changed, surface the new tab's URL & title; if the
        // previously-active webview is still around (e.g. we closed a non-active
        // tab), leave it as-is.
        if self.model.active_index() != prior_active_idx || result.removed_index == prior_active_idx
        {
            if let Some(webview) = self.active_webview() {
                webview.borrow_mut().set_visibility(true);
            }
            self.sync_active_tab_into_editor(ctx);
            self.sync_pane_title(ctx);
        }

        ctx.notify();
    }

    fn select_tab(&mut self, idx: usize, ctx: &mut ViewContext<Self>) {
        let prior_active_idx = self.model.active_index();
        if !self.model.select_tab(idx) {
            return;
        }

        if let Some(prev) = self.webviews.get(prior_active_idx) {
            prev.borrow_mut().set_visibility(false);
        }
        if let Some(next) = self.active_webview() {
            next.borrow_mut().set_visibility(true);
        }

        self.sync_active_tab_into_editor(ctx);
        self.sync_pane_title(ctx);
        ctx.notify();
    }

    /// Removes a tab UI state entry given its current index in `model.tabs()`
    /// *before* removal happened. The model has already removed the tab; we
    /// can't look it up there, so we identify it by scanning for an entry not
    /// in the remaining tabs.
    fn tab_ui_states_remove_for_index(&mut self, _removed_index: usize) -> Option<TabId> {
        let live: std::collections::HashSet<TabId> =
            self.model.tabs().iter().map(|t| t.id()).collect();
        let mut stale: Option<TabId> = None;
        for &id in self.tab_ui_states.keys() {
            if !live.contains(&id) {
                stale = Some(id);
                break;
            }
        }
        if let Some(id) = stale {
            self.tab_ui_states.remove(&id);
            Some(id)
        } else {
            None
        }
    }

    fn sync_active_tab_into_editor(&mut self, ctx: &mut ViewContext<Self>) {
        let url = self.model.current_url().to_string();
        self.url_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text_with_base_buffer(&url, ctx);
        });
    }

    fn handle_document_title(&mut self, msg: (TabId, String), ctx: &mut ViewContext<Self>) {
        let (tab_id, title) = msg;
        if self.model.set_title_for(tab_id, title) {
            // Only resync the pane title if the active tab's title changed.
            if self.model.active_tab().id() == tab_id {
                self.sync_pane_title(ctx);
            }
            ctx.notify();
        }
    }

    fn sync_pane_title(&self, ctx: &mut ViewContext<Self>) {
        self.pane_configuration.update(ctx, |configuration, ctx| {
            configuration.set_title(self.model.display_title(), ctx);
            configuration.set_title_secondary(self.model.current_url(), ctx);
        });
    }

    #[cfg(not(target_family = "wasm"))]
    fn persist_open_state(&self, open: bool) {
        let state = self.model.snapshot(open);
        if let Err(err) = persistence::save_to_default_dir(&state) {
            log::warn!("failed to persist browser state: {err}");
        }
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
            Icon::LeftSidebarClose,
            "Toggle browser pane (⌘⌥B)",
            self.collapse_button_mouse_state.clone(),
            false,
            false,
            BrowserViewAction::Collapse,
            app,
        ));
        toolbar.add_child(
            Container::new(self.render_toolbar_button(
                Icon::ArrowLeft,
                "Back",
                self.back_button_mouse_state.clone(),
                false,
                !self.model.can_go_back(),
                BrowserViewAction::Back,
                app,
            ))
            .with_margin_left(TOOLBAR_BUTTON_GAP)
            .finish(),
        );
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
                .with_min_width(URL_BAR_MIN_WIDTH)
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

        toolbar.add_child(
            Container::new(self.render_toolbar_button(
                Icon::LinkExternal,
                "Open in default browser",
                self.open_external_button_mouse_state.clone(),
                false,
                false,
                BrowserViewAction::OpenExternal,
                app,
            ))
            .with_margin_left(TOOLBAR_HORIZONTAL_PADDING)
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

    fn render_tab_strip(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let active = self.model.active_index();

        let mut row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Max);

        for (idx, tab) in self.model.tabs().iter().enumerate() {
            let title = tab.display_title().to_string();
            let tab_id = tab.id();
            let ui_state = self.tab_ui_states.get(&tab_id).cloned().unwrap_or_default();
            let chip = self.render_tab_chip(idx, tab_id, &title, idx == active, ui_state, app);
            let chip_with_margin = if idx == 0 {
                chip
            } else {
                Container::new(chip).with_margin_left(TAB_GAP).finish()
            };
            row.add_child(chip_with_margin);
        }

        row.add_child(
            Container::new(self.render_new_tab_button(app))
                .with_margin_left(TAB_GAP * 2.0)
                .finish(),
        );

        let _ = appearance;

        ConstrainedBox::new(
            Container::new(row.finish())
                .with_horizontal_padding(TOOLBAR_HORIZONTAL_PADDING)
                .with_background(theme.surface_1())
                .finish(),
        )
        .with_height(TAB_STRIP_HEIGHT)
        .finish()
    }

    fn render_tab_chip(
        &self,
        idx: usize,
        _tab_id: TabId,
        title: &str,
        is_active: bool,
        ui_state: TabUiState,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let active_bg = theme.background();
        let active_text = theme.main_text_color(theme.background());
        let inactive_text = theme.sub_text_color(theme.background());
        let hover_bg = theme.surface_2();
        let chip_text_color = if is_active {
            active_text
        } else {
            inactive_text
        };
        let title_text = title.to_string();
        let font_family = appearance.ui_font_family();
        let close_mouse = ui_state.close_mouse.clone();

        let close_button_color = chip_text_color;
        let close_button = Hoverable::new(close_mouse, move |hover_state| {
            let icon_color = if hover_state.is_hovered() {
                active_text
            } else {
                close_button_color
            };
            let icon = ConstrainedBox::new(Icon::X.to_warpui_icon(icon_color).finish())
                .with_width(TAB_CLOSE_BUTTON_SIZE)
                .with_height(TAB_CLOSE_BUTTON_SIZE)
                .finish();
            let mut container = Container::new(icon)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(3.0)));
            if hover_state.is_hovered() {
                container = container.with_background(hover_bg);
            }
            container.finish()
        })
        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(BrowserViewAction::CloseTab(idx)))
        .finish();

        let title_element = ConstrainedBox::new(
            Text::new_inline(title_text, font_family, 12.0)
                .with_color(chip_text_color.into())
                .with_clip(ClipConfig::end())
                .finish(),
        )
        .with_max_width(TAB_MAX_WIDTH - TAB_CLOSE_BUTTON_SIZE - TAB_CHIP_PADDING * 2.0 - 4.0)
        .finish();

        let row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Min)
            .with_child(Expanded::new(1.0, title_element).finish())
            .with_child(Container::new(close_button).with_margin_left(4.0).finish())
            .finish();

        let chip_mouse = ui_state.chip_mouse.clone();
        let accent = theme.accent();
        let chip = Hoverable::new(chip_mouse, move |hover_state| {
            let background = if is_active {
                Some(active_bg)
            } else if hover_state.is_hovered() {
                Some(hover_bg)
            } else {
                None
            };

            let mut container = Container::new(row)
                .with_horizontal_padding(TAB_CHIP_PADDING)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(TAB_BORDER_RADIUS)));
            if let Some(bg) = background {
                container = container.with_background(bg);
            }
            if is_active {
                container = container.with_border(Border::all(1.0).with_border_fill(accent));
            }
            container.finish()
        })
        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(BrowserViewAction::SelectTab(idx)))
        .finish();

        ConstrainedBox::new(Align::new(chip).finish())
            .with_min_width(TAB_MIN_WIDTH)
            .with_max_width(TAB_MAX_WIDTH)
            .with_height(TAB_HEIGHT)
            .finish()
    }

    fn render_new_tab_button(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder().clone();
        let color = blended_colors::text_main(theme, theme.background()).into();

        icon_button_with_color(
            appearance,
            Icon::Plus,
            false,
            self.new_tab_button_mouse_state.clone(),
            color,
        )
        .with_tooltip(move || ui_builder.tool_tip("New Tab".to_string()).build().finish())
        .build()
        .on_click(|ctx, _, _| ctx.dispatch_typed_action(BrowserViewAction::NewTab))
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

        // Layout invariant: the tab strip (TAB_STRIP_HEIGHT) + toolbar
        // (TOOLBAR_HEIGHT) occupy the top of the pane, and the wry webview
        // sits strictly below them via Flex::column. Because the native
        // overlay never intersects toolbar bounds, GPUI mouse hit-testing
        // routes clicks on toolbar icons to the GPUI buttons rather than the
        // webview. If you change this layout, preserve that invariant or the
        // native overlay will swallow toolbar clicks and tooltips.
        // Only the active tab's webview is rendered into the layout tree.
        // Inactive tabs keep their native views hidden via set_visibility(false).
        let webview_element: Box<dyn Element> = match self.active_webview() {
            Some(webview) => {
                Container::new(NativeWebViewElement::new(webview.clone(), self.window_id).finish())
                    .with_background(theme.background())
                    .finish()
            }
            None => Container::new(Container::new(Flex::row().finish()).finish())
                .with_background(theme.background())
                .finish(),
        };

        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(self.render_tab_strip(app))
            .with_child(self.render_toolbar(app))
            .with_child(Expanded::new(1.0, webview_element).finish())
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
            BrowserViewAction::NewTab => self.new_tab(ctx),
            BrowserViewAction::CloseTab(idx) => self.close_tab(*idx, ctx),
            BrowserViewAction::SelectTab(idx) => self.select_tab(*idx, ctx),
            BrowserViewAction::OpenExternal => {
                let url = self.model.current_url().to_string();
                ctx.open_url(&url);
            }
            BrowserViewAction::Collapse => {
                // The `workspace:toggle_browser_pane` global action is
                // registered in Phase 7; dispatching by string name resolves
                // at runtime, so it's safe to land before the handler exists.
                ctx.dispatch_global_action("workspace:toggle_browser_pane", &());
            }
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
        #[cfg(not(target_family = "wasm"))]
        {
            // Detach every native webview before the pane group shadow-closes
            // us. `UndoClosedPanes` keeps `BrowserView` alive, so Drop on
            // `NativeBrowserWebView` won't run on its own; without this the
            // WKWebView NSViews remain attached to the parent NSView and
            // paint as a visible artifact over the workspace.
            for webview in &self.webviews {
                webview.borrow_mut().detach_native();
            }
            self.persist_open_state(false);
        }
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
            left_of_title: Some(
                ConstrainedBox::new(Icon::Globe.to_warpui_icon(theme.foreground()).finish())
                    .with_width(16.)
                    .with_height(16.)
                    .finish(),
            ),
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
