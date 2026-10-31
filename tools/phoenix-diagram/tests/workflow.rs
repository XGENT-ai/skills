use phoenix_diagram::{diagnostic, input, layout::workflow};

use input::{parse_input, CompileOptions, DiagramType, InputContract, Quality};
use serde_json::Value;

fn near(actual: f64, expected: &Value, context: &str) {
    let expected = expected.as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= 0.0001,
        "{context}: {actual} != {expected}"
    );
}

#[test]
fn reference_compiler_geometry_and_rejections_match_each_input_contract() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/oracle.json")).unwrap();
    let mut failures = Vec::new();
    for case in oracle["cases"].as_array().unwrap() {
        if std::panic::catch_unwind(|| check_case(case)).is_err() {
            failures.push(format!("{}/{}", case["contract"], case["name"]));
        }
    }
    assert!(
        failures.is_empty(),
        "Compiler oracle mismatches: {}",
        failures.join(", ")
    );
}

fn check_case(case: &Value) {
    let name = format!(
        "{}/{}",
        case["contract"].as_str().unwrap(),
        case["name"].as_str().unwrap()
    );
    let options = CompileOptions {
        input_contract: Some(if case["contract"] == "legacy-2.17" {
            InputContract::Legacy217
        } else {
            InputContract::PhoenixV1
        }),
        quality: Some(if case["quality"] == "showcase" {
            Quality::Showcase
        } else {
            Quality::Standard
        }),
        ..Default::default()
    };
    let mut document = case["document"].clone();
    if let Some(backend) = case["legends"]
        .as_array()
        .and_then(|entries| entries.iter().find(|entry| entry["kind"] == "backend"))
    {
        // Pin the frozen presentation through the public explicit-label
        // contract. Neutral defaults have their own rendered regression.
        document["meta"]["legend"]["entries"]["backend"]["label"] = backend["label"].clone();
    }
    let bytes = serde_json::to_vec(&document).unwrap();
    let parsed = parse_input(&bytes, DiagramType::Workflow, &options)
        .unwrap_or_else(|errors| panic!("{name}: input errors {errors:?}"));
    let result = workflow::compile(&parsed, &options);
    // A readable phoenix-v1 workflow that the source rejects only for
    // composition hits may pass after the native repair passes.
    if name == "phoenix-v1/optional-placement-advice" {
        crossing_resolved_with_halos(case, &result.unwrap());
        return;
    }
    assert_eq!(
        result.is_ok(),
        case["ok"].as_bool().unwrap(),
        "{name}: {result:?}"
    );
    if let Err(diagnostics) = &result {
        assert!(
            case["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|expected| diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected["code"].as_str().unwrap())),
            "{name}: wrong causal diagnostics {diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.stage == "layout"),
            "{name}"
        );
    }
    if let Ok(scene) = result {
        assert_eq!(parsed.source_bytes, bytes, "{name}: original input");
        assert_eq!(parsed.document, document, "{name}: submitted facts");
        let expected = &case["receipt"];
        near(scene.view_box.width, &expected["viewBox"][0], &name);
        near(scene.view_box.height, &expected["viewBox"][1], &name);
        assert_eq!(
            scene.reader_fit,
            case["readerFit"].as_str().unwrap_or("contain"),
            "{name}"
        );
        assert_eq!(
            scene.nodes.len(),
            expected["nodes"].as_array().unwrap().len(),
            "{name}"
        );
        for node in &scene.nodes {
            let found = expected["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|expected| expected["id"] == node.authored.id)
                .unwrap();
            for (actual, key) in [
                (node.bounds.x, "x"),
                (node.bounds.y, "y"),
                (node.bounds.width, "width"),
                (node.bounds.height, "height"),
            ] {
                near(
                    actual,
                    &found[key],
                    &format!("{name}/{}{key}", node.authored.id),
                );
            }
            assert_eq!(
                parsed.document.pointer(&node.authored.source_path),
                Some(&node.authored.facts)
            );
        }
        for row in case["nodeText"].as_array().into_iter().flatten() {
            let node = scene
                .nodes
                .iter()
                .find(|node| node.authored.id == row["node"].as_str().unwrap())
                .unwrap();
            let text = node
                .text
                .iter()
                .find(|text| {
                    text.source_path
                        .ends_with(&format!("/{}", row["field"].as_str().unwrap()))
                })
                .unwrap();
            near(text.position.x, &row["x"], &format!("{name} node text x"));
            near(text.position.y, &row["y"], &format!("{name} node text y"));
            near(
                text.font_size,
                &row["fontSize"],
                &format!("{name} node text font"),
            );
        }
        if let Some(expected) = case["sigils"].as_array() {
            assert_eq!(
                scene
                    .nodes
                    .iter()
                    .filter(|node| node.sigil.is_some())
                    .count(),
                expected.len(),
                "{name}"
            );
            for sigil in expected {
                let actual = scene
                    .nodes
                    .iter()
                    .find(|node| node.authored.id == sigil["node"].as_str().unwrap())
                    .unwrap()
                    .sigil
                    .as_ref()
                    .unwrap();
                assert_eq!(actual.kind, sigil["kind"].as_str().unwrap(), "{name}");
                near(actual.position.x, &sigil["x"], &name);
                near(actual.position.y, &sigil["y"], &name);
                near(actual.size, &sigil["size"], &name);
            }
        }
        if let Some(expected) = case["legends"].as_array() {
            let actual = scene
                .legends
                .iter()
                .filter(|legend| legend.authored.kind != "legend-title")
                .collect::<Vec<_>>();
            assert_eq!(actual.len(), expected.len(), "{name}");
            for (actual, expected) in actual.iter().zip(expected) {
                assert_eq!(actual.authored.kind, expected["kind"], "{name}");
                assert_eq!(actual.text[0].content, expected["label"], "{name}");
                near(actual.bounds.x, &expected["x"], &name);
                near(actual.bounds.y + 10.0, &expected["y"], &name);
                near(actual.bounds.width, &expected["width"], &name);
            }
        }
        assert_eq!(
            scene.relations.len(),
            expected["edges"].as_array().unwrap().len(),
            "{name}"
        );
        let mut label_index = 0;
        for (relation, found) in scene
            .relations
            .iter()
            .zip(expected["edges"].as_array().unwrap())
        {
            assert_eq!(found["from"], relation.from, "{name}");
            assert_eq!(found["to"], relation.to, "{name}");
            assert_eq!(
                found["id"],
                relation
                    .authored
                    .facts
                    .get("id")
                    .cloned()
                    .unwrap_or(Value::Null),
                "{name}"
            );
            assert_eq!(
                relation.points.len(),
                found["points"].as_array().unwrap().len(),
                "{name}/{}",
                relation.authored.id
            );
            for (actual, expected) in relation
                .points
                .iter()
                .zip(found["points"].as_array().unwrap())
            {
                near(actual.x, &expected[0], &name);
                near(actual.y, &expected[1], &name);
            }
            assert_eq!(
                parsed.document.pointer(&relation.authored.source_path),
                Some(&relation.authored.facts)
            );
            if !relation.label.is_empty() {
                let label = &expected["labels"][label_index];
                label_index += 1;
                assert_eq!(label["label"], relation.label, "{name}");
                let plate = relation.label_plate.unwrap();
                near(plate.x + plate.width / 2.0, &label["x"], &name);
                near(plate.y + 10.0, &label["y"], &name);
                near(plate.width, &label["width"], &name);
            }
        }
        assert_eq!(
            label_index,
            expected["labels"].as_array().unwrap().len(),
            "{name}"
        );
    }
}

fn fixture(name: &str, contract: &str) -> Value {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/oracle.json")).unwrap();
    oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name && case["contract"] == contract)
        .unwrap()["document"]
        .clone()
}

fn compile_document(
    document: &Value,
    contract: InputContract,
) -> Result<phoenix_diagram::scene::CompiledDiagram, diagnostic::Diagnostics> {
    let options = CompileOptions {
        input_contract: Some(contract),
        quality: Some(
            if document.pointer("/meta/quality_profile") == Some(&Value::String("showcase".into()))
            {
                Quality::Showcase
            } else {
                Quality::Standard
            },
        ),
        ..Default::default()
    };
    let bytes = serde_json::to_vec(document).unwrap();
    let input = parse_input(&bytes, DiagramType::Workflow, &options).unwrap();
    let result = workflow::compile(&input, &options);
    assert_eq!(input.document, *document);
    assert_eq!(input.source_bytes, bytes);
    result
}

#[test]
fn automatic_primary_labels_can_exceed_the_auxiliary_text_width_limit() {
    let mut document: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.workflow.json"
    ))
    .unwrap();
    let label = "预算已批准且金额 ≤ 人民币10000元？";
    document["nodes"][0]["label"] = label.into();
    for quality in ["standard", "showcase"] {
        document["meta"]["quality_profile"] = quality.into();
        let compiled = compile_document(&document, InputContract::PhoenixV1)
            .unwrap_or_else(|errors| panic!("{quality}: {errors:?}"));
        let node = compiled
            .nodes
            .iter()
            .find(|node| node.authored.id == "prepare")
            .unwrap();
        assert!(node.bounds.width > 200.0);
        assert_eq!(node.authored.facts, document["nodes"][0]);
        assert_eq!(node.text[0].content, label);
        assert!(node.text[0].font_size >= 9.0);
        assert_eq!(compiled.relations.len(), 1);
        assert_eq!(compiled.relations[0].from, "prepare");
        assert_eq!(compiled.relations[0].to, "complete");
    }
    document["nodes"][0]["width"] = 200.into();
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "layout/constraint"
            && diagnostic.subject.path == "/nodes/0/label"
            && diagnostic.evidence["availableWidthPx"] == 192.0
    }));
}

#[test]
fn automatic_workflow_routes_remain_separated_after_corner_rounding() {
    let document: Value = serde_json::from_str(include_str!(
        "fixtures/workflow/withdrawal-fanin.workflow.json"
    ))
    .unwrap();
    let scene = compile_document(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!(
        scene.nodes.len(),
        document["nodes"].as_array().unwrap().len()
    );
    assert_eq!(
        scene.relations.len(),
        document["edges"].as_array().unwrap().len()
    );
    let html = phoenix_diagram::render::render(
        &scene,
        &phoenix_diagram::render::EmbeddedAssets::builtin(),
    )
    .unwrap();
    let checked = phoenix_diagram::check::check_html(&html, Quality::Showcase);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);

    let mut crossing = scene.clone();
    for (id, coordinates) in [
        (
            "e-submit-check",
            vec![
                (152.0, 119.0),
                (174.0, 119.0),
                (174.0, 201.0),
                (220.0, 201.0),
                (220.0, 217.0),
            ],
        ),
        (
            "e-supplement-check",
            vec![
                (184.0, 119.0),
                (168.0, 119.0),
                (168.0, 296.0),
                (220.0, 296.0),
                (220.0, 269.0),
            ],
        ),
    ] {
        crossing
            .relations
            .iter_mut()
            .find(|relation| relation.authored.id == id)
            .unwrap()
            .points = coordinates
            .into_iter()
            .map(|(x, y)| phoenix_diagram::scene::Point { x, y })
            .collect();
    }
    let rejected = phoenix_diagram::check::check_html(
        &phoenix_diagram::render::render(
            &crossing,
            &phoenix_diagram::render::EmbeddedAssets::builtin(),
        )
        .unwrap(),
        Quality::Showcase,
    );
    assert!(rejected
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "composition/proper-crossing"));
}

#[test]
fn pin_failures_propose_only_repairs_that_pass_the_complete_native_compiler() {
    let document = fixture("v2-absolute-pins", "phoenix-v1");
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    let mut count = 0;
    for diagnostic in diagnostics {
        for repair in diagnostic.evidence["verifiedRepairs"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let mut candidate = document.clone();
            for change in repair["changes"].as_array().unwrap() {
                let path = change["path"].as_str().unwrap();
                if change["op"] == "remove" {
                    let (parent, key) = path.rsplit_once('/').unwrap();
                    candidate
                        .pointer_mut(parent)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(key);
                } else {
                    *candidate.pointer_mut(path).unwrap() = change["value"].clone();
                }
            }
            compile_document(&candidate, InputContract::PhoenixV1).unwrap();
            count += 1;
        }
    }
    assert!(count > 0, "Pin conflict needs a real, executable repair");
}

#[test]
fn measured_capacity_failure_preserves_facts_and_pins_in_its_receipt_evidence() {
    let document = fixture("v2-insufficient-authored-canvas", "phoenix-v1");
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    let capacity = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "workflow/viewbox-capacity")
        .unwrap();
    let scene: phoenix_diagram::scene::CompiledDiagram =
        serde_json::from_value(capacity.evidence["measuredScene"].clone()).unwrap();
    assert_eq!(scene.meta, document["meta"]);
    assert_eq!(scene.view_box.width, 720.0);
    assert!(capacity.evidence["requiredViewBox"][0].as_f64().unwrap() > 720.0);
    for node in scene.nodes {
        assert_eq!(
            document.pointer(&node.authored.source_path),
            Some(&node.authored.facts)
        );
    }
}

#[test]
fn compiled_nodes_edges_containers_and_cards_retain_authored_facts_and_geometry_provenance() {
    let document = fixture("example:agent-tool-call.workflow.json", "phoenix-v1");
    let compiled = compile_document(&document, InputContract::PhoenixV1).unwrap();
    for authored in compiled
        .nodes
        .iter()
        .map(|node| &node.authored)
        .chain(compiled.relations.iter().map(|relation| &relation.authored))
        .chain(
            compiled
                .containers
                .iter()
                .map(|container| &container.authored),
        )
        .chain(compiled.cards.iter().map(|card| &card.authored))
        .chain(compiled.legends.iter().map(|legend| &legend.authored))
    {
        assert_eq!(
            document.pointer(&authored.source_path),
            Some(&authored.facts)
        );
        for path in &authored.geometry_provenance {
            assert!(document.pointer(path).is_some(), "{path}");
        }
    }
    assert_eq!(compiled.schema_version, 2);
    assert!(compiled
        .relations
        .iter()
        .all(|relation| !relation.crossover_halo && !relation.independent_ports));
}

#[test]
fn readable_geometry_is_stable_across_repeated_and_reordered_source_arrays() {
    let document = fixture("fixture:workflow-shared-trunks.workflow.json", "phoenix-v1");
    let first = compile_document(&document, InputContract::PhoenixV1).unwrap();
    let second = compile_document(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!(first, second);
    let mut reversed = document.clone();
    for collection in ["nodes", "edges", "groups", "phases"] {
        if let Some(array) = reversed.get_mut(collection).and_then(Value::as_array_mut) {
            array.reverse();
        }
    }
    let reordered = compile_document(&reversed, InputContract::PhoenixV1).unwrap();
    assert_eq!(first.view_box, reordered.view_box);
    assert_eq!(
        first
            .nodes
            .iter()
            .map(|node| (&node.authored.id, node.bounds))
            .collect::<Vec<_>>(),
        reordered
            .nodes
            .iter()
            .map(|node| (&node.authored.id, node.bounds))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        first
            .relations
            .iter()
            .map(|relation| (
                &relation.authored.id,
                &relation.points,
                relation.label_plate
            ))
            .collect::<Vec<_>>(),
        reordered
            .relations
            .iter()
            .map(|relation| (
                &relation.authored.id,
                &relation.points,
                relation.label_plate
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn finite_authored_numbers_cannot_overflow_into_non_finite_scene_geometry() {
    let mut document = fixture("v2-simple", "phoenix-v1");
    document["lanes"] = serde_json::json!([{"id":"a","label":"A"},{"id":"b","label":"B"}]);
    document["nodes"] = serde_json::json!([
        {"id":"a","lane":"a","col":0,"type":"backend","label":"A","height":1e308},
        {"id":"b","lane":"b","col":0,"type":"backend","label":"B","height":1e308},
    ]);
    document["edges"] = serde_json::json!([]);
    document.as_object_mut().unwrap().remove("mainPath");
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "layout/constraint"));
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.evidence.get("measuredScene").is_none()));
}

// The source's only error is one crossing between two automatic edges; the
// native repair draws exactly that pair with crossover halos and keeps every
// authored fact.
fn crossing_resolved_with_halos(case: &Value, scene: &phoenix_diagram::scene::CompiledDiagram) {
    let diagnostics = case["diagnostics"].as_array().unwrap();
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic["code"] == "composition/proper-crossing"));
    let pair = [
        diagnostics[0]["subject"]["index"].as_u64().unwrap(),
        diagnostics[0]["evidence"]["otherRelationship"]["index"]
            .as_u64()
            .unwrap(),
    ];
    let edges = case["document"]["edges"].as_array().unwrap();
    for relation in &scene.relations {
        let index: u64 = relation.authored.source_path["/edges/".len()..]
            .parse()
            .unwrap();
        assert_eq!(relation.authored.facts, edges[index as usize]);
        assert_eq!(
            relation.crossover_halo,
            pair.contains(&index),
            "edge {index}"
        );
    }
}

#[test]
fn optional_placement_advice_matches_the_source_and_keeps_all_semantic_facts() {
    let document = fixture("optional-placement-advice", "phoenix-v1");
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/oracle.json")).unwrap();
    let source = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "optional-placement-advice")
        .unwrap();
    // Natively the crossing is drawn with halos instead of failing; the
    // source's optional placement still passes and keeps every fact.
    crossing_resolved_with_halos(
        source,
        &compile_document(&document, InputContract::PhoenixV1).unwrap(),
    );
    let repair = source["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| &diagnostic["evidence"]["verifiedRepair"])
        .find(|repair| repair["kind"] == "optional-node-placement")
        .unwrap();
    let mut candidate = document.clone();
    for change in repair["changes"].as_array().unwrap() {
        let (parent, key) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
        assert!(parent.starts_with("/nodes/"));
        assert!(["col", "yOffset"].contains(&key));
        candidate
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), change["value"].clone());
    }
    compile_document(&candidate, InputContract::PhoenixV1).unwrap();
    let mut original = document.clone();
    for value in [&mut candidate, &mut original] {
        for node in value["nodes"].as_array_mut().unwrap() {
            node.as_object_mut().unwrap().remove("col");
            node.as_object_mut().unwrap().remove("yOffset");
        }
    }
    assert_eq!(candidate, original);
}

#[test]
fn pinned_route_failure_keeps_its_cause_before_intrinsic_text_sizing() {
    let document = fixture(
        "pinned-route-failure-precedes-intrinsic-text-sizing",
        "phoenix-v1",
    );
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    assert_eq!(diagnostics[0].code, "workflow/explicit-pin-conflict");
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "workflow/absolute-geometry-sizing")
        .unwrap();
    assert_eq!(warning.severity, diagnostic::Severity::Warning);
    assert!(
        warning.supported_fixes.is_empty(),
        "Intrinsic width is not a verified complete workflow repair"
    );
    assert_eq!(warning.evidence["affectedNodes"][0]["node"], "a");
}

#[test]
fn canonical_edge_order_preserves_the_sources_utf16_comparison() {
    let document = fixture("unicode-canonical-edge-order", "phoenix-v1");
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let compiled = compile_document(&document, contract).unwrap();
        assert_eq!(
            compiled
                .relations
                .iter()
                .map(|relation| relation.label.as_str())
                .collect::<Vec<_>>(),
            ["🌳", "\u{e000}"]
        );
    }
}

fn same_json_numbers(actual: &Value, expected: &Value, context: &str) {
    match (actual, expected) {
        (Value::Number(actual), Value::Number(expected)) => assert!(
            (actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() <= 0.0001,
            "{context}: {actual} != {expected}"
        ),
        (Value::Array(actual), Value::Array(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{context}");
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                same_json_numbers(actual, expected, &format!("{context}/{index}"));
            }
        }
        (Value::Object(actual), Value::Object(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{context}");
            for (key, expected) in expected {
                same_json_numbers(&actual[key], expected, &format!("{context}/{key}"));
            }
        }
        _ => assert_eq!(actual, expected, "{context}"),
    }
}

fn apply_pin_changes(document: &Value, changes: &[Value]) -> Value {
    let mut candidate = document.clone();
    for change in changes {
        let (parent, field) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
        assert!(parent.starts_with("/edges/"));
        assert!([
            "via",
            "channelX",
            "channelY",
            "route",
            "fromSide",
            "toSide",
            "labelAt",
            "labelSegment"
        ]
        .contains(&field));
        let edge = candidate
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap();
        if change["op"] == "set" {
            assert!(["labelAt", "route", "labelSegment"].contains(&field));
            edge.insert(field.into(), change["value"].clone());
        } else {
            assert_eq!(change["op"], "remove");
            assert!(edge.remove(field).is_some());
        }
    }
    candidate
}

fn visible_curve_pin_case(case: &Value) -> Value {
    let mut native = case.clone();
    if case["contract"] == "phoenix-v1" && case["name"] == "crossing-redundant-controls/showcase" {
        let changes = case["repairs"][0].as_array().unwrap();
        assert_eq!(
            changes
                .iter()
                .map(|change| change["path"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "/edges/0/via",
                "/edges/0/channelY",
                "/edges/1/via",
                "/edges/1/channelX"
            ]
        );
        // Visible-curve routing keeps the first route's fixed geometry. The
        // frozen source oracle still records its older four-assertion repair.
        native["repairs"][0] = Value::Array(changes[2..].to_vec());
        native["diagnostic"]["evidence"]["conflictingPins"] = Value::Array(
            case["diagnostic"]["evidence"]["conflictingPins"]
                .as_array()
                .unwrap()[2..]
                .to_vec(),
        );
    }
    native
}

#[test]
fn cross_edge_pin_classification_and_measured_evidence_match_the_frozen_compilers() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/pin-oracle.json")).unwrap();
    let mut failures = Vec::new();
    let mut source_exceptions = 0;
    for case in oracle["cases"].as_array().unwrap() {
        let native = visible_curve_pin_case(case);
        let case = &native;
        let name = format!("{}/{}", case["contract"], case["name"]);
        if std::panic::catch_unwind(|| {
            let contract = if case["contract"] == "legacy-2.17" {
                InputContract::Legacy217
            } else {
                InputContract::PhoenixV1
            };
            if case["sourceSchemaValid"] == false {
                let options = CompileOptions {
                    input_contract: Some(contract),
                    ..Default::default()
                };
                let errors = parse_input(
                    &serde_json::to_vec(&case["document"]).unwrap(),
                    DiagramType::Workflow,
                    &options,
                )
                .unwrap_err();
                assert!(
                    errors.iter().any(|error| error.code == "input/schema"),
                    "{name}: {errors:?}"
                );
                return;
            }
            let result = compile_document(&case["document"], contract);
            if case.get("sourceException").is_some() {
                // The frozen legacy classifier reads absent raw-candidate
                // points. Native code must fail the real unsolved edge safely.
                assert_eq!(case["sourceException"]["name"], "TypeError");
                let errors = result.unwrap_err();
                assert_eq!(errors[0].code, "workflow/solver-budget-exhausted");
                return;
            }
            assert_eq!(
                result.is_ok(),
                case["ok"].as_bool().unwrap(),
                "{name}: {result:?}"
            );
            match result {
                Ok(scene) => {
                    for relation in scene.relations {
                        let expected = case["receipt"]["edges"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|edge| {
                                if edge["id"].is_string() {
                                    edge["id"] == relation.authored.id
                                } else {
                                    edge["from"] == relation.from && edge["to"] == relation.to
                                }
                            })
                            .unwrap();
                        let actual = serde_json::json!(relation
                            .points
                            .iter()
                            .map(|p| [p.x, p.y])
                            .collect::<Vec<_>>());
                        same_json_numbers(&actual, &expected["points"], &name);
                    }
                }
                Err(errors) => {
                    let actual = &errors[0];
                    let expected = &case["diagnostic"];
                    assert_eq!(
                        actual.code,
                        expected["code"].as_str().unwrap(),
                        "{name}: {errors:?}"
                    );
                    assert_eq!(
                        actual.subject.identity.as_deref(),
                        expected["subject"]["edge"].as_str(),
                        "{name}"
                    );
                    if [
                        "workflow/explicit-pin-conflict",
                        "workflow/route-preset-conflict",
                        "workflow/solver-budget-exhausted",
                    ]
                    .contains(&actual.code.as_str())
                    {
                        for (field, expected) in expected["evidence"].as_object().unwrap() {
                            same_json_numbers(
                                &actual.evidence[field],
                                expected,
                                &format!("{name}/{field}"),
                            );
                        }
                        if let Some(path) = expected["subject"]["path"].as_str() {
                            assert_eq!(actual.subject.path, path, "{name}");
                        }
                    }
                }
            }
        })
        .is_err()
        {
            failures.push(name);
        }
        source_exceptions += usize::from(case.get("sourceException").is_some());
    }
    assert_eq!(
        source_exceptions, 2,
        "Keep source exceptions visible rather than inventing an oracle"
    );
    assert!(
        failures.is_empty(),
        "Cross-edge pin mismatches: {}",
        failures.join(", ")
    );
}

#[test]
fn cross_edge_pin_repairs_match_source_minimal_sets_and_recompile_without_semantic_changes() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/pin-oracle.json")).unwrap();
    let mut failures = Vec::new();
    let mut joint_cases = 0;
    for case in oracle["cases"].as_array().unwrap().iter().filter(|case| {
        case["sourceSchemaValid"] != false
            && [
                "workflow/explicit-pin-conflict",
                "workflow/route-preset-conflict",
            ]
            .contains(&case["diagnostic"]["code"].as_str().unwrap_or_default())
    }) {
        let native = visible_curve_pin_case(case);
        let case = &native;
        let name = format!("{}/{}", case["contract"], case["name"]);
        if std::panic::catch_unwind(|| {
            let contract = if case["contract"] == "legacy-2.17" {
                InputContract::Legacy217
            } else {
                InputContract::PhoenixV1
            };
            let diagnostics = compile_document(&case["document"], contract).unwrap_err();
            let actual = diagnostics[0].evidence["verifiedRepairs"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let changes = Value::Array(
                actual
                    .iter()
                    .map(|repair| {
                        assert_eq!(repair["scope"], "complete-workflow-compile");
                        repair["changes"].clone()
                    })
                    .collect(),
            );
            same_json_numbers(&changes, &case["repairs"], &name);
            // Beside the source's minimal repairs, a readable phoenix-v1
            // workflow without one may get a verified removal of every
            // authored routing control.
            let workflow_repair = diagnostics[0].evidence.get("verifiedWorkflowRepair");
            if let Some(repair) = workflow_repair {
                assert_eq!(case["contract"], "phoenix-v1", "{name}");
                assert!(actual.is_empty(), "{name}");
                compile_document(&apply_workflow_repair(&case["document"], repair), contract)
                    .unwrap_or_else(|errors| panic!("{name}: unverified {repair}: {errors:?}"));
            }
            assert_eq!(
                diagnostics[0].supported_fixes.len(),
                actual.len() + usize::from(workflow_repair.is_some()),
                "{name}: every advertised change must have proof"
            );
            for (repair_index, repair) in actual.into_iter().enumerate() {
                let changes = repair["changes"].as_array().unwrap();
                if changes.len() == 1
                    && changes[0]["op"] == "set"
                    && changes[0]["path"].as_str().unwrap().ends_with("/labelAt")
                {
                    let at = diagnostics[0].supported_fixes[repair_index]
                        .rsplit_once(" to ")
                        .unwrap()
                        .1;
                    let advertised: Value = serde_json::from_str(at).unwrap();
                    for coordinate in 0..2 {
                        assert_eq!(
                            advertised[coordinate].as_f64(),
                            changes[0]["value"][coordinate].as_f64(),
                            "{name}: advertised coordinates must retain the verified precision"
                        );
                    }
                }
                let candidate = apply_pin_changes(&case["document"], changes);
                let compiled = compile_document(&candidate, contract).unwrap_or_else(|errors| {
                    panic!("{name}: unverified repair {repair}: {errors:?}")
                });
                if case["contract"] == "phoenix-v1"
                    && case["name"] == "crossing-redundant-controls/showcase"
                {
                    assert_eq!(candidate["edges"][0], case["document"]["edges"][0]);
                    let rendered = phoenix_diagram::render::render(
                        &compiled,
                        &phoenix_diagram::render::EmbeddedAssets::builtin(),
                    )
                    .unwrap();
                    let checked = phoenix_diagram::check::check_html(&rendered, Quality::Showcase);
                    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
                }
                // Only the advertised optional geometry fields may change.
                let mut before = case["document"].clone();
                let mut after = candidate;
                for document in [&mut before, &mut after] {
                    for edge in document["edges"].as_array_mut().unwrap() {
                        for field in [
                            "via",
                            "channelX",
                            "channelY",
                            "route",
                            "fromSide",
                            "toSide",
                            "labelAt",
                            "labelSegment",
                        ] {
                            edge.as_object_mut().unwrap().remove(field);
                        }
                    }
                }
                assert_eq!(
                    before, after,
                    "{name}: preserve nodes, edge semantics and mainPath"
                );
                if changes.len() > 1 {
                    assert!(changes.iter().all(|change| change["op"] == "remove"));
                    for mask in 1..(1usize << changes.len()) - 1 {
                        let subset = changes
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| mask & (1 << index) != 0)
                            .map(|(_, change)| change.clone())
                            .collect::<Vec<_>>();
                        assert!(
                            compile_document(
                                &apply_pin_changes(&case["document"], &subset),
                                contract
                            )
                            .is_err(),
                            "{name}: advertised removal set was not minimal"
                        );
                    }
                }
            }
        })
        .is_err()
        {
            failures.push(name);
        }
        joint_cases += usize::from(
            case["repairs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|repair| repair.as_array().unwrap().len() > 1),
        );
    }
    assert_eq!(
        joint_cases, 4,
        "Retain all four multi-assertion repair cases"
    );
    assert!(
        failures.is_empty(),
        "Cross-edge repair mismatches: {}",
        failures.join(", ")
    );
}

#[test]
fn preset_label_segment_repair_preserves_the_fixed_route_and_requires_a_complete_pass() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/pin-oracle.json")).unwrap();
    for quality in ["standard", "showcase"] {
        let case = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| {
                case["contract"] == "phoenix-v1"
                    && case["name"] == format!("preset-label-local-segment/{quality}")
            })
            .unwrap();
        let original = case["document"].clone();
        let errors = compile_document(&original, InputContract::PhoenixV1).unwrap_err();
        let first = errors[0].evidence["verifiedRepairs"][0]["changes"]
            .as_array()
            .unwrap();
        same_json_numbers(&Value::Array(first.clone()), &case["repairs"][0], quality);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0]["path"], "/edges/0/labelSegment");
        assert!(errors[0].supported_fixes[0].starts_with("set labelSegment on edge \"follow-up\""));
        let candidate = apply_pin_changes(&original, first);
        let actual = compile_document(&candidate, InputContract::PhoenixV1).unwrap();
        let mut baseline = original.clone();
        baseline["edges"][0]
            .as_object_mut()
            .unwrap()
            .remove("label");
        let baseline = compile_document(&baseline, InputContract::PhoenixV1).unwrap();
        assert_eq!(actual.nodes, baseline.nodes);
        assert_eq!(actual.relations[0].points, baseline.relations[0].points);
        assert_eq!(
            actual.relations[0].label,
            original["edges"][0]["label"].as_str().unwrap()
        );
        assert_eq!(
            candidate["edges"][0]["route"],
            original["edges"][0]["route"]
        );
        let plate = actual.relations[0].label_plate.unwrap();
        for node in actual.nodes {
            assert!(
                plate.x + plate.width <= node.bounds.x
                    || plate.x >= node.bounds.x + node.bounds.width
                    || plate.y + plate.height <= node.bounds.y
                    || plate.y >= node.bounds.y + node.bounds.height
            );
        }
        let mut expected = original.clone();
        expected["edges"][0]["labelSegment"] = first[0]["value"].clone();
        assert_eq!(
            candidate, expected,
            "Only the suggested optional label segment changes"
        );
    }
    for case in oracle["cases"].as_array().unwrap().iter().filter(|case| {
        case["contract"] == "phoenix-v1"
            && case["name"].as_str().unwrap().starts_with("preset-label-")
            && !case["name"].as_str().unwrap().contains("local-segment")
    }) {
        let errors = compile_document(&case["document"], InputContract::PhoenixV1).unwrap_err();
        assert!(errors[0]
            .supported_fixes
            .iter()
            .all(|fix| !fix.starts_with("set labelSegment ")));
        assert!(errors[0].evidence["verifiedRepairs"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|repair| repair["changes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|change| !change["path"].as_str().unwrap().ends_with("/labelSegment"))));
    }
}

#[test]
fn fixed_phoenix_group_titles_move_off_member_nodes_onto_a_clear_frame_border() {
    use phoenix_diagram::geometry::{rects_overlap, segment_intersects_rect, text_glyph_box};
    for name in [
        "example:incident-response.workflow.json",
        "example:release-delivery.workflow.json",
    ] {
        let document = fixture(name, "phoenix-v1");
        let scene = compile_document(&document, InputContract::PhoenixV1).unwrap();
        let groups = scene
            .containers
            .iter()
            .filter(|container| container.authored.kind == "group")
            .collect::<Vec<_>>();
        assert_eq!(groups.len(), 2);
        for group in groups {
            let title = &group.text[0];
            let glyph = text_glyph_box(
                &title.content,
                title.position.x,
                title.position.y,
                title.font_size,
                &title.anchor,
                InputContract::PhoenixV1,
            );
            // On the frame's top border, or in its lane-title row when tall
            // members block the border.
            assert!(
                [group.bounds.y - 2.0, group.bounds.y - 16.0].contains(&title.position.y),
                "{} at {}",
                title.content,
                title.position.y
            );
            assert!(
                glyph.x >= group.bounds.x + 10.0
                    && glyph.x + glyph.width <= group.bounds.x + group.bounds.width - 10.0,
                "{} stays on its own frame border",
                title.content
            );
            for node in &scene.nodes {
                assert!(
                    !rects_overlap(&glyph, &node.bounds, 0.0),
                    "{} overlaps {}",
                    title.content,
                    node.authored.id
                );
            }
            for relation in &scene.relations {
                assert!(
                    relation
                        .points
                        .windows(2)
                        .all(|pair| !segment_intersects_rect(pair[0], pair[1], &glyph, 5.0)),
                    "{} is crossed by {}->{}",
                    title.content,
                    relation.from,
                    relation.to
                );
            }
        }
    }
    // The 2.17 contract keeps its frozen in-frame title offset.
    let legacy = compile_document(
        &fixture("example:incident-response.workflow.json", "legacy-2.17"),
        InputContract::Legacy217,
    )
    .unwrap();
    for group in legacy
        .containers
        .iter()
        .filter(|container| container.authored.kind == "group")
    {
        assert_eq!(group.text[0].position.y, group.bounds.y + 14.0);
    }
}

// `document` with every text in `paths` cut to `limit` text units; at zero a
// label keeps its first character and other texts are removed.
fn cut_texts(document: &Value, paths: &[&str], limit: u64) -> Value {
    use phoenix_diagram::geometry::text_units_for_contract;
    let mut candidate = document.clone();
    for path in paths {
        let text = document.pointer(path).unwrap().as_str().unwrap();
        let mut kept = String::new();
        for character in text.chars() {
            let next = format!("{kept}{character}");
            if text_units_for_contract(&next, InputContract::PhoenixV1) > limit as f64 {
                break;
            }
            kept = next;
        }
        if kept.is_empty() && path.ends_with("/label") {
            kept = text.chars().take(1).collect();
        }
        if kept.is_empty() {
            let (parent, field) = path.rsplit_once('/').unwrap();
            candidate
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
        } else {
            *candidate.pointer_mut(path).unwrap() = kept.into();
        }
    }
    candidate
}

// `document` after a whole-workflow repair: the removed edge fields, and any
// texts within its verified limit.
fn apply_workflow_repair(document: &Value, repair: &Value) -> Value {
    let mut candidate = document.clone();
    for change in repair["changes"].as_array().unwrap() {
        assert_eq!(change["op"], "remove");
        let (parent, field) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
        assert!(parent.starts_with("/edges/"));
        assert!(candidate
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field)
            .is_some());
    }
    match repair.get("textCapacity") {
        Some(capacity) => cut_texts(
            &candidate,
            &capacity["paths"]
                .as_array()
                .unwrap()
                .iter()
                .map(|path| path.as_str().unwrap())
                .collect::<Vec<_>>(),
            capacity["maximumUnits"].as_u64().unwrap(),
        ),
        None => candidate,
    }
}

fn repair_fixture(name: &str) -> Value {
    let text = match name {
        "composition" => include_str!("fixtures/workflow/repair-composition.workflow.json"),
        "preset-pins" => include_str!("fixtures/workflow/repair-preset-pins.workflow.json"),
        _ => unreachable!(),
    };
    serde_json::from_str(text).unwrap()
}

#[test]
fn first_draft_composition_hits_are_repaired_without_changing_authored_facts() {
    let document = repair_fixture("composition");
    let scene = compile_document(&document, InputContract::PhoenixV1).unwrap();
    let again = compile_document(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!(
        serde_json::to_value(&scene).unwrap(),
        serde_json::to_value(&again).unwrap()
    );
    let edges = document["edges"].as_array().unwrap();
    assert_eq!(scene.relations.len(), edges.len());
    for relation in &scene.relations {
        let index: usize = relation.authored.source_path["/edges/".len()..]
            .parse()
            .unwrap();
        assert_eq!(relation.authored.facts, edges[index]);
        assert!(relation.authored.geometry_provenance.is_empty());
        assert_eq!(relation.from, edges[index]["from"].as_str().unwrap());
        assert_eq!(relation.to, edges[index]["to"].as_str().unwrap());
    }
    // Crossings that the lanes cannot avoid are drawn with crossover halos.
    assert!(scene
        .relations
        .iter()
        .any(|relation| relation.crossover_halo));
}

#[test]
fn pin_conflicts_without_their_own_repair_get_a_verified_whole_workflow_removal() {
    let document = repair_fixture("preset-pins");
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    let conflict = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "workflow/route-preset-conflict")
        .unwrap();
    assert!(conflict.evidence.get("verifiedRepairs").is_none());
    let repair = &conflict.evidence["verifiedWorkflowRepair"];
    assert_eq!(conflict.supported_fixes.len(), 1);
    let edges = repair["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|change| change["path"].as_str().unwrap().split('/').nth(2).unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(edges.len() > 1, "{repair}");
    let candidate = apply_workflow_repair(&document, repair);
    assert!(candidate["edges"].as_array().unwrap().iter().all(|edge| {
        ["via", "channelX", "channelY", "route", "fromSide", "toSide"]
            .iter()
            .all(|field| edge.get(*field).is_none())
    }));
    compile_document(&candidate, InputContract::PhoenixV1).unwrap();
}

#[test]
fn text_capacity_diagnostics_give_the_largest_verified_common_limit() {
    let mut document: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.workflow.json"
    ))
    .unwrap();
    for (index, sublabel) in [
        "Checks the request owner, budget line and approval chain",
        "Archives the signed request with its complete approval record",
    ]
    .into_iter()
    .enumerate()
    {
        document["nodes"][index]["width"] = 120.into();
        document["nodes"][index]["sublabel"] = sublabel.into();
    }
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    let named = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.evidence.get("verifiedTextCapacity").is_some())
        .collect::<Vec<_>>();
    assert_eq!(named.len(), 2);
    let capacity = &named[0].evidence["verifiedTextCapacity"];
    let limit = capacity["maximumUnits"].as_u64().unwrap();
    assert!(limit > 0);
    assert!(named[0].supported_fixes[0].contains(&format!("within {limit} text units")));
    let paths = capacity["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(paths, ["/nodes/0/sublabel", "/nodes/1/sublabel"]);
    let cut = |limit| cut_texts(&document, &paths, limit);
    compile_document(&cut(limit), InputContract::PhoenixV1).unwrap();
    assert!(compile_document(&cut(limit + 1), InputContract::PhoenixV1).is_err());
}

#[test]
fn backward_main_path_steps_get_the_fewest_column_changes() {
    let columns = [3, 1, 1, 2, 1, 0, 1, 2];
    let nodes = columns
        .iter()
        .enumerate()
        .map(|(index, col)| {
            serde_json::json!({"id":format!("n{index}"),"lane":"work","col":col,"type":"backend","label":format!("Step {index}")})
        })
        .collect::<Vec<_>>();
    let edges = (1..columns.len())
        .map(|index| {
            serde_json::json!({"id":format!("e{index}"),"from":format!("n{}", index - 1),"to":format!("n{index}"),"label":"Next","role":"main"})
        })
        .collect::<Vec<_>>();
    let mut document = serde_json::json!({
        "schema_version": 2,
        "diagram_type": "workflow",
        "meta": {"title":"Steps","output":"steps.html","input_contract":"phoenix-v1","quality_profile":"showcase","locale":"en"},
        "lanes": [{"id":"work","label":"Work"}],
        "nodes": nodes,
        "edges": edges,
        "mainPath": (0..columns.len()).map(|index| format!("n{index}")).collect::<Vec<_>>()
    });
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        ..Default::default()
    };
    let parse = |document: &Value| {
        parse_input(
            &serde_json::to_vec(document).unwrap(),
            DiagramType::Workflow,
            &options,
        )
    };
    let diagnostics = parse(&document).unwrap_err();
    let backward = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "input/main-path")
        .collect::<Vec<_>>();
    assert_eq!(backward.len(), 3);
    let fix = &backward[0].supported_fixes[0];
    // The longest non-decreasing run (1, 1, 1, 1, 2) stays.
    assert!(
        fix.starts_with("Set /nodes/0/col to 1, /nodes/3/col to 1, /nodes/5/col to 1 so"),
        "{fix}"
    );
    for diagnostic in &backward {
        assert_eq!(&diagnostic.supported_fixes[0], fix);
    }
    for (index, col) in [(0, 1), (3, 1), (5, 1)] {
        document["nodes"][index]["col"] = col.into();
    }
    if let Err(diagnostics) = parse(&document) {
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "input/main-path"));
    }
}

fn approval_fixture(legend_case: bool) -> Value {
    // Sealed R9 workflow snapshot 2 and its independently measured placement
    // with enough target-port capacity. Neither fixture is an expected layout.
    serde_json::from_str(if legend_case {
        include_str!("fixtures/workflow/approval-legend.workflow.json")
    } else {
        include_str!("fixtures/workflow/approval-capacity.workflow.json")
    })
    .unwrap()
}

fn apply_optional_placement(document: &Value, repair: &Value) -> Value {
    assert_eq!(repair["scope"], "complete-workflow-compile");
    let changes = repair["changes"].as_array().unwrap();
    assert!(!changes.is_empty());
    let mut candidate = document.clone();
    for change in changes {
        let path = change["path"].as_str().expect("Repair change needs a path");
        let op = change["op"].as_str().expect("Repair change needs an op");
        let (parent, field) = path.rsplit_once('/').expect("Node-field repair path");
        assert!(parent.starts_with("/nodes/"), "{op} {path}");
        assert!(
            ["col", "yOffset", "height"].contains(&field),
            "Unexpected optional geometry change: {op} {path}"
        );
        assert_eq!(
            op,
            if document.pointer(path).is_some() {
                "replace"
            } else {
                "add"
            },
            "Repair op must match the existing placement interface: {op} {path}"
        );
        if field == "height" {
            assert!(
                document.pointer(path).is_none(),
                "Do not overwrite height: {op} {path}"
            );
        }
        candidate
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), change["value"].clone());
    }
    let mut facts = candidate.clone();
    for change in changes {
        let path = change["path"].as_str().unwrap();
        let (parent, field) = path.rsplit_once('/').unwrap();
        let node = facts.pointer_mut(parent).unwrap().as_object_mut().unwrap();
        match document.pointer(path) {
            Some(value) => {
                node.insert(field.into(), value.clone());
            }
            None => {
                node.remove(field);
            }
        }
    }
    assert_eq!(
        facts, *document,
        "Only advertised optional geometry changes"
    );
    candidate
}

fn assert_approval_facts(document: &Value, scene: &phoenix_diagram::scene::CompiledDiagram) {
    assert_eq!(scene.quality, Quality::Showcase);
    assert_eq!(scene.meta, document["meta"]);
    assert_eq!(
        scene.nodes.len(),
        document["nodes"].as_array().unwrap().len()
    );
    assert_eq!(
        scene.relations.len(),
        document["edges"].as_array().unwrap().len()
    );
    for authored in scene
        .nodes
        .iter()
        .map(|node| &node.authored)
        .chain(scene.relations.iter().map(|edge| &edge.authored))
        .chain(scene.containers.iter().map(|lane| &lane.authored))
        .chain(scene.cards.iter().map(|card| &card.authored))
    {
        assert_eq!(
            document.pointer(&authored.source_path),
            Some(&authored.facts)
        );
    }
    let facts = serde_json::to_value(&scene.facts).unwrap();
    assert_eq!(facts["main_path"], document["mainPath"]);
    assert_eq!(facts["semantic_checks"], document["semanticChecks"]);
    // Observed in the original height64 DTO: reconfirm is 10px, the other
    // primary labels are 11px, and sublabels are 8px. These are this fixture's
    // unchanged text sizes, not a general minimum for workflow labels.
    for node in &scene.nodes {
        for text in &node.text {
            if text.source_path.ends_with("/label") {
                let expected = if node.authored.id == "reconfirm" {
                    10.0
                } else {
                    11.0
                };
                assert_eq!(text.font_size, expected, "{}", text.source_path);
            } else if text.source_path.ends_with("/sublabel") {
                assert_eq!(text.font_size, 8.0, "{}", text.source_path);
            }
        }
    }
}

#[test]
fn optional_placement_advice_resolves_target_capacity_without_changing_approval_facts() {
    let document = approval_fixture(false);
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    let advice = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.evidence["verifiedRepair"]["kind"] == "optional-node-placement"
        })
        .expect("The failed workflow needs an optional complete, compilable repair");
    assert!(!advice.supported_fixes.is_empty());
    let repair = &advice.evidence["verifiedRepair"];
    let candidate = apply_optional_placement(&document, repair);
    let scene = compile_document(&candidate, InputContract::PhoenixV1)
        .expect("Apply every advertised change, then compile the complete workflow");
    assert_approval_facts(&candidate, &scene);
}

#[test]
fn automatic_approval_legend_clears_the_rendered_withdrawal_route() {
    let document = approval_fixture(true);
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        quality: Some(Quality::Showcase),
        ..Default::default()
    };
    let bytes = serde_json::to_vec(&document).unwrap();
    let input = parse_input(&bytes, DiagramType::Workflow, &options).unwrap();
    // Match delivery::prepare, including the shared sigil-label clearance.
    let scene = phoenix_diagram::layout::compile(&input, &options).unwrap();
    assert_eq!(input.document, document);
    assert_eq!(input.source_bytes, bytes);
    assert_approval_facts(&document, &scene);
    assert_eq!(scene.reader_fit, "width-first");
    let html = phoenix_diagram::render::render(
        &scene,
        &phoenix_diagram::render::EmbeddedAssets::builtin(),
    )
    .unwrap();
    let checked = phoenix_diagram::check::check_html(&html, Quality::Showcase);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
    assert_eq!(checked.composition["status"], "pass");
    assert!(checked.checks.iter().all(|check| check.status == "pass"));
}

#[test]
fn optional_capacity_advice_does_not_overwrite_an_explicit_target_height() {
    let mut document = approval_fixture(false);
    // State the old effective height explicitly: it is now an authored pin.
    document["nodes"][8]["height"] = 52.into();
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == diagnostic::Severity::Error
            && diagnostic.code.starts_with("composition/")
    }));
    for diagnostic in &diagnostics {
        if let Some(changes) = diagnostic.evidence["verifiedRepair"]["changes"].as_array() {
            for change in changes {
                assert_ne!(
                    change["path"], "/nodes/8/height",
                    "An optional repair must retain the supplied target height: {change}"
                );
            }
        }
    }
}

#[test]
fn automatic_legend_keeps_clear_fixed_and_older_scenes_unchanged() {
    for (name, contract, source_contract) in [
        ("v2-simple", InputContract::PhoenixV1, "phoenix-v1"),
        ("v2-simple", InputContract::Legacy217, "legacy-2.17"),
        ("v1-simple", InputContract::PhoenixV1, "phoenix-v1"),
    ] {
        let document = fixture(name, source_contract);
        let scene = compile_document(&document, contract).unwrap();
        let html = phoenix_diagram::render::render(
            &scene,
            &phoenix_diagram::render::EmbeddedAssets::builtin(),
        )
        .unwrap();
        let checked = phoenix_diagram::check::check_html(&html, scene.quality);
        assert_eq!(
            checked
                .checks
                .iter()
                .find(|check| check.name == "legend_clearance")
                .unwrap()
                .status,
            "pass",
            "{source_contract}/{name}: {:?}",
            checked.diagnostics
        );

        // An explicit version of the already selected side pair is a public
        // fixed-route control. Compare the entire scene, removing only the
        // pin metadata that differs between the two input documents.
        let mut pinned = document.clone();
        pinned["edges"][0]["fromSide"] = "right".into();
        pinned["edges"][0]["toSide"] = "left".into();
        let mut fixed = compile_document(&pinned, contract).unwrap();
        assert_eq!(fixed.relations[0].authored.facts, pinned["edges"][0]);
        fixed.relations[0].authored.facts = document["edges"][0].clone();
        fixed.relations[0]
            .authored
            .geometry_provenance
            .retain(|path| !["/edges/0/fromSide", "/edges/0/toSide"].contains(&path.as_str()));
        assert_eq!(scene, fixed, "{source_contract}/{name}: clear full scene");

        let mut framed = document.clone();
        framed["meta"]["viewBox"] =
            serde_json::json!([scene.view_box.width, scene.view_box.height]);
        let mut fixed = compile_document(&framed, contract).unwrap();
        assert_eq!(fixed.meta["viewBox"], framed["meta"]["viewBox"]);
        fixed.meta.as_object_mut().unwrap().remove("viewBox");
        let title = fixed
            .legends
            .iter_mut()
            .find(|legend| legend.authored.source_path == "/meta")
            .expect("The source fixture's legend title retains its meta facts");
        assert_eq!(title.authored.facts["viewBox"], framed["meta"]["viewBox"]);
        // The title copies its /meta source facts, including the same pin.
        // Normalize only that duplicate; retain every geometry/text field.
        title
            .authored
            .facts
            .as_object_mut()
            .unwrap()
            .remove("viewBox");
        assert_eq!(scene, fixed, "{source_contract}/{name}: fixed full scene");
    }
}

#[test]
fn a_wrapped_automatic_legend_can_grow_the_canvas_without_changing_contain_policy() {
    let document = fixture("v2-simple", "phoenix-v1");
    let baseline = compile_document(&document, InputContract::PhoenixV1).unwrap();
    // The frozen source case uses contain. Longer legend text exercises real
    // automatic canvas growth, not an assignment to the readerFit field.
    assert_eq!(baseline.reader_fit, "contain");
    let mut wrapped = document.clone();
    wrapped["meta"]["legend"] = serde_json::json!({
        "mode": "auto",
        "entries": {
            "frontend": {"label": "前".repeat(50)},
            "backend": {"label": "后".repeat(50)}
        }
    });
    let expanded = compile_document(&wrapped, InputContract::PhoenixV1).unwrap();
    assert!(expanded.view_box.height > baseline.view_box.height);
    assert_eq!(expanded.reader_fit, baseline.reader_fit);
    assert_eq!(
        expanded.nodes, baseline.nodes,
        "Node geometry and fonts stay intact"
    );
    assert_eq!(expanded.relations, baseline.relations);
    assert_eq!(expanded.containers, baseline.containers);
    assert_eq!(expanded.facts, baseline.facts);
    assert_eq!(expanded.quality, baseline.quality);
    let html = phoenix_diagram::render::render(
        &expanded,
        &phoenix_diagram::render::EmbeddedAssets::builtin(),
    )
    .unwrap();
    let checked = phoenix_diagram::check::check_html(&html, expanded.quality);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
}
