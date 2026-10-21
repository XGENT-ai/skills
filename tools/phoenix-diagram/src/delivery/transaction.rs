use super::prepare::{self, Compiled};
use super::provenance;
use super::support::*;
use super::*;
use crate::browser::{self, BrowserReceipt, BrowserStatus};
use crate::check::FileIdentity;
use crate::output::{self, FileBinding, OutputSlot};
use serde_json::json;

fn delivery_receipt(validation: &ValidationReceipt) -> DeliveryReceipt {
    DeliveryReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "deliver".into(),
        status: Status::Failed,
        diagnostics: validation.diagnostics.clone(),
        receipt_id: String::new(),
        candidate: validation.candidate.clone(),
        artifact: None,
        config: validation.config.clone(),
        config_sha256: validation.config_sha256.clone(),
        engine: validation.engine.clone(),
        input_contract: validation.config.as_ref().map(|c| c.input_contract),
        quality: validation.config.as_ref().map(|c| c.quality),
        compatibility: validation.compatibility.clone(),
        checks: validation.checks.clone(),
        composition: validation.composition.clone(),
        commit_state: CommitState::NotCommitted,
        evidence: validation.evidence.clone(),
        exit: validation.exit_code(),
    }
}

pub(super) fn deliver(request: &CompileRequest<'_>, output: &Path) -> DeliveryReceipt {
    let prepared = prepare::prepare(request);
    let mut receipt = delivery_receipt(&prepared.receipt);
    let Some(compiled) = prepared.compiled.as_ref() else {
        return receipt;
    };
    let source = prepared.source.as_ref().expect("compiled source");
    let run: Result<()> = (|| {
        let slot =
            output::capture_slot(output, &[source]).map_err(|e| output_error("deliver", e))?;
        publish(&slot, source, compiled, &mut receipt)?;
        Ok(())
    })();
    if let Err(failure) = run {
        record_delivery_failure(&mut receipt, failure);
    }
    receipt
}

fn own_delivery(slot: &OutputSlot) -> Result<()> {
    if let Some(file) = slot.previous() {
        let prior: DeliveryReceipt = serde_json::from_slice(file.bytes()).map_err(|_| {
            fail(
                "deliver",
                "delivery/receipt-conflict",
                "An unknown file occupies the delivery receipt path",
                3,
            )
        })?;
        if !header(
            prior.schema_version,
            &prior.tool,
            &prior.tool_version,
            &prior.command,
            "deliver",
        ) || !valid_id(&prior.receipt_id)
            || !valid_engine(&prior.engine)
        {
            return Err(fail(
                "deliver",
                "delivery/receipt-conflict",
                "The existing delivery receipt is not owned by this supported tool",
                3,
            ));
        }
    }
    Ok(())
}

fn own_summary(slot: &OutputSlot, output: &Path) -> Result<()> {
    if let Some(file) = slot.previous() {
        let prior: FinalizeReceipt = serde_json::from_slice(file.bytes()).map_err(|_| {
            fail(
                "finalize",
                "finalize/summary-conflict",
                "An unknown file occupies the finalize summary path",
                3,
            )
        })?;
        if !header(
            prior.schema_version,
            &prior.tool,
            &prior.tool_version,
            &prior.command,
            "finalize",
        ) || !valid_engine(&prior.engine)
            || prior.stages.len() != 4
            || prior
                .stages
                .iter()
                .zip(["validate", "deliver", "check", "browser-check"])
                .any(|(stage, name)| stage.name != name)
            || prior
                .artifact
                .as_ref()
                .is_some_and(|artifact| artifact.path != output)
        {
            return Err(fail(
                "finalize",
                "finalize/summary-conflict",
                "The existing finalize summary is not a supported pointer for this output",
                3,
            ));
        }
    }
    Ok(())
}

fn publish(
    slot: &OutputSlot,
    source: &FileBinding,
    compiled: &Compiled,
    receipt: &mut DeliveryReceipt,
) -> Result<FileBinding> {
    settled(slot, "deliver")?;
    let sidecars = paths(&slot.path(), None);
    let receipt_slot = slot
        .capture_related(&sidecars.delivery, &[source])
        .map_err(|e| output_error("deliver", e))?;
    settled(&receipt_slot, "deliver")?;
    own_delivery(&receipt_slot)?;
    receipt.receipt_id = unique_id()?;
    let publication = match output::publish_bound(slot, &compiled.html, &[source]) {
        Ok(publication) => publication,
        Err(error) => {
            receipt.evidence["htmlCommitted"] = json!(error.committed);
            receipt.commit_state = if error.committed {
                CommitState::CommittedUnreceipted
            } else if error.recovery_directory.is_some() {
                CommitState::RecoveryRequired
            } else {
                CommitState::NotCommitted
            };
            return Err(output_error("deliver", error));
        }
    };
    receipt.evidence["htmlCommitted"] = json!(true);
    receipt.evidence["publication"] = json!(publication);
    if !publication.cleanup_complete {
        receipt.commit_state = CommitState::RecoveryRequired;
        return Err(fail(
            "deliver",
            "output/cleanup-incomplete",
            "New HTML is visible but publication cleanup or recovery is incomplete",
            3,
        ));
    }
    let artifact =
        FileBinding::capture(&publication.path).map_err(|e| output_error("deliver", e))?;
    receipt.artifact = Some(reference(&artifact));
    receipt.commit_state = CommitState::CommittedUnreceipted;
    if artifact.bytes() != compiled.html || artifact.identity() != &publication.identity {
        return Err(fail(
            "deliver",
            "output/publication-changed",
            "New HTML changed before its delivery receipt was committed",
            3,
        ));
    }
    source.verify().map_err(|e| output_error("deliver", e))?;
    receipt.status = Status::Pass;
    receipt.commit_state = CommitState::Committed;
    receipt.exit = 0;
    receipt.evidence["deliveryReceiptPath"] = json!(receipt_slot.path());
    let bytes = json_bytes(receipt)?;
    let written = match write(&receipt_slot, &bytes, &[source, &artifact]) {
        Ok(file) => file,
        Err(failure) => {
            receipt.status = Status::Failed;
            receipt.commit_state = CommitState::CommittedUnreceipted;
            return Err(failure);
        }
    };
    verify(&[source, &artifact, &written], "deliver")?;
    settled(slot, "deliver")?;
    settled(&receipt_slot, "deliver")?;
    slot.verify_guard()
        .map_err(|e| output_error("deliver", e))?;
    Ok(artifact)
}

fn record_delivery_failure(receipt: &mut DeliveryReceipt, failure: Box<Failure>) {
    receipt.status = Status::Failed;
    receipt.exit = failure.exit;
    receipt.diagnostics.push(failure.diagnostic);
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
        stages: ["validate", "deliver", "check", "browser-check"]
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
        engine: request.engine.clone(),
        delivery_receipt_id: None,
        browser_receipt_identity: None,
        automated_status: Status::Failed,
        visual_review: VisualReview {
            status: VisualStatus::RequiredPending,
            record: None,
        },
        delivery_status: "incomplete".into(),
        produced_by_this_run: false,
        evidence: evidence_summary(&prepared.receipt.evidence),
        locale: prepared.locale.clone(),
        exit: prepared.receipt.exit_code(),
    };
    stage(&mut summary, 0, prepared.receipt.status, &prepared.receipt);
    let mut protected = Vec::new();
    if let Some(source) = prepared.source.as_ref() {
        protected.push(source);
    }
    let run: Result<()> = (|| {
        let slot =
            output::capture_slot(output, &protected).map_err(|e| output_error("finalize", e))?;
        if let Some(old) = slot.previous() {
            summary.artifact = Some(reference(old));
        }
        settled(&slot, "finalize")?;
        let sidecars = paths(&slot.path(), browser_options.out_dir.as_deref());
        let summary_slot = slot
            .capture_related(&sidecars.summary, &protected)
            .map_err(|e| output_error("finalize", e))?;
        settled(&summary_slot, "finalize")?;
        own_summary(&summary_slot, &slot.path())?;
        summary.evidence["summaryPath"] = json!(summary_slot.path());
        summary.evidence["reviewPath"] = json!(absolute(&sidecars.review)?);
        let mut gate_current = None;
        let mut browser_file = None;
        let mut screenshots = Vec::new();
        let pipeline = (|| {
            let Some(compiled) = prepared.compiled.as_ref() else {
                return Ok(());
            };
            let source = prepared.source.as_ref().expect("compiled source");
            let mut delivery = delivery_receipt(&prepared.receipt);
            let artifact = match publish(&slot, source, compiled, &mut delivery) {
                Ok(artifact) => artifact,
                Err(failure) => {
                    record_delivery_failure(&mut delivery, failure);
                    summary.produced_by_this_run = delivery.evidence["htmlCommitted"] == true;
                    if summary.produced_by_this_run || delivery.artifact.is_some() {
                        summary.artifact = delivery.artifact.clone();
                    }
                    summary.delivery_receipt_id =
                        (!delivery.receipt_id.is_empty()).then(|| delivery.receipt_id.clone());
                    summary.diagnostics.extend(delivery.diagnostics.clone());
                    summary.exit = delivery.exit_code();
                    stage(&mut summary, 1, Status::Failed, &delivery);
                    return Ok(());
                }
            };
            summary.artifact = Some(reference(&artifact));
            summary.produced_by_this_run = true;
            summary.delivery_receipt_id = Some(delivery.receipt_id.clone());
            stage(&mut summary, 1, Status::Pass, &delivery);
            let checked = provenance::inspect_locked(artifact.path(), request.engine, &slot);
            stage(&mut summary, 2, checked.status, &checked);
            if let Some(delivery_receipt) = checked.delivery_receipt.as_ref() {
                summary.evidence["deliveryReceipt"] = json!(delivery_receipt);
            }
            if checked.status != Status::Pass {
                summary.exit = checked.exit_code();
                summary.diagnostics.extend(checked.diagnostics);
                return Ok(());
            }
            if checked.candidate != summary.candidate
                || checked.artifact != summary.artifact
                || checked.config_sha256 != summary.config_sha256
                || checked.delivery_receipt_id != summary.delivery_receipt_id
            {
                summary.stages[2].status = Status::Failed;
                return Err(fail(
                    "finalize",
                    "finalize/gate-binding",
                    "Strict gate does not prove the current snapshot and delivery",
                    1,
                ));
            }
            gate_current = Some(provenance::current_locked(
                artifact.path(),
                request.engine,
                &slot,
            )?);
            let current = gate_current.as_ref().expect("verified browser candidate");
            let mut options = browser_options.clone();
            options.capture = true;
            let browser = browser::check_browser(&current.bound_artifact(), &options);
            stage(&mut summary, 3, browser_status(browser.status), &browser);
            let sealed = match seal_browser(&slot, current, &browser, &options, false) {
                Ok(file) => file,
                Err(failure) => {
                    summary.stages[3].status = Status::Failed;
                    return Err(failure);
                }
            };
            summary.browser_receipt_identity = Some(FileIdentity::of(sealed.bytes()));
            summary.evidence["browserReceipt"] = bound_json(&sealed);
            summary.evidence["reviewPath"] =
                json!(paths(current.artifact.path(), sealed.path().parent()).review);
            browser_file = Some(sealed);
            summary.diagnostics.extend(browser.diagnostics.clone());
            summary.exit = browser.exit_code();
            if browser.status != BrowserStatus::Pass {
                return Ok(());
            }
            if !browser::validate_receipt_coverage(&browser, true) {
                summary.stages[3].status = Status::Failed;
                return Err(fail("finalize","finalize/browser-scope","Browser receipt did not execute the complete required measurement and screenshot scope",1));
            }
            for screenshot in &browser.screenshots {
                let file = FileBinding::capture(&screenshot.path)
                    .map_err(|e| output_error("finalize", e))?;
                if file.path() != screenshot.path
                    || FileIdentity::of(file.bytes()) != screenshot.identity
                {
                    summary.stages[3].status = Status::Failed;
                    return Err(fail(
                        "finalize",
                        "finalize/screenshot-binding",
                        "A captured screenshot changed before the finalize summary commit",
                        1,
                    ));
                }
                screenshots.push(file);
            }
            current.verify()?;
            browser_file
                .as_ref()
                .expect("sealed browser receipt")
                .verify()
                .map_err(|e| output_error("finalize", e))?;
            slot.verify_guard()
                .map_err(|e| output_error("finalize", e))?;
            summary.status = Status::Pass;
            summary.automated_status = Status::Pass;
            summary.exit = 0;
            Ok(())
        })();
        if let Err(failure) = pipeline {
            summary.status = Status::Failed;
            summary.automated_status = Status::Failed;
            summary.exit = failure.exit;
            summary.diagnostics.push(failure.diagnostic);
        }
        if summary
            .stages
            .iter()
            .any(|stage| stage.status == Status::Unavailable)
        {
            summary.automated_status = Status::Unavailable;
            summary.status = Status::Unavailable;
        }
        // A summary is a current pointer. Published browser and image evidence stays sealed.
        let artifact_binding = if summary.produced_by_this_run {
            summary
                .artifact
                .as_ref()
                .map(|file| capture_expected(file, "finalize", "finalize/artifact-binding"))
                .transpose()?
        } else {
            None
        };
        let mut summary_protected = protected.clone();
        if let Some(artifact) = artifact_binding.as_ref() {
            summary_protected.push(artifact);
        }
        if !summary.produced_by_this_run {
            slot.verify_snapshot()
                .map_err(|e| output_error("finalize", e))?;
            if let Some(old) = slot.previous() {
                summary_protected.push(old);
            }
        }
        if let Some(current) = gate_current.as_ref() {
            current.verify()?;
            summary_protected.extend([&current.candidate, &current.artifact, &current.delivery]);
        }
        if let Some(file) = browser_file.as_ref() {
            summary_protected.push(file);
        }
        summary_protected.extend(screenshots.iter());
        settled(&slot, "finalize")?;
        let bytes = json_bytes(&summary)?;
        let written = write(&summary_slot, &bytes, &summary_protected)?;
        verify(&summary_protected, "finalize")?;
        settled(&slot, "finalize")?;
        settled(&summary_slot, "finalize")?;
        summary.evidence["summaryReceipt"] = bound_json(&written);
        Ok(())
    })();
    if let Err(failure) = run {
        summary.status = Status::Failed;
        summary.automated_status = Status::Failed;
        summary.delivery_status = "incomplete".into();
        summary.exit = failure.exit;
        summary.diagnostics.push(failure.diagnostic);
    }
    summary
}

pub(super) fn browser_check(
    output: &Path,
    engine: &EngineIdentity,
    options: &BrowserOptions,
    visual: bool,
) -> std::result::Result<BrowserCommandReceipt, Box<CheckReceipt>> {
    let command = if visual {
        "visual-check"
    } else {
        "browser-check"
    };
    let slot = match output::capture_slot(output, &[]) {
        Ok(slot) => slot,
        Err(error) => {
            let error = output_error(command, error);
            let mut failed = provenance::failed(engine, true, error);
            failed.command = command.into();
            failed.status = Status::Failed;
            return Err(Box::new(failed));
        }
    };
    let current = match provenance::current_locked(output, engine, &slot) {
        Ok(current) => current,
        Err(failure) => {
            let mut failed = provenance::failed(engine, true, failure);
            failed.command = command.into();
            return Err(Box::new(failed));
        }
    };
    let mut options = options.clone();
    options.capture = visual || options.capture;
    let mut receipt = browser::check_browser(&current.bound_artifact(), &options);
    receipt.command = command.into();
    let mut result = BrowserCommandReceipt {
        exit: receipt.exit_code(),
        receipt,
        evidence: json!({}),
    };
    match seal_browser(&slot, &current, &result.receipt, &options, visual) {
        Ok(file) => result.evidence["browserReceipt"] = bound_json(&file),
        Err(failure) => {
            result.receipt.status = BrowserStatus::Failed;
            result.receipt.diagnostics.push(failure.diagnostic);
            result.exit = failure.exit;
        }
    }
    Ok(result)
}

fn seal_browser(
    slot: &OutputSlot,
    current: &provenance::Current,
    receipt: &BrowserReceipt,
    options: &BrowserOptions,
    visual: bool,
) -> Result<FileBinding> {
    current.verify()?;
    let bound = current.bound_artifact();
    let command = if visual {
        "visual-check"
    } else {
        "browser-check"
    };
    if !header(
        receipt.schema_version,
        &receipt.tool,
        &receipt.tool_version,
        &receipt.command,
        command,
    ) || receipt.artifact_path != bound.path
        || receipt.candidate != bound.candidate
        || receipt.artifact != bound.artifact
        || receipt.config_sha256 != bound.config_sha256
        || receipt.engine != bound.engine
        || receipt.delivery_receipt_id != bound.delivery_receipt_id
    {
        return Err(fail(
            command,
            "browser/receipt-binding",
            "The browser receipt does not identify the current delivery and artifact",
            1,
        ));
    }
    slot.verify_guard()
        .map_err(|e| output_error("browser-check", e))?;
    let paths = paths(current.artifact.path(), options.out_dir.as_deref());
    let path = if visual { paths.visual } else { paths.browser };
    let protected = [&current.candidate, &current.artifact, &current.delivery];
    let sidecar = slot
        .capture_related(&path, &protected)
        .map_err(|e| output_error("browser-check", e))?;
    settled(&sidecar, "browser-check")?;
    if sidecar.previous().is_some() {
        return Err(fail(
            "browser-check",
            "browser/evidence-conflict",
            "Browser evidence is already sealed; use a fresh --out-dir",
            3,
        ));
    }
    let written = write(&sidecar, &json_bytes(receipt)?, &protected)?;
    current.verify()?;
    settled(&sidecar, "browser-check")?;
    Ok(written)
}

fn browser_status(status: BrowserStatus) -> Status {
    match status {
        BrowserStatus::Pass => Status::Pass,
        BrowserStatus::Failed => Status::Failed,
        BrowserStatus::Unavailable => Status::Unavailable,
    }
}
