use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-review-binary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("docs")).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
            .arg("review")
            .args(args)
            .arg("--json")
            .current_dir(&self.0)
            .output()
            .unwrap();
        let code = output.status.code().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let mut stream = serde_json::Deserializer::from_str(&stdout).into_iter::<Value>();
        let value = stream
            .next()
            .unwrap_or_else(|| panic!("exit {code}: no JSON for {args:?}; stderr: {stderr}"))
            .unwrap();
        assert!(value.is_object());
        assert!(stream.next().is_none(), "extra stdout: {stdout}");
        assert_eq!(value["ok"], code == 0, "{stdout}");
        if code == 0 {
            assert!(stderr.is_empty(), "{stderr}");
        } else {
            assert_eq!(value["exitCode"], code);
            assert!(!stderr.is_empty());
        }
        (code, value)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn root_help_lists_the_review_protocol() {
    let project = Project::new();
    for flag in ["--help", "-h"] {
        let output = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
            .arg(flag)
            .current_dir(&project.0)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("review <command>"), "{help}");
    }
    assert!(!project.0.join(".phoenix-ui").exists());
}

#[test]
fn review_argument_errors_keep_the_json_and_stderr_contract() {
    let project = Project::new();
    for args in [
        vec!["begin"],
        vec!["capture", "add"],
        vec!["judge-pack"],
        vec!["judge", "add"],
        vec!["finalize"],
        vec!["apply", "record"],
        vec!["status"],
        vec!["import"],
    ] {
        let (code, _) = project.run(&args);
        assert_eq!(code, 2, "{args:?}");
    }
    assert!(!project.0.join(".phoenix-ui").exists());
    assert_eq!(fs::read_dir(project.0.join("docs")).unwrap().count(), 0);
}

#[test]
fn review_status_is_read_only_when_no_cycle_exists() {
    let project = Project::new();
    let (code, state) = project.run(&["status", "--prefix", "docs/orders"]);
    assert_eq!(code, 0);
    assert_eq!(state["state"], "absent");
    assert!(state["verdict"].is_null());
    assert!(!project.0.join(".phoenix-ui").exists());
    assert_eq!(fs::read_dir(project.0.join("docs")).unwrap().count(), 0);
}

#[test]
fn review_begin_can_be_read_by_a_new_binary_process_without_implying_acceptance() {
    let project = Project::new();
    fs::write(project.0.join("DESIGN.md"), "Accepted icon size: 24px.\n").unwrap();
    fs::write(
        project.0.join("docs/orders.ui-target.md"),
        "# Orders\n\n## 单元\n\n| 单元 | 必需状态 | 视口 | 主题 | 来源 |\n| --- | --- | --- | --- | --- |\n| U1 Orders | default | 375×812 | light | [Design](../DESIGN.md) |\n\n## 规范要点\n\nAccepted icon size: 24px.\n\n## 采集\n\nOpen the orders screen.\n",
    )
    .unwrap();
    let (code, begun) = project.run(&[
        "begin",
        "--prefix",
        "docs/orders",
        "--target",
        "docs/orders.ui-target.md",
    ]);
    assert_eq!(code, 0);
    assert_eq!(begun["roundId"], "r1");
    assert_eq!(begun["testedVersionKnown"], false);
    let round = project.0.join(begun["round"].as_str().unwrap());
    let before = fs::read(&round).unwrap();
    let (code, state) = project.run(&["status", "--prefix", "docs/orders"]);
    assert_eq!(code, 0);
    assert_eq!(state["state"], "managed");
    assert_eq!(state["current"]["state"], "draft");
    assert!(state["verdict"].is_null());
    assert_eq!(fs::read(round).unwrap(), before);
}

#[test]
fn review_status_preserves_unsupported_state_and_exits_four() {
    let project = Project::new();
    let index = project.0.join("docs/orders.ui-review.json");
    let bytes = b"{\"schemaVersion\":999,\"history\":\"keep me\"}\n";
    fs::write(&index, bytes).unwrap();
    let (code, state) = project.run(&["status", "--prefix", "docs/orders"]);
    assert_eq!(code, 4);
    assert_eq!(state["state"], "unknown");
    assert!(state["verdict"].is_null());
    assert_eq!(fs::read(index).unwrap(), bytes);
}
