#[path = "support/scene_svg.rs"]
mod svg_oracle;

use phoenix_diagram::check::check_html;
use phoenix_diagram::geometry;
use phoenix_diagram::input::{DiagramType, Quality};
use phoenix_diagram::scene::{CompiledDiagram, DiagramFacts};
use scraper::Html;
use serde_json::{json, Value};
use svg_oracle::*;

fn oracle() -> Value {
    serde_json::from_str(include_str!("fixtures/tree/oracle.json")).unwrap()
}

fn cases() -> Vec<Value> {
    oracle()["cases"].as_array().unwrap().clone()
}

// The case the frozen renderer actually ran: the authored document plus the
// recorded Phoenix adaptation, if any.
fn reference_document(case: &Value) -> Value {
    let mut document = case["document"].clone();
    if let Some(layout) = case["adapted"]["layout"].as_object() {
        for (key, value) in layout {
            document["layout"][key] = value.clone();
        }
    }
    document
}

fn adapted(case: &Value, reason: &str) -> bool {
    case["adapted"]["reason"] == reason
}

fn scene_matches_layout_report(scene: &CompiledDiagram, report_case: &Value, name: &str) {
    let report = &report_case["layout"];
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
    let nodes = report["nodes"].as_array().unwrap();
    assert_eq!(scene.nodes.len(), nodes.len(), "{name}");
    for (node, expected) in scene.nodes.iter().zip(nodes) {
        let context = format!("{name}/{}", node.authored.id);
        assert_eq!(node.authored.id, expected["id"].as_str().unwrap());
        for (actual, field) in [
            (node.bounds.x, "x"),
            (node.bounds.y, "y"),
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
        let lines = node
            .text
            .iter()
            .filter(|text| text.source_path.ends_with("/label"))
            .count();
        assert_eq!(
            lines as u64,
            expected["lines"].as_u64().unwrap(),
            "{context} lines"
        );
        let stacked = node
            .authored
            .geometry_provenance
            .iter()
            .any(|p| p == "tree-stacked-leaf-run");
        assert_eq!(stacked, expected["stacked"] == true, "{context} stacked");
    }
    let edges = report["edges"].as_array().unwrap();
    assert_eq!(scene.relations.len(), edges.len(), "{name}");
    for (relation, expected) in scene.relations.iter().zip(edges) {
        assert_eq!(relation.from, expected["from"].as_str().unwrap());
        assert_eq!(relation.to, expected["to"].as_str().unwrap());
        let mut points = expected["points"].as_array().unwrap().clone();
        // An only-child list's spine runs on its rounded toggle x.
        if adapted(report_case, "composition/orthogonal-arrows") && points.len() == 3 {
            points[1][0] = points[0][0].clone();
        }
        assert_eq!(
            relation.points.len(),
            points.len(),
            "{name} {}",
            relation.to
        );
        for (point, expected) in relation.points.iter().zip(points) {
            near(
                point.x,
                expected[0].as_f64().unwrap(),
                SCENE,
                &format!("{name} {} x", relation.to),
            );
            near(
                point.y,
                expected[1].as_f64().unwrap(),
                SCENE,
                &format!("{name} {} y", relation.to),
            );
        }
    }
}

#[test]
fn frozen_tree_oracle_geometry_text_and_reader_attributes() {
    let mut rendered = 0;
    for case in cases().iter().filter(|case| case["ok"] == true) {
        let name = case["name"].as_str().unwrap();
        let scene = compile(&case["document"], DiagramType::Tree)
            .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        scene_matches_layout_report(&scene, case, name);
        let html = html(&scene);
        let actual = Html::parse_fragment(&svg_of(&html));
        let reference = Html::parse_fragment(case["svg"].as_str().unwrap());
        let root = |document: &Html, name: &str| {
            document
                .select(&select("svg"))
                .next()
                .unwrap()
                .value()
                .attr(name)
                .map(str::to_owned)
        };
        for attribute in [
            "viewBox",
            "data-tree-ui",
            "data-tree-direction",
            "data-reader-min-text",
        ] {
            assert_eq!(
                root(&actual, attribute),
                root(&reference, attribute),
                "{name} {attribute}"
            );
        }
        assert_eq!(
            root(&actual, "data-reader-fit"),
            Some(root(&reference, "data-reader-fit").unwrap_or_else(|| "contain".into())),
            "{name}: an authored canvas is read whole"
        );
        let nodes = Rules {
            extra: &["data-source-path", "data-graph-role"],
            skip: &[],
        };
        assert_eq!(
            same_selection(&reference, &actual, "g[data-node-id]", &nodes, name),
            case["document"]["nodes"].as_array().unwrap().len()
        );
        same_selection(&reference, &actual, "g[data-tree-toggle]", &nodes, name);
        same_selection(&reference, &actual, "g[data-tree-expand-all]", &nodes, name);
        // Routes are the scene routes compared above, drawn with 10px corners.
        same_selection(
            &reference,
            &actual,
            "path[data-edge-from]",
            &Rules {
                extra: &["data-graph-role"],
                skip: &["d", "data-composition-points"],
            },
            name,
        );
        for (index, edge) in actual.select(&select("path[data-edge-from]")).enumerate() {
            let points = &scene.relations[index].points;
            assert_eq!(
                edge.value().attr("d"),
                Some(geometry::rounded_path(points, 10.0).as_str())
            );
            let declared: Vec<f64> = numbers(edge.value().attr("data-composition-points").unwrap());
            let expected: Vec<f64> = points.iter().flat_map(|point| [point.x, point.y]).collect();
            assert_eq!(declared, expected, "{name} {index}");
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
    assert_eq!(rendered, 12);
}

#[test]
fn a_tree_too_wide_to_read_narrows_only_its_default_node_cap() {
    let case = cases()
        .into_iter()
        .find(|case| case["name"] == "archify-repository")
        .unwrap();
    assert_eq!(case["adapted"]["reason"], "composition/desktop-readability");
    let scene = compile(&case["document"], DiagramType::Tree).unwrap();
    let reference_width = case["adapted"]["unadaptedViewBox"][0].as_f64().unwrap();
    assert!(scene.view_box.width <= 1705.0 && reference_width > 1705.0);
    let explicit = compile(&reference_document(&case), DiagramType::Tree).unwrap();
    assert_eq!(scene.nodes, explicit.nodes);
    assert_eq!(scene.relations, explicit.relations);
    // An authored cap is kept as written, even when it is too wide to read.
    let mut authored = case["document"].clone();
    authored["layout"]["nodeMaxW"] = json!(220);
    let wide = compile(&authored, DiagramType::Tree).unwrap();
    near(wide.view_box.width, reference_width, SCENE, "authored cap");
    let check = check_html(html(&wide).as_bytes(), Quality::Showcase);
    assert!(check
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "composition/desktop-readability"));
}

#[test]
fn rejected_trees_keep_the_reference_diagnostic_codes() {
    let mut rejected = 0;
    for case in cases().iter().filter(|case| case["ok"] == false) {
        let name = case["name"].as_str().unwrap();
        let errors = compile(&case["document"], DiagramType::Tree).expect_err(name);
        assert_eq!(
            actual_codes(&errors),
            codes(case["diagnostics"].as_array().unwrap()),
            "{name}"
        );
        rejected += 1;
    }
    assert_eq!(rejected, 5);
}

fn example() -> CompiledDiagram {
    let document: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.tree.json"
    ))
    .unwrap();
    assert_eq!(document["meta"]["input_contract"], "phoenix-v1");
    compile(&document, DiagramType::Tree).unwrap()
}

#[test]
fn minimal_example_keeps_single_parent_facts_and_passes_showcase() {
    let scene = example();
    let DiagramFacts::Tree {
        root,
        nodes,
        direction,
    } = &scene.facts
    else {
        panic!("tree facts");
    };
    assert_eq!(direction, "down");
    assert!(
        nodes
            .iter()
            .filter(|node| node.get("parent").is_none())
            .count()
            == 1
    );
    assert_eq!(nodes[0]["id"], json!(root));
    assert_eq!(scene.relations.len(), nodes.len() - 1);
    let html = html(&scene);
    let check = check_html(html.as_bytes(), Quality::Showcase);
    assert_eq!(check.status, "pass", "{:?}", check.diagnostics);
    // Collapsed only marks the initial Reader state; the static SVG is whole.
    let document = Html::parse_document(&html);
    assert_eq!(
        document
            .select(&select("[data-tree-initially-collapsed]"))
            .count(),
        1
    );
    assert_eq!(
        document
            .select(&select("[data-tree-hidden], [data-tree-collapsed]"))
            .count(),
        0
    );
    assert_eq!(
        document.select(&select("g[data-node-id]")).count(),
        nodes.len()
    );
    // Print shows the whole tree even when no print event reaches the Reader.
    assert!(
        html.contains("@media print { svg[data-tree-ui] [data-tree-hidden] { display: inline; }")
    );
}

#[track_caller]
fn failing_codes(html: &str) -> Vec<String> {
    let check = check_html(html.as_bytes(), Quality::Showcase);
    assert_eq!(check.status, "failed");
    check.diagnostics.into_iter().map(|d| d.code).collect()
}

#[test]
fn artifact_check_rejects_lost_labels_edges_and_reader_hierarchy() {
    let scene = example();
    let html = html(&scene);
    let label = scene.nodes[1].label.clone();
    let first_line = &scene.nodes[1].text[0].content;
    let missing = html.replacen(&format!(">{first_line}</tspan>"), "></tspan>", 1);
    assert_ne!(missing, html);
    let codes = failing_codes(&missing);
    assert!(
        codes.contains(&"artifact/semantic-text".into()),
        "{label}: {codes:?}"
    );

    let start = html.find("<path data-edge-from=").unwrap();
    let end = start + html[start..].find("/>").unwrap() + 2;
    let no_edge = format!("{}{}", &html[..start], &html[end..]);
    assert!(failing_codes(&no_edge).contains(&"artifact/semantic-count".into()));

    let wrong_ancestors = html.replacen("data-tree-ancestors=\"", "data-tree-ancestors=\"x ", 1);
    assert!(failing_codes(&wrong_ancestors).contains(&"artifact/semantic-relation".into()));

    let collapsed_static = html.replacen(
        "data-tree-depth=\"1\"",
        "data-tree-depth=\"1\" data-tree-hidden=\"\"",
        1,
    );
    assert!(failing_codes(&collapsed_static).contains(&"artifact/semantic-count".into()));

    let lost_flag = html.replacen(" data-tree-initially-collapsed=\"\"", "", 1);
    assert!(failing_codes(&lost_flag).contains(&"artifact/semantic-relation".into()));

    let wrong_count = html.replacen("data-tree-count=\"", "data-tree-count=\"9", 1);
    assert!(failing_codes(&wrong_count).contains(&"artifact/semantic-count".into()));
}

#[test]
fn an_only_child_list_spine_stays_on_its_rounded_toggle_x() {
    let case = cases()
        .into_iter()
        .find(|case| case["name"] == "fractional-only-stack")
        .unwrap();
    let mut scene = compile(&case["document"], DiagramType::Tree).unwrap();
    let reference = &case["layout"]["edges"][1]["points"];
    let (toggle_x, spine_x) = (
        reference[0][0].as_f64().unwrap(),
        reference[1][0].as_f64().unwrap(),
    );
    assert!((toggle_x - spine_x).abs() == 0.5, "{reference}");
    assert_eq!(scene.relations[1].points[1].x, toggle_x);
    // The frozen geometry itself fails the orthogonal-route gate.
    for relation in scene.relations.iter_mut().skip(1) {
        relation.points[1].x = spine_x;
    }
    let check = check_html(html(&scene).as_bytes(), Quality::Showcase);
    assert!(check
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "composition/orthogonal-arrows"));
}
