//! Hook manifest rewriting (JS: tests/skills-cli.test.js "copyProviderHooks:
//! hook command path resolution (#399)", "hook manifest merge helpers"), in
//! the launcher generation.

use impeccable_common::jsp;
use impeccable_skills::hook_manifest::*;
use impeccable_skills::providers::Sys;
use serde_json::{json, Value};
use std::collections::HashMap;

fn claude_bundle_manifest() -> Value {
    json!({
        "description": "fresh claude hook",
        "hooks": {
            "PostToolUse": [{ "matcher": "Edit", "hooks": [{ "type": "command", "command": "\".claude/skills/phoenix-ui/scripts/phoenix-ui\" hook" }] }],
            "Stop": [{ "hooks": [{ "type": "command", "command": "[ ! -f \"${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui\" ] || \"${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui\" hook", "timeout": 30 }] }]
        }
    })
}

/// The launcher path a hook command points at, joined with the host's path
/// semantics the way `rewrite_hook_commands_for_platform` joins it.
fn skill_launcher(root: &str, provider: &str) -> String {
    jsp::join(&[
        root,
        provider,
        "skills",
        "phoenix-ui",
        "scripts",
        "phoenix-ui",
    ])
}

fn commands(v: &Value) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(o) => {
                for (k, c) in o {
                    if (k == "command" || k == "commandWindows") && c.is_string() {
                        out.push(format!("{k}={}", c.as_str().unwrap()));
                    } else {
                        walk(c, out);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|c| walk(c, out)),
            _ => {}
        }
    }
    walk(v, &mut out);
    out
}

fn assert_launcher_command(command: &str, launcher: &str, verb: &str) {
    assert_eq!(command, format!("{launcher} {verb}"));
}

#[test]
fn project_scope_keeps_claude_project_dir_token() {
    let out = rewrite_hook_commands_for_platform(
        &claude_bundle_manifest(),
        ".claude",
        "/proj",
        false,
        false,
    );
    let cmds = commands(&out);
    assert_eq!(cmds.len(), 2);
    for c in &cmds {
        assert_launcher_command(
            c.trim_start_matches("command="),
            r#""${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui""#,
            "hook",
        );
        assert!(!c.contains("/proj"));
    }
    // What we write is what every reader recognizes as ours.
    assert!(value_has_launcher_hook_marker(&out));
    // The bundled fixture uses the current Phoenix launcher spelling.
    assert!(value_has_launcher_hook_marker(&claude_bundle_manifest()));
}

#[test]
fn generic_rewrite_identifies_legacy_but_never_claims_it() {
    for command in [
        "node .claude/skills/impeccable/scripts/hook.mjs",
        "\".claude/skills/impeccable/scripts/impeccable\" hook",
        r#"if exist ".agents\skills\impeccable\scripts\impeccable.cmd" ".agents\skills\impeccable\scripts\impeccable.cmd" hook"#,
    ] {
        let legacy =
            json!({"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":command}]}]}});
        assert!(!value_has_launcher_hook_marker(&legacy));
        for provider in [".claude", ".agents"] {
            assert_eq!(
                rewrite_hook_commands_for_platform(&legacy, provider, "/installed", true, false),
                legacy
            );
            assert_eq!(
                rewrite_hook_commands_for_platform(&legacy, provider, "/installed", true, true),
                legacy
            );
        }
    }
}

#[test]
fn rewriting_a_complete_guard_is_idempotent_and_preserves_permissions() {
    let mut bundle = claude_bundle_manifest();
    bundle["permissions"] =
        json!({"allow":["Bash(.claude/skills/phoenix-ui/scripts/phoenix-ui hook)"]});
    let rewritten =
        rewrite_hook_commands_for_platform(&bundle, ".claude", "/installed", false, false);
    assert_eq!(rewritten["permissions"], bundle["permissions"]);
    assert_eq!(
        rewrite_hook_commands_for_platform(&rewritten, ".claude", "/installed", false, false),
        rewritten
    );
}

#[test]
fn absolute_path_is_single_quoted_and_inert_under_sh() {
    let root = "/tmp/imp-hook-$(touch pwned)-x";
    let out =
        rewrite_hook_commands_for_platform(&claude_bundle_manifest(), ".claude", root, true, false);
    for c in commands(&out) {
        // The launcher path is joined with the host's path semantics, so the
        // expectation is joined the same way (backslashes on Windows).
        let expected_path = skill_launcher(root, ".claude");
        assert_launcher_command(
            c.trim_start_matches("command="),
            &sh_single_quote(&expected_path),
            "hook",
        );
        assert!(!c.contains("${CLAUDE_PROJECT_DIR}"));
        assert!(!c.contains(&format!("\"{root}")));
    }
    // A single quote inside the path gets the '\'' escape.
    assert_eq!(sh_single_quote("/it's/here"), "'/it'\\''s/here'");
    #[cfg(unix)]
    {
        let dir = std::env::temp_dir().join(format!("imp-guard-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for c in commands(&out) {
            let cmd = c.trim_start_matches("command=");
            let status = std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(&dir)
                .output()
                .unwrap();
            // No launcher at that path: sh reports it missing, nothing else runs.
            assert_eq!(status.status.code(), Some(127));
        }
        assert!(!dir.join("pwned").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(unix)]
#[test]
fn rewritten_absolute_command_preserves_stdin_and_handler_failure_with_literal_shell_characters() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};
    let dir = tmp_dir("absolute-guard space");
    let root = dir.join("literal o'k \"double\" $(touch pwned)");
    let out = rewrite_hook_commands_for_platform(
        &claude_bundle_manifest(),
        ".claude",
        &root.to_string_lossy(),
        true,
        false,
    );
    let command = out["hooks"]["PostToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    let run = |input: &[u8]| {
        let mut child = Command::new("/bin/sh")
            .args(["-c", command])
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    };
    let launcher = skill_launcher(&root.to_string_lossy(), ".claude");
    write(
        &launcher,
        "#!/bin/sh\ncat > handler-stdin\nprintf '%s' \"$1\" > handler-args\nexit 7\n",
    );
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    let input = b"invalid event\nquotes ' \" and a NUL\0\n";
    let handled = run(input);
    assert_eq!(handled.status.code(), Some(7));
    assert!(handled.stdout.is_empty());
    assert_eq!(
        std::fs::read(dir.join("handler-stdin")).unwrap().as_slice(),
        &input[..]
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("handler-args")).unwrap(),
        "hook"
    );
    assert!(!dir.join("pwned").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn windows_form_keeps_double_quoted_absolute_path() {
    let root = "/home/u";
    let out =
        rewrite_hook_commands_for_platform(&claude_bundle_manifest(), ".claude", root, true, true);
    for c in commands(&out) {
        let p = skill_launcher(root, ".claude");
        // The Windows form is the JSON-quoted path, so a host path's
        // backslashes arrive escaped inside the command string.
        let q = serde_json::to_string(&p).unwrap();
        assert_launcher_command(c.trim_start_matches("command="), &q, "hook");
        assert!(!c.contains(&format!("'{p}")));
    }
}

#[test]
fn codex_gets_command_windows_sibling_pointing_at_cmd_shim() {
    let bundle = json!({
        "hooks": { "PostToolUse": [{ "matcher": "apply_patch", "hooks": [{ "type": "command", "command": "\".codex/skills/phoenix-ui/scripts/phoenix-ui\" hook", "timeout": 5 }] }] }
    });
    let out = rewrite_hook_commands_for_platform(&bundle, ".agents", "/proj", false, false);
    let entry = &out["hooks"]["PostToolUse"][0]["hooks"][0];
    assert_launcher_command(
        entry["command"].as_str().unwrap(),
        r#"".agents/skills/phoenix-ui/scripts/phoenix-ui""#,
        "hook",
    );
    assert_eq!(
        entry["commandWindows"],
        r#"cmd /d /c ".agents\skills\phoenix-ui\scripts\phoenix-ui.cmd" hook"#
    );
    // JS key order: existing keys, then the appended commandWindows.
    let keys: Vec<&String> = entry.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["type", "command", "timeout", "commandWindows"]);
    // Windows host: Codex keeps the POSIX command; the sibling handles cmd.exe.
    let win = rewrite_hook_commands_for_platform(&bundle, ".agents", "/proj", false, true);
    assert_eq!(
        win["hooks"]["PostToolUse"][0]["hooks"][0]["command"],
        entry["command"]
    );
    assert!(value_has_launcher_hook_marker(&out));
}

#[test]
fn cursor_runs_hook_before_edit() {
    let bundle = json!({ "version": 1, "hooks": { "preToolUse": [{ "command": "\".cursor/skills/phoenix-ui/scripts/phoenix-ui\" hook-before-edit", "timeout": 5 }] } });
    let out = rewrite_hook_commands_for_platform(&bundle, ".cursor", "/proj", false, false);
    assert_launcher_command(
        out["hooks"]["preToolUse"][0]["command"].as_str().unwrap(),
        r#"".cursor/skills/phoenix-ui/scripts/phoenix-ui""#,
        "hook-before-edit",
    );
    assert!(out["hooks"]["preToolUse"][0]
        .get("commandWindows")
        .is_none());
}

#[test]
fn github_manifests_pass_through_and_grok_is_rewritten() {
    // .github stays untouched: its committed manifest carries a portable
    // `$(git rev-parse ...)` command that must never become machine-local.
    let bundle = json!({ "version": 1, "hooks": { "postToolUse": [{ "type": "command", "bash": "[ ! -f \"$(git rev-parse --show-toplevel)/.github/skills/phoenix-ui/scripts/phoenix-ui\" ] || \"$(git rev-parse --show-toplevel)/.github/skills/phoenix-ui/scripts/phoenix-ui\" hook" }] } });
    assert_eq!(
        rewrite_hook_commands_for_platform(&bundle, ".github", "/proj", true, false),
        bundle
    );
    assert!(value_has_launcher_hook_marker(&bundle));
    // .grok is rewritten like the other command-hook providers (upstream
    // 49571365, #642): a global install must not leave the bundled
    // project-relative path behind.
    let grok = json!({ "hooks": { "PostToolUse": [{ "matcher": "Edit|Write|MultiEdit", "hooks": [{ "type": "command", "command": "\".grok/skills/phoenix-ui/scripts/phoenix-ui\" hook" }] }] } });
    let rel = rewrite_hook_commands_for_platform(&grok, ".grok", "/proj", false, false);
    assert_launcher_command(
        rel["hooks"]["PostToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap(),
        r#"".grok/skills/phoenix-ui/scripts/phoenix-ui""#,
        "hook",
    );
    let abs = rewrite_hook_commands_for_platform(&grok, ".grok", "/home/u", true, false);
    let p = skill_launcher("/home/u", ".grok");
    assert_launcher_command(
        abs["hooks"]["PostToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap(),
        &sh_single_quote(&p),
        "hook",
    );
    // Grok is not Codex: no commandWindows sibling is added.
    assert!(rel["hooks"]["PostToolUse"][0]["hooks"][0]
        .get("commandWindows")
        .is_none());
}

#[test]
fn merge_refreshes_ours_and_preserves_third_party_hooks() {
    let legacy = json!({ "matcher": "LegacyEdit", "hooks": [{ "type": "command", "command": "node .claude/skills/impeccable/scripts/hook.mjs" }] });
    let existing = json!({
        "permissions": { "allow": ["x", "Bash(.claude/skills/phoenix-ui/scripts/phoenix-ui hook)"] },
        "description": "old",
        "hooks": {
            "PostToolUse": [
                { "matcher": "Bash", "hooks": [{ "type": "command", "command": "echo hi" }] },
                legacy.clone(),
                { "matcher": "Edit", "hooks": [{ "type": "command", "command": "\"${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui\" hook" }] }
            ],
            "Stop": [{ "hooks": [{ "type": "command", "command": "\".claude/skills/phoenix-ui/scripts/phoenix-ui\" hook" }] }]
        }
    });
    let fresh = rewrite_hook_commands_for_platform(
        &claude_bundle_manifest(),
        ".claude",
        "/proj",
        false,
        false,
    );
    let merged = merge_hook_manifests(&existing, &fresh);
    let keys: Vec<&String> = merged.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["permissions", "description", "hooks"]);
    assert_eq!(merged["permissions"], existing["permissions"]);
    assert_eq!(merged["description"], "old");
    let post = merged["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 3);
    assert_eq!(post[0]["matcher"], "Bash");
    assert_eq!(post[1], legacy);
    assert_launcher_command(
        post[2]["hooks"][0]["command"].as_str().unwrap(),
        r#""${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui""#,
        "hook",
    );
    let stop = merged["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 1);
    assert_eq!(
        stop[0]["hooks"][0]["command"],
        post[2]["hooks"][0]["command"]
    );
    assert_eq!(merge_hook_manifests(&merged, &fresh), merged);
}

#[test]
fn prune_removes_only_ours_and_drops_empty_scaffolding() {
    let dir = std::env::temp_dir().join(format!("imp-prune-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.local.json");
    let p = path.to_string_lossy().to_string();
    std::fs::write(&path, serde_json::to_string_pretty(&json!({
        "other": 1,
        "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "\".claude/skills/phoenix-ui/scripts/phoenix-ui\" hook" }] }] }
    })).unwrap()).unwrap();
    assert!(file_has_launcher_hook_marker(&p));
    assert!(prune_impeccable_hook_from_manifest(&p).unwrap());
    let after: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(after, json!({ "other": 1 }));
    assert!(!prune_impeccable_hook_from_manifest(&p).unwrap());
    // A permissions entry naming the hook path is not a hook.
    std::fs::write(
        &path,
        r#"{"permissions":{"allow":["Bash(.claude/skills/phoenix-ui/scripts/phoenix-ui hook)"]}}"#,
    )
    .unwrap();
    assert!(!file_has_launcher_hook_marker(&p));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prune_preserves_legacy_and_third_party_hooks_and_never_rewrites_legacy_only_files() {
    let dir = tmp_dir("prune-legacy");
    let path = dir.join("settings.json");
    let name = path.to_string_lossy().into_owned();
    let legacy = json!({"hooks":[{"type":"command","command":"node .claude/skills/impeccable/scripts/hook.mjs"}]});
    let foreign = json!({"hooks":[{"type":"command","command":"echo keep-me"}]});
    let permissions = json!({"allow":["Bash(ls)"]});
    write(&name, &json!({"permissions":permissions.clone(), "hooks":{"Stop":[legacy.clone(), foreign.clone()]}}).to_string());
    let original = read(&name);
    assert!(!prune_impeccable_hook_from_manifest(&name).unwrap());
    assert_eq!(read(&name), original);
    let phoenix = json!({"hooks":[{"type":"command","command":"\".claude/skills/phoenix-ui/scripts/phoenix-ui\" hook"}]});
    write(&name, &json!({"permissions":permissions.clone(), "hooks":{"Stop":[legacy.clone(), foreign.clone(), phoenix]}}).to_string());
    assert!(prune_impeccable_hook_from_manifest(&name).unwrap());
    let after: Value = serde_json::from_str(&read(&name)).unwrap();
    assert_eq!(after["permissions"], permissions);
    assert_eq!(after["hooks"]["Stop"], json!([legacy, foreign]));
    let before_second = read(&name);
    assert!(!prune_impeccable_hook_from_manifest(&name).unwrap());
    assert_eq!(read(&name), before_second);
    let _ = std::fs::remove_dir_all(dir);
}

// ─── Copy into existing manifests ──────────────────────────────────────────

fn write(path: &str, content: &str) {
    let p = std::path::Path::new(path);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("imp-{name}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sys_with_home(home: &str) -> Sys {
    let mut env = HashMap::new();
    env.insert("HOME".to_string(), home.to_string());
    Sys::new(env, home.to_string())
}

#[test]
fn copy_refreshes_current_shared_guard_once_without_losing_user_or_legacy_entries() {
    let dir = tmp_dir("copy-shared");
    let root = dir.to_string_lossy().into_owned();
    let bundle = jsp::join(&[&root, "bundle"]);
    let project = jsp::join(&[&root, "project"]);
    write(
        &jsp::join(&[&bundle, ".claude", "settings.json"]),
        &claude_bundle_manifest().to_string(),
    );
    let shared = jsp::join(&[&project, ".claude", "settings.json"]);
    let local = jsp::join(&[&project, ".claude", "settings.local.json"]);
    let legacy = json!({"hooks":[{"type":"command","command":"node .claude/skills/impeccable/scripts/hook.mjs"}]});
    let foreign = json!({"matcher":"Bash", "hooks":[{"type":"command","command":"echo keep-me"}]});
    let phoenix = json!({"matcher":"Edit", "hooks":[{"type":"command","command":"\"${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui\" hook"}]});
    write(
        &shared,
        &json!({
            "model":"kept", "permissions":{"allow":["Bash(ls)"]},
            "hooks":{"PostToolUse":[foreign.clone(),legacy.clone(),phoenix.clone()]}
        })
        .to_string(),
    );
    write(&local, &json!({"permissions":{"allow":["Bash(cat)"]},"hooks":{"PostToolUse":[legacy.clone(),phoenix]}}).to_string());
    let sys = sys_with_home("/nonexistent-home");
    assert_eq!(
        copy_provider_hooks(&sys, &bundle, &project, &[".claude"], false, None).unwrap(),
        [".claude"]
    );
    let after: Value = serde_json::from_str(&read(&shared)).unwrap();
    assert_eq!(after["model"], "kept");
    assert_eq!(after["permissions"], json!({"allow":["Bash(ls)"]}));
    let post = after["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 3);
    assert_eq!(post[0], foreign);
    assert_eq!(post[1], legacy);
    assert_launcher_command(
        post[2]["hooks"][0]["command"].as_str().unwrap(),
        r#""${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui""#,
        "hook",
    );
    let local_after: Value = serde_json::from_str(&read(&local)).unwrap();
    assert_eq!(local_after["permissions"], json!({"allow":["Bash(cat)"]}));
    assert_eq!(local_after["hooks"]["PostToolUse"], json!([legacy]));
    let snapshot = read(&shared);
    assert!(
        copy_provider_hooks(&sys, &bundle, &project, &[".claude"], false, None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(read(&shared), snapshot);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn copy_installs_current_phoenix_beside_legacy_without_mutating_legacy_shared_settings() {
    let dir = tmp_dir("copy-legacy");
    let root = dir.to_string_lossy().into_owned();
    let bundle = jsp::join(&[&root, "bundle"]);
    let project = jsp::join(&[&root, "project"]);
    write(
        &jsp::join(&[&bundle, ".claude", "settings.json"]),
        &claude_bundle_manifest().to_string(),
    );
    let shared = jsp::join(&[&project, ".claude", "settings.json"]);
    let legacy = "{\n  \"model\":\"kept\",\n  \"hooks\":{\"PostToolUse\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"node .claude/skills/impeccable/scripts/hook.mjs\"}]}]}\n}\n";
    write(&shared, legacy);
    assert!(!hook_installed_for_provider(&project, ".claude"));
    let sys = sys_with_home("/nonexistent-home");
    assert_eq!(
        copy_provider_hooks(&sys, &bundle, &project, &[".claude"], false, None).unwrap(),
        [".claude"]
    );
    assert_eq!(read(&shared), legacy);
    assert!(hook_installed_for_provider(&project, ".claude"));
    let local: Value = serde_json::from_str(&read(&jsp::join(&[
        &project,
        ".claude",
        "settings.local.json",
    ])))
    .unwrap();
    assert_launcher_command(
        local["hooks"]["PostToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap(),
        r#""${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui""#,
        "hook",
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hook_artifacts_map_providers_to_manifest_files() {
    let dests = expected_hook_dests(
        "/p",
        &[
            ".claude", ".agents", ".cursor", ".github", ".grok", ".gemini",
        ],
    );
    // Manifest destinations are joined with the host's path semantics, so the
    // expectations are joined the same way (backslashes on Windows).
    assert_eq!(
        dests,
        [
            jsp::join(&["/p", ".claude", "settings.local.json"]),
            jsp::join(&["/p", ".codex", "hooks.json"]),
            jsp::join(&["/p", ".cursor", "hooks.json"]),
            jsp::join(&["/p", ".github", "hooks", "phoenix-ui.json"]),
            jsp::join(&["/p", ".grok", "hooks", "phoenix-ui.json"]),
            jsp::join(&["/p", ".gemini", "settings.json"]),
        ]
    );
    let a = hook_artifacts_for_provider("/b", "/p", ".claude");
    assert_eq!(a[0].src, jsp::join(&["/b", ".claude", "settings.json"]));
    assert_eq!(
        a[0].dest,
        jsp::join(&["/p", ".claude", "settings.local.json"])
    );
    assert_eq!(
        a[0].shared_dest.as_deref(),
        Some(jsp::join(&["/p", ".claude", "settings.json"]).as_str())
    );
    let c = hook_artifacts_for_provider("/b", "/p", ".agents");
    assert_eq!(c[0].src, jsp::join(&["/b", ".codex", "hooks.json"]));
    assert_eq!(c[0].dest, jsp::join(&["/p", ".codex", "hooks.json"]));
    assert!(c[0].shared_dest.is_none());
}

fn windows_user_scope_hook_command() -> String {
    let launcher = r"C:\Users\alice\.claude\skills\phoenix-ui\scripts\phoenix-ui";
    let q = json_string(launcher);
    format!("[ ! -f {q} ] || {q} hook")
}

#[test]
fn merge_json_escaped_windows_launcher_is_idempotent() {
    let cmd = windows_user_scope_hook_command();
    let hook_entry = |cmd: String| json!({ "matcher": "Edit", "hooks": [{ "type": "command", "command": cmd }] });
    let stop_entry =
        |cmd: String| json!({ "hooks": [{ "type": "command", "command": cmd, "timeout": 30 }] });
    let existing = json!({
        "hooks": {
            "PostToolUse": [hook_entry(cmd.clone())],
            "Stop": [stop_entry(cmd.clone())]
        }
    });
    let fresh = json!({
        "description": "fresh",
        "hooks": {
            "PostToolUse": [hook_entry(cmd.clone())],
            "Stop": [stop_entry(cmd.clone())]
        }
    });
    let merged = merge_hook_manifests(&existing, &fresh);
    assert_eq!(merged["hooks"]["PostToolUse"].as_array().unwrap().len(), 1);
    assert_eq!(merged["hooks"]["Stop"].as_array().unwrap().len(), 1);
    let merged2 = merge_hook_manifests(&merged, &fresh);
    assert_eq!(merged2["hooks"]["PostToolUse"].as_array().unwrap().len(), 1);
    assert_eq!(merged2["hooks"]["Stop"].as_array().unwrap().len(), 1);
}

#[test]
fn merge_heals_triplicated_stop_groups() {
    let cmd = windows_user_scope_hook_command();
    let stop_entry =
        json!({ "hooks": [{ "type": "command", "command": cmd.clone(), "timeout": 30 }] });
    let existing = json!({
        "hooks": {
            "Stop": [stop_entry.clone(), stop_entry.clone(), stop_entry]
        }
    });
    let fresh = json!({
        "hooks": {
            "Stop": [json!({ "hooks": [{ "type": "command", "command": cmd, "timeout": 30 }] })]
        }
    });
    let merged = merge_hook_manifests(&existing, &fresh);
    assert_eq!(merged["hooks"]["Stop"].as_array().unwrap().len(), 1);
}

#[test]
fn gemini_manifest_installs_and_rewrites_both_events() {
    let artifacts = hook_artifacts_for_provider("/bundle", "/project", ".gemini");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(
        artifacts[0].src,
        jsp::join(&["/bundle", ".gemini", "settings.json"])
    );
    assert_eq!(
        artifacts[0].dest,
        jsp::join(&["/project", ".gemini", "settings.json"])
    );
    let manifest = json!({"hooks": {
        "BeforeTool": [{"matcher": "^run_shell_command$", "hooks": [{"type": "command", "command": "[ ! -f \"$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui\" ] || \"$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui\" hook", "timeout": 5000}]}],
        "AfterAgent": [{"hooks": [{"type": "command", "command": "\"$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui\" hook", "timeout": 30000}]}]
    }});
    for absolute in [false, true] {
        let rewritten =
            rewrite_hook_commands_for_platform(&manifest, ".gemini", "/installed", absolute, false);
        // Gemini substitutes `$GEMINI_PROJECT_DIR` textually with a
        // shell-escaped path before the shell runs, so the token stays bare.
        let expected = if absolute {
            format!(
                "'{}'",
                jsp::join(&[
                    "/installed",
                    ".gemini",
                    "skills",
                    "phoenix-ui",
                    "scripts",
                    "phoenix-ui"
                ])
            )
        } else {
            "$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui".to_string()
        };
        for command in commands(&rewritten) {
            assert_launcher_command(command.trim_start_matches("command="), &expected, "hook");
        }
        assert_eq!(
            rewritten["hooks"]["AfterAgent"][0]["hooks"][0]["timeout"],
            30000
        );
        assert!(rewritten["hooks"].get("AfterTool").is_none());
    }
}

#[test]
fn gemini_windows_command_is_powershell() {
    // Gemini runs hooks through PowerShell on Windows and has no per-OS
    // command field, so a Windows install writes a PowerShell call of the
    // `.cmd` shim, still recognized as ours.
    let manifest = json!({"hooks": {"AfterAgent": [{"hooks": [{"type": "command",
        "command": "\"$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui\" hook"}]}]}});
    let rel = rewrite_hook_commands_for_platform(&manifest, ".gemini", "C:/p", false, true);
    let cmd = rel["hooks"]["AfterAgent"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        cmd,
        "& \"$env:GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui.cmd\" hook; exit $LASTEXITCODE"
    );
    assert!(value_has_launcher_hook_marker(&Value::String(cmd)));
    let abs = rewrite_hook_commands_for_platform(&manifest, ".gemini", "C:/Users/o'k", true, true);
    let cmd = abs["hooks"]["AfterAgent"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    let shim = jsp::join(&[
        "C:/Users/o''k",
        ".gemini",
        "skills",
        "phoenix-ui",
        "scripts",
        "phoenix-ui.cmd",
    ]);
    assert_eq!(cmd, format!("& '{shim}' hook; exit $LASTEXITCODE"));
    assert!(value_has_launcher_hook_marker(&Value::String(cmd)));
}

#[test]
fn gemini_install_merges_into_commented_settings_and_force_never_wipes() {
    let dir = tmp_dir("gemini-jsonc");
    let root = dir.to_string_lossy().into_owned();
    let bundle = jsp::join(&[&root, "bundle"]);
    let project = jsp::join(&[&root, "project"]);
    write(&jsp::join(&[&bundle, ".gemini", "settings.json"]), &json!({"hooks": {"AfterAgent": [{"hooks": [{"type": "command",
        "command": "[ ! -f \"$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui\" ] || \"$GEMINI_PROJECT_DIR/.gemini/skills/phoenix-ui/scripts/phoenix-ui\" hook"}]}]}}).to_string());
    let settings = jsp::join(&[&project, ".gemini", "settings.json"]);
    let original = "{\n  // pick a model\n  \"model\": {\"name\": \"gemini-3-pro\"},\n  /* MCP */ \"mcpServers\": {\"x\": {\"url\": \"http://a//b\"}}\n}\n";
    write(&settings, original);
    let sys = sys_with_home("/nonexistent-home");
    copy_provider_hooks(&sys, &bundle, &project, &[".gemini"], false, None).unwrap();
    let merged: Value = serde_json::from_str(&read(&settings)).unwrap();
    assert_eq!(merged["model"]["name"], "gemini-3-pro");
    assert_eq!(merged["mcpServers"]["x"]["url"], "http://a//b");
    assert!(merged["hooks"]["AfterAgent"].is_array());
    // Truly malformed: refused without --force, and --force never replaces
    // the user's whole settings file with a hooks-only manifest.
    write(&settings, "{ \"model\": ");
    assert!(copy_provider_hooks(&sys, &bundle, &project, &[".gemini"], false, None).is_err());
    assert!(copy_provider_hooks(&sys, &bundle, &project, &[".gemini"], true, None).is_err());
    assert_eq!(read(&settings), "{ \"model\": ");
    let _ = std::fs::remove_dir_all(&dir);
}
