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

use std::sync::{Arc, OnceLock};

use tokio::runtime::Runtime;

use crate::{
    agent::{AgentBackend, CastAgent},
    config::CastAgentConfig,
};

/// How often the background loop re-probes `GET /health`. Chosen to be
/// short enough that the UI pill flips within ~a minute of the gateway
/// coming back up, but long enough not to flood logs.
const HEALTH_PROBE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// Owns the dedicated tokio runtime and an initialized [`CastAgent`].
/// Cheap to clone via `Arc` — the inner runtime is shared.
pub struct CastAgentRuntime {
    agent: Arc<CastAgent>,
    handle: tokio::runtime::Handle,
    // Held to keep the runtime alive for the lifetime of this struct.
    _runtime: Arc<Runtime>,
}

impl CastAgentRuntime {
    /// Build a runtime + agent. Spawns a background thread so the runtime
    /// is multi-threaded without taking over the UI thread.
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

        Ok(Self {
            agent,
            handle,
            _runtime: runtime,
        })
    }

    /// Whether the Coven Gateway is currently reachable. Cheap, sync,
    /// safe to call on the UI thread on every render.
    pub fn is_available(&self) -> bool {
        self.agent.is_available()
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
