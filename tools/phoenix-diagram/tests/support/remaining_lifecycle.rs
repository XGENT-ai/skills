use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, InputContract};
use phoenix_diagram::layout::compile;
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn lifecycle_legacy_keeps_the_three_bands_and_actual_transition_direction() {
    let document = json!({"schema_version":1,"diagram_type":"lifecycle","meta":{"title":"Legacy bands","output":"lifecycle.html","quality_profile":"standard"},
        "lanes":[{"id":"main","label":"Main"},{"id":"wait","label":"Wait"},{"id":"terminal","label":"End"}],
        "states":[{"id":"a","type":"start","label":"A","lane":"main","col":0.0},
            {"id":"b","type":"waiting","label":"B","lane":"wait","col":0.0,"yOffset":10},
            {"id":"c","type":"success","label":"C","lane":"terminal","col":1.0}],
        "transitions":[{"id":"ab","from":"a","to":"b","route":"straight","fromSide":"bottom","toSide":"top","via":[[94,240],[402,240]]}]});
    let options = CompileOptions {
        input_contract: Some(InputContract::Legacy217),
        ..Default::default()
    };
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Lifecycle,
        &options,
    )
    .unwrap();
    let scene = compile(&input, &options).unwrap();
    assert_eq!(scene.nodes[0].bounds.x, 35.0);
    assert_eq!(scene.nodes[0].bounds.y, 126.0);
    assert_eq!(scene.nodes[1].bounds.x, 339.0);
    assert_eq!(scene.nodes[1].bounds.y, 288.0);
    assert_eq!(scene.nodes[2].bounds.x, 497.0);
    assert_eq!(scene.nodes[2].bounds.y, 450.0);
    assert_eq!(scene.relations[0].from, "a");
    assert_eq!(scene.relations[0].to, "b");
}

fn near(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "{context}: actual {actual}, frozen SVG {expected}"
    );
}
fn compare_rect(actual: phoenix_diagram::scene::Rect, expected: &Value, context: &str) {
    for (key, value) in [
        ("x", actual.x),
        ("y", actual.y),
        ("width", actual.width),
        ("height", actual.height),
    ] {
        near(
            value,
            expected[key].as_f64().unwrap(),
            &format!("{context}/{key}"),
        );
    }
}
fn compare_texts(actual: &[phoenix_diagram::scene::Text], expected: &Value, context: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "{context}: text rows");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let context = format!("{context}/{index}");
        assert_eq!(actual.content, expected["content"], "{context}");
        assert_eq!(actual.anchor, expected["anchor"], "{context}");
        near(actual.position.x, expected["x"].as_f64().unwrap(), &context);
        near(actual.position.y, expected["y"].as_f64().unwrap(), &context);
        near(
            actual.font_size,
            expected["font"].as_f64().unwrap(),
            &context,
        );
    }
}

fn compare_frozen_cases(only: Option<&str>) {
    let oracle: Value =
        serde_json::from_str(include_str!("../fixtures/lifecycle/oracle.json")).unwrap();
    let mut accepted = 0;
    let mut rejected = 0;
    for case in oracle["cases"].as_array().unwrap() {
        if only.is_some_and(|name| case["name"] != name) {
            continue;
        }
        let contract = if case["contract"] == "legacy-2.17" {
            InputContract::Legacy217
        } else {
            InputContract::PhoenixV1
        };
        let context = format!("{}/{}", case["contract"], case["name"]);
        let options = CompileOptions {
            input_contract: Some(contract),
            ..Default::default()
        };
        let bytes = serde_json::to_vec(&case["document"]).unwrap();
        let parsed = parse_input(&bytes, DiagramType::Lifecycle, &options);
        if case["ok"] == false {
            rejected += 1;
            if let Ok(input) = parsed {
                let errors = compile(&input, &options)
                    .expect_err(&format!("{context}: frozen source rejects this geometry"));
                assert!(!errors.is_empty(), "{context}");
            }
            continue;
        }
        accepted += 1;
        let input = parsed.unwrap_or_else(|e| panic!("{context}: input {e:?}"));
        let scene = compile(&input, &options).unwrap_or_else(|e| panic!("{context}: layout {e:?}"));
        let expected = &case["geometry"];
        near(
            scene.view_box.width,
            expected["viewBox"][2].as_f64().unwrap(),
            &context,
        );
        near(
            scene.view_box.height,
            expected["viewBox"][3].as_f64().unwrap(),
            &context,
        );
        assert_eq!(
            scene.nodes.len(),
            expected["nodes"].as_array().unwrap().len(),
            "{context}"
        );
        for (node, expected) in scene
            .nodes
            .iter()
            .zip(expected["nodes"].as_array().unwrap())
        {
            let context = format!("{context}/{}", node.authored.id);
            assert_eq!(node.authored.id, expected["id"], "{context}");
            compare_rect(node.bounds, &expected["bounds"], &context);
            compare_texts(&node.text, &expected["texts"], &context);
            assert_eq!(
                node.authored.facts,
                case["document"]["states"][scene
                    .nodes
                    .iter()
                    .position(|n| n.authored.id == node.authored.id)
                    .unwrap()],
                "{context}: input state facts changed"
            );
        }
        assert_eq!(
            scene.relations.len(),
            expected["edges"].as_array().unwrap().len(),
            "{context}"
        );
        for (edge, expected_edge) in scene
            .relations
            .iter()
            .zip(expected["edges"].as_array().unwrap())
        {
            let index = expected_edge["key"].as_u64().unwrap() as usize;
            assert_eq!(edge.from, expected_edge["from"], "{context}");
            assert_eq!(edge.to, expected_edge["to"], "{context}");
            assert_eq!(
                edge.points.len(),
                expected_edge["points"].as_array().unwrap().len(),
                "{context}/{index}"
            );
            for (point, expected) in edge
                .points
                .iter()
                .zip(expected_edge["points"].as_array().unwrap())
            {
                near(point.x, expected[0].as_f64().unwrap(), &context);
                near(point.y, expected[1].as_f64().unwrap(), &context);
            }
            assert_eq!(
                edge.junction.as_deref(),
                expected_edge["junction"].as_str(),
                "{context}/{index}: junction"
            );
            assert_eq!(
                edge.authored.facts, case["document"]["transitions"][index],
                "{context}: transition facts changed"
            );
            let expected_label = expected["labels"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["key"] == index);
            match (edge.label_plate, expected_label) {
                (Some(rect), Some(expected)) => {
                    compare_rect(
                        rect,
                        &expected["bounds"],
                        &format!("{context}/label/{index}"),
                    );
                    compare_texts(
                        &edge.text,
                        &expected["texts"],
                        &format!("{context}/label/{index}"),
                    );
                }
                (None, None) => {
                    if !edge.label.is_empty() && contract == InputContract::PhoenixV1 {
                        let owner = edge
                            .label_owner
                            .expect("a merged label must have a visible owner");
                        assert_eq!(scene.relations[owner].label, edge.label);
                        assert_eq!(scene.relations[owner].note, edge.note);
                        assert!(scene.relations[owner].label_plate.is_some());
                    }
                }
                _ => panic!("{context}/{index}: label plate visibility differs"),
            }
        }
        let frames: Vec<_> = scene
            .containers
            .iter()
            .filter(|c| c.authored.kind == "lifecycle-frame")
            .collect();
        assert_eq!(
            frames.len(),
            expected["frames"].as_array().unwrap().len(),
            "{context}"
        );
        for (frame, expected) in frames.iter().zip(expected["frames"].as_array().unwrap()) {
            compare_rect(frame.bounds, expected, &context);
        }
        if contract == InputContract::PhoenixV1 {
            let phoenix_diagram::scene::DiagramFacts::Lifecycle { presentation, .. } = &scene.facts
            else {
                panic!("{context}: missing typed lifecycle presentation")
            };
            assert_eq!(
                presentation.junctions.len(),
                expected["junctions"].as_array().unwrap().len(),
                "{context}"
            );
            for (point, expected) in presentation
                .junctions
                .iter()
                .zip(expected["junctions"].as_array().unwrap())
            {
                near(point.x, expected[0].as_f64().unwrap(), &context);
                near(point.y, expected[1].as_f64().unwrap(), &context);
            }
            for final_state in &presentation.final_states {
                assert!(
                    !scene.relations.iter().any(|edge| edge.from == *final_state),
                    "{context}: fabricated terminal state"
                );
            }
        }
        let mut all_texts = vec![];
        all_texts.extend(scene.containers.iter().flat_map(|c| c.text.clone()));
        all_texts.extend(scene.nodes.iter().flat_map(|n| n.text.clone()));
        all_texts.extend(scene.relations.iter().flat_map(|e| e.text.clone()));
        all_texts.extend(scene.legends.iter().flat_map(|l| l.text.clone()));
        compare_texts(
            &all_texts,
            &expected["texts"],
            &format!("{context}/all-svg-texts"),
        );
    }
    if only.is_none() {
        assert_eq!(accepted, 24);
        assert_eq!(rejected, 7);
    } else {
        assert_eq!(accepted + rejected, 1);
    }
}

#[test]
fn lifecycle_native_scene_matches_both_frozen_renderer_svgs_without_rounding() {
    compare_frozen_cases(None);
}
#[test]
fn lifecycle_multiple_parallel_groups_keep_the_largest_row_height() {
    compare_frozen_cases(Some("multiple-parallel-groups-share-row-height"));
}
#[test]
fn lifecycle_translations_require_an_authored_locale() {
    compare_frozen_cases(Some("translations-without-locale-are-not-registered"));
}
#[test]
fn lifecycle_literal_braces_are_valid_translation_text() {
    compare_frozen_cases(Some("literal-brace-is-valid-translation"));
}
#[test]
fn lifecycle_numeric_placeholders_preserve_the_bundled_fallback() {
    compare_frozen_cases(Some("numeric-placeholder-is-not-legend-title"));
}

#[test]
fn lifecycle_oracle_matches_verified_sources_and_actual_svg_bytes() {
    let oracle: Value =
        serde_json::from_str(include_str!("../fixtures/lifecycle/oracle.json")).unwrap();
    let legacy: Value =
        serde_json::from_str(include_str!("../../upstream/legacy-2.17-manifest.json")).unwrap();
    let reference: Value =
        serde_json::from_str(include_str!("../../upstream/archify-3.1.0-manifest.json")).unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        let manifest = if case["contract"] == "legacy-2.17" {
            &legacy
        } else {
            &reference
        };
        let source = manifest
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r[0].as_str()
                    .unwrap()
                    .ends_with("renderers/lifecycle/render-lifecycle.mjs")
            })
            .unwrap();
        assert_eq!(case["sourceSha256"], source[2]);
        if let Some(file) = case["svgFile"].as_str() {
            let bytes = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/lifecycle")
                    .join(file),
            )
            .unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(bytes)), case["svgSha256"]);
        }
    }
}
