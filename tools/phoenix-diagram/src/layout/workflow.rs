use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{fitted_node_font_size_for_contract, text_units_for_contract};
use crate::input::{CompileOptions, DiagramInput, DiagramType, InputContract};
use crate::scene::{Authored, Card, CompiledDiagram, DiagramFacts, Point, Rect, Text};
use serde_json::{json, Value};
use std::cmp::Ordering;

#[path = "workflow/measure.rs"]
mod measure;
#[path = "workflow/pins.rs"]
mod pins;
#[path = "workflow/placement.rs"]
mod placement;
#[path = "workflow/routing.rs"]
mod routing;
#[path = "workflow/validate.rs"]
mod validation;

#[derive(Clone, Debug)]
struct Entry {
    index: usize,
    value: Value,
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
fn number(value: &Value, key: &str, default: f64) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(default)
}
fn units(input: &DiagramInput, value: &str) -> f64 {
    text_units_for_contract(value, input.input_contract)
}
fn entries(document: &Value, collection: &str) -> Vec<Entry> {
    array(document, collection)
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, value)| Entry { index, value })
        .collect()
}
fn authored(entry: &Entry, collection: &str, kind: &str, pins: &[&str]) -> Authored {
    Authored {
        id: entry
            .value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{collection}-{}", entry.index)),
        source_path: format!("/{collection}/{}", entry.index),
        kind: kind.into(),
        facts: entry.value.clone(),
        geometry_provenance: pins
            .iter()
            .filter(|pin| entry.value.get(**pin).is_some())
            .map(|pin| format!("/{collection}/{}/{pin}", entry.index))
            .collect(),
    }
}
fn text(path: String, content: &str, x: f64, y: f64, font_size: f64, anchor: &str) -> Text {
    Text {
        source_path: path,
        content: content.into(),
        position: Point { x, y },
        font_size,
        anchor: anchor.into(),
    }
}
fn failure(code: &str, path: &str, message: impl Into<String>, evidence: Value) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(Some("workflow"), code, path, message);
    diagnostic.stage = "layout".into();
    diagnostic.evidence = evidence;
    diagnostic
}
fn stable_value(value: &Value) -> String {
    match value {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(stable_value).collect::<Vec<_>>().join(",")
        ),
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            format!(
                "{{{}}}",
                keys.into_iter()
                    .map(|key| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        stable_value(&object[key])
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        _ => value.to_string(),
    }
}
fn compare_edges(left: &Entry, right: &Entry) -> Ordering {
    for key in ["id", "from", "to", "label", "route"] {
        let compared = string(&left.value, key)
            .encode_utf16()
            .cmp(string(&right.value, key).encode_utf16());
        if compared != Ordering::Equal {
            return compared;
        }
    }
    stable_value(&left.value)
        .encode_utf16()
        .cmp(stable_value(&right.value).encode_utf16())
}

pub fn compile(
    input: &DiagramInput,
    options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    match compile_internal(input, options) {
        Ok(scene) => Ok(scene),
        Err(mut diagnostics) => {
            verify_repairs(input, options, &mut diagnostics);
            verify_text_capacity(input, options, &mut diagnostics);
            Err(diagnostics)
        }
    }
}

fn compile_internal(
    input: &DiagramInput,
    options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    compile_searching(input, options, true)
}

// Without `side_search` the repair passes stop where they would search side
// pairs; every layout that passes this way passes the same way with it.
fn compile_searching(
    input: &DiagramInput,
    options: &CompileOptions,
    side_search: bool,
) -> Result<CompiledDiagram, Diagnostics> {
    let mut widths = std::collections::BTreeMap::new();
    let mut minimum_tick = 60;
    loop {
        let result = compile_attempt(input, options, &widths, side_search);
        if input.input_contract != InputContract::PhoenixV1
            || number(&input.document, "schema_version", 1.0) != 2.0
            || input.quality != crate::input::Quality::Showcase
            || measure::absolute(&input.document)
        {
            return result;
        }
        let Err(diagnostics) = &result else {
            return result;
        };
        if diagnostics.is_empty()
            || diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code != "workflow/sublabel-readability")
        {
            return result;
        }
        let minimum = diagnostics.iter().fold(0.0_f64, |minimum, diagnostic| {
            minimum.max(number(&diagnostic.evidence, "requiredFontPx", 0.0))
        });
        let tick = (minimum * 10.0).round() as u32;
        if tick <= minimum_tick || tick > 80 {
            return result;
        }
        minimum_tick = tick;
        let Some(measured) = diagnostics[0].evidence.get("measuredScene") else {
            return result;
        };
        let Ok(scene) = serde_json::from_value::<CompiledDiagram>(measured.clone()) else {
            return result;
        };
        let mut changed = false;
        for node in &scene.nodes {
            if node.authored.facts.get("width").is_some()
                || string(&node.authored.facts, "sublabel").is_empty()
            {
                continue;
            }
            let needed = units(input, string(&node.authored.facts, "sublabel")) * minimum * 0.6;
            if needed <= node.bounds.width - 8.0 {
                continue;
            }
            let width = 200.0_f64.min(((needed + 8.0) / 4.0).ceil() * 4.0);
            if width > node.bounds.width {
                widths.insert(node.authored.id.clone(), width);
                changed = true;
            }
        }
        if !changed {
            return result;
        }
    }
}

fn compile_attempt(
    input: &DiagramInput,
    options: &CompileOptions,
    width_overrides: &std::collections::BTreeMap<String, f64>,
    side_search: bool,
) -> Result<CompiledDiagram, Diagnostics> {
    let mut feedback = routing::Feedback::default();
    let mut current = compile_plan(input, options, width_overrides, &feedback);
    if input.input_contract != InputContract::PhoenixV1
        || number(&input.document, "schema_version", 1.0) != 2.0
    {
        return current;
    }
    let mut best = current.clone();
    let mut best_feedback = feedback.clone();
    let error_count = |result: &Result<CompiledDiagram, Diagnostics>| {
        result
            .as_ref()
            .err()
            .map(|diagnostics| {
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
                    .count()
            })
            .unwrap_or(0)
    };
    for _ in 0..3 {
        if error_count(&best) == 0 {
            break;
        }
        let Some(diagnostics) = current.as_ref().err() else {
            break;
        };
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
            .collect::<Vec<_>>();
        if errors.is_empty()
            || errors.iter().any(|diagnostic| {
                ![
                    "composition/ambiguous-corridor",
                    "composition/proper-crossing",
                ]
                .contains(&diagnostic.code.as_str())
            })
        {
            break;
        }
        let mut selected = None;
        for diagnostic in errors {
            let a = diagnostic.evidence["relationIndex"].as_u64();
            let b = diagnostic
                .evidence
                .pointer("/otherRelationship/index")
                .and_then(Value::as_u64);
            if let (Some(a), Some(b)) = (a, b) {
                let sources = array(&input.document, "edges");
                let a = a as usize;
                let b = b as usize;
                let later = if compare_edges(
                    &Entry {
                        index: a,
                        value: sources[a].clone(),
                    },
                    &Entry {
                        index: b,
                        value: sources[b].clone(),
                    },
                ) == Ordering::Greater
                {
                    a
                } else {
                    b
                };
                if !feedback.route_first.contains(&later) {
                    selected = Some(later);
                    break;
                }
            }
        }
        let Some(index) = selected else {
            break;
        };
        feedback.route_first.push(index);
        feedback.fan_in_label_clearance = true;
        current = compile_plan(input, options, width_overrides, &feedback);
        let repairable = current.as_ref().err().is_some_and(|diagnostics| {
            input.quality == crate::input::Quality::Showcase
                && !measure::absolute(&input.document)
                && !diagnostics.is_empty()
                && diagnostics.iter().all(|diagnostic| {
                    diagnostic.code == "workflow/sublabel-readability"
                        && number(&diagnostic.evidence, "requiredFontPx", 9.0) <= 8.0
                })
        });
        if error_count(&current) < error_count(&best) || repairable {
            best = current.clone();
            best_feedback = feedback.clone();
        }
    }
    repair_composition(
        input,
        options,
        width_overrides,
        best,
        best_feedback,
        side_search,
    )
}

const COMPOSITION_REPAIRS: [&str; 3] = [
    "composition/ambiguous-corridor",
    "composition/proper-crossing",
    "composition/arrowhead-collision",
];

// Only a layout that still has composition hits gets these passes, so layouts
// that already compile keep their exact geometry. A pass is kept only when it
// strictly improves the rank. Readability and canvas errors are reported only
// after the layout checks pass, so reaching them is progress; among layout
// errors, other codes count first, then corridor and arrowhead hits, then
// crossings, which a halo can still resolve.
fn repair_composition(
    input: &DiagramInput,
    options: &CompileOptions,
    width_overrides: &std::collections::BTreeMap<String, f64>,
    mut best: Result<CompiledDiagram, Diagnostics>,
    mut feedback: routing::Feedback,
    side_search: bool,
) -> Result<CompiledDiagram, Diagnostics> {
    let base = feedback.clone();
    let mut searches = 0;
    // Stage 2: routing failed before any layout check; 1: layout checks
    // failed; 0: only readability or canvas checks remain.
    let rank = |result: &Result<CompiledDiagram, Diagnostics>| {
        let mut rank = (0, 0, 0, 0, 0);
        for diagnostic in result.as_ref().err().into_iter().flatten() {
            if diagnostic.severity != crate::diagnostic::Severity::Error {
                continue;
            }
            match diagnostic.code.as_str() {
                "workflow/sublabel-readability" | "workflow/viewbox-capacity" => rank.4 += 1,
                "composition/proper-crossing" => rank.3 += 1,
                code if COMPOSITION_REPAIRS.contains(&code) => rank.2 += 1,
                "workflow/solver-budget-exhausted"
                | "workflow/explicit-pin-conflict"
                | "workflow/route-preset-conflict"
                | "workflow/layout-feedback" => rank.0 = 2,
                _ => rank.1 += 1,
            }
        }
        if rank.0 == 0 && rank.1 + rank.2 + rank.3 > 0 {
            rank.0 = 1;
        }
        rank
    };
    for _ in 0..8 {
        let Err(diagnostics) = &best else {
            break;
        };
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.severity == crate::diagnostic::Severity::Error
                    && COMPOSITION_REPAIRS.contains(&diagnostic.code.as_str())
            })
            .collect::<Vec<_>>();
        let Some(scene) = errors
            .iter()
            .find_map(|diagnostic| diagnostic.evidence.get("measuredScene"))
            .and_then(|scene| serde_json::from_value::<CompiledDiagram>(scene.clone()).ok())
        else {
            break;
        };
        let pair = |diagnostic: &Diagnostic| {
            let a = diagnostic.evidence["relationIndex"].as_u64().or_else(|| {
                diagnostic
                    .subject
                    .path
                    .strip_prefix("/edges/")?
                    .parse()
                    .ok()
            });
            let b = diagnostic
                .evidence
                .pointer("/otherRelationship/index")
                .and_then(Value::as_u64);
            a.zip(b).map(|(a, b)| (a as usize, b as usize))
        };
        let mut proposals = Vec::new();
        // Pairs that meet at a node keep the sides they actually use, so the
        // next attempt spreads their ports on that side.
        let actual = actual_sides(&scene);
        let ends = |index: usize| {
            array(&input.document, "edges")
                .get(index)
                .map(|edge| [string(edge, "from"), string(edge, "to")])
        };
        let mut spread = feedback.clone();
        for (a, b) in errors
            .iter()
            .filter(|diagnostic| diagnostic.code != "composition/proper-crossing")
            .filter_map(|diagnostic| pair(diagnostic))
        {
            if let (Some(left), Some(right)) = (ends(a), ends(b)) {
                if left.iter().any(|id| right.contains(id)) {
                    for index in [a, b] {
                        if let Some(sides) = actual.get(&index) {
                            spread.sides.entry(index).or_insert_with(|| sides.clone());
                        }
                    }
                }
            }
        }
        if spread.sides != feedback.sides {
            proposals.push(spread);
        }
        if !feedback.channels
            && errors
                .iter()
                .any(|diagnostic| diagnostic.code == "composition/ambiguous-corridor")
        {
            let mut channels = feedback.clone();
            channels.channels = true;
            proposals.push(channels);
        }
        let mut halo = feedback.clone();
        for (a, b) in errors
            .iter()
            .filter(|diagnostic| diagnostic.code == "composition/proper-crossing")
            .filter_map(|diagnostic| pair(diagnostic))
        {
            halo.halo.extend([a, b]);
        }
        if halo.halo != feedback.halo {
            proposals.push(halo);
        }
        let forced_pair = errors
            .iter()
            .filter(|diagnostic| diagnostic.code != "composition/proper-crossing")
            .find_map(|diagnostic| pair(diagnostic));
        let mut improved = false;
        for next in proposals {
            let current = compile_plan(input, options, width_overrides, &next);
            if rank(&current) < rank(&best) {
                best = current;
                feedback = next;
                improved = true;
                break;
            }
        }
        if improved {
            continue;
        }
        // Only when none of those helps may an end pair of the first corridor
        // or arrowhead hit take another side pair, keeping the best strictly
        // better layout. Two such searches keep a failing workflow's report
        // prompt; the drafts this repairs need no more.
        let Some((a, b)) = forced_pair.filter(|_| side_search && searches < 2) else {
            break;
        };
        searches += 1;
        // Earlier passes can leave too little room for another side pair, so
        // the pair is also tried from the state before any repair pass and
        // from the plain first attempt.
        let mut starts = vec![feedback.clone()];
        for start in [base.clone(), routing::Feedback::default()] {
            if !starts.contains(&start) {
                starts.push(start);
            }
        }
        let mut chosen: Option<(Result<CompiledDiagram, Diagnostics>, routing::Feedback)> = None;
        for start in starts {
            if chosen.is_some() {
                break;
            }
            for index in [b, a] {
                let Some(edge) = array(&input.document, "edges").get(index) else {
                    continue;
                };
                if !["", "auto"].contains(&string(edge, "route"))
                    || [
                        "via", "channelX", "channelY", "fromSide", "toSide", "labelAt",
                    ]
                    .iter()
                    .any(|field| edge.get(*field).is_some())
                    || start.force.contains_key(&index)
                {
                    continue;
                }
                for from_side in ["right", "bottom", "left", "top"] {
                    for to_side in ["right", "bottom", "left", "top"] {
                        if actual.get(&index) == Some(&(from_side.to_owned(), to_side.to_owned())) {
                            continue;
                        }
                        let mut next = start.clone();
                        next.force
                            .insert(index, (from_side.to_owned(), to_side.to_owned()));
                        next.sides
                            .insert(index, (from_side.to_owned(), to_side.to_owned()));
                        let mut current = compile_plan(input, options, width_overrides, &next);
                        // A new side pair can move crossings and shared
                        // corridors elsewhere; halos and parallel lanes may
                        // still resolve those.
                        if rank(&current) >= rank(&best) {
                            let mut polished = next.clone();
                            for diagnostic in current.as_ref().err().into_iter().flatten() {
                                match diagnostic.code.as_str() {
                                    "composition/proper-crossing" => {
                                        polished.halo.extend(
                                            pair(diagnostic)
                                                .map(|(a, b)| [a, b])
                                                .into_iter()
                                                .flatten(),
                                        );
                                    }
                                    "composition/ambiguous-corridor" => polished.channels = true,
                                    _ => {}
                                }
                            }
                            if polished != next {
                                let again =
                                    compile_plan(input, options, width_overrides, &polished);
                                if rank(&again) < rank(&current) {
                                    current = again;
                                    next = polished;
                                }
                            }
                        }
                        if rank(&current) < rank(&best)
                            && chosen
                                .as_ref()
                                .is_none_or(|(result, _)| rank(&current) < rank(result))
                        {
                            chosen = Some((current, next));
                        }
                    }
                }
            }
        }
        let Some((current, next)) = chosen else {
            break;
        };
        best = current;
        feedback = next;
    }
    best
}

// The node side each routed end leaves or enters, keyed by source edge index.
fn actual_sides(scene: &CompiledDiagram) -> std::collections::BTreeMap<usize, (String, String)> {
    let side_at = |point: Point, bounds: &Rect| {
        let near = |a: f64, b: f64| (a - b).abs() <= 0.0001;
        if near(point.x, bounds.x) {
            "left"
        } else if near(point.x, bounds.x + bounds.width) {
            "right"
        } else if near(point.y, bounds.y) {
            "top"
        } else {
            "bottom"
        }
    };
    scene
        .relations
        .iter()
        .filter_map(|relation| {
            let index = relation
                .authored
                .source_path
                .rsplit('/')
                .next()?
                .parse()
                .ok()?;
            let node = |id: &str| scene.nodes.iter().find(|node| node.authored.id == id);
            let (from, to) = (node(&relation.from)?, node(&relation.to)?);
            Some((
                index,
                (
                    side_at(*relation.points.first()?, &from.bounds).to_owned(),
                    side_at(*relation.points.last()?, &to.bounds).to_owned(),
                ),
            ))
        })
        .collect()
}

fn compile_plan(
    input: &DiagramInput,
    _options: &CompileOptions,
    width_overrides: &std::collections::BTreeMap<String, f64>,
    routing_feedback: &routing::Feedback,
) -> Result<CompiledDiagram, Diagnostics> {
    if input.diagram_type != DiagramType::Workflow {
        return Err(vec![failure(
            "workflow/input-contract",
            "/diagram_type",
            "Workflow compiler requires workflow input",
            json!({"actual":input.diagram_type}),
        )]);
    }
    let document = &input.document;
    let readable = number(document, "schema_version", 1.0) == 2.0;
    let lanes = entries(document, "lanes");
    let mut nodes = entries(document, "nodes");
    let mut edges = entries(document, "edges");
    let mut phases = entries(document, "phases");
    let mut groups = entries(document, "groups");
    let lane_index = |node: &Value| {
        lanes
            .iter()
            .position(|lane| lane.value["id"] == node["lane"])
            .unwrap_or(usize::MAX)
    };
    if readable {
        nodes.sort_by(|a, b| {
            lane_index(&a.value)
                .cmp(&lane_index(&b.value))
                .then_with(|| number(&a.value, "col", 0.0).total_cmp(&number(&b.value, "col", 0.0)))
                .then_with(|| string(&a.value, "id").cmp(string(&b.value, "id")))
        });
        edges.sort_by(compare_edges);
        phases.sort_by(|a, b| {
            number(&a.value, "fromCol", 0.0)
                .total_cmp(&number(&b.value, "fromCol", 0.0))
                .then_with(|| {
                    number(&a.value, "toCol", 0.0).total_cmp(&number(&b.value, "toCol", 0.0))
                })
                .then_with(|| string(&a.value, "id").cmp(string(&b.value, "id")))
        });
        groups.sort_by(|a, b| {
            lane_index(&a.value)
                .cmp(&lane_index(&b.value))
                .then_with(|| {
                    number(&a.value, "fromCol", 0.0).total_cmp(&number(&b.value, "fromCol", 0.0))
                })
                .then_with(|| {
                    number(&a.value, "toCol", 0.0).total_cmp(&number(&b.value, "toCol", 0.0))
                })
                .then_with(|| string(&a.value, "id").cmp(string(&b.value, "id")))
        });
    }
    // A forced side pair is measured and routed exactly like authored
    // endpoint sides; the relation keeps the authored facts.
    let planned = edges
        .iter()
        .map(|edge| {
            let mut value = edge.value.clone();
            if let Some((from_side, to_side)) = routing_feedback.force.get(&edge.index) {
                value["fromSide"] = Value::String(from_side.clone());
                value["toSide"] = Value::String(to_side.clone());
            }
            Entry {
                index: edge.index,
                value,
            }
        })
        .collect::<Vec<_>>();
    let mut feedback_gap = 0.0_f64;
    let mut rank_minimums = std::collections::BTreeMap::new();
    let mut resolved = None;
    for attempt in 0..=3 {
        let measured = measure::measure(
            input,
            &lanes,
            &nodes,
            &planned,
            &phases,
            &groups,
            feedback_gap,
            &rank_minimums,
            width_overrides,
        )?;
        match routing::route(input, &measured, &planned, routing_feedback) {
            Ok(mut relations) => {
                for relation in &mut relations {
                    if let Some(edge) = edges.iter().find(|edge| {
                        routing_feedback.force.contains_key(&edge.index)
                            && relation.authored.source_path == format!("/edges/{}", edge.index)
                    }) {
                        relation.authored = routing::edge_authored(edge);
                    }
                }
                resolved = Some((measured, relations));
                break;
            }
            Err(mut diagnostics) => {
                if attempt == 3
                    || diagnostics.len() != 1
                    || diagnostics[0].code != "workflow/layout-feedback"
                {
                    let has_geometry_failure = diagnostics.iter().any(|diagnostic| {
                        [
                            "workflow/explicit-pin-conflict",
                            "workflow/route-preset-conflict",
                            "workflow/solver-budget-exhausted",
                        ]
                        .contains(&diagnostic.code.as_str())
                    });
                    for diagnostic in &mut diagnostics {
                        if diagnostic.code == "workflow/layout-feedback" {
                            diagnostic.code = "workflow/solver-budget-exhausted".into();
                        }
                    }
                    if has_geometry_failure {
                        if let Some(warning) = measure::absolute_sizing_warning(input, &measured) {
                            diagnostics.push(warning);
                        }
                    }
                    return Err(diagnostics);
                }
                let request = &diagnostics[0].evidence;
                let minimum = number(request, "minimum", 0.0);
                if string(request, "kind") == "lane-gap-minimum" && minimum > feedback_gap + 0.0001
                {
                    feedback_gap = minimum;
                } else if string(request, "kind") == "rank-gap-minimum" {
                    let key = (
                        number(request, "fromCol", 0.0) as usize,
                        number(request, "toCol", 0.0) as usize,
                    );
                    let current = rank_minimums.entry(key).or_insert(0.0_f64);
                    if minimum <= *current + 0.0001 {
                        diagnostics[0].code = "workflow/solver-budget-exhausted".into();
                        return Err(diagnostics);
                    }
                    *current = minimum;
                } else {
                    diagnostics[0].code = "workflow/solver-budget-exhausted".into();
                    return Err(diagnostics);
                }
            }
        }
    }
    let (measured, relations) =
        resolved.expect("bounded workflow feedback either resolves or returns diagnostics");
    let mut scene = CompiledDiagram {
        diagram_type: DiagramType::Workflow,
        schema_version: number(document, "schema_version", 1.0) as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: document["meta"].clone(),
        view_box: measured.view_box,
        reader_fit: "contain".into(),
        nodes: measured.nodes.clone(),
        relations,
        containers: measured.containers.clone(),
        text: measured.text.clone(),
        legends: measured.legends.clone(),
        cards: entries(document, "cards")
            .iter()
            .map(|entry| Card {
                authored: authored(entry, "cards", "card", &[]),
                bounds: Rect::default(),
                text: std::iter::once(text(
                    format!("/cards/{}/title", entry.index),
                    string(&entry.value, "title"),
                    0.0,
                    0.0,
                    14.0,
                    "start",
                ))
                .chain(
                    array(&entry.value, "items")
                        .iter()
                        .enumerate()
                        .map(|(i, item)| {
                            text(
                                format!("/cards/{}/items/{i}", entry.index),
                                item.as_str().unwrap_or_default(),
                                0.0,
                                0.0,
                                12.0,
                                "start",
                            )
                        }),
                )
                .collect(),
            })
            .collect(),
        facts: DiagramFacts::Workflow {
            semantic_checks: document.get("semanticChecks").cloned(),
            lanes: array(document, "lanes").to_vec(),
            phases: array(document, "phases").to_vec(),
            groups: array(document, "groups").to_vec(),
            main_path: array(document, "mainPath")
                .iter()
                .map(|id| id.as_str().unwrap_or_default().into())
                .collect(),
        },
        diagnostics: Vec::new(),
    };
    measure::clear_group_titles(input, &measured, &mut scene);
    validation::validate(input, &measured, &mut scene)?;
    measure::finalize(input, &measured, &mut scene)?;
    measure::validate_readable_text(input, &mut scene)?;
    Ok(scene)
}

fn verify_repairs(input: &DiagramInput, options: &CompileOptions, diagnostics: &mut Diagnostics) {
    for diagnostic in diagnostics.iter_mut() {
        if pins::verify(input, options, diagnostic)
            || pins::verify_preset(input, options, diagnostic)
        {
            continue;
        }
        let mut proposals: Vec<Vec<Value>> = Vec::new();
        if diagnostic.code == "workflow/viewbox-capacity" {
            if let Some(required) = diagnostic.evidence.get("requiredViewBox") {
                proposals.push(vec![
                    json!({"op":"set","path":"/meta/viewBox","value":required}),
                ]);
            }
            proposals.push(vec![json!({"op":"remove","path":"/meta/viewBox"})]);
        }
        if diagnostic.code == "workflow/explicit-pin-conflict"
            || diagnostic.code == "workflow/route-preset-conflict"
        {
            if let Some(index) = diagnostic
                .subject
                .path
                .split('/')
                .nth(2)
                .and_then(|index| index.parse::<usize>().ok())
            {
                if let Some(edge) = array(&input.document, "edges").get(index) {
                    let fields = [
                        "via",
                        "channelX",
                        "channelY",
                        "labelAt",
                        "labelDx",
                        "labelDy",
                        "labelSegment",
                        "route",
                        "fromSide",
                        "toSide",
                    ];
                    let changes = fields
                        .iter()
                        .filter(|field| edge.get(**field).is_some())
                        .map(
                            |field| json!({"op":"remove","path":format!("/edges/{index}/{field}")}),
                        )
                        .collect::<Vec<_>>();
                    for size in 1..=changes.len() {
                        proposals.extend(removal_combinations(&changes, size));
                    }
                }
            }
        }
        if diagnostic.code == "workflow/column-capacity" {
            if let Some(index) = diagnostic
                .subject
                .path
                .split('/')
                .nth(2)
                .and_then(|index| index.parse::<usize>().ok())
            {
                if let Some(edge) = array(&input.document, "edges").get(index) {
                    if let Some(index) = array(&input.document, "nodes")
                        .iter()
                        .position(|node| node["id"] == edge["to"])
                    {
                        let current = number(&input.document["nodes"][index], "col", 0.0) as usize;
                        let mut columns = (0..6).filter(|col| *col != current).collect::<Vec<_>>();
                        columns.sort_by_key(|col| (col.abs_diff(current), *col));
                        for col in columns {
                            proposals.push(vec![json!({"op":"set","path":format!("/nodes/{index}/col"),"value":col})]);
                        }
                    }
                }
            }
        }
        let mut verified = Vec::new();
        let pin_failure = [
            "workflow/explicit-pin-conflict",
            "workflow/route-preset-conflict",
        ]
        .contains(&diagnostic.code.as_str());
        let mut minimum_size = None;
        for changes in proposals {
            if pin_failure && minimum_size.is_some_and(|size| changes.len() > size) {
                break;
            }
            let mut candidate = input.document.clone();
            let mut applicable = true;
            for change in &changes {
                let path = change["path"].as_str().unwrap();
                if change["op"] == "set" {
                    if let Some(value) = candidate.pointer_mut(path) {
                        *value = change["value"].clone();
                    } else {
                        applicable = false;
                        break;
                    }
                } else {
                    let (parent, key) = path.rsplit_once('/').unwrap();
                    if let Some(object) =
                        candidate.pointer_mut(parent).and_then(Value::as_object_mut)
                    {
                        object.remove(key);
                    } else {
                        applicable = false;
                        break;
                    }
                }
            }
            if !applicable {
                continue;
            }
            let candidate_options = CompileOptions {
                input_contract: Some(input.input_contract),
                quality: Some(input.quality),
                ..options.clone()
            };
            let Ok(candidate_input) = crate::input::parse_input(
                &serde_json::to_vec(&candidate).expect("JSON candidate"),
                DiagramType::Workflow,
                &candidate_options,
            ) else {
                continue;
            };
            if compile_internal(&candidate_input, &candidate_options).is_ok() {
                minimum_size = Some(changes.len());
                diagnostic.supported_fixes.push(format!("If these geometry assertions are optional, apply {} together; the complete workflow then passes native compilation",changes.iter().map(|change|if change["op"]=="remove"{format!("remove {}",change["path"].as_str().unwrap())}else{format!("set {} to {}",change["path"].as_str().unwrap(),change["value"])}).collect::<Vec<_>>().join(", ")));
                verified.push(json!({"scope":"complete-workflow-compile","changes":changes}));
                if !pin_failure && verified.len() >= 3 {
                    break;
                }
            }
        }
        if !verified.is_empty() {
            if pin_failure {
                let mut paths = std::collections::BTreeSet::new();
                let pins=verified.iter().flat_map(|repair|repair["changes"].as_array().unwrap()).filter_map(|change|{let path=change["path"].as_str().unwrap();if !paths.insert(path.to_owned()){return None;}Some(json!({"path":path,"field":path.rsplit('/').next().unwrap(),"value":input.document.pointer(path).cloned().unwrap_or(Value::Null)}))}).collect::<Vec<_>>();
                if pins.len() == 1 {
                    diagnostic.subject.path = pins[0]["path"].as_str().unwrap().into();
                }
                diagnostic.evidence["conflictingPins"] = Value::Array(pins);
            }
            diagnostic.evidence["verifiedRepairs"] = Value::Array(verified);
        }
    }
    if input.input_contract == InputContract::PhoenixV1
        && number(&input.document, "schema_version", 1.0) == 2.0
    {
        verify_workflow_repair(input, options, diagnostics);
    }
    verify_composition_advice(input, options, diagnostics);
}

// When the named pins alone are not the cause, the whole workflow may still
// pass with automatic routing for every edge, by itself or with its texts
// within a verified limit, so geometry and text are repaired in one round.
// This sits beside the minimal repairs in `verifiedRepairs`.
fn verify_workflow_repair(
    input: &DiagramInput,
    options: &CompileOptions,
    diagnostics: &mut Diagnostics,
) {
    let unrepaired = |diagnostic: &Diagnostic| {
        ([
            "workflow/explicit-pin-conflict",
            "workflow/route-preset-conflict",
        ]
        .contains(&diagnostic.code.as_str())
            || COMPOSITION_REPAIRS.contains(&diagnostic.code.as_str()))
            && diagnostic.supported_fixes.is_empty()
    };
    if !diagnostics.iter().any(unrepaired) {
        return;
    }
    let changes = route_pin_removals(&input.document);
    if changes.is_empty() {
        return;
    }
    let mut candidate = input.document.clone();
    for change in &changes {
        let (parent, key) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
        candidate
            .pointer_mut(parent)
            .and_then(Value::as_object_mut)
            .unwrap()
            .remove(key);
    }
    let candidate_options = CompileOptions {
        input_contract: Some(input.input_contract),
        quality: Some(input.quality),
        ..options.clone()
    };
    let Ok(candidate_input) = crate::input::parse_input(
        &serde_json::to_vec(&candidate).expect("JSON candidate"),
        DiagramType::Workflow,
        &candidate_options,
    ) else {
        return;
    };
    let removals = changes
        .iter()
        .map(|change| format!("remove {}", change["path"].as_str().unwrap()))
        .collect::<Vec<_>>()
        .join(", ");
    let (fix, repair) = match compile_internal(&candidate_input, &candidate_options) {
        Ok(_) => (
            format!("If these geometry assertions are optional, apply {removals} together; the complete workflow then passes native compilation"),
            json!({"scope":"complete-workflow-compile","changes":changes}),
        ),
        Err(found)
            if found.iter().all(|diagnostic| {
                diagnostic.severity != crate::diagnostic::Severity::Error
                    || text_capacity_path(diagnostic).is_some()
            }) =>
        {
            let mut paths = found
                .iter()
                .filter_map(text_capacity_path)
                .map(str::to_owned)
                .collect();
            let Some(limit) = text_capacity_limit(input, options, &candidate, &mut paths, &[])
            else {
                return;
            };
            let listed = paths.iter().cloned().collect::<Vec<_>>().join(", ");
            (
                format!("If these geometry assertions are optional, apply {removals} together, and keep every text in [{listed}] within {limit} text units (a CJK character counts as 2), moving any detail beyond that into a card item; the complete workflow then passes native compilation"),
                json!({"scope":"complete-workflow-compile","changes":changes,"textCapacity":{"maximumUnits":limit,"paths":paths}}),
            )
        }
        Err(_) => return,
    };
    for diagnostic in diagnostics
        .iter_mut()
        .filter(|diagnostic| unrepaired(diagnostic))
    {
        diagnostic.supported_fixes.push(fix.clone());
        diagnostic.evidence["verifiedWorkflowRepair"] = repair.clone();
    }
}

// The node text field a text-capacity diagnostic names, if it is one.
fn text_capacity_path(diagnostic: &Diagnostic) -> Option<&str> {
    let path = diagnostic.subject.path.as_str();
    let fits = diagnostic.code == "workflow/sublabel-readability"
        || diagnostic.code == "layout/constraint"
            && diagnostic
                .message
                .ends_with("does not fit its authored width");
    (fits
        && ["/label", "/sublabel", "/tag"]
            .iter()
            .any(|field| path.ends_with(field)))
    .then_some(path)
}

// The longest prefix of `text` within `limit` text units.
fn within_units(input: &DiagramInput, text: &str, limit: f64) -> String {
    let mut result = String::new();
    for character in text.chars() {
        let mut next = result.clone();
        next.push(character);
        if units(input, &next) > limit {
            break;
        }
        result = next;
    }
    result
}

// `document` with every text in `paths` cut to `limit` text units; a label
// keeps its first character and other texts are removed at zero.
fn cut_texts(
    input: &DiagramInput,
    document: &Value,
    paths: &std::collections::BTreeSet<String>,
    limit: usize,
) -> Value {
    let mut candidate = document.clone();
    for path in paths {
        let text = string_at(document, path);
        let mut cut = within_units(input, &text, limit as f64);
        if cut.is_empty() && path.ends_with("/label") {
            cut = text.chars().take(1).collect();
        }
        if cut.is_empty() {
            let (parent, key) = path.rsplit_once('/').unwrap();
            if let Some(object) = candidate.pointer_mut(parent).and_then(Value::as_object_mut) {
                object.remove(key);
            }
        } else if let Some(value) = candidate.pointer_mut(path) {
            *value = Value::String(cut);
        }
    }
    candidate
}

fn string_at(document: &Value, path: &str) -> String {
    document
        .pointer(path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

// Text that does not fit is the author's to shorten, so the verified advice is
// a length: the largest common limit for every text in `paths` with which the
// complete workflow reports no text-capacity diagnostic and no error outside
// `tolerated`. Texts are measured in stages (authored width, then the final
// readability floor), so cutting the named texts can expose others; those
// join `paths`. A probe that must pass outright first skips the side-pair
// search: a layout that passes without it passes identically with it.
fn text_capacity_limit(
    input: &DiagramInput,
    options: &CompileOptions,
    document: &Value,
    paths: &mut std::collections::BTreeSet<String>,
    tolerated: &[(String, String)],
) -> Option<usize> {
    let candidate_options = CompileOptions {
        input_contract: Some(input.input_contract),
        quality: Some(input.quality),
        ..options.clone()
    };
    let probe = |paths: &std::collections::BTreeSet<String>, limit: usize| {
        let candidate = cut_texts(input, document, paths, limit);
        let candidate_input = crate::input::parse_input(
            &serde_json::to_vec(&candidate).expect("JSON capacity probe"),
            DiagramType::Workflow,
            &candidate_options,
        )
        .ok()?;
        let result = tolerated
            .is_empty()
            .then(|| compile_searching(&candidate_input, &candidate_options, false))
            .filter(Result::is_ok)
            .unwrap_or_else(|| compile_searching(&candidate_input, &candidate_options, true));
        Some(
            result
                .err()
                .unwrap_or_default()
                .into_iter()
                .filter(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
                .collect::<Vec<_>>(),
        )
    };
    let passes = |found: &[Diagnostic]| {
        found.iter().all(|diagnostic| {
            text_capacity_path(diagnostic).is_none()
                && tolerated.iter().any(|(code, path)| {
                    *code == diagnostic.code && *path == diagnostic.subject.path
                })
        })
    };
    let mut feasible = false;
    for _ in 0..4 {
        let found = probe(paths, 0)?;
        let exposed = found
            .iter()
            .filter_map(text_capacity_path)
            .filter(|path| !paths.contains(*path))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if exposed.is_empty() {
            feasible = passes(&found);
            break;
        }
        paths.extend(exposed);
    }
    if !feasible {
        return None;
    }
    let (mut low, mut high) = (
        0,
        paths
            .iter()
            .map(|path| units(input, &string_at(document, path)))
            .fold(0.0_f64, f64::max) as usize,
    );
    while low + 1 < high {
        let middle = (low + high) / 2;
        if probe(paths, middle).is_some_and(|found| passes(&found)) {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(low)
}

fn text_capacity_advice(
    input: &DiagramInput,
    path: &str,
    limit: usize,
    paths: &std::collections::BTreeSet<String>,
) -> String {
    let listed = paths.iter().cloned().collect::<Vec<_>>().join(", ");
    if limit == 0 {
        format!("Move the detail in {path} into a card item; with every text in [{listed}] moved to cards (labels keep one character) the complete workflow passes its text-capacity checks")
    } else {
        format!(
            "Keep every text in [{listed}] within {limit} text units (a CJK character counts as 2; {path} is now {}), moving any detail beyond that into a card item; the complete workflow then passes its text-capacity checks",
            units(input, &string_at(&input.document, path))
        )
    }
}

fn verify_text_capacity(
    input: &DiagramInput,
    options: &CompileOptions,
    diagnostics: &mut Diagnostics,
) {
    if input.input_contract != InputContract::PhoenixV1
        || number(&input.document, "schema_version", 1.0) != 2.0
    {
        return;
    }
    let mut paths = diagnostics
        .iter()
        .filter_map(text_capacity_path)
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    if paths.is_empty() {
        return;
    }
    let tolerated = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity == crate::diagnostic::Severity::Error
                && text_capacity_path(diagnostic).is_none()
        })
        .map(|diagnostic| (diagnostic.code.clone(), diagnostic.subject.path.clone()))
        .collect::<Vec<_>>();
    let Some(limit) = text_capacity_limit(input, options, &input.document, &mut paths, &tolerated)
    else {
        return;
    };
    for diagnostic in diagnostics.iter_mut() {
        let Some(path) = text_capacity_path(diagnostic).map(str::to_owned) else {
            continue;
        };
        diagnostic
            .supported_fixes
            .insert(0, text_capacity_advice(input, &path, limit, &paths));
        diagnostic.evidence["verifiedTextCapacity"] =
            json!({"scope":"complete-workflow-compile","maximumUnits":limit,"paths":paths});
    }
}

// Every authored routing control in the workflow, as removals.
fn route_pin_removals(document: &Value) -> Vec<Value> {
    let fields = [
        "via",
        "channelX",
        "channelY",
        "labelAt",
        "labelDx",
        "labelDy",
        "labelSegment",
        "route",
        "fromSide",
        "toSide",
    ];
    array(document, "edges")
        .iter()
        .enumerate()
        .flat_map(|(index, edge)| {
            fields
                .iter()
                .filter(move |field| {
                    edge.get(**field).is_some()
                        && !(**field == "route" && string(edge, "route") == "auto")
                })
                .map(move |field| json!({"op":"remove","path":format!("/edges/{index}/{field}")}))
        })
        .collect()
}

fn removal_combinations(values: &[Value], size: usize) -> Vec<Vec<Value>> {
    fn visit(
        values: &[Value],
        size: usize,
        start: usize,
        prefix: &mut Vec<Value>,
        output: &mut Vec<Vec<Value>>,
    ) {
        if prefix.len() == size {
            output.push(prefix.clone());
            return;
        }
        for index in start..=values.len() - (size - prefix.len()) {
            prefix.push(values[index].clone());
            visit(values, size, index + 1, prefix, output);
            prefix.pop();
        }
    }
    let mut output = Vec::new();
    visit(values, size, 0, &mut Vec::new(), &mut output);
    output
}

fn candidate_passes(input: &DiagramInput, options: &CompileOptions, candidate: &Value) -> bool {
    let candidate_options = CompileOptions {
        input_contract: Some(input.input_contract),
        quality: Some(input.quality),
        ..options.clone()
    };
    let Ok(candidate_input) = crate::input::parse_input(
        &serde_json::to_vec(candidate).expect("JSON proposal"),
        DiagramType::Workflow,
        &candidate_options,
    ) else {
        return false;
    };
    compile_internal(&candidate_input, &candidate_options).is_ok()
}

fn verify_composition_advice(
    input: &DiagramInput,
    options: &CompileOptions,
    diagnostics: &mut Diagnostics,
) {
    if input.input_contract != InputContract::PhoenixV1
        || number(&input.document, "schema_version", 1.0) != 2.0
        || measure::absolute(&input.document)
    {
        return;
    }
    let error_indexes = diagnostics
        .iter()
        .enumerate()
        .filter(|(_, diagnostic)| diagnostic.severity == crate::diagnostic::Severity::Error)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let Some(first) = error_indexes.first().copied() else {
        return;
    };
    if error_indexes
        .iter()
        .all(|index| diagnostics[*index].code == "composition/proper-crossing")
    {
        let a = diagnostics[first].evidence["relationIndex"].as_u64();
        let b = diagnostics[first]
            .evidence
            .pointer("/otherRelationship/index")
            .and_then(Value::as_u64);
        if let (Some(a), Some(b)) = (a, b) {
            let mut groups = Vec::new();
            for index in [a as usize, b as usize] {
                let edge = &input.document["edges"][index];
                groups.push(
                    ["fromSide", "toSide", "route"]
                        .iter()
                        .filter(|field| {
                            edge.get(**field).is_some() && string(edge, field) != "auto"
                        })
                        .map(
                            |field| json!({"op":"remove","path":format!("/edges/{index}/{field}")}),
                        )
                        .collect::<Vec<_>>(),
                );
            }
            let combined = groups.iter().flatten().cloned().collect::<Vec<_>>();
            groups.push(combined);
            for changes in groups {
                if changes.is_empty() {
                    continue;
                }
                let mut candidate = input.document.clone();
                for change in &changes {
                    let (parent, key) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
                    candidate
                        .pointer_mut(parent)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(key);
                }
                if candidate_passes(input, options, &candidate) {
                    diagnostics[first].evidence["verifiedRepair"] =
                        json!({"scope":"complete-workflow-compile","changes":changes});
                    diagnostics[first].supported_fixes.insert(0,"If these endpoint sides and route presets are optional, apply the complete removals in evidence.verifiedRepair; the full workflow then passes native compilation".into());
                    return;
                }
            }
        }
    }
    if input.quality != crate::input::Quality::Showcase
        || error_indexes.iter().any(|index| {
            ![
                "composition/proper-crossing",
                "composition/ambiguous-corridor",
            ]
            .contains(&diagnostics[*index].code.as_str())
        })
    {
        return;
    }
    if let Some((proposed, changes)) = placement::propose(&input.document) {
        if candidate_passes(input, options, &proposed) {
            diagnostics[first].evidence["verifiedRepair"] = json!({"scope":"complete-workflow-compile","kind":"optional-node-placement","changes":changes});
            diagnostics[first].supported_fixes.insert(0,"If node columns and row offsets are flexible, apply the complete optional-node-placement changes in evidence.verifiedRepair; every authored node, relation, lane, condition and mainPath is preserved and the complete workflow then passes native compilation".into());
        }
    }
}
