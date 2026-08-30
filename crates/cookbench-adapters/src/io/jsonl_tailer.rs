use std::{
    fs::{self, File, Metadata},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use super::TailLimits;

const FILE_ANCHOR_BYTES: u64 = 64;

/// A bounded outcome for one complete native JSONL line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TailRecord {
    /// A valid UTF-8 record. JSON interpretation belongs to the harness parser.
    Record(String),
    /// A malformed or resource-exhausting record isolated from later records.
    Rejected(TailRejection),
}

/// Reasons a complete JSONL line was intentionally not exposed to an adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TailRejection {
    LineTooLong { limit: usize },
    InvalidUtf8,
}

/// Errors for paths and file metadata. Malformed individual records are emitted
/// as [`TailRecord::Rejected`] instead so one session cannot poison another.
#[derive(Debug)]
pub enum TailError {
    InvalidLimits,
    RootNotDirectory(PathBuf),
    PathOutsideRoot(PathBuf),
    SymlinkNotAllowed(PathBuf),
    NotRegularFile(PathBuf),
    Io(io::Error),
}

impl std::fmt::Display for TailError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("JSONL tail limits must be positive"),
            Self::RootNotDirectory(path) => {
                write!(formatter, "root is not a directory: {}", path.display())
            }
            Self::PathOutsideRoot(path) => write!(
                formatter,
                "path is outside the configured root: {}",
                path.display()
            ),
            Self::SymlinkNotAllowed(path) => {
                write!(formatter, "symlinks are not allowed: {}", path.display())
            }
            Self::NotRegularFile(path) => {
                write!(formatter, "path is not a regular file: {}", path.display())
            }
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for TailError {}

impl From<io::Error> for TailError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    creation_time: u64,
    #[cfg(not(any(unix, windows)))]
    modified: Option<std::time::SystemTime>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileAnchor {
    length: u64,
    hash: u64,
}

impl FileIdentity {
    fn from_metadata(metadata: &Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            Self {
                // Stable Rust does not yet expose the Windows volume/file
                // index pair. NTFS creation time still detects normal atomic
                // replacement, while cursor-vs-length handles truncation.
                creation_time: metadata.creation_time(),
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            Self {
                modified: metadata.modified().ok(),
            }
        }
    }
}

/// Incrementally reads appended JSONL bytes from one regular file.
pub struct JsonlTailer {
    root: PathBuf,
    path: PathBuf,
    limits: TailLimits,
    identity: FileIdentity,
    cursor: u64,
    anchor: Option<FileAnchor>,
    partial: Vec<u8>,
    discarding_oversized_line: bool,
}

impl JsonlTailer {
    pub fn open(
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
        limits: TailLimits,
    ) -> Result<Self, TailError> {
        if !limits.validate() {
            return Err(TailError::InvalidLimits);
        }

        let root = canonical_root(root.as_ref())?;
        let path = checked_regular_file(&root, path.as_ref())?;
        let metadata = fs::metadata(&path)?;
        Ok(Self {
            root,
            path,
            limits,
            identity: FileIdentity::from_metadata(&metadata),
            cursor: 0,
            anchor: None,
            partial: Vec::new(),
            discarding_oversized_line: false,
        })
    }

    /// Reads at most `max_read_bytes_per_poll` appended bytes and emits only
    /// newline-terminated records. A partial final line remains bounded.
    pub fn poll(&mut self) -> Result<Vec<TailRecord>, TailError> {
        let checked = checked_regular_file(&self.root, &self.path)?;
        let metadata = fs::metadata(&checked)?;
        let identity = FileIdentity::from_metadata(&metadata);
        let anchor_matches =
            metadata.len() >= self.cursor && file_anchor(&checked, self.cursor)? == self.anchor;
        if identity != self.identity || metadata.len() < self.cursor || !anchor_matches {
            self.identity = identity;
            self.cursor = 0;
            self.anchor = None;
            self.partial.clear();
            self.discarding_oversized_line = false;
        }

        let available = metadata.len().saturating_sub(self.cursor);
        let to_read = available.min(self.limits.max_read_bytes_per_poll as u64) as usize;
        if to_read == 0 {
            return Ok(Vec::new());
        }

        let mut file = File::open(&checked)?;
        file.seek(SeekFrom::Start(self.cursor))?;
        let mut bytes = vec![0; to_read];
        file.read_exact(&mut bytes)?;
        self.cursor += to_read as u64;
        self.anchor = file_anchor_from(&mut file, self.cursor)?;

        let mut records = Vec::new();
        for byte in bytes {
            if self.discarding_oversized_line {
                if byte == b'\n' {
                    self.discarding_oversized_line = false;
                }
                continue;
            }

            if byte == b'\n' {
                let line = std::mem::take(&mut self.partial);
                records.push(decode_line(line, self.limits.max_record_bytes));
                continue;
            }

            if self.partial.len() == self.limits.max_partial_bytes
                || self.partial.len() == self.limits.max_record_bytes
            {
                self.partial.clear();
                self.discarding_oversized_line = true;
                records.push(TailRecord::Rejected(TailRejection::LineTooLong {
                    limit: self.limits.max_record_bytes,
                }));
                continue;
            }
            self.partial.push(byte);
        }
        Ok(records)
    }

    /// Moves the incremental cursor to the current EOF without reading native
    /// records. Startup uses this after metadata discovery so historical turns
    /// cannot be mistaken for a newly observed completion.
    pub fn skip_existing(&mut self) -> Result<(), TailError> {
        let checked = checked_regular_file(&self.root, &self.path)?;
        let metadata = fs::metadata(&checked)?;
        self.identity = FileIdentity::from_metadata(&metadata);
        self.cursor = metadata.len();
        self.anchor = file_anchor(&checked, self.cursor)?;
        self.partial.clear();
        self.discarding_oversized_line = false;
        Ok(())
    }

    /// Starts at a bounded suffix of the native file. If the suffix begins in
    /// the middle of a JSONL record, bytes are discarded through its newline.
    /// This reconstructs recent authoritative state without loading an entire
    /// long-running transcript into memory.
    pub fn seek_recent_window(&mut self, maximum_bytes: u64) -> Result<(), TailError> {
        let checked = checked_regular_file(&self.root, &self.path)?;
        let metadata = fs::metadata(&checked)?;
        self.identity = FileIdentity::from_metadata(&metadata);
        self.cursor = metadata.len().saturating_sub(maximum_bytes);
        self.anchor = file_anchor(&checked, self.cursor)?;
        self.partial.clear();
        self.discarding_oversized_line = self.cursor > 0;
        Ok(())
    }

    pub fn buffered_bytes(&self) -> usize {
        self.partial.len()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Stable byte-position hint used to seed normalized event ordering after
    /// restart. It reveals no native record content.
    pub fn cursor(&self) -> u64 {
        self.cursor
    }
}

fn file_anchor(path: &Path, cursor: u64) -> Result<Option<FileAnchor>, TailError> {
    let mut file = File::open(path)?;
    file_anchor_from(&mut file, cursor).map_err(TailError::from)
}

fn file_anchor_from(file: &mut File, cursor: u64) -> io::Result<Option<FileAnchor>> {
    let length = cursor.min(FILE_ANCHOR_BYTES);
    if length == 0 {
        return Ok(None);
    }

    file.seek(SeekFrom::Start(cursor - length))?;
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes)?;

    // Keep only a bounded continuity hash in memory. It lets stable Rust
    // detect path replacement on Windows without persisting or exposing the
    // sampled native session bytes.
    let hash = bytes.into_iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    Ok(Some(FileAnchor { length, hash }))
}

fn decode_line(mut line: Vec<u8>, max_record_bytes: usize) -> TailRecord {
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    if line.len() > max_record_bytes {
        return TailRecord::Rejected(TailRejection::LineTooLong {
            limit: max_record_bytes,
        });
    }
    match String::from_utf8(line) {
        Ok(record) => TailRecord::Record(record),
        Err(_) => TailRecord::Rejected(TailRejection::InvalidUtf8),
    }
}

pub(super) fn canonical_root(root: &Path) -> Result<PathBuf, TailError> {
    let root = fs::canonicalize(root)?;
    if !fs::metadata(&root)?.is_dir() {
        return Err(TailError::RootNotDirectory(root));
    }
    Ok(root)
}

pub(super) fn checked_regular_file(root: &Path, path: &Path) -> Result<PathBuf, TailError> {
    let original = path.to_path_buf();
    let link_metadata = fs::symlink_metadata(&original)?;
    if link_metadata.file_type().is_symlink() {
        return Err(TailError::SymlinkNotAllowed(original));
    }
    if !link_metadata.is_file() {
        return Err(TailError::NotRegularFile(original));
    }
    let canonical = fs::canonicalize(path)?;
    if !canonical.starts_with(root) {
        return Err(TailError::PathOutsideRoot(canonical));
    }
    Ok(canonical)
}
