use phoenix_diagram::{
    check::check_html,
    input::{DiagramType, InputContract, Quality},
    render::{render, EmbeddedAssets},
    scene::*,
};
use serde_json::json;

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><meta charset="utf-8"><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

fn authored(id: &str, kind: &str, facts: serde_json::Value) -> Authored {
    Authored {
        id: id.into(),
        source_path: format!("/components/{id}"),
        kind: kind.into(),
        facts,
        geometry_provenance: vec![],
    }
}

fn text(value: &str, x: f64, y: f64) -> Text {
    Text {
        source_path: "/components/0/label".into(),
        content: value.into(),
        position: Point { x, y },
        font_size: 11.0,
        anchor: "middle".into(),
    }
}

fn scene() -> CompiledDiagram {
    let nodes = vec![
        Node {
            authored: authored(
                "a",
                "frontend",
                json!({"id":"a","type":"frontend","label":"A"}),
            ),
            bounds: Rect {
                x: 10.0,
                y: 80.0,
                width: 120.0,
                height: 60.0,
            },
            sigil: None,
            label: "A".into(),
            text: vec![text("A", 70.0, 110.0)],
            detail_rows: Vec::new(),
        },
        Node {
            authored: authored(
                "b",
                "backend",
                json!({"id":"b","type":"backend","label":"B"}),
            ),
            bounds: Rect {
                x: 400.0,
                y: 80.0,
                width: 120.0,
                height: 60.0,
            },
            sigil: None,
            label: "B".into(),
            text: vec![text("B", 460.0, 110.0)],
            detail_rows: Vec::new(),
        },
    ];
    CompiledDiagram {
        diagram_type: DiagramType::Architecture,
        schema_version: 1,
        input_contract: InputContract::PhoenixV1,
        quality: Quality::Showcase,
        meta: json!({"title":"Example","subtitle":"Facts","locale":"en","output":"example.html"}),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        },
        reader_fit: "auto".into(),
        nodes,
        relations: vec![Relation {
            authored: authored(
                "request",
                "default",
                json!({"id":"request","from":"a","to":"b","label":"request","note":"asynchronous"}),
            ),
            crossover_halo: false,
            independent_ports: false,
            junction: None,
            label_owner: None,
            note_bounds: None,
            note_backdrop: None,
            from: "a".into(),
            to: "b".into(),
            variant: "default".into(),
            label: "request".into(),
            note: Some("asynchronous".into()),
            points: vec![Point { x: 130.0, y: 110.0 }, Point { x: 400.0, y: 110.0 }],
            label_plate: Some(Rect {
                x: 221.0,
                y: 84.0,
                width: 88.0,
                height: 35.0,
            }),
            text: vec![
                text("request", 265.0, 98.0),
                text("asynchronous", 265.0, 112.0),
            ],
            marker_start: None,
            marker_end: None,
        }],
        containers: vec![],
        text: vec![],
        legends: vec![],
        cards: vec![],
        facts: DiagramFacts::Architecture {
            layout: None,
            components: vec![],
            boundaries: vec![],
        },
        diagnostics: vec![],
    }
}

fn html(scene: &CompiledDiagram) -> Vec<u8> {
    render(scene, &EmbeddedAssets { template: TEMPLATE }).unwrap()
}

#[test]
fn text_attributes_and_script_json_are_separate_safe_literal_contexts() {
    let mut scene = scene();
    let literal = r#"$& </script><script>alert("x")</script> & '{{PHOENIX:SVG}}'"#;
    scene.meta["title"] = json!(literal);
    scene.nodes[0].label = literal.into();
    scene.nodes[0].text[0].content = literal.into();
    let original = scene.clone();
    let bytes = html(&scene);
    let rendered = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(scene, original);
    assert_eq!(bytes, html(&scene));
    assert!(!rendered.contains("<script>alert"));
    assert!(rendered.contains("&lt;/script&gt;"));
    assert!(rendered.contains("\\u003c/script\\u003e"));
    assert!(rendered.contains("{{PHOENIX:SVG}}"));
    assert!(rendered.contains("$&"));
}

#[test]
fn serializer_preserves_semantic_dom_and_native_coordinates() {
    let rendered = String::from_utf8(html(&scene())).unwrap();
    for expected in [
        "data-node-id=\"a\"",
        "data-node-id=\"b\"",
        "data-edge-from=\"a\"",
        "data-edge-to=\"b\"",
        "viewBox=\"0 0 800 600\"",
        "d=\"M 130 110 L 400 110\"",
        "request",
        "asynchronous",
    ] {
        assert!(rendered.contains(expected), "Missing {expected}");
    }
    assert!(!rendered.contains("{{PHOENIX:TITLE}}"));
}

#[test]
fn missing_duplicate_or_unknown_template_slots_fail_explicitly() {
    for template in [
        TEMPLATE.replace("{{PHOENIX:SVG}}", ""),
        format!("{TEMPLATE}{{{{PHOENIX:SVG}}}}"),
        TEMPLATE.replace("{{PHOENIX:SVG}}", "{{PHOENIX:UNKNOWN}}"),
    ] {
        let errors = render(
            &scene(),
            &EmbeddedAssets {
                template: &template,
            },
        )
        .unwrap_err();
        assert!(errors.iter().any(|error| error.code == "render/template"));
    }
}

#[test]
fn non_finite_resolved_geometry_cannot_be_serialized() {
    let mut scene = scene();
    scene.nodes[0].bounds.x = f64::INFINITY;
    let errors = render(&scene, &EmbeddedAssets { template: TEMPLATE }).unwrap_err();
    assert!(errors.iter().any(|error| error.code == "render/non-finite"));
}

#[test]
fn actual_html_is_checked_and_tampering_fails_independently_of_scene() {
    let bytes = html(&scene());
    let check = check_html(&bytes, Quality::Showcase);
    assert_eq!(check.status, "pass", "{check:?}");
    assert!(check.checks.len() >= 10);
    for (from, to) in [
        ("M 130 110 L 400 110", "M 130 110 L 400 150"),
        ("data-edge-to=\"b\"", "data-edge-to=\"missing\""),
        ("viewBox=\"0 0 800 600\"", "viewBox=\"0 0 NaN 600\""),
        ("M 130 110 L 400 110", "M 130 110 C 200 100 300 100 400 110"),
    ] {
        let changed = String::from_utf8(bytes.clone()).unwrap().replace(from, to);
        assert_ne!(changed.as_bytes(), bytes, "Mutation must hit actual output");
        let result = check_html(changed.as_bytes(), Quality::Showcase);
        assert_eq!(result.status, "failed", "{from} => {to}: {result:?}");
        assert!(!result.diagnostics.is_empty());
    }
}

#[test]
fn automatic_lifecycle_waiting_self_loop_passes_showcase_rhythm() {
    use phoenix_diagram::{
        input::{parse_input, CompileOptions},
        layout,
    };

    let document = json!({
        "schema_version": 3,
        "diagram_type": "lifecycle",
        "meta": {
            "title": "Waiting self-loop regression",
            "output": "minimal.html",
            "input_contract": "phoenix-v1",
            "quality_profile": "showcase",
            "locale": "en"
        },
        "mainPath": ["created", "waiting", "succeeded"],
        "states": [
            {"id": "created", "type": "start", "label": "Created"},
            {"id": "waiting", "type": "waiting", "label": "Waiting"},
            {"id": "succeeded", "type": "success", "label": "Succeeded"}
        ],
        "transitions": [
            {"id": "enqueue", "from": "created", "to": "waiting", "label": "queued"},
            {"id": "complete", "from": "waiting", "to": "succeeded", "label": "complete"},
            {"id": "stay", "from": "waiting", "to": "waiting", "label": "resources unavailable"}
        ]
    });
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        quality: Some(Quality::Showcase),
        ..Default::default()
    };
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Lifecycle,
        &options,
    )
    .unwrap();
    let scene = layout::compile(&input, &options).unwrap();
    assert_eq!(scene.nodes.len(), 3);
    assert_eq!(scene.relations.len(), 3);
    let self_loop = scene
        .relations
        .iter()
        .find(|relation| relation.from == "waiting" && relation.to == "waiting")
        .unwrap();
    assert!(self_loop
        .points
        .windows(2)
        .all(|pair| { (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y) >= 8.0 }));

    let original = scene.clone();
    let rendered = render(&scene, &EmbeddedAssets::builtin()).unwrap();
    assert_eq!(scene, original);
    let checked = check_html(&rendered, Quality::Showcase);
    assert_eq!(
        checked.composition["metrics"]["route_rhythm"], 0,
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);

    let document = scraper::Html::parse_document(std::str::from_utf8(&rendered).unwrap());
    let selector = scraper::Selector::parse(
        "path[data-graph-role='edge'][data-edge-from='waiting'][data-edge-to='waiting']",
    )
    .unwrap();
    let path = document.select(&selector).next().unwrap();
    let analysis =
        phoenix_diagram::check::path::analyze_svg_path(path.value().attr("d").unwrap()).unwrap();
    let horizontal_runs: Vec<_> = analysis
        .subpaths
        .iter()
        .flat_map(|part| &part.straight_segments)
        .filter(|segment| (segment.start.y - segment.end.y).abs() < 0.01)
        .collect();
    assert_eq!(
        horizontal_runs.len(),
        1,
        "The self-loop top run must remain"
    );
    assert!((horizontal_runs[0].end.x - horizontal_runs[0].start.x).abs() >= 8.0);

    let tampered_path = phoenix_diagram::geometry::rounded_path(&self_loop.points, 8.0);
    let target = format!(
        "data-graph-role=\"edge\" data-composition-points=\"{}\" d=\"",
        path.value().attr("data-composition-points").unwrap()
    );
    let original_path = format!("{target}{}\"", path.value().attr("d").unwrap());
    let original_html = std::str::from_utf8(&rendered).unwrap();
    assert_eq!(original_html.matches(&original_path).count(), 1);
    let tampered = original_html.replacen(&original_path, &format!("{target}{tampered_path}\""), 1);
    assert_ne!(tampered.as_bytes(), rendered);
    let checked = check_html(tampered.as_bytes(), Quality::Showcase);
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "artifact/path-binding"),
        "{:?}",
        checked.diagnostics
    );

    let mut spacious = scene.clone();
    let spacious_loop = spacious
        .relations
        .iter_mut()
        .find(|relation| relation.from == "waiting" && relation.to == "waiting")
        .unwrap();
    for (index, point) in spacious_loop.points.iter_mut().enumerate() {
        point.x += if index < 2 { -11.0 } else { 11.0 };
    }
    let expected = phoenix_diagram::geometry::rounded_path(&spacious_loop.points, 12.0);
    let rendered = render(&spacious, &EmbeddedAssets::builtin()).unwrap();
    let checked = check_html(&rendered, Quality::Showcase);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
    let document = scraper::Html::parse_document(std::str::from_utf8(&rendered).unwrap());
    let path = document.select(&selector).next().unwrap();
    assert_eq!(path.value().attr("d").unwrap(), expected);
}

#[test]
fn body_only_transform_is_a_semantic_geometry_failure() {
    let original = String::from_utf8(html(&scene())).unwrap();
    let altered = original.replacen(
        "data-graph-role=\"node-body\"",
        "data-graph-role=\"node-body\" transform=\"translate(20 0)\"",
        1,
    );
    assert_ne!(original, altered);
    let result = check_html(altered.as_bytes(), Quality::Showcase);
    assert!(
        result
            .checks
            .iter()
            .any(|entry| entry.name == "semantic_transforms" && entry.status == "failed"),
        "{result:?}"
    );
}

#[test]
fn known_endpoints_and_visible_variants_must_match_the_authored_relation() {
    let original = String::from_utf8(html(&scene())).unwrap();
    for altered in [
        original.replace("data-edge-from=\"a\"", "data-edge-from=\"b\""),
        original.replace("class=\"a-default\"", "class=\"a-security\""),
    ] {
        assert_ne!(original, altered);
        let result = check_html(altered.as_bytes(), Quality::Showcase);
        assert_eq!(result.status, "failed", "{result:?}");
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "artifact/semantic-relation"));
    }
}

#[test]
fn interior_route_changes_cannot_keep_the_original_geometry_declaration() {
    let original = String::from_utf8(html(&scene())).unwrap();
    let altered = original.replace(
        "M 130 110 L 400 110",
        "M 130 110 L 200 110 L 200 200 L 300 200 L 300 110 L 400 110",
    );
    assert_ne!(original, altered);
    let result = check_html(altered.as_bytes(), Quality::Showcase);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "artifact/path-binding"),
        "{result:?}"
    );
}

#[test]
fn legend_clearance_uses_actual_swatch_geometry() {
    let mut scene = scene();
    scene.legends.push(Card {
        authored: authored("legend-backend", "backend", json!({"type":"backend"})),
        bounds: Rect {
            x: 200.0,
            y: 490.0,
            width: 100.0,
            height: 14.0,
        },
        text: vec![text("Backend", 222.0, 500.0)],
    });
    let original = String::from_utf8(html(&scene)).unwrap();
    assert_eq!(
        check_html(original.as_bytes(), Quality::Showcase).status,
        "pass"
    );
    let altered = original.replace(
        "x=\"200\" y=\"491\" width=\"16\"",
        "x=\"200\" y=\"105\" width=\"16\"",
    );
    assert_ne!(original, altered);
    let result = check_html(altered.as_bytes(), Quality::Showcase);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "composition/legend-clearance"),
        "{result:?}"
    );
}

#[test]
fn workflow_phase_hairlines_are_reading_guides_rather_than_structural_obstacles() {
    let mut scene = scene();
    scene.diagram_type = DiagramType::Workflow;
    scene.schema_version = 2;
    scene.relations[0].points = vec![
        Point { x: 130.0, y: 110.0 },
        Point { x: 200.0, y: 110.0 },
        Point { x: 200.0, y: 40.0 },
        Point { x: 300.0, y: 40.0 },
        Point { x: 300.0, y: 110.0 },
        Point { x: 400.0, y: 110.0 },
    ];
    scene.containers.push(Container {
        authored: authored("phase", "phase", json!({"id":"phase","label":"Phase"})),
        bounds: Rect {
            x: 90.0,
            y: 35.0,
            width: 510.0,
            height: 10.0,
        },
        rule: Some([Point { x: 90.0, y: 40.0 }, Point { x: 600.0, y: 40.0 }]),
        label_plate: None,
        members: vec![],
        text: vec![],
    });
    let checked = check_html(&html(&scene), Quality::Showcase);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
    assert_eq!(
        checked
            .checks
            .iter()
            .find(|c| c.name == "container_border_runs")
            .unwrap()
            .scope["frames"],
        0
    );
}

#[test]
fn authored_cards_cannot_disappear_while_the_graph_still_passes() {
    let mut scene = scene();
    scene.cards.push(Card {
        authored: authored(
            "fact",
            "card",
            json!({"dot":"blue","title":"Evidence","items":["Keep this condition"]}),
        ),
        bounds: Rect::default(),
        text: vec![],
    });
    let original = String::from_utf8(html(&scene)).unwrap();
    assert_eq!(
        check_html(original.as_bytes(), Quality::Showcase).status,
        "pass"
    );
    let altered = original.replace("<li>Keep this condition</li>", "<li></li>");
    assert_ne!(original, altered);
    let result = check_html(altered.as_bytes(), Quality::Showcase);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "artifact/semantic-card"),
        "{result:?}"
    );
}

#[test]
fn a_crossover_requires_both_real_bound_underlays_not_only_flags() {
    let mut scene = scene();
    scene.nodes.clear();
    scene.relations.clear();
    for (id, x, y) in [
        ("a", 10.0, 250.0),
        ("b", 650.0, 250.0),
        ("c", 350.0, 40.0),
        ("d", 350.0, 450.0),
    ] {
        scene.nodes.push(Node {
            authored: authored(
                id,
                "frontend",
                json!({"id":id,"type":"frontend","label":id}),
            ),
            bounds: Rect {
                x,
                y,
                width: 80.0,
                height: 40.0,
            },
            sigil: None,
            label: id.into(),
            text: vec![text(id, x + 40.0, y + 24.0)],
            detail_rows: Vec::new(),
        });
    }
    for (id, from, to, points) in [
        (
            "horizontal",
            "a",
            "b",
            vec![Point { x: 90.0, y: 270.0 }, Point { x: 650.0, y: 270.0 }],
        ),
        (
            "vertical",
            "c",
            "d",
            vec![Point { x: 390.0, y: 80.0 }, Point { x: 390.0, y: 450.0 }],
        ),
    ] {
        scene.relations.push(Relation {
            authored: authored(id, "default", json!({"id":id,"from":from,"to":to})),
            crossover_halo: true,
            independent_ports: true,
            junction: None,
            label_owner: None,
            note_bounds: None,
            note_backdrop: None,
            from: from.into(),
            to: to.into(),
            variant: "default".into(),
            label: String::new(),
            note: None,
            points,
            label_plate: None,
            text: vec![],
            marker_start: None,
            marker_end: None,
        });
    }
    let rendered = String::from_utf8(html(&scene)).unwrap();
    let checked = check_html(rendered.as_bytes(), Quality::Showcase);
    assert_eq!(checked.status, "pass", "{checked:?}");
    assert_eq!(checked.composition["metrics"]["resolvedCrossovers"], 1);
    for altered in [
        rendered.replacen("stroke-width=\"5.5\"", "stroke-width=\"2\"", 1),
        rendered.replacen(
            "data-graph-role=\"automatic-crossover-underlay\"",
            "data-graph-role=\"unrelated\"",
            1,
        ),
    ] {
        assert_ne!(rendered, altered);
        let check = check_html(altered.as_bytes(), Quality::Showcase);
        assert_eq!(check.status, "failed", "{check:?}");
        assert!(check
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "composition/proper-crossing"));
    }
}

#[test]
fn legacy_sigils_clear_their_labels_and_a_touching_sigil_fails() {
    use phoenix_diagram::{
        geometry::{rects_overlap, text_glyph_box},
        input::{parse_input, CompileOptions},
        layout,
    };

    for (kind, file) in [
        (
            DiagramType::Sequence,
            include_str!("../upstream/legacy-2.17/examples/async-job-roundtrip.sequence.json"),
        ),
        (
            DiagramType::Dataflow,
            include_str!("../upstream/legacy-2.17/examples/product-analytics.dataflow.json"),
        ),
    ] {
        let options = CompileOptions {
            input_contract: Some(InputContract::Legacy217),
            ..Default::default()
        };
        let input = parse_input(file.as_bytes(), kind, &options).unwrap();
        let scene = layout::compile(&input, &options).unwrap();
        let mut moved = 0;
        for node in &scene.nodes {
            let Some(sigil) = &node.sigil else { continue };
            let mark = Rect {
                x: sigil.position.x,
                y: sigil.position.y,
                width: sigil.size,
                height: sigil.size,
            };
            for text in node
                .text
                .iter()
                .filter(|text| text.source_path.ends_with("/label"))
            {
                let label = text_glyph_box(
                    &text.content,
                    text.position.x,
                    text.position.y,
                    text.font_size,
                    &text.anchor,
                    InputContract::Legacy217,
                );
                assert!(!rects_overlap(&mark, &label, 1.0), "{}", node.authored.id);
            }
            moved += usize::from(sigil.position.y == node.bounds.y + 3.0);
        }
        assert!(moved > 0, "{kind:?} reference has a touching sigil to move");
        let rendered = render(&scene, &EmbeddedAssets::builtin()).unwrap();
        let checked = check_html(&rendered, scene.quality);
        assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);

        // Restoring one frozen corner position makes the sigil touch its label again.
        let node = scene
            .nodes
            .iter()
            .find(|node| {
                node.sigil
                    .as_ref()
                    .is_some_and(|sigil| sigil.position.y == node.bounds.y + 3.0)
            })
            .unwrap();
        let sigil = node.sigil.as_ref().unwrap();
        let html = String::from_utf8(rendered).unwrap();
        let current = format!(
            "translate({} {}) scale({})",
            phoenix_diagram::geometry::format_number(sigil.position.x),
            phoenix_diagram::geometry::format_number(sigil.position.y),
            phoenix_diagram::geometry::format_number(sigil.size / 16.0)
        );
        let frozen = format!(
            "translate({} {}) scale(0.6875)",
            phoenix_diagram::geometry::format_number(sigil.position.x),
            phoenix_diagram::geometry::format_number(node.bounds.y + 6.0)
        );
        assert_eq!(html.matches(&current).count(), 1, "{current}");
        let tampered = html.replace(&current, &frozen);
        let checked = check_html(tampered.as_bytes(), scene.quality);
        let entry = checked
            .checks
            .iter()
            .find(|entry| entry.name == "node_geometry")
            .unwrap();
        assert_eq!(entry.status, "failed");
        assert!(checked
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "artifact/sigil-label-overlap"));
    }
}

#[test]
fn reference_dataflow_joins_keep_the_frozen_arrowhead_scope() {
    use phoenix_diagram::{
        input::{parse_input, CompileOptions},
        layout,
    };

    // Both 3.1.0 examples converge two flows on one port: auto-routed in
    // event-stream, author-pinned in product-analytics. The frozen check
    // treats such shared ports as joins.
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        ..Default::default()
    };
    for file in [
        include_str!("fixtures/input/phoenix-v1/event-stream.dataflow.json"),
        include_str!("fixtures/input/phoenix-v1/product-analytics.dataflow.json"),
    ] {
        let input = parse_input(file.as_bytes(), DiagramType::Dataflow, &options).unwrap();
        let scene = layout::compile(&input, &options).unwrap();
        let rendered = render(&scene, &EmbeddedAssets::builtin()).unwrap();
        let checked = check_html(&rendered, scene.quality);
        assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
    }

    // The same coincident tips fail once the pair claims independent ports.
    let file = include_str!("fixtures/input/phoenix-v1/event-stream.dataflow.json");
    let input = parse_input(file.as_bytes(), DiagramType::Dataflow, &options).unwrap();
    let mut scene = layout::compile(&input, &options).unwrap();
    for relation in scene
        .relations
        .iter_mut()
        .filter(|relation| relation.to == "state")
    {
        relation.crossover_halo = true;
        relation.independent_ports = true;
    }
    let rendered = render(&scene, &EmbeddedAssets::builtin()).unwrap();
    let checked = check_html(&rendered, scene.quality);
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "composition/arrowhead-collision"),
        "{:?}",
        checked.diagnostics
    );
}
