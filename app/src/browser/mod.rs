pub(crate) mod about_home;
pub(crate) mod browser_model;
mod browser_view;
pub(crate) mod data_dir;
pub(crate) mod persistence;
pub(crate) mod popup_policy;
pub(crate) mod url_input;
pub(crate) mod webview_host;

pub use browser_model::{BrowserModel, DEFAULT_BROWSER_URL};
pub use browser_view::{BrowserView, BrowserViewEvent};
