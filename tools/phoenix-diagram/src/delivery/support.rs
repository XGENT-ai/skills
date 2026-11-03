use super::*;
use crate::check::{ArtifactCheck, FileIdentity};
use crate::diagnostic::Diagnostic;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(super) struct Failure {
    pub diagnostic: Diagnostic,
    pub exit: u8,
}
pub(super) type Result<T> = std::result::Result<T, Box<Failure>>;

pub(super) fn fail(stage: &str, code: &str, message: impl Into<String>, exit: u8) -> Box<Failure> {
    let mut diagnostic = Diagnostic::error(None, code, "", message);
    diagnostic.stage = stage.into();
    Box::new(Failure { diagnostic, exit })
}

pub(super) fn read(stage: &str, code: &str, path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| fail(stage, code, format!("{}: {e}", path.display()), 1))
}

pub(super) fn reference(path: &Path, bytes: &[u8]) -> Result<FileReference> {
    Ok(FileReference {
        path: absolute(path)?,
        identity: FileIdentity::of(bytes),
    })
}

/// Write a JSON document or HTML beside the output, replacing any older copy.
pub(super) fn write(stage: &str, path: &Path, bytes: &[u8]) -> Result<()> {
    crate::output::write(path, bytes)
        .map_err(|e| fail(stage, "output/write", format!("{}: {e}", path.display()), 4))
}

pub(super) fn json_bytes(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("receipts serialize");
    bytes.push(b'\n');
    bytes
}

pub(super) fn passed_check(check: &ArtifactCheck) -> bool {
    check.status == "pass"
        && !check.checks.is_empty()
        && check.checks.iter().all(|entry| entry.status == "pass")
        && check.composition["schemaVersion"] == 1
        && check.composition["status"] == "pass"
        && check.composition["summary"]["errors"] == 0
        && (check.quality != crate::input::Quality::Showcase
            || check.composition["summary"]["warnings"] == 0)
}

/// Finalize receipts and summaries repeat only the evidence status and digest.
pub(super) fn evidence_summary(validation: &Value) -> Value {
    match validation.get("status") {
        Some(status) => json!({"status":status,"evidenceSha256":validation["evidenceSha256"]}),
        None => json!({}),
    }
}

pub(super) fn config(input: &crate::input::DiagramInput) -> EffectiveConfig {
    EffectiveConfig {
        brand_resources: input
            .brands
            .iter()
            .map(|(path, brand)| (path.clone(), brand.resource.clone()))
            .collect(),
        diagram_type: input.diagram_type,
        input_contract: input.input_contract,
        quality: input.quality,
        viewer_revision: input.viewer_revision.clone(),
    }
}

pub(super) fn artifact_quality(bytes: &[u8]) -> crate::input::Quality {
    use scraper::{Html, Selector};
    let document = Html::parse_document(std::str::from_utf8(bytes).unwrap_or_default());
    let selector = Selector::parse("svg[data-quality-profile]").expect("static selector");
    if document
        .select(&selector)
        .next()
        .and_then(|svg| svg.value().attr("data-quality-profile"))
        == Some("standard")
    {
        crate::input::Quality::Standard
    } else {
        crate::input::Quality::Showcase
    }
}

/// `diagram.html` keeps its summary at `diagram.finalize-summary.json`.
pub(super) fn summary_path(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();
    let stem = name.strip_suffix(".html").unwrap_or(&name);
    output.with_file_name(format!("{stem}.finalize-summary.json"))
}

pub(super) fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.into())
    } else {
        Ok(std::env::current_dir()
            .map_err(|e| fail("delivery", "output/current-directory", e.to_string(), 4))?
            .join(path))
    }
}
