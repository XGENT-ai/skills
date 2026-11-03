//! Live variant sessions, held in the helper's memory for its lifetime:
//! every browser or agent event for a session is folded into that session's
//! snapshot by `apply_event`.

use crate::util::{date_parse_ms, iso_now, js_number, now_ms};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const COMPLETED_SESSION_PHASES: [&str; 2] = ["completed", "discarded"];
pub const GENERATION_FENCED_SESSION_PHASES: [&str; 5] = [
    "accept_requested",
    "discard_requested",
    "carbonize_required",
    "completed",
    "discarded",
];

const MOUNT_FAILURE_HISTORY: usize = 5;

/// Every session this helper has seen, keyed (and listed) by id.
#[derive(Default)]
pub struct Sessions {
    snapshots: BTreeMap<String, Map<String, Value>>,
}

impl Sessions {
    /// Fold one event (`{ id, type, ... }`) into its session; returns the
    /// new snapshot.
    pub fn append_event(&mut self, event: &Value) -> Result<Map<String, Value>, String> {
        let Some(obj) = event.as_object() else {
            return Err("event object required".to_string());
        };
        let Some(id) = obj
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            return Err("event id required".to_string());
        };
        if !obj
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| !t.is_empty())
        {
            return Err("event type required".to_string());
        }
        let prior = self
            .snapshots
            .get(id)
            .cloned()
            .unwrap_or_else(|| base_snapshot(id));
        let next = apply_event(&prior, &json!({ "ts": iso_now(), "event": event }));
        self.snapshots.insert(id.to_string(), next.clone());
        Ok(next)
    }

    pub fn has(&self, id: &str) -> bool {
        self.snapshots.contains_key(id)
    }

    /// The session's snapshot, terminal or not.
    pub fn get(&self, id: &str) -> Option<&Map<String, Value>> {
        self.snapshots.get(id)
    }

    /// Non-terminal sessions, ordered by id.
    pub fn active(&self) -> impl Iterator<Item = &Map<String, Value>> {
        self.snapshots.values().filter(|s| {
            let phase = s.get("phase").and_then(Value::as_str).unwrap_or("");
            !COMPLETED_SESSION_PHASES.contains(&phase)
        })
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// JS: baseSnapshot(id)
pub fn base_snapshot(id: &str) -> Map<String, Value> {
    let v = json!({
        "id": id,
        "phase": "new",
        "pageUrl": null,
        "sourceFile": null,
        "previewFile": null,
        "previewMode": null,
        "expectedVariants": 0,
        "arrivedVariants": 0,
        "visibleVariant": null,
        "paramValues": {},
        "checkpointRevision": 0,
        "browserCheckpointRevision": 0,
        "publicationCheckpointRevision": 0,
        "activeOwner": null,
        "sourceMarkers": {},
        "fallbackMode": null,
        "generationPhase": null,
        "generationCompletedAt": null,
        "generationTimings": {},
        "variantPlan": null,
        "generationCanceled": false,
        "generationCanceledAt": null,
        "cancelReason": null,
        "annotationArtifacts": [],
        "mountedVariants": [],
        "mountFailures": [],
        "renderState": null,
        "diagnostics": [],
        "updatedAt": null,
    });
    v.as_object().cloned().unwrap_or_default()
}

fn derive_render_state(s: &Map<String, Value>) -> Value {
    let mounted = s
        .get("mountedVariants")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let failures = s
        .get("mountFailures")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    if mounted > 0 {
        return json!("mounted");
    }
    if failures > 0 {
        return json!("failed");
    }
    if truthy(s.get("generationCompletedAt").unwrap_or(&Value::Null)) {
        return json!("pending");
    }
    Value::Null
}

fn is_nullish(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Null))
}

/// `a ?? b`
fn coalesce<'a>(a: Option<&'a Value>, b: &'a Value) -> Value {
    match a {
        Some(v) if !v.is_null() => v.clone(),
        _ => b.clone(),
    }
}

fn ts_ms(entry: &Value) -> Option<i64> {
    // JS: Date.parse(entry.ts || '') || null
    let ts = entry.get("ts").and_then(|t| t.as_str()).unwrap_or("");
    date_parse_ms(ts).filter(|v| *v != 0)
}

fn push_diag(next: &mut Map<String, Value>, d: Value) {
    let mut arr = next
        .get("diagnostics")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    arr.push(d);
    next.insert("diagnostics".to_string(), Value::Array(arr));
}

fn drop_diag(next: &mut Map<String, Value>, error: &str) {
    let mut arr = next
        .get("diagnostics")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    arr.retain(|d| d.get("error").and_then(|e| e.as_str()) != Some(error));
    next.insert("diagnostics".to_string(), Value::Array(arr));
}

/// JS: applyEvent(snapshot, entry)
pub fn apply_event(snapshot: &Map<String, Value>, entry: &Value) -> Map<String, Value> {
    let event: Map<String, Value> = match entry.get("event") {
        Some(Value::Object(o)) => o.clone(),
        Some(_) | None => entry.as_object().cloned().unwrap_or_default(),
    };
    let mut next = snapshot.clone();
    // Spread copies (JS: fresh objects for the mutable containers).
    for k in ["paramValues", "sourceMarkers", "generationTimings"] {
        let v = next
            .get(k)
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        next.insert(k.to_string(), Value::Object(v));
    }
    let vp = next.get("variantPlan").cloned().unwrap_or(Value::Null);
    next.insert(
        "variantPlan".to_string(),
        if truthy(&vp) { vp } else { Value::Null },
    );
    for k in [
        "annotationArtifacts",
        "mountedVariants",
        "mountFailures",
        "diagnostics",
    ] {
        let v = next
            .get(k)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        next.insert(k.to_string(), Value::Array(v));
    }
    let rs = next.get("renderState").cloned().unwrap_or(Value::Null);
    next.insert("renderState".to_string(), rs);
    next.insert(
        "updatedAt".to_string(),
        coalesce(
            entry.get("ts").filter(|t| truthy(t)),
            &Value::String(iso_now()),
        ),
    );

    let ev = |k: &str| event.get(k);
    let evt_type = event
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let phase_now = next
        .get("phase")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    let canceled = truthy(next.get("generationCanceled").unwrap_or(&Value::Null));
    let fenced = GENERATION_FENCED_SESSION_PHASES.contains(&phase_now.as_str());
    let now_or_ts = |entry: &Value| -> Value {
        match ts_ms(entry) {
            Some(ms) => json!(ms),
            None => json!(now_ms() as i64),
        }
    };

    macro_rules! set_if {
        ($k:expr, $v:expr) => {{
            let cur = next.get($k).cloned().unwrap_or(Value::Null);
            next.insert($k.to_string(), coalesce($v, &cur));
        }};
    }
    macro_rules! set {
        ($k:expr, $v:expr) => {
            next.insert($k.to_string(), $v);
        };
    }

    match evt_type.as_str() {
        "generate" => {
            set!("phase", json!("generate_requested"));
            // `origin: "agent"` marks a Go the generate verb fired; the
            // accept pipeline bakes those sessions itself. A plain Go
            // carries no origin and its snapshot stays exactly as it was.
            if let Some(origin) = ev("origin").filter(|v| truthy(v)) {
                set!("origin", origin.clone());
            }
            set_if!("pageUrl", ev("pageUrl"));
            set_if!("expectedVariants", ev("count"));
            set!("variantPlan", Value::Null);
            set!("mountedVariants", json!([]));
            set!("mountFailures", json!([]));
            set!("renderState", Value::Null);
            if let Some(sp) = ev("screenshotPath").filter(|v| truthy(v)) {
                let mut arts = next
                    .get("annotationArtifacts")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let exists_already = arts.iter().any(|a| {
                    a.get("path") == Some(sp)
                        && a.get("type").and_then(|t| t.as_str()) == Some("screenshot")
                });
                if !exists_already {
                    arts.push(json!({ "type": "screenshot", "path": sp }));
                }
                set!("annotationArtifacts", Value::Array(arts));
            }
        }
        "variant_plan" => {
            if !canceled && !fenced {
                set_if!("variantPlan", ev("plan"));
            }
        }
        "detector_waivers" => {
            if !canceled && !fenced {
                let mut w = next
                    .get("detectorWaivers")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if let Some(arr) = ev("waivers").and_then(|v| v.as_array()) {
                    w.extend(arr.iter().cloned());
                }
                set!("detectorWaivers", Value::Array(w));
            }
        }
        "agent_phase" => {
            set_if!("generationPhase", ev("phase"));
            if let Some(phase) = ev("phase").filter(|v| truthy(v)) {
                let key = match phase {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let at = match ev("at") {
                    Some(v) if !v.is_null() => v.clone(),
                    _ => ts_ms(entry).map(|ms| json!(ms)).unwrap_or(Value::Null),
                };
                let dur = coalesce(ev("durationMs"), &Value::Null);
                let mut timings = next
                    .get("generationTimings")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                timings.insert(key, json!({ "at": at, "durationMs": dur }));
                set!("generationTimings", Value::Object(timings));
            }
        }
        "variants_ready" | "agent_done" => {
            let carbonize = ev("carbonize") == Some(&Value::Bool(true));
            if (canceled || fenced)
                && !(evt_type == "agent_done" && carbonize && phase_now == "accept_requested")
            {
                push_diag(
                    &mut next,
                    json!({ "error": "late_generation_event_ignored", "type": evt_type, "phase": phase_now }),
                );
            } else {
                set!(
                    "phase",
                    json!(if carbonize {
                        "carbonize_required"
                    } else {
                        "variants_ready"
                    })
                );
                let at = match ev("at") {
                    Some(v) if !v.is_null() => v.clone(),
                    _ => now_or_ts(entry),
                };
                set!("generationCompletedAt", at);
                let sf = coalesce(
                    ev("sourceFile"),
                    &coalesce(ev("file"), next.get("sourceFile").unwrap_or(&Value::Null)),
                );
                set!("sourceFile", sf);
                set_if!("previewFile", ev("previewFile"));
                set_if!("previewMode", ev("previewMode"));
                let arrived = match ev("arrivedVariants") {
                    Some(v) if !v.is_null() => v.clone(),
                    _ => {
                        let expected = next.get("expectedVariants").cloned().unwrap_or(Value::Null);
                        if truthy(&expected) {
                            expected
                        } else {
                            let a = next.get("arrivedVariants").cloned().unwrap_or(Value::Null);
                            if truthy(&a) {
                                a
                            } else {
                                json!(0)
                            }
                        }
                    }
                };
                set!("arrivedVariants", arrived);
                if carbonize {
                    let file = match ev("file") {
                        Some(v) if truthy(v) => v.clone(),
                        _ => Value::Null,
                    };
                    push_diag(
                        &mut next,
                        json!({
                            "error": "carbonize_cleanup_required",
                            "file": file,
                            "message": "Accepted variant still has carbonize markers that must be folded into source CSS.",
                        }),
                    );
                }
                let rs = derive_render_state(&next);
                set!("renderState", rs);
            }
        }
        "variant_mounted" => {
            let variant = js_number(ev("variant"));
            match variant {
                Some(v) if v.fract() == 0.0 && v >= 1.0 => {
                    let v = v as i64;
                    let mut mounted: Vec<i64> = next
                        .get("mountedVariants")
                        .and_then(|m| m.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
                        .unwrap_or_default();
                    if !mounted.contains(&v) {
                        mounted.push(v);
                        mounted.sort();
                    }
                    set!("mountedVariants", json!(mounted));
                    let rs = derive_render_state(&next);
                    set!("renderState", rs);
                }
                _ => {
                    push_diag(
                        &mut next,
                        json!({ "error": "malformed_mount_ack", "type": evt_type, "variant": coalesce(ev("variant"), &Value::Null) }),
                    );
                }
            }
        }
        "variant_mount_failed" => {
            let variant = js_number(ev("variant"));
            match variant {
                Some(v) if v.fract() == 0.0 && v >= 1.0 => {
                    let mut failures = next
                        .get("mountFailures")
                        .and_then(|m| m.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let at = match ev("at") {
                        Some(a) if !a.is_null() => a.clone(),
                        _ => now_or_ts(entry),
                    };
                    failures.push(json!({
                        "variant": v as i64,
                        "url": ev("url").and_then(|u| u.as_str()).map(|s| json!(s)).unwrap_or(Value::Null),
                        "error": ev("error").and_then(|u| u.as_str()).map(|s| json!(s)).unwrap_or(Value::Null),
                        "at": at,
                    }));
                    if failures.len() > MOUNT_FAILURE_HISTORY {
                        failures = failures[failures.len() - MOUNT_FAILURE_HISTORY..].to_vec();
                    }
                    set!("mountFailures", Value::Array(failures));
                    let rs = derive_render_state(&next);
                    set!("renderState", rs);
                }
                _ => {
                    push_diag(
                        &mut next,
                        json!({ "error": "malformed_mount_ack", "type": evt_type, "variant": coalesce(ev("variant"), &Value::Null) }),
                    );
                }
            }
        }
        "checkpoint" => {
            if canceled || fenced {
                push_diag(
                    &mut next,
                    json!({ "error": "checkpoint_after_terminal_ignored", "phase": coalesce(ev("phase"), &Value::Null), "revision": coalesce(ev("revision"), &Value::Null) }),
                );
            } else {
                let publication = ev("revisionDomain").and_then(|v| v.as_str())
                    == Some("publication")
                    || (ev("reason").and_then(|v| v.as_str()) == Some("variants_progress")
                        && !truthy(ev("owner").unwrap_or(&Value::Null)));
                let domain = if publication {
                    "publication"
                } else {
                    "browser"
                };
                let field = if publication {
                    "publicationCheckpointRevision"
                } else {
                    "browserCheckpointRevision"
                };
                let current_revision: f64 = match next.get(field) {
                    Some(v) if !v.is_null() => js_number(Some(v)).unwrap_or(f64::NAN),
                    _ => {
                        if domain == "browser" {
                            match next.get("checkpointRevision") {
                                Some(v) if !v.is_null() => js_number(Some(v)).unwrap_or(f64::NAN),
                                _ => 0.0,
                            }
                        } else {
                            0.0
                        }
                    }
                };
                let rev_val = coalesce(ev("revision"), &json!(0));
                let rev_num = js_number(Some(&rev_val)).unwrap_or(f64::NAN);
                if rev_num >= current_revision {
                    set_if!("phase", ev("phase"));
                    let cur = next.get(field).cloned().unwrap_or(Value::Null);
                    let stored = coalesce(
                        ev("revision"),
                        &if current_revision.is_nan() {
                            cur
                        } else {
                            crate::util::js_num(current_revision)
                        },
                    );
                    set!(field, stored);
                    if domain == "browser" {
                        set_if!("checkpointRevision", ev("revision"));
                        set_if!("activeOwner", ev("owner"));
                    }
                    set_if!("arrivedVariants", ev("arrivedVariants"));
                    if domain == "browser" {
                        set_if!("visibleVariant", ev("visibleVariant"));
                    }
                    set_if!("sourceFile", ev("sourceFile"));
                    set_if!("previewFile", ev("previewFile"));
                    set_if!("previewMode", ev("previewMode"));
                    if domain == "browser" {
                        if let Some(pv) = ev("paramValues").filter(|v| truthy(v)) {
                            let copy = pv
                                .as_object()
                                .cloned()
                                .map(Value::Object)
                                .unwrap_or_else(|| json!({}));
                            set!("paramValues", copy);
                        }
                    }
                } else {
                    push_diag(
                        &mut next,
                        json!({ "error": "stale_checkpoint_ignored", "revision": ev("revision").cloned().unwrap_or(Value::Null), "revisionDomain": domain }),
                    );
                    // JS: `revision: event.revision` (undefined is dropped by JSON)
                    if is_nullish(ev("revision")) && ev("revision").is_none() {
                        if let Some(Value::Array(arr)) = next.get_mut("diagnostics") {
                            if let Some(Value::Object(last)) = arr.last_mut() {
                                last.shift_remove("revision");
                            }
                        }
                    }
                }
            }
        }
        "accept" | "accept_intent" => {
            set!("phase", json!("accept_requested"));
            set!("generationCanceled", json!(true));
            let at = match ev("at") {
                Some(a) if !a.is_null() => a.clone(),
                _ => now_or_ts(entry),
            };
            set!("generationCanceledAt", at);
            set!("cancelReason", json!("accept"));
            let vid = coalesce(
                ev("variantId"),
                next.get("visibleVariant").unwrap_or(&Value::Null),
            );
            let n = js_number(Some(&vid));
            set!(
                "visibleVariant",
                n.map(crate::util::js_num).unwrap_or(Value::Null)
            );
            if let Some(pv) = ev("paramValues").filter(|v| truthy(v)) {
                let copy = pv
                    .as_object()
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or_else(|| json!({}));
                set!("paramValues", copy);
            }
        }
        "manual_edit_apply" => {
            set!("phase", json!("manual_edit_apply_requested"));
            set_if!("pageUrl", ev("pageUrl"));
        }
        "steer" => {
            set!("phase", json!("steer_requested"));
            set_if!("pageUrl", ev("pageUrl"));
        }
        "carbonize_cleanup" => {
            set!("phase", json!("carbonize_cleanup_requested"));
            set_if!("sourceFile", ev("file"));
        }
        "steer_done" => {
            set!("phase", json!("steer_done"));
            let sf = coalesce(
                ev("sourceFile"),
                &coalesce(ev("file"), next.get("sourceFile").unwrap_or(&Value::Null)),
            );
            set!("sourceFile", sf);
            set_if!("previewFile", ev("previewFile"));
            set_if!("previewMode", ev("previewMode"));
            // JS: next.message = event.message ?? next.message (key appears
            // even when undefined... no: assigning undefined creates the key,
            // JSON.stringify drops it).
            match ev("message") {
                Some(v) if !v.is_null() => {
                    set!("message", v.clone());
                }
                _ => {
                    if let Some(cur) = next.get("message").cloned() {
                        set!("message", cur);
                    }
                }
            }
        }
        "discard" => {
            set!("phase", json!("discard_requested"));
            set!("generationCanceled", json!(true));
            let at = match ev("at") {
                Some(a) if !a.is_null() => a.clone(),
                _ => now_or_ts(entry),
            };
            set!("generationCanceledAt", at);
            set!("cancelReason", json!("discard"));
        }
        "discarded" => {
            set!("phase", json!("discarded"));
            drop_diag(&mut next, "carbonize_cleanup_required");
        }
        "complete" => {
            set!("phase", json!("completed"));
            let sf = coalesce(
                ev("sourceFile"),
                &coalesce(ev("file"), next.get("sourceFile").unwrap_or(&Value::Null)),
            );
            set!("sourceFile", sf);
            set_if!("previewFile", ev("previewFile"));
            set_if!("previewMode", ev("previewMode"));
            drop_diag(&mut next, "carbonize_cleanup_required");
        }
        "agent_error" => {
            if canceled && ev("sourceEventType").and_then(|v| v.as_str()) == Some("generate") {
                push_diag(
                    &mut next,
                    json!({ "error": "late_generation_event_ignored", "type": evt_type, "phase": phase_now }),
                );
            } else {
                set!("phase", json!("agent_error"));
                let msg = match ev("message") {
                    Some(v) if truthy(v) => v.clone(),
                    _ => json!("unknown agent error"),
                };
                push_diag(&mut next, json!({ "error": "agent_error", "message": msg }));
            }
        }
        _ => {
            push_diag(
                &mut next,
                json!({ "error": "unknown_event_type", "type": ev("type").cloned().unwrap_or(Value::Null) }),
            );
            if ev("type").is_none() {
                if let Some(Value::Array(arr)) = next.get_mut("diagnostics") {
                    if let Some(Value::Object(last)) = arr.last_mut() {
                        last.shift_remove("type");
                    }
                }
            }
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(event: Value) -> Value {
        json!({ "ts": "2026-01-01T00:00:00.000Z", "event": event })
    }

    fn has_diag(snapshot: &Map<String, Value>, error: &str) -> bool {
        snapshot
            .get("diagnostics")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .any(|d| d.get("error").and_then(|e| e.as_str()) == Some(error))
            })
            .unwrap_or(false)
    }

    fn replay(id: &str, events: &[Value]) -> Map<String, Value> {
        let mut snap = base_snapshot(id);
        for entry in events {
            snap = apply_event(&snap, entry);
        }
        snap
    }

    fn accept_carbonize_done(id: &str, terminal: &str) -> Map<String, Value> {
        replay(
            id,
            &[
                entry(json!({ "id": id, "type": "accept", "variantId": 2 })),
                entry(
                    json!({ "id": id, "type": "agent_done", "carbonize": true, "file": "index.html" }),
                ),
                entry(json!({ "id": id, "type": terminal })),
            ],
        )
    }

    #[test]
    fn complete_drops_carbonize_cleanup_required() {
        let snap = accept_carbonize_done("ab12cd34", "complete");
        assert_eq!(snap.get("phase").and_then(|p| p.as_str()), Some("completed"));
        assert!(!has_diag(&snap, "carbonize_cleanup_required"));
    }

    #[test]
    fn discarded_drops_carbonize_cleanup_required() {
        let snap = accept_carbonize_done("ab12cd34", "discarded");
        assert_eq!(snap.get("phase").and_then(|p| p.as_str()), Some("discarded"));
        assert!(!has_diag(&snap, "carbonize_cleanup_required"));
    }
}
