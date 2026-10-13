use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

const DRAFT: &[u8] = br#"{"schemaVersion":1,"revision":1,"finalization":null}"#;
const FINALIZED: &[u8] = br#"{"schemaVersion":1,"revision":2,"finalization":{"status":"finalized"},"score":0.700000000000000000000000000000000001}"#;
const ROUND: &str = "r1/round.json";

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-review-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn lock(&self) -> CycleLock {
        CycleLock::acquire(&self.0.join("demo.ui-review.json")).unwrap()
    }
    fn draft(&self, lock: &CycleLock) -> Snapshot {
        fs::write(self.0.join("capture.md"), b"original capture").unwrap();
        fs::write(self.0.join("target.md"), b"accepted target").unwrap();
        lock.replace(Path::new(ROUND), None, DRAFT, &[]).unwrap()
    }
    fn journal(&self) -> PathBuf {
        self.0.join("demo.ui-review.publish.json")
    }
    fn reports(&self) -> Vec<ReportFile> {
        vec![
            ReportFile {
                path: "capture.md".into(),
                expected_hash: Some(digest(b"original capture")),
                bytes: b"new capture".to_vec(),
            },
            ReportFile {
                path: "review.md".into(),
                expected_hash: None,
                bytes: b"new review".to_vec(),
            },
        ]
    }
    fn prepare(&self, lock: &CycleLock, draft: &Snapshot) {
        lock.prepare_publication(
            Path::new(ROUND),
            draft,
            FINALIZED,
            &self.reports(),
            &[("target.md".into(), digest(b"accepted target"))],
        )
        .unwrap();
    }
    fn published(&self, lock: &CycleLock) {
        assert_eq!(fs::read(self.0.join("capture.md")).unwrap(), b"new capture");
        assert_eq!(fs::read(self.0.join("review.md")).unwrap(), b"new review");
        assert_eq!(
            lock.read_finalized(Path::new(ROUND))
                .unwrap()
                .unwrap()
                .bytes,
            FINALIZED
        );
        assert!(!self.journal().exists());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn journal_precedes_reports_and_finalized_marker_is_last() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    assert!(lock.read_finalized(Path::new(ROUND)).unwrap().is_none());
    f.prepare(&lock, &draft);
    assert!(f.journal().exists());
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
    assert_eq!(
        fs::read(f.0.join("capture.md")).unwrap(),
        b"original capture"
    );
    let mut order = Vec::new();
    lock.recover_with(|number, path| {
        assert!(f.journal().exists());
        if number > 0 {
            order.push(path.to_owned());
        }
        if number < 3 {
            assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
            assert!(lock.read_finalized(Path::new(ROUND)).unwrap().is_none());
        }
    })
    .unwrap();
    assert_eq!(
        order,
        vec![
            PathBuf::from("capture.md"),
            PathBuf::from("review.md"),
            PathBuf::from(ROUND)
        ]
    );
    f.published(&lock);
    assert!(lock.recover_publication().unwrap().is_none());
}

#[test]
fn pending_publication_blocks_new_writes_and_finalized_rounds_are_immutable() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    f.prepare(&lock, &draft);
    assert_eq!(
        lock.prepare_publication(Path::new(ROUND), &draft, FINALIZED, &f.reports(), &[])
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(
        lock.replace(
            Path::new(ROUND),
            Some(&draft),
            br#"{"schemaVersion":1,"revision":2}"#,
            &[]
        )
        .unwrap_err()
        .code,
        3
    );
    let final_snapshot = lock.recover_publication().unwrap().unwrap();
    let next = br#"{"schemaVersion":1,"revision":3,"finalization":{"status":"finalized"}}"#;
    assert_eq!(
        lock.prepare_publication(Path::new(ROUND), &final_snapshot, next, &[], &[])
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(
        lock.replace(Path::new(ROUND), Some(&final_snapshot), next, &[])
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), FINALIZED);
}

#[test]
fn preparation_rejects_invalid_marker_revision_and_drift_before_any_report_write() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    for bytes in [
        br#"{"schemaVersion":1,"revision":2}"#.as_slice(),
        br#"{"schemaVersion":1,"revision":3,"finalization":{"status":"finalized"}}"#,
        br#"{"schemaVersion":2,"revision":2,"finalization":{"status":"finalized"}}"#,
    ] {
        assert_eq!(
            lock.prepare_publication(Path::new(ROUND), &draft, bytes, &f.reports(), &[])
                .unwrap_err()
                .code,
            2
        );
        assert!(!f.journal().exists());
    }
    fs::write(f.0.join("capture.md"), b"human edit").unwrap();
    assert_eq!(
        lock.prepare_publication(Path::new(ROUND), &draft, FINALIZED, &f.reports(), &[])
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(f.0.join("capture.md")).unwrap(), b"human edit");
    assert!(!f.journal().exists());
}

#[test]
fn a_forged_snapshot_cannot_hide_a_finalized_round() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    f.prepare(&lock, &draft);
    let mut forged = lock.recover_publication().unwrap().unwrap();
    forged.bytes = br#"{"schemaVersion":1,"revision":2,"finalization":null}"#.to_vec();
    assert_eq!(
        lock.replace(
            Path::new(ROUND),
            Some(&forged),
            br#"{"schemaVersion":1,"revision":3,"finalization":null}"#,
            &[],
        )
        .unwrap_err()
        .code,
        2,
    );
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), FINALIZED);
}

#[test]
fn recovery_preflights_all_reports_and_preserves_concurrent_edits() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    f.prepare(&lock, &draft);
    fs::write(f.0.join("review.md"), b"unexpected new report").unwrap();
    assert_eq!(lock.recover_publication().unwrap_err().code, 3);
    assert_eq!(
        fs::read(f.0.join("capture.md")).unwrap(),
        b"original capture"
    );
    assert_eq!(
        fs::read(f.0.join("review.md")).unwrap(),
        b"unexpected new report"
    );
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
    assert!(f.journal().exists());
    fs::remove_file(f.0.join("review.md")).unwrap();
    lock.recover_publication().unwrap();
    f.published(&lock);
}

#[test]
fn target_drift_keeps_round_unfinalized_until_source_is_restored() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    f.prepare(&lock, &draft);
    fs::write(f.0.join("target.md"), b"changed expectation").unwrap();
    assert_eq!(lock.recover_publication().unwrap_err().code, 3);
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
    assert_eq!(
        fs::read(f.0.join("capture.md")).unwrap(),
        b"original capture"
    );
    fs::write(f.0.join("target.md"), b"accepted target").unwrap();
    lock.recover_publication().unwrap();
    f.published(&lock);
}

#[test]
fn journal_corruption_and_future_schema_cannot_publish_or_be_replaced() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    f.prepare(&lock, &draft);
    let original = fs::read(f.journal()).unwrap();
    for invalid in [
        b"invalid".to_vec(),
        String::from_utf8(original.clone())
            .unwrap()
            .replacen("\"schemaVersion\": 1", "\"schemaVersion\": 2", 1)
            .into_bytes(),
    ] {
        fs::write(f.journal(), invalid).unwrap();
        assert_eq!(lock.recover_publication().unwrap_err().code, 4);
        assert_eq!(lock.read_finalized(Path::new(ROUND)).unwrap_err().code, 4);
        assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
    }
    fs::write(f.journal(), original).unwrap();
    lock.recover_publication().unwrap();
    f.published(&lock);
}

#[test]
fn publication_paths_reject_escapes_aliases_and_internal_files() {
    let f = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    for path in [
        PathBuf::from("../outside.md"),
        f.0.join("absolute.md"),
        "demo.ui-review.lock".into(),
        "demo.ui-review.publish.json".into(),
        ROUND.into(),
    ] {
        let report = ReportFile {
            path,
            expected_hash: None,
            bytes: b"new".to_vec(),
        };
        assert_eq!(
            lock.prepare_publication(Path::new(ROUND), &draft, FINALIZED, &[report], &[])
                .unwrap_err()
                .code,
            2
        );
        assert!(!f.journal().exists());
    }
    let mut reports = f.reports();
    reports.push(reports[0].clone());
    assert_eq!(
        lock.prepare_publication(Path::new(ROUND), &draft, FINALIZED, &reports, &[])
            .unwrap_err()
            .code,
        2
    );
}

#[cfg(unix)]
#[test]
fn recovery_refuses_a_symlink_inserted_after_preparation() {
    let f = Fixture::new();
    let outside = Fixture::new();
    let lock = f.lock();
    let draft = f.draft(&lock);
    f.prepare(&lock, &draft);
    fs::write(outside.0.join("report.md"), b"outside").unwrap();
    std::os::unix::fs::symlink(outside.0.join("report.md"), f.0.join("review.md")).unwrap();
    assert_eq!(lock.recover_publication().unwrap_err().code, 2);
    assert_eq!(fs::read(outside.0.join("report.md")).unwrap(), b"outside");
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
}

#[test]
fn process_exit_recovers_each_publication_boundary() {
    const CHILD: &str = "PHOENIX_UI_REVIEW_PUBLICATION_CHILD";
    if let Some(root) = std::env::var_os(CHILD) {
        let f = std::mem::ManuallyDrop::new(Fixture(root.into()));
        let lock = f.lock();
        let draft = f.draft(&lock);
        f.prepare(&lock, &draft);
        let stop: usize = std::env::var("PHOENIX_UI_REVIEW_PUBLICATION_STOP")
            .unwrap()
            .parse()
            .unwrap();
        lock.recover_with(|count, _| {
            if count == stop {
                fs::write(f.0.join("ready"), b"paused").unwrap();
                loop {
                    std::thread::park();
                }
            }
        })
        .unwrap();
        panic!("Publication never reached its interruption boundary");
    }
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    for stop in 0..=3 {
        let f = Fixture::new();
        let mut child = ChildGuard(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "publication::tests::process_exit_recovers_each_publication_boundary",
                    "--nocapture",
                ])
                .env(CHILD, &f.0)
                .env("PHOENIX_UI_REVIEW_PUBLICATION_STOP", stop.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
        let start = std::time::Instant::now();
        while !f.0.join("ready").exists() {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "Publication child exited before interruption"
            );
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "Publication child did not reach interruption"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let lock = f.lock();
        assert_eq!(
            lock.read_finalized(Path::new(ROUND)).unwrap().is_some(),
            stop == 3
        );
        lock.recover_publication().unwrap();
        f.published(&lock);
    }
}

#[test]
fn initial_draft_commits_frozen_files_before_cycle_and_can_resume_idempotently() {
    let f = Fixture::new();
    let lock = f.lock();
    let files = vec![
        ReportFile {
            path: "r1/target.md".into(),
            expected_hash: None,
            bytes: b"frozen expectation".to_vec(),
        },
        ReportFile {
            path: ROUND.into(),
            expected_hash: None,
            bytes: DRAFT.to_vec(),
        },
    ];
    let cycle = Path::new("demo.ui-review.json");
    lock.prepare_draft(cycle, None, DRAFT, &files, &[]).unwrap();
    assert!(!f.0.join(cycle).exists());
    assert!(!f.0.join("r1/target.md").exists());
    let mut writes = Vec::new();
    lock.recover_with(|number, path| {
        if number > 0 {
            writes.push(path.to_owned());
        }
        if number < 3 {
            assert!(!f.0.join(cycle).exists());
        }
    })
    .unwrap();
    assert_eq!(
        writes,
        vec![PathBuf::from("r1/target.md"), ROUND.into(), cycle.into()]
    );
    assert_eq!(lock.read(cycle).unwrap().unwrap().revision, 1);
    assert_eq!(
        fs::read(f.0.join("r1/target.md")).unwrap(),
        b"frozen expectation"
    );
    assert!(lock.recover_publication().unwrap().is_none());
}

#[test]
fn draft_updates_require_exact_revision_and_preserve_later_edits() {
    let f = Fixture::new();
    let lock = f.lock();
    let old = f.draft(&lock);
    let next = br#"{"schemaVersion":1,"revision":2,"finalization":null,"captures":["a1"]}"#;
    lock.prepare_draft(Path::new(ROUND), Some(&old), next, &f.reports(), &[])
        .unwrap();
    fs::write(f.0.join("capture.md"), b"manual edit").unwrap();
    assert_eq!(lock.recover_publication().unwrap_err().code, 3);
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
    assert_eq!(fs::read(f.0.join("capture.md")).unwrap(), b"manual edit");
    fs::write(f.0.join("capture.md"), b"original capture").unwrap();
    let current = lock.recover_publication().unwrap().unwrap();
    assert_eq!(current.bytes, next);
    assert!(lock.read_finalized(Path::new(ROUND)).unwrap().is_none());
    assert_eq!(
        lock.prepare_draft(Path::new(ROUND), Some(&old), next, &[], &[])
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn draft_journal_cannot_finalize_skip_revisions_or_reopen_history() {
    let f = Fixture::new();
    let lock = f.lock();
    let old = f.draft(&lock);
    for next in [DRAFT, FINALIZED, br#"{"schemaVersion":1,"revision":3}"#] {
        assert_eq!(
            lock.prepare_draft(Path::new(ROUND), Some(&old), next, &[], &[])
                .unwrap_err()
                .code,
            2
        );
        assert!(!f.journal().exists());
    }
    assert_eq!(
        lock.prepare_draft(Path::new("cycle.json"), None, FINALIZED, &[], &[])
            .unwrap_err()
            .code,
        2
    );
    f.prepare(&lock, &old);
    let final_state = lock.recover_publication().unwrap().unwrap();
    assert_eq!(
        lock.prepare_draft(
            Path::new(ROUND),
            Some(&final_state),
            br#"{"schemaVersion":1,"revision":3}"#,
            &[],
            &[]
        )
        .unwrap_err()
        .code,
        3
    );
    assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), FINALIZED);
}

#[test]
fn process_exit_recovers_each_initial_draft_boundary() {
    const CHILD: &str = "PHOENIX_UI_REVIEW_DRAFT_CHILD";
    const STOP: &str = "PHOENIX_UI_REVIEW_DRAFT_STOP";
    if let Some(root) = std::env::var_os(CHILD) {
        let f = std::mem::ManuallyDrop::new(Fixture(root.into()));
        let lock = f.lock();
        let files = vec![
            ReportFile {
                path: "r1/target.md".into(),
                expected_hash: None,
                bytes: b"expectation".to_vec(),
            },
            ReportFile {
                path: ROUND.into(),
                expected_hash: None,
                bytes: DRAFT.to_vec(),
            },
        ];
        lock.prepare_draft(Path::new("demo.ui-review.json"), None, DRAFT, &files, &[])
            .unwrap();
        let stop: usize = std::env::var(STOP).unwrap().parse().unwrap();
        lock.recover_with(|count, _| {
            if count == stop {
                fs::write(f.0.join("ready"), b"paused").unwrap();
                loop {
                    std::thread::park();
                }
            }
        })
        .unwrap();
        panic!("Draft did not reach its interruption boundary");
    }
    for stop in 0..=3 {
        let f = Fixture::new();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "publication::tests::process_exit_recovers_each_initial_draft_boundary",
                "--nocapture",
            ])
            .env(CHILD, &f.0)
            .env(STOP, stop.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        let ready = loop {
            if f.0.join("ready").exists() {
                break true;
            }
            if child.try_wait().unwrap().is_some()
                || start.elapsed() > std::time::Duration::from_secs(10)
            {
                break false;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let _ = child.kill();
        let status = child.wait().unwrap();
        assert!(ready, "Draft child exited before interruption: {status}");
        let lock = f.lock();
        assert_eq!(f.0.join("demo.ui-review.json").exists(), stop == 3);
        lock.recover_publication().unwrap();
        assert_eq!(fs::read(f.0.join("r1/target.md")).unwrap(), b"expectation");
        assert_eq!(fs::read(f.0.join(ROUND)).unwrap(), DRAFT);
        assert_eq!(
            lock.read(Path::new("demo.ui-review.json"))
                .unwrap()
                .unwrap()
                .revision,
            1
        );
        assert!(lock.recover_publication().unwrap().is_none());
    }
}
