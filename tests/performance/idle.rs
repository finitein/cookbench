//! Deterministic structural budget checks. Run with a dedicated integration
//! harness after wiring this directory into the release performance suite.

use cookbench_core::diagnostics::{
    AdapterDiagnostic, AdapterId, CapabilitySummary, DiagnosticsSnapshot, Health,
};

#[test]
fn idle_diagnostics_payload_is_bounded_without_session_content() {
    let diagnostics = DiagnosticsSnapshot::new(
        vec![AdapterDiagnostic {
            adapter: AdapterId::Codex,
            health: Health::Healthy,
            capabilities: CapabilitySummary::default(),
            parser_error_count: 0,
        }],
        std::iter::empty(),
        vec![],
        "/home/tester",
    );

    let bytes = serde_json::to_vec(&diagnostics).expect("diagnostics serialize");
    assert!(bytes.len() < 2_048, "idle diagnostics must stay small");
}
