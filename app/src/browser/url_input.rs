//! URL / search query resolver for the browser pane's address bar.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Url(String),
    Search(String),
}

pub fn resolve(raw: &str) -> Resolved {
    let _ = raw;
    unimplemented!("write me")
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
