use phoenix_ui_review::storage::CycleLock;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-review-storage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn lock(&self) -> CycleLock {
        CycleLock::acquire(&self.0.join("demo.ui-review.json")).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const FIRST: &[u8] =
    br#"{"schemaVersion":1,"revision":1,"score":0.700000000000000000000000000000000001}"#;
const SECOND: &[u8] = br#"{"schemaVersion":1,"revision":2,"score":"8"}"#;

#[test]
fn create_and_replace_preserve_raw_decimal_bytes_and_revision() {
    let f = Fixture::new();
    let lock = f.lock();
    let path = Path::new("r1/round.json");
    assert!(lock.read(path).unwrap().is_none());
    let first = lock.replace(path, None, FIRST, &[]).unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(first.bytes, FIRST);
    let second = lock.replace(path, Some(&first), SECOND, &[]).unwrap();
    assert_eq!(second.revision, 2);
    assert_eq!(fs::read(f.0.join(path)).unwrap(), SECOND);
    assert_eq!(fs::read_dir(f.0.join("r1")).unwrap().count(), 1);
}

#[test]
fn stale_revision_and_same_revision_source_edits_both_conflict() {
    let f = Fixture::new();
    let lock = f.lock();
    let path = Path::new("round.json");
    let first = lock.replace(path, None, FIRST, &[]).unwrap();
    assert_eq!(lock.replace(path, None, FIRST, &[]).unwrap_err().code, 3);
    let second = lock.replace(path, Some(&first), SECOND, &[]).unwrap();
    assert_eq!(
        lock.replace(path, Some(&first), SECOND, &[])
            .unwrap_err()
            .code,
        3
    );
    fs::write(
        f.0.join(path),
        br#"{"schemaVersion":1,"revision":2,"score":"7"}"#,
    )
    .unwrap();
    assert_eq!(
        lock.replace(
            path,
            Some(&second),
            br#"{"schemaVersion":1,"revision":3}"#,
            &[]
        )
        .unwrap_err()
        .code,
        3
    );
}

#[test]
fn only_next_revision_and_known_schema_can_be_written() {
    let f = Fixture::new();
    let lock = f.lock();
    let path = Path::new("round.json");
    for bytes in [
        br#"{"schemaVersion":2,"revision":1}"#.as_slice(),
        br#"{"schemaVersion":1,"revision":0}"#,
        br#"{"schemaVersion":1,"revision":2}"#,
        b"invalid",
    ] {
        assert_eq!(lock.replace(path, None, bytes, &[]).unwrap_err().code, 2);
        assert!(!f.0.join(path).exists());
    }
    fs::write(f.0.join(path), br#"{"schemaVersion":2,"revision":1}"#).unwrap();
    assert_eq!(lock.read(path).unwrap_err().code, 4);
}

#[test]
fn source_drift_refuses_publication_and_keeps_original_bytes() {
    let f = Fixture::new();
    let lock = f.lock();
    let path = Path::new("round.json");
    let first = lock.replace(path, None, FIRST, &[]).unwrap();
    fs::write(f.0.join("target.md"), "accepted target").unwrap();
    let hash = format!("{:x}", Sha256::digest(b"accepted target"));
    fs::write(f.0.join("target.md"), "edited target").unwrap();
    assert_eq!(
        lock.replace(
            path,
            Some(&first),
            SECOND,
            &[(PathBuf::from("target.md"), hash)]
        )
        .unwrap_err()
        .code,
        3
    );
    assert_eq!(fs::read(f.0.join(path)).unwrap(), FIRST);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 3);
}

#[test]
fn same_cycle_locks_conflict_and_different_cycles_are_independent() {
    let f = Fixture::new();
    let lock = f.lock();
    assert_eq!(
        CycleLock::acquire(&f.0.join("demo.ui-review.json"))
            .unwrap_err()
            .code,
        3
    );
    let other = CycleLock::acquire(&f.0.join("other.ui-review.json")).unwrap();
    drop(other);
    drop(lock);
    let recovered = f.lock();
    drop(recovered);
}

#[test]
fn process_exit_releases_the_cycle_lock() {
    const CHILD: &str = "PHOENIX_UI_REVIEW_STORAGE_LOCK_CHILD";
    if let Some(index) = std::env::var_os(CHILD) {
        let index = PathBuf::from(index);
        let _lock = CycleLock::acquire(&index).unwrap();
        fs::write(index.with_extension("ready"), b"locked").unwrap();
        loop {
            std::thread::park();
        }
    }
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new();
    let index = f.0.join("demo.ui-review.json");
    let mut child = ChildGuard(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "process_exit_releases_the_cycle_lock",
                "--nocapture",
            ])
            .env(CHILD, &index)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let started = std::time::Instant::now();
    while !index.with_extension("ready").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "Lock holder exited before acquiring"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "Lock holder did not start"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(CycleLock::acquire(&index).unwrap_err().code, 3);
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let recovered = CycleLock::acquire(&index).unwrap();
    drop(recovered);
}

#[test]
fn relative_paths_cannot_escape_the_cycle_directory() {
    let f = Fixture::new();
    let lock = f.lock();
    for path in [PathBuf::from("../outside.json"), f.0.join("absolute.json")] {
        assert_eq!(lock.replace(&path, None, FIRST, &[]).unwrap_err().code, 2);
    }
}

#[cfg(unix)]
#[test]
fn symlinked_state_paths_are_rejected() {
    let f = Fixture::new();
    let outside = Fixture::new();
    let lock = f.lock();
    std::os::unix::fs::symlink(&outside.0, f.0.join("linked")).unwrap();
    assert_eq!(
        lock.replace(Path::new("linked/round.json"), None, FIRST, &[])
            .unwrap_err()
            .code,
        2
    );
    assert!(!outside.0.join("round.json").exists());
}
