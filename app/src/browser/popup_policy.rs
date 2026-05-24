//! Policy decisions for popups (`window.open` / `target="_blank"`) inside
//! the embedded browser pane.
//!
//! `decide` classifies a popup-request URL into one of:
//!  - `Tab`: open as a new in-pane tab (http/https + same-page schemes).
//!  - `External`: hand off to the system browser via `ctx.open_url()`
//!    (mailto:, tel:, sms:, system schemes).
//!  - `Block`: drop silently. Used for `javascript:` URLs (would otherwise
//!    execute arbitrary script in a fresh context, a classic popup-XSS
//!    vector) and any other scheme we don't want to surface.
//!
//! Keep this module pure (no I/O, no log calls, no GPUI deps). The webview
//! host calls `decide` from wry's `with_new_window_req_handler` and the
//! BrowserView routes the result.

/// Classification of a popup request URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Open the URL as a new in-pane tab.
    Tab(String),
    /// Hand the URL off to the system handler (default browser, mail
    /// client, etc.).
    External(String),
    /// Drop the popup silently.
    Block,
}

pub fn decide(raw_url: &str) -> Decision {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Decision::Block;
    }

    // Defense in depth: never let a popup open a `javascript:` URL. WKWebView
    // would otherwise execute the script in a fresh frame.
    if is_scheme_eq(trimmed, "javascript") {
        return Decision::Block;
    }

    // `data:` popups are blocked in modern browsers (top-level navigation
    // restriction); mirror that here.
    if is_scheme_eq(trimmed, "data") {
        return Decision::Block;
    }

    // Web schemes → new tab.
    if is_scheme_eq(trimmed, "http")
        || is_scheme_eq(trimmed, "https")
        || is_scheme_eq(trimmed, "file")
        || is_scheme_eq(trimmed, "about")
        || is_scheme_eq(trimmed, "castcodes")
    {
        return Decision::Tab(trimmed.to_string());
    }

    // System schemes that should leave the app entirely (mail client, dialer,
    // OS-level handlers).
    for scheme in [
        "mailto",
        "tel",
        "sms",
        "facetime",
        "facetime-audio",
        "imessage",
    ] {
        if is_scheme_eq(trimmed, scheme) {
            return Decision::External(trimmed.to_string());
        }
    }

    // Unknown scheme — hand off to the system in case the user has a custom
    // handler registered, but don't drag it into our pane.
    if has_scheme(trimmed) {
        return Decision::External(trimmed.to_string());
    }

    // Schemeless input from a popup is suspicious; treat as a same-pane tab
    // load (the host will normalize via the model's `normalize_url`).
    Decision::Tab(trimmed.to_string())
}

fn is_scheme_eq(input: &str, scheme: &str) -> bool {
    let Some((head_scheme, _rest)) = input.split_once(':') else {
        return false;
    };

    // Either scheme:// or scheme:foo — both fine as long as the scheme
    // prefix matches exactly, ignoring ASCII case.
    head_scheme.eq_ignore_ascii_case(scheme)
}

fn has_scheme(input: &str) -> bool {
    let Some(colon) = input.find(':') else {
        return false;
    };
    let head = &input[..colon];
    !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        && head
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_and_https_are_tabs() {
        assert_eq!(
            decide("https://example.com"),
            Decision::Tab("https://example.com".to_string())
        );
        assert_eq!(
            decide("http://example.com/page?q=1"),
            Decision::Tab("http://example.com/page?q=1".to_string())
        );
    }

    #[test]
    fn javascript_popup_is_blocked() {
        assert_eq!(decide("javascript:alert(1)"), Decision::Block);
        assert_eq!(decide("JavaScript:fetch('/x')"), Decision::Block);
        assert_eq!(decide("  javascript:void(0)  "), Decision::Block);
    }

    #[test]
    fn data_popup_is_blocked() {
        assert_eq!(decide("data:text/html,<h1>x</h1>"), Decision::Block);
    }

    #[test]
    fn mailto_routes_to_system_handler() {
        assert_eq!(
            decide("mailto:hi@example.com"),
            Decision::External("mailto:hi@example.com".to_string())
        );
    }

    #[test]
    fn tel_and_sms_route_to_system_handler() {
        assert_eq!(
            decide("tel:+15551234"),
            Decision::External("tel:+15551234".to_string())
        );
        assert_eq!(
            decide("sms:+15551234"),
            Decision::External("sms:+15551234".to_string())
        );
    }

    #[test]
    fn empty_input_is_blocked() {
        assert_eq!(decide(""), Decision::Block);
        assert_eq!(decide("   "), Decision::Block);
    }

    #[test]
    fn file_and_about_are_tabs() {
        assert_eq!(
            decide("about:blank"),
            Decision::Tab("about:blank".to_string())
        );
        assert_eq!(
            decide("file:///tmp/x.html"),
            Decision::Tab("file:///tmp/x.html".to_string())
        );
    }

    #[test]
    fn castcodes_scheme_is_a_tab() {
        assert_eq!(
            decide("castcodes://settings"),
            Decision::Tab("castcodes://settings".to_string())
        );
    }

    #[test]
    fn unknown_custom_scheme_routes_external() {
        assert_eq!(
            decide("slack://open?team=T123"),
            Decision::External("slack://open?team=T123".to_string())
        );
    }

    #[test]
    fn schemeless_falls_back_to_tab() {
        // The model's `normalize_url` will add https:// on load.
        assert_eq!(
            decide("example.com"),
            Decision::Tab("example.com".to_string())
        );
    }

    #[test]
    fn non_ascii_before_colon_does_not_panic() {
        assert_eq!(
            decide("🦄javascript:alert(1)"),
            Decision::Tab("🦄javascript:alert(1)".to_string())
        );
    }
}
