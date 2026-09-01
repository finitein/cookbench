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

    let mut different_source = completed.clone();
    different_source.last_event.as_mut().unwrap().source = EventSource::Process;
    assert!(!cursor.acknowledges(&different_source));

    let mut different_confidence = completed.clone();
    different_confidence.last_event.as_mut().unwrap().confidence = 99;
    assert!(!cursor.acknowledges(&different_confidence));

    let mut different_sequence = completed.clone();
    different_sequence.last_event.as_mut().unwrap().sequence = 101;
    assert!(!cursor.acknowledges(&different_sequence));

    let mut different_timestamp = completed.clone();
    different_timestamp
        .last_event
        .as_mut()
        .unwrap()
        .timestamp_ms = 101;
    assert!(!cursor.acknowledges(&different_timestamp));

    let different_locator = stove("different-session", StoveState::Cooked, 100);
    assert!(!cursor.acknowledges(&different_locator));
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

#[test]
fn starting_planning_and_cooking_share_the_active_attention_rank() {
    let starting = stove("starting", StoveState::Starting, 10);
    let planning = stove("planning", StoveState::Planning, 20);
    let cooking = stove("cooking", StoveState::Cooking, 30);

    assert_eq!(
        ordered_stove_ids(&[starting.clone(), planning.clone(), cooking.clone()], &[]),
        vec![cooking.identity, planning.identity, starting.identity]
    );
}

#[test]
fn identity_tiebreaker_covers_host_harness_other_and_session() {
    let local_codex_alpha = stove("alpha", StoveState::Cooking, 10);
    let local_codex_beta = stove("beta", StoveState::Cooking, 10);
    let mut local_claude = stove("alpha", StoveState::Cooking, 10);
    local_claude.identity.harness = HarnessId::ClaudeCode;
    let mut local_other_a = stove("alpha", StoveState::Cooking, 10);
    local_other_a.identity.harness = HarnessId::Other("a".to_owned());
    let mut local_other_z = stove("alpha", StoveState::Cooking, 10);
    local_other_z.identity.harness = HarnessId::Other("z".to_owned());
    let mut ssh_codex = stove("alpha", StoveState::Cooking, 10);
    ssh_codex.identity.host = HostIdentity::ssh("test-machine");

    assert_eq!(
        ordered_stove_ids(
            &[
                ssh_codex.clone(),
                local_other_z.clone(),
                local_codex_beta.clone(),
                local_claude.clone(),
                local_other_a.clone(),
                local_codex_alpha.clone(),
            ],
            &[],
        ),
        vec![
            local_codex_alpha.identity,
            local_codex_beta.identity,
            local_claude.identity,
            local_other_a.identity,
            local_other_z.identity,
            ssh_codex.identity,
        ]
    );
}
