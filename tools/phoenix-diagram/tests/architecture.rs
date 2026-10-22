use input::{parse_input, CompileOptions, DiagramType, InputContract};
use phoenix_diagram::layout::architecture;
use phoenix_diagram::{diagnostic, geometry, input, render, scene};
use scraper::{Html, Selector};
use serde_json::{json, Value};

fn compile(
    document: &Value,
    contract: InputContract,
) -> Result<scene::CompiledDiagram, diagnostic::Diagnostics> {
    let options = CompileOptions {
        input_contract: Some(contract),
        quality: Some(if document["meta"]["quality_profile"] == "showcase" {
            input::Quality::Showcase
        } else {
            input::Quality::Standard
        }),
        ..Default::default()
    };
    let parsed = parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Architecture,
        &options,
    )
    .unwrap();
    architecture::compile(&parsed, &options)
}

fn near(actual: f64, expected: &Value, context: &str) {
    assert!(
        (actual - expected.as_f64().unwrap()).abs() < 0.00001,
        "{context}: {actual} != {expected}"
    );
}

#[test]
fn frozen_legacy_and_reference_geometry_oracles() {
    let corpus: Value =
        serde_json::from_slice(include_bytes!("fixtures/architecture/oracle.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        for (name, contract) in [
            ("legacy-2.17", InputContract::Legacy217),
            ("phoenix-v1", InputContract::PhoenixV1),
        ] {
            let expected = &case["oracles"][name];
            if expected["success"] != true {
                if case["name"] == "dominant-axis" || case["name"] == "obstacle" {
                    assert!(
                        compile(&case["input"], contract).is_err(),
                        "{} / {name} must retain the source rejection",
                        case["name"]
                    );
                }
                continue;
            }
            let label = format!("{name}/{}", case["name"]);
            let actual = compile(&case["input"], contract)
                .unwrap_or_else(|errors| panic!("{label}: {errors:?}"));
            assert_eq!(
                actual.schema_version,
                case["input"]["schema_version"].as_u64().unwrap() as u32
            );
            assert_eq!(actual.meta, case["input"]["meta"]);
            match &actual.facts {
                scene::DiagramFacts::Architecture {
                    layout,
                    components,
                    boundaries,
                } => {
                    assert_eq!(layout.as_ref(), case["input"].get("layout"));
                    assert_eq!(components, case["input"]["components"].as_array().unwrap());
                    assert_eq!(
                        boundaries,
                        case["input"]["boundaries"].as_array().unwrap_or(&vec![])
                    );
                }
                _ => panic!("Architecture must retain its diagram-specific facts"),
            }
            for (index, node) in actual.nodes.iter().enumerate() {
                assert_eq!(node.authored.facts, case["input"]["components"][index]);
                assert_eq!(node.authored.source_path, format!("/components/{index}"));
                assert!(!node.authored.geometry_provenance.is_empty());
            }
            for (index, edge) in actual.relations.iter().enumerate() {
                assert_eq!(edge.authored.facts, case["input"]["connections"][index]);
                assert_eq!(edge.authored.source_path, format!("/connections/{index}"));
                assert!(!edge.authored.geometry_provenance.is_empty());
            }
            near(actual.view_box.width, &expected["viewBox"][0], &label);
            near(actual.view_box.height, &expected["viewBox"][1], &label);
            assert_eq!(
                actual.nodes.len(),
                expected["components"].as_array().unwrap().len(),
                "{label}"
            );
            assert_eq!(
                actual.containers.len(),
                expected["boundaries"].as_array().unwrap().len(),
                "{label}"
            );
            for (node, oracle) in actual
                .nodes
                .iter()
                .zip(expected["components"].as_array().unwrap())
            {
                assert_eq!(node.authored.id, oracle["id"].as_str().unwrap());
                for (a, field) in [
                    (node.bounds.x, "x"),
                    (node.bounds.y, "y"),
                    (node.bounds.width, "width"),
                    (node.bounds.height, "height"),
                ] {
                    near(a, &oracle[field], &label);
                }
            }
            for (boundary, oracle) in actual
                .containers
                .iter()
                .zip(expected["boundaries"].as_array().unwrap())
            {
                for (a, field) in [
                    (boundary.bounds.x, "x"),
                    (boundary.bounds.y, "y"),
                    (boundary.bounds.width, "width"),
                    (boundary.bounds.height, "height"),
                ] {
                    near(a, &oracle[field], &label);
                }
                let plate = boundary
                    .label_plate
                    .expect("every architecture boundary has a measured title mask");
                for (a, field) in [
                    (plate.x, "x"),
                    (plate.y, "y"),
                    (plate.width, "width"),
                    (plate.height, "height"),
                ] {
                    near(
                        a,
                        &oracle["titlePlate"][field],
                        &format!("{label}/boundary-title/{field}"),
                    );
                }
            }
            assert_eq!(
                actual.relations.len(),
                expected["connections"].as_array().unwrap().len(),
                "{label}"
            );
            for (edge, oracle) in actual
                .relations
                .iter()
                .zip(expected["connections"].as_array().unwrap())
            {
                assert_eq!(
                    edge.crossover_halo,
                    oracle["crossoverHalo"].as_bool().unwrap()
                );
                assert_eq!(
                    edge.independent_ports,
                    oracle["independentPorts"].as_bool().unwrap()
                );
                assert_eq!(
                    edge.points.len(),
                    oracle["points"].as_array().unwrap().len(),
                    "{label}/{}: {:?}",
                    edge.from,
                    edge.points
                );
                for (point, oracle_point) in
                    edge.points.iter().zip(oracle["points"].as_array().unwrap())
                {
                    near(point.x, &oracle_point[0], &label);
                    near(point.y, &oracle_point[1], &label);
                }
                if oracle["labelAt"].is_array() {
                    near(edge.text[0].position.x, &oracle["labelAt"][0], &label);
                    near(edge.text[0].position.y, &oracle["labelAt"][1], &label);
                }
            }
        }
    }
}

#[test]
fn exact_source_facts_and_explicit_waypoints_survive_compilation() {
    let corpus: Value =
        serde_json::from_slice(include_bytes!("fixtures/architecture/oracle.json")).unwrap();
    let document = &corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "pins")
        .unwrap()["input"];
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let scene = compile(document, contract).unwrap();
        assert_eq!(scene.meta, document["meta"]);
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.nodes[0].authored.facts, document["components"][0]);
        assert_eq!(scene.nodes[0].authored.source_path, "/components/0");
        assert_eq!(
            scene.relations[0].authored.facts,
            document["connections"][0]
        );
        assert_eq!(scene.relations[0].authored.source_path, "/connections/0");
        assert!(!scene.relations[0].authored.geometry_provenance.is_empty());
        assert_eq!(scene.relations[0].points.len(), 6);
        assert_eq!(scene.relations[0].points[1], scene.relations[0].points[2]);
    }
}

#[test]
fn invalid_grid_and_long_text_have_precise_diagnostics() {
    let corpus: Value =
        serde_json::from_slice(include_bytes!("fixtures/architecture/oracle.json")).unwrap();
    let mut grid = corpus["cases"][1]["input"].clone();
    grid["components"][1]["col"] = json!(3);
    let errors = compile(&grid, InputContract::PhoenixV1).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "architecture/grid-column"
                && error.subject.path == "/components/1/col"),
        "{errors:?}"
    );
    let mut free = corpus["cases"][0]["input"].clone();
    free["components"][0]["label"] =
        json!("a very long responsibility that cannot fit the fixed box");
    let errors = compile(&free, InputContract::PhoenixV1).unwrap_err();
    assert!(
        errors.iter().any(
            |error| error.code == "architecture/component-label-overflow"
                && error.subject.path == "/components/0/label"
        ),
        "{errors:?}"
    );
}

fn scopes() -> Value {
    json!({"schema_version":1,"diagram_type":"architecture",
        "meta":{"title":"Boundary ownership","output":"diagram.html","quality_profile":"standard","legend":{"mode":"hidden"}},
        "components":[
            {"id":"a","type":"backend","label":"A","pos":[80,120]},
            {"id":"middle","type":"backend","label":"Other responsibility","pos":[240,120],"size":[120,60]},
            {"id":"c","type":"database","label":"C","pos":[400,120]}
        ],"boundaries":[{"kind":"region","label":"Scope","wraps":["a","c"]}],"connections":[]})
}

#[test]
fn showcase_rejects_false_membership_while_standard_and_legacy_keep_their_scope() {
    let mut document = scopes();
    document["components"][1]["label"] = json!("Other");
    let standard = compile(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!(standard.containers[0].members, vec!["a", "c"]);
    document["meta"]["quality_profile"] = json!("showcase");
    let errors = compile(&document, InputContract::PhoenixV1).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "layout/boundary-encloses-non-member"
                && error.subject.path == "/boundaries/0/wraps"),
        "{errors:?}"
    );
    let legacy = compile(&document, InputContract::Legacy217).unwrap();
    assert_eq!(legacy.containers[0].members, vec!["a", "c"]);
}

#[test]
fn deployment_frames_must_agree_with_their_nested_membership_direction() {
    let document = json!({"schema_version":1,"diagram_type":"architecture",
        "meta":{"title":"Nested scope","output":"diagram.html","quality_profile":"standard","engineering_profile":"deployment-ownership","legend":{"mode":"hidden"}},
        "components":[{"id":"a","type":"backend","label":"A","tag":"Team A","pos":[400,350]},{"id":"b","type":"backend","label":"B","tag":"Team B","pos":[750,350]}],
        "boundaries":[{"kind":"region","label":"Outer","wraps":["a","b"],"pad":10},{"kind":"security-group","label":"Inner","wraps":["a"],"pad":200}],"connections":[]});
    let errors = compile(&document, InputContract::PhoenixV1).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "architecture/boundary-membership"
                && error.subject.path == "/boundaries/0/wraps"),
        "{errors:?}"
    );
    let mut ordinary = document.clone();
    ordinary["meta"]
        .as_object_mut()
        .unwrap()
        .remove("engineering_profile");
    assert!(
        compile(&ordinary, InputContract::PhoenixV1).is_ok(),
        "ordinary boundaries are independent sets"
    );
}

#[test]
fn legend_translations_and_outside_cards_retain_all_source_text() {
    let corpus: Value =
        serde_json::from_slice(include_bytes!("fixtures/architecture/oracle.json")).unwrap();
    let mut document = corpus["cases"][0]["input"].clone();
    document["meta"]["locale"] = json!("ko");
    document["meta"]["legend"] = json!({"mode":"auto"});
    document["meta"]["translations"] =
        json!({"legend.title":"범례 교정","legend.architecture.backend":"서버 책임"});
    document["cards"] = json!([{"dot":"cyan","title":"Responsibility","items":["Do not drop this fact","Preserve $& and <literal> text"]}]);
    let scene = compile(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!(scene.legends.len(), 2);
    assert_eq!(scene.legends[0].text[0].content, "범례 교정");
    assert_eq!(scene.legends[1].text[0].content, "서버 책임");
    assert!(scene
        .legends
        .iter()
        .all(|legend| legend.bounds.width > 0.0 && legend.bounds.height > 0.0));
    assert_eq!(scene.cards[0].authored.facts, document["cards"][0]);
    assert_eq!(scene.cards[0].bounds, scene::Rect::default());
    assert_eq!(
        scene.cards[0].text[2].content,
        "Preserve $& and <literal> text"
    );
}

#[test]
fn pinned_endpoint_directions_fail_without_rewriting_the_input() {
    let corpus: Value =
        serde_json::from_slice(include_bytes!("fixtures/architecture/oracle.json")).unwrap();
    let mut document = corpus["cases"][0]["input"].clone();
    document["connections"][0] = json!({"id":"wrong-side","from":"client","to":"service","fromSide":"left","toSide":"left","route":"orthogonal-h"});
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        ..Default::default()
    };
    let bytes = serde_json::to_vec(&document).unwrap();
    let parsed = parse_input(&bytes, DiagramType::Architecture, &options).unwrap();
    let errors = architecture::compile(&parsed, &options).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "clean-flow/endpoint-side-direction"
                && error.subject.path == "/connections/0"),
        "{errors:?}"
    );
    assert_eq!(parsed.source_bytes, bytes);
    assert_eq!(parsed.document, document);
}

#[test]
fn deployment_semantics_fail_before_layout_when_ownership_is_unstated() {
    let document = json!({"schema_version":1,"diagram_type":"architecture",
        "meta":{"title":"Ownership","output":"diagram.html","engineering_profile":"deployment-ownership"},
        "components":[{"id":"a","type":"backend","label":"A","pos":[100,100]},{"id":"db","type":"database","label":"DB","tag":"Storage","pos":[400,100]}],
        "boundaries":[{"kind":"region","label":"Region","wraps":["a"]},{"kind":"security-group","label":"Private","wraps":["a","db"]}],
        "connections":[{"from":"a","to":"db"}]});
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let errors = compile(&document, contract).unwrap_err();
        for code in [
            "engineering/deployment-owner-missing",
            "engineering/deployment-region-scope",
            "engineering/deployment-private-region-consistency",
            "engineering/deployment-crossing-mechanism",
        ] {
            assert!(
                errors
                    .iter()
                    .any(|error| error.code == code && error.stage == "input"),
                "{code}: {errors:?}"
            );
        }
        let mut missing_kinds = document.clone();
        missing_kinds["boundaries"] = json!([]);
        let errors = compile(&missing_kinds, contract).unwrap_err();
        assert_eq!(
            errors
                .iter()
                .filter(|error| error.code == "engineering/deployment-boundary-kind")
                .count(),
            2
        );
        assert!(errors
            .iter()
            .any(|error| error.code == "engineering/deployment-private-state"));
    }
}

#[test]
fn automatic_canvas_covers_outer_unlabelled_routes_and_authored_canvas_stays_fixed() {
    let document = json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Outer corridor","output":"diagram.html","legend":{"mode":"hidden"}},
        "components":[{"id":"a","type":"backend","label":"A","pos":[60,100]},{"id":"b","type":"backend","label":"B","pos":[360,100]}],
        "connections":[{"from":"a","to":"b","fromSide":"bottom","toSide":"bottom","via":[[120,260],[700,260],[700,320],[420,320]]}]});
    let new = compile(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!((new.view_box.width, new.view_box.height), (740.0, 388.0));
    let old = compile(&document, InputContract::Legacy217).unwrap();
    assert_eq!((old.view_box.width, old.view_box.height), (520.0, 228.0));
    assert_eq!(old.relations[0].points, new.relations[0].points);
    let mut authored = document;
    authored["meta"]["viewBox"] = json!([800, 500]);
    let fixed = compile(&authored, InputContract::PhoenixV1).unwrap();
    assert_eq!(
        (fixed.view_box.width, fixed.view_box.height),
        (800.0, 500.0)
    );
    assert_eq!(fixed.relations[0].points, new.relations[0].points);
}

#[test]
fn integral_json_numbers_are_valid_grid_indices() {
    let document = json!({"schema_version":1.0,"diagram_type":"architecture","meta":{"title":"Grid number domain","output":"diagram.html","legend":{"mode":"hidden"}},
        "layout":{"mode":"grid","cols":6.0},
        "components":[{"id":"node","type":"backend","label":"Node","row":1.0,"col":5.0}]});
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let scene = compile(&document, contract).unwrap();
        assert_eq!(
            (scene.nodes[0].bounds.x, scene.nodes[0].bounds.y),
            (840.0, 184.0)
        );
    }
}

fn direction_cue_svg(scene: &scene::CompiledDiagram) -> Html {
    let bytes = render::render(scene, &render::EmbeddedAssets::builtin()).unwrap();
    Html::parse_document(std::str::from_utf8(&bytes).unwrap())
}

fn direction_cue_bounds(
    html: &Html,
    scene: &scene::CompiledDiagram,
    index: usize,
) -> Option<scene::Rect> {
    let path = html
        .select(&Selector::parse(&format!("path[data-edge-key='{index}']")).unwrap())
        .next()
        .unwrap();
    let reference = path.value().attr("marker-start")?;
    let marker = html
        .select(
            &Selector::parse(&format!(
                "marker[id='{}']",
                reference
                    .strip_prefix("url(#")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap()
            ))
            .unwrap(),
        )
        .next()
        .unwrap();
    assert_eq!(marker.value().attr("markerUnits"), Some("userSpaceOnUse"));
    let number = |key| marker.value().attr(key).unwrap().parse::<f64>().unwrap();
    let polygon = marker
        .select(&Selector::parse("polygon").unwrap())
        .next()
        .unwrap();
    let coordinates: Vec<f64> = polygon
        .value()
        .attr("points")
        .unwrap()
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|v| !v.is_empty())
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(
        coordinates.len(),
        6,
        "one triangular cue, not another relationship"
    );
    let points = &scene.relations[index].points;
    let length = (points[1].x - points[0].x).hypot(points[1].y - points[0].y);
    let (x, y) = (
        (points[1].x - points[0].x) / length,
        (points[1].y - points[0].y) / length,
    );
    let actual: Vec<_> = coordinates
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| scene::Point {
            x: points[0].x + x * (p[0] - number("refX")) - y * (p[1] - number("refY")),
            y: points[0].y + y * (p[0] - number("refX")) + x * (p[1] - number("refY")),
        })
        .collect();
    let min_x = actual.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let min_y = actual.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    Some(scene::Rect {
        x: min_x,
        y: min_y,
        width: actual.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max) - min_x,
        height: actual.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max) - min_y,
    })
}

#[test]
fn direction_cue_preserves_the_frozen_legacy_pins_and_all_facts() {
    let document: Value = serde_json::from_slice(include_bytes!(
        "../upstream/legacy-2.17/examples/web-app.architecture.json"
    ))
    .unwrap();
    let compiled = compile(&document, InputContract::Legacy217).unwrap();
    let html = direction_cue_svg(&compiled);
    let cue = direction_cue_bounds(&html, &compiled, 5)
        .expect("masked upward endpoint needs a redundant same-edge direction cue");
    assert!(cue.width > 5.0 && cue.height > 5.0);
    assert_eq!(
        html.select(&Selector::parse("path[data-edge-from][data-edge-to]").unwrap())
            .count(),
        9
    );
    let path = html
        .select(&Selector::parse("path[data-edge-key='5']").unwrap())
        .next()
        .unwrap();
    assert_eq!(path.value().attr("d"), Some("M 735 300 L 735 210"));
    assert_eq!(
        path.value().attr("data-composition-points"),
        Some("735,300;735,210")
    );
    assert_eq!(path.value().attr("marker-end"), Some("url(#arrowhead)"));
    let metadata: Value = serde_json::from_str(
        &html
            .select(&Selector::parse("#phoenix-diagram-metadata").unwrap())
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
    )
    .unwrap();
    assert_eq!(metadata["graph"]["relations"], document["connections"]);
    assert_eq!(metadata["graph"]["nodes"], document["components"]);
    assert_eq!(metadata["graph"]["relations"][5]["labelDy"], -68);
    let title = html
        .select(&Selector::parse("[data-graph-role='structural-frame-label-mask']").unwrap())
        .next()
        .unwrap();
    for (key, value) in [
        ("x", "224"),
        ("y", "130"),
        ("width", "123.4"),
        ("height", "16"),
    ] {
        assert_eq!(title.value().attr(key), Some(value));
    }
    for plate in compiled
        .nodes
        .iter()
        .map(|node| node.bounds)
        .chain(
            compiled
                .relations
                .iter()
                .filter_map(|edge| edge.label_plate),
        )
        .chain(
            compiled
                .containers
                .iter()
                .filter_map(|frame| frame.label_plate),
        )
    {
        assert!(
            !geometry::rects_overlap(&cue, &plate, 0.0),
            "cue crossed an existing node/label/title: {cue:?} {plate:?}"
        );
    }
    for other in html
        .select(&Selector::parse("path[data-edge-from][data-edge-to]").unwrap())
        .filter(|p| p.value().attr("data-edge-key") != Some("5"))
    {
        let analysis =
            phoenix_diagram::check::path::analyze_svg_path(other.value().attr("d").unwrap())
                .unwrap();
        let half_width = other
            .value()
            .attr("stroke-width")
            .unwrap()
            .parse::<f64>()
            .unwrap()
            / 2.0;
        for segment in analysis.subpaths.iter().flat_map(|part| &part.segments) {
            assert!(
                !geometry::segment_intersects_rect(segment.start, segment.end, &cue, half_width),
                "cue crossed another actual path"
            );
        }
    }
}

#[test]
fn direction_cue_is_generic_for_all_four_straight_directions() {
    for (source, target, from_side, to_side, label_at) in [
        ([100, 300], [100, 100], "top", "bottom", [160, 172]),
        ([100, 100], [100, 300], "bottom", "top", [160, 294]),
        ([100, 100], [400, 100], "right", "left", [383, 130]),
        ([400, 100], [100, 100], "left", "right", [237, 130]),
    ] {
        let document = json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Pinned cue","output":"diagram.html","legend":{"mode":"hidden"}},"components":[{"id":"origin","type":"backend","label":"Source","pos":source,"size":[120,60]},{"id":"destination","type":"database","label":"Target","pos":target,"size":[120,60]}],"connections":[{"from":"origin","to":"destination","fromSide":from_side,"toSide":to_side,"label":"R","labelAt":label_at}]});
        let compiled = compile(&document, InputContract::Legacy217).unwrap();
        let html = direction_cue_svg(&compiled);
        let cue = direction_cue_bounds(&html, &compiled, 0)
            .expect("the generic blocked-marker rule must not depend on sample ids");
        for plate in compiled.nodes.iter().map(|node| node.bounds).chain(
            compiled
                .relations
                .iter()
                .filter_map(|edge| edge.label_plate),
        ) {
            assert!(!geometry::rects_overlap(&cue, &plate, 0.0));
        }
        let path = html
            .select(&Selector::parse("path[data-edge-key='0']").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            path.value().attr("d"),
            Some(geometry::rounded_path(&compiled.relations[0].points, 8.0).as_str())
        );
        assert_eq!(path.value().attr("marker-end"), Some("url(#arrowhead)"));
    }
}

#[test]
fn direction_cue_leaves_unobstructed_and_fully_blocked_legacy_edges_unchanged() {
    let base = json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"No cue","output":"diagram.html","legend":{"mode":"hidden"}},"components":[{"id":"a","type":"backend","label":"A","pos":[100,100],"size":[120,60]},{"id":"b","type":"database","label":"B","pos":[100,184],"size":[120,60]}],"connections":[{"from":"a","to":"b","fromSide":"bottom","toSide":"top"}]});
    for labelled in [false, true] {
        let mut document = base.clone();
        if labelled {
            document["connections"][0]["label"] = json!("R");
            document["connections"][0]["labelAt"] = json!([160, 177]);
        }
        let compiled = compile(&document, InputContract::Legacy217).unwrap();
        let html = direction_cue_svg(&compiled);
        assert!(
            direction_cue_bounds(&html, &compiled, 0).is_none(),
            "do not move pins or invent an unsafe cue to force visual success"
        );
        let path = html
            .select(&Selector::parse("path[data-edge-key='0']").unwrap())
            .next()
            .unwrap();
        assert_eq!(path.value().attr("d"), Some("M 160 160 L 160 184"));
        assert_eq!(path.value().attr("marker-end"), Some("url(#arrowhead)"));
    }
}
