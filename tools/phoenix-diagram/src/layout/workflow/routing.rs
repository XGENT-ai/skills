use super::measure::{Measurement, LEGACY_COLUMNS};
use super::*;
use crate::geometry::composition::{
    collect_ambiguous_corridors, collect_border_runs, collect_proper_crossings, CorridorOptions,
    CrossingOptions, Frame, RoutedRelation,
};
use crate::geometry::{
    anchor, label_point, normalize_route_points, polyline_length, proper_segment_intersection,
    rects_overlap_for_contract, segment_intersects_rect, segment_rect_clearance,
};
use crate::scene::{Node, Relation};
use std::collections::{BTreeMap, BTreeSet};

#[path = "routing_repair.rs"]
mod repairs;

const SIDES: [&str; 4] = ["right", "bottom", "left", "top"];
#[derive(Clone)]
struct Candidate {
    points: Vec<Point>,
    from_side: String,
    to_side: String,
    cost: Vec<f64>,
}
struct Router<'a> {
    input: &'a DiagramInput,
    measured: &'a Measurement,
    edges: &'a [Entry],
    placed: Vec<Relation>,
    feedback: &'a Feedback,
}

#[derive(Clone, Default, PartialEq)]
pub(super) struct Feedback {
    pub route_first: Vec<usize>,
    pub fan_in_label_clearance: bool,
    // Repair passes only, keyed by source edge index. `sides` are the node
    // sides an earlier attempt actually used, so ports are spread among the
    // edges that really meet on a side; `halo` names automatic edges whose
    // remaining crossings are drawn as crossover halos.
    pub sides: BTreeMap<usize, (String, String)>,
    pub halo: BTreeSet<usize>,
    // Repair passes only: outside corridors also get parallel lanes, so
    // routes that share a detour can run side by side.
    pub channels: bool,
    // Repair passes only: the side pair an automatic edge must use.
    pub force: BTreeMap<usize, (String, String)>,
}

fn center(node: &Node) -> Point {
    Point {
        x: node.bounds.x + node.bounds.width / 2.0,
        y: node.bounds.y + node.bounds.height / 2.0,
    }
}
fn natural_sides(from: &Node, to: &Node) -> (&'static str, &'static str) {
    let a = center(from);
    let b = center(to);
    if b.x < a.x {
        ("left", "right")
    } else if b.x > a.x {
        ("right", "left")
    } else if b.y > a.y {
        ("bottom", "top")
    } else {
        ("top", "bottom")
    }
}
fn absolute_route(edge: &Value) -> bool {
    ["via", "channelX", "channelY"]
        .iter()
        .any(|field| edge.get(field).is_some())
}
fn automatic(edge: &Value) -> bool {
    !absolute_route(edge) && ["", "auto"].contains(&string(edge, "route"))
}
fn independent(edge: &Value) -> bool {
    automatic(edge)
        && edge.get("labelAt").is_none()
        && ["", "auto"].contains(&string(edge, "fromSide"))
        && ["", "auto"].contains(&string(edge, "toSide"))
}
fn point(value: &Value) -> Point {
    Point {
        x: value[0].as_f64().unwrap(),
        y: value[1].as_f64().unwrap(),
    }
}
fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
fn visible_route(points: &[Point]) -> Vec<Point> {
    crate::check::path::analyze_svg_path(&crate::geometry::workflow_v2_path(points))
        .expect("Finite workflow geometry generates a supported SVG path")
        .subpaths
        .remove(0)
        .points
}
fn stub(point: Point, side: &str) -> Point {
    match side {
        "left" => Point {
            x: point.x - 16.0,
            ..point
        },
        "right" => Point {
            x: point.x + 16.0,
            ..point
        },
        "top" => Point {
            y: point.y - 16.0,
            ..point
        },
        _ => Point {
            y: point.y + 16.0,
            ..point
        },
    }
}
fn corridor_y(start: Point, end: Point, from: &str, to: &str, y: f64) -> Vec<Point> {
    let a = stub(start, from);
    let b = stub(end, to);
    vec![a, Point { x: a.x, y }, Point { x: b.x, y }, b]
}
fn corridor_x(start: Point, end: Point, from: &str, to: &str, x: f64) -> Vec<Point> {
    let a = stub(start, from);
    let b = stub(end, to);
    vec![a, Point { x, y: a.y }, Point { x, y: b.y }, b]
}
fn join(start: Point, via: Vec<Point>, end: Point) -> Vec<Point> {
    normalize_route_points(
        &std::iter::once(start)
            .chain(via)
            .chain(std::iter::once(end))
            .collect::<Vec<_>>(),
    )
}
fn orthogonal(points: &[Point]) -> bool {
    points.windows(2).all(|pair| {
        ((pair[0].x - pair[1].x).abs() <= 0.0001) != ((pair[0].y - pair[1].y).abs() <= 0.0001)
    })
}
fn direction(a: Point, b: Point, side: &str) -> bool {
    match side {
        "left" => (b.y - a.y).abs() <= 0.0001 && b.x < a.x - 0.0001,
        "right" => (b.y - a.y).abs() <= 0.0001 && b.x > a.x + 0.0001,
        "top" => (b.x - a.x).abs() <= 0.0001 && b.y < a.y - 0.0001,
        _ => (b.x - a.x).abs() <= 0.0001 && b.y > a.y + 0.0001,
    }
}
fn honors(points: &[Point], from: &str, to: &str) -> bool {
    points.len() >= 2
        && direction(points[0], points[1], from)
        && direction(*points.last().unwrap(), points[points.len() - 2], to)
}
fn rhythm(points: &[Point]) -> bool {
    points.windows(2).enumerate().all(|(index, pair)| {
        distance(pair[0], pair[1]) + 0.0001
            >= if points.len() == 2 {
                28.0
            } else if index == 0 || index == points.len() - 2 {
                8.0
            } else {
                16.0
            }
    })
}
fn side_pairs(edge: &Value, primary: (&str, &str)) -> Vec<(String, String)> {
    let mut pairs = vec![(primary.0.to_owned(), primary.1.to_owned())];
    for from in SIDES {
        for to in SIDES {
            pairs.push((from.into(), to.into()));
        }
    }
    let mut seen = BTreeSet::new();
    pairs
        .into_iter()
        .filter(|(from, to)| {
            (["", "auto"].contains(&string(edge, "fromSide")) || from == string(edge, "fromSide"))
                && (["", "auto"].contains(&string(edge, "toSide")) || to == string(edge, "toSide"))
                && seen.insert((from.clone(), to.clone()))
        })
        .collect()
}

impl Router<'_> {
    fn node(&self, id: &str) -> &Node {
        self.measured
            .nodes
            .iter()
            .find(|node| node.authored.id == id)
            .unwrap()
    }
    fn lane(&self, node: &Node) -> usize {
        self.measured
            .lane_ids
            .iter()
            .position(|id| id == string(&node.authored.facts, "lane"))
            .unwrap()
    }
    fn overlap(&self, a: &Rect, b: &Rect, gap: f64) -> bool {
        rects_overlap_for_contract(a, b, gap, self.input.input_contract)
    }
    fn gap_y(&self, from: &Node, to: &Node, bias: f64) -> f64 {
        let a = self.lane(from);
        let b = self.lane(to);
        let bottom = self.measured.lane_tops[a] + self.measured.lane_heights[a];
        bottom + (self.measured.lane_tops[b] - bottom) * bias
    }
    // Endpoint boxes, anchors and authored sides are distinct route constraints.
    #[allow(clippy::too_many_arguments)]
    fn one_bend(
        &self,
        edge: &Value,
        from: &Node,
        to: &Node,
        start: Point,
        end: Point,
        from_side: &str,
        to_side: &str,
    ) -> Option<Vec<Point>> {
        if ["top", "bottom"].contains(&from_side) == ["top", "bottom"].contains(&to_side) {
            return None;
        }
        let corner = if ["top", "bottom"].contains(&from_side) {
            Point {
                x: start.x,
                y: end.y,
            }
        } else {
            Point {
                x: end.x,
                y: start.y,
            }
        };
        let points = join(start, vec![corner], end);
        if points.len() != 3
            || !honors(&points, from_side, to_side)
            || points
                .windows(2)
                .any(|pair| distance(pair[0], pair[1]) < 8.0)
            || !self.clears_nodes(edge, &points, from, to)
        {
            return None;
        }
        Some(points)
    }
    fn primary_sides(&self, edge: &Entry) -> (String, String) {
        let value = &edge.value;
        let from = self.node(string(value, "from"));
        let to = self.node(string(value, "to"));
        let natural = natural_sides(from, to);
        let authored = |field: &str, fallback: &str| {
            if ["", "auto"].contains(&string(value, field)) {
                fallback.to_owned()
            } else {
                string(value, field).to_owned()
            }
        };
        if let Some((from_side, to_side)) = self.feedback.sides.get(&edge.index) {
            if automatic(value) {
                return (authored("fromSide", from_side), authored("toSide", to_side));
            }
        }
        let mut preferred = (
            authored("fromSide", natural.0),
            authored("toSide", natural.1),
        );
        if automatic(value)
            && string(value, "fromSide").is_empty()
            && string(value, "toSide").is_empty()
            && from.authored.facts["lane"] != to.authored.facts["lane"]
            && center(from).x != center(to).x
            && center(from).y != center(to).y
        {
            let a = center(from);
            let b = center(to);
            let pairs = [
                (
                    if b.y < a.y { "top" } else { "bottom" },
                    if b.x < a.x { "right" } else { "left" },
                ),
                (
                    if b.x < a.x { "left" } else { "right" },
                    if b.y < a.y { "bottom" } else { "top" },
                ),
            ];
            for (from_side, to_side) in pairs {
                if self
                    .one_bend(
                        value,
                        from,
                        to,
                        anchor(&from.bounds, from_side),
                        anchor(&to.bounds, to_side),
                        from_side,
                        to_side,
                    )
                    .is_some()
                {
                    preferred = (from_side.into(), to_side.into());
                    break;
                }
            }
        }
        if !self.measured.readable {
            return preferred;
        }
        if self.measured.channel_labels.contains(&edge.index)
            && value.get("fromSide").is_none()
            && value.get("toSide").is_none()
        {
            preferred = ("top".into(), "top".into());
        }
        if !automatic(value) || value.get("channelX").is_some() || value.get("channelY").is_some() {
            return preferred;
        }
        let pairs = side_pairs(value, (&preferred.0, &preferred.1));
        let mut all = Vec::new();
        for (index, (a, b)) in pairs.iter().enumerate() {
            let candidates = self.candidates(
                edge,
                anchor(&from.bounds, a),
                anchor(&to.bounds, b),
                a,
                b,
                index * 9,
                natural,
            );
            if index == 0 && !candidates.is_empty() {
                return (a.clone(), b.clone());
            }
            all.extend(candidates);
        }
        all.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        all.first()
            .map(|candidate| (candidate.from_side.clone(), candidate.to_side.clone()))
            .unwrap_or(preferred)
    }
    fn label_position(&self, edge: &Value, points: &[Point]) -> Point {
        let source = label_point(edge, points);
        if edge.get("labelAt").is_some() {
            return source;
        }
        let segment_explicit = edge.get("labelSegment").is_some();
        if !self.measured.readable {
            if segment_explicit || points.len() != 3 {
                return source;
            }
            let index = if distance(points[0], points[1]) >= distance(points[1], points[2]) {
                0
            } else {
                1
            };
            let mut chosen = edge.clone();
            chosen["labelSegment"] = json!(index);
            let mut result = label_point(&chosen, points);
            if points[index].x == points[index + 1].x {
                result.y += 10.0;
            }
            return result;
        }
        if self.input.input_contract == InputContract::PhoenixV1
            && !segment_explicit
            && edge.get("labelDx").is_none()
            && edge.get("labelDy").is_none()
            && points.len() == 2
            && points[0].x == points[1].x
        {
            return Point {
                x: points[0].x
                    + if self.feedback.fan_in_label_clearance
                        && !super::measure::absolute(&self.input.document)
                    {
                        let xs = self
                            .edges
                            .iter()
                            .filter(|other| other.value != *edge && other.value["to"] == edge["to"])
                            .map(|other| center(self.node(string(&other.value, "from"))).x)
                            .filter(|x| (*x - points[0].x).abs() > 4.0)
                            .collect::<Vec<_>>();
                        if !xs.is_empty() && xs.iter().all(|x| *x < points[0].x) {
                            self.label_width(edge) / 2.0 + 8.0
                        } else if !xs.is_empty() && xs.iter().all(|x| *x > points[0].x) {
                            -self.label_width(edge) / 2.0 - 8.0
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    },
                y: (points[0].y + points[1].y) / 2.0,
            };
        }
        if segment_explicit || points.len() <= 2 {
            return source;
        }
        let mut segments: Vec<_> = points.windows(2).enumerate().collect();
        segments.sort_by(|(ia, a), (ib, b)| {
            ((b[0].y - b[1].y).abs() <= 0.0001)
                .cmp(&((a[0].y - a[1].y).abs() <= 0.0001))
                .then_with(|| distance(b[0], b[1]).total_cmp(&distance(a[0], a[1])))
                .then(ia.cmp(ib))
        });
        let on = |index: usize| {
            let mut chosen = edge.clone();
            chosen["labelSegment"] = json!(index);
            let mut position = label_point(&chosen, points);
            if points[index].x == points[index + 1].x {
                position.y += 10.0;
            }
            position
        };
        let preferred = on(segments[0].0);
        if self.input.input_contract == InputContract::Legacy217 {
            return preferred;
        }
        let width = self.label_width(edge);
        let clear = |position: Point| {
            !self.measured.nodes.iter().any(|node| {
                self.overlap(
                    &Rect {
                        x: position.x - width / 2.0,
                        y: position.y - 10.0,
                        width,
                        height: 14.0,
                    },
                    &node.bounds,
                    -2.0,
                )
            })
        };
        if string(edge, "label").is_empty()
            || (segments[0].1[0].y - segments[0].1[1].y).abs() > 0.0001
            || distance(segments[0].1[0], segments[0].1[1]) >= width + 8.0
            || clear(preferred)
        {
            return preferred;
        }
        for (index, segment) in &segments[1..] {
            if distance(segment[0], segment[1]) < 28.0 {
                continue;
            }
            let position = on(*index);
            if clear(position)
                && position.x - width / 2.0 >= 44.0
                && position.x + width / 2.0 <= self.measured.lane_width + 36.0
                && self
                    .measured
                    .lane_tops
                    .iter()
                    .zip(&self.measured.lane_heights)
                    .any(|(top, height)| {
                        position.y - 10.0 >= top + 4.0 && position.y + 4.0 <= top + height - 4.0
                    })
            {
                return position;
            }
        }
        preferred
    }
    fn label_width(&self, edge: &Value) -> f64 {
        30.0_f64.max(units(self.input, string(edge, "label")) * 4.8 + 10.0)
    }
    fn plate(&self, edge: &Value, points: &[Point]) -> Option<Rect> {
        if string(edge, "label").is_empty() {
            return None;
        }
        let position = self.label_position(edge, points);
        let width = self.label_width(edge);
        Some(Rect {
            x: position.x - width / 2.0,
            y: position.y - 10.0,
            width,
            height: 14.0,
        })
    }
    fn clears_nodes(&self, edge: &Value, points: &[Point], from: &Node, to: &Node) -> bool {
        for node in &self.measured.nodes {
            for (index, pair) in points.windows(2).enumerate() {
                let source = node.authored.id == from.authored.id;
                let target = node.authored.id == to.authored.id;
                // The source and target tests are independent, including a
                // self-edge whose first segment still touches its target.
                let endpoint_hit = (source && index > 0 || target && index < points.len() - 2)
                    && segment_intersects_rect(pair[0], pair[1], &node.bounds, 0.0);
                if endpoint_hit
                    || !source
                        && !target
                        && segment_intersects_rect(pair[0], pair[1], &node.bounds, 2.0)
                {
                    return false;
                }
            }
        }
        let _ = edge;
        true
    }
    fn rejection(
        &self,
        edge: &Value,
        points: &[Point],
        from_side: &str,
        to_side: &str,
    ) -> Option<&'static str> {
        if points.len() < 2 {
            return Some("route endpoints");
        }
        if !orthogonal(points) {
            return Some("orthogonal non-zero segments");
        }
        if !rhythm(points) {
            return Some("readable segment rhythm");
        }
        if !honors(points, from_side, to_side) {
            return Some("perpendicular endpoint-side direction");
        }
        let from = self.node(string(edge, "from"));
        let to = self.node(string(edge, "to"));
        if !self.clears_nodes(edge, points, from, to) {
            return Some("node clearance");
        }
        let plate = self.plate(edge, points);
        if edge.get("labelAt").is_none()
            && plate.is_some_and(|plate| {
                self.measured
                    .nodes
                    .iter()
                    .any(|node| self.overlap(&plate, &node.bounds, -2.0))
            })
        {
            return Some("edge-label node clearance");
        }
        for obstacle in &self.measured.label_obstacles {
            if points
                .windows(2)
                .any(|pair| segment_intersects_rect(pair[0], pair[1], obstacle, 0.0))
                || plate.is_some_and(|plate| self.overlap(&plate, obstacle, 0.0))
            {
                return Some("lane/phase/group label clearance");
            }
        }
        for other in &self.placed {
            if let Some(other_plate) = other.label_plate {
                if plate.is_some_and(|plate| self.overlap(&plate, &other_plate, -2.0))
                    || points.windows(2).any(|pair| {
                        segment_rect_clearance(pair[0], pair[1], &other_plate) + 0.0001 < 4.0
                    })
                {
                    return Some("placed edge-label clearance");
                }
            }
            if plate.is_some_and(|plate| {
                other
                    .points
                    .windows(2)
                    .any(|pair| segment_rect_clearance(pair[0], pair[1], &plate) + 0.0001 < 4.0)
            }) {
                return Some("placed edge-label clearance");
            }
        }
        if points.iter().any(|point| point.x < 0.0 || point.y < 0.0)
            || edge.get("labelAt").is_none()
                && plate.is_some_and(|plate| plate.x < 0.0 || plate.y < 0.0)
        {
            return Some("canvas origin");
        }
        let frames = self
            .measured
            .containers
            .iter()
            .filter(|container| container.authored.kind != "phase")
            .map(|container| {
                let mut frame = Frame::new(
                    &container.authored.kind,
                    &container.authored.id,
                    container.bounds,
                );
                frame.radius = if container.authored.kind == "lane" {
                    10.0
                } else if container.authored.kind == "group" {
                    9.0
                } else {
                    8.0
                };
                frame
            })
            .collect::<Vec<_>>();
        if !collect_border_runs(
            &[RoutedRelation::new(0, edge.clone(), points.to_vec())],
            &frames,
        )
        .is_empty()
        {
            return Some("structural-frame border clearance");
        }
        for legend in &self.measured.legends {
            if points
                .windows(2)
                .any(|pair| segment_intersects_rect(pair[0], pair[1], &legend.bounds, 0.0))
                || plate.is_some_and(|plate| self.overlap(&plate, &legend.bounds, 0.0))
            {
                return Some("legend clearance");
            }
        }
        None
    }
    fn placed_label_evidence(&self, edge: &Entry, points: &[Point]) -> Value {
        let candidate = self.plate(&edge.value, points);
        let edge_evidence = |facts: &Value, path: &str| json!({"edge":facts.get("id").unwrap_or(&Value::Null),"from":facts["from"],"to":facts["to"],"path":path});
        let label_evidence =
            |facts: &Value, rect: Rect| json!({"label":facts["label"],"rect":rect});
        let mut items = Vec::new();
        let mut extent = Rect {
            x: points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            y: points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
            width: 0.0,
            height: 0.0,
        };
        extent.width = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max) - extent.x;
        extent.height = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max) - extent.y;
        let mut query = extent;
        if let Some(label) = candidate {
            let right = (query.x + query.width).max(label.x + label.width);
            let bottom = (query.y + query.height).max(label.y + label.height);
            query.x = query.x.min(label.x);
            query.y = query.y.min(label.y);
            query.width = right - query.x;
            query.height = bottom - query.y;
        }
        query.x -= 4.0;
        query.y -= 4.0;
        query.width += 8.0;
        query.height += 8.0;
        let cells = |r: Rect| {
            [
                (r.x / 160.0).floor(),
                ((r.x + r.width) / 160.0).floor(),
                (r.y / 160.0).floor(),
                ((r.y + r.height) / 160.0).floor(),
            ]
        };
        let query_cells = cells(query);
        let walkable = |c: [f64; 4]| {
            c.iter()
                .all(|n| n.is_finite() && n.abs() <= 9007199254740991.0)
                && (c[1] - c[0] + 1.0) * (c[3] - c[2] + 1.0) <= 4096.0
        };
        for (order, other) in self.placed.iter().enumerate() {
            let x = other
                .points
                .iter()
                .map(|p| p.x)
                .fold(f64::INFINITY, f64::min);
            let y = other
                .points
                .iter()
                .map(|p| p.y)
                .fold(f64::INFINITY, f64::min);
            let route_rect = Rect {
                x,
                y,
                width: other
                    .points
                    .iter()
                    .map(|p| p.x)
                    .fold(f64::NEG_INFINITY, f64::max)
                    - x,
                height: other
                    .points
                    .iter()
                    .map(|p| p.y)
                    .fold(f64::NEG_INFINITY, f64::max)
                    - y,
            };
            for (is_label, rect) in std::iter::once((false, route_rect))
                .chain(other.label_plate.map(|rect| (true, rect)))
            {
                let item_cells = cells(rect);
                let key = if !walkable(query_cells) {
                    (0.0, 0.0)
                } else if !walkable(item_cells) {
                    (f64::INFINITY, f64::INFINITY)
                } else {
                    if item_cells[0] > query_cells[1]
                        || item_cells[1] < query_cells[0]
                        || item_cells[2] > query_cells[3]
                        || item_cells[3] < query_cells[2]
                    {
                        continue;
                    }
                    (
                        query_cells[0].max(item_cells[0]),
                        query_cells[2].max(item_cells[2]),
                    )
                };
                items.push((key, order, is_label, other));
            }
        }
        items.sort_by(|a, b| {
            a.0 .0
                .total_cmp(&b.0 .0)
                .then_with(|| a.0 .1.total_cmp(&b.0 .1))
                .then(a.1.cmp(&b.1))
                .then(a.2.cmp(&b.2))
        });
        for (_, _, is_label, other) in items {
            let facts = &other.authored.facts;
            let mut evidence = json!({"candidateEdge":edge_evidence(&edge.value,&format!("/edges/{}",edge.index)),"otherEdge":edge_evidence(facts,&other.authored.source_path)});
            if let Some(rect) = candidate {
                evidence["candidateLabel"] = label_evidence(&edge.value, rect);
            }
            if is_label {
                let rect = other.label_plate.unwrap();
                evidence["otherLabel"] = label_evidence(facts, rect);
                if let Some(own) = candidate.filter(|own| self.overlap(own, &rect, -2.0)) {
                    evidence["conflictType"] = json!("label-label");
                    evidence["rectangleMarginPx"] = json!(-2);
                    evidence["overlapPx"] = json!({"x":(own.x+own.width).min(rect.x+rect.width)-own.x.max(rect.x),"y":(own.y+own.height).min(rect.y+rect.height)-own.y.max(rect.y)});
                    return evidence;
                }
                for (index, pair) in points.windows(2).enumerate() {
                    let clearance = segment_rect_clearance(pair[0], pair[1], &rect);
                    if clearance + 0.0001 >= 4.0 {
                        continue;
                    }
                    evidence["conflictType"] = json!("route-label");
                    evidence["segment"] = json!({"edge":"candidate","index":index,"from":[pair[0].x,pair[0].y],"to":[pair[1].x,pair[1].y]});
                    evidence["measuredClearancePx"] = json!(clearance);
                    evidence["requiredClearancePx"] = json!(4);
                    return evidence;
                }
            } else if let Some(rect) = candidate {
                for (index, pair) in other.points.windows(2).enumerate() {
                    let clearance = segment_rect_clearance(pair[0], pair[1], &rect);
                    if clearance + 0.0001 >= 4.0 {
                        continue;
                    }
                    evidence["conflictType"] = json!("label-route");
                    evidence["segment"] = json!({"edge":"other","index":index,"from":[pair[0].x,pair[0].y],"to":[pair[1].x,pair[1].y]});
                    evidence["measuredClearancePx"] = json!(clearance);
                    evidence["requiredClearancePx"] = json!(4);
                    return evidence;
                }
            }
        }
        json!({})
    }
    fn preset_rejection(&self, edge: &Entry, candidate: &Candidate, invariant: &str) -> Value {
        let points = &candidate.points;
        if ["node clearance", "edge-label node clearance"].contains(&invariant) {
            for node in &self.measured.nodes {
                let role = if node.authored.id == string(&edge.value, "from") {
                    "source-endpoint"
                } else if node.authored.id == string(&edge.value, "to") {
                    "target-endpoint"
                } else {
                    "unrelated"
                };
                for (index, pair) in points.windows(2).enumerate() {
                    if role == "source-endpoint" && index == 0
                        || role == "target-endpoint" && index == points.len() - 2
                    {
                        continue;
                    }
                    let clearance = if role == "unrelated" { 2.0 } else { 0.0 };
                    if segment_intersects_rect(pair[0], pair[1], &node.bounds, clearance) {
                        return json!({"invariant":"node clearance","obstacleNode":node.authored.id,"obstacleRole":role,"segmentIndex":index,"from":[pair[0].x,pair[0].y],"to":[pair[1].x,pair[1].y],"clearancePx":clearance});
                    }
                }
            }
        }
        let mut evidence = json!({"invariant":invariant});
        if invariant == "placed edge-label clearance" {
            evidence.as_object_mut().unwrap().extend(
                self.placed_label_evidence(edge, points)
                    .as_object()
                    .unwrap()
                    .clone(),
            );
        } else if invariant == "edge-label node clearance" {
            if let Some(rect) = self.plate(&edge.value, points) {
                evidence["labelRect"] = json!(rect);
                if let Some(node) = self
                    .measured
                    .nodes
                    .iter()
                    .find(|node| self.overlap(&rect, &node.bounds, -2.0))
                {
                    evidence["obstacleNode"] = json!(node.authored.id);
                }
            }
        } else if invariant == "readable segment rhythm" {
            for (index, pair) in points.windows(2).enumerate() {
                let minimum = if points.len() == 2 {
                    28.0
                } else if index == 0 || index == points.len() - 2 {
                    8.0
                } else {
                    16.0
                };
                let actual = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
                if actual + 0.0001 < minimum {
                    evidence["segmentIndex"] = json!(index);
                    evidence["actualSegmentPx"] = json!(actual);
                    evidence["requiredSegmentPx"] = json!(minimum);
                    break;
                }
            }
        }
        evidence
    }
    // Keep source route constraints and candidate tie-break offset explicit.
    #[allow(clippy::too_many_arguments)]
    fn candidates(
        &self,
        edge: &Entry,
        start: Point,
        end: Point,
        from_side: &str,
        to_side: &str,
        offset: usize,
        natural: (&str, &str),
    ) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        for (ordinal, points) in self
            .raw_candidates(edge, start, end, from_side, to_side)
            .into_iter()
            .enumerate()
        {
            if self
                .rejection(&edge.value, &points, from_side, to_side)
                .is_none()
            {
                let cost = self.cost(&edge.value, &points, (offset + ordinal) as f64, natural);
                candidates.push(Candidate {
                    points,
                    from_side: from_side.into(),
                    to_side: to_side.into(),
                    cost,
                });
            }
        }
        candidates.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        candidates
    }
    fn raw_candidates(
        &self,
        edge: &Entry,
        start: Point,
        end: Point,
        from_side: &str,
        to_side: &str,
    ) -> Vec<Vec<Point>> {
        let from = self.node(string(&edge.value, "from"));
        let to = self.node(string(&edge.value, "to"));
        let fi = self.lane(from);
        let ti = self.lane(to);
        let mid = (start.x + end.x) / 2.0;
        let gap = if fi == ti {
            self.measured.lane_tops[fi] - 16.0
        } else {
            self.gap_y(from, to, number(&edge.value, "bias", 0.5))
        };
        let top = 8.0_f64.max(self.measured.lane_tops[fi].min(self.measured.lane_tops[ti]) - 16.0);
        let bottom = (self.measured.lane_tops[fi] + self.measured.lane_heights[fi])
            .max(self.measured.lane_tops[ti] + self.measured.lane_heights[ti])
            + 16.0;
        let mut vias = vec![
            Vec::new(),
            vec![Point {
                x: end.x,
                y: start.y,
            }],
            vec![Point {
                x: start.x,
                y: end.y,
            }],
            corridor_y(start, end, from_side, to_side, gap),
            corridor_x(start, end, from_side, to_side, mid),
            corridor_x(start, end, from_side, to_side, 20.0),
            corridor_x(
                start,
                end,
                from_side,
                to_side,
                self.measured.lane_width + 52.0,
            ),
            corridor_y(start, end, from_side, to_side, top),
            corridor_y(start, end, from_side, to_side, bottom),
        ];
        for shift in [-12.0, 12.0, -24.0, 24.0] {
            vias.push(corridor_y(start, end, from_side, to_side, gap + shift));
            vias.push(corridor_x(start, end, from_side, to_side, mid + shift));
        }
        if self.feedback.channels {
            for shift in [12.0, 24.0, 36.0] {
                vias.push(corridor_x(
                    start,
                    end,
                    from_side,
                    to_side,
                    self.measured.lane_width + 52.0 + shift,
                ));
                vias.push(corridor_y(start, end, from_side, to_side, bottom + shift));
                if top - shift >= 8.0 {
                    vias.push(corridor_y(start, end, from_side, to_side, top - shift));
                }
                vias.push(corridor_y(
                    start,
                    end,
                    from_side,
                    to_side,
                    gap + shift + 24.0,
                ));
                vias.push(corridor_y(
                    start,
                    end,
                    from_side,
                    to_side,
                    gap - shift - 24.0,
                ));
                vias.push(corridor_x(
                    start,
                    end,
                    from_side,
                    to_side,
                    mid + shift + 24.0,
                ));
                vias.push(corridor_x(
                    start,
                    end,
                    from_side,
                    to_side,
                    mid - shift - 24.0,
                ));
            }
        }
        vias.into_iter().map(|via| join(start, via, end)).collect()
    }
    fn cost(
        &self,
        edge: &Value,
        points: &[Point],
        ordinal: f64,
        natural: (&str, &str),
    ) -> Vec<f64> {
        let from = self.node(string(edge, "from"));
        let to = self.node(string(edge, "to"));
        let mut crossings = 0.0;
        let mut bends = 0.0;
        let mut shared = 0.0;
        let mut parallel = 0.0_f64;
        let mut label_deficit = 0.0;
        let plate = self.plate(edge, points);
        let visible = (self.input.input_contract == InputContract::PhoenixV1
            && self.measured.readable)
            .then(|| visible_route(points));
        for other in &self.placed {
            let shared_endpoint = [string(edge, "from"), string(edge, "to")]
                .iter()
                .any(|id| *id == other.from || *id == other.to);
            if !shared_endpoint || self.input.input_contract == InputContract::PhoenixV1 {
                let previous_crossings = crossings;
                for a in points.windows(2) {
                    for b in other.points.windows(2) {
                        if proper_segment_intersection(a[0], a[1], b[0], b[1]).is_some() {
                            crossings += 1.0;
                        }
                        if self.input.input_contract == InputContract::Legacy217 {
                            shared += axis_overlap(a[0], a[1], b[0], b[1]);
                        }
                    }
                }
                if crossings == previous_crossings {
                    if let Some(visible) = &visible {
                        let routed = [
                            RoutedRelation::new(0, edge.clone(), visible.clone()),
                            RoutedRelation::new(
                                1,
                                other.authored.facts.clone(),
                                visible_route(&other.points),
                            ),
                        ];
                        if !collect_proper_crossings(
                            &routed,
                            &CrossingOptions {
                                contract: self.input.input_contract,
                                merge_forward_collinear_waypoints: true,
                                include_shared_endpoints: Some(&|_, _| true),
                                ..Default::default()
                            },
                        )
                        .is_empty()
                        {
                            crossings += 1.0;
                        }
                    }
                }
                if !shared_endpoint && automatic(edge) {
                    bends += bend_contacts(points, &other.points) as f64;
                }
                let routed = [
                    RoutedRelation::new(0, edge.clone(), points.to_vec()),
                    RoutedRelation::new(1, other.authored.facts.clone(), other.points.clone()),
                ];
                if self.input.input_contract == InputContract::PhoenixV1 {
                    shared += collect_ambiguous_corridors(
                        &routed,
                        &CorridorOptions {
                            contract: self.input.input_contract,
                            include_shared_endpoints: Some(&|_, _| true),
                            allow_short_workflow_trunks: true,
                            ..Default::default()
                        },
                    )
                    .iter()
                    .map(|hit| hit.overlap_length)
                    .sum::<f64>();
                }
                if self.input.input_contract == InputContract::PhoenixV1 && automatic(edge) {
                    parallel += counterflow_deficit(points, &other.points);
                }
            }
            if let Some(plate) = plate {
                for pair in other.points.windows(2) {
                    let clearance = segment_rect_clearance(pair[0], pair[1], &plate);
                    if clearance.is_finite() {
                        label_deficit += (8.0 - clearance).max(0.0);
                    }
                }
            }
            if let Some(other_plate) = other.label_plate {
                if plate.is_some() || string(edge, "label").is_empty() {
                    for pair in points.windows(2) {
                        let clearance = segment_rect_clearance(pair[0], pair[1], &other_plate);
                        if clearance.is_finite() {
                            label_deficit += (8.0 - clearance).max(0.0);
                        }
                    }
                }
            }
        }
        let reverse = if ["return", "error"].contains(&string(edge, "role"))
            || number(&to.authored.facts, "col", 0.0) <= number(&from.authored.facts, "col", 0.0)
        {
            0.0
        } else {
            points
                .windows(2)
                .map(|pair| (pair[0].x - pair[1].x).max(0.0))
                .sum()
        };
        let lengths = points
            .windows(2)
            .map(|pair| distance(pair[0], pair[1]))
            .collect::<Vec<_>>();
        let length: f64 = lengths.iter().sum();
        let direct = distance(points[0], *points.last().unwrap());
        let interior = if lengths.len() > 2 {
            lengths[1..lengths.len() - 1]
                .iter()
                .map(|length| (28.0 - length).max(0.0))
                .sum()
        } else {
            0.0
        };
        let min_x = points
            .iter()
            .fold(f64::INFINITY, |min, point| min.min(point.x));
        let max_x = points
            .iter()
            .fold(f64::NEG_INFINITY, |max, point| max.max(point.x));
        let min_y = points
            .iter()
            .fold(f64::INFINITY, |min, point| min.min(point.y));
        let max_y = points
            .iter()
            .fold(f64::NEG_INFINITY, |max, point| max.max(point.y));
        let growth = (-min_x).max(0.0)
            + (max_x - self.measured.minimum_width).max(0.0)
            + (-min_y).max(0.0)
            + (max_y - self.measured.auto_height).max(0.0);
        let displacement = distance(points[0], anchor(&from.bounds, natural.0))
            + distance(*points.last().unwrap(), anchor(&to.bounds, natural.1));
        let legacy = (center(from).x
            - LEGACY_COLUMNS[number(&from.authored.facts, "col", 0.0) as usize])
            .abs()
            + (center(to).x - LEGACY_COLUMNS[number(&to.authored.facts, "col", 0.0) as usize])
                .abs();
        let common = vec![
            interior,
            points.len().saturating_sub(2) as f64,
            (if direct > 0.0 { length / direct } else { 1.0 } * 1000.0).round(),
            growth,
            (displacement * 1000.0).round(),
            legacy,
            ordinal,
        ];
        if self.input.input_contract == InputContract::Legacy217 {
            [vec![reverse, crossings, shared, label_deficit], common].concat()
        } else {
            let excessive = if independent(edge)
                && from.authored.facts["col"] == to.authored.facts["col"]
                && center(from).y != center(to).y
            {
                (length - 2.0 * (center(to).y - center(from).y).abs()).max(0.0)
            } else {
                0.0
            };
            [
                vec![
                    crossings,
                    bends,
                    shared,
                    reverse,
                    label_deficit,
                    self.later_arrival_blocks(edge, points),
                    parallel,
                    interior,
                    excessive,
                ],
                common[1..].to_vec(),
            ]
            .concat()
        }
    }
    // Preserve the source router's explicit boxes, anchors and authored sides.
    #[allow(clippy::too_many_arguments)]
    fn raw_via(
        &self,
        edge: &Value,
        from: &Node,
        to: &Node,
        start: Point,
        end: Point,
        from_side: &str,
        to_side: &str,
    ) -> Vec<Point> {
        if edge.get("via").is_some() {
            return array(edge, "via").iter().map(point).collect();
        }
        match string(edge, "route") {
            "straight" => Vec::new(),
            "drop" => {
                let y = number(
                    edge,
                    "channelY",
                    self.gap_y(from, to, number(edge, "bias", 0.5)),
                );
                vec![Point { x: start.x, y }, Point { x: end.x, y }]
            }
            "outside-right" => {
                let x = number(edge, "channelX", self.measured.lane_width + 52.0);
                vec![Point { x, y: start.y }, Point { x, y: end.y }]
            }
            "return-left" => {
                let x = number(edge, "channelX", from.bounds.x.min(to.bounds.x) - 28.0);
                vec![Point { x, y: start.y }, Point { x, y: end.y }]
            }
            "bottom-channel" => {
                let y = number(
                    edge,
                    "channelY",
                    (from.bounds.y + from.bounds.height).max(to.bounds.y + to.bounds.height) + 32.0,
                );
                vec![Point { x: start.x, y }, Point { x: end.x, y }]
            }
            "up-channel" => {
                let y = number(edge, "channelY", from.bounds.y.min(to.bounds.y) - 28.0);
                vec![Point { x: start.x, y }, Point { x: end.x, y }]
            }
            _ => {
                if self.measured.readable {
                    if let Some(x) = edge.get("channelX").and_then(Value::as_f64) {
                        if let Some(y) = edge.get("channelY").and_then(Value::as_f64) {
                            return vec![
                                Point { x, y: start.y },
                                Point { x, y },
                                Point { x: end.x, y },
                            ];
                        }
                        return vec![Point { x, y: start.y }, Point { x, y: end.y }];
                    }
                    if let Some(y) = edge.get("channelY").and_then(Value::as_f64) {
                        return vec![Point { x: start.x, y }, Point { x: end.x, y }];
                    }
                }
                if from.authored.facts["lane"] == to.authored.facts["lane"] {
                    if start.x == end.x || start.y == end.y {
                        Vec::new()
                    } else {
                        let x = (start.x + end.x) / 2.0;
                        vec![Point { x, y: start.y }, Point { x, y: end.y }]
                    }
                } else if let Some(points) =
                    self.one_bend(edge, from, to, start, end, from_side, to_side)
                {
                    points[1..points.len() - 1].to_vec()
                } else {
                    let y = self.gap_y(from, to, number(edge, "bias", 0.5));
                    vec![Point { x: start.x, y }, Point { x: end.x, y }]
                }
            }
        }
    }
}

fn compare_cost(a: &[f64], b: &[f64]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = x.total_cmp(y);
        if order != Ordering::Equal {
            return order;
        }
    }
    Ordering::Equal
}

fn axis_overlap(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if (a.y - b.y).abs() <= 0.0001 && (c.y - d.y).abs() <= 0.0001 && (a.y - c.y).abs() <= 0.0001 {
        (a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x))).max(0.0)
    } else if (a.x - b.x).abs() <= 0.0001
        && (c.x - d.x).abs() <= 0.0001
        && (a.x - c.x).abs() <= 0.0001
    {
        (a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y))).max(0.0)
    } else {
        0.0
    }
}

fn bend_contacts(left: &[Point], right: &[Point]) -> usize {
    bend_contact_points(left, right).len()
}
fn bend_contact_points(left: &[Point], right: &[Point]) -> Vec<Point> {
    let mut contacts: Vec<Point> = Vec::new();
    let near = |a: Point, b: Point| (a.x - b.x).hypot(a.y - b.y) <= 0.0001;
    for (points, other) in [(left, right), (right, left)] {
        for triple in points.windows(3) {
            let a = triple[0];
            let p = triple[1];
            let b = triple[2];
            let cross = (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x);
            if cross.abs() <= 0.0001 || near(p, other[0]) || near(p, *other.last().unwrap()) {
                continue;
            }
            if other.windows(2).any(|pair| {
                let start = pair[0];
                let end = pair[1];
                let length = (end.x - start.x).hypot(end.y - start.y);
                let cross =
                    (end.x - start.x) * (p.y - start.y) - (end.y - start.y) * (p.x - start.x);
                cross.abs() <= 0.0001 * length
                    && p.x >= start.x.min(end.x) - 0.0001
                    && p.x <= start.x.max(end.x) + 0.0001
                    && p.y >= start.y.min(end.y) - 0.0001
                    && p.y <= start.y.max(end.y) + 0.0001
            }) && !contacts.iter().any(|point| near(p, *point))
            {
                contacts.push(p);
            }
        }
    }
    contacts
}
fn counterflow_deficit(left: &[Point], right: &[Point]) -> f64 {
    let left = normalize_route_points(left);
    let right = normalize_route_points(right);
    let mut worst = 0.0_f64;
    for a in left.windows(2) {
        for b in right.windows(2) {
            let horizontal = (a[0].y - a[1].y).abs() <= 0.0001 && (b[0].y - b[1].y).abs() <= 0.0001;
            let vertical = (a[0].x - a[1].x).abs() <= 0.0001 && (b[0].x - b[1].x).abs() <= 0.0001;
            if !horizontal && !vertical {
                continue;
            }
            let (a0, a1, b0, b1, gap) = if horizontal {
                (a[0].x, a[1].x, b[0].x, b[1].x, (a[1].y - b[1].y).abs())
            } else {
                (a[0].y, a[1].y, b[0].y, b[1].y, (a[1].x - b[1].x).abs())
            };
            if (a1 - a0) * (b1 - b0) >= 0.0 || gap <= 0.0001 || gap >= 8.0 {
                continue;
            }
            let overlap = a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1));
            worst = worst.max((overlap - 24.0).max(0.0) * (8.0 - gap));
        }
    }
    worst
}

fn preset_matches(preset: &str, points: &[Point], from: &Node, to: &Node) -> bool {
    if preset.is_empty() || preset == "auto" {
        return true;
    }
    let points = normalize_route_points(points);
    if preset == "straight" {
        return points.len() == 2;
    }
    let topology = |axis: bool, coordinate: f64| {
        let start = points[0];
        let end = *points.last().unwrap();
        let expected = if axis {
            join(
                start,
                vec![
                    Point {
                        x: coordinate,
                        y: start.y,
                    },
                    Point {
                        x: coordinate,
                        y: end.y,
                    },
                ],
                end,
            )
        } else {
            join(
                start,
                vec![
                    Point {
                        x: start.x,
                        y: coordinate,
                    },
                    Point {
                        x: end.x,
                        y: coordinate,
                    },
                ],
                end,
            )
        };
        let pattern = |points: &[Point]| {
            points
                .windows(2)
                .map(|pair| (pair[0].y - pair[1].y).abs() <= 0.0001)
                .collect::<Vec<_>>()
        };
        pattern(&points) == pattern(&expected)
    };
    points.windows(2).any(|pair| {
        let horizontal = (pair[0].y - pair[1].y).abs() <= 0.0001;
        let vertical = (pair[0].x - pair[1].x).abs() <= 0.0001;
        match preset {
            "drop" => {
                if from.authored.facts["lane"] == to.authored.facts["lane"] {
                    return false;
                }
                if points.len() == 2 && vertical {
                    return true;
                }
                let upper = if center(from).y <= center(to).y {
                    from
                } else {
                    to
                };
                let lower = if center(from).y <= center(to).y {
                    to
                } else {
                    from
                };
                horizontal
                    && pair[0].y >= upper.bounds.y + upper.bounds.height - 0.0001
                    && pair[0].y <= lower.bounds.y + 0.0001
                    && topology(false, pair[0].y)
            }
            "outside-right" => {
                vertical
                    && pair[0].x
                        > (from.bounds.x + from.bounds.width).max(to.bounds.x + to.bounds.width)
                            + 0.0001
                    && topology(true, pair[0].x)
            }
            "return-left" => {
                vertical
                    && pair[0].x < from.bounds.x.min(to.bounds.x) - 0.0001
                    && topology(true, pair[0].x)
            }
            "bottom-channel" => {
                horizontal
                    && pair[0].y
                        > (from.bounds.y + from.bounds.height).max(to.bounds.y + to.bounds.height)
                            + 0.0001
                    && topology(false, pair[0].y)
            }
            "up-channel" => {
                horizontal
                    && pair[0].y < from.bounds.y.min(to.bounds.y) - 0.0001
                    && topology(false, pair[0].y)
            }
            _ => false,
        }
    })
}

impl Router<'_> {
    fn endpoint_candidates(
        &self,
        edge: &Value,
        node: &Node,
        side: &str,
        preferred: Point,
        counterpart: &Node,
    ) -> Vec<Point> {
        if self.input.input_contract != InputContract::PhoenixV1 || !independent(edge) {
            return vec![preferred];
        }
        let vertical = ["left", "right"].contains(&side);
        let own_center = anchor(&node.bounds, side);
        let coordinate = |point: Point| if vertical { point.y } else { point.x };
        let orthogonal = |point: Point| if vertical { point.x } else { point.y };
        let mut occupied = Vec::new();
        for other in &self.placed {
            for (id, point) in [
                (&other.from, other.points[0]),
                (&other.to, *other.points.last().unwrap()),
            ] {
                if id != &node.authored.id
                    || (orthogonal(point) - orthogonal(own_center)).abs() >= 0.0001
                {
                    continue;
                }
                let width = number(
                    edge,
                    "width",
                    if string(edge, "variant") == "emphasis" {
                        1.8
                    } else {
                        1.4
                    },
                );
                let other_width = number(
                    &other.authored.facts,
                    "width",
                    if other.variant == "emphasis" {
                        1.8
                    } else {
                        1.4
                    },
                );
                occupied.push((coordinate(point), 16.0_f64.max(3.5 * (width + other_width))));
            }
        }
        let clear = |point: Point| {
            occupied
                .iter()
                .all(|(other, gap)| (coordinate(point) - other).abs() >= gap - 0.0001)
        };
        if clear(preferred) {
            return vec![preferred];
        }
        let extent = if vertical {
            node.bounds.height
        } else {
            node.bounds.width
        };
        let direction = if coordinate(center(counterpart)) >= coordinate(own_center) {
            1.0
        } else {
            -1.0
        };
        let mut output = Vec::new();
        let mut offset = 0.0;
        while offset <= extent / 2.0 - 16.0 {
            for sign in [direction, -direction] {
                let mut point = own_center;
                if vertical {
                    point.y += sign * offset;
                } else {
                    point.x += sign * offset;
                }
                if clear(point) && !output.contains(&point) {
                    output.push(point);
                }
            }
            if output.len() >= 2 {
                output.truncate(2);
                return output;
            }
            offset += 12.0;
        }
        if output.is_empty() {
            vec![preferred]
        } else {
            output
        }
    }
    // Keep endpoint constraints and the source candidate ordinal independent.
    #[allow(clippy::too_many_arguments)]
    fn expanded_candidate(
        &self,
        edge: &Entry,
        start: Point,
        end: Point,
        from_side: &str,
        to_side: &str,
        ordinal: f64,
        natural: (&str, &str),
    ) -> Option<Candidate> {
        let base = self.measured.lane_width + 52.0;
        let initial = join(start, corridor_x(start, end, from_side, to_side, base), end);
        let label = self.plate(&edge.value, &initial);
        let mut x = base;
        if let Some(label) = label {
            for node in &self.measured.nodes {
                if self.overlap(&label, &node.bounds, -2.0) {
                    let deficit = node.bounds.x + node.bounds.width - 2.0 - label.x;
                    if deficit > 0.0 {
                        x = x.max(base + deficit * 2.0);
                    }
                }
            }
            for other in &self.placed {
                if let Some(plate) = other.label_plate {
                    if self.overlap(&label, &plate, -2.0) {
                        let deficit = plate.x + plate.width - 2.0 - label.x;
                        if deficit > 0.0 {
                            x = x.max(base + deficit * 2.0);
                        }
                    }
                }
                for pair in other.points.windows(2) {
                    if crate::geometry::segment_rect_clearance(pair[0], pair[1], &label) + 0.0001
                        < 4.0
                    {
                        let deficit = pair[0].x.max(pair[1].x) + 4.0 - label.x;
                        if deficit > 0.0 {
                            x = x.max(base + deficit * 2.0);
                        }
                    }
                }
            }
        }
        x = (x * 1000.0).ceil() / 1000.0;
        let mut rightmost = base;
        for node in &self.measured.nodes {
            rightmost = rightmost.max(node.bounds.x + node.bounds.width);
        }
        for relation in &self.placed {
            for point in &relation.points {
                rightmost = rightmost.max(point.x);
            }
            if let Some(plate) = relation.label_plate {
                rightmost = rightmost.max(plate.x + plate.width);
            }
        }
        let mut growth = 32.0_f64
            .max(label.map(|label| label.width).unwrap_or(0.0))
            .max(rightmost + 16.0 - x);
        let mut infeasible = base;
        for _ in 0..7 {
            if x > base + 0.0001 {
                let points = join(start, corridor_x(start, end, from_side, to_side, x), end);
                if self
                    .rejection(&edge.value, &points, from_side, to_side)
                    .is_none()
                {
                    let mut feasible = x;
                    let mut selected = points;
                    for _ in 0..53 {
                        if feasible - infeasible <= 0.001 {
                            break;
                        }
                        let mid = (((infeasible + feasible) / 2.0) * 1000.0).ceil() / 1000.0;
                        if mid >= feasible - 0.0001 {
                            break;
                        }
                        let points =
                            join(start, corridor_x(start, end, from_side, to_side, mid), end);
                        if self
                            .rejection(&edge.value, &points, from_side, to_side)
                            .is_none()
                        {
                            feasible = mid;
                            selected = points;
                        } else {
                            infeasible = mid;
                        }
                    }
                    let cost = self.cost(&edge.value, &selected, ordinal, natural);
                    return Some(Candidate {
                        points: selected,
                        from_side: from_side.into(),
                        to_side: to_side.into(),
                        cost,
                    });
                }
                infeasible = x;
            }
            x = (1000.0 * (x + growth)).ceil() / 1000.0;
            growth *= 2.0;
        }
        None
    }
    fn automatic_plan(
        &self,
        edge: &Entry,
        primary: (&str, &str),
        ports: (Option<Point>, Option<Point>),
    ) -> Result<Candidate, Diagnostics> {
        let value = &edge.value;
        let from = self.node(string(value, "from"));
        let to = self.node(string(value, "to"));
        let natural = natural_sides(from, to);
        let natural = (
            if ["", "auto"].contains(&string(value, "fromSide")) {
                natural.0
            } else {
                string(value, "fromSide")
            },
            if ["", "auto"].contains(&string(value, "toSide")) {
                natural.1
            } else {
                string(value, "toSide")
            },
        );
        let pairs = side_pairs(value, primary);
        let mut plans = Vec::new();
        let mut failed_attempts = Vec::new();
        let start =
            self.endpoint_candidates(value, from, natural.0, anchor(&from.bounds, natural.0), to)
                [0];
        let end =
            self.endpoint_candidates(value, to, natural.1, anchor(&to.bounds, natural.1), from)[0];
        let aligned = vec![start, end];
        // In a repair pass the edge's spread ports take precedence over the
        // centred straight shortcut.
        let spread = !self.feedback.sides.is_empty() && (ports.0.is_some() || ports.1.is_some());
        if self.input.input_contract == InputContract::PhoenixV1
            && value.get("fromSide").is_none()
            && value.get("toSide").is_none()
            && value.get("labelAt").is_none()
            && !spread
            && self
                .rejection(value, &aligned, natural.0, natural.1)
                .is_none()
        {
            plans.push(Candidate {
                cost: self.cost(value, &aligned, -1.0, natural),
                points: aligned,
                from_side: natural.0.into(),
                to_side: natural.1.into(),
            });
        }
        for (index, (from_side, to_side)) in pairs.iter().enumerate() {
            let preferred_start = if index == 0 {
                ports.0.unwrap_or_else(|| anchor(&from.bounds, from_side))
            } else {
                anchor(&from.bounds, from_side)
            };
            let preferred_end = if index == 0 {
                ports.1.unwrap_or_else(|| anchor(&to.bounds, to_side))
            } else {
                anchor(&to.bounds, to_side)
            };
            let starts = self.endpoint_candidates(value, from, from_side, preferred_start, to);
            let ends = self.endpoint_candidates(value, to, to_side, preferred_end, from);
            let mut port_ordinal = 0;
            for (si, start) in starts.iter().enumerate() {
                for (ei, end) in ends.iter().enumerate() {
                    if si != 0 || ei != 0 {
                        port_ordinal += 1;
                    }
                    let offset = index * 9 + port_ordinal * 144;
                    let candidates =
                        self.candidates(edge, *start, *end, from_side, to_side, offset, natural);
                    if candidates.is_empty() {
                        if let Some(candidate) = self.expanded_candidate(
                            edge,
                            *start,
                            *end,
                            from_side,
                            to_side,
                            (offset + 6) as f64,
                            natural,
                        ) {
                            plans.push(candidate);
                        } else {
                            failed_attempts.push((
                                index,
                                si == 0 && ei == 0,
                                from_side,
                                to_side,
                                self.raw_candidates(edge, *start, *end, from_side, to_side),
                            ));
                        }
                    } else {
                        plans.extend(candidates);
                    }
                }
            }
        }
        plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        self.repair_plans(value, &mut plans, natural);
        if let Some(plan) = plans.into_iter().next() {
            return Ok(plan);
        }
        // The source probes alternative ports before the preferred pair when
        // collecting failures; successful route costs retain their old order.
        failed_attempts.sort_by_key(|(pair, preferred, _, _, _)| {
            (
                *pair,
                self.input.input_contract == InputContract::PhoenixV1 && *preferred,
            )
        });
        let relevant_pin = value.get("labelAt").is_some()
            || self.placed.iter().any(|relation| {
                relation.authored.facts.get("labelAt").is_some()
                    || absolute_route(&relation.authored.facts)
            });
        let mut first_failure = None;
        let mut rank_feedback = false;
        let mut lane_feedback = false;
        let from_col = number(&from.authored.facts, "col", 0.0) as usize;
        let to_col = number(&to.authored.facts, "col", 0.0) as usize;
        let minimum = from.bounds.width / 2.0 + 32.0 + to.bounds.width / 2.0;
        for (_, _, from_side, to_side, raw) in failed_attempts {
            let conflict = if relevant_pin {
                raw.iter()
                    .filter(|points| points.len() >= 2)
                    .find_map(|points| {
                        let candidate = self.relation(
                            edge,
                            Candidate {
                                points: points.clone(),
                                from_side: from_side.clone(),
                                to_side: to_side.clone(),
                                cost: Vec::new(),
                            },
                        );
                        super::pins::failed_candidate(self.input, &candidate, &self.placed)
                    })
            } else {
                None
            };
            if let Some(conflict) = conflict {
                if first_failure.is_none() {
                    first_failure = Some(conflict);
                }
                continue;
            }
            let facing = raw
                .first()
                .and_then(|points| points.first().zip(points.last()))
                .is_some_and(|(start, end)| {
                    from_side == "right" && to_side == "left" && end.x > start.x
                        || from_side == "left" && to_side == "right" && start.x > end.x
                });
            let rank = facing
                && from_col != to_col
                && (self.measured.columns[to_col] - self.measured.columns[from_col]).abs() + 0.0001
                    < minimum;
            let lane = from.authored.facts["lane"] != to.authored.facts["lane"]
                && self.measured.lane_gap < 32.0;
            rank_feedback |= rank;
            lane_feedback |= lane;
            if !rank && !lane && first_failure.is_none() {
                // Keep ordinary failures as well as pin failures. A later
                // rejected candidate cannot replace an earlier cause.
                let mut families = vec![
                    "facing-straight",
                    "horizontal-then-vertical",
                    "vertical-then-horizontal",
                    "lane-gap-corridor",
                    "column-gap-corridor",
                    "outside-left",
                    "outside-right",
                    "top-corridor",
                    "bottom-corridor",
                ];
                for _ in 0..4 {
                    families.extend(["lane-gap-corridor", "column-gap-corridor"]);
                }
                let mut diagnostic = failure(
                    "workflow/solver-budget-exhausted",
                    &format!("/edges/{}", edge.index),
                    "No feasible readable-v2 automatic route",
                    json!({"attemptedCandidateFamilies":families,"candidateCount":raw.len()}),
                );
                diagnostic.subject.identity = value["id"].as_str().map(str::to_owned);
                first_failure = Some(diagnostic);
            }
        }
        if rank_feedback {
            return Err(vec![failure(
                "workflow/layout-feedback",
                &format!("/edges/{}", edge.index),
                "Rank gap requires measured route feedback",
                json!({"kind":"rank-gap-minimum","fromCol":from_col.min(to_col),"toCol":from_col.max(to_col),"minimum":minimum}),
            )]);
        }
        if lane_feedback {
            return Err(vec![failure(
                "workflow/layout-feedback",
                &format!("/edges/{}", edge.index),
                "Lane gap requires measured route feedback",
                json!({"kind":"lane-gap-minimum","minimum":32.0}),
            )]);
        }
        let code = if value.get("fromSide").is_some() || value.get("toSide").is_some() {
            "workflow/explicit-pin-conflict"
        } else {
            "workflow/solver-budget-exhausted"
        };
        if code == "workflow/solver-budget-exhausted" {
            if let Some(conflict) = first_failure {
                return Err(vec![conflict]);
            }
        }
        Err(vec![failure(
            code,
            &format!("/edges/{}", edge.index),
            "No feasible readable-v2 automatic route",
            json!({"from":value["from"],"to":value["to"],"candidateCount":pairs.len()*17,"attemptedCandidateFamilies":["facing-straight","horizontal-then-vertical","vertical-then-horizontal","lane-gap-corridor","column-gap-corridor","outside-left","outside-right","top-corridor","bottom-corridor"]}),
        )])
    }
    fn controlled_plan(&self, edge: &Entry) -> Result<Candidate, Diagnostics> {
        let value = &edge.value;
        let allowed_channels: &[&str] = match string(value, "route") {
            "" | "auto" => &["channelX", "channelY"],
            "drop" | "bottom-channel" | "up-channel" => &["channelY"],
            "outside-right" | "return-left" => &["channelX"],
            _ => &[],
        };
        let conflicts = ["channelX", "channelY"]
            .iter()
            .filter(|field| value.get(**field).is_some() && !allowed_channels.contains(field))
            .copied()
            .collect::<Vec<_>>();
        if !conflicts.is_empty() {
            return Err(vec![failure(
                "workflow/explicit-pin-conflict",
                &format!("/edges/{}/{}", edge.index, conflicts[0]),
                "Channel pin contradicts the authored route preset",
                json!({"route":value["route"],"allowedPins":allowed_channels,"conflictingPins":conflicts.iter().map(|field|json!({"field":field,"path":format!("/edges/{}/{field}",edge.index),"value":value[*field]})).collect::<Vec<_>>()}),
            )]);
        }
        let from = self.node(string(value, "from"));
        let to = self.node(string(value, "to"));
        let natural = natural_sides(from, to);
        let primary = (
            if ["", "auto"].contains(&string(value, "fromSide")) {
                natural.0
            } else {
                string(value, "fromSide")
            },
            if ["", "auto"].contains(&string(value, "toSide")) {
                natural.1
            } else {
                string(value, "toSide")
            },
        );
        let mut plans = Vec::new();
        let mut fallback = None;
        let mut directional = None;
        for (ordinal, (a, b)) in side_pairs(value, primary).into_iter().enumerate() {
            let start = anchor(&from.bounds, &a);
            let end = anchor(&to.bounds, &b);
            let via = self.raw_via(value, from, to, start, end, &a, &b);
            let authored_points = std::iter::once(start)
                .chain(via)
                .chain(std::iter::once(end))
                .collect::<Vec<_>>();
            let points = if absolute_route(value) {
                authored_points
            } else {
                normalize_route_points(&authored_points)
            };
            let cost = if points.len() >= 2 {
                self.cost(value, &points, ordinal as f64, primary)
            } else {
                Vec::new()
            };
            let candidate = Candidate {
                points,
                from_side: a,
                to_side: b,
                cost,
            };
            if fallback.is_none() {
                fallback = Some(candidate.clone());
            }
            if directional.is_none()
                && honors(&candidate.points, &candidate.from_side, &candidate.to_side)
            {
                directional = Some(candidate.clone());
            }
            if self
                .rejection(
                    value,
                    &candidate.points,
                    &candidate.from_side,
                    &candidate.to_side,
                )
                .is_some()
                || !preset_matches(string(value, "route"), &candidate.points, from, to)
            {
                continue;
            }
            if ["channelX", "channelY"].iter().any(|field| {
                value
                    .get(*field)
                    .and_then(Value::as_f64)
                    .is_some_and(|coordinate| {
                        !candidate.points.windows(2).any(|pair| {
                            if *field == "channelX" {
                                pair[0].x == coordinate
                                    && pair[1].x == coordinate
                                    && (pair[0].y - pair[1].y).abs() > 0.0001
                            } else {
                                pair[0].y == coordinate
                                    && pair[1].y == coordinate
                                    && (pair[0].x - pair[1].x).abs() > 0.0001
                            }
                        })
                    })
            }) {
                continue;
            }
            plans.push(candidate);
        }
        plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        if let Some(plan) = plans.into_iter().next() {
            return Ok(plan);
        }
        let fallback = if absolute_route(value) {
            directional.or(fallback)
        } else {
            fallback
        }
        .unwrap();
        let rejection = self
            .rejection(
                value,
                &fallback.points,
                &fallback.from_side,
                &fallback.to_side,
            )
            .unwrap_or("route preset compatibility");
        // Authoritative absolute geometry is retained for the final pairwise
        // classifier when the route itself is valid and only labels conflict.
        if absolute_route(value)
            && rejection == "placed edge-label clearance"
            && preset_matches(string(value, "route"), &fallback.points, from, to)
        {
            return Ok(fallback);
        }
        if !absolute_route(value) {
            let mut evidence = json!({"attemptedCandidateFamily":value["route"],"points":fallback.points.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),"fromSide":fallback.from_side,"toSide":fallback.to_side,"requiredEndpointStubPx":8,"requiredInteriorSegmentPx":16,"requiredDirectClearancePx":28});
            if self.input.input_contract == InputContract::PhoenixV1 {
                evidence.as_object_mut().unwrap().extend(
                    self.preset_rejection(edge, &fallback, rejection)
                        .as_object()
                        .unwrap()
                        .clone(),
                );
            }
            let mut diagnostic = failure(
                "workflow/route-preset-conflict",
                &format!("/edges/{}/route", edge.index),
                "Authored route preset cannot satisfy readable-v2 geometry",
                evidence,
            );
            diagnostic.subject.identity = value["id"].as_str().map(str::to_owned);
            return Err(vec![diagnostic]);
        }
        Err(vec![failure(
            if absolute_route(value) {
                "workflow/explicit-pin-conflict"
            } else {
                "workflow/route-preset-conflict"
            },
            &format!(
                "/edges/{}/{}",
                edge.index,
                if value.get("via").is_some() {
                    "via"
                } else if value.get("channelX").is_some() {
                    "channelX"
                } else if value.get("channelY").is_some() {
                    "channelY"
                } else {
                    "route"
                }
            ),
            "Authored route cannot satisfy readable-v2 geometry",
            json!({"invariant":self.rejection(value,&fallback.points,&fallback.from_side,&fallback.to_side).unwrap_or("route preset compatibility"),"points":fallback.points,"route":value["route"],"pins":value}),
        )])
    }
    fn relation(&self, edge: &Entry, candidate: Candidate) -> Relation {
        let author = edge_authored(edge);
        let plate = self.plate(&edge.value, &candidate.points);
        let label = string(&edge.value, "label");
        let position = if label.is_empty() {
            Point::default()
        } else {
            self.label_position(&edge.value, &candidate.points)
        };
        Relation {
            crossover_halo: self.feedback.halo.contains(&edge.index) && automatic(&edge.value),
            independent_ports: false,
            junction: None,
            label_owner: None,
            note_bounds: None,
            note_backdrop: None,
            from: string(&edge.value, "from").into(),
            to: string(&edge.value, "to").into(),
            variant: if string(&edge.value, "variant").is_empty() {
                "default".into()
            } else {
                string(&edge.value, "variant").into()
            },
            label: label.into(),
            note: edge
                .value
                .get("note")
                .and_then(Value::as_str)
                .map(str::to_owned),
            points: candidate.points,
            label_plate: plate,
            text: if label.is_empty() {
                Vec::new()
            } else {
                vec![text(
                    format!("{}/label", author.source_path),
                    label,
                    position.x,
                    position.y,
                    8.0,
                    "middle",
                )]
            },
            authored: author,
            marker_start: None,
            marker_end: None,
        }
    }
    fn validate_label_pin(&self, edge: &Entry, candidate: &Candidate) -> Result<(), Diagnostics> {
        if !self.measured.readable || edge.value.get("labelAt").is_none() {
            return Ok(());
        }
        let Some(plate) = self.plate(&edge.value, &candidate.points) else {
            return Ok(());
        };
        if plate.x < 0.0
            || plate.y < 0.0
            || self
                .measured
                .nodes
                .iter()
                .any(|node| self.overlap(&plate, &node.bounds, -2.0))
        {
            return Err(vec![failure(
                "workflow/explicit-pin-conflict",
                &format!("/edges/{}/labelAt", edge.index),
                "Authored labelAt conflicts with canvas origin or a node",
                json!({"offendingRect":plate,"conflictingPins":[{"field":"labelAt","path":format!("/edges/{}/labelAt",edge.index),"value":edge.value["labelAt"]}]}),
            )]);
        }
        Ok(())
    }
}

pub(super) fn edge_authored(edge: &Entry) -> Authored {
    authored(
        edge,
        "edges",
        if string(&edge.value, "role").is_empty() {
            "edge"
        } else {
            string(&edge.value, "role")
        },
        &[
            "fromSide",
            "toSide",
            "route",
            "via",
            "channelX",
            "channelY",
            "bias",
            "labelAt",
            "labelDx",
            "labelDy",
            "labelSegment",
            "width",
        ],
    )
}

pub(super) fn route(
    input: &DiagramInput,
    measured: &Measurement,
    edges: &[Entry],
    feedback: &Feedback,
) -> Result<Vec<Relation>, Diagnostics> {
    let mut router = Router {
        input,
        measured,
        edges,
        placed: Vec::new(),
        feedback,
    };
    let sides = edges
        .iter()
        .map(|edge| router.primary_sides(edge))
        .collect::<Vec<_>>();
    let ports = port_spread(&router, edges, &sides);
    let mut results: BTreeMap<usize, Relation> = BTreeMap::new();
    let order: Vec<usize> = (0..edges.len()).collect();
    if measured.readable && input.input_contract == InputContract::PhoenixV1 {
        for (index, edge) in edges.iter().enumerate() {
            if !absolute_route(&edge.value)
                || !edges.iter().any(|other| {
                    !absolute_route(&other.value)
                        && ["from", "to"].iter().any(|endpoint| {
                            other.value["from"] == edge.value[*endpoint]
                                || other.value["to"] == edge.value[*endpoint]
                        })
                })
            {
                continue;
            }
            let plan = router.controlled_plan(edge)?;
            let relation = router.relation(edge, plan);
            router.placed.push(relation.clone());
            results.insert(index, relation);
        }
        let has_label_pins = edges.iter().any(|edge| {
            ["labelAt", "labelDx", "labelDy", "labelSegment"]
                .iter()
                .any(|field| edge.value.get(*field).is_some())
        });
        if !has_label_pins {
            for (index, edge) in edges.iter().enumerate() {
                if !independent(&edge.value) || edge.value["from"] == edge.value["to"] {
                    continue;
                }
                if string(&edge.value, "role") != "main"
                    && edges.iter().any(|other| {
                        string(&other.value, "role") == "main"
                            && other.value["from"] == edge.value["to"]
                            && other.value["to"] == edge.value["from"]
                    })
                {
                    continue;
                }
                if let Ok(plan) = router.automatic_plan(
                    edge,
                    (&sides[index].0, &sides[index].1),
                    ports.get(&index).copied().unwrap_or_default(),
                ) {
                    if plan.points.len() == 2 {
                        let relation = router.relation(edge, plan);
                        router.placed.push(relation.clone());
                        results.insert(index, relation);
                    }
                }
            }
        }
    }
    let mut order = order;
    if measured.readable && input.input_contract == InputContract::PhoenixV1 {
        for source_index in feedback.route_first.iter().rev() {
            if let Some(index) = edges
                .iter()
                .position(|edge| edge.index == *source_index && independent(&edge.value))
            {
                order.retain(|other| *other != index);
                order.insert(0, index);
            }
        }
    }
    for index in order {
        if results.contains_key(&index) {
            continue;
        }
        let edge = &edges[index];
        let (from_side, to_side) = (&sides[index].0, &sides[index].1);
        let candidate = if !measured.readable {
            let from = router.node(string(&edge.value, "from"));
            let to = router.node(string(&edge.value, "to"));
            let spread = ports.get(&index).copied().unwrap_or_default();
            let start = spread.0.unwrap_or_else(|| anchor(&from.bounds, from_side));
            let end = spread.1.unwrap_or_else(|| anchor(&to.bounds, to_side));
            let via = router.raw_via(&edge.value, from, to, start, end, from_side, to_side);
            Candidate {
                points: std::iter::once(start)
                    .chain(via)
                    .chain(std::iter::once(end))
                    .collect(),
                from_side: from_side.clone(),
                to_side: to_side.clone(),
                cost: Vec::new(),
            }
        } else if automatic(&edge.value) {
            router.automatic_plan(
                edge,
                (from_side, to_side),
                ports.get(&index).copied().unwrap_or_default(),
            )?
        } else {
            router.controlled_plan(edge)?
        };
        router.validate_label_pin(edge, &candidate)?;
        let relation = router.relation(edge, candidate);
        router.placed.push(relation.clone());
        results.insert(index, relation);
    }
    let relations = results.into_values().collect::<Vec<_>>();
    if measured.readable {
        if let Some(diagnostic) = super::pins::pairwise(input, &relations) {
            return Err(vec![diagnostic]);
        }
    }
    Ok(relations)
}

fn port_spread(
    router: &Router<'_>,
    edges: &[Entry],
    sides: &[(String, String)],
) -> BTreeMap<usize, (Option<Point>, Option<Point>)> {
    // Members retain edge index, source-end flag, owner and counterpart nodes.
    type PortMember<'a> = (usize, bool, &'a Node, &'a Node);
    type PortGroups<'a> = BTreeMap<(String, String), Vec<PortMember<'a>>>;
    let mut groups: PortGroups<'_> = BTreeMap::new();
    for (index, edge) in edges.iter().enumerate() {
        if !automatic(&edge.value) || edge.value.get("labelAt").is_some() {
            continue;
        }
        let from = router.node(string(&edge.value, "from"));
        let to = router.node(string(&edge.value, "to"));
        groups
            .entry((from.authored.id.clone(), sides[index].0.clone()))
            .or_default()
            .push((index, true, from, to));
        groups
            .entry((to.authored.id.clone(), sides[index].1.clone()))
            .or_default()
            .push((index, false, to, from));
    }
    let mut result: BTreeMap<usize, (Option<Point>, Option<Point>)> = BTreeMap::new();
    for ((_, side), mut items) in groups {
        if items.len() < 2 {
            continue;
        }
        let vertical = ["left", "right"].contains(&side.as_str());
        let key = |entry: usize| {
            format!(
                "{}\0{}\0{}\0{}",
                string(&edges[entry].value, "id"),
                string(&edges[entry].value, "from"),
                string(&edges[entry].value, "to"),
                string(&edges[entry].value, "label")
            )
        };
        items.sort_by(|a, b| {
            (if vertical {
                center(a.3).y
            } else {
                center(a.3).x
            })
            .total_cmp(&if vertical {
                center(b.3).y
            } else {
                center(b.3).x
            })
            .then_with(|| key(a.0).encode_utf16().cmp(key(b.0).encode_utf16()))
        });
        let count = items.len();
        let extent = if vertical {
            items[0].2.bounds.height
        } else {
            items[0].2.bounds.width
        };
        let spacing = 14.0_f64.min((extent - 32.0).max(0.0) / (count - 1) as f64);
        if spacing <= 0.0 {
            continue;
        }
        for (ordinal, (index, source, node, _)) in items.into_iter().enumerate() {
            let offset = (ordinal as f64 - (count - 1) as f64 / 2.0) * spacing;
            let mut point = anchor(&node.bounds, &side);
            if vertical {
                point.y += offset;
            } else {
                point.x += offset;
            }
            let entry = result.entry(index).or_default();
            if source {
                entry.0 = Some(point);
            } else {
                entry.1 = Some(point);
            }
        }
    }
    result
}
