use phoenix_ui_install::{
    classify::{classify_project, ConflictKind, EntryKind, Inventory},
    sha256, JsonStep,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture {
    sandbox: PathBuf,
    project: PathBuf,
    metadata: Value,
}
impl Fixture {
    fn materialize(version: &str, providers: &[String], scenarios: &[&str], channel: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let fixture_base = std::env::var_os("PHOENIX_CLASSIFY_FIXTURE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let sandbox = fixture_base.join(format!(
            "phoenix-classify-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .unwrap();
        let script = repo.join("test/phoenix-ui/fixtures/migration/materialize.mjs");
        let mut fixture = Self {
            project: sandbox.join("project with spaces"),
            sandbox,
            metadata: Value::Null,
        };
        let options = serde_json::json!({"fixtureRoot":fixture.sandbox,"packageVersion":version,"providers":providers,"scenarios":scenarios,"channel":channel});
        let output=Command::new("node").args(["--input-type=module","-e","import {pathToFileURL} from 'node:url'; const {materializeMigrationFixture}=await import(pathToFileURL(process.argv[2])); console.log(JSON.stringify(materializeMigrationFixture(JSON.parse(process.argv[3]))));","classification-fixture-import"])
            .arg(script).arg(options.to_string()).output().unwrap();
        assert!(
            output.status.success(),
            "materialize failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
        fixture.project = PathBuf::from(metadata["projectRoot"].as_str().unwrap());
        fixture.metadata = metadata;
        fixture
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::env::var_os("PHOENIX_CLASSIFY_FIXTURE_ROOT").is_none() && self.sandbox.exists() {
            fs::remove_dir_all(&self.sandbox).unwrap();
        }
    }
}

fn classify_readonly(fixture: &Fixture, selected: &[String]) -> Inventory {
    let before = inventory(&fixture.sandbox);
    let result = classify_project(&fixture.project, selected).unwrap();
    assert_eq!(
        inventory(&fixture.sandbox),
        before,
        "classification changed sandbox bytes, permissions or entries"
    );
    assert!(!fixture.project.join(".phoenix-ui").exists());
    if std::env::var_os("PHOENIX_CLASSIFY_FIXTURE_ROOT").is_some() {
        static RECEIPT: AtomicU64 = AtomicU64::new(0);
        let n = RECEIPT.fetch_add(1, Ordering::Relaxed);
        let receipt = serde_json::json!({"project":fixture.project,"selectedProviders":selected,"readonlyVerified":true,"sandboxEntriesCompared":before.len(),"classification":result});
        fs::write(
            fixture.sandbox.join(format!("classification-{n}.json")),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    }
    result
}

fn write_json(fixture: &Fixture, path: &str, value: &Value) {
    fs::write(
        fixture.project.join(path),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
fn read_json(fixture: &Fixture, path: &str) -> Value {
    serde_json::from_slice(&fs::read(fixture.project.join(path)).unwrap()).unwrap()
}
fn has_conflict(result: &Inventory, path: &str, kind: ConflictKind) -> bool {
    result
        .conflicts
        .iter()
        .any(|c| c.path == path && c.kind == kind)
}

#[test]
fn duplicate_direct_handlers_are_ambiguous_and_unowned() {
    let selected = vec![".cursor".to_string()];
    let f = Fixture::materialize("0.6.0", &selected, &["unmodified"], "xgent-installer");
    let mut value = read_json(&f, ".cursor/hooks.json");
    let entries = value["hooks"]["preToolUse"].as_array_mut().unwrap();
    entries.push(entries[0].clone());
    write_json(&f, ".cursor/hooks.json", &value);
    let result = classify_readonly(&f, &selected);
    assert!(
        has_conflict(
            &result,
            ".cursor/hooks.json",
            ConflictKind::AmbiguousHandler
        ),
        "duplicate complete handlers must not become two owned entries"
    );
    assert!(result
        .shared
        .iter()
        .find(|s| s.path == ".cursor/hooks.json")
        .unwrap()
        .handlers
        .is_empty());
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, (Vec<u8>, u32)> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (Vec<u8>, u32)>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o7777
            };
            #[cfg(not(unix))]
            let mode = u32::from(meta.permissions().readonly());
            let bytes = if meta.file_type().is_symlink() {
                fs::read_link(&path)
                    .unwrap()
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec()
            } else if meta.is_dir() {
                b"directory".to_vec()
            } else if meta.is_file() {
                fs::read(&path).unwrap()
            } else {
                b"special-file".to_vec()
            };
            out.insert(path.strip_prefix(root).unwrap().into(), (bytes, mode));
            if meta.is_dir() && !meta.file_type().is_symlink() {
                walk(root, &path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn fixed_published_project_has_exact_owned_files_and_handlers_without_writes() {
    let selected = vec![".claude".to_string()];
    let f = Fixture::materialize(
        "0.3.0",
        &selected,
        &["custom-allow", "unrelated-entry"],
        "xgent-installer",
    );
    let before = inventory(&f.sandbox);
    let result = classify_project(&f.project, &selected).unwrap();
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert!(
        !result.exclusive.is_empty(),
        "supported full-content old files must be recognized"
    );
    assert_eq!(
        result
            .shared
            .iter()
            .map(|f| f.handlers.len())
            .sum::<usize>(),
        2
    );
    assert_eq!(inventory(&f.sandbox), before);
    assert_eq!(f.metadata["packageVersion"], "0.3.0");
}

fn source(version: &str) -> Value {
    let raw = match version {
        "0.3.0" => include_str!("../../../migration-sources/0.3.0.json"),
        "0.4.0" => include_str!("../../../migration-sources/0.4.0.json"),
        "0.5.0" => include_str!("../../../migration-sources/0.5.0.json"),
        "0.6.0" => include_str!("../../../migration-sources/0.6.0.json"),
        _ => panic!("test version must be fixed"),
    };
    serde_json::from_str(raw).unwrap()
}
fn fixed_providers(source: &Value, channel: &str) -> Vec<String> {
    source["providers"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, entry)| channel != "xgent-installer" || entry["installerSelectable"] != false)
        .map(|(name, _)| name.clone())
        .collect()
}
fn handler_count(result: &Inventory) -> usize {
    result.shared.iter().map(|s| s.handlers.len()).sum()
}
fn hook_providers(channel: &str) -> Vec<String> {
    let codex = if channel == "bundle" {
        ".codex"
    } else {
        ".agents"
    };
    [".claude", codex, ".cursor", ".github", ".grok"]
        .into_iter()
        .map(str::to_string)
        .collect()
}
const VERSIONS: [&str; 4] = ["0.3.0", "0.4.0", "0.5.0", "0.6.0"];

#[test]
fn all_fixed_versions_providers_and_channels_match_real_materialized_bytes() {
    let (mut combinations, mut files, mut metadata_hooks, mut shared_files, mut handlers) =
        (0, 0, 0, 0, 0);
    for version in VERSIONS {
        for channel in ["bundle", "xgent-installer"] {
            let selected = fixed_providers(&source(version), channel);
            let f = Fixture::materialize(
                version,
                &selected,
                &["unmodified", "custom-allow", "unrelated-entry"],
                channel,
            );
            let result = classify_readonly(&f, &selected);
            assert!(
                result.conflicts.is_empty(),
                "{version}/{channel}: {:?}",
                result.conflicts
            );
            let expected: BTreeMap<_, _> = f.metadata["inventory"]["projectFiles"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(path, _)| {
                    path.contains("/skills/impeccable/")
                        || path.contains("/agents/impeccable-")
                        || path.ends_with("/commands/impeccable.md")
                })
                .map(|(p, h)| (p.clone(), h.as_str().unwrap().to_string()))
                .collect();
            let actual: BTreeMap<_, _> = result
                .exclusive
                .iter()
                .map(|entry| {
                    assert!(entry.selected);
                    assert_eq!(entry.kind, EntryKind::File);
                    assert!(entry.sources.iter().any(|s| s.package_version == version));
                    assert_eq!(
                        sha256(&fs::read(f.project.join(&entry.path)).unwrap()),
                        entry.sha256
                    );
                    (entry.path.clone(), entry.sha256.clone())
                })
                .collect();
            assert_eq!(
                actual, expected,
                "{version}/{channel} complete exclusive inventory"
            );
            let expected_handlers = if version == "0.3.0" {
                8
            } else if channel == "bundle" {
                11
            } else {
                9
            };
            assert_eq!(
                handler_count(&result),
                expected_handlers,
                "{version}/{channel}"
            );
            for shared in &result.shared {
                assert_eq!(
                    shared.sha256,
                    sha256(&fs::read(f.project.join(&shared.path)).unwrap())
                );
                assert!(
                    !actual.contains_key(&shared.path),
                    "shared file cannot be exclusive"
                );
                for handler in &shared.handlers {
                    assert!(handler.selected);
                    assert!(handler
                        .sources
                        .iter()
                        .any(|s| s.package_version == version && s.channel == channel));
                }
            }
            for document in ["AGENTS.md", "CLAUDE.md", "PRODUCT.md", "DESIGN.md"] {
                assert!(result.preserved.iter().any(|p| p.path == document));
            }
            assert!(result
                .preserved
                .iter()
                .any(|p| p.path.ends_with("/skills/user-helper")));
            combinations += selected.len();
            files += actual.len();
            metadata_hooks += f.metadata["inventory"]["hookFiles"]
                .as_array()
                .unwrap()
                .len();
            shared_files += result.shared.len();
            handlers += handler_count(&result);
            println!(
                "fixed-matrix {}",
                serde_json::json!({"version":version,"channel":channel,"providers":selected.len(),"exclusiveFiles":actual.len(),"metadataHookFiles":f.metadata["inventory"]["hookFiles"].as_array().unwrap().len(),"sharedFiles":result.shared.len(),"ownedHandlers":handler_count(&result),"conflicts":result.conflicts.len(),"fixture":f.sandbox})
            );
        }
    }
    assert_eq!(combinations, 148);
    assert_eq!(metadata_hooks, 40);
    assert_eq!(shared_files, 47);
    assert_eq!(handlers, 76);
    println!(
        "fixed-matrix-total {}",
        serde_json::json!({"providerChannelCombinations":combinations,"exclusiveFiles":files,"metadataHookFiles":metadata_hooks,"sharedFiles":shared_files,"ownedHandlers":handlers,"supplementalGeminiFiles":3})
    );
}

#[test]
fn exact_synthetic_js_fixture_markers_match_all_applicable_hook_shapes() {
    let (mut combinations, mut handlers) = (0, 0);
    for version in VERSIONS {
        for channel in ["bundle", "xgent-installer"] {
            let selected = hook_providers(channel);
            let f = Fixture::materialize(
                version,
                &selected,
                &["legacy-js-hooks", "custom-allow", "unrelated-entry"],
                channel,
            );
            let result = classify_readonly(&f, &selected);
            assert!(
                result.conflicts.is_empty(),
                "{version}/{channel}: {:?}",
                result.conflicts
            );
            assert_eq!(
                handler_count(&result),
                if version == "0.3.0" { 8 } else { 9 }
            );
            assert!(result.shared.iter().flat_map(|s| &s.handlers).all(|h| h
                .sources
                .iter()
                .any(|s| s.package_version == version && s.channel == "legacy-js-fixture")));
            combinations += selected.len();
            handlers += handler_count(&result);
            assert!(!f.metadata["limitations"].as_array().unwrap().is_empty());
        }
    }
    assert_eq!(combinations, 40);
    assert_eq!(handlers, 70);
    println!(
        "legacy-js-fixture-total {}",
        serde_json::json!({"providerChannelCombinations":combinations,"ownedHandlers":handlers,"releasedJsBodiesClaimed":false})
    );
}

#[test]
fn mixed_outer_matcher_fields_are_preserved_but_direct_body_fields_conflict() {
    for version in VERSIONS {
        for channel in ["bundle", "xgent-installer"] {
            let selected = hook_providers(channel);
            let f = Fixture::materialize(
                version,
                &selected,
                &["mixed-handlers", "custom-allow", "unrelated-entry"],
                channel,
            );
            let result = classify_readonly(&f, &selected);
            assert!(has_conflict(
                &result,
                ".cursor/hooks.json",
                ConflictKind::CustomizedHandler
            ));
            assert!(has_conflict(
                &result,
                ".github/hooks/impeccable.json",
                ConflictKind::CustomizedHandler
            ));
            assert_eq!(
                result.conflicts.len(),
                2,
                "{version}/{channel}: {:?}",
                result.conflicts
            );
            assert_eq!(
                handler_count(&result),
                if version == "0.3.0" { 6 } else { 7 }
            );
            for handler in result.shared.iter().flat_map(|s| &s.handlers) {
                assert!(!handler
                    .value
                    .as_object()
                    .unwrap()
                    .contains_key("fixtureMatcherNote"));
                assert!(handler.parent.iter().any(|p|matches!(p,JsonStep::Match(identity) if !identity.fields.contains_key("fixtureMatcherNote"))));
            }
        }
    }
}

#[test]
fn compound_shell_wrappers_are_conflicts_in_every_supported_hook_shape() {
    for version in VERSIONS {
        for channel in ["bundle", "xgent-installer"] {
            let selected = hook_providers(channel);
            let f = Fixture::materialize(
                version,
                &selected,
                &["modified-handler", "unrelated-entry"],
                channel,
            );
            let result = classify_readonly(&f, &selected);
            assert_eq!(
                result
                    .conflicts
                    .iter()
                    .filter(|c| c.kind == ConflictKind::CustomizedHandler)
                    .count(),
                5,
                "{version}/{channel}: {:?}",
                result.conflicts
            );
            assert_eq!(
                handler_count(&result),
                if version == "0.3.0" { 3 } else { 4 }
            );
            assert!(result
                .shared
                .iter()
                .flat_map(|s| &s.handlers)
                .all(|h| !h.value.to_string().contains("user wrapper")));
        }
    }
}

#[test]
fn claude_shared_and_local_are_both_classified_at_entry_level() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize(
        "0.6.0",
        &selected,
        &[
            "claude-shared-and-local",
            "mixed-handlers",
            "custom-allow",
            "unrelated-entry",
        ],
        "xgent-installer",
    );
    let result = classify_readonly(&f, &selected);
    assert!(result.conflicts.is_empty());
    for path in [".claude/settings.json", ".claude/settings.local.json"] {
        assert_eq!(
            result
                .shared
                .iter()
                .find(|s| s.path == path)
                .unwrap()
                .handlers
                .len(),
            3
        );
        assert!(!result.exclusive.iter().any(|e| e.path == path));
    }
}

#[test]
fn gemini_supplement_is_bound_to_m1_digest_and_keeps_outer_neighbors() {
    for version in ["0.4.0", "0.5.0", "0.6.0"] {
        let selected = vec![".gemini".into()];
        let f = Fixture::materialize(version, &selected, &["unmodified"], "bundle");
        let path = ".gemini/settings.json";
        let bytes = fs::read(f.project.join(path)).unwrap();
        assert_eq!(
            sha256(&bytes),
            source(version)["providers"][".gemini"]["files"]["settings.json"]
        );
        assert!(source(version)["providers"][".gemini"]["hooks"]
            .as_array()
            .unwrap()
            .is_empty());
        let mut value = read_json(&f, path);
        value["permissions"] = serde_json::json!({"allow":["Read"],"unknown":9007199254740993_u64});
        for entries in value["hooks"].as_object_mut().unwrap().values_mut() {
            let outer = &mut entries[0];
            outer["fixtureMatcherNote"] = Value::String("keep outer".into());
            outer["hooks"].as_array_mut().unwrap().push(
                serde_json::json!({"type":"command","command":"echo impeccable notes","keep":true}),
            );
        }
        write_json(&f, path, &value);
        let result = classify_readonly(&f, &selected);
        assert!(result.conflicts.is_empty());
        assert_eq!(handler_count(&result), 2);
        assert!(!result.exclusive.iter().any(|e| e.path == path));
        value["hooks"]["AfterAgent"][0]["hooks"][0]["fixtureMatcherNote"] = Value::Bool(true);
        write_json(&f, path, &value);
        let result = classify_readonly(&f, &selected);
        assert!(has_conflict(&result, path, ConflictKind::CustomizedHandler));
        assert_eq!(handler_count(&result), 1);
    }
}

#[test]
fn changed_windows_command_or_timeout_is_not_owned() {
    for (provider, channel) in [(".agents", "xgent-installer"), (".codex", "bundle")] {
        let selected = vec![provider.into()];
        let f = Fixture::materialize("0.6.0", &selected, &[], channel);
        let path = ".codex/hooks.json";
        let mut value = read_json(&f, path);
        value["hooks"]["PostToolUse"][0]["hooks"][0]["commandWindows"] =
            Value::String("echo wrapper & custom.cmd".into());
        value["hooks"]["Stop"][0]["hooks"][0]["timeout"] = Value::from(31);
        write_json(&f, path, &value);
        let result = classify_readonly(&f, &selected);
        assert!(has_conflict(&result, path, ConflictKind::CustomizedHandler));
        assert_eq!(handler_count(&result), 0);
    }
}

#[test]
fn duplicate_nested_values_or_matcher_groups_are_ambiguous() {
    for duplicate_group in [false, true] {
        let selected = vec![".claude".into()];
        let f = Fixture::materialize("0.6.0", &selected, &[], "xgent-installer");
        let path = ".claude/settings.local.json";
        let mut value = read_json(&f, path);
        if duplicate_group {
            let entries = value["hooks"]["PostToolUse"].as_array_mut().unwrap();
            entries.push(entries[0].clone());
        } else {
            let entries = value["hooks"]["PostToolUse"][0]["hooks"]
                .as_array_mut()
                .unwrap();
            entries.push(entries[0].clone());
        }
        write_json(&f, path, &value);
        let result = classify_readonly(&f, &selected);
        assert!(has_conflict(&result, path, ConflictKind::AmbiguousHandler));
        assert_eq!(handler_count(&result), 2);
    }
}

#[test]
fn a_neighbor_group_with_the_same_matcher_leaves_the_old_group_claimable_whole() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "xgent-installer");
    let path = ".claude/settings.local.json";
    let mut value = read_json(&f, path);
    let entries = value["hooks"]["PostToolUse"].as_array_mut().unwrap();
    let old = entries[0].clone();
    let mut neighbor = old.clone();
    neighbor["unknown"] = Value::Bool(true);
    neighbor["hooks"] = serde_json::json!([{"command":"echo user neighbor"}]);
    entries.push(neighbor);
    write_json(&f, path, &value);
    let result = classify_readonly(&f, &selected);
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(handler_count(&result), 3);
    let claimed = result
        .shared
        .iter()
        .flat_map(|s| &s.handlers)
        .find(|h| h.value == old)
        .expect("the old group is claimed by its exact value");
    assert_eq!(
        claimed.parent,
        vec![
            JsonStep::Key("hooks".into()),
            JsonStep::Key("PostToolUse".into())
        ]
    );
}

#[test]
fn changed_matcher_or_event_cannot_claim_an_exact_old_command() {
    for event_changed in [false, true] {
        let selected = vec![".claude".into()];
        let f = Fixture::materialize("0.6.0", &selected, &[], "xgent-installer");
        let path = ".claude/settings.local.json";
        let mut value = read_json(&f, path);
        if event_changed {
            let old = value["hooks"]
                .as_object_mut()
                .unwrap()
                .remove("PostToolUse")
                .unwrap();
            value["hooks"]["UnknownEvent"] = old;
        } else {
            value["hooks"]["PostToolUse"][0]["matcher"] = Value::String("UserPattern".into());
        }
        write_json(&f, path, &value);
        let result = classify_readonly(&f, &selected);
        assert!(has_conflict(&result, path, ConflictKind::CustomizedHandler));
        assert_eq!(handler_count(&result), 2);
    }
}

#[test]
fn jsonc_and_unknown_numbers_keep_original_bytes() {
    let selected = vec![".cursor".into()];
    let f = Fixture::materialize(
        "0.6.0",
        &selected,
        &["custom-allow", "unrelated-entry"],
        "bundle",
    );
    let path = ".cursor/hooks.json";
    let bytes = fs::read_to_string(f.project.join(path)).unwrap();
    let text = format!(
        "// original JSONC note\n/* comment stays */\n{}",
        bytes.replacen('{', "{\n\"unknownCounter\": 9007199254740993,", 1)
    );
    fs::write(f.project.join(path), text.as_bytes()).unwrap();
    let result = classify_readonly(&f, &selected);
    assert!(result.conflicts.is_empty());
    assert_eq!(handler_count(&result), 1);
    assert_eq!(fs::read(f.project.join(path)).unwrap(), text.as_bytes());
}

#[test]
fn corrupt_duplicate_key_and_invalid_provider_shapes_are_conflicts() {
    let selected = vec![".cursor".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let path = ".cursor/hooks.json";
    for bytes in [
        "{broken",
        "{\"hooks\":{},\"hooks\":{}}",
        "{\"a\":1,\"\\u0061\":2}",
        "{\"hooks\":{\"preToolUse\":[{\"command\":\"a\",\"command\":\"b\"}]}}",
        "/* unterminated",
        "[]",
        "null",
        "{\"hooks\":[]}",
        "{\"hooks\":{\"preToolUse\":{}}}",
        "{\"hooks\":{\"preToolUse\":[{\"hooks\":{}}]}}",
    ] {
        fs::write(f.project.join(path), bytes).unwrap();
        let result = classify_readonly(&f, &selected);
        assert!(
            has_conflict(&result, path, ConflictKind::InvalidJson),
            "{bytes}: {:?}",
            result.conflicts
        );
        assert!(!result
            .shared
            .iter()
            .any(|s| s.path == path && !s.handlers.is_empty()));
    }
}

#[test]
fn modified_skill_and_unknown_launcher_bytes_are_retained_conflicts() {
    for version in ["0.3.0", "0.6.0"] {
        let selected = vec![".claude".into()];
        let f = Fixture::materialize(
            version,
            &selected,
            &["modified-skill", "unknown-upstream", "personal-shadow"],
            "xgent-installer",
        );
        let result = classify_readonly(&f, &selected);
        for path in [
            ".claude/skills/impeccable/SKILL.md",
            ".claude/skills/impeccable/scripts/impeccable",
        ] {
            assert!(has_conflict(&result, path, ConflictKind::CustomizedFile));
            assert!(!result.exclusive.iter().any(|e| e.path == path));
        }
    }
}

#[test]
fn unknown_old_tree_entries_and_pin_markers_never_gain_ownership() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "xgent-installer");
    let extra = ".claude/skills/impeccable/user-extension.txt";
    fs::write(f.project.join(extra), b"user bytes").unwrap();
    let pin = ".claude/skills/polish/SKILL.md";
    fs::create_dir_all(f.project.join(pin).parent().unwrap()).unwrap();
    fs::write(
        f.project.join(pin),
        b"<!-- impeccable-pinned-skill -->\nunsupported template",
    )
    .unwrap();
    let command = ".claude/commands/impeccable-custom.md";
    fs::create_dir_all(f.project.join(command).parent().unwrap()).unwrap();
    fs::write(f.project.join(command), b"custom old-name entry").unwrap();
    let result = classify_readonly(&f, &selected);
    for path in [extra, pin, command] {
        assert!(has_conflict(
            &result,
            path,
            ConflictKind::UnknownLegacyEntry
        ));
        assert!(!result.exclusive.iter().any(|e| e.path == path));
    }
}

#[test]
fn nonselected_active_old_file_and_hook_each_block_activation() {
    for hook_only in [false, true] {
        let all = vec![".claude".into(), ".cursor".into()];
        let f = Fixture::materialize("0.6.0", &all, &[], "xgent-installer");
        if hook_only {
            fs::remove_dir_all(f.project.join(".cursor/skills/impeccable")).unwrap();
        } else {
            fs::remove_file(f.project.join(".cursor/hooks.json")).unwrap();
        }
        let result = classify_readonly(&f, &[".claude".into()]);
        assert!(result
            .conflicts
            .iter()
            .any(|c| c.kind == ConflictKind::OutsideSelection
                && c.provider.as_deref() == Some(".cursor")));
        if hook_only {
            assert!(result
                .shared
                .iter()
                .flat_map(|s| &s.handlers)
                .any(|h| h.provider == ".cursor" && !h.selected));
        } else {
            assert!(result
                .exclusive
                .iter()
                .any(|e| e.provider == ".cursor" && !e.selected));
        }
    }
}

#[test]
fn fixed_pin_templates_are_owned_by_full_hash_and_modified_pins_conflict() {
    let selected = vec![".claude".into(), ".agents".into(), ".opencode".into()];
    for modified in [false, true] {
        let f = Fixture::materialize(
            "0.6.0",
            &selected,
            &[if modified {
                "modified-pin"
            } else {
                "pin-shortcut"
            }],
            "xgent-installer",
        );
        let result = classify_readonly(&f, &selected);
        let pins: Vec<_> = result
            .exclusive
            .iter()
            .filter(|e| e.kind == EntryKind::Pin)
            .collect();
        if modified {
            assert!(pins.is_empty());
            for path in [
                ".claude/skills/polish/SKILL.md",
                ".agents/skills/polish/SKILL.md",
                ".opencode/commands/impeccable-polish.md",
            ] {
                assert!(has_conflict(
                    &result,
                    path,
                    ConflictKind::UnknownLegacyEntry
                ));
            }
        } else {
            assert!(result.conflicts.is_empty());
            assert_eq!(pins.len(), 3);
            for pin in pins {
                assert_eq!(
                    sha256(&fs::read(f.project.join(&pin.path)).unwrap()),
                    pin.sha256
                );
                assert!(pin
                    .sources
                    .iter()
                    .any(|s| s.channel == "fixed-pin-template"));
            }
        }
    }
}

#[test]
fn copied_project_skill_is_owned_but_personal_copy_is_outside_inventory() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize(
        "0.6.0",
        &selected,
        &["copied-skill", "personal-shadow"],
        "xgent-installer",
    );
    let home = Path::new(f.metadata["homeRoot"].as_str().unwrap());
    fs::write(
        home.join(".claude/skills/impeccable/SKILL.md"),
        b"personal custom shadow",
    )
    .unwrap();
    let result = classify_readonly(&f, &selected);
    assert!(result.conflicts.is_empty());
    assert!(!result.exclusive.is_empty());
    assert!(result
        .exclusive
        .iter()
        .all(|e| !e.path.contains("user home") && !e.path.contains("skills-sh source")));
}

#[cfg(unix)]
#[test]
fn old_skill_symlink_is_a_conflict_and_external_target_is_untouched() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &["symlink-skill"], "xgent-installer");
    let result = classify_readonly(&f, &selected);
    assert!(has_conflict(
        &result,
        ".claude/skills/impeccable",
        ConflictKind::Symlink
    ));
    assert!(!result
        .exclusive
        .iter()
        .any(|e| e.path.starts_with(".claude/skills/impeccable/")));
}

#[cfg(unix)]
#[test]
fn provider_ancestor_and_dangling_shared_file_symlinks_are_never_followed() {
    use std::os::unix::fs::symlink;
    for ancestor in [false, true] {
        let selected = vec![".cursor".into()];
        let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
        let path = if ancestor {
            let outside = f.sandbox.join("outside provider");
            fs::rename(f.project.join(".cursor"), &outside).unwrap();
            symlink(&outside, f.project.join(".cursor")).unwrap();
            ".cursor"
        } else {
            fs::remove_file(f.project.join(".cursor/hooks.json")).unwrap();
            symlink(
                f.sandbox.join("missing target"),
                f.project.join(".cursor/hooks.json"),
            )
            .unwrap();
            ".cursor/hooks.json"
        };
        let result = classify_readonly(&f, &selected);
        assert!(has_conflict(&result, path, ConflictKind::Symlink));
        assert!(!result.shared.iter().any(|s| s.path == ".cursor/hooks.json"));
        if ancestor {
            assert!(result.exclusive.is_empty());
        }
    }
}

#[cfg(unix)]
#[test]
fn unrelated_auxiliary_symlink_is_preserved_without_ownership() {
    use std::os::unix::fs::symlink;
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let outside = f.sandbox.join("user auxiliary");
    fs::create_dir(&outside).unwrap();
    fs::write(
        outside.join("SKILL.md"),
        b"<!-- impeccable-pinned-skill -->\ncustom outside",
    )
    .unwrap();
    symlink(&outside, f.project.join(".claude/skills/auxiliary")).unwrap();
    let result = classify_readonly(&f, &selected);
    assert!(result.conflicts.is_empty());
    assert!(result
        .preserved
        .iter()
        .any(|p| p.path == ".claude/skills/auxiliary"));
    assert!(!result
        .exclusive
        .iter()
        .any(|e| e.path.contains("auxiliary")));
}

#[test]
fn live_residue_and_journal_referenced_sources_block_without_recovery_writes() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &["live-residue"], "bundle");
    let result = classify_readonly(&f, &selected);
    for path in [
        ".impeccable-live.json",
        ".impeccable/live/config.json",
        ".impeccable/live/inject-journal.json",
        ".impeccable/live/server.json",
        "index.html",
    ] {
        assert!(has_conflict(&result, path, ConflictKind::LiveResidue));
    }
    assert!(result
        .conflicts
        .iter()
        .all(|c| c.message.contains("old project launcher")
            || c.message.contains("live-inject --remove")));
    assert!(!result.exclusive.iter().any(|e| e.path == "index.html"));
    let mut journal = read_json(&f, ".impeccable/live/inject-journal.json");
    journal["artifacts"][0]["path"] = Value::String("../user home/personal.txt".into());
    write_json(&f, ".impeccable/live/inject-journal.json", &journal);
    let result = classify_readonly(&f, &selected);
    assert!(result
        .conflicts
        .iter()
        .any(|c| c.message.contains("unsafe project reference")));
}

#[test]
fn invalid_selection_fails_before_creating_any_project_state() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let before = inventory(&f.sandbox);
    for providers in [vec![], vec!["../personal".into()], vec![".unknown".into()]] {
        assert!(classify_project(&f.project, &providers).is_err());
    }
    assert_eq!(inventory(&f.sandbox), before);
    assert!(!f.project.join(".phoenix-ui").exists());
}

#[test]
fn handler_numbers_require_the_supported_exact_structure() {
    let selected = vec![".cursor".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let path = ".cursor/hooks.json";
    let original = fs::read_to_string(f.project.join(path)).unwrap();
    assert!(original.contains("\"timeout\": 5"));
    for literal in ["5.0", "5e0", "5.0000000000000001", "6"] {
        fs::write(
            f.project.join(path),
            original.replacen("\"timeout\": 5", &format!("\"timeout\": {literal}"), 1),
        )
        .unwrap();
        let result = classify_readonly(&f, &selected);
        assert!(
            has_conflict(&result, path, ConflictKind::CustomizedHandler),
            "{literal}: {:?}",
            result.conflicts
        );
        assert_eq!(handler_count(&result), 0);
    }
}

#[cfg(unix)]
#[test]
fn special_legacy_file_is_rejected_without_opening_a_fifo() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let path = ".claude/skills/impeccable/scripts/impeccable";
    let full = f.project.join(path);
    fs::remove_file(&full).unwrap();
    let cpath = CString::new(full.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
    let result = classify_readonly(&f, &selected);
    assert!(has_conflict(&result, path, ConflictKind::Unreadable));
    assert!(!result.exclusive.iter().any(|e| e.path == path));
}

#[cfg(unix)]
#[test]
fn unknown_legacy_name_shortcut_symlink_is_a_conflict_without_following() {
    use std::os::unix::fs::symlink;
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let path = ".claude/commands/impeccable-custom.md";
    fs::create_dir_all(f.project.join(path).parent().unwrap()).unwrap();
    let target = f.sandbox.join("external custom shortcut.md");
    fs::write(&target, b"user content outside the project").unwrap();
    symlink(&target, f.project.join(path)).unwrap();
    let result = classify_readonly(&f, &selected);
    assert!(has_conflict(&result, path, ConflictKind::Symlink));
    assert!(!result.exclusive.iter().any(|e| e.path == path));
}

#[test]
fn unknown_legacy_name_extensionless_agent_and_command_are_conflicts() {
    let selected = vec![".claude".into()];
    let f = Fixture::materialize("0.6.0", &selected, &[], "bundle");
    let paths = [".claude/agents/impeccable", ".claude/commands/impeccable"];
    for path in paths {
        fs::create_dir_all(f.project.join(path).parent().unwrap()).unwrap();
        fs::write(f.project.join(path), b"unknown old-name active entry").unwrap();
    }
    let result = classify_readonly(&f, &selected);
    for path in paths {
        assert!(has_conflict(
            &result,
            path,
            ConflictKind::UnknownLegacyEntry
        ));
        assert!(!result.exclusive.iter().any(|e| e.path == path));
    }
}

#[test]
fn gemini_named_old_handler_with_a_completely_unknown_command_is_a_conflict() {
    for version in ["0.4.0", "0.5.0", "0.6.0"] {
        let selected = vec![".gemini".into()];
        let f = Fixture::materialize(version, &selected, &[], "bundle");
        let path = ".gemini/settings.json";
        let mut value = read_json(&f, path);
        for (event, remaining) in [("BeforeTool", 1), ("AfterAgent", 0)] {
            value["hooks"][event][0]["hooks"][0]["command"] =
                Value::String("echo user replacement && unknown-design-command".into());
            value["hooks"][event][0]["hooks"].as_array_mut().unwrap().push(
                serde_json::json!({"name":"user-neighbor","type":"command","command":"echo impeccable notes", "unknown":true}),
            );
            write_json(&f, path, &value);
            let result = classify_readonly(&f, &selected);
            assert!(has_conflict(&result, path, ConflictKind::CustomizedHandler));
            assert_eq!(handler_count(&result), remaining);
            assert_eq!(result.conflicts.len(), 2 - remaining);
            assert!(result
                .conflicts
                .iter()
                .all(|c| !c.message.contains("handler 1")));
        }
    }
}
