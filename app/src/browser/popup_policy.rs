//! Policy for `target="_blank"` and `window.open()` requests in the
//! embedded browser pane.
//!
//! All in-pane link requests that would normally pop a new OS window
//! get redirected to either a new tab in the pane (default) or the
//! system default browser (for a small allowlist of schemes that don't
//! belong in an in-app webview).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupAction {
    /// Open the URL as a new tab in the same browser pane.
    Tab(String),
    /// Hand off to the OS default handler (mailto, tel, sms, castcodes).
    External(String),
    /// Reject the request entirely (empty input, `javascript:` URLs).
    Block,
}

/// Decides what to do with a requested new-window URL. Pure function.
pub fn decide(url: &str) -> PopupAction {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return PopupAction::Block;
    }

    let scheme = trimmed.split_once(':').map(|(scheme, _)| scheme);
    if scheme.is_some_and(|scheme| scheme.eq_ignore_ascii_case("javascript")) {
        return PopupAction::Block;
    }

    if scheme.is_some_and(|scheme| {
        ["mailto", "tel", "sms", "castcodes"]
            .iter()
            .any(|external| scheme.eq_ignore_ascii_case(external))
    }) {
        return PopupAction::External(trimmed.to_string());
    }

    PopupAction::Tab(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_blocked() {
        assert_eq!(decide(""), PopupAction::Block);
        assert_eq!(decide("   "), PopupAction::Block);
    }

    #[test]
    fn known_external_schemes_handed_off() {
        assert_eq!(
            decide("mailto:hi@example.com"),
            PopupAction::External("mailto:hi@example.com".into())
        );
        assert_eq!(
            decide("tel:+15555550100"),
            PopupAction::External("tel:+15555550100".into())
        );
        assert_eq!(
            decide("castcodes://settings"),
            PopupAction::External("castcodes://settings".into())
        );
    }

    #[test]
    fn javascript_scheme_is_blocked() {
        assert_eq!(decide("javascript:alert(1)"), PopupAction::Block);
        assert_eq!(decide("JaVaScRiPt:alert(1)"), PopupAction::Block);
    }

    #[test]
    fn external_schemes_are_case_insensitive() {
        assert_eq!(
            decide("MAILTO:hi@example.com"),
            PopupAction::External("MAILTO:hi@example.com".into())
        );
    }

    #[test]
    fn https_url_opens_as_new_tab() {
        assert_eq!(
            decide("https://example.com/path"),
            PopupAction::Tab("https://example.com/path".into())
        );
    }

    #[test]
    fn http_loopback_opens_as_new_tab() {
        assert_eq!(
            decide("http://localhost:3000/foo"),
            PopupAction::Tab("http://localhost:3000/foo".into())
        );
    }
}
