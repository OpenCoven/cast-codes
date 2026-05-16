//! Coven session bridging — list / open / close active sessions through
//! the gateway, with in-memory caching so the UI can render without
//! re-querying on every render pass.

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::gateway::GatewayClient;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CovenSession {
    pub id: String,
    pub name: String,
    pub status: SessionStatus,
    /// RFC3339 timestamp the session was last active.
    pub last_active: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Idle,
    Closed,
}

pub struct SessionStore {
    gateway: Arc<GatewayClient>,
    cache: RwLock<Vec<CovenSession>>,
}

impl SessionStore {
    pub fn new(gateway: Arc<GatewayClient>) -> Self {
        Self {
            gateway,
            cache: RwLock::new(Vec::new()),
        }
    }

    /// Fetch sessions from the gateway, updating the cache. Returns the
    /// cached value (possibly empty) if the gateway is unreachable.
    pub async fn list(&self) -> anyhow::Result<Vec<CovenSession>> {
        match self.gateway.list_sessions().await {
            Ok(sessions) => {
                let mut guard = self.cache.write().await;
                *guard = sessions.clone();
                Ok(sessions)
            }
            Err(err) => {
                log::warn!("cast_agent: session list failed: {err}");
                Ok(self.cache.read().await.clone())
            }
        }
    }

    pub async fn open(&self, name: &str) -> anyhow::Result<CovenSession> {
        let session = self.gateway.open_session(name).await?;
        let mut guard = self.cache.write().await;
        if let Some(existing) = guard.iter_mut().find(|s| s.id == session.id) {
            *existing = session.clone();
        } else {
            guard.push(session.clone());
        }
        Ok(session)
    }

    pub async fn close(&self, id: &str) -> anyhow::Result<()> {
        self.gateway.close_session(id).await?;
        let mut guard = self.cache.write().await;
        guard.retain(|s| s.id != id);
        Ok(())
    }

    pub async fn cached(&self) -> Vec<CovenSession> {
        self.cache.read().await.clone()
    }
}
