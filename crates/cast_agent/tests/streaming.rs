//! End-to-end test for `GatewayClient::stream_messages` against an
//! in-process stub WebSocket server. The stub accepts one connection,
//! reads the initial `AgentMessage` frame, and replies with a scripted
//! sequence of `MessageChunk` frames followed by a clean close. We then
//! drive the client and assert it receives the chunks in order.

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use cast_agent::{
    config::CastAgentConfig,
    gateway::{GatewayClient, MessageChunk},
    AgentMessage,
};

/// Install the workspace's rustls `CryptoProvider` exactly once per test
/// process. Production does this in `app/src/lib.rs`; tests have to do it
/// themselves because `reqwest::Client::build()` (called from
/// `GatewayClient::new`) otherwise panics with "No provider set".
fn install_crypto_provider_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_messages_yields_chunks_in_order() {
    install_crypto_provider_once();

    // Bind a TCP listener on a random local port and accept one WebSocket
    // upgrade. The stub replies with two deltas + done.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws handshake");

        // Read the initial AgentMessage frame so we know the client sent it.
        let initial = ws.next().await.expect("initial frame").expect("ok");
        let initial_text = match initial {
            WsMessage::Text(t) => t,
            other => panic!("expected text frame, got {other:?}"),
        };
        let parsed: AgentMessage = serde_json::from_str(&initial_text).expect("parse initial");
        assert_eq!(parsed.conversation_id, "conv-1");

        // Scripted replies.
        let chunks = [
            MessageChunk::Delta {
                conversation_id: "conv-1".into(),
                content: "Hello".into(),
            },
            MessageChunk::Delta {
                conversation_id: "conv-1".into(),
                content: ", world!".into(),
            },
            MessageChunk::Done {
                conversation_id: "conv-1".into(),
            },
        ];
        for chunk in &chunks {
            let text = serde_json::to_string(chunk).expect("encode");
            ws.send(WsMessage::Text(text)).await.expect("send");
        }
        // Clean close so the client stream ends.
        ws.close(None).await.expect("close");
    });

    // Build a client that points at the stub.
    let config = Arc::new(CastAgentConfig {
        gateway_url: format!("http://127.0.0.1:{port}"),
        token: None,
        request_timeout: std::time::Duration::from_secs(5),
    });
    let client = GatewayClient::new(config);

    let msg = AgentMessage {
        conversation_id: "conv-1".into(),
        body: serde_json::json!({ "text": "hi" }),
    };
    let mut stream = client.stream_messages(msg).await.expect("open stream");

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.expect("chunk ok"));
    }

    assert_eq!(
        received,
        vec![
            MessageChunk::Delta {
                conversation_id: "conv-1".into(),
                content: "Hello".into(),
            },
            MessageChunk::Delta {
                conversation_id: "conv-1".into(),
                content: ", world!".into(),
            },
            MessageChunk::Done {
                conversation_id: "conv-1".into(),
            },
        ]
    );

    server.await.expect("server task");
}
