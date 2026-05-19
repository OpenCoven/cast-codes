//! Cross-server diagnostics collector for the Cast Agent host substrate.
//!
//! Closes the coverage gap left by
//! [`LocalCodeEditorView::publish_diagnostics_to_cast_agent`](super::language_server_extension):
//! that publisher only fires for files currently open in a code editor.
//! The collector here listens to every [`LspServerModel`] running in
//! the app (via [`LspManagerModel`]'s `ServerStarted` events) and pushes
//! diagnostics to the cast_agent host substrate as they arrive, regardless
//! of whether the file is open in the UI. The gateway therefore sees every
//! active LSP error/warning, not just the ones the user is currently
//! looking at.
//!
//! Implementation: a singleton model that subscribes to `LspManagerModel`
//! events at app startup; for each new `LspServerModel`, it chain-subscribes
//! to `LspEvent::DiagnosticsUpdated` and re-applies the same path-scoped
//! replacement strategy used by the per-editor publisher (drop existing
//! entries for that path, append the new ones, cap to 50 globally).
//!
//! Both publishers can fire for the same file when it's open in a code
//! editor. That's intentional: both call
//! `cast_agent::update_host_substrate` with the same path-replacement
//! pattern, so the second call just overwrites the first's entries with
//! identical content. Idempotent; safe; no deduplication needed.

#![cfg(feature = "cast-agent")]

use std::collections::HashSet;

use lsp::{LanguageServerId, LspEvent, LspManagerModel, LspManagerModelEvent, LspServerModel};
use warpui::{AppContext, Entity, ModelContext, ModelHandle, SingletonEntity};

/// Eagerly instantiate the collector at app startup. Idempotent —
/// `add_singleton_model` only runs the builder once per type.
pub fn init(app: &mut AppContext) {
    app.add_singleton_model(|ctx| CastAgentDiagnosticsCollector::new(ctx));
}

#[derive(Default)]
pub struct CastAgentDiagnosticsCollector {
    /// Server ids we've already subscribed to. Prevents duplicate
    /// subscriptions when `LspManagerModel` emits `ServerStarted` for a
    /// server we already know about (e.g. on re-start, re-register).
    subscribed: HashSet<LanguageServerId>,
}

impl Entity for CastAgentDiagnosticsCollector {
    type Event = ();
}

impl SingletonEntity for CastAgentDiagnosticsCollector {}

impl CastAgentDiagnosticsCollector {
    fn new(ctx: &mut ModelContext<Self>) -> Self {
        let mut me = Self::default();

        let lsp_manager = LspManagerModel::handle(ctx);

        // 1) Catch servers that already exist when the collector boots.
        //    `code::init` may run after some workspace's LSP servers have
        //    already been registered, especially during session restore.
        let existing_servers: Vec<ModelHandle<LspServerModel>> = {
            let mgr = lsp_manager.as_ref(ctx);
            mgr.workspace_roots()
                .filter_map(|root| mgr.servers_for_workspace(root).map(|s| s.clone()))
                .flatten()
                .collect()
        };
        for server in existing_servers {
            me.subscribe_to_server(server, ctx);
        }

        // 2) Subscribe to manager events for new servers. The
        //    `ModelContext::subscribe_to_model` closure receives only
        //    `(me, event, ctx)`; capture a clone of the manager handle so
        //    we can re-resolve `servers_for_workspace(path)` inside the
        //    callback.
        let manager_for_callback = lsp_manager.clone();
        ctx.subscribe_to_model(&lsp_manager, move |me, event, ctx| {
            let LspManagerModelEvent::ServerStarted(path) = event else {
                return;
            };
            let new_servers: Vec<ModelHandle<LspServerModel>> = manager_for_callback
                .as_ref(ctx)
                .servers_for_workspace(path)
                .map(|s| s.clone())
                .unwrap_or_default();
            for server in new_servers {
                me.subscribe_to_server(server, ctx);
            }
        });

        me
    }

    fn subscribe_to_server(
        &mut self,
        server: ModelHandle<LspServerModel>,
        ctx: &mut ModelContext<Self>,
    ) {
        let id = server.as_ref(ctx).id();
        if !self.subscribed.insert(id) {
            return;
        }
        let server_for_callback = server.clone();
        ctx.subscribe_to_model(&server, move |_me, event, ctx| {
            if let LspEvent::DiagnosticsUpdated { path } = event {
                push_diagnostics(&server_for_callback, path, ctx);
            }
        });
    }
}

/// Convert the LSP server's raw diagnostics for `path` into
/// `cast_agent::DiagnosticEntry`s (Error+Warning only) and push them via
/// `update_host_substrate` with path-scoped replacement. Mirrors
/// [`LocalCodeEditorView::publish_diagnostics_to_cast_agent`](super::language_server_extension).
fn push_diagnostics(
    server: &ModelHandle<LspServerModel>,
    path: &std::path::Path,
    ctx: &AppContext,
) {
    const RECENT_ERRORS_MAX: usize = 50;

    let path_buf = path.to_path_buf();
    let entries: Vec<::ai::cast_agent::DiagnosticEntry> =
        match server.as_ref(ctx).diagnostics_for_path(path).ok().flatten() {
            Some(doc) => doc
                .diagnostics
                .iter()
                .filter_map(|d| {
                    let severity = match d.severity {
                        Some(lsp_types::DiagnosticSeverity::ERROR) => {
                            ::ai::cast_agent::DiagnosticSeverity::Error
                        }
                        Some(lsp_types::DiagnosticSeverity::WARNING) => {
                            ::ai::cast_agent::DiagnosticSeverity::Warning
                        }
                        _ => return None,
                    };
                    Some(::ai::cast_agent::DiagnosticEntry {
                        file: path_buf.clone(),
                        line: d.range.start.line,
                        severity,
                        message: d.message.clone(),
                    })
                })
                .collect(),
            None => Vec::new(),
        };

    let path_for_closure = path_buf;
    ::ai::cast_agent::update_host_substrate(move |host| {
        host.recent_errors.retain(|e| e.file != path_for_closure);
        host.recent_errors.extend(entries);
        let len = host.recent_errors.len();
        if len > RECENT_ERRORS_MAX {
            host.recent_errors.drain(0..len - RECENT_ERRORS_MAX);
        }
    });
}
