// Installer acceptance uses real temporary filesystems and the actual Cargo
// engine binary. Provider directory tests do not prove host loading (V-13).
use phoenix_diagram::install::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static NEXT: AtomicU64 = AtomicU64::new(0);
static ENGINE_DIGEST: OnceLock<(u64, String)> = OnceLock::new();
const SKILL: &str = ".agents/skills/phoenix-diagram";
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phoenix-diagram-install-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path.canonicalize().unwrap())
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
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write(path: &Path, bytes: &[u8], mode: u32) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
fn snapshot(root: &Path) -> BTreeMap<String, (String, u32)> {
    fn collect(root: &Path, directory: &Path, result: &mut BTreeMap<String, (String, u32)>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let contents = if meta.is_symlink() {
                format!("symlink:{}", fs::read_link(&path).unwrap().display())
            } else if meta.is_dir() {
                "directory".into()
            } else {
                digest(&fs::read(&path).unwrap())
            };
            result.insert(relative, (contents, meta.permissions().mode() & 0o7777));
            if meta.is_dir() {
                collect(root, &path, result);
            }
        }
    }
    let mut result = BTreeMap::new();
    collect(root, root, &mut result);
    result
}
fn platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "x86_64") => "linux-x64",
        other => panic!("unsupported native test platform: {other:?}"),
    }
}
fn fixture(root: &Workspace) -> InstallRequest {
    fs::create_dir(root.path("project")).unwrap();
    let bundle = root.path("bundle");
    fs::create_dir(&bundle).unwrap();
    let sources: &[(&str, &[u8], u32)] = &[
        ("scripts/phoenix-diagram", b"#!/bin/sh\nexit 0\n", 0o755),
        (
            "SKILL.md",
            b"---\nname: phoenix-diagram\ndescription: diagram\n---\n",
            0o644,
        ),
        ("schemas/architecture.schema.json", b"{}\n", 0o644),
        ("schemas/README.md", b"Schemas\n", 0o644),
        ("examples/sample.architecture.json", b"{}\n", 0o644),
        ("references/input.md", b"Reference\n", 0o644),
        (
            "scripts/phoenix-launcher.cjs",
            b"#!/usr/bin/env node\n",
            0o755,
        ),
        ("scripts/phoenix-bootstrap.cjs", b"'use strict';\n", 0o644),
        (
            "agents/openai.yaml",
            b"interface:\n  display_name: Phoenix\n",
            0o644,
        ),
    ];
    let files: Vec<_> = sources
        .iter()
        .map(|(path, bytes, mode)| {
            write(&bundle.join(path), bytes, *mode);
            json!({"path":path,"size":bytes.len(),"sha256":digest(bytes),"mode":mode})
        })
        .collect();
    let manifest = serde_json::to_vec(&json!({
        "schemaVersion":1,"tool":"phoenix-diagram","version":"0.2.0","files":files
    }))
    .unwrap();
    write(&bundle.join("manifest.json"), &manifest, 0o644);
    let compiled_path = std::env::var_os("PHOENIX_DIAGRAM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_phoenix-diagram")));
    let compiled = compiled_path.as_path();
    let (engine_size, engine_sha256) = ENGINE_DIGEST.get_or_init(|| {
        let bytes = fs::read(compiled).unwrap();
        (bytes.len() as u64, digest(&bytes))
    });
    // Cargo artifacts may have multiple links; probe only an owned byte copy.
    // A child process writes it: a write descriptor held by this process could
    // be inherited by a parallel test's fork and fail the exec with ETXTBSY.
    let engine = root.path("phoenix-diagram");
    let copied = Command::new("/bin/cp")
        .arg(compiled)
        .arg(&engine)
        .status()
        .unwrap();
    assert!(copied.success());
    assert_eq!(fs::metadata(&engine).unwrap().len(), *engine_size);
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o700)).unwrap();
    let metadata = fs::symlink_metadata(&engine).unwrap();
    assert!(metadata.is_file() && !metadata.is_symlink());
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(metadata.uid(), fs::metadata(&root.0).unwrap().uid());
    assert_eq!(digest(&fs::read(&engine).unwrap()), *engine_sha256);
    let version = serde_json::to_vec(&json!({
        "schemaVersion":1,"distribution":"development","tool":"phoenix-diagram",
        "toolVersion":"0.2.0","npmPackageVersion":"0.0.0-fixture","sourceCommit":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sourceInputSha256":env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256"),
        "inputContractSchema":1,"receiptSchema":1,"viewerRevision":"phoenix-v2",
        "bundleSha256":digest(&manifest),
        "engines":{platform():{"asset":format!("phoenix-diagram-{}",platform()),"size":engine_size,"sha256":engine_sha256}}
    }))
    .unwrap();
    write(&bundle.join("ENGINE.json"), &version, 0o644);
    InstallRequest {
        project: root.path("project"),
        bundle_root: bundle,
        engine,
        providers: Some(vec!["codex".into()]),
    }
}
fn load(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn change_version(request: &InstallRequest, update: impl FnOnce(&mut Value)) {
    let path = request.bundle_root.join("ENGINE.json");
    let mut version = load(&path);
    update(&mut version);
    write(&path, &serde_json::to_vec(&version).unwrap(), 0o644);
}
fn change_manifest(request: &InstallRequest, update: impl FnOnce(&mut Value)) {
    let path = request.bundle_root.join("manifest.json");
    let mut manifest = load(&path);
    update(&mut manifest);
    let bytes = serde_json::to_vec(&manifest).unwrap();
    write(&path, &bytes, 0o644);
    change_version(request, |version| {
        version["bundleSha256"] = json!(digest(&bytes))
    });
}
#[test]
fn install_dry_run_has_no_project_writes_and_uses_real_engine() {
    let root = Workspace::new();
    let request = fixture(&root);
    let before = snapshot(&request.project);
    let receipt = plan_install(&request).unwrap();
    assert_eq!(receipt.status, InstallStatus::Planned);
    assert!(receipt
        .writes
        .iter()
        .any(|write| write.path == format!("{SKILL}/SKILL.md")));
    assert!(receipt.providers.contains(&".agents".into()));
    assert!(receipt.providers.contains(&".codex".into()));
    assert!(receipt
        .writes
        .iter()
        .all(|write| !write.path.starts_with(".codex/")));
    assert_eq!(snapshot(&request.project), before);
}

#[test]
fn install_replaces_a_symlinked_skill_folder_without_touching_its_target() {
    let root = Workspace::new();
    let request = fixture(&root);
    let target = root.path("linked skill");
    write(&target.join("SKILL.md"), b"linked source", 0o644);
    let link = request.project.join(SKILL);
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
    assert!(!fs::symlink_metadata(&link).unwrap().is_symlink());
    assert_eq!(
        fs::read(link.join("SKILL.md")).unwrap(),
        fs::read(request.bundle_root.join("SKILL.md")).unwrap()
    );
    assert_eq!(fs::read(target.join("SKILL.md")).unwrap(), b"linked source");
}

#[test]
fn install_apply_overwrites_tool_files_without_history() {
    let root = Workspace::new();
    let request = fixture(&root);
    write(
        &request.project.join(".phoenix-ui/install/active.json"),
        b"user UI state\n",
        0o600,
    );
    write(
        &request.project.join(".git/info/exclude"),
        b"user excludes\n",
        0o644,
    );
    write(
        &request.project.join(".codex/settings.json"),
        b"user settings\n",
        0o644,
    );
    let receipt = apply_install(&request).unwrap();
    assert_eq!(receipt.status, InstallStatus::Installed);
    assert_eq!(
        fs::read(request.project.join(format!("{SKILL}/ENGINE.json"))).unwrap(),
        fs::read(request.bundle_root.join("ENGINE.json")).unwrap()
    );
    assert_eq!(
        fs::metadata(
            request
                .project
                .join(format!("{SKILL}/scripts/phoenix-launcher.cjs"))
        )
        .unwrap()
        .permissions()
        .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        fs::read(request.project.join(".phoenix-ui/install/active.json")).unwrap(),
        b"user UI state\n"
    );
    assert_eq!(
        fs::read(request.project.join(".git/info/exclude")).unwrap(),
        b"user excludes\n"
    );
    assert_eq!(
        fs::read(request.project.join(".codex/settings.json")).unwrap(),
        b"user settings\n"
    );
    assert!(!request.project.join(".codex/skills").exists());
    assert!(!request.project.join(".codex/hooks.json").exists());
    assert!(!request.project.join(".phoenix-diagram/install").exists());
    write(
        &request.project.join(format!("{SKILL}/SKILL.md")),
        b"custom tool copy",
        0o644,
    );
    write(
        &request.project.join(format!("{SKILL}/extra.txt")),
        b"extra",
        0o644,
    );
    write(
        &request
            .project
            .join(".phoenix-diagram/install/transactions/old/backup"),
        b"old",
        0o600,
    );
    write(
        &request.project.join(".phoenix-diagram/output/user.json"),
        b"business data",
        0o644,
    );
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
    assert_eq!(
        fs::read(request.project.join(format!("{SKILL}/SKILL.md"))).unwrap(),
        fs::read(request.bundle_root.join("SKILL.md")).unwrap()
    );
    assert!(!request.project.join(".phoenix-diagram/install").exists());
    assert_eq!(
        fs::read(request.project.join(".phoenix-diagram/output/user.json")).unwrap(),
        b"business data"
    );
    assert!(!request.project.join(format!("{SKILL}/extra.txt")).exists());
    let before = snapshot(&request.project);
    assert_eq!(
        plan_install(&request).unwrap().status,
        InstallStatus::Unchanged
    );
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Unchanged
    );
    assert_eq!(snapshot(&request.project), before);
}

#[test]
fn install_rejects_bundle_tampering_before_any_project_write() {
    for problem in ["bytes", "mode", "engine", "probe"] {
        let root = Workspace::new();
        let mut request = fixture(&root);
        match problem {
            "bytes" => write(&request.bundle_root.join("SKILL.md"), b"changed", 0o644),
            "mode" => fs::set_permissions(
                request.bundle_root.join("SKILL.md"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap(),
            "engine" => write(&request.bundle_root.join("ENGINE.json"), b"{}", 0o644),
            "probe" => {
                let fake = root.path("fake-engine");
                write(&fake, b"#!/bin/sh\nprintf 'phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2\\n'\n", 0o755);
                request.engine = fake;
            }
            _ => unreachable!(),
        }
        let before = snapshot(&request.project);
        let error = apply_install(&request).unwrap_err();
        let expected = match problem {
            "bytes" | "mode" => "install/bundle-file-mismatch",
            "engine" => "install/invalid-version-manifest",
            "probe" => "install/engine-mismatch",
            _ => unreachable!(),
        };
        assert_eq!(error.code, expected, "{problem}: {error:?}");
        assert_eq!(snapshot(&request.project), before, "{problem}");
    }
}

#[test]
fn install_provider_defaults_detect_codex_without_codex_skill_writes() {
    let root = Workspace::new();
    let mut request = fixture(&root);
    request.providers = None;
    let default = plan_install(&request).unwrap();
    assert_eq!(default.providers, [".claude"]);
    fs::create_dir(request.project.join(".codex")).unwrap();
    let detected = plan_install(&request).unwrap();
    assert_eq!(detected.providers, [".agents", ".codex"]);
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
    assert!(!request.project.join(".codex/skills").exists());
}

#[test]
fn install_manifest_paths_stay_inside_the_skill_layout() {
    for path in [
        "/absolute",
        "../escape",
        "schemas/../SKILL.md",
        "hooks.json",
        "settings.json",
        "AGENTS.md",
        "../phoenix-ui/SKILL.md",
        "scripts/renderer.mjs",
        "scripts/phoenix-diagram/renderer.mjs",
        "ENGINE.json",
        "schemas/nested/file.json",
        "agents/other.yaml",
    ] {
        let root = Workspace::new();
        let request = fixture(&root);
        change_manifest(&request, |manifest| {
            manifest["files"][0]["path"] = json!(path)
        });
        let before = snapshot(&request.project);
        let error = apply_install(&request).unwrap_err();
        assert_eq!(
            error.code, "install/unsafe-bundle-file",
            "{path}: {error:?}"
        );
        assert_eq!(snapshot(&request.project), before);
    }
}

#[test]
fn install_checks_engine_manifest_identity_and_probe() {
    for field in ["schemaVersion", "tool", "toolVersion", "bundleSha256"] {
        let root = Workspace::new();
        let request = fixture(&root);
        change_version(&request, |version| {
            version[field] = if field == "schemaVersion" {
                json!(2)
            } else {
                json!("wrong")
            }
        });
        let before = snapshot(&request.project);
        assert_eq!(
            apply_install(&request).unwrap_err().code,
            "install/version-identity-mismatch",
            "{field}"
        );
        assert_eq!(snapshot(&request.project), before);
    }
    let root = Workspace::new();
    let mut request = fixture(&root);
    let fake = root.path("wrong-probe");
    let bytes = b"#!/bin/sh\nprintf 'wrong-native-engine\\n'\n";
    write(&fake, bytes, 0o755);
    request.engine = fake;
    change_version(&request, |version| {
        version["engines"][platform()]["size"] = json!(bytes.len());
        version["engines"][platform()]["sha256"] = json!(digest(bytes));
    });
    assert_eq!(
        apply_install(&request).unwrap_err().code,
        "install/engine-probe"
    );
    assert!(snapshot(&request.project).is_empty());
}

#[test]
fn install_accepts_a_later_npm_version_for_the_same_engine() {
    let root = Workspace::new();
    let request = fixture(&root);
    change_version(&request, |version| {
        version["npmPackageVersion"] = json!("99.0.0")
    });
    let receipt = plan_install(&request).unwrap();
    assert_eq!(receipt.npm_package_version, "99.0.0");
}

#[test]
fn install_all_nineteen_provider_names_have_only_eighteen_skill_roots() {
    let root = Workspace::new();
    let mut request = fixture(&root);
    let names = [
        "agent", "agents", "claude", "codex", "cursor", "dsh", "gemini", "github", "grok",
        "hermes", "kiro", "opencode", "pi", "qoder", "rovodev", "trae", "trae-cn", "veto", "vibe",
    ];
    request.providers = Some(names.iter().map(|name| format!(".{name}")).collect());
    let receipt = apply_install(&request).unwrap();
    assert_eq!(receipt.status, InstallStatus::Installed, "{receipt:?}");
    assert_eq!(receipt.providers.len(), 19);
    for name in names {
        let path = request
            .project
            .join(format!(".{name}/skills/phoenix-diagram/SKILL.md"));
        assert_eq!(path.is_file(), name != "codex", "{name}");
    }
    assert!(receipt
        .writes
        .iter()
        .all(|write| !write.path.contains("hook") && !write.path.contains("settings")));
}
