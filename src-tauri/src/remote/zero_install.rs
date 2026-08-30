//! Zero-install observation over existing system OpenSSH configuration.
//!
//! The source discovers only recent, size-bounded JSONL candidates, reads a
//! bounded suffix for each one, and emits normalized lifecycle events. It never
//! writes remote files, opens a port, deploys the bridge, or controls an agent.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use cookbench_core::{
    domain::{EventKind, EventMetadata, EventSource, HarnessId, StoveEvent, StoveIdentity},
    remote::{PollInterval, RemoteHost, RemoteSessionIdentity, SessionRoot},
};

use super::{
    reconnect::ReconnectState,
    ssh::{
        automatic_session_roots, session_paths, suffix_bytes, SshError, SshInvocation, SshRunner,
    },
};

const MAX_CANDIDATES_PER_POLL: usize = 64;

/// A content-free parser boundary. The first-party harness adapters own JSONL
/// schemas; this transport owns only secure remote observation and event order.
pub trait RemoteLifecycleParser {
    fn parse_suffix(&self, path: &str, suffix: &[u8]) -> Option<ParsedRemoteSession>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedRemoteSession {
    pub harness: HarnessId,
    pub native_session_id: String,
    pub project_root: Option<String>,
    pub events: Vec<EventKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteStoveEvent {
    pub stove: StoveIdentity,
    pub project_root: Option<String>,
    pub event: StoveEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemotePoll {
    pub events: Vec<RemoteStoveEvent>,
    pub disconnected: bool,
    pub restored: bool,
    pub transport_error: Option<String>,
}

pub struct ZeroInstallSshSource<R> {
    host: RemoteHost,
    runner: R,
    reconnect: ReconnectState,
    known_stoves: Vec<StoveIdentity>,
    observed_suffixes: Vec<(String, Vec<u8>)>,
    sequence: u64,
    sequence_counter: Option<Arc<AtomicU64>>,
    resolved_automatic_roots: Option<Vec<SessionRoot>>,
}

impl<R: SshRunner> ZeroInstallSshSource<R> {
    pub fn new(host: RemoteHost, runner: R) -> Self {
        Self {
            host,
            runner,
            reconnect: ReconnectState::default(),
            known_stoves: Vec::new(),
            observed_suffixes: Vec::new(),
            sequence: 0,
            sequence_counter: None,
            resolved_automatic_roots: None,
        }
    }

    pub fn with_sequence_counter(mut self, counter: Arc<AtomicU64>) -> Self {
        self.sequence_counter = Some(counter);
        self
    }

    pub fn host(&self) -> &RemoteHost {
        &self.host
    }

    pub fn poll_interval(&self) -> PollInterval {
        self.reconnect.next_interval()
    }

    /// Keeps manually cleared Cookbench presentation state from receiving a
    /// later transport-only disconnect. Native files remain untouched.
    pub fn forget_stove(&mut self, stove: &StoveIdentity) {
        self.known_stoves.retain(|known| known != stove);
    }

    /// Returns `true` only when a previously disconnected host has recovered.
    pub fn discover(&mut self) -> Result<(Vec<String>, bool), SshError> {
        let roots = self.discovery_roots()?;
        let result = roots.into_iter().try_fold(Vec::new(), |mut paths, root| {
            let output = self
                .runner
                .run(&SshInvocation::discover(&self.host, &root))?;
            paths.extend(session_paths(&output, &root)?);
            Ok::<_, SshError>(paths)
        });

        match result {
            Ok(paths) => {
                let restored = self.reconnect.record_success(!paths.is_empty());
                Ok((paths, restored))
            }
            Err(error) => {
                self.reconnect.record_failure(&error);
                Err(error)
            }
        }
    }

    /// Observes bounded JSONL suffixes and returns events for the app store.
    /// A transport failure becomes `ConnectionLost` for known remote Stoves;
    /// it never emits a completion event. On recovery, restoration precedes
    /// new lifecycle records so state-machine restoration is authoritative.
    pub fn observe<P: RemoteLifecycleParser>(&mut self, parser: &P) -> RemotePoll {
        let paths = match self.recent_paths() {
            Ok(paths) => paths,
            Err(error) => return self.disconnected_poll(error),
        };
        let has_recent_candidates = !paths.is_empty();

        let mut parsed = Vec::new();
        for path in paths.into_iter().take(MAX_CANDIDATES_PER_POLL) {
            let output = match SshInvocation::read_suffix(&self.host, &path)
                .and_then(|invocation| self.runner.run(&invocation))
            {
                Ok(output) => output,
                Err(error) => return self.disconnected_poll(error),
            };
            let suffix = match suffix_bytes(&output) {
                Ok(suffix) => suffix,
                Err(error) => return self.disconnected_poll(error),
            };
            let Some(appended) = self.appended_suffix(&path, suffix) else {
                continue;
            };
            if let Some(session) = parser.parse_suffix(&path, &appended) {
                parsed.push(session);
            }
        }

        let restored = self.reconnect.record_success(has_recent_candidates);
        let mut events = Vec::new();
        if restored {
            let known = self.known_stoves.clone();
            for stove in known {
                events.push(self.normalized(stove, None, EventKind::ConnectionRestored));
            }
        }
        for session in parsed {
            let Ok(identity) =
                RemoteSessionIdentity::new(&self.host, session.harness, session.native_session_id)
            else {
                continue;
            };
            let stove = identity.stove_identity();
            if !self.known_stoves.contains(&stove) {
                self.known_stoves.push(stove.clone());
            }
            for kind in session.events {
                events.push(self.normalized(stove.clone(), session.project_root.clone(), kind));
            }
        }

        RemotePoll {
            events,
            disconnected: false,
            restored,
            transport_error: None,
        }
    }

    fn recent_paths(&mut self) -> Result<Vec<String>, SshError> {
        let roots = self.discovery_roots()?;
        roots.into_iter().try_fold(Vec::new(), |mut paths, root| {
            let output = self
                .runner
                .run(&SshInvocation::discover(&self.host, &root))?;
            paths.extend(session_paths(&output, &root)?);
            Ok::<_, SshError>(paths)
        })
    }

    fn discovery_roots(&mut self) -> Result<Vec<SessionRoot>, SshError> {
        if !self.host.uses_automatic_roots() {
            return Ok(self.host.session_roots().to_vec());
        }
        if let Some(roots) = &self.resolved_automatic_roots {
            return Ok(roots.clone());
        }
        let output = self
            .runner
            .run(&SshInvocation::resolve_automatic_roots(&self.host))?;
        let roots = automatic_session_roots(&output)?;
        self.resolved_automatic_roots = Some(roots.clone());
        Ok(roots)
    }

    fn disconnected_poll(&mut self, error: SshError) -> RemotePoll {
        let changed = self.reconnect.record_failure(&error);
        let events = if changed {
            self.known_stoves
                .clone()
                .into_iter()
                .map(|stove| self.normalized(stove, None, EventKind::ConnectionLost))
                .collect()
        } else {
            Vec::new()
        };
        RemotePoll {
            events,
            disconnected: true,
            restored: false,
            transport_error: Some(error.to_string()),
        }
    }

    fn appended_suffix(&mut self, path: &str, suffix: &[u8]) -> Option<Vec<u8>> {
        if let Some((_, previous)) = self
            .observed_suffixes
            .iter_mut()
            .find(|(known_path, _)| known_path == path)
        {
            if previous.as_slice() == suffix {
                return None;
            }
            let overlap = suffix_overlap(previous, suffix);
            let appended = suffix[overlap..].to_vec();
            *previous = suffix.to_vec();
            return (!appended.is_empty()).then_some(appended);
        }
        self.observed_suffixes
            .push((path.to_owned(), suffix.to_vec()));
        Some(suffix.to_vec())
    }

    fn normalized(
        &mut self,
        stove: StoveIdentity,
        project_root: Option<String>,
        kind: EventKind,
    ) -> RemoteStoveEvent {
        let sequence = if let Some(counter) = &self.sequence_counter {
            counter.fetch_add(1, Ordering::AcqRel).saturating_add(1)
        } else {
            self.sequence = self.sequence.saturating_add(1);
            self.sequence
        };
        RemoteStoveEvent {
            stove,
            project_root,
            event: StoveEvent::new(
                kind,
                EventMetadata::new(EventSource::StructuredSession, 90, sequence, sequence),
            ),
        }
    }
}

fn suffix_overlap(previous: &[u8], current: &[u8]) -> usize {
    if previous.is_empty() || current.is_empty() {
        return 0;
    }
    let mut prefix = vec![0; current.len()];
    for index in 1..current.len() {
        let mut matched = prefix[index - 1];
        while matched > 0 && current[index] != current[matched] {
            matched = prefix[matched - 1];
        }
        if current[index] == current[matched] {
            matched += 1;
        }
        prefix[index] = matched;
    }
    let mut matched = 0;
    for byte in previous {
        while matched > 0 && *byte != current[matched] {
            matched = prefix[matched - 1];
        }
        if *byte == current[matched] {
            matched += 1;
            if matched == current.len() {
                matched = prefix[matched - 1];
            }
        }
    }
    matched
}
