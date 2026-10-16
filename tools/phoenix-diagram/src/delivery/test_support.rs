//! Transaction fixtures only; they provide no real-browser or visual acceptance.
use super::*;
use crate::check::FileIdentity;
use crate::input::sha256;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[allow(dead_code)]
#[path = "../../tests/support/browser_protocol.rs"]
pub mod browser_protocol;
use browser_protocol::{launcher, Mode, ProtocolFixture};

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><meta charset="utf-8"><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;
const ASSETS: EmbeddedAssets<'static> = EmbeddedAssets { template: TEMPLATE };
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub root: PathBuf,
    pub candidate: PathBuf,
    pub output: PathBuf,
    pub engine: EngineIdentity,
    options: CompileOptions,
}

impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "phoenix-review-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let candidate = root.join("candidate.json");
        let output = root.join("diagram.html");
        fs::write(&candidate,serde_json::to_vec_pretty(&json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Transaction fixture","output":"diagram.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"components":[{"id":"service","type":"backend","label":"Service","pos":[120,120]}],"connections":[]})).unwrap()).unwrap();
        let engine = EngineIdentity {
            source_input_sha256: sha256(b"transaction test source identity"),
            input_contract_schema: 1,
            receipt_schema: 1,
            viewer_revision: "phoenix-v1".into(),
            binary: FileIdentity::of(&fs::read(std::env::current_exe().unwrap()).unwrap()),
        };
        Self {
            root,
            candidate,
            output,
            engine,
            options: CompileOptions::default(),
        }
    }
    pub fn request(&self) -> CompileRequest<'_> {
        CompileRequest {
            kind: DiagramType::Architecture,
            candidate: &self.candidate,
            options: &self.options,
            engine: &self.engine,
            assets: &ASSETS,
        }
    }
    pub fn finalize(&self) -> (FinalizeReceipt, ProtocolFixture) {
        let protocol = ProtocolFixture::new(Mode::Complete {
            remote: false,
            change_artifact: None,
            long: false,
        });
        let executable = launcher(&self.root, Some(&protocol.endpoint));
        let options = BrowserOptions {
            executable: Some(executable),
            ..Default::default()
        };
        let summary = super::finalize(&self.request(), &self.output, &options);
        assert_eq!(
            summary.status,
            Status::Pass,
            "Protocol fixture must cover the receipt contract: {:?}",
            summary.diagnostics
        );
        assert_eq!(summary.automated_status, Status::Pass);
        assert!(summary
            .stages
            .iter()
            .all(|stage| stage.status == Status::Pass));
        (summary, protocol)
    }
    pub fn record(&self, summary: &FinalizeReceipt) -> PathBuf {
        use crate::browser::BrowserReceipt;
        let browser: BrowserReceipt = serde_json::from_slice(
            &fs::read(summary.evidence["browserReceipt"]["path"].as_str().unwrap()).unwrap(),
        )
        .unwrap();
        let record=VisualReviewRecord {
            schema_version:1,tool:"phoenix-diagram".into(),tool_version:crate::TOOL_VERSION.into(),command:"review-record".into(),status:Status::Pass,diagnostics:vec![],
            artifact:summary.artifact.as_ref().unwrap().bound_file(),candidate:summary.candidate.as_ref().unwrap().bound_file(),config_sha256:summary.config_sha256.clone().unwrap(),engine:self.engine.clone(),
            delivery_receipt_id:summary.delivery_receipt_id.clone().unwrap(),browser_receipt_identity:summary.browser_receipt_identity.clone().unwrap(),
            screenshot_identities:browser.screenshots.iter().map(|shot|ObservedScreenshot{file:crate::browser::BoundFile{path:shot.path.clone(),identity:shot.identity.clone()},viewport:shot.viewport,theme:shot.theme,observed:true,observations:"Synthetic protocol observation for transaction testing; no real visual acceptance.".into()}).collect(),
            reviewer:"Protocol fixture; no actual image review".into(),scope:["diagram","labels","cards","relationship-occlusion","hierarchy","controls"].into_iter().map(str::to_owned).collect(),verdict:VisualStatus::Passed,findings:vec![],
        };
        let path = self.root.join("review-input.json");
        fs::write(&path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
