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
        serde_json::from_str(include_str!("fixtures/timeline/oracle.json")).unwrap();
    oracle["cases"].as_array().unwrap().clone()
}

/// The document Phoenix receives: the reference input plus any phoenix-v1
/// precision the case declares.
fn phoenix_document(case: &Value) -> Value {
    let mut document = case["document"].clone();
    if let Some(precision) = case["phoenix"]["precision"].as_object() {
        for event in document["events"].as_array_mut().unwrap() {
            if let Some(value) = precision.get(event["id"].as_str().unwrap()) {
                event["precision"] = value.clone();
            }
        }
    }
    document
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
    let DiagramFacts::Timeline {
        timezone, breaks, ..
    } = &scene.facts
    else {
        panic!("{name}")
    };
    assert_eq!(timezone, report["timezone"].as_str().unwrap(), "{name}");
    let expected_breaks = report["breaks"].as_array().unwrap();
    assert_eq!(breaks.len(), expected_breaks.len(), "{name} breaks");
    for ((pair, gap), expected) in breaks
        .iter()
        .zip(containers(scene, "timeline-break"))
        .zip(expected_breaks)
    {
        assert_eq!(pair[0], expected["from"].as_i64().unwrap(), "{name}");
        assert_eq!(pair[1], expected["to"].as_i64().unwrap(), "{name}");
        let [start, end] = gap.rule.unwrap();
        near(
            start.x,
            expected["x0"].as_f64().unwrap(),
            SCENE,
            &format!("{name} break x0"),
        );
        near(
            end.x,
            expected["x1"].as_f64().unwrap(),
            SCENE,
            &format!("{name} break x1"),
        );
    }
    let lanes = report["lanes"].as_array().unwrap();
    let actual_lanes: Vec<_> = containers(scene, "timeline-lane").collect();
    assert_eq!(actual_lanes.len(), lanes.len(), "{name} lanes");
    for (lane, expected) in actual_lanes.iter().zip(lanes) {
        assert_eq!(lane.authored.id, expected["id"].as_str().unwrap());
        near(lane.bounds.y, expected["y"].as_f64().unwrap(), SCENE, name);
        near(
            lane.bounds.height,
            expected["height"].as_f64().unwrap(),
            SCENE,
            name,
        );
        near(
            lane.rule.unwrap()[0].y,
            expected["lineY"].as_f64().unwrap(),
            SCENE,
            name,
        );
    }
    let events = report["events"].as_array().unwrap();
    assert_eq!(scene.nodes.len(), events.len(), "{name} events");
    for (node, expected) in scene.nodes.iter().zip(events) {
        let context = format!("{name}/{}", node.authored.id);
        assert_eq!(node.authored.id, expected["id"].as_str().unwrap());
        for (actual, field) in [
            (node.bounds.x, "left"),
            (node.bounds.y, "top"),
            (node.bounds.width, "width"),
            (node.bounds.height, "height"),
        ] {
            near(
                actual,
                expected[field].as_f64().unwrap(),
                SCENE,
                &format!("{context}.{field}"),
            );
        }
        let stem = containers(scene, "timeline-stem")
            .find(|stem| stem.authored.id == node.authored.id)
            .unwrap();
        near(
            stem.rule.unwrap()[0].x,
            expected["x"].as_f64().unwrap(),
            SCENE,
            &format!("{context}.x"),
        );
    }
    let ticks = report["ticks"].as_array().unwrap();
    let shown: Vec<_> = containers(scene, "timeline-tick").collect();
    assert!(!shown.is_empty(), "{name} ticks");
    for tick in shown {
        let t = tick.authored.facts["t"].as_i64().unwrap();
        let expected = ticks
            .iter()
            .find(|candidate| candidate["t"].as_i64() == Some(t))
            .unwrap_or_else(|| panic!("{name} tick {t}"));
        near(
            tick.rule.unwrap()[0].x,
            expected["x"].as_f64().unwrap(),
            SCENE,
            &format!("{name} tick {t}"),
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

fn day_precision_differences(case: &Value) -> bool {
    case["phoenix"]["precision"].is_object()
}

#[test]
fn frozen_timeline_oracle_geometry_text_and_reader_attributes() {
    let (mut rendered, mut legends, mut breaks) = (0, 0, 0);
    for case in cases().iter().filter(|case| case["ok"] == true) {
        let name = case["name"].as_str().unwrap();
        let scene = compile(&phoenix_document(case), DiagramType::Timeline)
            .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        scene_matches_layout_report(&scene, &case["layout"], name);
        let html = html(&scene);
        let actual = Html::parse_fragment(&svg_of(&html));
        let reference = Html::parse_fragment(case["svg"].as_str().unwrap());
        for attribute in [
            "viewBox",
            "data-motion-choreography",
            "data-timeline-ui",
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
            "g[data-timeline-lane]",
            "g[data-timeline-break]",
            "g[data-timeline-evidence]",
            "line.tl-grid",
            "line.tl-axis",
            "text.tl-tick",
            "line[data-timeline-stem]",
            "circle.tl-halo",
            "circle.tl-pin",
        ] {
            let count = same_selection(&reference, &actual, css, &plain, name);
            assert!(count > 0 || css == "g[data-timeline-break]", "{name} {css}");
        }
        legends += usize::from(
            actual
                .select(&select("g[data-legend-semantic-kind='legend-title']"))
                .next()
                .is_some(),
        );
        breaks += actual.select(&select("g[data-timeline-break]")).count();
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
        let entries =
            "g[data-legend-semantic-kind]:not([data-legend-semantic-kind='legend-title'])";
        same_selection(&reference, &actual, entries, &legend, name);
        let title = |document: &Html, css: &str| {
            document.select(&select(css)).next().map(|text| {
                let value = text.value();
                (
                    text.text().collect::<String>(),
                    value.attr("x").map(str::to_owned),
                    value.attr("y").map(str::to_owned),
                    value.attr("font-size").map(str::to_owned),
                )
            })
        };
        assert_eq!(
            title(&actual, "g[data-legend-semantic-kind='legend-title'] text"),
            title(&reference, "g[data-legend] > text"),
            "{name} legend title"
        );
        if day_precision_differences(case) {
            day_precision_matches_reference_except_labels(case, &scene, &reference, &actual);
        } else {
            same_selection(
                &reference,
                &actual,
                "text[data-timeline-axis-caption]",
                &plain,
                name,
            );
            same_selection(
                &reference,
                &actual,
                "g[data-node-id]",
                &Rules {
                    extra: &["data-source-path", "data-graph-role"],
                    skip: &[],
                },
                name,
            );
        }
        let check = check_html(html.as_bytes(), scene.quality);
        assert_eq!(check.status, "pass", "{name}: {:?}", check.diagnostics);
        assert!(
            check.diagnostics.is_empty(),
            "{name}: {:?}",
            check.diagnostics
        );
        rendered += 1;
    }
    assert_eq!(rendered, 28);
    assert!(
        legends > 10 && breaks > 5,
        "{legends} legends, {breaks} breaks"
    );
}

// Date-only events sit at the same instants; their labels state the authored
// date, and the caption discloses the noon placement.
fn day_precision_matches_reference_except_labels(
    case: &Value,
    scene: &CompiledDiagram,
    reference: &Html,
    actual: &Html,
) {
    let name = case["name"].as_str().unwrap();
    let locale = phoenix_diagram::i18n::resolve(&case["document"]["meta"]);
    let (date_only, disclosure) = (
        locale.text("timeline.precision.day", &[]),
        locale.text("timeline.precision.dayDisclosure", &[]),
    );
    let caption = |document: &Html| {
        document
            .select(&select("text[data-timeline-axis-caption]"))
            .next()
            .unwrap()
            .text()
            .collect::<String>()
    };
    assert_eq!(
        caption(actual),
        format!("{} · {disclosure}", caption(reference)),
        "{name}"
    );
    let rules = Rules {
        extra: &["data-source-path", "data-graph-role"],
        skip: &["aria-label", "data-node-context"],
    };
    let left: Vec<_> = reference.select(&select("g[data-node-id]")).collect();
    let right: Vec<_> = actual.select(&select("g[data-node-id]")).collect();
    assert_eq!(left.len(), right.len());
    for (reference_node, actual_node) in left.into_iter().zip(right) {
        let id = actual_node.value().attr("data-node-id").unwrap();
        let precision = &case["phoenix"]["precision"][id];
        let time = |node: scraper::ElementRef<'_>| {
            node.select(&select("text.tl-time"))
                .next()
                .unwrap()
                .text()
                .collect::<String>()
        };
        if precision == "day" {
            let at = actual_node.value().attr("data-timeline-at").unwrap();
            assert_eq!(
                time(actual_node),
                format!("{} · {date_only}", &at[..10]),
                "{name} {id}"
            );
            assert!(
                !actual_node
                    .value()
                    .attr("data-node-context")
                    .unwrap()
                    .contains("12:00"),
                "{name} {id}"
            );
            let shapes = Rules {
                extra: &["data-graph-role"],
                skip: &[],
            };
            for css in ["rect", "line", "circle", "text[data-node-label]"] {
                let (a, b): (Vec<_>, Vec<_>) = (
                    reference_node.select(&select(css)).collect(),
                    actual_node.select(&select(css)).collect(),
                );
                assert_eq!(a.len(), b.len());
                for (a, b) in a.into_iter().zip(b) {
                    same_element(a, b, &shapes, &format!("{name} {id}"));
                }
            }
        } else {
            assert_eq!(time(actual_node), time(reference_node), "{name} {id}");
            same_element(reference_node, actual_node, &rules, &format!("{name} {id}"));
        }
    }
    let DiagramFacts::Timeline { events, .. } = &scene.facts else {
        panic!()
    };
    assert_eq!(
        events
            .iter()
            .filter(|event| event.precision == "day")
            .count(),
        2
    );
}

#[test]
fn fixed_offset_zones_match_the_frozen_renderer_end_to_end() {
    let fixed: Vec<_> = cases()
        .into_iter()
        .filter(|case| {
            case["ok"] == true && case["name"].as_str().unwrap().starts_with("incident-zone-")
        })
        .filter(|case| {
            case["document"]["meta"]["timezone"]
                .as_str()
                .unwrap()
                .starts_with(['+', '-', '\u{2212}'])
        })
        .collect();
    assert_eq!(fixed.len(), 6);
    for case in fixed {
        let zone = case["document"]["meta"]["timezone"].as_str().unwrap();
        let scene = compile(&case["document"], DiagramType::Timeline).unwrap();
        let caption = &scene.text[0].content;
        let reference = Html::parse_fragment(case["svg"].as_str().unwrap());
        let expected = reference
            .select(&select("text[data-timeline-axis-caption]"))
            .next()
            .unwrap()
            .text()
            .collect::<String>();
        assert_eq!(caption, &expected, "{zone}");
        assert!(caption.contains(&format!("{zone} (UTC")), "{caption}");
    }
}

#[test]
fn rejected_timelines_keep_the_reference_diagnostic_codes() {
    let mut rejected = 0;
    for case in cases().iter().filter(|case| case["ok"] == false) {
        let name = case["name"].as_str().unwrap();
        let errors = compile(&case["document"], DiagramType::Timeline).expect_err(name);
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
        "../../../skills/phoenix-diagram/examples/minimal.timeline.json"
    ))
    .unwrap();
    assert_eq!(document["meta"]["input_contract"], "phoenix-v1");
    assert!(document["meta"]["evidence"].is_string());
    compile(&document, DiagramType::Timeline).unwrap()
}

#[test]
fn minimal_example_keeps_time_facts_and_passes_showcase() {
    let scene = example();
    let DiagramFacts::Timeline {
        evidence, events, ..
    } = &scene.facts
    else {
        panic!("timeline facts")
    };
    assert_eq!(evidence, "observed");
    assert!(events.iter().any(|event| event.precision == "day"));
    assert!(scene.relations.is_empty());
    let html = html(&scene);
    let check = check_html(html.as_bytes(), Quality::Showcase);
    assert_eq!(check.status, "pass", "{:?}", check.diagnostics);
    let metadata = Html::parse_document(&html);
    let metadata: Value = serde_json::from_str(
        &metadata
            .select(&select("#phoenix-diagram-metadata"))
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
    )
    .unwrap();
    assert!(metadata["facts"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["precision"] == "day"));
    assert_eq!(metadata["graph"]["relations"], json!([]));
}

#[test]
fn events_in_different_years_name_their_year_in_labels_ticks_and_details() {
    let document = json!({
        "schema_version": 1,
        "diagram_type": "timeline",
        "meta": {"title": "Release history", "output": "history.html", "evidence": "observed"},
        "events": [
            {"id": "a", "at": "2020-01-01T00:00Z", "title": "Start"},
            {"id": "b", "at": "2026-01-01T00:00Z", "title": "Today"}
        ]
    });
    let scene = compile(&document, DiagramType::Timeline).unwrap();
    let html = html(&scene);
    let check = check_html(html.as_bytes(), scene.quality);
    assert_eq!(check.status, "pass", "{:?}", check.diagnostics);
    let svg = Html::parse_fragment(&svg_of(&html));
    for (id, date) in [("a", "2020-01-01"), ("b", "2026-01-01")] {
        let node = svg
            .select(&select(&format!("g[data-node-id='{id}']")))
            .next()
            .unwrap();
        let time: String = node
            .select(&select("text.tl-time"))
            .next()
            .unwrap()
            .text()
            .collect();
        assert_eq!(time, format!("{date} 00:00"), "{id}");
        let context = node.value().attr("data-node-context").unwrap();
        assert!(
            context.contains(&format!("{date} 00:00:00")),
            "{id}: {context}"
        );
    }
    let dated: Vec<String> = svg
        .select(&select("text.tl-tick"))
        .map(|tick| tick.text().collect::<String>())
        .filter(|label| label.split(' ').next().unwrap().contains('-'))
        .collect();
    assert!(!dated.is_empty());
    for label in dated {
        assert_eq!(
            label.split(' ').next().unwrap().split('-').count(),
            3,
            "{label}"
        );
    }
}

#[track_caller]
fn failing_codes(html: &str) -> Vec<String> {
    let check = check_html(html.as_bytes(), Quality::Showcase);
    assert_eq!(check.status, "failed");
    check.diagnostics.into_iter().map(|d| d.code).collect()
}

#[test]
fn artifact_check_rejects_lost_event_text_lanes_and_any_relationship() {
    let scene = example();
    let html = html(&scene);
    let node = &scene.nodes[0];
    let title = node
        .text
        .iter()
        .find(|text| text.source_path.ends_with("/title"))
        .unwrap();
    let missing_title = html.replacen(&format!(">{}</tspan>", title.content), "></tspan>", 1);
    assert_ne!(missing_title, html);
    assert!(failing_codes(&missing_title).contains(&"artifact/semantic-text".into()));

    let time = node
        .text
        .iter()
        .find(|text| text.source_path.ends_with("/at"))
        .unwrap();
    let wrong_time = html.replacen(&format!(">{}</text>", time.content), ">00:00</text>", 1);
    assert_ne!(wrong_time, html);
    assert!(failing_codes(&wrong_time).contains(&"artifact/semantic-text".into()));

    let lost_disclosure = html.replacen(
        " · Date-only events sit at noon of their stated offset.",
        "",
        1,
    );
    assert_ne!(lost_disclosure, html);
    assert!(failing_codes(&lost_disclosure).contains(&"artifact/semantic-text".into()));

    let first_lane = scene
        .containers
        .iter()
        .find(|c| c.authored.kind == "timeline-lane")
        .unwrap();
    let moved = html.replacen(
        &format!("data-timeline-lane=\"{}\"", first_lane.authored.id),
        "data-timeline-lane=\"elsewhere\"",
        1,
    );
    assert!(failing_codes(&moved).contains(&"artifact/semantic-relation".into()));

    let causal = html.replacen(
        "<g id=\"node-",
        "<path data-edge-from=\"a\" data-edge-to=\"b\" d=\"M 0 0 L 1 0\"/><g id=\"node-",
        1,
    );
    assert!(failing_codes(&causal).contains(&"artifact/semantic-count".into()));
    let arrowhead = html.replacen(
        "class=\"tl-stem ",
        "marker-end=\"url(#arrowhead)\" class=\"tl-stem ",
        1,
    );
    assert_ne!(arrowhead, html);
    assert!(failing_codes(&arrowhead).contains(&"artifact/semantic-count".into()));
}
