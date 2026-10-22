use super::test_support::Fixture;
use super::*;
use crate::check::FileIdentity;
use serde_json::{json, Value};
use std::fs;

#[test]
fn record_closes_the_same_protocol_fixture_identity_and_is_idempotent() {
    let fixture = Fixture::new();
    let (summary, protocol) = fixture.finalize();
    let record = fixture.record(&summary);
    let command_count = protocol.commands.lock().unwrap().len();
    let accepted = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(accepted.status, Status::Pass, "{:?}", accepted.diagnostics);
    assert_eq!(accepted.delivery_status, "complete");
    assert_eq!(accepted.delivery_receipt_id, summary.delivery_receipt_id);
    assert_eq!(
        accepted.browser_receipt_identity,
        summary.browser_receipt_identity
    );
    let first = accepted.evidence["summaryReceipt"].clone();
    let again = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(again.status, Status::Pass, "{:?}", again.diagnostics);
    assert_eq!(again.evidence["summaryReceipt"], first);
    assert_eq!(
        protocol.commands.lock().unwrap().len(),
        command_count,
        "Review must not invoke the browser again"
    );
    assert_eq!(
        check(&fixture.output, true, &fixture.engine).delivery_receipt_id,
        summary.delivery_receipt_id
    );
    let stored: FinalizeReceipt =
        serde_json::from_slice(&fs::read(sidecar_paths(&fixture.output, None).summary).unwrap())
            .unwrap();
    assert_eq!(stored.stages, summary.stages);
    assert_eq!(stored.visual_review.status, VisualStatus::Passed);
    assert_eq!(stored.delivery_status, "complete");
}

#[test]
fn a_sealed_record_survives_summary_io_failure_and_the_same_record_can_retry() {
    let fixture = Fixture::new();
    let (summary, protocol) = fixture.finalize();
    let record = fixture.record(&summary);
    let failed = super::review::record_observed(&fixture.output, &record, &fixture.engine, || {
        Err(std::io::Error::other(
            "injected summary publication I/O failure",
        ))
    });
    assert_eq!(failed.status, Status::Failed);
    assert_eq!(failed.exit_code(), 4);
    assert_eq!(failed.delivery_status, "incomplete");
    let sealed = sidecar_paths(&fixture.output, None).review;
    assert_eq!(fs::read(&sealed).unwrap(), fs::read(&record).unwrap());
    assert_eq!(
        failed.evidence["visualReviewRecord"]["identity"],
        json!(FileIdentity::of(&fs::read(&record).unwrap()))
    );
    let stored: FinalizeReceipt =
        serde_json::from_slice(&fs::read(sidecar_paths(&fixture.output, None).summary).unwrap())
            .unwrap();
    assert_eq!(stored.visual_review.status, VisualStatus::RequiredPending);
    assert_eq!(stored.delivery_status, "incomplete");
    let command_count = protocol.commands.lock().unwrap().len();
    let retried = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(retried.status, Status::Pass, "{:?}", retried.diagnostics);
    assert_eq!(retried.delivery_status, "complete");
    assert_eq!(retried.delivery_receipt_id, summary.delivery_receipt_id);
    assert_eq!(
        retried.browser_receipt_identity,
        summary.browser_receipt_identity
    );
    assert_eq!(protocol.commands.lock().unwrap().len(), command_count);
}

#[test]
fn review_rejects_each_changed_evidence_binding_and_an_incomplete_observation_scope() {
    let fixture = Fixture::new();
    let (summary, _protocol) = fixture.finalize();
    let record = fixture.record(&summary);
    let original = fs::read(&record).unwrap();
    let mut mutations = Vec::new();
    for (field, value) in [
        (
            "deliveryReceiptId",
            json!("11111111-1111-4111-8111-111111111111"),
        ),
        ("configSha256", json!("f".repeat(64))),
        ("reviewer", json!("")),
        ("scope", json!(["diagram", "labels"])),
    ] {
        let mut modified: Value = serde_json::from_slice(&original).unwrap();
        modified[field] = value;
        mutations.push(modified);
    }
    let mut skipped: Value = serde_json::from_slice(&original).unwrap();
    skipped["screenshotIdentities"][0]["observed"] = false.into();
    mutations.push(skipped);
    let mut duplicate: Value = serde_json::from_slice(&original).unwrap();
    duplicate["screenshotIdentities"][1] = duplicate["screenshotIdentities"][0].clone();
    mutations.push(duplicate);
    for modified in mutations {
        fs::write(&record, serde_json::to_vec_pretty(&modified).unwrap()).unwrap();
        let rejected = record_review(&fixture.output, &record, &fixture.engine);
        assert_eq!(
            rejected.status,
            Status::Failed,
            "A changed review binding or omitted view cannot pass: {modified}"
        );
        assert_eq!(rejected.delivery_status, "incomplete");
        assert!(!sidecar_paths(&fixture.output, None).review.exists());
    }
    fs::write(&record, &original).unwrap();
    let mut files = vec![
        fixture.candidate.clone(),
        fixture.output.clone(),
        sidecar_paths(&fixture.output, None).delivery,
        Path::new(summary.evidence["browserReceipt"]["path"].as_str().unwrap()).to_owned(),
    ];
    let record_json: Value = serde_json::from_slice(&original).unwrap();
    files.push(
        Path::new(
            record_json["screenshotIdentities"][0]["file"]["path"]
                .as_str()
                .unwrap(),
        )
        .to_owned(),
    );
    for path in files {
        let before = fs::read(&path).unwrap();
        fs::write(&path, b"changed binding").unwrap();
        let rejected = record_review(&fixture.output, &record, &fixture.engine);
        assert_eq!(
            rejected.status,
            Status::Failed,
            "Changed file {} must not be accepted",
            path.display()
        );
        assert_eq!(rejected.delivery_status, "incomplete");
        fs::write(&path, before).unwrap();
        assert!(!sidecar_paths(&fixture.output, None).review.exists());
    }
    assert_eq!(
        record_review(&fixture.output, &record, &fixture.engine).status,
        Status::Pass
    );
}

#[test]
fn review_accepts_a_failed_verdict_without_claiming_complete_and_preserves_sealed_records() {
    let fixture = Fixture::new();
    let (summary, _protocol) = fixture.finalize();
    let record = fixture.record(&summary);
    let mut document: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    document["verdict"] = json!("failed");
    document["findings"] = json!(["Fixture defect; no real image observation"]);
    fs::write(&record, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    let accepted = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(accepted.status, Status::Pass, "{:?}", accepted.diagnostics);
    assert_eq!(accepted.exit_code(), 0);
    assert_eq!(accepted.delivery_status, "incomplete");
    assert_eq!(accepted.visual_review.status, VisualStatus::Failed);
    let sealed = fs::read(sidecar_paths(&fixture.output, None).review).unwrap();
    document["verdict"] = json!("passed");
    document["findings"] = json!([]);
    fs::write(&record, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    let conflict = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(conflict.exit_code(), 3);
    assert_eq!(conflict.delivery_status, "incomplete");
    assert_eq!(
        fs::read(sidecar_paths(&fixture.output, None).review).unwrap(),
        sealed
    );
}

#[test]
fn a_forged_browser_scope_or_not_run_stage_cannot_be_promoted_by_review() {
    let fixture = Fixture::new();
    let (summary, _protocol) = fixture.finalize();
    let record = fixture.record(&summary);
    let path = sidecar_paths(&fixture.output, None).summary;
    let original = fs::read(&path).unwrap();
    let mut forged: Value = serde_json::from_slice(&original).unwrap();
    forged["stages"][2]["status"] = json!("not-run");
    fs::write(&path, serde_json::to_vec_pretty(&forged).unwrap()).unwrap();
    let rejected = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(rejected.status, Status::Failed);
    assert!(rejected
        .diagnostics
        .iter()
        .any(|d| d.code == "review/stage-scope"));
    fs::write(&path, &original).unwrap();
    let browser_path = Path::new(summary.evidence["browserReceipt"]["path"].as_str().unwrap());
    let mut browser: Value = serde_json::from_slice(&fs::read(browser_path).unwrap()).unwrap();
    browser["coverage"][0]["checks"]
        .as_array_mut()
        .unwrap()
        .pop();
    let bytes = serde_json::to_vec_pretty(&browser).unwrap();
    fs::write(browser_path, &bytes).unwrap();
    let identity = FileIdentity::of(&bytes);
    let mut forged: Value = serde_json::from_slice(&original).unwrap();
    forged["stages"][3]["receipt"] = browser;
    forged["browserReceiptIdentity"] = json!(identity);
    forged["evidence"]["browserReceipt"]["identity"] = json!(identity);
    fs::write(&path, serde_json::to_vec_pretty(&forged).unwrap()).unwrap();
    let rejected = record_review(&fixture.output, &record, &fixture.engine);
    assert_eq!(rejected.status, Status::Failed);
    assert!(rejected
        .diagnostics
        .iter()
        .any(|d| d.code == "review/browser-scope"));
    assert!(!sidecar_paths(&fixture.output, None).review.exists());
}
