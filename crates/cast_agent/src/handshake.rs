//! The mandatory `coven.daemon.v1` client handshake.
//!
//! `docs/CLIENT-INTEGRATION.md` in OpenCoven/coven requires every client
//! to treat the handshake as mandatory before any other request:
//!
//! 1. `GET /api/v1/health`.
//! 2. Confirm `apiVersion == "coven.daemon.v1"`.
//! 3. Read the `capabilities` block for the fields the client needs.
//!
//! Skipping it means depending on undefined response shapes from a future
//! daemon version. This module owns the response shapes and the pure
//! classification logic; the transport lives in `gateway`.

use serde::Deserialize;

/// The daemon API contract version CastCodes speaks.
pub const DAEMON_API_VERSION: &str = "coven.daemon.v1";

/// Machine-readable capability flags from `GET /api/v1/health`.
///
/// Unknown fields are ignored (additive daemon changes must not break the
/// client); missing fields default to `false`/empty so an older daemon
/// degrades to "capability absent", never to a parse error.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DaemonCapabilities {
    /// Sessions API (`/sessions`, `/sessions/:id`) is available.
    pub sessions: bool,
    /// Events API (`/events`) is available.
    pub events: bool,
    /// Travel profile/delta/state APIs are available.
    pub travel: bool,
    /// Scheduler decision and recovery APIs are available.
    pub scheduler: bool,
    /// Hub control-plane APIs are available.
    pub hub: bool,
    /// Hub-outbound executor poll/dispatch APIs are available.
    pub executor_dispatch: bool,
    /// Cursor type supported; `"sequence"` means `afterSeq` is stable.
    pub event_cursor: String,
    /// All errors use the `{ error: { code, message, details } }` shape.
    pub structured_errors: bool,
}

/// Daemon process metadata from the `daemon` block of the health payload.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DaemonProcessInfo {
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub socket: Option<String>,
}

/// Parsed `GET /api/v1/health` response.
///
/// `api_version` is `None` for the legacy TCP bridge, which predates the
/// versioned contract — version enforcement only applies to the Unix
/// daemon transport.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DaemonHealth {
    pub ok: bool,
    pub api_version: Option<String>,
    pub coven_version: Option<String>,
    pub capabilities: DaemonCapabilities,
    pub daemon: Option<DaemonProcessInfo>,
}

impl DaemonHealth {
    /// Health value used for a successful probe of the legacy TCP bridge,
    /// which has no versioned contract. Capabilities are conservatively
    /// empty — callers needing daemon capabilities should be on Unix.
    pub fn legacy_bridge() -> Self {
        Self {
            ok: true,
            ..Self::default()
        }
    }
}

/// Connection state produced by the handshake. This is the authoritative
/// availability signal for all UI surfaces: anything other than
/// [`ConnectionState::Ready`] means "do not issue further requests".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ConnectionState {
    /// No probe has completed yet (startup).
    #[default]
    Unknown,
    /// Transport-level failure: socket missing, connection refused,
    /// timeout, or non-2xx health status.
    Unreachable,
    /// The daemon answered health but does not speak
    /// [`DAEMON_API_VERSION`]. `api_version` is whatever it reported
    /// (empty string when it reported none).
    Incompatible { api_version: String },
    /// Handshake succeeded; requests may proceed.
    Ready(DaemonHealth),
}

impl ConnectionState {
    pub fn is_ready(&self) -> bool {
        matches!(self, ConnectionState::Ready(_))
    }

    /// Capabilities when ready; `None` otherwise.
    pub fn capabilities(&self) -> Option<&DaemonCapabilities> {
        match self {
            ConnectionState::Ready(health) => Some(&health.capabilities),
            _ => None,
        }
    }
}

/// Classify a `GET /api/v1/health` response from the Unix daemon
/// transport. Pure so it can be unit-tested without a socket.
///
/// - non-2xx → [`ConnectionState::Unreachable`] (the daemon is not
///   serving its contract);
/// - unparseable body → [`ConnectionState::Incompatible`] (something
///   answered, but not a `coven.daemon.v1` daemon);
/// - parsed but wrong/missing `apiVersion` → [`ConnectionState::Incompatible`];
/// - parsed and matching → [`ConnectionState::Ready`].
pub fn classify_health_response(status: u16, body: &[u8]) -> ConnectionState {
    if !(200..300).contains(&status) {
        return ConnectionState::Unreachable;
    }
    let Ok(health) = serde_json::from_slice::<DaemonHealth>(body) else {
        return ConnectionState::Incompatible {
            api_version: String::new(),
        };
    };
    match health.api_version.as_deref() {
        Some(DAEMON_API_VERSION) => ConnectionState::Ready(health),
        other => ConnectionState::Incompatible {
            api_version: other.unwrap_or_default().to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTRACT_HEALTH: &str = r#"{
        "ok": true,
        "apiVersion": "coven.daemon.v1",
        "covenVersion": "0.1.6",
        "capabilities": {
            "sessions": true,
            "events": true,
            "travel": true,
            "scheduler": true,
            "hub": true,
            "executorDispatch": true,
            "eventCursor": "sequence",
            "structuredErrors": true
        },
        "daemon": { "pid": 40184, "startedAt": "2026-07-20T02:43:40Z", "socket": "/tmp/coven.sock" },
        "hub": { "role": "hub", "hubId": "hub_x", "nodesTotal": 0, "nodesAvailable": 0 }
    }"#;

    #[test]
    fn matching_version_is_ready_with_capabilities() {
        let state = classify_health_response(200, CONTRACT_HEALTH.as_bytes());
        let ConnectionState::Ready(health) = state else {
            panic!("expected Ready, got {state:?}");
        };
        assert!(health.ok);
        assert_eq!(health.api_version.as_deref(), Some(DAEMON_API_VERSION));
        assert_eq!(health.coven_version.as_deref(), Some("0.1.6"));
        assert!(health.capabilities.sessions);
        assert!(health.capabilities.events);
        assert!(health.capabilities.structured_errors);
        assert_eq!(health.capabilities.event_cursor, "sequence");
        let daemon = health.daemon.expect("daemon block");
        assert_eq!(daemon.pid, Some(40184));
    }

    #[test]
    fn future_version_is_incompatible() {
        let body = br#"{ "ok": true, "apiVersion": "coven.daemon.v2" }"#;
        assert_eq!(
            classify_health_response(200, body),
            ConnectionState::Incompatible {
                api_version: "coven.daemon.v2".into()
            }
        );
    }

    #[test]
    fn missing_version_is_incompatible() {
        // Something is serving the socket, but it never named the
        // contract — treat as incompatible, not ready.
        let body = br#"{ "ok": true }"#;
        assert_eq!(
            classify_health_response(200, body),
            ConnectionState::Incompatible {
                api_version: String::new()
            }
        );
    }

    #[test]
    fn unparseable_body_is_incompatible() {
        assert_eq!(
            classify_health_response(200, b"<html>not a daemon</html>"),
            ConnectionState::Incompatible {
                api_version: String::new()
            }
        );
    }

    #[test]
    fn non_2xx_is_unreachable() {
        assert_eq!(
            classify_health_response(503, b"{}"),
            ConnectionState::Unreachable
        );
    }

    #[test]
    fn unknown_additive_fields_are_ignored() {
        // A future daemon may add fields anywhere; the client must not
        // fail closed on additive changes (per the compatibility policy).
        let body = br#"{
            "ok": true,
            "apiVersion": "coven.daemon.v1",
            "capabilities": { "sessions": true, "newThing": "yes" },
            "futureTopLevel": { "x": 1 }
        }"#;
        assert!(classify_health_response(200, body).is_ready());
    }

    #[test]
    fn missing_capabilities_default_to_absent() {
        let body = br#"{ "ok": true, "apiVersion": "coven.daemon.v1" }"#;
        let state = classify_health_response(200, body);
        let caps = state.capabilities().expect("ready");
        assert!(!caps.sessions);
        assert!(!caps.structured_errors);
        assert!(caps.event_cursor.is_empty());
    }
}
