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
}
