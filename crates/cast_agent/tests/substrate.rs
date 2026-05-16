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
