use phoenix_ui_install::installer::{install, Request, Status};
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
            "phoenix-overwrite-{}-{}",
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
    fn run(
        &self,
        dry_run: bool,
    ) -> phoenix_ui_install::Result<phoenix_ui_install::installer::Report> {
        let bundle = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .unwrap()
            .join("dist/phoenix-ui/bundle");
        install(&Request {
            project: &self.0,
            bundle_root: &bundle,
            providers: vec![".claude".into()],
            home: None,
            scan_sources: false,
            keep_impeccable: true,
            dry_run,
            transaction_id: "unused".into(),
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn overwrite_cleans_old_install_state_and_repeat_needs_no_receipt() {
    let f = Fixture::new();
    f.write(".claude/skills/phoenix-ui/SKILL.md", "customized old copy");
    f.write(".phoenix-ui/install/active.json", "broken old receipt");
    f.write(".phoenix-ui/migrations/old/backup", "old backup");
    f.write(".phoenix-ui/config.json", "{\"user\":true}");
    let planned = f.run(true).unwrap();
    assert_eq!(planned.status, Status::Planned);
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/phoenix-ui/SKILL.md")).unwrap(),
        "customized old copy"
    );
    assert!(f.0.join(".phoenix-ui/install/active.json").exists());
    let installed = f.run(false).unwrap();
    assert_eq!(installed.status, Status::Installed);
    assert!(installed.transaction_id.is_none());
    assert!(installed.backups.is_none());
    assert!(!f.0.join(".phoenix-ui/install").exists());
    assert!(!f.0.join(".phoenix-ui/migrations").exists());
    assert_eq!(
        fs::read_to_string(f.0.join(".phoenix-ui/config.json")).unwrap(),
        "{\"user\":true}"
    );
    assert_ne!(
        fs::read_to_string(f.0.join(".claude/skills/phoenix-ui/SKILL.md")).unwrap(),
        "customized old copy"
    );
    assert_eq!(f.run(false).unwrap().status, Status::Unchanged);
}
#[test]
fn overwrite_replaces_only_phoenix_handlers_in_mixed_groups() {
    let f = Fixture::new();
    let other = json!({"type":"command","command":"my-other-hook"});
    let settings = json!({"keep":true,"hooks":{"Stop":[{"matcher":"keep","hooks":[{"type":"command","command":".claude/skills/phoenix-ui/scripts/phoenix-ui hook"},other.clone()]}]}});
    f.write(".claude/settings.json", &settings.to_string());
    assert_eq!(f.run(false).unwrap().status, Status::Installed);
    let after: Value =
        serde_json::from_slice(&fs::read(f.0.join(".claude/settings.json")).unwrap()).unwrap();
    assert_eq!(after["keep"], true);
    assert_eq!(
        after["hooks"]["Stop"][0],
        json!({"matcher":"keep","hooks":[other]})
    );
    assert_eq!(f.run(false).unwrap().status, Status::Unchanged);
}
#[test]
fn invalid_shared_config_is_rejected_before_overwriting_tools() {
    let f = Fixture::new();
    f.write(".claude/settings.json", "[]");
    f.write(".claude/skills/phoenix-ui/SKILL.md", "old tool");
    assert!(f.run(false).is_err());
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/phoenix-ui/SKILL.md")).unwrap(),
        "old tool"
    );
}
