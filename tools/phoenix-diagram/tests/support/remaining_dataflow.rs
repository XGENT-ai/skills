use phoenix_diagram::geometry::{rects_overlap, segment_intersects_rect, text_glyph_box};
use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, InputContract, Quality};
use phoenix_diagram::layout as dataflow;
use phoenix_diagram::scene::{CompiledDiagram, DiagramFacts, Point, Rect, Text};
use serde_json::{json, Value};

fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/dataflow/oracle.json")).unwrap()
}
fn near(actual: f64, expected: &Value, context: &str) {
    assert!(
        (actual - expected.as_f64().unwrap()).abs() < 1e-8,
        "{context}: {actual} != {expected}"
    );
}
fn bounds(actual: Rect, expected: &Value, context: &str) {
    for (value, key) in [
        (actual.x, "x"),
        (actual.y, "y"),
        (actual.width, "width"),
        (actual.height, "height"),
    ] {
        near(value, &expected[key], &format!("{context}/{key}"));
    }
}
fn texts(actual: &[Text], expected: &[Value], context: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: every rendered line must survive"
    );
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let context = format!("{context}/text/{index}");
        assert_eq!(
            actual.content,
            expected["text"].as_str().unwrap(),
            "{context}"
        );
        assert_eq!(
            actual.anchor,
            expected["anchor"].as_str().unwrap(),
            "{context}"
        );
        near(actual.position.x, &expected["x"], &context);
        near(actual.position.y, &expected["y"], &context);
        near(actual.font_size, &expected["fontSize"], &context);
    }
}
/// An authored horizontal channel whose endpoint faces the frozen renderers
/// chose automatically (lateral, by node order).
fn inferred_channel(flow: &Value) -> bool {
    matches!(
        flow["route"].as_str(),
        Some("bottom-channel" | "top-channel")
    ) && flow.get("via").is_none()
        && ["fromSide", "toSide"]
            .iter()
            .all(|key| flow[*key].as_str().is_none_or(|side| side == "auto"))
}
fn on_border(points: &[Point], rect: &Rect) -> bool {
    let overlap = |a: f64, b: f64, low: f64, high: f64| a.max(b).min(high) - a.min(b).max(low);
    points.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        (a.x == b.x
            && (a.x == rect.x || a.x == rect.x + rect.width)
            && overlap(a.y, b.y, rect.y, rect.y + rect.height) > 0.0)
            || (a.y == b.y
                && (a.y == rect.y || a.y == rect.y + rect.height)
                && overlap(a.x, b.x, rect.x, rect.x + rect.width) > 0.0)
    })
}
/// D25 replaces only the frozen route of such a channel: its riser lands on
/// the target's lateral border, so the final segment and arrowhead run along
/// the border. The repaired route keeps the oracle's channel row and shape
/// but reaches both nodes perpendicularly through the face toward the channel.
fn channel_attachment(scene: &CompiledDiagram, index: usize, oracle: &Value, context: &str) {
    let flow = &scene.relations[index];
    let bounds = |id: &str| {
        scene
            .nodes
            .iter()
            .find(|node| node.authored.id == id)
            .unwrap()
            .bounds
    };
    let frozen: Vec<_> = oracle["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|point| Point {
            x: point[0].as_f64().unwrap(),
            y: point[1].as_f64().unwrap(),
        })
        .collect();
    let (from, to) = (bounds(&flow.from), bounds(&flow.to));
    assert!(
        on_border(&frozen, &to),
        "{context}: oracle no longer encodes the lateral channel defect; restore its exact route"
    );
    let points = &flow.points;
    assert_eq!(points.len(), frozen.len(), "{context}: channel route shape");
    assert_eq!(points.len(), 4, "{context}: channel route shape");
    let row = frozen[1].y;
    assert!(
        points[1].y == row && points[2].y == row,
        "{context}: keep the oracle channel row {row}: {points:?}"
    );
    for (rect, face, beyond) in [(from, points[0], points[1]), (to, points[3], points[2])] {
        let face_y = if row < rect.y + rect.height / 2.0 {
            rect.y
        } else {
            rect.y + rect.height
        };
        assert!(
            face.x == beyond.x
                && face.y == face_y
                && (beyond.y - face.y) * (row - face.y) > 0.0
                && face.x >= rect.x + 16.0
                && face.x <= rect.x + rect.width - 16.0,
            "{context}: channel must meet {rect:?} perpendicularly on its face toward row {row}: {points:?}"
        );
    }
    for node in &scene.nodes {
        assert!(
            !on_border(points, &node.bounds),
            "{context}: channel runs along {}: {points:?}",
            node.authored.id
        );
        if node.authored.id != flow.from && node.authored.id != flow.to {
            assert!(
                points.windows(2).all(|pair| !segment_intersects_rect(
                    pair[0],
                    pair[1],
                    &node.bounds,
                    2.0
                )),
                "{context}: channel crosses {}: {points:?}",
                node.authored.id
            );
        }
    }
}

#[test]
fn dataflow_double_contract_matches_actual_unrounded_svg_oracles() {
    let mut accepted = 0;
    let mut rejected = 0;
    let mut repaired_channels = 0;
    for case in corpus()["cases"].as_array().unwrap() {
        let contract = if case["contract"] == "legacy-2.17" {
            InputContract::Legacy217
        } else {
            InputContract::PhoenixV1
        };
        let options = CompileOptions {
            input_contract: Some(contract),
            quality: case["qualityOption"].as_str().map(|quality| {
                if quality == "standard" {
                    Quality::Standard
                } else {
                    Quality::Showcase
                }
            }),
            ..Default::default()
        };
        let bytes = serde_json::to_vec(&case["document"]).unwrap();
        let context = format!("{}/{}", case["contract"], case["name"]);
        let input = match parse_input(&bytes, DiagramType::Dataflow, &options) {
            Ok(input) => input,
            Err(errors) => {
                assert_ne!(
                    case["ok"], true,
                    "{context}: valid source core rejected by input gate: {errors:?}"
                );
                rejected += 1;
                continue;
            }
        };
        let before = input.document.clone();
        let result = dataflow::compile(&input, &options);
        assert_eq!(
            input.document, before,
            "{context}: authored source must remain intact"
        );
        assert_eq!(
            input.source_bytes, bytes,
            "{context}: input bytes must remain intact"
        );
        if case["ok"] != true {
            let errors = result.expect_err(&format!(
                "{context}: source failed layout must not become an accepted scene"
            ));
            assert!(!errors.is_empty(), "{context}");
            assert!(
                errors.iter().all(|error| !error.code.is_empty()
                    && (error.subject.path.is_empty() || error.subject.path.starts_with('/'))),
                "{context}"
            );
            for expected in case["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|error| {
                    error["code"]
                        .as_str()
                        .is_some_and(|code| code.starts_with("dataflow/node-"))
                })
            {
                assert!(
                    errors
                        .iter()
                        .any(|actual| actual.code == expected["code"].as_str().unwrap()),
                    "{context}: retain actionable source error {}",
                    expected["code"]
                );
            }
            rejected += 1;
            continue;
        }
        let scene = result
            .unwrap_or_else(|errors| panic!("{context}: source accepted actual SVG: {errors:?}"));
        accepted += 1;
        assert_eq!(scene.schema_version, 1);
        assert_eq!(
            serde_json::to_value(scene.quality).unwrap(),
            case["effectiveQuality"],
            "{context}: effective quality must drive the compiled layout"
        );
        assert_eq!(scene.meta, input.document["meta"]);
        let DiagramFacts::Dataflow {
            stages,
            nodes,
            flows,
        } = &scene.facts
        else {
            panic!("{context}: retain dataflow facts");
        };
        assert_eq!(stages, input.document["stages"].as_array().unwrap());
        assert_eq!(nodes, input.document["nodes"].as_array().unwrap());
        assert_eq!(flows, input.document["flows"].as_array().unwrap());
        let expected = &case["expected"];
        near(scene.view_box.width, &expected["viewBox"][0], &context);
        near(scene.view_box.height, &expected["viewBox"][1], &context);
        assert_eq!(
            scene.reader_fit,
            expected["readerFit"].as_str().unwrap(),
            "{context}"
        );
        assert_eq!(
            scene.nodes.len(),
            expected["nodes"].as_array().unwrap().len(),
            "{context}"
        );
        for (index, (node, oracle)) in scene
            .nodes
            .iter()
            .zip(expected["nodes"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(
                node.authored.id,
                oracle["id"].as_str().unwrap(),
                "{context}"
            );
            assert_eq!(node.authored.facts, input.document["nodes"][index]);
            assert_eq!(node.authored.source_path, format!("/nodes/{index}"));
            assert!(!node.authored.geometry_provenance.is_empty());
            bounds(
                node.bounds,
                &oracle["bounds"],
                &format!("{context}/nodes/{index}"),
            );
            texts(&node.text, oracle["texts"].as_array().unwrap(), &context);
            if oracle["sigil"].is_null() {
                assert!(node.sigil.is_none());
            } else {
                let sigil = node.sigil.as_ref().unwrap();
                assert_eq!(
                    sigil.kind,
                    oracle["sigil"]["kind"].as_str().unwrap(),
                    "{context}"
                );
                near(sigil.position.x, &oracle["sigil"]["x"], &context);
                // A frozen sigil that touches its label moves into the card's
                // top rail and shrinks to clear it (recorded P2 deviation).
                let frozen = Rect {
                    x: oracle["sigil"]["x"].as_f64().unwrap(),
                    y: oracle["sigil"]["y"].as_f64().unwrap(),
                    width: oracle["sigil"]["size"].as_f64().unwrap(),
                    height: oracle["sigil"]["size"].as_f64().unwrap(),
                };
                let touches = node
                    .text
                    .iter()
                    .filter(|text| text.source_path.ends_with("/label"))
                    .any(|text| {
                        let label = text_glyph_box(
                            &text.content,
                            text.position.x,
                            text.position.y,
                            text.font_size,
                            &text.anchor,
                            input.input_contract,
                        );
                        rects_overlap(&frozen, &label, 1.0)
                    });
                if touches {
                    assert_eq!(sigil.position.y, node.bounds.y + 3.0, "{context}");
                    assert!(sigil.size >= 7.0 && sigil.size < frozen.width, "{context}");
                } else {
                    near(sigil.position.y, &oracle["sigil"]["y"], &context);
                    near(sigil.size, &oracle["sigil"]["size"], &context);
                }
            }
        }
        assert_eq!(
            scene.relations.len(),
            expected["flows"].as_array().unwrap().len(),
            "{context}"
        );
        for (index, (flow, oracle)) in scene
            .relations
            .iter()
            .zip(expected["flows"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(flow.authored.facts, input.document["flows"][index]);
            assert_eq!(flow.authored.source_path, format!("/flows/{index}"));
            assert!(!flow.authored.geometry_provenance.is_empty());
            assert_eq!(
                flow.from,
                input.document["flows"][index]["from"].as_str().unwrap()
            );
            assert_eq!(
                flow.to,
                input.document["flows"][index]["to"].as_str().unwrap()
            );
            assert_eq!(
                flow.label,
                input.document["flows"][index]["label"].as_str().unwrap()
            );
            assert_eq!(
                flow.note.as_deref(),
                input.document["flows"][index]["classification"].as_str()
            );
            assert_eq!(
                flow.crossover_halo,
                oracle["halo"].as_bool().unwrap(),
                "{context}"
            );
            if inferred_channel(&input.document["flows"][index]) {
                channel_attachment(&scene, index, oracle, &format!("{context}/flows/{index}"));
                repaired_channels += 1;
            } else {
                assert_eq!(
                    flow.points.len(),
                    oracle["points"].as_array().unwrap().len(),
                    "{context}/flows/{index}: preserve all actual route points"
                );
                for (actual, point) in flow.points.iter().zip(oracle["points"].as_array().unwrap())
                {
                    near(actual.x, &point[0], &context);
                    near(actual.y, &point[1], &context);
                }
            }
            bounds(
                flow.label_plate.unwrap(),
                &oracle["plate"],
                &format!("{context}/flows/{index}/label"),
            );
            texts(&flow.text, oracle["texts"].as_array().unwrap(), &context);
        }
        assert_eq!(
            scene.containers.len(),
            expected["frames"].as_array().unwrap().len(),
            "{context}"
        );
        let mut stage_texts = Vec::new();
        for (index, (stage, oracle)) in scene
            .containers
            .iter()
            .zip(expected["frames"].as_array().unwrap())
            .enumerate()
        {
            bounds(
                stage.bounds,
                &oracle["bounds"],
                &format!("{context}/stages/{index}"),
            );
            assert_eq!(stage.authored.facts, input.document["stages"][index]);
            assert_eq!(
                stage.members,
                input.document["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|node| node["stage"].as_f64() == Some(index as f64))
                    .map(|node| node["id"].as_str().unwrap().to_owned())
                    .collect::<Vec<_>>()
            );
            stage_texts.extend(stage.text.iter().cloned());
        }
        texts(
            &stage_texts,
            expected["stageTexts"].as_array().unwrap(),
            &format!("{context}/stage-headers"),
        );
        let title = scene
            .legends
            .iter()
            .filter(|card| card.authored.kind == "legend-title")
            .flat_map(|card| card.text.clone())
            .collect::<Vec<_>>();
        texts(
            &title,
            expected["legendTitle"].as_array().unwrap(),
            &format!("{context}/legend-title"),
        );
        let entries = scene
            .legends
            .iter()
            .filter(|card| card.authored.kind != "legend-title")
            .collect::<Vec<_>>();
        assert_eq!(
            entries.len(),
            expected["legend"].as_array().unwrap().len(),
            "{context}/legend"
        );
        for (entry, oracle) in entries.iter().zip(expected["legend"].as_array().unwrap()) {
            assert_eq!(
                entry.authored.kind,
                oracle["kind"].as_str().unwrap(),
                "{context}/legend"
            );
            near(entry.bounds.x, &oracle["x"], &context);
            near(entry.bounds.width, &oracle["width"], &context);
            near(entry.bounds.y + 10.0, &oracle["baseline"], &context);
            texts(
                &entry.text,
                oracle["texts"].as_array().unwrap(),
                &format!("{context}/legend"),
            );
        }
    }
    assert_eq!(accepted, 62);
    assert_eq!(rejected, 22);
    // The event-stream poison-event flow in both examples and contracts.
    assert_eq!(repaired_channels, 4);
}

#[test]
fn dataflow_schema_integers_accept_float_lexemes_and_keep_explicit_geometry() {
    let corpus = corpus();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case["name"] == "row-dogleg-label-segment-zero" && case["contract"] == "legacy-2.17"
        })
        .unwrap();
    let mut document = case["document"].clone();
    document["schema_version"] = json!(1.0);
    document["nodes"][0]["stage"] = json!(0.0);
    document["nodes"][0]["row"] = json!(0.0);
    document["nodes"][1]["stage"] = json!(1.0);
    document["nodes"][1]["row"] = json!(1.0);
    document["flows"][0]["labelSegment"] = json!(0.0);
    let options = CompileOptions {
        input_contract: Some(InputContract::Legacy217),
        ..Default::default()
    };
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Dataflow,
        &options,
    )
    .unwrap();
    let scene = dataflow::compile(&input, &options).unwrap();
    bounds(
        scene.nodes[0].bounds,
        &case["expected"]["nodes"][0]["bounds"],
        "float integer lexemes",
    );
    assert_eq!(
        scene.relations[0].text[0].position,
        phoenix_diagram::scene::Point {
            x: 181.75,
            y: 147.0
        }
    );
}
