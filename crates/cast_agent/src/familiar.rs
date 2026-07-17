//! Familiar selection: the pure resolution from a chosen Familiar id (or
//! the configured default) to the concrete `RequestTarget` sent on the
//! daemon session request, plus a cached view of the daemon catalog.

use std::sync::{Arc, RwLock};

use crate::daemon_schema::DaemonFamiliar;
use crate::gateway::GatewayClient;

/// Backend harnesses offered when the daemon persona catalog is empty.
/// Single source of truth for the fallback list; extend here.
pub const SUPPORTED_HARNESSES: &[&str] = &["coven-code", "codex", "claude"];

/// What actually goes on the daemon session request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTarget {
    /// A daemon-catalog Familiar id, sent as `familiarId`. The daemon maps
    /// it to a harness + system prompt (daemon-owned; future contract).
    Familiar(String),
    /// A concrete backend harness, sent as `harness`.
    Harness(String),
}

/// Resolve a selection to a request target.
/// Precedence: explicit `selected` (per-conversation) > `default` > the
/// first `SUPPORTED_HARNESSES` entry. A selection that matches a catalog id
/// becomes `Familiar`; otherwise it is treated as a raw harness string.
pub fn resolve(
    selected: Option<&str>,
    default: Option<&str>,
    catalog: &[DaemonFamiliar],
) -> RequestTarget {
    let choice = selected
        .filter(|s| !s.is_empty())
        .or(default.filter(|s| !s.is_empty()));
    match choice {
        Some(id) if catalog.iter().any(|f| f.id == id) => RequestTarget::Familiar(id.to_string()),
        Some(other) => RequestTarget::Harness(other.to_string()),
        None => RequestTarget::Harness(SUPPORTED_HARNESSES[0].to_string()),
    }
}

/// Cached view of the daemon Familiar catalog. Mirrors `SessionStore`:
/// `list()` refreshes from the gateway (never errors — the gateway call is
/// already graceful), `snapshot()` is a cheap sync read for the UI thread.
pub struct FamiliarStore {
    gateway: Arc<GatewayClient>,
    cache: RwLock<Vec<DaemonFamiliar>>,
}

impl FamiliarStore {
    pub fn new(gateway: Arc<GatewayClient>) -> Self {
        Self { gateway, cache: RwLock::new(Vec::new()) }
    }

    /// Refresh the cached catalog from the gateway and return it.
    pub async fn list(&self) -> Vec<DaemonFamiliar> {
        let fetched = self.gateway.list_familiars().await;
        let mut guard = self.cache.write().unwrap_or_else(|p| p.into_inner());
        *guard = fetched.clone();
        fetched
    }

    /// Sync snapshot of the cached catalog. Safe from the UI thread.
    pub fn snapshot(&self) -> Vec<DaemonFamiliar> {
        self.cache.read().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fam(id: &str) -> DaemonFamiliar {
        DaemonFamiliar {
            id: id.into(),
            name: id.into(),
            display_name: String::new(),
            emoji: String::new(),
            role: String::new(),
            description: String::new(),
            status: String::new(),
            last_seen: String::new(),
            active_sessions: 0,
            memory_freshness: String::new(),
        }
    }

    #[test]
    fn selected_catalog_id_resolves_to_familiar() {
        let cat = vec![fam("nova")];
        assert_eq!(resolve(Some("nova"), None, &cat), RequestTarget::Familiar("nova".into()));
    }

    #[test]
    fn selected_wins_over_default() {
        let cat = vec![fam("nova"), fam("sage")];
        assert_eq!(resolve(Some("sage"), Some("nova"), &cat), RequestTarget::Familiar("sage".into()));
    }

    #[test]
    fn default_used_when_no_selection() {
        let cat = vec![fam("nova")];
        assert_eq!(resolve(None, Some("nova"), &cat), RequestTarget::Familiar("nova".into()));
    }

    #[test]
    fn empty_catalog_falls_back_to_harness() {
        assert_eq!(resolve(None, None, &[]), RequestTarget::Harness("coven-code".into()));
    }

    #[test]
    fn non_catalog_selection_is_raw_harness() {
        assert_eq!(resolve(Some("codex"), None, &[]), RequestTarget::Harness("codex".into()));
    }
}
