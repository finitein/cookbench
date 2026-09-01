use cookbench_core::{
    domain::{
        EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity, Stove, StoveIdentity,
        StoveState,
    },
    persistence::CookedAttentionCursor,
    presentation::ordered_stove_ids,
};

fn stove(id: &str, state: StoveState, timestamp_ms: u64) -> Stove {
    let identity = StoveIdentity::new(HostIdentity::local("test-machine"), HarnessId::Codex, id);
    Stove {
        project: ProjectIdentity::new(HostIdentity::local("test-machine"), "/synthetic/project"),
        identity,
        state,
        progress: None,
        state_before_disconnect: None,
        last_event: Some(EventMetadata::new(
            EventSource::Hook,
            100,
            timestamp_ms,
            timestamp_ms,
        )),
    }
}

#[test]
fn attention_orders_statuses_then_recent_events_then_identity() {
    let needs_human = stove("needs-human", StoveState::NeedsHuman, 1);
    let failed = stove("failed", StoveState::Failed, 100);
    let disconnected = stove("disconnected", StoveState::Disconnected, 100);
    let cooked_newer = stove("cooked-newer", StoveState::Cooked, 90);
    let cooked_older = stove("cooked-older", StoveState::Cooked, 10);
    let cooking = stove("cooking", StoveState::Cooking, 100);
    let acknowledged = stove("acknowledged", StoveState::Cooked, 100);
    let mut cursor = CookedAttentionCursor::from_stove(&acknowledged).unwrap();
    cursor.acknowledged_at_ms = 101;

    let ordered = ordered_stove_ids(
        &[
            cooking.clone(),
            cooked_older.clone(),
            failed.clone(),
            acknowledged.clone(),
            disconnected.clone(),
            needs_human.clone(),
            cooked_newer.clone(),
        ],
        &[cursor],
    );

    assert_eq!(
        ordered,
        vec![
            needs_human.identity,
            failed.identity,
            disconnected.identity,
            cooked_newer.identity,
            cooked_older.identity,
            cooking.identity,
            acknowledged.identity,
        ]
    );
}

#[test]
fn acknowledgement_applies_only_to_the_exact_cooked_completion() {
    let completed = stove("same-session", StoveState::Cooked, 100);
    let cursor = CookedAttentionCursor::from_stove(&completed).unwrap();
    assert!(cursor.acknowledges(&completed));

    let relit_and_completed = stove("same-session", StoveState::Cooked, 101);
    assert!(!cursor.acknowledges(&relit_and_completed));

    let cooking = stove("same-session", StoveState::Cooking, 101);
    assert!(!cursor.acknowledges(&cooking));
}

#[test]
fn missing_events_and_equal_timestamps_have_a_stable_identity_tiebreaker() {
    let mut first = stove("alpha", StoveState::Cooking, 10);
    first.last_event = None;
    let mut second = stove("beta", StoveState::Cooking, 10);
    second.last_event = None;
    let same_timestamp = stove("aardvark", StoveState::Cooking, 0);

    assert_eq!(
        ordered_stove_ids(
            &[second.clone(), first.clone(), same_timestamp.clone()],
            &[]
        ),
        vec![same_timestamp.identity, first.identity, second.identity]
    );
}
