use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::envelope::EventEnvelope;

pub const MAX_ENVELOPES: usize = 128;
pub const MAX_SPOOL_BYTES: u64 = 1024 * 1024;

static FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum SpoolError {
    Missing,
    Full,
    Io,
}

impl SpoolError {
    pub const fn diagnostic(&self) -> &'static str {
        match self {
            Self::Missing => "Cookbench hook spool is unavailable",
            Self::Full => "Cookbench hook spool is full",
            Self::Io => "Cookbench hook spool write failed",
        }
    }
}

pub fn write_atomic(spool: &Path, envelope: &EventEnvelope) -> Result<(), SpoolError> {
    let metadata = fs::metadata(spool).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => SpoolError::Missing,
        _ => SpoolError::Io,
    })?;
    if !metadata.is_dir() {
        return Err(SpoolError::Missing);
    }

    // A failed non-blocking lock acquisition is intentionally treated as a
    // temporary spool failure rather than making a harness wait behind another hook.
    let lock = SpoolLock::acquire(spool)?;
    let bytes = serde_json::to_vec(envelope).map_err(|_| SpoolError::Io)?;
    let (entries, used_bytes) = spool_usage(spool)?;
    if entries >= MAX_ENVELOPES || used_bytes.saturating_add(bytes.len() as u64) > MAX_SPOOL_BYTES {
        return Err(SpoolError::Full);
    }

    let sequence = FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let base_name = format!(
        "event-{}-{}-{sequence}",
        envelope.received_at_ms,
        std::process::id(),
    );
    let temporary = spool.join(format!(".{base_name}.tmp"));
    let destination = spool.join(format!("{base_name}.json"));

    let result = write_file(&temporary, &bytes).and_then(|_| fs::rename(&temporary, &destination));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(SpoolError::Io);
    }
    drop(lock);
    Ok(())
}

struct SpoolLock {
    path: PathBuf,
}

impl SpoolLock {
    fn acquire(spool: &Path) -> Result<Self, SpoolError> {
        let path = spool.join(".cookbench-hook.lock");
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&path).map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => SpoolError::Full,
            _ => SpoolError::Io,
        })?;
        Ok(Self { path })
    }
}

impl Drop for SpoolLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn spool_usage(spool: &Path) -> Result<(usize, u64), SpoolError> {
    let mut entries = 0usize;
    let mut used_bytes = 0u64;
    for entry in fs::read_dir(spool).map_err(|_| SpoolError::Io)? {
        let entry = entry.map_err(|_| SpoolError::Io)?;
        let file_type = entry.file_type().map_err(|_| SpoolError::Io)?;
        if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        {
            entries = entries.saturating_add(1);
            used_bytes =
                used_bytes.saturating_add(entry.metadata().map_err(|_| SpoolError::Io)?.len());
        }
    }
    Ok((entries, used_bytes))
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file: File = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_json_envelopes_count_toward_capacity() {
        let path =
            std::env::temp_dir().join(format!("cookbench-hook-spool-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("spool should be created");
        fs::write(path.join("incomplete.tmp"), b"{}").expect("temporary file should be written");
        fs::write(path.join("event.json"), b"{}").expect("envelope should be written");
        assert_eq!(
            spool_usage(&path).expect("usage should be available"),
            (1, 2)
        );
        fs::remove_dir_all(path).expect("spool should be removed");
    }
}
