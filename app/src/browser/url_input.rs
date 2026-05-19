//! URL / search query resolver for the browser pane's address bar.

use serde::{Deserialize, Serialize};

/// Search engine choices exposed in `general.default_search_engine`.
///
/// Each variant maps to a URL prefix that is concatenated with the
/// percent-encoded query. Adding a new variant requires:
/// 1. Adding the case to `Self::query_url_prefix`,
/// 2. Adding the case to `Self::display_name`,
/// 3. Adding the variant to `Self::all()`.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub enum SearchEngine {
    #[default]
    Google,
    DuckDuckGo,
    Bing,
    Kagi,
    Brave,
}

impl SearchEngine {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::DuckDuckGo => "DuckDuckGo",
            Self::Bing => "Bing",
            Self::Kagi => "Kagi",
            Self::Brave => "Brave Search",
        }
    }

    /// Prefix for a search query. The percent-encoded query is appended to
    /// the returned string verbatim, so each prefix must end in the query
    /// parameter delimiter (`=`).
    fn query_url_prefix(self) -> &'static str {
        match self {
            Self::Google => "https://www.google.com/search?q=",
            Self::DuckDuckGo => "https://duckduckgo.com/?q=",
            Self::Bing => "https://www.bing.com/search?q=",
            Self::Kagi => "https://kagi.com/search?q=",
            Self::Brave => "https://search.brave.com/search?q=",
        }
    }

    pub fn search_url(self, query: &str) -> String {
        let encoded = percent_encode_query(query);
        format!("{}{}", self.query_url_prefix(), encoded)
    }

    /// All variants, in stable display order. Used by settings UI.
    pub fn all() -> &'static [Self] {
        &[
            Self::Google,
            Self::DuckDuckGo,
            Self::Bing,
            Self::Kagi,
            Self::Brave,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Url(String),
    Search(String),
}

pub fn resolve_with_engine(raw: &str, engine: SearchEngine) -> Resolved {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Resolved::Url("about:home".to_string());
    }

    for scheme in [
        "http://",
        "https://",
        "file://",
        "about:",
        "data:",
        "castcodes://",
    ] {
        if trimmed.starts_with(scheme) {
            return Resolved::Url(trimmed.to_string());
        }
    }

    let looks_like_host = !trimmed.contains(char::is_whitespace)
        && (trimmed.contains('.') || is_loopback_host(trimmed));

    if looks_like_host {
        let scheme = if is_loopback_host(trimmed) {
            "http://"
        } else {
            "https://"
        };
        return Resolved::Url(format!("{scheme}{trimmed}"));
    }

    Resolved::Search(engine.search_url(trimmed))
}

fn is_loopback_host(input: &str) -> bool {
    let host_port = input.split_once('/').map(|(h, _)| h).unwrap_or(input);
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        rest.split_once(']').map(|(h, _)| h).unwrap_or(host_port)
    } else if host_port == "::1" {
        host_port
    } else {
        host_port
            .split_once(':')
            .map(|(h, _)| h)
            .unwrap_or(host_port)
    };
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

    fn resolve(raw: &str) -> Resolved {
        resolve_with_engine(raw, SearchEngine::default())
    }

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
        assert_eq!(resolve("::1"), Resolved::Url("http://::1".to_string()));
        assert_eq!(
            resolve("[::1]:3000"),
            Resolved::Url("http://[::1]:3000".to_string())
        );
    }

    #[test]
    fn default_search_engine_is_google() {
        assert_eq!(SearchEngine::default(), SearchEngine::Google);
    }

    #[test]
    fn freetext_defaults_to_google_search() {
        assert_eq!(
            resolve("rust async traits"),
            Resolved::Search("https://www.google.com/search?q=rust%20async%20traits".to_string())
        );
        assert_eq!(
            resolve("what is the time"),
            Resolved::Search("https://www.google.com/search?q=what%20is%20the%20time".to_string())
        );
    }

    #[test]
    fn freetext_uses_selected_engine() {
        assert_eq!(
            resolve_with_engine("rust async traits", SearchEngine::DuckDuckGo),
            Resolved::Search("https://duckduckgo.com/?q=rust%20async%20traits".to_string())
        );
        assert_eq!(
            resolve_with_engine("rust async traits", SearchEngine::Bing),
            Resolved::Search("https://www.bing.com/search?q=rust%20async%20traits".to_string())
        );
        assert_eq!(
            resolve_with_engine("rust", SearchEngine::Kagi),
            Resolved::Search("https://kagi.com/search?q=rust".to_string())
        );
        assert_eq!(
            resolve_with_engine("rust", SearchEngine::Brave),
            Resolved::Search("https://search.brave.com/search?q=rust".to_string())
        );
    }

    #[test]
    fn input_with_spaces_but_dotty_is_still_search() {
        assert_eq!(
            resolve("foo.bar baz"),
            Resolved::Search("https://www.google.com/search?q=foo.bar%20baz".to_string())
        );
    }

    #[test]
    fn all_variants_have_distinct_prefixes() {
        let mut seen = std::collections::HashSet::new();
        for engine in SearchEngine::all() {
            assert!(
                seen.insert(engine.query_url_prefix()),
                "duplicate prefix for {:?}",
                engine
            );
        }
    }
}
