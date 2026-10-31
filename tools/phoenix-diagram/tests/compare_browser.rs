use phoenix_diagram::{
    browser::{self, BrowserOptions, BrowserStatus},
    check::FileIdentity,
    compare::{ComparisonBinding, ComparisonBrowserReceipt, ComparisonSideBinding},
    delivery::EngineIdentity,
};
use serde_json::json;
use std::collections::BTreeMap;

fn identity() -> FileIdentity {
    FileIdentity::of(b"fixture")
}
fn binding() -> ComparisonBinding {
    let side = |path: &str| ComparisonSideBinding {
        candidate: browser::BoundFile {
            path: path.into(),
            identity: identity(),
        },
        semantic_sha256: "a".repeat(64),
        config_sha256: "b".repeat(64),
        engine: EngineIdentity {
            source_input_sha256: "c".repeat(64),
            input_contract_schema: 1,
            receipt_schema: 1,
            viewer_revision: "phoenix-v2".into(),
            binary: identity(),
        },
        resources: BTreeMap::new(),
    };
    ComparisonBinding {
        receipt: browser::BoundFile {
            path: "/fixture/comparison.html.compare.json".into(),
            identity: identity(),
        },
        receipt_id: "comparison-fixture".into(),
        base: side("/fixture/base.json"),
        head: side("/fixture/head.json"),
        comparison_ir: FileIdentity::of(&serde_json::to_vec(&json!({"schemaVersion":1,"changes":{"components":[],"connections":[],"boundaries":[]}})).unwrap()),
        artifact: browser::BoundFile {
            path: "/fixture/comparison.html".into(),
            identity: identity(),
        },
    }
}
fn empty_receipt() -> ComparisonBrowserReceipt {
    ComparisonBrowserReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: phoenix_diagram::TOOL_VERSION.into(),
        command: "visual-check".into(),
        kind: "architecture-comparison-browser".into(),
        status: BrowserStatus::Pass,
        diagnostics: vec![],
        comparison: binding(),
        browser_version: Some(json!({"product":"Protocol fixture; not a real browser"})),
        coverage: vec![],
        checks: vec![],
        screenshots: vec![],
        evidence: json!({}),
    }
}
#[test]
fn a_parent_only_or_single_view_pass_cannot_claim_comparison_coverage() {
    assert!(!browser::comparison::validate_receipt_coverage(
        &empty_receipt(),
        false
    ));
    assert!(!browser::comparison::validate_receipt_coverage(
        &empty_receipt(),
        true
    ));
}

#[path = "support/comparison_browser_fixture.rs"]
mod comparison_browser_fixture;

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-compare-browser-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn join(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
    fn binding(&self) -> ComparisonBinding {
        let mut value = binding();
        for (file, name, bytes) in [
            (
                &mut value.artifact,
                "comparison.html",
                b"synthetic parent fixture".as_slice(),
            ),
            (
                &mut value.receipt,
                "comparison.html.compare.json",
                b"synthetic immutable compare receipt".as_slice(),
            ),
            (
                &mut value.base.candidate,
                "base.json",
                b"synthetic base input".as_slice(),
            ),
            (
                &mut value.head.candidate,
                "head.json",
                b"synthetic head input".as_slice(),
            ),
        ] {
            file.path = self.join(name);
            std::fs::write(&file.path, bytes).unwrap();
            file.identity = FileIdentity::of(bytes);
        }
        value
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn protocol_measurements_cover_each_view_on_the_same_bound_parent_file() {
    // This tests protocol/receipt dispatch using synthetic DOM facts and blank
    // fixture PNGs. It does not claim that Chrome rendered or reviewed the page.
    let temp = Temp::new();
    let binding = temp.binding();
    let fixture = comparison_browser_fixture::Fixture::new();
    let receipt =
        browser::comparison::inspect(&binding, &fixture.options(&temp.join("evidence"), true));
    assert_eq!(
        receipt.status,
        BrowserStatus::Pass,
        "{:?}",
        receipt.diagnostics
    );
    assert!(browser::comparison::validate_receipt_coverage(
        &receipt, true
    ));
    assert_eq!(receipt.coverage.len(), 30);
    assert_eq!(receipt.screenshots.len(), 12);
    for view in ["before", "delta", "after"] {
        assert_eq!(
            receipt
                .coverage
                .iter()
                .filter(|entry| entry.view_id == view)
                .count(),
            10
        );
        assert_eq!(
            receipt
                .screenshots
                .iter()
                .filter(|entry| entry.view_id == view)
                .count(),
            4
        );
    }
    let commands = fixture.commands();
    let probes: Vec<_> = commands
        .iter()
        .filter(|command| {
            command["method"] == "Runtime.evaluate"
                && command["params"]["expression"]
                    .as_str()
                    .is_some_and(|expression| expression.contains("var viewId = "))
        })
        .collect();
    assert_eq!(probes.len(), 30);
    for view in ["before", "delta", "after"] {
        let declaration = format!("var viewId = \"{view}\";");
        assert_eq!(
            probes
                .iter()
                .filter(|command| command["params"]["expression"]
                    .as_str()
                    .unwrap()
                    .contains(&declaration))
                .count(),
            10,
            "{view}"
        );
    }
    let navigations: Vec<_> = commands
        .iter()
        .filter(|command| command["method"] == "Page.navigate")
        .collect();
    assert_eq!(navigations.len(), 30);
    for command in navigations {
        let url = url::Url::parse(command["params"]["url"].as_str().unwrap()).unwrap();
        assert_eq!(url.to_file_path().unwrap(), binding.artifact.path);
    }
    for screenshot in &receipt.screenshots {
        assert_eq!(
            FileIdentity::of(&std::fs::read(&screenshot.screenshot.path).unwrap()),
            screenshot.screenshot.identity
        );
    }
    let html_files: Vec<_> = std::fs::read_dir(&temp.0)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "html")
        })
        .collect();
    assert_eq!(html_files, vec![binding.artifact.path]);
}

#[test]
fn changed_missing_or_aliased_comparison_sources_fail_before_browser_discovery() {
    for source in ["artifact", "receipt", "base", "head", "alias"] {
        for missing in [false, true] {
            let temp = Temp::new();
            let mut binding = temp.binding();
            if source == "alias" {
                binding.head.candidate = binding.base.candidate.clone();
            } else {
                let target = match source {
                    "artifact" => &binding.artifact.path,
                    "receipt" => &binding.receipt.path,
                    "base" => &binding.base.candidate.path,
                    _ => &binding.head.candidate.path,
                };
                if missing {
                    std::fs::remove_file(target).unwrap();
                } else {
                    std::fs::write(target, b"later replacement").unwrap();
                }
            }
            let receipt = browser::comparison::inspect(
                &binding,
                &BrowserOptions {
                    executable: Some(temp.join("missing-browser")),
                    ..Default::default()
                },
            );
            assert_eq!(receipt.status, BrowserStatus::Failed);
            assert!(receipt.coverage.is_empty());
            assert!(receipt.screenshots.is_empty());
            assert_eq!(
                receipt.diagnostics[0].code,
                match source {
                    "artifact" => "browser/artifact-changed",
                    "receipt" => "browser/comparison-receipt-changed",
                    _ => "browser/candidate-changed",
                }
            );
            assert!(!receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "browser/configured-path"));
        }
    }
}
