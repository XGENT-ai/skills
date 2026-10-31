#[path = "support/scene_svg.rs"]
mod svg_oracle;

use phoenix_diagram::check::check_html;
use phoenix_diagram::input::{DiagramType, Quality};
use phoenix_diagram::scene::{CompiledDiagram, DiagramFacts};
use scraper::Html;
use serde_json::{json, Value};
use svg_oracle::*;

fn cases() -> Vec<Value> {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/waterfall/oracle.json")).unwrap();
    oracle["cases"].as_array().unwrap().clone()
}

fn containers<'a>(
    scene: &'a CompiledDiagram,
    kind: &'a str,
) -> impl Iterator<Item = &'a phoenix_diagram::scene::Container> {
    scene
        .containers
        .iter()
        .filter(move |c| c.authored.kind == kind)
}

fn scene_matches_layout_report(scene: &CompiledDiagram, report: &Value, name: &str) {
    near(
        scene.view_box.width,
        report["viewBox"][0].as_f64().unwrap(),
        SCENE,
        name,
    );
    near(
        scene.view_box.height,
        report["viewBox"][1].as_f64().unwrap(),
        SCENE,
        name,
    );
    let DiagramFacts::Waterfall { unit, .. } = &scene.facts else {
        panic!("{name}")
    };
    assert_eq!(unit, report["unit"].as_str().unwrap(), "{name}");
    let axis = containers(scene, "waterfall-axis")
        .next()
        .unwrap()
        .rule
        .unwrap();
    near(
        axis[0].x,
        report["axis"]["x0"].as_f64().unwrap(),
        SCENE,
        name,
    );
    near(
        axis[1].x,
        report["axis"]["x1"].as_f64().unwrap(),
        SCENE,
        name,
    );
    let total = containers(scene, "waterfall-total").next().unwrap();
    near(
        total.authored.facts["wall"].as_f64().unwrap(),
        report["wall"].as_f64().unwrap(),
        SCENE,
        name,
    );
    let ticks: Vec<_> = containers(scene, "waterfall-tick").collect();
    let expected_ticks = report["ticks"].as_array().unwrap();
    assert_eq!(ticks.len(), expected_ticks.len(), "{name} ticks");
    for (tick, expected) in ticks.iter().zip(expected_ticks) {
        near(
            tick.authored.facts["t"].as_f64().unwrap(),
            expected["t"].as_f64().unwrap(),
            SCENE,
            &format!("{name} tick"),
        );
        near(
            tick.rule.unwrap()[0].x,
            expected["x"].as_f64().unwrap(),
            SCENE,
            &format!("{name} tick x"),
        );
    }
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(scene.nodes.len(), rows.len(), "{name} rows");
    for (node, expected) in scene.nodes.iter().zip(rows) {
        let context = format!("{name}/{}", node.authored.id);
        assert_eq!(node.authored.id, expected["id"].as_str().unwrap());
        let row = containers(scene, "waterfall-row")
            .find(|row| row.authored.id == node.authored.id)
            .unwrap();
        let [_, end] = row.rule.unwrap();
        near(
            node.bounds.x,
            expected["x0"].as_f64().unwrap(),
            SCENE,
            &format!("{context}.x0"),
        );
        near(
            end.x,
            expected["x1"].as_f64().unwrap(),
            SCENE,
            &format!("{context}.x1"),
        );
        near(
            row.bounds.y - 1.0,
            expected["y"].as_f64().unwrap(),
            SCENE,
            &format!("{context}.y"),
        );
        assert_eq!(row.authored.facts["depth"], expected["depth"], "{context}");
        assert_eq!(row.authored.facts["open"], expected["open"], "{context}");
        near(
            row.authored.facts["end"].as_f64().unwrap(),
            expected["end"].as_f64().unwrap(),
            SCENE,
            &format!("{context}.end"),
        );
        let label = node
            .text
            .iter()
            .find(|text| text.source_path.ends_with("/duration"))
            .unwrap();
        assert_eq!(
            label.content,
            expected["label"].as_str().unwrap(),
            "{context}"
        );
    }
}

fn root_attribute(document: &Html, name: &str) -> Option<String> {
    document
        .select(&select("svg"))
        .next()
        .unwrap()
        .value()
        .attr(name)
        .map(str::to_owned)
}

#[test]
fn frozen_waterfall_oracle_geometry_text_and_reader_attributes() {
    let (mut rendered, mut legends, mut guides) = (0, 0, 0);
    for case in cases().iter().filter(|case| case["ok"] == true) {
        let name = case["name"].as_str().unwrap();
        let scene = compile(&case["document"], DiagramType::Waterfall)
            .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        scene_matches_layout_report(&scene, &case["layout"], name);
        let html = html(&scene);
        let actual = Html::parse_fragment(&svg_of(&html));
        let reference = Html::parse_fragment(case["svg"].as_str().unwrap());
        for attribute in [
            "viewBox",
            "data-motion-choreography",
            "data-waterfall-ui",
            "data-waterfall-unit",
            "data-reader-fit",
            "data-reader-min-text",
        ] {
            assert_eq!(
                root_attribute(&actual, attribute),
                root_attribute(&reference, attribute),
                "{name} {attribute}"
            );
        }
        let plain = Rules {
            extra: &[],
            skip: &[],
        };
        for css in [
            "g[data-waterfall-evidence]",
            "text[data-waterfall-total]",
            "text.wf-caps",
            "line.wf-axis",
            "line.wf-grid",
            "text.wf-num:not([data-waterfall-total])",
        ] {
            let count = same_selection(&reference, &actual, css, &plain, name);
            assert!(count > 0, "{name} {css}");
        }
        let count = same_selection(
            &reference,
            &actual,
            "g[data-node-id]",
            &Rules {
                extra: &["data-source-path", "data-graph-role"],
                skip: &[],
            },
            name,
        );
        assert_eq!(count, case["document"]["spans"].as_array().unwrap().len());
        // Ownership guides are drawn as in the reference, without relation identity.
        let guide = Rules {
            extra: &[],
            skip: &["data-edge-from", "data-edge-to", "data-edge-key"],
        };
        guides += same_selection(&reference, &actual, "path.wf-guide", &guide, name);
        assert_eq!(
            actual
                .select(&select("[data-edge-from], [marker-end], [marker-start]"))
                .count(),
            0,
            "{name}"
        );
        let legend = Rules {
            extra: &[
                "data-graph-role",
                "data-legend-y",
                "data-legend-height",
                "text-anchor",
                "data-source-path",
            ],
            skip: &[],
        };
        same_selection(
            &reference,
            &actual,
            "g[data-legend-semantic-kind]:not([data-legend-semantic-kind='legend-title'])",
            &legend,
            name,
        );
        legends += usize::from(
            actual
                .select(&select("g[data-legend-semantic-kind='legend-title']"))
                .next()
                .is_some(),
        );
        let check = check_html(html.as_bytes(), scene.quality);
        assert_eq!(check.status, "pass", "{name}: {:?}", check.diagnostics);
        assert!(
            check.diagnostics.is_empty(),
            "{name}: {:?}",
            check.diagnostics
        );
        rendered += 1;
    }
    assert_eq!(rendered, 11);
    assert!(
        legends >= 5 && guides > 20,
        "{legends} legends, {guides} guides"
    );
}

#[test]
fn rejected_waterfalls_keep_the_reference_diagnostic_codes() {
    let mut rejected = 0;
    for case in cases().iter().filter(|case| case["ok"] == false) {
        let name = case["name"].as_str().unwrap();
        let errors = compile(&case["document"], DiagramType::Waterfall).expect_err(name);
        assert_eq!(
            actual_codes(&errors),
            codes(case["diagnostics"].as_array().unwrap()),
            "{name}"
        );
        rejected += 1;
    }
    assert_eq!(rejected, 8);
}

fn example() -> CompiledDiagram {
    let document: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.waterfall.json"
    ))
    .unwrap();
    assert_eq!(document["meta"]["input_contract"], "phoenix-v1");
    assert!(document["meta"]["evidence"].is_string());
    compile(&document, DiagramType::Waterfall).unwrap()
}

#[test]
fn minimal_example_keeps_recorded_timing_and_passes_showcase() {
    let scene = example();
    let DiagramFacts::Waterfall { spans, .. } = &scene.facts else {
        panic!("waterfall facts")
    };
    let open = spans.iter().find(|span| span.id == "receipt").unwrap();
    assert_eq!((open.end, open.status.as_str()), (None, "incomplete"));
    assert!(scene.relations.is_empty());
    let html = html(&scene);
    let check = check_html(html.as_bytes(), Quality::Showcase);
    assert_eq!(check.status, "pass", "{:?}", check.diagnostics);
    let document = Html::parse_document(&html);
    let receipt = document
        .select(&select("g[data-node-id='receipt']"))
        .next()
        .unwrap();
    // An open span states a lower bound to the last recorded instant.
    assert!(receipt
        .text()
        .collect::<String>()
        .contains("incomplete, ≥ 30 ms"));
    assert_eq!(receipt.value().attr("data-waterfall-end"), Some(""));
    let metadata: Value = serde_json::from_str(
        &document
            .select(&select("#phoenix-diagram-metadata"))
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
    )
    .unwrap();
    assert_eq!(metadata["graph"]["relations"], json!([]));
}

#[track_caller]
fn failing_codes(html: &str) -> Vec<String> {
    let check = check_html(html.as_bytes(), Quality::Showcase);
    assert_eq!(check.status, "failed");
    check.diagnostics.into_iter().map(|d| d.code).collect()
}

#[test]
fn artifact_check_rejects_lost_span_text_moved_bars_and_any_relationship() {
    let scene = example();
    let html = html(&scene);
    let missing_name = html.replacen(">Reserve stock</text>", "></text>", 1);
    assert_ne!(missing_name, html);
    assert!(failing_codes(&missing_name).contains(&"artifact/semantic-text".into()));

    let wrong_duration = html.replacen(">180 ms</text>", ">18 ms</text>", 1);
    assert_ne!(wrong_duration, html);
    assert!(failing_codes(&wrong_duration).contains(&"artifact/semantic-text".into()));

    let stock = scene
        .nodes
        .iter()
        .find(|node| node.authored.id == "stock")
        .unwrap();
    let moved = html.replacen(
        &format!(
            "data-graph-role=\"node-body\" x=\"{}\"",
            phoenix_diagram::geometry::format_number(stock.bounds.x)
        ),
        &format!(
            "data-graph-role=\"node-body\" x=\"{}\"",
            phoenix_diagram::geometry::format_number(stock.bounds.x + 30.0)
        ),
        1,
    );
    assert_ne!(moved, html);
    assert!(failing_codes(&moved).contains(&"artifact/semantic-relation".into()));

    let causal = html.replacen(
        "class=\"wf-guide\"",
        "data-edge-from=\"request\" data-edge-to=\"auth\" class=\"wf-guide\"",
        1,
    );
    assert_ne!(causal, html);
    assert!(failing_codes(&causal).contains(&"artifact/semantic-count".into()));
    let arrowhead = html.replacen(
        "class=\"wf-guide\"",
        "marker-end=\"url(#arrowhead)\" class=\"wf-guide\"",
        1,
    );
    assert!(failing_codes(&arrowhead).contains(&"artifact/semantic-count".into()));
}
