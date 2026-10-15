//! `phoenix-ui skills install|update` is the compatibility spelling of the
//! top-level verbs; its project branch must reach the transactional installer.
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-skills-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("project")).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        Self(root.canonicalize().unwrap())
    }

    fn project(&self) -> PathBuf {
        self.0.join("project")
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let home = self.0.join("home");
        let output = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
            .args(args)
            .current_dir(self.project())
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env_remove("PHOENIX_UI_BUNDLE_PATH")
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "{args:?}: {e}; stdout: {stdout}; stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output.status.code().unwrap(), value)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bundle() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join("dist/phoenix-ui/bundle")
        .to_string_lossy()
        .into_owned()
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.insert(path.clone(), Vec::new());
                stack.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    files
}

#[test]
fn skills_namespace_project_install_and_update_are_transactional() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let bundle = bundle();
    let args = |verb: &'static str, dry_run: bool| {
        let mut args = vec![
            "skills",
            verb,
            "--project",
            "--providers=claude",
            "--yes",
            "--bundle-root",
            &bundle,
            "--json",
        ];
        if dry_run {
            args.push("--dry-run");
        }
        args
    };
    for verb in ["install", "update"] {
        let before = tree(&project);
        let (code, report) = sandbox.run(&args(verb, true));
        assert_eq!(
            (code, report["status"].as_str()),
            (0, Some("planned")),
            "{verb}: {report}"
        );
        assert_eq!(
            tree(&project),
            before,
            "{verb} --dry-run wrote project files"
        );
    }

    let (code, installed) = sandbox.run(&args("install", false));
    assert_eq!(
        (code, installed["status"].as_str()),
        (0, Some("installed")),
        "{installed}"
    );
    assert!(project.join(".phoenix-ui/install/active.json").is_file());
    let id = installed["transactionId"].as_str().unwrap();
    assert!(project
        .join(format!(".phoenix-ui/install/journals/{id}.json"))
        .is_file());
    let (code, again) = sandbox.run(&args("update", false));
    assert_eq!(
        (code, again["status"].as_str()),
        (0, Some("unchanged")),
        "{again}"
    );

    // A user change to an owned file is a conflict, never overwritten by an update.
    let skill = project.join(".claude/skills/phoenix-ui/SKILL.md");
    let mut edited = fs::read(&skill).unwrap();
    edited.extend_from_slice(b"\nlocal note\n");
    fs::write(&skill, &edited).unwrap();
    let (code, conflict) = sandbox.run(&args("update", false));
    assert_eq!(
        (code, conflict["status"].as_str()),
        (3, Some("conflict")),
        "{conflict}"
    );
    assert_eq!(fs::read(&skill).unwrap(), edited);
}
