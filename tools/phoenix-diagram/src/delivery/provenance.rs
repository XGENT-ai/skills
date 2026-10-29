use super::support::*;
use super::*;
use crate::browser::{BoundArtifact, BoundFile};
use crate::check::{check_html, ArtifactCheck, FileIdentity};
use crate::input::{parse_input, CompileOptions};
use crate::output::{self, FileBinding, OutputSlot};
use std::path::Path;

pub(super) struct Current {
    pub candidate: FileBinding,
    pub artifact: FileBinding,
    pub delivery: FileBinding,
    pub receipt: DeliveryReceipt,
    pub checked: ArtifactCheck,
}

impl Current {
    pub fn verify(&self) -> Result<()> {
        verify(&[&self.candidate, &self.artifact, &self.delivery], "check")
    }
    pub fn bound_artifact(&self) -> BoundArtifact {
        BoundArtifact {
            path: self.artifact.path().to_owned(),
            candidate: reference(&self.candidate).bound_file(),
            artifact: FileIdentity::of(self.artifact.bytes()),
            config_sha256: self.receipt.config_sha256.clone().expect("verified config"),
            engine: serde_json::to_value(&self.receipt.engine).expect("engine serializes"),
            delivery_receipt_id: self.receipt.receipt_id.clone(),
        }
    }
}

pub(super) fn current(output: &Path, engine: &EngineIdentity) -> Result<Current> {
    let slot = output::capture_slot(output, &[]).map_err(|error| output_error("check", error))?;
    current_locked(output, engine, &slot)
}

pub(super) fn current_locked(
    output: &Path,
    engine: &EngineIdentity,
    slot: &OutputSlot,
) -> Result<Current> {
    if !valid_engine(engine) {
        return Err(fail(
            "check",
            "engine/identity",
            "Current engine identity is unsupported",
            2,
        ));
    }
    settled(slot, "check")?;
    let artifact = FileBinding::capture(output).map_err(|e| output_error("check", e))?;
    let paths = paths(artifact.path(), None);
    let receipt_slot = slot
        .capture_related(&paths.delivery, &[&artifact])
        .map_err(|e| output_error("check", e))?;
    settled(&receipt_slot, "check")?;
    let delivery = FileBinding::capture(&paths.delivery).map_err(|e| {
        let mut error = output_error("check", e);
        error.diagnostic.code = "provenance/delivery-receipt".into();
        error
    })?;
    let receipt: DeliveryReceipt = serde_json::from_slice(delivery.bytes())
        .map_err(|e| fail("check", "provenance/delivery-receipt", e.to_string(), 1))?;
    if !header(
        receipt.schema_version,
        &receipt.tool,
        &receipt.tool_version,
        &receipt.command,
        "deliver",
    ) || receipt.status != Status::Pass
        || receipt.commit_state != CommitState::Committed
        || !valid_id(&receipt.receipt_id)
    {
        return Err(fail(
            "check",
            "provenance/delivery-receipt",
            "The delivery receipt does not describe a committed supported delivery",
            1,
        ));
    }
    if &receipt.engine != engine {
        return Err(fail(
            "check",
            "provenance/engine",
            "The delivery receipt belongs to a different engine",
            1,
        ));
    }
    let Some(candidate_ref) = receipt.candidate.as_ref() else {
        return Err(fail(
            "check",
            "provenance/candidate-binding",
            "The original candidate binding is missing",
            1,
        ));
    };
    let candidate = capture_expected(candidate_ref, "check", "provenance/candidate-binding")?;
    if receipt.artifact.as_ref() != Some(&reference(&artifact)) {
        return Err(fail(
            "check",
            "provenance/artifact-binding",
            "HTML no longer matches the delivered original file binding",
            1,
        ));
    }
    let Some(cfg) = receipt.config.as_ref() else {
        return Err(fail(
            "check",
            "provenance/config",
            "Effective compile options are missing",
            1,
        ));
    };
    if cfg.viewer_revision != engine.viewer_revision
        || receipt.input_contract != Some(cfg.input_contract)
        || receipt.quality != Some(cfg.quality)
    {
        return Err(fail(
            "check",
            "provenance/config",
            "Compile options disagree with the receipt contracts",
            1,
        ));
    }
    let options = CompileOptions {
        input_contract: Some(cfg.input_contract),
        quality: Some(cfg.quality),
        viewer_revision: cfg.viewer_revision.clone(),
    };
    let mut parsed =
        parse_input(candidate.bytes(), cfg.diagram_type, &options).map_err(|diagnostics| {
            let mut error = fail(
                "check",
                "provenance/candidate",
                "Original candidate no longer satisfies the bound input contract",
                1,
            );
            error.diagnostic.evidence =
                serde_json::to_value(diagnostics).expect("diagnostics serialize");
            error
        })?;
    let resources = std::str::from_utf8(artifact.bytes())
        .ok()
        .and_then(|s| super::metadata::embedded(&scraper::Html::parse_document(s)))
        .and_then(|m| m.get("brandResources").cloned())
        .map(serde_json::from_value::<crate::brands::ResolvedBrands>)
        .transpose()
        .map_err(|_| {
            fail(
                "check",
                "brand/digest-mismatch",
                "Invalid embedded brand resources",
                1,
            )
        })?
        .unwrap_or_default();
    let brands = crate::brands::bound(&parsed, &resources).map_err(|_| {
        fail(
            "check",
            "brand/digest-mismatch",
            "Embedded brand resources disagree with the input",
            1,
        )
    })?;
    parsed.bind_brands(brands);
    crate::brands::sanitize_document(&mut parsed.document, parsed.diagram_type);
    // Git is not consulted again: the artifact's evidence must be the one the
    // receipt digested, and it must cite exactly the candidate's sources.
    let evidence = std::str::from_utf8(artifact.bytes())
        .ok()
        .and_then(|source| super::metadata::embedded(&scraper::Html::parse_document(source)))
        .ok_or("Artifact metadata is missing")
        .and_then(|metadata| crate::evidence::bound(&parsed, &metadata))
        .map_err(|message| fail("check", "provenance/evidence-mismatch", message, 1))?;
    if receipt.evidence.get("evidenceSha256") != Some(&serde_json::json!(evidence.digest()))
        || receipt.evidence.get("status") != Some(&serde_json::json!(evidence.status))
    {
        return Err(fail(
            "check",
            "provenance/evidence-mismatch",
            "Artifact source evidence differs from the delivery receipt",
            1,
        ));
    }
    parsed.bind_evidence(evidence.digest().as_deref());
    if config(&parsed) != *cfg
        || receipt.config_sha256.as_deref() != Some(parsed.config_sha256.as_str())
        || receipt.compatibility.as_ref() != Some(&parsed.compatibility)
    {
        return Err(fail(
            "check",
            "provenance/config",
            "Candidate, effective options, config digest, or compatibility no longer match",
            1,
        ));
    }
    if !super::metadata::matches(artifact.bytes(), &parsed) {
        return Err(fail("check","provenance/config","Rendered effective configuration or authored facts disagree with the original candidate",1));
    }
    let checked = check_html(artifact.bytes(), cfg.quality);
    if !passed_check(&checked) {
        let mut error = fail(
            "check",
            "provenance/artifact-check",
            "Current HTML fails independent artifact checks",
            1,
        );
        error.diagnostic.evidence =
            serde_json::to_value(&checked).expect("artifact check serializes");
        return Err(error);
    }
    if !scope_names(&receipt.checks)
        || receipt.checks != checked.checks
        || receipt.composition != checked.composition
    {
        return Err(fail(
            "check",
            "provenance/check-scope",
            "Delivered named checks or their actual scopes do not match independent current checks",
            1,
        ));
    }
    let current = Current {
        candidate,
        artifact,
        delivery,
        receipt,
        checked,
    };
    current.verify()?;
    settled(slot, "check")?;
    settled(&receipt_slot, "check")?;
    slot.verify_guard()
        .map_err(|error| output_error("check", error))?;
    Ok(current)
}

fn empty(engine: &EngineIdentity, strict: bool) -> CheckReceipt {
    CheckReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "check".into(),
        status: Status::Failed,
        diagnostics: vec![],
        provenance: "unknown".into(),
        scope: if strict {
            "strict-provenance"
        } else {
            "artifact-only"
        }
        .into(),
        candidate: None,
        artifact: None,
        config_sha256: None,
        engine: engine.clone(),
        delivery_receipt_id: None,
        delivery_receipt: None,
        checks: vec![],
        composition: serde_json::Value::Null,
        exit: 1,
    }
}

pub(super) fn failed(engine: &EngineIdentity, strict: bool, failure: Box<Failure>) -> CheckReceipt {
    let mut result = empty(engine, strict);
    result.exit = failure.exit;
    result.diagnostics.push(failure.diagnostic);
    result
}

fn checked_current(current: Result<Current>, engine: &EngineIdentity) -> CheckReceipt {
    let mut result = empty(engine, true);
    match current {
        Ok(current) => {
            result.status = Status::Pass;
            result.provenance = "current".into();
            result.exit = 0;
            result.candidate = Some(reference(&current.candidate));
            result.artifact = Some(reference(&current.artifact));
            result.config_sha256 = current.receipt.config_sha256.clone();
            result.delivery_receipt_id = Some(current.receipt.receipt_id);
            result.delivery_receipt = Some(BoundFile {
                path: current.delivery.path().to_owned(),
                identity: FileIdentity::of(current.delivery.bytes()),
            });
            result.checks = current.checked.checks;
            result.composition = current.checked.composition;
            result.diagnostics = current.checked.diagnostics;
            result
        }
        Err(failure) => failed(engine, true, failure),
    }
}

pub(super) fn inspect_locked(
    output: &Path,
    engine: &EngineIdentity,
    slot: &OutputSlot,
) -> CheckReceipt {
    checked_current(current_locked(output, engine, slot), engine)
}

pub(super) fn inspect(output: &Path, strict: bool, engine: &EngineIdentity) -> CheckReceipt {
    let mut result = empty(engine, strict);
    if strict {
        return checked_current(current(output, engine), engine);
    } else {
        let inspected: Result<()> = (|| {
            let artifact = FileBinding::capture(output).map_err(|e| output_error("check", e))?;
            let checked = check_html(artifact.bytes(), artifact_quality(artifact.bytes()));
            result.artifact = Some(reference(&artifact));
            result.checks = checked.checks.clone();
            result.composition = checked.composition.clone();
            result.diagnostics = checked.diagnostics.clone();
            verify(&[&artifact], "check")?;
            if passed_check(&checked) {
                result.status = Status::Pass;
                result.exit = 0;
            }
            Ok(())
        })();
        if let Err(failure) = inspected {
            result.diagnostics.push(failure.diagnostic);
            result.exit = failure.exit;
        }
    }
    result
}
