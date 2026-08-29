use std::sync::Arc;

use cookbench_adapters::{
    AdapterRegistry, FixtureAdapter, HarnessAdapter, HostSource, RegistryError, ResumeAction,
    SessionLocatorKind,
};
use cookbench_core::domain::{EventKind, HostIdentity};
use tokio::sync::mpsc;

#[tokio::test]
async fn fixture_adapter_reports_each_supported_capability_and_obeys_it() {
    let adapter = FixtureAdapter;
    let source = HostSource::local(HostIdentity::local("fixture-host"));
    let capabilities = adapter.capabilities();

    assert!(capabilities.discovery);
    assert!(capabilities.watch_events);
    assert!(capabilities.structured_progress);
    assert!(capabilities.locator);
    assert!(capabilities.resume);

    let sessions = adapter.discover(&source).await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].host, HostIdentity::local("fixture-host"));
    assert_eq!(sessions[0].title.as_deref(), Some("Sanitized fixture task"));
    assert_eq!(sessions[0].locator.kind, SessionLocatorKind::LocalPath);

    let (sender, mut receiver) = mpsc::channel(4);
    let handle = adapter.watch(sender.into()).await.unwrap();
    let discovered = receiver.recv().await.unwrap();
    let progress = receiver.recv().await.unwrap();
    assert_eq!(discovered.stove.native_session_id, "fixture-session-001");
    assert!(matches!(
        discovered.event.kind,
        EventKind::SessionDiscovered
    ));
    assert!(matches!(
        progress.event.kind,
        EventKind::PlanUpdated {
            completed: 1,
            total: 3
        }
    ));
    assert!(!handle.is_cancelled());
    handle.cancel();
    assert!(handle.is_cancelled());

    assert_eq!(
        adapter.locate(&sessions[0]),
        Some(sessions[0].locator.clone())
    );
    assert!(adapter
        .resume(&sessions[0])
        .iter()
        .any(|action| matches!(action, ResumeAction::SuggestedCommand { .. })));

    let remote = adapter
        .discover(&HostSource::ssh(HostIdentity::ssh("fixture-ssh-host")))
        .await
        .unwrap();
    assert_eq!(remote[0].host, HostIdentity::ssh("fixture-ssh-host"));
    assert_eq!(remote[0].locator.kind, SessionLocatorKind::RemotePath);
}

#[test]
fn registry_rejects_duplicate_adapter_ids() {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(FixtureAdapter)).unwrap();

    let error = registry.register(Arc::new(FixtureAdapter)).unwrap_err();
    assert!(matches!(error, RegistryError::DuplicateAdapterId(_)));
}
