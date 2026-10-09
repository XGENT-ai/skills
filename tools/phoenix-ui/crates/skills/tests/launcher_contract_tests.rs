//! Phoenix shell shims forward to the same local Node launcher. These tests
//! do not run a provider, download an engine, or modify a real user profile.

use std::path::PathBuf;

fn launcher_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../skills/phoenix-ui/src/scripts")
}

#[test]
fn launchers_use_the_local_node_entry_without_acquisition() {
    for name in ["phoenix-ui", "phoenix-ui.cmd"] {
        let text = std::fs::read_to_string(launcher_dir().join(name)).unwrap();
        assert!(text.contains("phoenix-launcher.cjs"));
        for download in [
            "curl",
            "wget",
            "Invoke-WebRequest",
            "certutil",
            "releases/download",
            "npx",
            "npm install",
        ] {
            assert!(
                !text.contains(download),
                "{name} still acquires an engine: {download}"
            );
        }
    }
}

#[test]
fn cmd_launcher_keeps_exit_status_and_has_no_multiline_blocks() {
    let text = std::fs::read_to_string(launcher_dir().join("phoenix-ui.cmd")).unwrap();
    assert!(text.contains("%*"));
    assert!(text.contains("exit /b %errorlevel%"));
    for line in text.lines() {
        assert_ne!(line.trim(), "exit /b");
        assert!(!line.trim_end().ends_with('('));
    }
}

#[test]
fn node_entry_supplies_its_local_skill_directory_and_self() {
    let text = std::fs::read_to_string(launcher_dir().join("phoenix-launcher.cjs")).unwrap();
    assert!(text.contains("skillDir: path.resolve(__dirname, '..')"));
    assert!(text.contains("process.argv.slice(2)"));
    assert!(text.contains("shellQuote(__filename)"));
}

#[cfg(unix)]
#[test]
fn shell_launcher_preserves_arguments_and_child_exit_status() {
    let root = std::env::temp_dir().join(format!("phoenix-shell-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::copy(launcher_dir().join("phoenix-ui"), root.join("phoenix-ui")).unwrap();
    std::fs::write(
        root.join("phoenix-launcher.cjs"),
        "process.stdout.write(JSON.stringify(process.argv.slice(2))); process.exitCode = 23;\n",
    )
    .unwrap();
    let output = std::process::Command::new("sh")
        .arg(root.join("phoenix-ui"))
        .args(["context", "argument with spaces", "$(literal)", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23));
    let args: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        args,
        ["context", "argument with spaces", "$(literal)", "--json"]
    );
    std::fs::remove_dir_all(root).unwrap();
}
