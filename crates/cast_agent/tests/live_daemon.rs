//! Live integration tests against the local Coven stack.
//!
//! These tests are skipped by default (`#[ignore]`) because they need a
//! running OpenCoven daemon (Unix socket at `~/.coven/coven.sock`) and
//! the CastCodes ↔ Coven gateway bridge on `127.0.0.1:3999`. Both come
//! up as launchd services on a typical developer machine.
//!
//! Invoke explicitly with:
//! ```text
//! cargo test -p cast_agent --test live_daemon -- --ignored --nocapture
//! ```
//!
//! By default, `CastAgentConfig::load()` uses the TCP transport (the
//! bridge at `:3999`). Set `COVEN_SOCKET=$HOME/.coven/coven.sock` if you
//! want to exercise the direct-Unix path instead — the assertions in
//! these tests are transport-agnostic.

use std::sync::Arc;

use futures::StreamExt;

use cast_agent::{
    agent::AgentMessage,
    config::CastAgentConfig,
    gateway::{GatewayClient, MessageChunk},
    session::{CovenSession, SessionStatus},
};

fn install_crypto_provider_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// List sessions through whatever transport the default config picks.
/// On a stock machine this exercises the bridge at `:3999`; with
/// `COVEN_SOCKET=...` set, it exercises the direct-Unix path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live coven daemon + bridge (or COVEN_SOCKET set)"]
async fn live_daemon_lists_real_sessions() {
    install_crypto_provider_once();

    let cfg = CastAgentConfig::load();
    let client = GatewayClient::new(Arc::new(cfg));
    client.health_probe().await;
    assert!(
        client.is_available(),
        "health probe should succeed against the live daemon/bridge"
    );

    let sessions: Vec<CovenSession> = client
        .list_sessions()
        .await
        .expect("list_sessions against live daemon");

    println!("live transport returned {} sessions", sessions.len());
    for s in sessions.iter().take(5) {
        println!(
            "  - {} [{:?}] {} (cwd: {:?})",
            &s.id[..8.min(s.id.len())],
            s.status,
            s.name,
            s.cwd
        );
    }

    assert!(
        !sessions.is_empty(),
        "live daemon has at least one historical session; got 0 — \
         daemon may be stale or bridge may be returning an empty list"
    );

    for s in &sessions {
        assert!(!s.id.is_empty(), "id must be non-empty");
        assert!(!s.name.is_empty(), "name must be derived (title/cwd/id)");
        assert!(
            matches!(
                s.status,
                SessionStatus::Active | SessionStatus::Idle | SessionStatus::Closed
            ),
            "status must be one of the three UI states"
        );
    }
}

/// Live end-to-end chat-send test: drives `send_message` against the
/// live stack. With the default TCP transport this goes through the
/// bridge at `:3999`, which forwards to the daemon's session lifecycle.
/// With `COVEN_SOCKET=...` set, it talks to the daemon directly.
///
/// **This costs LLM tokens.** Gated behind both `#[ignore]` and a
/// `CAST_AGENT_LIVE_WRITE=1` env var so it can't be triggered by a
/// blanket `--ignored` run.
///
/// Configure the harness and project_root via the message body
/// (`harness`, `projectRoot`, `title`). When omitted, the bridge falls
/// back to `CASTCODES_BRIDGE_HARNESS` / `CASTCODES_BRIDGE_PROJECT_ROOT`
/// or sensible defaults (`claude` / `$HOME`).
///
/// Invoke explicitly:
/// ```text
/// CAST_AGENT_LIVE_WRITE=1 \
///   cargo test -p cast_agent --test live_daemon \
///     send_message_runs_real_session -- --ignored --nocapture
/// ```
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live LLM session; requires CAST_AGENT_LIVE_WRITE=1"]
async fn send_message_runs_real_session() {
    install_crypto_provider_once();

    if std::env::var("CAST_AGENT_LIVE_WRITE").ok().as_deref() != Some("1") {
        eprintln!(
            "skipping: set CAST_AGENT_LIVE_WRITE=1 to opt in to a real LLM call \
             (this test spawns a harness and burns tokens)"
        );
        return;
    }

    let cfg = CastAgentConfig::load();
    println!(
        "transport: {} (socket={:?}, gateway_url={})",
        if cfg.socket_path.is_some() {
            "unix"
        } else {
            "tcp/bridge"
        },
        cfg.socket_path,
        cfg.gateway_url
    );

    let client = GatewayClient::new(Arc::new(cfg));
    client.health_probe().await;
    assert!(client.is_available(), "daemon/bridge must be reachable");

    // Allow env overrides so the test can target a specific harness +
    // project_root without modifying source. These flow into the message
    // body so they propagate via either transport.
    let harness = std::env::var("CAST_AGENT_HARNESS")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "codex".to_string());
    let project_root = std::env::var("CAST_AGENT_PROJECT_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "/tmp".to_string());

    let msg = AgentMessage {
        conversation_id: "cast-agent-live-smoke".into(),
        body: serde_json::json!({
            "prompt": "Reply with exactly the four characters: OK.",
            "title": "cast_agent live smoke",
            "harness": harness,
            "projectRoot": project_root,
        }),
    };

    let resp = client
        .send_message(msg)
        .await
        .expect("send_message against live daemon");

    // Print only an 8-char prefix — matches the read-path test's display
    // pattern and avoids CodeQL `rust/cleartext-logging` flagging the
    // full session uid even from gated test code.
    let id_prefix: String = resp
        .conversation_id
        .chars()
        .take(8.min(resp.conversation_id.len()))
        .collect();
    println!("daemon session id:  {id_prefix}\u{2026}");
    println!(
        "final status:       {}",
        resp.body
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
    );
    println!(
        "exit code:          {:?}",
        resp.body.get("exit_code").and_then(|v| v.as_i64())
    );
    let text = resp
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    println!("--- output (first 800 chars) ---");
    println!("{}", text.chars().take(800).collect::<String>());
    println!("--- end output ---");

    assert!(!resp.conversation_id.is_empty(), "daemon must assign an id");
    assert!(
        !text.is_empty(),
        "session produced no output events; harness may have failed to spawn \
         (check daemon PATH if the response is empty)"
    );
}

/// Live end-to-end streamed chat test against the bridge's
/// `WS /v1/messages/stream` endpoint. Gated identically to
/// `send_message_runs_real_session` (costs LLM tokens; opt in via
/// `CAST_AGENT_LIVE_WRITE=1`).
///
/// Verifies:
/// - The WS upgrade succeeds against the bridge on :3999.
/// - At least one `MessageChunk::Delta` arrives with non-empty content.
/// - The stream ends with `MessageChunk::Done` (or via clean WS close)
///   rather than an `Error` frame.
/// - The accumulated content of all Delta frames is non-empty.
///
/// Invoke explicitly:
/// ```text
/// CAST_AGENT_LIVE_WRITE=1 \
///   cargo test -p cast_agent --test live_daemon \
///     stream_messages_streams_real_session -- --ignored --nocapture
/// ```
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live LLM session over WS; requires CAST_AGENT_LIVE_WRITE=1"]
async fn stream_messages_streams_real_session() {
    install_crypto_provider_once();

    if std::env::var("CAST_AGENT_LIVE_WRITE").ok().as_deref() != Some("1") {
        eprintln!(
            "skipping: set CAST_AGENT_LIVE_WRITE=1 to opt in to a real LLM call \
             (this test spawns a harness and burns tokens)"
        );
        return;
    }

    // The bridge owns the WS endpoint; force the TCP transport even if
    // somebody has COVEN_SOCKET set in their shell.
    std::env::set_var("COVEN_SOCKET", "");
    let cfg = CastAgentConfig::load();
    assert!(
        cfg.socket_path.is_none(),
        "this test must run via the TCP/bridge transport (WS endpoint lives on the bridge)"
    );
    println!("transport: tcp/bridge ({})", cfg.gateway_url);

    let client = GatewayClient::new(Arc::new(cfg));
    client.health_probe().await;
    assert!(client.is_available(), "bridge must be reachable");

    let harness = std::env::var("CAST_AGENT_HARNESS")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "codex".to_string());
    let project_root = std::env::var("CAST_AGENT_PROJECT_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "/tmp".to_string());

    let conversation_id = "cast-agent-live-stream-smoke".to_string();
    let msg = AgentMessage {
        conversation_id: conversation_id.clone(),
        body: serde_json::json!({
            "prompt": "Reply with exactly the four characters: OK.",
            "title": "cast_agent live stream smoke",
            "harness": harness,
            "projectRoot": project_root,
        }),
    };

    let mut stream = client
        .stream_messages(msg)
        .await
        .expect("open WS stream against bridge");

    let mut deltas = 0usize;
    let mut accumulated = String::new();
    let mut saw_done = false;
    let mut saw_error: Option<String> = None;

    // Cap the wall-clock so a stuck stream can't pin the test runner.
    let collect = async {
        while let Some(item) = stream.next().await {
            let chunk = item.expect("chunk parse ok");
            match chunk {
                MessageChunk::Delta {
                    conversation_id: cid,
                    content,
                } => {
                    assert_eq!(cid, conversation_id, "delta correlates");
                    deltas += 1;
                    accumulated.push_str(&content);
                }
                MessageChunk::Done { conversation_id: cid } => {
                    assert_eq!(cid, conversation_id, "done correlates");
                    saw_done = true;
                    break;
                }
                MessageChunk::Error {
                    conversation_id: _,
                    message,
                } => {
                    saw_error = Some(message);
                    break;
                }
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(300), collect)
        .await
        .expect("stream completed within 5 min");

    println!("deltas:      {deltas}");
    println!("done frame:  {saw_done}");
    println!(
        "accumulated (first 800 chars):\n{}",
        accumulated.chars().take(800).collect::<String>()
    );

    if let Some(err_msg) = saw_error {
        panic!("bridge sent Error frame: {err_msg}");
    }
    assert!(deltas > 0, "expected at least one Delta frame");
    assert!(saw_done, "expected stream to close with a Done frame");
    assert!(
        !accumulated.is_empty(),
        "accumulated delta content should be non-empty"
    );
}
