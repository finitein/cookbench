use cookbench_core::domain::{HarnessId, HostIdentity, StoveState};
use cookbench_core::notifications::{
    evaluate, BoundedQueue, DestinationId, NotificationContext, NotificationEventKind,
    NotificationRule, NotificationSettings, QueueItem, RuleScope, Template, TemplateContext,
};

fn context() -> NotificationContext {
    NotificationContext {
        stove_id: "stove-1".into(),
        project: "cookbench".into(),
        host: HostIdentity::local("laptop"),
        harness: HarnessId::Codex,
        destination: DestinationId::new("team"),
        state: StoveState::Cooked,
        event: NotificationEventKind::Cooked,
        progress_percent: None,
        milestone: None,
        task: Some("Ship v1".into()),
        agent: Some("codex".into()),
        activity: Some("completed".into()),
        duration: Some("4m".into()),
        completed_at: Some("12:00".into()),
    }
}

#[test]
fn scoped_rules_override_global_in_deterministic_order() {
    let settings = NotificationSettings {
        global: NotificationRule::enabled_for([NotificationEventKind::Cooked]),
        rules: vec![
            NotificationRule::for_scope(RuleScope::Project("cookbench".into())).with_enabled(false),
            NotificationRule::for_scope(RuleScope::Host("laptop".into())).with_enabled(true),
            NotificationRule::for_scope(RuleScope::Harness(HarnessId::Codex)).with_enabled(false),
            NotificationRule::for_scope(RuleScope::Destination(DestinationId::new("team")))
                .with_enabled(true),
            NotificationRule::for_scope(RuleScope::Stove("stove-1".into())).with_enabled(false),
        ],
    };

    assert!(!evaluate(&settings, &context()).should_notify);
}

#[test]
fn milestone_events_only_pass_configured_milestones() {
    let settings = NotificationSettings {
        global: NotificationRule::enabled_for([NotificationEventKind::ProgressMilestone])
            .with_milestones([25, 75]),
        rules: vec![],
    };
    let mut event = context();
    event.event = NotificationEventKind::ProgressMilestone;
    event.state = StoveState::Cooking;
    event.milestone = Some(50);
    assert!(!evaluate(&settings, &event).should_notify);
    event.milestone = Some(75);
    assert!(evaluate(&settings, &event).should_notify);
}

#[test]
fn templates_reject_private_or_unknown_placeholders_and_bound_output() {
    assert!(Template::parse("{project} {prompt}").is_err());
    assert!(Template::parse("{project} {unknown}").is_err());
    let template = Template::parse("{project}: {state} {progress}").unwrap();
    let rendered = template
        .render(&TemplateContext::from(&context()), 18)
        .unwrap();
    assert_eq!(rendered, "cookbench: Cooked ");
    assert!(Template::parse("{project}")
        .unwrap()
        .render(&TemplateContext::from(&context()), 0)
        .is_err());
}

#[test]
fn dedupe_coalesces_and_prioritizes_critical_events_in_a_bounded_queue() {
    let mut queue = BoundedQueue::new(2, 100, 2);
    let cooking = QueueItem::new(context(), "one".into(), 0);
    assert!(queue.push(cooking.clone()).is_enqueued());
    assert!(queue.push(cooking).is_deduplicated());

    let mut second = context();
    second.stove_id = "stove-2".into();
    second.event = NotificationEventKind::NeedsHuman;
    second.state = StoveState::NeedsHuman;
    assert!(queue
        .push(QueueItem::new(second, "two".into(), 1))
        .is_enqueued());

    let mut critical = context();
    critical.stove_id = "stove-3".into();
    critical.event = NotificationEventKind::Failed;
    critical.state = StoveState::Failed;
    assert!(queue
        .push(QueueItem::new(critical, "three".into(), 2))
        .is_enqueued());
    assert_eq!(
        queue.pop_ready(2).unwrap().context.event,
        NotificationEventKind::Failed
    );
}

#[test]
fn queue_coalesces_rapid_transitions_and_expires_retries() {
    let mut queue = BoundedQueue::new(4, 10, 1);
    let mut first = context();
    first.event = NotificationEventKind::CookingStarted;
    first.state = StoveState::Cooking;
    assert!(queue
        .push(QueueItem::new(first, "started".into(), 0))
        .is_enqueued());
    assert!(queue
        .push(QueueItem::new(context(), "cooked".into(), 5))
        .is_coalesced());
    assert_eq!(queue.len(), 1);
    assert_eq!(
        queue.pop_ready(5).unwrap().context.event,
        NotificationEventKind::Cooked
    );

    let mut retry = context();
    retry.stove_id = "retry-stove".into();
    let item = QueueItem::new(retry, "retry".into(), 10);
    assert!(queue.push(item).is_enqueued());
    assert!(queue.record_retry_failure("retry-stove", 11));
    assert!(queue.pop_ready(22).is_none());

    let mut delayed_queue = BoundedQueue::new(2, 5_000, 2);
    let delayed = QueueItem::new(context(), "delayed".into(), 100);
    assert!(delayed_queue.requeue_failed(delayed, 100));
    assert!(delayed_queue.pop_ready(1_099).is_none());
    assert!(delayed_queue.pop_ready(1_100).is_some());
}
