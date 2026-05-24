use super::*;

#[test]
#[cfg(feature = "test-util")]
fn worktree_manager_flag_round_trip() {
    let flag = FeatureFlag::WorktreeManager;
    let _guard = flag.override_enabled(true);
    assert!(flag.is_enabled());
    drop(_guard);
    let _guard2 = flag.override_enabled(false);
    assert!(!flag.is_enabled());
}

#[test]
#[ignore = "CORE-3768 - need to clean up PREVIEW_FLAGS, but this is a temporary fix for the cluttered changelog"]
fn test_all_preview_flags_have_a_description() {
    for flag in PREVIEW_FLAGS {
        assert!(
            flag.flag_description()
                .is_some_and(|description| !description.is_empty()),
            "Missing description for preview-enabled flag {flag:?}"
        );
    }
}
