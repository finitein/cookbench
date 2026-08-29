use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::jsonl_tailer::canonical_root;

/// Errors from metadata-only discovery or the bounded filesystem event stream.
#[derive(Debug)]
pub enum DirectoryWatchError {
    Io(std::io::Error),
    Notify(notify::Error),
    RootNotDirectory(PathBuf),
}

impl std::fmt::Display for DirectoryWatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Notify(error) => error.fmt(formatter),
            Self::RootNotDirectory(path) => {
                write!(formatter, "root is not a directory: {}", path.display())
            }
        }
    }
}

impl std::error::Error for DirectoryWatchError {}

impl From<std::io::Error> for DirectoryWatchError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<notify::Error> for DirectoryWatchError {
    fn from(error: notify::Error) -> Self {
        Self::Notify(error)
    }
}

/// A read-only directory watcher backed by native filesystem events.
///
/// The receiver is bounded; when an event burst fills it, extra paths are
/// discarded rather than growing memory or triggering directory polling.
pub struct DirectoryWatch {
    _watcher: RecommendedWatcher,
    receiver: Receiver<PathBuf>,
}

impl DirectoryWatch {
    pub fn open(root: impl AsRef<Path>, capacity: usize) -> Result<Self, DirectoryWatchError> {
        let root = canonical_root(root.as_ref()).map_err(|error| match error {
            super::TailError::RootNotDirectory(path) => DirectoryWatchError::RootNotDirectory(path),
            super::TailError::Io(error) => DirectoryWatchError::Io(error),
            _ => DirectoryWatchError::RootNotDirectory(root.as_ref().to_path_buf()),
        })?;
        let capacity = capacity.max(1);
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let root_for_callback = root.clone();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let Ok(event) = result else { return };
            if !is_relevant(&event.kind) {
                return;
            }
            for path in event.paths {
                if path
                    .extension()
                    .is_some_and(|extension| extension == "jsonl")
                    && path_is_inside(&root_for_callback, &path)
                {
                    let _ = sender.try_send(path);
                }
            }
        })?;
        watcher.watch(&root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            receiver,
        })
    }

    pub fn try_recv(&self) -> Result<Option<PathBuf>, DirectoryWatchError> {
        match self.receiver.try_recv() {
            Ok(path) => Ok(Some(path)),
            Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => Ok(None),
        }
    }
}

/// Discovers native JSONL paths recursively using entries and metadata only.
/// Symlinked files and directories are never followed.
pub fn discover_jsonl_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, DirectoryWatchError> {
    let root = canonical_root(root.as_ref()).map_err(|error| match error {
        super::TailError::RootNotDirectory(path) => DirectoryWatchError::RootNotDirectory(path),
        super::TailError::Io(error) => DirectoryWatchError::Io(error),
        _ => DirectoryWatchError::RootNotDirectory(root.as_ref().to_path_buf()),
    })?;
    let mut discovered = Vec::new();
    discover_into(&root, &mut discovered)?;
    discovered.sort();
    Ok(discovered)
}

fn discover_into(
    directory: &Path,
    discovered: &mut Vec<PathBuf>,
) -> Result<(), DirectoryWatchError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            discover_into(&path, discovered)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
        {
            discovered.push(path);
        }
    }
    Ok(())
}

fn path_is_inside(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
        && fs::symlink_metadata(path).is_ok_and(|metadata| !metadata.file_type().is_symlink())
}

fn is_relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}
