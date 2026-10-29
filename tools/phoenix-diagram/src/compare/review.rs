use super::*;
use crate::{
    browser::{self, BoundFile, BrowserOptions, BrowserStatus},
    check::FileIdentity,
    delivery::{EngineIdentity, Stage, Status, VisualReview, VisualStatus},
    output::{self, FileBinding, OutputSlot},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

fn bound(file: &FileBinding) -> BoundFile {
    BoundFile {
        path: file.path().to_owned(),
        identity: FileIdentity::of(file.bytes()),
    }
}
fn comparison_stem(output: &Path) -> String {
    let name = output.file_name().unwrap_or_default().to_string_lossy();
    if name.len() <= 180 && name.encode_utf16().count() <= 180 {
        name.into_owned()
    } else {
        format!("comparison-{}", crate::input::sha256(name.as_bytes()))
    }
}
pub fn paths(output: &Path, out_dir: Option<&Path>) -> ComparisonPaths {
    let parent = output.parent().unwrap_or(Path::new("."));
    let stem = comparison_stem(output);
    ComparisonPaths {
        summary: parent.join(format!("{stem}.compare-summary.json")),
        browser: out_dir
            .unwrap_or(parent)
            .join(format!("{stem}.compare-browser.json")),
        visual: out_dir
            .unwrap_or(parent)
            .join(format!("{stem}.compare-visual.json")),
        review: parent.join(format!("{stem}.compare-review.json")),
    }
}
fn settled(slot: &OutputSlot) -> Result<(), Failure> {
    if !slot.pending_publications().map_err(from_output)?.is_empty() {
        return Err(fail(
            "output/recovery-required",
            "A pending publication must be recovered before comparison evidence is updated",
            3,
        ));
    }
    Ok(())
}
fn write(
    slot: &OutputSlot,
    value: &impl Serialize,
    protected: &[&FileBinding],
) -> Result<FileBinding, Failure> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|e| fail("compare/json", e.to_string(), 4))?;
    bytes.push(b'\n');
    write_bytes(slot, &bytes, protected)
}
fn write_bytes(
    slot: &OutputSlot,
    bytes: &[u8],
    protected: &[&FileBinding],
) -> Result<FileBinding, Failure> {
    settled(slot)?;
    let publication =
        output::publish_bound_with_mode(slot, bytes, 0o600, protected).map_err(from_output)?;
    if !publication.cleanup_complete {
        return Err(fail(
            "output/recovery-required",
            "Comparison evidence publication requires cleanup",
            3,
        ));
    }
    let file = FileBinding::capture(&publication.path).map_err(from_output)?;
    if file.identity() != &publication.identity || file.bytes() != bytes {
        return Err(fail(
            "compare/evidence-changed",
            "Comparison evidence changed while it was published",
            3,
        ));
    }
    Ok(file)
}
fn owned_summary(slot: &OutputSlot) -> Result<(), Failure> {
    if let Some(previous) = slot.previous() {
        let summary: ComparisonSummary =
            serde_json::from_slice(previous.bytes()).map_err(|_| {
                fail(
                    "compare/summary-conflict",
                    "An unknown file occupies the comparison summary path",
                    3,
                )
            })?;
        if summary.schema_version != 1
            || summary.tool != "phoenix-diagram"
            || summary.tool_version != crate::TOOL_VERSION
            || summary.kind != "architecture-comparison-summary"
            || summary.command != "visual-check"
        {
            return Err(fail(
                "compare/summary-conflict",
                "The existing file is not a supported comparison summary",
                3,
            ));
        }
    }
    Ok(())
}
fn owned_browser(slot: &OutputSlot) -> Result<(), Failure> {
    if let Some(previous) = slot.previous() {
        let receipt: ComparisonBrowserReceipt =
            serde_json::from_slice(previous.bytes()).map_err(|_| {
                fail(
                    "compare/browser-conflict",
                    "An unknown file occupies the comparison browser receipt path",
                    3,
                )
            })?;
        if receipt.schema_version != 1
            || receipt.tool != "phoenix-diagram"
            || receipt.tool_version != crate::TOOL_VERSION
            || receipt.kind != "architecture-comparison-browser"
            || !matches!(receipt.command.as_str(), "browser-check" | "visual-check")
        {
            return Err(fail(
                "compare/browser-conflict",
                "The existing file is not comparison browser evidence",
                3,
            ));
        }
    }
    Ok(())
}
fn check_receipt(current: &verification::Current) -> ComparisonCheckReceipt {
    ComparisonCheckReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "check".into(),
        kind: "architecture-comparison-check".into(),
        status: Status::Pass,
        diagnostics: vec![],
        strict_provenance: true,
        comparison: Some(current.binding.clone()),
        checks: current.checks.clone(),
        exit: 0,
    }
}
fn stage_status(status: BrowserStatus) -> Status {
    match status {
        BrowserStatus::Pass => Status::Pass,
        BrowserStatus::Failed => Status::Failed,
        BrowserStatus::Unavailable => Status::Unavailable,
    }
}
fn capture_bound(expected: &BoundFile) -> Result<FileBinding, Failure> {
    let file = FileBinding::capture(&expected.path).map_err(from_output)?;
    if bound(&file) != *expected {
        return Err(fail(
            "compare/evidence-changed",
            "Comparison evidence no longer has its recorded bytes",
            3,
        ));
    }
    Ok(file)
}
fn failed(command: &str, error: Failure) -> (Value, u8) {
    (
        json!({"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":crate::TOOL_VERSION,"kind":"architecture-comparison-browser","command":command,"status":"failed","diagnostics":error.diagnostics,"evidence":{}}),
        error.exit,
    )
}

pub fn browser_check(
    output: &Path,
    engine: &EngineIdentity,
    options: &BrowserOptions,
    visual: bool,
) -> (Value, u8) {
    let command = if visual {
        "visual-check"
    } else {
        "browser-check"
    };
    let run = (|| {
        let current = verification::current(output, engine)?;
        let protected = current.files.iter().collect::<Vec<_>>();
        let guard = output::capture_slot(output, &protected[1..]).map_err(from_output)?;
        settled(&guard)?;
        if guard.previous().map(bound) != Some(current.binding.artifact.clone()) {
            return Err(fail(
                "compare/artifact-changed",
                "Comparison changed before browser inspection",
                3,
            ));
        }
        let locations = paths(current.files[0].path(), options.out_dir.as_deref());
        let summary_slot = if visual {
            let slot = guard
                .capture_related(&locations.summary, &protected)
                .map_err(from_output)?;
            settled(&slot)?;
            owned_summary(&slot)?;
            Some(slot)
        } else {
            None
        };
        let mut options = options.clone();
        options.capture = visual;
        let mut browser = browser::comparison::inspect(&current.binding, &options);
        browser.command = command.into();
        current.verify()?;
        let path = if visual {
            &locations.visual
        } else {
            &locations.browser
        };
        let browser_slot = guard
            .capture_related(path, &protected)
            .map_err(from_output)?;
        settled(&browser_slot)?;
        owned_browser(&browser_slot)?;
        let mut images = vec![];
        for shot in &browser.screenshots {
            images.push(capture_bound(&BoundFile {
                path: shot.screenshot.path.clone(),
                identity: shot.screenshot.identity.clone(),
            })?);
        }
        let mut all_protected = protected.clone();
        all_protected.extend(images.iter());
        let browser_file = write(&browser_slot, &browser, &all_protected)?;
        let browser_ref = bound(&browser_file);
        let mut evidence = json!({"browserReceipt":browser_ref});
        if let Some(summary_slot) = summary_slot.as_ref() {
            let check = check_receipt(&current);
            let measured = stage_status(browser.status);
            let automated = if measured == Status::Pass
                && browser::comparison::validate_receipt_coverage(&browser, true)
            {
                Status::Pass
            } else if measured == Status::Unavailable {
                Status::Unavailable
            } else {
                Status::Failed
            };
            let summary = ComparisonSummary {
                schema_version: 1,
                tool: "phoenix-diagram".into(),
                tool_version: crate::TOOL_VERSION.into(),
                kind: "architecture-comparison-summary".into(),
                command: "visual-check".into(),
                status: automated,
                diagnostics: browser.diagnostics.clone(),
                comparison: current.binding.clone(),
                stages: vec![
                    Stage {
                        name: "check".into(),
                        status: Status::Pass,
                        receipt: Some(json!(check)),
                    },
                    Stage {
                        name: "visual-check".into(),
                        status: measured,
                        receipt: Some(json!(browser)),
                    },
                ],
                automated_status: automated,
                visual_review: VisualReview {
                    status: VisualStatus::RequiredPending,
                    record: None,
                },
                delivery_status: "incomplete".into(),
                evidence: json!({"browserReceipt":browser_ref}),
            };
            all_protected.push(&browser_file);
            let written = write(summary_slot, &summary, &all_protected)?;
            evidence["summaryReceipt"] = json!(bound(&written));
        }
        current.verify()?;
        let exit = browser.exit_code();
        let mut value = json!(browser);
        value["evidence"] = evidence;
        Ok((value, exit))
    })();
    match run {
        Ok(value) => value,
        Err(error) => failed(command, error),
    }
}
fn review_empty() -> ComparisonReviewReceipt {
    ComparisonReviewReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "review-record".into(),
        kind: "architecture-comparison-review-receipt".into(),
        status: Status::Failed,
        diagnostics: vec![],
        comparison: None,
        automated_status: Status::NotRun,
        visual_review: VisualReview {
            status: VisualStatus::RequiredPending,
            record: None,
        },
        delivery_status: "incomplete".into(),
        evidence: json!({}),
        exit: 1,
    }
}
fn validate_observations(
    record: &ComparisonVisualReviewRecord,
    browser: &ComparisonBrowserReceipt,
) -> Result<Vec<FileBinding>, Failure> {
    let scope = record
        .scope
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if record.scope.len() != 6
        || scope
            != BTreeSet::from([
                "diagram",
                "labels",
                "cards",
                "relationship-occlusion",
                "hierarchy",
                "controls",
            ])
    {
        return Err(fail(
            "compare/review-scope",
            "Review must cover diagram, labels, cards, relationships, hierarchy and controls",
            1,
        ));
    }
    if record.reviewer.trim().is_empty() {
        return Err(fail(
            "compare/review-observations",
            "A sealed review must identify its actual reviewer",
            1,
        ));
    }
    if !matches!(
        record.verdict,
        VisualStatus::Passed | VisualStatus::Failed | VisualStatus::Unavailable
    ) {
        return Err(fail(
            "compare/review-verdict",
            "A review record must state passed, failed or unavailable",
            1,
        ));
    }
    if record.screenshot_identities.len() != 12 || browser.screenshots.len() != 12 {
        return Err(fail(
            "compare/review-screenshots",
            "Review must identify all twelve current screenshots",
            1,
        ));
    }
    if record.verdict == VisualStatus::Passed
        && (record.reviewer.trim().is_empty()
            || !record.findings.is_empty()
            || record
                .screenshot_identities
                .iter()
                .any(|s| !s.observed || s.observations.trim().is_empty()))
    {
        return Err(fail("compare/review-observations","Passed requires a named reviewer, actual observations for every screenshot, and no findings",1));
    }
    if record.verdict != VisualStatus::Passed
        && (record.findings.is_empty() || record.findings.iter().any(|f| f.trim().is_empty()))
    {
        return Err(fail(
            "compare/review-observations",
            "Failed or unavailable review must explain findings or limitations",
            1,
        ));
    }
    let mut seen = BTreeSet::new();
    let mut files = vec![];
    for observed in &record.screenshot_identities {
        if observed.observed && observed.observations.trim().is_empty() {
            return Err(fail(
                "compare/review-observations",
                "An observed screenshot requires its actual observations",
                1,
            ));
        }
        let key = (
            observed.view_id.clone(),
            observed.viewport.width,
            observed.viewport.height,
            format!("{:?}", observed.theme),
        );
        if !seen.insert(key) {
            return Err(fail(
                "compare/review-screenshots",
                "Review repeats a view, viewport and theme",
                1,
            ));
        }
        if !browser.screenshots.iter().any(|shot| {
            shot.view_id == observed.view_id
                && shot.screenshot.viewport == observed.viewport
                && shot.screenshot.theme == observed.theme
                && shot.screenshot.path == observed.file.path
                && shot.screenshot.identity == observed.file.identity
        }) {
            return Err(fail(
                "compare/review-screenshots",
                "Review does not match the current three-view screenshot evidence",
                1,
            ));
        }
        files.push(capture_bound(&observed.file)?);
    }
    Ok(files)
}
pub fn record_review(
    output: &Path,
    draft: &Path,
    engine: &EngineIdentity,
) -> ComparisonReviewReceipt {
    record_review_with_after_seal(output, draft, engine, |_| Ok(()))
}
pub(super) fn record_review_with_after_seal(
    output: &Path,
    draft: &Path,
    engine: &EngineIdentity,
    after_seal: impl FnOnce(&Path) -> std::io::Result<()>,
) -> ComparisonReviewReceipt {
    let mut result = review_empty();
    let attempt = (|| {
        let current = verification::current(output, engine)?;
        result.comparison = Some(current.binding.clone());
        let protected = current.files.iter().collect::<Vec<_>>();
        let guard = output::capture_slot(output, &protected[1..]).map_err(from_output)?;
        settled(&guard)?;
        let locations = paths(current.files[0].path(), None);
        let summary_slot = guard
            .capture_related(&locations.summary, &protected)
            .map_err(from_output)?;
        settled(&summary_slot)?;
        owned_summary(&summary_slot)?;
        let summary_file = summary_slot.previous().ok_or_else(|| {
            fail(
                "compare/summary-required",
                "Run visual-check before recording a comparison review",
                1,
            )
        })?;
        let mut summary: ComparisonSummary = serde_json::from_slice(summary_file.bytes())
            .map_err(|e| fail("compare/summary", e.to_string(), 1))?;
        if summary.comparison != current.binding
            || summary.automated_status != Status::Pass
            || summary.stages.len() != 2
            || summary.stages[0].name != "check"
            || summary.stages[1].name != "visual-check"
            || summary.stages.iter().any(|s| s.status != Status::Pass)
            || summary.stages[0].receipt.as_ref() != Some(&json!(check_receipt(&current)))
        {
            return Err(fail(
                "compare/summary-binding",
                "Both current comparison automatic stages must pass",
                1,
            ));
        }
        let browser_ref: BoundFile =
            serde_json::from_value(summary.evidence["browserReceipt"].clone()).map_err(|_| {
                fail(
                    "compare/browser-required",
                    "Comparison summary is missing its browser receipt",
                    1,
                )
            })?;
        let browser_file = capture_bound(&browser_ref)?;
        let browser: ComparisonBrowserReceipt = serde_json::from_slice(browser_file.bytes())
            .map_err(|e| fail("compare/browser-receipt", e.to_string(), 1))?;
        if browser.comparison != current.binding
            || !browser::comparison::validate_receipt_coverage(&browser, true)
            || summary.stages[1].receipt.as_ref() != Some(&json!(browser))
        {
            return Err(fail(
                "compare/browser-scope",
                "Current browser evidence must cover all three views and twelve screenshots",
                1,
            ));
        }
        result.automated_status = Status::Pass;
        let draft = FileBinding::capture(draft).map_err(from_output)?;
        let record: ComparisonVisualReviewRecord = serde_json::from_slice(draft.bytes())
            .map_err(|e| fail("compare/review-record", e.to_string(), 1))?;
        if record.schema_version != 1
            || record.kind != "architecture-comparison-review"
            || record.command != "review-record"
            || record.comparison != current.binding
            || record.browser_receipt != browser_ref
        {
            return Err(fail(
                "compare/review-binding",
                "Review must bind the current dual-input comparison and browser receipt",
                1,
            ));
        }
        let images = validate_observations(&record, &browser)?;
        let stamp = crate::input::sha256(
            &serde_json::to_vec(&json!({"comparison":current.binding,"browser":browser_ref}))
                .unwrap(),
        );
        let name = format!(
            "{}.compare-review-{}.json",
            comparison_stem(&current.binding.artifact.path),
            &stamp[..24]
        );
        let sealed_path = locations.review.parent().unwrap().join(name);
        let mut all = protected.clone();
        all.push(&browser_file);
        all.push(&draft);
        all.extend(images.iter());
        let sealed_slot = guard
            .capture_related(&sealed_path, &all)
            .map_err(from_output)?;
        settled(&sealed_slot)?;
        let sealed = if let Some(old) = sealed_slot.previous() {
            if old.bytes() != draft.bytes() {
                return Err(fail(
                    "compare/review-conflict",
                    "A different review is already sealed for this comparison and browser evidence",
                    3,
                ));
            }
            FileBinding::capture(old.path()).map_err(from_output)?
        } else {
            write_bytes(&sealed_slot, draft.bytes(), &all)?
        };
        after_seal(sealed.path()).map_err(|e| {
            let mut failure = fail("compare/summary-publication", e.to_string(), 4);
            failure.diagnostics[0].evidence = json!({"sealedReviewRecord":bound(&sealed)});
            failure.diagnostics[0]
                .supported_fixes
                .push("Retry review-record with the same record bytes".into());
            failure
        })?;
        summary.status = if record.verdict == VisualStatus::Passed {
            Status::Pass
        } else if record.verdict == VisualStatus::Unavailable {
            Status::Unavailable
        } else {
            Status::Failed
        };
        summary.diagnostics = if record.verdict == VisualStatus::Passed {
            vec![]
        } else {
            fail("compare/visual-review", record.findings.join("; "), 1).diagnostics
        };
        summary.visual_review = VisualReview {
            status: record.verdict,
            record: Some(bound(&sealed)),
        };
        summary.delivery_status = if record.verdict == VisualStatus::Passed {
            "complete"
        } else {
            "incomplete"
        }
        .into();
        summary.evidence["reviewRecord"] = json!(bound(&sealed));
        all.push(&sealed);
        current.verify()?;
        for file in &all {
            file.verify().map_err(from_output)?;
        }
        let summary_written = if serde_json::from_slice::<ComparisonSummary>(summary_file.bytes())
            .ok()
            .as_ref()
            == Some(&summary)
        {
            FileBinding::capture(summary_file.path()).map_err(from_output)?
        } else {
            match write(&summary_slot, &summary, &all) {
                Ok(file) => file,
                Err(mut e) => {
                    for d in &mut e.diagnostics {
                        d.evidence["sealedReviewRecord"] = json!(bound(&sealed));
                        d.supported_fixes.push("Retry review-record with the same record bytes after resolving the summary conflict".into());
                    }
                    return Err(e);
                }
            }
        };
        current.verify()?;
        result.status = if record.verdict == VisualStatus::Passed {
            Status::Pass
        } else if record.verdict == VisualStatus::Unavailable {
            Status::Unavailable
        } else {
            Status::Failed
        };
        result.exit = if record.verdict == VisualStatus::Passed {
            0
        } else if record.verdict == VisualStatus::Unavailable {
            5
        } else {
            1
        };
        result.automated_status = Status::Pass;
        result.visual_review = summary.visual_review;
        result.delivery_status = summary.delivery_status;
        result.evidence = json!({"summaryReceipt":bound(&summary_written),"browserReceipt":browser_ref,"reviewRecord":bound(&sealed)});
        Ok::<_, Failure>(())
    })();
    if let Err(mut e) = attempt {
        for d in &mut e.diagnostics {
            d.stage = "review-record".into();
        }
        result.diagnostics = e.diagnostics;
        result.exit = e.exit;
    }
    result
}
