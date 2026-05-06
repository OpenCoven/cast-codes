#![cfg(feature = "local_fs")]

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use repo_metadata::repositories::DetectedRepositories;
use warpui::{App, EntityId};

use crate::pane_group::WorkingDirectoriesModel;

#[test]
fn refresh_working_directories_collapses_subroots_to_nearest_repo_root() {
    App::test((), |mut app| async move {
        let detected_repos_handle = app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let repo_root = temp_dir.path().join("repo");
        let repo_a = repo_root.join("a");
        let repo_b = repo_root.join("b");
        fs::create_dir_all(&repo_a).expect("create repo/a");
        fs::create_dir_all(&repo_b).expect("create repo/b");

        // Use dunce::canonicalize to match the behavior of warp_util::standardized_path::StandardizedPath and normalize_cwd,
        // which strip the Windows extended-length path prefix (\\?\) for consistent comparison.
        let canonical_repo_root = dunce::canonicalize(&repo_root).expect("canonical repo root");

        // Seed DetectedRepositories so get_root_for_path resolves to this repo.
        detected_repos_handle.update(&mut app, |repos, _ctx| {
            let canonical =
                warp_util::standardized_path::StandardizedPath::from_local_canonicalized(
                    canonical_repo_root.as_path(),
                )
                .expect("canonicalized path");
            repos.insert_test_repo_root(canonical);
        });

        let pane_group_id = EntityId::new();
        let terminal_1 = EntityId::new();
        let terminal_2 = EntityId::new();

        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());
        let roots: Vec<PathBuf> = working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![
                    (terminal_1, repo_a.to_string_lossy().to_string()),
                    (terminal_2, repo_b.to_string_lossy().to_string()),
                ],
                vec![],
                Some(terminal_1),
                ctx,
            );

            model
                .most_recent_directories_for_pane_group(pane_group_id)
                .expect("pane group exists")
                .map(|dir| dir.path)
                .collect()
        });

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], canonical_repo_root);
    });
}

#[test]
fn refresh_working_directories_preserves_non_repo_paths_and_dedupes() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let dir_1 = temp_dir.path().join("dir-1");
        let dir_2 = temp_dir.path().join("dir-2");
        fs::create_dir_all(&dir_1).expect("create dir-1");
        fs::create_dir_all(&dir_2).expect("create dir-2");

        // Use dunce::canonicalize to match the behavior of normalize_cwd,
        // which strips the Windows extended-length path prefix (\\?\) for consistent comparison.
        let canonical_1 = dunce::canonicalize(&dir_1).expect("canonical dir-1");
        let canonical_2 = dunce::canonicalize(&dir_2).expect("canonical dir-2");

        let pane_group_id = EntityId::new();
        let terminal_1 = EntityId::new();
        let terminal_2 = EntityId::new();
        let terminal_3 = EntityId::new();

        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());
        let roots: HashSet<PathBuf> = working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![
                    (terminal_1, dir_1.to_string_lossy().to_string()),
                    (terminal_2, dir_2.to_string_lossy().to_string()),
                    // Duplicate root should be deduped.
                    (terminal_3, dir_1.to_string_lossy().to_string()),
                ],
                vec![],
                Some(terminal_1),
                ctx,
            );

            model
                .most_recent_directories_for_pane_group(pane_group_id)
                .expect("pane group exists")
                .map(|dir| dir.path)
                .collect()
        });

        assert_eq!(
            roots,
            HashSet::from_iter([canonical_1, canonical_2]),
            "should preserve non-repo roots and dedupe exact paths"
        );
    });
}

#[test]
fn refresh_working_directories_includes_detected_repos_under_non_repo_parent() {
    App::test((), |mut app| async move {
        let detected_repos_handle = app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = temp_dir.path().join("workspace");
        let repo_1 = workspace.join("repo-1");
        let repo_2 = workspace.join("repo-2");
        fs::create_dir_all(&repo_1).expect("create repo-1");
        fs::create_dir_all(&repo_2).expect("create repo-2");

        let canonical_workspace = dunce::canonicalize(&workspace).expect("canonical workspace");
        let canonical_repo_1 = dunce::canonicalize(&repo_1).expect("canonical repo-1");
        let canonical_repo_2 = dunce::canonicalize(&repo_2).expect("canonical repo-2");

        detected_repos_handle.update(&mut app, |repos, _ctx| {
            for repo_path in [&canonical_repo_1, &canonical_repo_2] {
                let canonical =
                    warp_util::standardized_path::StandardizedPath::from_local_canonicalized(
                        repo_path,
                    )
                    .expect("canonicalized repo path");
                repos.insert_test_repo_root(canonical);
            }
        });

        let pane_group_id = EntityId::new();
        let terminal = EntityId::new();
        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());

        let repos: HashSet<PathBuf> = working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![(terminal, canonical_workspace.to_string_lossy().to_string())],
                vec![],
                Some(terminal),
                ctx,
            );

            model
                .most_recent_repositories_for_pane_group(pane_group_id)
                .expect("pane group repos exist")
                .collect()
        });

        assert_eq!(
            repos,
            HashSet::from_iter([canonical_repo_1.clone(), canonical_repo_2.clone()])
        );

        working_directories_handle.read(&app, |model, _ctx| {
            assert_eq!(
                model.get_terminal_id_for_root_path(pane_group_id, &canonical_repo_1),
                Some(terminal)
            );
            assert_eq!(
                model.get_terminal_id_for_root_path(pane_group_id, &canonical_repo_2),
                Some(terminal)
            );
        });
    });
}

#[test]
fn refresh_working_directories_omits_deleted_repos_under_non_repo_parent() {
    App::test((), |mut app| async move {
        let detected_repos_handle = app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = temp_dir.path().join("workspace");
        let repo_1 = workspace.join("repo-1");
        let deleted_repo = workspace.join("deleted-repo");
        fs::create_dir_all(&repo_1).expect("create repo-1");
        fs::create_dir_all(&deleted_repo).expect("create deleted-repo");

        let canonical_workspace = dunce::canonicalize(&workspace).expect("canonical workspace");
        let canonical_repo_1 = dunce::canonicalize(&repo_1).expect("canonical repo-1");
        let canonical_deleted_repo =
            dunce::canonicalize(&deleted_repo).expect("canonical deleted-repo");

        detected_repos_handle.update(&mut app, |repos, _ctx| {
            for repo_path in [&canonical_repo_1, &canonical_deleted_repo] {
                let canonical =
                    warp_util::standardized_path::StandardizedPath::from_local_canonicalized(
                        repo_path,
                    )
                    .expect("canonicalized repo path");
                repos.insert_test_repo_root(canonical);
            }
        });

        fs::remove_dir_all(&canonical_deleted_repo).expect("remove deleted repo");

        let pane_group_id = EntityId::new();
        let terminal = EntityId::new();
        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());

        let repos: HashSet<PathBuf> = working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![(terminal, canonical_workspace.to_string_lossy().to_string())],
                vec![],
                Some(terminal),
                ctx,
            );

            model
                .most_recent_repositories_for_pane_group(pane_group_id)
                .expect("pane group repos exist")
                .collect()
        });

        assert_eq!(repos, HashSet::from_iter([canonical_repo_1]));
        working_directories_handle.read(&app, |model, _ctx| {
            assert_eq!(
                model.get_terminal_id_for_root_path(pane_group_id, &canonical_deleted_repo),
                None
            );
        });
    });
}

#[test]
fn refresh_working_directories_does_not_focus_child_repos_from_non_repo_parent() {
    App::test((), |mut app| async move {
        let detected_repos_handle = app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = temp_dir.path().join("workspace");
        let repo_1 = workspace.join("repo-1");
        let repo_2 = workspace.join("repo-2");
        fs::create_dir_all(&repo_1).expect("create repo-1");
        fs::create_dir_all(&repo_2).expect("create repo-2");

        let canonical_workspace = dunce::canonicalize(&workspace).expect("canonical workspace");
        let canonical_repo_1 = dunce::canonicalize(&repo_1).expect("canonical repo-1");
        let canonical_repo_2 = dunce::canonicalize(&repo_2).expect("canonical repo-2");

        detected_repos_handle.update(&mut app, |repos, _ctx| {
            for repo_path in [&canonical_repo_1, &canonical_repo_2] {
                let canonical =
                    warp_util::standardized_path::StandardizedPath::from_local_canonicalized(
                        repo_path,
                    )
                    .expect("canonicalized repo path");
                repos.insert_test_repo_root(canonical);
            }
        });

        let pane_group_id = EntityId::new();
        let terminal = EntityId::new();
        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());

        working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![(terminal, canonical_workspace.to_string_lossy().to_string())],
                vec![],
                Some(terminal),
                ctx,
            );
        });

        working_directories_handle.read(&app, |model, _ctx| {
            assert_eq!(
                model.focused_repo.get(&pane_group_id).cloned().flatten(),
                None,
                "a non-repo parent with multiple detected child repos should not pick an arbitrary focused repo"
            );
        });
    });
}

#[test]
fn refresh_working_directories_keeps_repo_terminal_mapping_over_parent_fallback() {
    App::test((), |mut app| async move {
        let detected_repos_handle = app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = temp_dir.path().join("workspace");
        let repo_1_src = workspace.join("repo-1/src");
        let repo_2 = workspace.join("repo-2");
        fs::create_dir_all(&repo_1_src).expect("create repo-1/src");
        fs::create_dir_all(&repo_2).expect("create repo-2");

        let repo_1 = workspace.join("repo-1");
        let canonical_workspace = dunce::canonicalize(&workspace).expect("canonical workspace");
        let canonical_repo_1 = dunce::canonicalize(&repo_1).expect("canonical repo-1");
        let canonical_repo_1_src = dunce::canonicalize(&repo_1_src).expect("canonical repo-1/src");
        let canonical_repo_2 = dunce::canonicalize(&repo_2).expect("canonical repo-2");

        detected_repos_handle.update(&mut app, |repos, _ctx| {
            for repo_path in [&canonical_repo_1, &canonical_repo_2] {
                let canonical =
                    warp_util::standardized_path::StandardizedPath::from_local_canonicalized(
                        repo_path,
                    )
                    .expect("canonicalized repo path");
                repos.insert_test_repo_root(canonical);
            }
        });

        let pane_group_id = EntityId::new();
        let repo_terminal = EntityId::new();
        let parent_terminal = EntityId::new();
        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());

        working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![
                    (
                        repo_terminal,
                        canonical_repo_1_src.to_string_lossy().to_string(),
                    ),
                    (
                        parent_terminal,
                        canonical_workspace.to_string_lossy().to_string(),
                    ),
                ],
                vec![],
                Some(parent_terminal),
                ctx,
            );
        });

        working_directories_handle.read(&app, |model, _ctx| {
            assert_eq!(
                model.get_terminal_id_for_root_path(pane_group_id, &canonical_repo_1),
                Some(repo_terminal),
                "fallback mappings from a non-repo parent should not overwrite repo-local terminals"
            );
            assert_eq!(
                model.get_terminal_id_for_root_path(pane_group_id, &canonical_repo_2),
                Some(parent_terminal)
            );
        });
    });
}

#[test]
fn refresh_working_directories_focuses_repo_from_focused_terminal_cwd() {
    App::test((), |mut app| async move {
        let detected_repos_handle = app.add_singleton_model(|_| DetectedRepositories::default());

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = temp_dir.path().join("workspace");
        let repo_1_src = workspace.join("repo-1/src");
        let repo_2 = workspace.join("repo-2");
        fs::create_dir_all(&repo_1_src).expect("create repo-1/src");
        fs::create_dir_all(&repo_2).expect("create repo-2");

        let repo_1 = workspace.join("repo-1");
        let canonical_repo_1 = dunce::canonicalize(&repo_1).expect("canonical repo-1");
        let canonical_repo_2 = dunce::canonicalize(&repo_2).expect("canonical repo-2");
        let canonical_repo_1_src = dunce::canonicalize(&repo_1_src).expect("canonical repo-1/src");

        detected_repos_handle.update(&mut app, |repos, _ctx| {
            for repo_path in [&canonical_repo_1, &canonical_repo_2] {
                let canonical =
                    warp_util::standardized_path::StandardizedPath::from_local_canonicalized(
                        repo_path,
                    )
                    .expect("canonicalized repo path");
                repos.insert_test_repo_root(canonical);
            }
        });

        let pane_group_id = EntityId::new();
        let focused_terminal = EntityId::new();
        let working_directories_handle = app.add_model(|_| WorkingDirectoriesModel::new());

        working_directories_handle.update(&mut app, |model, ctx| {
            model.refresh_working_directories_for_pane_group(
                pane_group_id,
                vec![(
                    focused_terminal,
                    canonical_repo_1_src.to_string_lossy().to_string(),
                )],
                vec![],
                Some(focused_terminal),
                ctx,
            );
        });

        working_directories_handle.read(&app, |model, _ctx| {
            assert_eq!(
                model.focused_repo.get(&pane_group_id).cloned().flatten(),
                Some(canonical_repo_1),
                "focused repo should come from the focused terminal's actual cwd"
            );
        });
    });
}
