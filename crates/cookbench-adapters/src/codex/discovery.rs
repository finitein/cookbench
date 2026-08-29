use std::{
    env,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use cookbench_core::domain::{
    EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity, StoveEvent,
    StoveIdentity,
};

use crate::io::{discover_jsonl_files, JsonlTailer, TailLimits, TailRecord};
use crate::{
    AdapterCapabilities, AdapterError, AdapterEvent, EventSink, HarnessAdapter, HostSource,
    NativeSession, ResumeAction, SessionLocator, SessionLocatorKind, WatchHandle,
};

use super::parser::parse_record;

/// Read-only Codex adapter. `root` is the `sessions` directory, never the
/// broader home directory, which keeps recursive discovery tightly bounded.
#[derive(Clone, Debug)]
pub struct CodexAdapter {
    root: PathBuf,
    limits: TailLimits,
    local_host: HostIdentity,
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new(HostIdentity::local("localhost"))
    }
}

impl CodexAdapter {
    pub fn new(local_host: HostIdentity) -> Self {
        Self::with_root(default_codex_home().join("sessions"), local_host)
    }

    pub fn with_root(root: impl Into<PathBuf>, local_host: HostIdentity) -> Self {
        Self {
            root: root.into(),
            limits: TailLimits::default(),
            local_host,
        }
    }

    pub fn with_limits(mut self, limits: TailLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn sessions_for(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let paths = discover_jsonl_files(&self.root)
            .map_err(|error| AdapterError::Message(error.to_string()))?;
        paths
            .into_iter()
            .filter_map(|path| self.session_from_path(source, &path).transpose())
            .collect()
    }

    /// Reads bounded structural metadata for one already-selected native path.
    /// Callers should filter candidates by metadata before invoking this method.
    pub fn session_from_path(
        &self,
        source: &HostSource,
        path: &Path,
    ) -> Result<Option<NativeSession>, AdapterError> {
        let fallback_id = path
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned);
        let mut tailer = JsonlTailer::open(&self.root, path, self.limits)
            .map_err(|error| AdapterError::Message(error.to_string()))?;
        let mut session_id = None;
        let mut cwd = None;
        for sequence in 1..=1 {
            let records = tailer
                .poll()
                .map_err(|error| AdapterError::Message(error.to_string()))?;
            if records.is_empty() {
                break;
            }
            for record in records {
                if let TailRecord::Record(line) = record {
                    if let Some(parsed) = parse_record(
                        &line,
                        sequence,
                        self.limits.max_json_nesting,
                        self.limits.max_json_field_bytes,
                    ) {
                        session_id = session_id.or(parsed.session_id);
                        cwd = cwd.or(parsed.cwd);
                    }
                }
            }
            if session_id.is_some() {
                break;
            }
        }
        let Some(native_session_id) = session_id.or(fallback_id) else {
            return Ok(None);
        };
        let host = source.host().clone();
        let project = cwd.map(|cwd| ProjectIdentity::new(host.clone(), cwd));
        let locator_kind = match source {
            HostSource::Local(_) => SessionLocatorKind::LocalPath,
            HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
        };
        let locator = SessionLocator::new(locator_kind, path.to_string_lossy())?;
        NativeSession::new(
            host,
            HarnessId::Codex,
            native_session_id,
            project,
            None,
            locator,
        )
        .map(Some)
    }

    async fn emit_snapshot(&self, sink: &EventSink) -> Result<(), AdapterError> {
        let source = HostSource::local(self.local_host.clone());
        for session in self.sessions_for(&source)? {
            let stove = StoveIdentity::new(
                session.host.clone(),
                HarnessId::Codex,
                session.native_session_id.clone(),
            );
            sink.emit(AdapterEvent::new(
                stove.clone(),
                StoveEvent::new(
                    EventKind::SessionDiscovered,
                    EventMetadata::new(EventSource::StructuredSession, 100, 0, 0),
                ),
            ))
            .await?;
            let mut tailer =
                JsonlTailer::open(&self.root, Path::new(&session.locator.value), self.limits)
                    .map_err(|error| AdapterError::Message(error.to_string()))?;
            let mut sequence = 0;
            loop {
                let records = tailer
                    .poll()
                    .map_err(|error| AdapterError::Message(error.to_string()))?;
                if records.is_empty() {
                    break;
                }
                for record in records {
                    sequence += 1;
                    if let TailRecord::Record(line) = record {
                        if let Some(parsed) = parse_record(
                            &line,
                            sequence,
                            self.limits.max_json_nesting,
                            self.limits.max_json_field_bytes,
                        ) {
                            if let Some(event) = parsed.event {
                                sink.emit(AdapterEvent::new(stove.clone(), event)).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Resolves Codex's home directory without creating it. `CODEX_HOME` is an
/// explicit override; otherwise the user's standard `.codex` directory wins.
pub fn default_codex_home() -> PathBuf {
    codex_home_from(
        env::var_os("CODEX_HOME"),
        env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")),
    )
}

/// Resolves a home path from supplied environment values. Kept separate from
/// environment access so `CODEX_HOME` behavior remains deterministic in tests.
pub fn codex_home_from(
    codex_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> PathBuf {
    codex_home
        .map(PathBuf::from)
        .or_else(|| home.map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

/// Process metadata supplied by a local process observer. It contains only
/// correlation identifiers and never process arguments, prompt text, or code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexProcess {
    pub pid: u32,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
}

/// Correlates a process to an already discovered session by exact native ID,
/// then by exact canonical project root. Ambiguous project matches are omitted.
pub fn correlate_processes(
    sessions: &[NativeSession],
    processes: &[CodexProcess],
) -> Vec<(u32, StoveIdentity)> {
    processes
        .iter()
        .filter_map(|process| {
            let by_id = process.session_id.as_ref().and_then(|id| {
                sessions.iter().find(|session| {
                    session.harness == HarnessId::Codex && &session.native_session_id == id
                })
            });
            let mut matches = sessions.iter().filter(|session| {
                session.harness == HarnessId::Codex
                    && process.cwd.as_ref().is_some_and(|cwd| {
                        session
                            .project
                            .as_ref()
                            .is_some_and(|project| &project.canonical_root == cwd)
                    })
            });
            let session = by_id.or_else(|| {
                let session = matches.next()?;
                matches.next().is_none().then_some(session)
            })?;
            Some((
                process.pid,
                StoveIdentity::new(
                    session.host.clone(),
                    HarnessId::Codex,
                    session.native_session_id.clone(),
                ),
            ))
        })
        .collect()
}

#[async_trait]
impl HarnessAdapter for CodexAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::Codex
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
        self.sessions_for(source)
    }

    async fn watch(&self, sink: EventSink) -> Result<WatchHandle, AdapterError> {
        // Snapshot observation is intentionally read-only. The app's directory
        // watch drives subsequent bounded tails with the same parser.
        self.emit_snapshot(&sink).await?;
        Ok(WatchHandle::new())
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == HarnessId::Codex).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness == HarnessId::Codex {
            vec![ResumeAction::OpenSessionLocation(session.locator.clone())]
        } else {
            Vec::new()
        }
    }
}
