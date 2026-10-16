use super::support::*;
use super::*;
use crate::check::{check_html, FileIdentity};
use crate::input::parse_input;
use crate::output::FileBinding;

pub(super) struct Compiled {
    pub html: Vec<u8>,
}

pub(super) struct Preparation {
    pub receipt: ValidationReceipt,
    pub source: Option<FileBinding>,
    pub compiled: Option<Compiled>,
}

pub(super) fn prepare(request: &CompileRequest<'_>) -> Preparation {
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
            layout: None,
            exit: 1,
        },
        source: None,
        compiled: None,
    };
    let build = (|| {
        if !valid_engine(request.engine)
            || request.options.viewer_revision != request.engine.viewer_revision
        {
            return Err(fail("validate", "engine/identity", "Compile options and the current engine identity must match the supported contracts", 2));
        }
        let source =
            FileBinding::capture(request.candidate).map_err(|e| output_error("input", e))?;
        result.receipt.candidate = Some(reference(&source));
        result.source = Some(source);
        let source = result.source.as_ref().expect("captured source");
        let input = match parse_input(source.bytes(), request.kind, request.options) {
            Ok(input) => input,
            Err(diagnostics) => {
                result.receipt.diagnostics = diagnostics;
                return Ok(());
            }
        };
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
        let html = match crate::render::render(&scene, request.assets) {
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
        source.verify().map_err(|e| {
            let mut failure = output_error("input", e);
            failure.diagnostic.code = "input/changed".into();
            failure
        })?;
        if passed_check(&checked) {
            result.receipt.status = Status::Pass;
            result.receipt.exit = 0;
            result.compiled = Some(Compiled { html });
        }
        Ok(())
    })();
    if let Err(failure) = build {
        result.receipt.diagnostics.push(failure.diagnostic);
        result.receipt.exit = failure.exit;
    }
    result
}
