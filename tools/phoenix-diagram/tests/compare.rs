use phoenix_diagram::{
    compare::{self, CompareReceipt, CompareRequest},
    delivery::{CompileRequest, Status, VisualStatus},
    input::CompileOptions,
    render::EmbeddedAssets,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "phoenix-comparison-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(fs::canonicalize(p).unwrap())
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
fn document() -> Value {
    json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Service","output":"diagram.html","input_contract":"phoenix-v1","quality_profile":"standard","legend":{"mode":"hidden"}},"components":[{"id":"api","type":"backend","label":"API","pos":[100,100]},{"id":"db","type":"database","label":"Store","pos":[400,100]}],"connections":[{"id":"read","from":"api","to":"db","label":"Read"}]})
}
fn request<'a>(
    base: &'a Path,
    head: &'a Path,
    output: &'a Path,
    receipt: Option<&'a Path>,
    options: &'a CompileOptions,
    assets: &'a EmbeddedAssets<'a>,
) -> CompareRequest<'a> {
    let side = |candidate| CompileRequest {
        kind: phoenix_diagram::input::DiagramType::Architecture,
        candidate,
        options,
        assets,
        repo_root: None,
        brand_cache: None,
    };
    CompareRequest {
        base: side(base),
        head: side(head),
        output,
        receipt,
    }
}
fn sources(root: &Workspace) -> (PathBuf, PathBuf) {
    let base = root.path("base.json");
    let head = root.path("head.json");
    let before = document();
    let mut after = before.clone();
    after["components"][1]["pos"] = json!([400, 250]);
    after["components"][1]["label"] = json!("Records");
    fs::write(&base, serde_json::to_vec(&before).unwrap()).unwrap();
    fs::write(&head, serde_json::to_vec(&after).unwrap()).unwrap();
    (base, head)
}
#[test]
fn compare_writes_html_and_summary_and_review_needs_a_visual_check() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let receipt_path = root.path("private-comparison.json");
    let options = CompileOptions::default();
    let assets = EmbeddedAssets::builtin();
    let receipt = compare::run(&request(
        &base,
        &head,
        &output,
        Some(&receipt_path),
        &options,
        &assets,
    ));
    assert_eq!(receipt.status, Status::Pass, "{:?}", receipt.diagnostics);
    assert_eq!(receipt.exit_code(), 0);
    assert_eq!(receipt.browser_status, Status::NotRun);
    assert_eq!(receipt.visual_review.status, VisualStatus::RequiredPending);
    assert_eq!(receipt.delivery_status, "incomplete");
    assert!(compare::is_comparison(&output));
    let original_html = fs::read(&output).unwrap();
    let original_receipt = fs::read(&receipt_path).unwrap();
    assert!(!String::from_utf8_lossy(&original_html).contains(root.0.to_str().unwrap()));
    let checked = compare::check(&output);
    assert_eq!(checked.status, Status::Pass, "{:?}", checked.diagnostics);
    assert_eq!(checked.checks, receipt.checks);
    let rejected = compare::record_review(&output, &base);
    assert_eq!(rejected.status, Status::Failed);
    assert!(rejected
        .diagnostics
        .iter()
        .any(|d| d.code == "review/automated-gates"));
    assert_eq!(fs::read(&output).unwrap(), original_html);
    assert_eq!(fs::read(&receipt_path).unwrap(), original_receipt);
}
#[test]
fn invalid_side_and_missing_stable_connection_id_preserve_the_existing_pair() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let receipt_path = root.path("comparison.html.compare.json");
    let options = CompileOptions::default();
    let assets = EmbeddedAssets::builtin();
    fs::write(&output, b"old html").unwrap();
    fs::write(&receipt_path, b"old receipt").unwrap();
    fs::write(&head, b"{partial").unwrap();
    let failed = compare::run(&request(&base, &head, &output, None, &options, &assets));
    assert_eq!(failed.status, Status::Failed);
    assert_eq!(fs::read(&output).unwrap(), b"old html");
    assert_eq!(fs::read(&receipt_path).unwrap(), b"old receipt");
    let mut no_id = document();
    no_id["connections"][0]
        .as_object_mut()
        .unwrap()
        .remove("id");
    fs::write(&head, serde_json::to_vec(&no_id).unwrap()).unwrap();
    let failed = compare::run(&request(&base, &head, &output, None, &options, &assets));
    assert!(
        failed
            .diagnostics
            .iter()
            .any(|d| d.code == "delta/relationship-id-required"),
        "{:?}",
        failed.diagnostics
    );
    assert_eq!(fs::read(&output).unwrap(), b"old html");
    assert_eq!(fs::read(&receipt_path).unwrap(), b"old receipt");
}
#[test]
fn the_summary_must_sit_beside_the_comparison_html() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let options = CompileOptions::default();
    let assets = EmbeddedAssets::builtin();
    let other = root.path("other");
    fs::create_dir(&other).unwrap();
    let target = other.join("receipt.json");
    let failed = compare::run(&request(
        &base,
        &head,
        &output,
        Some(&target),
        &options,
        &assets,
    ));
    assert_eq!(failed.exit_code(), 2);
    assert!(!output.exists() && !target.exists());
}

#[path = "support/comparison_browser_fixture.rs"]
mod browser_fixture;
#[test]
fn protocol_visual_check_updates_the_summary_and_review_covers_twelve_images() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let summary_path = root.path("comparison.html.compare.json");
    let options = CompileOptions::default();
    let assets = EmbeddedAssets::builtin();
    let comparison = compare::run(&request(&base, &head, &output, None, &options, &assets));
    assert_eq!(
        comparison.status,
        Status::Pass,
        "{:?}",
        comparison.diagnostics
    );
    let html = fs::read(&output).unwrap();
    let fixture = browser_fixture::Fixture::new();
    let browser_options = fixture.options(&root.path("evidence"), true);
    let (response, exit) = compare::browser_check(&output, &browser_options, true);
    assert_eq!(exit, 0, "{response:#}");
    assert_eq!(response["summary"], json!(summary_path));
    let read_summary =
        || -> CompareReceipt { serde_json::from_slice(&fs::read(&summary_path).unwrap()).unwrap() };
    let summary = read_summary();
    assert_eq!(summary.browser_status, Status::Pass);
    assert_eq!(summary.screenshots.len(), 12);
    assert_eq!(summary.visual_review.status, VisualStatus::RequiredPending);
    let draft = root.path("review.json");
    let write = |screenshots: &[std::path::PathBuf]| {
        let screenshots: Vec<_> = screenshots
            .iter()
            .map(|path| json!({"path":path,"observations":"Synthetic protocol fixture observation; not real image acceptance"}))
            .collect();
        fs::write(
            &draft,
            serde_json::to_vec(&json!({"reviewer":"synthetic state-machine test","verdict":"passed","screenshots":screenshots,"findings":[]})).unwrap(),
        )
        .unwrap();
    };
    write(&summary.screenshots[1..]);
    let missing = compare::record_review(&output, &draft);
    assert_eq!(missing.status, Status::Failed);
    assert_eq!(missing.delivery_status, "incomplete");
    write(&summary.screenshots);
    let completed = compare::record_review(&output, &draft);
    assert_eq!(
        completed.status,
        Status::Pass,
        "{:?}",
        completed.diagnostics
    );
    assert_eq!(completed.delivery_status, "complete");
    assert_eq!(read_summary().delivery_status, "complete");
    assert_eq!(fs::read(&output).unwrap(), html);
    assert!(!fixture.commands().is_empty());
}

#[test]
fn public_cli_compares_checks_and_reports_its_custom_receipt_without_finalizing() {
    use std::{
        os::unix::fs::{MetadataExt, PermissionsExt},
        process::Command,
    };
    let root = Workspace::new();
    let executable = root.path("phoenix-diagram");
    // Copy in a child to avoid inheritable write descriptors in parallel tests.
    assert!(Command::new("/bin/cp")
        .arg(env!("CARGO_BIN_EXE_phoenix-diagram"))
        .arg(&executable)
        .status()
        .unwrap()
        .success());
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(fs::symlink_metadata(&executable).unwrap().nlink(), 1);
    let (base, head) = sources(&root);
    let output = root.path("cli-comparison.html");
    let receipt = root.path("cli-receipt.json");
    let compared = Command::new(&executable)
        .arg("compare")
        .arg("architecture")
        .arg(&base)
        .arg(&head)
        .arg(&output)
        .arg("--receipt")
        .arg(&receipt)
        .args([
            "--base-contract",
            "phoenix-v1",
            "--head-contract",
            "phoenix-v1",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        compared.status.success(),
        "{}",
        String::from_utf8_lossy(&compared.stdout)
    );
    let json: Value = serde_json::from_slice(&compared.stdout).unwrap();
    assert_eq!(json["kind"], "architecture-comparison");
    assert_eq!(json["deliveryStatus"], "incomplete");
    assert_eq!(json["browserStatus"], "not-run");
    assert!(receipt.is_file());
    let checked = Command::new(&executable)
        .arg("check")
        .arg(&output)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );
    let checked: Value = serde_json::from_slice(&checked.stdout).unwrap();
    assert_eq!(checked["kind"], "architecture-comparison-check");
}
