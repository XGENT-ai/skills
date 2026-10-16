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
    let bytes = serde_json::to_vec(&case["document"]).unwrap();
    let parsed = parse_input(&bytes, DiagramType::Workflow, &options)
        .unwrap_or_else(|errors| panic!("{name}: input errors {errors:?}"));
    let result = workflow::compile(&parsed, &options);
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
        assert_eq!(parsed.document, case["document"], "{name}: authored facts");
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

#[test]
fn optional_placement_advice_matches_the_source_and_keeps_all_semantic_facts() {
    let document = fixture("optional-placement-advice", "phoenix-v1");
    let diagnostics = compile_document(&document, InputContract::PhoenixV1).unwrap_err();
    let repair = diagnostics
        .iter()
        .map(|diagnostic| &diagnostic.evidence["verifiedRepair"])
        .find(|repair| repair["kind"] == "optional-node-placement")
        .expect("A fully verified source placement proposal");
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/oracle.json")).unwrap();
    let source = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "optional-placement-advice")
        .unwrap();
    let expected = source["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| &diagnostic["evidence"]["verifiedRepair"])
        .find(|repair| repair["kind"] == "optional-node-placement")
        .unwrap();
    assert_eq!(repair["changes"], expected["changes"]);
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

#[test]
fn cross_edge_pin_classification_and_measured_evidence_match_the_frozen_compilers() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/pin-oracle.json")).unwrap();
    let mut failures = Vec::new();
    let mut source_exceptions = 0;
    for case in oracle["cases"].as_array().unwrap() {
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
            assert_eq!(
                diagnostics[0].supported_fixes.len(),
                actual.len(),
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
                compile_document(&candidate, contract).unwrap_or_else(|errors| {
                    panic!("{name}: unverified repair {repair}: {errors:?}")
                });
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
        "Both contracts have two-edge minimal sets of two and four removals"
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
            && case["name"].as_str().unwrap().contains("local-segment") == false
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
