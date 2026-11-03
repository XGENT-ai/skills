use phoenix_diagram::{
    browser::{self, BrowserStatus},
    check::FileIdentity,
};
use serde_json::json;

fn ir_identity() -> FileIdentity {
    FileIdentity::of(
        &serde_json::to_vec(
            &json!({"schemaVersion":1,"changes":{"components":[],"connections":[],"boundaries":[]}}),
        )
        .unwrap(),
    )
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
        Self(path.canonicalize().unwrap())
    }
    fn join(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
    fn artifact(&self) -> std::path::PathBuf {
        let path = self.join("comparison.html");
        std::fs::write(&path, b"synthetic parent fixture").unwrap();
        path
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
    let artifact = temp.artifact();
    let fixture = comparison_browser_fixture::Fixture::new();
    let receipt = browser::comparison::inspect(
        &artifact,
        &ir_identity(),
        &fixture.options(&temp.join("evidence"), true),
    );
    assert_eq!(
        receipt.status,
        BrowserStatus::Pass,
        "{:?}",
        receipt.diagnostics
    );
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
        assert_eq!(url.to_file_path().unwrap(), artifact);
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
    assert_eq!(html_files, vec![artifact]);
}
