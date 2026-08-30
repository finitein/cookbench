use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Serialize};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub trait Versioned {
    const CURRENT_VERSION: u32;

    fn version(&self) -> u32;
}

#[derive(Debug)]
pub enum PersistenceError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion { found: u32, supported: u32 },
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "persistence I/O error: {error}"),
            Self::Json(error) => write!(formatter, "invalid persistence JSON: {error}"),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "unsupported persistence version {found}; this Cookbench build supports up to {supported}"
            ),
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnsupportedVersion { .. } => None,
        }
    }
}

impl From<io::Error> for PersistenceError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for PersistenceError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

/// A versioned JSON file written without ever modifying its live destination.
pub struct AtomicJsonFile<T> {
    path: PathBuf,
    marker: std::marker::PhantomData<T>,
}

impl<T> AtomicJsonFile<T> {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            marker: std::marker::PhantomData,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl<T> AtomicJsonFile<T>
where
    T: Default + DeserializeOwned + Serialize + Versioned,
{
    pub fn load(&self) -> Result<T, PersistenceError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(T::default()),
            Err(error) => return Err(error.into()),
        };
        let value: T = serde_json::from_slice(&bytes)?;
        ensure_supported_version(&value)?;
        Ok(value)
    }

    pub fn save(&self, value: &T) -> Result<(), PersistenceError> {
        ensure_supported_version(value)?;
        let bytes = serde_json::to_vec(value)?;
        let directory = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(directory)?;

        let (temporary_path, mut temporary_file) = create_temporary_file(directory, &self.path)?;
        let result = (|| -> Result<(), PersistenceError> {
            temporary_file.write_all(&bytes)?;
            temporary_file.flush()?;
            temporary_file.sync_all()?;
            drop(temporary_file);
            replace_file(&temporary_path, &self.path)?;
            sync_directory(directory);
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }
}

fn ensure_supported_version<T: Versioned>(value: &T) -> Result<(), PersistenceError> {
    if value.version() > T::CURRENT_VERSION {
        return Err(PersistenceError::UnsupportedVersion {
            found: value.version(),
            supported: T::CURRENT_VERSION,
        });
    }
    Ok(())
}

fn create_temporary_file(
    directory: &Path,
    destination: &Path,
) -> Result<(PathBuf, File), io::Error> {
    let stem = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cookbench.json");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for _ in 0..32 {
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(".{stem}.{timestamp}.{}.tmp", counter));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique Cookbench persistence temporary file",
    ))
}

#[cfg(not(windows))]
fn replace_file(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary_path, destination)
}

#[cfg(windows)]
fn replace_file(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    if !destination.exists() {
        return fs::rename(temporary_path, destination);
    }

    use std::{ffi::c_void, os::windows::ffi::OsStrExt};

    extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut c_void,
            reserved: *mut c_void,
        ) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let destination = wide(destination);
    let temporary_path = wide(temporary_path);
    // ReplaceFileW preserves the old destination until Windows has installed the
    // completed replacement; remove-and-rename would expose a missing file.
    // Windows can transiently deny replacement while an indexer or reader has
    // the destination open, so follow the platform guidance and retry briefly.
    for attempt in 0..64 {
        let replaced = unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                temporary_path.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if replaced != 0 {
            return Ok(());
        }

        let error = io::Error::last_os_error();
        let retryable = matches!(error.raw_os_error(), Some(5 | 32 | 1175));
        if !retryable || attempt == 63 {
            return Err(error);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    unreachable!("bounded Windows replacement loop always returns")
}

#[cfg(unix)]
fn sync_directory(directory: &Path) {
    if let Ok(file) = File::open(directory) {
        let _ = file.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_: &Path) {}
