use super::*;
use crate::{
    browser::BrowserOptions,
    check::FileIdentity,
    delivery::{CompileRequest, EngineIdentity, Status},
    input::{sha256, CompileOptions, DiagramType},
    render::EmbeddedAssets,
};
use serde_json::json;
use std::{fs, path::PathBuf};
#[path = "../../tests/support/comparison_browser_fixture.rs"]
mod fixture;
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn sealed_review_survives_summary_failure_and_only_same_bytes_can_resume() {
    let dir = std::env::temp_dir().join(format!("phoenix-compare-seal-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let project = Project(fs::canonicalize(&dir).unwrap());
    let base = project.0.join("base.json");
    let head = project.0.join("head.json");
    let output = project.0.join("comparison.html");
    let before = json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Service","output":"diagram.html","input_contract":"phoenix-v1","quality_profile":"standard","legend":{"mode":"hidden"}},"components":[{"id":"api","type":"backend","label":"API","pos":[100,100]},{"id":"db","type":"database","label":"Store","pos":[400,100]}],"connections":[{"id":"read","from":"api","to":"db","label":"Read"}]});
    let mut after = before.clone();
    after["components"][1]["pos"] = json!([400, 250]);
    after["components"][1]["label"] = json!("Records");
    fs::write(&base, serde_json::to_vec(&before).unwrap()).unwrap();
    fs::write(&head, serde_json::to_vec(&after).unwrap()).unwrap();
    let options = CompileOptions::default();
    let assets = EmbeddedAssets::builtin();
    let engine = EngineIdentity {
        source_input_sha256: sha256(b"compare review test"),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v2".into(),
        binary: FileIdentity::of(b"compare review test"),
    };
    let side = |candidate| CompileRequest {
        kind: DiagramType::Architecture,
        candidate,
        options: &options,
        engine: &engine,
        assets: &assets,
        repo_root: None,
        brand_cache: None,
    };
    let receipt = run(&CompareRequest {
        base: side(&base),
        head: side(&head),
        output: &output,
        receipt: None,
    });
    assert_eq!(receipt.status, Status::Pass, "{:?}", receipt.diagnostics);
    let fixture = fixture::Fixture::new();
    let options = fixture.options(&project.0.join("evidence"), true);
    let (response, exit) = browser_check(&output, &engine, &options, true);
    assert_eq!(exit, 0, "{response:#}");
    let summary_path = PathBuf::from(
        response["evidence"]["summaryReceipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let summary: ComparisonSummary =
        serde_json::from_slice(&fs::read(&summary_path).unwrap()).unwrap();
    let browser: ComparisonBrowserReceipt = serde_json::from_slice(
        &fs::read(
            response["evidence"]["browserReceipt"]["path"]
                .as_str()
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut record = json!({"schemaVersion":1,"kind":"architecture-comparison-review","command":"review-record","comparison":summary.comparison,"browserReceipt":response["evidence"]["browserReceipt"],"screenshotIdentities":browser.screenshots.iter().map(|s|json!({"viewId":s.view_id,"file":{"path":s.screenshot.path,"identity":s.screenshot.identity},"viewport":s.screenshot.viewport,"theme":s.screenshot.theme,"observed":true,"observations":"Synthetic protocol fixture only; not real image acceptance"})).collect::<Vec<_>>(),"reviewer":"synthetic state-machine test","scope":["diagram","labels","cards","relationship-occlusion","hierarchy","controls"],"verdict":"passed","findings":[]});
    let draft = project.0.join("review.json");
    fs::write(&draft, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    let failed = review::record_review_with_after_seal(&output, &draft, &engine, |_| {
        Err(std::io::Error::other("injected summary failure"))
    });
    assert_eq!(failed.exit_code(), 4);
    let sealed = PathBuf::from(
        failed.diagnostics[0].evidence["sealedReviewRecord"]["path"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(fs::read(&sealed).unwrap(), fs::read(&draft).unwrap());
    let still_pending: ComparisonSummary =
        serde_json::from_slice(&fs::read(&summary_path).unwrap()).unwrap();
    assert_eq!(still_pending.delivery_status, "incomplete");
    record["screenshotIdentities"][0]["observations"] = json!("Different synthetic bytes");
    let changed = project.0.join("changed.json");
    fs::write(&changed, serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(record_review(&output, &changed, &engine).exit_code(), 3);
    let resumed = record_review(&output, &draft, &engine);
    assert_eq!(resumed.status, Status::Pass, "{:?}", resumed.diagnostics);
    assert_eq!(resumed.delivery_status, "complete");
    assert_eq!(resumed.visual_review.record.unwrap().path, sealed);
    assert!(!fixture.commands().is_empty());
}
