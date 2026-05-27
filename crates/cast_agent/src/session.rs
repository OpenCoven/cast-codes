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
//!
//! Two on-wire schemas are supported:
//!
//! - The live OpenCoven daemon (`@opencoven/cli`) returns
//!   [`DaemonSessionRecord`] over `/api/v1/sessions` (Unix socket).
//!   The gateway client deserializes that shape and maps it into
//!   [`CovenSession`] via [`convert_daemon_sessions`].
//! - A hypothetical Coven Gateway returning `Vec<CovenSession>` directly
//!   is still supported on the TCP transport for back-compat. That path
//!   does no mapping.
//!
//! [`CovenSession`] is the UI-facing type; only the gateway boundary knows
//! about the daemon's richer record.

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

/// On-wire shape returned by the OpenCoven daemon's `/api/v1/sessions`
/// endpoint. Field names are snake_case to match the daemon. Kept
/// `pub(crate)` because the gateway is the only producer; the UI sees
/// only [`CovenSession`].
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)] // schema-shaped fields kept for documentation + forward use
pub(crate) struct DaemonSessionRecord {
    pub id: String,
    #[serde(default)]
    pub project_root: Option<String>,
    #[serde(default)]
    pub harness: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub status: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

impl DaemonSessionRecord {
    fn display_name(&self) -> String {
        if let Some(title) = self.title.as_ref().filter(|t| !t.trim().is_empty()) {
            return title.clone();
        }
        if let Some(root) = self.project_root.as_ref() {
            if let Some(last) = std::path::Path::new(root).file_name() {
                return last.to_string_lossy().into_owned();
            }
            if !root.is_empty() {
                return root.clone();
            }
        }
        self.id.clone()
    }

    fn mapped_status(&self) -> SessionStatus {
        // Daemon statuses observed in the wild: `created`, `running`,
        // `completed`, `killed`, `failed`, `orphaned`, `idle`. Unknown
        // statuses fall through to Idle so a future daemon can add new
        // values without crashing the UI.
        match self.status.as_str() {
            "running" => SessionStatus::Active,
            "created" | "idle" => SessionStatus::Idle,
            "completed" | "killed" | "failed" | "orphaned" => SessionStatus::Closed,
            _ => SessionStatus::Idle,
        }
    }

    fn last_active(&self) -> Option<String> {
        // `updated_at` reflects the most recent activity; fall back to
        // `created_at` if the daemon didn't populate it (shouldn't happen
        // for live sessions but is technically optional).
        self.updated_at.clone().or_else(|| self.created_at.clone())
    }

    fn cwd(&self) -> Option<PathBuf> {
        self.project_root
            .as_ref()
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
    }
}

impl From<DaemonSessionRecord> for CovenSession {
    fn from(rec: DaemonSessionRecord) -> Self {
        let name = rec.display_name();
        let status = rec.mapped_status();
        let last_active = rec.last_active();
        let cwd = rec.cwd();
        CovenSession {
            id: rec.id,
            name,
            status,
            last_active,
            cwd,
        }
    }
}

// Only called from the Unix-transport gateway; gated to avoid
// `-D dead-code` on Windows/wasm builds. The unit tests below still
// exercise it via `cfg(test)` on all platforms.
#[cfg(any(unix, test))]
pub(crate) fn convert_daemon_sessions(records: Vec<DaemonSessionRecord>) -> Vec<CovenSession> {
    records.into_iter().map(CovenSession::from).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(status: &str) -> DaemonSessionRecord {
        DaemonSessionRecord {
            id: "abc".into(),
            project_root: Some("/Users/x/proj".into()),
            harness: Some("claude".into()),
            title: Some("hello".into()),
            status: status.into(),
            exit_code: None,
            archived_at: None,
            created_at: Some("2026-05-26T00:00:00Z".into()),
            updated_at: Some("2026-05-26T00:01:00Z".into()),
            conversation_id: None,
        }
    }

    #[test]
    fn status_mapping_covers_known_daemon_states() {
        assert_eq!(rec("running").mapped_status(), SessionStatus::Active);
        assert_eq!(rec("created").mapped_status(), SessionStatus::Idle);
        assert_eq!(rec("idle").mapped_status(), SessionStatus::Idle);
        assert_eq!(rec("completed").mapped_status(), SessionStatus::Closed);
        assert_eq!(rec("killed").mapped_status(), SessionStatus::Closed);
        assert_eq!(rec("failed").mapped_status(), SessionStatus::Closed);
        assert_eq!(rec("orphaned").mapped_status(), SessionStatus::Closed);
    }

    #[test]
    fn unknown_status_defaults_to_idle() {
        assert_eq!(rec("future_state").mapped_status(), SessionStatus::Idle);
    }

    #[test]
    fn display_name_prefers_title_then_basename_then_id() {
        let r = rec("running");
        assert_eq!(CovenSession::from(r).name, "hello");

        let r = DaemonSessionRecord {
            title: None,
            ..rec("running")
        };
        assert_eq!(CovenSession::from(r).name, "proj");

        let r = DaemonSessionRecord {
            title: None,
            project_root: None,
            ..rec("running")
        };
        assert_eq!(CovenSession::from(r).name, "abc");
    }

    #[test]
    fn last_active_prefers_updated_then_created() {
        let r = rec("running");
        assert_eq!(
            CovenSession::from(r).last_active,
            Some("2026-05-26T00:01:00Z".into())
        );

        let r = DaemonSessionRecord {
            updated_at: None,
            ..rec("running")
        };
        assert_eq!(
            CovenSession::from(r).last_active,
            Some("2026-05-26T00:00:00Z".into())
        );
    }

    #[test]
    fn deserializes_live_daemon_payload() {
        let raw = r#"[{
            "id": "4467fa62-970b-42a1-bd0e-8af676ec4888",
            "project_root": "/Users/buns/.openclaw/workspace",
            "harness": "codex",
            "title": "session list",
            "status": "completed",
            "exit_code": 0,
            "archived_at": null,
            "created_at": "2026-05-27T01:02:52.227936000Z",
            "updated_at": "2026-05-27T01:04:03.328045000Z",
            "conversation_id": null
        }]"#;
        let records: Vec<DaemonSessionRecord> = serde_json::from_str(raw).expect("parse");
        let mapped = convert_daemon_sessions(records);
        assert_eq!(mapped.len(), 1);
        let s = &mapped[0];
        assert_eq!(s.id, "4467fa62-970b-42a1-bd0e-8af676ec4888");
        assert_eq!(s.name, "session list");
        assert_eq!(s.status, SessionStatus::Closed);
        assert_eq!(
            s.cwd.as_deref().and_then(|p| p.to_str()),
            Some("/Users/buns/.openclaw/workspace")
        );
    }
}
