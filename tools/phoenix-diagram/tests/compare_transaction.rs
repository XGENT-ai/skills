#[path = "../src/output.rs"]
#[allow(dead_code)] // This private copy exposes only the pair-transaction observer seam here.
#[expect(
    unused_imports,
    reason = "The worker-only checkpoint is unused in this private output copy"
)]
mod output;
#[path = "../src/compare/transaction.rs"]
mod transaction;

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
use phoenix_diagram::diagnostic;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use transaction::{test_support::commit as commit_observed, PairPhase};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "phoenix-compare-transaction-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn before_pair(root: &Workspace) -> (PathBuf, PathBuf) {
    let html = root.path("comparison.html");
    let receipt = root.path("comparison.html.compare.json");
    fs::write(&html, b"old html").unwrap();
    fs::write(&receipt, b"old receipt").unwrap();
    (html, receipt)
}
fn assert_bytes(file: &Path, expected: &[u8]) {
    assert_eq!(fs::read(file).unwrap(), expected);
}

#[test]
fn commit_binds_both_files_and_read_only_verification_rejects_later_claimants() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let transaction_id = tx.id().to_owned();
    let directory = tx.recovery_directory().to_owned();
    let publication = tx.commit(b"new html", b"new receipt", &[]).unwrap();
    assert_eq!(publication.transaction_id, transaction_id);
    assert_eq!(publication.recovery_directory, directory);
    assert_bytes(&html, b"new html");
    assert_bytes(&receipt, b"new receipt");
    transaction::verify_committed(&directory, &publication.html, &publication.receipt).unwrap();
    fs::remove_file(&receipt).unwrap();
    fs::write(&receipt, b"new receipt").unwrap();
    let changed = output::FileBinding::capture(&receipt).unwrap();
    assert!(transaction::verify_committed(&directory, &publication.html, &changed).is_err());
}

#[test]
fn pair_rejects_different_parents_and_input_or_output_aliases() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    assert!(transaction::begin(&html, &html, &[]).is_err());
    let input = output::FileBinding::capture(&html).unwrap();
    assert!(transaction::begin(&html, &receipt, &[&input]).is_err());
    let other = root.path("other");
    fs::create_dir(&other).unwrap();
    assert!(transaction::begin(&html, &other.join("receipt.json"), &[]).is_err());
}

fn interrupt() -> std::io::Error {
    std::io::Error::other("injected pair interruption")
}

#[test]
fn second_member_failure_restores_both_old_files_and_recovery_is_idempotent() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let directory = tx.recovery_directory().to_owned();
    let result = commit_observed(tx, b"new html", b"new receipt", &[], &mut |phase, _| {
        if phase == PairPhase::BeforeMember(1) {
            return Err(interrupt());
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_bytes(&html, b"old html");
    assert_bytes(&receipt, b"old receipt");
    assert_eq!(transaction::recover(&directory).exit_code(), 0);
    assert_eq!(transaction::recover(&directory).exit_code(), 0);
    assert!(transaction::verify_committed(
        &directory,
        &output::FileBinding::capture(&html).unwrap(),
        &output::FileBinding::capture(&receipt).unwrap()
    )
    .is_err());
}

#[test]
fn partial_first_publication_failure_restores_original_pair() {
    for interrupted_phase in [
        output::Phase::AfterRetire,
        output::Phase::AfterPublishLink,
        output::Phase::BeforeBackupCleanup,
    ] {
        let root = Workspace::new();
        let (html, receipt) = before_pair(&root);
        let tx = transaction::begin(&html, &receipt, &[]).unwrap();
        assert!(
            commit_observed(tx, b"new html", b"new receipt", &[], &mut |phase, _| {
                if phase == PairPhase::Publication(0, interrupted_phase) {
                    return Err(interrupt());
                }
                Ok(())
            })
            .is_err()
        );
        assert_bytes(&html, b"old html");
        assert_bytes(&receipt, b"old receipt");
    }
}

#[test]
fn originally_absent_pair_is_restored_to_absence() {
    let root = Workspace::new();
    let html = root.path("comparison.html");
    let receipt = root.path("comparison.json");
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let directory = tx.recovery_directory().to_owned();
    assert!(
        commit_observed(tx, b"new html", b"new receipt", &[], &mut |phase, _| {
            if phase == PairPhase::BeforeMember(1) {
                return Err(interrupt());
            }
            Ok(())
        })
        .is_err()
    );
    assert!(!html.exists());
    assert!(!receipt.exists());
    assert_eq!(transaction::recover(&directory).exit_code(), 0);
}

#[test]
fn later_claimant_is_preserved_and_prevents_any_pair_rollback_until_resolved() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let directory = tx.recovery_directory().to_owned();
    let result = commit_observed(tx, b"new html", b"new receipt", &[], &mut |phase, _| {
        if phase == PairPhase::BeforeMember(1) {
            fs::remove_file(&receipt)?;
            fs::write(&receipt, b"later receipt")?;
            return Err(interrupt());
        }
        Ok(())
    });
    assert!(matches!(result, Err(ref error) if error.kind == output::ErrorKind::RecoveryRequired));
    assert_bytes(&html, b"new html");
    assert_bytes(&receipt, b"later receipt");
    assert_eq!(transaction::recover(&directory).exit_code(), 3);
    fs::remove_file(&receipt).unwrap();
    assert_eq!(transaction::recover(&directory).exit_code(), 0);
    assert_bytes(&html, b"old html");
    assert_bytes(&receipt, b"old receipt");
}

#[test]
fn changed_input_prevents_commit_and_restores_old_pair() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let input_path = root.path("head.json");
    fs::write(&input_path, b"input").unwrap();
    let input = output::FileBinding::capture(&input_path).unwrap();
    let tx = transaction::begin(&html, &receipt, &[&input]).unwrap();
    assert!(commit_observed(
        tx,
        b"new html",
        b"new receipt",
        &[&input],
        &mut |phase, _| {
            if phase == PairPhase::BeforeMember(1) {
                fs::write(&input_path, b"changed input")?;
            }
            Ok(())
        }
    )
    .is_err());
    assert_bytes(&html, b"old html");
    assert_bytes(&receipt, b"old receipt");
    assert_bytes(&input_path, b"changed input");
}

#[test]
fn tampered_backup_preserves_current_pair_and_recovery_location() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let directory = tx.recovery_directory().to_owned();
    assert!(
        commit_observed(tx, b"new html", b"new receipt", &[], &mut |phase, dir| {
            if phase == PairPhase::BeforeMember(1) {
                fs::write(dir.join("before-0"), b"tampered backup")?;
                return Err(interrupt());
            }
            Ok(())
        })
        .is_err()
    );
    assert_eq!(transaction::recover(&directory).exit_code(), 3);
    assert_bytes(&html, b"new html");
    assert_bytes(&receipt, b"old receipt");
    assert_bytes(&directory.join("before-0"), b"tampered backup");
}

#[test]
fn prepared_transaction_can_be_recovered_without_publishing() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let directory = tx.recovery_directory().to_owned();
    drop(tx);
    assert_eq!(transaction::recover(&directory).exit_code(), 0);
    assert_bytes(&html, b"old html");
    assert_bytes(&receipt, b"old receipt");
    assert!(!directory.join("before-0").exists());
    assert!(!directory.join("before-1").exists());
}

#[test]
fn committed_recovery_only_cleans_owned_files_and_preserves_later_output() {
    let root = Workspace::new();
    let (html, receipt) = before_pair(&root);
    let tx = transaction::begin(&html, &receipt, &[]).unwrap();
    let directory = tx.recovery_directory().to_owned();
    let publication = tx.commit(b"new html", b"new receipt", &[]).unwrap();
    assert_eq!(transaction::recover(&directory).exit_code(), 0);
    publication.html.verify().unwrap();
    publication.receipt.verify().unwrap();
    fs::write(&html, b"later html").unwrap();
    assert_eq!(transaction::recover(&directory).exit_code(), 3);
    assert_bytes(&html, b"later html");
    assert_bytes(&receipt, b"new receipt");
}

#[test]
fn same_content_replaced_backup_and_unknown_private_entries_are_preserved() {
    for unknown in [false, true] {
        let root = Workspace::new();
        let (html, receipt) = before_pair(&root);
        let tx = transaction::begin(&html, &receipt, &[]).unwrap();
        let directory = tx.recovery_directory().to_owned();
        drop(tx);
        let mut replaced_backup = None;
        if unknown {
            fs::write(directory.join("foreign"), b"foreign").unwrap();
        } else {
            let backup = directory.join("before-0");
            let original = output::FileBinding::capture(&backup).unwrap();
            let temporary = directory.join("replacement");
            fs::write(&temporary, original.bytes()).unwrap();
            fs::set_permissions(
                &temporary,
                fs::Permissions::from_mode(original.identity().mode),
            )
            .unwrap();
            fs::rename(&temporary, &backup).unwrap();
            let replacement = output::FileBinding::capture(&backup).unwrap();
            let mut identity = replacement.identity().clone();
            assert_ne!(identity.inode, original.identity().inode);
            identity.inode = original.identity().inode.clone();
            assert_eq!(&identity, original.identity());
            replaced_backup = Some(original);
        }
        assert_eq!(transaction::recover(&directory).exit_code(), 3);
        assert_bytes(&html, b"old html");
        assert_bytes(&receipt, b"old receipt");
        drop(replaced_backup);
    }
}
