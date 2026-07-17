//! Streaming chat over the Unix daemon transport, verified against an
//! in-process stub daemon.
//!
//! The stub speaks the minimal HTTP/1.1-over-Unix-socket surface that
//! `cast_agent`'s `unix_http` client expects (Connection: close, the
//! client reads to EOF), and drives one chat turn through the session
//! lifecycle the streaming path relies on:
//!
//! - `POST /api/v1/sessions` → returns a running session record.
//! - `GET  /api/v1/events?...&afterSeq=0` → two `output` events.
//! - `GET  /api/v1/sessions/<id>` → `running` on the first status poll,
//!   `completed` on the second (so the stream sees a terminal flip).
//!
//! The assertion is that `stream_messages` surfaces each `output` event
//! as a `Delta` as it arrives and ends with a single `Done`.

#![cfg(unix)]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use cast_agent::{
    agent::AgentMessage,
    config::CastAgentConfig,
    gateway::{GatewayClient, MessageChunk},
};

fn events_after_seq_zero() -> String {
    serde_json::json!({
        "events": [
            { "seq": 1, "kind": "output", "payload_json": "{\"data\":\"Hello \"}" },
            { "seq": 2, "kind": "output", "payload_json": "{\"data\":\"world\"}" },
        ],
        "hasMore": false
    })
    .to_string()
}

fn empty_events() -> String {
    r#"{"events":[],"hasMore":false}"#.to_string()
}

async fn handle_conn(mut stream: UnixStream, status_polls: Arc<AtomicUsize>) {
    // Read the request headers, then drain the Content-Length body. The
    // client sends `Connection: close` and then reads to EOF, so it never
    // half-closes its write side — we must not `read_to_end` (deadlock).
    // But we MUST consume the POST body before responding + closing: if we
    // close while the client is still writing its body, the client's write
    // fails with ECONNRESET. That raced favourably locally but reset under
    // CI's heavy parallelism.
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        match stream.read(&mut tmp).await {
            Ok(0) => break buf.len(),
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break buf.len(),
        }
    };

    // Parse Content-Length (case-insensitive) and read the remaining body
    // so the client's `write_all(body)` completes before we close.
    let content_length: usize = String::from_utf8_lossy(&buf[..header_end.min(buf.len())])
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            lower
                .strip_prefix("content-length:")
                .and_then(|v| v.trim().parse().ok())
        })
        .unwrap_or(0);
    while buf.len() < header_end + content_length {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }

    let head = String::from_utf8_lossy(&buf);
    let req_line = head.lines().next().unwrap_or("");
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    let body = if method == "POST" && path == "/api/v1/sessions" {
        r#"{"id":"sess-1","status":"running","exit_code":null}"#.to_string()
    } else if path.starts_with("/api/v1/events") {
        if path.contains("afterSeq=0") {
            events_after_seq_zero()
        } else {
            empty_events()
        }
    } else if path.starts_with("/api/v1/sessions/sess-1") {
        // First poll: still running. Second poll onward: terminal.
        let n = status_polls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            r#"{"id":"sess-1","status":"running","exit_code":null}"#.to_string()
        } else {
            r#"{"id":"sess-1","status":"completed","exit_code":0}"#.to_string()
        }
    } else {
        "{}".to_string()
    };

    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.flush().await;
    // Dropping `stream` closes the connection, giving the client its EOF.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn streams_daemon_output_incrementally_until_done() {
    let socket_path = std::env::temp_dir().join(format!(
        "cast_agent_stream_stub_{}.sock",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path).expect("bind stub daemon socket");

    let status_polls = Arc::new(AtomicUsize::new(0));
    let accept_polls = status_polls.clone();
    let server = tokio::spawn(async move {
        while let Ok((conn, _)) = listener.accept().await {
            let polls = accept_polls.clone();
            tokio::spawn(handle_conn(conn, polls));
        }
    });

    let cfg = CastAgentConfig {
        socket_path: Some(socket_path.clone()),
        ..CastAgentConfig::default()
    };
    let client = GatewayClient::new(Arc::new(cfg));

    let msg = AgentMessage {
        conversation_id: "conv-ignored".to_string(),
        body: serde_json::json!({ "prompt": "hi", "harness": "coven-code" }),
    };

    let mut stream = client
        .stream_messages(msg)
        .await
        .expect("stream_messages should launch the daemon session");

    let mut deltas = String::new();
    let mut done = false;
    let mut delta_count = 0usize;
    while let Some(chunk) = stream.next().await {
        match chunk.expect("stream chunk should not error") {
            MessageChunk::Delta { content, .. } => {
                delta_count += 1;
                deltas.push_str(&content);
            }
            MessageChunk::Done { .. } => {
                done = true;
                break;
            }
            MessageChunk::Error { message, .. } => panic!("unexpected stream error: {message}"),
        }
    }

    server.abort();
    let _ = std::fs::remove_file(&socket_path);

    assert_eq!(deltas, "Hello world", "deltas should concatenate in order");
    assert_eq!(
        delta_count, 2,
        "each output event should arrive as its own delta"
    );
    assert!(done, "stream should terminate with a Done chunk");
}
