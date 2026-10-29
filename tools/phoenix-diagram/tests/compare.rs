use phoenix_diagram::{
    check::FileIdentity,
    compare::{self, CompareRequest},
    delivery::{CompileRequest, EngineIdentity, Status, VisualStatus},
    input::{sha256, CompileOptions},
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
fn engine() -> EngineIdentity {
    EngineIdentity {
        source_input_sha256: sha256(b"comparison engine"),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v2".into(),
        binary: FileIdentity::of(b"comparison engine"),
    }
}
fn request<'a>(
    base: &'a Path,
    head: &'a Path,
    output: &'a Path,
    receipt: Option<&'a Path>,
    options: &'a CompileOptions,
    engine: &'a EngineIdentity,
    assets: &'a EmbeddedAssets<'a>,
) -> CompareRequest<'a> {
    let side = |candidate| CompileRequest {
        kind: phoenix_diagram::input::DiagramType::Architecture,
        candidate,
        options,
        engine,
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
fn compare_publishes_an_immutable_pair_then_requires_a_separate_visual_summary() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let receipt_path = root.path("private-comparison.json");
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets::builtin();
    let receipt = compare::run(&request(
        &base,
        &head,
        &output,
        Some(&receipt_path),
        &options,
        &engine,
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
    let checked = compare::check(&output, true, &engine);
    assert_eq!(checked.status, Status::Pass, "{:?}", checked.diagnostics);
    assert_eq!(
        checked.comparison.as_ref().unwrap().receipt.path,
        receipt_path
    );
    let rejected = compare::record_review(&output, &base, &engine);
    assert_eq!(rejected.status, Status::Failed);
    assert!(rejected
        .diagnostics
        .iter()
        .any(|d| d.code == "compare/summary-required"));
    assert_eq!(fs::read(&output).unwrap(), original_html);
    assert_eq!(fs::read(&receipt_path).unwrap(), original_receipt);
    let mut changed = document();
    changed["components"][0]["label"] = json!("Changed source");
    fs::write(&head, serde_json::to_vec(&changed).unwrap()).unwrap();
    let stale = compare::check(&output, true, &engine);
    assert_ne!(stale.status, Status::Pass);
    assert_eq!(stale.exit_code(), 3);
}
#[test]
fn invalid_side_and_missing_stable_connection_id_preserve_the_existing_pair() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let receipt_path = root.path("comparison.html.compare.json");
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets::builtin();
    fs::write(&output, b"old html").unwrap();
    fs::write(&receipt_path, b"old receipt").unwrap();
    fs::write(&head, b"{partial").unwrap();
    let failed = compare::run(&request(
        &base, &head, &output, None, &options, &engine, &assets,
    ));
    assert_eq!(failed.status, Status::Failed);
    assert_eq!(fs::read(&output).unwrap(), b"old html");
    assert_eq!(fs::read(&receipt_path).unwrap(), b"old receipt");
    let mut no_id = document();
    no_id["connections"][0]
        .as_object_mut()
        .unwrap()
        .remove("id");
    fs::write(&head, serde_json::to_vec(&no_id).unwrap()).unwrap();
    let failed = compare::run(&request(
        &base, &head, &output, None, &options, &engine, &assets,
    ));
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
fn the_two_inputs_and_both_outputs_reject_aliases_and_cross_parent_receipts() {
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets::builtin();
    let original = fs::read(&base).unwrap();
    for r in [
        request(&base, &base, &output, None, &options, &engine, &assets),
        request(&base, &head, &base, None, &options, &engine, &assets),
        request(
            &base,
            &head,
            &output,
            Some(&output),
            &options,
            &engine,
            &assets,
        ),
    ] {
        assert_eq!(compare::run(&r).exit_code(), 3);
    }
    let other = root.path("other");
    fs::create_dir(&other).unwrap();
    let target = other.join("receipt.json");
    assert_eq!(
        compare::run(&request(
            &base,
            &head,
            &output,
            Some(&target),
            &options,
            &engine,
            &assets
        ))
        .exit_code(),
        3
    );
    assert_eq!(fs::read(&base).unwrap(), original);
}

use phoenix_diagram::browser::BrowserOptions;
#[path = "support/comparison_browser_fixture.rs"]
mod browser_fixture;
#[test]
fn protocol_evidence_drives_an_independent_summary_and_sealed_twelve_image_review() {
    use phoenix_diagram::compare::{
        ComparisonBrowserReceipt, ComparisonObservedScreenshot, ComparisonSummary,
        ComparisonVisualReviewRecord,
    };
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("comparison.html");
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets::builtin();
    let comparison = compare::run(&request(
        &base, &head, &output, None, &options, &engine, &assets,
    ));
    assert_eq!(
        comparison.status,
        Status::Pass,
        "{:?}",
        comparison.diagnostics
    );
    let immutable_html = fs::read(&output).unwrap();
    let immutable_receipt = fs::read(root.path("comparison.html.compare.json")).unwrap();
    let fixture = browser_fixture::Fixture::new();
    let browser_options = fixture.options(&root.path("evidence"), true);
    let (response, exit) = compare::browser_check(&output, &engine, &browser_options, true);
    assert_eq!(exit, 0, "{response:#}");
    let summary_path = PathBuf::from(
        response["evidence"]["summaryReceipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let summary: ComparisonSummary =
        serde_json::from_slice(&fs::read(&summary_path).unwrap()).unwrap();
    assert_eq!(summary.automated_status, Status::Pass);
    assert_eq!(summary.visual_review.status, VisualStatus::RequiredPending);
    assert_eq!(summary.delivery_status, "incomplete");
    assert_eq!(summary.stages.len(), 2);
    let browser_path = PathBuf::from(
        response["evidence"]["browserReceipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let browser: ComparisonBrowserReceipt =
        serde_json::from_slice(&fs::read(&browser_path).unwrap()).unwrap();
    assert_eq!(browser.screenshots.len(), 12);
    assert!(phoenix_diagram::browser::comparison::validate_receipt_coverage(&browser, true));
    let mut record = ComparisonVisualReviewRecord {
        schema_version: 1,
        kind: "architecture-comparison-review".into(),
        command: "review-record".into(),
        comparison: summary.comparison.clone(),
        browser_receipt: serde_json::from_value(response["evidence"]["browserReceipt"].clone())
            .unwrap(),
        screenshot_identities: browser
            .screenshots
            .iter()
            .map(|s| ComparisonObservedScreenshot {
                view_id: s.view_id.clone(),
                file: phoenix_diagram::browser::BoundFile {
                    path: s.screenshot.path.clone(),
                    identity: s.screenshot.identity.clone(),
                },
                viewport: s.screenshot.viewport,
                theme: s.screenshot.theme,
                observed: true,
                observations:
                    "Synthetic protocol fixture observations; this is not real image acceptance"
                        .into(),
            })
            .collect(),
        reviewer: "synthetic state-machine test".into(),
        scope: [
            "diagram",
            "labels",
            "cards",
            "relationship-occlusion",
            "hierarchy",
            "controls",
        ]
        .map(str::to_owned)
        .into(),
        verdict: VisualStatus::Passed,
        findings: vec![],
    };
    let draft = root.path("review.json");
    let full = record.screenshot_identities.clone();
    record.screenshot_identities.pop();
    fs::write(&draft, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    let missing = compare::record_review(&output, &draft, &engine);
    assert_eq!(missing.status, Status::Failed);
    assert_eq!(missing.delivery_status, "incomplete");
    record.screenshot_identities = full;
    let mut json_record = serde_json::to_value(&record).unwrap();
    json_record["candidate"] = json!({});
    fs::write(&draft, serde_json::to_vec(&json_record).unwrap()).unwrap();
    assert_eq!(
        compare::record_review(&output, &draft, &engine).status,
        Status::Failed
    );
    fs::write(&draft, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    let completed = compare::record_review(&output, &draft, &engine);
    assert_eq!(
        completed.status,
        Status::Pass,
        "{:?}",
        completed.diagnostics
    );
    assert_eq!(completed.delivery_status, "complete");
    let repeated = compare::record_review(&output, &draft, &engine);
    assert_eq!(repeated.status, Status::Pass, "{:?}", repeated.diagnostics);
    assert_eq!(
        repeated.visual_review.record,
        completed.visual_review.record
    );
    record.screenshot_identities[0]
        .observations
        .push_str(" Different bytes");
    fs::write(&draft, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    let conflict = compare::record_review(&output, &draft, &engine);
    assert_eq!(conflict.exit_code(), 3);
    assert!(conflict
        .diagnostics
        .iter()
        .any(|d| d.code == "compare/review-conflict"));
    assert_eq!(fs::read(&output).unwrap(), immutable_html);
    assert_eq!(
        fs::read(root.path("comparison.html.compare.json")).unwrap(),
        immutable_receipt
    );
    assert!(!fixture.commands().is_empty());
}

#[test]
fn public_cli_compares_checks_and_reports_its_custom_receipt_without_finalizing() {
    use std::process::Command;
    let root = Workspace::new();
    let (base, head) = sources(&root);
    let output = root.path("cli-comparison.html");
    let receipt = root.path("cli-receipt.json");
    let compared = Command::new(env!("CARGO_BIN_EXE_phoenix-diagram"))
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
    let checked = Command::new(env!("CARGO_BIN_EXE_phoenix-diagram"))
        .arg("check")
        .arg(&output)
        .args(["--strict-provenance", "--json"])
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );
    let checked: Value = serde_json::from_slice(&checked.stdout).unwrap();
    assert_eq!(checked["kind"], "architecture-comparison-check");
    assert_eq!(
        checked["comparison"]["receipt"]["path"],
        receipt.to_str().unwrap()
    );
    assert!(!root
        .path("cli-comparison.html.compare-summary.json")
        .exists());
}
