// Real temporary filesystems and real child-process SIGKILL; no in-memory FS.
#[path = "../src/output.rs"]
mod output;
use output::test_support::{self, Phase};
use output::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{symlink, DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let name = format!(
            "phoenix-output-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
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
fn write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
}
fn old_output(root: &Workspace) -> PathBuf {
    let output = root.path("diagram.html");
    write(&output, b"old artifact\n");
    fs::set_permissions(&output, fs::Permissions::from_mode(0o640)).unwrap();
    output
}
fn observer_error() -> std::io::Error {
    std::io::Error::other("injected interruption")
}
fn recovery_in(root: &Workspace) -> PathBuf {
    let names: Vec<_> = fs::read_dir(&root.0)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            if name.starts_with(".phoenix-diagram-recovery-") {
                if entry.file_type().ok()?.is_dir() {
                    Some(entry.path())
                } else {
                    name.strip_suffix(".cleanup.json")
                        .map(|name| root.path(name))
                }
            } else {
                None
            }
        })
        .collect();
    let mut names = names;
    if let Ok(entries) = fs::read_dir(root.path(".phoenix-diagram-output-cleanup")) {
        names.extend(entries.map(|entry| entry.unwrap()).filter_map(|entry| {
            let name = entry.file_name();
            let id = name.to_str()?.strip_suffix(".json")?;
            (id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| root.path(&format!(".phoenix-diagram-recovery-{id}")))
        }));
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 1, "recovery locators: {names:?}");
    names.pop().unwrap()
}
fn crash(root: &Workspace, mode: &str, phase: Phase) -> std::process::Output {
    crash_at(&root.0, mode, phase)
}
fn crash_at(root: &Path, mode: &str, phase: Phase) -> std::process::Output {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "output_child_process", "--nocapture"])
        .env("PHOENIX_OUTPUT_TEST_ROOT", root)
        .env("PHOENIX_OUTPUT_TEST_MODE", mode)
        .env("PHOENIX_OUTPUT_TEST_PHASE", format!("{phase:?}"))
        .output()
        .unwrap();
    assert_eq!(
        status.status.signal(),
        Some(libc::SIGKILL),
        "child {:?}\n{}\n{}",
        status.status,
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    status
}
fn interrupted() -> (Workspace, PathBuf, PathBuf) {
    let root = Workspace::new();
    let output = old_output(&root);
    let child = crash(&root, "publish", Phase::AfterRetire);
    let recovery = recovery_in(&root);
    assert!(
        !output.exists(),
        "public after SIGKILL: {:?}; child checkpoint: {}",
        fs::read(&output),
        String::from_utf8_lossy(&child.stderr)
    );
    assert_eq!(
        fs::read(recovery.join("previous")).unwrap(),
        b"old artifact\n"
    );
    (root, output, recovery)
}

#[test]
fn output_child_process() {
    let Some(root) = std::env::var_os("PHOENIX_OUTPUT_TEST_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let mode = std::env::var("PHOENIX_OUTPUT_TEST_MODE").unwrap();
    let wanted = std::env::var("PHOENIX_OUTPUT_TEST_PHASE").unwrap_or_default();
    let mut observer = |phase: Phase, paths: &output::test_support::Observation| {
        if format!("{phase:?}") == wanted {
            eprintln!(
                "kill checkpoint {phase:?}: public={:?}",
                fs::read(&paths.target)
            );
            if unsafe { libc::kill(libc::getpid(), libc::SIGKILL) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
            // Self-SIGKILL may return before Darwin terminates every thread.
            // Do not execute publication code beyond the claimed checkpoint.
            loop {
                unsafe { libc::pause() };
            }
        }
        Ok(())
    };
    match mode.as_str() {
        "publish" => {
            let slot = capture_slot(&root.join("diagram.html"), &[]).unwrap();
            test_support::publish(&slot, b"new artifact\n", &[], &mut observer).unwrap();
        }
        "recover" => {
            let root = Workspace(root);
            let recovery = recovery_in(&root);
            let result = test_support::recover(&recovery, &mut observer);
            std::mem::forget(root);
            assert_eq!(result.exit_code(), 0, "{result:?}");
        }
        "capture" => match capture_slot(&root.join("diagram.html"), &[]) {
            Ok(_) => std::process::exit(0),
            Err(error) => std::process::exit(error.exit_code() as i32),
        },
        "pending" => match pending_existing_publications(&root.join("diagram.html")) {
            Ok(_) => std::process::exit(0),
            Err(error) => std::process::exit(error.exit_code() as i32),
        },
        _ => panic!("Unknown child mode"),
    }
}

#[test]
fn output_snapshot_is_fd_bound_and_input_changes_prevent_publication() {
    let root = Workspace::new();
    let input = root.path("input.json");
    write(&input, b"original input");
    let input = FileBinding::capture(&input).unwrap();
    assert_eq!(input.bytes(), b"original input");
    let output = old_output(&root);
    let slot = capture_slot(&output, &[&input]).unwrap();
    write(input.path(), b"changed input");
    let error = publish_bound(&slot, b"new", &[&input]).unwrap_err();
    assert_eq!(error.code, "input/changed");
    assert_eq!(fs::read(output).unwrap(), b"old artifact\n");
    assert_eq!(input.bytes(), b"original input");
}

#[test]
fn output_creates_and_replaces_exact_bytes_and_mode() {
    let root = Workspace::new();
    let output = root.path("new.html");
    {
        let slot = capture_slot(&output, &[]).unwrap();
        assert!(slot.previous().is_none());
        let result = publish_bound(&slot, b"new artifact\n", &[]).unwrap();
        assert_eq!(result.exit_code(), 0);
        assert!(result.cleanup_complete);
        assert_eq!(fs::read(&output).unwrap(), b"new artifact\n");
        assert_eq!(fs::metadata(&output).unwrap().nlink(), 1);
        assert_eq!(result.identity.bytes, 13);
        assert!(result.recovery_directory.is_none());
    }
    {
        fs::set_permissions(&output, fs::Permissions::from_mode(0o640)).unwrap();
        let slot = capture_slot(&output, &[]).unwrap();
        let previous = slot.previous().unwrap().identity().inode.clone();
        let result = publish_bound(&slot, b"replacement", &[]).unwrap();
        assert!(result.cleanup_complete);
        assert_ne!(result.identity.inode, previous);
        assert_eq!(fs::metadata(&output).unwrap().mode() & 0o777, 0o640);
        assert_eq!(fs::read(output).unwrap(), b"replacement");
    }
}

#[test]
fn output_rejects_input_aliases_and_unknown_entries_without_creating_a_lock() {
    let root = Workspace::new();
    let input = root.path("input.json");
    write(&input, b"input");
    let binding = FileBinding::capture(&input).unwrap();
    assert!(capture_slot(&input, &[&binding]).is_err());
    let alias = root.path("alias.html");
    symlink(&input, &alias).unwrap();
    assert!(capture_slot(&alias, &[&binding]).is_err());
    fs::remove_file(&alias).unwrap();
    fs::hard_link(&input, &alias).unwrap();
    assert!(capture_slot(&alias, &[]).is_err());
    assert_eq!(fs::read(&alias).unwrap(), b"input");
    let directory = root.path("directory.html");
    fs::create_dir(&directory).unwrap();
    assert!(capture_slot(&directory, &[]).is_err());
    assert!(fs::read_dir(&root.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".phoenix-diagram-lock-")
    }));
}

#[test]
fn output_os_lock_is_process_scoped_and_sidecars_share_the_existing_guard() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "output_child_process"])
        .env("PHOENIX_OUTPUT_TEST_ROOT", &root.0)
        .env("PHOENIX_OUTPUT_TEST_MODE", "capture")
        .output()
        .unwrap();
    assert_eq!(child.status.code(), Some(3));
    publish_bound(&slot, b"new artifact\n", &[]).unwrap();
    let sidecar = slot
        .capture_related(&root.path("diagram.delivery.json"), &[])
        .unwrap();
    publish_bound(&sidecar, b"receipt", &[]).unwrap();
    let summary = slot
        .capture_related(&root.path("diagram.finalize-summary.json"), &[])
        .unwrap();
    publish_bound(&summary, b"summary", &[]).unwrap();
    assert!(slot.capture_related(&output, &[]).is_err());
    assert_eq!(fs::read(sidecar.path()).unwrap(), b"receipt");
    assert_eq!(fs::read(summary.path()).unwrap(), b"summary");
}

#[test]
fn output_later_claimant_wins_after_old_retirement() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::BeforePublishLink {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&paths.target)?
                .write_all(b"later claimant")?;
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.exit_code(), 3);
    assert_eq!(fs::read(&output).unwrap(), b"later claimant");
    let recovery = error.recovery_directory.unwrap();
    assert_eq!(
        fs::read(recovery.join("previous")).unwrap(),
        b"old artifact\n"
    );
    drop(slot);
    for _ in 0..2 {
        let result = recover_publication(&recovery);
        assert_eq!(result.exit_code(), 3);
        assert_eq!(fs::read(&output).unwrap(), b"later claimant");
    }
}

#[test]
fn output_public_replacement_before_retirement_preserves_the_successor() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::BeforeRetire {
            fs::remove_file(&paths.target)?;
            fs::write(&paths.target, b"successor")?;
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.exit_code(), 3);
    assert_eq!(fs::read(output).unwrap(), b"successor");
    assert!(!error.committed);
}

#[test]
fn output_unknown_symlink_and_hardlink_claimants_are_never_removed() {
    for hardlink in [false, true] {
        let root = Workspace::new();
        let output = old_output(&root);
        let claimant = root.path("claimant");
        write(&claimant, b"claimant bytes");
        let slot = capture_slot(&output, &[]).unwrap();
        let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
            if phase == Phase::BeforePublishLink {
                if hardlink {
                    fs::hard_link(&claimant, &paths.target)?;
                } else {
                    symlink(&claimant, &paths.target)?;
                }
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.exit_code(), 3);
        assert_eq!(fs::read(&output).unwrap(), b"claimant bytes");
        let recovery = error.recovery_directory.unwrap();
        drop(slot);
        assert_eq!(recover_publication(&recovery).exit_code(), 3);
        assert_eq!(fs::read(&claimant).unwrap(), b"claimant bytes");
        assert_eq!(
            fs::symlink_metadata(&output)
                .unwrap()
                .file_type()
                .is_symlink(),
            !hardlink
        );
    }
}

#[test]
fn output_parent_replacement_never_writes_into_the_replacement_tree() {
    let root = Workspace::new();
    let actual = root.path("actual");
    fs::create_dir(&actual).unwrap();
    let output = actual.join("diagram.html");
    write(&output, b"old");
    let displaced = root.path("displaced");
    let slot = capture_slot(&output, &[]).unwrap();
    let result = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::BeforePublishLink {
            fs::rename(&actual, &displaced)?;
            fs::create_dir(&actual)?;
            fs::write(&paths.target, b"foreign tree")?;
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&output).unwrap(), b"foreign tree");
    assert_eq!(fs::read(displaced.join("diagram.html")).unwrap(), b"old");
}

#[test]
fn output_stage_successor_and_private_no_clobber_collisions_are_preserved() {
    for after_link in [false, true] {
        let root = Workspace::new();
        let output = old_output(&root);
        let slot = capture_slot(&output, &[]).unwrap();
        let mut retained = None;
        let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
            if phase
                == if after_link {
                    Phase::AfterPublishLink
                } else {
                    Phase::BeforePublishLink
                }
            {
                fs::remove_file(&paths.stage)?;
                fs::write(&paths.stage, b"private successor")?;
                retained = Some(paths.stage.clone());
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.exit_code(), 3);
        assert_eq!(fs::read(retained.unwrap()).unwrap(), b"private successor");
        assert_eq!(fs::read(output).unwrap(), b"old artifact\n");
    }
}

#[test]
fn output_killed_publisher_has_private_durable_locator_and_recovers_idempotently() {
    let (root, output, recovery) = interrupted();
    assert_eq!(fs::metadata(&recovery).unwrap().mode() & 0o777, 0o700);
    let record = recovery.join("publication-recovery-v1.json");
    assert_eq!(fs::metadata(&record).unwrap().mode() & 0o777, 0o600);
    assert_eq!(fs::metadata(&record).unwrap().nlink(), 1);
    let record: serde_json::Value = serde_json::from_slice(&fs::read(record).unwrap()).unwrap();
    assert_eq!(record["schemaVersion"], 1);
    assert_eq!(record["tool"], "phoenix-diagram");
    assert_eq!(record["kind"], "output-publication");
    assert_eq!(record["target"]["requestedPath"], output.to_str().unwrap());
    let real_parent = fs::canonicalize(output.parent().unwrap()).unwrap();
    assert_eq!(
        record["target"]["parentPath"],
        real_parent.to_str().unwrap()
    );
    assert_eq!(
        record["target"]["commitPath"],
        real_parent
            .join(output.file_name().unwrap())
            .to_str()
            .unwrap()
    );
    let result = recover_publication(&recovery);
    assert_eq!(result.status, RecoveryStatus::Recovered, "{result:?}");
    assert_eq!(
        result.target,
        Some(real_parent.join(output.file_name().unwrap()))
    );
    assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
    assert_eq!(fs::metadata(&output).unwrap().mode() & 0o777, 0o640);
    assert!(!recovery.exists());
    let repeated = recover_publication(&recovery);
    assert_eq!(repeated.status, RecoveryStatus::Absent);
    assert!(repeated.target.is_none());
    drop(root);
}

#[test]
fn output_recovery_can_be_killed_after_link_backup_cleanup_and_witness_moves() {
    for phase in [
        Phase::AfterRecoveryLink,
        Phase::AfterRecoveryBackupCleanup,
        Phase::AfterRecordCleanup,
        Phase::AfterDirectoryCleanup,
        Phase::AfterWitnessQuarantine,
    ] {
        let (root, output, recovery) = interrupted();
        crash(&root, "recover", phase);
        assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
        let result = recover_publication(&recovery);
        assert_eq!(result.exit_code(), 0, "{phase:?}: {result:?}");
        assert_eq!(fs::metadata(&output).unwrap().nlink(), 1);
        assert_eq!(
            recover_publication(&recovery).status,
            RecoveryStatus::Absent
        );
    }
}

#[test]
fn output_killed_after_new_publication_keeps_the_new_public_claimant() {
    let root = Workspace::new();
    let output = old_output(&root);
    crash(&root, "publish", Phase::AfterPublishLink);
    let recovery = recovery_in(&root);
    let result = recover_publication(&recovery);
    assert_eq!(result.exit_code(), 3);
    assert_eq!(fs::read(output).unwrap(), b"new artifact\n");
    assert_eq!(
        fs::read(recovery.join("previous")).unwrap(),
        b"old artifact\n"
    );
}

#[test]
fn output_tampered_record_backup_permissions_and_extra_hardlinks_are_preserved() {
    for case in 0..4 {
        let (_root, output, recovery) = interrupted();
        let record = recovery.join("publication-recovery-v1.json");
        let backup = recovery.join("previous");
        match case {
            0 => write(&record, b"{\"kind\":\"install\"}"),
            1 => write(&backup, b"tampered"),
            2 => fs::set_permissions(&backup, fs::Permissions::from_mode(0o600)).unwrap(),
            _ => fs::hard_link(&record, recovery.join("record-alias")).unwrap(),
        };
        let before = fs::read(&backup).unwrap();
        let result = recover_publication(&recovery);
        assert_eq!(result.exit_code(), 3, "{result:?}");
        assert!(!output.exists());
        assert_eq!(fs::read(backup).unwrap(), before);
        assert!(record.exists());
    }
}

#[test]
fn output_unknown_private_entries_survive_and_prevent_a_successful_cleanup() {
    let (_root, output, recovery) = interrupted();
    let extra = recovery.join("user-kept");
    write(&extra, b"keep me");
    let result = recover_publication(&recovery);
    assert_eq!(result.exit_code(), 3, "{result:?}");
    assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
    assert_eq!(fs::read(extra).unwrap(), b"keep me");
    assert!(recovery.exists());
}

#[test]
fn output_replaced_recovery_parent_and_copied_records_cannot_claim_ownership() {
    let (root, output, recovery) = interrupted();
    let foreign = root.path("foreign");
    fs::create_dir(&foreign).unwrap();
    let moved = foreign.join(recovery.file_name().unwrap());
    fs::rename(&recovery, &moved).unwrap();
    let result = recover_publication(&moved);
    assert_eq!(result.exit_code(), 3);
    assert!(result.target.is_none());
    assert!(!output.exists());
    assert_eq!(fs::read(moved.join("previous")).unwrap(), b"old artifact\n");
}

#[test]
fn output_unknown_cleanup_witness_is_preserved_without_claiming_cleanup_success() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let mut witness = None;
    let result = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::BeforeRecordCleanup {
            let path = paths.directory.with_extension("cleanup.json");
            fs::write(&path, b"foreign witness")?;
            witness = Some(path);
        }
        Ok(())
    })
    .unwrap();
    assert!(!result.cleanup_complete);
    assert_eq!(result.exit_code(), 3);
    assert_eq!(fs::read(witness.unwrap()).unwrap(), b"foreign witness");
    assert_eq!(fs::read(&output).unwrap(), b"new");
}

#[test]
fn output_recovery_link_race_preserves_the_later_claimant() {
    let (_root, output, recovery) = interrupted();
    let result = test_support::recover(&recovery, &mut |phase, paths| {
        if phase == Phase::BeforeRecoveryLink {
            fs::write(&paths.target, b"new claimant during recovery")?;
        }
        Ok(())
    });
    assert_eq!(result.exit_code(), 3);
    assert_eq!(fs::read(output).unwrap(), b"new claimant during recovery");
    assert_eq!(
        fs::read(recovery.join("previous")).unwrap(),
        b"old artifact\n"
    );
}

#[test]
fn output_new_slot_interruption_recovers_only_the_original_absent_state() {
    let root = Workspace::new();
    let output = root.path("diagram.html");
    crash(&root, "publish", Phase::AfterRetire);
    let recovery = recovery_in(&root);
    let result = recover_publication(&recovery);
    assert_eq!(result.exit_code(), 0, "{result:?}");
    assert!(!output.exists());
    assert!(!recovery.exists());
}

#[test]
fn output_pre_retirement_failure_keeps_original_bytes() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, _| {
        if phase == Phase::RecoveryDurable {
            return Err(observer_error());
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
    assert_eq!(fs::metadata(&output).unwrap().nlink(), 1);
    assert!(error.recovery_directory.is_none(), "{error:?}");
    assert!(!error.committed);
}

#[test]
fn output_stage_sync_failure_cleans_only_the_owned_stage() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, _| {
        if phase == Phase::StageSynced {
            return Err(observer_error());
        }
        Ok(())
    })
    .unwrap_err();
    assert!(!error.committed);
    assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
    assert!(fs::read_dir(&root.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".phoenix-diagram-recovery-")
    }));
}

#[test]
fn output_absent_recovery_does_not_create_a_missing_parent() {
    let root = Workspace::new();
    let path = root
        .path("missing-parent")
        .join(format!(".phoenix-diagram-recovery-{}", "b".repeat(32)));
    let result = recover_publication(&path);
    assert_eq!(result.status, RecoveryStatus::Absent, "{result:?}");
    assert!(result.target.is_none());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn output_large_and_symlink_recovery_records_are_rejected_before_reading_content() {
    for symlink_record in [false, true] {
        let (root, output, recovery) = interrupted();
        let record = recovery.join("publication-recovery-v1.json");
        fs::remove_file(&record).unwrap();
        if symlink_record {
            let user = root.path("user-data");
            write(&user, b"user data");
            symlink(&user, &record).unwrap();
        } else {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&record)
                .unwrap();
            file.set_len(1024 * 1024 * 1024).unwrap();
            fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let result = recover_publication(&recovery);
        assert_eq!(result.exit_code(), 3, "{result:?}");
        assert!(!output.exists());
        assert!(record.symlink_metadata().is_ok());
    }
}

#[test]
fn output_recovery_parent_swap_during_link_does_not_create_a_foreign_public_name() {
    let root = Workspace::new();
    let actual = root.path("actual");
    fs::create_dir(&actual).unwrap();
    let output = actual.join("diagram.html");
    write(&output, b"old artifact\n");
    crash_at(&actual, "publish", Phase::AfterRetire);
    let recovery = fs::read_dir(&actual)
        .unwrap()
        .map(|e| e.unwrap())
        .find(|e| {
            e.file_type().unwrap().is_dir()
                && e.file_name()
                    .to_string_lossy()
                    .starts_with(".phoenix-diagram-recovery-")
        })
        .unwrap()
        .path();
    let displaced = root.path("displaced");
    let result = test_support::recover(&recovery, &mut |phase, _| {
        if phase == Phase::BeforeRecoveryLink {
            fs::rename(&actual, &displaced)?;
            fs::create_dir(&actual)?;
            let foreign = actual.join(recovery.file_name().unwrap());
            fs::DirBuilder::new().mode(0o700).create(&foreign)?;
            fs::write(foreign.join("previous"), b"foreign backup")?;
        }
        Ok(())
    });
    assert_eq!(result.exit_code(), 3, "{result:?}");
    assert!(!output.exists());
    assert_eq!(
        fs::read(actual.join(recovery.file_name().unwrap()).join("previous")).unwrap(),
        b"foreign backup"
    );
    assert_eq!(
        fs::read(
            displaced
                .join(recovery.file_name().unwrap())
                .join("previous")
        )
        .unwrap(),
        b"old artifact\n"
    );
}

#[test]
fn output_backup_tampering_during_a_failed_publish_cannot_be_restored_as_original() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::AfterRetire {
            fs::write(&paths.backup, b"tampered backup")?;
            return Err(observer_error());
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.exit_code(), 3);
    assert!(!output.exists());
    let recovery = error.recovery_directory.unwrap();
    assert_eq!(
        fs::read(recovery.join("previous")).unwrap(),
        b"tampered backup"
    );
    drop(slot);
    assert_eq!(recover_publication(&recovery).exit_code(), 3);
    assert!(!output.exists());
}

#[test]
fn output_new_stage_binding_uses_the_created_fd_even_for_a_same_byte_successor() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let mut stage = None;
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::StageCreated {
            fs::remove_file(&paths.stage)?;
            fs::write(&paths.stage, b"new")?;
            stage = Some(paths.stage.clone());
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.exit_code(), 3);
    assert_eq!(fs::read(stage.unwrap()).unwrap(), b"new");
    assert_eq!(fs::read(output).unwrap(), b"old artifact\n");
}

#[test]
fn output_recovery_journal_binding_is_captured_from_its_original_write_fd() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let mut record = None;
    let error = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::BeforeRecordWrite {
            let path = paths.directory.join("publication-recovery-v1.json");
            fs::remove_file(&path)?;
            fs::write(&path, b"foreign journal")?;
            record = Some(path);
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.exit_code(), 3);
    assert_eq!(fs::read(record.unwrap()).unwrap(), b"foreign journal");
    assert_eq!(fs::read(output).unwrap(), b"old artifact\n");
}

#[test]
fn output_pending_publications_are_empty_only_after_clean_commit() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    assert!(slot.pending_publications().unwrap().is_empty());
    publish_bound(&slot, b"new", &[]).unwrap();
    assert!(slot.pending_publications().unwrap().is_empty());
}

#[test]
fn output_pending_publications_bind_primary_and_related_slots_independently() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let related = slot
        .capture_related(&root.path("diagram.delivery.json"), &[])
        .unwrap();
    let result = test_support::publish(&related, b"receipt", &[], &mut |phase, _| {
        if phase == Phase::BeforeBackupCleanup {
            Err(observer_error())
        } else {
            Ok(())
        }
    })
    .unwrap();
    assert!(!result.cleanup_complete);
    let other = capture_slot(&root.path("other.html"), &[]).unwrap();
    let result = test_support::publish(&other, b"other", &[], &mut |phase, _| {
        if phase == Phase::BeforeBackupCleanup {
            Err(observer_error())
        } else {
            Ok(())
        }
    })
    .unwrap();
    assert!(!result.cleanup_complete);
    assert!(slot.pending_publications().unwrap().is_empty());
    let pending = related.pending_publications().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].target, related.path());
    assert_eq!(pending[0].phase, "prepared");
}

#[test]
fn output_pending_cleanup_witness_is_visible_even_when_the_directory_is_absent() {
    let root = Workspace::new();
    let output = old_output(&root);
    crash(&root, "publish", Phase::AfterDirectoryCleanup);
    let recovery = recovery_in(&root);
    assert!(!recovery.exists());
    let slot = capture_slot(&output, &[]).unwrap();
    let pending = slot.pending_publications().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].phase, "cleanup");
    assert!(pending[0].witness.as_ref().unwrap().exists());
    assert_eq!(
        pending[0].recovery_directory.file_name(),
        recovery.file_name()
    );
}

#[test]
fn output_pending_query_fails_closed_on_an_unknown_namespace_entry() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let unknown = root.path(&format!(".phoenix-diagram-recovery-{}", "a".repeat(32)));
    write(&unknown, b"foreign object");
    let error = slot.pending_publications().unwrap_err();
    assert_eq!(error.code, "output/unknown-recovery-namespace");
    assert_eq!(error.exit_code(), 3);
    assert_eq!(fs::read(unknown).unwrap(), b"foreign object");
}

#[test]
fn output_late_public_claimant_before_record_cleanup_is_not_reported_as_current() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let result = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::BeforeRecordCleanup {
            fs::remove_file(&paths.target)?;
            fs::write(&paths.target, b"late final claimant")?;
        }
        Ok(())
    });
    match result {
        Ok(publication) => assert_ne!(
            publication.exit_code(),
            0,
            "A visible later claimant cannot be current for this publication"
        ),
        Err(error) => {
            assert!(error.committed);
            assert_eq!(error.exit_code(), 3);
        }
    }
    assert_eq!(fs::read(output).unwrap(), b"late final claimant");
}

#[test]
fn output_original_snapshot_guard_detects_a_later_writer_and_is_stale_after_publish() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    slot.verify_snapshot().unwrap();
    write(&output, b"changed original inode");
    assert!(slot.verify_snapshot().is_err());
    drop(slot);
    let slot = capture_slot(&output, &[]).unwrap();
    publish_bound(&slot, b"new", &[]).unwrap();
    assert!(slot.verify_snapshot().is_err());
    slot.verify_guard().unwrap();
    FileBinding::capture(&output).unwrap().verify().unwrap();
}

#[test]
fn output_explicit_mode_is_applied_to_the_candidate_before_any_public_link() {
    for mode in [0o600, 0o644, 0o755] {
        for existing in [false, true] {
            let root = Workspace::new();
            let output = if existing {
                old_output(&root)
            } else {
                root.path("diagram.html")
            };
            let slot = capture_slot(&output, &[]).unwrap();
            let mut observed = false;
            let publication = test_support::publish_with_mode(
                &slot,
                b"mode-bound bytes",
                mode,
                &[],
                &mut |phase, paths| {
                    if phase == Phase::StageCreated {
                        observed = true;
                        assert_eq!(fs::metadata(&paths.stage)?.mode() & 0o7777, mode);
                        if existing {
                            assert_eq!(fs::read(&paths.target)?, b"old artifact\n");
                        } else {
                            assert!(!paths.target.exists());
                        }
                    }
                    Ok(())
                },
            )
            .unwrap();
            assert!(observed);
            assert_eq!(publication.exit_code(), 0);
            assert_eq!(publication.identity.mode, mode);
            assert_eq!(fs::metadata(&output).unwrap().mode() & 0o7777, mode);
            assert_eq!(fs::read(&output).unwrap(), b"mode-bound bytes");
        }
    }
}

#[test]
fn output_explicit_mode_rejects_special_permissions_before_creating_a_transaction() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let error = publish_bound_with_mode(&slot, b"new", 0o4644, &[]).unwrap_err();
    assert_eq!(error.exit_code(), 2);
    assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
    assert_eq!(fs::metadata(output).unwrap().nlink(), 1);
    assert!(slot.pending_publications().unwrap().is_empty());
}

#[test]
fn output_pending_query_validates_other_target_records_before_ignoring_them() {
    for tamper in [
        "copy",
        "hardlink",
        "mode",
        "checksum",
        "symlink",
        "directory",
    ] {
        let root = Workspace::new();
        let output = old_output(&root);
        let slot = capture_slot(&output, &[]).unwrap();
        let related = slot
            .capture_related(&root.path("diagram.delivery.json"), &[])
            .unwrap();
        let publication = test_support::publish(&related, b"receipt", &[], &mut |phase, _| {
            if phase == Phase::BeforeBackupCleanup {
                Err(observer_error())
            } else {
                Ok(())
            }
        })
        .unwrap();
        let recovery = publication.recovery_directory.unwrap();
        let record = recovery.join("publication-recovery-v1.json");
        assert!(slot.pending_publications().unwrap().is_empty());
        let _record_guard = FileBinding::capture(&record).unwrap();
        match tamper {
            "copy" => {
                let bytes = fs::read(&record).unwrap();
                fs::remove_file(&record).unwrap();
                write(&record, &bytes);
                fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "hardlink" => fs::hard_link(&record, root.path("foreign-link")).unwrap(),
            "mode" => fs::set_permissions(&record, fs::Permissions::from_mode(0o644)).unwrap(),
            "checksum" => {
                let mut value: serde_json::Value =
                    serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
                value["candidate"]["sha256"] = serde_json::json!("0".repeat(64));
                write(&record, &serde_json::to_vec(&value).unwrap());
            }
            "symlink" => {
                let foreign = root.path("foreign-record");
                fs::rename(&record, &foreign).unwrap();
                symlink(foreign, &record).unwrap();
            }
            "directory" => {
                let holding = root.path("holding");
                fs::rename(&recovery, &holding).unwrap();
                fs::DirBuilder::new().mode(0o700).create(&recovery).unwrap();
                fs::rename(holding.join("publication-recovery-v1.json"), &record).unwrap();
            }
            _ => unreachable!(),
        }
        let error = slot.pending_publications().unwrap_err();
        assert_eq!(error.exit_code(), 3, "{tamper}: {error:?}");
        assert!(record.symlink_metadata().is_ok());
        assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
    }
}

#[test]
fn output_recovery_does_not_report_a_late_claimant_as_the_recovered_original() {
    for checkpoint in [Phase::BeforeRecordCleanup, Phase::AfterDirectoryCleanup] {
        let (_root, output, recovery) = interrupted();
        let result = test_support::recover(&recovery, &mut |phase, paths| {
            if phase == checkpoint {
                fs::remove_file(&paths.target)?;
                fs::write(&paths.target, b"late recovery claimant")?;
            }
            Ok(())
        });
        assert_eq!(result.exit_code(), 3, "{checkpoint:?}: {result:?}");
        assert_eq!(result.target, Some(fs::canonicalize(&output).unwrap()));
        assert_eq!(fs::read(output).unwrap(), b"late recovery claimant");
    }
}

#[test]
fn output_cleanup_witness_successors_are_preserved_at_the_final_delete_boundary() {
    for kind in ["file", "symlink", "hardlink"] {
        let root = Workspace::new();
        let output = old_output(&root);
        let slot = capture_slot(&output, &[]).unwrap();
        let user_file = root.path("user-witness-target");
        write(&user_file, b"user witness claimant");
        let mut witness = None;
        let result = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
            if phase == Phase::BeforeWitnessDelete {
                let path = paths.directory.with_extension("cleanup.json");
                fs::remove_file(&path)?;
                match kind {
                    "file" => fs::write(&path, b"user witness claimant")?,
                    "symlink" => symlink(&user_file, &path)?,
                    "hardlink" => fs::hard_link(&user_file, &path)?,
                    _ => unreachable!(),
                }
                witness = Some(path);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(result.exit_code(), 3, "{kind}: {result:?}");
        assert_eq!(fs::read(&output).unwrap(), b"new");
        let witness = witness.unwrap();
        assert!(witness.symlink_metadata().is_ok(), "{kind}");
        assert_eq!(fs::read(witness).unwrap(), b"user witness claimant");
        assert_eq!(fs::read(user_file).unwrap(), b"user witness claimant");
    }
}

#[test]
fn output_quarantine_survives_kill_and_explicit_recovery_ignores_other_transactions() {
    let root = Workspace::new();
    let output = old_output(&root);
    crash(&root, "publish", Phase::AfterWitnessQuarantine);
    let recovery = recovery_in(&root);
    assert!(!recovery.exists());
    let cleanup = root.path(".phoenix-diagram-output-cleanup");
    assert_eq!(fs::metadata(&cleanup).unwrap().mode() & 0o7777, 0o700);
    let marker = cleanup.join("owner-v1.json");
    assert_eq!(fs::metadata(&marker).unwrap().mode() & 0o7777, 0o600);
    assert_eq!(fs::metadata(&marker).unwrap().nlink(), 1);
    let slot = capture_slot(&output, &[]).unwrap();
    let pending = slot.pending_publications().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].phase, "cleanup");
    let record = pending[0].witness.clone().unwrap();
    assert_eq!(
        record.parent().unwrap(),
        fs::canonicalize(&cleanup).unwrap()
    );
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(
        value["recordIdentity"]["inode"],
        fs::metadata(&record).unwrap().ino().to_string()
    );
    assert_eq!(
        record.file_name().unwrap().to_str().unwrap(),
        format!("{}.json", value["transactionId"].as_str().unwrap())
    );
    drop(slot);

    let other = capture_slot(&root.path("other.html"), &[]).unwrap();
    let publication = test_support::publish(&other, b"other", &[], &mut |phase, _| {
        if phase == Phase::AfterWitnessQuarantine {
            Err(observer_error())
        } else {
            Ok(())
        }
    })
    .unwrap();
    assert!(!publication.cleanup_complete);
    let other_pending = other.pending_publications().unwrap();
    assert_eq!(other_pending.len(), 1);
    let other_record = other_pending[0].witness.clone().unwrap();
    let other_inode = fs::metadata(&other_record).unwrap().ino();
    let result = recover_publication(&recovery);
    assert_eq!(result.status, RecoveryStatus::Preserved, "{result:?}");
    assert_eq!(result.code, "output/committed-output-preserved");
    assert_eq!(fs::read(&output).unwrap(), b"new artifact\n");
    assert!(!record.exists());
    assert_eq!(fs::metadata(&other_record).unwrap().ino(), other_inode);
    assert_eq!(
        recover_publication(&recovery).status,
        RecoveryStatus::Absent
    );
    let other_recovery = publication.recovery_directory.unwrap();
    drop(other);
    assert_eq!(recover_publication(&other_recovery).exit_code(), 3);
    assert!(!other_record.exists());
    let slot = capture_slot(&output, &[]).unwrap();
    assert!(slot.pending_publications().unwrap().is_empty());
    assert_eq!(fs::read_dir(cleanup).unwrap().count(), 1);
}

#[test]
fn output_unknown_cleanup_directories_are_never_adopted_from_empty_or_private_modes() {
    for kind in ["empty", "file", "symlink", "foreign-marker"] {
        let root = Workspace::new();
        let output = old_output(&root);
        let slot = capture_slot(&output, &[]).unwrap();
        let cleanup = root.path(".phoenix-diagram-output-cleanup");
        match kind {
            "file" => write(&cleanup, b"unknown directory claimant"),
            "symlink" => {
                let foreign = root.path("foreign");
                fs::DirBuilder::new().mode(0o700).create(&foreign).unwrap();
                write(&foreign.join("user-data"), b"user data");
                symlink(foreign, &cleanup).unwrap();
            }
            _ => {
                fs::DirBuilder::new().mode(0o700).create(&cleanup).unwrap();
                if kind == "foreign-marker" {
                    write(&cleanup.join("owner-v1.json"), b"unknown marker");
                    fs::set_permissions(
                        cleanup.join("owner-v1.json"),
                        fs::Permissions::from_mode(0o600),
                    )
                    .unwrap();
                }
            }
        }
        let error = publish_bound(&slot, b"new", &[]).unwrap_err();
        assert_eq!(error.exit_code(), 3, "{kind}: {error:?}");
        assert_eq!(slot.pending_publications().unwrap_err().exit_code(), 3);
        assert_eq!(fs::read(&output).unwrap(), b"old artifact\n");
        assert!(cleanup.symlink_metadata().is_ok());
    }
}

#[test]
fn output_cleanup_marker_and_directory_bindings_and_unknown_entries_fail_closed() {
    for tamper in ["mode", "hardlink", "copy", "checksum", "directory", "extra"] {
        let root = Workspace::new();
        let output = old_output(&root);
        let slot = capture_slot(&output, &[]).unwrap();
        publish_bound(&slot, b"new", &[]).unwrap();
        let cleanup = root.path(".phoenix-diagram-output-cleanup");
        let marker = cleanup.join("owner-v1.json");
        let _marker_guard = FileBinding::capture(&marker).unwrap();
        match tamper {
            "mode" => fs::set_permissions(&cleanup, fs::Permissions::from_mode(0o755)).unwrap(),
            "hardlink" => fs::hard_link(&marker, root.path("foreign-marker-link")).unwrap(),
            "copy" => {
                let bytes = fs::read(&marker).unwrap();
                fs::remove_file(&marker).unwrap();
                write(&marker, &bytes);
                fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "checksum" => {
                let mut value: serde_json::Value =
                    serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
                value["checksum"] = serde_json::json!("0".repeat(64));
                write(&marker, &serde_json::to_vec(&value).unwrap());
            }
            "directory" => {
                let holding = root.path("holding");
                fs::rename(&cleanup, &holding).unwrap();
                fs::DirBuilder::new().mode(0o700).create(&cleanup).unwrap();
                fs::rename(holding.join("owner-v1.json"), &marker).unwrap();
            }
            "extra" => write(&cleanup.join("user-notes.txt"), b"user notes"),
            _ => unreachable!(),
        }
        let error = slot.pending_publications().unwrap_err();
        assert_eq!(error.exit_code(), 3, "{tamper}: {error:?}");
        assert!(marker.symlink_metadata().is_ok());
        assert_eq!(fs::read(&output).unwrap(), b"new");
        if tamper == "extra" {
            assert_eq!(
                fs::read(cleanup.join("user-notes.txt")).unwrap(),
                b"user notes"
            );
        }
    }
}

#[test]
fn output_quarantined_records_still_require_the_original_journal_inode_and_single_link() {
    for tamper in ["copy", "hardlink", "mode"] {
        let root = Workspace::new();
        let output = old_output(&root);
        crash(&root, "publish", Phase::AfterWitnessQuarantine);
        let recovery = recovery_in(&root);
        let slot = capture_slot(&output, &[]).unwrap();
        let record = slot.pending_publications().unwrap()[0]
            .witness
            .clone()
            .unwrap();
        let _record_guard = FileBinding::capture(&record).unwrap();
        drop(slot);
        match tamper {
            "copy" => {
                let bytes = fs::read(&record).unwrap();
                fs::remove_file(&record).unwrap();
                write(&record, &bytes);
                fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
            }
            "hardlink" => fs::hard_link(&record, root.path("foreign-link")).unwrap(),
            "mode" => fs::set_permissions(&record, fs::Permissions::from_mode(0o644)).unwrap(),
            _ => unreachable!(),
        }
        let result = recover_publication(&recovery);
        assert_eq!(result.exit_code(), 3, "{tamper}: {result:?}");
        assert!(result.target.is_none());
        assert!(record.symlink_metadata().is_ok());
        assert_eq!(fs::read(output).unwrap(), b"new artifact\n");
    }
}

#[test]
fn output_later_public_witness_claimant_cannot_be_removed_after_quarantine() {
    let root = Workspace::new();
    let output = old_output(&root);
    let slot = capture_slot(&output, &[]).unwrap();
    let mut witness = None;
    let result = test_support::publish(&slot, b"new", &[], &mut |phase, paths| {
        if phase == Phase::AfterWitnessQuarantine {
            let path = paths.directory.with_extension("cleanup.json");
            fs::write(&path, b"later witness claimant")?;
            witness = Some(path);
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(result.exit_code(), 3);
    assert_eq!(
        fs::read(witness.unwrap()).unwrap(),
        b"later witness claimant"
    );
    assert_eq!(fs::read(&output).unwrap(), b"new");
    assert_eq!(slot.pending_publications().unwrap_err().exit_code(), 3);
    let recovery = result.recovery_directory.unwrap();
    drop(slot);
    assert_eq!(recover_publication(&recovery).exit_code(), 3);
}

type FilesystemSnapshotRow = (PathBuf, u32, u64, u64, u64, i64, i64, Vec<u8>);

fn filesystem_snapshot(root: &Path) -> Vec<FilesystemSnapshotRow> {
    fn visit(root: &Path, relative: &Path, rows: &mut Vec<FilesystemSnapshotRow>) {
        let path = root.join(relative);
        let metadata = path.symlink_metadata().unwrap();
        let content = if metadata.is_file() {
            fs::read(&path).unwrap()
        } else if metadata.file_type().is_symlink() {
            use std::os::unix::ffi::OsStrExt;
            fs::read_link(&path)
                .unwrap()
                .as_os_str()
                .as_bytes()
                .to_vec()
        } else {
            Vec::new()
        };
        rows.push((
            relative.to_owned(),
            metadata.mode(),
            metadata.dev(),
            metadata.ino(),
            metadata.nlink(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            content,
        ));
        if metadata.is_dir() {
            let mut names: Vec<_> = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            names.sort();
            for name in names {
                visit(root, &relative.join(name), rows);
            }
        }
    }
    let mut rows = Vec::new();
    visit(root, Path::new(""), &mut rows);
    rows
}

#[test]
fn output_existing_lock_query_reads_interrupted_hardlinks_and_quarantine_without_writes() {
    for checkpoint in [Phase::RecoveryDurable, Phase::AfterWitnessQuarantine] {
        let root = Workspace::new();
        let output = old_output(&root);
        crash(&root, "publish", checkpoint);
        let before = filesystem_snapshot(&root.0);
        if checkpoint == Phase::RecoveryDurable {
            assert_eq!(fs::metadata(&output).unwrap().nlink(), 2);
        }
        let pending = pending_existing_publications(&output).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].target, fs::canonicalize(&output).unwrap());
        assert_eq!(
            pending[0].phase,
            if checkpoint == Phase::RecoveryDurable {
                "prepared"
            } else {
                "cleanup"
            }
        );
        assert_eq!(filesystem_snapshot(&root.0), before);
    }
}

#[test]
fn output_existing_lock_query_rejects_missing_busy_or_untrusted_bindings_without_writes() {
    for kind in [
        "clean", "missing", "busy", "mode", "symlink", "hardlink", "bytes", "owner", "parent",
    ] {
        let root = Workspace::new();
        let mut output = old_output(&root);
        let mut held = if kind == "missing" {
            None
        } else {
            Some(capture_slot(&output, &[]).unwrap())
        };
        let lock = fs::read_dir(&root.0)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".phoenix-diagram-lock-")
            });
        if kind != "busy" {
            drop(held.take());
        }
        match kind {
            "mode" => {
                fs::set_permissions(lock.as_ref().unwrap(), fs::Permissions::from_mode(0o644))
                    .unwrap()
            }
            "symlink" => {
                let lock = lock.as_ref().unwrap();
                let foreign = root.path("foreign-lock");
                fs::rename(lock, &foreign).unwrap();
                symlink(foreign, lock).unwrap();
            }
            "hardlink" => fs::hard_link(lock.as_ref().unwrap(), root.path("foreign-lock")).unwrap(),
            "bytes" => {
                let lock = lock.as_ref().unwrap();
                let mut bytes = fs::read(lock).unwrap();
                bytes[0] = b'!';
                write(lock, &bytes);
            }
            "owner" => {
                let cleanup = root.path(".phoenix-diagram-output-cleanup");
                fs::DirBuilder::new().mode(0o700).create(cleanup).unwrap();
            }
            "parent" => {
                let replacement = root.path("replacement");
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&replacement)
                    .unwrap();
                let lock = lock.as_ref().unwrap();
                fs::copy(lock, replacement.join(lock.file_name().unwrap())).unwrap();
                output = replacement.join("diagram.html");
                write(&output, b"other parent artifact");
            }
            _ => {}
        }
        let before = filesystem_snapshot(&root.0);
        if kind == "busy" {
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "output_child_process"])
                .env("PHOENIX_OUTPUT_TEST_ROOT", &root.0)
                .env("PHOENIX_OUTPUT_TEST_MODE", "pending")
                .output()
                .unwrap();
            assert_eq!(child.status.code(), Some(3));
        } else {
            let result = pending_existing_publications(&output);
            if kind == "clean" {
                assert!(result.unwrap().is_empty());
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.exit_code(), 3, "{kind}: {error:?}");
                if kind == "missing" {
                    assert_eq!(error.code, "output/existing-lock-absent");
                }
            }
        }
        assert_eq!(filesystem_snapshot(&root.0), before, "{kind}");
    }
}
