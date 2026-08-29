use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use cookbench_core::domain::{HostIdentity, ProjectIdentity};

use crate::{
    io::discover_jsonl_files, AdapterError, HostSource, NativeSession, SessionLocator,
    SessionLocatorKind,
};

use super::parser::parse_record;

const PROJECTS_DIRECTORY: &str = "projects";

/// Resolves Claude Code's native projects directory. `CLAUDE_CONFIG_DIR` is
/// deliberately read once by the caller so watching remains deterministic.
pub fn default_projects_root() -> Result<PathBuf, AdapterError> {
    let config_root = env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".claude")))
        .ok_or_else(|| {
            AdapterError::Message("could not determine Claude configuration directory".into())
        })?;
    Ok(config_root.join(PROJECTS_DIRECTORY))
}

pub fn discover_sessions(
    root: &Path,
    source: &HostSource,
) -> Result<Vec<NativeSession>, AdapterError> {
    if !matches!(source, HostSource::Local(_)) {
        return Ok(Vec::new());
    }
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let root = root
        .canonicalize()
        .map_err(|error| AdapterError::Message(error.to_string()))?;

    let mut sessions = Vec::new();
    for path in
        discover_jsonl_files(&root).map_err(|error| AdapterError::Message(error.to_string()))?
    {
        let Some(native_session_id) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        if native_session_id.is_empty() {
            continue;
        }
        let project = project_from_session_path(&root, &path, source.host());
        let locator = SessionLocator::new(SessionLocatorKind::LocalPath, path.to_string_lossy())?;
        sessions.push(NativeSession::new(
            source.host().clone(),
            cookbench_core::domain::HarnessId::ClaudeCode,
            native_session_id,
            project,
            title_from_session(&path),
            locator,
        )?);
    }
    sessions.sort_by(|left, right| left.native_session_id.cmp(&right.native_session_id));
    Ok(sessions)
}

/// Builds metadata for one already-selected transcript path. Runtime callers
/// use this after a metadata-only freshness filter so stale history bodies are
/// never opened merely to decide whether a session is a startup candidate.
pub fn discover_session(
    root: &Path,
    path: &Path,
    source: &HostSource,
) -> Result<Option<NativeSession>, AdapterError> {
    if !matches!(source, HostSource::Local(_)) || !path.starts_with(root) {
        return Ok(None);
    }
    let Some(native_session_id) = path.file_stem().and_then(|name| name.to_str()) else {
        return Ok(None);
    };
    if native_session_id.is_empty() {
        return Ok(None);
    }
    let locator = SessionLocator::new(SessionLocatorKind::LocalPath, path.to_string_lossy())?;
    NativeSession::new(
        source.host().clone(),
        cookbench_core::domain::HarnessId::ClaudeCode,
        native_session_id,
        project_from_session_path(root, path, source.host()),
        title_from_session(path),
        locator,
    )
    .map(Some)
}

fn title_from_session(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    for index in 0..32 {
        let Some(line) = read_bounded_line(&mut reader, 16 * 1024) else {
            break;
        };
        if let Some(parsed) =
            parse_record(&line, crate::io::TailLimits::default(), index as u64 + 1)
        {
            if parsed.title.is_some() {
                return parsed.title;
            }
        }
    }
    None
}

fn read_bounded_line(reader: &mut BufReader<File>, maximum: usize) -> Option<String> {
    let mut line = Vec::new();
    loop {
        let buffer = reader.fill_buf().ok()?;
        if buffer.is_empty() {
            return (!line.is_empty())
                .then(|| String::from_utf8(line).ok())
                .flatten();
        }
        let take = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|position| position + 1)
            .unwrap_or(buffer.len());
        if line.len().saturating_add(take) > maximum {
            // Do not load an unbounded native record merely to derive an
            // optional title. The session remains discoverable without one.
            return None;
        }
        line.extend_from_slice(&buffer[..take]);
        let reached_newline = buffer[take - 1] == b'\n';
        reader.consume(take);
        if reached_newline {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return String::from_utf8(line).ok();
        }
    }
}

pub fn encode_project_path(path: &Path) -> Option<String> {
    let path = path.to_str()?;
    if !path.starts_with('/') || path.contains('\0') {
        return None;
    }
    // Claude uses a dash-separated directory name. Percent-escape dashes first
    // so Cookbench can round-trip a path without accepting traversal segments.
    let encoded = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.replace('%', "%25").replace('-', "%2D"))
        .collect::<Vec<_>>()
        .join("-");
    (!encoded.is_empty()).then_some(format!("-{encoded}"))
}

pub fn decode_project_path(encoded: &str) -> Option<PathBuf> {
    let encoded = encoded.strip_prefix('-')?;
    if encoded.is_empty() {
        return None;
    }
    let mut path = PathBuf::from("/");
    for segment in encoded.split('-') {
        let segment = decode_segment(segment)?;
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('/')
            || segment.contains('\\')
        {
            return None;
        }
        path.push(segment);
    }
    Some(path)
}

fn project_from_session_path(
    root: &Path,
    path: &Path,
    host: &HostIdentity,
) -> Option<ProjectIdentity> {
    let project_directory = path
        .strip_prefix(root)
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?;
    decode_project_path(project_directory).and_then(|project| {
        project
            .to_str()
            .map(|project| ProjectIdentity::new(host.clone(), project))
    })
}

fn decode_segment(segment: &str) -> Option<String> {
    let mut decoded = String::with_capacity(segment.len());
    let bytes = segment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let value = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
            decoded.push(char::from(value));
            index += 3;
        } else {
            decoded.push(bytes[index].into());
            index += 1;
        }
    }
    Some(decoded)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}
