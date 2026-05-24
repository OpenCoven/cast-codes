//! Page-init shim that suppresses `window.alert/confirm/prompt`.
//!
//! Background: wry 0.38 doesn't bridge WKWebView's `runJavaScript*Panel`
//! UIDelegate methods on macOS, and there's no public `with_*_dialog_handler`
//! on the builder. Without intervention, pages calling these functions can
//! hang or silently fail in unpredictable ways depending on the host
//! WebKit version. The injected shim replaces them with safe-default
//! returns so pages get a deterministic answer (alert→undefined,
//! confirm→false, prompt→null) and the page event loop never stalls.
//!
//! Limitations:
//!   - No host visibility today. A follow-up can route the suppressed
//!     dialog messages through the existing IPC channel (introduced in
//!     #128 for find-in-page) and surface them as a non-modal banner.
//!   - Sites that genuinely require user confirmation (e.g. "are you
//!     sure?" before deletion) fall back to their no-confirmation path.
//!     Pragmatic deny-by-default mirrors how a user would reflexively
//!     dismiss an unwanted dialog.

/// JS shim loaded via `WebViewBuilder::with_initialization_script` so it
/// runs once per page-creation, before any page-author script.
pub(crate) const INIT_SCRIPT: &str = include_str!("../../assets/bundled/js/dialogs.js");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_script_is_loaded() {
        // Sanity check the bundled asset wired in.
        assert!(!INIT_SCRIPT.is_empty());
        assert!(INIT_SCRIPT.contains("__castcodes_dialogs_installed__"));
    }

    #[test]
    fn init_script_overrides_all_three_dialog_functions() {
        for fn_name in ["window.alert", "window.confirm", "window.prompt"] {
            assert!(
                INIT_SCRIPT.contains(fn_name),
                "init script doesn't override {fn_name}"
            );
        }
    }

    #[test]
    fn init_script_returns_safe_defaults() {
        // confirm should return false and prompt should return null —
        // the de-facto "user cancelled" answers. The grep checks the
        // literal return statements survive any future refactor.
        assert!(
            INIT_SCRIPT.contains("return false"),
            "confirm doesn't deny by default"
        );
        assert!(
            INIT_SCRIPT.contains("return null"),
            "prompt doesn't cancel by default"
        );
    }
}
