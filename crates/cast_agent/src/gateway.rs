//! HTTP + WebSocket client for the Coven Gateway.
//!
//! Endpoints used:
//! - `GET  /health`                 — startup probe; populates `is_available()`.
//! - `POST /v1/messages`            — send a chat message, returns a response body.
//! - `WS   /v1/messages/stream`     — stream a chat response back as chunks.
//! - `GET  /v1/sessions`            — list active Coven sessions.
//! - `POST /v1/sessions`            — open a session by name.
//! - `DELETE /v1/sessions/:id`      — close a session.
//! - `GET  /v1/substrate`           — gateway-managed slices of substrate context.
//!
//! Auth header is `Authorization: Bearer <token>` when [`CastAgentConfig::token`] is set.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use futures::{SinkExt, Stream, StreamExt};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message as WsMessage};

use crate::{
    agent::{AgentMessage, AgentResponse},
    config::CastAgentConfig,
    session::CovenSession,
};

/// A chunk of a streamed chat response from `/v1/messages/stream`.
///
/// The gateway emits one JSON-encoded `MessageChunk` per WebSocket text
/// frame. `Delta` carries a partial content fragment; `Done` marks the end
/// of the stream (and the gateway will then close the WS); `Error` reports
/// an in-flight failure and is followed by close.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageChunk {
    Delta {
        conversation_id: String,
        content: String,
    },
    Done {
        conversation_id: String,
    },
    Error {
        conversation_id: String,
        message: String,
    },
}

pub struct GatewayClient {
    config: Arc<CastAgentConfig>,
    http: reqwest::Client,
    available: AtomicBool,
}

impl GatewayClient {
    pub fn new(config: Arc<CastAgentConfig>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .build()
            .expect("cast_agent: failed to build reqwest client (TLS init?)");
        Self {
            config,
            http,
            available: AtomicBool::new(false),
        }
    }

    /// Hit `GET /health` and update `is_available()`. Never panics; logs on
    /// failure and falls back to `false` (degraded mode).
    pub async fn health_probe(&self) {
        let url = format!("{}/health", self.config.gateway_url.trim_end_matches('/'));
        let ok = match self.http.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(err) => {
                log::warn!(
                    "cast_agent: Coven Gateway health probe failed for {url}: {err} — running in degraded mode"
                );
                false
            }
        };
        self.available.store(ok, Ordering::Release);
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }

    fn auth_header(&self) -> Option<(&'static str, String)> {
        self.config
            .token
            .as_ref()
            .map(|t| ("Authorization", format!("Bearer {t}")))
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}{}",
            self.config.gateway_url.trim_end_matches('/'),
            path
        )
    }

    pub async fn send_message(&self, msg: AgentMessage) -> anyhow::Result<AgentResponse> {
        let mut req = self.http.post(self.url("/v1/messages")).json(&msg);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .with_context(|| "POST /v1/messages")?
            .error_for_status()?;
        Ok(resp.json::<AgentResponse>().await?)
    }

    pub async fn list_sessions(&self) -> anyhow::Result<Vec<CovenSession>> {
        let mut req = self.http.get(self.url("/v1/sessions"));
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .with_context(|| "GET /v1/sessions")?
            .error_for_status()?;
        Ok(resp.json::<Vec<CovenSession>>().await?)
    }

    pub async fn open_session(&self, name: &str) -> anyhow::Result<CovenSession> {
        #[derive(serde::Serialize)]
        struct OpenBody<'a> {
            name: &'a str,
        }
        let mut req = self
            .http
            .post(self.url("/v1/sessions"))
            .json(&OpenBody { name });
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .with_context(|| "POST /v1/sessions")?
            .error_for_status()?;
        Ok(resp.json::<CovenSession>().await?)
    }

    pub async fn close_session(&self, id: &str) -> anyhow::Result<()> {
        let mut req = self.http.delete(self.url(&format!("/v1/sessions/{id}")));
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        req.send()
            .await
            .with_context(|| format!("DELETE /v1/sessions/{id}"))?
            .error_for_status()?;
        Ok(())
    }

    /// Build a `ws://` / `wss://` URL for the given path by rewriting the
    /// HTTP scheme of `gateway_url`. Falls back to `ws://` if the scheme
    /// is unrecognised.
    fn ws_url(&self, path: &str) -> String {
        let base = self.config.gateway_url.trim_end_matches('/');
        let scheme_swapped = if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = base.strip_prefix("http://") {
            format!("ws://{rest}")
        } else if base.starts_with("ws://") || base.starts_with("wss://") {
            base.to_string()
        } else {
            // Unknown scheme — assume insecure and hope the server tells us.
            format!("ws://{base}")
        };
        format!("{scheme_swapped}{path}")
    }

    /// Open a streaming chat session against `/v1/messages/stream`.
    ///
    /// Wire protocol: the client sends the [`AgentMessage`] as a single
    /// JSON text frame, then the server emits one JSON-encoded
    /// [`MessageChunk`] per text frame and closes the socket when done.
    /// Binary frames are ignored; ping/pong is handled by tokio-tungstenite.
    ///
    /// The returned stream surfaces transport, JSON, and protocol errors as
    /// `Err` items; a clean server close ends the stream. Callers should
    /// not assume `MessageChunk::Done` is always the last item — a stream
    /// can also end on `MessageChunk::Error` or on a transport failure.
    ///
    /// Boxed so the returned stream is `Unpin` and callers can drive it
    /// with `.next().await` without manual pinning.
    pub async fn stream_messages(
        &self,
        msg: AgentMessage,
    ) -> anyhow::Result<MessageStream> {
        let url = self.ws_url("/v1/messages/stream");
        let mut request = url
            .as_str()
            .into_client_request()
            .with_context(|| format!("invalid WebSocket URL {url}"))?;
        if let Some((k, v)) = self.auth_header() {
            request.headers_mut().insert(
                k,
                v.parse()
                    .with_context(|| "Authorization header is not valid")?,
            );
        }

        let (mut ws, _http_resp) = tokio_tungstenite::connect_async(request)
            .await
            .with_context(|| format!("connect to {url}"))?;

        let initial = serde_json::to_string(&msg)?;
        ws.send(WsMessage::Text(initial))
            .await
            .with_context(|| "send initial AgentMessage frame")?;

        // Drive the read side as a stream. Each text frame is parsed as a
        // `MessageChunk`. Non-text frames are skipped (tungstenite already
        // handles ping/pong internally), and close frames end the stream.
        let stream = futures::stream::unfold(ws, |mut ws| async move {
            loop {
                match ws.next().await {
                    Some(Ok(WsMessage::Text(text))) => {
                        let parsed: anyhow::Result<MessageChunk> =
                            serde_json::from_str(&text).map_err(Into::into);
                        return Some((parsed, ws));
                    }
                    Some(Ok(WsMessage::Close(_))) | None => return None,
                    Some(Ok(_)) => continue, // ignore binary / ping / pong
                    Some(Err(err)) => return Some((Err(err.into()), ws)),
                }
            }
        });

        Ok(Box::pin(stream))
    }
}

/// Boxed message stream returned by [`GatewayClient::stream_messages`].
/// Boxing makes the stream `Unpin`, so callers can drive it with
/// `.next().await` without manual `pin_mut!`.
pub type MessageStream =
    std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<MessageChunk>> + Send>>;
