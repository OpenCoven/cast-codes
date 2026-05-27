//! HTTP + WebSocket client for the Coven Gateway.
//!
//! Two transports, chosen by config:
//!
//! - **Unix transport** (preferred): `tokio::net::UnixStream` to
//!   `~/.coven/coven.sock`. Talks `/api/v1/*` to the live `coven` daemon.
//!   This is what the npm-distributed daemon actually serves. Non-streamed
//!   chat is driven through the daemon's session lifecycle. WebSocket
//!   streaming is not supported by the daemon, so `stream_messages` returns
//!   a clear error in this mode.
//! - **TCP transport** (legacy): `reqwest` to `gateway_url`. Talks
//!   `/v1/*` to a hypothetical Coven Gateway that mirrors the schema
//!   CastCodes originally shipped against. Kept for back-compat with
//!   environments that have such a gateway in front of (or instead of)
//!   the daemon.
//!
//! Endpoints used on Unix:
//! - `GET  /api/v1/health`                — startup probe.
//! - `GET  /api/v1/sessions`              — list active Coven sessions.
//! - `POST /api/v1/sessions`              — open a session.
//! - `POST /api/v1/sessions/:id/kill`     — close (kill) a session
//!   (the daemon exposes no DELETE method; kill is the closest analogue).
//!
//! Endpoints used on TCP (legacy):
//! - `GET  /health`, `POST /v1/messages`, `WS /v1/messages/stream`,
//!   `GET/POST /v1/sessions`, `DELETE /v1/sessions/:id`.
//!
//! Auth header is `Authorization: Bearer <token>` when
//! [`CastAgentConfig::token`] is set on TCP requests. The Unix-transport
//! daemon currently relies on process-local trust via socket file mode.

#[cfg(unix)]
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{anyhow, Context};
use futures::{SinkExt, Stream, StreamExt};
#[cfg(unix)]
use instant::Instant;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message as WsMessage};

use crate::{
    agent::{AgentMessage, AgentResponse},
    config::CastAgentConfig,
    session::CovenSession,
};
// Unix transport: direct HTTP/1.1 over the daemon's socket. The
// `daemon_chat` and `unix_http` modules + the daemon-shape adapter
// only compile on Unix; the cross-platform default is the TCP/bridge
// path below.
#[cfg(unix)]
use crate::{
    daemon_chat,
    session::{convert_daemon_sessions, DaemonSessionRecord},
    unix_http,
};

/// Maximum time we wait for a non-interactive daemon session to reach a
/// terminal status before we give up and kill it. Chat turns through
/// real harnesses can take a while (Codex exploration, Claude tool use),
/// but a single non-interactive prompt should not exceed five minutes —
/// past that, almost certainly stuck.
const DAEMON_SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Interval between event polls during a chat turn. Short enough for
/// reasonable interactivity, long enough that a wedged daemon doesn't
/// turn into a busy loop. Only used by the Unix-transport poll loop.
#[cfg(unix)]
const DAEMON_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

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

enum Transport {
    #[cfg(unix)]
    Unix { socket: PathBuf },
    Tcp {
        http: reqwest::Client,
    },
}

fn build_http_client(config: &CastAgentConfig) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(config.request_timeout)
        .build()
        .expect("cast_agent: failed to build reqwest client (TLS init?)")
}

pub struct GatewayClient {
    config: Arc<CastAgentConfig>,
    transport: Transport,
    available: AtomicBool,
}

impl GatewayClient {
    pub fn new(config: Arc<CastAgentConfig>) -> Self {
        // On non-Unix targets the `socket_path` config field is ignored —
        // Windows has no `tokio::net::UnixStream`, wasm doesn't run the
        // daemon — and the TCP/bridge path is the only option.
        let transport = match () {
            #[cfg(unix)]
            () => match config.socket_path.clone() {
                Some(socket) => Transport::Unix { socket },
                None => Transport::Tcp {
                    http: build_http_client(&config),
                },
            },
            #[cfg(not(unix))]
            () => Transport::Tcp {
                http: build_http_client(&config),
            },
        };
        Self {
            config,
            transport,
            available: AtomicBool::new(false),
        }
    }

    /// Hit `GET /health` (or `/api/v1/health` on Unix) and update
    /// `is_available()`. Never panics; logs on failure and falls back to
    /// `false` (degraded mode).
    pub async fn health_probe(&self) {
        let ok = match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => {
                match unix_http::request(
                    socket,
                    "GET",
                    "/api/v1/health",
                    None,
                    self.config.request_timeout,
                )
                .await
                {
                    Ok(resp) => (200..300).contains(&resp.status),
                    Err(err) => {
                        log::warn!(
                            "cast_agent: Coven daemon health probe failed for {}: {err} — running in degraded mode",
                            socket.display()
                        );
                        false
                    }
                }
            }
            Transport::Tcp { http } => {
                let url = format!("{}/health", self.config.gateway_url.trim_end_matches('/'));
                match http.get(&url).send().await {
                    Ok(resp) => resp.status().is_success(),
                    Err(err) => {
                        log::warn!(
                            "cast_agent: Coven Gateway health probe failed for {url}: {err} — running in degraded mode"
                        );
                        false
                    }
                }
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

    fn tcp_url(&self, path: &str) -> String {
        format!("{}{}", self.config.gateway_url.trim_end_matches('/'), path)
    }

    /// Send a non-streamed chat message.
    ///
    /// On the **Unix transport**, this drives the daemon's session
    /// lifecycle: extract a text prompt from `msg.body`, `POST
    /// /api/v1/sessions` with `launchMode: "nonInteractive"`, poll the
    /// `/api/v1/events` stream and `/api/v1/sessions/:id` status until
    /// the session reaches a terminal status, and return the accumulated
    /// output as an [`AgentResponse`]. The returned `conversation_id`
    /// is the daemon's session id (not the input `msg.conversation_id`),
    /// so callers can join the result back to the session list and
    /// fetch full event history later. On timeout, the session is killed.
    ///
    /// On the **TCP transport**, this falls through to the legacy
    /// `POST /v1/messages` shape unchanged.
    pub async fn send_message(&self, msg: AgentMessage) -> anyhow::Result<AgentResponse> {
        match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => self.send_message_via_daemon(socket, msg).await,
            Transport::Tcp { http } => {
                // /v1/messages is inherently long-running on the bridge —
                // it create-polls-collects a daemon session. The default
                // request_timeout (used by /health, /v1/sessions) is too
                // short for chat. Override per-call to match the Unix
                // path's `DAEMON_SESSION_TIMEOUT`.
                let mut req = http
                    .post(self.tcp_url("/v1/messages"))
                    .timeout(DAEMON_SESSION_TIMEOUT)
                    .json(&msg);
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
        }
    }

    /// Unix-transport implementation of `send_message`. Pulled out so
    /// the create-poll-collect dance doesn't bloat the public method.
    #[cfg(unix)]
    async fn send_message_via_daemon(
        &self,
        socket: &std::path::Path,
        msg: AgentMessage,
    ) -> anyhow::Result<AgentResponse> {
        let prompt = daemon_chat::extract_prompt(&msg.body).ok_or_else(|| {
            anyhow!(
                "could not extract a prompt from AgentMessage.body; \
                 expected `prompt`, `text`, `message`, or `messages[].content` \
                 (got: {})",
                serde_json::to_string(&msg.body).unwrap_or_default()
            )
        })?;

        let project_root = std::env::var("CAST_AGENT_PROJECT_ROOT")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                msg.body
                    .get("projectRoot")
                    .and_then(serde_json::Value::as_str)
                    .map(String::from)
            })
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .ok_or_else(|| anyhow!("could not determine projectRoot for daemon session"))?;

        let harness = std::env::var("CAST_AGENT_HARNESS")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                msg.body
                    .get("harness")
                    .and_then(serde_json::Value::as_str)
                    .map(String::from)
            })
            .unwrap_or_else(|| "claude".into());

        let title = msg
            .body
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| prompt.chars().take(60).collect::<String>());

        let launch_body = serde_json::json!({
            "projectRoot": project_root,
            "harness": harness,
            "prompt": prompt,
            "launchMode": "nonInteractive",
            "title": title,
        });
        let launch_bytes = serde_json::to_vec(&launch_body)?;

        // No debug log of the launch parameters: `harness` and
        // `project_root` both flow from user-controlled body fields, and
        // CodeQL's `rust/cleartext-logging` rule treats anything from
        // request bodies that lands in a log as sensitive. The daemon
        // already records the full launch body in its session events,
        // so any debugging that needs the values has them there.
        let launch_resp = unix_http::request(
            socket,
            "POST",
            "/api/v1/sessions",
            Some(launch_bytes.as_slice()),
            self.config.request_timeout,
        )
        .await
        .with_context(|| "POST /api/v1/sessions (unix)")?;
        let launched = launch_resp.into_json::<DaemonSessionRecord>()?;
        let session_id = launched.id.clone();

        match self
            .collect_session_output(socket, &session_id, prompt.len())
            .await
        {
            Ok((output, final_status, exit_code)) => Ok(AgentResponse {
                conversation_id: session_id,
                body: serde_json::json!({
                    "text": output,
                    "status": final_status,
                    "exit_code": exit_code,
                    "harness": harness,
                    "project_root": project_root,
                }),
            }),
            Err(err) => {
                // Intentionally do NOT log the daemon session id — the
                // daemon already records it in its events and CodeQL
                // flags session-uid cleartext logging.
                log::warn!(
                    "cast_agent: chat collect failed: {err}; \
                     killing session to release the daemon slot"
                );
                let kill_path = format!("/api/v1/sessions/{session_id}/kill");
                let _ = unix_http::request(
                    socket,
                    "POST",
                    &kill_path,
                    Some(b"{}"),
                    self.config.request_timeout,
                )
                .await;
                Err(err)
            }
        }
    }

    /// Poll the daemon for events + status of the given session until it
    /// reaches a terminal status or the timeout fires. Returns the
    /// accumulated `output` text, the final status string, and the exit
    /// code (if any).
    #[cfg(unix)]
    async fn collect_session_output(
        &self,
        socket: &std::path::Path,
        session_id: &str,
        _prompt_len: usize,
    ) -> anyhow::Result<(String, String, Option<i32>)> {
        let deadline = Instant::now() + DAEMON_SESSION_TIMEOUT;
        let mut after_seq: u64 = 0;
        let mut output = String::new();

        loop {
            if Instant::now() > deadline {
                return Err(anyhow!(
                    "session {session_id} did not reach a terminal status within {:?}",
                    DAEMON_SESSION_TIMEOUT
                ));
            }

            // Drain new events.
            let events_path = format!("/api/v1/events?sessionId={session_id}&afterSeq={after_seq}");
            let events_resp = unix_http::request(
                socket,
                "GET",
                &events_path,
                None,
                self.config.request_timeout,
            )
            .await
            .with_context(|| format!("GET {events_path}"))?;
            let page = events_resp.into_json::<daemon_chat::DaemonEventsPage>()?;
            for ev in &page.events {
                if ev.kind == "output" {
                    if let Some(data) = daemon_chat::parse_output_data(&ev.payload_json) {
                        output.push_str(&data);
                    }
                }
                if ev.seq > after_seq {
                    after_seq = ev.seq;
                }
            }

            // Check terminal status. The daemon emits status transitions
            // as events too, but the session record is the authoritative
            // source — race conditions where the last output event is
            // emitted just before the status flip resolve correctly
            // because we drain events before checking status.
            let status_path = format!("/api/v1/sessions/{session_id}");
            let status_resp = unix_http::request(
                socket,
                "GET",
                &status_path,
                None,
                self.config.request_timeout,
            )
            .await
            .with_context(|| format!("GET {status_path}"))?;
            let rec = status_resp.into_json::<DaemonSessionRecord>()?;
            if daemon_chat::is_terminal_status(&rec.status) {
                // One final event drain to capture anything emitted in
                // the gap between our last fetch and the status flip.
                let final_events_path =
                    format!("/api/v1/events?sessionId={session_id}&afterSeq={after_seq}");
                if let Ok(resp) = unix_http::request(
                    socket,
                    "GET",
                    &final_events_path,
                    None,
                    self.config.request_timeout,
                )
                .await
                {
                    if let Ok(page) = resp.into_json::<daemon_chat::DaemonEventsPage>() {
                        for ev in &page.events {
                            if ev.kind == "output" {
                                if let Some(data) = daemon_chat::parse_output_data(&ev.payload_json)
                                {
                                    output.push_str(&data);
                                }
                            }
                        }
                    }
                }
                return Ok((output, rec.status, rec.exit_code));
            }

            tokio::time::sleep(DAEMON_POLL_INTERVAL).await;
        }
    }

    pub async fn list_sessions(&self) -> anyhow::Result<Vec<CovenSession>> {
        match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => {
                let resp = unix_http::request(
                    socket,
                    "GET",
                    "/api/v1/sessions",
                    None,
                    self.config.request_timeout,
                )
                .await
                .with_context(|| "GET /api/v1/sessions (unix)")?;
                let records = resp.into_json::<Vec<DaemonSessionRecord>>()?;
                Ok(convert_daemon_sessions(records))
            }
            Transport::Tcp { http } => {
                let mut req = http.get(self.tcp_url("/v1/sessions"));
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
        }
    }

    pub async fn open_session(&self, name: &str) -> anyhow::Result<CovenSession> {
        match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => {
                // Daemon's POST /api/v1/sessions takes a richer body than
                // CastCodes' historical `{name}` shape. We map `name` to
                // `title` and use the current working directory for
                // projectRoot; harness defaults to `claude` since that's
                // the typical CastCodes flow.
                let cwd = std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let body = serde_json::json!({
                    "title": name,
                    "projectRoot": cwd,
                    "harness": "claude",
                });
                let resp = unix_http::request(
                    socket,
                    "POST",
                    "/api/v1/sessions",
                    Some(serde_json::to_vec(&body)?.as_slice()),
                    self.config.request_timeout,
                )
                .await
                .with_context(|| "POST /api/v1/sessions (unix)")?;
                let record = resp.into_json::<DaemonSessionRecord>()?;
                Ok(CovenSession::from(record))
            }
            Transport::Tcp { http } => {
                #[derive(serde::Serialize)]
                struct OpenBody<'a> {
                    name: &'a str,
                }
                let mut req = http
                    .post(self.tcp_url("/v1/sessions"))
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
        }
    }

    pub async fn close_session(&self, id: &str) -> anyhow::Result<()> {
        match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => {
                // The daemon exposes session lifecycle via
                // `POST /api/v1/sessions/:id/kill` rather than DELETE.
                let path = format!("/api/v1/sessions/{id}/kill");
                let resp = unix_http::request(
                    socket,
                    "POST",
                    &path,
                    Some(b"{}"),
                    self.config.request_timeout,
                )
                .await
                .with_context(|| format!("POST {path} (unix)"))?;
                resp.ensure_2xx()
            }
            Transport::Tcp { http } => {
                let mut req = http.delete(self.tcp_url(&format!("/v1/sessions/{id}")));
                if let Some((k, v)) = self.auth_header() {
                    req = req.header(k, v);
                }
                req.send()
                    .await
                    .with_context(|| format!("DELETE /v1/sessions/{id}"))?
                    .error_for_status()?;
                Ok(())
            }
        }
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

    /// Open a streaming chat session against `/v1/messages/stream`. Only
    /// supported on the TCP transport — the Unix-socket daemon does not
    /// serve a WebSocket endpoint.
    pub async fn stream_messages(&self, msg: AgentMessage) -> anyhow::Result<MessageStream> {
        #[cfg(unix)]
        {
            if matches!(self.transport, Transport::Unix { .. }) {
                return Err(anyhow!(
                    "stream_messages is not supported on the Unix daemon transport \
                     (daemon does not serve WebSocket endpoints)"
                ));
            }
        }

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

        let stream = futures::stream::unfold(ws, |mut ws| async move {
            loop {
                match ws.next().await {
                    Some(Ok(WsMessage::Text(text))) => {
                        let parsed: anyhow::Result<MessageChunk> =
                            serde_json::from_str(&text).map_err(Into::into);
                        return Some((parsed, ws));
                    }
                    Some(Ok(WsMessage::Close(_))) | None => return None,
                    Some(Ok(_)) => continue,
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
pub type MessageStream = std::pin::Pin<Box<dyn Stream<Item = anyhow::Result<MessageChunk>> + Send>>;
