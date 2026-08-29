use async_trait::async_trait;
use cookbench_core::domain::{
    EventKind, EventMetadata, EventSource, HarnessId, ProjectIdentity, StoveEvent,
};

use crate::{
    AdapterCapabilities, AdapterError, AdapterEvent, EventSink, HarnessAdapter, HostSource,
    NativeSession, ResumeAction, SessionLocator, SessionLocatorKind, WatchHandle,
};

/// Deterministic metadata-only adapter used to prove the adapter contract.
#[derive(Clone, Copy, Debug, Default)]
pub struct FixtureAdapter;

impl FixtureAdapter {
    const ID: &'static str = "fixture";

    fn fixture_session(source: &HostSource) -> Result<NativeSession, AdapterError> {
        let host = source.host().clone();
        NativeSession::new(
            host.clone(),
            HarnessId::Other(Self::ID.to_owned()),
            "fixture-session-001",
            Some(ProjectIdentity::new(host, "/sanitized/project")),
            Some("Sanitized fixture task".to_owned()),
            SessionLocator::new(
                match source {
                    HostSource::Local(_) => SessionLocatorKind::LocalPath,
                    HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
                },
                "fixtures/session-001.jsonl",
            )?,
        )
    }
}

#[async_trait]
impl HarnessAdapter for FixtureAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::Other(Self::ID.to_owned())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            discovery: true,
            watch_events: true,
            structured_progress: true,
            locator: true,
            resume: true,
        }
    }

    async fn discover(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError> {
        Ok(vec![Self::fixture_session(source)?])
    }

    async fn watch(&self, sink: EventSink) -> Result<WatchHandle, AdapterError> {
        let session = Self::fixture_session(&HostSource::local(
            cookbench_core::domain::HostIdentity::local("fixture-host"),
        ))?;
        let identity = cookbench_core::domain::StoveIdentity::new(
            session.host,
            session.harness,
            session.native_session_id,
        );
        sink.emit(AdapterEvent::new(
            identity.clone(),
            StoveEvent::new(
                EventKind::SessionDiscovered,
                EventMetadata::new(EventSource::StructuredSession, 100, 1, 1),
            ),
        ))
        .await?;
        sink.emit(AdapterEvent::new(
            identity,
            StoveEvent::new(
                EventKind::PlanUpdated {
                    completed: 1,
                    total: 3,
                },
                EventMetadata::new(EventSource::StructuredSession, 100, 2, 2),
            ),
        ))
        .await?;
        Ok(WatchHandle::new())
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == self.id()).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness != self.id() {
            return Vec::new();
        }

        vec![
            ResumeAction::OpenSessionLocation(session.locator.clone()),
            ResumeAction::SuggestedCommand {
                program: "fixture-agent".to_owned(),
                args: vec!["resume".to_owned(), session.native_session_id.clone()],
            },
        ]
    }
}
