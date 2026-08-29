//! Versioned, presentation-only payloads for the optional GNOME Shell extension.
//!
//! The desktop app remains authoritative. This bridge deliberately projects a
//! stove snapshot onto a tiny allowlist and never reads native session files,
//! credentials, notification settings, or harness configuration.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::app_state::{StoveSnapshot, StoveStateWire};

pub const PROTOCOL_VERSION: u32 = 1;
pub const PRESENTATION_FILE_NAME: &str = "gnome-presentation-v1.json";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GnomePresentationSnapshot {
    pub version: u32,
    pub revision: u64,
    pub stoves: Vec<GnomeStoveSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GnomeStoveSummary {
    pub harness: String,
    pub project: String,
    pub state: &'static str,
    pub progress: Option<GnomeProgress>,
    pub retained_completion: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GnomeProgress {
    pub completed: u32,
    pub total: u32,
}

impl From<&StoveSnapshot> for GnomePresentationSnapshot {
    fn from(snapshot: &StoveSnapshot) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            revision: snapshot.revision,
            stoves: snapshot
                .stoves
                .iter()
                .map(|stove| GnomeStoveSummary {
                    harness: bounded(&stove.harness.label),
                    project: bounded(&stove.project_label),
                    state: state_name(stove.state),
                    progress: (stove.state == StoveStateWire::Cooking)
                        .then(|| {
                            stove.progress.as_ref().map(|progress| GnomeProgress {
                                completed: progress.completed,
                                total: progress.total,
                            })
                        })
                        .flatten(),
                    retained_completion: stove.retained_completion,
                })
                .collect(),
        }
    }
}

/// The extension file is a cache, not a source of truth. A missing runtime
/// directory or failed write is safe: the optional extension simply renders
/// nothing while the main Cookbench application continues normally.
pub fn write_presentation_file(
    runtime_dir: &Path,
    snapshot: &GnomePresentationSnapshot,
) -> io::Result<PathBuf> {
    let directory = runtime_dir.join("cookbench");
    fs::create_dir_all(&directory)?;
    let target = directory.join(PRESENTATION_FILE_NAME);
    let temporary = directory.join(format!(".{PRESENTATION_FILE_NAME}.tmp"));
    fs::write(&temporary, encode_json(snapshot))?;
    fs::rename(&temporary, &target)?;
    Ok(target)
}

/// Removes the optional cache when Cookbench shuts down. Failure to remove a
/// file never changes authoritative stove state, but callers can log the I/O
/// error so a stale panel summary is not mistaken for a live application.
pub fn remove_presentation_file(runtime_dir: &Path) -> io::Result<()> {
    let target = runtime_dir.join("cookbench").join(PRESENTATION_FILE_NAME);
    match fs::remove_file(target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn bounded(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(128)
        .collect()
}

const fn state_name(state: StoveStateWire) -> &'static str {
    match state {
        StoveStateWire::Starting => "starting",
        StoveStateWire::Planning => "planning",
        StoveStateWire::Cooking => "cooking",
        StoveStateWire::NeedsHuman => "needsHuman",
        StoveStateWire::Cooked => "cooked",
        StoveStateWire::Failed => "failed",
        StoveStateWire::Disconnected => "disconnected",
    }
}

// `serde_json` is intentionally not a desktop runtime dependency. The bridge
// serializes only its fixed schema and escapes every user-derived label.
fn encode_json(snapshot: &GnomePresentationSnapshot) -> String {
    let stoves = snapshot
        .stoves
        .iter()
        .map(|stove| {
            let progress = stove.progress.as_ref().map_or_else(
                || "null".to_owned(),
                |progress| {
                    format!(
                        "{{\"completed\":{},\"total\":{}}}",
                        progress.completed, progress.total
                    )
                },
            );
            format!(
                "{{\"harness\":\"{}\",\"project\":\"{}\",\"state\":\"{}\",\"progress\":{},\"retainedCompletion\":{}}}",
                escape_json(&stove.harness),
                escape_json(&stove.project),
                stove.state,
                progress,
                stove.retained_completion,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"version\":{},\"revision\":{},\"stoves\":[{}]}}",
        snapshot.version, snapshot.revision, stoves
    )
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            character if character.is_control() => "".chars().collect::<Vec<_>>(),
            character => std::iter::once(character).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_only_user_derived_fields() {
        assert_eq!(escape_json("\""), "\\\"");
        assert_eq!(escape_json("\\"), "\\\\");
    }
}
