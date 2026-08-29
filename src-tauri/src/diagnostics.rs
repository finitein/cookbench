//! Desktop-only assembly for the sanitized core diagnostics model.

use cookbench_core::diagnostics::{AdapterDiagnostic, DiagnosticsSnapshot, FallbackReason};

/// Builds a diagnostics payload from already-sanitized, structural facts.
/// This accepts counts, capability flags, paths, and enumerated fallbacks only;
/// callers must never pass raw parser records, prompts, commands, or settings.
pub fn collect_diagnostics(
    adapters: Vec<AdapterDiagnostic>,
    source_paths: impl IntoIterator<Item = String>,
    fallback_reasons: Vec<FallbackReason>,
    home_directory: &str,
) -> DiagnosticsSnapshot {
    DiagnosticsSnapshot::new(adapters, source_paths, fallback_reasons, home_directory)
}
