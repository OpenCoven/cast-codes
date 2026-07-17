//! HTTP + WebSocket client for the Coven Gateway.
//!
//! Two transports, chosen by config:
//!
//! - **Unix transport** (preferred): `tokio::net::UnixStream` to
//!   `~/.coven/coven.sock`. Talks `/api/v1/*` to the live `coven` daemon.
//!   This is what the npm-distributed daemon actually serves. Both
//!   non-streamed (`send_message`) and streamed (`stream_messages`) chat
//!   are driven through the daemon's session lifecycle; since the daemon
//!   serves no WebSocket, streaming polls its event log incrementally and
//!   surfaces each new `output` event as a `MessageChunk::Delta`.
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

use anyhow::Context;
// `anyhow!()` only fires inside cfg(unix) branches; importing
// unconditionally would trip `-D unused-imports` on Windows/wasm.
#[cfg(unix)]
use anyhow::anyhow;
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
    Unix {
        socket: PathBuf,
    },
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
        let launched = self.launch_daemon_session(socket, &msg).await?;
        let session_id = launched.session_id.clone();

        match self.collect_session_output(socket, &session_id).await {
            Ok((output, final_status, exit_code)) => Ok(AgentResponse {
                conversation_id: session_id,
                body: serde_json::json!({
                    "text": output,
                    "status": final_status,
                    "exit_code": exit_code,
                    "harness": launched.harness,
                    "project_root": launched.project_root,
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

    /// Extract the launch parameters from an [`AgentMessage`] and create a
    /// non-interactive daemon session for it (`POST /api/v1/sessions`).
    /// Shared by the non-streaming [`send_message_via_daemon`] and the
    /// streaming [`stream_messages_via_daemon`] chat paths.
    #[cfg(unix)]
    async fn launch_daemon_session(
        &self,
        socket: &std::path::Path,
        msg: &AgentMessage,
    ) -> anyhow::Result<LaunchedDaemonSession> {
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
            // CastCodes' native agent is coven-code; headless callers that
            // don't specify a harness get it too, matching the agent panel.
            .unwrap_or_else(|| "coven-code".into());

        let familiar_id = msg
            .body
            .get("familiarId")
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .map(String::from);

        let title = msg
            .body
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| prompt.chars().take(60).collect::<String>());

        let mut launch_body = serde_json::json!({
            "projectRoot": project_root,
            "harness": harness,
            "prompt": prompt,
            "launchMode": "nonInteractive",
            "title": title,
        });
        // No debug log of familiar_id: it flows from the request body and
        // CodeQL's rust/cleartext-logging rule treats request-body fields as
        // sensitive (mirrors the existing no-log comment below).
        if let Some(fid) = &familiar_id {
            launch_body["familiarId"] = serde_json::Value::String(fid.clone());
        }
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

        Ok(LaunchedDaemonSession {
            session_id: launched.id,
            harness,
            project_root,
        })
    }

    /// Stream a chat turn through the daemon: launch a non-interactive
    /// session, then poll `/api/v1/events` incrementally and surface each
    /// new `output` event as a [`MessageChunk::Delta`] as it arrives —
    /// real incremental streaming, unlike [`send_message_via_daemon`]
    /// which accumulates and returns once at terminal status. Emits a
    /// final [`MessageChunk::Done`] when the session reaches a terminal
    /// status, or an `Err` on transport failure / timeout (the panel
    /// renders a stream error in-band). On timeout the session is killed
    /// to release the daemon slot.
    #[cfg(unix)]
    async fn stream_messages_via_daemon(
        &self,
        socket: &std::path::Path,
        msg: AgentMessage,
    ) -> anyhow::Result<MessageStream> {
        let launched = self.launch_daemon_session(socket, &msg).await?;
        let state = DaemonStreamState {
            socket: socket.to_path_buf(),
            conversation_id: launched.session_id.clone(),
            session_id: launched.session_id,
            after_seq: 0,
            deadline: Instant::now() + DAEMON_SESSION_TIMEOUT,
            request_timeout: self.config.request_timeout,
            poll_interval: DAEMON_POLL_INTERVAL,
            pending: std::collections::VecDeque::new(),
            finished: false,
        };

        let stream = futures::stream::unfold(state, |mut st| async move {
            // Emit any already-buffered chunk before doing more I/O.
            if let Some(chunk) = st.pending.pop_front() {
                return Some((Ok(chunk), st));
            }
            if st.finished {
                return None;
            }

            loop {
                if Instant::now() > st.deadline {
                    // Best-effort kill so the daemon slot is released.
                    let kill_path = format!("/api/v1/sessions/{}/kill", st.session_id);
                    let _ = unix_http::request(
                        &st.socket,
                        "POST",
                        &kill_path,
                        Some(b"{}"),
                        st.request_timeout,
                    )
                    .await;
                    st.finished = true;
                    return Some((
                        Err(anyhow!(
                            "session did not reach a terminal status within {:?}",
                            DAEMON_SESSION_TIMEOUT
                        )),
                        st,
                    ));
                }

                // Drain new output events into pending Deltas.
                match drain_output_deltas(
                    &st.socket,
                    &st.session_id,
                    &st.conversation_id,
                    &mut st.after_seq,
                    st.request_timeout,
                )
                .await
                {
                    Ok(deltas) => st.pending.extend(deltas),
                    Err(err) => {
                        st.finished = true;
                        return Some((Err(err), st));
                    }
                }

                // Check for a terminal session status.
                let status_path = format!("/api/v1/sessions/{}", st.session_id);
                let rec = match unix_http::request(
                    &st.socket,
                    "GET",
                    &status_path,
                    None,
                    st.request_timeout,
                )
                .await
                .and_then(|r| r.into_json::<DaemonSessionRecord>())
                {
                    Ok(rec) => rec,
                    Err(err) => {
                        st.finished = true;
                        return Some((Err(err.context("stream: GET session status")), st));
                    }
                };
                if daemon_chat::is_terminal_status(&rec.status) {
                    // Final drain for output emitted just before the flip.
                    if let Ok(deltas) = drain_output_deltas(
                        &st.socket,
                        &st.session_id,
                        &st.conversation_id,
                        &mut st.after_seq,
                        st.request_timeout,
                    )
                    .await
                    {
                        st.pending.extend(deltas);
                    }
                    st.pending.push_back(MessageChunk::Done {
                        conversation_id: st.conversation_id.clone(),
                    });
                    st.finished = true;
                    return st.pending.pop_front().map(|chunk| (Ok(chunk), st));
                }

                // Yield the first delta we buffered this round, if any;
                // otherwise wait and poll again.
                if let Some(chunk) = st.pending.pop_front() {
                    return Some((Ok(chunk), st));
                }
                tokio::time::sleep(st.poll_interval).await;
            }
        });

        Ok(Box::pin(stream))
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

    /// GET the Familiar persona catalog. Never errors: returns an empty
    /// catalog when the daemon does not implement the endpoint (404), is
    /// unreachable, or returns a malformed body. The TCP/bridge transport
    /// has no familiars endpoint, so it always yields an empty catalog.
    pub async fn list_familiars(&self) -> Vec<crate::daemon_schema::DaemonFamiliar> {
        match &self.transport {
            #[cfg(unix)]
            Transport::Unix { socket } => {
                match unix_http::request(
                    socket,
                    "GET",
                    "/api/v1/familiars",
                    None,
                    self.config.request_timeout,
                )
                .await
                {
                    Ok(resp) => parse_familiars(resp.status, &resp.body),
                    Err(err) => {
                        log::debug!(
                            "cast_agent: GET /api/v1/familiars failed: {err}; empty catalog"
                        );
                        Vec::new()
                    }
                }
            }
            Transport::Tcp { .. } => Vec::new(),
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

    /// Open a streaming chat session. On the TCP transport this connects to
    /// the gateway's `/v1/messages/stream` WebSocket; on the Unix transport
    /// it launches a daemon session and streams its event log incrementally
    /// (the daemon serves no WebSocket endpoint). Both yield the same
    /// [`MessageChunk`] stream.
    pub async fn stream_messages(&self, msg: AgentMessage) -> anyhow::Result<MessageStream> {
        // The daemon serves no WebSocket endpoint, so on the Unix transport
        // we stream by incrementally polling its event log instead.
        #[cfg(unix)]
        {
            if let Transport::Unix { socket } = &self.transport {
                return self.stream_messages_via_daemon(socket, msg).await;
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

/// Parse a `/api/v1/familiars` response into a catalog. Returns an empty
/// Vec for any non-2xx status or malformed body so the UI degrades to the
/// harness fallback instead of surfacing an error. Only used by the
/// `#[cfg(unix)]` daemon transport, so gated to keep non-unix builds
/// (Windows/wasm clippy) free of a dead-code warning.
#[cfg(unix)]
fn parse_familiars(status: u16, body: &[u8]) -> Vec<crate::daemon_schema::DaemonFamiliar> {
    if !(200..300).contains(&status) {
        log::debug!("cast_agent: /familiars returned HTTP {status}; treating as empty catalog");
        return Vec::new();
    }
    match serde_json::from_slice::<Vec<crate::daemon_schema::DaemonFamiliar>>(body) {
        Ok(list) => list,
        Err(err) => {
            log::debug!("cast_agent: /familiars body did not parse ({err}); empty catalog");
            Vec::new()
        }
    }
}

/// A daemon session launched for a chat turn: its id plus the resolved
/// launch parameters the caller echoes back in its response.
#[cfg(unix)]
struct LaunchedDaemonSession {
    session_id: String,
    harness: String,
    project_root: String,
}

/// Owned state threaded through the daemon streaming `unfold`. Holds
/// everything the poll loop needs so the returned stream borrows nothing
/// from the `GatewayClient`.
#[cfg(unix)]
struct DaemonStreamState {
    socket: std::path::PathBuf,
    session_id: String,
    conversation_id: String,
    after_seq: u64,
    deadline: Instant,
    request_timeout: std::time::Duration,
    poll_interval: std::time::Duration,
    pending: std::collections::VecDeque<MessageChunk>,
    finished: bool,
}

/// Fetch `output` events newer than `*after_seq` for a daemon session and
/// convert each into a [`MessageChunk::Delta`], advancing `*after_seq`
/// past every event seen. Empty output payloads are skipped so the stream
/// never emits blank deltas. Shared by the streaming poll loop.
#[cfg(unix)]
async fn drain_output_deltas(
    socket: &std::path::Path,
    session_id: &str,
    conversation_id: &str,
    after_seq: &mut u64,
    request_timeout: std::time::Duration,
) -> anyhow::Result<Vec<MessageChunk>> {
    let events_path = format!(
        "/api/v1/events?sessionId={session_id}&afterSeq={}",
        *after_seq
    );
    let page = unix_http::request(socket, "GET", &events_path, None, request_timeout)
        .await
        .and_then(|r| r.into_json::<daemon_chat::DaemonEventsPage>())
        .with_context(|| "GET /api/v1/events (stream)")?;

    let mut deltas = Vec::new();
    for ev in &page.events {
        if ev.kind == "output" {
            if let Some(data) = daemon_chat::parse_output_data(&ev.payload_json) {
                if !data.is_empty() {
                    deltas.push(MessageChunk::Delta {
                        conversation_id: conversation_id.to_string(),
                        content: data,
                    });
                }
            }
        }
        if ev.seq > *after_seq {
            *after_seq = ev.seq;
        }
    }
    Ok(deltas)
}

#[cfg(all(test, unix))]
mod familiars_tests {
    use super::*;

    #[test]
    fn parses_2xx_catalog() {
        let body = br#"[{"id":"nova"},{"id":"sage"}]"#;
        let out = parse_familiars(200, body);
        assert_eq!(
            out.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["nova", "sage"]
        );
    }

    #[test]
    fn empty_on_404() {
        assert!(parse_familiars(404, b"not found").is_empty());
    }

    #[test]
    fn empty_on_malformed_2xx_body() {
        assert!(parse_familiars(200, b"{not json}").is_empty());
    }
}
