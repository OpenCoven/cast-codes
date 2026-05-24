pub(crate) mod about_home;
pub(crate) mod browser_model;
mod browser_view;
#[cfg(not(target_family = "wasm"))]
pub(crate) mod data_dir;
#[cfg(not(target_family = "wasm"))]
pub(crate) mod dialogs;
pub(crate) mod find;
pub(crate) mod persistence;
pub(crate) mod popup_policy;
pub(crate) mod url_input;
pub(crate) mod webview_host;

pub use browser_model::DEFAULT_BROWSER_URL;
pub use browser_view::{BrowserView, BrowserViewEvent};
