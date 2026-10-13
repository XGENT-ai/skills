//! Read-only wire-shape discovery. Examples round-trip through the same serde
//! input types as the workflow; they are deliberately not usable evidence.
use super::{to_value, Outcome};
use crate::{
    capture_evidence::{CaptureInput, Observation, Origin, TestedVersion, WorkingTree},
    policy::{Disposition, GapStatus, RefactorHistory, Severity},
    protocol::{applied, finalize, judgement, legacy},
    storage::{error, StoreError},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

const KINDS: &[&str] = &[
    "tested-version",
    "capture",
    "judge",
    "mappings",
    "adjudication",
    "runtime-check",
    "applied",
    "import",
];

fn typed<T: DeserializeOwned + Serialize>(value: Value) -> Outcome {
    // Parse the JSON wire representation, including Decimal's raw token parser.
    let value: T = serde_json::from_str(&value.to_string()).map_err(|e| {
        error(
            4,
            format!("Built-in discovery example does not match its input type: {e}"),
        )
    })?;
    to_value(&value)
}

fn entry<T: DeserializeOwned + Serialize>(
    name: &str,
    input_for: &str,
    examples: Vec<(&str, Value)>,
) -> Outcome {
    let examples = examples
        .into_iter()
        .map(|(label, value)| Ok(json!({"label": label, "value": typed::<T>(value)?})))
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(json!({"type": name, "inputFor": input_for, "examples": examples}))
}

pub(super) fn query(kind: Option<&str>) -> Outcome {
    if let Some(kind) = kind {
        if !KINDS.contains(&kind) {
            return Err(error(
                2,
                format!(
                    "Unknown review schema kind {kind}; expected {}",
                    KINDS.join(", ")
                ),
            ));
        }
    }
    let file = json!({"path": "example/replace-with-real-evidence", "sha256": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_SHA256"});
    let cell = json!({"unitId": "EXAMPLE_UNIT_FROM_BEGIN", "stateId": "EXAMPLE_STATE_FROM_BEGIN",
        "viewportId": "EXAMPLE_VIEWPORT_FROM_BEGIN", "themeId": "EXAMPLE_THEME_FROM_BEGIN", "stepId": "EXAMPLE_STEP_FROM_BEGIN"});
    let version = json!({"head": null, "workingTree": WorkingTree::Unknown, "buildId": null});
    let dirty = WorkingTree::Dirty {
        sha256: "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_WORKING_TREE_SHA256".into(),
    };
    let missing = json!({"schemaVersion": 1, "roundId": "EXAMPLE_ROUND_FROM_BEGIN", "cell": cell,
        "attempt": 1, "observation": {"status": "missing", "reason": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_MISSING_REASON"}});
    let captured = json!({"schemaVersion": 1, "roundId": "EXAMPLE_ROUND_FROM_BEGIN", "cell": cell, "attempt": 1,
        "observation": {"status": "captured", "image": file, "origin": Origin::Browser,
            "tool": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_CAPTURE_TOOL", "address": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_ADDRESS",
            "takenAt": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_TIMESTAMP",
            "viewport": {"id": cell["viewportId"], "width": 1, "height": 1, "dpr": 1.0, "fullPage": false},
            "testedVersion": version, "interactions": [{"action": "EXAMPLE_ONLY", "observed": "EXAMPLE_ONLY"}],
            "humanCheck": {"opened": false, "currentProduct": false, "currentVersion": false, "notes": "EXAMPLE_ONLY_NO_IMAGE_WAS_OPENED"}}});
    let basis = json!({"cell": cell, "image": file, "notes": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_REASSESSMENT"});
    let judge = json!({"schemaVersion": 1, "cycleId": "EXAMPLE_CYCLE_FROM_STATUS", "roundId": "EXAMPLE_ROUND_FROM_BEGIN",
        "packetHash": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_PACKET_HASH", "reviewerId": "EXAMPLE_REVIEWER_ID",
        "rawSha256": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_RAW_SHA256",
        "provenance": {"freshContext": false, "readOnly": false, "inheritedModel": false, "openedImages": [file], "notes": "EXAMPLE_ONLY_NO_REVIEW_WAS_RUN"},
        "conversion": judgement::Conversion::ReviewerStructured,
        "units": [{"unitId": cell["unitId"], "scores": ["0", "0", "0", "0"], "reasons": ["EXAMPLE_ONLY", "EXAMPLE_ONLY", "EXAMPLE_ONLY", "EXAMPLE_ONLY"],
            "gaps": [{"key": "EXAMPLE_TEMPORARY_GAP_KEY", "cell": cell, "image": file, "dimension": "EXAMPLE_ONLY",
                "location": "EXAMPLE_ONLY", "current": "EXAMPLE_ONLY", "expected": "EXAMPLE_ONLY", "basis": file,
                "basisLocation": "EXAMPLE_ONLY", "fix": "EXAMPLE_ONLY", "severity": Severity::P2, "recoverable": "0", "summary": "EXAMPLE_ONLY"}],
            "suggestions": ["EXAMPLE_ONLY"], "unseenCells": [{"cell": cell, "reason": "EXAMPLE_ONLY"}]}],
        "unreviewedUnits": [{"unitId": "EXAMPLE_OTHER_UNIT_FROM_PACKET", "reason": "EXAMPLE_ONLY"}],
        "previous": [{"id": "EXAMPLE_ISSUE_ID_FROM_PACKET", "status": GapStatus::Unverified, "basis": basis}],
        "summary": "EXAMPLE_ONLY_NO_REVIEW_WAS_RUN"});
    let conversion = judgement::Conversion::HostVerified {
        raw_sha256: "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_RAW_SHA256".into(),
        checked_units: vec!["EXAMPLE_UNIT_FROM_PACKET".into()],
        checked_gaps: vec!["EXAMPLE_GAP_FROM_REVIEW".into()],
        notes: "EXAMPLE_ONLY_NO_CONVERSION_WAS_CHECKED".into(),
    };
    let mut converted = judge.clone();
    converted["conversion"] = to_value(&conversion)?;
    let decision = json!({"source": file, "quote": "EXAMPLE_ONLY_NOT_A_USER_QUOTE", "date": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_DATE",
        "testedVersion": version, "issueIds": ["EXAMPLE_ISSUE_ID_FROM_REPORT"], "impact": "EXAMPLE_ONLY", "kind": finalize::DecisionKind::Direction});
    let adjudication = json!({"schemaVersion": 1, "cycleId": "EXAMPLE_CYCLE_FROM_STATUS", "roundId": "EXAMPLE_ROUND_FROM_BEGIN",
        "machineChecks": [file], "mainChecks": [file], "decisions": [decision], "applied": [], "notes": "EXAMPLE_ONLY_NO_ADJUDICATION_WAS_RUN"});
    let mut waiver = adjudication.clone();
    waiver["decisions"][0]["kind"] = to_value(&finalize::DecisionKind::WholeUiWaiver)?;
    let check = json!({"schemaVersion": 1, "kind": finalize::CheckKind::Page, "tool": "EXAMPLE_ONLY", "takenAt": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_TIMESTAMP",
        "testedVersion": version, "cell": cell, "image": file, "status": finalize::CheckStatus::Unavailable, "notes": "EXAMPLE_ONLY_NO_CHECK_WAS_RUN",
        "findings": [{"key": "EXAMPLE_FINDING_KEY", "existingIssue": null, "severity": Severity::P2, "description": "EXAMPLE_ONLY",
            "expected": "EXAMPLE_ONLY", "basis": file, "fix": "EXAMPLE_ONLY"}],
        "reassessed": [{"issueId": "EXAMPLE_ISSUE_ID_FROM_REPORT", "status": GapStatus::Unverified}]});
    let mut source_check = check.clone();
    source_check["kind"] = to_value(&finalize::CheckKind::Source)?;
    let scopes = json!([
        applied::RefactorScope::Global,
        applied::RefactorScope::Unit {
            unit_id: "EXAMPLE_UNIT_FROM_REPORT".into()
        }
    ]);
    let applied = json!({"schemaVersion": 1, "cycleId": "EXAMPLE_CYCLE_FROM_STATUS", "roundId": "EXAMPLE_ROUND_FROM_STATUS",
        "recordPath": "example/replace-with-original-record.md", "at": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_TIMESTAMP", "testedVersion": version,
        "items": [{"issueId": "EXAMPLE_ISSUE_ID_FROM_REPORT", "verification": "EXAMPLE_ONLY", "disposition": Disposition::Untreated,
            "changes": [file], "basis": [file], "selfCheck": {"kind": "not_run", "reason": "EXAMPLE_ONLY"}, "notes": "EXAMPLE_ONLY"}],
        "restructureScopes": [{"scope": scopes[0], "hypothesis": "EXAMPLE_ONLY", "change": "EXAMPLE_ONLY", "expected": "EXAMPLE_ONLY"},
            {"scope": scopes[1], "hypothesis": "EXAMPLE_ONLY", "change": "EXAMPLE_ONLY", "expected": "EXAMPLE_ONLY"}],
        "decisions": [decision], "checks": [{"command": "EXAMPLE_ONLY", "status": finalize::CheckStatus::Unavailable, "summary": "EXAMPLE_ONLY_NO_COMMAND_WAS_RUN"}],
        "unfinished": ["EXAMPLE_ONLY"]});
    let mut page_applied = applied.clone();
    page_applied["recordPath"] = Value::Null;
    page_applied["items"][0]["selfCheck"] =
        json!({"kind": "page", "capture": captured, "notes": "EXAMPLE_ONLY_NO_PAGE_WAS_CHECKED"});
    let legacy_map = json!({"kind": "legacyMarkdown", "schemaVersion": 1, "prefix": "example/replace-with-prefix", "target": "example/replace-with-target.md",
        "plan": null, "recordPath": null, "threshold": "0",
        "rounds": [{"roundId": "EXAMPLE_LEGACY_ROUND", "report": file, "targetFingerprint": {"algorithm": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_ALGORITHM", "value": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_FINGERPRINT"},
            "fullScore": {"state": legacy::Knowledge::Known, "value": "0"}}],
        "gaps": [{"id": "EXAMPLE_LEGACY_ISSUE_ID", "unitId": cell["unitId"], "severity": Severity::P2, "origin": finalize::GapOrigin::Reviewer,
            "assessments": [{"roundId": "EXAMPLE_LEGACY_ROUND", "status": GapStatus::Unverified,
                "priorDisposition": {"state": legacy::Knowledge::Known, "disposition": Disposition::Untreated}}],
            "sinceLastAssessment": {"state": legacy::Knowledge::Unknown, "disposition": null}, "pendingDecision": false}],
        "refactors": {"global": RefactorHistory::Unknown, "units": [{"unitId": cell["unitId"], "refactor": RefactorHistory::Unknown}]},
        "units": [{"unitId": cell["unitId"], "roundId": "EXAMPLE_LEGACY_ROUND", "judge": file, "images": [file]}], "sources": [file]});
    let rebase = json!({"kind": "projectionRebase", "schemaVersion": 1, "prefix": "example/replace-with-prefix", "cycleRevision": 0,
        "file": "example/replace-with-generated-file.md", "currentSha256": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_CURRENT_SHA256", "savedCopy": file});
    let mut inputs = json!({
        "tested-version": entry::<TestedVersion>("TestedVersion", "begin --tested-version", vec![
            ("clean", json!({"head": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_HEAD", "workingTree": WorkingTree::Clean, "buildId": null})),
            ("dirty", json!({"head": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_HEAD", "workingTree": dirty, "buildId": null})),
            ("build-id", json!({"head": null, "workingTree": WorkingTree::Unknown, "buildId": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_BUILD_ID"})),
            ("unknown", version),
        ])?,
        "capture": entry::<CaptureInput>("CaptureInput", "capture add --input; applied page selfCheck", vec![("missing", missing.clone()), ("captured", captured.clone())])?,
        "judge": entry::<judgement::JudgeInput>("JudgeInput", "judge add --input", vec![("reviewerStructured", judge.clone()), ("hostVerified", converted)])?,
        "mappings": entry::<Vec<judgement::GapMapping>>("GapMapping[]", "judge add --mappings", vec![("array", json!([{"key": "EXAMPLE_TEMPORARY_GAP_KEY", "id": "EXAMPLE_ISSUE_ID_FROM_PACKET", "basis": "EXAMPLE_ONLY_REPLACE_WITH_ACTUAL_MAPPING_BASIS"}]))])?,
        "adjudication": entry::<finalize::Adjudication>("Adjudication", "finalize --input", vec![("direction", adjudication), ("wholeUiWaiver", waiver)])?,
        "runtime-check": entry::<finalize::RuntimeCheck>("RuntimeCheck", "JSON files referenced by Adjudication.machineChecks/mainChecks", vec![("page", check), ("source", source_check)])?,
        "applied": entry::<applied::Input>("applied::Input", "apply record --input", vec![("not_run", applied.clone()), ("page", page_applied.clone())])?,
        "import": {"inputFor": "import --input", "examples": [
            {"label": "legacyMarkdown", "type": "LegacyMap", "value": typed::<legacy::LegacyMap>(legacy_map)?},
            {"label": "projectionRebase", "type": "Rebase", "value": typed::<legacy::Rebase>(rebase)?},
        ]},
    });
    if let Some(kind) = kind {
        inputs
            .as_object_mut()
            .unwrap()
            .retain(|name, _| name == kind);
    }
    Ok(json!({
        "reviewSchema": 1, "discoveryVersion": 1, "format": "typed-examples", "isJsonSchema": false,
        "readOnly": true, "examplesAreEvidence": false, "kinds": KINDS, "selectedKind": kind,
        "usage": "phoenix-ui review schema --json [--kind <kind>]",
        "notice": "Every example is a serde-checked wire shape, not evidence or a successful workflow input. EXAMPLE_ONLY strings and numeric/boolean values are placeholders. Obtain real cycle/round/cell/packet/revision values from actual begin, judge-pack or status responses; obtain file hashes, tested versions and provenance from actual evidence. No cycle is read or written. Operation validation still checks current state, evidence, paths and policy.",
        "fieldNotes": [
            "Examples expand nested objects and array elements, including optional/defaulted fields. Empty arrays are allowed by serde; each operation enforces its evidence requirements.",
            "Option fields may be omitted or null: TestedVersion.head/buildId, Finding.existingIssue, applied::Input.recordPath, LegacyMap.plan/recordPath, LegacyRound.targetFingerprint, LegacyScore.value, LegacyDisposition.disposition. Non-null string values and FileEvidence/LegacyFingerprint shapes are illustrated by their other occurrences.",
            "Defaulted fields: Decision.kind=direction, RuntimeCheck.reassessed=[], LegacyScore.value=null, LegacyDisposition.disposition=null.",
            "Decimal fields accept JSON numbers or numeric strings and serialize as canonical decimal strings. Scores/reasons are arrays of exactly four elements; dimension limits remain 3/3/3/1.",
            "AssessmentBasis also parses legacy text for compatibility, but judge add requires CurrentEvidence objects for every previous conclusion; 本轮未复核 is derived by the engine, not submitted by a reviewer.",
            "Adjudication.machineChecks/mainChecks reference RuntimeCheck JSON files by FileEvidence. Schema 1 retains Adjudication.applied, but finalize currently requires it to be empty; register actual dispositions through apply record. Applied page selfCheck contains CaptureInput, not a path.",
        ],
        "inputs": inputs,
        "variants": {
            "WorkingTree": [WorkingTree::Clean, dirty, WorkingTree::Unknown],
            "Origin": [Origin::Browser, Origin::Simulator, Origin::Native],
            "Observation": typed::<Vec<Observation>>(json!([missing["observation"], captured["observation"]]))?,
            "Conversion": [judgement::Conversion::ReviewerStructured, conversion],
            "AssessmentBasis": typed::<Vec<judgement::AssessmentBasis>>(json!([basis, "EXAMPLE_ONLY_LEGACY_TEXT_PARSER_COMPATIBILITY"]))?,
            "Severity": [Severity::P0, Severity::P1, Severity::P2, Severity::P3],
            "GapStatus": [GapStatus::Resolved, GapStatus::Partial, GapStatus::Present, GapStatus::Regressed, GapStatus::Unverified, GapStatus::NotApplicable, GapStatus::NotReviewed],
            "Disposition": [Disposition::Modified, Disposition::Declined, Disposition::AwaitingDecision, Disposition::Blocked, Disposition::Untreated],
            "DecisionKind": [finalize::DecisionKind::Direction, finalize::DecisionKind::WholeUiWaiver],
            "CheckKind": [finalize::CheckKind::Page, finalize::CheckKind::Source],
            "CheckStatus": [finalize::CheckStatus::Passed, finalize::CheckStatus::Failed, finalize::CheckStatus::Unavailable],
            "SelfCheck": typed::<Vec<applied::SelfCheck>>(json!([applied["items"][0]["selfCheck"], page_applied["items"][0]["selfCheck"]]))?,
            "RefactorScope": scopes,
            "Knowledge": [legacy::Knowledge::Known, legacy::Knowledge::Absent, legacy::Knowledge::Unknown],
            "RefactorHistory": [RefactorHistory::NotRecorded, RefactorHistory::Recorded, RefactorHistory::Unknown],
            "GapOrigin": [finalize::GapOrigin::Reviewer, finalize::GapOrigin::Runtime],
        },
    }))
}
