use cookbench_core::domain::{
    EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity, Stove,
    StoveEvent, StoveIdentity, StoveState,
};
use cookbench_core::state_machine::reduce;

fn stove() -> Stove {
    Stove::new(
        StoveIdentity::new(
            HostIdentity::local("developer-machine"),
            HarnessId::Codex,
            "session-1",
        ),
        ProjectIdentity::new(HostIdentity::local("developer-machine"), "/work/cookbench"),
    )
}

fn event(sequence: u64, kind: EventKind) -> StoveEvent {
    StoveEvent::new(
        kind,
        EventMetadata::new(
            EventSource::StructuredSession,
            100,
            sequence,
            sequence * 1_000,
        ),
    )
}

#[test]
fn cooking_needs_human_cooking_cooked() {
    let cooking = reduce(&stove(), &event(1, EventKind::ToolStarted)).unwrap();
    let needs_human = reduce(&cooking, &event(2, EventKind::QuestionAsked)).unwrap();
    let resumed = reduce(&needs_human, &event(3, EventKind::ToolStarted)).unwrap();
    let cooked = reduce(&resumed, &event(4, EventKind::TurnCompleted)).unwrap();

    assert_eq!(cooking.state, StoveState::Cooking);
    assert_eq!(needs_human.state, StoveState::NeedsHuman);
    assert_eq!(resumed.state, StoveState::Cooking);
    assert_eq!(cooked.state, StoveState::Cooked);
}

#[test]
fn a_new_prompt_relights_the_same_cooked_stove() {
    let identity = stove().identity.clone();
    let cooked = reduce(&stove(), &event(1, EventKind::TurnCompleted)).unwrap();
    let relit = reduce(&cooked, &event(2, EventKind::UserPromptSubmitted)).unwrap();

    assert_eq!(relit.identity, identity);
    assert_eq!(relit.state, StoveState::Cooking);
}

#[test]
fn connection_restore_returns_to_the_previous_state() {
    let cooking = reduce(&stove(), &event(1, EventKind::ToolStarted)).unwrap();
    let disconnected = reduce(&cooking, &event(2, EventKind::ConnectionLost)).unwrap();
    let restored = reduce(&disconnected, &event(3, EventKind::ConnectionRestored)).unwrap();

    assert_eq!(disconnected.state, StoveState::Disconnected);
    assert_eq!(
        disconnected.state_before_disconnect,
        Some(StoveState::Cooking)
    );
    assert_eq!(restored.state, StoveState::Cooking);
    assert_eq!(restored.state_before_disconnect, None);
}

#[test]
fn recoverable_tool_error_does_not_fail_a_stove() {
    let cooking = reduce(&stove(), &event(1, EventKind::ToolStarted)).unwrap();
    let still_cooking = reduce(
        &cooking,
        &event(2, EventKind::ToolCompleted { succeeded: false }),
    )
    .unwrap();

    assert_eq!(still_cooking.state, StoveState::Cooking);
}

#[test]
fn final_session_failure_fails_the_stove() {
    let failed = reduce(&stove(), &event(1, EventKind::SessionFailed)).unwrap();

    assert_eq!(failed.state, StoveState::Failed);
}

#[test]
fn manual_clear_removes_only_a_cooked_stove() {
    let cooked = reduce(&stove(), &event(1, EventKind::TurnCompleted)).unwrap();
    let removed = reduce(&cooked, &event(2, EventKind::ClearRequested)).unwrap();

    assert_eq!(removed.state, StoveState::Removed);
}

#[test]
fn inactivity_does_not_imply_completion() {
    let cooking = reduce(&stove(), &event(1, EventKind::ToolStarted)).unwrap();
    let after_silence = reduce(&cooking, &event(2, EventKind::Tick)).unwrap();

    assert_eq!(after_silence.state, StoveState::Cooking);
}

#[test]
fn stale_high_authority_event_cannot_overwrite_newer_state() {
    let cooking = reduce(&stove(), &event(10, EventKind::ToolStarted)).unwrap();
    let stale_hook = StoveEvent::new(
        EventKind::TurnCompleted,
        EventMetadata::new(EventSource::Hook, 100, 9, 99_999),
    );

    let unchanged = reduce(&cooking, &stale_hook).unwrap();

    assert_eq!(unchanged.state, StoveState::Cooking);
}

#[test]
fn authority_breaks_ties_between_equally_recent_events() {
    let hook_activity = StoveEvent::new(
        EventKind::ToolStarted,
        EventMetadata::new(EventSource::Hook, 100, 10, 10_000),
    );
    let structured_completion = StoveEvent::new(
        EventKind::TurnCompleted,
        EventMetadata::new(EventSource::StructuredSession, 100, 10, 10_000),
    );
    let cooking = reduce(&stove(), &hook_activity).unwrap();
    let unchanged = reduce(&cooking, &structured_completion).unwrap();

    assert_eq!(unchanged.state, StoveState::Cooking);
}

#[test]
fn only_structured_progress_is_retained_and_plan_totals_can_change() {
    let first = reduce(
        &stove(),
        &event(
            1,
            EventKind::PlanUpdated {
                completed: 2,
                total: 5,
            },
        ),
    )
    .unwrap();
    let revised = reduce(
        &first,
        &event(
            2,
            EventKind::PlanUpdated {
                completed: 4,
                total: 7,
            },
        ),
    )
    .unwrap();
    let inferred = StoveEvent::new(
        EventKind::PlanUpdated {
            completed: 5,
            total: 6,
        },
        EventMetadata::new(EventSource::Inference, 100, 3, 3_000),
    );
    let unchanged = reduce(&revised, &inferred).unwrap();

    assert_eq!(revised.progress.unwrap().fraction(), (4, 7));
    assert_eq!(unchanged.progress.unwrap().fraction(), (4, 7));
}
