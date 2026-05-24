use super::slugify_branch;

#[test]
fn slugify_simple() {
    assert_eq!(slugify_branch("main"), "main");
}

#[test]
fn slugify_with_slash() {
    assert_eq!(slugify_branch("feature/foo"), "feature-foo");
}

#[test]
fn slugify_with_whitespace_and_punct() {
    assert_eq!(slugify_branch("feat/Foo Bar.baz"), "feat-foo-bar.baz");
}

#[test]
fn slugify_collapses_runs() {
    assert_eq!(slugify_branch("a///b___c"), "a-b-c");
}

#[test]
fn slugify_trims_edges() {
    assert_eq!(slugify_branch("/feature/foo/"), "feature-foo");
}

#[test]
fn slugify_empty_falls_back() {
    assert_eq!(slugify_branch("///"), "worktree");
    assert_eq!(slugify_branch(""), "worktree");
}

#[test]
fn slugify_lowercases() {
    assert_eq!(slugify_branch("Feature/UPPER"), "feature-upper");
}

#[test]
fn slugify_unicode_strips() {
    assert_eq!(slugify_branch("feat/✨sparkle"), "feat-sparkle");
}
