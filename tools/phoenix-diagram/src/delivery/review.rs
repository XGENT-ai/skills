use super::provenance::{self, Current};
use super::support::*;
use super::*;
use crate::browser::{self, BoundFile, BrowserReceipt, BrowserStatus};
use crate::check::FileIdentity;
use crate::output::{self, FileBinding};
use serde_json::json;

const REVIEW_SCOPE: [&str; 6] = [
    "diagram",
    "labels",
    "cards",
    "relationship-occlusion",
    "hierarchy",
    "controls",
];

pub(super) fn record(output: &Path, record_path: &Path, engine: &EngineIdentity) -> ReviewReceipt {
    record_observed(output, record_path, engine, || Ok(()))
}

pub(super) fn record_observed(
    output: &Path,
    record_path: &Path,
    engine: &EngineIdentity,
    before_summary: impl FnOnce() -> std::io::Result<()>,
) -> ReviewReceipt {
    let mut result = ReviewReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "review-record".into(),
        status: Status::Failed,
        diagnostics: vec![],
        delivery_receipt_id: None,
        browser_receipt_identity: None,
        automated_status: Status::Failed,
        visual_review: VisualReview {
            status: VisualStatus::RequiredPending,
            record: None,
        },
        delivery_status: "incomplete".into(),
        evidence: json!({}),
        exit: 1,
    };
    let run = (|| {
        let record_file =
            FileBinding::capture(record_path).map_err(|e| output_error("review-record", e))?;
        let record: VisualReviewRecord = serde_json::from_slice(record_file.bytes())
            .map_err(|e| fail("review-record", "review/record-format", e.to_string(), 1))?;
        let slot = output::capture_slot(output, &[&record_file])
            .map_err(|e| output_error("review-record", e))?;
        let current = provenance::current_locked(output, engine, &slot)?;
        if slot.previous().map(reference) != Some(reference(&current.artifact)) {
            return Err(fail(
                "review-record",
                "review/artifact-changed",
                "Output changed before the review lock was acquired",
                3,
            ));
        }
        let sidecars = paths(current.artifact.path(), None);
        let protected = [
            &current.candidate,
            &current.artifact,
            &current.delivery,
            &record_file,
        ];
        let summary_slot = slot
            .capture_related(&sidecars.summary, &protected)
            .map_err(|e| output_error("review-record", e))?;
        settled(&summary_slot, "review-record")?;
        let summary_file = summary_slot.previous().ok_or_else(|| {
            fail(
                "review-record",
                "review/summary-missing",
                "No current four-gate finalize summary exists",
                1,
            )
        })?;
        let mut summary: FinalizeReceipt = serde_json::from_slice(summary_file.bytes())
            .map_err(|e| fail("review-record", "review/summary-format", e.to_string(), 1))?;
        let browser_ref: BoundFile =
            serde_json::from_value(summary.evidence["browserReceipt"].clone())
                .map_err(|e| fail("review-record", "review/browser-binding", e.to_string(), 1))?;
        let browser_slot = slot
            .capture_related(
                &browser_ref.path,
                &[
                    &current.candidate,
                    &current.artifact,
                    &current.delivery,
                    summary_file,
                    &record_file,
                ],
            )
            .map_err(|e| output_error("review-record", e))?;
        settled(&browser_slot, "review-record")?;
        let browser_file = FileBinding::capture(&browser_ref.path)
            .map_err(|e| output_error("review-record", e))?;
        if browser_file.path() != browser_ref.path
            || FileIdentity::of(browser_file.bytes()) != browser_ref.identity
        {
            return Err(fail(
                "review-record",
                "review/browser-binding",
                "Sealed browser evidence no longer matches the summary",
                1,
            ));
        }
        let browser: BrowserReceipt = serde_json::from_slice(browser_file.bytes())
            .map_err(|e| fail("review-record", "review/browser-format", e.to_string(), 1))?;
        verify_summary(&summary, &current, &browser, &browser_ref)?;
        let expected_paths = paths(current.artifact.path(), browser_file.path().parent());
        if browser_file.path() != expected_paths.browser
            || summary.evidence["reviewPath"] != json!(expected_paths.review)
        {
            return Err(fail(
                "review-record",
                "review/evidence-path",
                "Summary does not name this output's sealed evidence paths",
                1,
            ));
        }
        result.delivery_receipt_id = Some(current.receipt.receipt_id.clone());
        result.browser_receipt_identity = Some(browser_ref.identity.clone());
        result.automated_status = Status::Pass;
        verify_record(&record, &current, &browser, &browser_ref)?;
        let mut screenshots = Vec::new();
        for screenshot in &browser.screenshots {
            let file = FileBinding::capture(&screenshot.path)
                .map_err(|e| output_error("review-record", e))?;
            if file.path() != screenshot.path
                || FileIdentity::of(file.bytes()) != screenshot.identity
                || !png_dimensions(file.bytes())
                    .is_some_and(|dimensions| dimensions == (screenshot.width, screenshot.height))
            {
                return Err(fail(
                    "review-record",
                    "review/screenshot-binding",
                    "A current screenshot is missing, changed, or not the declared PNG image",
                    1,
                ));
            }
            screenshots.push(file);
        }
        let mut protected = vec![
            &current.candidate,
            &current.artifact,
            &current.delivery,
            summary_file,
            &browser_file,
        ];
        protected.extend(screenshots.iter());
        if record_file.path() != expected_paths.review {
            protected.push(&record_file);
        }
        let review_slot = slot
            .capture_related(&expected_paths.review, &protected)
            .map_err(|e| output_error("review-record", e))?;
        settled(&review_slot, "review-record")?;
        let sealed = if let Some(existing) = review_slot.previous() {
            if existing.bytes() != record_file.bytes() {
                return Err(fail(
                    "review-record",
                    "review/record-conflict",
                    "The sealed review path already belongs to different record bytes",
                    3,
                ));
            }
            existing
                .verify()
                .map_err(|e| output_error("review-record", e))?;
            FileBinding::capture(existing.path()).map_err(|e| output_error("review-record", e))?
        } else {
            write(&review_slot, record_file.bytes(), &protected)?
        };
        result.evidence["visualReviewRecord"] = bound_json(&sealed);
        result.visual_review = VisualReview {
            status: record.verdict,
            record: Some(reference(&sealed).bound_file()),
        };
        result.delivery_status = if record.verdict == VisualStatus::Passed {
            "complete"
        } else {
            "incomplete"
        }
        .into();
        summary.visual_review = result.visual_review.clone();
        summary.delivery_status = result.delivery_status.clone();
        summary.evidence["visualReviewRecord"] = bound_json(&sealed);
        summary.diagnostics.retain(|d| {
            !matches!(
                d.code.as_str(),
                "review/visual-failed" | "review/visual-unavailable"
            )
        });
        if matches!(
            record.verdict,
            VisualStatus::Failed | VisualStatus::Unavailable
        ) {
            let code = if record.verdict == VisualStatus::Failed {
                "review/visual-failed"
            } else {
                "review/visual-unavailable"
            };
            let mut diagnostic =
                fail("visual-review", code, record.findings.join("; "), 0).diagnostic;
            diagnostic.severity = crate::diagnostic::Severity::Warning;
            summary.diagnostics.push(diagnostic);
        }
        // The record remains sealed if publishing this current pointer fails.
        let mut protected = vec![
            &current.candidate,
            &current.artifact,
            &current.delivery,
            &browser_file,
            &sealed,
        ];
        protected.extend(screenshots.iter());
        if record_file.path() != sealed.path() {
            protected.push(&record_file);
        }
        current.verify()?;
        verify(&protected, "review-record")?;
        slot.verify_guard()
            .map_err(|e| output_error("review-record", e))?;
        let bytes = json_bytes(&summary)?;
        let updated = if summary_file.bytes() == bytes {
            summary_file
                .verify()
                .map_err(|e| output_error("review-record", e))?;
            FileBinding::capture(summary_file.path())
                .map_err(|e| output_error("review-record", e))?
        } else {
            before_summary().map_err(|error| {
                fail(
                    "review-record",
                    "review/summary-write",
                    error.to_string(),
                    4,
                )
            })?;
            write(&summary_slot, &bytes, &protected)?
        };
        current.verify()?;
        verify(&protected, "review-record")?;
        settled(&summary_slot, "review-record")?;
        settled(&browser_slot, "review-record")?;
        settled(&review_slot, "review-record")?;
        result.evidence["summaryReceipt"] = bound_json(&updated);
        result.status = Status::Pass;
        result.exit = 0;
        Ok(())
    })();
    if let Err(failure) = run {
        result.status = Status::Failed;
        result.delivery_status = "incomplete".into();
        result.diagnostics.push(failure.diagnostic);
        result.exit = failure.exit;
    }
    result
}

fn verify_summary(
    summary: &FinalizeReceipt,
    current: &Current,
    browser: &BrowserReceipt,
    browser_ref: &BoundFile,
) -> Result<()> {
    if !header(
        summary.schema_version,
        &summary.tool,
        &summary.tool_version,
        &summary.command,
        "finalize",
    ) || summary.status != Status::Pass
        || summary.automated_status != Status::Pass
        || !summary.produced_by_this_run
        || summary.candidate.as_ref() != Some(&reference(&current.candidate))
        || summary.artifact.as_ref() != Some(&reference(&current.artifact))
        || summary.config_sha256 != current.receipt.config_sha256
        || summary.engine != current.receipt.engine
        || summary.delivery_receipt_id.as_deref() != Some(current.receipt.receipt_id.as_str())
        || summary.browser_receipt_identity.as_ref() != Some(&browser_ref.identity)
        || summary.stages.len() != 4
    {
        return Err(fail(
            "review-record",
            "review/summary-binding",
            "Summary is not the current four-gate delivery",
            1,
        ));
    }
    for (stage, name) in
        summary
            .stages
            .iter()
            .zip(["validate", "deliver", "check", "browser-check"])
    {
        if stage.name != name || stage.status != Status::Pass || stage.receipt.is_none() {
            return Err(fail(
                "review-record",
                "review/stage-scope",
                "All four original named gates must have actual passing receipts",
                1,
            ));
        }
    }
    let validation: ValidationReceipt = read_stage(&summary.stages[0])?;
    let delivery: DeliveryReceipt = read_stage(&summary.stages[1])?;
    let check: CheckReceipt = read_stage(&summary.stages[2])?;
    let stage_browser: BrowserReceipt = read_stage(&summary.stages[3])?;
    if !header(
        validation.schema_version,
        &validation.tool,
        &validation.tool_version,
        &validation.command,
        "validate",
    ) || validation.status != Status::Pass
        || validation.candidate.as_ref() != Some(&reference(&current.candidate))
        || validation.artifact.as_ref() != Some(&FileIdentity::of(current.artifact.bytes()))
        || validation.config_sha256 != current.receipt.config_sha256
        || validation.config != current.receipt.config
        || validation.engine != current.receipt.engine
        || validation.checks != current.checked.checks
        || validation.composition != current.checked.composition
        || validation.compatibility != current.receipt.compatibility
        || delivery != current.receipt
        || !header(
            check.schema_version,
            &check.tool,
            &check.tool_version,
            &check.command,
            "check",
        )
        || check.status != Status::Pass
        || check.provenance != "current"
        || check.scope != "strict-provenance"
        || check.candidate != current.receipt.candidate
        || check.artifact != current.receipt.artifact
        || check.engine != current.receipt.engine
        || check.config_sha256 != current.receipt.config_sha256
        || check.delivery_receipt_id.as_deref() != Some(current.receipt.receipt_id.as_str())
        || check.delivery_receipt.as_ref() != Some(&reference(&current.delivery).bound_file())
        || check.checks != current.checked.checks
        || check.composition != current.checked.composition
        || stage_browser != *browser
    {
        return Err(fail("review-record","review/stage-binding","Original stage evidence is missing scope or belongs to another input, artifact, or receipt",1));
    }
    if !header(
        browser.schema_version,
        &browser.tool,
        &browser.tool_version,
        &browser.command,
        "browser-check",
    ) || browser.status != BrowserStatus::Pass
        || browser.artifact_path != current.artifact.path()
        || browser.artifact != FileIdentity::of(current.artifact.bytes())
        || browser.candidate != reference(&current.candidate).bound_file()
        || browser.config_sha256
            != current
                .receipt
                .config_sha256
                .clone()
                .expect("verified config")
        || browser.engine != json!(current.receipt.engine)
        || browser.delivery_receipt_id != current.receipt.receipt_id
        || !browser::validate_receipt_coverage(browser, true)
    {
        return Err(fail("review-record","review/browser-scope","Browser evidence does not cover the current artifact and required actual measurements and screenshots",1));
    }
    Ok(())
}

fn read_stage<T: serde::de::DeserializeOwned>(stage: &Stage) -> Result<T> {
    serde_json::from_value(stage.receipt.clone().expect("verified present receipt"))
        .map_err(|e| fail("review-record", "review/stage-format", e.to_string(), 1))
}

fn verify_record(
    record: &VisualReviewRecord,
    current: &Current,
    browser: &BrowserReceipt,
    browser_ref: &BoundFile,
) -> Result<()> {
    if !header(
        record.schema_version,
        &record.tool,
        &record.tool_version,
        &record.command,
        "review-record",
    ) || record.status != Status::Pass
        || record.artifact != reference(&current.artifact).bound_file()
        || record.candidate != reference(&current.candidate).bound_file()
        || record.config_sha256
            != current
                .receipt
                .config_sha256
                .clone()
                .expect("verified config")
        || record.engine != current.receipt.engine
        || record.delivery_receipt_id != current.receipt.receipt_id
        || record.browser_receipt_identity != browser_ref.identity
        || record.reviewer.trim().is_empty()
        || record.verdict == VisualStatus::RequiredPending
        || record.screenshot_identities.len() != 4
        || record.scope.len() != REVIEW_SCOPE.len()
        || !REVIEW_SCOPE.iter().all(|name| {
            record
                .scope
                .iter()
                .filter(|scope| scope.as_str() == *name)
                .count()
                == 1
        })
    {
        return Err(fail("review-record","review/record-binding","Review must identify this delivery, browser evidence, reviewer, and complete observation scope",1));
    }
    for screenshot in &browser.screenshots {
        let matching: Vec<_> = record
            .screenshot_identities
            .iter()
            .filter(|seen| {
                seen.file.path == screenshot.path
                    && seen.file.identity == screenshot.identity
                    && seen.viewport == screenshot.viewport
                    && seen.theme == screenshot.theme
            })
            .collect();
        if matching.len() != 1
            || (matching[0].observed && matching[0].observations.trim().is_empty())
            || (record.verdict == VisualStatus::Passed && !matching[0].observed)
        {
            return Err(fail("review-record","review/observation-scope","Review observations must bind all four current dark/light screenshots and their viewports",1));
        }
    }
    if record.verdict != VisualStatus::Passed
        && (record.findings.is_empty()
            || record
                .findings
                .iter()
                .any(|finding| finding.trim().is_empty()))
    {
        return Err(fail(
            "review-record",
            "review/findings",
            "Failed or unavailable review requires its actual finding or limitation",
            1,
        ));
    }
    Ok(())
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ))
}
