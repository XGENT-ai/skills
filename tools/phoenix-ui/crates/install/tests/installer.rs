use impeccable_common::Io;
use phoenix_ui_install::{
    cli,
    installer::{self, Request, Status},
    sha256, Checkpoint,
};
use serde_json::{json, Value};
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
        let path = std::env::temp_dir().join(format!(
            "installer-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("project with spaces")).unwrap();
        fs::create_dir_all(path.join("user home")).unwrap();
        Self(path.canonicalize().unwrap())
    }
    fn materialize(version: &str, providers: &[&str], scenarios: &[&str]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "installer-legacy-{}-{}",
            std::process::id(),
            sha256(format!("{version}{providers:?}{scenarios:?}").as_bytes())
        ));
        let _ = fs::remove_dir_all(&root);
        let options = json!({"fixtureRoot": root, "packageVersion": version, "providers": providers, "scenarios": scenarios, "channel": "xgent-installer"});
        let output = Command::new("node")
            .args(["--input-type=module", "-e", "import {pathToFileURL} from 'node:url'; const {materializeMigrationFixture}=await import(pathToFileURL(process.argv[2])); materializeMigrationFixture(JSON.parse(process.argv[3]));", "installer-fixture"])
            .arg(repository().join("test/phoenix-ui/fixtures/migration/materialize.mjs"))
            .arg(options.to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self(root.canonicalize().unwrap())
    }
    fn project(&self) -> PathBuf {
        self.0.join("project with spaces")
    }
    fn home(&self) -> PathBuf {
        self.0.join("user home")
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .to_path_buf()
}
fn bundle() -> &'static Path {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| repository().join("vendor/phoenix-ui/bundle"))
}
fn auxiliary() -> &'static Path {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| repository().join("skills"))
}

fn request<'a>(
    sandbox: &'a Sandbox,
    project: &'a Path,
    providers: &[&str],
    dry_run: bool,
    id: &str,
) -> Request<'a> {
    let _ = sandbox;
    Request {
        project,
        bundle_root: bundle(),
        auxiliary_root: Some(auxiliary()),
        providers: providers.iter().map(|p| p.to_string()).collect(),
        home: Some(sandbox.home()),
        dry_run,
        transaction_id: id.into(),
    }
}

/// Every entry below `root` except Phoenix's own state directory.
fn entries(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative == ".phoenix-ui" {
                continue;
            }
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                out.insert(relative, "dir".into());
                walk(root, &path, out);
            } else {
                out.insert(relative, sha256(&fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn run_cli(args: &[&str], cwd: &Path, home: &Path) -> (i32, String, String) {
    let mut env = std::collections::HashMap::new();
    env.insert("HOME".to_string(), home.to_string_lossy().into_owned());
    env.insert(
        "USERPROFILE".to_string(),
        home.to_string_lossy().into_owned(),
    );
    let (mut io, captured) = Io::captured("", cwd.to_path_buf(), env);
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let code = if args[0] == "migrate" {
        cli::run_migrate(&args[1..], &mut io)
    } else {
        cli::run_install(&args[1..], &mut io)
    };
    drop(io);
    let stdout = String::from_utf8_lossy(&captured.stdout.borrow()).into_owned();
    let stderr = String::from_utf8_lossy(&captured.stderr.borrow()).into_owned();
    (code, stdout, stderr)
}

#[test]
fn fresh_install_is_idempotent_and_rollback_leaves_the_original_tree() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    fs::write(project.join("package.json"), "{}\n").unwrap();
    let before = entries(&project);
    let planned = installer::install(&request(
        &sandbox,
        &project,
        &[".claude"],
        true,
        "fresh-dry",
    ))
    .unwrap();
    assert_eq!(planned.status, Status::Planned);
    assert!(planned
        .writes
        .iter()
        .any(|p| p == ".claude/skills/phoenix-ui/scripts/ENGINE.json"));
    assert!(planned
        .writes
        .iter()
        .any(|p| p == ".claude/skills/review-ui/SKILL.md"));
    assert_eq!(entries(&project), before, "dry-run wrote project entries");
    assert!(!project.join(".phoenix-ui").exists());

    let installed =
        installer::install(&request(&sandbox, &project, &[".claude"], false, "fresh-1")).unwrap();
    assert_eq!(installed.status, Status::Installed);
    assert!(
        installed.transition_entries.is_empty(),
        "no old skill, no transitional entry"
    );
    let engine = fs::read(project.join(".claude/skills/phoenix-ui/scripts/ENGINE.json")).unwrap();
    assert_eq!(
        engine,
        fs::read(bundle().parent().unwrap().join("VERSION.json")).unwrap()
    );
    assert_eq!(
        fs::read(project.join(".claude/skills/design-md/SKILL.md")).unwrap(),
        fs::read(auxiliary().join("design-md/SKILL.md")).unwrap()
    );
    let again =
        installer::install(&request(&sandbox, &project, &[".claude"], false, "fresh-2")).unwrap();
    assert_eq!(again.status, Status::Unchanged);
    assert!(
        !project.join(".phoenix-ui/migrations/fresh-2").exists(),
        "an unchanged run must not open a transaction"
    );

    let rolled = installer::rollback(&project, "fresh-1").unwrap();
    assert_eq!(rolled.status, Status::RolledBack);
    let after: BTreeMap<_, _> = entries(&project)
        .into_iter()
        .filter(|(path, kind)| {
            !(kind == "dir"
                && [".claude", ".claude/skills", ".claude/agents"].contains(&path.as_str()))
        })
        .collect();
    assert_eq!(
        after, before,
        "rollback must remove Phoenix files and the directories it left empty"
    );
}

#[test]
fn migration_retires_owned_old_entries_keeps_user_data_and_rolls_back_exactly() {
    let sandbox = Sandbox::materialize(
        "0.6.0",
        &[".claude", ".agents"],
        &[
            "claude-shared-and-local",
            "mixed-handlers",
            "custom-allow",
            "pin-shortcut",
        ],
    );
    let project = sandbox.project();
    let before = entries(&project);
    let providers = [".agents", ".claude", ".codex"];
    let report =
        installer::install(&request(&sandbox, &project, &providers, false, "migrate-1")).unwrap();
    assert_eq!(report.status, Status::Installed, "{:?}", report.conflicts);
    assert_eq!(
        report.transition_entries,
        vec![
            ".agents/skills/impeccable/SKILL.md",
            ".claude/skills/impeccable/SKILL.md"
        ]
    );
    assert!(report.migration_sources.iter().any(|s| s == "0.6.0"));
    let transition =
        fs::read_to_string(project.join(".agents/skills/impeccable/SKILL.md")).unwrap();
    assert!(
        transition.contains("`$phoenix-ui <command>`")
            && transition.contains("disable-model-invocation: true")
    );
    assert!(
        fs::read_to_string(project.join(".claude/skills/impeccable/SKILL.md"))
            .unwrap()
            .contains("`/phoenix-ui <command>`")
    );
    assert!(
        !project.join(".claude/skills/impeccable/scripts").exists(),
        "retired old tree left empty directories"
    );
    let settings: Value =
        serde_json::from_slice(&fs::read(project.join(".claude/settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["permissions"]["allow"],
        json!(["Read", "Bash(git status:*)"])
    );
    let settings_text = serde_json::to_string(&settings).unwrap();
    assert!(
        settings_text.contains("phoenix-ui")
            && !settings_text.contains("skills/impeccable/scripts")
    );
    assert!(
        fs::read_to_string(project.join(".claude/skills/polish/SKILL.md"))
            .unwrap()
            .contains("phoenix-ui-pinned-skill")
    );
    assert!(report.findings.iter().any(|f| f.path == "AGENTS.md"
        && f.kind == "document-old-entry"
        && !f.blocking_integration));
    for user in [
        "AGENTS.md",
        "CLAUDE.md",
        "PRODUCT.md",
        "DESIGN.md",
        ".claude/skills/user-helper/SKILL.md",
    ] {
        assert_eq!(
            entries(&project).get(user),
            before.get(user),
            "{user} changed"
        );
    }
    let again =
        installer::install(&request(&sandbox, &project, &providers, false, "migrate-2")).unwrap();
    assert_eq!(again.status, Status::Unchanged, "{:?}", again.conflicts);

    let rolled = installer::rollback(&project, "migrate-1").unwrap();
    assert_eq!(rolled.status, Status::RolledBack, "{:?}", rolled.conflicts);
    let after: BTreeMap<_, _> = entries(&project)
        .into_iter()
        .filter(|(path, _)| before.contains_key(path))
        .collect();
    assert_eq!(after, before, "rollback did not restore the original bytes");
    let leftovers: Vec<_> = entries(&project)
        .into_keys()
        .filter(|p| !before.contains_key(p))
        .collect();
    assert!(
        leftovers.is_empty(),
        "rollback left new entries: {leftovers:?}"
    );
}

#[test]
fn external_auxiliary_copies_are_kept_and_only_a_different_copy_keeps_integration_pending() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let copy = |skill: &str| {
        for entry in fs::read_dir(auxiliary().join(skill)).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                let target = project
                    .join(".claude/skills")
                    .join(skill)
                    .join(entry.file_name());
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::copy(entry.path(), target).unwrap();
            }
        }
    };
    copy("design-md");
    fs::create_dir_all(project.join(".claude/skills/design-md/references")).unwrap();
    fs::copy(
        auxiliary().join("design-md/references/format.md"),
        project.join(".claude/skills/design-md/references/format.md"),
    )
    .unwrap();
    fs::create_dir_all(project.join(".claude/skills/design-md/evals")).unwrap();
    fs::copy(
        auxiliary().join("design-md/evals/scenarios.md"),
        project.join(".claude/skills/design-md/evals/scenarios.md"),
    )
    .unwrap();
    let identical =
        installer::install(&request(&sandbox, &project, &[".claude"], false, "aux-1")).unwrap();
    assert_eq!(identical.status, Status::Installed);
    assert!(identical
        .findings
        .iter()
        .any(|f| f.path == ".claude/skills/design-md"
            && f.kind == "external-copy-identical"
            && !f.blocking_integration));
    assert!(
        !identical
            .writes
            .iter()
            .any(|p| p.starts_with(".claude/skills/design-md/")),
        "an external copy must not become Phoenix-owned"
    );

    fs::write(
        project.join(".claude/skills/review-ui/SKILL.md"),
        "user edit\n",
    )
    .unwrap();
    let edited =
        installer::install(&request(&sandbox, &project, &[".claude"], false, "aux-2")).unwrap();
    assert_eq!(
        edited.status,
        Status::Conflict,
        "a modified Phoenix-owned file must not be overwritten"
    );

    let other = Sandbox::new();
    let project = other.project();
    fs::create_dir_all(project.join(".claude/skills/review-ui")).unwrap();
    fs::write(
        project.join(".claude/skills/review-ui/SKILL.md"),
        "external version\n",
    )
    .unwrap();
    let pending =
        installer::install(&request(&other, &project, &[".claude"], false, "aux-3")).unwrap();
    assert_eq!(pending.status, Status::IntegrationPending);
    assert_eq!(pending.status.exit_code(), 3);
    assert!(
        project.join(".claude/skills/phoenix-ui/SKILL.md").is_file(),
        "Phoenix still activates"
    );
    assert_eq!(
        fs::read_to_string(project.join(".claude/skills/review-ui/SKILL.md")).unwrap(),
        "external version\n"
    );
}

#[test]
fn personal_old_entries_block_activation_without_writing() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    fs::create_dir_all(sandbox.home().join(".claude/skills/impeccable")).unwrap();
    fs::write(
        sandbox.home().join(".claude/skills/impeccable/SKILL.md"),
        "old\n",
    )
    .unwrap();
    let before = entries(&project);
    let report = installer::install(&request(
        &sandbox,
        &project,
        &[".claude"],
        false,
        "personal-1",
    ))
    .unwrap();
    assert_eq!(report.status, Status::Conflict);
    assert!(report
        .conflicts
        .iter()
        .any(|c| c.path.ends_with(".claude/skills/impeccable")));
    assert_eq!(entries(&project), before);
    assert!(!project.join(".phoenix-ui/install/active.json").exists());

    let shadow = Sandbox::new();
    let project = shadow.project();
    fs::create_dir_all(shadow.home().join(".claude/skills/review-ui")).unwrap();
    fs::write(
        shadow.home().join(".claude/skills/review-ui/SKILL.md"),
        "personal\n",
    )
    .unwrap();
    let report = installer::install(&request(
        &shadow,
        &project,
        &[".claude"],
        false,
        "personal-2",
    ))
    .unwrap();
    assert_eq!(report.status, Status::IntegrationPending);
    assert!(report
        .findings
        .iter()
        .any(|f| f.kind == "personal-copy-differs"
            && f.message.contains("Claude loads this personal copy")));
}

#[test]
fn recovering_an_interrupted_install_leaves_no_directories_that_hide_auxiliary_skills() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        installer::install_with(
            &request(&sandbox, &project, &[".claude"], false, "interrupted"),
            |point| {
                if matches!(&point, Checkpoint::AfterReplace(path) if path.starts_with(".claude/skills/design-reference/"))
                {
                    panic!("fixture interruption");
                }
            },
        )
    }));
    assert!(interrupted.is_err());
    assert!(project.join(".claude/skills/apply-ui-review").exists());
    let report = installer::install(&request(
        &sandbox,
        &project,
        &[".claude"],
        false,
        "after-interruption",
    ))
    .unwrap();
    assert_eq!(report.recovered.as_deref(), Some("rolled-back:interrupted"));
    assert_eq!(report.status, Status::Installed, "{:?}", report.findings);
    for skill in ["apply-ui-review", "design-md", "design-reference"] {
        assert!(
            project
                .join(".claude/skills")
                .join(skill)
                .join("SKILL.md")
                .is_file(),
            "{skill}"
        );
    }
}

#[test]
fn personal_older_upstream_commands_are_reported_without_blocking() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let skills = sandbox.home().join(".agents/skills");
    for (name, body) in [
        ("polish", "Run /teach-impeccable first.\n"),
        ("audit", "Quality checks.\n"),
        ("my-notes", "Personal notes.\n"),
    ] {
        fs::create_dir_all(skills.join(name)).unwrap();
        fs::write(skills.join(name).join("SKILL.md"), body).unwrap();
    }
    // skills.sh records where each global skill came from.
    fs::write(
        sandbox.home().join(".agents/.skill-lock.json"),
        r#"{"version":3,"skills":{"audit":{"source":"pbakaus/impeccable"},"my-notes":{"source":"me/notes"}}}"#,
    )
    .unwrap();
    let report = installer::install(&request(
        &sandbox,
        &project,
        &[".agents", ".codex"],
        false,
        "personal-old-commands",
    ))
    .unwrap();
    assert_eq!(report.status, Status::Installed);
    let old: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.kind == "personal-old-command")
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(old.len(), 2, "{old:?}");
    assert!(old.iter().any(|p| p.ends_with(".agents/skills/audit")));
    assert!(old.iter().any(|p| p.ends_with(".agents/skills/polish")));
    assert!(report
        .findings
        .iter()
        .filter(|f| f.kind == "personal-old-command")
        .all(|f| !f.blocking_integration && f.message.contains("reload")));
}

#[test]
fn cli_writes_one_json_object_and_maps_exit_codes() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let project_arg = project.to_string_lossy().into_owned();
    let bundle_arg = bundle().to_string_lossy().into_owned();
    let aux_arg = auxiliary().to_string_lossy().into_owned();
    let (code, stdout, stderr) = run_cli(
        &["install", "--project", &project_arg, "--json"],
        &project,
        &sandbox.home(),
    );
    assert_eq!((code, stdout.as_str()), (2, ""), "{stderr}");
    assert!(stderr.contains("--bundle-root"));
    let (code, _, stderr) = run_cli(
        &[
            "install",
            "--project",
            &project_arg,
            "--bundle-root",
            &bundle_arg,
            "--providers",
            "nope",
        ],
        &project,
        &sandbox.home(),
    );
    assert_eq!(code, 2, "{stderr}");
    let (code, stdout, stderr) = run_cli(
        &[
            "install",
            "--project",
            &project_arg,
            "--bundle-root",
            &bundle_arg,
            "--auxiliary-root",
            &aux_arg,
            "--providers=codex",
            "--dry-run",
            "--json",
        ],
        &project,
        &sandbox.home(),
    );
    assert_eq!(code, 0, "{stderr}");
    let value: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["providers"], json!([".agents", ".codex"]));
    assert_eq!(value["status"], "planned");
    let (code, stdout, _) = run_cli(
        &[
            "install",
            "--project",
            &project_arg,
            "--bundle-root",
            &bundle_arg,
            "--transaction-id",
            "cli-1",
        ],
        &project,
        &sandbox.home(),
    );
    assert_eq!(code, 0);
    assert!(
        stdout.contains("installed")
            && stdout.contains("phoenix-ui migrate --rollback cli-1 --project")
    );
    let (code, stdout, _) = run_cli(
        &[
            "migrate",
            "--rollback",
            "cli-1",
            "--project",
            &project_arg,
            "--json",
        ],
        &project,
        &sandbox.home(),
    );
    assert_eq!(code, 0);
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["status"],
        "rolled-back"
    );
    let (code, _, stderr) = run_cli(
        &["migrate", "--project", &project_arg],
        &project,
        &sandbox.home(),
    );
    assert_eq!(code, 2, "{stderr}");
}

/// Codex and Claude allow several hook groups without a matcher in one event;
/// XGENT's own goal guard adds one next to Phoenix's (and, in 0.5/0.6, next to
/// the old impeccable group). Equal matcher identity must not block install,
/// a guard rewrite that reorders the groups, migration or rollback.
#[test]
fn neighboring_hook_groups_without_a_matcher_do_not_make_phoenix_ambiguous() {
    let guard = json!({"hooks": [{"type": "command", "command": "python3 \"$(git rev-parse --show-toplevel)/.codex/hooks/context-goal-guard.py\" --threshold 65", "timeout": 3}]});
    let write_guard = |project: &Path| {
        let file = project.join(".codex/hooks.json");
        let mut value: Value = fs::read(&file)
            .ok()
            .map(|b| serde_json::from_slice(&b).unwrap())
            .unwrap_or_else(|| json!({}));
        for event in ["PreToolUse", "Stop"] {
            let hooks = value
                .as_object_mut()
                .unwrap()
                .entry("hooks")
                .or_insert_with(|| json!({}));
            let groups = hooks
                .as_object_mut()
                .unwrap()
                .entry(event)
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap();
            // The Node guard strips its own group and appends it again.
            groups.retain(|g| *g != guard);
            groups.push(guard.clone());
        }
        fs::create_dir_all(project.join(".codex")).unwrap();
        fs::write(&file, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
    };

    let sandbox = Sandbox::new();
    let project = sandbox.project();
    fs::write(project.join("package.json"), "{}\n").unwrap();
    write_guard(&project);
    let original = entries(&project);
    let providers = [".agents", ".codex"];
    let first =
        installer::install(&request(&sandbox, &project, &providers, false, "groups-1")).unwrap();
    assert_eq!(first.status, Status::Installed, "{:?}", first.conflicts);
    write_guard(&project);
    let second =
        installer::install(&request(&sandbox, &project, &providers, false, "groups-2")).unwrap();
    assert_eq!(second.status, Status::Unchanged, "{:?}", second.conflicts);
    let rolled = installer::rollback(&project, "groups-1").unwrap();
    assert_eq!(rolled.status, Status::RolledBack);
    let left: Vec<_> = entries(&project)
        .into_iter()
        .filter(|(path, kind)| {
            !(kind == "dir" && [".agents", ".agents/skills"].contains(&path.as_str()))
        })
        .map(|(path, _)| path)
        .collect();
    assert_eq!(left, original.keys().cloned().collect::<Vec<_>>());
    let codex: Value =
        serde_json::from_slice(&fs::read(project.join(".codex/hooks.json")).unwrap()).unwrap();
    for event in ["PreToolUse", "Stop"] {
        assert_eq!(codex["hooks"][event], json!([guard.clone()]), "{event}");
    }

    let legacy = Sandbox::materialize("0.6.0", &[".agents"], &[]);
    let project = legacy.project();
    write_guard(&project);
    let before = entries(&project);
    let read_codex = |project: &Path| -> Value {
        serde_json::from_slice(&fs::read(project.join(".codex/hooks.json")).unwrap()).unwrap()
    };
    let codex_before = read_codex(&project);
    let migrated = installer::install(&request(
        &legacy,
        &project,
        &providers,
        false,
        "legacy-groups",
    ))
    .unwrap();
    assert_eq!(
        migrated.status,
        Status::Installed,
        "{:?}",
        migrated.conflicts
    );
    let codex = read_codex(&project);
    let commands: Vec<String> = codex["hooks"]["Stop"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["hooks"].as_array().unwrap().iter())
        .map(|h| h["command"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        commands
            .iter()
            .filter(|c| c.contains("context-goal-guard.py"))
            .count(),
        1
    );
    assert_eq!(
        commands
            .iter()
            .filter(|c| c.contains("skills/phoenix-ui/scripts/"))
            .count(),
        1
    );
    assert!(!commands
        .iter()
        .any(|c| c.contains("skills/impeccable/scripts/")));
    write_guard(&project);
    let again = installer::install(&request(
        &legacy,
        &project,
        &providers,
        false,
        "legacy-groups-2",
    ))
    .unwrap();
    assert_eq!(again.status, Status::Unchanged, "{:?}", again.conflicts);
    let rolled = installer::rollback(&project, "legacy-groups").unwrap();
    assert_eq!(rolled.status, Status::RolledBack);
    // The guard rewrote the file after migration; groups come back, its order stays.
    let after = entries(&project);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    for (path, hash) in &before {
        if path != ".codex/hooks.json" {
            assert_eq!(after.get(path), Some(hash), "{path}");
        }
    }
    let groups = |value: &Value, event: &str| {
        let mut out: Vec<String> = value["hooks"][event]
            .as_array()
            .unwrap()
            .iter()
            .map(Value::to_string)
            .collect();
        out.sort();
        out
    };
    let codex_after = read_codex(&project);
    for event in ["PostToolUse", "PreToolUse", "Stop"] {
        assert_eq!(
            groups(&codex_after, event),
            groups(&codex_before, event),
            "{event}"
        );
    }
}

/// Backups and the install lock are machine-local: they go to the repository's
/// `.git/info/exclude`, never to a committed ignore file.
#[test]
fn backups_and_lock_are_excluded_locally_once_without_touching_committed_files() {
    let sandbox = Sandbox::new();
    let repo = sandbox.project();
    let project = repo.join("apps/web");
    fs::create_dir_all(repo.join(".git/info")).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::write(repo.join(".git/info/exclude"), "# user rule\n*.log").unwrap();
    fs::write(project.join("package.json"), "{}\n").unwrap();
    let dry = installer::install(&request(
        &sandbox,
        &project,
        &[".claude"],
        true,
        "exclude-dry",
    ))
    .unwrap();
    assert_eq!(dry.status, Status::Planned);
    assert_eq!(
        fs::read_to_string(repo.join(".git/info/exclude")).unwrap(),
        "# user rule\n*.log"
    );
    installer::install(&request(
        &sandbox,
        &project,
        &[".claude"],
        false,
        "exclude-1",
    ))
    .unwrap();
    let first = fs::read_to_string(repo.join(".git/info/exclude")).unwrap();
    assert!(first.starts_with("# user rule\n*.log\n"), "{first}");
    assert!(first.contains("**/.phoenix-ui/migrations/\n"), "{first}");
    assert!(
        first.contains("**/.phoenix-ui/install/install.lock\n"),
        "{first}"
    );
    assert!(!project.join(".gitignore").exists() && !repo.join(".gitignore").exists());
    fs::write(project.join("extra.txt"), "change\n").unwrap();
    installer::install(&request(
        &sandbox,
        &project,
        &[".claude", ".cursor"],
        false,
        "exclude-2",
    ))
    .unwrap();
    assert_eq!(
        fs::read_to_string(repo.join(".git/info/exclude")).unwrap(),
        first
    );
}

/// The inherited `--project` is a boolean for the current project; project
/// installs refuse the user-level-only options instead of ignoring them.
#[test]
fn inherited_install_flags_select_the_project_and_reject_user_level_options() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    fs::write(project.join("package.json"), "{}\n").unwrap();
    let bundle = bundle().to_string_lossy().into_owned();
    for args in [
        vec![
            "install",
            "--project",
            "--dry-run",
            "--json",
            "--bundle-root",
            &bundle,
        ],
        vec![
            "install",
            "--scope=project",
            "-y",
            "--dry-run",
            "--json",
            "--bundle-root",
            &bundle,
        ],
        vec!["install", "--dry-run", "--json", "--bundle-root", &bundle],
    ] {
        let (code, stdout, stderr) = run_cli(&args, &project, &sandbox.home());
        assert_eq!(code, 0, "{args:?}: {stderr}");
        let report: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["status"], "planned");
        assert_eq!(report["project"], project.display().to_string());
    }
    for flag in ["--force", "--no-hooks", "--scope=global"] {
        let (code, stdout, stderr) = run_cli(
            &["install", flag, "--bundle-root", &bundle],
            &project,
            &sandbox.home(),
        );
        assert_eq!(code, 2, "{flag}");
        assert!(stdout.is_empty());
        assert!(
            stderr.contains("--user") || stderr.contains("--global"),
            "{stderr}"
        );
    }
    assert!(!project.join(".phoenix-ui").exists());
}
