use std::{
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use cookbench_core::domain::{HarnessId, ProjectIdentity};
use serde::Deserialize;

use crate::{
    AdapterCapabilities, AdapterError, EventSink, HarnessAdapter, HostSource, NativeSession,
    ResumeAction, SessionLocator, SessionLocatorKind, WatchHandle,
};

const MAX_SCANNED_ENTRIES: usize = 4_096;
const MAX_DISCOVERY_DEPTH: usize = 8;
const MAX_TITLE_BYTES: usize = 160;

#[derive(Clone, Debug)]
pub struct GrokAdapter {
    sessions_root: PathBuf,
}

impl GrokAdapter {
    pub fn new(sessions_root: PathBuf) -> Self {
        Self { sessions_root }
    }

    pub fn from_environment() -> Result<Self, AdapterError> {
        Ok(Self::new(default_sessions_root()?))
    }

    pub fn sessions_root(&self) -> &Path {
        &self.sessions_root
    }
}

pub fn default_sessions_root() -> Result<PathBuf, AdapterError> {
    if let Ok(home) = std::env::var("GROK_HOME") {
        return Ok(PathBuf::from(home).join("sessions"));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| AdapterError::Message("HOME is not set".into()))?;
    Ok(PathBuf::from(home).join(".grok").join("sessions"))
}

pub fn discover_sessions(
    sessions_root: &Path,
    source: &HostSource,
) -> Result<Vec<NativeSession>, AdapterError> {
    if !sessions_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut discovered = Vec::new();
    let mut scanned = 0usize;
    collect_summaries(sessions_root, 0, &mut scanned, source, &mut discovered)?;
    Ok(discovered)
}

fn collect_summaries(
    directory: &Path,
    depth: usize,
    scanned: &mut usize,
    source: &HostSource,
    discovered: &mut Vec<NativeSession>,
) -> Result<(), AdapterError> {
    if depth > MAX_DISCOVERY_DEPTH || *scanned >= MAX_SCANNED_ENTRIES {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|error| AdapterError::Message(error.to_string()))?;
    for entry in entries {
        if *scanned >= MAX_SCANNED_ENTRIES {
            break;
        }
        let entry = entry.map_err(|error| AdapterError::Message(error.to_string()))?;
        *scanned += 1;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| AdapterError::Message(error.to_string()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let summary = path.join("summary.json");
            if summary.is_file() {
                if let Ok(session) = session_from_summary(&summary, source) {
                    discovered.push(session);
                }
                continue;
            }
            collect_summaries(&path, depth + 1, scanned, source, discovered)?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct SummaryFile {
    info: Option<SummaryInfo>,
    generated_title: Option<String>,
    #[serde(default)]
    title_is_manual: bool,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SummaryInfo {
    session_id: Option<String>,
    cwd: Option<String>,
}

fn session_from_summary(path: &Path, source: &HostSource) -> Result<NativeSession, AdapterError> {
    let bytes = fs::read(path).map_err(|error| AdapterError::Message(error.to_string()))?;
    if bytes.len() > 64 * 1024 {
        return Err(AdapterError::invalid_session_metadata(
            "summary.json exceeds the bounded discovery limit",
        ));
    }
    let summary: SummaryFile = serde_json::from_slice(&bytes)
        .map_err(|_| AdapterError::invalid_session_metadata("summary.json is not valid JSON"))?;
    let info = summary.info.ok_or_else(|| {
        AdapterError::invalid_session_metadata("summary.json is missing info")
    })?;
    let native_session_id = info
        .session_id
        .filter(|value| !value.is_empty() && value.len() <= NativeSession::MAX_ID_BYTES)
        .ok_or_else(|| AdapterError::invalid_session_metadata("missing session id"))?;
    let cwd = info
        .cwd
        .filter(|value| Path::new(value).is_absolute() && value.len() <= 1024);
    let title = if summary.title_is_manual {
        summary.title.or(summary.generated_title)
    } else {
        summary.generated_title.or(summary.title)
    }
    .and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty() && trimmed.len() <= MAX_TITLE_BYTES).then(|| trimmed.to_owned())
    });

    let session_dir = path
        .parent()
        .ok_or_else(|| AdapterError::invalid_session_metadata("summary path has no parent"))?;
    let updates = session_dir.join("updates.jsonl");
    let locator_path = if updates.is_file() {
        updates
    } else {
        path.to_owned()
    };
    let host = source.host().clone();
    let project = cwd.map(|root| ProjectIdentity::new(host.clone(), root));
    let locator_kind = match source {
        HostSource::Local(_) => SessionLocatorKind::LocalPath,
        HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
    };
    NativeSession::new(
        host,
        HarnessId::Other("grok_cli".into()),
        native_session_id,
        project,
        title,
        SessionLocator::new(locator_kind, locator_path.to_string_lossy())?,
    )
}

#[async_trait]
impl HarnessAdapter for GrokAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::Other("grok_cli".into())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            discovery: true,
            watch_events: false,
            structured_progress: false,
            locator: true,
            resume: true,
        }
    }

    async fn discover(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError> {
        discover_sessions(&self.sessions_root, source)
    }

    async fn watch(&self, _sink: EventSink) -> Result<WatchHandle, AdapterError> {
        Err(AdapterError::UnsupportedCapability(
            "Grok Build watch events",
        ))
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == self.id()).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness != self.id() {
            return Vec::new();
        }
        vec![ResumeAction::SuggestedCommand {
            program: "grok".to_owned(),
            args: vec!["--resume".to_owned(), session.native_session_id.clone()],
        }]
    }
}

#[cfg(test)]
mod tests {
    use cookbench_core::domain::HostIdentity;
    use super::*;

    #[test]
    fn discovers_synthetic_fixture_session() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/grok_build/sessions");
        let source = HostSource::local(HostIdentity::local("fixture-host"));
        let sessions = discover_sessions(&root, &source).expect("discover");
        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!(
            session.native_session_id,
            "01999999-aaaa-7bbb-8ccc-ddddeeeeffff"
        );
        assert_eq!(session.title.as_deref(), Some("Synthetic fixture task"));
        assert_eq!(
            session.project.as_ref().map(|p| p.canonical_root.as_str()),
            Some("/synthetic/project")
        );
        assert!(session.locator.value.ends_with("updates.jsonl"));
    }
}
