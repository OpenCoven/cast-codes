use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};

define_settings_group!(WorktreeManagerSettings, settings: [
    staging_directory: StagingDirectory {
        type: Option<String>,
        default: None,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "worktree_manager.staging_directory",
        description: "Path template for the worktree staging directory. \
                      Supports placeholders: <repo-root> (absolute path to the \
                      repository root) and <branch-slug> (URL-safe branch name).",
    },
    prune_on_remove: PruneOnRemove {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "worktree_manager.prune_on_remove",
        description: "Whether to run `git worktree prune` automatically after \
                      removing a worktree.",
    },
]);

#[cfg(test)]
mod tests {
    use settings::{manager::SettingsManager, Setting as _};
    use warpui::{App, SingletonEntity as _};
    use warpui_extras::user_preferences;

    use super::{PruneOnRemove, StagingDirectory, WorktreeManagerSettings};

    fn init_preferences(ctx: &mut warpui::AppContext) {
        ctx.add_singleton_model(move |_| {
            settings::PublicPreferences::new(
                Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
            )
        });
        ctx.add_singleton_model(move |_| {
            settings::PrivatePreferences::new(
                Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
            )
        });
    }

    #[test]
    fn test_staging_directory_default_is_none() {
        App::test((), |mut app| async move {
            app.update(init_preferences);
            app.add_singleton_model(|_| SettingsManager::default());
            WorktreeManagerSettings::register(&mut app);

            app.read(|ctx| {
                let settings = WorktreeManagerSettings::as_ref(ctx);
                assert_eq!(
                    *settings.staging_directory.value(),
                    StagingDirectory::default_value(),
                );
                assert!(
                    settings.staging_directory.value().is_none(),
                    "staging_directory should default to None",
                );
            });
        });
    }

    #[test]
    fn test_prune_on_remove_default_is_false() {
        App::test((), |mut app| async move {
            app.update(init_preferences);
            app.add_singleton_model(|_| SettingsManager::default());
            WorktreeManagerSettings::register(&mut app);

            app.read(|ctx| {
                let settings = WorktreeManagerSettings::as_ref(ctx);
                assert_eq!(
                    *settings.prune_on_remove.value(),
                    PruneOnRemove::default_value(),
                );
                assert!(
                    !*settings.prune_on_remove.value(),
                    "prune_on_remove should default to false",
                );
            });
        });
    }
}
