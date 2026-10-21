#[path = "support/svg_oracle.rs"]
mod svg_oracle;

use phoenix_diagram::check::check_html;
use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, Quality};
use phoenix_diagram::scene::{DiagramFacts, Marker};
use serde_json::Value;
use std::collections::BTreeSet;
use svg_oracle::*;

fn corpus() -> Value {
    serde_json::from_str(include_str!("fixtures/class/oracle.json")).unwrap()
}
fn cases(ok: bool) -> Vec<Value> {
    corpus()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["ok"] == ok)
        .cloned()
        .collect()
}
fn class(case: &Value) -> phoenix_diagram::scene::CompiledDiagram {
    compile(&case["document"], DiagramType::Class, quality(case))
        .unwrap_or_else(|errors| panic!("{}: {errors:#?}", case["name"]))
}

// Deviation from the source: on a canvas wider than the artifact gate's 930px
// desktop width, the kind badge and keyword are raised to its 6px floor. Every
// other value, and every value on a narrower canvas, is the source's.
fn desktop_floor(case: &Value, mut elements: Vec<svg_oracle::Element>) -> Vec<svg_oracle::Element> {
    let width = case["layout"]["viewBox"][0].as_f64().unwrap();
    if width <= 930.0 {
        return elements;
    }
    let floor = (6.0 * width / 930.0 * 10.0).ceil() / 10.0;
    for element in &mut elements {
        let header = element.attr("class").is_some_and(|class| {
            class.contains("cl-badge-letter") || class.contains("cl-stereotype")
        });
        if header && element.number("font-size") < floor {
            element
                .attrs
                .insert("font-size".into(), Value::String(floor.to_string()));
        }
    }
    elements
}

const READER_KEYS: [&str; 26] = [
    "data-class-ui",
    "data-reader-min-text",
    "data-animation",
    "data-preset",
    "data-quality-profile",
    "data-node-id",
    "data-node-label",
    "data-node-kind",
    "data-node-sublabel",
    "data-node-context",
    "data-class-kind",
    "data-class-compartment",
    "data-class-member",
    "data-class-continuation",
    "data-class-relationship",
    "data-class-bus",
    "data-detail",
    "data-edge-from",
    "data-edge-to",
    "data-edge-label",
    "data-edge-key",
    "data-edge-id",
    "data-legend-semantic-kind",
    "data-animate",
    "aria-hidden",
    "role",
];
const DRAWING_KEYS: [&str; 26] = [
    "x",
    "y",
    "width",
    "height",
    "rx",
    "x1",
    "y1",
    "x2",
    "y2",
    "d",
    "class",
    "font-size",
    "font-weight",
    "font-style",
    "text-anchor",
    "stroke-width",
    "stroke",
    "style",
    "data-detail",
    "data-class-member",
    "data-class-continuation",
    "data-class-compartment",
    "data-node-label",
    "aria-hidden",
    "aria-label",
    "data-animate",
];

#[test]
fn scene_geometry_matches_the_frozen_renderer() {
    for case in cases(true) {
        let name = case["name"].as_str().unwrap();
        let scene = class(&case);
        let layout = &case["layout"];
        near(
            scene.view_box.width,
            layout["viewBox"][0].as_f64().unwrap(),
            1e-8,
            name,
        );
        near(
            scene.view_box.height,
            layout["viewBox"][1].as_f64().unwrap(),
            1e-8,
            name,
        );
        let oracle = desktop_floor(&case, oracle_elements(&case));
        let groups = find(&oracle, |e| {
            e.tag == "g" && e.attr("data-node-id").is_some()
        });
        assert_eq!(scene.nodes.len(), groups.len(), "{name}");
        for (node, group) in scene.nodes.iter().zip(&groups) {
            let context = format!("{name}/{}", node.authored.id);
            assert_eq!(
                Some(node.authored.id.as_str()),
                oracle[*group].attr("data-node-id")
            );
            let body = descendants(&oracle, *group)
                .into_iter()
                .find(|index| oracle[*index].attr("class") == Some("c-mask"))
                .unwrap();
            for (value, key) in [
                (node.bounds.x, "x"),
                (node.bounds.y, "y"),
                (node.bounds.width, "width"),
                (node.bounds.height, "height"),
            ] {
                near(
                    value,
                    oracle[body].number(key),
                    1e-8,
                    &format!("{context}/{key}"),
                );
            }
            // Badge, keyword and name, then every member line: nothing is truncated.
            let texts: Vec<_> = descendants(&oracle, *group)
                .into_iter()
                .filter(|index| oracle[*index].tag == "text")
                .map(|index| &oracle[index])
                .collect();
            let actual: Vec<_> = node
                .text
                .iter()
                .chain(node.detail_rows.iter().flat_map(|row| &row.text))
                .collect();
            assert_eq!(actual.len(), texts.len(), "{context}");
            for (text, expected) in actual.iter().zip(texts) {
                assert_eq!(text.content, expected.full, "{context}");
                near(text.position.x, expected.number("x"), 1e-8, &context);
                near(text.position.y, expected.number("y"), 1e-8, &context);
                near(text.font_size, expected.number("font-size"), 1e-8, &context);
            }
        }
        let expected = layout["relationships"].as_array().unwrap();
        assert_eq!(scene.relations.len(), expected.len(), "{name}");
        for (index, (relation, oracle_route)) in scene.relations.iter().zip(expected).enumerate() {
            let context = format!("{name}/relationship-{index}");
            assert_eq!(relation.from, oracle_route["from"].as_str().unwrap());
            assert_eq!(relation.to, oracle_route["to"].as_str().unwrap());
            let points = oracle_route["points"].as_array().unwrap();
            assert_eq!(
                relation.points.len(),
                points.len(),
                "{context}: {:?}",
                relation.points
            );
            for (point, expected) in relation.points.iter().zip(points) {
                near(point.x, expected[0].as_f64().unwrap(), 1e-8, &context);
                near(point.y, expected[1].as_f64().unwrap(), 1e-8, &context);
            }
            let label = find(&oracle, |e| {
                e.tag == "text"
                    && e.parent >= 0
                    && oracle[e.parent as usize].attr("data-edge-key") == Some(&index.to_string())
            });
            assert_eq!(relation.text.len(), label.len(), "{context}");
            if let (Some(text), Some(expected)) = (relation.text.first(), label.first()) {
                near(
                    text.position.x,
                    oracle[*expected].number("x"),
                    1e-8,
                    &context,
                );
                near(
                    text.position.y,
                    oracle[*expected].number("y"),
                    1e-8,
                    &context,
                );
            }
        }
    }
}

#[test]
fn rendered_svg_matches_the_frozen_renderer() {
    for case in cases(true) {
        let name = case["name"].as_str().unwrap();
        let scene = class(&case);
        let actual = rendered_elements(&html(&scene));
        let expected = desktop_floor(&case, oracle_elements(&case));
        let root = (&expected[0], &actual[0]);
        assert_eq!(root.1.attr("data-class-ui"), Some(""), "{name}");
        assert_eq!(
            root.0.attr("data-reader-min-text"),
            root.1.attr("data-reader-min-text"),
            "{name}"
        );
        if let Some(fit) = root.0.attr("data-reader-fit") {
            assert_eq!(root.1.attr("data-reader-fit"), Some(fit), "{name}");
        }
        assert_eq!(
            numbers(root.0.attr("viewBox").unwrap()),
            numbers(root.1.attr("viewBox").unwrap()),
            "{name}"
        );
        for selector in [
            |e: &svg_oracle::Element| e.tag == "g" && e.attr("data-node-id").is_some(),
            |e: &svg_oracle::Element| e.tag == "g" && e.attr("data-edge-key").is_some(),
        ] {
            let left = find(&expected, selector);
            let right = find(&actual, selector);
            assert_eq!(left.len(), right.len(), "{name}");
            for (a, b) in left.iter().zip(&right) {
                let context = format!(
                    "{name}/{:?}",
                    expected[*a]
                        .attr("data-node-id")
                        .or(expected[*a].attr("data-edge-key"))
                );
                same_attributes(
                    &expected[*a],
                    &actual[*b],
                    &[
                        "data-node-id",
                        "data-node-label",
                        "data-node-kind",
                        "data-node-sublabel",
                        "data-node-context",
                        "aria-label",
                        "class",
                        "data-class-kind",
                        "data-edge-key",
                        "data-class-relationship",
                    ],
                    1e-5,
                    &context,
                );
                same_subtree(&expected, *a, &actual, *b, &DRAWING_KEYS, &context);
            }
        }
        let left = find(&expected, |e| {
            e.tag == "path" && e.attr("data-edge-key").is_some()
        });
        let right = find(&actual, |e| {
            e.tag == "path" && e.attr("data-edge-key").is_some()
        });
        assert_eq!(left.len(), right.len(), "{name}");
        for (a, b) in left.iter().zip(&right) {
            let context = format!("{name}/path-{a}");
            same_attributes(
                &expected[*a],
                &actual[*b],
                &[
                    "d",
                    "data-composition-points",
                    "class",
                    "stroke-width",
                    "marker-start",
                    "marker-end",
                    "data-class-relationship",
                    "data-class-bus",
                    "data-edge-from",
                    "data-edge-to",
                    "data-edge-label",
                    "data-edge-id",
                    "data-animate",
                ],
                1e-5,
                &context,
            );
            let motion =
                |element: &svg_oracle::Element| element.attr("data-motion-path").map(numbers);
            let (x, y) = (motion(&expected[*a]), motion(&actual[*b]));
            assert_eq!(x.is_some(), y.is_some(), "{context}: motion path");
            for (p, q) in x.unwrap_or_default().iter().zip(y.unwrap_or_default()) {
                near(*p, q, 1e-5, &context);
            }
        }
        let legend = |elements: &[svg_oracle::Element]| {
            find(elements, |e| {
                e.tag == "g"
                    && e.attr("data-legend-semantic-kind")
                        .is_some_and(|kind| kind != "legend-title")
            })
        };
        let (left, right) = (legend(&expected), legend(&actual));
        assert_eq!(left.len(), right.len(), "{name}: legend entries");
        for (a, b) in left.iter().zip(&right) {
            let context = format!(
                "{name}/legend/{:?}",
                expected[*a].attr("data-legend-semantic-kind")
            );
            same_attributes(
                &expected[*a],
                &actual[*b],
                &[
                    "data-legend-semantic-kind",
                    "data-legend-x",
                    "data-legend-baseline",
                    "data-legend-width",
                ],
                1e-5,
                &context,
            );
            same_subtree(
                &expected,
                *a,
                &actual,
                *b,
                &[
                    "x",
                    "y",
                    "width",
                    "height",
                    "rx",
                    "d",
                    "class",
                    "font-size",
                    "font-weight",
                    "stroke-width",
                    "marker-start",
                    "marker-end",
                    "aria-label",
                ],
                &context,
            );
        }
        let skip = |e: &svg_oracle::Element| {
            e.attr("data-legend-semantic-kind") == Some("legend-title")
                || (e.tag == "text"
                    && e.attr("data-source-path")
                        .is_some_and(|path| path.starts_with("/meta")))
                || (e.tag == "g" && e.attr("data-graph-role") == Some("legend"))
        };
        let reference_skip =
            |e: &svg_oracle::Element| e.tag == "g" && e.attr("data-legend-semantic-kind").is_some();
        assert_eq!(
            reader_projection(&expected, &READER_KEYS, reference_skip),
            reader_projection(&actual, &READER_KEYS, skip),
            "{name}: Reader attributes"
        );
    }
}

#[test]
fn rejected_geometry_keeps_the_source_diagnostic_codes() {
    for case in cases(false) {
        let name = case["name"].as_str().unwrap();
        let errors = compile(&case["document"], DiagramType::Class, quality(&case))
            .err()
            .unwrap_or_else(|| panic!("{name} must be rejected"));
        let actual: BTreeSet<_> = errors.iter().map(|error| error.code.as_str()).collect();
        let expected: BTreeSet<_> = case["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "{name}: {errors:#?}");
    }
}

#[test]
fn every_member_and_relationship_notation_survives_into_the_scene() {
    for case in cases(true) {
        let scene = class(&case);
        let document = &case["document"];
        match &scene.facts {
            DiagramFacts::Class {
                types,
                relationships,
            } => {
                assert_eq!(types, document["types"].as_array().unwrap());
                assert_eq!(
                    relationships,
                    document["relationships"].as_array().unwrap_or(&vec![])
                );
            }
            _ => panic!("class facts"),
        }
        for (index, node) in scene.nodes.iter().enumerate() {
            let raw = &document["types"][index];
            assert_eq!(node.authored.facts, *raw);
            let members = raw["attributes"].as_array().map_or(0, Vec::len)
                + raw["methods"].as_array().map_or(0, Vec::len);
            assert_eq!(node.detail_rows.len(), members);
            // A wrapped signature still states every character of the member.
            for row in &node.detail_rows {
                let member = &raw[row.role.as_str()][row
                    .source_path
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()];
                let visible: String = row
                    .text
                    .iter()
                    .map(|text| text.content.as_str())
                    .collect::<String>();
                let strip = |value: &str| {
                    value
                        .chars()
                        .filter(|c| !c.is_whitespace() && *c != ':')
                        .collect::<String>()
                };
                let mut expected = String::new();
                if let Some(glyph) = member["visibility"].as_str() {
                    expected.push_str(match glyph {
                        "public" => "+",
                        "protected" => "#",
                        "package" => "~",
                        _ => "-",
                    });
                }
                expected.push_str(member["name"].as_str().unwrap());
                if row.role == "methods" {
                    expected.push_str(&format!(
                        "({})",
                        member["parameters"].as_str().unwrap_or("")
                    ));
                    expected.push_str(member["returns"].as_str().unwrap_or(""));
                } else {
                    expected.push_str(member["type"].as_str().unwrap_or(""));
                }
                assert_eq!(strip(&visible), strip(&expected), "{}", row.source_path);
            }
        }
        for relation in &scene.relations {
            let kind = relation.authored.facts["kind"].as_str().unwrap();
            let (start, end) = match kind {
                "dependency" | "association" => (None, Some(Marker::Open)),
                "inheritance" | "realization" => (None, Some(Marker::Triangle)),
                "composition" => (Some(Marker::DiamondFilled), None),
                _ => (Some(Marker::DiamondHollow), None),
            };
            assert_eq!(
                (relation.marker_start, relation.marker_end),
                (start, end),
                "{kind}"
            );
        }
    }
}

#[test]
fn rendered_oracle_cases_pass_the_artifact_gate_in_their_profile() {
    for case in cases(true) {
        let name = case["name"].as_str().unwrap();
        let scene = class(&case);
        let result = check_html(html(&scene).as_bytes(), quality(&case));
        assert_eq!(result.status, "pass", "{name}: {:#?}", result.diagnostics);
    }
}

#[test]
fn the_gate_notices_a_dropped_member_or_notation() {
    let case = cases(true)
        .into_iter()
        .find(|case| case["name"] == "every-kind-and-notation")
        .unwrap();
    let rendered = html(&class(&case));
    let baseline = check_html(rendered.as_bytes(), Quality::Showcase);
    assert_eq!(baseline.status, "pass", "{:#?}", baseline.diagnostics);
    for (from, to) in [
        (">Repository</text>", "></text>"),
        (
            "marker-start=\"url(#cl-diamond-filled)\"",
            "marker-start=\"url(#cl-diamond-hollow)\"",
        ),
        (
            "marker-end=\"url(#cl-triangle)\"",
            "marker-end=\"url(#cl-open)\"",
        ),
        ("data-class-kind=\"enum\"", "data-class-kind=\"class\""),
    ] {
        assert!(rendered.contains(from), "{from}");
        let result = check_html(rendered.replacen(from, to, 1).as_bytes(), Quality::Showcase);
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.name == "semantic_content" && check.status == "failed"),
            "{from}: {:#?}",
            result.checks
        );
    }
}

#[test]
fn a_wrapped_member_reads_whole_across_its_lines() {
    let case = cases(true)
        .into_iter()
        .find(|case| case["name"] == "member-wrapping")
        .unwrap();
    let rendered = html(&class(&case));
    // Dropping one continuation line must fail even though every other line stays.
    let marker = "data-class-continuation=\"\"";
    let start = rendered.find(marker).unwrap();
    let open = rendered[..start].rfind("<text").unwrap();
    let close = start + rendered[start..].find("</text>").unwrap() + "</text>".len();
    let tampered = format!("{}{}", &rendered[..open], &rendered[close..]);
    let result = check_html(tampered.as_bytes(), Quality::Showcase);
    assert!(
        result
            .checks
            .iter()
            .any(|check| check.name == "semantic_content" && check.status == "failed"),
        "{:#?}",
        result.checks
    );
}

#[test]
fn the_bundled_example_compiles_under_its_persisted_contract() {
    let bytes = include_bytes!("../../../skills/phoenix-diagram/examples/minimal.class.json");
    let options = CompileOptions::default();
    let input = parse_input(bytes, DiagramType::Class, &options).unwrap();
    let scene = phoenix_diagram::layout::compile(&input, &options).unwrap();
    let rendered = html(&scene);
    assert!(
        rendered.contains("data-class-bus=\"gateway\""),
        "the example draws its hierarchy bus"
    );
    let result = check_html(rendered.as_bytes(), Quality::Showcase);
    assert_eq!(result.status, "pass", "{:#?}", result.diagnostics);
}

// Two ends that meet at one point collide only where a symbol is drawn: a
// composition carries its diamond at the source, an association its arrow at
// the target.
fn arrowhead_collision(kind: &str, outward: bool) -> bool {
    let relation = |id: &str, other: &str| {
        let (from, to) = if outward {
            ("hub", other)
        } else {
            (other, "hub")
        };
        serde_json::json!({"id":id,"from":from,"to":to,"kind":kind})
    };
    let document = serde_json::json!({
        "schema_version": 1, "diagram_type": "class",
        "meta": {"title": "Ends", "output": "ends.html", "quality_profile": "showcase"},
        "layout": {"mode": "grid"},
        "types": [
            {"id": "hub", "label": "Hub", "kind": "class", "row": 0, "col": 1},
            {"id": "a", "label": "A", "kind": "class", "row": 1, "col": 0},
            {"id": "b", "label": "B", "kind": "class", "row": 1, "col": 2}
        ],
        "relationships": [relation("first", "a"), relation("second", "b")]
    });
    let mut scene = compile(&document, DiagramType::Class, Quality::Showcase).unwrap();
    let point = |x: f64, y: f64| phoenix_diagram::scene::Point { x, y };
    let (mut first, mut second) = (
        vec![point(10.0, 300.0), point(10.0, 200.0), point(100.0, 200.0)],
        vec![point(20.0, 320.0), point(20.0, 200.0), point(100.0, 200.0)],
    );
    if outward {
        first.reverse();
        second.reverse();
    }
    scene.relations[0].points = first;
    scene.relations[1].points = second;
    check_html(html(&scene).as_bytes(), Quality::Showcase)
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "composition/arrowhead-collision")
}

#[test]
fn arrowhead_checks_follow_the_ends_that_carry_a_marker() {
    assert!(arrowhead_collision("association", false));
    assert!(!arrowhead_collision("composition", false));
    assert!(arrowhead_collision("composition", true));
    assert!(!arrowhead_collision("association", true));
}
