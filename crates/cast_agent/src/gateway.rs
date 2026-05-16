//! HTTP + WebSocket client for the Coven Gateway.
//!
//! Endpoints used:
//! - `GET  /health`          — startup probe; populates `is_available()`.
//! - `POST /v1/messages`     — send a chat message, returns a response body.
//! - `GET  /v1/sessions`     — list active Coven sessions.
//! - `POST /v1/sessions`     — open a session by name.
//! - `DELETE /v1/sessions/:id` — close a session.
//! - `GET  /v1/substrate`    — gateway-managed slices of substrate context.
//!
//! Auth header is `Authorization: Bearer <token>` when [`CastAgentConfig::token`] is set.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;

use crate::{
    agent::{AgentMessage, AgentResponse},
    config::CastAgentConfig,
    session::CovenSession,
};

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
}
