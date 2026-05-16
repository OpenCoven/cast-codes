//! Top-level [`CastAgent`] entry point and the [`AgentBackend`] trait that
//! the rest of CastCodes (notably `crates/ai`) calls into.

use std::sync::Arc;

use crate::{
    comux::ComuxBridge,
    config::CastAgentConfig,
    gateway::GatewayClient,
    session::{CovenSession, SessionStore},
    substrate::{Substrate, SubstrateCollector},
};

/// Generic message sent to the agent — body is provider-shaped JSON so
/// `crates/ai` can keep its existing serialization without leaking types.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentMessage {
    pub conversation_id: String,
    pub body: serde_json::Value,
}

/// Generic response from the agent. The gateway currently only supports the
/// non-streamed `POST /v1/messages` path; streaming will be added separately.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentResponse {
    pub conversation_id: String,
    pub body: serde_json::Value,
}

/// The substrate manager + AI agent backend trait that the host calls into.
///
/// All methods are async. `is_available` is sync so the UI can poll cheaply.
#[async_trait::async_trait]
pub trait AgentBackend: Send + Sync {
    /// Send a chat message and await the (non-streamed) response.
    async fn send_message(&self, msg: AgentMessage) -> anyhow::Result<AgentResponse>;

    /// Collect the current workspace substrate (panes, branch, errors, etc.).
    async fn get_substrate(&self) -> anyhow::Result<Substrate>;

    /// List all active Coven sessions reachable via the gateway.
    async fn list_sessions(&self) -> anyhow::Result<Vec<CovenSession>>;

    /// Open a session by name. The gateway will create it if missing.
    async fn open_session(&self, name: &str) -> anyhow::Result<CovenSession>;

    /// Close a session by id. Idempotent.
    async fn close_session(&self, id: &str) -> anyhow::Result<()>;

    /// Whether the backend can reach its gateway right now. Cached.
    fn is_available(&self) -> bool;

    /// Display name for telemetry / UI ("Cast Agent").
    fn agent_name(&self) -> &'static str;
}

/// Concrete Cast Agent. Wraps the gateway client, substrate collector,
/// session store, and Comux bridge.
pub struct CastAgent {
    config: Arc<CastAgentConfig>,
    gateway: Arc<GatewayClient>,
    substrate: Arc<SubstrateCollector>,
    sessions: Arc<SessionStore>,
    comux: Arc<ComuxBridge>,
}

impl CastAgent {
    /// Build a CastAgent from config (or defaults if `None`).
    /// Construction is non-blocking: the gateway health probe runs on a
    /// detached task so a down gateway can't stall startup. `is_available()`
    /// will start returning true once the probe lands.
    pub async fn new(config: Option<CastAgentConfig>) -> Self {
        let config = Arc::new(config.unwrap_or_default());
        let gateway = Arc::new(GatewayClient::new(config.clone()));
        let probe_gateway = gateway.clone();
        tokio::spawn(async move { probe_gateway.health_probe().await });
        let substrate = Arc::new(SubstrateCollector::new());
        let sessions = Arc::new(SessionStore::new(gateway.clone()));
        let comux = Arc::new(ComuxBridge::new());
        Self {
            config,
            gateway,
            substrate,
            sessions,
            comux,
        }
    }

    /// Construct with config loaded from the standard sources
    /// (env + `~/.coven/config.toml`).
    pub async fn from_environment() -> Self {
        Self::new(Some(CastAgentConfig::load())).await
    }

    pub fn config(&self) -> &CastAgentConfig {
        &self.config
    }

    /// Re-run the `GET /health` probe to refresh `is_available()`. Cheap,
    /// safe to call on a periodic loop.
    pub async fn health_probe(&self) {
        self.gateway.health_probe().await;
    }
}

#[async_trait::async_trait]
impl AgentBackend for CastAgent {
    async fn send_message(&self, msg: AgentMessage) -> anyhow::Result<AgentResponse> {
        self.gateway.send_message(msg).await
    }

    async fn get_substrate(&self) -> anyhow::Result<Substrate> {
        let mut substrate = self.substrate.collect().await?;
        // Augment with Comux pane data when the daemon is reachable.
        substrate.comux_panes = self.comux.list_panes().await.unwrap_or_default();
        Ok(substrate)
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<CovenSession>> {
        self.sessions.list().await
    }

    async fn open_session(&self, name: &str) -> anyhow::Result<CovenSession> {
        self.sessions.open(name).await
    }

    async fn close_session(&self, id: &str) -> anyhow::Result<()> {
        self.sessions.close(id).await
    }

    fn is_available(&self) -> bool {
        self.gateway.is_available()
    }

    fn agent_name(&self) -> &'static str {
        "Cast Agent"
    }
}
