//! Handler for the `castcodes://` custom URL scheme.
//!
//! Registered on the wry WebViewBuilder via `with_custom_protocol`. Without
//! a handler installed, the scheme passes through our URL normalizer
//! (`browser_model::normalize_url`) but WKWebView errors out with
//! "scheme not supported". This module makes the scheme actually load
//! something — currently just an About page; the routing table is
//! intentionally small so we can grow it deliberately rather than
//! discover ad-hoc deep links in the wild.
//!
//! Known routes:
//!   - `castcodes://about`  → About page (version, bundle id, scheme docs).
//!   - anything else        → 404 HTML listing the known routes.

use std::borrow::Cow;
use wry::http::{header, Request, Response, StatusCode};

const ABOUT_TEMPLATE: &str = include_str!("../../assets/bundled/html/castcodes_about.html");

/// Pure routing function. `path` is the URL portion after the scheme
/// (e.g. `"about"` for `castcodes://about`); leading/trailing slashes
/// and query strings are tolerated.
pub(crate) fn route(path: &str) -> Route {
    let normalized = path
        .trim_start_matches('/')
        .split_once('?')
        .map(|(p, _)| p)
        .unwrap_or(path)
        .trim_start_matches('/')
        .trim_end_matches('/');

    match normalized {
        "" | "about" => Route::About,
        _ => Route::NotFound,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    About,
    NotFound,
}

/// Build the HTTP-shaped response for an incoming `castcodes://...`
/// request. Pure: no I/O, just template interpolation.
pub(crate) fn handle(request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let uri = request.uri();
    // wry hands us the full URI as `castcodes://<host>/<path>?<query>`. We
    // accept either the `host` or the path as the route key so users can
    // write either `castcodes://about` or `castcodes:///about`.
    let host = uri.host().unwrap_or("");
    let path = uri.path();
    let raw = route_key(host, path);

    let route = route(&raw);
    match route {
        Route::About => respond_html(StatusCode::OK, render_about()),
        Route::NotFound => respond_html(StatusCode::NOT_FOUND, render_not_found(&raw)),
    }
}

fn render_about() -> String {
    ABOUT_TEMPLATE
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
        .replace("{{APP_ID}}", warp_core::brand::PUBLIC_APP_ID)
        .replace("{{SCHEME}}", warp_core::brand::PUBLIC_URL_SCHEME)
}

fn render_not_found(requested: &str) -> String {
    // Inline 404 — small enough to not need its own asset.
    let scheme = warp_core::brand::PUBLIC_URL_SCHEME;
    let escaped = html_escape(requested);
    format!(
        "<!doctype html>\n\
<html lang=\"en\"><head><meta charset=\"utf-8\"><title>Not Found</title>\n\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">\n\
<style>:root{{color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, system-ui, sans-serif; --bg:#0f0f12; --fg:#e8e8ed;}}@media (prefers-color-scheme: light){{:root{{--bg:#fafafa;--fg:#1a1a1a;}}}}html,body{{margin:0;padding:0;height:100%;background:var(--bg);color:var(--fg);}}main{{max-width:36rem;margin:0 auto;padding:4rem 2rem;}}h1{{margin:0 0 1rem 0;font-size:28px;}}p{{font-size:14px;opacity:0.7;}}code{{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px;}}</style></head>\n\
<body><main>\n\
<h1>Unknown route</h1>\n\
<p>No handler for <code>{scheme}://{escaped}</code>.</p>\n\
<p>Known routes: <code>{scheme}://about</code>.</p>\n\
</main></body></html>\n"
    )
}

fn route_key(host: &str, path: &str) -> String {
    if host.is_empty() {
        path.to_string()
    } else if path == "/" {
        host.to_string()
    } else {
        format!("{host}{path}")
    }
}

fn respond_html(status: StatusCode, body: String) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Cow::Owned(body.into_bytes()))
        // The builder only fails on malformed headers; we hardcode both.
        // unwrap is safe and serves as documentation that this can't fail.
        .expect("static html response is well-formed")
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wry::http::Request;

    fn req(uri: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(uri)
            .body(Vec::new())
            .expect("valid request")
    }

    #[test]
    fn route_about_aliases() {
        assert_eq!(route("about"), Route::About);
        assert_eq!(route("/about"), Route::About);
        assert_eq!(route("/about/"), Route::About);
        assert_eq!(route("about?x=1"), Route::About);
        // Empty path also lands on About — `castcodes://about/` parses
        // to host=about, path="/", and we accept either layout.
        assert_eq!(route(""), Route::About);
    }

    #[test]
    fn route_unknown_is_not_found() {
        assert_eq!(route("settings"), Route::NotFound);
        assert_eq!(route("/help"), Route::NotFound);
        assert_eq!(route("downloads/active"), Route::NotFound);
    }

    #[test]
    fn handle_about_returns_200_html() {
        let response = handle(&req("castcodes://about"));
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok());
        assert_eq!(ct, Some("text/html; charset=utf-8"));
        let body = std::str::from_utf8(response.body().as_ref()).expect("utf-8");
        assert!(body.contains("CastCodes"));
        assert!(body.contains(env!("CARGO_PKG_VERSION")));
        assert!(body.contains(&format!("{}://about", warp_core::brand::PUBLIC_URL_SCHEME)));
        // Templates were interpolated, not left as placeholders.
        assert!(!body.contains("{{VERSION}}"));
        assert!(!body.contains("{{APP_ID}}"));
        assert!(!body.contains("{{SCHEME}}"));
    }

    #[test]
    fn handle_host_form_is_about() {
        // `castcodes://about` parses with host="about", path="/" — make
        // sure we resolve that to About, not Not Found.
        let response = handle(&req("castcodes://about/"));
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn handle_unknown_returns_404() {
        let response = handle(&req("castcodes://settings"));
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = std::str::from_utf8(response.body().as_ref()).expect("utf-8");
        // The 404 page should mention what was requested so users can
        // spot typos.
        assert!(body.contains("settings"));
        assert!(body.contains("Known routes"));
    }

    #[test]
    fn handle_unknown_authority_form_echoes_without_synthetic_slash() {
        let response = handle(&req("castcodes://settings"));
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = std::str::from_utf8(response.body().as_ref()).expect("utf-8");
        assert!(body.contains("castcodes://settings"));
        assert!(!body.contains("castcodes://settings/"));
    }

    #[test]
    fn route_key_keeps_real_host_paths() {
        assert_eq!(route_key("downloads", "/active"), "downloads/active");
        assert_eq!(route_key("settings", "/"), "settings");
        assert_eq!(route_key("", "/about"), "/about");
    }

    #[test]
    fn html_escape_neutralizes_metacharacters() {
        assert_eq!(
            html_escape("<script>alert('x')</script>"),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
    }

    #[test]
    fn render_not_found_escapes_html_metacharacters() {
        // Defense against an attacker getting a user to visit a crafted
        // `castcodes://...` URL whose path string is rendered back into
        // the 404 page. The renderer MUST escape HTML metacharacters.
        // (We test `render_not_found` directly because attacker-shaped
        // inputs aren't always expressible in a valid `http::Uri`.)
        let html = render_not_found("<script>alert('x')</script>");
        assert!(!html.contains("<script>"), "404 leaked unescaped HTML");
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&#39;"));
    }
}
