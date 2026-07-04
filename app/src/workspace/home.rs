//! CastCodes Home
//!
//! This is the landing page for new tabs if session creation isn't supported (e.g. on the web).
//! It's barebones at the moment, but may grow into a more full-featured admin experience.

use warpui::ViewContext;

use super::view::Workspace;
use crate::pane_group::{AnyPaneContent, FilePane};

const WARP_HOME_TITLE: &str = "Welcome to CastCodes";
const WARP_HOME_CONTENT: &str = r#"
CastCodes is a local-first terminal and code workspace.

Use this home view to:
* Open a terminal session when local shells are available
* Open a project and keep terminal context attached to it
* Manage CastCodes settings and workspace objects"#;

/// Create a static "home page" pane.
pub fn create_home_pane(ctx: &mut ViewContext<Workspace>) -> Box<dyn AnyPaneContent> {
    let pane = FilePane::new(
        None,
        None,
        #[cfg(feature = "local_fs")]
        None,
        ctx,
    );
    pane.file_view(ctx).update(ctx, |pane, ctx| {
        pane.open_static(WARP_HOME_TITLE, WARP_HOME_CONTENT, ctx);
    });
    Box::new(pane)
}

#[cfg(test)]
mod tests {
    use super::{WARP_HOME_CONTENT, WARP_HOME_TITLE};

    #[test]
    fn home_copy_is_castcodes_local_first() {
        let inherited_web_home = ["Warp", "on Web"].join(" ");
        assert_eq!(WARP_HOME_TITLE, "Welcome to CastCodes");
        assert!(WARP_HOME_CONTENT.contains("local-first"));
        assert!(!WARP_HOME_CONTENT.contains(&inherited_web_home));
        assert!(!WARP_HOME_CONTENT.contains("shared sessions"));
    }
}
