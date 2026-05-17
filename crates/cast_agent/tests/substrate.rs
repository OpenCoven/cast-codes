//! Verify that `CastAgentRuntime` correctly overlays the host-pushed
//! [`HostSubstrate`] on top of the cast_agent-collected base
//! ([`Substrate`]) when building substrate snapshots for the gateway.
//!
//! The test never touches the actual `runtime::global()` singleton —
//! it constructs a fresh `CastAgentRuntime` from a default
//! `CastAgentConfig` so it can run in isolation. The gateway is never
//! reached because `build_substrate` only needs the local-side state
//! (shell CWD, git branch, host snapshot, Comux); the unreachable
//! `http://localhost:3000` default just turns the health probe amber.

use std::path::PathBuf;

use cast_agent::{
    config::CastAgentConfig,
    runtime::CastAgentRuntime,
    substrate::{DiagnosticEntry, DiagnosticSeverity, HostSubstrate, PaneInfo},
};

/// Install the workspace's rustls `CryptoProvider` exactly once per test
/// process. Required because `GatewayClient::new` (called transitively
/// from `CastAgentRuntime::boot`) builds a `reqwest::Client` that needs
/// the provider; production installs it in `app/src/lib.rs::run`.
fn install_crypto_provider_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

#[test]
fn host_substrate_starts_empty_and_overlays_on_build() {
    install_crypto_provider_once();

    // Boot a real runtime (its inner tokio runtime + health probe spawn,
    // but the unreachable gateway just leaves `is_available` false).
    let runtime = CastAgentRuntime::new_isolated(Some(CastAgentConfig::default()))
        .expect("runtime boots");

    // Fresh runtime: host substrate is `Default::default()`.
    let empty = runtime.host_substrate();
    assert!(empty.active_file.is_none());
    assert!(empty.open_panes.is_empty());
    assert!(empty.recent_errors.is_empty());

    // Build before any host push: cast_agent-owned fields are populated;
    // host-owned fields are still empty.
    let base = runtime
        .handle()
        .block_on(runtime.build_substrate())
        .expect("build substrate");
    assert!(base.active_file.is_none());
    assert!(base.open_panes.is_empty());
    assert!(base.recent_errors.is_empty());
    assert!(!base.shell_cwd.as_os_str().is_empty());

    // Push a host snapshot.
    let host = HostSubstrate {
        active_file: Some(PathBuf::from("/tmp/example.rs")),
        open_panes: vec![PaneInfo {
            id: "pane-1".into(),
            title: "zsh".into(),
            cwd: PathBuf::from("/tmp"),
            active: true,
        }],
        recent_errors: vec![DiagnosticEntry {
            file: PathBuf::from("/tmp/example.rs"),
            line: 42,
            severity: DiagnosticSeverity::Error,
            message: "unused variable: `x`".into(),
        }],
    };
    runtime.set_host_substrate(host.clone());

    // Snapshot reflects the push.
    let pushed = runtime.host_substrate();
    assert_eq!(pushed.active_file, host.active_file);
    assert_eq!(pushed.open_panes.len(), 1);
    assert_eq!(pushed.recent_errors.len(), 1);

    // Build now overlays the host fields on top of the cast_agent base.
    let merged = runtime
        .handle()
        .block_on(runtime.build_substrate())
        .expect("build substrate post-push");
    assert_eq!(merged.active_file, host.active_file);
    assert_eq!(merged.open_panes.len(), 1);
    assert_eq!(merged.open_panes[0].id, "pane-1");
    assert_eq!(merged.recent_errors.len(), 1);
    assert_eq!(merged.recent_errors[0].line, 42);
    // The cast_agent-owned shell_cwd is preserved through the merge.
    assert_eq!(merged.shell_cwd, base.shell_cwd);
}

#[test]
fn update_host_substrate_patches_one_field_without_clobbering_others() {
    install_crypto_provider_once();

    let runtime = CastAgentRuntime::new_isolated(Some(CastAgentConfig::default()))
        .expect("runtime boots");

    // Seed open_panes and recent_errors first so we have something to lose.
    runtime.set_host_substrate(HostSubstrate {
        active_file: None,
        open_panes: vec![PaneInfo {
            id: "p1".into(),
            title: "zsh".into(),
            cwd: PathBuf::from("/tmp"),
            active: true,
        }],
        recent_errors: vec![DiagnosticEntry {
            file: PathBuf::from("/tmp/a.rs"),
            line: 1,
            severity: DiagnosticSeverity::Warning,
            message: "warn".into(),
        }],
    });

    // Patch just active_file via update_host_substrate — simulates the
    // call ActiveFileModel::active_file_changed makes in production.
    runtime.update_host_substrate(|h| {
        h.active_file = Some(PathBuf::from("/tmp/focused.rs"));
    });

    let after = runtime.host_substrate();
    assert_eq!(
        after.active_file.as_deref(),
        Some(std::path::Path::new("/tmp/focused.rs"))
    );
    assert_eq!(after.open_panes.len(), 1, "panes should be preserved");
    assert_eq!(after.recent_errors.len(), 1, "errors should be preserved");
}

#[test]
fn pane_info_carries_terminal_cwd_through_build_substrate() {
    install_crypto_provider_once();

    let runtime = CastAgentRuntime::new_isolated(Some(CastAgentConfig::default()))
        .expect("runtime boots");

    // Simulate the post-#38-enrichment shape: each pane carries its
    // terminal session's CWD. The gateway should see the same CWDs back
    // through `build_substrate` after the host pushes them.
    runtime.set_host_substrate(HostSubstrate {
        active_file: None,
        open_panes: vec![
            PaneInfo {
                id: "tab-0".into(),
                title: "zsh ~/proj/a".into(),
                cwd: PathBuf::from("/home/u/proj/a"),
                active: true,
            },
            PaneInfo {
                id: "tab-1".into(),
                title: "zsh /tmp".into(),
                cwd: PathBuf::from("/tmp"),
                active: false,
            },
        ],
        recent_errors: Vec::new(),
    });

    let built = runtime
        .handle()
        .block_on(runtime.build_substrate())
        .expect("build substrate");
    assert_eq!(built.open_panes.len(), 2);
    assert_eq!(built.open_panes[0].cwd, PathBuf::from("/home/u/proj/a"));
    assert_eq!(built.open_panes[1].cwd, PathBuf::from("/tmp"));
}

#[test]
fn update_host_substrate_path_replaces_recent_errors() {
    install_crypto_provider_once();

    let runtime = CastAgentRuntime::new_isolated(Some(CastAgentConfig::default()))
        .expect("runtime boots");

    let path_a = PathBuf::from("/repo/a.rs");
    let path_b = PathBuf::from("/repo/b.rs");

    // First publish: file A has 2 errors, file B has 1 error.
    runtime.update_host_substrate(|h| {
        h.recent_errors.extend([
            DiagnosticEntry {
                file: path_a.clone(),
                line: 10,
                severity: DiagnosticSeverity::Error,
                message: "unused".into(),
            },
            DiagnosticEntry {
                file: path_a.clone(),
                line: 20,
                severity: DiagnosticSeverity::Warning,
                message: "shadowed".into(),
            },
            DiagnosticEntry {
                file: path_b.clone(),
                line: 5,
                severity: DiagnosticSeverity::Error,
                message: "syntax".into(),
            },
        ]);
    });

    // Second publish: file A now has only 1 entry. Path-scoped replacement
    // should drop A's old 2 entries before appending the new 1 — file B
    // stays untouched.
    let path_a_for_replace = path_a.clone();
    let path_b_for_assert = path_b.clone();
    runtime.update_host_substrate(move |h| {
        h.recent_errors.retain(|e| e.file != path_a_for_replace);
        h.recent_errors.push(DiagnosticEntry {
            file: path_a_for_replace.clone(),
            line: 10,
            severity: DiagnosticSeverity::Error,
            message: "unused".into(),
        });
    });

    let after = runtime.host_substrate();
    let from_a: Vec<_> = after
        .recent_errors
        .iter()
        .filter(|e| e.file == path_a)
        .collect();
    let from_b: Vec<_> = after
        .recent_errors
        .iter()
        .filter(|e| e.file == path_b_for_assert)
        .collect();
    assert_eq!(from_a.len(), 1, "file A replaced from 2 entries to 1");
    assert_eq!(from_a[0].message, "unused");
    assert_eq!(from_b.len(), 1, "file B untouched by file A's replacement");
    assert_eq!(from_b[0].message, "syntax");
}

#[test]
fn update_host_substrate_replaces_open_panes_atomically() {
    install_crypto_provider_once();

    let runtime = CastAgentRuntime::new_isolated(Some(CastAgentConfig::default()))
        .expect("runtime boots");

    // Seed an active_file and an initial open_panes list, then replace
    // open_panes wholesale — simulates how `Workspace::publish_open_panes_to_cast_agent`
    // patches the host substrate when tabs change. `active_file` must
    // survive untouched.
    runtime.set_host_substrate(HostSubstrate {
        active_file: Some(PathBuf::from("/repo/src/main.rs")),
        open_panes: vec![PaneInfo {
            id: "tab-0".into(),
            title: "old".into(),
            cwd: PathBuf::new(),
            active: true,
        }],
        recent_errors: Vec::new(),
    });

    runtime.update_host_substrate(|h| {
        h.open_panes = vec![
            PaneInfo {
                id: "tab-0".into(),
                title: "zsh".into(),
                cwd: PathBuf::new(),
                active: false,
            },
            PaneInfo {
                id: "tab-1".into(),
                title: "main.rs".into(),
                cwd: PathBuf::new(),
                active: true,
            },
        ];
    });

    let after = runtime.host_substrate();
    assert_eq!(after.open_panes.len(), 2, "open_panes replaced wholesale");
    assert_eq!(after.open_panes[0].title, "zsh");
    assert_eq!(after.open_panes[1].title, "main.rs");
    assert!(after.open_panes[1].active, "tab-1 is the active tab now");
    assert_eq!(
        after.active_file.as_deref(),
        Some(std::path::Path::new("/repo/src/main.rs")),
        "active_file slice survives the open_panes patch"
    );
}
