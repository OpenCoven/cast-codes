#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

use pathfinder_geometry::rect::RectF;
use warpui::{AppContext, WindowId};

use super::browser_model::TabId;

pub(crate) struct NativeBrowserWebView {
    tab_id: TabId,
    #[cfg(not(target_family = "wasm"))]
    webview: Option<wry::WebView>,
    title_tx: async_channel::Sender<(TabId, String)>,
    pending_url: Option<String>,
    bounds: Option<RectF>,
    desired_visible: bool,
    attach_error_logged: bool,
}

impl NativeBrowserWebView {
    pub(crate) fn new(
        tab_id: TabId,
        initial_url: impl Into<String>,
        title_tx: async_channel::Sender<(TabId, String)>,
        desired_visible: bool,
    ) -> Self {
        Self {
            tab_id,
            #[cfg(not(target_family = "wasm"))]
            webview: None,
            title_tx,
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
            let title_tx = self.title_tx.clone();
            let tab_id = self.tab_id;
            match wry::WebViewBuilder::new_as_child(&parent)
                .with_url(url)
                .with_bounds(Self::wry_rect(bounds))
                .with_visible(self.desired_visible)
                .with_accept_first_mouse(true)
                .with_document_title_changed_handler(move |title| {
                    let _ = title_tx.try_send((tab_id, title));
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
