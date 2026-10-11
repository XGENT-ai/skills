use phoenix_ui_install::{
    sha256, Change, ErrorKind, ExpectedFile, JsonPatch, JsonSlot, JsonStep, Manifest, Plan, Project,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const PATH: &str = ".claude/settings.json";
const TEXT: &str = "{\n // preserved comment\n \"phoenix\": { \"version\" : \"0.1.0\" },\n \"hooks\": { \"Stop\": [ { \"type\" : \"command\", \"command\" : \"phoenix fixture hook\" }, {\"type\":\"command\",\"command\":\"user hook\"} ] },\n \"unknownNumber\":1.230000000000000000000000\n}\n";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-shared-assertion-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(root.join(PATH), TEXT).unwrap();
        Self(root)
    }
    fn project(&self) -> Project {
        Project::open(&self.0).unwrap()
    }
    fn plan(&self, patch: JsonPatch) -> Plan {
        Plan {
            transaction_id: "assertion-fixture".into(),
            manifest: Manifest {
                version: "fixture".into(),
                sha256: sha256(b"manifest"),
                providers: vec![".claude".into()],
                migration_sources: vec![],
            },
            expected_receipt: ExpectedFile::Absent,
            changes: vec![Change::SharedJson {
                path: PATH.into(),
                expected: ExpectedFile::Hash(sha256(TEXT.as_bytes())),
                patches: vec![patch],
            }],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn member() -> JsonPatch {
    let value = json!({"version":"0.1.0"});
    JsonPatch {
        parent: vec![],
        slot: JsonSlot::Member("phoenix".into()),
        before: Some(value.clone()),
        after: Some(value),
    }
}
fn handler() -> JsonPatch {
    let value = json!({"type":"command","command":"phoenix fixture hook"});
    JsonPatch {
        parent: vec![JsonStep::Key("hooks".into()), JsonStep::Key("Stop".into())],
        slot: JsonSlot::Element(0),
        before: Some(value.clone()),
        after: Some(value),
    }
}

#[test]
fn unchanged_member_ownership_preserves_all_jsonc_bytes_and_rolls_back() {
    let f = Fixture::new();
    let p = f.plan(member());
    f.project().preflight(&p).unwrap();
    f.project().apply(&p).unwrap();
    assert_eq!(fs::read(f.0.join(PATH)).unwrap(), TEXT.as_bytes());
    let verified = f.project().verified_installation().unwrap().unwrap();
    assert_eq!(
        verified.json_patches[PATH][0].before,
        verified.json_patches[PATH][0].after
    );
    f.project().rollback("assertion-fixture").unwrap();
    assert_eq!(fs::read(f.0.join(PATH)).unwrap(), TEXT.as_bytes());
}

#[test]
fn unchanged_array_handler_remains_owned_without_insertion_or_retirement() {
    let f = Fixture::new();
    f.project().apply(&f.plan(handler())).unwrap();
    assert_eq!(fs::read(f.0.join(PATH)).unwrap(), TEXT.as_bytes());
    f.project().verified_installation().unwrap().unwrap();
    f.project().rollback("assertion-fixture").unwrap();
    assert_eq!(fs::read(f.0.join(PATH)).unwrap(), TEXT.as_bytes());
}

#[test]
fn unchanged_assertion_never_claims_a_different_current_entry() {
    let f = Fixture::new();
    let mut patch = handler();
    patch.before.as_mut().unwrap()["command"] = json!("different");
    patch.after = patch.before.clone();
    assert_eq!(
        f.project().preflight(&f.plan(patch)).unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(fs::read(f.0.join(PATH)).unwrap(), TEXT.as_bytes());
    assert!(!f.0.join(".phoenix-ui").exists());
}

#[test]
fn unchanged_owned_handler_drift_blocks_verification_and_rollback() {
    let f = Fixture::new();
    f.project().apply(&f.plan(handler())).unwrap();
    let changed = TEXT.replace("phoenix fixture hook", "customized hook");
    fs::write(f.0.join(PATH), &changed).unwrap();
    assert_eq!(
        f.project().verified_installation().unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(
        f.project().rollback("assertion-fixture").unwrap_err().kind,
        ErrorKind::Conflict
    );
    assert_eq!(fs::read_to_string(f.0.join(PATH)).unwrap(), changed);
}

#[test]
fn absent_to_absent_patch_is_still_invalid_and_read_only() {
    let f = Fixture::new();
    let mut patch = member();
    patch.before = None;
    patch.after = None;
    assert_eq!(
        f.project().preflight(&f.plan(patch)).unwrap_err().kind,
        ErrorKind::InvalidPlan
    );
    assert_eq!(fs::read(f.0.join(PATH)).unwrap(), TEXT.as_bytes());
    assert!(!f.0.join(".phoenix-ui").exists());
}
