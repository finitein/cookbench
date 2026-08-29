//! Conversion for Codex's structured `update_plan` payload.

use serde_json::Value;

/// Returns completed and total steps for an `update_plan` payload. A plan is
/// deliberately treated as authoritative even when its total changes.
pub fn plan_progress(payload: &Value) -> Option<(u32, u32)> {
    let steps = payload.get("plan")?.as_array()?;
    let total = u32::try_from(steps.len()).ok()?;
    if total == 0 {
        return None;
    }
    let completed = steps
        .iter()
        .filter(|step| {
            matches!(
                step.get("status").and_then(Value::as_str),
                Some("completed") | Some("complete") | Some("done")
            )
        })
        .count();
    Some((u32::try_from(completed).ok()?, total))
}
