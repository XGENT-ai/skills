//! Applies staged live copy edits through the current host session.
//! The explicit mock provider is retained for deterministic tests.

use crate::event_validation::truthy;
use crate::manual_edits::evidence::{arr, is_path_inside_or_equal};
use crate::util::{jsp, Env};
use serde_json::{json, Map, Value};

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
