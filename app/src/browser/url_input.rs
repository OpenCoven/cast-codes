//! URL / search query resolver for the browser pane's address bar.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Url(String),
    Search(String),
}

pub fn resolve(raw: &str) -> Resolved {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Resolved::Url("about:home".to_string());
    }

    for scheme in ["http://", "https://", "file://", "about:", "data:", "castcodes://"] {
        if trimmed.starts_with(scheme) {
            return Resolved::Url(trimmed.to_string());
        }
    }

    let looks_like_host = !trimmed.contains(char::is_whitespace)
        && (trimmed.contains('.') || is_loopback_host(trimmed));

    if looks_like_host {
        let scheme = if is_loopback_host(trimmed) { "http://" } else { "https://" };
        return Resolved::Url(format!("{scheme}{trimmed}"));
    }

    let encoded = percent_encode_query(trimmed);
    Resolved::Search(format!("https://duckduckgo.com/?q={encoded}"))
}

fn is_loopback_host(input: &str) -> bool {
    let host = input.split_once('/').map(|(h, _)| h).unwrap_or(input);
    let host = host.split_once(':').map(|(h, _)| h).unwrap_or(host);
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
}

fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_goes_to_about_home() {
        assert_eq!(resolve(""), Resolved::Url("about:home".to_string()));
        assert_eq!(resolve("   "), Resolved::Url("about:home".to_string()));
    }

    #[test]
    fn known_schemes_pass_through() {
        for url in [
            "http://example.com",
            "https://example.com",
            "file:///tmp/x.html",
            "about:blank",
            "data:text/html,<h1>hi</h1>",
            "castcodes://settings",
        ] {
            assert_eq!(resolve(url), Resolved::Url(url.to_string()));
        }
    }

    #[test]
    fn bare_hostname_gets_https() {
        assert_eq!(
            resolve("example.com"),
            Resolved::Url("https://example.com".to_string())
        );
        assert_eq!(
            resolve("example.com/path?q=1"),
            Resolved::Url("https://example.com/path?q=1".to_string())
        );
    }

    #[test]
    fn loopback_gets_http_not_https() {
        assert_eq!(
            resolve("localhost"),
            Resolved::Url("http://localhost".to_string())
        );
        assert_eq!(
            resolve("localhost:3000"),
            Resolved::Url("http://localhost:3000".to_string())
        );
        assert_eq!(
            resolve("127.0.0.1:8080/api"),
            Resolved::Url("http://127.0.0.1:8080/api".to_string())
        );
    }

    #[test]
    fn freetext_becomes_duckduckgo_search() {
        assert_eq!(
            resolve("rust async traits"),
            Resolved::Search("https://duckduckgo.com/?q=rust%20async%20traits".to_string())
        );
        assert_eq!(
            resolve("what is the time"),
            Resolved::Search("https://duckduckgo.com/?q=what%20is%20the%20time".to_string())
        );
    }

    #[test]
    fn input_with_spaces_but_dotty_is_still_search() {
        assert_eq!(
            resolve("foo.bar baz"),
            Resolved::Search("https://duckduckgo.com/?q=foo.bar%20baz".to_string())
        );
    }
}
