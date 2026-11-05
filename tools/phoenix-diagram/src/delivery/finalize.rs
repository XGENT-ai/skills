use super::prepare::{self, Compiled};
use super::support::*;
use super::*;
use crate::browser::{self, BrowserReceipt, BrowserStatus};
use crate::check::check_html;
use serde_json::json;

fn delivery_receipt(validation: &ValidationReceipt) -> DeliveryReceipt {
    DeliveryReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "deliver".into(),
        status: Status::Failed,
        diagnostics: validation.diagnostics.clone(),
        candidate: validation.candidate.clone(),
        artifact: None,
        config: validation.config.clone(),
        config_sha256: validation.config_sha256.clone(),
        engine: validation.engine.clone(),
        quality: validation.config.as_ref().map(|c| c.quality),
        checks: validation.checks.clone(),
        composition: validation.composition.clone(),
        evidence: validation.evidence.clone(),
        model: validation.model.clone(),
        exit: validation.exit_code(),
    }
}

pub(super) fn deliver(request: &CompileRequest<'_>, output: &Path) -> DeliveryReceipt {
    let prepared = prepare::prepare(request);
    let mut receipt = delivery_receipt(&prepared.receipt);
    if let Some(compiled) = prepared.compiled.as_ref() {
        if let Err(failure) = publish(request, output, compiled, &mut receipt) {
            receipt.exit = failure.exit;
            receipt.diagnostics.push(failure.diagnostic);
        }
    }
    receipt
}

fn publish(
    request: &CompileRequest<'_>,
    output: &Path,
    compiled: &Compiled,
    receipt: &mut DeliveryReceipt,
) -> Result<()> {
    if let (Ok(input), Ok(target)) = (
        std::fs::canonicalize(request.candidate),
        std::fs::canonicalize(output),
    ) {
        if input == target {
            return Err(fail(
                "deliver",
                "output/input-alias",
                "The output path names the candidate input",
                2,
            ));
        }
    }
    write("deliver", output, &compiled.html)?;
    receipt.artifact = Some(reference(output, &compiled.html)?);
    receipt.status = Status::Pass;
    receipt.exit = 0;
    Ok(())
}

fn stage(
    summary: &mut FinalizeReceipt,
    index: usize,
    status: Status,
    receipt: &impl serde::Serialize,
) {
    summary.stages[index].status = status;
    summary.stages[index].receipt =
        Some(serde_json::to_value(receipt).expect("stage receipt serializes"));
}

pub(super) fn finalize(
    request: &CompileRequest<'_>,
    output: &Path,
    browser_options: &BrowserOptions,
) -> FinalizeReceipt {
    let prepared = prepare::prepare(request);
    let mut summary = FinalizeReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "finalize".into(),
        status: Status::Failed,
        diagnostics: prepared.receipt.diagnostics.clone(),
        stages: ["validate", "deliver", "browser-check"]
            .into_iter()
            .map(|name| Stage {
                name: name.into(),
                status: Status::NotRun,
                receipt: None,
            })
            .collect(),
        candidate: prepared.receipt.candidate.clone(),
        artifact: None,
        config_sha256: prepared.receipt.config_sha256.clone(),
        engine: prepared.receipt.engine.clone(),
        automated_status: Status::Failed,
        screenshots: vec![],
        visual_review: VisualReview::pending(),
        delivery_status: "incomplete".into(),
        evidence: evidence_summary(&prepared.receipt.evidence),
        model: prepared.receipt.model.clone(),
        locale: prepared.locale.clone(),
        exit: prepared.receipt.exit_code(),
    };
    stage(&mut summary, 0, prepared.receipt.status, &prepared.receipt);
    let path = summary_path(output);
    let run = (|| -> Result<()> {
        summary.evidence["summaryPath"] = json!(absolute(&path)?);
        let Some(compiled) = prepared.compiled.as_ref() else {
            return Ok(());
        };
        let mut delivery = delivery_receipt(&prepared.receipt);
        let published = publish(request, output, compiled, &mut delivery);
        stage(&mut summary, 1, delivery.status, &delivery);
        published?;
        summary.artifact = delivery.artifact.clone();
        let mut options = browser_options.clone();
        options.capture = true;
        let browser = browser::check_browser(output, &options);
        stage(&mut summary, 2, browser_status(browser.status), &browser);
        summary.diagnostics.extend(browser.diagnostics.clone());
        summary.screenshots = browser.screenshots.iter().map(|s| s.path.clone()).collect();
        summary.exit = browser.exit_code();
        match browser.status {
            BrowserStatus::Pass => {
                summary.status = Status::Pass;
                summary.automated_status = Status::Pass;
            }
            BrowserStatus::Unavailable => {
                summary.status = Status::Unavailable;
                summary.automated_status = Status::Unavailable;
            }
            BrowserStatus::Failed => {}
        }
        Ok(())
    })();
    if let Err(failure) = run {
        summary.exit = failure.exit;
        summary.diagnostics.push(failure.diagnostic);
    }
    if let Err(failure) = write("finalize", &path, &json_bytes(&summary)) {
        summary.status = Status::Failed;
        summary.automated_status = Status::Failed;
        summary.exit = failure.exit;
        summary.diagnostics.push(failure.diagnostic);
    }
    summary
}

pub(super) fn check(output: &Path) -> CheckReceipt {
    let mut result = CheckReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "check".into(),
        status: Status::Failed,
        diagnostics: vec![],
        artifact: None,
        engine: EngineIdentity::current(),
        checks: vec![],
        composition: serde_json::Value::Null,
        exit: 1,
    };
    let inspected = (|| -> Result<()> {
        let bytes = read("check", "output/read", output)?;
        let checked = check_html(&bytes, artifact_quality(&bytes));
        result.artifact = Some(reference(output, &bytes)?);
        if passed_check(&checked) {
            result.status = Status::Pass;
            result.exit = 0;
        }
        result.checks = checked.checks;
        result.composition = checked.composition;
        result.diagnostics = checked.diagnostics;
        Ok(())
    })();
    if let Err(failure) = inspected {
        result.diagnostics.push(failure.diagnostic);
        result.exit = failure.exit;
    }
    result
}

pub(super) fn browser_check(
    output: &Path,
    options: &BrowserOptions,
    visual: bool,
) -> BrowserReceipt {
    let mut options = options.clone();
    options.capture = visual || options.capture;
    let mut receipt = browser::check_browser(output, &options);
    receipt.command = if visual {
        "visual-check"
    } else {
        "browser-check"
    }
    .into();
    receipt
}

fn browser_status(status: BrowserStatus) -> Status {
    match status {
        BrowserStatus::Pass => Status::Pass,
        BrowserStatus::Failed => Status::Failed,
        BrowserStatus::Unavailable => Status::Unavailable,
    }
}
