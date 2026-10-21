use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-diagram-cli-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn candidate(&self) -> PathBuf {
        let file = self.0.join("candidate.json");
        fs::write(&file,serde_json::to_vec(&json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"CLI facts","output":"diagram.html","input_contract":"phoenix-v1","quality_profile":"standard","legend":{"mode":"hidden"}},"components":[{"id":"a","type":"frontend","label":"Client","pos":[60,100]},{"id":"b","type":"backend","label":"Service","pos":[360,100]}],"connections":[{"from":"a","to":"b","label":"request"}]})).unwrap()).unwrap();
        file
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn native_engine() -> (Project, PathBuf) {
    let root = Project::new();
    let executable = root.0.join("phoenix-diagram");
    // Cargo can hardlink its public executable to the deps artifact. Keep the
    // runtime's single-link contract and execute a copy of the compiler bytes.
    fs::copy(env!("CARGO_BIN_EXE_phoenix-diagram"), &executable).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(fs::symlink_metadata(&executable).unwrap().nlink(), 1);
    (root, executable)
}
fn run(args: &[&std::ffi::OsStr]) -> Output {
    let (_engine_root, executable) = native_engine();
    Command::new(executable).args(args).output().unwrap()
}
fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
fn flag(value: &str) -> &std::ffi::OsStr {
    value.as_ref()
}

#[test]
fn cli_doctor_keeps_package_recovery_context_without_assuming_a_registry_release() {
    let (_engine_root, executable) = native_engine();
    let direct = Command::new(&executable)
        .args(["doctor", "--json"])
        .env_remove("PHOENIX_DIAGRAM_SETUP_COMMAND")
        .env(
            "PHOENIX_DIAGRAM_BROWSER",
            "/missing-phoenix-diagram-browser",
        )
        .output()
        .unwrap();
    let recovery = parsed(&direct)["recovery"]["engineInstall"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!recovery.contains("npm exec"), "{recovery}");
    assert!(recovery.contains(env!("PHOENIX_DIAGRAM_NPM_PACKAGE_VERSION")));
    let expected = "node '/candidate/bin/phoenix-diagram.js' engine install --project . --release-dir '/candidate/release'";
    let launched = Command::new(executable)
        .args(["doctor", "--json"])
        .env("PHOENIX_DIAGRAM_SETUP_COMMAND", expected)
        .env(
            "PHOENIX_DIAGRAM_BROWSER",
            "/missing-phoenix-diagram-browser",
        )
        .output()
        .unwrap();
    assert_eq!(parsed(&launched)["recovery"]["engineInstall"], expected);
}

#[test]
fn cli_help_and_engine_probe_work_without_writing_project_files() {
    let probe = run(&[flag("engine-probe")]);
    assert!(probe.status.success());
    assert_eq!(
        String::from_utf8(probe.stdout).unwrap(),
        "phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2\n"
    );
    let help = run(&[flag("--help")]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    for command in [
        "validate",
        "deliver",
        "check",
        "browser-check",
        "visual-check",
        "finalize",
        "review-record",
        "recover-output",
    ] {
        assert!(text.contains(command), "{command}");
    }
    assert!(text.starts_with("phoenix-diagram 0.2.0\n"));
    assert!(text.contains(
        "Types: architecture, workflow, sequence, dataflow, lifecycle, erd, class, tree, timeline, waterfall."
    ));
    let recovery = run(&[flag("recover-output"), flag("--help")]);
    assert!(recovery.status.success());
    let text = String::from_utf8(recovery.stdout).unwrap();
    assert!(text.contains("publication-recovery-v1.json") && text.contains("ls -la"));
}

#[test]
fn cli_guide_routes_the_ten_types_and_lists_alternatives_when_ambiguous() {
    let guide = |scenario: &str| {
        let output = run(&[flag("guide"), flag(scenario), flag("--json")]);
        assert!(output.status.success());
        parsed(&output)["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["type"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    for (scenario, kind) in [
        ("订单库的表结构与外键", "erd"),
        ("支付处理器的类图与接口实现", "class"),
        ("仓库目录结构", "tree"),
        ("事故复盘时间线", "timeline"),
        ("一次请求的耗时瀑布", "waterfall"),
        ("convert this erDiagram", "erd"),
        ("convert this classDiagram", "class"),
    ] {
        assert_eq!(guide(scenario), [kind], "{scenario}");
    }
    let output = run(&[flag("guide"), flag("类图"), flag("--json")]);
    assert_eq!(
        parsed(&output)["matches"][0]["schema"],
        "schemas/class.schema.json"
    );
    assert_eq!(guide("something vague").len(), 10);
}

#[test]
fn cli_validate_uses_the_builtin_reader_and_only_exposes_layout_on_request() {
    let project = Project::new();
    let candidate = project.candidate();
    let output = run(&[
        flag("validate"),
        flag("architecture"),
        candidate.as_os_str(),
        flag("--json"),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt = parsed(&output);
    assert_eq!(receipt["command"], "validate");
    assert_eq!(receipt["status"], "pass");
    assert!(receipt.get("layout").is_none());
    let with_layout = run(&[
        flag("validate"),
        flag("architecture"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert!(with_layout.status.success());
    assert!(parsed(&with_layout)["layout"].is_object());
    assert_eq!(fs::read_dir(&project.0).unwrap().count(), 1);
}

#[test]
fn cli_keeps_rejecting_a_hardlinked_engine_and_accepts_its_single_link_bytes() {
    let project = Project::new();
    let candidate = project.candidate();
    let (engine_root, executable) = native_engine();
    let alias = engine_root.0.join("cargo-alias");
    fs::hard_link(&executable, &alias).unwrap();
    let args = [
        flag("validate"),
        flag("architecture"),
        candidate.as_os_str(),
        flag("--json"),
    ];
    let rejected = Command::new(&executable).args(args).output().unwrap();
    assert_eq!(rejected.status.code(), Some(4));
    assert_eq!(
        parsed(&rejected)["diagnostics"][0]["code"],
        "engine/current-binary"
    );
    fs::remove_file(alias).unwrap();
    let accepted = Command::new(executable).args(args).output().unwrap();
    assert!(accepted.status.success(), "{:?}", parsed(&accepted));
}

#[test]
fn cli_delivery_and_strict_check_bind_the_original_source() {
    let project = Project::new();
    let candidate = project.candidate();
    let html = project.0.join("diagram.html");
    let delivery = run(&[
        flag("deliver"),
        flag("architecture"),
        candidate.as_os_str(),
        html.as_os_str(),
        flag("--json"),
    ]);
    assert!(
        delivery.status.success(),
        "{}",
        String::from_utf8_lossy(&delivery.stderr)
    );
    assert_eq!(parsed(&delivery)["commitState"], "committed");
    let check = run(&[
        flag("check"),
        html.as_os_str(),
        flag("--strict-provenance"),
        flag("--json"),
    ]);
    assert!(check.status.success(), "{:?}", parsed(&check));
    let changed = fs::read_to_string(&candidate)
        .unwrap()
        .replace("CLI facts", "New facts");
    fs::write(&candidate, changed).unwrap();
    let stale = run(&[
        flag("check"),
        html.as_os_str(),
        flag("--strict-provenance"),
        flag("--json"),
    ]);
    assert!(!stale.status.success());
    assert_eq!(parsed(&stale)["status"], "failed");
}

#[test]
fn cli_usage_errors_are_json_and_recovery_absence_is_idempotent() {
    for args in [
        vec![
            flag("validate"),
            flag("unknown"),
            flag("unused.json"),
            flag("--json"),
        ],
        vec![
            flag("check"),
            flag("unused.html"),
            flag("--unknown"),
            flag("--json"),
        ],
        vec![
            flag("validate"),
            flag("architecture"),
            flag("unused.json"),
            flag("--repo-root"),
            flag("/nonexistent/phoenix-repository"),
            flag("--json"),
        ],
        vec![
            flag("check"),
            flag("unused.html"),
            flag("--repo-root"),
            flag("."),
            flag("--json"),
        ],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(parsed(&output)["tool"], "phoenix-diagram");
        assert_eq!(
            parsed(&output)["diagnostics"][0]["code"],
            "arguments/invalid"
        );
    }
    let project = Project::new();
    let recovery = project
        .0
        .join("missing-parent/.phoenix-diagram-recovery-0123456789abcdef0123456789abcdef");
    let output = run(&[flag("recover-output"), recovery.as_os_str(), flag("--json")]);
    assert!(output.status.success());
    assert_eq!(parsed(&output)["recoveryStatus"], "absent");
    assert!(!project.0.join("missing-parent").exists());
}

#[test]
fn cli_finalize_reports_environment_failure_after_the_html_commit() {
    let project = Project::new();
    let candidate = project.candidate();
    let html = project.0.join("final.html");
    let (_engine_root, executable) = native_engine();
    let output = Command::new(executable)
        .args([
            flag("finalize"),
            flag("architecture"),
            candidate.as_os_str(),
            html.as_os_str(),
            flag("--json"),
        ])
        .env("PHOENIX_DIAGRAM_BROWSER", project.0.join("missing-browser"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(Path::new(&html).is_file());
    let receipt = parsed(&output);
    assert_eq!(receipt["deliveryStatus"], "incomplete");
    assert_eq!(receipt["stages"][3]["status"], "failed");
}
