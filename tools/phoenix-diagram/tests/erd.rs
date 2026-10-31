#[path = "support/svg_oracle.rs"]
mod svg_oracle;

use phoenix_diagram::check::check_html;
use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, Quality};
use phoenix_diagram::scene::{DiagramFacts, Marker};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use svg_oracle::*;

fn corpus() -> Value {
    serde_json::from_str(include_str!("fixtures/erd/oracle.json")).unwrap()
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
fn erd(case: &Value) -> phoenix_diagram::scene::CompiledDiagram {
    compile(&case["document"], DiagramType::Erd, quality(case))
        .unwrap_or_else(|errors| panic!("{}: {errors:#?}", case["name"]))
}

const READER_KEYS: [&str; 26] = [
    "data-erd-ui",
    "data-reader-min-text",
    "data-animation",
    "data-preset",
    "data-quality-profile",
    "data-node-id",
    "data-node-label",
    "data-node-kind",
    "data-node-tag",
    "data-node-sublabel",
    "data-node-context",
    "data-er-row",
    "data-er-badge",
    "data-er-ends",
    "data-er-trunk",
    "data-domain-band",
    "data-domain-label",
    "data-detail",
    "data-edge-from",
    "data-edge-to",
    "data-edge-label",
    "data-edge-key",
    "data-edge-id",
    "data-legend-semantic-kind",
    "data-animate",
    "data-semantic-sigil",
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
    "cx",
    "cy",
    "r",
    "d",
    "transform",
    "class",
    "font-size",
    "font-weight",
    "text-anchor",
    "stroke-width",
    "data-detail",
    "data-er-row",
    "data-er-badge",
    "data-node-label",
    "data-semantic-sigil",
    "data-domain-label",
    "aria-label",
];

#[test]
fn scene_geometry_matches_the_frozen_renderer() {
    for case in cases(true) {
        let name = case["name"].as_str().unwrap();
        let scene = erd(&case);
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
        let oracle = oracle_elements(&case);
        let groups = find(&oracle, |e| {
            e.tag == "g" && e.attr("data-node-id").is_some()
        });
        assert_eq!(scene.nodes.len(), groups.len(), "{name}");
        for (node, group) in scene.nodes.iter().zip(&groups) {
            assert_eq!(
                Some(node.authored.id.as_str()),
                oracle[*group].attr("data-node-id")
            );
            let body = descendants(&oracle, *group)
                .into_iter()
                .find(|index| oracle[*index].attr("class") == Some("c-mask"))
                .unwrap();
            let context = format!("{name}/{}", node.authored.id);
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
            // Title, then every row: badges, name and type at their exact baselines.
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
            assert_eq!(
                actual.len(),
                texts.len(),
                "{context}: every column survives"
            );
            for (text, expected) in actual.iter().zip(texts) {
                assert_eq!(text.content, expected.text, "{context}");
                near(text.position.x, expected.number("x"), 1e-8, &context);
                near(text.position.y, expected.number("y"), 1e-8, &context);
                near(text.font_size, expected.number("font-size"), 1e-8, &context);
            }
            assert_eq!(
                node.detail_rows.len(),
                node.authored.facts["attributes"]
                    .as_array()
                    .map_or(0, Vec::len)
            );
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

/// The table detail Phoenix adds after the reference context: each foreign
/// key's column and the column it references.
fn foreign_key_targets(entity: &Value) -> String {
    let targets: Vec<String> = entity["attributes"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|attribute| {
            let target = attribute["references"].as_str()?;
            Some(format!(
                "{} → {target}",
                attribute["name"].as_str().unwrap()
            ))
        })
        .collect();
    if targets.is_empty() {
        String::new()
    } else {
        format!(" \u{203a} {}", targets.join(", "))
    }
}

/// The frozen renderer omits foreign key targets; strip them to compare the rest.
fn without_foreign_key_targets(
    mut elements: Vec<svg_oracle::Element>,
    case: &Value,
) -> Vec<svg_oracle::Element> {
    for entity in case["document"]["entities"].as_array().unwrap() {
        let suffix = foreign_key_targets(entity);
        if suffix.is_empty() {
            continue;
        }
        let id = entity["id"].as_str().unwrap();
        let node = find(&elements, |e| {
            e.tag == "g" && e.attr("data-node-id") == Some(id)
        })[0];
        for key in ["data-node-context", "aria-label"] {
            let value = elements[node].attr(key).unwrap();
            assert!(value.contains(&suffix), "{id} {key}: {value}");
            let value = value.replacen(&suffix, "", 1);
            elements[node]
                .attrs
                .insert(key.into(), Value::String(value));
        }
        for index in descendants(&elements, node) {
            if elements[index].tag == "title" {
                assert!(elements[index].text.contains(&suffix), "{id} title");
                elements[index].text = elements[index].text.replacen(&suffix, "", 1);
            }
        }
    }
    elements
}

#[test]
fn rendered_svg_matches_the_frozen_renderer() {
    for case in cases(true) {
        let name = case["name"].as_str().unwrap();
        let scene = erd(&case);
        let actual = without_foreign_key_targets(rendered_elements(&html(&scene)), &case);
        let expected = oracle_elements(&case);
        let root = (&expected[0], &actual[0]);
        assert_eq!(root.1.attr("data-erd-ui"), Some("cards"), "{name}");
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
            |e: &svg_oracle::Element| e.tag == "g" && e.attr("data-domain-band").is_some(),
            |e: &svg_oracle::Element| e.tag == "g" && e.attr("data-edge-key").is_some(),
        ] {
            let left = find(&expected, selector);
            let right = find(&actual, selector);
            assert_eq!(left.len(), right.len(), "{name}");
            for (a, b) in left.iter().zip(&right) {
                let context = format!("{name}/{:?}", expected[*a].attrs);
                same_attributes(
                    &expected[*a],
                    &actual[*b],
                    &[
                        "data-node-id",
                        "data-node-label",
                        "data-node-kind",
                        "data-node-tag",
                        "data-node-context",
                        "aria-label",
                        "class",
                        "data-domain-band",
                        "data-edge-key",
                        "data-er-ends",
                    ],
                    1e-5,
                    &context,
                );
                same_subtree(&expected, *a, &actual, *b, &DRAWING_KEYS, &context);
            }
        }
        for selector in [
            |e: &svg_oracle::Element| e.tag == "path" && e.attr("data-edge-key").is_some(),
            |e: &svg_oracle::Element| e.tag == "path" && e.attr("data-er-trunk").is_some(),
        ] {
            let left = find(&expected, selector);
            let right = find(&actual, selector);
            assert_eq!(left.len(), right.len(), "{name}");
            for (a, b) in left.iter().zip(&right) {
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
                        "data-er-ends",
                        "data-edge-from",
                        "data-edge-to",
                        "data-edge-label",
                        "data-edge-id",
                        "data-animate",
                    ],
                    1e-5,
                    &format!("{name}/path-{a}"),
                );
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
            // The shared legend writer owns the label's ink; geometry and text match.
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
                    "x1",
                    "y1",
                    "x2",
                    "y2",
                    "cx",
                    "cy",
                    "r",
                    "d",
                    "transform",
                    "font-size",
                    "stroke-width",
                    "data-er-badge",
                    "aria-label",
                ],
                &context,
            );
        }
        let title = |elements: &[svg_oracle::Element], legend_title: bool| {
            find(elements, |e| {
                e.tag == "text"
                    && e.parent >= 0
                    && if legend_title {
                        elements[e.parent as usize].attr("data-legend-semantic-kind")
                            == Some("legend-title")
                    } else {
                        elements[e.parent as usize].attr("data-legend") == Some("")
                    }
            })
        };
        let (left, right) = (title(&expected, false), title(&actual, true));
        assert_eq!(left.len(), right.len(), "{name}: legend title");
        for (a, b) in left.iter().zip(&right) {
            assert_eq!(expected[*a].text, actual[*b].text);
            same_attributes(
                &expected[*a],
                &actual[*b],
                &["x", "y", "font-size"],
                1e-5,
                name,
            );
        }
        let skip = |e: &svg_oracle::Element| {
            e.attr("data-legend-semantic-kind") == Some("legend-title")
                || (e.tag == "text"
                    && e.attr("data-source-path")
                        .is_some_and(|path| path.starts_with("/meta")))
        };
        assert_eq!(
            reader_projection(&expected, &READER_KEYS, |_| false),
            reader_projection(&actual, &READER_KEYS, skip),
            "{name}: Reader attributes"
        );
    }
}

#[test]
fn rejected_geometry_keeps_the_source_diagnostic_codes() {
    for case in cases(false) {
        let name = case["name"].as_str().unwrap();
        let errors = compile(&case["document"], DiagramType::Erd, quality(&case))
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
        assert!(errors.iter().all(|error| error.stage == "layout"), "{name}");
    }
}

#[test]
fn every_column_key_and_cardinality_survives_into_the_scene() {
    for case in cases(true) {
        let scene = erd(&case);
        let document = &case["document"];
        match &scene.facts {
            DiagramFacts::Erd {
                entities,
                relationships,
            } => {
                assert_eq!(entities, document["entities"].as_array().unwrap());
                assert_eq!(
                    relationships,
                    document["relationships"].as_array().unwrap_or(&vec![])
                );
            }
            _ => panic!("ERD facts"),
        }
        for (index, node) in scene.nodes.iter().enumerate() {
            assert_eq!(node.authored.facts, document["entities"][index]);
            assert_eq!(node.authored.source_path, format!("/entities/{index}"));
            for (row, attribute) in node.detail_rows.iter().zip(
                document["entities"][index]["attributes"]
                    .as_array()
                    .unwrap(),
            ) {
                let content: Vec<_> = row.text.iter().map(|text| text.content.as_str()).collect();
                assert!(content.contains(&attribute["name"].as_str().unwrap()));
                if let Some(kind) = attribute["type"].as_str().filter(|kind| !kind.is_empty()) {
                    assert!(content.contains(&kind));
                }
                for role in match &attribute["key"] {
                    Value::String(role) => vec![role.clone()],
                    Value::Array(roles) => roles
                        .iter()
                        .map(|role| role.as_str().unwrap().into())
                        .collect(),
                    _ => vec![],
                } {
                    assert!(content.contains(&role.to_uppercase().as_str()));
                }
            }
        }
        for (index, relation) in scene.relations.iter().enumerate() {
            let raw = &document["relationships"][index];
            assert_eq!(relation.authored.facts, *raw);
            let expected = |from: bool| {
                let cardinality = raw[if from {
                    "fromCardinality"
                } else {
                    "toCardinality"
                }]
                .as_str()
                .unwrap();
                let optional = raw[if from { "fromOptional" } else { "toOptional" }] == true;
                match (cardinality, optional) {
                    ("one", false) => Marker::ErOne,
                    ("one", true) => Marker::ErOneOptional,
                    ("many", false) => Marker::ErMany,
                    _ => Marker::ErManyOptional,
                }
            };
            assert_eq!(relation.marker_start, Some(expected(true)));
            assert_eq!(relation.marker_end, Some(expected(false)));
            if raw["identifying"] == false {
                assert_eq!(relation.variant, "dashed");
            }
        }
    }
}

#[test]
fn rendered_oracle_cases_pass_the_artifact_gate_in_their_profile() {
    for case in cases(true) {
        let name = case["name"].as_str().unwrap();
        let scene = erd(&case);
        let result = check_html(html(&scene).as_bytes(), quality(&case));
        assert_eq!(result.status, "pass", "{name}: {:#?}", result.diagnostics);
        let names: Vec<_> = result
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect();
        for expected in [
            "semantic_content",
            "relationship_endpoints",
            "route_binding",
            "arrowhead_separation",
            "relationship_crossings",
        ] {
            assert!(names.contains(&expected), "{name}: {names:?}");
        }
    }
}

#[test]
fn the_gate_notices_a_dropped_column_key_or_end() {
    let case = cases(true)
        .into_iter()
        .find(|case| case["name"] == "minimal-example")
        .unwrap();
    let rendered = html(&erd(&case));
    let baseline = check_html(rendered.as_bytes(), Quality::Showcase);
    assert_eq!(baseline.status, "pass", "{:#?}", baseline.diagnostics);
    for (from, to) in [
        (">customer_id</text>", "></text>"),
        (">PK</text>", ">UK</text>"),
        (">timestamptz</text>", "></text>"),
        (
            "marker-start=\"url(#er-many-start)\"",
            "marker-start=\"url(#er-one-start)\"",
        ),
    ] {
        assert!(rendered.contains(from), "{from}");
        let tampered = rendered.replacen(from, to, 1);
        let result = check_html(tampered.as_bytes(), Quality::Showcase);
        assert_eq!(result.status, "failed", "{from}");
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
fn a_foreign_key_names_its_target_column_in_the_table_detail() {
    let mut contexts = vec![];
    for target in ["customer.id", "customer.external_id"] {
        let document = json!({
            "schema_version": 1,
            "diagram_type": "erd",
            "meta": {"title": "Orders", "output": "orders.html", "quality_profile": "showcase"},
            "entities": [
                {"id": "customer", "label": "customer", "row": 0, "col": 0, "attributes": [
                    {"name": "id", "type": "bigint", "key": "pk"},
                    {"name": "external_id", "type": "bigint", "key": "uk"}
                ]},
                {"id": "orders", "label": "orders", "row": 0, "col": 1, "attributes": [
                    {"name": "id", "type": "bigint", "key": "pk"},
                    {"name": "customer_ref", "type": "bigint", "key": "fk", "references": target}
                ]}
            ],
            "relationships": [
                {"from": "orders", "to": "customer", "fromCardinality": "many", "toCardinality": "one"}
            ]
        });
        let scene = compile(&document, DiagramType::Erd, Quality::Showcase).unwrap();
        let rendered = html(&scene);
        let result = check_html(rendered.as_bytes(), Quality::Showcase);
        assert_eq!(result.status, "pass", "{target}: {:#?}", result.diagnostics);
        let elements = rendered_elements(&rendered);
        let orders = find(&elements, |e| {
            e.tag == "g" && e.attr("data-node-id") == Some("orders")
        })[0];
        let expected = format!("customer_ref → {target}");
        for key in ["data-node-context", "aria-label"] {
            let value = elements[orders].attr(key).unwrap();
            assert!(value.contains(&expected), "{key}: {value}");
        }
        let title = descendants(&elements, orders)
            .into_iter()
            .find(|index| elements[*index].tag == "title")
            .unwrap();
        assert!(
            elements[title].text.contains(&expected),
            "{}",
            elements[title].text
        );
        let context = elements[orders].attr("data-node-context").unwrap();
        // The node index groups tables by the first segment.
        assert_eq!(context.split(" \u{203a} ").next(), Some("Entity"));
        contexts.push(context.to_owned());
    }
    assert_ne!(contexts[0], contexts[1]);
}

#[test]
fn a_short_jog_between_side_by_side_tables_is_drawn_straight() {
    // Host first drafts: a two-column table beside a four-column one in the same
    // row. Their centres are 20px apart, so a dogleg would leave a 4px straight
    // piece between its rounded corners, below the micro-segment floor.
    let table = |id: &str, row: u64, col: u64, columns: usize| {
        let attributes: Vec<_> = (0..columns)
            .map(|n| {
                let mut column = json!({"name": format!("c{n}"), "type": "bigint"});
                if n == 0 {
                    column["key"] = json!("pk");
                }
                column
            })
            .collect();
        json!({"id": id, "label": id, "row": row, "col": col, "attributes": attributes})
    };
    for (left, right) in [("PUBLISHER", "EDITION"), ("product", "order_item")] {
        let document = json!({
            "schema_version": 1,
            "diagram_type": "erd",
            "meta": {"title": "Schema", "output": "schema.html", "quality_profile": "showcase"},
            "layout": {"mode": "grid"},
            "entities": [table("a", 0, 0, 2), table("b", 0, 1, 3), table(left, 1, 0, 2), table(right, 1, 1, 4)],
            "relationships": [
                {"from": "a", "to": "b", "label": "writes", "fromCardinality": "one", "toCardinality": "many", "toOptional": true},
                {"from": "b", "to": right, "label": "published as", "fromCardinality": "one", "toCardinality": "many"},
                {"from": left, "to": right, "label": "prints", "fromCardinality": "one", "fromOptional": true, "toCardinality": "many", "toOptional": true}
            ]
        });
        let scene = compile(&document, DiagramType::Erd, Quality::Showcase).unwrap();
        let result = check_html(html(&scene).as_bytes(), Quality::Showcase);
        assert_eq!(result.status, "pass", "{left}: {:#?}", result.diagnostics);
        let relation = &scene.relations[2];
        assert_eq!(relation.points.len(), 2, "{left}: {:?}", relation.points);
        assert_eq!(relation.points[0].y, relation.points[1].y);
    }
}

#[test]
fn the_bundled_example_compiles_under_its_persisted_contract() {
    let bytes = include_bytes!("../../../skills/phoenix-diagram/examples/minimal.erd.json");
    let options = CompileOptions::default();
    let input = parse_input(bytes, DiagramType::Erd, &options).unwrap();
    assert_eq!(input.quality, Quality::Showcase);
    let scene = phoenix_diagram::layout::compile(&input, &options).unwrap();
    let result = check_html(html(&scene).as_bytes(), Quality::Showcase);
    assert_eq!(result.status, "pass", "{:#?}", result.diagnostics);
    let document: Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(scene.meta, document["meta"]);
    assert_eq!(
        json!(scene.nodes.len()),
        json!(document["entities"].as_array().unwrap().len())
    );
}
