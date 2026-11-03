use super::*;
use crate::{
    check::FileIdentity,
    delivery::{self, CompileRequest, Status, VisualReview},
    diagnostic::Diagnostic,
    input::{sha256, DiagramType},
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct CompareRequest<'a> {
    pub base: CompileRequest<'a>,
    pub head: CompileRequest<'a>,
    pub output: &'a Path,
    pub receipt: Option<&'a Path>,
}
fn empty() -> CompareReceipt {
    CompareReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "compare".into(),
        kind: "architecture-comparison".into(),
        status: Status::Failed,
        diagnostics: vec![],
        comparator_version: 1,
        canonical_version: 1,
        base: None,
        head: None,
        comparison_ir: None,
        changes: json!({}),
        summary: json!({}),
        artifact: None,
        checks: vec![],
        browser_status: Status::NotRun,
        browser: None,
        screenshots: vec![],
        visual_review: VisualReview::pending(),
        delivery_status: "incomplete".into(),
        limitations: vec![],
        exit: 1,
    }
}
fn path(path: &Path) -> Result<PathBuf, Failure> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| fail("compare/output", "Output must name a file", 2))?;
    Ok(std::fs::canonicalize(parent)
        .map_err(|e| fail("compare/output", e.to_string(), 4))?
        .join(name))
}
fn side(prepared: &delivery::prepare::Preparation) -> CompareSide {
    let c = prepared
        .compiled
        .as_ref()
        .expect("successful comparison side");
    CompareSide {
        candidate: prepared
            .receipt
            .candidate
            .clone()
            .expect("captured candidate"),
        raw: FileIdentity::of(&c.input.source_bytes),
        semantic_sha256: sha256(&canonical::canonical_json(
            &serde_json::from_slice::<Value>(&c.input.source_bytes).expect("validated input JSON"),
        )),
        config: prepared.receipt.config.clone().expect("validated config"),
        config_sha256: c.input.config_sha256.clone(),
        engine: prepared.receipt.engine.clone(),
        evidence: prepared.receipt.evidence.clone(),
        compatibility: c.input.compatibility.clone(),
        rendered: FileIdentity::of(&c.html),
    }
}
fn read(path: &Path) -> Result<Vec<u8>, Failure> {
    std::fs::read(path).map_err(|e| fail("compare/input", format!("{}: {e}", path.display()), 1))
}
pub fn run(request: &CompareRequest<'_>) -> CompareReceipt {
    let mut receipt = empty();
    let attempt = (|| {
        if request.base.kind != DiagramType::Architecture
            || request.head.kind != DiagramType::Architecture
        {
            return Err(fail(
                "compare/type",
                "Both comparison inputs must be architecture",
                2,
            ));
        }
        let html_path = path(request.output)?;
        let receipt_path =
            path(&request.receipt.map(Path::to_owned).unwrap_or_else(|| {
                PathBuf::from(format!("{}.compare.json", html_path.display()))
            }))?;
        if receipt_path.parent() != html_path.parent() {
            return Err(fail(
                "compare/output",
                "The comparison receipt must sit beside the HTML",
                2,
            ));
        }
        let base = delivery::prepare::prepare_bytes(&request.base, &read(request.base.candidate)?);
        let head = delivery::prepare::prepare_bytes(&request.head, &read(request.head.candidate)?);
        for (name, prepared) in [("base", &base), ("head", &head)] {
            if prepared.compiled.is_none() {
                let mut diagnostic=Diagnostic::error(Some("architecture"),"compare/input",name,"Comparison requires both inputs to pass their complete compile and artifact checks");
                diagnostic.stage = "compare".into();
                diagnostic.evidence =
                    serde_json::to_value(&prepared.receipt).expect("validation receipt");
                receipt.diagnostics.push(diagnostic);
                receipt.exit = receipt.exit.max(prepared.receipt.exit_code());
            }
        }
        if base.compiled.is_none() || head.compiled.is_none() {
            return Ok(());
        }
        let before = base.compiled.as_ref().unwrap();
        let after = head.compiled.as_ref().unwrap();
        receipt.base = Some(side(&base));
        receipt.head = Some(side(&head));
        let base_doc: Value =
            serde_json::from_slice(&before.input.source_bytes).expect("validated JSON");
        let head_doc: Value =
            serde_json::from_slice(&after.input.source_bytes).expect("validated JSON");
        let ir=canonical::compare_architecture(&base_doc,&head_doc,&json!({"baseVerified":before.evidence.status==crate::evidence::EvidenceStatus::Verified,"headVerified":after.evidence.status==crate::evidence::EvidenceStatus::Verified,"baseRawSha256":before.input.source_sha256,"headRawSha256":after.input.source_sha256,"baseSemanticSha256":receipt.base.as_ref().unwrap().semantic_sha256,"headSemanticSha256":receipt.head.as_ref().unwrap().semantic_sha256,"baseBytes":before.input.source_bytes.len(),"headBytes":after.input.source_bytes.len()})).map_err(|diagnostics|Failure {diagnostics,exit:1})?;
        let ir_bytes = serde_json::to_vec(&ir).expect("comparison IR");
        let receipt_filename = receipt_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| fail("compare/output", "The receipt needs a UTF-8 filename", 2))?;
        let html = render::render(
            &before.scene,
            &after.scene,
            &before.html,
            &after.html,
            &ir,
            receipt_filename,
        )
        .map_err(|diagnostics| Failure {
            diagnostics,
            exit: 1,
        })?;
        receipt.changes = ir["changes"].clone();
        receipt.summary = ir["summary"].clone();
        receipt.limitations = ir["limitations"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        receipt.comparison_ir = Some(ComparisonIr {
            identity: FileIdentity::of(&ir_bytes),
            document: ir,
        });
        receipt.artifact = Some(crate::browser::BoundFile {
            path: html_path.clone(),
            identity: FileIdentity::of(&html),
        });
        receipt.checks = verification::check_structure(&html, &receipt)?;
        receipt.status = Status::Pass;
        receipt.exit = 0;
        write(&html_path, &html)?;
        write(&receipt_path, &json_bytes(&receipt))
    })();
    if let Err(error) = attempt {
        receipt.status = Status::Failed;
        receipt.exit = error.exit;
        receipt.diagnostics.extend(error.diagnostics);
    }
    receipt
}
