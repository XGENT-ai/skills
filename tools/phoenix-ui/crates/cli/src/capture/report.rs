//! `capture-report.json`: one provenance record per screenshot, merged by
//! file name so a partial re-capture keeps the record of what it reused.

use serde_json::{json, Map, Value};

pub const SCHEMA: &str = "phoenix-ui-capture-v1";

/// This run's records, before merging.
#[derive(Default)]
pub struct Run {
    pub shots: Vec<Value>,
    pub failures: Vec<Value>,
    pub sheets: Vec<Value>,
}

fn key(entry: &Value) -> (String, String) {
    let text = |k: &str| {
        entry
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let viewport = entry
        .pointer("/viewport/label")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| text("viewport"));
    (text("state"), viewport)
}

fn file(entry: &Value) -> &str {
    entry.get("file").and_then(Value::as_str).unwrap_or("")
}

/// Replace the entries this run re-captured; keep the rest as they were.
pub fn merge(existing: Option<Value>, run: &Run, head: Map<String, Value>) -> Value {
    let existing = existing.filter(|e| e.get("schema").and_then(Value::as_str) == Some(SCHEMA));
    let old = |k: &str| -> Vec<Value> {
        existing
            .as_ref()
            .and_then(|e| e.get(k))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let touched_files: Vec<&str> = run.shots.iter().chain(&run.failures).map(file).collect();
    let touched_states: Vec<(String, String)> =
        run.shots.iter().chain(&run.failures).map(key).collect();

    let mut shots: Vec<Value> = Vec::new();
    for entry in old("shots") {
        if let Some(new) = run.shots.iter().find(|s| file(s) == file(&entry)) {
            shots.push(new.clone());
        } else if !touched_files.contains(&file(&entry)) {
            shots.push(entry);
        }
    }
    for new in &run.shots {
        if !shots.iter().any(|s| file(s) == file(new)) {
            shots.push(new.clone());
        }
    }
    let mut failures: Vec<Value> = old("failures")
        .into_iter()
        .filter(|f| !touched_states.contains(&key(f)))
        .collect();
    failures.extend(run.failures.iter().cloned());
    let mut sheets: Vec<Value> = old("sheets")
        .into_iter()
        .filter(|s| !run.sheets.iter().any(|n| file(n) == file(s)))
        .collect();
    sheets.extend(run.sheets.iter().cloned());

    let mut out = Map::new();
    out.insert("schema".into(), json!(SCHEMA));
    out.extend(head);
    out.insert("shots".into(), Value::Array(shots));
    out.insert("failures".into(), Value::Array(failures));
    out.insert("sheets".into(), Value::Array(sheets));
    Value::Object(out)
}

/// Replace every occurrence of a secret in every string with `***`.
pub fn redact(value: &mut Value, secrets: &[String]) {
    match value {
        Value::String(s) => *s = redact_str(s, secrets),
        Value::Array(items) => items.iter_mut().for_each(|v| redact(v, secrets)),
        Value::Object(map) => map.values_mut().for_each(|v| redact(v, secrets)),
        _ => {}
    }
}

pub fn redact_str(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_string();
    for secret in secrets.iter().filter(|s| !s.is_empty()) {
        out = out.replace(secret.as_str(), "***");
    }
    out
}

/// One terminal line per shot and failure, then the summary.
pub fn lines(run: &Run, report_path: &str) -> Vec<String> {
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let mut out = Vec::new();
    let mut invalid = 0;
    for shot in &run.shots {
        let (state, viewport) = key(shot);
        if shot.get("valid").and_then(Value::as_bool) == Some(true) {
            let findings = shot
                .get("findings")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            out.push(format!(
                "{state} {viewport} {} findings={findings}",
                file(shot)
            ));
        } else {
            invalid += 1;
            let reasons: Vec<String> = shot
                .get("invalidReasons")
                .and_then(Value::as_array)
                .map(|r| {
                    r.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            out.push(format!(
                "INVALID {state} {viewport} {}: {}",
                text(shot, "invalidFile"),
                reasons.join("；")
            ));
        }
        if let Some(error) = shot.get("findingsError").and_then(Value::as_str) {
            out.push(format!(
                "SCAN-ERROR {state} {viewport} {}: {error}",
                file(shot)
            ));
        }
        for problem in shot
            .pointer("/derived/problems")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            out.push(format!(
                "DERIVED {state} {viewport} {}: {problem}",
                file(shot)
            ));
        }
    }
    for failure in &run.failures {
        let (state, viewport) = key(failure);
        let step = failure.get("step").and_then(Value::as_u64).unwrap_or(0);
        out.push(format!(
            "FAILED {state} {viewport} step {step} ({}): {}",
            text(failure, "do"),
            text(failure, "reason")
        ));
        for hint in failure
            .get("hints")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            out.push(format!("  可改为：{hint}"));
        }
    }
    for sheet in &run.sheets {
        if let Some(problem) = sheet.get("problem").and_then(Value::as_str) {
            out.push(format!(
                "DERIVED sheet {}: {problem}",
                text(sheet, "viewport")
            ));
        }
    }
    out.push(format!(
        "capture: {} shots, {invalid} invalid, {} failed → {report_path}",
        run.shots.len(),
        run.failures.len()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(file: &str, state: &str, at: &str) -> Value {
        json!({ "file": file, "state": state, "viewport": { "label": "desktop" }, "valid": true, "capturedAt": at })
    }

    #[test]
    fn merge_replaces_retaken_files_and_keeps_the_rest() {
        let existing = json!({
            "schema": SCHEMA,
            "shots": [shot("desktop.png", "initial", "t0"), shot("desktop-a.png", "a", "t0"), shot("desktop-b.png", "b", "t0")],
            "failures": [{ "state": "c", "viewport": "desktop", "file": "desktop-c.png" }],
            "sheets": [],
        });
        let run = Run {
            shots: vec![
                shot("desktop-a.png", "a", "t1"),
                shot("desktop-c.png", "c", "t1"),
            ],
            failures: vec![json!({ "state": "b", "viewport": "desktop", "file": "desktop-b.png" })],
            sheets: vec![],
        };
        let merged = merge(Some(existing), &run, Map::new());
        let files: Vec<(&str, &str)> = merged["shots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["file"].as_str().unwrap(),
                    s["capturedAt"].as_str().unwrap(),
                )
            })
            .collect();
        // Untouched shots keep their original capture time; the failed state's
        // old picture is gone; the recovered state is no longer a failure.
        assert_eq!(
            files,
            [
                ("desktop.png", "t0"),
                ("desktop-a.png", "t1"),
                ("desktop-c.png", "t1")
            ]
        );
        let failures: Vec<&str> = merged["failures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["state"].as_str().unwrap())
            .collect();
        assert_eq!(failures, ["b"]);
    }

    #[test]
    fn an_untouched_shot_keeps_its_own_tool_identity() {
        let old = |file: &str, sha: &str| {
            json!({
                "file": file, "state": "initial", "viewport": { "label": "desktop" },
                "valid": true, "capturedAt": "t0",
                "tool": { "name": "phoenix-ui capture", "version": "0.1.0", "binarySha256": sha }
            })
        };
        let existing = json!({
            "schema": SCHEMA,
            "tool": { "name": "phoenix-ui capture", "version": "0.1.0", "binarySha256": "b0" },
            "shots": [old("kept.png", "b0"), old("retaken.png", "b0")],
            "failures": [],
            "sheets": [],
        });
        let run = Run {
            shots: vec![json!({
                "file": "retaken.png", "state": "initial", "viewport": { "label": "desktop" },
                "valid": true, "capturedAt": "t1",
                "tool": { "name": "phoenix-ui capture", "version": "0.2.0", "binarySha256": "b1" }
            })],
            failures: vec![],
            sheets: vec![],
        };
        let mut head = Map::new();
        head.insert(
            "tool".into(),
            json!({ "name": "phoenix-ui capture", "version": "0.2.0", "binarySha256": "b1" }),
        );
        let merged = merge(Some(existing), &run, head);
        let get = |file: &str| {
            merged["shots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["file"] == file)
                .cloned()
                .unwrap()
        };
        // A partial re-capture by another engine must not rewrite the kept
        // shot's provenance: only entries re-taken this run get the new tool.
        assert_eq!(get("kept.png")["tool"]["binarySha256"], "b0");
        assert_eq!(get("retaken.png")["tool"]["binarySha256"], "b1");
        assert_eq!(merged["tool"]["binarySha256"], "b1");
    }

    #[test]
    fn a_report_of_another_schema_is_replaced() {
        let run = Run {
            shots: vec![shot("desktop.png", "initial", "t1")],
            ..Run::default()
        };
        let merged = merge(
            Some(json!({ "schema": "other", "shots": [shot("x.png", "x", "t0")] })),
            &run,
            Map::new(),
        );
        assert_eq!(merged["shots"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn redaction_reaches_every_string() {
        let mut value = json!({ "url": "http://h/?token=s3cret", "list": ["s3cret s3cret", 1], "nested": { "m": "pw:hunter2" } });
        redact(
            &mut value,
            &["s3cret".into(), "hunter2".into(), String::new()],
        );
        assert_eq!(
            value,
            json!({ "url": "http://h/?token=***", "list": ["*** ***", 1], "nested": { "m": "pw:***" } })
        );
    }
}
