//! Phoenix hook recognition: which manifest commands run the Phoenix
//! launcher's hooks.

use once_cell::sync::Lazy;
use regex::Regex;

static PHOENIX_HOOK_MARKER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"skills/phoenix-ui/scripts/phoenix-ui(?:\.cmd|\.exe)?["']?\s+hook(?:-before-edit|-probe|-after-edit|-stop)?(?:\s|$|["'&|;)])"#).unwrap()
});

static PHOENIX_DESIGN_HOOK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"skills/phoenix-ui/scripts/phoenix-ui(?:\.cmd|\.exe)?["']?\s+hook(?:-before-edit)?(?:\s|$|["'&|;)])"#).unwrap()
});

/// User-scope Windows commands embed a JSON-quoted path whose backslashes are
/// doubled in the command string (#784). #604's single `\`→`/` replace is not
/// enough: `\\` becomes `//`, which breaks `skills/phoenix-ui` matching.
/// A leading `//` after a quote (or at the start of the string) is a UNC
/// prefix and stays two slashes, so doctor still probes `//server/share/...`.
fn normalize_hook_separators(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut chars = command.chars().peekable();
    let mut prev: Option<char> = None;
    while let Some(ch) = chars.next() {
        if ch != '\\' && ch != '/' {
            out.push(ch);
            prev = Some(ch);
            continue;
        }
        let mut n = 1usize;
        while matches!(chars.peek(), Some('\\' | '/')) {
            chars.next();
            n += 1;
        }
        out.push('/');
        if n >= 2 && matches!(prev, None | Some('"' | '\'')) {
            out.push('/');
        }
        prev = Some('/');
    }
    out
}

/// True when `command` invokes the Phoenix launcher with any hook verb.
pub fn is_launcher_hook_command(command: &str) -> bool {
    PHOENIX_HOOK_MARKER.is_match(&normalize_hook_separators(command))
}

/// True when `command` runs the Phoenix design hook itself (`hook` or
/// Cursor's `hook-before-edit`); `hook-probe` and the other admin verbs do
/// not count as an installed hook.
pub fn is_launcher_design_hook_command(command: &str) -> bool {
    PHOENIX_DESIGN_HOOK.is_match(&normalize_hook_separators(command))
}

/// The launcher path inside a design-hook `command`, for existence checks.
/// `None` when the command runs no design hook or the token cannot be
/// isolated (a `'\''` escape sequence, for instance).
pub fn hook_program_token(command: &str) -> Option<String> {
    if command.contains("'\\''") {
        return None;
    }
    let command = normalize_hook_separators(command);
    if !is_launcher_design_hook_command(&command) {
        return None;
    }
    static QUOTED: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#""([^"]*skills/phoenix-ui/scripts/phoenix-ui(?:\.cmd|\.exe)?)""#).unwrap()
    });
    static SINGLE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"'([^']*skills/phoenix-ui/scripts/phoenix-ui(?:\.cmd|\.exe)?)'").unwrap()
    });
    static BARE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"([^\s"'|&;()]*skills/phoenix-ui/scripts/phoenix-ui(?:\.cmd|\.exe)?)"#)
            .unwrap()
    });
    if let Some(m) = QUOTED.captures(&command) {
        return Some(m[1].to_string());
    }
    if let Some(m) = SINGLE.captures(&command) {
        return Some(m[1].to_string());
    }
    BARE.captures(&command).map(|m| m[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_launcher_forms() {
        for cmd in [
            "\"${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui\" hook",
            "\".agents/skills/phoenix-ui/scripts/phoenix-ui.cmd\" hook",
            "'/x/.cursor/skills/phoenix-ui/scripts/phoenix-ui' hook-before-edit",
            "\"$(git rev-parse --show-toplevel)/.github/skills/phoenix-ui/scripts/phoenix-ui\" hook",
            "if exist \".agents/skills/phoenix-ui/scripts/phoenix-ui.cmd\" (\".agents/skills/phoenix-ui/scripts/phoenix-ui.cmd\" hook & exit /b)",
        ] {
            assert!(is_launcher_hook_command(cmd), "{cmd}");
            assert!(is_launcher_design_hook_command(cmd), "{cmd}");
        }
        let probe = "\".claude/skills/phoenix-ui/scripts/phoenix-ui\" hook-probe";
        assert!(is_launcher_hook_command(probe));
        assert!(!is_launcher_design_hook_command(probe));
    }

    #[test]
    fn rejects_unrelated_commands() {
        for cmd in [
            "node \".claude/skills/impeccable/scripts/hook.mjs\"",
            "\".claude/skills/impeccable/scripts/impeccable\" hook",
            "\".claude/skills/phoenix-ui/scripts/phoenix-ui\" context",
            "phoenix-ui hook",
            "echo skills/phoenix-ui/scripts/phoenix-ui-hook",
        ] {
            assert!(!is_launcher_hook_command(cmd), "{cmd}");
        }
    }

    #[test]
    fn extracts_program_token() {
        assert_eq!(
            hook_program_token(
                "\"${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui\" hook"
            )
            .as_deref(),
            Some("${CLAUDE_PROJECT_DIR}/.claude/skills/phoenix-ui/scripts/phoenix-ui")
        );
        assert_eq!(
            hook_program_token(
                "'/x/.cursor/skills/phoenix-ui/scripts/phoenix-ui' hook-before-edit"
            )
            .as_deref(),
            Some("/x/.cursor/skills/phoenix-ui/scripts/phoenix-ui")
        );
        assert_eq!(
            hook_program_token(".agents/skills/phoenix-ui/scripts/phoenix-ui hook").as_deref(),
            Some(".agents/skills/phoenix-ui/scripts/phoenix-ui")
        );
        assert_eq!(
            hook_program_token("'/x/it'\\''s/.claude/skills/phoenix-ui/scripts/phoenix-ui' hook"),
            None
        );
        assert_eq!(hook_program_token("echo hi"), None);
    }

    #[test]
    fn recognizes_windows_launcher_paths() {
        let launcher = r"C:\Users\alice\.claude\skills\phoenix-ui\scripts\phoenix-ui";
        let quoted = serde_json::to_string(launcher).unwrap();
        let cmd = format!("[ ! -f {quoted} ] || {quoted} hook");
        assert!(is_launcher_design_hook_command(&cmd), "{cmd}");
        assert_eq!(
            hook_program_token(&cmd).as_deref(),
            Some("C:/Users/alice/.claude/skills/phoenix-ui/scripts/phoenix-ui")
        );
        let single = format!("\"{launcher}\" hook");
        assert!(is_launcher_design_hook_command(&single), "{single}");
    }

    #[test]
    fn preserves_unc_prefix_in_program_token() {
        let launcher = r"\\server\share\.claude\skills\phoenix-ui\scripts\phoenix-ui";
        let quoted = serde_json::to_string(launcher).unwrap();
        let json_escaped = format!("[ ! -f {quoted} ] || {quoted} hook");
        assert_eq!(
            hook_program_token(&json_escaped).as_deref(),
            Some("//server/share/.claude/skills/phoenix-ui/scripts/phoenix-ui")
        );
        let single = format!("\"{launcher}\" hook");
        assert_eq!(
            hook_program_token(&single).as_deref(),
            Some("//server/share/.claude/skills/phoenix-ui/scripts/phoenix-ui")
        );
    }
}

/// Removes `//` and `/* */` comments outside JSON strings, the dialect
/// Gemini CLI reads its `settings.json` in (`strip-json-comments`). Returns
/// the stripped text and whether any comment was removed.
pub fn strip_json_comments(text: &str) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let (mut in_string, mut escaped, mut stripped) = (false, false, false);
    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped { escaped = false } else if ch == '\\' { escaped = true } else if ch == '"' { in_string = false }
            continue;
        }
        match (ch, chars.peek()) {
            ('"', _) => { in_string = true; out.push(ch); }
            ('/', Some('/')) => {
                stripped = true;
                while let Some(&c) = chars.peek() { if c == '\n' { break } chars.next(); }
            }
            ('/', Some('*')) => {
                stripped = true;
                chars.next();
                let mut prev = '\0';
                for c in chars.by_ref() { if prev == '*' && c == '/' { break } prev = c; }
                out.push(' ');
            }
            _ => out.push(ch),
        }
    }
    (out, stripped)
}

/// Parses a hook manifest that may carry comments. `None` when it is not JSON
/// even after the comments are removed.
pub fn parse_manifest_jsonc(text: &str) -> Option<(serde_json::Value, bool)> {
    let (stripped, had_comments) = strip_json_comments(text);
    serde_json::from_str(&stripped).ok().map(|v| (v, had_comments))
}

#[cfg(test)]
mod jsonc_tests {
    use super::*;

    #[test]
    fn strips_comments_outside_strings_only() {
        let (v, had) = parse_manifest_jsonc("{\n // a\n \"u\": \"http://x//y\", /* b */ \"s\": \"/* no */\\\"//\"\n}").unwrap();
        assert!(had);
        assert_eq!(v["u"], "http://x//y");
        assert_eq!(v["s"], "/* no */\"//");
        assert_eq!(parse_manifest_jsonc("{\"a\": 1}").unwrap().1, false);
        assert!(parse_manifest_jsonc("{ \"a\": ").is_none());
        assert_eq!(crate::context_cli::hook_manifests_for("gemini"), &[".gemini/settings.json"]);
    }
}
