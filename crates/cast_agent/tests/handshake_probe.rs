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
    config::CastAgentConfig, gateway::GatewayClient, ConnectionState, DAEMON_API_VERSION,
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
