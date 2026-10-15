use phoenix_ui_install::{
    classify::classify_project, data::prepare_legacy_data, sha256, Change, ExpectedFile, Manifest,
    Plan, Project, RollbackPolicy,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf, PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::var_os("PHOENIX_DATA_FIXTURE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = root.join(format!(
            "data-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        let path = path.canonicalize().unwrap();
        Self(path.clone(), path)
    }
    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn project(&self) -> Project {
        Project::open(&self.0).unwrap()
    }
    fn materialize(version: &str, selected: &[String], scenarios: &[&str]) -> Self {
        Self::materialize_channel(version, selected, scenarios, "bundle")
    }
    fn materialize_channel(
        version: &str,
        selected: &[String],
        scenarios: &[&str],
        channel: &str,
    ) -> Self {
        let mut fixture = Self::new();
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .unwrap()
            .join("test/phoenix-ui/fixtures/migration/materialize.mjs");
        let options = json!({"fixtureRoot":fixture.0,"packageVersion":version,"providers":selected,"channel":channel,"scenarios":scenarios});
        let output=Command::new("node").args(["--input-type=module","-e","import {pathToFileURL} from 'node:url'; const {materializeMigrationFixture}=await import(pathToFileURL(process.argv[2])); materializeMigrationFixture(JSON.parse(process.argv[3]));","data-fixture-import"]).arg(script).arg(options.to_string()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fixture.0.push("project with spaces");
        fixture
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::env::var_os("PHOENIX_DATA_FIXTURE_ROOT").is_none() {
            fs::remove_dir_all(&self.1).unwrap();
        }
    }
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, (String, u32)> {
    fn walk(root: &Path, at: &Path, values: &mut BTreeMap<PathBuf, (String, u32)>) {
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
            let hash = if meta.is_symlink() {
                format!("symlink:{}", fs::read_link(&path).unwrap().display())
            } else if meta.is_dir() {
                "directory".into()
            } else if meta.is_file() {
                sha256(&fs::read(&path).unwrap())
            } else {
                "special".into()
            };
            values.insert(path.strip_prefix(root).unwrap().into(), (hash, mode));
            if meta.is_dir() && !meta.is_symlink() {
                walk(root, &path, values);
            }
        }
    }
    let mut values = BTreeMap::new();
    walk(root, root, &mut values);
    values
}

fn prepare(project: &Project, selected: &[String]) -> phoenix_ui_install::data::PreparedData {
    let before = inventory(project.root());
    let classified = classify_project(project.root(), selected).unwrap();
    let prepared = prepare_legacy_data(project, &classified, true).unwrap();
    assert_eq!(
        inventory(project.root()),
        before,
        "data preparation wrote project bytes, paths or permissions"
    );
    if std::env::var_os("PHOENIX_DATA_FIXTURE_ROOT").is_some() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = project.root().parent().unwrap().join(format!(
            "data-receipt-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(path,serde_json::to_vec_pretty(&json!({"project":project.root(),"readonlyVerified":true,"entriesCompared":before.len(),"prepared":prepared})).unwrap()).unwrap();
    }
    prepared
}

fn selected(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}
fn payload<'a>(prepared: &'a phoenix_ui_install::data::PreparedData, target: &str) -> &'a [u8] {
    prepared
        .changes
        .iter()
        .find_map(|change| match change {
            Change::Exclusive {
                path,
                after: Some(payload),
                ..
            } if path == target => Some(payload.bytes.as_slice()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing persistent data/pin payload for {target}"))
}
fn plan(prepared: phoenix_ui_install::data::PreparedData, id: &str) -> Plan {
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    Plan {
        transaction_id: id.into(),
        manifest: Manifest {
            version: "data-fixture".into(),
            sha256: sha256(b"fixed-data-fixture"),
            providers: selected(&[".claude"]),
            migration_sources: vec!["0.6.0".into()],
        },
        expected_receipt: ExpectedFile::Absent,
        changes: prepared.changes,
    }
}

#[test]
fn empty_project_prepares_no_data_or_state() {
    let fixture = Fixture::new();
    let prepared = prepare(&fixture.project(), &selected(&[".claude"]));
    assert!(prepared.changes.is_empty());
    assert!(prepared.sources.is_empty());
    assert!(prepared.conflicts.is_empty());
    assert!(!fixture.0.join(".phoenix-ui").exists());
}

#[test]
fn config_and_local_copy_preserve_bytes_unknown_fields_and_use_retain_rollback() {
    let fixture = Fixture::new();
    let shared=b"{\n  \"hook\":{\"enabled\":true},\n \"buildPath\":\"comp\",\n \"unknown\":{\"precise\":1.230000000000000000000000}\n}\n";
    let local = b"{\"browser\":\"Safari\",\"projectRoots\":[\"apps/*\"],\"localUnknown\":true}\n";
    fixture.write(".impeccable/config.json", shared);
    fixture.write(".impeccable/config.local.json", local);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, mode) in [
            (".impeccable/config.json", 0o600),
            (".impeccable/config.local.json", 0o640),
        ] {
            fs::set_permissions(fixture.0.join(path), fs::Permissions::from_mode(mode)).unwrap();
        }
    }
    let project = fixture.project();
    let prepared = prepare(&project, &selected(&[".claude"]));
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    assert_eq!(payload(&prepared, ".phoenix-ui/config.json"), shared);
    assert_eq!(payload(&prepared, ".phoenix-ui/config.local.json"), local);
    assert!(prepared.unconsumed.iter().any(|u| u.field == "unknown"));
    assert!(prepared.changes.iter().all(|c| matches!(
        c,
        Change::Exclusive {
            rollback: RollbackPolicy::Retain,
            ..
        }
    )));
    let p = plan(prepared, "data-copy-retain");
    project.apply(&p).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, mode) in [
            (".phoenix-ui/config.json", 0o600),
            (".phoenix-ui/config.local.json", 0o640),
        ] {
            assert_eq!(
                fs::metadata(fixture.0.join(path))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                mode
            );
        }
    }
    fs::write(
        fixture.0.join(".phoenix-ui/config.json"),
        b"{\"newUserChoice\":true}",
    )
    .unwrap();
    project.rollback(&p.transaction_id).unwrap();
    assert_eq!(
        fs::read(fixture.0.join(".phoenix-ui/config.json")).unwrap(),
        b"{\"newUserChoice\":true}"
    );
    assert_eq!(
        fs::read(fixture.0.join(".impeccable/config.json")).unwrap(),
        shared
    );
}

/// Phoenix readers parse `.phoenix-ui` config and sidecar files as strict JSON, so a
/// commented file would migrate and then be unreadable or silently ignored.
#[test]
fn commented_config_and_sidecar_stay_in_place_as_conflicts() {
    for (path, bytes) in [
        (
            ".impeccable/config.json",
            &b"{\n // user comment\n \"buildPath\":\"code\"\n}\n"[..],
        ),
        (
            ".impeccable/config.local.json",
            &b"{\"browser\":\"Safari\" /* mine */}"[..],
        ),
        (
            ".impeccable/design.json",
            &b"{\n // generated\n \"schemaVersion\":2\n}\n"[..],
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(path, bytes);
        let prepared = prepare(&fixture.project(), &selected(&[".claude"]));
        assert!(prepared.changes.is_empty(), "{path}");
        let conflict = prepared
            .conflicts
            .iter()
            .find(|c| c.path == path)
            .unwrap_or_else(|| panic!("{path}: {:?}", prepared.conflicts));
        assert_eq!(
            conflict.kind,
            phoenix_ui_install::classify::ConflictKind::InvalidJson
        );
        assert!(conflict.message.contains("strict JSON"), "{conflict:?}");
        assert!(!fixture.0.join(".phoenix-ui").exists());
    }
}

#[test]
fn different_existing_phoenix_config_is_a_readonly_conflict() {
    let fixture = Fixture::new();
    fixture.write(".impeccable/config.json", b"{\"buildPath\":\"comp\"}");
    fixture.write(".phoenix-ui/config.json", b"{\"buildPath\":\"code\"}");
    let prepared = prepare(&fixture.project(), &selected(&[".claude"]));
    assert!(prepared.changes.is_empty());
    assert!(prepared
        .conflicts
        .iter()
        .any(|c| c.path == ".phoenix-ui/config.json"));
}

#[test]
fn fixed_pins_have_per_file_payload_evidence_and_restore_old_bytes() {
    let ids = selected(&[".claude", ".agents", ".codex", ".opencode"]);
    let fixture = Fixture::materialize("0.6.0", &ids, &["pin-shortcut"]);
    let project = fixture.project();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [
            ".claude/skills/polish/SKILL.md",
            ".agents/skills/polish/SKILL.md",
            ".codex/skills/polish/SKILL.md",
            ".opencode/commands/impeccable-polish.md",
        ] {
            fs::set_permissions(project.root().join(path), fs::Permissions::from_mode(0o640))
                .unwrap();
        }
    }
    let classified = classify_project(project.root(), &ids).unwrap();
    let old: Vec<_> = classified
        .exclusive
        .iter()
        .filter(|f| f.kind == phoenix_ui_install::classify::EntryKind::Pin)
        .map(|f| {
            (
                f.path.clone(),
                fs::read(project.root().join(&f.path)).unwrap(),
            )
        })
        .collect();
    let prepared = prepare(&project, &ids);
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    assert_eq!(
        prepared.resolved_pin_paths.len(),
        4,
        "all complete fixed pins need new payload evidence"
    );
    for pin in &prepared.resolved_pin_paths {
        assert_eq!(sha256(&pin.payload.bytes), pin.payload.sha256);
        assert_eq!(payload(&prepared, &pin.new_path), pin.payload.bytes);
        let text = String::from_utf8(pin.payload.bytes.clone()).unwrap();
        assert!(text.contains("phoenix-ui-pinned-"));
        assert!(!text.contains("impeccable"));
    }
    let p = plan(prepared, "fixed-pin-regeneration");
    project.apply(&p).unwrap();
    assert!(project
        .root()
        .join(".opencode/commands/phoenix-ui-polish.md")
        .exists());
    assert!(!project
        .root()
        .join(".opencode/commands/impeccable-polish.md")
        .exists());
    project.rollback(&p.transaction_id).unwrap();
    for (path, bytes) in old {
        assert_eq!(fs::read(project.root().join(&path)).unwrap(), bytes);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(project.root().join(&path))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                0o640
            );
        }
    }
    assert!(!project
        .root()
        .join(".opencode/commands/phoenix-ui-polish.md")
        .exists());
}

fn design_bytes() -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({"schemaVersion":2,"generatedAt":"2026-10-08T00:00:00Z","title":"Source design",
        "extensions":{"colorMeta":{"primary":{"role":"primary","displayName":"Red","canonical":"#f00","tonalRamp":["#f00"],"tokenUnknown":7}},"motion":[{"name":"ease","value":"ease","purpose":"transition","motionUnknown":true}]},
        "components":[{"name":"Button","kind":"button","refersTo":"button","description":"body","html":"<button>impeccable source prose</button>","css":"button{}","componentUnknown":true}],
        "narrative":{"northStar":"purpose","rules":[{"name":"Rule","body":"Keep impeccable in historical text","section":"colors","ruleUnknown":"keep"}],"dos":["use tokens"],"donts":["rewrite prose"]},"designUnknown":{"keep":true}})).unwrap()
}

#[test]
fn registered_path_fields_change_without_rewriting_unknown_text() {
    let fixture = Fixture::new();
    let bytes=b"{\n  \"hook\":{\"auditLog\":\".impeccable/logs/audit.jsonl\",\"unknown\":\".impeccable/untouched\"},\n \"unknownNumber\":1.2300000000000000000000\n}\n";
    fixture.write(".impeccable/config.json", bytes);
    let prepared = prepare(&fixture.project(), &selected(&[".claude"]));
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    let expected = String::from_utf8(bytes.to_vec()).unwrap().replacen(
        ".impeccable/logs/audit.jsonl",
        ".phoenix-ui/logs/audit.jsonl",
        1,
    );
    assert_eq!(
        payload(&prepared, ".phoenix-ui/config.json"),
        expected.as_bytes()
    );
    assert!(prepared
        .mappings
        .iter()
        .any(|m| m.field.as_deref() == Some("hook.auditLog")
            && m.after == ".phoenix-ui/logs/audit.jsonl"));
}

#[test]
fn brief_and_historical_target_paths_map_only_to_validated_copied_artifacts() {
    let fixture = Fixture::new();
    fixture.write(".impeccable/design.json", &design_bytes());
    let brief=b"---\nversion: 1\nslug: \"design\"\nprimary_target: \".impeccable/design.json\"\nrelated_targets: [\"route:/home\",\".impeccable/design.json\"]\nunknown: .impeccable/keep\n---\n\nKeep impeccable prose.\n";
    fixture.write(".impeccable/surfaces/design.md", brief);
    let target = fixture
        .0
        .join(".impeccable/design.json")
        .to_string_lossy()
        .into_owned();
    let history = format!(
        "---\ntimestamp: 2026-10-08T00-00-00Z\nslug: design\nclosed: true\ntarget_path: {}\ntarget_identity: {}\nunknown: .impeccable/keep\n---\nAccepted prose is historical evidence only.\n",
        serde_json::to_string(&target).unwrap(),
        serde_json::to_string(&format!("file:{target}")).unwrap()
    );
    fixture.write(
        ".impeccable/critique/2026-10-08T00-00-00Z~0001__design.md",
        history.as_bytes(),
    );
    let prepared = prepare(&fixture.project(), &selected(&[".claude"]));
    assert!(prepared.conflicts.is_empty(), "{:?}", prepared.conflicts);
    assert_eq!(
        payload(&prepared, ".phoenix-ui/design.json"),
        design_bytes()
    );
    let rewritten =
        String::from_utf8(payload(&prepared, ".phoenix-ui/surfaces/design.md").to_vec()).unwrap();
    assert!(rewritten.contains("primary_target: \".phoenix-ui/design.json\""));
    assert!(rewritten.contains("unknown: .impeccable/keep"));
    assert!(rewritten.ends_with("Keep impeccable prose.\n"));
    let history = String::from_utf8(
        payload(
            &prepared,
            ".phoenix-ui/critique/2026-10-08T00-00-00Z~0001__design.md",
        )
        .to_vec(),
    )
    .unwrap();
    assert!(history.contains("closed: true"));
    assert!(history.contains(".phoenix-ui/design.json"));
    for field in [
        "primary_target",
        "related_targets[1]",
        "target_path",
        "target_identity",
    ] {
        assert!(
            prepared
                .mappings
                .iter()
                .any(|m| m.field.as_deref() == Some(field)),
            "missing field mapping {field}"
        );
    }
}

#[cfg(unix)]
#[test]
fn identical_existing_data_keeps_current_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let bytes = b"{\"browser\":\"Safari\"}";
    fixture.write(".impeccable/config.local.json", bytes);
    fixture.write(".phoenix-ui/config.local.json", bytes);
    fs::set_permissions(
        fixture.0.join(".impeccable/config.local.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    fs::set_permissions(
        fixture.0.join(".phoenix-ui/config.local.json"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    let project = fixture.project();
    let prepared = prepare(&project, &selected(&[".claude"]));
    let p = plan(prepared, "identical-local-data");
    project.apply(&p).unwrap();
    assert_eq!(
        fs::metadata(fixture.0.join(".phoenix-ui/config.local.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o640
    );
}

#[test]
fn question_residue_names_the_actual_stop_key_and_blocks_all_activation_changes() {
    let fixture = Fixture::new();
    fixture.write(".impeccable/config.json", b"{\"buildPath\":\"code\"}");
    fixture.write(
        ".impeccable/questions/ab12-question.state.json",
        b"{\"pid\":2147483647,\"port\":0,\"token\":\"fixture-do-not-copy\"}",
    );
    let prepared = prepare(&fixture.project(), &selected(&[".claude"]));
    assert!(prepared.changes.is_empty());
    assert!(
        prepared.conflicts.iter().any(|c| c
            .message
            .contains("serve-question --stop --key 'ab12-question'")),
        "{:?}",
        prepared.conflicts
    );
    let evidence = serde_json::to_string(&prepared).unwrap();
    assert!(!evidence.contains("fixture-do-not-copy"));
}

#[test]
fn invalid_config_fields_and_duplicate_json_keys_block_without_writes() {
    let cases: &[&[u8]] = &[
        b"{broken",
        b"[]",
        b"{\"buildPath\":\"code\",\"buildPath\":\"comp\"}",
        b"{\"hook\":{\"enabled\":\"true\"}}",
        b"{\"hook\":{\"quiet\":3}}",
        b"{\"hook\":{\"consent\":\"maybe\"}}",
        b"{\"hook\":{\"perEditRules\":\"later\"}}",
        b"{\"hook\":{\"limits\":{\"maxChars\":0}}}",
        b"{\"detector\":{\"ignoreRules\":[1]}}",
        b"{\"detector\":{\"ignoreValues\":[{\"rule\":\"x\"}]}}",
        b"{\"detector\":{\"extensions\":[false]}}",
        b"{\"detector\":{\"designSystem\":{\"enabled\":0}}}",
        b"{\"detector\":{\"advisoryRules\":\"sometimes\"}}",
        b"{\"browser\":\"\"}",
        b"{\"buildPath\":\"other\"}",
        b"{\"updateCheck\":1}",
        b"{\"projectRoots\":false}",
        b"{\"onlyUnknown\":true}",
    ];
    for bytes in cases {
        let f = Fixture::new();
        f.write(".impeccable/config.json", bytes);
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty(), "{}", String::from_utf8_lossy(bytes));
        assert!(p
            .conflicts
            .iter()
            .any(|c| c.path == ".impeccable/config.json"));
    }
    println!("data-invalid-config-cases {}", cases.len());
}

#[test]
fn nested_config_fields_validate_and_unknown_members_remain_unconsumed() {
    let f = Fixture::new();
    let bytes=serde_json::to_vec_pretty(&json!({"hook":{"enabled":true,"quiet":false,"perEditRules":"all","consent":"accepted","limits":{"maxFindings":7,"maxChars":8000,"maxFileBytes":131072,"limitNote":"keep"}},"detector":{"advisoryRules":"include","ignoreRules":["rule"],"ignoreFiles":["generated/*"],"ignoreValues":[{"rule":"x","value":"y","files":["src/*"],"createdAt":"stamp","reason":"purpose","ignoreNote":true}],"designSystem":{"enabled":true,"systemNote":3},"extensions":["css",{"ext":".html","extensionNote":"keep"}]},"stalenessCheck":false,"$schema":"https://example.invalid/schema","version":1})).unwrap();
    f.write(".impeccable/config.json", &bytes);
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert_eq!(payload(&p, ".phoenix-ui/config.json"), bytes);
    for field in [
        "hook.limits.limitNote",
        "detector.ignoreValues[0].ignoreNote",
        "detector.designSystem.systemNote",
        "detector.extensions[1].extensionNote",
        "stalenessCheck",
        "$schema",
        "version",
    ] {
        assert!(p.unconsumed.iter().any(|u| u.field == field), "{field}");
    }
}

#[test]
fn runtime_fields_embedded_in_persistent_config_are_never_copied() {
    for field in [
        "pid",
        "port",
        "token",
        "pending",
        "cache",
        "dedup",
        "live",
        "sessionId",
        "capture",
    ] {
        let f = Fixture::new();
        let mut value = json!({"hook":{"enabled":true},"runtimeUnknown":{}});
        value["runtimeUnknown"][field] = json!("secret-fixture-state");
        f.write(
            ".impeccable/config.json",
            &serde_json::to_vec(&value).unwrap(),
        );
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty());
        assert!(!p.conflicts.is_empty());
        assert!(!serde_json::to_string(&p)
            .unwrap()
            .contains("secret-fixture-state"));
        let f = Fixture::new();
        let mut value: Value = serde_json::from_slice(&design_bytes()).unwrap();
        value[field] = json!("secret-fixture-state");
        f.write(
            ".impeccable/design.json",
            &serde_json::to_vec(&value).unwrap(),
        );
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(
            p.changes.is_empty(),
            "runtime root field {field} in design sidecar"
        );
        assert!(!p.conflicts.is_empty());
        assert!(!serde_json::to_string(&p)
            .unwrap()
            .contains("secret-fixture-state"));
    }
    println!("data-runtime-field-cases {}", 18);
}

#[test]
fn schema_two_design_preserves_all_bytes_and_reports_nested_unknowns() {
    let f = Fixture::new();
    let bytes = design_bytes();
    f.write(".impeccable/design.json", &bytes);
    f.write("DESIGN.md", b"# impeccable is historical project text\n");
    f.write("PRODUCT.md", b"Do not rebrand this product.\n");
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert_eq!(payload(&p, ".phoenix-ui/design.json"), bytes);
    for field in [
        "designUnknown",
        "extensions.colorMeta.primary.tokenUnknown",
        "extensions.motion[0].motionUnknown",
        "components[0].componentUnknown",
        "narrative.rules[0].ruleUnknown",
    ] {
        assert!(p.unconsumed.iter().any(|u| u.field == field));
    }
    assert!(p
        .changes
        .iter()
        .all(|c| c.path().starts_with(".phoenix-ui/")));
}

#[test]
fn unsupported_design_versions_and_known_nested_wrong_types_are_blocking() {
    for edit in [
        "old-schema",
        "future-schema",
        "color-shape",
        "component-shape",
        "narrative-shape",
        "no-marker",
    ] {
        let f = Fixture::new();
        let mut value: Value = serde_json::from_slice(&design_bytes()).unwrap();
        match edit {
            "old-schema" => value["schemaVersion"] = json!(1),
            "future-schema" => value["schemaVersion"] = json!(3),
            "color-shape" => {
                value["extensions"]["colorMeta"]["primary"]["tonalRamp"] = json!(false)
            }
            "component-shape" => value["components"][0]["html"] = json!(12),
            "narrative-shape" => value["narrative"]["dos"] = json!("use tokens"),
            "no-marker" => {
                value.as_object_mut().unwrap().remove("schemaVersion");
            }
            _ => unreachable!(),
        }
        f.write(
            ".impeccable/design.json",
            &serde_json::to_vec(&value).unwrap(),
        );
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty(), "{edit}");
        assert!(!p.conflicts.is_empty(), "{edit}");
    }
}

#[test]
fn producer_briefs_and_closed_critique_history_keep_prose_crlf_and_unknowns() {
    let f = Fixture::new();
    let brief=b"---\r\nversion: 1\r\nslug: \"home\"\r\nprimary_target: \"src/home.tsx\"\r\nrelated_targets: [\"route:/home\",\"https://example.invalid/home\"]\r\nunknown: impeccable prose\r\n---\r\n\r\n# Surface brief\r\nKeep source text.\r\n";
    let history=b"---\ntimestamp: 2026-10-08T00-00-00Z\nslug: home\nclosed: true\nunknown: accepted prose does not create trust\n---\nDo not recompute or relabel history.\n";
    f.write(".impeccable/surfaces/home.md", brief);
    f.write(
        ".impeccable/critique/2026-10-08T00-00-00Z__home.md",
        history,
    );
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert_eq!(payload(&p, ".phoenix-ui/surfaces/home.md"), brief);
    assert_eq!(
        payload(&p, ".phoenix-ui/critique/2026-10-08T00-00-00Z__home.md"),
        history
    );
    assert!(p.changes.iter().all(|c| matches!(
        c,
        Change::Exclusive {
            rollback: RollbackPolicy::Retain,
            ..
        }
    )));
}

#[test]
fn malformed_or_filename_only_briefs_and_history_are_not_claimed() {
    for (path, bytes) in [
        (".impeccable/surfaces/home.md", "# No producer schema\n"),
        (
            ".impeccable/surfaces/home.md",
            "---\nversion: 1\nversion: 1\nslug: \"home\"\nprimary_target: \"home.html\"\nrelated_targets: []\n---\n",
        ),
        (
            ".impeccable/surfaces/home.md",
            "---\nversion: 1\nslug: \"other\"\nprimary_target: \"home.html\"\nrelated_targets: []\n---\n",
        ),
        (
            ".impeccable/critique/custom.md",
            "---\ntimestamp: 2026-10-08T00-00-00Z\nslug: home\n---\n",
        ),
        (
            ".impeccable/critique/2026-10-08T00-00-00Z__home.md",
            "---\ntimestamp: 2026-10-08T00-00-00Z\nslug: other\n---\n",
        ),
        (
            ".impeccable/critique/2026-10-08T00-00-00Z__home.md",
            "---\ntimestamp: 2026-10-08T00-00-00Z\nslug: home\nclosed: yes\n---\n",
        ),
    ] {
        let f = Fixture::new();
        f.write(path, bytes.as_bytes());
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty());
        assert!(p.conflicts.iter().any(|c| c.path == path));
    }
}

#[test]
fn unregistered_old_references_block_while_external_history_references_are_never_read() {
    let f = Fixture::new();
    f.write(".impeccable/surfaces/home.md",b"---\nversion: 1\nslug: \"home\"\nprimary_target: \".impeccable/build/accepted.png\"\nrelated_targets: []\n---\nKeep old source.\n");
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.changes.is_empty());
    assert!(p.conflicts.iter().any(|c| {
        c.message
            .contains("explicitly validated persistent artifact")
    }));
    let f = Fixture::new();
    f.write(".impeccable/critique/2026-10-08T00-00-00Z__home.md",b"---\ntimestamp: 2026-10-08T00-00-00Z\nslug: home\ntarget_path: /not-an-existing-outside-project-file\ntarget_identity: \"file:/not-an-existing-outside-project-file\"\n---\nHistory with old external target.\n");
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert!(
        payload(&p, ".phoenix-ui/critique/2026-10-08T00-00-00Z__home.md")
            .ends_with(b"History with old external target.\n")
    );
}

#[test]
fn unknown_persistent_build_accepted_history_and_brief_families_are_preserved_conflicts() {
    for path in [
        ".impeccable/brief.json",
        ".impeccable/build/state.json",
        ".impeccable/build/spec.json",
        ".impeccable/mocks/accepted.png",
        ".impeccable/accepted/current.json",
        ".impeccable/history/old.json",
        ".impeccable/review/components.json",
    ] {
        let f = Fixture::new();
        f.write(".impeccable/config.json", b"{\"buildPath\":\"code\"}");
        f.write(path, b"unsupported synthetic family");
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty(), "{path}");
        assert!(
            p.conflicts.iter().any(|c| path.starts_with(&c.path)),
            "{path}: {:?}",
            p.conflicts
        );
        assert!(p.unconsumed.iter().any(|u| path.starts_with(&u.path)));
        assert_eq!(
            fs::read(f.0.join(path)).unwrap(),
            b"unsupported synthetic family"
        );
    }
}

#[test]
fn disposable_queues_and_cache_remain_old_and_are_not_migrated_or_dumped() {
    let f = Fixture::new();
    for path in [
        ".impeccable/hook.cache.json",
        ".impeccable/hook.pending.json",
        ".impeccable/cache/session.json",
        ".impeccable/pending/answer.json",
        ".impeccable/dedup/session.json",
    ] {
        f.write(path, b"secret-fixture-state-never-read");
    }
    f.write(".impeccable/config.json", b"{\"buildPath\":\"code\"}");
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.unconsumed.len(), 5);
    assert!(!serde_json::to_string(&p)
        .unwrap()
        .contains("secret-fixture-state-never-read"));
}

#[test]
fn live_and_source_only_injection_or_variant_residue_blocks_without_deleting_markers() {
    for marker in [
        "<!-- impeccable-live-start -->",
        "<!-- impeccable-live-end -->",
        "{/* impeccable-live-start */}",
        "/* impeccable-live-end */",
        "{/* impeccable-variants-start ab12cd34 */}",
        "/* impeccable-variants-end ab12cd34 */",
        "{/* impeccable-carbonize-start */}",
        "/* impeccable-carbonize-end */",
        "data-impeccable-csp-original",
        "<!-- impeccable-variants-start id -->",
        "<!-- impeccable-variants-end id -->",
        "data-impeccable-variants",
        "<!-- impeccable-carbonize-start -->",
        "<!-- impeccable-carbonize-end -->",
        "impeccable-param-values",
    ] {
        let f = Fixture::new();
        f.write(".impeccable/config.json", b"{\"hook\":{\"enabled\":true}}");
        f.write("src/page.html", marker.as_bytes());
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty());
        assert!(p
            .conflicts
            .iter()
            .any(|c| c.path == "src/page.html" && c.message.contains("live-inject --remove")));
        assert_eq!(
            fs::read(f.0.join("src/page.html")).unwrap(),
            marker.as_bytes()
        );
    }
    for path in [
        ".impeccable/live/sessions/ab.jsonl",
        ".impeccable-live/sessions/ab.jsonl",
        ".impeccable/capture/ready.json",
    ] {
        let f = Fixture::new();
        f.write(
            path,
            b"{\"pid\":2147483647,\"port\":0,\"token\":\"fixture-runtime-secret\"}",
        );
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty());
        assert!(!p.conflicts.is_empty());
        assert!(!serde_json::to_string(&p)
            .unwrap()
            .contains("fixture-runtime-secret"));
    }
}

#[cfg(unix)]
#[test]
fn state_target_source_and_pin_symlinks_do_not_read_or_write_external_bytes() {
    use std::os::unix::fs::symlink;
    for link in [
        ".impeccable/config.json",
        ".impeccable/surfaces",
        ".phoenix-ui/config.json",
        "src",
        ".claude/skills/polish/SKILL.md",
    ] {
        let f = Fixture::new();
        let outside = f.1.parent().unwrap().join(format!(
            "data-outside-{}",
            f.1.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("sentinel"), b"external sentinel").unwrap();
        f.write(".impeccable/config.json", b"{\"buildPath\":\"code\"}");
        let path = f.0.join(link);
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(&outside, &path).unwrap();
        let before = inventory(&outside);
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(p.changes.is_empty(), "{link}");
        assert!(!p.conflicts.is_empty(), "{link}");
        assert_eq!(inventory(&outside), before);
        if std::env::var_os("PHOENIX_DATA_FIXTURE_ROOT").is_none() {
            fs::remove_dir_all(outside).unwrap();
        }
    }
}

#[cfg(unix)]
#[test]
fn fifo_and_oversized_sources_are_rejected_without_blocking_or_state_writes() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let f = Fixture::new();
    fs::create_dir(f.0.join(".impeccable")).unwrap();
    let path = f.0.join(".impeccable/config.json");
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.changes.is_empty());
    assert!(!p.conflicts.is_empty());
    let f = Fixture::new();
    f.write("page.html", &vec![b'x'; 4 * 1024 * 1024 + 1]);
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.changes.is_empty());
    assert!(p.conflicts.iter().any(|c| c.path == "page.html"));
}

#[test]
fn fixed_pin_inventory_staleness_customization_wrong_path_and_existing_target_are_conflicts() {
    for edit in ["changed", "missing", "wrong-path", "existing-new"] {
        let ids = selected(&[".claude", ".opencode"]);
        let f = Fixture::materialize("0.6.0", &ids, &["pin-shortcut"]);
        let project = f.project();
        let mut classified = classify_project(project.root(), &ids).unwrap();
        match edit {
            "changed" => f.write(
                ".claude/skills/polish/SKILL.md",
                b"User changed this pin after classification.",
            ),
            "missing" => fs::remove_file(f.0.join(".claude/skills/polish/SKILL.md")).unwrap(),
            "wrong-path" => {
                let bytes = fs::read(f.0.join(".claude/skills/polish/SKILL.md")).unwrap();
                f.write(".claude/skills/other/SKILL.md", &bytes);
                classified
                    .exclusive
                    .iter_mut()
                    .find(|file| {
                        file.kind == phoenix_ui_install::classify::EntryKind::Pin
                            && file.provider == ".claude"
                    })
                    .unwrap()
                    .path = ".claude/skills/other/SKILL.md".into();
            }
            "existing-new" => f.write(
                ".opencode/commands/phoenix-ui-polish.md",
                b"User owns this Phoenix command.",
            ),
            _ => unreachable!(),
        }
        let before = inventory(project.root());
        let p = prepare_legacy_data(&project, &classified, true).unwrap();
        assert_eq!(inventory(project.root()), before);
        assert!(p.changes.is_empty(), "{edit}");
        assert!(p.resolved_pin_paths.is_empty());
        assert!(!p.conflicts.is_empty());
    }
}

#[test]
fn custom_or_outside_selection_pins_are_kept_and_never_resolved() {
    let all = selected(&[".claude", ".opencode"]);
    let f = Fixture::materialize("0.6.0", &all, &["modified-pin"]);
    let p = prepare(&f.project(), &all);
    assert!(p.changes.is_empty());
    assert!(p.resolved_pin_paths.is_empty());
    assert!(!p.conflicts.is_empty());
    let f = Fixture::materialize("0.6.0", &all, &["pin-shortcut"]);
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.changes.is_empty());
    assert!(p.resolved_pin_paths.is_empty());
    assert!(p
        .conflicts
        .iter()
        .any(|c| c.kind == phoenix_ui_install::classify::ConflictKind::OutsideSelection));
}

#[test]
fn pin_apply_and_rollback_use_current_hash_cas_and_keep_later_custom_edits() {
    let ids = selected(&[".claude"]);
    let f = Fixture::materialize("0.6.0", &ids, &["pin-shortcut"]);
    let project = f.project();
    let p = plan(prepare(&project, &ids), "pin-cas-apply");
    f.write(
        ".claude/skills/polish/SKILL.md",
        b"custom after preparation",
    );
    let before = inventory(project.root());
    let e = project.apply(&p).unwrap_err();
    assert_eq!(e.kind, phoenix_ui_install::ErrorKind::Conflict);
    assert_eq!(inventory(project.root()), before);
    let f = Fixture::materialize("0.6.0", &ids, &["pin-shortcut"]);
    let project = f.project();
    let p = plan(prepare(&project, &ids), "pin-cas-rollback");
    project.apply(&p).unwrap();
    f.write(
        ".claude/skills/polish/SKILL.md",
        b"new Phoenix pin edited by the user",
    );
    let before = inventory(project.root());
    let e = project.rollback(&p.transaction_id).unwrap_err();
    assert_eq!(e.kind, phoenix_ui_install::ErrorKind::Conflict);
    assert_eq!(
        fs::read(project.root().join(".claude/skills/polish/SKILL.md")).unwrap(),
        b"new Phoenix pin edited by the user"
    );
    assert_eq!(inventory(project.root()), before);
}

#[test]
fn interrupted_data_and_pin_apply_recovers_pins_but_keeps_retained_data() {
    use phoenix_ui_install::{Checkpoint, Recovery};
    let ids = selected(&[".claude"]);
    let f = Fixture::materialize("0.6.0", &ids, &["pin-shortcut"]);
    f.write(".impeccable/config.json", b"{\"buildPath\":\"code\"}");
    let project = f.project();
    let original = fs::read(project.root().join(".claude/skills/polish/SKILL.md")).unwrap();
    let p = plan(prepare(&project, &ids), "data-pin-interruption");
    let interruption = std::panic::catch_unwind(|| {
        project.apply_with(&p, |checkpoint| {
            if checkpoint == Checkpoint::BeforeReceipt {
                panic!("fixture interrupted before receipt");
            }
        })
    });
    assert!(interruption.is_err());
    assert_eq!(
        project.recover().unwrap(),
        Recovery::RolledBack(p.transaction_id)
    );
    assert_eq!(
        fs::read(project.root().join(".claude/skills/polish/SKILL.md")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(project.root().join(".phoenix-ui/config.json")).unwrap(),
        b"{\"buildPath\":\"code\"}"
    );
    assert!(project.active_receipt_verified().unwrap().is_none());
}

#[test]
fn fixed_release_148_provider_channel_inputs_regenerate_every_exact_pin_readonly() {
    let mut inputs = 0;
    let mut pins = 0;
    for version in ["0.3.0", "0.4.0", "0.5.0", "0.6.0"] {
        for channel in ["bundle", "xgent-installer"] {
            let file = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../migration-sources/{version}.json"));
            let source: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
            let ids: Vec<String> = source["providers"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, entry)| {
                    channel != "xgent-installer" || entry["installerSelectable"] != false
                })
                .map(|(id, _)| id.clone())
                .collect();
            let f = Fixture::materialize_channel(
                version,
                &ids,
                &[
                    "unmodified",
                    "custom-allow",
                    "unrelated-entry",
                    "pin-shortcut",
                ],
                channel,
            );
            let mut selection = ids.clone();
            if selection.contains(&".agents".into()) && !selection.contains(&".codex".into()) {
                selection.push(".codex".into());
            }
            let project = f.project();
            let p = prepare(&project, &selection);
            assert!(
                p.conflicts.is_empty(),
                "{version}/{channel}: {:?}",
                p.conflicts
            );
            assert_eq!(p.resolved_pin_paths.len(), ids.len());
            for pin in &p.resolved_pin_paths {
                assert_eq!(pin.payload.sha256, sha256(&pin.payload.bytes));
                assert_eq!(payload(&p, &pin.new_path), pin.payload.bytes);
                assert_eq!(
                    pin.old_sha256,
                    sha256(&fs::read(project.root().join(&pin.old_path)).unwrap())
                );
                assert!(!String::from_utf8_lossy(&pin.payload.bytes).contains("impeccable"));
            }
            inputs += ids.len();
            pins += p.resolved_pin_paths.len();
            println!(
                "data-fixed-matrix {}",
                json!({"version":version,"channel":channel,"providerChannelInputs":ids.len(),"regeneratedPins":p.resolved_pin_paths.len(),"changes":p.changes.len(),"fixture":f.0})
            );
        }
    }
    assert_eq!(inputs, 148);
    assert_eq!(pins, 148);
    println!(
        "data-fixed-total {}",
        json!({"providerChannelInputs":inputs,"regeneratedPins":pins})
    );
}

#[test]
fn unsupported_config_reports_unknown_fields_and_keeps_each_provider_conflict() {
    use phoenix_ui_install::classify::{Conflict, ConflictKind, Inventory};
    let f = Fixture::new();
    f.write(".impeccable/config.json", b"{\"onlyUnknown\":true}");
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.changes.is_empty());
    assert!(p.unconsumed.iter().any(|u| u.field == "onlyUnknown"));
    let f = Fixture::new();
    let mut classified = Inventory::default();
    for provider in [".agents", ".codex"] {
        classified.conflicts.push(Conflict {
            path: ".codex/hooks.json".into(),
            provider: Some(provider.into()),
            kind: ConflictKind::OutsideSelection,
            message: "outside migration selection".into(),
        });
    }
    let p = prepare_legacy_data(&f.project(), &classified, true).unwrap();
    assert_eq!(
        p.conflicts.len(),
        2,
        "distinct provider ownership conflicts must survive report normalization"
    );
}

#[test]
fn generated_pages_and_nested_provider_named_source_directories_are_scanned_for_residue() {
    for path in ["dist/index.html", "src/.claude/page.html"] {
        let f = Fixture::new();
        f.write(path, b"<!-- impeccable-live-start -->");
        let p = prepare(&f.project(), &selected(&[".claude"]));
        assert!(
            p.conflicts.iter().any(|c| c.path == path),
            "active residue was skipped at {path}"
        );
    }
}

#[test]
fn absolute_registered_audit_path_moves_to_new_project_namespace() {
    let f = Fixture::new();
    let before =
        f.0.join(".impeccable/logs/audit.jsonl")
            .to_string_lossy()
            .into_owned();
    f.write(
        ".impeccable/config.json",
        &serde_json::to_vec(&json!({"hook":{"auditLog":before}})).unwrap(),
    );
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    let value: Value = serde_json::from_slice(payload(&p, ".phoenix-ui/config.json")).unwrap();
    assert_eq!(
        value["hook"]["auditLog"],
        f.0.join(".phoenix-ui/logs/audit.jsonl")
            .to_string_lossy()
            .replace('\\', "/")
    );
    assert!(p
        .mappings
        .iter()
        .any(|m| m.field.as_deref() == Some("hook.auditLog")));
}

#[test]
fn renamed_pin_payload_exists_before_its_old_command_is_retired() {
    use phoenix_ui_install::Checkpoint;
    let ids = selected(&[".opencode"]);
    let f = Fixture::materialize("0.6.0", &ids, &["pin-shortcut"]);
    let project = f.project();
    let p = plan(prepare(&project, &ids), "pin-expand-before-retire");
    let mut checked = false;
    project
        .apply_with(&p, |checkpoint| {
            if checkpoint
                == Checkpoint::AfterReplace(".opencode/commands/impeccable-polish.md".into())
            {
                checked = true;
                assert!(
                    project
                        .root()
                        .join(".opencode/commands/phoenix-ui-polish.md")
                        .exists(),
                    "new pin must exist before old command retirement"
                );
            }
        })
        .unwrap();
    assert!(checked);
}

#[test]
fn unknown_flat_frontmatter_emitted_as_plain_strings_stays_byte_exact() {
    let f = Fixture::new();
    let bytes=b"---\ntimestamp: 2026-10-08T00-00-00Z\nslug: home\nunknownObject: [object Object]\nunknownArray: a,b\nunknownString: +3\nunknownQuote: \"unfinished\n---\nUnconsumed producer metadata and prose.\n";
    f.write(".impeccable/critique/2026-10-08T00-00-00Z__home.md", bytes);
    let p = prepare(&f.project(), &selected(&[".claude"]));
    assert!(p.conflicts.is_empty(), "{:?}", p.conflicts);
    assert_eq!(
        payload(&p, ".phoenix-ui/critique/2026-10-08T00-00-00Z__home.md"),
        bytes
    );
    for field in [
        "unknownObject",
        "unknownArray",
        "unknownString",
        "unknownQuote",
    ] {
        assert!(p.unconsumed.iter().any(|u| u.field == field));
    }
}
