mod support;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use impeccable_common::Io;
use impeccable_skills::local_bundle::LocalBundle;
use impeccable_skills::providers::{Sys, PROVIDER_DIRS};
use serde_json::{json, Value};

struct Fixture {
    root: PathBuf,
    bundle: PathBuf,
    project: PathBuf,
    home: PathBuf,
    tmp: PathBuf,
}

fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) {
    let path = path.as_ref();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix local bundle {} {}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let flat = root.join("flat providers");
        for provider in PROVIDER_DIRS {
            let skill = flat.join(provider).join("skills/phoenix-ui");
            write(
                skill.join("SKILL.md"),
                "---\nname: phoenix-ui\nmetadata:\n  version: 0.1.0\n---\nPhoenix local fixture.\n",
            );
            write(skill.join("scripts/phoenix-ui"), "#!/bin/sh\nexit 0\n");
            write(
                skill.join("scripts/phoenix-launcher.cjs"),
                "// deterministic local fixture\n",
            );
            write(skill.join("reference/font.woff2"), [0xff, 0x00, 0x91, 0x42]);
            impeccable_skills::util::set_executable(
                &skill.join("scripts/phoenix-ui").to_string_lossy(),
            )
            .unwrap();
        }
        write(flat.join(".claude/settings.json"), json!({"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"\".claude/skills/phoenix-ui/scripts/phoenix-ui\" hook"}]}]}}).to_string());
        write(flat.join(".codex/hooks.json"), json!({"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"\".agents/skills/phoenix-ui/scripts/phoenix-ui\" hook"}]}]}}).to_string());
        write(
            flat.join(".opencode/commands/phoenix-ui.md"),
            "# Phoenix UI command bridge\n",
        );
        write(
            flat.join(".claude/agents/phoenix-ui-finish-reviewer.md"),
            "---\nname: phoenix-ui-finish-reviewer\n---\nReviewer.\n",
        );
        let bundle = PathBuf::from(support::pack(&flat.to_string_lossy()));
        let project = root.join("project with spaces");
        let home = root.join("isolated home");
        let tmp = root.join("staging");
        for dir in [&project, &home, &tmp] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::create_dir(project.join(".git")).unwrap();
        Self {
            root,
            bundle,
            project,
            home,
            tmp,
        }
    }

    fn env(&self, bundle: bool) -> HashMap<String, String> {
        let mut env = HashMap::from([
            ("HOME".into(), self.home.to_string_lossy().into_owned()),
            (
                "USERPROFILE".into(),
                self.home.to_string_lossy().into_owned(),
            ),
            ("TMPDIR".into(), self.tmp.to_string_lossy().into_owned()),
            ("TEMP".into(), self.tmp.to_string_lossy().into_owned()),
            (
                "PHOENIX_UI_DOWNLOAD_BASE".into(),
                "https://invalid.example/no-network".into(),
            ),
        ]);
        if bundle {
            env.insert(
                "PHOENIX_UI_BUNDLE_PATH".into(),
                self.bundle.to_string_lossy().into_owned(),
            );
        }
        env
    }

    fn sys(&self) -> Sys {
        Sys::new(self.env(true), self.project.to_string_lossy().into_owned())
    }

    fn run(&self, args: &[&str], bundle: bool) -> (i32, String, String) {
        let (mut io, capture) = Io::captured("", self.project.clone(), self.env(bundle));
        let code = impeccable_skills::run(
            &args.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
            &mut io,
        );
        let stdout = String::from_utf8(capture.stdout.borrow().clone()).unwrap();
        let stderr = String::from_utf8(capture.stderr.borrow().clone()).unwrap();
        (code, stdout, stderr)
    }

    fn untouched(&self) {
        assert_eq!(std::fs::read_dir(&self.project).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(&self.home).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&self.tmp).unwrap().count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn bare_operational_entries_return_unavailable_without_writes() {
    let fixture = Fixture::new();
    for verb in ["install", "update", "link", "check"] {
        let (code, stdout, stderr) = fixture.run(&[verb, "--json", "--yes"], false);
        assert_eq!(code, 4, "{verb}: {stderr}");
        assert_eq!(
            serde_json::from_str::<Value>(&stdout).unwrap()["status"],
            "unavailable"
        );
        fixture.untouched();
    }
}

#[test]
fn help_is_static_and_preserves_all_24_command_contracts() {
    let fixture = Fixture::new();
    let (code, stdout, stderr) = fixture.run(&["help", "--json"], false);
    assert_eq!(code, 0, "{stderr}");
    let value: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["commands"].as_object().unwrap().len(), 24);
    for command in ["init", "shape", "generate", "live", "layout", "typeset"] {
        assert!(value["commands"][command]["description"].as_str().is_some());
    }
    for verb in ["install", "link", "update", "check"] {
        assert_eq!(fixture.run(&[verb, "--help"], false).0, 0);
    }
    fixture.untouched();
}

#[test]
fn local_install_check_update_preserve_providers_and_unrelated_skills() {
    let fixture = Fixture::new();
    write(
        fixture.project.join(".claude/skills/user-skill/SKILL.md"),
        "user-owned\n",
    );
    let providers = PROVIDER_DIRS.join(",");
    let (code, stdout, stderr) = fixture.run(
        &[
            "install",
            "-y",
            "--project",
            &format!("--providers={providers}"),
        ],
        true,
    );
    assert_eq!(code, 0, "{stdout}\n{stderr}");
    assert_eq!(PROVIDER_DIRS.len(), 18);
    for provider in PROVIDER_DIRS {
        let skill = fixture.project.join(provider).join("skills/phoenix-ui");
        assert!(skill.join("SKILL.md").is_file(), "missing {provider}");
        assert_eq!(
            std::fs::read(skill.join("scripts/ENGINE.json")).unwrap(),
            std::fs::read(fixture.bundle.parent().unwrap().join("VERSION.json")).unwrap()
        );
    }
    assert!(!fixture.project.join(".codex/skills").exists());
    let hooks = std::fs::read_to_string(fixture.project.join(".codex/hooks.json")).unwrap();
    assert!(hooks.contains(".agents/skills/phoenix-ui/scripts/phoenix-ui"));
    assert!(hooks.contains("commandWindows"));
    assert!(fixture
        .project
        .join(".claude/agents/phoenix-ui-finish-reviewer.md")
        .is_file());
    assert!(fixture
        .project
        .join(".opencode/commands/phoenix-ui.md")
        .is_file());
    assert!(fixture
        .run(&["check", "--json"], true)
        .1
        .contains("\"current\""));
    write(
        fixture
            .project
            .join(".veto/skills/phoenix-ui/reference/font.woff2"),
        [0x01, 0x02],
    );
    assert!(fixture
        .run(&["check", "--json"], true)
        .1
        .contains("\"drift\""));
    let updated = fixture.run(&["update", "-y", "--project", "--no-hooks"], true);
    assert_eq!(updated.0, 0, "{}\n{}", updated.1, updated.2);
    assert!(fixture
        .run(&["check", "--json"], true)
        .1
        .contains("\"current\""));
    assert_eq!(
        std::fs::read(fixture.project.join(".claude/skills/user-skill/SKILL.md")).unwrap(),
        b"user-owned\n"
    );
    assert_eq!(std::fs::read_dir(&fixture.home).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&fixture.tmp).unwrap().count(), 0);
    let local = fixture.run(&["check", "--json"], false);
    assert_eq!(local.0, 0, "{}", local.2);
    let local: Value = serde_json::from_str(&local.1).unwrap();
    assert_eq!(local["status"], "local");
    assert_eq!(local["bundleVerified"], false);
}

#[test]
fn flag_source_with_spaces_overrides_environment_and_creates_usable_links() {
    let fixture = Fixture::new();
    let path = fixture.bundle.to_string_lossy();
    let linked = fixture.run(
        &[
            "link",
            "-y",
            "--bundle-root",
            &path,
            "--providers=claude,codex,veto",
        ],
        false,
    );
    assert_eq!(linked.0, 0, "{}\n{}", linked.1, linked.2);
    for provider in [".claude", ".agents", ".veto"] {
        let skill = fixture.project.join(provider).join("skills/phoenix-ui");
        assert!(skill.is_symlink());
        assert!(skill.join("scripts/ENGINE.json").is_file());
        assert!(skill
            .canonicalize()
            .unwrap()
            .starts_with(fixture.project.join(".phoenix-ui/linked-bundles")));
    }
    assert_eq!(
        fixture
            .run(
                &[
                    "link",
                    "-y",
                    "--bundle-root",
                    &path,
                    "--providers=claude,codex,veto"
                ],
                false
            )
            .0,
        0
    );
    assert_eq!(std::fs::read_dir(&fixture.tmp).unwrap().count(), 0);
}

#[test]
fn mismatched_manifest_or_blob_aborts_before_any_project_write() {
    let fixture = Fixture::new();
    let mut bytes = std::fs::read(fixture.bundle.join("manifest.json")).unwrap();
    bytes.push(b' ');
    std::fs::write(fixture.bundle.join("manifest.json"), bytes).unwrap();
    for verb in ["install", "update", "link", "check"] {
        let result = fixture.run(&[verb, "--json", "-y"], true);
        assert_eq!(result.0, 4, "{verb}: {} {}", result.1, result.2);
        assert_eq!(
            serde_json::from_str::<Value>(&result.1).unwrap()["status"],
            "invalid"
        );
        fixture.untouched();
    }

    let fixture = Fixture::new();
    let entry = std::fs::read_dir(fixture.bundle.join("blobs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    std::fs::write(entry.path(), b"tampered").unwrap();
    for verb in ["install", "link"] {
        let result = fixture.run(&[verb, "-y", "--providers=claude"], true);
        assert_ne!(result.0, 0, "{verb}: {} {}", result.1, result.2);
        assert!(!fixture.project.join(".claude/skills").exists());
    }
}

#[test]
fn staging_adds_engine_manifest_and_preserves_exec_and_binary_data() {
    let fixture = Fixture::new();
    let bundle = LocalBundle::load(&fixture.sys()).unwrap();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(fixture.bundle.join("manifest.json")).unwrap())
            .unwrap();
    for entry in manifest["providers"].as_object().unwrap().values() {
        assert!(entry["files"]
            .get("skills/phoenix-ui/scripts/ENGINE.json")
            .is_none());
    }
    let version = std::fs::read(fixture.bundle.parent().unwrap().join("VERSION.json")).unwrap();
    assert!(!fixture
        .bundle
        .join("blobs")
        .join(support::sha(&version))
        .exists());
    let staging = bundle.stage(&fixture.sys()).unwrap();
    let skill = Path::new(&staging).join(".veto/skills/phoenix-ui");
    assert_eq!(
        std::fs::read(skill.join("scripts/ENGINE.json")).unwrap(),
        version
    );
    assert_eq!(
        std::fs::read(skill.join("reference/font.woff2")).unwrap(),
        [0xff, 0x00, 0x91, 0x42]
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(
            std::fs::metadata(skill.join("scripts/phoenix-ui"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
        assert_eq!(
            std::fs::metadata(skill.join("SKILL.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
    impeccable_skills::util::rm_rf(&staging);
}
