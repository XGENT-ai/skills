use super::*;
use crate::{
    browser::{BrowserOptions, BrowserStatus},
    check::FileIdentity,
    delivery::{review, ReviewReceipt, Status, VisualReview},
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The comparison summary named by the HTML, beside it.
pub(super) fn summary(output: &Path) -> Result<(PathBuf, Vec<u8>, CompareReceipt), Failure> {
    let html = std::fs::read(output)
        .map_err(|e| fail("compare/artifact", format!("{}: {e}", output.display()), 1))?;
    let parsed = render::parse(&html).map_err(|diagnostics| Failure {
        diagnostics,
        exit: 1,
    })?;
    let path = output.with_file_name(&parsed.receipt_filename);
    let receipt: CompareReceipt = std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
        .map_err(|e| fail("compare/receipt", format!("{}: {e}", path.display()), 1))?;
    if receipt.artifact.as_ref().map(|a| &a.identity) != Some(&FileIdentity::of(&html)) {
        return Err(fail(
            "compare/artifact-changed",
            "The comparison HTML differs from its summary; run compare again",
            1,
        ));
    }
    Ok((path, html, receipt))
}

/// Measure every view and record the result in the comparison summary.
pub fn browser_check(output: &Path, options: &BrowserOptions, visual: bool) -> (Value, u8) {
    let (path, _, mut receipt) = match summary(output) {
        Ok(found) => found,
        Err(failure) => {
            return failed(
                if visual {
                    "visual-check"
                } else {
                    "browser-check"
                },
                failure,
            )
        }
    };
    let mut options = options.clone();
    options.capture = visual;
    let ir = receipt
        .comparison_ir
        .as_ref()
        .map(|ir| ir.identity.clone())
        .unwrap_or_else(|| FileIdentity::of(b""));
    let browser = crate::browser::comparison::inspect(output, &ir, &options);
    let exit = browser.exit_code();
    receipt.browser_status = match browser.status {
        BrowserStatus::Pass => Status::Pass,
        BrowserStatus::Failed => Status::Failed,
        BrowserStatus::Unavailable => Status::Unavailable,
    };
    receipt.screenshots = browser
        .screenshots
        .iter()
        .map(|s| s.screenshot.path.clone())
        .collect();
    receipt.visual_review = VisualReview::pending();
    receipt.delivery_status = "incomplete".into();
    let mut value = serde_json::to_value(&browser).expect("browser receipt serializes");
    receipt.browser = Some(browser);
    if let Err(failure) = write(&path, &json_bytes(&receipt)) {
        return failed(
            &receipt.browser.as_ref().expect("just set").command,
            failure,
        );
    }
    value["summary"] = json!(path);
    (value, exit)
}

fn failed(command: &str, failure: Failure) -> (Value, u8) {
    (
        json!({"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":crate::TOOL_VERSION,"command":command,"kind":"architecture-comparison-browser","status":"failed","diagnostics":failure.diagnostics}),
        failure.exit,
    )
}

pub fn record_review(output: &Path, record_path: &Path) -> ReviewReceipt {
    let mut result = ReviewReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "review-record".into(),
        status: Status::Failed,
        diagnostics: vec![],
        summary: PathBuf::new(),
        automated_status: Status::Failed,
        visual_review: VisualReview::pending(),
        delivery_status: "incomplete".into(),
        exit: 1,
    };
    let run = (|| {
        let (path, _, mut receipt) = summary(output)?;
        result.summary = path.clone();
        result.automated_status = receipt.browser_status;
        if receipt.browser_status != Status::Pass || receipt.screenshots.is_empty() {
            return Err(fail(
                "review/automated-gates",
                "Run visual-check until it passes before recording a review",
                1,
            ));
        }
        let record = review::read_record(record_path)
            .map_err(|message| fail("review/record-format", message, 2))?;
        review::validate(&record, &receipt.screenshots)
            .map_err(|message| fail("review/record", message, 2))?;
        receipt.visual_review = VisualReview {
            status: record.verdict,
            record: Some(record),
        };
        receipt.delivery_status = review::delivery_status(receipt.visual_review.status).into();
        write(&path, &json_bytes(&receipt))?;
        result.visual_review = receipt.visual_review;
        result.delivery_status = receipt.delivery_status;
        result.status = Status::Pass;
        result.exit = 0;
        Ok(())
    })();
    if let Err(mut failure) = run {
        for diagnostic in &mut failure.diagnostics {
            diagnostic.stage = "review-record".into();
        }
        result.diagnostics = failure.diagnostics;
        result.exit = failure.exit;
    }
    result
}
