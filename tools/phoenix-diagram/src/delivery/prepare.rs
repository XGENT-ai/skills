use super::support::*;
use super::*;
use crate::check::{check_html, FileIdentity};
use crate::input::{parse_input, DiagramType};
use std::path::Path;

pub(crate) struct Compiled {
    pub html: Vec<u8>,
    pub input: crate::input::DiagramInput,
    pub scene: crate::scene::CompiledDiagram,
    pub evidence: crate::evidence::SourceEvidence,
}

pub(crate) struct Preparation {
    pub receipt: ValidationReceipt,
    pub compiled: Option<Compiled>,
    /// The interface language the artifact resolves, for finalize receipts.
    pub locale: serde_json::Value,
}

pub(crate) fn prepare(request: &CompileRequest<'_>) -> Preparation {
    match std::fs::read(request.candidate) {
        Ok(bytes) => prepare_bytes(request, &bytes),
        Err(error) => {
            let mut result = empty();
            let failure = fail(
                "input",
                "input/read",
                format!("{}: {error}", request.candidate.display()),
                1,
            );
            result.receipt.diagnostics.push(failure.diagnostic);
            result
        }
    }
}

fn empty() -> Preparation {
    Preparation {
        receipt: ValidationReceipt {
            schema_version: 1,
            tool: "phoenix-diagram".into(),
            tool_version: crate::TOOL_VERSION.into(),
            command: "validate".into(),
            status: Status::Failed,
            diagnostics: vec![],
            candidate: None,
            artifact: None,
            config: None,
            config_sha256: None,
            engine: EngineIdentity::current(),
            compatibility: None,
            checks: vec![],
            composition: serde_json::Value::Null,
            evidence: serde_json::json!({}),
            model: serde_json::Value::Null,
            layout: None,
            exit: 1,
        },
        compiled: None,
        locale: serde_json::json!({}),
    }
}

/// Compile the candidate bytes already read from `request.candidate`.
pub(crate) fn prepare_bytes(request: &CompileRequest<'_>, bytes: &[u8]) -> Preparation {
    let mut result = empty();
    let build = (|| -> Result<()> {
        result.receipt.candidate = Some(reference(request.candidate, bytes)?);
        let mut input = match parse_input(bytes, request.kind, request.options) {
            Ok(input) => input,
            Err(diagnostics) => {
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
        let cache = request
            .brand_cache
            .map(Path::to_owned)
            .unwrap_or_else(crate::brands::default_cache);
        let brands = match crate::brands::prepare(&input, &cache) {
            Ok(brands) => brands,
            Err(diagnostics) => {
                result.receipt.exit = if diagnostics.iter().any(|d| d.code == "brand/cache-missing")
                {
                    5
                } else {
                    1
                };
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
        input.bind_brands(brands);
        crate::brands::sanitize_document(&mut input.document, input.diagram_type);
        let evidence = match crate::evidence::verify(&input, request.repo_root) {
            Ok(evidence) => evidence,
            Err(diagnostics) => {
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
        input.bind_evidence(evidence.digest().as_deref());
        let locale = crate::i18n::resolve(&input.document["meta"]);
        result.locale = serde_json::json!({"requested":locale.requested,"resolved":locale.resolved,"fallback":locale.fallback});
        result.receipt.evidence = evidence.receipt();
        result.receipt.config = Some(config(&input));
        result.receipt.config_sha256 = Some(input.config_sha256.clone());
        result.receipt.compatibility = Some(input.compatibility.clone());
        let scene = match crate::layout::compile(&input, request.options) {
            Ok(scene) => scene,
            Err(diagnostics) => {
                // Rule verdicts do not depend on the drawing, so a chart that
                // cannot be laid out still reports them.
                if input.diagram_type == DiagramType::Statechart {
                    result.receipt.model = crate::statechart::verdict(&input.document);
                }
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
        if let crate::scene::DiagramFacts::Statechart { model, .. } = &scene.facts {
            result.receipt.model = model.clone();
        }
        let html = match crate::render::render_with_evidence(&scene, request.assets, &evidence) {
            Ok(html) => html,
            Err(diagnostics) => {
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
        let checked = check_html(&html, input.quality);
        result.receipt.artifact = Some(FileIdentity::of(&html));
        result.receipt.checks = checked.checks.clone();
        result.receipt.composition = checked.composition.clone();
        result.receipt.diagnostics = scene.diagnostics.clone();
        result
            .receipt
            .diagnostics
            .extend(checked.diagnostics.clone());
        result.receipt.layout =
            Some(serde_json::to_value(&scene).expect("compiled scene serializes"));
        if passed_check(&checked) {
            result.receipt.status = Status::Pass;
            result.receipt.exit = 0;
            result.compiled = Some(Compiled {
                html,
                input,
                scene,
                evidence,
            });
        }
        Ok(())
    })();
    if let Err(failure) = build {
        result.receipt.diagnostics.push(failure.diagnostic);
        result.receipt.exit = failure.exit;
    }
    result
}
