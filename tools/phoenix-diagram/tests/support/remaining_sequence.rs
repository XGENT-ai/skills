use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, InputContract, Quality};
use phoenix_diagram::layout as sequence;
use phoenix_diagram::{
    diagnostic::Diagnostics,
    scene::{CompiledDiagram, DiagramFacts, Point, Rect, Text},
};
use serde_json::{json, Value};

fn corpus() -> Value {
    serde_json::from_slice(include_bytes!("../fixtures/sequence/oracle.json")).unwrap()
}
fn compile(document: &Value, contract: InputContract) -> Result<CompiledDiagram, Diagnostics> {
    compile_quality(document, contract, None)
}
fn compile_quality(
    document: &Value,
    contract: InputContract,
    quality: Option<Quality>,
) -> Result<CompiledDiagram, Diagnostics> {
    let options = CompileOptions {
        input_contract: Some(contract),
        quality,
        ..Default::default()
    };
    let input = parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Sequence,
        &options,
    )?;
    sequence::compile(&input, &options)
}
fn case_quality(case: &Value) -> Quality {
    serde_json::from_value(case["effectiveQuality"].clone()).unwrap()
}
fn near(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "{context}: {actual} != {expected}"
    );
}
fn point(actual: Point, expected: &Value, context: &str) {
    near(actual.x, expected[0].as_f64().unwrap(), context);
    near(actual.y, expected[1].as_f64().unwrap(), context);
}
fn rect(actual: Rect, expected: &Value, context: &str) {
    for (a, k) in [
        (actual.x, "x"),
        (actual.y, "y"),
        (actual.width, "width"),
        (actual.height, "height"),
    ] {
        near(a, expected[k].as_f64().unwrap(), &format!("{context}/{k}"));
    }
}
fn text(actual: &Text, expected: &Value, context: &str) {
    assert_eq!(
        actual.content,
        expected["content"].as_str().unwrap(),
        "{context}"
    );
    point(actual.position, &expected["position"], context);
    near(
        actual.font_size,
        expected["fontSize"].as_f64().unwrap(),
        context,
    );
    assert_eq!(
        actual.anchor,
        expected["anchor"].as_str().unwrap(),
        "{context}"
    );
}

#[test]
fn sequence_frozen_svg_geometry_and_all_text_match_both_contracts() {
    let corpus = corpus();
    let mut checked = 0;
    for case in corpus["cases"].as_array().unwrap() {
        for (name, contract) in [
            ("legacy-2.17", InputContract::Legacy217),
            ("phoenix-v1", InputContract::PhoenixV1),
        ] {
            let oracle = &case["oracles"][name];
            if oracle["success"] != true {
                continue;
            }
            let context = format!("{name}/{}", case["name"]);
            let actual = compile_quality(&case["input"], contract, Some(case_quality(case)))
                .unwrap_or_else(|errors| panic!("{context}: {errors:?}"));
            checked += 1;
            assert_eq!(
                actual.reader_fit,
                oracle["readerFit"].as_str().unwrap_or("legacy"),
                "{context}"
            );
            for (a, index) in [
                (actual.view_box.x, 0),
                (actual.view_box.y, 1),
                (actual.view_box.width, 2),
                (actual.view_box.height, 3),
            ] {
                near(a, oracle["viewBox"][index].as_f64().unwrap(), &context);
            }
            assert_eq!(
                actual.nodes.len(),
                oracle["participants"].as_array().unwrap().len(),
                "{context}"
            );
            for (node, expected) in actual
                .nodes
                .iter()
                .zip(oracle["participants"].as_array().unwrap())
            {
                assert_eq!(node.authored.id, expected["id"].as_str().unwrap());
                rect(node.bounds, &expected["bounds"], &context);
                assert_eq!(
                    node.text.len(),
                    expected["texts"].as_array().unwrap().len(),
                    "{context}"
                );
                for (actual, expected) in
                    node.text.iter().zip(expected["texts"].as_array().unwrap())
                {
                    text(actual, expected, &context);
                }
            }
            assert_eq!(
                actual.relations.len(),
                oracle["messages"].as_array().unwrap().len(),
                "{context}"
            );
            for (message, expected) in actual
                .relations
                .iter()
                .zip(oracle["messages"].as_array().unwrap())
            {
                assert_eq!(
                    message.points.len(),
                    expected["points"].as_array().unwrap().len(),
                    "{context}"
                );
                for (actual, expected) in message
                    .points
                    .iter()
                    .zip(expected["points"].as_array().unwrap())
                {
                    point(*actual, expected, &context);
                }
                rect(
                    message.label_plate.unwrap(),
                    &expected["labelPlate"],
                    &context,
                );
                let expected_text: Vec<_> = expected["texts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .chain(expected["noteTexts"].as_array().unwrap())
                    .collect();
                assert_eq!(message.text.len(), expected_text.len(), "{context}");
                for (actual, expected) in message.text.iter().zip(expected_text) {
                    text(actual, expected, &context);
                }
                match (message.note_backdrop, &expected["backdrop"]) {
                    (None, Value::Null) => {}
                    (Some(actual), expected) => rect(actual, expected, &context),
                    _ => panic!("{context}: note backdrop changed"),
                }
            }
            let lifelines: Vec<_> = actual
                .containers
                .iter()
                .filter(|item| item.authored.kind == "lifeline")
                .collect();
            assert_eq!(
                lifelines.len(),
                oracle["lifelines"].as_array().unwrap().len(),
                "{context}"
            );
            for (line, expected) in lifelines
                .iter()
                .zip(oracle["lifelines"].as_array().unwrap())
            {
                let points = line.rule.unwrap();
                point(points[0], &expected[0], &context);
                point(points[1], &expected[1], &context);
            }
            let activations: Vec<_> = actual
                .containers
                .iter()
                .filter(|item| item.authored.kind == "activation")
                .collect();
            assert_eq!(
                activations.len(),
                oracle["activations"].as_array().unwrap().len(),
                "{context}"
            );
            for (actual, expected) in activations
                .iter()
                .zip(oracle["activations"].as_array().unwrap())
            {
                rect(actual.bounds, expected, &context);
            }
            let segments: Vec<_> = actual
                .containers
                .iter()
                .filter(|item| item.authored.kind == "segment")
                .collect();
            assert_eq!(
                segments.len(),
                oracle["segments"].as_array().unwrap().len(),
                "{context}"
            );
            for (actual, expected) in segments.iter().zip(oracle["segments"].as_array().unwrap()) {
                rect(actual.bounds, &expected["bounds"], &context);
                rect(
                    actual.label_plate.unwrap(),
                    &expected["labelPlate"],
                    &context,
                );
                assert_eq!(
                    actual.text.len(),
                    expected["texts"].as_array().unwrap().len()
                );
                for (actual, expected) in actual
                    .text
                    .iter()
                    .zip(expected["texts"].as_array().unwrap())
                {
                    text(actual, expected, &context);
                }
            }
            let entries: Vec<_> = actual
                .legends
                .iter()
                .filter(|item| item.authored.kind != "legend-title")
                .collect();
            assert_eq!(
                entries.len(),
                oracle["legends"].as_array().unwrap().len(),
                "{context}"
            );
            for (actual, expected) in entries.iter().zip(oracle["legends"].as_array().unwrap()) {
                assert_eq!(actual.authored.kind, expected["kind"].as_str().unwrap());
                near(actual.bounds.x, expected["x"].as_f64().unwrap(), &context);
                near(
                    actual.bounds.width,
                    expected["width"].as_f64().unwrap(),
                    &context,
                );
                for (actual, expected) in actual
                    .text
                    .iter()
                    .zip(expected["texts"].as_array().unwrap())
                {
                    text(actual, expected, &context);
                }
            }
            if let Some(expected) = oracle.get("legendTitle") {
                let actual = actual
                    .legends
                    .iter()
                    .find(|item| item.authored.kind == "legend-title")
                    .unwrap();
                text(&actual.text[0], expected, &context);
            }
        }
    }
    assert_eq!(checked, 131);
}

#[test]
fn sequence_effective_quality_changes_phoenix_measurement_without_rewriting_input() {
    let corpus = corpus();
    let authored = &corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "quality-default")
        .unwrap()["input"];
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let default = compile(authored, contract).unwrap();
        let standard = compile_quality(authored, contract, Some(Quality::Standard)).unwrap();
        let explicit = compile_quality(authored, contract, Some(Quality::Showcase)).unwrap();
        let mut persisted = authored.clone();
        persisted["meta"]["quality_profile"] = json!("showcase");
        let saved = compile(&persisted, contract).unwrap();
        assert_eq!(default.meta, authored["meta"]);
        assert_eq!(standard.meta, authored["meta"]);
        assert_eq!(explicit.meta, authored["meta"]);
        assert_eq!(default.nodes, explicit.nodes);
        assert_eq!(default.relations, explicit.relations);
        assert_eq!(default.relations, saved.relations);
        assert_eq!(default.view_box, saved.view_box);
        assert_eq!(default.relations[0].points, standard.relations[0].points);
        if contract == InputContract::PhoenixV1 {
            assert_eq!(default.relations[0].text[0].font_size, 11.0);
            assert_eq!(default.relations[0].label_plate.unwrap().height, 18.0);
            assert_eq!(standard.relations[0].text[0].font_size, 9.0);
            assert_eq!(standard.relations[0].label_plate.unwrap().height, 16.0);
            assert!(
                default.relations[0].label_plate.unwrap().width
                    > standard.relations[0].label_plate.unwrap().width
            );
        } else {
            assert_eq!(default.nodes, standard.nodes);
            assert_eq!(default.relations, standard.relations);
            assert_eq!(default.nodes[0].bounds.height, 54.0);
            assert_eq!(default.relations[0].text[0].font_size, 9.0);
            assert_eq!(default.relations[0].label_plate.unwrap().height, 16.0);
        }
    }
}

#[test]
fn sequence_all_source_rejections_remain_rejections() {
    for case in corpus()["cases"].as_array().unwrap() {
        for (name, contract) in [
            ("legacy-2.17", InputContract::Legacy217),
            ("phoenix-v1", InputContract::PhoenixV1),
        ] {
            if case["oracles"][name]["success"] == false {
                assert!(
                    compile_quality(&case["input"], contract, Some(case_quality(case))).is_err(),
                    "{name}/{} must preserve the source rejection",
                    case["name"]
                );
            }
        }
    }
}

#[test]
fn sequence_legend_translation_overrides_follow_source_locale_and_placeholder_rules() {
    let corpus = corpus();
    let authored = &corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "quality-default")
        .unwrap()["input"];
    for (locale, translations, title, entry) in [
        (
            None,
            json!({"legend.title":"Override","legend.sequence.default":"Wire call"}),
            "Legend",
            "message",
        ),
        (
            Some("en"),
            json!({"legend.title":"Override","legend.sequence.default":"Wire call"}),
            "Override",
            "Wire call",
        ),
        (
            Some("en"),
            json!({"legend.title":"{1}","legend.sequence.default":"{_}"}),
            "Legend",
            "message",
        ),
    ] {
        let mut document = authored.clone();
        document["meta"]["translations"] = translations;
        if let Some(locale) = locale {
            document["meta"]["locale"] = json!(locale);
        }
        let scene = compile(&document, InputContract::PhoenixV1).unwrap();
        assert_eq!(
            scene
                .legends
                .iter()
                .find(|item| item.authored.kind == "legend-title")
                .unwrap()
                .text[0]
                .content,
            title
        );
        assert_eq!(
            scene
                .legends
                .iter()
                .find(|item| item.authored.kind == "default")
                .unwrap()
                .text[0]
                .content,
            entry
        );
        assert_eq!(scene.meta, document["meta"]);
    }
}

#[test]
fn sequence_source_diagnostic_paths_and_bounded_repairs_are_verified() {
    let mut checked = 0;
    let mut repaired = 0;
    for case in corpus()["cases"].as_array().unwrap() {
        let oracle = &case["oracles"]["phoenix-v1"];
        if oracle["success"] == true {
            continue;
        }
        let errors = compile_quality(
            &case["input"],
            InputContract::PhoenixV1,
            Some(case_quality(case)),
        )
        .unwrap_err();
        for expected in oracle["diagnostics"].as_array().unwrap() {
            let code = expected["code"].as_str().unwrap();
            if !(code.starts_with("sequence/") || code.starts_with("legend/")) {
                continue;
            }
            let error = errors
                .iter()
                .find(|error| {
                    error.code == code
                        && error.subject.path == expected["subject"]["path"].as_str().unwrap()
                })
                .unwrap_or_else(|| {
                    panic!(
                        "{}: missing source diagnostic {expected:?}; got {errors:?}",
                        case["name"]
                    )
                });
            checked += 1;
            if !code.starts_with("sequence/") {
                continue;
            }
            assert_eq!(
                error.supported_fixes,
                serde_json::from_value::<Vec<String>>(expected["supportedFixes"].clone()).unwrap(),
                "{}/{}",
                case["name"],
                code
            );
            for fix in &error.supported_fixes {
                let Some((path, value)) = fix
                    .strip_prefix("set ")
                    .and_then(|fix| fix.split_once(" to "))
                else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<Value>(value) else {
                    continue;
                };
                let mut candidate = case["input"].clone();
                *candidate.pointer_mut(path).unwrap() = value;
                compile_quality(
                    &candidate,
                    InputContract::PhoenixV1,
                    Some(case_quality(case)),
                )
                .unwrap_or_else(|errors| {
                    panic!(
                        "{}/{}: offered repair {fix} does not hold: {errors:?}",
                        case["name"], code
                    )
                });
                repaired += 1;
            }
        }
    }
    assert!(
        checked >= 8,
        "must cover source note, segment, self-message and legend diagnostics"
    );
    assert!(
        repaired >= 3,
        "must recompile real note, segment and canvas repairs"
    );
}

#[test]
fn sequence_omitted_column_fit_keeps_distinct_legacy_geometry() {
    let document = json!({"schema_version":1.0,"diagram_type":"sequence","meta":{"title":"Defaults","output":"diagram.html"},"participants":[{"id":"a","type":"backend","label":"A"},{"id":"b","type":"database","label":"B"},{"id":"c","type":"frontend","label":"C"}],"messages":[{"from":"a","to":"b","y":200.0,"label":"request"}]});
    let old = compile(&document, InputContract::Legacy217).unwrap();
    let new = compile(&document, InputContract::PhoenixV1).unwrap();
    assert_eq!((old.view_box.width, old.view_box.height), (920.0, 760.0));
    assert_eq!(
        (
            old.nodes[0].bounds.x,
            old.nodes[0].bounds.width,
            old.nodes[0].bounds.height
        ),
        (19.0, 86.0, 54.0)
    );
    assert_eq!(old.nodes[0].text[0].position.y, 94.0);
    assert_eq!((new.view_box.width, new.view_box.height), (560.0, 327.0));
    assert_eq!(
        (
            new.nodes[0].bounds.x,
            new.nodes[0].bounds.width,
            new.nodes[0].bounds.height
        ),
        (62.0, 121.0, 60.0)
    );
    assert_eq!(new.nodes[0].text[0].position.y, 108.0);
    assert_eq!(new.relations[0].points[0].y, old.relations[0].points[0].y);
}

#[test]
fn sequence_all_facts_and_authored_ranges_survive_without_reordering() {
    let source = corpus();
    let mut document = source["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "timeline-standard-spread")
        .unwrap()["input"]
        .clone();
    document["cards"] = json!([{"dot":"cyan","title":"Protocol","items":["Keep every note"]}]);
    let bytes = serde_json::to_vec(&document).unwrap();
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let options = CompileOptions {
            input_contract: Some(contract),
            ..Default::default()
        };
        let input = parse_input(&bytes, DiagramType::Sequence, &options).unwrap();
        let scene = sequence::compile(&input, &options).unwrap();
        assert_eq!(input.document, document);
        assert_eq!(input.source_bytes, bytes);
        assert_eq!(scene.meta, document["meta"]);
        assert_eq!(scene.cards[0].authored.facts, document["cards"][0]);
        match &scene.facts {
            DiagramFacts::Sequence {
                participants,
                messages,
                activations,
                segments,
            } => {
                assert_eq!(participants, document["participants"].as_array().unwrap());
                assert_eq!(messages, document["messages"].as_array().unwrap());
                assert_eq!(activations, document["activations"].as_array().unwrap());
                assert_eq!(segments, document["segments"].as_array().unwrap());
            }
            _ => panic!("Sequence facts must keep their typed variant"),
        }
        for (index, message) in scene.relations.iter().enumerate() {
            assert_eq!(message.authored.facts, document["messages"][index]);
            assert_eq!(message.authored.source_path, format!("/messages/{index}"));
            assert_eq!(
                message.from,
                document["messages"][index]["from"].as_str().unwrap()
            );
            assert_eq!(
                message.to,
                document["messages"][index]["to"].as_str().unwrap()
            );
            assert!(!message.authored.geometry_provenance.is_empty());
            assert_eq!(
                message.points[0].y,
                document["messages"][index]["y"].as_f64().unwrap()
            );
            assert_eq!(
                message.note.as_deref(),
                document["messages"][index]["note"].as_str()
            );
        }
    }
}

#[test]
fn sequence_long_notes_keep_graphemes_and_use_the_leftmost_spanned_gap() {
    let source = corpus();
    for case in source["cases"].as_array().unwrap().iter().filter(|case| {
        case["name"].as_str().unwrap().starts_with("note-")
            || case["name"].as_str().unwrap().starts_with("reverse-skip")
    }) {
        if case["oracles"]["phoenix-v1"]["success"] != true {
            continue;
        }
        let scene = compile(&case["input"], InputContract::PhoenixV1).unwrap();
        let message = &scene.relations[0];
        let note = message.note.as_ref().unwrap();
        let lines: Vec<_> = message
            .text
            .iter()
            .filter(|text| text.source_path == "/messages/0/note")
            .collect();
        assert!(!lines.is_empty());
        let non_space = |text: &str| {
            text.chars()
                .filter(|character| !character.is_whitespace() && *character != '\u{feff}')
                .collect::<String>()
        };
        assert_eq!(
            non_space(
                &lines
                    .iter()
                    .map(|text| text.content.as_str())
                    .collect::<String>()
            ),
            non_space(note)
        );
        let first_center = scene.nodes[0].bounds.x + scene.nodes[0].bounds.width / 2.0;
        near(
            lines[0].position.x,
            first_center + 19.0,
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn sequence_unsupported_or_crowded_geometry_has_actionable_paths() {
    let source = corpus();
    for (name, code, path) in [
        (
            "negative-self-message",
            "sequence/self-message-unsupported",
            "/messages/0",
        ),
        ("negative-short-canvas", "input/schema", "/meta/viewBox/1"),
        (
            "negative-note-overlap",
            "sequence/note-overlap",
            "/messages/0/note",
        ),
        (
            "negative-note-canvas",
            "sequence/note-canvas-limit",
            "/messages/0/note",
        ),
        (
            "negative-segment-note-label",
            "sequence/segment-label-overlap",
            "/segments/0/from",
        ),
    ] {
        let case = source["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap();
        let errors = compile(&case["input"], InputContract::PhoenixV1).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == code && error.subject.path == path),
            "{name}: {errors:?}"
        );
    }
}
