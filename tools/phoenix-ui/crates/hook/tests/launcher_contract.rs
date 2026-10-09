use impeccable_common::Io;
use impeccable_detect::MissingHtmlEngine;
use impeccable_hook::{
    admin,
    hook_lib::{ensure_hook_git_excludes, has_live_preview_markers, Runtime},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "phoenix hook launchers {} {} {}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dir = std::fs::canonicalize(dir).unwrap();
        let dir = dir.to_string_lossy();
        Self(PathBuf::from(dir.strip_prefix(r"\\?\").unwrap_or(&dir)))
    }

    fn skill(&self, provider: &str, name: &str) {
        std::fs::create_dir_all(self.0.join(provider).join("skills").join(name)).unwrap();
    }

    fn write(&self, path: &str, content: &str) {
        let file = self.0.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, content).unwrap();
    }

    fn manifest(&self, path: &str) -> Value {
        serde_json::from_str(&std::fs::read_to_string(self.0.join(path)).unwrap()).unwrap()
    }

    fn runtime(&self) -> Runtime<'static> {
        static HTML: MissingHtmlEngine = MissingHtmlEngine;
        Runtime::new(
            self.0.to_string_lossy().into_owned(),
            HashMap::new(),
            "/phoenix-ui".into(),
            "phoenix-ui",
            &HTML,
        )
    }

    fn on(&self) -> String {
        let runtime = self.runtime();
        let (mut io, output) = Io::captured("", self.0.clone(), HashMap::new());
        assert_eq!(admin::run(&runtime, &["on".into()], &mut io), 0);
        let stdout = String::from_utf8(output.stdout.borrow().clone()).unwrap();
        stdout
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn new_manifests_use_phoenix_launchers_and_codex_windows_guard() {
    let fixture = Fixture::new();
    for provider in [".claude", ".agents", ".cursor", ".github", ".gemini"] {
        fixture.skill(provider, "phoenix-ui");
        // Both generations may coexist while migration is being prepared.
        fixture.skill(provider, "impeccable");
    }
    fixture.on();
    for path in [
        ".claude/settings.local.json",
        ".codex/hooks.json",
        ".cursor/hooks.json",
        ".github/hooks/phoenix-ui.json",
        ".gemini/settings.json",
    ] {
        let manifest = fixture.manifest(path);
        let text = manifest.to_string();
        assert!(text.contains("phoenix-ui"), "{path}: {text}");
        assert!(!text.contains("impeccable"), "{path}: {text}");
    }
    let codex = fixture.manifest(".codex/hooks.json");
    let expected_suffix = format!(
        " -- \"{}\" \"codex\" \".agents\\skills\\phoenix-ui\\scripts\\phoenix-ui.cmd\" hook",
        env!("CARGO_PKG_VERSION")
    );
    for event in ["PostToolUse", "Stop"] {
        let command = codex["hooks"][event][0]["hooks"][0]["commandWindows"]
            .as_str()
            .unwrap();
        assert!(command.starts_with("node -e \""), "{command}");
        assert!(command.ends_with(&expected_suffix), "{command}");
        assert!(command.contains("Reinstall the pinned @xgent-ai/skills package"));
        assert!(!command.contains("exit /b"));
    }
    let gemini = fixture.manifest(".gemini/settings.json");
    assert_eq!(
        gemini["hooks"]["BeforeTool"][0]["hooks"][0]["name"],
        "phoenix-ui-session"
    );
    assert_eq!(
        gemini["hooks"]["AfterAgent"][0]["hooks"][0]["name"],
        "phoenix-ui-completion"
    );
    assert!(fixture.on().contains("Hook manifests already installed"));
}

#[test]
fn legacy_skill_directories_do_not_count_as_phoenix_installs() {
    let fixture = Fixture::new();
    for provider in [".claude", ".agents", ".cursor", ".github", ".gemini"] {
        fixture.skill(provider, "impeccable");
    }
    assert!(fixture.on().contains("No installed provider skill folders"));
    assert!(!fixture.0.join(".codex/hooks.json").exists());
}

#[test]
fn legacy_ignore_block_is_repaired_without_changing_foreign_patterns() {
    let fixture = Fixture::new();
    fixture.write(".git/info/exclude", "user-pattern\n\n# impeccable-hook-ignore-start .\n.impeccable/hook.cache.json\n# impeccable-hook-ignore-end .\n");
    let runtime = fixture.runtime();
    let result = ensure_hook_git_excludes(&runtime, &runtime.proc_cwd);
    assert!(result.changed);
    let repaired = std::fs::read_to_string(fixture.0.join(".git/info/exclude")).unwrap();
    assert!(repaired.starts_with("user-pattern\n\n"));
    assert!(repaired.contains("# phoenix-ui-hook-ignore-start .\n"));
    assert!(repaired.contains(".phoenix-ui/hook.cache.json\n"));
    assert!(!repaired.contains("impeccable"));
    assert!(!ensure_hook_git_excludes(&runtime, &runtime.proc_cwd).changed);
}

#[test]
fn live_preview_recognizes_phoenix_and_legacy_scaffolding() {
    for content in [
        "<div data-phoenix-ui-variants=\"abcdef\"></div>",
        "/* phoenix-ui-carbonize-start abcdef */",
        "<div data-impeccable-variants=\"abcdef\"></div>",
        "/* impeccable-carbonize-start abcdef */",
    ] {
        assert!(has_live_preview_markers(content), "{content}");
    }
    assert!(!has_live_preview_markers("<div>user content</div>"));
}

#[test]
fn shared_silent_guard_is_repaired_without_losing_foreign_entries_or_permissions() {
    let fixture = Fixture::new();
    fixture.skill(".claude", "phoenix-ui");
    fixture.skill(".claude", "impeccable");
    let old = r#"[ ! -f "${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui" ] || "${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui" hook"#;
    fixture.write(
        ".claude/settings.json",
        &json!({
            "model":"kept",
            "hooks":{"PostToolUse":[
                {"hooks":[{"type":"command","command":"echo other"}]},
                {"matcher":"Edit","hooks":[{"type":"command","command":old}]}
            ]}
        })
        .to_string(),
    );
    fixture.write(
        ".claude/settings.local.json",
        &json!({
            "permissions":{"allow":["Bash(ls)"]},
            "hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":old}]}]}
        })
        .to_string(),
    );
    let output = fixture.on();
    assert!(
        output.contains("Installed or repaired hook manifests for: .claude"),
        "{output}"
    );
    let shared = fixture.manifest(".claude/settings.json");
    assert_eq!(shared["model"], "kept");
    assert_eq!(
        shared["hooks"]["PostToolUse"][0]["hooks"][0]["command"],
        "echo other"
    );
    assert_eq!(shared["hooks"]["PostToolUse"].as_array().unwrap().len(), 2);
    assert!(!shared.to_string().contains("impeccable"));
    let command = shared["hooks"]["PostToolUse"][1]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert!(command.contains("node -e "), "{command}");
    assert!(command.contains("Reinstall the pinned @xgent-ai/skills package"));
    let local = fixture.manifest(".claude/settings.local.json");
    assert_eq!(local["permissions"]["allow"], json!(["Bash(ls)"]));
    assert!(local.get("hooks").is_none());
    assert!(fixture
        .on()
        .contains("Hook manifests already installed for: .claude"));
}

#[test]
fn hooks_on_preserves_legacy_shared_hook_without_claiming_it() {
    let fixture = Fixture::new();
    fixture.skill(".claude", "phoenix-ui");
    let legacy = r#"{"model":"kept","hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"node skills/impeccable/scripts/hook.mjs"}]}]}}"#;
    fixture.write(".claude/settings.json", legacy);
    fixture.on();
    assert_eq!(
        std::fs::read_to_string(fixture.0.join(".claude/settings.json")).unwrap(),
        legacy
    );
    let local = fixture.manifest(".claude/settings.local.json");
    assert!(local
        .to_string()
        .contains("skills/phoenix-ui/scripts/phoenix-ui"));
    assert!(!local.to_string().contains("impeccable"));
}

#[cfg(unix)]
#[test]
fn posix_guard_allows_missing_launchers_and_preserves_handler_failure() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.skill(".agents", "phoenix-ui");
    fixture.skill(".agents", "impeccable");
    fixture.on();
    let manifest = fixture.manifest(".codex/hooks.json");
    let command = manifest["hooks"]["PostToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    let run = || {
        std::process::Command::new("sh")
            .args(["-c", command])
            .current_dir(&fixture.0)
            .env("PHOENIX_UI_HOME", fixture.0.join("cache"))
            .env("PHOENIX_UI_PROVIDER_ID", "codex")
            .env("CODEX_SESSION_ID", "guard-test")
            .env("CLAUDE_PROJECT_DIR", "")
            .output()
            .unwrap()
    };
    let missing = run();
    assert_eq!(
        missing.status.code(),
        Some(0),
        "missing launcher must allow the event"
    );
    assert!(String::from_utf8(missing.stdout)
        .unwrap()
        .contains("Phoenix UI unavailable:"));
    assert!(
        run().stdout.is_empty(),
        "same session must be reminded once"
    );
    fixture.write(
        ".agents/skills/phoenix-ui/scripts/phoenix-ui",
        "#!/bin/sh\nprintf '%s' \"$1\" > handler-called\nexit 7\n",
    );
    std::fs::set_permissions(
        fixture
            .0
            .join(".agents/skills/phoenix-ui/scripts/phoenix-ui"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert_eq!(
        run().status.code(),
        Some(7),
        "guard must propagate handler failure"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("handler-called")).unwrap(),
        "hook"
    );
}

#[cfg(windows)]
#[test]
fn codex_windows_node_guard_works_in_cmd_and_powershell_and_preserves_failure() {
    let fixture = Fixture::new();
    fixture.skill(".agents", "phoenix-ui");
    fixture.on();
    let manifest = fixture.manifest(".codex/hooks.json");
    let command = manifest["hooks"]["PostToolUse"][0]["hooks"][0]["commandWindows"]
        .as_str()
        .unwrap();
    for shell in ["cmd", "powershell.exe"] {
        let run = || {
            let mut process = std::process::Command::new(shell);
            if shell == "cmd" {
                process.args(["/d", "/c", command]);
            } else {
                process.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("{command}; exit $LASTEXITCODE"),
                ]);
            }
            process
                .current_dir(&fixture.0)
                .env("PHOENIX_UI_HOME", fixture.0.join("cache"))
                .env("PHOENIX_UI_PROVIDER_ID", "codex")
                .env("CODEX_SESSION_ID", shell)
                .env("CLAUDE_PROJECT_DIR", "")
                .output()
                .unwrap()
        };
        let missing = run();
        assert_eq!(missing.status.code(), Some(0), "missing launcher: {shell}");
        assert!(String::from_utf8(missing.stdout)
            .unwrap()
            .contains("Phoenix UI unavailable:"));
        assert!(run().stdout.is_empty(), "deduplication: {shell}");
        fixture.write(
            ".agents/skills/phoenix-ui/scripts/phoenix-ui.cmd",
            "@echo off\necho %1>handler-called\nexit /b 7\n",
        );
        assert_eq!(run().status.code(), Some(7), "handler failure: {shell}");
        assert_eq!(
            std::fs::read_to_string(fixture.0.join("handler-called"))
                .unwrap()
                .trim(),
            "hook"
        );
        std::fs::remove_file(
            fixture
                .0
                .join(".agents/skills/phoenix-ui/scripts/phoenix-ui.cmd"),
        )
        .unwrap();
    }
}
