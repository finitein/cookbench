use std::{
    fs,
    io::{BufRead, BufReader},
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
const MAX_DISCOVERY_DEPTH: usize = 4;
const MAX_TITLE_BYTES: usize = 160;
const MAX_METADATA_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct GooseAdapter {
    sessions_root: PathBuf,
}

impl GooseAdapter {
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
    if let Ok(home) = std::env::var("GOOSE_HOME") {
        return Ok(PathBuf::from(home).join("sessions"));
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("goose").join("sessions"));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| AdapterError::Message("HOME is not set".into()))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("goose")
        .join("sessions"))
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
    collect_sessions(sessions_root, 0, &mut scanned, source, &mut discovered)?;
    Ok(discovered)
}

fn collect_sessions(
    directory: &Path,
    depth: usize,
    scanned: &mut usize,
    source: &HostSource,
    discovered: &mut Vec<NativeSession>,
) -> Result<(), AdapterError> {
    if depth > MAX_DISCOVERY_DEPTH || *scanned >= MAX_SCANNED_ENTRIES {
        return Ok(());
    }
    let entries =
        fs::read_dir(directory).map_err(|error| AdapterError::Message(error.to_string()))?;
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
            collect_sessions(&path, depth + 1, scanned, source, discovered)?;
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "sessions.db" || !name.ends_with(".jsonl"))
        {
            continue;
        }
        if let Ok(Some(session)) = session_from_path(&path, source) {
            discovered.push(session);
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct MetadataLine {
    id: Option<String>,
    description: Option<String>,
    working_dir: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

pub fn session_from_path(
    path: &Path,
    source: &HostSource,
) -> Result<Option<NativeSession>, AdapterError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if file_name == "sessions.db" || !file_name.ends_with(".jsonl") {
        return Ok(None);
    }
    let file = fs::File::open(path).map_err(|error| AdapterError::Message(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut first = String::new();
    reader
        .read_line(&mut first)
        .map_err(|error| AdapterError::Message(error.to_string()))?;
    let first = first.trim();
    if first.is_empty() || first.len() > MAX_METADATA_BYTES {
        return Ok(None);
    }
    let Ok(metadata) = serde_json::from_str::<MetadataLine>(first) else {
        return Ok(None);
    };
    let native_session_id = metadata
        .id
        .filter(|value| !value.is_empty() && value.len() <= NativeSession::MAX_ID_BYTES)
        .or_else(|| {
            file_name
                .strip_suffix(".jsonl")
                .map(str::to_owned)
                .filter(|value| !value.is_empty() && value.len() <= NativeSession::MAX_ID_BYTES)
        });
    let Some(native_session_id) = native_session_id else {
        return Ok(None);
    };
    let title = metadata
        .description
        .or(metadata.name)
        .and_then(|value| sanitize_title(&value));
    let host = source.host().clone();
    let project = metadata
        .working_dir
        .filter(|value| Path::new(value).is_absolute() && value.len() <= 4 * 1024)
        .map(|value| ProjectIdentity::new(host.clone(), value));
    let locator_kind = match source {
        HostSource::Local(_) => SessionLocatorKind::LocalPath,
        HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
    };
    Ok(Some(NativeSession::new(
        host,
        HarnessId::Other("goose".into()),
        native_session_id,
        project,
        title,
        SessionLocator::new(locator_kind, path.to_string_lossy())?,
    )?))
}

fn sanitize_title(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_TITLE_BYTES
        || trimmed.chars().any(char::is_control)
    {
        return None;
    }
    Some(trimmed.to_owned())
}

#[async_trait]
impl HarnessAdapter for GooseAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::Other("goose".into())
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
        Err(AdapterError::UnsupportedCapability("Goose watch events"))
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == self.id()).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness != self.id() {
            return Vec::new();
        }
        vec![ResumeAction::SuggestedCommand {
            program: "goose".to_owned(),
            args: vec![
                "session".to_owned(),
                "--resume".to_owned(),
                "--session-id".to_owned(),
                session.native_session_id.clone(),
            ],
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cookbench_core::domain::HostIdentity;

    #[test]
    fn discovers_synthetic_fixture_session() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/goose/sessions");
        let source = HostSource::local(HostIdentity::local("fixture-host"));
        let sessions = discover_sessions(&root, &source).expect("discover");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].native_session_id, "20260906_1");
        assert_eq!(
            sessions[0].title.as_deref(),
            Some("Synthetic Goose fixture")
        );
        assert_eq!(
            sessions[0]
                .project
                .as_ref()
                .map(|p| p.canonical_root.as_str()),
            Some("/synthetic/goose-project")
        );
    }

    #[test]
    fn ignores_sqlite_database_files() {
        let path = PathBuf::from("/tmp/sessions.db");
        let source = HostSource::local(HostIdentity::local("fixture-host"));
        assert_eq!(session_from_path(&path, &source).expect("parse"), None);
    }
}
