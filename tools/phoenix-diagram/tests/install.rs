// Installer acceptance uses real temporary filesystems and the actual Cargo
// engine binary. Provider directory tests do not prove host loading (V-13).
pub use phoenix_diagram::{
    diagnostic, INPUT_CONTRACT_SCHEMA, RECEIPT_SCHEMA, TOOL_VERSION, VIEWER_REVISION,
};
#[path = "../src/acceptance.rs"]
#[allow(dead_code)] // The shared gate copy also contains the unused preview-worker companion.
mod acceptance;
mod output {
    pub(crate) use super::acceptance::checkpoint as acceptance_checkpoint;
    pub use phoenix_diagram::output::*;
}
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
        Checkpoint::BeforeLegacyRemove(_) => "before-legacy",
        Checkpoint::AfterLegacyRemove(_) => "legacy-remove",
        Checkpoint::BeforeRollback(_) => "before-rollback",
        Checkpoint::AfterRollbackJournalRetire => "rollback-journal-retire",
        Checkpoint::AfterRollbackJournalPublish => "rollback-journal-publish",
        Checkpoint::RollbackAuditArchived => "compat-audit",
        Checkpoint::RollbackCompatibilityPublished => "compat-publish",
        Checkpoint::AfterRollback(_) => "rollback",
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
    } else if action == "migrate" {
        install::test_support::migrate(&request, true, false, &mut observer)
    } else if let Some(id) = action.strip_prefix("rollback:") {
        install::test_support::rollback(&request.project, id, false, &mut observer)
    } else if let Some(id) = action.strip_prefix("migration-recover:") {
        install::test_support::recover_migration(&request.project, id, &mut observer)
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

// Migration uses complete frozen trees in disposable project copies only.
fn legacy_fixture(
    root: &Workspace,
    request: &InstallRequest,
    source: &str,
    provider: &str,
) -> PathBuf {
    let extraction = root.path(&format!("legacy-{provider}"));
    fs::create_dir(&extraction).unwrap();
    let upstream = Path::new(env!("CARGO_MANIFEST_DIR")).join("upstream");
    let (archive, member) = if source == "reference-3.1.0" {
        ("archify-3.1.0-source.tar.gz", "archify")
    } else {
        ("archify-2.17-full-source.tar.gz", source)
    };
    let extracted = Command::new("tar")
        .arg("-xzf")
        .arg(upstream.join(archive))
        .arg("-C")
        .arg(&extraction)
        .arg(member)
        .output()
        .unwrap();
    assert!(
        extracted.status.success(),
        "{}",
        String::from_utf8_lossy(&extracted.stderr)
    );
    let target = request.project.join(format!("{provider}/skills/archify"));
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::rename(extraction.join(member), &target).unwrap();
    target
}
fn migrate_fixture(request: &InstallRequest, confirm: bool, dry_run: bool) -> InstallReceipt {
    install::test_support::migrate(request, confirm, dry_run, &mut |_, _| Ok(())).unwrap()
}
#[test]
fn migration_complete_sources_confirmation_and_direct_removal() {
    for (source, count) in [
        ("legacy-2.17", 219),
        ("legacy-2.17-when-to-use", 219),
        ("reference-3.1.0", 157),
    ] {
        let root = Workspace::new();
        let request = fixture(&root);
        let legacy = legacy_fixture(&root, &request, source, ".agents");
        write(
            &request.project.join(".claude/settings.json"),
            b"shared settings\n",
            0o644,
        );
        write(
            &request.project.join(".phoenix-ui/untouched"),
            b"ui\n",
            0o644,
        );
        write(&request.project.join("AGENTS.md"), b"authored\n", 0o644);
        let original = snapshot(&request.project);
        let planned = migrate_fixture(&request, false, true);
        assert_eq!(planned.status, InstallStatus::Planned, "{planned:?}");
        assert_eq!(planned.migration_sources, vec![source]);
        assert!(planned.removals.len() >= count);
        assert_eq!(snapshot(&request.project), original);
        let failure =
            install::test_support::migrate(&request, false, false, &mut |_, _| Ok(())).unwrap_err();
        assert_eq!(failure.code, "install/confirmation-required");
        assert_eq!(snapshot(&request.project), original);
        let applied = migrate_fixture(&request, true, false);
        assert_eq!(applied.status, InstallStatus::Installed, "{applied:?}");
        assert!(!legacy.exists());
        assert!(applied.removals.iter().all(|r| r.status == "removed"));
        assert_eq!(applied.legacy_backup_policy, "none");
        assert_eq!(
            load(&request.project.join(".phoenix-diagram/install/active.json"))["schemaVersion"],
            2
        );
        let journal = load(applied.journal.as_ref().unwrap());
        assert_eq!(journal["schemaVersion"], 2);
        assert_eq!(
            journal["migration"]["roots"][0]["kind"],
            "legacy-root-remove"
        );
        assert!(journal["operations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|op| op["kind"] == "file-write"
                && !op["path"].as_str().unwrap().contains("archify")));
        assert_eq!(
            fs::read(request.project.join(".claude/settings.json")).unwrap(),
            b"shared settings\n"
        );
        assert_eq!(
            fs::read(request.project.join(".phoenix-ui/untouched")).unwrap(),
            b"ui\n"
        );
        assert_eq!(
            fs::read(request.project.join("AGENTS.md")).unwrap(),
            b"authored\n"
        );
        let id = applied.transaction_id.unwrap();
        let before = snapshot(&request.project);
        let dry = install::rollback(&request.project, &id, true).unwrap();
        assert_eq!(dry.status, InstallStatus::Planned);
        assert_eq!(snapshot(&request.project), before);
        let rolled = install::rollback(&request.project, &id, false).unwrap();
        assert_eq!(rolled.status, InstallStatus::RolledBack, "{rolled:?}");
        assert!(!legacy.exists());
        assert!(!request.project.join(format!("{SKILL}/SKILL.md")).exists());
        assert!(rolled.journal.unwrap().ends_with("rolled-back-v2.json"));
        let before = snapshot(&request.project);
        assert_eq!(
            install::rollback(&request.project, &id, false)
                .unwrap()
                .status,
            InstallStatus::RolledBack
        );
        assert_eq!(snapshot(&request.project), before);
    }
}
#[test]
fn migration_preserves_unknown_modes_extra_links_and_manager_claims_without_writes() {
    for fault in [
        "mode",
        "extra",
        "missing",
        "modified",
        "symlink",
        "hardlink",
        "manager",
        "empty-dir",
    ] {
        let root = Workspace::new();
        let request = fixture(&root);
        let legacy = legacy_fixture(&root, &request, "legacy-2.17", ".agents");
        match fault {
            "mode" => {
                fs::set_permissions(legacy.join("SKILL.md"), fs::Permissions::from_mode(0o664))
                    .unwrap()
            }
            "extra" => write(&legacy.join("authored.txt"), b"extra", 0o644),
            "missing" => fs::remove_file(legacy.join("SKILL.md")).unwrap(),
            "modified" => write(&legacy.join("SKILL.md"), b"custom", 0o644),
            "symlink" => {
                fs::remove_file(legacy.join("SKILL.md")).unwrap();
                symlink(
                    request.bundle_root.join("SKILL.md"),
                    legacy.join("SKILL.md"),
                )
                .unwrap();
            }
            "hardlink" => fs::hard_link(legacy.join("SKILL.md"), root.path("linked")).unwrap(),
            "manager" => write(
                &request.project.join("skills-lock.json"),
                br#"{"version":1,"skills":{"archify":{"source":"XGENT-ai/skills"}}}"#,
                0o644,
            ),
            "empty-dir" => fs::create_dir(legacy.join("authored")).unwrap(),
            _ => unreachable!(),
        }
        let before = snapshot(&request.project);
        let receipt = migrate_fixture(&request, true, false);
        assert_eq!(
            receipt.status,
            InstallStatus::Conflict,
            "{fault}: {receipt:?}"
        );
        assert!(receipt.writes.is_empty());
        assert_eq!(snapshot(&request.project), before, "{fault}");
        assert!(!request.project.join(".phoenix-diagram").exists());
    }
}
#[test]
fn migration_resumes_deletion_cursor_and_retains_later_claimants() {
    for fault in ["resume", "extra", "same-content-inode", "rebuilt-parent"] {
        let root = Workspace::new();
        let request = fixture(&root);
        let legacy = legacy_fixture(&root, &request, "legacy-2.17", ".agents");
        let mut observed = false;
        let applied =
            install::test_support::migrate(&request, true, false, &mut |checkpoint, _| {
                if matches!(
                    checkpoint,
                    install::test_support::Checkpoint::AfterLegacyRemove(_)
                ) && !observed
                {
                    observed = true;
                    return Err(std::io::Error::other(
                        "interrupted after unlink before cursor",
                    ));
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(
            applied.status,
            InstallStatus::RecoveryRequired,
            "{applied:?}"
        );
        let id = applied.transaction_id.unwrap();
        match fault {
            "extra" => write(&legacy.join("claimant.txt"), b"later", 0o644),
            "same-content-inode" => {
                let path = legacy.join("SKILL.md");
                let bytes = fs::read(&path).unwrap();
                let mode = fs::metadata(&path).unwrap().mode() & 0o7777;
                fs::rename(&path, root.path("original-skill")).unwrap();
                write(&path, &bytes, mode);
            }
            "rebuilt-parent" => {
                let parent = request.project.join(".agents/skills/phoenix-diagram");
                fs::rename(&parent, root.path("original-diagram-parent")).unwrap();
                fs::create_dir(&parent).unwrap();
            }
            _ => {}
        }
        let receipt = install::recover_migration(&request.project, &id).unwrap();
        if fault == "resume" {
            assert_eq!(receipt.status, InstallStatus::Recovered, "{receipt:?}");
            assert!(!legacy.exists());
            assert!(request.project.join(format!("{SKILL}/SKILL.md")).exists());
            assert_eq!(
                install::recover_migration(&request.project, &id)
                    .unwrap()
                    .status,
                InstallStatus::Unchanged
            );
        } else {
            assert_eq!(
                receipt.status,
                InstallStatus::RecoveryRequired,
                "{fault}: {receipt:?}"
            );
            assert!(legacy.exists());
            if fault == "same-content-inode" {
                assert!(receipt
                    .removals
                    .iter()
                    .any(|r| r.path.ends_with("/SKILL.md") && r.status == "preserved"));
            }
        }
    }
}
#[test]
fn migration_upgrade_rollback_rebinds_prior_ownership_and_survives_interruption() {
    let root = Workspace::new();
    let request = fixture(&root);
    let first = apply_install(&request).unwrap();
    assert_eq!(first.status, InstallStatus::Installed);
    let before = fs::read(request.project.join(format!("{SKILL}/SKILL.md"))).unwrap();
    let old_identity = fs::metadata(request.project.join(format!("{SKILL}/SKILL.md")))
        .unwrap()
        .ino();
    legacy_fixture(&root, &request, "legacy-2.17", ".agents");
    update_skill(
        &request,
        b"---\nname: phoenix-diagram\ndescription: upgraded\n---\n",
    );
    let applied = migrate_fixture(&request, true, false);
    assert_eq!(applied.status, InstallStatus::Installed, "{applied:?}");
    let id = applied.transaction_id.unwrap();
    let mut stopped = false;
    let rolled =
        install::test_support::rollback(&request.project, &id, false, &mut |checkpoint, _| {
            if matches!(
                checkpoint,
                install::test_support::Checkpoint::AfterRollback(_)
            ) && !stopped
            {
                stopped = true;
                return Err(std::io::Error::other("interrupted reverse"));
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(rolled.status, InstallStatus::RecoveryRequired, "{rolled:?}");
    let recovered = install::recover_migration(&request.project, &id).unwrap();
    assert_eq!(recovered.status, InstallStatus::RolledBack, "{recovered:?}");
    assert_eq!(
        fs::read(request.project.join(format!("{SKILL}/SKILL.md"))).unwrap(),
        before
    );
    assert_ne!(
        fs::metadata(request.project.join(format!("{SKILL}/SKILL.md")))
            .unwrap()
            .ino(),
        old_identity
    );
    let active = load(&request.project.join(".phoenix-diagram/install/active.json"));
    assert_eq!(active["transactionId"], first.transaction_id.unwrap());
    assert_eq!(
        plan_install(&request).unwrap().status,
        InstallStatus::Planned
    );
    let before_snapshot = snapshot(&request.project);
    assert_eq!(
        install::rollback(&request.project, &id, false)
            .unwrap()
            .status,
        InstallStatus::RolledBack
    );
    assert_eq!(snapshot(&request.project), before_snapshot);
}
#[test]
fn migration_rollback_preflight_preserves_later_edits_and_backup_tampering() {
    for fault in ["later-edit", "backup"] {
        let root = Workspace::new();
        let request = fixture(&root);
        apply_install(&request).unwrap();
        legacy_fixture(&root, &request, "legacy-2.17", ".agents");
        update_skill(&request, b"new skill\n");
        let applied = migrate_fixture(&request, true, false);
        let id = applied.transaction_id.unwrap();
        if fault == "later-edit" {
            write(
                &request.project.join(format!("{SKILL}/SKILL.md")),
                b"user edit",
                0o644,
            );
        } else {
            let journal = load(applied.journal.as_ref().unwrap());
            let op = journal["operations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|op| op["backupPath"].is_string())
                .unwrap();
            let transaction = applied.backup.as_ref().unwrap().parent().unwrap();
            write(
                &transaction.join(op["backupPath"].as_str().unwrap()),
                b"tampered",
                0o600,
            );
        }
        let before = snapshot(&request.project);
        assert!(install::rollback(&request.project, &id, false).is_err());
        assert_eq!(snapshot(&request.project), before, "{fault}");
    }
}

#[test]
fn migration_request_rejects_ambiguous_modes_without_state() {
    let root = Workspace::new();
    let request = fixture(&root);
    assert!(plan_install(&request)
        .unwrap()
        .reinstall_instructions
        .is_empty());
    let before = snapshot(&request.project);
    let failure = install::migrate(&MigrationRequest {
        mode: MigrationMode::Recover,
        project: request.project.clone(),
        providers: None,
        bundle_root: Some(request.bundle_root),
        transaction_id: Some("a".repeat(32)),
        dry_run: false,
        confirm_remove_archify: false,
    })
    .unwrap_err();
    assert_eq!(failure.code, "install/migration-arguments");
    assert_eq!(snapshot(&request.project), before);
}

#[test]
fn migration_real_kill_boundaries_recover_only_confirmed_transaction() {
    for checkpoint in ["before-legacy", "legacy-remove", "link", "active"] {
        let root = Workspace::new();
        let request = fixture(&root);
        legacy_fixture(&root, &request, "legacy-2.17", ".agents");
        crash(&request, &root, "migrate", checkpoint);
        let journal_path = fs::read_dir(request.project.join(".phoenix-diagram/install/journals"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name().unwrap().to_str().unwrap().len() == 37
                    && p.extension().is_some_and(|e| e == "json")
            })
            .unwrap();
        let journal = load(&journal_path);
        let id = journal["transactionId"].as_str().unwrap();
        let receipt = install::recover_migration(&request.project, id).unwrap();
        assert_eq!(
            receipt.status,
            InstallStatus::Recovered,
            "{checkpoint}: {receipt:?}"
        );
        assert!(!request.project.join(".agents/skills/archify").exists());
        assert_eq!(load(&journal_path)["phase"], "committed");
        assert!(receipt.removals.iter().all(|r| r.status == "removed"));
        // A real kill after reverse publication is resolved from the same durable
        // reverse identity, without reconstructing any removed legacy bytes.
        crash(&request, &root, &format!("rollback:{id}"), "rollback");
        let rolled = install::recover_migration(&request.project, id).unwrap();
        assert_eq!(
            rolled.status,
            InstallStatus::RolledBack,
            "{checkpoint}: {rolled:?}"
        );
        assert!(!request.project.join(".agents/skills/archify").exists());
    }
}

#[test]
fn migration_rebuilt_legacy_root_before_active_never_commits_or_cleans_claimant() {
    let root = Workspace::new();
    let request = fixture(&root);
    let legacy = legacy_fixture(&root, &request, "legacy-2.17", ".agents");
    let applied = install::test_support::migrate(&request, true, false, &mut |checkpoint, _| {
        if checkpoint == install::test_support::Checkpoint::BeforeActive {
            write(&legacy.join("later.txt"), b"later claimant", 0o644);
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        applied.status,
        InstallStatus::RecoveryRequired,
        "{applied:?}"
    );
    assert_eq!(load(applied.journal.as_ref().unwrap())["phase"], "applying");
    let recovered =
        install::recover_migration(&request.project, applied.transaction_id.as_deref().unwrap())
            .unwrap();
    assert_eq!(recovered.status, InstallStatus::RecoveryRequired);
    assert_eq!(
        fs::read(legacy.join("later.txt")).unwrap(),
        b"later claimant"
    );
}

#[test]
fn migration_fresh_rollback_is_idempotent_and_rejects_forged_empty_cleanup_owner() {
    for tamper in [false, true] {
        let root = Workspace::new();
        let request = fixture(&root);
        let applied = migrate_fixture(&request, true, false);
        assert_eq!(applied.status, InstallStatus::Installed, "{applied:?}");
        let id = applied.transaction_id.unwrap();
        assert_eq!(
            install::rollback(&request.project, &id, false)
                .unwrap()
                .status,
            InstallStatus::RolledBack
        );
        if tamper {
            let path = request.project.join(
                ".phoenix-diagram/install/journals/.phoenix-diagram-output-cleanup/owner-v1.json",
            );
            let mut owner = load(&path);
            owner["checksum"] = json!("0".repeat(64));
            let bytes = serde_json::to_vec(&owner).unwrap();
            write(&path, &bytes, 0o600);
            let before = snapshot(&request.project);
            assert!(install::rollback(&request.project, &id, false).is_err());
            assert_eq!(snapshot(&request.project), before);
        } else {
            let before = snapshot(&request.project);
            assert_eq!(
                install::rollback(&request.project, &id, false)
                    .unwrap()
                    .status,
                InstallStatus::RolledBack
            );
            assert_eq!(snapshot(&request.project), before);
            assert_eq!(
                plan_install(&request).unwrap().status,
                InstallStatus::Planned
            );
            assert_eq!(
                apply_install(&request).unwrap().status,
                InstallStatus::Installed
            );
            let before = snapshot(&request.project);
            assert!(install::rollback(&request.project, &id, false).is_err());
            assert_eq!(snapshot(&request.project), before);
        }
    }
}

#[test]
fn migration_all_complete_frozen_sources_are_recognized_read_only() {
    for source in ["legacy-2.17", "legacy-2.17-when-to-use", "reference-3.1.0"] {
        let root = Workspace::new();
        let request = fixture(&root);
        legacy_fixture(&root, &request, source, ".agents");
        let before = snapshot(&request.project);
        let planned = migrate_fixture(&request, false, true);
        assert_eq!(
            planned.status,
            InstallStatus::Planned,
            "{source}: {planned:?}"
        );
        assert_eq!(planned.migration_sources, vec![source]);
        assert!(planned.removals.iter().all(|r| r.status == "planned"));
        assert_eq!(snapshot(&request.project), before);
    }
}

#[test]
fn migration_rollback_preserves_same_content_replaced_previous_journal() {
    let root = Workspace::new();
    let request = fixture(&root);
    let first = apply_install(&request).unwrap();
    update_skill(&request, b"upgraded before journal claimant\n");
    let applied = migrate_fixture(&request, true, false);
    assert_eq!(applied.status, InstallStatus::Installed);
    let original = first.journal.unwrap();
    let moved = root.path("displaced-previous-journal");
    let mut bytes = vec![];
    let mut replacement_inode = 0;
    let rolled = install::test_support::rollback(
        &request.project,
        applied.transaction_id.as_deref().unwrap(),
        false,
        &mut |checkpoint, _| {
            if checkpoint
                == install::test_support::Checkpoint::BeforeRollback(
                    ".phoenix-diagram/install/active.json".into(),
                )
            {
                bytes = fs::read(&original).unwrap();
                fs::rename(&original, &moved).unwrap();
                write(&original, &bytes, 0o600);
                replacement_inode = fs::metadata(&original).unwrap().ino();
                assert_ne!(replacement_inode, fs::metadata(&moved).unwrap().ino());
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(rolled.status, InstallStatus::RecoveryRequired, "{rolled:?}");
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(fs::metadata(&original).unwrap().ino(), replacement_inode);
}

#[test]
fn migration_rollback_journal_rebind_real_kill_is_resumable() {
    for checkpoint in ["rollback-journal-retire", "rollback-journal-publish"] {
        let root = Workspace::new();
        let request = fixture(&root);
        let first = apply_install(&request).unwrap();
        update_skill(
            &request,
            b"upgrade for journal rebind kill
",
        );
        let applied = migrate_fixture(&request, true, false);
        let id = applied.transaction_id.unwrap();
        crash(&request, &root, &format!("rollback:{id}"), checkpoint);
        let recovered = install::recover_migration(&request.project, &id).unwrap();
        assert_eq!(
            recovered.status,
            InstallStatus::RolledBack,
            "{checkpoint}: {recovered:?}"
        );
        assert_eq!(
            load(&request.project.join(".phoenix-diagram/install/active.json"))["transactionId"],
            first.transaction_id.unwrap()
        );
        assert_eq!(
            plan_install(&request).unwrap().status,
            InstallStatus::Planned
        );
    }
}

#[test]
fn direct_v1_rollback_expansion_real_kill_recovers_and_repeats() {
    for checkpoint in ["rollback", "compat-audit", "compat-publish"] {
        let root = Workspace::new();
        let request = fixture(&root);
        let initial = apply_install(&request).unwrap();
        install::test_support::v1_fixture(&request.project);
        let id = initial.transaction_id.unwrap();
        assert_eq!(load(initial.journal.as_ref().unwrap())["schemaVersion"], 1);
        crash(&request, &root, &format!("rollback:{id}"), checkpoint);
        let result = install::recover_migration(&request.project, &id).unwrap();
        assert_eq!(
            result.status,
            InstallStatus::RolledBack,
            "{checkpoint}: {result:?}"
        );
        assert!(!request
            .project
            .join(".phoenix-diagram/install/active.json")
            .exists());
        let journal = load(result.journal.as_ref().unwrap());
        assert_eq!(journal["schemaVersion"], 2);
        assert_eq!(journal["phase"], "rolled-back");
        assert!(journal["operations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|op| op["kind"] == "file-write"));
        assert_eq!(
            load(initial.journal.as_ref().unwrap())["phase"],
            "recovered"
        );
        assert_eq!(load(initial.journal.as_ref().unwrap())["schemaVersion"], 1);
        let before = snapshot(&request.project);
        assert_eq!(
            install::rollback(&request.project, &id, false)
                .unwrap()
                .status,
            InstallStatus::RolledBack
        );
        assert_eq!(snapshot(&request.project), before);
        assert_eq!(
            plan_install(&request).unwrap().status,
            InstallStatus::Planned
        );
    }
}
