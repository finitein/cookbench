use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use cookbench_adapters::io::{discover_jsonl_files, JsonlTailer, TailLimits, TailRecord};

#[derive(Debug)]
struct TempDir(PathBuf);

impl TempDir {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> TempDir {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "cookbench-jsonl-tailer-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).expect("temporary directory");
    TempDir(path)
}

fn append(path: &Path, text: &str) {
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open fixture")
        .write_all(text.as_bytes())
        .expect("append fixture")
}

fn records(records: Vec<TailRecord>) -> Vec<String> {
    records
        .into_iter()
        .filter_map(|record| match record {
            TailRecord::Record(record) => Some(record),
            TailRecord::Rejected(_) => None,
        })
        .collect()
}

#[test]
fn emits_appended_complete_records_once() {
    let temp = fixture();
    let path = temp.path().join("session.jsonl");
    fs::write(&path, "").unwrap();
    let mut tailer = JsonlTailer::open(temp.path(), &path, TailLimits::default()).unwrap();

    append(&path, "{\"event\":\"one\"}\n{\"event\":\"two\"}\n");
    assert_eq!(
        records(tailer.poll().unwrap()),
        ["{\"event\":\"one\"}", "{\"event\":\"two\"}"]
    );
    assert!(tailer.poll().unwrap().is_empty());
}

#[test]
fn buffers_partial_line_until_a_newline_arrives() {
    let temp = fixture();
    let path = temp.path().join("session.jsonl");
    fs::write(&path, "").unwrap();
    let mut tailer = JsonlTailer::open(temp.path(), &path, TailLimits::default()).unwrap();

    append(&path, "{\"event\":\"partial");
    assert!(tailer.poll().unwrap().is_empty());
    append(&path, "\"}\n");
    assert_eq!(records(tailer.poll().unwrap()), ["{\"event\":\"partial\"}"]);
}

#[test]
fn truncation_and_file_replacement_reset_the_cursor_safely() {
    let temp = fixture();
    let path = temp.path().join("session.jsonl");
    fs::write(&path, "{\"event\":\"first\"}\n").unwrap();
    let mut tailer = JsonlTailer::open(temp.path(), &path, TailLimits::default()).unwrap();
    assert_eq!(records(tailer.poll().unwrap()), ["{\"event\":\"first\"}"]);

    fs::write(&path, "{\"x\":1}\n").unwrap();
    assert_eq!(records(tailer.poll().unwrap()), ["{\"x\":1}"]);

    let rotated = temp.path().join("session.rotated.jsonl");
    fs::rename(&path, &rotated).unwrap();
    fs::write(&path, "{\"event\":\"replacement\"}\n").unwrap();
    assert_eq!(
        records(tailer.poll().unwrap()),
        ["{\"event\":\"replacement\"}"]
    );
}

#[test]
fn same_length_rewrite_resets_the_cursor_without_platform_file_ids() {
    let temp = fixture();
    let path = temp.path().join("session.jsonl");
    fs::write(&path, "{\"event\":\"one\"}\n").unwrap();
    let mut tailer = JsonlTailer::open(temp.path(), &path, TailLimits::default()).unwrap();
    assert_eq!(records(tailer.poll().unwrap()), ["{\"event\":\"one\"}"]);

    fs::write(&path, "{\"event\":\"two\"}\n").unwrap();
    assert_eq!(records(tailer.poll().unwrap()), ["{\"event\":\"two\"}"]);
}

#[test]
fn rejects_oversized_lines_without_retaining_them() {
    let temp = fixture();
    let path = temp.path().join("session.jsonl");
    fs::write(&path, "").unwrap();
    let limits = TailLimits {
        max_record_bytes: 16,
        max_partial_bytes: 16,
        ..TailLimits::default()
    };
    let mut tailer = JsonlTailer::open(temp.path(), &path, limits).unwrap();

    append(&path, "0123456789abcdef0123456789\n{\"ok\":true}\n");
    let output = tailer.poll().unwrap();
    assert!(matches!(output.first(), Some(TailRecord::Rejected(_))));
    assert_eq!(records(output), ["{\"ok\":true}"]);
    assert!(tailer.buffered_bytes() <= 16);
}

#[test]
fn invalid_utf8_is_isolated_to_its_record() {
    let temp = fixture();
    let path = temp.path().join("session.jsonl");
    fs::write(&path, b"{\"ok\":1}\n\xff\xfe\n{\"ok\":2}\n").unwrap();
    let mut tailer = JsonlTailer::open(temp.path(), &path, TailLimits::default()).unwrap();

    let output = tailer.poll().unwrap();
    assert!(matches!(output[1], TailRecord::Rejected(_)));
    assert_eq!(records(output), ["{\"ok\":1}", "{\"ok\":2}"]);
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_that_escape_the_configured_root() {
    use std::os::unix::fs::symlink;

    let root = fixture();
    let outside = fixture();
    let target = outside.path().join("secret.jsonl");
    fs::write(&target, "{\"not\":\"ours\"}\n").unwrap();
    let link = root.path().join("escape.jsonl");
    symlink(&target, &link).unwrap();

    assert!(JsonlTailer::open(root.path(), &link, TailLimits::default()).is_err());
    assert!(discover_jsonl_files(root.path()).unwrap().is_empty());
}

#[test]
fn discovery_reads_metadata_and_paths_without_opening_historical_bodies() {
    let temp = fixture();
    for index in 0..1_000 {
        fs::write(
            temp.path().join(format!("session-{index}.jsonl")),
            "this body is deliberately not JSON and must not be read",
        )
        .unwrap();
    }

    let discovered = discover_jsonl_files(temp.path()).unwrap();
    assert_eq!(discovered.len(), 1_000);
    assert!(discovered
        .iter()
        .all(|path| path.extension().is_some_and(|ext| ext == "jsonl")));
}
