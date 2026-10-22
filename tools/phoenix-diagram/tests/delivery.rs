use phoenix_diagram::{
    browser::BrowserOptions,
    check::FileIdentity,
    delivery::{self, CommitState, CompileRequest, EngineIdentity, Status, VisualStatus},
    input::{sha256, CompileOptions, DiagramType},
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
        Self(path)
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
fn engine() -> EngineIdentity {
    EngineIdentity {
        source_input_sha256: sha256(b"delivery test fixture source identity"),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v1".into(),
        binary: FileIdentity::of(&fs::read(std::env::current_exe().unwrap()).unwrap()),
    }
}
fn request<'a>(
    candidate: &'a Path,
    options: &'a CompileOptions,
    engine: &'a EngineIdentity,
    assets: &'a EmbeddedAssets<'a>,
) -> CompileRequest<'a> {
    CompileRequest {
        kind: DiagramType::Architecture,
        candidate,
        options,
        engine,
        assets,
    }
}

#[test]
fn validation_is_read_only_and_delivery_is_deterministic_with_distinct_run_identity() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, source()).unwrap();
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let request = request(&candidate, &options, &engine, &assets);
    let validated = delivery::validate(&request);
    assert_eq!(
        validated.status,
        Status::Pass,
        "{:?}",
        validated.diagnostics
    );
    assert_eq!(validated.exit_code(), 0);
    assert!(!validated.checks.is_empty());
    assert_eq!(fs::read(&candidate).unwrap(), source());
    assert_eq!(
        fs::read_dir(&temp.0).unwrap().count(),
        1,
        "validate must not write output or sidecars"
    );
    let first = delivery::deliver(&request, &temp.join("first.html"));
    let second = delivery::deliver(&request, &temp.join("second.html"));
    assert_eq!(first.status, Status::Pass, "{:?}", first.diagnostics);
    assert_eq!(second.status, Status::Pass, "{:?}", second.diagnostics);
    assert_eq!(first.commit_state, CommitState::Committed);
    assert_ne!(first.receipt_id, second.receipt_id);
    assert_eq!(
        fs::read(temp.join("first.html")).unwrap(),
        fs::read(temp.join("second.html")).unwrap()
    );
    assert_eq!(
        validated.artifact,
        first.artifact.as_ref().map(|file| file.identity.clone())
    );
    let checked = delivery::check(&temp.join("first.html"), true, &engine);
    assert_eq!(checked.status, Status::Pass, "{:?}", checked.diagnostics);
    assert_eq!(checked.provenance, "current");
    assert_eq!(checked.scope, "strict-provenance");
}

#[test]
fn strict_provenance_rejects_replaced_source_engine_and_missing_named_checks() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    let output = temp.join("report.html");
    fs::write(&candidate, source()).unwrap();
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let delivered = delivery::deliver(&request(&candidate, &options, &engine, &assets), &output);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    let mut another = engine.clone();
    another.source_input_sha256 = sha256(b"different source");
    assert_ne!(
        delivery::check(&output, true, &another).status,
        Status::Pass
    );
    let replacement = temp.join("replacement.json");
    fs::write(&replacement, source()).unwrap();
    fs::rename(&replacement, &candidate).unwrap();
    let changed = delivery::check(&output, true, &engine);
    assert_ne!(
        changed.status,
        Status::Pass,
        "same bytes do not restore the captured input inode"
    );
    assert!(changed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "provenance/candidate-binding"));
    let second = temp.join("second.html");
    let delivered = delivery::deliver(&request(&candidate, &options, &engine, &assets), &second);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    let sidecar = delivery::sidecar_paths(&second, None).delivery;
    let mut receipt: Value = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    receipt["checks"].as_array_mut().unwrap().pop();
    fs::write(&sidecar, serde_json::to_vec(&receipt).unwrap()).unwrap();
    let checked = delivery::check(&second, true, &engine);
    assert_ne!(checked.status, Status::Pass);
    assert!(checked
        .diagnostics
        .iter()
        .any(|d| d.code == "provenance/check-scope"));
    assert_eq!(
        delivery::check(&second, false, &engine).status,
        Status::Pass,
        "artifact-only checking states its weaker scope"
    );
}

#[test]
fn aliases_and_foreign_sidecars_are_preserved_before_html_commit() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, source()).unwrap();
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let req = request(&candidate, &options, &engine, &assets);
    let aliased = delivery::deliver(&req, &candidate);
    assert_eq!(aliased.exit_code(), 3);
    assert_eq!(fs::read(&candidate).unwrap(), source());
    let output = temp.join("report.html");
    fs::write(&output, b"old artifact").unwrap();
    let sidecar = delivery::sidecar_paths(&output, None).delivery;
    fs::write(&sidecar, b"foreign file").unwrap();
    let failed = delivery::deliver(&req, &output);
    assert_eq!(failed.exit_code(), 3);
    assert_eq!(failed.commit_state, CommitState::NotCommitted);
    assert_eq!(fs::read(&output).unwrap(), b"old artifact");
    assert_eq!(fs::read(&sidecar).unwrap(), b"foreign file");
    #[cfg(unix)]
    {
        let linked = temp.join("linked.html");
        std::os::unix::fs::symlink(&candidate, &linked).unwrap();
        assert_eq!(delivery::deliver(&req, &linked).exit_code(), 3);
        let hard = temp.join("hard.html");
        fs::hard_link(&candidate, &hard).unwrap();
        assert_eq!(delivery::deliver(&req, &hard).exit_code(), 3);
        fs::remove_file(&hard).unwrap();
    }
}

#[test]
fn finalize_short_circuits_and_identifies_an_old_artifact_without_claiming_it() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, b"{}").unwrap();
    let output = temp.join("report.html");
    fs::write(&output, b"previous artifact").unwrap();
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let receipt = delivery::finalize(
        &request(&candidate, &options, &engine, &assets),
        &output,
        &BrowserOptions::default(),
    );
    assert_eq!(
        receipt
            .stages
            .iter()
            .map(|stage| stage.name.as_str())
            .collect::<Vec<_>>(),
        ["validate", "deliver", "check", "browser-check"]
    );
    assert_eq!(receipt.stages[0].status, Status::Failed);
    assert!(receipt.stages[1..]
        .iter()
        .all(|stage| stage.status == Status::NotRun && stage.receipt.is_none()));
    assert!(!receipt.produced_by_this_run);
    assert_eq!(
        receipt.artifact.unwrap().identity,
        FileIdentity::of(b"previous artifact")
    );
    assert_eq!(receipt.visual_review.status, VisualStatus::RequiredPending);
    assert_eq!(receipt.delivery_status, "incomplete");
    assert_eq!(fs::read(&output).unwrap(), b"previous artifact");
    assert!(!delivery::sidecar_paths(&output, None).browser.exists());
}

#[test]
fn browser_failure_keeps_the_new_html_and_review_cannot_invent_a_pass() {
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    let output = temp.join("report.html");
    fs::write(&candidate, source()).unwrap();
    fs::write(&output, b"old artifact").unwrap();
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let browser = BrowserOptions {
        executable: Some(temp.join("missing-chrome")),
        ..BrowserOptions::default()
    };
    let receipt = delivery::finalize(
        &request(&candidate, &options, &engine, &assets),
        &output,
        &browser,
    );
    assert!(
        receipt.stages[..3]
            .iter()
            .all(|stage| stage.status == Status::Pass),
        "{:?}",
        receipt.diagnostics
    );
    assert_eq!(receipt.stages[3].status, Status::Failed);
    assert!(receipt.produced_by_this_run);
    assert_ne!(fs::read(&output).unwrap(), b"old artifact");
    assert_eq!(delivery::check(&output, true, &engine).status, Status::Pass);
    let record = temp.join("review.json");
    fs::write(&record, b"{}").unwrap();
    let reviewed = delivery::record_review(&output, &record, &engine);
    assert_ne!(reviewed.status, Status::Pass);
    assert_eq!(reviewed.delivery_status, "incomplete");
    assert_eq!(fs::read(&record).unwrap(), b"{}");
    assert!(!delivery::sidecar_paths(&output, None).review.exists());
}

#[test]
fn sidecar_names_are_bounded_and_have_one_shared_reserved_namespace() {
    for basename in [
        format!("{}.html", "a".repeat(248)),
        format!("{}.html", "图".repeat(82)),
        format!("x.~phoenix-diagram-{}.html", "f".repeat(64)),
    ] {
        let paths = delivery::sidecar_paths(Path::new(&basename), None);
        for path in [
            paths.delivery,
            paths.summary,
            paths.browser,
            paths.visual,
            paths.review,
        ] {
            let component = path.file_name().unwrap().to_str().unwrap();
            assert!(
                component.len() <= 255 && component.encode_utf16().count() <= 255,
                "{component}"
            );
            assert!(component.contains(".~phoenix-diagram-"));
            assert_eq!(component.matches(".~phoenix-diagram-").count(), 1);
        }
    }
}

#[test]
fn strict_config_cannot_claim_another_input_contract_for_the_existing_html() {
    use phoenix_diagram::input::{parse_input, InputContract};
    let temp = Temp::new();
    let candidate = temp.join("candidate.json");
    let output = temp.join("report.html");
    let mut document: Value = serde_json::from_slice(&source()).unwrap();
    document["meta"]
        .as_object_mut()
        .unwrap()
        .remove("input_contract");
    let bytes = serde_json::to_vec_pretty(&document).unwrap();
    fs::write(&candidate, &bytes).unwrap();
    let options = CompileOptions::default();
    let engine = engine();
    let assets = EmbeddedAssets { template: TEMPLATE };
    let delivered = delivery::deliver(&request(&candidate, &options, &engine, &assets), &output);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    let sidecar = delivery::sidecar_paths(&output, None).delivery;
    let mut receipt: Value = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    let another = parse_input(
        &bytes,
        DiagramType::Architecture,
        &CompileOptions {
            input_contract: Some(InputContract::PhoenixV1),
            ..CompileOptions::default()
        },
    )
    .unwrap();
    receipt["inputContract"] = json!("phoenix-v1");
    receipt["config"]["inputContract"] = json!("phoenix-v1");
    receipt["configSha256"] = json!(another.config_sha256);
    receipt["compatibility"] = json!(another.compatibility);
    fs::write(&sidecar, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let checked = delivery::check(&output, true, &engine);
    assert_eq!(checked.status,Status::Failed,"Existing HTML still declares legacy-2.17; changing only receipt config cannot bind a new contract");
    assert!(checked
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "provenance/config"));
}
