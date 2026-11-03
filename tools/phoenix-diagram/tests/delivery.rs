#[path = "support/browser_protocol.rs"]
#[allow(dead_code)]
mod browser_protocol;
use browser_protocol::{launcher, Mode, ProtocolFixture};
use phoenix_diagram::{
    browser::BrowserOptions,
    check::FileIdentity,
    delivery::{self, CompileRequest, FinalizeReceipt, Status, VisualStatus},
    input::{CompileOptions, DiagramType},
    render::EmbeddedAssets,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><meta charset="utf-8"><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phoenix-delivery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn source() -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "schema_version":1,"diagram_type":"architecture",
        "meta":{"title":"Delivery test","output":"report.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},
        "components":[{"id":"service","type":"backend","label":"Service","pos":[120,120]}],"connections":[]
    })).unwrap()
}
fn request<'a>(
    candidate: &'a Path,
    options: &'a CompileOptions,
    assets: &'a EmbeddedAssets<'a>,
) -> CompileRequest<'a> {
    CompileRequest {
        kind: DiagramType::Architecture,
        candidate,
        options,
        assets,
        repo_root: None,
        brand_cache: None,
    }
}
fn missing_browser(temp: &Temp) -> BrowserOptions {
    BrowserOptions {
        executable: Some(temp.join("missing-chrome")),
        ..BrowserOptions::default()
    }
}
fn summary(output: &Path) -> Value {
    serde_json::from_slice(&fs::read(delivery::finalize_summary_path(output)).unwrap()).unwrap()
}
fn json_of(receipt: &FinalizeReceipt) -> Value {
    serde_json::to_value(receipt).unwrap()
}

#[test]
fn validation_is_read_only_and_delivery_writes_the_checked_html() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, source()).unwrap();
    let options = CompileOptions::default();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let request = request(&candidate, &options, &assets);
    let validated = delivery::validate(&request);
    assert_eq!(
        validated.status,
        Status::Pass,
        "{:?}",
        validated.diagnostics
    );
    assert!(!validated.checks.is_empty());
    assert_eq!(
        fs::read_dir(&temp.0).unwrap().count(),
        1,
        "validate must not write output or sidecars"
    );
    let output = temp.join("report.html");
    fs::write(&output, b"old artifact").unwrap();
    let delivered = delivery::deliver(&request, &output);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    let html = fs::read(&output).unwrap();
    assert_eq!(
        delivered.artifact.as_ref().map(|file| &file.identity),
        Some(&FileIdentity::of(&html))
    );
    assert_eq!(validated.artifact, Some(FileIdentity::of(&html)));
    assert_eq!(
        fs::read_dir(&temp.0).unwrap().count(),
        2,
        "deliver writes only the HTML"
    );
    let checked = delivery::check(&output);
    assert_eq!(checked.status, Status::Pass, "{:?}", checked.diagnostics);
    assert_eq!(checked.checks, delivered.checks);
}

#[test]
fn delivery_refuses_to_overwrite_its_candidate() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, source()).unwrap();
    let options = CompileOptions::default();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let aliased = delivery::deliver(&request(&candidate, &options, &assets), &candidate);
    assert_eq!(aliased.exit_code(), 2);
    assert_eq!(fs::read(&candidate).unwrap(), source());
}

#[test]
fn finalize_stops_at_the_first_failed_stage_and_keeps_the_old_html() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, b"{}").unwrap();
    let output = temp.join("report.html");
    fs::write(&output, b"previous artifact").unwrap();
    let options = CompileOptions::default();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let receipt = delivery::finalize(
        &request(&candidate, &options, &assets),
        &output,
        &missing_browser(&temp),
    );
    assert_eq!(
        receipt
            .stages
            .iter()
            .map(|stage| stage.name.as_str())
            .collect::<Vec<_>>(),
        ["validate", "deliver", "browser-check"]
    );
    assert_eq!(receipt.stages[0].status, Status::Failed);
    assert!(receipt.stages[1..]
        .iter()
        .all(|stage| stage.status == Status::NotRun && stage.receipt.is_none()));
    assert!(receipt.artifact.is_none());
    assert_eq!(receipt.delivery_status, "incomplete");
    assert_eq!(fs::read(&output).unwrap(), b"previous artifact");
    assert_eq!(summary(&output), json_of(&receipt));
}

#[test]
fn browser_failure_keeps_the_new_html_and_review_cannot_invent_a_pass() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    let output = temp.join("report.html");
    fs::write(&candidate, source()).unwrap();
    fs::write(&output, b"old artifact").unwrap();
    let options = CompileOptions::default();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let receipt = delivery::finalize(
        &request(&candidate, &options, &assets),
        &output,
        &missing_browser(&temp),
    );
    assert!(
        receipt.stages[..2]
            .iter()
            .all(|stage| stage.status == Status::Pass),
        "{:?}",
        receipt.diagnostics
    );
    assert_eq!(receipt.stages[2].status, Status::Failed);
    assert_eq!(receipt.exit_code(), 4);
    assert!(receipt
        .diagnostics
        .iter()
        .any(|entry| entry.code == "browser/configured-path"));
    assert_eq!(
        receipt.locale,
        json!({"requested":null,"resolved":"en","fallback":false})
    );
    assert_eq!(
        receipt.evidence["status"], "none",
        "an uncited diagram reports no source evidence"
    );
    assert_ne!(fs::read(&output).unwrap(), b"old artifact");
    assert_eq!(summary(&output), json_of(&receipt));
    let record = temp.join("review.json");
    fs::write(&record, b"{}").unwrap();
    let reviewed = delivery::record_review(&output, &record);
    assert_ne!(reviewed.status, Status::Pass);
    assert_eq!(reviewed.delivery_status, "incomplete");
    assert_eq!(summary(&output), json_of(&receipt));
}

#[test]
fn a_recorded_review_completes_only_the_finalized_html() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    let output = temp.join("report.html");
    let evidence = temp.join("evidence");
    fs::write(&candidate, source()).unwrap();
    let protocol = ProtocolFixture::new(Mode::Complete {
        remote: false,
        long: false,
    });
    let options = CompileOptions::default();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let finalized = delivery::finalize(
        &request(&candidate, &options, &assets),
        &output,
        &BrowserOptions {
            executable: Some(launcher(&temp.0, Some(&protocol.endpoint))),
            out_dir: Some(evidence.clone()),
            ..BrowserOptions::default()
        },
    );
    assert_eq!(
        finalized.automated_status,
        Status::Pass,
        "{:?}",
        finalized.diagnostics
    );
    assert_eq!(
        finalized.visual_review.status,
        VisualStatus::RequiredPending
    );
    assert_eq!(finalized.screenshots.len(), 4);
    assert!(finalized
        .screenshots
        .iter()
        .all(|path| path.starts_with(&evidence) && path.is_file()));
    let write_record = |verdict: &str, observation: &str, findings: Value| {
        let path = temp.join("review.json");
        let screenshots: Vec<_> = finalized
            .screenshots
            .iter()
            .map(|path| json!({"path":path,"observations":observation}))
            .collect();
        fs::write(
            &path,
            serde_json::to_vec(&json!({"reviewer":"Protocol fixture; no actual image review","verdict":verdict,"screenshots":screenshots,"findings":findings})).unwrap(),
        )
        .unwrap();
        path
    };
    for (verdict, observation, findings) in [
        ("passed", "", json!([])),
        ("failed", "Labels overlap", json!([])),
        ("required-pending", "Seen", json!([])),
    ] {
        let rejected =
            delivery::record_review(&output, &write_record(verdict, observation, findings));
        assert_eq!(
            rejected.exit_code(),
            2,
            "{verdict}: {:?}",
            rejected.diagnostics
        );
    }
    let mut partial: Value =
        serde_json::from_slice(&fs::read(write_record("passed", "Seen", json!([]))).unwrap())
            .unwrap();
    partial["screenshots"].as_array_mut().unwrap().pop();
    let partial_path = temp.join("partial.json");
    fs::write(&partial_path, serde_json::to_vec(&partial).unwrap()).unwrap();
    assert_eq!(
        delivery::record_review(&output, &partial_path).exit_code(),
        2
    );
    assert_eq!(summary(&output), json_of(&finalized));

    let failed = delivery::record_review(
        &output,
        &write_record("failed", "Seen", json!(["Labels overlap"])),
    );
    assert_eq!(failed.status, Status::Pass, "{:?}", failed.diagnostics);
    assert_eq!(failed.visual_review.status, VisualStatus::Failed);
    assert_eq!(failed.delivery_status, "incomplete");
    let passed = delivery::record_review(
        &output,
        &write_record(
            "passed",
            "Diagram, labels and controls are readable",
            json!([]),
        ),
    );
    assert_eq!(passed.status, Status::Pass, "{:?}", passed.diagnostics);
    assert_eq!(passed.delivery_status, "complete");
    let completed = summary(&output);
    assert_eq!(completed["visualReview"], json!(passed.visual_review));
    assert_eq!(completed["deliveryStatus"], "complete");

    fs::write(&output, b"edited after finalize").unwrap();
    let stale = delivery::record_review(
        &output,
        &write_record(
            "passed",
            "Diagram, labels and controls are readable",
            json!([]),
        ),
    );
    assert!(stale
        .diagnostics
        .iter()
        .any(|d| d.code == "review/artifact-changed"));
}

#[test]
fn resolved_interface_language_drives_lang_and_the_finalize_receipt() {
    let temp = Temp::new();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let browser = missing_browser(&temp);
    for (index, (authored, resolved, fallback)) in [
        ("zh-cn", "zh-CN", false),
        ("fr", "en", true),
        ("zh-tw", "zh-TW", false),
        ("es-MX", "en", true),
    ]
    .into_iter()
    .enumerate()
    {
        let mut document: Value = serde_json::from_slice(&source()).unwrap();
        document["meta"]["locale"] = json!(authored);
        let candidate = temp.join(&format!("candidate-{index}.json"));
        fs::write(&candidate, serde_json::to_vec(&document).unwrap()).unwrap();
        let output = temp.join(&format!("report-{index}.html"));
        let options = CompileOptions::default();
        let receipt =
            delivery::finalize(&request(&candidate, &options, &assets), &output, &browser);
        assert_eq!(
            receipt.stages[1].status,
            Status::Pass,
            "{:?}",
            receipt.diagnostics
        );
        assert_eq!(
            receipt.locale,
            json!({"requested":authored,"resolved":resolved,"fallback":fallback})
        );
        let html = fs::read_to_string(&output).unwrap();
        assert!(html.starts_with(&format!("<!doctype html><html lang=\"{resolved}\"")));
        assert!(html.contains(&format!(" lang=\"{resolved}\" data-diagram-type=")));
    }
}
