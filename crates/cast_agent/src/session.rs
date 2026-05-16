//! Coven session bridging — list / open / close active sessions through
//! the gateway, with in-memory caching so the UI can render without
//! re-querying on every render pass.
//!
//! The cache uses [`std::sync::RwLock`] (not `tokio::sync::RwLock`) so the
//! UI thread can take a synchronous read snapshot on every render — the
//! mutator side is the background refresh loop running on the cast_agent
//! runtime, and contention is negligible (one writer, brief critical
//! section). Switching to `arc-swap` would be marginally faster but adds
//! a dependency for no measurable win at this list size.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::gateway::GatewayClient;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CovenSession {
    pub id: String,
    pub name: String,
    pub status: SessionStatus,
    /// RFC3339 timestamp the session was last active.
    pub last_active: Option<String>,
    /// Working directory the session was opened in. `None` when the gateway
    /// didn't return one (older gateway versions or sessions opened without
    /// a directory) — UI uses this to decide whether the row is clickable.
    #[serde(default)]
    pub cwd: Option<PathBuf>,
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
                let mut guard = self
                    .cache
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *guard = sessions.clone();
                Ok(sessions)
            }
            Err(err) => {
                log::warn!("cast_agent: session list failed: {err}");
                Ok(self.snapshot())
            }
        }
    }

    pub async fn open(&self, name: &str) -> anyhow::Result<CovenSession> {
        let session = self.gateway.open_session(name).await?;
        let mut guard = self
            .cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = guard.iter_mut().find(|s| s.id == session.id) {
            *existing = session.clone();
        } else {
            guard.push(session.clone());
        }
        Ok(session)
    }

    pub async fn close(&self, id: &str) -> anyhow::Result<()> {
        self.gateway.close_session(id).await?;
        let mut guard = self
            .cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.retain(|s| s.id != id);
        Ok(())
    }

    /// Sync snapshot of the cached session list. Safe to call from the
    /// UI thread — uses a [`std::sync::RwLock`] and recovers from
    /// poisoning by returning the inner data unchanged.
    pub fn snapshot(&self) -> Vec<CovenSession> {
        self.cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}
