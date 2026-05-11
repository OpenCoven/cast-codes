//! Module for utilities related to editing items in the file tree.

#[cfg(test)]
#[path = "editing_tests.rs"]
mod tests;

use repo_metadata::file_tree_store::{FileTreeDirectoryEntryState, FileTreeEntryState};
use repo_metadata::{FileMetadata, FileTreeEntry};
use std::cmp::Ordering;
use std::sync::Arc;
use warp_util::standardized_path::StandardizedPath;
use warpui::{elements::MouseStateHandle, SingletonEntity as _, ViewContext};

use super::{FileTreeIdentifier, FileTreeItem, FileTreeView};
use crate::{
    code::file_tree::{
        view::{PendingEdit, PendingEditKind},
        FileTreeEvent,
    },
    send_telemetry_from_ctx,
    server::telemetry::TelemetryEvent,
};

/// Custom ordering function for items in the file tree.
///
/// Directories are ordered first, sorted alphabetically.
/// Files are ordered second, sorted alphabetically.
/// Within each group, dotfiles (entries starting with a dot) are ordered first.
pub(super) fn sort_entries_for_file_tree(
    entry_1: &StandardizedPath,
    entry_2: &StandardizedPath,
    entry_map: &FileTreeEntry,
) -> Ordering {
    use std::cmp::Ordering;

    // Entries missing from the map sort before present entries, and compare
    // equal to each other. Using the same `Ordering` on both sides would
    // violate antisymmetry and cause `sorted_by` to panic with
    // "user-provided comparison function does not correctly implement a total order".
    let (entry_1, entry_2) = match (entry_map.get(entry_1), entry_map.get(entry_2)) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Less,
        (Some(_), None) => return Ordering::Greater,
        (Some(e1), Some(e2)) => (e1, e2),
    };

    let is_dir_1 = matches!(entry_1, FileTreeEntryState::Directory(_));
    let is_dir_2 = matches!(entry_2, FileTreeEntryState::Directory(_));

    // Order directories before any files.
    match (is_dir_1, is_dir_2) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        // Both are same type, continue with alphabetical sort.
        _ => {}
    }

    // Same antisymmetry requirement for missing file names.
    let (name_1, name_2) = match (entry_1.path().file_name(), entry_2.path().file_name()) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Less,
        (Some(_), None) => return Ordering::Greater,
        (Some(n1), Some(n2)) => (n1, n2),
    };

    let starts_with_dot_1 = name_1.starts_with('.');
    let starts_with_dot_2 = name_2.starts_with('.');

    // Items starting with "." come first.
    match (starts_with_dot_1, starts_with_dot_2) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => name_1.cmp(name_2),
    }
}

impl FileTreeView {
    /// Creates a new file below the directory at the given identifier.
    pub(super) fn create_new_file(&mut self, id: &FileTreeIdentifier, ctx: &mut ViewContext<Self>) {
        let Some(root_dir) = self.root_directories.get_mut(&id.root) else {
            return;
        };
        let (path, depth) = match root_dir.items.get(id.index) {
            Some(FileTreeItem::File { .. }) => {
                log::warn!("Cannot create a new file below a file");
                return;
            }
            Some(FileTreeItem::DirectoryHeader {
                directory, depth, ..
            }) => (directory.path.clone(), *depth),
            _ => return,
        };

        // Ensure the parent directory is expanded before creating a file beneath it.
        if !self.is_folder_expanded(&id.root, &path) {
            self.toggle_folder_expansion(&id.root, &path, ctx);
        }

        // Create a dummy FileTreeItem for the file we are about to create--we'll replace
        // this with something real once the user types in the actual file.
        let new_item_index = id.index + 1;
        let Some(root_dir) = self.root_directories.get_mut(&id.root) else {
            return;
        };
        root_dir.items.insert(
            new_item_index,
            FileTreeItem::File {
                metadata: FileMetadata::from_standardized(path.join("new_file"), false).into(),
                depth: depth + 1,
                mouse_state_handle: MouseStateHandle::default(),
                draggable_state: warpui::elements::DraggableState::default(),
            },
        );

        // Ensure the new item we just created is selected.
        let new_id = FileTreeIdentifier {
            root: id.root.clone(),
            index: new_item_index,
        };
        self.select_id(&new_id, ctx);

        // Ensure the editor is focused.
        ctx.focus(&self.editor_view);
        self.pending_edit = Some(PendingEdit {
            id: new_id,
            kind: PendingEditKind::CreateNewFile,
        });
    }

    /// Creates a new folder below the directory at the given identifier.
    pub(super) fn create_new_folder(
        &mut self,
        id: &FileTreeIdentifier,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(root_dir) = self.root_directories.get_mut(&id.root) else {
            return;
        };
        let (path, depth) = match root_dir.items.get(id.index) {
            Some(FileTreeItem::File { .. }) => {
                log::warn!("Cannot create a new folder below a file");
                return;
            }
            Some(FileTreeItem::DirectoryHeader {
                directory, depth, ..
            }) => (directory.path.clone(), *depth),
            _ => return,
        };

        // Ensure the parent directory is expanded before creating a folder beneath it.
        if !self.is_folder_expanded(&id.root, &path) {
            self.toggle_folder_expansion(&id.root, &path, ctx);
        }

        let new_item_index = id.index + 1;
        let Some(root_dir) = self.root_directories.get_mut(&id.root) else {
            return;
        };
        root_dir.items.insert(
            new_item_index,
            FileTreeItem::DirectoryHeader {
                directory: FileTreeDirectoryEntryState {
                    path: Arc::new(path.join("new_folder")),
                    ignored: false,
                    loaded: true,
                },
                depth: depth + 1,
                mouse_state_handle: MouseStateHandle::default(),
                draggable_state: warpui::elements::DraggableState::default(),
            },
        );

        let new_id = FileTreeIdentifier {
            root: id.root.clone(),
            index: new_item_index,
        };
        self.select_id(&new_id, ctx);

        ctx.focus(&self.editor_view);
        self.pending_edit = Some(PendingEdit {
            id: new_id,
            kind: PendingEditKind::CreateNewFolder,
        });
    }

    /// Starts a rename edit on the item at the given identifier.
    pub(super) fn start_rename(&mut self, id: &FileTreeIdentifier, ctx: &mut ViewContext<Self>) {
        let Some(root_dir) = self.root_directories.get(&id.root) else {
            return;
        };
        let Some(item) = root_dir.items.get(id.index) else {
            return;
        };
        // Prefill the editor with the current file or directory name.
        let current_name = item
            .path()
            .file_name()
            .map(|s| s.to_owned())
            .unwrap_or_default();

        self.pending_edit = Some(PendingEdit {
            id: id.clone(),
            kind: PendingEditKind::RenameExisting,
        });

        self.editor_view.update(ctx, |view, ctx| {
            view.set_buffer_text(&current_name, ctx);
        });
        ctx.focus(&self.editor_view);
    }

    /// Commits a pending edit to the file tree.
    pub(super) fn commit_pending_edit(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(pending_edit) = self.pending_edit.take() else {
            return;
        };

        let file_tree_id = pending_edit.id.clone();

        let buffer_content = self.editor_view.as_ref(ctx).buffer_text(ctx);
        self.editor_view.update(ctx, |view, ctx| {
            view.clear_buffer(ctx);
        });

        match pending_edit.kind {
            PendingEditKind::CreateNewFile => {
                if buffer_content.is_empty() {
                    self.remove_pending_create_placeholder(&file_tree_id, ctx);
                    return;
                }

                let Some(new_entry) = ({
                    let Some(root_dir) = self.root_directories.get_mut(&file_tree_id.root) else {
                        return;
                    };
                    let Some(item) = root_dir.items.get_mut(file_tree_id.index) else {
                        return;
                    };

                    if let FileTreeItem::File { metadata, .. } = item {
                        let mut new_std = (*metadata.path).clone();
                        new_std.set_file_name(&buffer_content);
                        let local_path = new_std.to_local_path_lossy();

                        if let Err(e) = std::fs::File::create_new(&local_path) {
                            log::warn!("Failed to create file: {e}");
                            None
                        } else {
                            metadata.path = Arc::new(new_std);

                            send_telemetry_from_ctx!(TelemetryEvent::FileTreeItemCreated, ctx);

                            Some(FileTreeEntryState::File(metadata.clone()))
                        }
                    } else {
                        None
                    }
                }) else {
                    self.remove_pending_create_placeholder(&file_tree_id, ctx);
                    return;
                };

                if let Some(root_dir) = self.root_directories.get_mut(&file_tree_id.root) {
                    // Ensure the file tree has the new item we've just created.
                    Self::insert_entry(&mut root_dir.entry, new_entry);
                }

                self.open_in_new_pane(&file_tree_id, ctx);
                self.rebuild_flattened_items();
            }
            PendingEditKind::CreateNewFolder => {
                if buffer_content.is_empty() {
                    self.remove_pending_create_placeholder(&file_tree_id, ctx);
                    return;
                }

                let mut created_path = None;
                let Some(new_entry) = ({
                    let Some(root_dir) = self.root_directories.get_mut(&file_tree_id.root) else {
                        return;
                    };
                    let Some(item) = root_dir.items.get_mut(file_tree_id.index) else {
                        return;
                    };

                    if let FileTreeItem::DirectoryHeader { directory, .. } = item {
                        let mut new_std = (*directory.path).clone();
                        new_std.set_file_name(&buffer_content);
                        let local_path = new_std.to_local_path_lossy();

                        if let Err(e) = std::fs::create_dir(&local_path) {
                            log::warn!("Failed to create folder: {e}");
                            None
                        } else {
                            directory.path = Arc::new(new_std.clone());
                            directory.loaded = true;
                            created_path = Some(new_std);

                            send_telemetry_from_ctx!(TelemetryEvent::FileTreeItemCreated, ctx);

                            Some(FileTreeEntryState::Directory(directory.clone()))
                        }
                    } else {
                        None
                    }
                }) else {
                    self.remove_pending_create_placeholder(&file_tree_id, ctx);
                    return;
                };

                #[cfg(feature = "local_fs")]
                if let Some(created_path) = created_path.as_ref() {
                    self.refresh_local_parent_directory_after_create(
                        &file_tree_id.root,
                        created_path,
                        ctx,
                    );
                }

                if let Some(root_dir) = self.root_directories.get_mut(&file_tree_id.root) {
                    Self::insert_entry(&mut root_dir.entry, new_entry);
                }

                self.rebuild_flattened_items();
                if let Some(created_path) = created_path {
                    if let Some(new_id) =
                        self.find_directory_header_id(&file_tree_id.root, &created_path)
                    {
                        self.select_id(&new_id, ctx);
                    }
                }
            }
            PendingEditKind::RenameExisting => {
                let Some(root_dir) = self.root_directories.get(&file_tree_id.root) else {
                    return;
                };
                let Some(item) = root_dir.items.get(file_tree_id.index) else {
                    return;
                };
                if buffer_content.is_empty() {
                    return;
                }
                let old_std_path = item.path().clone();
                let mut new_std_path = old_std_path.clone();
                new_std_path.set_file_name(&buffer_content);

                let old_path = old_std_path.to_local_path_lossy();
                let new_path = new_std_path.to_local_path_lossy();
                if let Err(e) = std::fs::rename(&old_path, &new_path) {
                    log::warn!(
                        "Failed to rename {} -> {}: {e}",
                        old_path.display(),
                        new_path.display()
                    );
                    return;
                }

                // Update the in-memory model immediately so the UI reflects the change without delay.
                if let Some(root_dir) = self.root_directories.get_mut(&file_tree_id.root) {
                    root_dir.entry.rename_path(&old_std_path, &new_std_path);
                }

                // Emit event to notify workspace that a file was renamed
                ctx.emit(FileTreeEvent::FileRenamed {
                    old_path: old_path.clone(),
                    new_path: new_path.clone(),
                });

                // Rebuild and select the renamed item using its FileTreeIdentifier
                self.rebuild_flatten_items_impl(Some(&file_tree_id), None, None);
                ctx.notify();
            }
        }
    }

    /// Cancels a pending edit and discards any changes.
    pub(super) fn cancel_pending_edit(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(pending_edit) = self.pending_edit.take() {
            let id = &pending_edit.id;
            if self.selected_item.as_ref() == Some(id) {
                self.selected_item = None;
            }
            self.editor_view.update(ctx, |view, ctx| {
                view.clear_buffer(ctx);
            });
            // Only remove placeholders in create flows.
            if matches!(
                pending_edit.kind,
                PendingEditKind::CreateNewFile | PendingEditKind::CreateNewFolder
            ) {
                self.remove_pending_create_placeholder(id, ctx);
            }
        }
        ctx.notify();
    }

    /// Inserts a new entry into the tree.
    fn insert_entry(root_entry: &mut FileTreeEntry, child_entry: FileTreeEntryState) {
        let Some(parent) = child_entry.path().parent() else {
            return;
        };

        if root_entry
            .insert_child_state(&parent, child_entry)
            .is_none()
        {
            log::warn!("Failed to insert file tree entry under parent: {parent}");
        }
    }

    #[cfg(feature = "local_fs")]
    fn refresh_local_parent_directory_after_create(
        &mut self,
        root_path: &StandardizedPath,
        created_path: &StandardizedPath,
        ctx: &mut ViewContext<Self>,
    ) {
        if self
            .root_directories
            .get(root_path)
            .is_some_and(|root_dir| root_dir.is_remote())
        {
            return;
        }

        let Some(parent_path) = created_path.parent() else {
            return;
        };
        let Some(backing_root) = self
            .root_directories
            .get(root_path)
            .map(|root_dir| root_dir.entry.root_directory().as_ref().clone())
        else {
            return;
        };

        let repository_id = repo_metadata::RepositoryIdentifier::local(backing_root.clone());
        if repo_metadata::RepoMetadataModel::as_ref(ctx)
            .get_repository(&repository_id, ctx)
            .is_none()
        {
            return;
        }

        let load_result = self.repository_metadata_model.update(
            ctx,
            |model: &mut repo_metadata::RepoMetadataModel, ctx| {
                model.load_directory(&backing_root, &parent_path, ctx)
            },
        );
        if let Err(error) = load_result {
            log::warn!("Failed to refresh parent directory after file tree create: {error}");
            return;
        }

        if let Some(state) =
            repo_metadata::RepoMetadataModel::as_ref(ctx).get_repository(&repository_id, ctx)
        {
            if let Some(root_dir) = self.root_directories.get_mut(root_path) {
                root_dir.entry = state.entry.clone();
            }
        }
    }

    fn remove_pending_create_placeholder(
        &mut self,
        id: &FileTreeIdentifier,
        ctx: &mut ViewContext<Self>,
    ) {
        if let Some(root_dir) = self.root_directories.get_mut(&id.root) {
            if id.index < root_dir.items.len() {
                root_dir.items.remove(id.index);
            }
        }
        if self.selected_item.as_ref() == Some(id) {
            self.selected_item = None;
        }
        ctx.notify();
    }

    pub(super) fn handle_pending_edit(&mut self, ctx: &mut ViewContext<Self>) {
        if self.pending_edit.is_none() {
            return;
        };

        let editor_contents = self.editor_view.as_ref(ctx).buffer_text(ctx);
        // If the editor is empty and the editor was dismissed, cancel the editor.
        // Otherwise commit the editor. This matches VSCode's behavior.
        if editor_contents.is_empty() {
            self.cancel_pending_edit(ctx);
        } else {
            self.commit_pending_edit(ctx);
        }
    }
}
