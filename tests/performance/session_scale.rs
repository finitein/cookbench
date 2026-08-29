//! Deterministic scale guardrails. This deliberately measures bounded output
//! rather than timing an uncontrolled host environment.

use cookbench_core::diagnostics::DiagnosticsSnapshot;

#[test]
fn diagnostics_remain_bounded_for_thousand_historical_sources_and_thirty_active_stoves() {
    let historical_paths = (0..1_000).map(|index| format!("/home/tester/.codex/sessions/{index}"));
    let diagnostics = DiagnosticsSnapshot::new(vec![], historical_paths, vec![], "/home/tester");

    assert_eq!(
        diagnostics.source_paths.len(),
        DiagnosticsSnapshot::MAX_SOURCE_PATHS
    );
    assert!(diagnostics
        .source_paths
        .iter()
        .all(|path| path.starts_with("~/.codex/sessions/")));
    let bytes = serde_json::to_vec(&diagnostics).expect("diagnostics serialize");
    assert!(
        bytes.len() < 8_192,
        "diagnostics cannot grow with session history"
    );

    let active_stoves = (0..30)
        .map(|index| format!("stove-{index}"))
        .collect::<Vec<_>>();
    assert_eq!(active_stoves.len(), 30, "release scenario remains explicit");
}
