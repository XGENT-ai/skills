//! D25: an authored horizontal channel route leaves and enters its nodes
//! through the face toward the channel, so the final segment is
//! perpendicular to the target face and the arrowhead points into the node.
use phoenix_diagram::{
    check::{check_html, path::analyze_svg_path},
    input::{parse_input, CompileOptions, DiagramType, InputContract},
    layout,
    render::{render, EmbeddedAssets},
    scene::{Point, Rect},
};
use scraper::{ElementRef, Html, Selector};
use serde_json::json;

const EPSILON: f64 = 1e-6;
// Same corner inset the dataflow router uses for clean face attachment.
const CORNER_INSET: f64 = 16.0;

fn selector(value: &str) -> Selector {
    Selector::parse(value).unwrap()
}
fn numeric(element: ElementRef<'_>, attr: &str) -> f64 {
    element.value().attr(attr).unwrap().parse().unwrap()
}
fn rect(element: ElementRef<'_>) -> Rect {
    Rect {
        x: numeric(element, "x"),
        y: numeric(element, "y"),
        width: numeric(element, "width"),
        height: numeric(element, "height"),
    }
}
fn first_child_rect<'a>(group: ElementRef<'a>, class: &str) -> ElementRef<'a> {
    group
        .children()
        .filter_map(ElementRef::wrap)
        .find(|child| child.value().name() == "rect" && child.value().attr("class") == Some(class))
        .unwrap()
}

struct Edge {
    from: String,
    to: String,
    label: String,
    points: Vec<Point>,
    plate: Option<Rect>,
}
struct Rendered {
    nodes: Vec<(String, Rect)>,
    edges: Vec<Edge>,
}

/// Compile, render and check the actual HTML, then read the drawn geometry.
fn rendered(source: &[u8], contract: InputContract) -> Rendered {
    let options = CompileOptions {
        input_contract: Some(contract),
        ..Default::default()
    };
    let input = parse_input(source, DiagramType::Dataflow, &options).unwrap();
    let scene = layout::compile(&input, &options).unwrap();
    let bytes = render(&scene, &EmbeddedAssets::builtin()).unwrap();
    let checked = check_html(&bytes, scene.quality);
    assert_eq!(checked.status, "pass", "{:?}", checked.diagnostics);
    let html = Html::parse_document(std::str::from_utf8(&bytes).unwrap());
    let nodes = html
        .select(&selector("g[data-node-id]"))
        .map(|group| {
            (
                group.value().attr("data-node-id").unwrap().to_owned(),
                rect(first_child_rect(group, "c-mask")),
            )
        })
        .collect();
    let edges = html
        .select(&selector("path[data-graph-role='edge'][data-edge-from]"))
        .map(|edge| {
            let value = edge.value();
            assert!(
                value.attr("marker-end").is_some(),
                "every flow keeps its arrowhead"
            );
            let analysis = analyze_svg_path(value.attr("d").unwrap()).unwrap();
            assert_eq!(analysis.subpaths.len(), 1);
            let key = value.attr("data-edge-key").unwrap();
            let plate = html
                .select(&selector(&format!(
                    "g[data-detail='context'][data-edge-key='{key}']"
                )))
                .next()
                .map(|group| rect(group.select(&selector("rect")).next().unwrap()));
            Edge {
                from: value.attr("data-edge-from").unwrap().into(),
                to: value.attr("data-edge-to").unwrap().into(),
                label: value.attr("data-edge-label").unwrap().into(),
                points: analysis.subpaths[0].points.clone(),
                plate,
            }
        })
        .collect();
    Rendered { nodes, edges }
}

fn node(rendered: &Rendered, id: &str) -> Rect {
    rendered
        .nodes
        .iter()
        .find(|(node, _)| node == id)
        .unwrap()
        .1
}

/// A segment lying on a node border for a positive length draws the stroke
/// (and possibly the arrowhead) along that border instead of across it.
fn border_runs(points: &[Point], rect: &Rect) -> bool {
    let (left, right) = (rect.x, rect.x + rect.width);
    let (top, bottom) = (rect.y, rect.y + rect.height);
    let overlap = |a: f64, b: f64, low: f64, high: f64| a.max(b).min(high) - a.min(b).max(low);
    points.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        let vertical = (a.x - b.x).abs() < EPSILON;
        let horizontal = (a.y - b.y).abs() < EPSILON;
        (vertical
            && [left, right].iter().any(|x| (a.x - x).abs() < EPSILON)
            && overlap(a.y, b.y, top, bottom) > EPSILON)
            || (horizontal
                && [top, bottom].iter().any(|y| (a.y - y).abs() < EPSILON)
                && overlap(a.x, b.x, left, right) > EPSILON)
    })
}

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// `face` is "bottom" or "top": the route must leave `from` and enter `to`
/// perpendicularly through that face, clear of the corners.
fn assert_channel_attached(rendered: &Rendered, label: &str, face: &str, context: &str) {
    let edge = rendered
        .edges
        .iter()
        .find(|edge| edge.label == label)
        .unwrap_or_else(|| panic!("{context}: {label} flow must be rendered"));
    let points = &edge.points;
    assert!(points.len() >= 2, "{context}: {points:?}");
    // +1 when the outward normal of the face points down the page.
    let outward = if face == "bottom" { 1.0 } else { -1.0 };
    let face_y = |rect: &Rect| {
        if face == "bottom" {
            rect.y + rect.height
        } else {
            rect.y
        }
    };
    for (id, at, beyond, role) in [
        (&edge.from, points[0], points[1], "leave"),
        (
            &edge.to,
            points[points.len() - 1],
            points[points.len() - 2],
            "enter",
        ),
    ] {
        let bounds = node(rendered, id);
        assert!(
            (at.x - beyond.x).abs() < EPSILON && (beyond.y - at.y) * outward > EPSILON,
            "{context}: {label} must {role} {id} perpendicular to its {face} face: {points:?}"
        );
        assert!(
            (at.y - face_y(&bounds)).abs() < EPSILON,
            "{context}: {label} must {role} {id} on its {face} face {bounds:?}: {points:?}"
        );
        assert!(
            at.x > bounds.x + CORNER_INSET && at.x < bounds.x + bounds.width - CORNER_INSET,
            "{context}: {label} must {role} {id} inside its {face} span, not at a corner/border {bounds:?}: {points:?}"
        );
    }

    // No segment of any flow runs collinear with any node border.
    for edge in &rendered.edges {
        for (id, bounds) in &rendered.nodes {
            assert!(
                !border_runs(&edge.points, bounds),
                "{context}: {}->{} {} runs along the border of {id} {bounds:?}: {:?}",
                edge.from,
                edge.to,
                edge.label,
                edge.points
            );
        }
    }

    // The authored label stays on the channel and off every node.
    let plate = edge
        .plate
        .unwrap_or_else(|| panic!("{context}: {label} label must render"));
    for (id, bounds) in &rendered.nodes {
        assert!(
            !overlaps(&plate, bounds),
            "{context}: {label} label {plate:?} covers {id} {bounds:?}"
        );
    }
}

#[test]
fn reference_bottom_channel_enters_the_target_through_its_bottom_face() {
    for (contract, source) in [
        (
            InputContract::PhoenixV1,
            include_str!("fixtures/input/phoenix-v1/event-stream.dataflow.json"),
        ),
        (
            InputContract::Legacy217,
            include_str!("fixtures/input/legacy-2.17/event-stream.dataflow.json"),
        ),
    ] {
        let rendered = rendered(source.as_bytes(), contract);
        assert_channel_attached(
            &rendered,
            "poison event",
            "bottom",
            &format!("{contract:?}/event-stream"),
        );
    }
}

#[test]
fn inferred_channel_sides_face_the_authored_channel_row() {
    // Side-by-side nodes: lateral sides would put both risers on borders.
    for (route, face, channel) in [
        ("bottom-channel", "bottom", 220),
        ("top-channel", "top", 104),
    ] {
        for channel_y in [None, Some(channel)] {
            let mut flow =
                json!({"id": "ab", "from": "a", "to": "b", "label": "payload", "route": route});
            if let Some(y) = channel_y {
                flow["channelY"] = json!(y);
            }
            let document = json!({
                "schema_version": 1,
                "diagram_type": "dataflow",
                "meta": {"title": "Channel", "output": "channel.html", "viewBox": [940, 720], "quality_profile": "standard"},
                "stages": [{"label": "Source"}, {"label": "Sink"}],
                "nodes": [
                    {"id": "a", "type": "frontend", "label": "Input", "stage": 0, "row": 0},
                    {"id": "b", "type": "backend", "label": "Output", "stage": 1, "row": 0}
                ],
                "flows": [flow]
            });
            for contract in [InputContract::PhoenixV1, InputContract::Legacy217] {
                let rendered = rendered(&serde_json::to_vec(&document).unwrap(), contract);
                let context = format!("{contract:?}/{route}/channelY={channel_y:?}");
                assert_channel_attached(&rendered, "payload", face, &context);
                if let Some(y) = channel_y {
                    let points = &rendered.edges[0].points;
                    assert!(
                        points[1..points.len() - 1]
                            .iter()
                            .all(|point| (point.y - y as f64).abs() < EPSILON),
                        "{context}: authored channelY must stay the channel row: {points:?}"
                    );
                }
            }
        }
    }
}
