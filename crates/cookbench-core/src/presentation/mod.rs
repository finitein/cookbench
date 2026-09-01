//! Pure presentation ordering for the Stove views.
//!
//! This module intentionally knows nothing about windows or native menus so
//! every presentation surface uses the same attention rules.

use std::cmp::Ordering;

use crate::{
    domain::{HarnessId, HostKind, Stove, StoveIdentity, StoveState},
    persistence::CookedAttentionCursor,
};

/// Returns Stove identities ordered by the current level of user attention.
///
/// Equal priorities use the most recent event first and a canonical Stove
/// identity comparison as the final tie breaker. The result is therefore
/// stable even when observation order changes or metadata is missing.
pub fn ordered_stove_ids(
    stoves: &[Stove],
    cooked_attention_cursors: &[CookedAttentionCursor],
) -> Vec<StoveIdentity> {
    let mut ordered: Vec<&Stove> = stoves.iter().collect();
    ordered.sort_by(|left, right| {
        attention_rank(left, cooked_attention_cursors)
            .cmp(&attention_rank(right, cooked_attention_cursors))
            .then_with(|| event_timestamp(right).cmp(&event_timestamp(left)))
            .then_with(|| compare_identity(&left.identity, &right.identity))
    });
    ordered
        .into_iter()
        .map(|stove| stove.identity.clone())
        .collect()
}

fn attention_rank(stove: &Stove, cooked_attention_cursors: &[CookedAttentionCursor]) -> u8 {
    match stove.state {
        StoveState::NeedsHuman => 0,
        StoveState::Failed => 1,
        StoveState::Disconnected => 2,
        StoveState::Cooked
            if !cooked_attention_cursors
                .iter()
                .any(|cursor| cursor.acknowledges(stove)) =>
        {
            3
        }
        StoveState::Starting | StoveState::Planning | StoveState::Cooking => 4,
        StoveState::Cooked => 5,
        StoveState::Removed => 6,
    }
}

fn event_timestamp(stove: &Stove) -> u64 {
    stove
        .last_event
        .as_ref()
        .map_or(0, |event| event.timestamp_ms)
}

fn compare_identity(left: &StoveIdentity, right: &StoveIdentity) -> Ordering {
    host_kind_order(&left.host.kind)
        .cmp(&host_kind_order(&right.host.kind))
        .then_with(|| left.host.id.cmp(&right.host.id))
        .then_with(|| compare_harness(&left.harness, &right.harness))
        .then_with(|| left.native_session_id.cmp(&right.native_session_id))
}

fn host_kind_order(kind: &HostKind) -> u8 {
    match kind {
        HostKind::Local => 0,
        HostKind::Ssh => 1,
    }
}

fn compare_harness(left: &HarnessId, right: &HarnessId) -> Ordering {
    harness_order(left)
        .cmp(&harness_order(right))
        .then_with(|| match (left, right) {
            (HarnessId::Other(left), HarnessId::Other(right)) => left.cmp(right),
            _ => Ordering::Equal,
        })
}

fn harness_order(harness: &HarnessId) -> u8 {
    match harness {
        HarnessId::Codex => 0,
        HarnessId::ClaudeCode => 1,
        HarnessId::Pi => 2,
        HarnessId::Other(_) => 3,
    }
}
