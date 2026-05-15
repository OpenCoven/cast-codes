pub const DEFAULT_BROWSER_URL: &str = "https://opencoven.ai";

pub type TabId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserTab {
    id: TabId,
    current_url: String,
    back_history: Vec<String>,
    forward_history: Vec<String>,
    loading: bool,
    title: String,
}

impl BrowserTab {
    fn new(id: TabId, url: impl Into<String>) -> Self {
        Self {
            id,
            current_url: normalize_url(url.into()),
            back_history: Vec::new(),
            forward_history: Vec::new(),
            loading: false,
            title: String::new(),
        }
    }

    pub fn id(&self) -> TabId {
        self.id
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

    fn navigate(&mut self, url: impl Into<String>) -> Option<String> {
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

    fn go_back(&mut self) -> Option<String> {
        let previous_url = self.back_history.pop()?;
        self.forward_history.push(self.current_url.clone());
        self.current_url = previous_url;
        self.loading = true;
        self.title.clear();

        Some(self.current_url.clone())
    }

    fn go_forward(&mut self) -> Option<String> {
        let next_url = self.forward_history.pop()?;
        self.back_history.push(self.current_url.clone());
        self.current_url = next_url;
        self.loading = true;
        self.title.clear();

        Some(self.current_url.clone())
    }

    fn reload(&mut self) -> String {
        self.loading = true;
        self.current_url.clone()
    }

    fn set_title(&mut self, title: impl Into<String>) -> bool {
        let title = title.into();
        let changed = self.title != title || self.loading;
        self.title = title;
        self.loading = false;
        changed
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClosedTabResult {
    pub removed_index: usize,
    pub new_tab_id: Option<TabId>,
}

#[derive(Debug, Clone)]
pub struct BrowserModel {
    tabs: Vec<BrowserTab>,
    active: usize,
    next_id: TabId,
}

impl BrowserModel {
    pub fn new(initial_url: impl Into<String>) -> Self {
        let mut model = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 0,
        };
        model.push_tab(initial_url);
        model
    }

    fn push_tab(&mut self, url: impl Into<String>) -> TabId {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(BrowserTab::new(id, url));
        id
    }

    pub fn tabs(&self) -> &[BrowserTab] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_tab(&self) -> &BrowserTab {
        &self.tabs[self.active]
    }

    fn active_tab_mut(&mut self) -> &mut BrowserTab {
        &mut self.tabs[self.active]
    }

    pub fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// Adds a new tab and makes it active. Returns the new tab's id and index.
    pub fn add_tab(&mut self, url: impl Into<String>) -> (TabId, usize) {
        let id = self.push_tab(url);
        let idx = self.tabs.len() - 1;
        self.active = idx;
        (id, idx)
    }

    /// Activates the tab at `idx` if it differs from the current active tab.
    /// Returns true if the active tab changed.
    pub fn select_tab(&mut self, idx: usize) -> bool {
        if idx >= self.tabs.len() || idx == self.active {
            return false;
        }
        self.active = idx;
        true
    }

    /// Closes the tab at `idx`. If the last tab is closed, a fresh default tab
    /// is added so the pane always has one tab.
    pub fn close_tab(&mut self, idx: usize) -> Option<ClosedTabResult> {
        if idx >= self.tabs.len() {
            return None;
        }

        let was_active = idx == self.active;
        self.tabs.remove(idx);

        let new_tab_id = if self.tabs.is_empty() {
            let id = self.push_tab(DEFAULT_BROWSER_URL);
            self.active = 0;
            Some(id)
        } else {
            if was_active {
                self.active = idx.min(self.tabs.len() - 1);
            } else if idx < self.active {
                self.active -= 1;
            }
            None
        };

        Some(ClosedTabResult {
            removed_index: idx,
            new_tab_id,
        })
    }

    // --- delegated active-tab methods ---

    pub fn current_url(&self) -> &str {
        self.active_tab().current_url()
    }

    pub fn display_title(&self) -> &str {
        self.active_tab().display_title()
    }

    pub fn is_loading(&self) -> bool {
        self.active_tab().is_loading()
    }

    pub fn can_go_back(&self) -> bool {
        self.active_tab().can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.active_tab().can_go_forward()
    }

    pub fn navigate(&mut self, url: impl Into<String>) -> Option<String> {
        self.active_tab_mut().navigate(url)
    }

    pub fn go_back(&mut self) -> Option<String> {
        self.active_tab_mut().go_back()
    }

    pub fn go_forward(&mut self) -> Option<String> {
        self.active_tab_mut().go_forward()
    }

    pub fn reload(&mut self) -> String {
        self.active_tab_mut().reload()
    }

    /// Updates the title of the tab identified by `id`. Returns whether
    /// anything changed (title text or loading flag).
    pub fn set_title_for(&mut self, id: TabId, title: impl Into<String>) -> bool {
        let Some(idx) = self.index_of(id) else {
            return false;
        };
        self.tabs[idx].set_title(title)
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
    fn tracks_history_on_active_tab() {
        let mut model = BrowserModel::new("https://one.test");

        model.navigate("https://two.test");
        model.navigate("https://three.test");

        assert!(model.can_go_back());
        assert_eq!(model.go_back().as_deref(), Some("https://two.test"));
        assert!(model.can_go_forward());
        assert_eq!(model.go_forward().as_deref(), Some("https://three.test"));
    }

    #[test]
    fn multi_tab_lifecycle() {
        let mut model = BrowserModel::new("https://a.test");
        assert_eq!(model.tabs().len(), 1);
        let (b_id, b_idx) = model.add_tab("https://b.test");
        assert_eq!(model.tabs().len(), 2);
        assert_eq!(model.active_index(), b_idx);

        // Switching tabs doesn't drop state.
        model.select_tab(0);
        assert_eq!(model.current_url(), "https://a.test");
        model.select_tab(1);
        assert_eq!(model.current_url(), "https://b.test");
        assert_eq!(model.index_of(b_id), Some(1));

        // Closing the active tab keeps the pane alive.
        model.close_tab(1);
        assert_eq!(model.tabs().len(), 1);
        assert_eq!(model.current_url(), "https://a.test");

        // Closing the only remaining tab inserts a fresh default tab.
        let result = model.close_tab(0).unwrap();
        assert_eq!(model.tabs().len(), 1);
        assert!(result.new_tab_id.is_some());
        assert_eq!(model.current_url(), DEFAULT_BROWSER_URL);
    }

    #[test]
    fn title_updates_target_specific_tab() {
        let mut model = BrowserModel::new("https://a.test");
        let (b_id, _) = model.add_tab("https://b.test");
        // Active is now tab `b`; updating the inactive tab's title still works.
        let a_id = model.tabs()[0].id();
        model.set_title_for(a_id, "Page A");
        model.set_title_for(b_id, "Page B");
        assert_eq!(model.tabs()[0].display_title(), "Page A");
        assert_eq!(model.tabs()[1].display_title(), "Page B");
    }
}
