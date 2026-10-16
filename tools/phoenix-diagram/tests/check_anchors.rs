use phoenix_diagram::{
    check::{self, anchors},
    diagnostic, geometry, input, scene,
};

use geometry::composition::{Frame, RoutedRelation};
use input::{CompileOptions, DiagramType, InputContract};
use phoenix_diagram::{
    layout,
    render::{render, EmbeddedAssets},
};
use scene::{CompiledDiagram, Point, Rect};
use scraper::{ElementRef, Html, Selector};
use serde_json::{json, Value};

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

fn select(css: &str) -> Selector {
    Selector::parse(css).unwrap()
}
fn number(element: ElementRef<'_>, key: &str) -> f64 {
    element.value().attr(key).unwrap().parse().unwrap()
}
fn rect(element: ElementRef<'_>) -> Rect {
    Rect {
        x: number(element, "x"),
        y: number(element, "y"),
        width: number(element, "width"),
        height: number(element, "height"),
    }
}

// Deliberately read real d commands, never data-composition-points.
fn inspect(html: &str, kind: DiagramType) -> diagnostic::Diagnostics {
    let document = Html::parse_document(html);
    let svg = document.select(&select("svg")).next().unwrap();
    let metadata: Value = serde_json::from_str(
        &document
            .select(&select("#phoenix-diagram-metadata"))
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
    )
    .unwrap();
    let nodes: Vec<_> = svg
        .select(&select("g[data-node-id]"))
        .map(|group| {
            (
                group.value().attr("data-node-id").unwrap().to_owned(),
                rect(
                    group
                        .select(&select("rect[data-graph-role='node-body']"))
                        .next()
                        .unwrap(),
                ),
            )
        })
        .collect();
    let mut routes = Vec::new();
    for (index, edge) in svg.select(&select("path[data-edge-from]")).enumerate() {
        let relation = json!({"from":edge.value().attr("data-edge-from"),"to":edge.value().attr("data-edge-to")});
        let analysis = check::path::analyze_svg_path(edge.value().attr("d").unwrap()).unwrap();
        for (part_index, part) in analysis.subpaths.iter().enumerate() {
            let mut route = RoutedRelation::new(index, relation.clone(), part.points.clone());
            route.source_endpoint = part_index == 0;
            route.target_endpoint = part_index + 1 == analysis.subpaths.len();
            routes.push(route);
        }
    }
    let frames: Vec<_> = svg
        .select(&select("rect[data-graph-role='structural-frame']"))
        .map(|element| {
            let mut frame = Frame::new(
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
            frame.radius = element.value().attr("rx").unwrap_or("0").parse().unwrap();
            frame
        })
        .collect();
    anchors::check_anchors(kind, svg, &nodes, &routes, &frames, &metadata)
}

fn compile(document: Value, kind: DiagramType, contract: InputContract) -> CompiledDiagram {
    let options = CompileOptions {
        input_contract: Some(contract),
        ..Default::default()
    };
    let input =
        input::parse_input(&serde_json::to_vec(&document).unwrap(), kind, &options).unwrap();
    layout::compile(&input, &options).unwrap()
}
fn example(kind: DiagramType) -> CompiledDiagram {
    let raw = match kind {
        DiagramType::Architecture => {
            include_str!("../../../skills/phoenix-diagram/examples/minimal.architecture.json")
        }
        DiagramType::Workflow => {
            include_str!("../../../skills/phoenix-diagram/examples/minimal.workflow.json")
        }
        DiagramType::Sequence => {
            include_str!("../../../skills/phoenix-diagram/examples/minimal.sequence.json")
        }
        DiagramType::Dataflow => {
            include_str!("../../../skills/phoenix-diagram/examples/minimal.dataflow.json")
        }
        DiagramType::Lifecycle => {
            include_str!("../../../skills/phoenix-diagram/examples/minimal.lifecycle.json")
        }
    };
    compile(
        serde_json::from_str(raw).unwrap(),
        kind,
        InputContract::PhoenixV1,
    )
}
fn lifecycle(name: &str, contract: InputContract) -> CompiledDiagram {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/lifecycle/oracle.json")).unwrap();
    let contract_name = if contract == InputContract::PhoenixV1 {
        "phoenix-v1"
    } else {
        "legacy-2.17"
    };
    let case = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name && case["contract"] == contract_name)
        .unwrap();
    compile(case["document"].clone(), DiagramType::Lifecycle, contract)
}
fn html(scene: &CompiledDiagram) -> String {
    String::from_utf8(render(scene, &EmbeddedAssets { template: TEMPLATE }).unwrap()).unwrap()
}
fn assert_code(scene: &CompiledDiagram, code: &str) {
    let errors = inspect(&html(scene), scene.diagram_type);
    assert!(
        errors.iter().any(|error| error.code == code),
        "Expected {code}: {errors:?}"
    );
    for error in errors {
        assert_eq!(error.stage, "artifact");
        assert!(!error.subject.path.is_empty());
        assert!(!error.supported_fixes.is_empty());
    }
}

#[test]
fn actual_renderer_endpoints_accept_all_five_diagram_types() {
    for kind in DiagramType::ALL {
        let errors = inspect(&html(&example(kind)), kind);
        assert!(errors.is_empty(), "{kind:?}: {errors:?}");
    }
}

#[test]
fn jointly_tampered_d_and_route_declarations_cannot_float_from_known_nodes() {
    for kind in [
        DiagramType::Architecture,
        DiagramType::Workflow,
        DiagramType::Dataflow,
    ] {
        let mut scene = example(kind);
        scene.relations[0].points.first_mut().unwrap().x += 10.0;
        scene.relations[0].points.last_mut().unwrap().x -= 10.0;
        let rendered = html(&scene);
        let edges = Html::parse_document(&rendered);
        let edge = edges
            .select(&select("path[data-edge-from]"))
            .next()
            .unwrap();
        assert!(edge.value().attr("data-composition-points").is_some());
        assert_code(&scene, "artifact/endpoint-binding");
    }
}

#[test]
fn public_artifact_gate_checks_endpoints_after_both_path_and_annotation_tampering() {
    let mut scene = example(DiagramType::Architecture);
    let baseline = check::check_html(html(&scene).as_bytes(), input::Quality::Showcase);
    assert_eq!(baseline.status, "pass", "{baseline:?}");
    scene.relations[0].points[0].x += 10.0;
    scene.relations[0].points.last_mut().unwrap().x -= 10.0;
    let result = check::check_html(html(&scene).as_bytes(), input::Quality::Showcase);
    assert_eq!(result.status, "failed", "{result:?}");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|error| error.code == "artifact/endpoint-binding"),
        "{result:?}"
    );
    assert!(
        result
            .checks
            .iter()
            .any(|check| check.name == "relationship_endpoints" && check.status == "failed"),
        "{result:?}"
    );
}

#[test]
fn explicit_sides_are_checked_against_raw_facts_and_actual_body_sides() {
    let mut scene = example(DiagramType::Architecture);
    scene.relations[0].authored.facts["fromSide"] = json!("left");
    // Removing the authored-side SVG attribute cannot remove the raw contract.
    let rendered = html(&scene).replace(" data-authored-fromSide=\"left\"", "");
    let errors = inspect(&rendered, DiagramType::Architecture);
    assert!(
        errors
            .iter()
            .any(|error| error.code == "artifact/endpoint-side-position"),
        "{errors:?}"
    );
}

fn tangent_scene() -> CompiledDiagram {
    let mut scene = example(DiagramType::Architecture);
    let start = scene.relations[0].points[0];
    let end = *scene.relations[0].points.last().unwrap();
    scene.relations[0].points = vec![
        start,
        Point {
            x: start.x,
            y: start.y + 40.0,
        },
        Point {
            x: end.x,
            y: end.y + 40.0,
        },
        end,
    ];
    scene
}

#[test]
fn explicit_sides_reject_tangents_and_backwards_segments() {
    let mut scene = tangent_scene();
    scene.relations[0].authored.facts["fromSide"] = json!("right");
    scene.relations[0].authored.facts["toSide"] = json!("left");
    assert_code(&scene, "artifact/endpoint-side-direction");
    let mut scene = example(DiagramType::Architecture);
    scene.relations[0].authored.facts["fromSide"] = json!("right");
    let start = scene.relations[0].points[0];
    scene.relations[0].points.insert(
        1,
        Point {
            x: start.x - 25.0,
            y: start.y,
        },
    );
    assert_code(&scene, "artifact/endpoint-side-direction");
}

#[test]
fn named_and_via_routes_do_not_acquire_unstated_endpoint_directions() {
    let mut named = tangent_scene();
    named.relations[0].authored.facts["route"] = json!("orthogonal-v");
    assert!(inspect(&html(&named), DiagramType::Architecture).is_empty());
    let mut via = tangent_scene();
    via.relations[0].authored.facts["via"] = json!(via.relations[0].points[1..3]
        .iter()
        .map(|point| [point.x, point.y])
        .collect::<Vec<_>>());
    assert!(inspect(&html(&via), DiagramType::Architecture).is_empty());
}

#[test]
fn actual_path_must_preserve_authored_via_points_instead_of_only_its_annotation() {
    let mut scene = tangent_scene();
    scene.relations[0].authored.facts["via"] = json!(scene.relations[0].points[1..3]
        .iter()
        .map(|point| [point.x, point.y])
        .collect::<Vec<_>>());
    scene.relations[0].points[1].y += 20.0;
    scene.relations[0].points[2].y += 20.0;
    assert_code(&scene, "artifact/route-pin");
}

#[test]
fn automatic_routes_must_leave_the_actual_node_boundary_perpendicularly() {
    assert_code(&tangent_scene(), "artifact/endpoint-side-direction");
}

#[test]
fn sequence_anchors_use_participant_offsets_and_authored_timeline_y() {
    let mut wrong_offset = example(DiagramType::Sequence);
    wrong_offset.relations[0].points[0].x += 3.0;
    assert_code(&wrong_offset, "artifact/sequence-anchor");
    let mut wrong_y = example(DiagramType::Sequence);
    for point in &mut wrong_y.relations[0].points {
        point.y += 8.0;
    }
    assert_code(&wrong_y, "artifact/sequence-anchor");
    let mut changed_line = example(DiagramType::Sequence);
    let line = changed_line
        .containers
        .iter_mut()
        .find(|container| container.authored.kind == "lifeline")
        .unwrap();
    line.rule.as_mut().unwrap()[1].y = 150.0;
    assert_code(&changed_line, "artifact/sequence-anchor");
}

#[test]
fn lifecycle_frames_shared_buses_and_distinct_signatures_keep_source_semantics() {
    for name in [
        "composite-frame-exit",
        "sibling-composite-frame-exits",
        "nonconsecutive-shared-bus-and-junctions",
        "partially-overlapping-brackets-use-bus",
        "distinct-notes-stay-separate",
    ] {
        let scene = lifecycle(name, InputContract::PhoenixV1);
        let errors = inspect(&html(&scene), DiagramType::Lifecycle);
        assert!(errors.is_empty(), "{name}: {errors:?}");
    }
}

#[test]
fn lifecycle_frame_exit_requires_real_frame_bounds_and_matching_raw_sources() {
    let original = lifecycle("composite-frame-exit", InputContract::PhoenixV1);
    let index = original
        .relations
        .iter()
        .position(|relation| relation.junction.is_some())
        .unwrap();
    let mut moved = original.clone();
    moved.relations[index].points[0].y += 8.0;
    assert_code(&moved, "artifact/lifecycle-frame-exit");
    let mut shrunken = original.clone();
    let frame = shrunken
        .containers
        .iter_mut()
        .find(|container| container.authored.kind == "lifecycle-frame")
        .unwrap();
    frame.bounds.width = 1.0;
    assert_code(&shrunken, "artifact/lifecycle-frame-exit");
    let mut forged = example(DiagramType::Lifecycle);
    forged.relations[0].junction = Some("created+complete".into());
    forged.relations[0].points[0].x += 8.0;
    assert_code(&forged, "artifact/endpoint-binding");
    let mut changed_signature = original;
    changed_signature.relations[index].authored.facts["note"] = json!("A separate meaning");
    assert_code(&changed_signature, "artifact/endpoint-binding");
}

#[test]
fn legacy_lifecycle_authoritative_via_preserves_its_direction_exception() {
    let mut scene = lifecycle("explicit-pins-and-float-columns", InputContract::Legacy217);
    // The legacy source deliberately exempts via routes from its direction gate.
    let index = scene
        .relations
        .iter()
        .position(|relation| relation.authored.facts["via"].is_array())
        .unwrap();
    let relation = &mut scene.relations[index];
    let end = *relation.points.last().unwrap();
    relation.authored.facts["fromSide"] = json!("right");
    let from = scene
        .nodes
        .iter()
        .find(|node| node.authored.id == relation.from)
        .unwrap()
        .bounds;
    let start = Point {
        x: from.x + from.width,
        y: from.y + from.height / 2.0,
    };
    relation.points = vec![
        Point {
            x: from.x + from.width,
            y: start.y,
        },
        Point {
            x: from.x + from.width,
            y: start.y + 20.0,
        },
        Point {
            x: end.x,
            y: start.y + 20.0,
        },
        end,
    ];
    relation.authored.facts["via"] = json!(relation.points[1..3]
        .iter()
        .map(|point| [point.x, point.y])
        .collect::<Vec<_>>());
    let errors = inspect(&html(&scene), DiagramType::Lifecycle);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn legacy_automatic_routes_preserve_the_channel_their_axis_branch_consumes() {
    let mut scene = lifecycle("explicit-pins-and-float-columns", InputContract::Legacy217);
    let relation = &mut scene.relations[1];
    // Both selected sides are vertical and the endpoints differ on both axes:
    // the frozen auto branch consumes channelY just as the drop preset does.
    relation.authored.facts["route"] = json!("auto");
    assert!(inspect(&html(&scene), DiagramType::Lifecycle).is_empty());
    scene.relations[1].points[1].y += 15.0;
    scene.relations[1].points[2].y += 15.0;
    assert_code(&scene, "artifact/route-pin");
}

#[test]
fn workflow_via_does_not_waive_a_simultaneous_consumed_channel_pin() {
    let mut scene = example(DiagramType::Workflow);
    let start = scene.relations[0].points[0];
    let end = *scene.relations[0].points.last().unwrap();
    let via = vec![
        Point {
            x: start.x + 24.0,
            y: start.y,
        },
        Point {
            x: start.x + 24.0,
            y: start.y + 40.0,
        },
        Point {
            x: end.x - 24.0,
            y: start.y + 40.0,
        },
        Point {
            x: end.x - 24.0,
            y: end.y,
        },
    ];
    let relation = &mut scene.relations[0];
    relation.authored.facts["via"] = json!(via
        .iter()
        .map(|point| [point.x, point.y])
        .collect::<Vec<_>>());
    relation.authored.facts["channelX"] = json!(start.x + 24.0);
    relation.authored.facts["channelY"] = json!(start.y + 40.0);
    relation.points = std::iter::once(start)
        .chain(via)
        .chain(std::iter::once(end))
        .collect();
    assert!(inspect(&html(&scene), DiagramType::Workflow).is_empty());
    scene.relations[0].authored.facts["channelX"] = json!(start.x + 29.0);
    assert_code(&scene, "artifact/route-pin");
}
