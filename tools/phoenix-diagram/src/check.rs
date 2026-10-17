use crate::diagnostic::{Diagnostic, Diagnostics, Severity};
use crate::geometry::{self, composition as geometry_checks};
use crate::input::{sha256, DiagramType, InputContract, Quality};
use crate::scene::{Point, Rect};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub mod anchors;
pub mod path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileIdentity {
    pub sha256: String,
    pub bytes: u64,
}

impl FileIdentity {
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            sha256: sha256(bytes),
            bytes: bytes.len() as u64,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ArtifactCheckEntry {
    pub name: String,
    pub status: String,
    pub scope: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ArtifactCheck {
    pub status: String,
    pub artifact: FileIdentity,
    pub quality: Quality,
    pub checks: Vec<ArtifactCheckEntry>,
    pub composition: Value,
    pub diagnostics: Diagnostics,
}

struct Inspection {
    result: ArtifactCheck,
    kind: Option<DiagramType>,
    metrics: Value,
}

impl Inspection {
    fn diagnostic(
        &self,
        code: &str,
        path: &str,
        message: impl Into<String>,
        evidence: Value,
    ) -> Diagnostic {
        let mut diagnostic =
            Diagnostic::error(self.kind.map(DiagramType::as_str), code, path, message);
        diagnostic.stage = "artifact".into();
        diagnostic.evidence = evidence;
        diagnostic
    }

    fn record(&mut self, name: &str, scope: Value, diagnostics: Diagnostics) {
        let failed = diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error);
        self.result.checks.push(ArtifactCheckEntry {
            name: name.into(),
            status: if failed { "failed" } else { "pass" }.into(),
            scope,
        });
        self.result.diagnostics.extend(diagnostics);
    }

    fn hits<T: Serialize>(
        &mut self,
        name: &str,
        code: &str,
        hits: &[T],
        scope: Value,
        always_error: bool,
    ) {
        let mut diagnostics = Vec::new();
        for (index, hit) in hits.iter().enumerate() {
            let mut diagnostic = self.diagnostic(
                code,
                &format!("/svg/{name}/{index}"),
                format!("Rendered geometry fails {name}"),
                serde_json::to_value(hit).expect("serializable geometry evidence"),
            );
            if !always_error && self.result.quality == Quality::Standard {
                diagnostic.severity = Severity::Warning;
            }
            diagnostics.push(diagnostic);
        }
        self.record(name, scope, diagnostics);
        self.metrics[name] = serde_json::json!(hits.len());
    }

    fn finish(mut self) -> ArtifactCheck {
        let errors = self
            .result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        let warnings = self
            .result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count();
        self.result.status =
            if errors == 0 && (self.result.quality == Quality::Standard || warnings == 0) {
                "pass"
            } else {
                "failed"
            }
            .into();
        self.result.composition = serde_json::json!({"schemaVersion":1,"profile":self.result.quality,"status":self.result.status,"summary":{"errors":errors,"warnings":warnings},"metrics":self.metrics});
        self.result
    }
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static selector")
}

fn number(element: ElementRef<'_>, attribute: &str, default: f64) -> f64 {
    element
        .value()
        .attr(attribute)
        .map(|value| {
            value
                .strip_suffix('%')
                .unwrap_or(value)
                .parse()
                .unwrap_or(f64::NAN)
        })
        .unwrap_or(default)
}

fn rect(element: ElementRef<'_>) -> Rect {
    Rect {
        x: number(element, "x", 0.0),
        y: number(element, "y", 0.0),
        width: number(element, "width", f64::NAN),
        height: number(element, "height", f64::NAN),
    }
}

fn has_transform(element: ElementRef<'_>) -> bool {
    std::iter::once(element)
        .chain(element.ancestors().filter_map(ElementRef::wrap))
        .any(|ancestor| {
            ancestor
                .value()
                .attr("transform")
                .is_some_and(|value| !value.trim().is_empty())
        })
}

fn route_points(value: &str) -> Option<Vec<Point>> {
    value
        .split(';')
        .map(|pair| {
            let (x, y) = pair.split_once(',')?;
            let point = Point {
                x: x.parse().ok()?,
                y: y.parse().ok()?,
            };
            geometry::finite_point(point).then_some(point)
        })
        .collect()
}

fn same_point(left: Point, right: Point) -> bool {
    (left.x - right.x).abs() < 0.00001 && (left.y - right.y).abs() < 0.00001
}

fn path_matches_declaration(
    actual: &path::PathAnalysis,
    points: &[Point],
    kind: Option<DiagramType>,
    workflow_v2: bool,
    authored: Option<&Value>,
) -> bool {
    let expected = match kind {
        Some(DiagramType::Architecture) => geometry::rounded_path(points, 8.0),
        Some(DiagramType::Workflow) if workflow_v2 => {
            let mut radius: f64 = 8.0;
            for (i, pair) in points.windows(2).enumerate() {
                let corners = usize::from(i > 0) + usize::from(i + 2 < points.len());
                if corners > 0 {
                    radius = radius.min(
                        (((pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y) - 8.0)
                            / corners as f64)
                            .max(0.0),
                    );
                }
            }
            geometry::rounded_path(points, radius)
        }
        Some(DiagramType::Lifecycle) if workflow_v2 => geometry::lifecycle_v3_path(points),
        Some(DiagramType::Lifecycle) => geometry::rounded_path(
            points,
            authored
                .and_then(|v| v["cornerRadius"].as_f64())
                .unwrap_or(10.0),
        ),
        _ => geometry::polyline_path(points),
    };
    let Ok(expected) = path::analyze_svg_path(&expected) else {
        return false;
    };
    actual.subpaths.len() == expected.subpaths.len()
        && actual
            .subpaths
            .iter()
            .zip(expected.subpaths)
            .all(|(actual, expected)| {
                actual.closed == expected.closed
                    && actual.points.len() == expected.points.len()
                    && actual
                        .points
                        .iter()
                        .zip(expected.points)
                        .all(|(a, b)| same_point(*a, b))
            })
}

fn visible_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| !geometry::text::is_js_whitespace(*c))
        .collect()
}

fn edge_class(kind: Option<DiagramType>, variant: &str) -> &'static str {
    match variant {
        "emphasis" => "a-emphasis",
        "security" => "a-security",
        "dashed" => "a-dashed",
        "return" if kind != Some(DiagramType::Sequence) => "a-return",
        _ => "a-default",
    }
}

fn verified_crossover_halo(edge: ElementRef<'_>) -> bool {
    if edge.value().attr("data-composition-crossover") != Some("halo") {
        return false;
    }
    let mut previous = edge.prev_sibling();
    while let Some(node) = previous {
        if let Some(underlay) = ElementRef::wrap(node) {
            let width = number(edge, "stroke-width", f64::NAN);
            let mask_width = number(underlay, "stroke-width", f64::NAN);
            return underlay.value().name() == "path"
                && underlay.value().attr("data-graph-role")
                    == Some("automatic-crossover-underlay")
                && underlay.value().attr("d") == edge.value().attr("d")
                && underlay.value().attr("fill") == Some("none")
                && underlay.value().attr("stroke") == Some("var(--mask)")
                && underlay.value().attr("pointer-events") == Some("none")
                && width.is_finite()
                && mask_width.is_finite()
                && mask_width >= width + 3.0;
        }
        if node
            .value()
            .as_text()
            .is_none_or(|text| !text.trim().is_empty())
        {
            return false;
        }
        previous = node.prev_sibling();
    }
    false
}

pub fn check_html(bytes: &[u8], quality: Quality) -> ArtifactCheck {
    let mut inspection = Inspection {
        result: ArtifactCheck {
            status: "not-run".into(),
            artifact: FileIdentity::of(bytes),
            quality,
            checks: Vec::new(),
            composition: Value::Null,
            diagnostics: Vec::new(),
        },
        kind: None,
        metrics: serde_json::json!({}),
    };
    let source = match std::str::from_utf8(bytes) {
        Ok(source) => source,
        Err(error) => {
            let diagnostic =
                inspection.diagnostic("artifact/encoding", "", error.to_string(), Value::Null);
            inspection.record("utf8", Value::Null, vec![diagnostic]);
            return inspection.finish();
        }
    };
    let document = Html::parse_document(source);
    let svgs: Vec<_> = document.select(&selector("svg")).collect();
    let diagnostics = if svgs.len() == 1 {
        vec![]
    } else {
        vec![inspection.diagnostic(
            "artifact/single-svg",
            "/svg",
            "Exactly one main SVG is required",
            serde_json::json!({"found":svgs.len()}),
        )]
    };
    inspection.record(
        "single_svg",
        serde_json::json!({"found":svgs.len()}),
        diagnostics,
    );
    if svgs.len() != 1 {
        return inspection.finish();
    }
    let svg = svgs[0];
    inspection.kind = svg
        .value()
        .attr("data-diagram-type")
        .and_then(|kind| kind.parse().ok());
    let contract = match svg.value().attr("data-input-contract") {
        Some("phoenix-v1") => InputContract::PhoenixV1,
        _ => InputContract::Legacy217,
    };
    let mut contract_errors = Vec::new();
    if inspection.kind.is_none() {
        contract_errors.push(inspection.diagnostic(
            "artifact/diagram-type",
            "/svg/data-diagram-type",
            "SVG must declare a supported diagram type",
            Value::Null,
        ));
    }
    if !matches!(
        svg.value().attr("data-input-contract"),
        Some("phoenix-v1" | "legacy-2.17")
    ) {
        contract_errors.push(inspection.diagnostic(
            "artifact/input-contract",
            "/svg/data-input-contract",
            "SVG input contract is unknown",
            Value::Null,
        ));
    }
    let profile = match quality {
        Quality::Standard => "standard",
        Quality::Showcase => "showcase",
    };
    if svg.value().attr("data-quality-profile") != Some(profile) {
        contract_errors.push(inspection.diagnostic(
            "artifact/quality-binding",
            "/svg/data-quality-profile",
            "Requested and rendered quality profiles differ",
            Value::Null,
        ));
    }
    inspection.record(
        "diagram_contract",
        serde_json::json!({"contract":contract,"quality":quality}),
        contract_errors,
    );
    let view_box: Vec<f64> = svg
        .value()
        .attr("viewBox")
        .or_else(|| svg.value().attr("viewbox"))
        .unwrap_or("")
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|value| !value.is_empty())
        .map(|value| value.parse().unwrap_or(f64::NAN))
        .collect();
    let mut finite_diagnostics = Vec::new();
    if view_box.len() != 4
        || view_box.iter().any(|value| !value.is_finite())
        || view_box.get(2).is_none_or(|value| *value <= 0.0)
        || view_box.get(3).is_none_or(|value| *value <= 0.0)
    {
        finite_diagnostics.push(inspection.diagnostic(
            "artifact/viewbox",
            "/svg/viewBox",
            "Canvas must have four finite values and positive size",
            Value::Null,
        ));
    }
    let numeric = [
        "x",
        "y",
        "x1",
        "y1",
        "x2",
        "y2",
        "width",
        "height",
        "cx",
        "cy",
        "r",
        "rx",
        "ry",
        "font-size",
        "stroke-width",
        "opacity",
        "fill-opacity",
        "stroke-opacity",
        "markerWidth",
        "markerHeight",
        "refX",
        "refY",
    ];
    for (index, element) in std::iter::once(svg)
        .chain(svg.select(&selector("*")))
        .enumerate()
    {
        for (name, value) in element.value().attrs() {
            if numeric
                .iter()
                .any(|attribute| name.eq_ignore_ascii_case(attribute))
                && !value
                    .strip_suffix('%')
                    .unwrap_or(value)
                    .parse::<f64>()
                    .is_ok_and(f64::is_finite)
            {
                finite_diagnostics.push(inspection.diagnostic(
                    "artifact/non-finite",
                    &format!("/svg/elements/{index}/{name}"),
                    "Numeric SVG attribute is not finite",
                    serde_json::json!({"value":value}),
                ));
            }
            if ["d", "points", "transform", "viewbox", "stroke-dasharray"]
                .iter()
                .any(|attribute| name.eq_ignore_ascii_case(attribute))
                && value
                    .split(|character: char| !character.is_ascii_alphabetic())
                    .any(|token| matches!(token, "NaN" | "Infinity" | "undefined"))
            {
                finite_diagnostics.push(inspection.diagnostic(
                    "artifact/non-finite",
                    &format!("/svg/elements/{index}/{name}"),
                    "SVG geometry contains a non-finite token",
                    Value::Null,
                ));
            }
        }
    }
    inspection.record(
        "finite_svg",
        serde_json::json!({"viewBox":view_box}),
        finite_diagnostics,
    );
    if inspection
        .result
        .checks
        .iter()
        .any(|entry| entry.status == "failed")
    {
        return inspection.finish();
    }
    let canvas = Rect {
        x: view_box[0],
        y: view_box[1],
        width: view_box[2],
        height: view_box[3],
    };
    let metadata = document
        .select(&selector("script#phoenix-diagram-metadata"))
        .next()
        .and_then(|script| serde_json::from_str::<Value>(&script.text().collect::<String>()).ok());
    let mut nodes = Vec::new();
    let mut node_labels = HashMap::new();
    let mut node_errors = Vec::new();
    let mut content_errors = Vec::new();
    let mut ids = HashSet::new();
    let mut transform_errors = Vec::new();
    let mut readability_errors = Vec::new();
    for (index, node) in svg.select(&selector("g[data-node-id]")).enumerate() {
        let id = node.value().attr("data-node-id").unwrap_or("").to_owned();
        if !ids.insert(id.clone()) {
            node_errors.push(inspection.diagnostic(
                "artifact/duplicate-id",
                &format!("/svg/nodes/{index}"),
                "Rendered node identity is duplicated",
                serde_json::json!({"id":id}),
            ));
        }
        let boxes: Vec<_> = node
            .children()
            .filter_map(ElementRef::wrap)
            .filter(|element| {
                element.value().name() == "rect"
                    && element.value().attr("data-graph-role") == Some("node-body")
                    && !element
                        .value()
                        .has_class("c-mask", scraper::CaseSensitivity::CaseSensitive)
            })
            .collect();
        if boxes.len() != 1 {
            node_errors.push(inspection.diagnostic(
                "artifact/node-box",
                &format!("/svg/nodes/{index}"),
                "Node must have one visible resolved box",
                serde_json::json!({"boxes":boxes.len()}),
            ));
            continue;
        }
        if has_transform(boxes[0]) {
            transform_errors.push(inspection.diagnostic(
                "artifact/semantic-transform",
                &format!("/svg/nodes/{index}"),
                "Semantic geometry must be in the main SVG coordinate system",
                Value::Null,
            ));
        }
        let bounds = rect(boxes[0]);
        if !geometry::finite_rect(&bounds) || bounds.width <= 0.0 || bounds.height <= 0.0 {
            node_errors.push(inspection.diagnostic(
                "artifact/node-box",
                &format!("/svg/nodes/{index}"),
                "Node box is invalid",
                Value::Null,
            ));
        }
        nodes.push((id.clone(), bounds));
        let texts: Vec<_> = node.select(&selector("text")).collect();
        node_labels.insert(
            id,
            texts
                .iter()
                .map(|text| text.text().collect::<String>())
                .collect::<Vec<_>>(),
        );
        for text in texts {
            if text.value().attr("data-detail") == Some("fine") {
                continue;
            }
            let projected = number(text, "font-size", 16.0) * (930.0 / canvas.width).min(1.0);
            if projected < 6.0 {
                let mut diagnostic = inspection.diagnostic(
                    "composition/desktop-readability",
                    &format!("/svg/nodes/{index}/text"),
                    "Node text projects below the readable floor",
                    serde_json::json!({"projectedPx":projected,"minimumPx":6.0}),
                );
                if quality == Quality::Standard {
                    diagnostic.severity = Severity::Warning;
                }
                readability_errors.push(diagnostic);
            }
        }
    }
    inspection.record(
        "node_geometry",
        serde_json::json!({"nodes":nodes.len()}),
        node_errors,
    );
    let mut containment = Vec::new();
    let mut overlap = Vec::new();
    let gap = if matches!(
        inspection.kind,
        Some(DiagramType::Dataflow | DiagramType::Lifecycle)
    ) {
        10.0
    } else {
        8.0
    };
    for (index, (id, bounds)) in nodes.iter().enumerate() {
        if bounds.x < canvas.x - geometry::EPSILON
            || bounds.y < canvas.y - geometry::EPSILON
            || bounds.x + bounds.width > canvas.x + canvas.width + geometry::EPSILON
            || bounds.y + bounds.height > canvas.y + canvas.height + geometry::EPSILON
        {
            containment.push(inspection.diagnostic(
                "composition/canvas-containment",
                &format!("/svg/nodes/{index}"),
                "Node exceeds the canvas",
                serde_json::json!({"id":id,"bounds":bounds,"canvas":canvas}),
            ));
        }
        for (other_id, other) in &nodes[index + 1..] {
            if geometry::rects_overlap_for_contract(bounds, other, gap, contract) {
                overlap.push(inspection.diagnostic(
                    "composition/node-overlap",
                    &format!("/svg/nodes/{index}"),
                    "Rendered node boxes overlap",
                    serde_json::json!({"left":id,"right":other_id,"gap":gap}),
                ));
            }
        }
    }
    inspection.record(
        "node_separation",
        serde_json::json!({"nodes":nodes.len(),"gap":gap}),
        overlap,
    );
    inspection.record(
        "canvas_containment",
        serde_json::json!({"nodes":nodes.len()}),
        containment,
    );
    inspection.record(
        "desktop_readability",
        serde_json::json!({"diagramWidthPx":930,"minimumPx":6}),
        readability_errors,
    );
    let mut actual_routes = Vec::new();
    let mut straight_routes = Vec::new();
    let mut path_errors = Vec::new();
    let mut binding_errors = Vec::new();
    let mut orthogonal_errors = Vec::new();
    let mut endpoint_errors = Vec::new();
    let mut edge_labels = HashMap::new();
    let mut edge_facts = Vec::new();
    for (index, edge) in svg
        .select(&selector("path[data-edge-from], line[data-edge-from]"))
        .enumerate()
    {
        if has_transform(edge) {
            transform_errors.push(inspection.diagnostic(
                "artifact/semantic-transform",
                &format!("/svg/relationships/{index}"),
                "Relationship geometry must use main SVG coordinates",
                Value::Null,
            ));
        }
        let mut relation = serde_json::json!({"from":edge.value().attr("data-edge-from"),"to":edge.value().attr("data-edge-to"),"id":edge.value().attr("data-edge-id"),"key":edge.value().attr("data-edge-key"),"label":edge.value().attr("data-edge-label"),"width":number(edge,"stroke-width",1.4),"junction":edge.value().attr("data-composition-junction")});
        relation["visibleClass"] = edge
            .value()
            .attr("class")
            .unwrap_or("")
            .split_whitespace()
            .find(|c| c.starts_with("a-"))
            .map(|v| Value::String(v.into()))
            .unwrap_or(Value::Null);
        let halo = verified_crossover_halo(edge);
        relation["verifiedHalo"] = Value::Bool(halo);
        relation["independentPorts"] =
            Value::Bool(halo && edge.value().attr("data-composition-independent") == Some("true"));
        for (field, attribute) in [
            ("fromSide", "data-authored-fromside"),
            ("toSide", "data-authored-toside"),
            ("role", "data-authored-role"),
            ("route", "data-authored-route"),
        ] {
            if let Some(value) = edge.value().attr(attribute) {
                relation[field] = Value::String(value.into());
            }
        }
        for endpoint in ["from", "to"] {
            if !relation[endpoint]
                .as_str()
                .is_some_and(|id| ids.contains(id))
            {
                endpoint_errors.push(inspection.diagnostic(
                    "artifact/unknown-endpoint",
                    &format!("/svg/relationships/{index}/{endpoint}"),
                    "Relationship endpoint is not a rendered node",
                    relation.clone(),
                ));
            }
        }
        let analysis = if edge.value().name() == "line" {
            let a = Point {
                x: number(edge, "x1", 0.0),
                y: number(edge, "y1", 0.0),
            };
            let b = Point {
                x: number(edge, "x2", 0.0),
                y: number(edge, "y2", 0.0),
            };
            path::analyze_svg_path(&format!("M {} {} L {} {}", a.x, a.y, b.x, b.y))
        } else {
            path::analyze_svg_path(edge.value().attr("d").unwrap_or(""))
        };
        let analysis = match analysis {
            Ok(analysis) => analysis,
            Err(error) => {
                path_errors.push(inspection.diagnostic(
                    error.code,
                    &format!("/svg/relationships/{index}/d"),
                    error.to_string(),
                    serde_json::json!({"offset":error.token_offset,"command":error.command}),
                ));
                continue;
            }
        };
        let declared = edge
            .value()
            .attr("data-composition-points")
            .and_then(route_points);
        let actual_start = analysis
            .subpaths
            .first()
            .and_then(|part| part.points.first())
            .copied();
        let actual_end = analysis
            .subpaths
            .last()
            .and_then(|part| part.points.last())
            .copied();
        if let Some(declared) = declared.as_ref() {
            if declared.len() < 2
                || actual_start
                    .zip(declared.first().copied())
                    .is_none_or(|(actual, claimed)| !same_point(actual, claimed))
                || actual_end
                    .zip(declared.last().copied())
                    .is_none_or(|(actual, claimed)| !same_point(actual, claimed))
                || !path_matches_declaration(
                    &analysis,
                    declared,
                    inspection.kind,
                    svg.value().attr("data-layout-contract") == Some("readable-v2")
                        || svg.value().attr("data-lifecycle-layout") == Some("main-path-v3"),
                    metadata
                        .as_ref()
                        .and_then(|m| m.pointer(&format!("/graph/relations/{index}"))),
                )
            {
                binding_errors.push(inspection.diagnostic(
                    "artifact/path-binding",
                    &format!("/svg/relationships/{index}/d"),
                    "Actual path differs from the complete declared route geometry",
                    relation.clone(),
                ));
            }
        } else {
            binding_errors.push(inspection.diagnostic(
                "artifact/path-binding",
                &format!("/svg/relationships/{index}"),
                "Relationship is missing finite route geometry",
                relation.clone(),
            ));
        }
        let authored_straight = edge.value().attr("data-composition-route") == Some("straight")
            && analysis.subpaths.len() == 1
            && analysis.subpaths[0].segments.len() == 1
            && analysis.subpaths[0].straight_segments.len() == 1;
        for part in &analysis.subpaths {
            for segment in &part.straight_segments {
                if (segment.start.x - segment.end.x).abs() > 0.01
                    && (segment.start.y - segment.end.y).abs() > 0.01
                    && !authored_straight
                {
                    orthogonal_errors.push(inspection.diagnostic(
                        "composition/orthogonal-arrows",
                        &format!("/svg/relationships/{index}/d"),
                        "Diagonal segment requires an explicitly authored direct straight route",
                        serde_json::json!({"start":segment.start,"end":segment.end}),
                    ));
                }
            }
        }
        let parts = analysis.route_subpaths();
        let part_count = parts.len();
        for (part_index, points) in parts.into_iter().enumerate() {
            if points.len() < 2 {
                continue;
            }
            let mut routed = geometry_checks::RoutedRelation::new(index, relation.clone(), points);
            routed.source_endpoint = part_index == 0;
            routed.target_endpoint = part_index + 1 == part_count;
            straight_routes.push(routed);
        }
        for (part_index, part) in analysis.subpaths.iter().enumerate() {
            let mut routed =
                geometry_checks::RoutedRelation::new(index, relation.clone(), part.points.clone());
            routed.source_endpoint = part_index == 0;
            routed.target_endpoint = part_index + 1 == analysis.subpaths.len();
            actual_routes.push(routed);
        }
        edge_facts.push(relation);
    }
    for group in svg.select(&selector("g[data-edge-key]")) {
        if let Some(key) = group.value().attr("data-edge-key") {
            edge_labels.insert(
                key.to_owned(),
                group
                    .select(&selector("text"))
                    .map(|text| text.text().collect::<String>())
                    .collect::<Vec<_>>(),
            );
        }
    }
    inspection.record(
        "semantic_transforms",
        serde_json::json!({"nodes":nodes.len(),"relationships":edge_facts.len()}),
        transform_errors,
    );
    inspection.record(
        "svg_path_data",
        serde_json::json!({"relationships":edge_facts.len()}),
        path_errors,
    );
    inspection.record(
        "route_binding",
        serde_json::json!({"relationships":edge_facts.len()}),
        binding_errors,
    );
    inspection.record(
        "orthogonal_arrows",
        serde_json::json!({"relationships":edge_facts.len()}),
        orthogonal_errors,
    );
    if let Some(metadata) = metadata.as_ref() {
        let actual_cards: Vec<_> = document.select(&selector(".cards > .card")).collect();
        let authored_cards = metadata["cards"].as_array();
        if authored_cards.is_none_or(|cards| cards.len() != actual_cards.len()) {
            content_errors.push(inspection.diagnostic(
                "artifact/semantic-card",
                "/metadata/cards",
                "Authored card count differs from actual HTML cards",
                serde_json::json!({"actual":actual_cards.len()}),
            ));
        }
        if let Some(cards) = authored_cards {
            for (index, (card, actual)) in cards.iter().zip(actual_cards).enumerate() {
                let title = actual
                    .select(&selector("h3"))
                    .map(|e| e.text().collect::<String>())
                    .collect::<Vec<_>>();
                let items: Vec<_> = actual
                    .select(&selector("li"))
                    .map(|e| e.text().collect::<String>())
                    .collect();
                let expected_items: Vec<_> = card["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                if title.len() != 1
                    || title[0] != card["title"].as_str().unwrap_or("")
                    || items.iter().map(String::as_str).collect::<Vec<_>>() != expected_items
                {
                    content_errors.push(inspection.diagnostic(
                        "artifact/semantic-card",
                        &format!("/metadata/cards/{index}"),
                        "Authored card text is missing or reordered in actual HTML",
                        serde_json::json!({"title":title,"items":items}),
                    ));
                }
            }
        }
        let authored_nodes = metadata.pointer("/graph/nodes").and_then(Value::as_array);
        let authored_relations = metadata
            .pointer("/graph/relations")
            .and_then(Value::as_array);
        if authored_nodes.is_none_or(|items| items.len() != nodes.len())
            || authored_relations.is_none_or(|items| items.len() != edge_facts.len())
        {
            content_errors.push(inspection.diagnostic(
                "artifact/semantic-count",
                "/metadata/graph",
                "Rendered entity/relation counts differ from native facts",
                Value::Null,
            ));
        }
        if let Some(authored_nodes) = authored_nodes {
            for node in authored_nodes {
                let id = node["id"].as_str().unwrap_or("");
                for field in ["label", "sublabel", "tag"] {
                    if let Some(text) = node[field].as_str().filter(|text| !text.is_empty()) {
                        if !node_labels.get(id).is_some_and(|texts| {
                            visible_text(&texts.join("")).contains(&visible_text(text))
                        }) {
                            content_errors.push(inspection.diagnostic(
                                "artifact/semantic-text",
                                "/metadata/graph/nodes",
                                "Authored node text is missing from actual SVG text",
                                serde_json::json!({"id":id,"field":field}),
                            ));
                        }
                    }
                }
            }
        }
        if let Some(authored_relations) = authored_relations {
            for (index, relation) in authored_relations.iter().enumerate() {
                let resolved = metadata.pointer(&format!("/presentation/relations/{index}"));
                let actual = edge_facts.get(index);
                let variant = relation["variant"]
                    .as_str()
                    .or_else(|| resolved.and_then(|v| v["variant"].as_str()))
                    .unwrap_or("default");
                if actual.is_none_or(|actual| {
                    ["from", "to", "id"]
                        .iter()
                        .any(|field| actual[field] != relation[field])
                        || actual["key"].as_str() != Some(index.to_string().as_str())
                        || actual["visibleClass"].as_str()
                            != Some(edge_class(inspection.kind, variant))
                }) {
                    content_errors.push(inspection.diagnostic("artifact/semantic-relation",&format!("/metadata/graph/relations/{index}"),"Actual relationship identity, direction or visible variant differs from its facts",serde_json::json!({"authored":relation,"actual":actual})));
                }
                let label_owner = resolved
                    .and_then(|v| v["labelOwner"].as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(index);
                for field in ["label", "note", "classification"] {
                    if let Some(text) = relation[field].as_str().filter(|text| !text.is_empty()) {
                        if !edge_labels
                            .get(&label_owner.to_string())
                            .is_some_and(|texts| {
                                visible_text(&texts.join("")).contains(&visible_text(text))
                            })
                        {
                            content_errors.push(inspection.diagnostic(
                                "artifact/semantic-text",
                                "/metadata/graph/relations",
                                "Authored relationship text is missing from actual SVG text",
                                serde_json::json!({"index":index,"field":field}),
                            ));
                        }
                    }
                }
            }
        }
    } else {
        content_errors.push(inspection.diagnostic(
            "artifact/metadata",
            "/metadata",
            "Native semantic metadata is missing or invalid",
            Value::Null,
        ));
    }
    inspection.record(
        "semantic_content",
        serde_json::json!({"nodes":nodes.len(),"relationships":edge_facts.len()}),
        content_errors,
    );
    let frames: Vec<_> = svg
        .select(&selector("[data-graph-role='structural-frame']"))
        .map(|element| {
            let mut frame = geometry_checks::Frame::new(
                element
                    .value()
                    .attr("data-composition-frame-kind")
                    .unwrap_or("boundary"),
                element
                    .value()
                    .attr("data-composition-frame-id")
                    .unwrap_or(""),
                rect(element),
            );
            frame.radius = number(element, "rx", 0.0);
            if element.value().name() == "line" {
                let start = Point {
                    x: number(element, "x1", 0.0),
                    y: number(element, "y1", 0.0),
                };
                let end = Point {
                    x: number(element, "x2", 0.0),
                    y: number(element, "y2", 0.0),
                };
                frame.line = Some((start, end));
                frame.bounds = Rect {
                    x: start.x.min(end.x),
                    y: start.y.min(end.y),
                    width: (end.x - start.x).abs(),
                    height: (end.y - start.y).abs(),
                };
            }
            frame
        })
        .collect();
    if let (Some(kind), Some(metadata)) = (inspection.kind, metadata.as_ref()) {
        endpoint_errors.extend(anchors::check_anchors(
            kind,
            svg,
            &nodes,
            &actual_routes,
            &frames,
            metadata,
        ));
    }
    inspection.record(
        "relationship_endpoints",
        serde_json::json!({"relationships":edge_facts.len()}),
        endpoint_errors,
    );
    let labels: Vec<_> = svg
        .select(&selector("g[data-edge-key]"))
        .filter_map(|element| {
            let index = element
                .value()
                .attr("data-edge-key")?
                .parse::<isize>()
                .ok()?;
            let plate = element
                .children()
                .filter_map(ElementRef::wrap)
                .find(|element| element.value().name() == "rect")?;
            let relation = edge_facts.get(index as usize)?.clone();
            Some(geometry_checks::LabelRect::new(
                index,
                relation,
                rect(plate),
            ))
        })
        .collect();
    let workflow_v2 = svg.value().attr("data-layout-contract") == Some("readable-v2");
    let shared_crossings = |left: &Value, right: &Value| {
        workflow_v2 || (left["independentPorts"] == true && right["independentPorts"] == true)
    };
    let all_crossings = geometry_checks::collect_proper_crossings(
        &actual_routes,
        &geometry_checks::CrossingOptions {
            contract,
            merge_forward_collinear_waypoints: workflow_v2,
            include_shared_endpoints: Some(&shared_crossings),
            ..Default::default()
        },
    );
    let resolved = |left: &Value, right: &Value, _hit: &geometry_checks::ProperCrossing| {
        left["verifiedHalo"] == true && right["verifiedHalo"] == true
    };
    let crossings = geometry_checks::collect_proper_crossings(
        &actual_routes,
        &geometry_checks::CrossingOptions {
            contract,
            merge_forward_collinear_waypoints: workflow_v2,
            include_shared_endpoints: Some(&shared_crossings),
            crossing_resolved: Some(&resolved),
        },
    );
    inspection.metrics["resolvedCrossovers"] =
        serde_json::json!(all_crossings.len() - crossings.len());
    inspection.hits(
        "relationship_crossings",
        "composition/proper-crossing",
        &crossings,
        serde_json::json!({"paths":actual_routes.len(),"contract":contract}),
        false,
    );
    let corridors = geometry_checks::collect_ambiguous_corridors(
        &straight_routes,
        &geometry_checks::CorridorOptions {
            contract,
            allow_short_workflow_trunks: workflow_v2,
            ..Default::default()
        },
    );
    inspection.hits(
        "relationship_corridors",
        "composition/ambiguous-corridor",
        &corridors,
        serde_json::json!({"straightSubpaths":straight_routes.len(),"contract":contract}),
        false,
    );
    if contract == InputContract::PhoenixV1 {
        let arrows = geometry_checks::collect_arrowhead_collisions(&actual_routes, workflow_v2);
        inspection.hits(
            "arrowhead_separation",
            "composition/arrowhead-collision",
            &arrows,
            serde_json::json!({"relationships":edge_facts.len()}),
            false,
        );
    }
    let border_runs = geometry_checks::collect_border_runs(&straight_routes, &frames);
    inspection.hits(
        "container_border_runs",
        "composition/container-border-run",
        &border_runs,
        serde_json::json!({"frames":frames.len()}),
        true,
    );
    let rhythm = geometry_checks::collect_route_rhythm_issues(&straight_routes, 16.0, 8.0);
    inspection.hits(
        "route_rhythm",
        "composition/route-rhythm",
        &rhythm,
        serde_json::json!({"straightSubpaths":straight_routes.len()}),
        false,
    );
    let label_clearance =
        geometry_checks::collect_label_route_clearance(&labels, &actual_routes, 4.0);
    inspection.hits(
        "label_route_clearance",
        "composition/label-route-clearance",
        &label_clearance,
        serde_json::json!({"labels":labels.len(),"minimumPx":4}),
        false,
    );
    if contract == InputContract::PhoenixV1 {
        let overflow = geometry_checks::collect_label_canvas_overflow(
            &labels,
            &[canvas.width, canvas.height],
            0.5,
        );
        inspection.hits(
            "label_canvas_containment",
            "composition/label-canvas-containment",
            &overflow,
            serde_json::json!({"labels":labels.len()}),
            false,
        );
    }
    let obstacles: Vec<_> = nodes
        .iter()
        .map(|(id, bounds)| geometry_checks::Obstacle {
            id: id.clone(),
            bounds: *bounds,
        })
        .collect();
    let obstacle_hits =
        geometry_checks::collect_route_obstacle_hits(&actual_routes, &obstacles, 0.0);
    inspection.hits(
        "non_target_node_clearance",
        "composition/route-node-overlap",
        &obstacle_hits,
        serde_json::json!({"nodes":nodes.len(),"relationships":edge_facts.len()}),
        true,
    );
    let mut legend_hits = Vec::new();
    for (index, legend) in svg
        .select(&selector("[data-graph-role='legend']"))
        .enumerate()
    {
        let mut boxes = Vec::new();
        for element in legend.select(&selector("rect,text,path,line,circle")) {
            let bounds = match element.value().name() {
                "rect" => Some(rect(element)),
                "text" => {
                    let font = number(element, "font-size", f64::NAN);
                    let width = (geometry::text_units_for_contract(
                        &element.text().collect::<String>(),
                        contract,
                    ) * font
                        * 0.62)
                        .max(font);
                    let x = number(element, "x", f64::NAN)
                        - match element.value().attr("text-anchor") {
                            Some("middle") => width / 2.0,
                            Some("end") => width,
                            _ => 0.0,
                        };
                    Some(Rect {
                        x,
                        y: number(element, "y", f64::NAN) - font,
                        width,
                        height: font * 1.25,
                    })
                }
                "circle" => {
                    let r = number(element, "r", f64::NAN);
                    Some(Rect {
                        x: number(element, "cx", f64::NAN) - r,
                        y: number(element, "cy", f64::NAN) - r,
                        width: r * 2.0,
                        height: r * 2.0,
                    })
                }
                "path" => path::analyze_svg_path(element.value().attr("d").unwrap_or(""))
                    .ok()
                    .and_then(|p| p.bounds),
                "line" => {
                    let x = number(element, "x1", f64::NAN);
                    let y = number(element, "y1", f64::NAN);
                    let x2 = number(element, "x2", f64::NAN);
                    let y2 = number(element, "y2", f64::NAN);
                    Some(Rect {
                        x: x.min(x2),
                        y: y.min(y2),
                        width: (x - x2).abs(),
                        height: (y - y2).abs(),
                    })
                }
                _ => None,
            };
            if let Some(bounds) = bounds.filter(geometry::finite_rect) {
                boxes.push(bounds);
            }
        }
        for bounds in boxes {
            for route in &actual_routes {
                if route
                    .points
                    .windows(2)
                    .any(|pair| geometry::segment_intersects_rect(pair[0], pair[1], &bounds, 2.0))
                {
                    legend_hits.push(serde_json::json!({"legend":index,"relationship":route.relation_index,"bounds":bounds}));
                }
            }
        }
    }
    inspection.hits(
        "legend_clearance",
        "composition/legend-clearance",
        &legend_hits,
        serde_json::json!({"legends":svg.select(&selector("[data-graph-role='legend']")).count()}),
        true,
    );
    inspection.finish()
}
