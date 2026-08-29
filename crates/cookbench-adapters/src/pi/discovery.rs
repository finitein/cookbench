use std::{
    env, fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use cookbench_core::domain::{HarnessId, ProjectIdentity};

use crate::{
    AdapterCapabilities, AdapterError, EventSink, HarnessAdapter, HostSource, NativeSession,
    ResumeAction, SessionLocator, SessionLocatorKind, WatchHandle,
};

use super::parser::parse_session_file;

/// Read-only adapter for Pi session trees. Roots are configurable for portable
/// installs and tests; an unset adapter uses Pi's conventional local root.
#[derive(Clone, Debug)]
pub struct PiAdapter {
    roots: Vec<PathBuf>,
}

impl PiAdapter {
    pub fn new() -> Self {
        Self {
            roots: vec![default_session_root()],
        }
    }

    pub fn with_roots(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
        }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    fn discover_root(
        &self,
        source: &HostSource,
        root: &Path,
    ) -> Result<Vec<NativeSession>, AdapterError> {
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        collect_jsonl_files(root, &mut files)?;
        files.sort();

        files
            .into_iter()
            .map(|path| self.session_from_file(source, path))
            .collect()
    }

    /// Reads bounded structural metadata for one already-selected native path.
    pub fn session_from_file(
        &self,
        source: &HostSource,
        path: PathBuf,
    ) -> Result<NativeSession, AdapterError> {
        let parsed = parse_session_file(&path)?;
        let host = source.host().clone();
        let locator_kind = match source {
            HostSource::Local(_) => SessionLocatorKind::LocalPath,
            HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
        };
        let project = parsed
            .project
            .map(|root| ProjectIdentity::new(host.clone(), root));
        NativeSession::new(
            host,
            HarnessId::Pi,
            parsed.native_session_id,
            project,
            parsed.title,
            SessionLocator::new(locator_kind, path.to_string_lossy())?,
        )
    }

    /// Builds a content-free identity for a metadata-filtered runtime path.
    /// Lifecycle replay may then inspect only the shared bounded suffix.
    pub fn session_metadata_from_path(
        &self,
        source: &HostSource,
        path: PathBuf,
    ) -> Result<NativeSession, AdapterError> {
        let native_session_id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .ok_or_else(|| AdapterError::Message("Pi session path has no safe identity".into()))?;
        let locator_kind = match source {
            HostSource::Local(_) => SessionLocatorKind::LocalPath,
            HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
        };
        NativeSession::new(
            source.host().clone(),
            HarnessId::Pi,
            native_session_id,
            None,
            None,
            SessionLocator::new(locator_kind, path.to_string_lossy())?,
        )
    }
}

impl Default for PiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HarnessAdapter for PiAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::Pi
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            discovery: true,
            watch_events: false,
            structured_progress: true,
            locator: true,
            resume: true,
        }
    }

    async fn discover(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError> {
        let mut sessions = Vec::new();
        for root in &self.roots {
            sessions.extend(self.discover_root(source, root)?);
        }
        sessions.sort_by(|left, right| left.native_session_id.cmp(&right.native_session_id));
        sessions.dedup_by(|left, right| left.native_session_id == right.native_session_id);
        Ok(sessions)
    }

    async fn watch(&self, _sink: EventSink) -> Result<WatchHandle, AdapterError> {
        // File watching is supplied by the shared adapter watcher. Returning a
        // handle here keeps this parser read-only and avoids starting Pi.
        Ok(WatchHandle::new())
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == HarnessId::Pi).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness != HarnessId::Pi {
            return Vec::new();
        }
        vec![
            ResumeAction::OpenSessionLocation(session.locator.clone()),
            ResumeAction::SuggestedCommand {
                program: "pi".to_owned(),
                args: vec!["--resume".to_owned(), session.native_session_id.clone()],
            },
        ]
    }
}

fn default_session_root() -> PathBuf {
    env::var_os("PI_SESSION_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".pi/agent/sessions")))
        .or_else(|| {
            env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".pi/agent/sessions"))
        })
        .unwrap_or_else(|| PathBuf::from(".pi/agent/sessions"))
}

fn collect_jsonl_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), AdapterError> {
    let entries = fs::read_dir(root).map_err(|error| {
        AdapterError::Message(format!(
            "cannot read Pi session root {}: {error}",
            root.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            AdapterError::Message(format!("cannot read Pi session entry: {error}"))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
        {
            files.push(path);
        }
    }
    Ok(())
}
