//! Sanitized operational diagnostics.
//!
//! Diagnostics are intentionally summary-only. They describe Cookbench's own
//! health without retaining harness conversations, command text, credentials,
//! webhook endpoints, or SSH material.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterId {
    Codex,
    ClaudeCode,
    Pi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Health {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySummary {
    pub discovery: bool,
    pub watch_events: bool,
    pub structured_progress: bool,
    pub locator: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDiagnostic {
    pub adapter: AdapterId,
    pub health: Health,
    pub capabilities: CapabilitySummary,
    pub parser_error_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FallbackReason {
    BlurUnavailable,
    WaylandBestEffortOverlay,
    ExactFocusUnavailable,
    FocusPermissionDenied,
    RemoteSourceDisconnected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub adapters: Vec<AdapterDiagnostic>,
    pub source_paths: Vec<String>,
    pub fallback_reasons: Vec<FallbackReason>,
}

impl DiagnosticsSnapshot {
    pub const MAX_ADAPTERS: usize = 3;
    pub const MAX_SOURCE_PATHS: usize = 32;
    pub const MAX_FALLBACK_REASONS: usize = 16;

    pub fn new(
        adapters: Vec<AdapterDiagnostic>,
        source_paths: impl IntoIterator<Item = String>,
        fallback_reasons: Vec<FallbackReason>,
        home_directory: &str,
    ) -> Self {
        Self {
            adapters: adapters.into_iter().take(Self::MAX_ADAPTERS).collect(),
            source_paths: source_paths
                .into_iter()
                .filter_map(|path| redact_source_path(&path, home_directory))
                .take(Self::MAX_SOURCE_PATHS)
                .collect(),
            fallback_reasons: fallback_reasons
                .into_iter()
                .take(Self::MAX_FALLBACK_REASONS)
                .collect(),
        }
    }
}

/// Redacts the local home prefix and rejects paths that could disclose secrets.
/// A rejected path is omitted from diagnostics rather than represented verbatim.
pub fn redact_source_path(path: &str, home_directory: &str) -> Option<String> {
    if path.is_empty() || looks_sensitive(path) {
        return None;
    }

    let path = path.replace('\\', "/");
    let home = home_directory
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned();
    if home.is_empty() {
        return Some(path);
    }

    if path == home {
        return Some("~".to_owned());
    }
    if let Some(suffix) = path.strip_prefix(&home) {
        if suffix.starts_with('/') {
            return Some(format!("~{suffix}"));
        }
    }
    Some(path)
}

fn looks_sensitive(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    value.chars().any(char::is_control)
        || lowered.contains("://")
        || lowered.contains(".ssh/")
        || [
            "credential",
            "password",
            "secret",
            "token",
            "webhook",
            "id_rsa",
            "id_ed25519",
        ]
        .iter()
        .any(|needle| lowered.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{
        redact_source_path, AdapterDiagnostic, AdapterId, CapabilitySummary, DiagnosticsSnapshot,
        FallbackReason, Health,
    };

    #[test]
    fn snapshot_redacts_home_paths_and_omits_sensitive_values() {
        let snapshot = DiagnosticsSnapshot::new(
            vec![AdapterDiagnostic {
                adapter: AdapterId::Codex,
                health: Health::Healthy,
                capabilities: CapabilitySummary::default(),
                parser_error_count: 2,
            }],
            [
                "/Users/alex/.codex/sessions".to_owned(),
                "/Users/alex/.ssh/id_ed25519".to_owned(),
                "https://hooks.example.invalid/private".to_owned(),
            ],
            vec![],
            "/Users/alex",
        );

        assert_eq!(snapshot.source_paths, vec!["~/.codex/sessions".to_owned()]);
        let encoded = serde_json::to_string(&snapshot).expect("diagnostics serialize");
        assert!(!encoded.contains("alex"));
        assert!(!encoded.contains("hooks.example"));
    }

    #[test]
    fn malformed_path_corpus_never_returns_sensitive_text() {
        for value in [
            "\0",
            "ssh://private-host",
            "/home/dev/.ssh/config",
            "/home/dev/.config/credentials.json",
            "/tmp/webhook-token.txt",
            "/safe/project",
        ] {
            let result = redact_source_path(value, "/home/dev");
            assert!(result.as_deref().is_none_or(|path| !path.contains(".ssh")));
            assert!(result
                .as_deref()
                .is_none_or(|path| !path.contains("credential")));
            assert!(result
                .as_deref()
                .is_none_or(|path| !path.contains("webhook")));
        }
    }

    #[test]
    fn snapshot_caps_every_collection() {
        let snapshot = DiagnosticsSnapshot::new(
            Vec::new(),
            (0..100).map(|index| format!("/safe/{index}")),
            vec![FallbackReason::BlurUnavailable; 100],
            "/home/dev",
        );

        assert_eq!(
            snapshot.source_paths.len(),
            DiagnosticsSnapshot::MAX_SOURCE_PATHS
        );
        assert_eq!(
            snapshot.fallback_reasons.len(),
            DiagnosticsSnapshot::MAX_FALLBACK_REASONS
        );
    }
}
