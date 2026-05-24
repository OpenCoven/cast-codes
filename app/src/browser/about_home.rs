//! Built-in start page for new tabs. Served as a `data:` URL so we don't
//! need a custom URL-scheme handler for v1.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;

const ABOUT_HOME_HTML: &str = include_str!("../../assets/bundled/html/about_home.html");

/// Returns the data: URL representing the new-tab page.
pub fn url() -> String {
    let encoded = B64.encode(ABOUT_HOME_HTML.as_bytes());
    format!("data:text/html;charset=utf-8;base64,{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_is_data_scheme() {
        let u = url();
        assert!(u.starts_with("data:text/html;charset=utf-8;base64,"));
        let prefix = "data:text/html;charset=utf-8;base64,";
        let body = &u[prefix.len()..];
        let decoded = B64.decode(body).expect("base64 decodes");
        let decoded_str = String::from_utf8(decoded).expect("utf8");
        assert!(decoded_str.contains("<h1>New Tab</h1>"));
    }

    #[test]
    fn html_uses_theme_tokens_not_drift_values() {
        // The old hand-tuned values (`#0e0e10` / `#e8e8ea`) were off by
        // one channel unit from the actual CastCodes tokens. The page now
        // declares the correct dark tokens as CSS variables; this test
        // guards against re-introducing the drift values.
        assert!(
            !ABOUT_HOME_HTML.contains("#0e0e10"),
            "drift bg snuck back in"
        );
        assert!(
            !ABOUT_HOME_HTML.contains("#e8e8ea"),
            "drift fg snuck back in"
        );
        assert!(ABOUT_HOME_HTML.contains("#0f0f12"), "dark bg token missing");
        assert!(ABOUT_HOME_HTML.contains("#e8e8ed"), "dark fg token missing");
    }

    #[test]
    fn html_honors_system_color_scheme() {
        // The previous version locked `color-scheme: dark`, which made the
        // page look out of place in a light desktop appearance.
        assert!(
            !ABOUT_HOME_HTML.contains("color-scheme: dark;"),
            "color-scheme is still locked to dark"
        );
        assert!(ABOUT_HOME_HTML.contains("color-scheme: light dark"));
        assert!(ABOUT_HOME_HTML.contains("prefers-color-scheme: light"));
    }

    #[test]
    fn html_declares_csp() {
        // The page is served as a data: URL, but CSP belt-and-braces:
        // no scripts, no external resources of any kind.
        assert!(
            ABOUT_HOME_HTML.contains("Content-Security-Policy"),
            "CSP meta tag missing"
        );
        assert!(ABOUT_HOME_HTML.contains("default-src 'none'"));
    }
}
