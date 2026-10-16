use input::{parse_input, CompileOptions, DiagramType, InputContract};
use phoenix_diagram::layout::architecture;
use phoenix_diagram::{diagnostic, input, scene};
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
