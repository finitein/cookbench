//! Bounded metadata-only inventory for sessions outside normal startup discovery.
//!
//! This is deliberately separate from ordinary discovery: it is used to give
//! the archive a recoverable reference to old native sessions, never to tail
//! or restore them into the active Bar.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use cookbench_core::{
    domain::{ProjectIdentity, StoveIdentity, StoveState},
    persistence::{RetainedStovePresentation, SessionRecord},
};

use super::{
    roots_for_kind, session_from_path, LocalObservationConfig, ParserKind, MAX_DISCOVERY_DEPTH,
    MAX_SCANNED_ENTRIES,
};

const EXPIRY_AGE: Duration = Duration::from_secs(2 * 24 * 60 * 60);
const MAX_ARCHIVE_RECORDS: usize = 4_096;

/// Discovers a bounded set of locally configured sessions whose native files
/// are strictly older than Cookbench's 48-hour active-discovery window.
///
/// Candidates are selected using filesystem metadata first. Only selected
/// regular JSONL files inside a canonical configured root reach an adapter's
/// existing bounded one-path metadata parser. The resulting records contain
/// only identity, native path, timestamp, project presentation, and the
/// honest unknown-state representation (`Disconnected`).
pub fn discover_expired_local_sessions(
    config: &LocalObservationConfig,
    now: SystemTime,
    limit: usize,
) -> Vec<SessionRecord> {
    let output_limit = limit.min(MAX_ARCHIVE_RECORDS);
    if output_limit == 0 {
        return Vec::new();
    }
    let cutoff = now
        .checked_sub(EXPIRY_AGE)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut scanned = 0;
    let mut candidates = Vec::new();

    for kind in [ParserKind::Codex, ParserKind::Claude, ParserKind::Pi] {
        for root in roots_for_kind(kind, config) {
            let Some(root) = canonical_directory(root) else {
                continue;
            };
            collect_expired_jsonl(kind, &root, &root, 0, &mut scanned, cutoff, &mut candidates);
            if scanned >= MAX_SCANNED_ENTRIES {
                break;
            }
        }
        if scanned >= MAX_SCANNED_ENTRIES {
            break;
        }
    }

    candidates.sort_by(|left, right| {
        modified_at(right)
            .cmp(&modified_at(left))
            .then_with(|| left.cmp(right))
    });

    candidates
        .into_iter()
        .filter_map(|(kind, path, modified)| {
            let session = session_from_path(kind, config, &path)?;
            if kind == ParserKind::Codex
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem == session.native_session_id)
            {
                // Codex falls back to the filename when its first complete
                // structural metadata record is unavailable. The inventory
                // must fail closed because that fallback cannot prove the
                // file is a user-owned root session rather than a subagent.
                return None;
            }
            if session.native_session_id == "skill-injections" {
                return None;
            }
            let identity = StoveIdentity::new(
                session.host.clone(),
                session.harness.clone(),
                session.native_session_id,
            );
            let project = session
                .project
                .unwrap_or_else(|| ProjectIdentity::new(session.host, "(unknown project)"));
            let presentation = presentation_for(&project);
            SessionRecord::new(
                identity,
                Some(path.to_string_lossy().into_owned()),
                epoch_ms(modified)?,
                presentation,
                StoveState::Disconnected,
            )
        })
        .take(output_limit)
        .collect()
}

fn canonical_directory(root: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(root).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    fs::canonicalize(root).ok()
}

fn collect_expired_jsonl(
    kind: ParserKind,
    root: &Path,
    directory: &Path,
    depth: usize,
    scanned: &mut usize,
    cutoff: SystemTime,
    output: &mut Vec<(ParserKind, PathBuf, SystemTime)>,
) {
    if depth > MAX_DISCOVERY_DEPTH || *scanned >= MAX_SCANNED_ENTRIES {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        *scanned = scanned.saturating_add(1);
        if *scanned > MAX_SCANNED_ENTRIES {
            return;
        }
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            // Every recursive directory must retain containment after
            // canonicalization, rather than trusting a lexical path prefix.
            if fs::canonicalize(&path)
                .ok()
                .is_some_and(|canonical| canonical.starts_with(root))
            {
                collect_expired_jsonl(kind, root, &path, depth + 1, scanned, cutoff, output);
            }
            continue;
        }
        if !metadata.is_file()
            || !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
        {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified >= cutoff {
            continue;
        }
        let Ok(canonical) = fs::canonicalize(&path) else {
            continue;
        };
        if !canonical.starts_with(root) {
            continue;
        }
        output.push((kind, canonical, modified));
    }
}

fn presentation_for(project: &ProjectIdentity) -> RetainedStovePresentation {
    let root = project.canonical_root.clone();
    let label = root
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("(unknown project)")
        .to_owned();
    RetainedStovePresentation::new(label, root)
}

fn modified_at(candidate: &(ParserKind, PathBuf, SystemTime)) -> SystemTime {
    candidate.2
}

fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}
