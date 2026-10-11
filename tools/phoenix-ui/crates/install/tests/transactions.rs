use phoenix_ui_install::{
    sha256, Change, Checkpoint, ErrorKind, ExpectedFile, Identity, JsonPatch, JsonSlot, JsonStep,
    Manifest, Payload, Permissions, Plan, Project, Recovery, RollbackPolicy,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const SETTINGS: &str = ".claude/settings.local.json";
const ACTIVE: &str = ".phoenix-ui/install/active.json";
const OLD: &str = ".claude/skills/impeccable/SKILL.md";
const NEW: &str = ".claude/skills/phoenix-ui/SKILL.md";
const SHARED: &str = "{\n // preserve this comment\n \"hooks\": {\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"old fixture launcher\",\"timeout\":30},{\"type\":\"command\",\"command\":\"guard --threshold 5\",\"unknown\":1.23000}],\"outerUnknown\":true}]},\n \"permissions\": {\"allow\":[\"Read\"]},\n \"unknownNumber\":1.230000000000000000000000,\n \"url\":\"http://local//path\"\n}\n";

struct Fixture(PathBuf);

impl Fixture {
    fn empty() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-install-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn old() -> Self {
        let f = Self::empty();
        f.write(SETTINGS, SHARED.as_bytes());
        f.write(OLD, b"supported old skill bytes\n");
        f.write("PRODUCT.md", b"User-owned product requirements\n");
        f
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn read(&self, relative: &str) -> Vec<u8> {
        fs::read(self.0.join(relative)).unwrap()
    }
    fn project(&self) -> Project {
        Project::open(&self.0).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn patch(before: Option<Value>, after: Option<Value>) -> JsonPatch {
    JsonPatch {
        parent: vec![
            JsonStep::Key("hooks".into()),
            JsonStep::Key("Stop".into()),
            JsonStep::Match(Identity {
                fields: [("matcher".into(), None)].into(),
                required_keys: vec!["hooks".into()],
            }),
            JsonStep::Key("hooks".into()),
        ],
        slot: JsonSlot::Element(0),
        before,
        after,
    }
}

fn plan(f: &Fixture, transaction: &str) -> Plan {
    Plan {
        transaction_id: transaction.into(),
        manifest: Manifest {
            version: "candidate-a".into(),
            sha256: sha256(b"fixed release manifest"),
            providers: vec![".claude".into()],
            migration_sources: vec!["0.3.0".into(), "0.6.0".into()],
        },
        expected_receipt: ExpectedFile::Absent,
        changes: vec![
            Change::Exclusive {
                path: OLD.into(),
                expected: ExpectedFile::Hash(sha256(&f.read(OLD))),
                after: None,
                rollback: RollbackPolicy::Restore,
            },
            Change::SharedJson {
                path: SETTINGS.into(),
                expected: ExpectedFile::Hash(sha256(&f.read(SETTINGS))),
                patches: vec![patch(
                    Some(json!({"type":"command","command":"old fixture launcher","timeout":30})),
                    Some(json!({"type":"command","command":"new fixture launcher","timeout":30})),
                )],
            },
            Change::Exclusive {
                path: NEW.into(),
                expected: ExpectedFile::Absent,
                after: Some(Payload::new(
                    b"new skill\n".to_vec(),
                    Permissions::file(0o644),
                )),
                rollback: RollbackPolicy::Restore,
            },
            Change::Exclusive {
                path: ".phoenix-ui/config.json".into(),
                expected: ExpectedFile::Absent,
                after: Some(Payload::new(
                    b"{\"schema\":1}\n".to_vec(),
                    Permissions::file(0o600),
                )),
                rollback: RollbackPolicy::Retain,
            },
        ],
    }
}

fn fresh_plan(transaction: &str) -> Plan {
    Plan {
        transaction_id: transaction.into(),
        manifest: Manifest {
            version: "a".into(),
            sha256: sha256(b"a"),
            providers: vec![".claude".into()],
            migration_sources: vec![],
        },
        expected_receipt: ExpectedFile::Absent,
        changes: vec![Change::SharedJson {
            path: SETTINGS.into(),
            expected: ExpectedFile::Absent,
            patches: vec![patch(
                None,
                Some(json!({"type":"command","command":"new fixture launcher","timeout":30})),
            )],
        }],
    }
}

#[test]
fn rollback_preserves_later_allow_guard_unknown_fields_and_original_text() {
    let f = Fixture::old();
    let p = plan(&f, "preserve-neighbors");
    let project = f.project();
    project.apply(&p).unwrap();
    let changes = |text: String| {
        text.replace("\"Read\"", "\"Read\",\"Bash(git status:*)\"")
            .replace("guard --threshold 5", "guard --threshold 9")
            .replace(
                "\"outerUnknown\":true",
                "\"outerUnknown\":true,\"addedAfter\":{\"arbitrary\":1.25000}",
            )
    };
    let edited = changes(String::from_utf8(f.read(SETTINGS)).unwrap());
    f.write(SETTINGS, edited.as_bytes());
    project.rollback(&p.transaction_id).unwrap();
    assert_eq!(
        String::from_utf8(f.read(SETTINGS)).unwrap(),
        changes(SHARED.into()),
        "rollback must keep later unowned edits and original unowned bytes"
    );
    assert_eq!(f.read(OLD), b"supported old skill bytes\n");
    assert_eq!(f.read("PRODUCT.md"), b"User-owned product requirements\n");
    assert!(f.0.join(".phoenix-ui/config.json").is_file());
    assert!(!f.0.join(NEW).exists());
    assert!(!f.0.join(ACTIVE).exists());
}

fn inventory(root: &Path) -> BTreeMap<String, (Vec<u8>, u32)> {
    fn walk(root: &Path, dir: &Path, result: &mut BTreeMap<String, (Vec<u8>, u32)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).unwrap();
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o7777
            };
            #[cfg(not(unix))]
            let mode = u32::from(meta.permissions().readonly());
            let bytes = if meta.file_type().is_symlink() {
                fs::read_link(&path)
                    .unwrap()
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec()
            } else if meta.is_dir() {
                b"directory".to_vec()
            } else {
                fs::read(&path).unwrap()
            };
            result.insert(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                (bytes, mode),
            );
            if meta.is_dir() && !meta.file_type().is_symlink() {
                walk(root, &path, result);
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

#[test]
fn preflight_is_read_only_on_success_and_on_every_rejected_input() {
    let f = Fixture::old();
    let p = plan(&f, "dry-run");
    let before = inventory(&f.0);
    assert_eq!(f.project().preflight(&p).unwrap().shared_json, [SETTINGS]);
    assert_eq!(inventory(&f.0), before);
    let mut modified = p.clone();
    if let Change::Exclusive { expected, .. } = &mut modified.changes[0] {
        *expected = ExpectedFile::Hash(sha256(b"different published bytes"));
    }
    assert_eq!(
        f.project().apply(&modified).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
    for invalid in [
        "../outside",
        "/absolute",
        "a/../b",
        "a\\b",
        "a:stream",
        "a//b",
        "a/.",
        "a.",
        "a ",
        "PRODUCT.md",
        ".phoenix-ui/install/journal.json",
    ] {
        let mut modified = p.clone();
        if let Change::Exclusive { path, .. } = &mut modified.changes[2] {
            *path = invalid.into();
        }
        assert!(
            f.project().apply(&modified).is_err(),
            "must reject {invalid}"
        );
        assert_eq!(
            inventory(&f.0),
            before,
            "preflight failed for {invalid} without writes"
        );
    }
}

#[test]
fn malformed_json_and_duplicate_keys_fail_before_project_writes() {
    for invalid in [
        "{\"hooks\":",
        "{ /* unfinished",
        "{\"hooks\":{},\"hooks\":{}}",
        "[]",
        "{\"permissions\":1,}",
    ] {
        let f = Fixture::old();
        f.write(SETTINGS, invalid.as_bytes());
        let p = plan(&f, "bad-json");
        let before = inventory(&f.0);
        assert_eq!(f.project().apply(&p).unwrap_err().kind, ErrorKind::Conflict);
        assert_eq!(inventory(&f.0), before);
    }
}

#[test]
fn exact_owned_values_reject_wrappers_and_ambiguous_matchers() {
    for text in [
        SHARED.replace(
            "old fixture launcher",
            "echo user-wrapper && old fixture launcher",
        ),
        // A neighbor group is fine; only two groups holding the exact owned
        // handler leave the removal ambiguous.
        SHARED.replace(
            "\"Stop\":[",
            "\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"old fixture launcher\",\"timeout\":30}]},",
        ),
    ] {
        let f = Fixture::old();
        f.write(SETTINGS, text.as_bytes());
        let p = plan(&f, "exact-entry");
        let before = inventory(&f.0);
        assert_eq!(f.project().apply(&p).unwrap_err().kind, ErrorKind::Conflict);
        assert_eq!(inventory(&f.0), before);
    }
}

#[test]
fn payloads_are_verified_before_staging_or_lock_creation() {
    let f = Fixture::old();
    let mut p = plan(&f, "bad-payload");
    if let Change::Exclusive {
        after: Some(payload),
        ..
    } = &mut p.changes[2]
    {
        payload.bytes.push(0);
    }
    let before = inventory(&f.0);
    assert_eq!(
        f.project().apply(&p).unwrap_err().kind,
        ErrorKind::InvalidPlan
    );
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn receipt_is_last_and_all_originals_are_backed_up_before_any_replacement() {
    let f = Fixture::old();
    let p = plan(&f, "activation-order");
    let mut order = Vec::new();
    f.project()
        .apply_with(&p, |point| match point {
            Checkpoint::JournalReady => {
                let journal: Value = serde_json::from_slice(
                    &f.read(".phoenix-ui/install/journals/activation-order.json"),
                )
                .unwrap();
                assert_eq!(journal["completed"], 0);
                for (i, op) in journal["operations"].as_array().unwrap().iter().enumerate() {
                    if !op["before"].is_null() {
                        let path = format!(".phoenix-ui/migrations/activation-order/{i}.before");
                        assert_eq!(sha256(&f.read(&path)), op["before"]["hash"]);
                    }
                }
                assert!(!f.0.join(NEW).exists());
                assert_eq!(f.read(SETTINGS), SHARED.as_bytes());
            }
            Checkpoint::AfterReplace(path) => {
                assert!(!f.0.join(ACTIVE).exists());
                if path == SETTINGS {
                    assert!(f.0.join(NEW).is_file());
                    assert!(f.0.join(OLD).is_file());
                }
                order.push(path);
            }
            Checkpoint::AfterReceipt => {
                assert!(!f.0.join(OLD).exists());
                assert!(f.0.join(NEW).is_file());
                order.push(ACTIVE.into());
            }
            _ => {}
        })
        .unwrap();
    assert_eq!(
        order,
        [NEW, ".phoenix-ui/config.json", SETTINGS, OLD, ACTIVE]
    );
}

#[test]
fn unchanged_shared_files_restore_exact_bytes_and_retained_new_data_survives() {
    let f = Fixture::old();
    let p = plan(&f, "exact-rollback");
    let project = f.project();
    project.apply(&p).unwrap();
    f.write(
        ".phoenix-ui/config.json",
        b"{\"schema\":2,\"editedAfter\":true}\n",
    );
    f.write(
        ".phoenix-ui/ui/reviews/new-evidence.json",
        b"accepted user evidence\n",
    );
    project.rollback(&p.transaction_id).unwrap();
    assert_eq!(f.read(SETTINGS), SHARED.as_bytes());
    assert_eq!(
        f.read(".phoenix-ui/config.json"),
        b"{\"schema\":2,\"editedAfter\":true}\n"
    );
    assert_eq!(
        f.read(".phoenix-ui/ui/reviews/new-evidence.json"),
        b"accepted user evidence\n"
    );
    let before = inventory(&f.0);
    project.rollback(&p.transaction_id).unwrap();
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn fresh_hook_scaffolds_preserve_subsequently_added_guard_and_settings() {
    let f = Fixture::empty();
    let p = fresh_plan("new-settings");
    let project = f.project();
    project.apply(&p).unwrap();
    let text = String::from_utf8(f.read(SETTINGS)).unwrap().replace("\"timeout\":30}", "\"timeout\":30},{\"command\":\"new independent guard --threshold 11\",\"unknown\":1.25000}").replace("\"hooks\":{", "\"permissions\":{\"allow\":[\"Read\"]},\"unknown\":1.00000,\"hooks\":{");
    f.write(SETTINGS, text.as_bytes());
    project.rollback(&p.transaction_id).unwrap();
    let text = String::from_utf8(f.read(SETTINGS)).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["permissions"]["allow"], json!(["Read"]));
    assert_eq!(
        value["hooks"]["Stop"][0]["hooks"],
        json!([{"command":"new independent guard --threshold 11","unknown":1.25}])
    );
    assert!(text.contains("\"unknown\":1.25000"));
    assert!(text.contains("\"unknown\":1.00000"));
}

#[test]
fn fresh_inserted_handler_modification_is_an_owned_conflict() {
    let f = Fixture::empty();
    let p = fresh_plan("new-handler-drift");
    let project = f.project();
    project.apply(&p).unwrap();
    let text = String::from_utf8(f.read(SETTINGS)).unwrap().replace(
        "new fixture launcher",
        "echo user-wrapper && new fixture launcher",
    );
    f.write(SETTINGS, text.as_bytes());
    let before = inventory(&f.0);
    assert_eq!(
        project.rollback(&p.transaction_id).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn fresh_handler_changed_during_rollback_cas_also_blocks_recovery() {
    let f = Fixture::empty();
    let p = fresh_plan("new-handler-cas");
    let project = f.project();
    project.apply(&p).unwrap();
    let error = project
        .rollback_with(&p.transaction_id, |point| {
            if point == Checkpoint::BeforeReplace(SETTINGS.into()) {
                let text = String::from_utf8(f.read(SETTINGS)).unwrap().replace(
                    "new fixture launcher",
                    "echo user-wrapper && new fixture launcher",
                );
                f.write(SETTINGS, text.as_bytes());
            }
        })
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    let before = inventory(&f.0);
    assert_eq!(project.recover().unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(
        inventory(&f.0),
        before,
        "recovery must not delete the modified fresh handler or its active receipt"
    );
    assert!(String::from_utf8(f.read(SETTINGS))
        .unwrap()
        .contains("echo user-wrapper && new fixture launcher"));
    assert!(f.0.join(ACTIVE).is_file());
}

#[test]
fn owned_drift_or_invalid_json_blocks_entire_rollback_before_any_restore() {
    for replacement in ["owned-handler", "invalid-json", "exclusive-file"] {
        let f = Fixture::old();
        let p = plan(&f, "rollback-conflict");
        let project = f.project();
        project.apply(&p).unwrap();
        match replacement {
            "owned-handler" => f.write(
                SETTINGS,
                String::from_utf8(f.read(SETTINGS))
                    .unwrap()
                    .replace(
                        "new fixture launcher",
                        "echo user-wrapper && new fixture launcher",
                    )
                    .as_bytes(),
            ),
            "invalid-json" => f.write(SETTINGS, b"{broken"),
            _ => f.write(NEW, b"edited after installation\n"),
        }
        let before = inventory(&f.0);
        assert_eq!(
            project.rollback(&p.transaction_id).unwrap_err().kind,
            ErrorKind::Conflict
        );
        assert_eq!(
            inventory(&f.0),
            before,
            "rollback preflight failed without restoring a previous operation"
        );
    }
}

#[test]
fn cas_rechecks_after_staging_and_keeps_unowned_new_bytes() {
    let f = Fixture::old();
    let p = plan(&f, "apply-cas");
    let project = f.project();
    let error = project
        .apply_with(&p, |point| {
            if point == Checkpoint::BeforeReplace(NEW.into()) {
                f.write(NEW, b"another writer's new skill\n");
            }
        })
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert_eq!(f.read(NEW), b"another writer's new skill\n");
    assert_eq!(f.read(SETTINGS), SHARED.as_bytes());
    assert!(!f.0.join(ACTIVE).exists());
    assert_eq!(project.recover().unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(f.read(NEW), b"another writer's new skill\n");
}

#[test]
fn stage_cas_rechecks_bytes_and_does_not_remove_another_writers_stage() {
    let f = Fixture::old();
    let p = plan(&f, "stage-cas");
    let project = f.project();
    let path = ".claude/skills/phoenix-ui/.SKILL.md.phoenix-stage-stage-cas";
    let error = project
        .apply_with(&p, |point| {
            if point == Checkpoint::BeforeReplace(NEW.into()) {
                f.write(path, b"another writer changed the staged bytes\n");
            }
        })
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert_eq!(f.read(path), b"another writer changed the staged bytes\n");
    assert!(!f.0.join(NEW).exists());
    assert!(!f.0.join(ACTIVE).exists());
    let before = inventory(&f.0);
    assert_eq!(project.recover().unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn receipt_stage_cas_rejects_changed_bytes_without_activating_or_erasing_them() {
    let f = Fixture::old();
    let p = plan(&f, "receipt-stage-cas");
    let project = f.project();
    let path = ".phoenix-ui/install/.active.json.phoenix-stage-receipt-stage-cas";
    let error = project
        .apply_with(&p, |point| {
            if point == Checkpoint::BeforeReplace(ACTIVE.into()) {
                f.write(path, b"another writer changed the receipt stage\n");
            }
        })
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(!f.0.join(ACTIVE).exists());
    assert_eq!(f.read(path), b"another writer changed the receipt stage\n");
    let before = inventory(&f.0);
    assert_eq!(project.recover().unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(
        inventory(&f.0),
        before,
        "a modified receipt stage must stop recovery before other managed entries are restored"
    );
}

#[test]
fn rollback_cas_conflict_can_be_recovered_without_erasing_later_shared_edit() {
    let f = Fixture::old();
    let p = plan(&f, "rollback-cas");
    let project = f.project();
    project.apply(&p).unwrap();
    let mut changed = false;
    let error = project
        .rollback_with(&p.transaction_id, |point| {
            if !changed && point == Checkpoint::BeforeReplace(SETTINGS.into()) {
                changed = true;
                let text = String::from_utf8(f.read(SETTINGS))
                    .unwrap()
                    .replace("guard --threshold 5", "guard --threshold 17");
                f.write(SETTINGS, text.as_bytes());
            }
        })
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(String::from_utf8(f.read(SETTINGS))
        .unwrap()
        .contains("new fixture launcher"));
    assert_eq!(
        project.recover().unwrap(),
        Recovery::RolledBack(p.transaction_id)
    );
    assert_eq!(
        String::from_utf8(f.read(SETTINGS)).unwrap(),
        SHARED.replace("guard --threshold 5", "guard --threshold 17")
    );
}

#[test]
fn final_owned_validation_prevents_receipt_activation_after_intervening_drift() {
    let f = Fixture::old();
    let p = plan(&f, "last-validation");
    let project = f.project();
    let error = project
        .apply_with(&p, |point| {
            if point == Checkpoint::BeforeReceipt {
                f.write(NEW, b"changed during activation\n");
            }
        })
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert!(!f.0.join(ACTIVE).exists());
    assert_eq!(project.recover().unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(f.read(NEW), b"changed during activation\n");
}

#[test]
fn duplicate_owned_handler_is_a_conflict_even_with_identical_bytes() {
    let f = Fixture::old();
    let p = plan(&f, "duplicate-handler");
    let project = f.project();
    project.apply(&p).unwrap();
    let text = String::from_utf8(f.read(SETTINGS)).unwrap();
    let handler = "{\"type\":\"command\",\"command\":\"new fixture launcher\",\"timeout\":30}";
    f.write(
        SETTINGS,
        text.replace(handler, &format!("{handler},{handler}"))
            .as_bytes(),
    );
    let before = inventory(&f.0);
    assert_eq!(
        project.rollback(&p.transaction_id).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn identical_retry_is_idempotent_and_cannot_rebind_a_transaction_id() {
    let f = Fixture::old();
    let p = plan(&f, "retry");
    let project = f.project();
    let receipt = project.apply(&p).unwrap();
    let text = String::from_utf8(f.read(SETTINGS))
        .unwrap()
        .replace("guard --threshold 5", "guard --threshold 19");
    f.write(SETTINGS, text.as_bytes());
    let before = inventory(&f.0);
    assert!(project.preflight(&p).unwrap().already_applied);
    assert_eq!(project.apply(&p).unwrap().plan_sha256, receipt.plan_sha256);
    assert_eq!(inventory(&f.0), before);
    let mut changed = p.clone();
    changed.manifest.version = "another-version".into();
    assert_eq!(
        project.apply(&changed).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn upgrade_rollback_restores_previous_active_receipt_and_entry_bytes() {
    let f = Fixture::old();
    let p = plan(&f, "candidate-a");
    let project = f.project();
    project.apply(&p).unwrap();
    let receipt_a = f.read(ACTIVE);
    let mut b = p.clone();
    b.transaction_id = "candidate-b".into();
    b.manifest.version = "b".into();
    b.expected_receipt = ExpectedFile::Hash(sha256(&receipt_a));
    b.changes = vec![Change::Exclusive {
        path: NEW.into(),
        expected: ExpectedFile::Hash(sha256(&f.read(NEW))),
        after: Some(Payload::new(
            b"candidate-b skill\n".to_vec(),
            Permissions::file(0o644),
        )),
        rollback: RollbackPolicy::Restore,
    }];
    project.apply(&b).unwrap();
    let before = inventory(&f.0);
    assert_eq!(
        project.rollback(&p.transaction_id).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
    project.rollback(&b.transaction_id).unwrap();
    assert_eq!(f.read(ACTIVE), receipt_a);
    assert_eq!(f.read(NEW), b"new skill\n");
    project.rollback(&p.transaction_id).unwrap();
    assert_eq!(f.read(OLD), b"supported old skill bytes\n");
    assert_eq!(f.read(SETTINGS), SHARED.as_bytes());
}

#[cfg(unix)]
#[test]
fn original_permissions_are_recorded_backed_up_and_restored() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::old();
    fs::set_permissions(f.0.join(OLD), fs::Permissions::from_mode(0o751)).unwrap();
    fs::set_permissions(f.0.join(SETTINGS), fs::Permissions::from_mode(0o640)).unwrap();
    let p = plan(&f, "permissions");
    let project = f.project();
    project.apply(&p).unwrap();
    let backup = f.0.join(".phoenix-ui/migrations/permissions/3.before");
    assert_eq!(
        fs::metadata(&backup).unwrap().permissions().mode() & 0o7777,
        0o751
    );
    assert_eq!(
        fs::metadata(f.0.join(".phoenix-ui/migrations/permissions"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    project.rollback(&p.transaction_id).unwrap();
    assert_eq!(
        fs::metadata(f.0.join(OLD)).unwrap().permissions().mode() & 0o7777,
        0o751
    );
    assert_eq!(
        fs::metadata(f.0.join(SETTINGS))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o640
    );
}

#[cfg(unix)]
#[test]
fn escaping_or_dangling_symlinks_are_rejected_without_writing_targets() {
    use std::os::unix::fs::symlink;
    let outside = Fixture::empty();
    outside.write("sentinel", b"outside bytes\n");
    for relative in [".claude/skills/phoenix-ui", ".phoenix-ui"] {
        let f = Fixture::old();
        let p = plan(&f, "symlink");
        let path = f.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(&outside.0, &path).unwrap();
        let before = inventory(&f.0);
        let external = inventory(&outside.0);
        assert_eq!(f.project().apply(&p).unwrap_err().kind, ErrorKind::Conflict);
        assert_eq!(inventory(&f.0), before);
        assert_eq!(inventory(&outside.0), external);
    }
    let f = Fixture::old();
    let p = plan(&f, "dangling");
    fs::remove_file(f.0.join(OLD)).unwrap();
    symlink(outside.0.join("missing"), f.0.join(OLD)).unwrap();
    let before = inventory(&f.0);
    assert_eq!(f.project().apply(&p).unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(inventory(&f.0), before);
}

#[cfg(unix)]
#[test]
fn nonwritable_parent_fails_read_only_preflight() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::old();
    let p = plan(&f, "readonly");
    fs::set_permissions(
        f.0.join(".claude/skills"),
        fs::Permissions::from_mode(0o555),
    )
    .unwrap();
    let before = inventory(&f.0);
    assert_eq!(f.project().apply(&p).unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(inventory(&f.0), before);
    fs::set_permissions(
        f.0.join(".claude/skills"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
}

#[test]
fn corrupted_backup_prevents_any_rollback_mutation() {
    let f = Fixture::old();
    let p = plan(&f, "corrupt-backup");
    let project = f.project();
    project.apply(&p).unwrap();
    f.write(
        ".phoenix-ui/migrations/corrupt-backup/3.before",
        b"damaged original backup\n",
    );
    let before = inventory(&f.0);
    assert_eq!(
        project.rollback(&p.transaction_id).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
}

#[test]
fn changed_journal_ownership_is_rejected_before_restoring_any_entry() {
    let f = Fixture::old();
    let p = plan(&f, "journal-ownership");
    let project = f.project();
    project.apply(&p).unwrap();
    let path = ".phoenix-ui/install/journals/journal-ownership.json";
    let mut journal: Value = serde_json::from_slice(&f.read(path)).unwrap();
    journal["operations"][0]["after"]["hash"] = json!(sha256(b"changed journal ownership"));
    f.write(path, &serde_json::to_vec_pretty(&journal).unwrap());
    let before = inventory(&f.0);
    assert_eq!(
        project.rollback(&p.transaction_id).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(inventory(&f.0), before);
}

#[cfg(unix)]
#[test]
fn write_permission_loss_after_the_first_mutation_leaves_a_recoverable_journal() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::old();
    let p = plan(&f, "lost-write-permission");
    let project = f.project();
    let error = project
        .apply_with(&p, |point| {
            if point == Checkpoint::AfterReplace(NEW.into()) {
                fs::set_permissions(f.0.join(".phoenix-ui"), fs::Permissions::from_mode(0o555))
                    .unwrap();
            }
        })
        .unwrap_err();
    fs::set_permissions(f.0.join(".phoenix-ui"), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(error.kind, ErrorKind::Io | ErrorKind::Conflict));
    assert!(f.0.join(NEW).is_file());
    assert!(!f.0.join(ACTIVE).exists());
    assert_eq!(f.read(SETTINGS), SHARED.as_bytes());
    assert_eq!(
        project.recover().unwrap(),
        Recovery::RolledBack(p.transaction_id)
    );
    assert!(!f.0.join(NEW).exists());
    assert_eq!(f.read(OLD), b"supported old skill bytes\n");
    assert_eq!(f.read(SETTINGS), SHARED.as_bytes());
}

#[test]
fn an_os_lock_rejects_another_installer_but_stale_pid_text_does_not() {
    let f = Fixture::old();
    let p = plan(&f, "os-lock");
    f.write(
        ".phoenix-ui/install/install.lock",
        b"stale PID 2147483647\n",
    );
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.0.join(".phoenix-ui/install/install.lock"))
        .unwrap();
    file.try_lock().unwrap();
    let before = inventory(&f.0);
    assert_eq!(f.project().apply(&p).unwrap_err().kind, ErrorKind::Busy);
    assert_eq!(inventory(&f.0), before);
    drop(file);
    f.project().apply(&p).unwrap();
    assert_eq!(
        f.read(".phoenix-ui/install/install.lock"),
        b"stale PID 2147483647\n"
    );
}

fn checkpoint_name(point: &Checkpoint) -> Option<&'static str> {
    match point {
        Checkpoint::JournalReady => Some("journal-ready"),
        Checkpoint::BeforeReplace(path) if path == NEW => Some("before-new"),
        Checkpoint::BeforeReplace(path) if path == ACTIVE => Some("before-active-replace"),
        Checkpoint::AfterReplace(path) if path == NEW => Some("after-new"),
        Checkpoint::AfterReplace(path) if path == SETTINGS => Some("after-shared"),
        Checkpoint::BeforeReceipt => Some("before-receipt"),
        Checkpoint::AfterReceipt => Some("after-receipt"),
        Checkpoint::AfterRollback(path) if path == SETTINGS => Some("rollback-after-shared"),
        _ => None,
    }
}

/// Runs only in a child of the kill/concurrency tests. It pauses at an exact
/// checkpoint until its owning test kills it; it never targets another process.
#[test]
fn transaction_child() {
    let Ok(root) = std::env::var("PHOENIX_INSTALL_TEST_CHILD_ROOT") else {
        return;
    };
    let point = std::env::var("PHOENIX_INSTALL_TEST_CHILD_POINT").unwrap();
    let action = std::env::var("PHOENIX_INSTALL_TEST_CHILD_ACTION").unwrap();
    let f = Fixture(PathBuf::from(root));
    let p = if action == "fresh-rollback" {
        fresh_plan("child-txn")
    } else {
        plan(&f, "child-txn")
    };
    let project = f.project();
    let mut pause = |checkpoint: Checkpoint| {
        if checkpoint_name(&checkpoint) == Some(point.as_str()) {
            let ready = f.0.join("child-ready");
            fs::write(&ready, point.as_bytes()).unwrap();
            fs::File::open(&ready).unwrap().sync_all().unwrap();
            loop {
                std::thread::park();
            }
        }
    };
    if action == "fresh-rollback" {
        project.apply(&p).unwrap();
        let text = String::from_utf8(f.read(SETTINGS)).unwrap().replace(
            "\"hooks\":{",
            "\"permissions\":{\"allow\":[\"Read\"]},\"unknown\":1.25000,\"hooks\":{",
        );
        f.write(SETTINGS, text.as_bytes());
        project
            .rollback_with(&p.transaction_id, &mut pause)
            .unwrap();
    } else if action == "rollback" {
        project.apply(&p).unwrap();
        project
            .rollback_with(&p.transaction_id, &mut pause)
            .unwrap();
    } else {
        project.apply_with(&p, &mut pause).unwrap();
    }
    panic!("child did not reach requested checkpoint {point}");
}

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn paused_child(f: &Fixture, point: &str, action: &str) -> ChildGuard {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "transaction_child", "--nocapture"])
        .env("PHOENIX_INSTALL_TEST_CHILD_ROOT", &f.0)
        .env("PHOENIX_INSTALL_TEST_CHILD_POINT", point)
        .env("PHOENIX_INSTALL_TEST_CHILD_ACTION", action)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut guard = ChildGuard(child);
    let deadline = Instant::now() + Duration::from_secs(20);
    while !f.0.join("child-ready").is_file() {
        assert!(
            guard.0.try_wait().unwrap().is_none(),
            "child exited before {point}"
        );
        assert!(Instant::now() < deadline, "child did not reach {point}");
        std::thread::sleep(Duration::from_millis(10));
    }
    guard
}

#[test]
fn killed_apply_is_recoverable_at_each_durable_boundary_and_receipt_is_last() {
    for point in [
        "journal-ready",
        "before-new",
        "after-new",
        "after-shared",
        "before-receipt",
        "before-active-replace",
        "after-receipt",
    ] {
        let f = Fixture::old();
        let mut child = paused_child(&f, point, "apply");
        let project = f.project();
        assert_eq!(
            project.recover().unwrap_err().kind,
            ErrorKind::Busy,
            "live child holds the OS lock at {point}"
        );
        if point == "after-receipt" {
            assert!(f.0.join(ACTIVE).is_file());
        } else {
            assert!(!f.0.join(ACTIVE).exists());
        }
        child.0.kill().unwrap();
        let status = child.0.wait().unwrap();
        assert!(!status.success());
        eprintln!("killed owned child at {point}: {status}");
        if point == "after-receipt" {
            assert_eq!(
                project.recover().unwrap(),
                Recovery::Activated("child-txn".into())
            );
            project.rollback("child-txn").unwrap();
        } else {
            assert_eq!(
                project.recover().unwrap(),
                Recovery::RolledBack("child-txn".into())
            );
        }
        assert_eq!(
            f.read(OLD),
            b"supported old skill bytes\n",
            "original skill survives {point}"
        );
        assert_eq!(
            f.read(SETTINGS),
            SHARED.as_bytes(),
            "original hook bytes survive {point}"
        );
        assert_eq!(f.read("PRODUCT.md"), b"User-owned product requirements\n");
        assert!(!f.0.join(NEW).exists());
        assert!(!f.0.join(ACTIVE).exists());
        assert!(
            inventory(&f.0)
                .keys()
                .all(|path| !path.contains(".phoenix-stage-")),
            "recovery must retire exact journaled stages at {point}"
        );
        assert_eq!(project.recover().unwrap(), Recovery::Clean);
    }
}

#[test]
fn killed_rollback_resumes_and_keeps_later_unrelated_shared_changes() {
    let f = Fixture::old();
    let mut child = paused_child(&f, "rollback-after-shared", "rollback");
    let text = String::from_utf8(f.read(SETTINGS))
        .unwrap()
        .replace("guard --threshold 5", "guard --threshold 23");
    f.write(SETTINGS, text.as_bytes());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let project = f.project();
    assert_eq!(
        project.recover().unwrap(),
        Recovery::RolledBack("child-txn".into())
    );
    assert_eq!(
        String::from_utf8(f.read(SETTINGS)).unwrap(),
        SHARED.replace("guard --threshold 5", "guard --threshold 23")
    );
    assert_eq!(f.read(OLD), b"supported old skill bytes\n");
    assert!(!f.0.join(NEW).exists());
    assert!(!f.0.join(ACTIVE).exists());
}

#[test]
fn killed_fresh_rollback_recognizes_its_durable_inverse_fingerprint() {
    let f = Fixture::empty();
    let mut child = paused_child(&f, "rollback-after-shared", "fresh-rollback");
    let restored = f.read(SETTINGS);
    assert!(!String::from_utf8(restored.clone())
        .unwrap()
        .contains("new fixture launcher"));
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert_eq!(
        f.project().recover().unwrap(),
        Recovery::RolledBack("child-txn".into())
    );
    assert_eq!(f.read(SETTINGS), restored);
    assert!(String::from_utf8(restored)
        .unwrap()
        .contains("\"unknown\":1.25000"));
    assert!(!f.0.join(ACTIVE).exists());
}

#[test]
fn an_interrupted_stage_with_new_bytes_is_a_conflict_and_is_never_deleted() {
    let f = Fixture::old();
    let mut child = paused_child(&f, "before-new", "apply");
    let path = ".claude/skills/phoenix-ui/.SKILL.md.phoenix-stage-child-txn";
    f.write(path, b"new bytes in a journaled stage\n");
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let before = inventory(&f.0);
    assert_eq!(f.project().recover().unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(inventory(&f.0), before);
    assert_eq!(f.read(path), b"new bytes in a journaled stage\n");
}

#[test]
fn all_fixed_m1_sources_and_provider_hook_shapes_preserve_mixed_neighbors() {
    let sources = [
        include_str!("../../../migration-sources/0.3.0.json"),
        include_str!("../../../migration-sources/0.4.0.json"),
        include_str!("../../../migration-sources/0.5.0.json"),
        include_str!("../../../migration-sources/0.6.0.json"),
    ];
    let mut count = 0;
    for source in sources {
        let source: Value = serde_json::from_str(source).unwrap();
        for (provider, entry) in source["providers"].as_object().unwrap() {
            for hook in entry["hooks"].as_array().unwrap() {
                let f = Fixture::empty();
                let mut original = hook["value"].clone();
                original["permissions"] = json!({"allow":["Read"],"unknownPolicy":"keep"});
                original["fixtureUnknown"] = json!({"arbitrary":[true,42]});
                let mut patches = Vec::new();
                for (event, entries) in original["hooks"].as_object_mut().unwrap() {
                    for outer in entries.as_array_mut().unwrap() {
                        let nested = outer["hooks"].is_array();
                        let parent = if nested {
                            vec![
                                JsonStep::Key("hooks".into()),
                                JsonStep::Key(event.clone()),
                                JsonStep::Match(Identity {
                                    fields: [("matcher".into(), outer.get("matcher").cloned())]
                                        .into(),
                                    required_keys: vec!["hooks".into()],
                                }),
                                JsonStep::Key("hooks".into()),
                            ]
                        } else {
                            vec![JsonStep::Key("hooks".into()), JsonStep::Key(event.clone())]
                        };
                        let owned = if nested {
                            outer["hooks"].as_array().unwrap().clone()
                        } else {
                            vec![outer.clone()]
                        };
                        for (i, old) in owned.into_iter().enumerate() {
                            let mut new = old.clone();
                            for key in ["command", "commandWindows", "bash", "powershell"] {
                                if new[key].is_string() {
                                    new[key] =
                                        json!(format!("phoenix fixture {provider}/{event}/{key}"));
                                }
                            }
                            patches.push(JsonPatch {
                                parent: parent.clone(),
                                slot: JsonSlot::Element(i),
                                before: Some(old),
                                after: Some(new),
                            });
                        }
                        if nested {
                            outer["fixtureMatcherNote"] = json!("preserve outer field");
                            outer["hooks"].as_array_mut().unwrap().push(
                                json!({"command":"neighbor guard --threshold 5","unknown":true}),
                            );
                        }
                    }
                    entries.as_array_mut().unwrap().push(json!({"type":"command","command":"echo impeccable notes are user data","unrelated":true}));
                }
                let original_bytes = serde_json::to_vec_pretty(&original).unwrap();
                let path = if hook["shape"] == "bundle" {
                    entry["hookArtifact"]["source"].as_str().unwrap()
                } else {
                    entry["hookArtifact"]["destination"].as_str().unwrap()
                };
                f.write(path, &original_bytes);
                let p = Plan {
                    transaction_id: "m1-source".into(),
                    manifest: Manifest {
                        version: "candidate-a".into(),
                        sha256: sha256(source["bundleManifestSha256"].as_str().unwrap().as_bytes()),
                        providers: vec![provider.clone()],
                        migration_sources: vec![source["packageVersion"].as_str().unwrap().into()],
                    },
                    expected_receipt: ExpectedFile::Absent,
                    changes: vec![Change::SharedJson {
                        path: path.into(),
                        expected: ExpectedFile::Hash(sha256(&original_bytes)),
                        patches,
                    }],
                };
                let project = f.project();
                project.apply(&p).unwrap_or_else(|e| {
                    panic!(
                        "{} {provider} {}: {e}",
                        source["packageVersion"], hook["shape"]
                    )
                });
                let text = String::from_utf8(f.read(path))
                    .unwrap()
                    .replace(
                        "neighbor guard --threshold 5",
                        "neighbor guard --threshold 29",
                    )
                    .replace("\"Read\"", "\"Read\",\"Bash(git status:*)\"");
                f.write(path, text.as_bytes());
                project.rollback(&p.transaction_id).unwrap();
                let expected = String::from_utf8(original_bytes)
                    .unwrap()
                    .replace(
                        "neighbor guard --threshold 5",
                        "neighbor guard --threshold 29",
                    )
                    .replace("\"Read\"", "\"Read\",\"Bash(git status:*)\"");
                assert_eq!(
                    String::from_utf8(f.read(path)).unwrap(),
                    expected,
                    "{} {provider} {}",
                    source["packageVersion"],
                    hook["shape"]
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 44);
    eprintln!("M1 fixed source coverage: {count} provider/channel hook manifests across 0.3.0, 0.4.0, 0.5.0 and 0.6.0");
}
