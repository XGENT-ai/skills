use impeccable_live::paths::{live_server_path, read_live_server_info, write_live_server_info};
use impeccable_live::util::Env;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let name = format!(
            "phoenix server info {} {}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        Self(std::fs::canonicalize(dir).unwrap())
    }

    fn cwd(&self) -> &str {
        self.0.to_str().unwrap()
    }
    fn file(&self) -> PathBuf {
        PathBuf::from(live_server_path(self.cwd(), &Env::new()))
    }
    fn write(&self, value: &Value) -> String {
        write_live_server_info(self.cwd(), &Env::new(), value)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn record(token: &str) -> Value {
    json!({"pid": std::process::id(), "port": 8400, "token": token})
}

fn assert_only_server_file(file: &Path) {
    let entries: Vec<_> = std::fs::read_dir(file.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, ["server.json"]);
}

#[test]
fn server_info_replaces_an_existing_complete_record() {
    let fixture = Fixture::new();
    let first = record("first");
    let second = record("second");
    assert_eq!(Path::new(&fixture.write(&first)), fixture.file());
    assert_eq!(
        read_live_server_info(fixture.cwd(), &Env::new())
            .unwrap()
            .0
            .raw,
        first
    );
    assert_eq!(Path::new(&fixture.write(&second)), fixture.file());
    assert_eq!(
        read_live_server_info(fixture.cwd(), &Env::new())
            .unwrap()
            .0
            .raw,
        second
    );
    assert_only_server_file(&fixture.file());
}

#[test]
fn readers_never_observe_partial_json_during_concurrent_publication() {
    let fixture = Fixture::new();
    let values: Vec<_> = ['a', 'b', 'c', 'd']
        .into_iter()
        .map(|letter| record(&letter.to_string().repeat(2 * 1024 * 1024)))
        .collect();
    let old = record("previous");
    fixture.write(&old);
    let mut complete = vec![serde_json::to_vec(&old).unwrap()];
    complete.extend(
        values
            .iter()
            .map(|value| serde_json::to_vec(value).unwrap()),
    );
    let complete = Arc::new(complete);
    let stop = Arc::new(AtomicBool::new(false));
    let start = Arc::new(Barrier::new(values.len() + 2));
    let file = fixture.file();
    let reader = {
        let (complete, stop, start, file) =
            (complete.clone(), stop.clone(), start.clone(), file.clone());
        std::thread::spawn(move || {
            start.wait();
            let mut reads = 0;
            while !stop.load(Ordering::Acquire) || reads == 0 {
                let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
                reads += 1;
                if !complete.iter().any(|value| value == &bytes) {
                    let parsed = serde_json::from_slice::<Value>(&bytes)
                        .err()
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| "JSON mixed concurrent records".into());
                    return Err(format!(
                        "read {reads} observed {} bytes, not a complete published record: {parsed}",
                        bytes.len()
                    ));
                }
            }
            Ok::<_, String>(reads)
        })
    };
    let writers: Vec<_> = values
        .into_iter()
        .map(|value| {
            let start = start.clone();
            let cwd = fixture.cwd().to_string();
            std::thread::spawn(move || {
                start.wait();
                for _ in 0..12 {
                    write_live_server_info(&cwd, &Env::new(), &value);
                }
            })
        })
        .collect();
    start.wait();
    for writer in writers {
        writer.join().unwrap();
    }
    stop.store(true, Ordering::Release);
    assert!(reader.join().unwrap().unwrap() > 0);
    let final_bytes = std::fs::read(&file).unwrap();
    assert!(complete.iter().skip(1).any(|value| value == &final_bytes));
    assert_only_server_file(&file);
}

#[test]
fn first_publication_is_complete_as_soon_as_the_file_exists() {
    let fixture = Fixture::new();
    let value = record(&"initial".repeat(1024 * 1024));
    let expected = serde_json::to_vec(&value).unwrap();
    let cwd = fixture.cwd().to_string();
    let file = fixture.file();
    let writer = std::thread::spawn(move || write_live_server_info(&cwd, &Env::new(), &value));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !file.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "server info never appeared"
        );
        std::thread::yield_now();
    }
    // Read exactly once when existence is first observable; retrying an
    // invalid record here would conceal the startup race.
    let actual = std::fs::read(&file).unwrap();
    writer.join().unwrap();
    assert_eq!(
        actual.len(),
        expected.len(),
        "first visible record was partial"
    );
    assert_eq!(actual, expected);
    assert_only_server_file(&file);
}

#[test]
fn failed_publication_preserves_the_previous_complete_record() {
    let fixture = Fixture::new();
    let old = record("previous");
    fixture.write(&old);
    let file = fixture.file();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let parent = file.parent().unwrap();
        let permissions = std::fs::metadata(parent).unwrap().permissions();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let denied = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(parent.join("permission-probe"));
        fixture.write(&record("replacement"));
        std::fs::set_permissions(parent, permissions).unwrap();
        assert_eq!(
            denied.err().map(|error| error.kind()),
            Some(std::io::ErrorKind::PermissionDenied),
            "this failure fixture requires an unprivileged Unix user"
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Permit data reads/writes but deny deletion/rename of this record.
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0x1 | 0x2)
            .open(&file)
            .unwrap();
        fixture.write(&record("replacement"));
        drop(held);
    }
    assert_eq!(
        std::fs::read(&file).unwrap(),
        serde_json::to_vec(&old).unwrap()
    );
    assert_only_server_file(&file);
}

#[test]
fn failed_rename_preserves_the_destination_and_cleans_staging() {
    let fixture = Fixture::new();
    let file = fixture.file();
    std::fs::create_dir_all(&file).unwrap();
    let old = serde_json::to_vec(&record("previous")).unwrap();
    std::fs::write(file.join("existing-record.json"), &old).unwrap();
    fixture.write(&record("replacement"));
    assert!(file.is_dir());
    assert_eq!(
        std::fs::read(file.join("existing-record.json")).unwrap(),
        old
    );
    assert_only_server_file(&file);
}
