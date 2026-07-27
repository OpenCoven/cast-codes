use crate::Builder;
use warpui::integration::{AssertionOutcome, TestStep};

pub fn test_oss_app_menu_startup() -> Builder {
    Builder::new().with_step(TestStep::new("Assert OSS app startup").add_named_assertion(
        "initial root view exists",
        |app, window_id| {
            if app
                .root_view::<warp::root_view::RootView>(window_id)
                .is_some()
            {
                AssertionOutcome::Success
            } else {
                AssertionOutcome::failure(format!(
                    "root view should exist for window_id={window_id}"
                ))
            }
        },
    ))
}
