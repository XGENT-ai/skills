//! Actual SVG projections of the frozen renderers, beyond scene DTO parity.
use phoenix_diagram::{
    input::{parse_input, CompileOptions, DiagramType, InputContract, Quality},
    layout,
    render::{render, EmbeddedAssets},
};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;

fn html(document: &Value, kind: DiagramType, contract: &str, quality: &str) -> Html {
    let options = CompileOptions {
        input_contract: Some(if contract == "legacy-2.17" {
            InputContract::Legacy217
        } else {
            InputContract::PhoenixV1
        }),
        quality: Some(if quality == "showcase" {
            Quality::Showcase
        } else {
            Quality::Standard
        }),
        ..Default::default()
    };
    let input = parse_input(&serde_json::to_vec(document).unwrap(), kind, &options).unwrap();
    let scene = layout::compile(&input, &options).unwrap();
    Html::parse_document(
        std::str::from_utf8(&render(&scene, &EmbeddedAssets::builtin()).unwrap()).unwrap(),
    )
}
fn selector(value: &str) -> Selector {
    Selector::parse(value).unwrap()
}
fn numeric(element: ElementRef<'_>, attr: &str) -> f64 {
    element.value().attr(attr).unwrap().parse().unwrap()
}
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.00001
}
fn same_box(element: ElementRef<'_>, expected: &Value) -> bool {
    ["x", "y", "width", "height"]
        .into_iter()
        .all(|key| near(numeric(element, key), expected[key].as_f64().unwrap()))
}
fn same_path(actual: &str, expected: &str) -> bool {
    let actual = phoenix_diagram::check::path::analyze_svg_path(actual).unwrap();
    let expected = phoenix_diagram::check::path::analyze_svg_path(expected).unwrap();
    actual.subpaths.len() == expected.subpaths.len()
        && actual
            .subpaths
            .iter()
            .zip(&expected.subpaths)
            .all(|(a, b)| {
                a.closed == b.closed
                    && a.points.len() == b.points.len()
                    && a.points
                        .iter()
                        .zip(&b.points)
                        .all(|(a, b)| near(a.x, b.x) && near(a.y, b.y))
            })
}

#[test]
fn lifecycle_legends_preserve_initial_final_and_state_type_symbols() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.lifecycle.json"
    ))
    .unwrap();
    let actual = html(&document, DiagramType::Lifecycle, "phoenix-v1", "showcase");
    let entry = |kind: &str| {
        actual
            .select(&selector(&format!("[data-legend-semantic-kind='{kind}']")))
            .next()
            .unwrap_or_else(|| panic!("Missing semantic lifecycle legend: {kind}"))
    };
    assert_eq!(entry("start").select(&selector("circle")).count(), 1);
    assert!(entry("start").select(&selector("path")).count() >= 2);
    assert_eq!(
        entry("success").select(&selector("rect.c-backend")).count(),
        1
    );
    assert_eq!(
        entry("final").select(&selector("rect.c-external")).count(),
        2
    );

    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/lifecycle/oracle.json")).unwrap();
    let document = &oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["ok"] == true && case["contract"] == "legacy-2.17")
        .unwrap()["document"];
    let actual = html(
        document,
        DiagramType::Lifecycle,
        "legacy-2.17",
        document["meta"]["quality_profile"]
            .as_str()
            .unwrap_or("standard"),
    );
    for (kind, class) in [("active", "c-backend"), ("success", "c-database")] {
        let entries: Vec<_> = actual
            .select(&selector(&format!("[data-legend-semantic-kind='{kind}']")))
            .collect();
        assert_eq!(entries.len(), 1, "Missing legacy lifecycle legend: {kind}");
        assert_eq!(
            entries[0]
                .select(&selector(&format!("rect.{class}")))
                .count(),
            1
        );
    }
    assert!(actual.select(&selector("[data-legend-semantic-kind^='legend-']:not([data-legend-semantic-kind='legend-title'])")).next().is_none());
}

#[test]
fn legacy_success_legend_does_not_claim_a_state_with_outgoing_transitions_is_terminal() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/lifecycle/oracle.json")).unwrap();
    let original = &oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "deployment-release")
        .unwrap()["document"];
    assert!(original["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["from"] == "live" && edge["to"] == "paused"));
    for (locale, expected) in [("en", "success"), ("zh-CN", "成功")] {
        let mut document = original.clone();
        document["meta"]["locale"] = locale.into();
        let actual = html(&document, DiagramType::Lifecycle, "legacy-2.17", "showcase");
        let entry = actual
            .select(&selector("[data-legend-semantic-kind='success']"))
            .next()
            .unwrap();
        let labels: Vec<_> = entry
            .select(&selector("text"))
            .map(|text| text.text().collect::<String>())
            .collect();
        assert!(
            labels.iter().any(|label| label == expected),
            "{locale}: {labels:?}"
        );
        document["meta"]["legend"]["entries"]["success"]["label"] =
            "Authored success outcome".into();
        let explicit = html(&document, DiagramType::Lifecycle, "legacy-2.17", "showcase");
        assert!(explicit
            .select(&selector("[data-legend-semantic-kind='success'] text"))
            .any(|text| text.text().collect::<String>() == "Authored success outcome"));
    }
}

#[test]
fn workflow_default_backend_legend_does_not_invent_an_agent_in_any_bundled_locale() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.workflow.json"
    ))
    .unwrap();
    for contract in ["legacy-2.17", "phoenix-v1"] {
        for (locale, expected) in [
            (None, "Processing logic"),
            (Some("en"), "Processing logic"),
            (Some("zh-CN"), "处理逻辑"),
            (Some("zh-TW"), "處理邏輯"),
            (Some("es"), "Lógica de procesamiento"),
            (Some("ko"), "처리 로직"),
        ] {
            // The frozen legacy input contract only accepts these two locales.
            if contract == "legacy-2.17" && !matches!(locale, None | Some("en" | "zh-CN")) {
                continue;
            }
            let mut document = original.clone();
            document["meta"]["input_contract"] = contract.into();
            document["meta"].as_object_mut().unwrap().remove("locale");
            if let Some(locale) = locale {
                document["meta"]["locale"] = locale.into();
            }
            let actual = html(&document, DiagramType::Workflow, contract, "showcase");
            let labels: Vec<_> = actual
                .select(&selector("[data-legend-semantic-kind='backend'] text"))
                .map(|text| text.text().collect::<String>())
                .collect();
            assert!(
                labels.iter().any(|label| label == expected),
                "{contract}/{locale:?}: {labels:?}"
            );
        }
        let mut document = original.clone();
        document["meta"]["input_contract"] = contract.into();
        if contract == "phoenix-v1" {
            document["meta"]["translations"]["legend.workflow.backend"] =
                "Authored processing".into();
            let actual = html(&document, DiagramType::Workflow, contract, "showcase");
            assert!(actual
                .select(&selector("[data-legend-semantic-kind='backend'] text"))
                .any(|text| text.text().collect::<String>() == "Authored processing"));
        }
        document["meta"]["legend"]["entries"]["backend"]["label"] = "Authored performer".into();
        let actual = html(&document, DiagramType::Workflow, contract, "showcase");
        assert!(actual
            .select(&selector("[data-legend-semantic-kind='backend'] text"))
            .any(|text| text.text().collect::<String>() == "Authored performer"));
    }
}

#[test]
fn dataflow_arrow_legends_remain_noninteractive_even_when_node_kinds_match() {
    let document: Value = serde_json::from_str(include_str!(
        "../upstream/legacy-2.17/examples/product-analytics.dataflow.json"
    ))
    .unwrap();
    let actual = html(&document, DiagramType::Dataflow, "legacy-2.17", "showcase");
    assert_eq!(
        actual
            .select(&selector("[data-legend-semantic-kind='security']"))
            .count(),
        1
    );
    for entry in actual.select(&selector("[data-graph-role='legend']")) {
        let kind = entry.value().attr("data-legend-semantic-kind").unwrap();
        if kind == "database" {
            assert_eq!(entry.value().attr("data-legend-kind"), Some("database"));
        } else {
            assert!(
                entry.value().attr("data-legend-kind").is_none(),
                "Arrow/title legend {kind} must not acquire a node-count badge"
            );
        }
    }
}

#[test]
fn lifecycle_rendering_preserves_frozen_paths_primary_labels_and_state_markers() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/lifecycle/oracle.json")).unwrap();
    let mut failures = vec![];
    let mut adapted_paths = 0;
    for case in oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["ok"] == true)
    {
        let contract = case["contract"].as_str().unwrap();
        let key = format!("{contract}/{}", case["name"]);
        let actual = html(
            &case["document"],
            DiagramType::Lifecycle,
            contract,
            case["document"]["meta"]["quality_profile"]
                .as_str()
                .unwrap_or("standard"),
        );
        let source = Html::parse_document(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/lifecycle")
                    .join(case["svgFile"].as_str().unwrap()),
            )
            .unwrap(),
        );
        for expected in source.select(&selector("path[data-edge-key]")) {
            let edge_key = expected.value().attr("data-edge-key").unwrap();
            let edge = actual
                .select(&selector("path[data-graph-role='edge']"))
                .find(|e| e.value().attr("data-edge-key") == Some(edge_key))
                .unwrap();
            let source_path = expected.value().attr("d").unwrap();
            // The frozen source emits a 2px straight cap on these two self-loops.
            let adaptation = match (
                contract,
                case["name"].as_str().unwrap(),
                edge_key,
                expected.value().attr("data-edge-id"),
            ) {
                ("phoenix-v1", "nested-arcs-and-self-loop", "4", Some("bb")) => Some((
                    "M 338 94 L 338 76 Q 338 64 350 64 L 352 64 Q 364 64 364 76 L 364 94",
                    "M 338 94 L 338 73 Q 338 64 347 64 L 355 64 Q 364 64 364 73 L 364 94",
                    "338,94;338,64;364,64;364,94",
                )),
                ("phoenix-v1", "lower-parallels-and-loop", "6", Some("xx")) => Some((
                    "M 162.5 248 L 162.5 270 Q 162.5 282 174.5 282 L 176.5 282 Q 188.5 282 188.5 270 L 188.5 248",
                    "M 162.5 248 L 162.5 273 Q 162.5 282 171.5 282 L 179.5 282 Q 188.5 282 188.5 273 L 188.5 248",
                    "162.5,248;162.5,282;188.5,282;188.5,248",
                )),
                _ => None,
            };
            let expected_path = if let Some((frozen, corrected, points)) = adaptation {
                adapted_paths += 1;
                assert_eq!(source_path, frozen, "{key}: frozen self-loop {edge_key}");
                for element in [expected, edge] {
                    assert_eq!(
                        element.value().attr("data-composition-points"),
                        Some(points),
                        "{key}: unchanged self-loop points {edge_key}"
                    );
                }
                let analysis =
                    phoenix_diagram::check::path::analyze_svg_path(edge.value().attr("d").unwrap())
                        .unwrap();
                let runs: Vec<_> = analysis
                    .subpaths
                    .iter()
                    .flat_map(|part| &part.straight_segments)
                    .collect();
                assert_eq!(runs.len(), 3, "{key}: retained self-loop runs {edge_key}");
                assert!(
                    runs.iter().all(|run| {
                        (run.end.x - run.start.x).hypot(run.end.y - run.start.y) >= 8.0
                    }),
                    "{key}: self-loop run clearance {edge_key}"
                );
                corrected
            } else {
                source_path
            };
            if !same_path(edge.value().attr("d").unwrap(), expected_path)
                || [
                    "class",
                    "stroke-width",
                    "marker-end",
                    "data-composition-junction",
                ]
                .into_iter()
                .any(|a| edge.value().attr(a) != expected.value().attr(a))
            {
                failures.push(format!("{key}: route/style/junction {edge_key}"));
            }
        }
        for node in source.select(&selector("g[data-node-id]")) {
            let id = node.value().attr("data-node-id").unwrap();
            let rendered = actual
                .select(&selector("g[data-node-id]"))
                .find(|n| n.value().attr("data-node-id") == Some(id))
                .unwrap();
            let expected = node
                .select(&selector("text[data-node-label]"))
                .next()
                .unwrap();
            if !rendered
                .select(&selector("text[data-node-label]"))
                .any(|t| {
                    t.text().collect::<String>() == expected.text().collect::<String>()
                        && t.value().attr("data-detail").is_none()
                        && t.value().attr("class") == Some("t-primary")
                })
            {
                failures.push(format!("{key}: primary label {id}"));
            }
            let source_rect = node.select(&selector("rect")).nth(1).unwrap();
            let body = rendered
                .select(&selector("rect[data-graph-role='node-body']"))
                .next()
                .unwrap();
            if body.value().attr("rx") != source_rect.value().attr("rx")
                || body.value().attr("class") != source_rect.value().attr("class")
            {
                failures.push(format!("{key}: node corner/tone {id}"));
            }
            for marker in [
                "[data-lifecycle-initial-marker]",
                "rect[style='fill: none']",
            ] {
                if rendered.select(&selector(marker)).count()
                    != node.select(&selector(marker)).count()
                {
                    failures.push(format!("{key}: state marker {id} {marker}"));
                }
            }
        }
        for marker in [
            "circle[data-lifecycle-junction]",
            "rect[data-lifecycle-frame]",
        ] {
            if actual.select(&selector(marker)).count() != source.select(&selector(marker)).count()
            {
                failures.push(format!("{key}: {marker}"));
            }
        }
        if actual.select(&selector("[data-composition-frame-kind='lifecycle-band'][data-graph-role='structural-frame']")).next().is_some() {failures.push(format!("{key}: band became a structural frame"));}
    }
    assert_eq!(adapted_paths, 2, "Only the two frozen 2px self-loops adapt");
    assert!(
        failures.is_empty(),
        "{} SVG failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn sequence_lifelines_activations_note_masks_and_legend_arrows_reach_svg() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/sequence/oracle.json")).unwrap();
    let mut failures = vec![];
    let mut backdrops = 0;
    for case in oracle["cases"].as_array().unwrap() {
        for contract in ["legacy-2.17", "phoenix-v1"] {
            let source = &case["oracles"][contract];
            if source["success"] != true {
                continue;
            }
            let key = format!("{contract}/{}", case["name"]);
            let actual = html(
                &case["input"],
                DiagramType::Sequence,
                contract,
                case["effectiveQuality"].as_str().unwrap_or("standard"),
            );
            for points in source["lifelines"].as_array().unwrap() {
                let found = actual
                    .select(&selector("line[data-graph-role='reading-guide']"))
                    .any(|line| {
                        ["x1", "y1", "x2", "y2"]
                            .into_iter()
                            .zip([
                                points[0][0].as_f64().unwrap(),
                                points[0][1].as_f64().unwrap(),
                                points[1][0].as_f64().unwrap(),
                                points[1][1].as_f64().unwrap(),
                            ])
                            .all(|(k, v)| near(numeric(line, k), v))
                            && line.value().attr("stroke-width") == Some("0.8")
                            && line.value().attr("stroke-dasharray") == Some("3,7")
                    });
                if !found {
                    failures.push(format!("{key}: lifeline {points}"));
                }
            }
            for expected in source["activations"].as_array().unwrap() {
                if !actual
                    .select(&selector("rect[data-graph-role='activation']"))
                    .any(|r| {
                        same_box(r, expected)
                            && r.value().attr("rx") == Some("3")
                            && r.value().attr("stroke-width") == Some("1")
                    })
                {
                    failures.push(format!("{key}: activation {expected}"));
                }
            }
            for message in source["messages"].as_array().unwrap() {
                if !message["backdrop"].is_null() {
                    backdrops += 1;
                    if !actual
                        .select(&selector("rect[data-graph-role='message-note-background']"))
                        .any(|r| same_box(r, &message["backdrop"]))
                    {
                        failures.push(format!("{key}: note background"));
                    }
                }
            }
            for legend in source["legends"].as_array().unwrap() {
                let kind = legend["kind"].as_str().unwrap();
                let entry = actual
                    .select(&selector("g[data-graph-role='legend']"))
                    .find(|e| e.value().attr("data-legend-semantic-kind") == Some(kind))
                    .unwrap();
                if entry.select(&selector("path[marker-end]")).count() != 1 {
                    failures.push(format!("{key}: arrow swatch {kind}"));
                }
            }
        }
    }
    assert!(
        backdrops > 0,
        "source fixture must exercise real note backdrops"
    );
    assert!(
        failures.is_empty(),
        "{} SVG failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn dataflow_renders_stage_frames_nodes_and_flow_legend_shapes() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/dataflow/oracle.json")).unwrap();
    let mut failures = vec![];
    for case in oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["ok"] == true)
    {
        let key = format!("{}/{}", case["contract"], case["name"]);
        let actual = html(
            &case["document"],
            DiagramType::Dataflow,
            case["contract"].as_str().unwrap(),
            case["effectiveQuality"].as_str().unwrap_or("standard"),
        );
        if actual
            .select(&selector("rect[data-graph-role='node-body']"))
            .any(|r| r.value().attr("rx") != Some("6"))
        {
            failures.push(format!("{key}: node corner"));
        }
        if actual
            .select(&selector("rect[data-composition-frame-kind='stage']"))
            .any(|r| r.value().attr("rx") != Some("10"))
        {
            failures.push(format!("{key}: stage corner"));
        }
        for legend in case["expected"]["legend"].as_array().unwrap() {
            let kind = legend["kind"].as_str().unwrap();
            if kind == "database" {
                continue;
            }
            let entry = actual
                .select(&selector("g[data-graph-role='legend']"))
                .find(|e| e.value().attr("data-legend-semantic-kind") == Some(kind))
                .unwrap();
            if entry.select(&selector("path[marker-end]")).count() != 1 {
                failures.push(format!("{key}: arrow swatch {kind}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} SVG failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(16)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
