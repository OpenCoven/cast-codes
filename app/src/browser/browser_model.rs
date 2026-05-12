pub const DEFAULT_BROWSER_URL: &str = "https://opencoven.ai";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserModel {
    current_url: String,
    back_history: Vec<String>,
    forward_history: Vec<String>,
    loading: bool,
    title: String,
}

impl BrowserModel {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            current_url: normalize_url(url.into()),
            back_history: Vec::new(),
            forward_history: Vec::new(),
            loading: false,
            title: String::new(),
        }
    }

    pub fn current_url(&self) -> &str {
        &self.current_url
    }

    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            &self.current_url
        } else {
            &self.title
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_history.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_history.is_empty()
    }

    pub fn navigate(&mut self, url: impl Into<String>) -> Option<String> {
        let next_url = normalize_url(url.into());
        if next_url == self.current_url {
            return None;
        }

        self.back_history.push(self.current_url.clone());
        self.forward_history.clear();
        self.current_url = next_url;
        self.loading = true;
        self.title.clear();

        Some(self.current_url.clone())
    }

    pub fn go_back(&mut self) -> Option<String> {
        let previous_url = self.back_history.pop()?;
        self.forward_history.push(self.current_url.clone());
        self.current_url = previous_url;
        self.loading = true;
        self.title.clear();

        Some(self.current_url.clone())
    }

    pub fn go_forward(&mut self) -> Option<String> {
        let next_url = self.forward_history.pop()?;
        self.back_history.push(self.current_url.clone());
        self.current_url = next_url;
        self.loading = true;
        self.title.clear();

        Some(self.current_url.clone())
    }

    pub fn reload(&mut self) -> String {
        self.loading = true;
        self.current_url.clone()
    }

    pub fn set_title(&mut self, title: impl Into<String>) -> bool {
        let title = title.into();
        let changed = self.title != title || self.loading;
        self.title = title;
        self.loading = false;
        changed
    }
}

fn normalize_url(url: impl Into<String>) -> String {
    let url = url.into();
    let url = url.trim();

    if url.is_empty() {
        return DEFAULT_BROWSER_URL.to_string();
    }

    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
        || url.starts_with("about:")
        || url.starts_with("data:")
    {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_empty_and_bare_urls() {
        assert_eq!(normalize_url(""), DEFAULT_BROWSER_URL);
        assert_eq!(normalize_url("opencoven.ai"), "https://opencoven.ai");
        assert_eq!(
            normalize_url("http://localhost:3000"),
            "http://localhost:3000"
        );
    }

    #[test]
    fn tracks_history() {
        let mut model = BrowserModel::new("https://one.test");

        model.navigate("https://two.test");
        model.navigate("https://three.test");

        assert!(model.can_go_back());
        assert_eq!(model.go_back().as_deref(), Some("https://two.test"));
        assert!(model.can_go_forward());
        assert_eq!(model.go_forward().as_deref(), Some("https://three.test"));
    }
}
