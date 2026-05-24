// Dialog suppression shim for the CastCodes embedded browser pane.
//
// wry 0.38 does not expose a JS-dialog handler API on macOS (the WKWebView
// `runJavaScript*Panel` UIDelegate methods are not bridged). Without
// intervention, pages that call `window.alert`, `window.confirm`, or
// `window.prompt` either render nothing (WKWebView default with no
// UIDelegate handler) or in some configurations hang waiting for a
// response that never comes.
//
// This shim is injected at page-creation time via
// `WebViewBuilder::with_initialization_script` so it runs before any
// page script. It replaces the three native functions with safe-default
// returns:
//
//   window.alert(msg)   → no-op + console.warn; returns undefined
//   window.confirm(msg) → no-op + console.warn; returns false (deny)
//   window.prompt(msg)  → no-op + console.warn; returns null (cancel)
//
// The deny/cancel returns mirror what users would pick if they reflexively
// dismissed the dialog. Pages get a deterministic answer and don't hang.
//
// A follow-up can wire host-side IPC to surface a non-modal banner so the
// user sees what was suppressed; today the suppression is silent except
// for the console.warn.

(function () {
    "use strict";

    if (window.__castcodes_dialogs_installed__) {
        return;
    }
    window.__castcodes_dialogs_installed__ = true;

    function safeString(value) {
        if (value === undefined || value === null) return "";
        try {
            return String(value);
        } catch (_e) {
            return "[unstringifiable]";
        }
    }

    window.alert = function (message) {
        try {
            console.warn(
                "[castcodes] window.alert suppressed:",
                safeString(message)
            );
        } catch (_e) { /* noop */ }
    };

    window.confirm = function (message) {
        try {
            console.warn(
                "[castcodes] window.confirm suppressed (returning false):",
                safeString(message)
            );
        } catch (_e) { /* noop */ }
        return false;
    };

    window.prompt = function (message, defaultValue) {
        try {
            console.warn(
                "[castcodes] window.prompt suppressed (returning null):",
                safeString(message),
                "default=" + safeString(defaultValue)
            );
        } catch (_e) { /* noop */ }
        return null;
    };
})();
