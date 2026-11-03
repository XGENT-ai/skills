use super::support::*;
use super::*;
use crate::check::FileIdentity;

pub(super) fn record(output: &Path, record_path: &Path) -> ReviewReceipt {
    let summary_path = summary_path(output);
    let mut result = ReviewReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "review-record".into(),
        status: Status::Failed,
        diagnostics: vec![],
        summary: summary_path.clone(),
        automated_status: Status::Failed,
        visual_review: VisualReview::pending(),
        delivery_status: "incomplete".into(),
        exit: 1,
    };
    let run = (|| -> Result<()> {
        let mut summary: FinalizeReceipt = serde_json::from_slice(&read(
            "review-record",
            "review/summary-missing",
            &summary_path,
        )?)
        .map_err(|e| fail("review-record", "review/summary-format", e.to_string(), 1))?;
        result.automated_status = summary.automated_status;
        if summary.automated_status != Status::Pass {
            return Err(fail(
                "review-record",
                "review/automated-gates",
                "Run finalize until its automated gates pass before recording a review",
                1,
            ));
        }
        let html = read("review-record", "review/artifact-changed", output)?;
        if summary.artifact.as_ref().map(|a| &a.identity) != Some(&FileIdentity::of(&html)) {
            return Err(fail(
                "review-record",
                "review/artifact-changed",
                "The HTML differs from the finalized one; run finalize again",
                1,
            ));
        }
        let record = read_record(record_path)
            .map_err(|message| fail("review-record", "review/record-format", message, 2))?;
        validate(&record, &summary.screenshots)
            .map_err(|message| fail("review-record", "review/record", message, 2))?;
        summary.visual_review = VisualReview {
            status: record.verdict,
            record: Some(record),
        };
        summary.delivery_status = delivery_status(summary.visual_review.status).into();
        write("review-record", &summary_path, &json_bytes(&summary))?;
        result.visual_review = summary.visual_review;
        result.delivery_status = summary.delivery_status;
        result.status = Status::Pass;
        result.exit = 0;
        Ok(())
    })();
    if let Err(failure) = run {
        result.diagnostics.push(failure.diagnostic);
        result.exit = failure.exit;
    }
    result
}

pub(crate) fn delivery_status(review: VisualStatus) -> &'static str {
    if review == VisualStatus::Passed {
        "complete"
    } else {
        "incomplete"
    }
}

pub(crate) fn read_record(path: &Path) -> std::result::Result<ReviewRecord, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// A record names every current screenshot once. A pass needs an observation
/// for each image; a failed or unavailable review needs its findings.
pub(crate) fn validate(
    record: &ReviewRecord,
    screenshots: &[std::path::PathBuf],
) -> std::result::Result<(), &'static str> {
    if record.reviewer.trim().is_empty() {
        return Err("Name the reviewer");
    }
    if record.verdict == VisualStatus::RequiredPending {
        return Err("Choose passed, failed or unavailable");
    }
    let mut listed: Vec<_> = record.screenshots.iter().map(|s| &s.path).collect();
    listed.sort();
    let mut expected: Vec<_> = screenshots.iter().collect();
    expected.sort();
    if listed != expected {
        return Err("List each current screenshot path from the summary exactly once");
    }
    if record.verdict == VisualStatus::Passed {
        if record
            .screenshots
            .iter()
            .any(|s| s.observations.trim().is_empty())
        {
            return Err("A passed review records what was seen in every screenshot");
        }
    } else if record.findings.iter().all(|f| f.trim().is_empty()) {
        return Err("A failed or unavailable review states its findings or limitation");
    }
    Ok(())
}
