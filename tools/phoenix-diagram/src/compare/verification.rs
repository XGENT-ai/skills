use super::*;
use crate::{
    browser::BoundFile,
    check::{check_html, ArtifactCheckEntry, FileIdentity},
    delivery::{EngineIdentity, Status},
    input::{parse_input, sha256, CompileOptions, DiagramType},
    output::FileBinding,
};
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn is_comparison(output: &Path) -> bool {
    FileBinding::capture(output)
        .ok()
        .and_then(|f| String::from_utf8(f.bytes().to_vec()).ok())
        .is_some_and(|s| {
            Html::parse_document(&s)
                .select(&Selector::parse("script#phoenix-diagram-comparison").unwrap())
                .next()
                .is_some()
        })
}
fn metadata(bytes: &[u8]) -> Result<Value, Failure> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| crate::delivery::metadata::embedded(&Html::parse_document(s)))
        .ok_or_else(|| fail("compare/metadata", "Subview metadata is missing", 1))
}
fn checked_entry(name: impl Into<String>, scope: Value) -> ArtifactCheckEntry {
    ArtifactCheckEntry {
        name: name.into(),
        status: "pass".into(),
        scope,
    }
}
pub(super) fn check_structure(
    html: &[u8],
    receipt: &CompareReceipt,
) -> Result<Vec<ArtifactCheckEntry>, Failure> {
    let parsed = render::parse(html).map_err(|diagnostics| Failure {
        diagnostics,
        exit: 1,
    })?;
    let ir = receipt
        .comparison_ir
        .as_ref()
        .ok_or_else(|| fail("compare/ir", "Comparison IR is missing", 1))?;
    if receipt.receipt_id != parsed.receipt_id
        || parsed.ir != ir.document
        || FileIdentity::of(&serde_json::to_vec(&ir.document).unwrap()) != ir.identity
        || receipt.changes != ir.document["changes"]
        || receipt.summary != ir.document["summary"]
    {
        return Err(fail(
            "compare/ir",
            "Immutable comparison identity or IR differs",
            1,
        ));
    }
    let mut checks = vec![];
    for (view, side) in [
        ("before", receipt.base.as_ref()),
        ("after", receipt.head.as_ref()),
    ] {
        let side = side.ok_or_else(|| {
            fail(
                "compare/input-binding",
                "Both comparison sides are required",
                1,
            )
        })?;
        let bytes = parsed
            .views
            .get(view)
            .ok_or_else(|| fail("compare/views", "Missing comparison view", 1))?;
        if FileIdentity::of(bytes) != side.rendered {
            return Err(fail(
                "compare/view-binding",
                "Before/After bytes differ from the compiled snapshot",
                1,
            ));
        }
        let result = check_html(bytes, side.config.quality);
        if result.status != "pass" || result.checks.iter().any(|c| c.status != "pass") {
            return Err(Failure {
                diagnostics: result.diagnostics,
                exit: 1,
            });
        }
        for check in result.checks {
            checks.push(checked_entry(
                format!("{view}/{}", check.name),
                json!({"viewId":view,"quality":side.config.quality,"measurement":check.scope}),
            ));
        }
    }
    let delta = parsed
        .views
        .get("delta")
        .ok_or_else(|| fail("compare/views", "Missing Delta view", 1))?;
    let delta_check = check_html(
        delta,
        receipt.head.as_ref().expect("checked head").config.quality,
    );
    let required = [
        "single_svg",
        "diagram_contract",
        "finite_svg",
        "semantic_transforms",
        "svg_path_data",
    ];
    let scopes = delta_check
        .checks
        .iter()
        .filter(|c| required.contains(&c.name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if scopes.len() != required.len() || scopes.iter().any(|c| c.status != "pass") {
        return Err(fail(
            "compare/delta-safety",
            "Delta fails finite geometry, contract or path safety",
            1,
        ));
    }
    checks.push(checked_entry(
        "delta/geometry-and-path-safety",
        json!({"viewId":"delta","scope":"comparison-overlay","checks":scopes}),
    ));
    let mut states: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (collection, prefix, id_field) in [
        ("components", "component", "id"),
        ("connections", "relationship", "id"),
        ("boundaries", "boundary", "key"),
    ] {
        for item in ir.document["changes"][collection]
            .as_array()
            .ok_or_else(|| fail("compare/ir", "Invalid changes collection", 1))?
        {
            let id = item[id_field]
                .as_str()
                .ok_or_else(|| fail("compare/ir", "Change identity is missing", 1))?;
            let classes = item["classifications"]
                .as_array()
                .ok_or_else(|| fail("compare/ir", "Change classifications are missing", 1))?
                .iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| fail("compare/ir", "Invalid classification", 1))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(" ");
            states.insert(
                format!("{prefix}:{id}"),
                (item["status"].as_str().unwrap_or_default().into(), classes),
            );
        }
    }
    let changed = states.keys().cloned().collect::<BTreeSet<_>>();
    let after = metadata(&parsed.views["after"])?;
    for (collection, prefix) in [("nodes", "component"), ("relations", "relationship")] {
        for item in after["graph"][collection].as_array().into_iter().flatten() {
            if let Some(id) = item["id"].as_str() {
                states
                    .entry(format!("{prefix}:{id}"))
                    .or_insert(("same".into(), String::new()));
            }
        }
    }
    for b in after["facts"]["boundaries"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let (Some(kind), Some(label)) = (b["kind"].as_str(), b["label"].as_str()) {
            states
                .entry(format!("boundary:{kind}:{label}"))
                .or_insert(("same".into(), String::new()));
        }
    }
    let document = Html::parse_document(
        std::str::from_utf8(delta)
            .map_err(|_| fail("compare/encoding", "Invalid Delta encoding", 1))?,
    );
    let mut seen = BTreeSet::new();
    for element in document.select(&Selector::parse("[data-delta-key]").unwrap()) {
        let key = element.value().attr("data-delta-key").unwrap();
        let Some((state, classes)) = states.get(key) else {
            return Err(fail(
                "compare/delta-markers",
                "Delta contains an unknown change identity",
                1,
            ));
        };
        if element.value().attr("data-delta-state") != Some(state.as_str())
            || element
                .value()
                .attr("data-delta-classifications")
                .unwrap_or("")
                != classes
        {
            return Err(fail(
                "compare/delta-markers",
                "Delta markers disagree with the authored IR",
                1,
            ));
        }
        seen.insert(key.to_owned());
    }
    if !changed.is_subset(&seen) {
        return Err(fail(
            "compare/delta-markers",
            "Delta is missing authored changes",
            1,
        ));
    }
    let parent = Html::parse_document(
        std::str::from_utf8(html)
            .map_err(|_| fail("compare/encoding", "Invalid comparison encoding", 1))?,
    );
    if parent
        .select(
            &Selector::parse("script[src],link[href],img[src],iframe[src],object,embed").unwrap(),
        )
        .next()
        .is_some()
    {
        return Err(fail(
            "compare/offline",
            "Comparison contains a resource outside its embedded views",
            1,
        ));
    }
    checks.push(checked_entry("comparison/structure",json!({"viewIds":["before","delta","after"],"deltaMarkers":changed,"iframeNamespaces":true,"offline":true,"unchangedFacts":"retained in Before and After"})));
    Ok(checks)
}
fn expected(file: &crate::delivery::FileReference) -> Result<FileBinding, Failure> {
    let binding = FileBinding::capture(&file.path).map_err(from_output)?;
    if binding.identity() != &file.original_binding
        || FileIdentity::of(binding.bytes()) != file.identity
    {
        return Err(fail(
            "compare/input-changed",
            "An original comparison input or its physical identity changed",
            3,
        ));
    }
    Ok(binding)
}
fn validate_side(
    side: &CompareSide,
    source: &FileBinding,
    html: &[u8],
    engine: &EngineIdentity,
) -> Result<(Value, crate::scene::CompiledDiagram), Failure> {
    if &side.engine != engine
        || side.config.diagram_type != DiagramType::Architecture
        || side.config.viewer_revision != engine.viewer_revision
        || FileIdentity::of(source.bytes()) != side.raw
    {
        return Err(fail(
            "compare/engine-binding",
            "Input or engine identity differs from the comparison",
            1,
        ));
    }
    let options = CompileOptions {
        input_contract: Some(side.config.input_contract),
        quality: Some(side.config.quality),
        viewer_revision: side.config.viewer_revision.clone(),
    };
    let mut input = parse_input(source.bytes(), DiagramType::Architecture, &options).map_err(
        |diagnostics| Failure {
            diagnostics,
            exit: 1,
        },
    )?;
    let raw_doc = input.document.clone();
    if sha256(&canonical::canonical_json(&raw_doc)) != side.semantic_sha256 {
        return Err(fail(
            "compare/semantic-binding",
            "Comparison semantic digest differs from the source",
            1,
        ));
    }
    let meta = metadata(html)?;
    let resources = meta
        .get("brandResources")
        .cloned()
        .map(serde_json::from_value::<crate::brands::ResolvedBrands>)
        .transpose()
        .map_err(|_| {
            fail(
                "compare/resources",
                "Invalid embedded resource identities",
                1,
            )
        })?
        .unwrap_or_default();
    let resources = crate::brands::bound(&input, &resources).map_err(|diagnostics| Failure {
        diagnostics,
        exit: 1,
    })?;
    input.bind_brands(resources);
    crate::brands::sanitize_document(&mut input.document, input.diagram_type);
    let evidence =
        crate::evidence::bound(&input, &meta).map_err(|e| fail("compare/evidence", e, 1))?;
    input.bind_evidence(evidence.digest().as_deref());
    let resource_ids = input
        .brands
        .iter()
        .map(|(path, b)| (path.clone(), b.resource.clone()))
        .collect::<BTreeMap<_, _>>();
    if input.config_sha256 != side.config_sha256
        || input.compatibility != side.compatibility
        || resource_ids != side.config.brand_resources
        || side.evidence != evidence.receipt()
        || !crate::delivery::metadata::matches(html, &input)
    {
        return Err(fail(
            "compare/config-binding",
            "Comparison config, evidence, resources or authored facts differ",
            1,
        ));
    }
    let scene = crate::layout::compile(&input, &options).map_err(|diagnostics| Failure {
        diagnostics,
        exit: 1,
    })?;
    Ok((raw_doc, scene))
}
pub(super) struct Current {
    pub files: Vec<FileBinding>,
    pub binding: ComparisonBinding,
    pub checks: Vec<ArtifactCheckEntry>,
}
impl Current {
    pub fn verify(&self) -> Result<(), Failure> {
        for file in &self.files {
            file.verify().map_err(from_output)?;
        }
        Ok(())
    }
}
fn side_binding(side: &CompareSide) -> ComparisonSideBinding {
    ComparisonSideBinding {
        candidate: side.candidate.bound_file(),
        semantic_sha256: side.semantic_sha256.clone(),
        config_sha256: side.config_sha256.clone(),
        engine: side.engine.clone(),
        resources: side.config.brand_resources.clone(),
    }
}
pub(super) fn current(output: &Path, engine: &EngineIdentity) -> Result<Current, Failure> {
    let html = FileBinding::capture(output).map_err(from_output)?;
    let parsed = render::parse(html.bytes()).map_err(|diagnostics| Failure {
        diagnostics,
        exit: 1,
    })?;
    let name = Path::new(&parsed.receipt_filename);
    if name.components().count() != 1
        || name.file_name().and_then(|n| n.to_str()) != Some(parsed.receipt_filename.as_str())
    {
        return Err(fail(
            "compare/receipt-path",
            "Comparison receipt must be a safe relative filename",
            1,
        ));
    }
    let receipt_file =
        FileBinding::capture(&html.path().parent().unwrap().join(name)).map_err(from_output)?;
    let receipt: CompareReceipt = serde_json::from_slice(receipt_file.bytes())
        .map_err(|e| fail("compare/receipt", e.to_string(), 1))?;
    if receipt.schema_version != 1
        || receipt.tool != "phoenix-diagram"
        || receipt.tool_version != crate::TOOL_VERSION
        || receipt.command != "compare"
        || receipt.kind != "architecture-comparison"
        || receipt.status != Status::Pass
        || receipt.comparator_version != 1
        || receipt.canonical_version != 1
        || receipt.receipt_id != parsed.receipt_id
    {
        return Err(fail(
            "compare/receipt",
            "Unsupported or uncommitted comparison receipt",
            1,
        ));
    }
    let artifact = BoundFile {
        path: html.path().to_owned(),
        identity: FileIdentity::of(html.bytes()),
    };
    if receipt.artifact.as_ref() != Some(&artifact) {
        return Err(fail(
            "compare/artifact-binding",
            "Comparison HTML differs from its immutable receipt",
            1,
        ));
    }
    let directory = receipt.transaction["directory"]
        .as_str()
        .ok_or_else(|| fail("compare/transaction", "The pair journal is missing", 1))?;
    transaction::verify_committed(Path::new(directory), &html, &receipt_file)
        .map_err(from_output)?;
    let base = receipt
        .base
        .as_ref()
        .ok_or_else(|| fail("compare/input-binding", "Base input is missing", 1))?;
    let head = receipt
        .head
        .as_ref()
        .ok_or_else(|| fail("compare/input-binding", "Head input is missing", 1))?;
    let base_file = expected(&base.candidate)?;
    let head_file = expected(&head.candidate)?;
    let identities = [&html, &receipt_file, &base_file, &head_file]
        .map(|f| (f.identity().device.clone(), f.identity().inode.clone()))
        .into_iter()
        .collect::<BTreeSet<_>>();
    if identities.len() != 4 {
        return Err(fail(
            "compare/file-alias",
            "Comparison inputs and both immutable outputs must be distinct files",
            3,
        ));
    }
    let (base_doc, base_scene) = validate_side(base, &base_file, &parsed.views["before"], engine)?;
    let (head_doc, head_scene) = validate_side(head, &head_file, &parsed.views["after"], engine)?;
    let ir=canonical::compare_architecture(&base_doc,&head_doc,&json!({"baseVerified":base.evidence["status"]=="verified","headVerified":head.evidence["status"]=="verified","baseRawSha256":base.raw.sha256,"headRawSha256":head.raw.sha256,"baseSemanticSha256":base.semantic_sha256,"headSemanticSha256":head.semantic_sha256,"baseBytes":base.raw.bytes,"headBytes":head.raw.bytes})).map_err(|diagnostics|Failure {diagnostics,exit:1})?;
    if receipt.comparison_ir.as_ref().map(|i| &i.document) != Some(&ir) {
        return Err(fail(
            "compare/ir-binding",
            "Comparison IR cannot be reproduced from its bound inputs",
            1,
        ));
    }
    let reproduced = render::render(
        &base_scene,
        &head_scene,
        &parsed.views["before"],
        &parsed.views["after"],
        &ir,
        &parsed.receipt_filename,
        &parsed.receipt_id,
    )
    .map_err(|diagnostics| Failure {
        diagnostics,
        exit: 1,
    })?;
    if reproduced != html.bytes() {
        return Err(fail(
            "compare/render-binding",
            "Comparison markup cannot be reproduced from both bound snapshots",
            1,
        ));
    }
    let checks = check_structure(html.bytes(), &receipt)?;
    if checks != receipt.checks {
        return Err(fail(
            "compare/check-scope",
            "Original comparison check scopes differ",
            1,
        ));
    }
    let binding = ComparisonBinding {
        receipt: BoundFile {
            path: receipt_file.path().to_owned(),
            identity: FileIdentity::of(receipt_file.bytes()),
        },
        receipt_id: receipt.receipt_id.clone(),
        base: side_binding(base),
        head: side_binding(head),
        comparison_ir: receipt.comparison_ir.as_ref().unwrap().identity.clone(),
        artifact,
    };
    let current = Current {
        files: vec![html, receipt_file, base_file, head_file],
        binding,
        checks,
    };
    current.verify()?;
    Ok(current)
}
pub fn check(output: &Path, strict: bool, engine: &EngineIdentity) -> ComparisonCheckReceipt {
    let mut result = ComparisonCheckReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "check".into(),
        kind: "architecture-comparison-check".into(),
        status: Status::Failed,
        diagnostics: vec![],
        strict_provenance: strict,
        comparison: None,
        checks: vec![],
        exit: 1,
    };
    match current(output, engine) {
        Ok(current) => {
            result.status = Status::Pass;
            result.exit = 0;
            result.comparison = Some(current.binding);
            result.checks = current.checks;
        }
        Err(mut e) => {
            for d in &mut e.diagnostics {
                d.stage = "check".into();
            }
            result.diagnostics = e.diagnostics;
            result.exit = e.exit;
        }
    }
    result
}
