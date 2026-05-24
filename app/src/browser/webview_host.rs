#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

#[cfg(not(target_family = "wasm"))]
use std::{cell::RefCell, rc::Rc};

use pathfinder_geometry::rect::RectF;
use warpui::{AppContext, WindowId};

use super::browser_model::TabId;
#[cfg(not(target_family = "wasm"))]
use super::popup_policy::{self, Decision};

/// Events the native webview layer can push back to `BrowserView`.
///
/// All variants carry the originating `TabId` (when relevant) so the
/// receiver doesn't need a parallel mapping. Popup events don't carry a
/// `TabId` — the host treats them as "open a new tab" and the new tab gets
/// its own id.
#[derive(Debug, Clone)]
pub(crate) enum NativeWebViewEvent {
    /// Document title changed (raw from WKWebView).
    TitleChanged(TabId, String),
    /// A page-load lifecycle event fired. `loading=true` means a load
    /// started; `false` means it finished (success or failure).
    LoadingChanged(TabId, bool),
    /// Top-level navigation began. Used to keep the model URL in sync with
    /// HTTP redirects and direct user nav. Does NOT cover in-page
    /// `history.pushState`, which doesn't fire wry's navigation handler.
    NavigationStarted(TabId, String),
    /// A popup classified as a new in-pane tab. Host should add a tab.
    PopupOpenTab(String),
    /// A popup classified as external. Host should hand off to `ctx.open_url`.
    PopupOpenExternal(String),
}

#[cfg(not(target_family = "wasm"))]
pub(crate) type SharedWebContext = Rc<RefCell<wry::WebContext>>;

pub(crate) struct NativeBrowserWebView {
    tab_id: TabId,
    #[cfg(not(target_family = "wasm"))]
    webview: Option<wry::WebView>,
    event_tx: async_channel::Sender<NativeWebViewEvent>,
    #[cfg(not(target_family = "wasm"))]
    web_context: Option<SharedWebContext>,
    pending_url: Option<String>,
    bounds: Option<RectF>,
    desired_visible: bool,
    attach_error_logged: bool,
}

impl NativeBrowserWebView {
    pub(crate) fn new(
        tab_id: TabId,
        initial_url: impl Into<String>,
        event_tx: async_channel::Sender<NativeWebViewEvent>,
        #[cfg(not(target_family = "wasm"))] web_context: Option<SharedWebContext>,
        desired_visible: bool,
    ) -> Self {
        Self {
            tab_id,
            #[cfg(not(target_family = "wasm"))]
            webview: None,
            event_tx,
            #[cfg(not(target_family = "wasm"))]
            web_context,
            pending_url: Some(initial_url.into()),
            bounds: None,
            desired_visible,
            attach_error_logged: false,
        }
    }

    pub(crate) fn load_url(&mut self, url: &str) {
        self.pending_url = Some(url.to_string());

        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.load_url(url) {
                log::warn!("failed to load browser pane URL {url}: {err}");
            }
        }
    }

    pub(crate) fn go_back(&self) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.evaluate_script("history.back()") {
                log::warn!("failed to navigate browser pane back: {err}");
            }
        }
    }

    pub(crate) fn go_forward(&self) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.evaluate_script("history.forward()") {
                log::warn!("failed to navigate browser pane forward: {err}");
            }
        }
    }

    pub(crate) fn reload(&self) {
        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.evaluate_script("location.reload()") {
                log::warn!("failed to reload browser pane: {err}");
            }
        }
    }

    pub(crate) fn set_visibility(&mut self, visible: bool) {
        self.desired_visible = visible;

        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            if let Err(err) = webview.set_visible(visible) {
                log::warn!("failed to update browser pane visibility: {err}");
            }
        }
    }

    /// Drop the underlying native webview without changing `desired_visible`.
    ///
    /// Why: when the pane is closed, `UndoClosedPanes` keeps `BrowserView`
    /// alive in a shadow state, so `Drop` on `NativeBrowserWebView` never
    /// runs and the WKWebView NSView stays attached to the parent NSView,
    /// painting as a visible artifact over the workspace. Dropping the
    /// `wry::WebView` here triggers wry's own `Drop`, which removes the
    /// native view from its superview immediately. If the pane is later
    /// restored (Cmd+Shift+T), `set_bounds`/`attach_if_needed` will rebuild
    /// the webview from `pending_url`.
    pub(crate) fn detach_native(&mut self) {
        #[cfg(not(target_family = "wasm"))]
        {
            if let Some(webview) = self.webview.take() {
                let _ = webview.set_visible(false);
                drop(webview);
            }
            // Allow a fresh attach if the pane is ever re-painted.
            self.attach_error_logged = false;
        }
    }

    pub(crate) fn set_bounds(&mut self, window_id: WindowId, bounds: RectF, app: &AppContext) {
        self.bounds = Some(bounds);
        self.attach_if_needed(window_id, bounds, app);

        #[cfg(not(target_family = "wasm"))]
        if let Some(webview) = &self.webview {
            let rect = Self::wry_rect(bounds);

            if let Err(err) = webview.set_bounds(rect) {
                log::warn!("failed to resize browser pane webview: {err}");
            }
            if self.desired_visible {
                if let Err(err) = webview.set_visible(true) {
                    log::warn!("failed to show browser pane webview: {err}");
                }
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
            let tab_id = self.tab_id;

            let title_tx = self.event_tx.clone();
            let nav_tx = self.event_tx.clone();
            let load_tx = self.event_tx.clone();
            let popup_tx = self.event_tx.clone();

            let mut builder = wry::WebViewBuilder::new_as_child(&parent)
                .with_url(url)
                .with_bounds(Self::wry_rect(bounds))
                .with_visible(self.desired_visible)
                .with_accept_first_mouse(true)
                .with_document_title_changed_handler(move |title| {
                    let _ = title_tx.try_send(NativeWebViewEvent::TitleChanged(tab_id, title));
                })
                .with_navigation_handler(move |url| {
                    // Track top-level nav so the model can resync URL on
                    // HTTP redirects. Always allow (return true) — we do
                    // not gate navigation here.
                    let _ = nav_tx.try_send(NativeWebViewEvent::NavigationStarted(tab_id, url));
                    true
                })
                .with_on_page_load_handler(move |event, _url| {
                    let loading = matches!(event, wry::PageLoadEvent::Started);
                    let _ = load_tx.try_send(NativeWebViewEvent::LoadingChanged(tab_id, loading));
                })
                .with_new_window_req_handler(move |url| {
                    // Classify popups via our policy and dispatch through the
                    // event channel; always return `false` so wry doesn't
                    // spawn an OS-level window in parallel.
                    match popup_policy::decide(&url) {
                        Decision::Tab(u) => {
                            let _ = popup_tx.try_send(NativeWebViewEvent::PopupOpenTab(u));
                        }
                        Decision::External(u) => {
                            let _ = popup_tx.try_send(NativeWebViewEvent::PopupOpenExternal(u));
                        }
                        Decision::Block => {
                            log::debug!("blocked popup request: {url}");
                        }
                    }
                    false
                });

            // NOTE (wry 0.38 on macOS): `with_web_context` is a no-op here —
            // `wkwebview/mod.rs:95` ignores the parameter. The wiring is
            // kept correct for the current macOS attach path so this starts
            // isolating pane data if wry adds macOS `WKWebsiteDataStore`
            // plumbing. See `data_dir.rs` for the platform reality check.
            let webview_result = if let Some(ctx) = &self.web_context {
                let mut ctx_borrow = ctx.borrow_mut();
                builder = builder.with_web_context(&mut ctx_borrow);
                builder.build()
            } else {
                builder.build()
            };

            match webview_result {
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
