use impeccable_common::Io;
use phoenix_ui_install::{cli, install, Report, Status};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
    fn write(&self, path: &str, bytes: &str) {
        let file = self.0.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, bytes).unwrap();
    }
    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.0.join(path)).unwrap()
    }
    fn run(&self, providers: &str, dry_run: bool) -> Report {
        install(&self.0, &bundle(), Some(providers), dry_run).unwrap()
    }
    fn cli(&self, args: &[&str]) -> (i32, String, String) {
        let (mut io, captured) = Io::captured("", self.0.clone(), Default::default());
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let code = cli::run_install(&args, &mut io);
        drop(io);
        let stdout = String::from_utf8_lossy(&captured.stdout.borrow()).into_owned();
        let stderr = String::from_utf8_lossy(&captured.stderr.borrow()).into_owned();
        (code, stdout, stderr)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bundle() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join("dist/phoenix-ui/bundle")
}

#[test]
fn install_overwrites_tools_removes_old_state_and_repeats_unchanged() {
    let f = Fixture::new();
    f.write(".claude/skills/phoenix-ui/SKILL.md", "customized old copy");
    f.write(
        ".claude/skills/phoenix-ui/reference/retired.md",
        "no longer shipped",
    );
    f.write(".claude/agents/mine.md", "user agent");
    f.write(".phoenix-ui/install/active.json", "old receipt");
    f.write(".phoenix-ui/migrations/old/backup", "old backup");
    f.write(".phoenix-ui/config.json", "{\"user\":true}");

    let planned = f.run(".claude", true);
    assert_eq!(planned.status, Status::Planned);
    assert!(planned
        .writes
        .iter()
        .any(|p| p == ".claude/skills/phoenix-ui/scripts/ENGINE.json"));
    assert_eq!(planned.shared_json, [".claude/settings.json"]);
    assert_eq!(
        planned.removals,
        [
            ".claude/skills/phoenix-ui/reference/retired.md",
            ".phoenix-ui/install",
            ".phoenix-ui/migrations"
        ]
    );
    assert_eq!(
        f.read(".claude/skills/phoenix-ui/SKILL.md"),
        "customized old copy"
    );
    assert!(!f.0.join(".claude/settings.json").exists());

    assert_eq!(f.run(".claude", false).status, Status::Installed);
    assert_eq!(
        fs::read(f.0.join(".claude/skills/phoenix-ui/scripts/ENGINE.json")).unwrap(),
        fs::read(bundle().join("../VERSION.json")).unwrap()
    );
    assert_ne!(
        f.read(".claude/skills/phoenix-ui/SKILL.md"),
        "customized old copy"
    );
    assert!(!f
        .0
        .join(".claude/skills/phoenix-ui/reference/retired.md")
        .exists());
    assert_eq!(f.read(".claude/agents/mine.md"), "user agent");
    assert!(!f.0.join(".phoenix-ui/install").exists());
    assert!(!f.0.join(".phoenix-ui/migrations").exists());
    assert_eq!(f.read(".phoenix-ui/config.json"), "{\"user\":true}");
    let settings: Value = serde_json::from_str(&f.read(".claude/settings.json")).unwrap();
    assert!(settings["hooks"].to_string().contains("phoenix-ui"));
    assert!(settings.get("description").is_none(), "{settings}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let launcher = f.0.join(".claude/skills/phoenix-ui/scripts/phoenix-ui");
        assert_eq!(
            fs::metadata(launcher).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    let again = f.run(".claude", false);
    assert_eq!(again.status, Status::Unchanged);
    assert!(again.writes.is_empty() && again.shared_json.is_empty());
}

#[test]
fn hook_merge_replaces_only_phoenix_handlers() {
    let f = Fixture::new();
    let other = json!({"type":"command","command":"my-other-hook"});
    let settings = json!({"keep":true,"hooks":{"Stop":[{"matcher":"keep","hooks":[
        {"type":"command","command":".claude/skills/phoenix-ui/scripts/phoenix-ui hook"},
        other.clone()
    ]}]}});
    f.write(".claude/settings.json", &settings.to_string());
    assert_eq!(f.run(".claude", false).status, Status::Installed);
    let after: Value = serde_json::from_str(&f.read(".claude/settings.json")).unwrap();
    assert_eq!(after["keep"], true);
    assert_eq!(
        after["hooks"]["Stop"][0],
        json!({"matcher":"keep","hooks":[other]})
    );
    assert!(after["hooks"]["Stop"].as_array().unwrap().len() > 1);
    assert_eq!(f.run(".claude", false).status, Status::Unchanged);
}

#[test]
fn invalid_hook_config_fails_before_any_write() {
    let f = Fixture::new();
    f.write(".codex/hooks.json", "[]");
    f.write(".agents/skills/phoenix-ui/SKILL.md", "old tool");
    assert!(install(&f.0, &bundle(), Some("codex"), false).is_err());
    assert_eq!(f.read(".agents/skills/phoenix-ui/SKILL.md"), "old tool");
    assert_eq!(f.read(".codex/hooks.json"), "[]");
}

#[cfg(unix)]
#[test]
fn symlinked_skill_folder_is_replaced_by_a_copy_without_touching_its_target() {
    let f = Fixture::new();
    f.write("source/SKILL.md", "linked source");
    fs::create_dir_all(f.0.join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(f.0.join("source"), f.0.join(".claude/skills/phoenix-ui")).unwrap();
    let planned = f.run(".claude", true);
    assert_eq!(planned.removals, [".claude/skills/phoenix-ui"]);
    f.run(".claude", false);
    let skill = f.0.join(".claude/skills/phoenix-ui");
    assert!(fs::symlink_metadata(&skill).unwrap().is_dir());
    assert!(skill.join("scripts/ENGINE.json").exists());
    assert_eq!(f.read("source/SKILL.md"), "linked source");
    assert_eq!(fs::read_dir(f.0.join("source")).unwrap().count(), 1);
}

#[test]
fn cli_selects_providers_reports_json_and_maps_exit_codes() {
    let f = Fixture::new();
    let bundle = bundle().to_string_lossy().into_owned();
    let (code, stdout, stderr) = f.cli(&["--json"]);
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert!(stderr.contains("--bundle-root"), "{stderr}");
    for unknown in [
        &["--providers", "nope"][..],
        &["--keep-impeccable"],
        &["--yes"],
    ] {
        let mut args = vec!["--bundle-root", &bundle];
        args.extend(unknown);
        assert_eq!(f.cli(&args).0, 2, "{unknown:?}");
    }
    let missing = f.0.join("missing").to_string_lossy().into_owned();
    let (code, _, stderr) = f.cli(&["--bundle-root", &missing]);
    assert_eq!(code, 4, "{stderr}");

    let (code, stdout, stderr) = f.cli(&[
        "--project",
        "--bundle-root",
        &bundle,
        "--providers=codex",
        "--dry-run",
        "--json",
    ]);
    assert_eq!(code, 0, "{stderr}");
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["providers"], json!([".agents", ".codex"]));
    assert_eq!(report["status"], "planned");
    assert_eq!(report["project"], f.0.display().to_string());
    assert!(!f.0.join(".agents").exists());

    // Without --providers the folders already in the project are selected.
    fs::create_dir(f.0.join(".cursor")).unwrap();
    let (code, stdout, _) = f.cli(&["--bundle-root", &bundle, "--json"]);
    assert_eq!(code, 0);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["providers"], json!([".cursor"]));
    assert_eq!(report["status"], "installed");
}
