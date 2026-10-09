//! Applies staged live copy edits through the current host session.
//! The explicit mock provider is retained for deterministic tests.

use crate::event_validation::truthy;
use crate::manual_edits::evidence::{arr, is_path_inside_or_equal};
use crate::util::{exists, jsp, Env};
use impeccable_common::proc;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::process::{Command, Stdio};

/// JS: parseCopyEditBatchResult(text)
pub fn parse_copy_edit_batch_result(text: &str) -> Option<Value> {
    let parsed = parse_copy_edit_agent_result(text)?;
    match parsed.get("status").and_then(|s| s.as_str()) {
        Some("done") | Some("partial") | Some("error") => Some(normalize_batch_result(&parsed)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Batch runner
// ---------------------------------------------------------------------------

/// JS: runCopyEditBatchAgent(batch, opts)
#[allow(clippy::too_many_arguments)]
pub fn run_copy_edit_batch_agent(
    batch: &Value,
    cwd: &str,
    env: &Env,
    provider: Option<&str>,
    _timeout_ms: Option<f64>,
    apply_batch_to_source: Option<&mut dyn FnMut(&Value, Option<&Value>) -> Result<Value, String>>,
    chat_available: Option<&dyn Fn() -> bool>,
) -> Result<Value, String> {
    let requested = provider
        .filter(|p| !p.is_empty())
        .or_else(|| env.get("PHOENIX_UI_LIVE_COPY_AGENT").map(String::as_str))
        .unwrap_or("auto");
    let mode = impeccable_context::util::js_trim(requested).to_ascii_lowercase();
    let host_available = || chat_available.map(|f| f()).unwrap_or(false);
    match mode.as_str() {
        "mock" => {
            let delay_ms = env_number(env, "PHOENIX_UI_LIVE_COPY_AGENT_MOCK_DELAY_MS");
            if delay_ms > 0.0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms as u64));
            }
            mock_batch_result(batch, env, cwd)
        }
        "chat" | "host" | "auto" | "" => {
            if matches!(mode.as_str(), "auto" | "") && !host_available() {
                return Err(describe_no_provider_error(env, false));
            }
            let Some(cb) = apply_batch_to_source else {
                return Err(describe_no_provider_error(env, host_available()));
            };
            let repair = batch.get("repair").filter(|v| truthy(Some(v)));
            let raw = cb(batch, repair)?;
            let raw = if truthy(Some(&raw)) { raw } else { json!({}) };
            Ok(normalize_batch_result(&raw))
        }
        "0" | "false" | "off" | "none" => Err(
            "Phoenix UI live copy editing is disabled. Edit the source manually, or enable the current host session.".into(),
        ),
        _ => Err(format!(
            "External live copy-edit runners are disabled (requested {}). Use the current Phoenix UI host session or edit the source manually.",
            serde_json::to_string(&mode).unwrap_or_default()
        )),
    }
}

fn env_number(env: &Env, key: &str) -> f64 {
    match env.get(key) {
        Some(v) if !v.is_empty() => {
            let n = impeccable_core::js::string_to_number(v);
            if n.is_nan() {
                f64::NAN
            } else {
                n
            }
        }
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Post-apply checks
// ---------------------------------------------------------------------------

static MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)^\s*(?:<!--|\{/\*)\s*(?:phoenix-ui|impeccable)-carbonize-(?:start|end)\b|^\s*(?:<!--|\{/\*)\s*(?:phoenix-ui|impeccable)-variants-(?:start|end)\b",
    )
    .unwrap()
});
static ATTR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\bdata-(?:phoenix-ui|impeccable)-(?:variants?|original-text|editable|text-wrap)\s*=",
    )
    .unwrap()
});

/// JS: runCopyEditPostApplyChecks({ cwd, files })
pub fn run_copy_edit_post_apply_checks(cwd: &str, files: &[String]) -> Value {
    let mut failures: Vec<Value> = Vec::new();
    let mut warnings: Vec<Value> = Vec::new();
    let mut seen: HashSet<&String> = HashSet::new();
    let unique_files: Vec<&String> = files
        .iter()
        .filter(|f| !impeccable_context::util::js_trim(f).is_empty())
        .filter(|f| seen.insert(f))
        .collect();

    for relative_file in unique_files {
        let file = jsp::resolve(cwd, &[relative_file]);
        if !is_path_inside_or_equal(cwd, &file) || !exists(&file) {
            warnings
                .push(json!({ "file": relative_file, "reason": "file_missing_or_outside_cwd" }));
            continue;
        }
        let content = match std::fs::read(&file) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => {
                failures.push(json!({
                    "file": relative_file,
                    "reason": "read_failed",
                    "message": impeccable_context::util::node_read_error(&file, &e),
                }));
                continue;
            }
        };
        if let Some(marker) = find_leftover_impeccable_marker(&content) {
            failures.push(json!({
                "file": relative_file,
                "reason": "leftover_impeccable_marker",
                "marker": marker,
            }));
        }
        if relative_file.ends_with(".json") {
            if serde_json::from_str::<Value>(&content).is_err() {
                let message = crate::json_error::json_parse_error(&content)
                    .unwrap_or_else(|| "Unexpected token".to_string());
                failures.push(json!({
                    "file": relative_file,
                    "reason": "invalid_json",
                    "message": message,
                }));
            }
        }
        if let Some(check) = check_framework_source_syntax(cwd, relative_file, &file) {
            if let Some(failure) = check.get("failure") {
                failures.push(failure.clone());
            }
            if let Some(warning) = check.get("warning") {
                warnings.push(warning.clone());
            }
        }
        if relative_file.ends_with(".mjs")
            || relative_file.ends_with(".cjs")
            || relative_file.ends_with(".js")
        {
            if let Ok(check) = Command::new(proc::node_exe())
                .arg("--check")
                .arg(&file)
                .current_dir(cwd)
                .output()
            {
                if !check.status.success() {
                    let stderr = String::from_utf8_lossy(&check.stderr).into_owned();
                    let stdout = String::from_utf8_lossy(&check.stdout).into_owned();
                    let message = if !stderr.is_empty() { stderr } else { stdout };
                    failures.push(json!({
                        "file": relative_file,
                        "reason": "invalid_js",
                        "message": impeccable_context::util::js_trim(&message),
                    }));
                }
            }
            // A missing `node` cannot report a syntax error; skip the check.
        }
    }
    if let Some(validation) = run_manual_edit_validation_script(cwd) {
        if let Some(failure) = validation.get("failure") {
            failures.push(failure.clone());
        }
        if let Some(warning) = validation.get("warning") {
            warnings.push(warning.clone());
        }
    }
    json!({
        "ok": failures.is_empty(),
        "failures": failures,
        "warnings": warnings,
    })
}

/// JS: checkFrameworkSourceSyntax(relativeFile, content). `@babel/parser` is a
/// Node dependency, so the parse runs in a short `node -e` script that resolves
/// the parser from the project. A missing node or parser reproduces the JS
/// `require` failure path (`syntax_parser_unavailable`).
fn check_framework_source_syntax(cwd: &str, relative_file: &str, absolute: &str) -> Option<Value> {
    if !(relative_file.ends_with(".jsx")
        || relative_file.ends_with(".tsx")
        || relative_file.ends_with(".ts"))
    {
        return None;
    }
    let mut plugins = vec!["jsx"];
    if relative_file.ends_with(".ts") || relative_file.ends_with(".tsx") {
        plugins.push("typescript");
    }
    let script = r#"
const { createRequire } = require('node:module');
const fs = require('node:fs');
let parser;
try {
  parser = createRequire(process.env.PHOENIX_UI_SYNTAX_CWD + '/noop.js')('@babel/parser');
} catch {
  process.exit(3);
}
const content = fs.readFileSync(process.env.PHOENIX_UI_SYNTAX_FILE, 'utf-8');
const plugins = JSON.parse(process.env.PHOENIX_UI_SYNTAX_PLUGINS);
try {
  parser.parse(content, { sourceType: 'module', plugins, errorRecovery: false });
  process.exit(0);
} catch (err) {
  process.stderr.write(err.message || String(err));
  process.exit(4);
}
"#;
    let output = Command::new(proc::node_exe())
        .arg("-e")
        .arg(script)
        .current_dir(cwd)
        .env("PHOENIX_UI_SYNTAX_CWD", cwd)
        .env("PHOENIX_UI_SYNTAX_FILE", absolute)
        .env(
            "PHOENIX_UI_SYNTAX_PLUGINS",
            serde_json::to_string(&plugins).unwrap_or_else(|_| "[]".into()),
        )
        .output();
    let Ok(output) = output else {
        return Some(json!({
            "warning": { "file": relative_file, "reason": "syntax_parser_unavailable" }
        }));
    };
    match output.status.code() {
        Some(0) => None,
        Some(4) => {
            let message = String::from_utf8_lossy(&output.stderr).into_owned();
            Some(json!({
                "failure": {
                    "file": relative_file,
                    "reason": "invalid_source_syntax",
                    "message": message,
                }
            }))
        }
        _ => Some(json!({
            "warning": { "file": relative_file, "reason": "syntax_parser_unavailable" }
        })),
    }
}

/// JS: findLeftoverImpeccableMarker(content)
fn find_leftover_impeccable_marker(content: &str) -> Option<String> {
    if let Some(m) = MARKER_RE.find(content) {
        return Some(m.as_str().to_string());
    }
    for line in content.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        for m in ATTR_RE.find_iter(line) {
            if !is_inside_quoted_literal(line, m.start()) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

/// JS: isInsideQuotedLiteral(line, index)
fn is_inside_quoted_literal(line: &str, index: usize) -> bool {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, ch) in line.char_indices() {
        if i >= index {
            break;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '`' {
            quote = Some(ch);
        }
    }
    quote.is_some()
}

/// JS: runManualEditValidationScript(cwd)
fn run_manual_edit_validation_script(cwd: &str) -> Option<Value> {
    let script = read_manual_edit_validation_script(cwd)?;
    // JS: spawnSync(script, { shell: true }): /bin/sh -c on unix, cmd.exe on
    // Windows.
    let comspec = std::env::var("ComSpec").ok();
    let child = proc::shell(&script, comspec.as_deref())
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return Some(json!({
                "failure": {
                    "file": "package.json",
                    "reason": "manual_edit_validation_failed",
                    "message": e.to_string(),
                }
            }));
        }
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(30_000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    return Some(json!({
                        "failure": {
                            "file": "package.json",
                            "reason": "manual_edit_validation_failed",
                            "message": format!("spawnSync {} ETIMEDOUT", script),
                        }
                    }));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                return Some(json!({
                    "failure": {
                        "file": "package.json",
                        "reason": "manual_edit_validation_failed",
                        "message": e.to_string(),
                    }
                }));
            }
        }
    }
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return Some(json!({
                "failure": {
                    "file": "package.json",
                    "reason": "manual_edit_validation_failed",
                    "message": e.to_string(),
                }
            }));
        }
    };
    if output.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let joined: Vec<String> = [stderr, stdout]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    Some(json!({
        "failure": {
            "file": "package.json",
            "reason": "manual_edit_validation_failed",
            "message": impeccable_context::util::js_trim(&joined.join("\n")),
        }
    }))
}

fn read_manual_edit_validation_script(cwd: &str) -> Option<String> {
    let pkg_path = jsp::join(&[cwd, "package.json"]);
    if !exists(&pkg_path) {
        return None;
    }
    let raw = std::fs::read(&pkg_path).ok()?;
    let pkg: Value = serde_json::from_str(&String::from_utf8_lossy(&raw)).ok()?;
    let script = pkg.get("scripts").and_then(|s| {
        // Existing projects may still own the legacy validation script.
        s.get("phoenix-ui:manual-edit-validate")
            .or_else(|| s.get("impeccable:manual-edit-validate"))
    })?;
    match script {
        Value::String(s) if !impeccable_context::util::js_trim(s).is_empty() => Some(s.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Result normalization / mock
// ---------------------------------------------------------------------------

/// JS: normalizeBatchResult(result)
fn normalize_batch_result(result: &Value) -> Value {
    let status = match result.get("status").and_then(|s| s.as_str()) {
        Some("partial") => "partial",
        Some("error") => "error",
        _ => "done",
    };
    let applied_entry_ids: Vec<Value> = match result.get("appliedEntryIds") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|v| matches!(v, Value::String(_)))
            .cloned()
            .collect(),
        _ => vec![],
    };
    let failed: Vec<Value> = match result.get("failed") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|item| truthy(Some(item)))
            .map(|item| {
                let mut m = Map::new();
                let entry_id = item
                    .get("entryId")
                    .filter(|v| truthy(Some(v)))
                    .or_else(|| item.get("id").filter(|v| truthy(Some(v))))
                    .cloned()
                    .unwrap_or(Value::Null);
                m.insert("entryId".into(), entry_id);
                let reason = item
                    .get("reason")
                    .filter(|v| truthy(Some(v)))
                    .or_else(|| item.get("message").filter(|v| truthy(Some(v))))
                    .cloned()
                    .unwrap_or(json!("failed"));
                m.insert("reason".into(), reason);
                m.insert(
                    "candidates".into(),
                    Value::Array(match item.get("candidates") {
                        Some(Value::Array(c)) => c.clone(),
                        _ => vec![],
                    }),
                );
                Value::Object(m)
            })
            .collect(),
        _ => vec![],
    };
    let files: Vec<Value> = match result.get("files") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|v| matches!(v, Value::String(_)))
            .cloned()
            .collect(),
        _ => vec![],
    };
    let notes: Vec<Value> = match result.get("notes") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|v| matches!(v, Value::String(_)))
            .cloned()
            .collect(),
        _ => vec![],
    };
    let warnings: Vec<Value> = match result.get("warnings") {
        Some(Value::Array(a)) => a
            .iter()
            .filter(|v| truthy(Some(v)))
            .map(|w| match w {
                Value::String(s) => json!({ "message": s }),
                other => other.clone(),
            })
            .filter(|w| w.is_object() || w.is_array())
            .collect(),
        _ => vec![],
    };
    let mut m = Map::new();
    m.insert("status".into(), json!(status));
    m.insert(
        "message".into(),
        match result.get("message") {
            Some(v) if truthy(Some(v)) => v.clone(),
            _ => Value::Null,
        },
    );
    m.insert("appliedEntryIds".into(), Value::Array(applied_entry_ids));
    m.insert("failed".into(), Value::Array(failed));
    m.insert("files".into(), Value::Array(files));
    m.insert("notes".into(), Value::Array(notes));
    m.insert("warnings".into(), Value::Array(warnings));
    Value::Object(m)
}

/// JS: mockBatchResult(batch, env, cwd)
fn mock_batch_result(batch: &Value, env: &Env, cwd: &str) -> Result<Value, String> {
    apply_mock_writes(env, cwd)?;
    if let Some(raw) = env.get("PHOENIX_UI_LIVE_COPY_AGENT_MOCK_RESULT") {
        if !raw.is_empty() {
            if let Some(parsed) = parse_copy_edit_batch_result(raw) {
                return Ok(parsed);
            }
            return Err("Invalid PHOENIX_UI_LIVE_COPY_AGENT_MOCK_RESULT JSON".to_string());
        }
    }
    let applied: Vec<Value> = arr(batch.get("entries"))
        .iter()
        .filter_map(|e| e.get("id").cloned())
        .filter(|v| truthy(Some(v)))
        .collect();
    Ok(json!({
        "status": "done",
        "appliedEntryIds": applied,
        "failed": [],
        "files": [],
        "notes": ["mock copy-edit batch result"],
    }))
}

/// JS: applyMockWrites(env, cwd)
fn apply_mock_writes(env: &Env, cwd: &str) -> Result<(), String> {
    let Some(raw) = env.get("PHOENIX_UI_LIVE_COPY_AGENT_MOCK_WRITES") else {
        return Ok(());
    };
    if raw.is_empty() {
        return Ok(());
    }
    let writes: Option<Value> = serde_json::from_str(raw).ok();
    let Some(Value::Object(writes)) = writes else {
        return Err("Invalid PHOENIX_UI_LIVE_COPY_AGENT_MOCK_WRITES JSON".to_string());
    };
    for (relative_file, content) in writes {
        let Value::String(content) = content else {
            continue;
        };
        let absolute = jsp::resolve(cwd, &[&relative_file]);
        if !is_path_inside_or_equal(cwd, &absolute) {
            continue;
        }
        if let Some(parent) = std::path::Path::new(&absolute).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&absolute, content);
    }
    Ok(())
}

/// JS: parseCopyEditAgentResult(text)
pub fn parse_copy_edit_agent_result(text: &str) -> Option<Value> {
    let trimmed = impeccable_context::util::js_trim(text);
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed_outer) = serde_json::from_str::<Value>(trimmed) {
        if truthy(Some(&parsed_outer)) {
            if let Some(Value::String(inner)) = parsed_outer.get("result") {
                if let Some(nested) = parse_copy_edit_agent_result(inner) {
                    return Some(nested);
                }
            }
            if matches!(
                parsed_outer.get("status").and_then(|s| s.as_str()),
                Some("done") | Some("partial") | Some("error")
            ) {
                return Some(parsed_outer);
            }
        }
    }
    // `/\{[\s\S]*\}/`: the first `{` through the last `}`.
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end < start {
        return None;
    }
    let parsed: Value = serde_json::from_str(&trimmed[start..=end]).ok()?;
    if matches!(
        parsed.get("status").and_then(|s| s.as_str()),
        Some("done") | Some("partial") | Some("error")
    ) {
        return Some(parsed);
    }
    None
}

// ---------------------------------------------------------------------------
// Provider selection
// ---------------------------------------------------------------------------

/// Choose only a local host callback or an explicitly requested test mock.
/// Legacy runner configuration is not inherited by Phoenix UI.
pub fn choose_copy_edit_agent(
    env: &Env,
    chat_available: Option<&dyn Fn() -> bool>,
) -> Option<String> {
    let mode = env
        .get("PHOENIX_UI_LIVE_COPY_AGENT")
        .map(|s| impeccable_context::util::js_trim(s).to_ascii_lowercase())
        .unwrap_or_default();
    match mode.as_str() {
        "mock" => Some("mock".into()),
        "" | "auto" | "host" | "chat" if chat_available.map(|f| f()).unwrap_or(false) => {
            Some("chat".into())
        }
        _ => None,
    }
}

/// The server uses this before connecting the existing poll/reply queue.
pub fn host_copy_edit_requested(env: &Env, chat_available: bool) -> bool {
    choose_copy_edit_agent(env, Some(&|| chat_available)).as_deref() == Some("chat")
}

pub fn describe_no_provider_error(_env: &Env, chat_available: bool) -> String {
    if chat_available {
        "Phoenix UI live copy editing needs the current host's local apply callback. Continue the live session in your host, or edit the source manually.".into()
    } else {
        "No current Phoenix UI host session is polling this live server. Start Phoenix UI live in your current host session, or edit the source manually and discard the staged browser copies. External agent runners are disabled.".into()
    }
}
