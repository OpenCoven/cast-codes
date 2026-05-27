// Find scripts only run from the native (non-wasm) wry webview host;
// on wasm the JS strings + helper functions exist in tree but are
// unreachable. Allow dead_code on wasm builds only.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

//! Find-in-page overlay model + injected JS glue.
//!
//! The overlay is a thin row that renders below the toolbar when active.
//! When the user types a query, the host evaluates a small JS routine
//! (`bundled/js/find.js`) inside the active tab's webview, which walks the
//! main document and wraps matches in highlight spans. Match counts come
//! back via `window.ipc.postMessage` and surface as
//! `NativeWebViewEvent::FindResults`.
//!
//! Limitations (v0; tracked in #126 follow-ups):
//!   - Main document only — no iframe / shadow-DOM traversal.
//!   - Case-insensitive plain-string match (no regex / whole-word).
//!   - Re-walks the DOM on every query (O(n) per keystroke).
//!   - No Cmd+F keybinding yet — opened via the toolbar magnifier button.

use serde::Deserialize;

/// The find script. Loaded once into each tab's webview before the user's
/// first find query, and idempotent on re-injection (clears prior state).
pub(crate) const FIND_SCRIPT: &str = include_str!("../../assets/bundled/js/find.js");

/// Inline state held by `BrowserView` while the find overlay is open.
#[derive(Debug, Clone, Default)]
pub(crate) struct FindState {
    pub query: String,
    /// 1-based index of the active match, or 0 when no matches.
    pub current: usize,
    pub total: usize,
}

impl FindState {
    pub(crate) fn count_label(&self) -> String {
        if self.query.is_empty() {
            String::new()
        } else if self.total == 0 {
            "No matches".to_string()
        } else {
            format!("{} of {}", self.current, self.total)
        }
    }
}

/// Build the JS expression that sets the find query inside the page.
/// The query is JSON-encoded so it survives quotes, backslashes, etc.
pub(crate) fn set_query_script(query: &str) -> String {
    format!(
        "(function() {{ if (window.__castcodes_find__) {{ window.__castcodes_find__.setQuery({}); }} }})();",
        json_string(query)
    )
}

pub(crate) fn next_script() -> &'static str {
    "(function() { if (window.__castcodes_find__) window.__castcodes_find__.next(); })();"
}

pub(crate) fn prev_script() -> &'static str {
    "(function() { if (window.__castcodes_find__) window.__castcodes_find__.prev(); })();"
}

pub(crate) fn clear_script() -> &'static str {
    "(function() { if (window.__castcodes_find__) window.__castcodes_find__.clear(); })();"
}

/// Shape of the JSON message the find script posts back via
/// `window.ipc.postMessage`. See `assets/bundled/js/find.js`.
//
// Constructed only by serde inside webview_host's macOS-gated IPC
// handler; on Linux/Windows the type compiles but nothing deserializes
// into it. Allowed at the struct level until the non-macOS wry wiring
// lands.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FindResultsMessage {
    pub kind: String,
    pub current: usize,
    pub total: usize,
}

fn json_string(s: &str) -> String {
    // Tiny dependency-free JSON string serializer — we only ever emit a
    // single string literal so we don't need serde_json.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_label_empty_query_is_blank() {
        let s = FindState::default();
        assert_eq!(s.count_label(), "");
    }

    #[test]
    fn count_label_no_matches() {
        let s = FindState {
            query: "abc".to_string(),
            current: 0,
            total: 0,
        };
        assert_eq!(s.count_label(), "No matches");
    }

    #[test]
    fn count_label_with_matches() {
        let s = FindState {
            query: "abc".to_string(),
            current: 3,
            total: 12,
        };
        assert_eq!(s.count_label(), "3 of 12");
    }

    #[test]
    fn json_string_escapes_quotes_and_backslash() {
        assert_eq!(json_string("hello"), "\"hello\"");
        assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_string("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn json_string_escapes_control_chars() {
        assert_eq!(json_string("a\nb"), "\"a\\nb\"");
        assert_eq!(json_string("a\tb"), "\"a\\tb\"");
        assert_eq!(json_string("a\x01b"), "\"a\\u0001b\"");
    }

    #[test]
    fn set_query_script_injects_query_as_json() {
        let s = set_query_script("hello \"world\"");
        // Must contain the JSON-quoted query (escaped inner quotes).
        assert!(s.contains("setQuery(\"hello \\\"world\\\"\")"), "got: {s}");
    }

    #[test]
    fn find_script_const_loads() {
        // The included asset must be non-empty and contain the public
        // namespace marker so we know we're including the right file.
        assert!(FIND_SCRIPT.contains("__castcodes_find__"));
    }
}
