//! Page-init shim that denies camera / microphone / geolocation /
//! notification / generic Permissions-API requests.
//!
//! ## Why JS instead of a native handler
//!
//! wry 0.38 hardcodes `WKPermissionDecisionGrant` for media-capture on
//! macOS (`wkwebview/mod.rs:798` literally calls
//! `decision_handler.call((1,))`) and exposes no override hook. So
//! WKWebView-level deny isn't reachable from our code without forking
//! wry. The next-best defense is to intercept the JS-level entry points
//! (`navigator.mediaDevices.getUserMedia`, etc.) so page scripts get a
//! `NotAllowedError` before the request reaches WebKit.
//!
//! The upstream fix is a wry change that bridges the permission
//! decision handler, which would let us deny at the WebKit boundary
//! and cover sub-frames + service workers as well. Tracked as a wry
//! issue follow-up.

/// JS shim loaded via `WebViewBuilder::with_initialization_script` so it
/// runs once per page-creation, before any page-author script.
pub(crate) const INIT_SCRIPT: &str = include_str!("../../assets/bundled/js/permissions.js");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_script_is_loaded() {
        assert!(!INIT_SCRIPT.is_empty());
        assert!(INIT_SCRIPT.contains("__castcodes_permissions_installed__"));
    }

    #[test]
    fn init_script_overrides_media_devices() {
        for fn_name in ["getUserMedia", "getDisplayMedia", "enumerateDevices"] {
            assert!(INIT_SCRIPT.contains(fn_name), "missing {fn_name} override");
        }
    }

    #[test]
    fn init_script_overrides_geolocation() {
        assert!(INIT_SCRIPT.contains("getCurrentPosition"));
        assert!(INIT_SCRIPT.contains("watchPosition"));
        assert!(INIT_SCRIPT.contains("PERMISSION_DENIED"));
    }

    #[test]
    fn init_script_overrides_permissions_api() {
        assert!(INIT_SCRIPT.contains("navigator.permissions"));
        assert!(INIT_SCRIPT.contains("\"camera\""));
        assert!(INIT_SCRIPT.contains("\"microphone\""));
        assert!(INIT_SCRIPT.contains("\"notifications\""));
        assert!(INIT_SCRIPT.contains("\"clipboard-read\""));
        // The resolved state must always be "denied" for known names.
        assert!(INIT_SCRIPT.contains("state: \"denied\""));
    }

    #[test]
    fn init_script_overrides_notification_api() {
        assert!(INIT_SCRIPT.contains("Notification.requestPermission"));
        // The returned permission status must be "denied".
        assert!(
            INIT_SCRIPT.contains("Promise.resolve(\"denied\")"),
            "Notification.requestPermission should resolve denied"
        );
    }

    #[test]
    fn init_script_uses_not_allowed_error_name() {
        // Pages branch on err.name === "NotAllowedError" — the
        // standard MediaDevices rejection. Make sure we emit that
        // specific shape rather than a generic Error.
        assert!(INIT_SCRIPT.contains("NotAllowedError"));
        assert!(INIT_SCRIPT.contains("DOMException"));
    }
}
