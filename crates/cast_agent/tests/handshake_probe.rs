//! End-to-end `coven.daemon.v1` handshake over a real Unix socket.
//!
//! `handshake::classify_health_response` is unit-tested in isolation; these
//! tests prove the full transport path enforces it: `GatewayClient` speaks
//! HTTP/1.1 to an in-process stub daemon's socket, runs `health_probe()`,
//! and the cached `connection_state()` / `is_available()` reflect the
//! daemon's `GET /api/v1/health` answer:
//!
//! - contract-compliant health → `Ready` with cached capabilities;
//! - wrong `apiVersion` → `Incompatible`, recovering to `Ready` on the
//!   next probe once the daemon speaks the contract (the 30 s loop's
//!   update-and-retry semantics);
//! - missing socket / non-2xx health → `Unreachable`.

#![cfg(unix)]

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use cast_agent::{
    config::CastAgentConfig, gateway::GatewayClient, runtime::CastAgentRuntime, ConnectionState,
    DAEMON_API_VERSION,
};

/// A `GET /api/v1/health` body that satisfies the `coven.daemon.v1`
/// contract, with a representative capabilities block.
fn contract_health_body() -> String {
    serde_json::json!({
        "ok": true,
        "apiVersion": DAEMON_API_VERSION,
        "covenVersion": "0.1.6",
        "capabilities": {
            "sessions": true,
            "events": true,
            "eventCursor": "sequence",
            "structuredErrors": true
        },
        "daemon": { "pid": 4242, "socket": "/tmp/coven.sock" }
    })
    .to_string()
}

/// Serve one connection: read the request head (health probes carry no
/// body), then answer with `status` + the body picked by `nth_body` for
/// this connection's ordinal. `Connection: close` + EOF matches what the
/// `unix_http` client expects.
async fn handle_conn(
    mut stream: UnixStream,
    ordinal: usize,
    status: u16,
    nth_body: fn(usize) -> String,
) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut tmp).await {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    }

    let body = nth_body(ordinal);
    let resp = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.flush().await;
}

/// Bind a stub daemon on a fresh socket and serve every connection with
/// `status` + `nth_body(connection_ordinal)`. Returns the socket path and
/// the serve task (abort it when done).
fn spawn_stub_daemon(
    tag: &str,
    status: u16,
    nth_body: fn(usize) -> String,
) -> (PathBuf, tokio::task::JoinHandle<()>) {
    let socket_path = std::env::temp_dir().join(format!(
        "cast_agent_handshake_{tag}_{}.sock",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path).expect("bind stub daemon socket");

    let server = tokio::spawn(async move {
        let conns = Arc::new(AtomicUsize::new(0));
        while let Ok((conn, _)) = listener.accept().await {
            let ordinal = conns.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(handle_conn(conn, ordinal, status, nth_body));
        }
    });
    (socket_path, server)
}

fn client_for(socket_path: &std::path::Path) -> GatewayClient {
    let cfg = CastAgentConfig {
        socket_path: Some(socket_path.to_path_buf()),
        ..CastAgentConfig::default()
    };
    GatewayClient::new(Arc::new(cfg))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn probe_against_contract_daemon_is_ready_with_capabilities() {
    let (socket_path, server) = spawn_stub_daemon("ready", 200, |_| contract_health_body());
    let client = client_for(&socket_path);

    assert_eq!(
        client.connection_state(),
        ConnectionState::Unknown,
        "no probe has run yet"
    );
    assert!(!client.is_available(), "Unknown must read as unavailable");

    client.health_probe().await;

    let state = client.connection_state();
    let ConnectionState::Ready(health) = state else {
        panic!("expected Ready after a contract handshake, got {state:?}");
    };
    assert_eq!(health.api_version.as_deref(), Some(DAEMON_API_VERSION));
    assert_eq!(health.coven_version.as_deref(), Some("0.1.6"));
    assert!(health.capabilities.sessions, "capabilities must be cached");
    assert_eq!(health.capabilities.event_cursor, "sequence");
    assert!(client.is_available());

    server.abort();
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn probe_against_wrong_version_is_incompatible_until_daemon_updates() {
    // First connection answers with a future contract version, every
    // subsequent one with the compliant body — an "update Coven" flow.
    let (socket_path, server) = spawn_stub_daemon("version", 200, |ordinal| {
        if ordinal == 0 {
            r#"{ "ok": true, "apiVersion": "coven.daemon.v2" }"#.to_string()
        } else {
            contract_health_body()
        }
    });
    let client = client_for(&socket_path);

    client.health_probe().await;
    assert_eq!(
        client.connection_state(),
        ConnectionState::Incompatible {
            api_version: "coven.daemon.v2".to_string()
        },
        "a 2xx health answer with the wrong apiVersion must not read as ready"
    );
    assert!(!client.is_available());

    // The daemon now speaks the contract: the next loop probe recovers.
    client.health_probe().await;
    assert!(
        client.connection_state().is_ready(),
        "probe must overwrite Incompatible once the daemon speaks {DAEMON_API_VERSION:?}"
    );
    assert!(client.is_available());

    server.abort();
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn probe_against_missing_socket_is_unreachable() {
    let socket_path = std::env::temp_dir().join(format!(
        "cast_agent_handshake_absent_{}.sock",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&socket_path);

    let client = client_for(&socket_path);
    client.health_probe().await;

    assert_eq!(client.connection_state(), ConnectionState::Unreachable);
    assert!(!client.is_available());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn probe_against_erroring_daemon_is_unreachable() {
    let (socket_path, server) = spawn_stub_daemon("http500", 500, |_| "{}".to_string());
    let client = client_for(&socket_path);

    client.health_probe().await;
    assert_eq!(
        client.connection_state(),
        ConnectionState::Unreachable,
        "non-2xx health means the daemon is not serving its contract"
    );
    assert!(!client.is_available());

    server.abort();
    let _ = std::fs::remove_file(&socket_path);
}

/// Serve every connection with a per-path response and count hits per
/// path prefix. Unlike [`spawn_stub_daemon`], this parses the request
/// line so tests can assert which endpoints were (not) called.
fn spawn_path_counting_daemon(
    tag: &str,
    health_body: &'static str,
) -> (
    PathBuf,
    tokio::task::JoinHandle<()>,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let socket_path = std::env::temp_dir().join(format!(
        "cast_agent_handshake_{tag}_{}.sock",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path).expect("bind stub daemon socket");

    let health_hits = Arc::new(AtomicUsize::new(0));
    let other_hits = Arc::new(AtomicUsize::new(0));
    let (health_ctr, other_ctr) = (health_hits.clone(), other_hits.clone());

    let server = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let (health_ctr, other_ctr) = (health_ctr.clone(), other_ctr.clone());
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut tmp).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => buf.extend_from_slice(&tmp[..n]),
                    }
                }
                let head = String::from_utf8_lossy(&buf);
                let path = head
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("")
                    .to_string();
                let body = if path.starts_with("/api/v1/health") {
                    health_ctr.fetch_add(1, Ordering::SeqCst);
                    health_body.to_string()
                } else {
                    other_ctr.fetch_add(1, Ordering::SeqCst);
                    r#"{"sessions":[]}"#.to_string()
                };
                let resp = format!(
                    "HTTP/1.1 200 X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len(),
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.flush().await;
            });
        }
    });
    (socket_path, server, health_hits, other_hits)
}

/// The handshake is mandatory before any other request: while the daemon
/// is off-contract, the runtime's background refresh loop must keep
/// probing `GET /api/v1/health` and never touch `/api/v1/sessions` or the
/// familiar catalog. Plain `#[test]`: `new_isolated` owns its runtime and
/// `Handle::block_on` panics inside another tokio context, so the stub
/// gets a dedicated runtime instead.
#[test]
fn refresh_loop_never_polls_sessions_while_daemon_is_off_contract() {
    let stub_rt = tokio::runtime::Runtime::new().expect("stub runtime");
    let (socket_path, server, health_hits, other_hits) = {
        let _guard = stub_rt.enter();
        spawn_path_counting_daemon(
            "gating",
            r#"{ "ok": true, "apiVersion": "coven.daemon.v2" }"#,
        )
    };

    let cfg = CastAgentConfig {
        socket_path: Some(socket_path.clone()),
        ..CastAgentConfig::default()
    };
    let runtime = CastAgentRuntime::new_isolated(Some(cfg)).expect("boot isolated runtime");

    // The initial refresh cycle runs immediately at boot; give it (and the
    // boot-time probe) ample time to land before asserting.
    std::thread::sleep(std::time::Duration::from_millis(1500));

    assert!(
        health_hits.load(Ordering::SeqCst) >= 1,
        "the refresh loop must keep running the handshake probe"
    );
    assert_eq!(
        other_hits.load(Ordering::SeqCst),
        0,
        "no session/familiar request may be issued while the daemon is Incompatible"
    );
    assert!(!runtime.is_available());
    assert!(runtime.sessions().is_empty());

    server.abort();
    let _ = std::fs::remove_file(&socket_path);
}
