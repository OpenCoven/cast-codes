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
    menu::{MenuItem, MenuItemFields},
    pane_group::{
        focus_state::PaneFocusHandle,
        pane::view::{self, HeaderContent, StandardHeader, StandardHeaderOptions},
        BackingView, PaneConfiguration, PaneEvent,
    },
    ui_components::{
        blended_colors,
        buttons::{icon_button_with_color, small_icon_button_with_color},
        icons::Icon,
    },
};

use super::about_home;
use super::browser_model::{BrowserModel, TabId, DEFAULT_BROWSER_URL};
#[cfg(not(target_family = "wasm"))]
use super::data_dir;
use super::find::FindState;
use super::persistence;
use super::url_input::{resolve_with_engine, Resolved};
#[cfg(not(target_family = "wasm"))]
use super::webview_host::SharedWebContext;
use super::webview_host::{NativeBrowserWebView, NativeWebViewEvent};
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

/// Classification of the active URL for the SSL/security indicator in the
/// URL bar. Loopback http URLs are intentionally treated as `Secure` since
/// they're served from the user's own machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecurityState {
    /// HTTPS or loopback/localhost HTTP — no warning needed.
    Secure,
    /// HTTP to a non-loopback host — show a warning.
    Insecure,
    /// Schemes for which a security indicator is irrelevant (no icon).
    Neutral,
}

fn classify_security(url: &str) -> SecurityState {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return SecurityState::Secure;
    }
    if lower.starts_with("http://") {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                let host = host
                    .strip_prefix('[')
                    .and_then(|rest| rest.strip_suffix(']'))
                    .unwrap_or(host);
                if host.eq_ignore_ascii_case("localhost")
                    || matches!(host, "127.0.0.1" | "::1" | "0.0.0.0")
                {
                    return SecurityState::Secure;
                }
            }
        }
        return SecurityState::Insecure;
    }
    SecurityState::Neutral
}

const URL_BAR_HEIGHT: f32 = 32.0;
const URL_BAR_MIN_WIDTH: f32 = 160.0;
// Toolbar = URL bar height + 4pt total vertical padding (2pt each side). The
// previous 48pt left ~16pt of dead space around a 32pt input and made the
// browser chrome look bulky relative to neighboring panes.
const TOOLBAR_HEIGHT: f32 = 36.0;
// Matches the global `--tabbar-height` design token in
// `resources/design-tokens.css` (2.125rem = 34px). Bumped from 32pt
// after audit finding A2 — keeping the browser tab strip in lock-step
// with the workspace tab bar makes nested panes visually consistent.
const TAB_STRIP_HEIGHT: f32 = 34.0;
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
/// Height of the page-load progress strip below the toolbar. Renders
/// accent-colored when the active tab is loading; transparent otherwise.
const LOADING_STRIP_HEIGHT: f32 = 2.0;
/// Size of the SSL/security indicator rendered inside the URL bar.
const SECURITY_ICON_SIZE: f32 = 14.0;

/// Zoom levels, mirroring Chrome's stepping. Indices are stored per tab
/// in `BrowserView::tab_zoom_steps`; `ZOOM_STEPS[level]` gives the
/// multiplier passed to `wry::WebView::zoom`.
const ZOOM_STEPS: &[f32] = &[
    0.50, 0.67, 0.75, 0.80, 0.90, 1.00, 1.10, 1.25, 1.50, 1.75, 2.00,
];
/// Index of the 1.00 (100%) step. New tabs start here.
const DEFAULT_ZOOM_STEP: u8 = 5;

fn zoom_level_for_step(step: u8) -> f32 {
    let idx = (step as usize).min(ZOOM_STEPS.len() - 1);
    ZOOM_STEPS[idx]
}

fn zoom_step_in(current: u8) -> u8 {
    let max = (ZOOM_STEPS.len() - 1) as u8;
    current.saturating_add(1).min(max)
}

fn zoom_step_out(current: u8) -> u8 {
    current.saturating_sub(1)
}

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
    /// Open the find-in-page overlay, or close it if already open.
    ToggleFind,
    /// Advance to the next find-in-page match.
    FindNext,
    /// Step back to the previous find-in-page match.
    FindPrev,
    /// Close the find overlay and clear all highlights in the page.
    CloseFind,
    /// Step the active tab's zoom up (toward larger).
    ZoomIn,
    /// Step the active tab's zoom down (toward smaller).
    ZoomOut,
    /// Reset the active tab's zoom to 100%.
    ZoomReset,
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
    /// Channel for tab-tagged events from all webviews (titles, page-load
    /// lifecycle, popup requests, navigation redirects).
    event_tx: async_channel::Sender<NativeWebViewEvent>,
    /// Shared WebKit data store for every tab in this pane. Lifetime spans
    /// the pane (and outlives every wry::WebView built from it — required
    /// per wry's docs). `None` on wasm.
    #[cfg(not(target_family = "wasm"))]
    web_context: Option<SharedWebContext>,
    /// Per-tab UI mouse states keyed by stable [`TabId`] so they survive tab
    /// closures (which shift indices).
    tab_ui_states: HashMap<TabId, TabUiState>,
    workspace_tab_visible: bool,
    back_button_mouse_state: MouseStateHandle,
    forward_button_mouse_state: MouseStateHandle,
    reload_button_mouse_state: MouseStateHandle,
    new_tab_button_mouse_state: MouseStateHandle,
    collapse_button_mouse_state: MouseStateHandle,
    open_external_button_mouse_state: MouseStateHandle,
    find_toggle_button_mouse_state: MouseStateHandle,
    find_next_button_mouse_state: MouseStateHandle,
    find_prev_button_mouse_state: MouseStateHandle,
    find_close_button_mouse_state: MouseStateHandle,
    /// Editor used by the find overlay. Kept around even when the overlay
    /// is hidden so the input view's focus state survives toggle cycles.
    find_editor: ViewHandle<EditorView>,
    /// `Some` while the find overlay is visible.
    find_state: Option<FindState>,
    /// Per-tab zoom step index (transient — not persisted). Stable
    /// across renders; cleared when a tab closes.
    tab_zoom_steps: HashMap<TabId, u8>,
}

impl BrowserView {
    /// Read-only access to the underlying model. Used by the workspace to
    /// snapshot tab state for persistence.
    pub(crate) fn model(&self) -> &BrowserModel {
        &self.model
    }
}

impl BrowserView {
    pub fn new(
        initial_url: Option<String>,
        #[cfg(not(target_family = "wasm"))] session_id: &str,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let model = BrowserModel::new(initial_url.unwrap_or_default());
        let pane_configuration =
            ctx.add_model(|_ctx| PaneConfiguration::new(model.display_title()));
        let (event_tx, event_rx) = async_channel::unbounded::<NativeWebViewEvent>();

        #[cfg(not(target_family = "wasm"))]
        let web_context: Option<SharedWebContext> = {
            // Per-session data dir isolates cookies/localStorage/IndexedDB
            // per workspace tab on Linux + Windows. macOS shares the
            // WKWebsiteDataStore default store regardless (wry 0.38
            // limitation, see `data_dir`), but creating the directory keeps
            // the layout consistent for future macOS plumbing.
            let dir = data_dir::browser_data_dir(session_id);
            // Construct the WebContext even when dir is None — wry handles
            // the missing-dir case internally with its platform default.
            Some(Rc::new(RefCell::new(wry::WebContext::new(dir))))
        };

        let initial_tab_id = model.active_tab().id();
        let native_webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
            initial_tab_id,
            webview_url_for(model.current_url()),
            event_tx.clone(),
            #[cfg(not(target_family = "wasm"))]
            web_context.clone(),
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
        ctx.spawn_stream_local(event_rx, Self::handle_webview_event, |_, _| {});

        let find_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(12.0), appearance),
                    select_all_on_focus: true,
                    clear_selections_on_blur: false,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    ..Default::default()
                },
                ctx,
            );
            editor.set_placeholder_text("Find in page", ctx);
            editor
        });

        ctx.subscribe_to_view(&find_editor, move |view, _, event, ctx| {
            match event {
                EditorEvent::Edited(_) => view.handle_find_query_changed(ctx),
                EditorEvent::Enter => view.handle_action(&BrowserViewAction::FindNext, ctx),
                EditorEvent::Escape => view.handle_action(&BrowserViewAction::CloseFind, ctx),
                _ => {}
            }
        });

        Self {
            model,
            window_id: ctx.window_id(),
            url_editor,
            pane_configuration,
            focus_handle: None,
            webviews: vec![native_webview],
            event_tx,
            #[cfg(not(target_family = "wasm"))]
            web_context,
            tab_ui_states,
            workspace_tab_visible: true,
            back_button_mouse_state: MouseStateHandle::default(),
            forward_button_mouse_state: MouseStateHandle::default(),
            reload_button_mouse_state: MouseStateHandle::default(),
            new_tab_button_mouse_state: MouseStateHandle::default(),
            collapse_button_mouse_state: MouseStateHandle::default(),
            open_external_button_mouse_state: MouseStateHandle::default(),
            find_toggle_button_mouse_state: MouseStateHandle::default(),
            find_next_button_mouse_state: MouseStateHandle::default(),
            find_prev_button_mouse_state: MouseStateHandle::default(),
            find_close_button_mouse_state: MouseStateHandle::default(),
            find_editor,
            find_state: None,
            tab_zoom_steps: {
                let mut map = HashMap::new();
                map.insert(initial_tab_id, DEFAULT_ZOOM_STEP);
                map
            },
        }
    }

    /// Construct a BrowserView from previously-persisted tab state.
    #[cfg(not(target_family = "wasm"))]
    pub fn from_state(
        state: super::browser_model::BrowserState,
        session_id: &str,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let model = BrowserModel::restore(state);
        let pane_configuration =
            ctx.add_model(|_ctx| PaneConfiguration::new(model.display_title()));
        let (event_tx, event_rx) = async_channel::unbounded::<NativeWebViewEvent>();

        let web_context: Option<SharedWebContext> = {
            // Per-session data dir; see `Self::new` for the platform notes.
            let dir = data_dir::browser_data_dir(session_id);
            Some(Rc::new(RefCell::new(wry::WebContext::new(dir))))
        };

        let active_idx = model.active_index();
        let mut webviews = Vec::with_capacity(model.tabs().len());
        let mut tab_ui_states = HashMap::new();
        let mut tab_zoom_steps = HashMap::new();
        for (idx, tab) in model.tabs().iter().enumerate() {
            let tab_id = tab.id();
            webviews.push(Rc::new(RefCell::new(NativeBrowserWebView::new(
                tab_id,
                webview_url_for(tab.current_url()),
                event_tx.clone(),
                web_context.clone(),
                idx == active_idx,
            ))));
            tab_ui_states.insert(tab_id, TabUiState::default());
            tab_zoom_steps.insert(tab_id, DEFAULT_ZOOM_STEP);
        }

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
        ctx.spawn_stream_local(event_rx, Self::handle_webview_event, |_, _| {});

        let find_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(12.0), appearance),
                    select_all_on_focus: true,
                    clear_selections_on_blur: false,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    ..Default::default()
                },
                ctx,
            );
            editor.set_placeholder_text("Find in page", ctx);
            editor
        });

        ctx.subscribe_to_view(&find_editor, move |view, _, event, ctx| {
            match event {
                EditorEvent::Edited(_) => view.handle_find_query_changed(ctx),
                EditorEvent::Enter => view.handle_action(&BrowserViewAction::FindNext, ctx),
                EditorEvent::Escape => view.handle_action(&BrowserViewAction::CloseFind, ctx),
                _ => {}
            }
        });

        Self {
            model,
            window_id: ctx.window_id(),
            url_editor,
            pane_configuration,
            focus_handle: None,
            webviews,
            event_tx,
            web_context,
            tab_ui_states,
            workspace_tab_visible: true,
            back_button_mouse_state: MouseStateHandle::default(),
            forward_button_mouse_state: MouseStateHandle::default(),
            reload_button_mouse_state: MouseStateHandle::default(),
            new_tab_button_mouse_state: MouseStateHandle::default(),
            collapse_button_mouse_state: MouseStateHandle::default(),
            open_external_button_mouse_state: MouseStateHandle::default(),
            find_toggle_button_mouse_state: MouseStateHandle::default(),
            find_next_button_mouse_state: MouseStateHandle::default(),
            find_prev_button_mouse_state: MouseStateHandle::default(),
            find_close_button_mouse_state: MouseStateHandle::default(),
            find_editor,
            find_state: None,
            tab_zoom_steps,
        }
    }

    /// Apply the active tab's zoom step to its native webview.
    /// Idempotent — safe to call after every transition that might leave
    /// the visible webview at a different zoom than the model says.
    fn apply_active_tab_zoom(&self) {
        #[cfg(not(target_family = "wasm"))]
        {
            let tab_id = self.model.active_tab().id();
            let step = *self
                .tab_zoom_steps
                .get(&tab_id)
                .unwrap_or(&DEFAULT_ZOOM_STEP);
            if let Some(webview) = self.active_webview() {
                webview.borrow().set_zoom(zoom_level_for_step(step));
            }
        }
    }

    fn zoom_in(&mut self, ctx: &mut ViewContext<Self>) {
        let tab_id = self.model.active_tab().id();
        let cur = *self.tab_zoom_steps.get(&tab_id).unwrap_or(&DEFAULT_ZOOM_STEP);
        let next = zoom_step_in(cur);
        if next == cur {
            return;
        }
        self.tab_zoom_steps.insert(tab_id, next);
        self.apply_active_tab_zoom();
        ctx.notify();
    }

    fn zoom_out(&mut self, ctx: &mut ViewContext<Self>) {
        let tab_id = self.model.active_tab().id();
        let cur = *self.tab_zoom_steps.get(&tab_id).unwrap_or(&DEFAULT_ZOOM_STEP);
        let next = zoom_step_out(cur);
        if next == cur {
            return;
        }
        self.tab_zoom_steps.insert(tab_id, next);
        self.apply_active_tab_zoom();
        ctx.notify();
    }

    fn zoom_reset(&mut self, ctx: &mut ViewContext<Self>) {
        let tab_id = self.model.active_tab().id();
        if self.tab_zoom_steps.get(&tab_id) == Some(&DEFAULT_ZOOM_STEP) {
            return;
        }
        self.tab_zoom_steps.insert(tab_id, DEFAULT_ZOOM_STEP);
        self.apply_active_tab_zoom();
        ctx.notify();
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

    fn sync_webview_visibility(&mut self) {
        let active_idx = self.model.active_index();
        for (idx, webview) in self.webviews.iter().enumerate() {
            webview
                .borrow_mut()
                .set_visibility(self.workspace_tab_visible && idx == active_idx);
        }
    }

    fn active_webview(&self) -> Option<&Rc<RefCell<NativeBrowserWebView>>> {
        self.webviews.get(self.model.active_index())
    }

    /// Hide or show the native webview for the currently-active intra-pane
    /// browser tab. Inactive intra-pane tabs are already kept hidden by
    /// [`Self::select_tab`], so only the active webview's NSView needs to
    /// flip when the owning workspace tab changes focus.
    ///
    /// Without this, switching workspace tabs leaves the WKWebView NSView
    /// attached to the parent NSView and painting over whichever tab is
    /// now active. Mirrors the `detach_native` pattern used by `close()`
    /// for the same root cause, but reversibly — the webview stays alive
    /// so navigation/load state survives the round trip.
    pub(crate) fn set_workspace_tab_visible(&mut self, visible: bool) {
        if self.workspace_tab_visible == visible {
            return;
        }

        self.workspace_tab_visible = visible;
        self.sync_webview_visibility();
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
        self.open_tab(DEFAULT_BROWSER_URL.to_string(), ctx);
    }

    /// Open a new in-pane tab loading `url`, making it active. Shared by
    /// the `NewTab` action and the popup handler (Action 5).
    fn open_tab(&mut self, url: String, ctx: &mut ViewContext<Self>) {
        // Hide the currently active tab before adding the new one.
        if let Some(prev_active) = self.webviews.get(self.model.active_index()) {
            prev_active.borrow_mut().set_visibility(false);
        }

        let (tab_id, _idx) = self.model.add_tab(&url);
        let webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
            tab_id,
            webview_url_for(self.model.current_url()),
            self.event_tx.clone(),
            #[cfg(not(target_family = "wasm"))]
            self.web_context.clone(),
            self.workspace_tab_visible,
        )));
        self.webviews.push(webview);
        self.tab_ui_states.insert(tab_id, TabUiState::default());
        self.tab_zoom_steps.insert(tab_id, DEFAULT_ZOOM_STEP);

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
            // Drop the zoom state too — keep the map aligned with the live
            // tab set so memory doesn't leak across tab churn.
            self.tab_zoom_steps.remove(&removed_tab_id);
        }

        // If we replaced the last tab with a fresh default tab, create a matching webview.
        if let Some(new_tab_id) = result.new_tab_id {
            let webview = Rc::new(RefCell::new(NativeBrowserWebView::new(
                new_tab_id,
                webview_url_for(DEFAULT_BROWSER_URL),
                self.event_tx.clone(),
                #[cfg(not(target_family = "wasm"))]
                self.web_context.clone(),
                self.workspace_tab_visible,
            )));
            self.webviews.push(webview);
            self.tab_ui_states.insert(new_tab_id, TabUiState::default());
            self.tab_zoom_steps.insert(new_tab_id, DEFAULT_ZOOM_STEP);
        }

        // If the active tab changed, surface the new tab's URL & title; if the
        // previously-active webview is still around (e.g. we closed a non-active
        // tab), leave it as-is.
        if self.model.active_index() != prior_active_idx || result.removed_index == prior_active_idx
        {
            if let Some(webview) = self.active_webview() {
                webview
                    .borrow_mut()
                    .set_visibility(self.workspace_tab_visible);
            }
            self.sync_active_tab_into_editor(ctx);
            self.sync_pane_title(ctx);
            self.apply_active_tab_zoom();
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
            next.borrow_mut().set_visibility(self.workspace_tab_visible);
        }

        self.sync_active_tab_into_editor(ctx);
        self.sync_pane_title(ctx);
        // Re-apply the new active tab's stored zoom so the webview matches
        // our model state. wry's WebView keeps its zoom across visibility
        // toggles, but this also covers the lazy-attach case where the
        // webview was rebuilt at 100% after a pane close+restore cycle.
        self.apply_active_tab_zoom();
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

    fn handle_webview_event(&mut self, event: NativeWebViewEvent, ctx: &mut ViewContext<Self>) {
        match event {
            NativeWebViewEvent::TitleChanged(tab_id, title) => {
                if self.model.set_title_for(tab_id, title) {
                    if self.model.active_tab().id() == tab_id {
                        self.sync_pane_title(ctx);
                    }
                    ctx.notify();
                }
            }
            NativeWebViewEvent::LoadingChanged(tab_id, loading) => {
                if self.model.set_loading_for(tab_id, loading) {
                    ctx.notify();
                }
            }
            NativeWebViewEvent::NavigationStarted(tab_id, url) => {
                // Catches HTTP redirects (the user-typed URL was already set
                // optimistically by `navigate`; same URL → no-op). In-page
                // `history.pushState` doesn't reach this handler.
                if self.model.replace_current_url_for(tab_id, url) {
                    if self.model.active_tab().id() == tab_id {
                        self.sync_active_tab_into_editor(ctx);
                        self.sync_pane_title(ctx);
                    }
                    ctx.notify();
                }
            }
            NativeWebViewEvent::PopupOpenTab(url) => {
                self.open_tab(url, ctx);
            }
            NativeWebViewEvent::PopupOpenExternal(url) => {
                ctx.open_url(&url);
            }
            NativeWebViewEvent::FindResults(tab_id, current, total) => {
                if self.model.active_tab().id() != tab_id {
                    // Find results from a now-background tab — ignore.
                    return;
                }
                if let Some(state) = self.find_state.as_mut() {
                    state.current = current;
                    state.total = total;
                    ctx.notify();
                }
            }
        }
    }

    fn handle_find_query_changed(&mut self, ctx: &mut ViewContext<Self>) {
        let query = self.find_editor.as_ref(ctx).buffer_text(ctx);
        if let Some(state) = self.find_state.as_mut() {
            state.query = query.clone();
        }
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = self.active_webview() {
            webview.borrow().find_set_query(&query);
        }
        ctx.notify();
    }

    fn toggle_find(&mut self, ctx: &mut ViewContext<Self>) {
        if self.find_state.is_some() {
            self.close_find(ctx);
        } else {
            self.find_state = Some(FindState::default());
            self.find_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text_with_base_buffer("", ctx);
            });
            ctx.focus(&self.find_editor);
            ctx.notify();
        }
    }

    fn close_find(&mut self, ctx: &mut ViewContext<Self>) {
        self.find_state = None;
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = self.active_webview() {
            webview.borrow().find_clear();
        }
        ctx.focus(&self.url_editor);
        ctx.notify();
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

        let security_indicator = match classify_security(self.model.current_url()) {
            SecurityState::Secure => Some(
                ConstrainedBox::new(
                    Icon::LockClosed
                        .to_warpui_icon(blended_colors::text_main(theme, theme.surface_1()).into())
                        .finish(),
                )
                .with_width(SECURITY_ICON_SIZE)
                .with_height(SECURITY_ICON_SIZE)
                .finish(),
            ),
            SecurityState::Insecure => Some(
                ConstrainedBox::new(Icon::AlertTriangle.to_warpui_icon(theme.accent()).finish())
                    .with_width(SECURITY_ICON_SIZE)
                    .with_height(SECURITY_ICON_SIZE)
                    .finish(),
            ),
            SecurityState::Neutral => None,
        };

        let mut url_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Max);
        if let Some(indicator) = security_indicator {
            url_row.add_child(Container::new(indicator).with_margin_right(6.0).finish());
        }
        url_row.add_child(
            Expanded::new(
                1.0,
                Clipped::new(ChildView::new(&self.url_editor).finish()).finish(),
            )
            .finish(),
        );

        let editor = Container::new(
            ConstrainedBox::new(url_row.finish())
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
                Icon::Search,
                "Find in page",
                self.find_toggle_button_mouse_state.clone(),
                self.find_state.is_some(),
                false,
                BrowserViewAction::ToggleFind,
                app,
            ))
            .with_margin_left(TOOLBAR_HORIZONTAL_PADDING)
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
            .with_margin_left(TOOLBAR_BUTTON_GAP)
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

    fn render_loading_strip(&self, app: &AppContext) -> Box<dyn Element> {
        let theme = Appearance::as_ref(app).theme();
        // Always reserve the height so the webview area doesn't reflow when
        // loading toggles. Background switches to accent while loading.
        let mut container = Container::new(Flex::row().finish());
        if self.model.is_loading() {
            container = container.with_background(theme.accent());
        }
        ConstrainedBox::new(container.finish())
            .with_height(LOADING_STRIP_HEIGHT)
            .finish()
    }

    fn render_find_overlay(&self, app: &AppContext) -> Option<Box<dyn Element>> {
        let state = self.find_state.as_ref()?;
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let font_family = appearance.ui_font_family();

        let input = Container::new(
            ConstrainedBox::new(
                Clipped::new(ChildView::new(&self.find_editor).finish()).finish(),
            )
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

        let count_label = Text::new_inline(state.count_label(), font_family, 12.0)
            .with_color(blended_colors::text_main(theme, theme.background()))
            .finish();

        let mut row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Max);
        row.add_child(Expanded::new(1.0, input).finish());
        row.add_child(
            Container::new(count_label)
                .with_margin_left(TOOLBAR_HORIZONTAL_PADDING)
                .finish(),
        );
        row.add_child(
            Container::new(self.render_toolbar_button(
                Icon::ArrowUp,
                "Previous match",
                self.find_prev_button_mouse_state.clone(),
                false,
                state.total == 0,
                BrowserViewAction::FindPrev,
                app,
            ))
            .with_margin_left(TOOLBAR_BUTTON_GAP)
            .finish(),
        );
        row.add_child(
            Container::new(self.render_toolbar_button(
                Icon::ArrowDown,
                "Next match",
                self.find_next_button_mouse_state.clone(),
                false,
                state.total == 0,
                BrowserViewAction::FindNext,
                app,
            ))
            .with_margin_left(TOOLBAR_BUTTON_GAP)
            .finish(),
        );
        row.add_child(
            Container::new(self.render_toolbar_button(
                Icon::X,
                "Close find",
                self.find_close_button_mouse_state.clone(),
                false,
                false,
                BrowserViewAction::CloseFind,
                app,
            ))
            .with_margin_left(TOOLBAR_BUTTON_GAP)
            .finish(),
        );

        Some(
            ConstrainedBox::new(
                Container::new(row.finish())
                    .with_horizontal_padding(TOOLBAR_HORIZONTAL_PADDING)
                    .with_background(theme.surface_1())
                    .with_border(Border::top(1.0).with_border_fill(theme.surface_3()))
                    .finish(),
            )
            .with_height(TOOLBAR_HEIGHT)
            .finish(),
        )
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

        // The close-X used to be a hand-rolled Hoverable so the icon could
        // brighten on hover. We trade that micro-effect for an accessible
        // tooltip + standard Button focus/keyboard semantics, which only
        // Button exposes today. The surface_2 hover background remains.
        let ui_builder = appearance.ui_builder().clone();
        let close_button = small_icon_button_with_color(
            appearance,
            Icon::X,
            TAB_CLOSE_BUTTON_SIZE,
            close_mouse,
            chip_text_color.into(),
        )
        .with_tooltip(move || {
            ui_builder
                .tool_tip("Close tab".to_string())
                .build()
                .finish()
        })
        .build()
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
        .with_defer_events_to_children()
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

        let mut column = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(self.render_tab_strip(app))
            .with_child(self.render_toolbar(app));
        if let Some(overlay) = self.render_find_overlay(app) {
            column = column.with_child(overlay);
        }
        column
            .with_child(self.render_loading_strip(app))
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
            BrowserViewAction::ToggleFind => self.toggle_find(ctx),
            BrowserViewAction::CloseFind => self.close_find(ctx),
            BrowserViewAction::FindNext => {
                #[cfg(not(target_family = "wasm"))]
                if let Some(webview) = self.active_webview() {
                    webview.borrow().find_next();
                }
            }
            BrowserViewAction::FindPrev => {
                #[cfg(not(target_family = "wasm"))]
                if let Some(webview) = self.active_webview() {
                    webview.borrow().find_prev();
                }
            }
            BrowserViewAction::ZoomIn => self.zoom_in(ctx),
            BrowserViewAction::ZoomOut => self.zoom_out(ctx),
            BrowserViewAction::ZoomReset => self.zoom_reset(ctx),
        }
    }
}

impl BackingView for BrowserView {
    type PaneHeaderOverflowMenuAction = BrowserViewAction;
    type CustomAction = BrowserViewAction;
    type AssociatedData = ();

    fn pane_header_overflow_menu_items(
        &self,
        _ctx: &AppContext,
    ) -> Vec<MenuItem<BrowserViewAction>> {
        let tab_id = self.model.active_tab().id();
        let cur = *self.tab_zoom_steps.get(&tab_id).unwrap_or(&DEFAULT_ZOOM_STEP);
        let pct = (zoom_level_for_step(cur) * 100.0).round() as i32;
        let reset_label = format!("Reset zoom ({pct}%)");
        let modifier = if cfg!(target_os = "macos") {
            "⌘"
        } else {
            "Ctrl"
        };
        vec![
            MenuItemFields::new_with_label("Zoom in", &format!("{modifier}+"))
                .with_on_select_action(BrowserViewAction::ZoomIn)
                .into_item(),
            MenuItemFields::new_with_label("Zoom out", &format!("{modifier}−"))
                .with_on_select_action(BrowserViewAction::ZoomOut)
                .into_item(),
            MenuItemFields::new_with_label(&reset_label, &format!("{modifier}0"))
                .with_on_select_action(BrowserViewAction::ZoomReset)
                .into_item(),
        ]
    }

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

#[cfg(test)]
mod tests {
    use super::{
        classify_security, zoom_level_for_step, zoom_step_in, zoom_step_out, SecurityState,
        DEFAULT_ZOOM_STEP, TAB_STRIP_HEIGHT, ZOOM_STEPS,
    };

    #[test]
    fn https_is_secure() {
        assert_eq!(
            classify_security("https://example.com"),
            SecurityState::Secure
        );
        assert_eq!(
            classify_security("HTTPS://EXAMPLE.COM/path?q=1"),
            SecurityState::Secure
        );
    }

    #[test]
    fn remote_http_is_insecure() {
        assert_eq!(
            classify_security("http://example.com"),
            SecurityState::Insecure
        );
        assert_eq!(
            classify_security("http://example.com:8080/path"),
            SecurityState::Insecure
        );
    }

    #[test]
    fn loopback_http_is_treated_as_secure() {
        for url in [
            "http://localhost",
            "http://localhost:3000",
            "http://localhost:3000/api",
            "http://127.0.0.1",
            "http://127.0.0.1:8080/path",
            "http://0.0.0.0:9000",
            "http://[::1]",
            "http://[::1]:3000/path",
            "http://user:pass@localhost:3000",
        ] {
            assert_eq!(
                classify_security(url),
                SecurityState::Secure,
                "{url} should be treated as Secure (loopback)"
            );
        }
    }

    #[test]
    fn about_and_file_and_data_are_neutral() {
        for url in [
            "about:home",
            "about:blank",
            "file:///tmp/x.html",
            "data:text/html,<h1>hi</h1>",
            "castcodes://settings",
        ] {
            assert_eq!(
                classify_security(url),
                SecurityState::Neutral,
                "{url} should classify as Neutral"
            );
        }
    }

    #[test]
    fn empty_input_is_neutral() {
        assert_eq!(classify_security(""), SecurityState::Neutral);
    }

    #[test]
    fn tab_strip_height_matches_global_design_token() {
        // Locks the constant to the `--tabbar-height` token in
        // `resources/design-tokens.css` (2.125rem = 34px). If you
        // intentionally change the token, update this test in the same
        // PR so the divergence is reviewed.
        assert_eq!(TAB_STRIP_HEIGHT, 34.0);
    }

    #[test]
    fn default_zoom_step_is_100_percent() {
        assert_eq!(
            (zoom_level_for_step(DEFAULT_ZOOM_STEP) * 100.0).round() as i32,
            100,
            "default step should be the 1.00 index"
        );
    }

    #[test]
    fn zoom_step_in_advances_toward_max() {
        let mut s = DEFAULT_ZOOM_STEP;
        let mut _levels = Vec::new();
        for _ in 0..10 {
            let next = zoom_step_in(s);
            assert!(next >= s);
            s = next;
            _levels.push(zoom_level_for_step(s));
        }
        // Hit the ceiling — final step is the top of ZOOM_STEPS.
        assert_eq!(s, (ZOOM_STEPS.len() - 1) as u8);
        assert_eq!(zoom_level_for_step(s), *ZOOM_STEPS.last().unwrap());
    }

    #[test]
    fn zoom_step_in_saturates_at_max() {
        let max = (ZOOM_STEPS.len() - 1) as u8;
        assert_eq!(zoom_step_in(max), max);
        assert_eq!(zoom_step_in(max + 5), max);
    }

    #[test]
    fn zoom_step_out_retreats_toward_zero() {
        let mut s = DEFAULT_ZOOM_STEP;
        for _ in 0..10 {
            let next = zoom_step_out(s);
            assert!(next <= s);
            s = next;
        }
        assert_eq!(s, 0);
    }

    #[test]
    fn zoom_step_out_saturates_at_zero() {
        assert_eq!(zoom_step_out(0), 0);
    }

    #[test]
    fn zoom_step_round_trip() {
        // In then out should return to original step.
        let s = DEFAULT_ZOOM_STEP;
        assert_eq!(zoom_step_out(zoom_step_in(s)), s);
    }

    #[test]
    fn zoom_level_for_oob_step_clamps() {
        // Index past the end clamps to max instead of panicking — the
        // step is a u8 from user-driven state, defensive guard.
        let last = ZOOM_STEPS.last().copied().unwrap();
        assert_eq!(zoom_level_for_step(255), last);
    }

    #[test]
    fn zoom_steps_are_monotonic() {
        // Each step should be strictly larger than the previous so the
        // UI feels coherent. Catches a typo'd table swap.
        for w in ZOOM_STEPS.windows(2) {
            assert!(w[0] < w[1], "ZOOM_STEPS not monotonic: {:?}", w);
        }
        // And the table contains 1.00 exactly at DEFAULT_ZOOM_STEP.
        assert_eq!(ZOOM_STEPS[DEFAULT_ZOOM_STEP as usize], 1.00);
    }
}
