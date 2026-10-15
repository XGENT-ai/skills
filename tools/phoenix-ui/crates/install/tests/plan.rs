use phoenix_ui_install::{plan::build_install_plan, sha256, Change, Manifest, Project};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::var_os("PHOENIX_PLAN_FIXTURE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = root.join(format!(
            "plan-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        if std::env::var_os("PHOENIX_PLAN_FIXTURE_ROOT").is_none() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn bundle() -> PathBuf {
    std::env::var_os("PHOENIX_PLAN_BUNDLE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .unwrap()
                .join("dist/phoenix-ui/bundle")
        })
}
fn manifest(providers: &[String]) -> Manifest {
    Manifest {
        version: "0.1.0".into(),
        sha256: sha256(&fs::read(bundle().join("manifest.json")).unwrap()),
        providers: providers.to_vec(),
        migration_sources: Vec::new(),
    }
}
#[test]
fn real_bundle_fresh_install_contains_verified_payloads_and_hook_patches() {
    let sandbox = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    let selected = vec![".claude".to_string(), ".agents".into(), ".codex".into()];
    let prepared = build_install_plan(
        &project,
        &bundle(),
        &selected,
        manifest(&selected),
        "fresh-real-bundle",
    )
    .unwrap();
    let plan = prepared.plan.unwrap();
    assert!(plan.changes.iter().any(|change| matches!(change, Change::Exclusive{path,after:Some(_),..} if path==".claude/skills/phoenix-ui/SKILL.md")), "fresh install omitted the bundle skill payload");
    assert!(
        plan.changes.iter().any(
            |change| matches!(change, Change::SharedJson{path,..} if path==".codex/hooks.json")
        ),
        "Codex hook-only payload must become entry patches"
    );
    assert!(
        !sandbox.0.join(".phoenix-ui").exists(),
        "planning must be read-only"
    );
}

use phoenix_ui_install::{
    classify::{ConflictKind, EntryKind},
    plan::PreparedInstall,
    Checkpoint, ErrorKind, ExpectedFile, Permissions, Plan,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, process::Command};

impl Sandbox {
    fn write(&self, path: &str, bytes: &[u8]) {
        let full = self.0.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }
    fn materialize(version: &str, providers: &[String], scenarios: &[&str], channel: &str) -> Self {
        let sandbox = Self::new();
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .unwrap()
            .join("test/phoenix-ui/fixtures/migration/materialize.mjs");
        let options = json!({"fixtureRoot":sandbox.0,"packageVersion":version,"providers":providers,"scenarios":scenarios,"channel":channel});
        let output = Command::new("node").args(["--input-type=module","-e","import {pathToFileURL} from 'node:url'; const {materializeMigrationFixture}=await import(pathToFileURL(process.argv[2])); console.log(JSON.stringify(materializeMigrationFixture(JSON.parse(process.argv[3]))));","plan-fixture-import"]).arg(script).arg(options.to_string()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        sandbox
    }
    fn project(&self) -> PathBuf {
        self.0.join("project with spaces")
    }
}
fn files(root: &Path) -> BTreeMap<PathBuf, (String, u32)> {
    fn walk(root: &Path, at: &Path, result: &mut BTreeMap<PathBuf, (String, u32)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o7777
            };
            #[cfg(not(unix))]
            let mode = u32::from(meta.permissions().readonly());
            let digest = if meta.is_symlink() {
                format!("symlink:{}", fs::read_link(&path).unwrap().display())
            } else if meta.is_dir() {
                "directory".into()
            } else if meta.is_file() {
                sha256(&fs::read(&path).unwrap())
            } else {
                "special".into()
            };
            result.insert(path.strip_prefix(root).unwrap().into(), (digest, mode));
            if meta.is_dir() && !meta.is_symlink() {
                walk(root, &path, result);
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
fn readonly(project: &Project, selected: &[String], m: Manifest, id: &str) -> PreparedInstall {
    let before = files(project.root());
    let prepared = build_install_plan(project, &bundle(), selected, m, id).unwrap();
    assert_eq!(
        files(project.root()),
        before,
        "builder wrote project bytes, entries or permissions"
    );
    if std::env::var_os("PHOENIX_PLAN_FIXTURE_ROOT").is_some() {
        let directory = project.root().parent().unwrap();
        let receipt = json!({"project":project.root(),"transactionId":id,"readonlyVerified":true,"entriesCompared":before.len(),"inventory":prepared.inventory,"conflicts":prepared.conflicts,"preserved":prepared.preserved,"alreadyInstalled":prepared.already_installed,"changes":prepared.plan.as_ref().map(|p| p.changes.iter().map(|c| json!({"path":c.path(),"kind":match c {Change::Exclusive{after:Some(_),..}=>"write",Change::Exclusive{after:None,..}=>"remove",Change::SharedJson{..}=>"shared-json"}})).collect::<Vec<_>>())});
        fs::write(
            directory.join(format!(
                "plan-receipt-{id}-{}.json",
                &sha256(project.root().to_string_lossy().as_bytes())[..12]
            )),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    }
    prepared
}
fn all_providers() -> Vec<String> {
    let m: Value =
        serde_json::from_slice(&fs::read(bundle().join("manifest.json")).unwrap()).unwrap();
    m["providers"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}
fn fixed_source(version: &str) -> Value {
    let raw = match version {
        "0.3.0" => include_str!("../../../migration-sources/0.3.0.json"),
        "0.4.0" => include_str!("../../../migration-sources/0.4.0.json"),
        "0.5.0" => include_str!("../../../migration-sources/0.5.0.json"),
        "0.6.0" => include_str!("../../../migration-sources/0.6.0.json"),
        _ => panic!("fixed version required"),
    };
    serde_json::from_str(raw).unwrap()
}
fn read_json(root: &Path, path: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
}
fn select(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}
fn apply_fresh(sandbox: &Sandbox, selected: &[String], id: &str) -> (Project, Plan) {
    let project = Project::open(&sandbox.0).unwrap();
    let prepared = readonly(&project, selected, manifest(selected), id);
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    let plan = prepared.plan.unwrap();
    project.apply(&plan).unwrap();
    (project, plan)
}

#[test]
fn every_real_provider_file_and_executable_is_planned_from_verified_bundle_bytes() {
    let sandbox = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    let selected = all_providers();
    let prepared = readonly(&project, &selected, manifest(&selected), "every-provider");
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    let plan = prepared.plan.unwrap();
    let bundle_root = bundle();
    let m: Value =
        serde_json::from_slice(&fs::read(bundle_root.join("manifest.json")).unwrap()).unwrap();
    let mut payloads = 0;
    let mut hooks = 0;
    for change in &plan.changes {
        if let Change::Exclusive {
            path,
            after: Some(payload),
            expected,
            ..
        } = change
        {
            let (provider, relative) = path.split_once('/').unwrap();
            let entry = &m["providers"][provider];
            let sha = entry["files"][relative].as_str().unwrap();
            assert_eq!(payload.sha256, sha);
            assert_eq!(
                payload.bytes,
                fs::read(bundle_root.join("blobs").join(sha)).unwrap()
            );
            assert_eq!(*expected, ExpectedFile::Absent);
            let executable = entry["exec"].as_array().unwrap().contains(&json!(relative));
            assert_eq!(
                payload.permissions,
                Permissions::file(if executable { 0o755 } else { 0o644 })
            );
            payloads += 1;
        } else if let Change::SharedJson {
            expected, patches, ..
        } = change
        {
            assert_eq!(*expected, ExpectedFile::Absent);
            assert!(!patches.is_empty());
            hooks += 1;
        }
        assert!(!change.path().ends_with("ENGINE.json"));
    }
    let expected_count = m["providers"]
        .as_object()
        .unwrap()
        .values()
        .map(|p| p["files"].as_object().unwrap().len())
        .sum::<usize>();
    assert_eq!(payloads + hooks, expected_count);
    assert_eq!(selected.len(), 19);
    assert_eq!(hooks, 6);
    println!(
        "new-bundle-matrix {}",
        json!({"providers":selected.len(),"payloadFiles":payloads,"sharedHookFiles":hooks,"bundleFiles":expected_count})
    );
}

#[test]
fn fixed_release_148_provider_channel_inputs_prepare_strict_migration_plans_readonly() {
    let (mut combinations, mut old_files, mut old_handlers) = (0, 0, 0);
    for version in ["0.3.0", "0.4.0", "0.5.0", "0.6.0"] {
        for channel in ["bundle", "xgent-installer"] {
            let source = fixed_source(version);
            let old_providers: Vec<_> = source["providers"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, e)| channel != "xgent-installer" || e["installerSelectable"] != false)
                .map(|(p, _)| p.clone())
                .collect();
            let sandbox = Sandbox::materialize(
                version,
                &old_providers,
                &["unmodified", "custom-allow", "unrelated-entry"],
                channel,
            );
            let mut selected = old_providers.clone();
            if selected.contains(&".agents".into()) && !selected.contains(&".codex".into()) {
                selected.push(".codex".into());
            }
            let project = Project::open(sandbox.project()).unwrap();
            let id = format!("fixed-{}-{channel}", version.replace('.', "-"));
            let prepared = readonly(&project, &selected, manifest(&selected), &id);
            assert!(
                prepared.conflicts.is_empty(),
                "{version}/{channel}: {:?}",
                prepared.conflicts
            );
            let plan = prepared.plan.as_ref().unwrap();
            for old in &prepared.inventory.exclusive {
                assert_eq!(old.kind, EntryKind::File);
                assert!(plan.changes.iter().any(|c|matches!(c,Change::Exclusive{path,expected:ExpectedFile::Hash(hash),after:None,..} if path==&old.path&&hash==&old.sha256)));
            }
            for shared in &prepared.inventory.shared {
                for old in &shared.handlers {
                    assert!(plan.changes.iter().any(|c|matches!(c,Change::SharedJson{path,patches,..} if path==&shared.path&&patches.iter().any(|p| p.parent==old.parent&&p.before.as_ref()==Some(&old.value)&&p.after.is_none()))));
                }
            }
            combinations += old_providers.len();
            old_files += prepared.inventory.exclusive.len();
            old_handlers += prepared
                .inventory
                .shared
                .iter()
                .map(|s| s.handlers.len())
                .sum::<usize>();
            println!(
                "plan-fixed-matrix {}",
                json!({"version":version,"channel":channel,"inputProviders":old_providers.len(),"selectedNewProviders":selected.len(),"legacyFiles":prepared.inventory.exclusive.len(),"legacyHandlers":prepared.inventory.shared.iter().map(|s|s.handlers.len()).sum::<usize>(),"planChanges":plan.changes.len(),"fixture":sandbox.0})
            );
        }
    }
    assert_eq!(combinations, 148);
    assert_eq!(old_handlers, 76);
    println!(
        "plan-fixed-total {}",
        json!({"providerChannelInputs":combinations,"legacyFiles":old_files,"legacyHandlers":old_handlers})
    );
}

#[test]
fn claude_real_migration_apply_removes_only_owned_entries_and_rollback_keeps_later_neighbors() {
    let selected = select(&[".claude"]);
    let sandbox = Sandbox::materialize(
        "0.6.0",
        &selected,
        &["unmodified", "custom-allow", "mixed-handlers"],
        "xgent-installer",
    );
    let project = Project::open(sandbox.project()).unwrap();
    let original = fs::read(project.root().join(".claude/settings.local.json")).unwrap();
    let prepared = readonly(
        &project,
        &selected,
        manifest(&selected),
        "claude-mixed-migrate",
    );
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    let plan = prepared.plan.unwrap();
    project.apply(&plan).unwrap();
    for old in &prepared.inventory.exclusive {
        assert!(!project.root().join(&old.path).exists());
    }
    let active = read_json(project.root(), ".claude/settings.json");
    let old = read_json(project.root(), ".claude/settings.local.json");
    assert_eq!(
        old["hooks"]["PostToolUse"][0]["fixtureMatcherNote"],
        "preserve this surrounding entry"
    );
    assert!(old["hooks"]["PostToolUse"][0]["hooks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["command"] == "node \"custom hook with spaces.js\""));
    assert_eq!(
        active["hooks"]["Stop"][0]["hooks"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "exactly one new design hook expected"
    );
    let file = project.root().join(".claude/settings.json");
    let mut value = active;
    // This fixed shared settings fixture has no allow list; the user's later
    // addition must survive the inverse patch just like a changed neighbor.
    assert!(value.get("permissions").is_none());
    value["permissions"] = json!({"allow":["Bash(later)"]});
    value["laterUnknown"] = json!({"keep":true});
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    project.rollback("claude-mixed-migrate").unwrap();
    assert_eq!(
        fs::read(project.root().join(".claude/settings.local.json")).unwrap(),
        original,
        "unchanged shared file restores exact old bytes"
    );
    let rolled = read_json(project.root(), ".claude/settings.json");
    assert_eq!(rolled["laterUnknown"], json!({"keep":true}));
    assert!(rolled["permissions"]["allow"]
        .as_array()
        .unwrap()
        .contains(&json!("Bash(later)")));
    assert!(!project
        .root()
        .join(".claude/skills/phoenix-ui/SKILL.md")
        .exists());
}

#[test]
fn jsonc_third_party_guard_and_unknown_numeric_spelling_survive_new_hook_apply() {
    let sandbox = Sandbox::new();
    let selected = select(&[".claude"]);
    let before=b"{\n // keep comment\n \"unknownNumber\":1.230000000000000000000,\n \"permissions\":{\"allow\":[\"Read\"]},\n \"hooks\":{\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"guard --threshold 5\",\"timeout\":9}],\"outerUnknown\":true}]}\n}\n";
    sandbox.write(".claude/settings.json", before);
    let (project, _) = apply_fresh(&sandbox, &selected, "jsonc-fresh");
    let after =
        String::from_utf8(fs::read(project.root().join(".claude/settings.json")).unwrap()).unwrap();
    for text in [
        "// keep comment",
        "\"unknownNumber\":1.230000000000000000000",
        "guard --threshold 5",
        "\"outerUnknown\":true",
    ] {
        assert!(after.contains(text), "lost {text}");
    }
    project.rollback("jsonc-fresh").unwrap();
    assert_eq!(
        fs::read(project.root().join(".claude/settings.json")).unwrap(),
        before
    );
}

#[test]
fn unowned_same_named_phoenix_files_and_complete_handlers_are_conflicts() {
    let selected = select(&[".claude"]);
    let sandbox = Sandbox::new();
    sandbox.write(
        ".claude/skills/phoenix-ui/SKILL.md",
        b"user customized skill",
    );
    let project = Project::open(&sandbox.0).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "unowned-file");
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
    let sandbox = Sandbox::new();
    let m: Value =
        serde_json::from_slice(&fs::read(bundle().join("manifest.json")).unwrap()).unwrap();
    let hash = m["providers"][".claude"]["files"]["settings.json"]
        .as_str()
        .unwrap();
    sandbox.write(
        ".claude/settings.json",
        &fs::read(bundle().join("blobs").join(hash)).unwrap(),
    );
    let project = Project::open(&sandbox.0).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "unowned-handler");
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
}

#[test]
fn wrapped_or_customized_legacy_entries_and_nonselected_provider_block_activation() {
    for scenarios in [&["modified-skill"][..], &["modified-handler"][..]] {
        let selected = select(&[".claude"]);
        let sandbox = Sandbox::materialize("0.6.0", &selected, scenarios, "xgent-installer");
        let project = Project::open(sandbox.project()).unwrap();
        let prepared = readonly(
            &project,
            &selected,
            manifest(&selected),
            "legacy-customized",
        );
        assert!(prepared.plan.is_none());
        assert!(!prepared.conflicts.is_empty());
    }
    let sandbox = Sandbox::materialize(
        "0.6.0",
        &select(&[".claude", ".cursor"]),
        &["unmodified"],
        "bundle",
    );
    let selected = select(&[".claude"]);
    let project = Project::open(sandbox.project()).unwrap();
    let prepared = readonly(&project, &selected, manifest(&selected), "outside-legacy");
    assert!(prepared.plan.is_none());
    assert!(prepared
        .conflicts
        .iter()
        .any(|c| c.kind == ConflictKind::OutsideSelection));
}

#[test]
fn broken_duplicate_key_or_malformed_hook_json_are_readonly_conflicts() {
    for bytes in [
        b"{broken".as_slice(),
        b"{\"hooks\":{},\"hooks\":{}}".as_slice(),
        b"{\"hooks\":{\"Stop\":{}}}".as_slice(),
    ] {
        let sandbox = Sandbox::new();
        sandbox.write(".claude/settings.json", bytes);
        let selected = select(&[".claude"]);
        let project = Project::open(&sandbox.0).unwrap();
        let prepared = readonly(&project, &selected, manifest(&selected), "json-conflict");
        assert!(prepared.plan.is_none());
        assert!(!prepared.conflicts.is_empty());
    }
    // Phoenix adds its own groups, so neighbors without a matcher do not conflict.
    let sandbox = Sandbox::new();
    sandbox.write(
        ".claude/settings.json",
        b"{\"hooks\":{\"Stop\":[{\"hooks\":[]},{\"hooks\":[]}]}}",
    );
    let selected = select(&[".claude"]);
    let project = Project::open(&sandbox.0).unwrap();
    let prepared = readonly(&project, &selected, manifest(&selected), "neighbor-groups");
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    assert!(prepared.plan.is_some());
}

#[test]
fn unknown_duplicate_or_mismatched_provider_selection_is_rejected_without_writes() {
    let sandbox = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    for providers in [
        select(&[".unknown"]),
        select(&[".claude", ".claude"]),
        vec![],
    ] {
        let before = files(&sandbox.0);
        let err = build_install_plan(
            &project,
            &bundle(),
            &providers,
            manifest(&providers),
            "providers",
        )
        .unwrap_err();
        assert_eq!(err.kind, ErrorKind::InvalidPlan);
        assert_eq!(files(&sandbox.0), before);
    }
    let selected = select(&[".claude"]);
    let err = build_install_plan(
        &project,
        &bundle(),
        &selected,
        manifest(&select(&[".cursor"])),
        "mismatch",
    )
    .unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidPlan);
}

#[test]
fn verified_identical_install_is_idempotent_and_customized_owned_file_blocks_retry() {
    let sandbox = Sandbox::new();
    let selected = select(&[".agent"]);
    let (project, _) = apply_fresh(&sandbox, &selected, "receipt-first");
    let p = readonly(
        &project,
        &selected,
        manifest(&selected),
        "different-retry-id",
    );
    assert!(p.already_installed);
    assert!(p.plan.is_none());
    assert!(p.conflicts.is_empty());
    sandbox.write(
        ".agent/skills/phoenix-ui/SKILL.md",
        b"user changed installed skill",
    );
    let p = readonly(&project, &selected, manifest(&selected), "customized-retry");
    assert!(!p.already_installed);
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
}

#[test]
fn active_receipt_provider_subsets_are_explicit_conflicts() {
    let sandbox = Sandbox::new();
    let old = select(&[".agent", ".vibe"]);
    let (project, _) = apply_fresh(&sandbox, &old, "two-providers");
    let selected = select(&[".agent"]);
    let p = readonly(&project, &selected, manifest(&selected), "subset");
    assert!(p.plan.is_none());
    assert!(p.conflicts.iter().any(
        |c| c.kind == ConflictKind::OutsideSelection && c.provider.as_deref() == Some(".vibe")
    ));
}

#[test]
fn plan_cas_rejects_changes_between_readonly_preparation_and_apply() {
    let sandbox = Sandbox::new();
    let selected = select(&[".claude"]);
    let project = Project::open(&sandbox.0).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "stale-plan");
    let plan = p.plan.unwrap();
    sandbox.write(".claude/settings.json", b"{\"later\":true}\n");
    let before = files(&sandbox.0);
    let error = project.apply(&plan).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
    assert_eq!(files(&sandbox.0), before);
    assert!(!sandbox.0.join(".phoenix-ui").exists());
}

#[test]
fn later_unknown_phoenix_wrapper_blocks_idempotent_retry_and_is_preserved() {
    let sandbox = Sandbox::new();
    let selected = select(&[".claude"]);
    let (project, _) = apply_fresh(&sandbox, &selected, "before-wrapper");
    let file = project.root().join(".claude/settings.json");
    let mut value = read_json(project.root(), ".claude/settings.json");
    value["hooks"]["Stop"][0]["hooks"].as_array_mut().unwrap().push(json!({"type":"command","command":"sh -c '.claude/skills/phoenix-ui/scripts/phoenix-ui hook; echo custom'","timeout":30}));
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "after-wrapper");
    assert!(
        !p.already_installed,
        "an extra active Phoenix wrapper must not be accepted as an identical install"
    );
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
}

struct BundleFixture {
    sandbox: Sandbox,
    root: PathBuf,
    manifest: Value,
}
impl BundleFixture {
    fn new() -> Self {
        let sandbox = Sandbox::new();
        let root = sandbox.0.join("bundle");
        fs::create_dir_all(root.join("blobs")).unwrap();
        let original = bundle();
        for entry in fs::read_dir(original.join("blobs")).unwrap() {
            let path = entry.unwrap().path();
            fs::copy(&path, root.join("blobs").join(path.file_name().unwrap())).unwrap();
        }
        let manifest = read_json(&original, "manifest.json");
        let result = Self {
            sandbox,
            root,
            manifest,
        };
        result.save();
        result
    }
    fn save(&self) {
        fs::write(
            self.root.join("manifest.json"),
            serde_json::to_vec_pretty(&self.manifest).unwrap(),
        )
        .unwrap();
    }
    fn install_manifest(&self, selected: &[String]) -> Manifest {
        Manifest {
            version: "0.1.1-fixture".into(),
            sha256: sha256(&fs::read(self.root.join("manifest.json")).unwrap()),
            providers: selected.to_vec(),
            migration_sources: Vec::new(),
        }
    }
    fn replace(&mut self, provider: &str, path: &str, bytes: &[u8]) {
        let hash = sha256(bytes);
        fs::write(self.root.join("blobs").join(&hash), bytes).unwrap();
        self.manifest["providers"][provider]["files"][path] = json!(hash);
        self.save();
    }
}
fn readonly_bundle(
    project: &Project,
    b: &BundleFixture,
    selected: &[String],
    id: &str,
) -> PreparedInstall {
    let before = files(project.root());
    let bundle_before = files(&b.sandbox.0);
    let p =
        build_install_plan(project, &b.root, selected, b.install_manifest(selected), id).unwrap();
    assert_eq!(files(project.root()), before);
    assert_eq!(files(&b.sandbox.0), bundle_before);
    p
}

#[test]
fn verified_bundle_update_replaces_exact_owned_file_and_handler_and_rolls_back() {
    let sandbox = Sandbox::new();
    let selected = select(&[".claude"]);
    let (project, first) = apply_fresh(&sandbox, &selected, "update-original");
    let before_skill = fs::read(project.root().join(".claude/skills/phoenix-ui/SKILL.md")).unwrap();
    let before_hook =
        read_json(project.root(), ".claude/settings.json")["hooks"]["Stop"][0].clone();
    let mut b = BundleFixture::new();
    let mut changed = before_skill.clone();
    changed.extend_from_slice(b"\nFixture release update\n");
    b.replace(".claude", "skills/phoenix-ui/SKILL.md", &changed);
    let hook_sha = b.manifest["providers"][".claude"]["files"]["settings.json"]
        .as_str()
        .unwrap();
    let mut hooks = read_json(&b.root.join("blobs"), hook_sha);
    hooks["hooks"]["Stop"][0]["hooks"][0]["statusMessage"] = json!("Updated fixture deep pass");
    // Phoenix owns the whole group, so an update replaces the group value.
    let new_hook = hooks["hooks"]["Stop"][0].clone();
    b.replace(
        ".claude",
        "settings.json",
        &serde_json::to_vec_pretty(&hooks).unwrap(),
    );
    let file = project.root().join(".claude/settings.json");
    let mut value = read_json(project.root(), ".claude/settings.json");
    value["laterUnknown"] = json!(true);
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let p = readonly_bundle(&project, &b, &selected, "verified-update");
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert!(!p.already_installed);
    let plan = p.plan.unwrap();
    assert!(plan.changes.iter().any(|c|matches!(c,Change::SharedJson{path,patches,..} if path==".claude/settings.json" && patches.iter().any(|p|p.before.as_ref()==Some(&before_hook)&&p.after.as_ref()==Some(&new_hook)))));
    project.apply(&plan).unwrap();
    assert_eq!(
        fs::read(project.root().join(".claude/skills/phoenix-ui/SKILL.md")).unwrap(),
        changed
    );
    let value = read_json(project.root(), ".claude/settings.json");
    assert_eq!(value["hooks"]["Stop"].as_array().unwrap(), &vec![new_hook]);
    assert_eq!(value["laterUnknown"], true);
    let p = readonly_bundle(&project, &b, &selected, "updated-retry");
    assert!(p.already_installed);
    assert!(p.plan.is_none());
    project.rollback("verified-update").unwrap();
    assert_eq!(
        fs::read(project.root().join(".claude/skills/phoenix-ui/SKILL.md")).unwrap(),
        before_skill
    );
    let value = read_json(project.root(), ".claude/settings.json");
    assert_eq!(
        value["hooks"]["Stop"].as_array().unwrap(),
        &vec![before_hook]
    );
    assert_eq!(value["laterUnknown"], true);
    assert_eq!(
        project
            .active_receipt_verified()
            .unwrap()
            .unwrap()
            .transaction_id,
        first.transaction_id
    );
}

#[test]
fn modified_owned_handler_is_a_conflict_even_with_later_unrelated_allow() {
    let sandbox = Sandbox::new();
    let selected = select(&[".codex"]);
    let (project, _) = apply_fresh(&sandbox, &selected, "owned-hook-original");
    let file = project.root().join(".codex/hooks.json");
    let mut value = read_json(project.root(), ".codex/hooks.json");
    value["permissions"] = json!({"allow":["later"]});
    value["hooks"]["Stop"][0]["hooks"][0]["command"] = json!("custom user replacement");
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let p = readonly(
        &project,
        &selected,
        manifest(&selected),
        "changed-owned-handler",
    );
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
    assert!(!p.already_installed);
    assert_eq!(
        read_json(project.root(), ".codex/hooks.json")["permissions"]["allow"],
        json!(["later"])
    );
}

#[test]
fn corrupt_manifest_blob_schema_and_unsafe_paths_are_rejected_before_project_writes() {
    let sandbox = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    let selected = select(&[".agent"]);
    let before = files(&sandbox.0);
    let b = BundleFixture::new();
    let mut m = b.install_manifest(&selected);
    m.sha256 = "0".repeat(64);
    assert_eq!(
        build_install_plan(&project, &b.root, &selected, m, "bad-manifest-sha")
            .unwrap_err()
            .kind,
        ErrorKind::InvalidPlan
    );
    let b = BundleFixture::new();
    let sha = b.manifest["providers"][".agent"]["files"]["skills/phoenix-ui/SKILL.md"]
        .as_str()
        .unwrap();
    fs::write(b.root.join("blobs").join(sha), b"short corrupt blob").unwrap();
    assert_eq!(
        build_install_plan(
            &project,
            &b.root,
            &selected,
            b.install_manifest(&selected),
            "bad-blob"
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidPlan
    );
    for mutation in [
        "schema",
        "traversal",
        "case-collision",
        "file-directory",
        "exec",
        "unknown-provider",
    ] {
        let mut b = BundleFixture::new();
        let sha = b.manifest["providers"][".agent"]["files"]["skills/phoenix-ui/SKILL.md"].clone();
        match mutation {
            "schema" => b.manifest["schema"] = json!(2),
            "traversal" => b.manifest["providers"][".agent"]["files"]["../outside"] = sha,
            "case-collision" => {
                b.manifest["providers"][".agent"]["files"]["skills/phoenix-ui/skill.md"] = sha
            }
            "file-directory" => {
                b.manifest["providers"][".agent"]["files"]["skills/phoenix-ui"] = sha
            }
            "exec" => b.manifest["providers"][".agent"]["exec"] = json!(["missing"]),
            "unknown-provider" => {
                b.manifest["providers"][".unknown"] = b.manifest["providers"][".agent"].clone()
            }
            _ => unreachable!(),
        };
        b.save();
        assert_eq!(
            build_install_plan(
                &project,
                &b.root,
                &selected,
                b.install_manifest(&selected),
                mutation
            )
            .unwrap_err()
            .kind,
            ErrorKind::InvalidPlan,
            "{mutation}"
        );
    }
    let b = BundleFixture::new();
    fs::write(b.root.join("manifest.json"),b"{\"schema\":1,\"schema\":1,\"derivedRuntimeManifest\":\"skills/phoenix-ui/scripts/ENGINE.json\",\"providers\":{}}").unwrap();
    assert_eq!(
        build_install_plan(
            &project,
            &b.root,
            &selected,
            b.install_manifest(&selected),
            "duplicate-manifest"
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidPlan
    );
    assert_eq!(files(&sandbox.0), before);
}

#[cfg(unix)]
#[test]
fn project_and_bundle_symlinks_and_special_files_are_rejected_without_following() {
    use std::os::unix::fs::symlink;
    let sandbox = Sandbox::new();
    let outside = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    let selected = select(&[".claude"]);
    symlink(&outside.0, sandbox.0.join(".claude")).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "project-symlink");
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
    assert!(files(&outside.0).is_empty());
    let sandbox = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    let b = BundleFixture::new();
    let linked = Sandbox::new();
    symlink(&b.root, linked.0.join("bundle-link")).unwrap();
    let err = build_install_plan(
        &project,
        &linked.0.join("bundle-link"),
        &selected,
        b.install_manifest(&selected),
        "bundle-link",
    )
    .unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidPlan);
    let sha = b.manifest["providers"][".claude"]["files"]["settings.json"]
        .as_str()
        .unwrap();
    let blob = b.root.join("blobs").join(sha);
    let bytes = fs::read(&blob).unwrap();
    fs::remove_file(&blob).unwrap();
    outside.write("blob", &bytes);
    symlink(outside.0.join("blob"), &blob).unwrap();
    assert!(build_install_plan(
        &project,
        &b.root,
        &selected,
        b.install_manifest(&selected),
        "blob-symlink"
    )
    .is_err());
    sandbox.write(".claude/placeholder", b"");
    let fifo = sandbox.0.join(".claude/settings.json");
    let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let p = readonly(&project, &selected, manifest(&selected), "fifo");
    assert!(p.plan.is_none());
    assert!(!p.conflicts.is_empty());
}

#[test]
fn real_hook_plan_recovers_interruption_and_refuses_owned_drift() {
    for drift in [false, true] {
        let sandbox = Sandbox::new();
        let project = Project::open(&sandbox.0).unwrap();
        let selected = select(&[".codex"]);
        let p = readonly(&project, &selected, manifest(&selected), "real-hook-crash");
        let plan = p.plan.unwrap();
        let crash = std::panic::catch_unwind(|| {
            project.apply_with(&plan, |checkpoint| {
                if checkpoint == Checkpoint::AfterReplace(".codex/hooks.json".into()) {
                    panic!("fixture interruption after real hook replace");
                }
            })
        });
        assert!(crash.is_err());
        let bytes = fs::read(project.root().join(".codex/hooks.json")).unwrap();
        if drift {
            let mut value = read_json(project.root(), ".codex/hooks.json");
            value["hooks"]["Stop"][0]["hooks"][0]["timeout"] = json!(99);
            fs::write(
                project.root().join(".codex/hooks.json"),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
            let before = fs::read(project.root().join(".codex/hooks.json")).unwrap();
            let error = project.recover().unwrap_err();
            assert_eq!(error.kind, ErrorKind::Conflict);
            assert_eq!(
                fs::read(project.root().join(".codex/hooks.json")).unwrap(),
                before
            );
            fs::write(project.root().join(".codex/hooks.json"), &bytes).unwrap();
        }
        project.recover().unwrap();
        assert!(!project.root().join(".codex/hooks.json").exists());
        assert!(!project
            .root()
            .join(".phoenix-ui/install/active.json")
            .exists());
    }
}

#[test]
fn all_six_shared_hook_shapes_apply_exact_bundle_handlers_and_rollback() {
    let sandbox = Sandbox::new();
    let selected = select(&[
        ".claude", ".codex", ".cursor", ".gemini", ".github", ".grok",
    ]);
    let (project, plan) = apply_fresh(&sandbox, &selected, "six-hook-shapes");
    let source: Value =
        serde_json::from_slice(&fs::read(bundle().join("manifest.json")).unwrap()).unwrap();
    let mut handlers = 0;
    for change in &plan.changes {
        if let Change::SharedJson { path, .. } = change {
            let (provider, relative) = path.split_once('/').unwrap();
            let sha = source["providers"][provider]["files"][relative]
                .as_str()
                .unwrap();
            let expected: Value =
                serde_json::from_slice(&fs::read(bundle().join("blobs").join(sha)).unwrap())
                    .unwrap();
            let actual = read_json(project.root(), path);
            assert_eq!(
                actual, expected,
                "{provider}: complete command/Windows/timeout/metadata must match the bundle"
            );
            for array in actual["hooks"].as_object().unwrap().values() {
                for outer in array.as_array().unwrap() {
                    handlers += outer
                        .get("hooks")
                        .and_then(Value::as_array)
                        .map_or(1, Vec::len);
                }
            }
        }
    }
    assert_eq!(handlers, 11);
    project.rollback("six-hook-shapes").unwrap();
    for change in &plan.changes {
        assert!(
            !project.root().join(change.path()).exists(),
            "{}",
            change.path()
        );
    }
    println!(
        "real-new-hook-acceptance {}",
        json!({"providers":6,"completeHandlers":handlers,"applyRollback":true})
    );
}

#[test]
fn verified_partial_receipt_does_not_prove_complete_bundle_installation() {
    let sandbox = Sandbox::new();
    let project = Project::open(&sandbox.0).unwrap();
    let selected = select(&[".codex"]);
    project
        .apply(&Plan {
            transaction_id: "partial-receipt".into(),
            manifest: manifest(&selected),
            expected_receipt: ExpectedFile::Absent,
            changes: vec![Change::Exclusive {
                path: ".codex/other-owned-file".into(),
                expected: ExpectedFile::Absent,
                after: Some(phoenix_ui_install::Payload::new(
                    b"other owned bytes".to_vec(),
                    Permissions::file(0o644),
                )),
                rollback: phoenix_ui_install::RollbackPolicy::Restore,
            }],
        })
        .unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "complete-partial");
    assert!(
        !p.already_installed,
        "manifest equality in a valid partial receipt is not a complete bundle install"
    );
    assert!(p.plan.is_some());
    assert!(p.conflicts.is_empty());
}

#[test]
fn mixed_direct_handler_body_and_unknown_named_gemini_command_remain_conflicts() {
    let selected = select(&[".cursor"]);
    let sandbox = Sandbox::materialize("0.6.0", &selected, &["mixed-handlers"], "bundle");
    let project = Project::open(sandbox.project()).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "direct-body-note");
    assert!(p.plan.is_none());
    assert!(p
        .conflicts
        .iter()
        .any(|c| c.kind == ConflictKind::CustomizedHandler));
    let selected = select(&[".gemini"]);
    let sandbox = Sandbox::materialize("0.6.0", &selected, &["unmodified"], "bundle");
    let project = Project::open(sandbox.project()).unwrap();
    let file = project.root().join(".gemini/settings.json");
    let mut value = read_json(project.root(), ".gemini/settings.json");
    value["hooks"]["BeforeTool"][0]["hooks"][0]["command"] =
        json!("node entirely-unknown-command.js");
    value["neighbor"] = json!("keep");
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let p = readonly(
        &project,
        &selected,
        manifest(&selected),
        "unknown-gemini-command",
    );
    assert!(p.plan.is_none());
    assert!(p
        .conflicts
        .iter()
        .any(|c| c.kind == ConflictKind::CustomizedHandler));
}

#[test]
fn pin_reconstruction_is_reported_for_caller_and_live_residue_blocks_plan() {
    let selected = select(&[".claude"]);
    let sandbox = Sandbox::materialize("0.6.0", &selected, &["pin-shortcut"], "xgent-installer");
    let project = Project::open(sandbox.project()).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "caller-pin");
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    let pins: Vec<_> = p
        .inventory
        .exclusive
        .iter()
        .filter(|f| f.kind == EntryKind::Pin)
        .collect();
    assert!(!pins.is_empty());
    for pin in pins {
        assert!(!p
            .plan
            .as_ref()
            .unwrap()
            .changes
            .iter()
            .any(|c| c.path() == pin.path));
        assert!(p
            .preserved
            .iter()
            .any(|item| item.path == pin.path && item.reason.contains("caller")));
    }
    let sandbox = Sandbox::materialize("0.6.0", &selected, &["live-residue"], "xgent-installer");
    let project = Project::open(sandbox.project()).unwrap();
    let p = readonly(&project, &selected, manifest(&selected), "live-blocked");
    assert!(p.plan.is_none());
    assert!(p
        .conflicts
        .iter()
        .any(|c| c.kind == ConflictKind::LiveResidue));
}
