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
    PATH.get_or_init(|| repository().join("dist/phoenix-ui/bundle"))
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
        scan_sources: false,
        keep_impeccable: false,
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
fn ordinary_install_leaves_source_scanning_to_explicit_migration() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let source = project.join("src/page.html");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "<!-- impeccable-live-start -->").unwrap();
    let before = entries(&project);
    let bundle_arg = bundle().to_string_lossy().into_owned();
    for (verb, expected) in [("install", "planned"), ("migrate", "conflict")] {
        let mut args = vec![
            verb,
            "--project",
            "--bundle-root",
            &bundle_arg,
            "--dry-run",
            "--json",
        ];
        if verb == "migrate" {
            args.extend(["--from", "impeccable"]);
        }
        let (code, stdout, stderr) = run_cli(&args, &project, &sandbox.home());
        assert_eq!(code, if verb == "migrate" { 3 } else { 0 }, "{stderr}");
        let report: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["status"], expected, "{report}");
        if verb == "migrate" {
            assert!(report["conflicts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["path"] == "src/page.html"));
        }
        assert_eq!(
            entries(&project),
            before,
            "{verb} --dry-run wrote project files"
        );
    }
}

#[test]
fn ordinary_install_still_blocks_live_residue_in_fixed_legacy_paths() {
    for path in [
        ".impeccable/live/sessions/old.jsonl",
        ".impeccable-live.json",
        ".impeccable-live/session.json",
    ] {
        let sandbox = Sandbox::new();
        let project = sandbox.project();
        let file = project.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, "{}").unwrap();
        let before = entries(&project);
        let report = installer::install(&request(
            &sandbox,
            &project,
            &[".claude"],
            false,
            "fixed-residue",
        ))
        .unwrap();
        assert_eq!(report.status, Status::Conflict, "{path}: {report:?}");
        assert_eq!(
            entries(&project),
            before,
            "fixed legacy residue must block before writes"
        );
    }
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
    fs::copy(
        auxiliary().join("design-md/evals/fresh-context.md"),
        project.join(".claude/skills/design-md/evals/fresh-context.md"),
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

fn git(cwd: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", cwd.join("no-global-config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
        .status
        .success()
}

/// A migrated `config.local.json` holds machine paths and personal choices; it is
/// ignored as soon as the install finishes, in a subproject and in a linked
/// worktree, and a reinstall adds the line to an exclude block written earlier.
#[test]
fn migrated_local_config_is_ignored_and_a_reinstall_restores_the_protection() {
    let sandbox = Sandbox::new();
    let repo = sandbox.project();
    assert!(git(&repo, &["init", "-q"]));
    assert!(git(
        &repo,
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init"
        ]
    ));
    let linked = sandbox.0.join("linked worktree");
    assert!(git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &linked.to_string_lossy()
        ]
    ));
    let local = "apps/web/.phoenix-ui/config.local.json";
    for (root, id) in [(&repo, "local-config-1"), (&linked, "local-config-2")] {
        let project = root.join("apps/web");
        fs::create_dir_all(project.join(".impeccable")).unwrap();
        fs::write(
            project.join(".impeccable/config.local.json"),
            "{\"browser\":\"/Users/me/Chrome\"}\n",
        )
        .unwrap();
        let report =
            installer::install(&request(&sandbox, &project, &[".claude"], false, id)).unwrap();
        assert_eq!(report.status, Status::Installed, "{:?}", report.conflicts);
        assert!(root.join(local).is_file());
        assert!(git(root, &["check-ignore", "-q", local]), "{id}");
    }
    // An earlier candidate wrote the block without the local configuration.
    let exclude = repo.join(".git/info/exclude");
    let current = fs::read_to_string(&exclude).unwrap();
    fs::write(
        &exclude,
        current.replace("**/.phoenix-ui/config.local.json\n", ""),
    )
    .unwrap();
    assert!(!git(&repo, &["check-ignore", "-q", local]));
    let again = installer::install(&request(
        &sandbox,
        &repo.join("apps/web"),
        &[".claude"],
        false,
        "local-config-3",
    ))
    .unwrap();
    assert_eq!(again.status, Status::Unchanged, "{:?}", again.conflicts);
    assert!(git(&repo, &["check-ignore", "-q", local]));
    assert_eq!(fs::read_to_string(&exclude).unwrap(), current);
}

/// Claude decides a plugin's state from `enabledPlugins` (local over project over
/// user); the install registry only records that it was installed.
#[test]
fn only_an_effectively_enabled_old_plugin_blocks_activation() {
    let sandbox = Sandbox::new();
    let project = sandbox.project();
    let home = sandbox.home();
    fs::create_dir_all(home.join(".claude/plugins")).unwrap();
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        home.join(".claude/plugins/installed_plugins.json"),
        serde_json::to_vec(&json!({"version": 2, "plugins": {"impeccable@fixture-marketplace": [
            {"scope": "user", "installPath": "/plugins/cache/impeccable/1.0.0", "version": "1.0.0"}]}}))
        .unwrap(),
    )
    .unwrap();
    let id = "impeccable@fixture-marketplace";
    let set = |path: PathBuf, state: Option<bool>| match state {
        Some(state) => fs::write(
            path,
            serde_json::to_vec(&json!({"enabledPlugins": {id: state}})).unwrap(),
        )
        .unwrap(),
        None => {
            let _ = fs::remove_file(path);
        }
    };
    for (user, shared, local, blocked) in [
        (Some(false), None, None, None),
        (None, None, None, None),
        (Some(true), None, None, Some(".claude/settings.json")),
        (Some(false), Some(true), None, Some(".claude/settings.json")),
        (Some(false), Some(true), Some(false), None),
        (Some(true), None, Some(false), None),
        (
            Some(false),
            None,
            Some(true),
            Some(".claude/settings.local.json"),
        ),
    ] {
        set(home.join(".claude/settings.json"), user);
        set(project.join(".claude/settings.json"), shared);
        set(project.join(".claude/settings.local.json"), local);
        let case = format!("user {user:?}, project {shared:?}, local {local:?}");
        let report =
            installer::install(&request(&sandbox, &project, &[".claude"], true, "plugins"))
                .unwrap();
        let plugin: Vec<_> = report
            .conflicts
            .iter()
            .filter(|c| c.message.contains("plugin"))
            .collect();
        match blocked {
            None => {
                assert!(plugin.is_empty(), "{case}: {plugin:?}");
                assert_eq!(report.status, Status::Planned, "{case}");
            }
            Some(file) => {
                assert_eq!(report.status, Status::Conflict, "{case}");
                assert_eq!(plugin.len(), 1, "{case}: {plugin:?}");
                assert!(plugin[0].path.ends_with(file), "{case}: {plugin:?}");
                let expected = if user == Some(true) && shared.is_none() {
                    home.join(file)
                } else {
                    project.join(file)
                };
                assert_eq!(Path::new(&plugin[0].path), expected, "{case}");
            }
        }
    }
}

/// In a repository Claude reads the local settings at the checkout root, or the
/// main checkout's root from a linked worktree; a file an older Claude left in
/// the starting directory still applies below it. A worktree without a main
/// checkout, and a repository rooted at the home directory, keep it in the
/// starting directory. Expectations match `claude plugin list` 2.1.296.
#[test]
fn old_plugin_state_follows_the_local_settings_claude_reads_in_a_repository() {
    let sandbox = Sandbox::new();
    let home = sandbox.home();
    let repo = sandbox.project();
    let commit = |dir: &Path| {
        assert!(git(
            dir,
            &[
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "init"
            ]
        ));
    };
    assert!(git(&repo, &["init", "-q"]));
    commit(&repo);
    let linked = sandbox.0.join("linked worktree");
    assert!(git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &linked.to_string_lossy()
        ]
    ));
    let bare = sandbox.0.join("bare.git");
    let bare_linked = sandbox.0.join("bare worktree");
    assert!(git(
        &sandbox.0,
        &[
            "clone",
            "-q",
            "--bare",
            &repo.to_string_lossy(),
            &bare.to_string_lossy()
        ]
    ));
    assert!(git(
        &bare,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &bare_linked.to_string_lossy()
        ]
    ));
    assert!(git(&home, &["init", "-q"]));
    let web = repo.join("apps/web");
    let linked_web = linked.join("apps/web");
    let bare_web = bare_linked.join("apps/web");
    let home_web = home.join("apps/web");
    for dir in [
        &repo,
        &web,
        &linked,
        &linked_web,
        &bare_linked,
        &bare_web,
        &home_web,
    ] {
        fs::create_dir_all(dir.join(".claude")).unwrap();
    }
    let id = "impeccable@fixture-marketplace";
    let local = ".claude/settings.local.json";
    let user = home.join(".claude/settings.json");
    fs::create_dir_all(home.join(".claude/plugins")).unwrap();
    fs::write(
        home.join(".claude/plugins/installed_plugins.json"),
        serde_json::to_vec(&json!({"version": 2, "plugins": {id: [
            {"scope": "user", "installPath": "/plugins/cache/impeccable/1.0.0", "version": "1.0.0"}]}}))
        .unwrap(),
    )
    .unwrap();
    let files = [
        user.clone(),
        repo.join(local),
        web.join(".claude/settings.json"),
        web.join(local),
        linked.join(local),
        linked_web.join(local),
        bare_linked.join(local),
        bare_web.join(local),
        home.join(local),
        home_web.join(local),
    ];
    for (project, settings, blocked) in [
        (&repo, vec![(&user, true), (&repo.join(local), false)], None),
        (&web, vec![(&user, true), (&repo.join(local), false)], None),
        (
            &web,
            vec![(&user, false), (&repo.join(local), true)],
            Some(repo.join(local)),
        ),
        (
            &web,
            vec![
                (&user, true),
                (&repo.join(local), true),
                (&web.join(local), false),
            ],
            Some(repo.join(local)),
        ),
        (
            &web,
            vec![
                (&user, false),
                (&repo.join(local), false),
                (&web.join(local), true),
            ],
            None,
        ),
        (&web, vec![(&user, true), (&web.join(local), false)], None),
        (
            &web,
            vec![(&user, false), (&web.join(local), true)],
            Some(web.join(local)),
        ),
        (
            &web,
            vec![
                (&user, false),
                (&web.join(".claude/settings.json"), true),
                (&web.join(local), false),
            ],
            None,
        ),
        (
            &linked,
            vec![(&user, true), (&repo.join(local), false)],
            None,
        ),
        (
            &linked,
            vec![
                (&user, true),
                (&repo.join(local), true),
                (&linked.join(local), false),
            ],
            Some(repo.join(local)),
        ),
        (
            &linked_web,
            vec![(&user, true), (&repo.join(local), false)],
            None,
        ),
        (
            &linked_web,
            vec![(&user, false), (&linked.join(local), true)],
            None,
        ),
        (
            &bare_web,
            vec![(&user, false), (&bare_linked.join(local), true)],
            None,
        ),
        (
            &bare_web,
            vec![(&user, false), (&bare_web.join(local), true)],
            Some(bare_web.join(local)),
        ),
        (
            &home_web,
            vec![(&user, false), (&home.join(local), true)],
            None,
        ),
        (
            &home_web,
            vec![(&user, true), (&home_web.join(local), false)],
            None,
        ),
    ] {
        for file in &files {
            let _ = fs::remove_file(file);
        }
        for (file, state) in &settings {
            fs::write(
                file,
                serde_json::to_vec(&json!({"enabledPlugins": {id: state}})).unwrap(),
            )
            .unwrap();
        }
        let case = format!(
            "{} with {:?}",
            project.display(),
            settings
                .iter()
                .map(|(file, state)| (file.strip_prefix(&sandbox.0).unwrap(), state))
                .collect::<Vec<_>>()
        );
        let report =
            installer::install(&request(&sandbox, project, &[".claude"], true, "plugins")).unwrap();
        let plugin: Vec<_> = report
            .conflicts
            .iter()
            .filter(|c| c.message.contains("plugin"))
            .map(|c| PathBuf::from(&c.path))
            .collect();
        match blocked {
            None => {
                assert!(plugin.is_empty(), "{case}: {plugin:?}");
                assert_eq!(report.status, Status::Planned, "{case}");
            }
            Some(file) => {
                assert_eq!(report.status, Status::Conflict, "{case}");
                assert_eq!(plugin, [file], "{case}");
            }
        }
    }
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
