use phoenix_ui_install::{
    sha256, Change, Checkpoint, ErrorKind, ExpectedFile, JsonPatch, JsonSlot, JsonStep, Manifest,
    Payload, Permissions, Plan, Project, RollbackPolicy,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-receipt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn project(&self) -> Project {
        Project::open(&self.0).unwrap()
    }
    fn plan(&self) -> Plan {
        Plan {
            transaction_id: "receipt-check".into(),
            manifest: Manifest {
                version: "fixture-a".into(),
                sha256: sha256(b"manifest"),
                providers: vec![".claude".into()],
                migration_sources: vec![],
            },
            expected_receipt: ExpectedFile::Absent,
            changes: vec![
                Change::Exclusive {
                    path: ".claude/skills/phoenix-ui/SKILL.md".into(),
                    expected: ExpectedFile::Absent,
                    after: Some(Payload::new(
                        b"Fixed Phoenix fixture".to_vec(),
                        Permissions::file(0o644),
                    )),
                    rollback: RollbackPolicy::Restore,
                },
                Change::Exclusive {
                    path: ".phoenix-ui/config.json".into(),
                    expected: ExpectedFile::Absent,
                    after: Some(Payload::new(b"{}".to_vec(), Permissions::file(0o600))),
                    rollback: RollbackPolicy::Retain,
                },
                Change::SharedJson {
                    path: ".claude/settings.json".into(),
                    expected: ExpectedFile::Absent,
                    patches: vec![JsonPatch {
                        parent: vec![JsonStep::Key("hooks".into()), JsonStep::Key("Stop".into())],
                        slot: JsonSlot::Element(0),
                        before: None,
                        after: Some(json!({"type":"command","command":"fixed fixture hook"})),
                    }],
                },
            ],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn empty_active_receipt_probe_is_read_only() {
    let f = Fixture::new();
    assert!(f.project().active_receipt_verified().unwrap().is_none());
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
}

#[test]
fn active_receipt_verifies_journal_and_owned_payloads_without_writes() {
    let f = Fixture::new();
    let receipt = f.project().apply(&f.plan()).unwrap();
    let before = fs::read(f.0.join(".phoenix-ui/install/active.json")).unwrap();
    let verified = f.project().active_receipt_verified().unwrap().unwrap();
    assert_eq!(verified.transaction_id, receipt.transaction_id);
    assert_eq!(verified.ownership_sha256, receipt.ownership_sha256);
    assert_eq!(
        fs::read(f.0.join(".phoenix-ui/install/active.json")).unwrap(),
        before
    );
    let installation = f.project().verified_installation().unwrap().unwrap();
    assert_eq!(installation.receipt_sha256, sha256(&before));
    assert_eq!(installation.json_patches[".claude/settings.json"].len(), 1);
    assert_eq!(
        installation.json_patches[".claude/settings.json"][0].after,
        Some(json!({"type":"command","command":"fixed fixture hook"}))
    );
}

#[test]
fn active_receipt_rejects_managed_skill_and_hook_drift() {
    let f = Fixture::new();
    f.project().apply(&f.plan()).unwrap();
    fs::write(f.0.join(".claude/skills/phoenix-ui/SKILL.md"), "Customized").unwrap();
    assert_eq!(
        f.project().active_receipt_verified().unwrap_err().kind,
        ErrorKind::Conflict
    );
    fs::write(
        f.0.join(".claude/skills/phoenix-ui/SKILL.md"),
        "Fixed Phoenix fixture",
    )
    .unwrap();
    let p = f.0.join(".claude/settings.json");
    let s = fs::read_to_string(&p).unwrap();
    fs::write(&p, s.replace("fixed fixture hook", "customized hook")).unwrap();
    assert_eq!(
        f.project().active_receipt_verified().unwrap_err().kind,
        ErrorKind::Conflict
    );
}

#[test]
fn active_receipt_preserves_unrelated_json_and_mutable_config() {
    let f = Fixture::new();
    f.project().apply(&f.plan()).unwrap();
    let p = f.0.join(".claude/settings.json");
    let s = fs::read_to_string(&p).unwrap();
    let s=s.replace("\"command\":\"fixed fixture hook\"}","\"command\":\"fixed fixture hook\"},{\"type\":\"command\",\"command\":\"unrelated user hook\"}");
    fs::write(
        &p,
        s.replacen(
            '{',
            "{\n // retained user comment\n \"userNumber\":1.230000000000000000000000,",
            1,
        ),
    )
    .unwrap();
    fs::write(
        f.0.join(".phoenix-ui/config.json"),
        "{\"userPreference\":true}",
    )
    .unwrap();
    let receipt = f.project().active_receipt_verified().unwrap().unwrap();
    assert_eq!(receipt.transaction_id, "receipt-check");
    assert!(fs::read_to_string(&p)
        .unwrap()
        .contains("1.230000000000000000000000"));
}

#[test]
fn active_receipt_rejects_missing_or_altered_journal_and_future_schema() {
    let f = Fixture::new();
    f.project().apply(&f.plan()).unwrap();
    let p = f.0.join(".phoenix-ui/install/journals/receipt-check.json");
    let old = fs::read(&p).unwrap();
    fs::remove_file(&p).unwrap();
    assert_eq!(
        f.project().active_receipt_verified().unwrap_err().kind,
        ErrorKind::Conflict
    );
    fs::write(&p, &old).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&old).unwrap();
    value["ownership_sha256"] = json!("0".repeat(64));
    fs::write(&p, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
        f.project().active_receipt_verified().unwrap_err().kind,
        ErrorKind::Conflict
    );
    fs::write(&p, old).unwrap();
    let active = f.0.join(".phoenix-ui/install/active.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&active).unwrap()).unwrap();
    value["schema_version"] = json!(99);
    fs::write(&active, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
        f.project().active_receipt_verified().unwrap_err().kind,
        ErrorKind::Conflict
    );
}

#[test]
fn interrupted_install_probe_requires_recovery_and_does_not_recover_itself() {
    let f = Fixture::new();
    let _ = std::panic::catch_unwind(|| {
        f.project().apply_with(&f.plan(), |point| {
            if point == Checkpoint::JournalReady {
                panic!("fixture interruption");
            }
        })
    });
    let journal = f.0.join(".phoenix-ui/install/journals/receipt-check.json");
    let old = fs::read(&journal).unwrap();
    assert_eq!(
        f.project().active_receipt_verified().unwrap_err().kind,
        ErrorKind::RecoveryRequired
    );
    assert_eq!(fs::read(journal).unwrap(), old);
}
