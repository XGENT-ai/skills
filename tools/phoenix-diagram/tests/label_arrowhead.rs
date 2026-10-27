//! D26: an automatically placed relationship label never covers an arrowhead,
//! so every drawn route keeps a readable direction. Author-pinned labels keep
//! their positions.
use phoenix_diagram::{
    check::{check_html, path::analyze_svg_path},
    geometry::composition::{collect_label_arrowhead_overlaps, LabelRect, RoutedRelation},
    input::{parse_input, CompileOptions, DiagramType, InputContract},
    layout,
    render::{render, EmbeddedAssets},
    scene::{Point, Rect},
};
use scraper::{ElementRef, Html, Selector};
use serde_json::{json, Value};

fn selector(value: &str) -> Selector {
    Selector::parse(value).unwrap()
}
fn numeric(element: ElementRef<'_>, attr: &str) -> f64 {
    element.value().attr(attr).unwrap().parse().unwrap()
}

struct Drawn {
    id: String,
    points: Vec<Point>,
    stroke: f64,
    plate: Option<Rect>,
}

/// Compile, render and check the actual HTML, then read the drawn routes and
/// label plates.
fn drawn(source: &[u8], kind: DiagramType) -> Vec<Drawn> {
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        ..Default::default()
    };
    let input = parse_input(source, kind, &options).unwrap();
    let scene = layout::compile(&input, &options).unwrap();
    let bytes = render(&scene, &EmbeddedAssets::builtin()).unwrap();
    let checked = check_html(&bytes, scene.quality);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
    let html = Html::parse_document(std::str::from_utf8(&bytes).unwrap());
    html.select(&selector("path[data-graph-role='edge'][marker-end]"))
        .map(|edge| {
            let value = edge.value();
            let analysis = analyze_svg_path(value.attr("d").unwrap()).unwrap();
            let key = value.attr("data-edge-key").unwrap();
            let plate = html
                .select(&selector(&format!(
                    "g[data-detail='context'][data-edge-key='{key}'] > rect.c-mask"
                )))
                .next()
                .map(|plate| Rect {
                    x: numeric(plate, "x"),
                    y: numeric(plate, "y"),
                    width: numeric(plate, "width"),
                    height: numeric(plate, "height"),
                });
            Drawn {
                id: value.attr("data-edge-id").unwrap().into(),
                points: analysis.subpaths.last().unwrap().points.clone(),
                stroke: numeric(edge, "stroke-width"),
                plate,
            }
        })
        .collect()
}

/// Tip and centroid of the rendered marker (markerWidth 10, markerHeight 7,
/// refX 9) scaled by the stroke width: the tip one width beyond the route
/// end, the base nine back and 3.5 to each side.
fn arrowhead(points: &[Point], stroke: f64) -> [Point; 2] {
    let end = points[points.len() - 1];
    let previous = points
        .iter()
        .rev()
        .skip(1)
        .find(|point| (point.x - end.x).abs() + (point.y - end.y).abs() > 1e-6)
        .unwrap();
    let length = ((end.x - previous.x).powi(2) + (end.y - previous.y).powi(2)).sqrt();
    let (dx, dy) = ((end.x - previous.x) / length, (end.y - previous.y) / length);
    let corners = [(1.0, 0.0), (-9.0, 3.5), (-9.0, -3.5)].map(|(u, v): (f64, f64)| Point {
        x: end.x + (u * dx - v * dy) * stroke,
        y: end.y + (u * dy + v * dx) * stroke,
    });
    let centroid = Point {
        x: corners.iter().map(|p| p.x).sum::<f64>() / 3.0,
        y: corners.iter().map(|p| p.y).sum::<f64>() / 3.0,
    };
    [corners[0], centroid]
}

fn inside(point: Point, rect: &Rect) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

/// No plate hides an arrowhead's tip or the centre of its body.
fn assert_clear(edges: &[Drawn]) {
    assert!(edges.iter().any(|edge| edge.plate.is_some()));
    for label in edges.iter().filter(|edge| edge.plate.is_some()) {
        let plate = label.plate.unwrap();
        for arrow in edges {
            let [tip, centroid] = arrowhead(&arrow.points, arrow.stroke);
            assert!(
                !inside(tip, &plate) && !inside(centroid, &plate),
                "label of {} covers the arrowhead of {}: plate {plate:?}, tip {tip:?}, centroid {centroid:?}",
                label.id,
                arrow.id
            );
        }
    }
}

#[test]
fn automatic_workflow_return_labels_leave_every_arrowhead_visible() {
    // Original v5 author candidate whose E-D label covered its own arrowhead.
    let edges = drawn(
        include_bytes!("fixtures/workflow/retry-label-arrowhead.workflow.json"),
        DiagramType::Workflow,
    );
    assert_eq!(edges.len(), 7);
    assert_clear(&edges);
}

#[test]
fn automatic_dataflow_labels_on_upward_flows_leave_the_arrowhead_visible() {
    // Original v5 author candidate: two upward flows labelled toward their tips.
    let edges = drawn(
        include_bytes!("fixtures/dataflow/upward-label-arrowhead.dataflow.json"),
        DiagramType::Dataflow,
    );
    assert_eq!(edges.len(), 9);
    assert_clear(&edges);
}

#[test]
fn label_arrowhead_overlaps_include_the_label_own_route() {
    let route = |index: usize, points: [(f64, f64); 2]| {
        RoutedRelation::new(
            index,
            json!({"from":"a","to":"b"}),
            points.iter().map(|(x, y)| Point { x: *x, y: *y }).collect(),
        )
    };
    let routed = [
        route(0, [(100.0, 200.0), (100.0, 100.0)]),
        route(1, [(0.0, 0.0), (40.0, 0.0)]),
    ];
    let label = |rect: Rect| LabelRect::new(0, json!({"from":"a","to":"b","label":"retry"}), rect);
    let covering = label(Rect {
        x: 80.0,
        y: 101.0,
        width: 40.0,
        height: 14.0,
    });
    let clear = label(Rect {
        x: 80.0,
        y: 140.0,
        width: 40.0,
        height: 14.0,
    });
    // Grazes only the base corner of the upward arrowhead (base at y 112.6).
    let grazing = label(Rect {
        x: 60.0,
        y: 111.0,
        width: 36.0,
        height: 14.0,
    });
    let hits = collect_label_arrowhead_overlaps(&[covering, clear, grazing], &routed);
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].label_index, hits[0].arrow_relation_index), (0, 0));
    let value: Value = serde_json::to_value(&hits[0]).unwrap();
    assert!(value["arrowhead"]["height"].as_f64().unwrap() > 12.0);
}
