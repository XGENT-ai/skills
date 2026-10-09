use impeccable_common::Io;
use impeccable_context::run_pin;
use std::collections::HashMap;
use std::path::PathBuf;

struct Fixture {
    root: PathBuf,
    env: HashMap<String, String>,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-ui-pin-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let home = root.join("home").to_string_lossy().into_owned();
        let fixture = Self {
            root,
            env: HashMap::from([("HOME".into(), home.clone()), ("USERPROFILE".into(), home)]),
        };
        fixture.write("package.json", "{\"name\":\"phoenix-pin-fixture\"}\n");
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn install(&self, harness: &str) {
        self.write(
            &format!("{harness}/skills/phoenix-ui/SKILL.md"),
            "---\nname: phoenix-ui\n---\n",
        );
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path(relative)).unwrap()
    }

    fn run(&self, cwd: &str, args: &[&str]) -> (i32, String, String) {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        let (mut io, capture) = Io::captured("", self.path(cwd), self.env.clone());
        let code = run_pin(&args, &mut io);
        let stdout = String::from_utf8(capture.stdout.borrow().clone()).unwrap();
        let stderr = String::from_utf8(capture.stderr.borrow().clone()).unwrap();
        (code, stdout, stderr)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn current_install_pins_from_a_subdirectory_with_each_provider_contract() {
    let mut fixture = Fixture::new();
    for harness in [
        ".claude",
        ".cursor",
        ".codex",
        ".agents",
        ".opencode",
        ".veto",
    ] {
        fixture.install(harness);
    }
    std::fs::create_dir_all(fixture.path("app/nested")).unwrap();
    fixture.write(
        ".agents/skills/phoenix-ui/scripts/command-metadata.json",
        r#"{"audit":{"description":"Installed Phoenix audit metadata","argumentHint":"[current target]"}}"#,
    );
    fixture.env.insert(
        "PHOENIX_UI_SKILL_DIR".into(),
        fixture
            .path(".agents/skills/phoenix-ui")
            .to_string_lossy()
            .into_owned(),
    );

    let (code, stdout, stderr) = fixture.run("app/nested", &["pin", "audit"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("in 6 location(s)"), "{stdout}");
    let mut originals = Vec::new();
    for (harness, prefix) in [
        (".claude", "/"),
        (".cursor", "/"),
        (".codex", "$"),
        (".agents", "$"),
        (".veto", "/"),
    ] {
        let path = format!("{harness}/skills/audit/SKILL.md");
        let content = fixture.read(&path);
        assert!(content.contains("<!-- phoenix-ui-pinned-skill -->"));
        assert!(content.contains("Installed Phoenix audit metadata"));
        assert!(content.contains(&format!("Invoke {prefix}phoenix-ui audit")));
        assert!(!content.contains("impeccable"), "{content}");
        if prefix == "$" {
            assert!(content.contains("metadata:\n  argument-hint: \"[current target]\""));
            assert!(!content.contains("user-invocable:"));
        } else {
            assert!(content.contains("argument-hint: \"[current target]\"\nuser-invocable: true"));
        }
        originals.push((path, content));
    }
    let bridge = fixture.read(".opencode/commands/phoenix-ui-audit.md");
    assert!(bridge.contains("agent: build\nsubtask: true"));
    assert!(bridge.contains("<!-- phoenix-ui-pinned-command -->"));
    assert!(
        bridge.contains("Load the `phoenix-ui` skill via the skill tool (name: \"phoenix-ui\")")
    );
    assert!(bridge.contains("<skill-base-dir>/scripts/phoenix-ui context"));
    assert!(bridge.contains("<skill-base-dir>/reference/audit.md"));
    assert!(bridge.ends_with("$ARGUMENTS\n"));
    assert!(!fixture.path(".opencode/skills/audit").exists());
    assert!(!fixture
        .path(".opencode/commands/impeccable-audit.md")
        .exists());
    originals.push((".opencode/commands/phoenix-ui-audit.md".into(), bridge));
    assert_eq!(fixture.run("app/nested", &["pin", "audit"]).0, 0);
    for (path, content) in originals {
        assert_eq!(fixture.read(&path), content);
    }

    let (code, stdout, stderr) = fixture.run("app/nested", &["unpin", "audit"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("from 6 location(s)"), "{stdout}");
    assert!(stdout.contains("Use Phoenix UI's 'audit' workflow"));
    for harness in [".claude", ".cursor", ".codex", ".agents", ".veto"] {
        assert!(!fixture.path(&format!("{harness}/skills/audit")).exists());
        assert!(fixture
            .path(&format!("{harness}/skills/phoenix-ui/SKILL.md"))
            .is_file());
    }
    assert!(!fixture
        .path(".opencode/commands/phoenix-ui-audit.md")
        .exists());
    assert!(fixture
        .run("app/nested", &["unpin", "audit"])
        .1
        .contains("No pinned 'audit' shortcut found."));
}

#[test]
fn fallback_metadata_invokes_the_current_phoenix_skill() {
    let mut fixture = Fixture::new();
    for harness in [".claude", ".agents", ".opencode"] {
        fixture.install(harness);
    }
    fixture.write(
        ".agents/skills/phoenix-ui/scripts/command-metadata.json",
        "{}",
    );
    fixture.env.insert(
        "PHOENIX_UI_SKILL_DIR".into(),
        fixture
            .path(".agents/skills/phoenix-ui")
            .to_string_lossy()
            .into_owned(),
    );
    assert_eq!(fixture.run("", &["pin", "polish"]).0, 0);
    for (harness, prefix) in [(".claude", "/"), (".agents", "$")] {
        let content = fixture.read(&format!("{harness}/skills/polish/SKILL.md"));
        assert!(content.contains(&format!("Shortcut for {prefix}phoenix-ui polish.")));
        assert!(content.contains("argument-hint: \"[target]\""));
    }
    assert!(fixture
        .read(".opencode/commands/phoenix-ui-polish.md")
        .contains("Phoenix UI sub-command shortcut; runs the polish workflow via /phoenix-ui."));
}

#[test]
fn mixed_install_preserves_legacy_shortcuts_and_handwritten_files() {
    let fixture = Fixture::new();
    for harness in [".claude", ".cursor", ".agents", ".opencode"] {
        fixture.install(harness);
    }
    let legacy = "<!-- impeccable-pinned-skill -->\nInvoke /impeccable audit.\n";
    let handwritten = "---\nname: audit\n---\nMy own audit instructions.\n";
    let old_command = "<!-- impeccable-pinned-command -->\nLoad the impeccable skill.\n";
    fixture.write(".claude/skills/impeccable/SKILL.md", "legacy main skill\n");
    fixture.write(".claude/skills/audit/SKILL.md", legacy);
    fixture.write(".claude/skills/audit/user-notes.md", "retain these notes\n");
    fixture.write(".cursor/skills/audit/SKILL.md", handwritten);
    fixture.write(".opencode/commands/impeccable-audit.md", old_command);
    fixture.write(".opencode/commands/phoenix-ui-audit.md", old_command);
    fixture.write(".opencode/skills/audit/SKILL.md", legacy);

    let (code, stdout, stderr) = fixture.run("", &["pin", "audit"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("in 1 location(s)"), "{stdout}");
    assert!(stdout.contains("non-pinned skill already exists"));
    assert!(stdout.contains("non-pinned command already exists"));
    assert!(fixture
        .read(".agents/skills/audit/SKILL.md")
        .contains("$phoenix-ui audit"));
    let (code, stdout, stderr) = fixture.run("", &["unpin", "audit"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("from 1 location(s)"), "{stdout}");
    assert!(!fixture.path(".agents/skills/audit").exists());
    assert_eq!(fixture.read(".claude/skills/audit/SKILL.md"), legacy);
    assert_eq!(
        fixture.read(".claude/skills/audit/user-notes.md"),
        "retain these notes\n"
    );
    assert_eq!(
        fixture.read(".claude/skills/impeccable/SKILL.md"),
        "legacy main skill\n"
    );
    assert_eq!(fixture.read(".cursor/skills/audit/SKILL.md"), handwritten);
    assert_eq!(
        fixture.read(".opencode/commands/impeccable-audit.md"),
        old_command
    );
    assert_eq!(
        fixture.read(".opencode/commands/phoenix-ui-audit.md"),
        old_command
    );
    assert_eq!(fixture.read(".opencode/skills/audit/SKILL.md"), legacy);
}

#[test]
fn legacy_only_installs_are_left_for_explicit_migration() {
    let fixture = Fixture::new();
    let legacy = "<!-- impeccable-pinned-skill -->\nInvoke $impeccable live.\n";
    for (harness, name) in [(".claude", "impeccable"), (".codex", "i-impeccable")] {
        fixture.write(
            &format!("{harness}/skills/{name}/SKILL.md"),
            "legacy installation\n",
        );
        fixture.write(&format!("{harness}/skills/live/SKILL.md"), legacy);
    }
    fixture.write(
        ".opencode/skills/impeccable/SKILL.md",
        "legacy installation\n",
    );
    fixture.write(
        "home/.config/opencode/skills/impeccable/SKILL.md",
        "legacy user installation\n",
    );
    let old_command = "<!-- impeccable-pinned-command -->\nold command\n";
    fixture.write(".opencode/commands/impeccable-live.md", old_command);
    fixture.write(
        "home/.config/opencode/commands/impeccable-live.md",
        old_command,
    );
    let (code, stdout, stderr) = fixture.run("", &["pin", "live"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "No harness directories with Phoenix UI installed found.\n"
    );
    assert_eq!(fixture.run("", &["unpin", "live"]).0, 0);
    for harness in [".claude", ".codex"] {
        assert_eq!(
            fixture.read(&format!("{harness}/skills/live/SKILL.md")),
            legacy
        );
    }
    for scope in [".opencode", "home/.config/opencode"] {
        assert_eq!(
            fixture.read(&format!("{scope}/commands/impeccable-live.md")),
            old_command
        );
        assert!(!fixture
            .path(&format!("{scope}/commands/phoenix-ui-live.md"))
            .exists());
    }
}

#[test]
fn opencode_global_precedence_and_unpin_work_after_the_skill_is_removed() {
    for (override_dir, xdg_dir, expected) in [
        (Some("oc override"), Some("xdg"), "oc override"),
        (None, Some("xdg"), "xdg/opencode"),
        (None, None, "home/.config/opencode"),
    ] {
        let mut fixture = Fixture::new();
        for scope in ["oc override", "xdg/opencode", "home/.config/opencode"] {
            fixture.write(
                &format!("{scope}/skills/phoenix-ui/SKILL.md"),
                "---\nname: phoenix-ui\n---\n",
            );
        }
        if let Some(dir) = override_dir {
            fixture.env.insert(
                "OPENCODE_CONFIG_DIR".into(),
                fixture.path(dir).to_string_lossy().into_owned(),
            );
        }
        if let Some(dir) = xdg_dir {
            fixture.env.insert(
                "XDG_CONFIG_HOME".into(),
                fixture.path(dir).to_string_lossy().into_owned(),
            );
        }
        let legacy_path = format!("{expected}/commands/impeccable-polish.md");
        let legacy = "<!-- impeccable-pinned-command -->\nlegacy command\n";
        fixture.write(&legacy_path, legacy);
        let (code, stdout, stderr) = fixture.run("", &["pin", "polish"]);
        assert_eq!(code, 0, "{stderr}");
        assert!(stdout.contains("in 1 location(s)"), "{stdout}");
        for scope in ["oc override", "xdg/opencode", "home/.config/opencode"] {
            assert_eq!(
                fixture
                    .path(&format!("{scope}/commands/phoenix-ui-polish.md"))
                    .is_file(),
                scope == expected
            );
        }
        let command_path = format!("{expected}/commands/phoenix-ui-polish.md");
        assert!(fixture
            .read(&command_path)
            .contains("scripts/phoenix-ui context"));
        std::fs::remove_dir_all(fixture.path(&format!("{expected}/skills/phoenix-ui"))).unwrap();
        let (code, stdout, stderr) = fixture.run("", &["unpin", "polish"]);
        assert_eq!(code, 0, "{stderr}");
        assert!(stdout.contains("from 1 location(s)"), "{stdout}");
        assert!(!fixture.path(&command_path).exists());
        assert_eq!(fixture.read(&legacy_path), legacy);
        assert!(fixture
            .run("", &["unpin", "polish"])
            .1
            .contains("No pinned 'polish' shortcut found."));
    }
}

#[test]
fn opencode_pin_and_unpin_preserve_a_foreign_current_name_command() {
    let fixture = Fixture::new();
    fixture.install(".opencode");
    let authored = "---\ndescription: My own shortcut\nagent: plan\n---\nKeep this command.\n";
    fixture.write(".opencode/commands/phoenix-ui-polish.md", authored);
    let (code, stdout, stderr) = fixture.run("", &["pin", "polish"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("non-pinned command already exists"));
    let (code, stdout, stderr) = fixture.run("", &["unpin", "polish"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("not a pinned command"));
    assert_eq!(
        fixture.read(".opencode/commands/phoenix-ui-polish.md"),
        authored
    );
}

#[test]
fn usage_and_missing_install_diagnostics_use_the_current_tool_name() {
    let fixture = Fixture::new();
    for args in [vec![], vec!["pin"]] {
        let (code, stdout, stderr) = fixture.run("", &args);
        assert_eq!(code, 1);
        assert!(stdout.starts_with("Usage: phoenix-ui pin <pin|unpin> <command>\n"));
        assert!(stdout.contains("Available commands: craft, init"));
        assert!(stderr.is_empty());
    }
    let (code, stdout, stderr) = fixture.run("", &["pin", "audit"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stdout,
        "No harness directories with Phoenix UI installed found.\n"
    );
    let (code, stdout, stderr) = fixture.run("", &["toggle", "audit"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(stderr, "Unknown action: toggle. Use 'pin' or 'unpin'.\n");
    let (code, stdout, stderr) = fixture.run("", &["pin", "doctor"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.starts_with("Unknown command: doctor\n"));
}
