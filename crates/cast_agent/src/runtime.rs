//! Dedicated tokio runtime + global handle for the Cast Agent backend.
//!
//! The host (`crates/ai`, `app`) does not own a multi-threaded tokio runtime —
//! the GUI runs on its own executor. To keep cast_agent's async surface
//! usable without blocking the UI thread, we spawn a single
//! [`tokio::runtime::Runtime`] on a background OS thread at first access and
//! expose a sync `is_available()` accessor that the UI can poll on every
//! render. A periodic health probe keeps the cached availability bit fresh.
//!
//! Construction is lazy and idempotent: the first call to [`global`] spins
//! up the runtime; subsequent calls return the same `Arc`.

use std::sync::{Arc, OnceLock, RwLock};

use tokio::runtime::Runtime;

use crate::{
    agent::{AgentBackend, CastAgent},
    config::CastAgentConfig,
    session::CovenSession,
    substrate::{HostSubstrate, Substrate},
};

/// How often the background loop re-probes `GET /health`. Chosen to be
/// short enough that the UI pill flips within ~a minute of the gateway
/// coming back up, but long enough not to flood logs.
const HEALTH_PROBE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// How often the background loop re-fetches the active session list.
/// Slower than the health probe because sessions change less frequently
/// and the request is more expensive than `GET /health`.
const SESSION_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Owns the dedicated tokio runtime and an initialized [`CastAgent`].
/// Cheap to clone via `Arc` — the inner runtime is shared.
pub struct CastAgentRuntime {
    agent: Arc<CastAgent>,
    handle: tokio::runtime::Handle,
    /// Latest host-owned substrate slice pushed via
    /// [`Self::set_host_substrate`]. Read by [`Self::build_substrate`] to
    /// enrich the [`crate::agent::CastAgent::get_substrate`] output before
    /// it ships to the gateway. `std::sync::RwLock` so both the UI thread
    /// (writer) and the cast-agent runtime threads (reader) can use it
    /// without going through tokio's async lock.
    host: Arc<RwLock<HostSubstrate>>,
    // Held to keep the runtime alive for the lifetime of this struct.
    _runtime: Arc<Runtime>,
}

impl CastAgentRuntime {
    /// Build a fresh isolated runtime + agent. Spawns a background thread
    /// so the runtime is multi-threaded without taking over the UI thread.
    ///
    /// Production callers should go through [`global`] for the
    /// process-wide singleton; this constructor exists so tests can build
    /// an isolated instance without sharing state with the rest of the
    /// process. Each call spawns a fresh tokio runtime + worker threads,
    /// so don't call it on a hot path.
    pub fn new_isolated(config: Option<CastAgentConfig>) -> std::io::Result<Self> {
        Self::boot(config)
    }

    /// Internal boot. Kept private so the only way to get a process-wide
    /// runtime is via [`global`], which guarantees singleton semantics.
    fn boot(config: Option<CastAgentConfig>) -> std::io::Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("cast-agent")
                .enable_all()
                .build()?,
        );
        let handle = runtime.handle().clone();
        let agent = handle.block_on(async { Arc::new(CastAgent::new(config).await) });

        // Periodic health probe so `is_available()` reflects current state.
        let probe_agent = agent.clone();
        handle.spawn(async move {
            loop {
                tokio::time::sleep(HEALTH_PROBE_INTERVAL).await;
                probe_agent.health_probe().await;
            }
        });

        // Periodic session refresh so `sessions_snapshot()` stays current.
        // Initial fetch runs immediately so the UI has data on first render
        // (modulo network latency); subsequent fetches happen on the interval.
        let session_agent = agent.clone();
        handle.spawn(async move {
            session_agent.refresh_sessions().await;
            loop {
                tokio::time::sleep(SESSION_REFRESH_INTERVAL).await;
                session_agent.refresh_sessions().await;
            }
        });

        Ok(Self {
            agent,
            handle,
            host: Arc::new(RwLock::new(HostSubstrate::default())),
            _runtime: runtime,
        })
    }

    /// Whether the Coven Gateway is currently reachable. Cheap, sync,
    /// safe to call on the UI thread on every render.
    pub fn is_available(&self) -> bool {
        self.agent.is_available()
    }

    /// Sync snapshot of the cached Coven session list. Safe to call from
    /// the UI render thread — reads a [`std::sync::RwLock`] populated by
    /// the background refresh loop.
    pub fn sessions(&self) -> Vec<CovenSession> {
        self.agent.sessions_snapshot()
    }

    /// Display name ("Cast Agent").
    pub fn agent_name(&self) -> &'static str {
        self.agent.agent_name()
    }

    /// Underlying agent for async callers that want the full API.
    pub fn agent(&self) -> &Arc<CastAgent> {
        &self.agent
    }

    /// Tokio [`Handle`] for callers that want to spawn their own tasks
    /// on the cast-agent runtime (e.g. session click-through).
    pub fn handle(&self) -> &tokio::runtime::Handle {
        &self.handle
    }

    /// Replace the host-owned substrate slice. Called by `app/src` whenever
    /// editor focus changes, panes open/close, or LSP diagnostics arrive.
    /// Sync, never blocks on the gateway — the next time the runtime
    /// builds a [`Substrate`] for a gateway call it picks up the new
    /// values. Lock poisoning is recovered by replacing the inner state.
    pub fn set_host_substrate(&self, host: HostSubstrate) {
        let mut guard = self
            .host
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = host;
    }

    /// Patch the host-owned substrate slice in place. Holds the write
    /// lock for the closure call so callers can update one field (e.g.
    /// `active_file`) without read-modify-write races against other
    /// publishers (pane lifecycle, LSP). Sync, never blocks the runtime.
    pub fn update_host_substrate<F>(&self, f: F)
    where
        F: FnOnce(&mut HostSubstrate),
    {
        let mut guard = self
            .host
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard);
    }

    /// Snapshot the host-owned substrate slice. Useful for tests and for
    /// the UI if it wants to render whatever the host last reported
    /// (rather than re-computing).
    pub fn host_substrate(&self) -> HostSubstrate {
        self.host
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Build a [`Substrate`] for the gateway: collect the cast_agent-owned
    /// slices (shell CWD, git branch, Comux panes) via `CastAgent`, then
    /// overlay the host-pushed [`HostSubstrate`] on top.
    pub async fn build_substrate(&self) -> anyhow::Result<Substrate> {
        let mut substrate = self.agent.get_substrate().await?;
        substrate.apply_host(self.host_substrate());
        Ok(substrate)
    }
}

/// Process-wide singleton. First call spins up the runtime; later calls
/// return the same handle. Returns `None` if the runtime failed to start
/// (e.g. cannot spawn threads) — the UI should treat that as offline.
pub fn global() -> Option<&'static CastAgentRuntime> {
    static INSTANCE: OnceLock<Option<CastAgentRuntime>> = OnceLock::new();
    INSTANCE
        .get_or_init(|| match CastAgentRuntime::boot(Some(CastAgentConfig::load())) {
            Ok(rt) => Some(rt),
            Err(err) => {
                log::warn!("cast_agent: runtime failed to start: {err}");
                None
            }
        })
        .as_ref()
}

/// Sync convenience that the agent panel can call on every render.
/// Returns `false` if the runtime never started.
pub fn is_available() -> bool {
    global().map(CastAgentRuntime::is_available).unwrap_or(false)
}

/// Sync convenience for the agent panel's session list. Returns an empty
/// `Vec` if the runtime never started or the first refresh hasn't landed.
pub fn sessions() -> Vec<CovenSession> {
    global().map(CastAgentRuntime::sessions).unwrap_or_default()
}

/// Sync convenience for the host to push a fresh substrate snapshot. No-op
/// if the runtime never started.
pub fn set_host_substrate(host: HostSubstrate) {
    if let Some(rt) = global() {
        rt.set_host_substrate(host);
    }
}

/// Sync convenience for the host to patch one or more substrate fields
/// without losing the others. No-op if the runtime never started.
pub fn update_host_substrate<F>(f: F)
where
    F: FnOnce(&mut HostSubstrate),
{
    if let Some(rt) = global() {
        rt.update_host_substrate(f);
    }
}
