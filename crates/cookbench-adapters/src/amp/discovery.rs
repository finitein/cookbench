use std::{
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use cookbench_core::domain::{HarnessId, ProjectIdentity};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    AdapterCapabilities, AdapterError, EventSink, HarnessAdapter, HostSource, NativeSession,
    ResumeAction, SessionLocator, SessionLocatorKind, WatchHandle,
};

const MAX_SCANNED_ENTRIES: usize = 4_096;
const MAX_DISCOVERY_DEPTH: usize = 4;
const MAX_TITLE_BYTES: usize = 160;
const MAX_METADATA_BYTES: usize = 256 * 1024;
const MAX_ID_BYTES: usize = NativeSession::MAX_ID_BYTES;

#[derive(Clone, Debug)]
pub struct AmpAdapter {
    threads_root: PathBuf,
}

impl AmpAdapter {
    pub fn new(threads_root: PathBuf) -> Self {
        Self { threads_root }
    }

    pub fn from_environment() -> Result<Self, AdapterError> {
        Ok(Self::new(default_threads_root()?))
    }

    pub fn threads_root(&self) -> &Path {
        &self.threads_root
    }
}

pub fn default_threads_root() -> Result<PathBuf, AdapterError> {
    if let Ok(override_dir) = std::env::var("AMP_THREADS_DIR") {
        return Ok(PathBuf::from(override_dir));
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("amp").join("threads"));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| AdapterError::Message("HOME is not set".into()))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("amp")
        .join("threads"))
}

pub fn discover_sessions(
    threads_root: &Path,
    source: &HostSource,
) -> Result<Vec<NativeSession>, AdapterError> {
    if !threads_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut discovered = Vec::new();
    let mut scanned = 0usize;
    collect_sessions(threads_root, 0, &mut scanned, source, &mut discovered)?;
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
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
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
struct ThreadHeader {
    /// Amp revision counter; presence is the format sniff.
    v: Option<Value>,
    id: Option<String>,
    title: Option<String>,
    #[serde(default)]
    env: Option<Value>,
}

pub fn session_from_path(
    path: &Path,
    source: &HostSource,
) -> Result<Option<NativeSession>, AdapterError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !file_name.ends_with(".json") {
        return Ok(None);
    }
    let metadata = fs::metadata(path).map_err(|error| AdapterError::Message(error.to_string()))?;
    if !metadata.is_file() || metadata.len() > MAX_METADATA_BYTES as u64 {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(path).map_err(|error| AdapterError::Message(error.to_string()))?;
    if contents.len() > MAX_METADATA_BYTES {
        return Ok(None);
    }
    let Ok(header) = serde_json::from_str::<ThreadHeader>(&contents) else {
        return Ok(None);
    };
    // Require Amp's `v` revision key so stray JSON nearby is ignored.
    if header.v.is_none() {
        return Ok(None);
    }
    let native_session_id = header
        .id
        .filter(|value| !value.is_empty() && value.len() <= MAX_ID_BYTES)
        .or_else(|| {
            file_name
                .strip_suffix(".json")
                .map(str::to_owned)
                .filter(|value| !value.is_empty() && value.len() <= MAX_ID_BYTES)
        });
    let Some(native_session_id) = native_session_id else {
        return Ok(None);
    };
    // Amp thread ids are T-<uuid>-shaped; refuse unrelated JSON even with `v`.
    if !native_session_id.starts_with("T-") {
        return Ok(None);
    }
    let title = header.title.and_then(|value| sanitize_title(&value));
    let host = source.host().clone();
    let project = project_from_env(header.env.as_ref(), &host);
    let locator_kind = match source {
        HostSource::Local(_) => SessionLocatorKind::LocalPath,
        HostSource::Ssh(_) => SessionLocatorKind::RemotePath,
    };
    Ok(Some(NativeSession::new(
        host,
        HarnessId::Other("amp".into()),
        native_session_id,
        project,
        title,
        SessionLocator::new(locator_kind, path.to_string_lossy())?,
    )?))
}

fn project_from_env(
    env: Option<&Value>,
    host: &cookbench_core::domain::HostIdentity,
) -> Option<ProjectIdentity> {
    let trees = env?
        .get("initial")
        .or(env)
        .and_then(|value| value.get("trees"))
        .and_then(Value::as_array)?;
    for tree in trees {
        let uri = tree.get("uri").and_then(Value::as_str)?;
        let path = uri.strip_prefix("file://").unwrap_or(uri).trim().to_owned();
        if crate::adapter::is_absolute_session_path(&path)
            && path.len() <= 4 * 1024
            && !path.chars().any(char::is_control)
        {
            return Some(ProjectIdentity::new(host.clone(), path));
        }
    }
    None
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
impl HarnessAdapter for AmpAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::Other("amp".into())
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
        discover_sessions(&self.threads_root, source)
    }

    async fn watch(&self, _sink: EventSink) -> Result<WatchHandle, AdapterError> {
        Err(AdapterError::UnsupportedCapability("Amp watch events"))
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == self.id()).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness != self.id() {
            return Vec::new();
        }
        // Local-first Amp accepted thread continue; modern server Amp may ignore
        // on-disk threads. Keep this as a user-mediated suggestion only.
        vec![ResumeAction::SuggestedCommand {
            program: "amp".to_owned(),
            args: vec![
                "threads".to_owned(),
                "continue".to_owned(),
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
    fn discovers_synthetic_fixture_thread() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/amp/threads");
        let source = HostSource::local(HostIdentity::local("fixture-host"));
        let sessions = discover_sessions(&root, &source).expect("discover");
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].native_session_id,
            "T-0199aaaa-bbbb-7ccc-8ddd-eeeeffff0001"
        );
        assert_eq!(sessions[0].title.as_deref(), Some("Synthetic Amp fixture"));
        assert_eq!(
            sessions[0]
                .project
                .as_ref()
                .map(|p| p.canonical_root.as_str()),
            Some("/synthetic/amp-project")
        );
    }

    #[test]
    fn ignores_json_without_amp_revision_key() {
        let source = HostSource::local(HostIdentity::local("fixture-host"));
        let dir = std::env::temp_dir().join(format!("cookbench-amp-sniff-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("T-stray.json");
        fs::write(&path, r#"{"id":"T-stray","title":"no v key"}"#).unwrap();
        assert_eq!(session_from_path(&path, &source).expect("parse"), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
