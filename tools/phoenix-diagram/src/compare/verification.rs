use super::*;
use crate::{
    check::{check_html, ArtifactCheckEntry, FileIdentity},
    delivery::Status,
};
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn is_comparison(output: &Path) -> bool {
    std::fs::read(output)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
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
    if parsed.ir != ir.document
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
/// Re-run the comparison artifact checks on the written HTML.
pub fn check(output: &Path) -> ComparisonCheckReceipt {
    let mut result = ComparisonCheckReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "check".into(),
        kind: "architecture-comparison-check".into(),
        status: Status::Failed,
        diagnostics: vec![],
        checks: vec![],
        exit: 1,
    };
    let checked = super::review::summary(output)
        .and_then(|(_, html, receipt)| check_structure(&html, &receipt));
    match checked {
        Ok(checks) => {
            result.status = Status::Pass;
            result.exit = 0;
            result.checks = checks;
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
