// Installer acceptance uses real temporary filesystems and the actual Cargo
// engine binary. Provider directory tests do not prove host loading (V-13).
pub use phoenix_diagram::{
    diagnostic, output, INPUT_CONTRACT_SCHEMA, RECEIPT_SCHEMA, TOOL_VERSION, VIEWER_REVISION,
};
#[path = "../src/install.rs"]
mod install;
use install::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{symlink, DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
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
    let compiled = Path::new(env!("CARGO_BIN_EXE_phoenix-diagram"));
    let (engine_size, engine_sha256) = ENGINE_DIGEST.get_or_init(|| {
        let bytes = fs::read(compiled).unwrap();
        (bytes.len() as u64, digest(&bytes))
    });
    // Cargo artifacts may have multiple links; probe only an owned byte copy.
    let engine = root.path("phoenix-diagram");
    assert_eq!(fs::copy(compiled, &engine).unwrap(), *engine_size);
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o700)).unwrap();
    let metadata = fs::symlink_metadata(&engine).unwrap();
    assert!(metadata.is_file() && !metadata.is_symlink());
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(metadata.uid(), fs::metadata(&root.0).unwrap().uid());
    assert_eq!(digest(&fs::read(&engine).unwrap()), *engine_sha256);
    let version = serde_json::to_vec(&json!({
        "schemaVersion":1,"distribution":"development","tool":"phoenix-diagram",
        "toolVersion":"0.2.0","npmPackageVersion":env!("PHOENIX_DIAGRAM_NPM_PACKAGE_VERSION"),"sourceCommit":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sourceInputSha256":env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256"),
        "inputContractSchema":1,"receiptSchema":1,"viewerRevision":"phoenix-v2",
        "bundleSha256":digest(&manifest),
        "engines":{platform():{"asset":format!("phoenix-diagram-{}",platform()),"size":engine_size,"sha256":engine_sha256}}
    }))
    .unwrap();
    write(&bundle.join("ENGINE.json"), &version, 0o644);
    let version_manifest = root.path("VERSION.json");
    write(&version_manifest, &version, 0o644);
    InstallRequest {
        project: root.path("project"),
        bundle_root: bundle,
        version_manifest,
        engine,
        providers: Some(vec!["codex".into()]),
    }
}
fn load(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn change_version(request: &InstallRequest, update: impl FnOnce(&mut Value)) {
    let mut version = load(&request.version_manifest);
    update(&mut version);
    let bytes = serde_json::to_vec(&version).unwrap();
    write(&request.version_manifest, &bytes, 0o644);
    write(&request.bundle_root.join("ENGINE.json"), &bytes, 0o644);
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
fn update_skill(request: &InstallRequest, bytes: &[u8]) {
    write(&request.bundle_root.join("SKILL.md"), bytes, 0o644);
    change_manifest(request, |manifest| {
        let entry = manifest["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["path"] == "SKILL.md")
            .unwrap();
        entry["size"] = json!(bytes.len());
        entry["sha256"] = json!(digest(bytes));
    });
}
fn checkpoint_name(checkpoint: &install::test_support::Checkpoint) -> &'static str {
    use install::test_support::Checkpoint;
    match checkpoint {
        Checkpoint::LockAcquired => "lock",
        Checkpoint::JournalReady => "journal",
        Checkpoint::AfterRetire(_) => "retire",
        Checkpoint::AfterLink(path) if path == ".phoenix-diagram/install/active.json" => {
            "active-link"
        }
        Checkpoint::AfterLink(_) => "link",
        Checkpoint::AfterPublish(_) => "publish",
        Checkpoint::BeforeActive => "before-active",
        Checkpoint::AfterActive => "active",
        Checkpoint::AfterRecover(_) => "recover",
        Checkpoint::AfterRecoverRetire(_) => "undo-retire",
        Checkpoint::AfterRecoverLink(_) => "restore-link",
        _ => "other",
    }
}
struct ChildGuard(std::process::Child);
impl std::ops::Deref for ChildGuard {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn child(request: &InstallRequest, root: &Workspace, action: &str, checkpoint: &str) -> ChildGuard {
    let path = root.path("request.json");
    write(&path, &serde_json::to_vec(request).unwrap(), 0o600);
    ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "install_child_process", "--nocapture"])
            .env("PHOENIX_DIAGRAM_INSTALL_TEST_REQUEST", path)
            .env("PHOENIX_DIAGRAM_INSTALL_TEST_ACTION", action)
            .env("PHOENIX_DIAGRAM_INSTALL_TEST_CHECKPOINT", checkpoint)
            .env("PHOENIX_DIAGRAM_INSTALL_TEST_MARKER", root.path("ready"))
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    )
}
fn crash(request: &InstallRequest, root: &Workspace, action: &str, checkpoint: &str) {
    let status = child(request, root, action, checkpoint).wait().unwrap();
    assert_eq!(
        status.signal(),
        Some(libc::SIGKILL),
        "{action}/{checkpoint}: {status:?}"
    );
}

#[test]
fn install_child_process() {
    let Some(path) = std::env::var_os("PHOENIX_DIAGRAM_INSTALL_TEST_REQUEST") else {
        return;
    };
    let request: InstallRequest = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let action = std::env::var("PHOENIX_DIAGRAM_INSTALL_TEST_ACTION").unwrap();
    let wanted = std::env::var("PHOENIX_DIAGRAM_INSTALL_TEST_CHECKPOINT").unwrap();
    let mut observer = |checkpoint, _: &Path| {
        if checkpoint_name(&checkpoint) == wanted {
            if action == "hold" {
                write(
                    &PathBuf::from(
                        std::env::var_os("PHOENIX_DIAGRAM_INSTALL_TEST_MARKER").unwrap(),
                    ),
                    b"ready",
                    0o600,
                );
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            unsafe {
                assert_eq!(libc::kill(libc::getpid(), libc::SIGKILL), 0);
                loop {
                    libc::pause();
                }
            }
        }
        Ok(())
    };
    if action == "hold" {
        install::test_support::hold_lock(&request.project, &mut observer).unwrap();
        panic!("lock observer unexpectedly returned");
    }
    let receipt = if action == "recover" {
        install::test_support::recover(&request.project, &mut observer)
    } else {
        install::test_support::apply(&request, &mut observer)
    }
    .unwrap();
    panic!("checkpoint was not reached: {wanted}, receipt={receipt:?}");
}

#[test]
fn install_dry_run_has_no_project_writes_and_uses_real_engine() {
    let root = Workspace::new();
    let request = fixture(&root);
    let before = snapshot(&request.project);
    let receipt = plan_install(&request).unwrap();
    assert_eq!(receipt.status, InstallStatus::Planned);
    assert_eq!(receipt.exit_code(), 0);
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
    assert!(receipt.engine.is_some());
    assert!(receipt
        .recovery_command
        .as_deref()
        .is_some_and(|s| !s.contains("rollback")));
    let recovery = receipt.recovery_command.as_ref().unwrap();
    assert!(!recovery.contains("npm exec"), "{recovery}");
    let retried = Command::new("/bin/sh")
        .args(["-c", &format!("{recovery} --dry-run")])
        .output()
        .unwrap();
    assert!(
        retried.status.success(),
        "{}",
        String::from_utf8_lossy(&retried.stderr)
    );
    let report: Value = serde_json::from_slice(&retried.stdout).unwrap();
    assert_eq!(report["status"], "planned");
    assert_eq!(snapshot(&request.project), before);
}

#[test]
fn install_apply_is_owned_idempotent_and_isolated() {
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
    assert_eq!(receipt.exit_code(), 0);
    assert_eq!(
        fs::read(request.project.join(format!("{SKILL}/ENGINE.json"))).unwrap(),
        fs::read(&request.version_manifest).unwrap()
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
    let active = load(&request.project.join(".phoenix-diagram/install/active.json"));
    assert_eq!(active["tool"], "phoenix-diagram");
    assert_eq!(
        active["engineManifestSha256"],
        digest(&fs::read(&request.version_manifest).unwrap())
    );
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
fn install_never_adopts_unowned_identical_bytes() {
    let root = Workspace::new();
    let request = fixture(&root);
    let bytes = fs::read(request.bundle_root.join("SKILL.md")).unwrap();
    write(
        &request.project.join(format!("{SKILL}/SKILL.md")),
        &bytes,
        0o644,
    );
    let before = snapshot(&request.project);
    let receipt = apply_install(&request).unwrap();
    assert_eq!(receipt.status, InstallStatus::Conflict);
    assert_eq!(receipt.exit_code(), 3);
    assert!(receipt
        .conflicts
        .iter()
        .any(|c| c.code == "install/unowned-file"));
    assert_eq!(snapshot(&request.project), before);
}

#[test]
fn install_committed_identical_bytes_with_new_inode_preserve_conflict() {
    use std::os::unix::fs::MetadataExt;
    let root = Workspace::new();
    let request = fixture(&root);
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
    let target = request.project.join(format!("{SKILL}/SKILL.md"));
    let bytes = fs::read(&target).unwrap();
    let original_inode = fs::metadata(&target).unwrap().ino();
    fs::rename(&target, root.path("saved-original")).unwrap();
    write(&target, &bytes, 0o644);
    let replacement_inode = fs::metadata(&target).unwrap().ino();
    assert_ne!(replacement_inode, original_inode);
    let before = snapshot(&request.project);
    let planned = plan_install(&request).unwrap();
    assert_eq!(planned.status, InstallStatus::Conflict, "{planned:?}");
    assert!(planned
        .conflicts
        .iter()
        .any(|conflict| conflict.code == "install/managed-file-modified"));
    assert_eq!(snapshot(&request.project), before);
    let applied = apply_install(&request).unwrap();
    assert_eq!(applied.status, InstallStatus::Conflict, "{applied:?}");
    assert_eq!(snapshot(&request.project), before);
    assert_eq!(fs::metadata(&target).unwrap().ino(), replacement_inode);
    assert_eq!(fs::read(&target).unwrap(), bytes);
    assert_eq!(fs::read(root.path("saved-original")).unwrap(), bytes);
}

#[test]
fn install_preserves_user_modifications_extra_files_and_symlink_parents() {
    for problem in ["modified", "extra", "symlink"] {
        let root = Workspace::new();
        let request = fixture(&root);
        if problem == "symlink" {
            fs::create_dir(root.path("external")).unwrap();
            symlink(root.path("external"), request.project.join(".agents")).unwrap();
        } else {
            assert_eq!(
                apply_install(&request).unwrap().status,
                InstallStatus::Installed
            );
            let path = request.project.join(format!(
                "{SKILL}/{}",
                if problem == "modified" {
                    "SKILL.md"
                } else {
                    "notes.txt"
                }
            ));
            write(&path, b"user-owned later bytes\n", 0o644);
        }
        let before = snapshot(&request.project);
        let receipt = apply_install(&request).unwrap();
        assert_eq!(
            receipt.status,
            InstallStatus::Conflict,
            "{problem}: {receipt:?}"
        );
        assert_eq!(snapshot(&request.project), before, "{problem}");
        assert!(
            fs::read_dir(root.path("external"))
                .map(|r| r.count())
                .unwrap_or(0)
                == 0
        );
    }
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
            "engine" => "install/engine-manifest-mismatch",
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
fn install_manifest_allowlist_is_strict_and_preflight_is_read_only() {
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
    for problem in [
        "duplicate-path",
        "unknown-field",
        "duplicate-key",
        "extra-file",
        "symlink",
    ] {
        let root = Workspace::new();
        let request = fixture(&root);
        let expected = match problem {
            "duplicate-path" => {
                change_manifest(&request, |manifest| {
                    let entry = manifest["files"][0].clone();
                    manifest["files"].as_array_mut().unwrap().push(entry);
                });
                "install/unsafe-bundle-file"
            }
            "unknown-field" => {
                change_manifest(&request, |manifest| manifest["hooks"] = json!({}));
                "install/invalid-bundle-manifest"
            }
            "duplicate-key" => {
                let path = request.bundle_root.join("manifest.json");
                let previous = fs::read_to_string(&path).unwrap();
                let bytes = previous
                    .replacen('{', "{\"schemaVersion\":1,", 1)
                    .into_bytes();
                write(&path, &bytes, 0o644);
                change_version(&request, |version| {
                    version["bundleSha256"] = json!(digest(&bytes))
                });
                "install/invalid-bundle-manifest"
            }
            "extra-file" => {
                write(&request.bundle_root.join("hooks.json"), b"{}", 0o644);
                "install/extra-bundle-entry"
            }
            "symlink" => {
                let path = request.bundle_root.join("SKILL.md");
                fs::rename(&path, root.path("source-skill")).unwrap();
                symlink(root.path("source-skill"), path).unwrap();
                "install/unsafe-file"
            }
            _ => unreachable!(),
        };
        let before = snapshot(&request.project);
        assert_eq!(
            apply_install(&request).unwrap_err().code,
            expected,
            "{problem}"
        );
        assert_eq!(snapshot(&request.project), before, "{problem}");
    }
}

#[test]
fn install_rechecks_all_native_version_and_probe_identity() {
    for field in [
        "schemaVersion",
        "tool",
        "toolVersion",
        "sourceInputSha256",
        "inputContractSchema",
        "receiptSchema",
        "viewerRevision",
        "bundleSha256",
    ] {
        let root = Workspace::new();
        let request = fixture(&root);
        change_version(&request, |version| {
            version[field] =
                if ["schemaVersion", "inputContractSchema", "receiptSchema"].contains(&field) {
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
fn install_rejects_npm_version_outside_the_compiled_candidate() {
    let root = Workspace::new();
    let request = fixture(&root);
    let compiled = env!("PHOENIX_DIAGRAM_NPM_PACKAGE_VERSION");
    let unrelated = if compiled == "0.0.0-unrelated" {
        "0.0.1-unrelated"
    } else {
        "0.0.0-unrelated"
    };
    change_version(&request, |version| {
        version["npmPackageVersion"] = json!(unrelated)
    });
    let before = snapshot(&request.project);
    let error = plan_install(&request).unwrap_err();
    assert_eq!(error.code, "install/version-identity-mismatch");
    assert_eq!(snapshot(&request.project), before);
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

#[test]
fn install_real_kill_recovers_each_publication_boundary_and_retries() {
    for checkpoint in ["journal", "retire", "link", "publish", "before-active"] {
        let root = Workspace::new();
        let request = fixture(&root);
        crash(&request, &root, "apply", checkpoint);
        let before = snapshot(&request.project);
        let receipt = plan_install(&request).unwrap();
        assert_eq!(
            receipt.status,
            InstallStatus::RecoveryRequired,
            "{checkpoint}: {receipt:?}"
        );
        assert_eq!(snapshot(&request.project), before, "dryrun/{checkpoint}");
        let recovery = recover_install(&request.project).unwrap();
        assert_eq!(
            recovery.status,
            InstallStatus::Recovered,
            "{checkpoint}: {recovery:?}"
        );
        assert!(!request
            .project
            .join(format!("{SKILL}/ENGINE.json"))
            .exists());
        assert!(!request
            .project
            .join(".phoenix-diagram/install/active.json")
            .exists());
        assert_eq!(
            apply_install(&request).unwrap().status,
            InstallStatus::Installed,
            "retry/{checkpoint}"
        );
    }
}

#[test]
fn install_explicit_retry_recovers_and_committed_active_finishes() {
    for checkpoint in ["publish", "active-link", "active"] {
        let root = Workspace::new();
        let request = fixture(&root);
        crash(&request, &root, "apply", checkpoint);
        let receipt = apply_install(&request).unwrap();
        assert_eq!(
            receipt.status,
            if checkpoint == "active" || checkpoint == "active-link" {
                InstallStatus::Unchanged
            } else {
                InstallStatus::Installed
            },
            "{checkpoint}: {receipt:?}"
        );
        assert!(!receipt.recovered.is_empty());
        assert_eq!(
            plan_install(&request).unwrap().status,
            InstallStatus::Unchanged
        );
    }
}

#[test]
fn install_recovery_preserves_later_modified_bytes_and_inode() {
    use std::os::unix::fs::MetadataExt;
    for problem in ["bytes", "identical-new-inode", "unknown-hardlink"] {
        let root = Workspace::new();
        let request = fixture(&root);
        crash(&request, &root, "apply", "publish");
        let target = request.project.join(format!("{SKILL}/ENGINE.json"));
        match problem {
            "bytes" => write(&target, b"later user modification\n", 0o644),
            "identical-new-inode" => {
                let bytes = fs::read(&target).unwrap();
                fs::rename(&target, root.path("user-retired")).unwrap();
                write(&target, &bytes, 0o644);
            }
            "unknown-hardlink" => fs::hard_link(&target, root.path("user-link")).unwrap(),
            _ => unreachable!(),
        }
        let before = fs::read(&target).unwrap();
        let metadata = fs::metadata(&target).unwrap();
        let receipt = recover_install(&request.project).unwrap();
        assert_eq!(
            receipt.status,
            InstallStatus::Conflict,
            "{problem}: {receipt:?}"
        );
        assert!(
            receipt.conflicts.iter().any(|conflict| conflict.code
                == if problem == "unknown-hardlink" {
                    "install/recovery-unknown-hardlink"
                } else {
                    "install/recovery-later-modification"
                }),
            "{problem}: {receipt:?}"
        );
        assert_eq!(fs::read(&target).unwrap(), before);
        assert_eq!(fs::metadata(&target).unwrap().ino(), metadata.ino());
        assert_eq!(fs::metadata(&target).unwrap().nlink(), metadata.nlink());
    }
}

#[test]
fn install_upgrade_recovery_restores_original_before_inode_and_is_repeatable() {
    use std::os::unix::fs::MetadataExt;
    let root = Workspace::new();
    let request = fixture(&root);
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
    let path = request.project.join(format!("{SKILL}/ENGINE.json"));
    let before = fs::read(&path).unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    update_skill(&request, b"updated managed skill\n");
    crash(&request, &root, "apply", "retire");
    let pending = plan_install(&request).unwrap();
    for entry in fs::read_dir(pending.backup.unwrap()).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            assert_eq!(entry.file_name(), ".phoenix-diagram-output-cleanup");
            assert_eq!(
                entry.metadata().unwrap().permissions().mode() & 0o7777,
                0o700
            );
        } else {
            assert!(kind.is_file());
            assert_eq!(
                entry.metadata().unwrap().permissions().mode() & 0o7777,
                0o600
            );
        }
    }
    crash(&request, &root, "recover", "restore-link");
    let receipt = recover_install(&request.project).unwrap();
    assert_eq!(receipt.status, InstallStatus::Recovered, "{receipt:?}");
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
}

#[test]
fn install_os_lock_serializes_real_processes_and_releases_after_kill() {
    let root = Workspace::new();
    let request = fixture(&root);
    let mut child = child(&request, &root, "hold", "lock");
    let started = std::time::Instant::now();
    while !root.path("ready").is_file() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "child did not acquire the real lock"
        );
        assert!(
            child.try_wait().unwrap().is_none(),
            "child exited before holding the lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let error = apply_install(&request).unwrap_err();
    assert_eq!(error.kind, InstallErrorKind::Busy);
    assert_eq!(error.exit_code(), 3);
    child.kill().unwrap();
    assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGKILL));
    assert_eq!(
        apply_install(&request).unwrap().status,
        InstallStatus::Installed
    );
}

#[test]
fn install_unknown_parent_claimant_is_preserved_at_the_mutation_boundary() {
    let root = Workspace::new();
    let request = fixture(&root);
    fs::create_dir_all(request.project.join(".agents/skills")).unwrap();
    fs::create_dir(root.path("external")).unwrap();
    let mut changed = false;
    let receipt = install::test_support::apply(&request, &mut |checkpoint, _| {
        if !changed
            && matches!(
                checkpoint,
                install::test_support::Checkpoint::BeforePublish(_)
            )
        {
            fs::rename(request.project.join(".agents"), root.path("retired-parent")).unwrap();
            symlink(root.path("external"), request.project.join(".agents")).unwrap();
            changed = true;
        }
        Ok(())
    })
    .unwrap();
    assert!(changed);
    assert_eq!(
        receipt.status,
        InstallStatus::RecoveryRequired,
        "{receipt:?}"
    );
    assert!(receipt
        .conflicts
        .iter()
        .any(|conflict| conflict.code == "install/parent-changed"));
    assert!(fs::read_dir(root.path("external"))
        .unwrap()
        .next()
        .is_none());
    assert!(fs::symlink_metadata(request.project.join(".agents"))
        .unwrap()
        .is_symlink());
}

#[test]
fn install_invalid_active_or_journal_never_grants_ownership() {
    for problem in ["active", "journal", "forged"] {
        let root = Workspace::new();
        let request = fixture(&root);
        let active_path = request.project.join(".phoenix-diagram/install/active.json");
        if problem == "forged" {
            let source = fs::read(request.bundle_root.join("SKILL.md")).unwrap();
            write(
                &request.project.join(format!("{SKILL}/SKILL.md")),
                &source,
                0o644,
            );
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(active_path.parent().unwrap())
                .unwrap();
            write(
                &active_path,
                b"{\"schemaVersion\":1,\"tool\":\"phoenix-diagram\",\"ownership\":[]}",
                0o600,
            );
        } else {
            assert_eq!(
                apply_install(&request).unwrap().status,
                InstallStatus::Installed
            );
            let path = if problem == "active" {
                active_path.clone()
            } else {
                request
                    .project
                    .join(load(&active_path)["journal"].as_str().unwrap())
            };
            let mut value = load(&path);
            value["checksum"] = json!("0".repeat(64));
            write(&path, &serde_json::to_vec(&value).unwrap(), 0o600);
        }
        let before = snapshot(&request.project);
        let receipt = apply_install(&request).unwrap();
        assert_eq!(
            receipt.status,
            InstallStatus::Conflict,
            "{problem}: {receipt:?}"
        );
        assert!(receipt.conflicts.iter().any(|conflict| conflict.code
            == if problem == "journal" {
                "install/invalid-journal"
            } else {
                "install/invalid-active"
            }));
        assert_eq!(snapshot(&request.project), before, "{problem}");
    }
}

#[test]
fn install_unknown_or_tampered_output_cleanup_is_not_adopted() {
    for problem in ["empty", "tampered-owner"] {
        let root = Workspace::new();
        let request = fixture(&root);
        let namespace = request
            .project
            .join(".phoenix-diagram/install/journals/.phoenix-diagram-output-cleanup");
        if problem == "empty" {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&namespace)
                .unwrap();
        } else {
            assert_eq!(
                apply_install(&request).unwrap().status,
                InstallStatus::Installed
            );
            let owner_path = namespace.join("owner-v1.json");
            let mut owner = load(&owner_path);
            owner["checksum"] = json!("0".repeat(64));
            write(&owner_path, &serde_json::to_vec(&owner).unwrap(), 0o600);
        }
        let before = snapshot(&request.project);
        let receipt = apply_install(&request).unwrap();
        assert_eq!(
            receipt.status,
            InstallStatus::Conflict,
            "{problem}: {receipt:?}"
        );
        assert!(
            receipt
                .conflicts
                .iter()
                .any(|conflict| if problem == "empty" {
                    conflict.code == "install/unverified-output-namespace"
                } else {
                    conflict.code == "install/output/cleanup-owner-binding-mismatch"
                }),
            "{problem}: {receipt:?}"
        );
        assert_eq!(snapshot(&request.project), before);
    }
}
