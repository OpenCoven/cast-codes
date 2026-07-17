//! Wire types for the Coven daemon `/api/v1/*` endpoints that are specific
//! to the Familiars catalog. Shapes match the daemon contract described in
//! `specs/castcodes-coven-internal-loop/PLAN-01-start-coding.md`
//! (`DaemonFamiliar`). Deserialization is tolerant: unknown fields are
//! ignored and absent fields default, so daemon schema drift degrades
//! gracefully instead of hard-failing the client.

use serde::Deserialize;

/// A persona entry from `GET /api/v1/familiars`. A Familiar resolves to a
/// backend harness + system prompt daemon-side; the client only displays
/// and selects it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DaemonFamiliar {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub active_sessions: u32,
    #[serde(default)]
    pub memory_freshness: String,
}

impl DaemonFamiliar {
    /// Best label for the picker: `display_name` if set, else `name`, else `id`.
    pub fn label(&self) -> &str {
        if !self.display_name.is_empty() {
            &self.display_name
        } else if !self.name.is_empty() {
            &self.name
        } else {
            &self.id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_catalog_entry() {
        let json = r#"{
            "id": "nova", "name": "nova", "display_name": "Nova",
            "emoji": "🌟", "role": "builder", "description": "Ships features",
            "status": "online", "last_seen": "2026-07-17T00:00:00Z",
            "active_sessions": 2, "memory_freshness": "fresh"
        }"#;
        let f: DaemonFamiliar = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, "nova");
        assert_eq!(f.label(), "Nova");
        assert_eq!(f.active_sessions, 2);
    }

    #[test]
    fn tolerates_missing_and_unknown_fields() {
        let json = r#"{"id": "sage", "unexpected": "ignored"}"#;
        let f: DaemonFamiliar = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, "sage");
        assert_eq!(f.label(), "sage"); // falls back to id
        assert_eq!(f.status, ""); // defaulted
    }
}
