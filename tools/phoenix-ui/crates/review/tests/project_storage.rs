use phoenix_ui_review::{publication::ReportFile, storage::CycleLock};
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
        let root = std::env::temp_dir().join(format!(
            "phoenix-project-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const DRAFT: &[u8] = br#"{"schemaVersion":1,"revision":1,"finalization":null}"#;
const FINAL: &[u8] = br#"{"schemaVersion":1,"revision":2,"finalization":{"status":"finalized"}}"#;

#[test]
fn frozen_sources_and_plan_records_can_live_elsewhere_inside_the_project() {
    let f = Fixture::new();
    let lock =
        CycleLock::acquire_project(&f.0, Path::new("docs/reviews/feature.ui-review.json")).unwrap();
    let round = Path::new("docs/reviews/feature.ui-captures/r1/round.json");
    let draft = lock.replace(round, None, DRAFT, &[]).unwrap();
    fs::create_dir_all(f.0.join("docs/design")).unwrap();
    let source = PathBuf::from("docs/design/accepted.md");
    fs::write(f.0.join(&source), "Accepted target").unwrap();
    let source_hash = format!("{:x}", Sha256::digest(b"Accepted target"));
    let record = PathBuf::from("docs/plan/feature.records/M1.md");
    lock.prepare_publication(
        round,
        &draft,
        FINAL,
        &[ReportFile {
            path: record.clone(),
            expected_hash: None,
            bytes: b"UI acceptance repairs\n".to_vec(),
        }],
        &[(source, source_hash)],
    )
    .unwrap();
    assert!(f
        .0
        .join("docs/reviews/feature.ui-review.publish.json")
        .is_file());
    assert_eq!(lock.recover_publication().unwrap().unwrap().revision, 2);
    assert_eq!(
        fs::read(f.0.join(record)).unwrap(),
        b"UI acceptance repairs\n"
    );
    assert!(!f
        .0
        .join("docs/reviews/feature.ui-review.publish.json")
        .exists());
    assert_eq!(lock.read_finalized(round).unwrap().unwrap().bytes, FINAL);
}

#[test]
fn same_basename_cycles_have_distinct_locks_and_pending_journals() {
    let f = Fixture::new();
    let first =
        CycleLock::acquire_project(&f.0, Path::new("reviews/a/feature.ui-review.json")).unwrap();
    assert_eq!(
        CycleLock::acquire_project(&f.0, Path::new("reviews/a/feature.ui-review.json"))
            .unwrap_err()
            .code,
        3
    );
    let second =
        CycleLock::acquire_project(&f.0, Path::new("reviews/b/feature.ui-review.json")).unwrap();
    let first_round = Path::new("reviews/a/feature.ui-captures/r1/round.json");
    let draft = first.replace(first_round, None, DRAFT, &[]).unwrap();
    first
        .prepare_publication(first_round, &draft, FINAL, &[], &[])
        .unwrap();
    let second_round = Path::new("reviews/b/feature.ui-captures/r1/round.json");
    second.replace(second_round, None, DRAFT, &[]).unwrap();
    assert!(second.recover_publication().unwrap().is_none());
    first.recover_publication().unwrap();
    assert!(second.read_finalized(second_round).unwrap().is_none());
}

#[test]
fn escaping_or_absolute_index_paths_are_rejected_before_creating_directories() {
    let f = Fixture::new();
    for index in [
        f.0.join("absolute/feature.ui-review.json"),
        PathBuf::from("new/../../outside/feature.ui-review.json"),
        PathBuf::from("new/./feature.ui-review.json"),
    ] {
        assert_eq!(
            CycleLock::acquire_project(&f.0, &index).unwrap_err().code,
            2
        );
        assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn project_storage_rejects_linked_index_parents_and_outputs() {
    let f = Fixture::new();
    let outside = Fixture::new();
    std::os::unix::fs::symlink(&outside.0, f.0.join("linked")).unwrap();
    assert_eq!(
        CycleLock::acquire_project(&f.0, Path::new("linked/feature.ui-review.json"))
            .unwrap_err()
            .code,
        2
    );
    assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
    let lock = CycleLock::acquire_project(&f.0, Path::new("feature.ui-review.json")).unwrap();
    assert_eq!(
        lock.replace(Path::new("linked/round.json"), None, DRAFT, &[])
            .unwrap_err()
            .code,
        2
    );
    assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
}
