use super::support::*;
use super::*;
use crate::check::{check_html, FileIdentity};
use crate::input::parse_input;
use crate::output::FileBinding;
use std::path::Path;

pub(crate) struct Compiled {
    pub html: Vec<u8>,
    pub resources: Vec<FileBinding>,
    pub input: crate::input::DiagramInput,
    pub scene: crate::scene::CompiledDiagram,
    pub evidence: crate::evidence::SourceEvidence,
}

pub(crate) struct Preparation {
    pub receipt: ValidationReceipt,
    pub source: Option<FileBinding>,
    pub compiled: Option<Compiled>,
    /// The interface language the artifact resolves, for finalize receipts.
    pub locale: serde_json::Value,
}

pub(crate) fn prepare(request: &CompileRequest<'_>) -> Preparation {
    prepare_with_source(
        request,
        || FileBinding::capture(request.candidate).map(Some),
        None,
    )
}

/// Compare keeps both original descriptors alive and compiles those exact bytes.
pub(crate) fn prepare_captured(request: &CompileRequest<'_>, source: FileBinding) -> Preparation {
    prepare_with_source(request, || Ok(Some(source)), None)
}

/// The preview controller retains the original descriptor and owns publication.
/// Its same-engine worker compiles these bytes without reopening that pathname.
pub(crate) fn prepare_bytes(request: &CompileRequest<'_>, bytes: &[u8]) -> Preparation {
    prepare_with_source(request, || Ok(None), Some(bytes))
}

fn prepare_with_source(
    request: &CompileRequest<'_>,
    capture: impl FnOnce() -> crate::output::OutputResult<Option<FileBinding>>,
    snapshot: Option<&[u8]>,
) -> Preparation {
    let mut result = Preparation {
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
            engine: request.engine.clone(),
            compatibility: None,
            checks: vec![],
            composition: serde_json::Value::Null,
            evidence: serde_json::json!({}),
            layout: None,
            exit: 1,
        },
        source: None,
        compiled: None,
        locale: serde_json::json!({}),
    };
    let build = (|| {
        if !valid_engine(request.engine)
            || request.options.viewer_revision != request.engine.viewer_revision
        {
            return Err(fail("validate", "engine/identity", "Compile options and the current engine identity must match the supported contracts", 2));
        }
        let source = capture().map_err(|e| output_error("input", e))?;
        result.receipt.candidate = source.as_ref().map(reference);
        result.source = source;
        let bytes = result
            .source
            .as_ref()
            .map(FileBinding::bytes)
            .or(snapshot)
            .expect("file or private preview snapshot");
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
        let resources = crate::brands::resource_bindings(&brands, &cache)
            .map_err(|e| fail("brands", e.code, e.message.clone(), e.exit_code()))?;
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
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
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
        if let Some(source) = result.source.as_ref() {
            source.verify().map_err(|e| {
                let mut failure = output_error("input", e);
                failure.diagnostic.code = "input/changed".into();
                failure
            })?;
        }
        if passed_check(&checked) {
            result.receipt.status = Status::Pass;
            result.receipt.exit = 0;
            result.compiled = Some(Compiled {
                html,
                resources,
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
