// Pure predicates ported from the two frozen geometry.mjs sources; see NOTICE.md.
// These collectors report geometry. Callers own quality/profile gates and diagnostics.
use crate::geometry::{
    cross_product, finite_point, finite_rect, normalize_route_points, proper_segment_intersection,
    segment_intersects_rect, segment_rect_clearance, Point, Rect, EPSILON,
};
use crate::input::InputContract;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedRelation {
    pub relation_index: usize,
    pub relation: Value,
    pub points: Vec<Point>,
    // Lifecycle bracket pieces may omit an actual source/target endpoint.
    pub source_endpoint: bool,
    pub target_endpoint: bool,
    // Foreign SVG paths can carry explicit analysis segments, including curves.
    // Only the border-run collector uses this override, matching the source.
    pub segments: Option<Vec<Segment>>,
}

impl RoutedRelation {
    pub fn new(relation_index: usize, relation: Value, points: Vec<Point>) -> Self {
        Self {
            relation_index,
            relation,
            points,
            source_endpoint: true,
            target_endpoint: true,
            segments: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub kind: String,
    pub id: String,
    pub bounds: Rect,
    pub radius: f64,
    pub line: Option<(Point, Point)>,
}

impl Frame {
    pub fn new(kind: impl Into<String>, id: impl Into<String>, bounds: Rect) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            bounds,
            radius: 0.0,
            line: None,
        }
    }

    pub fn from_value(value: &Value) -> Self {
        let mut frame = Self::new(
            value["kind"]
                .as_str()
                .or(value["shape"].as_str())
                .unwrap_or("frame"),
            value["id"].as_str().unwrap_or(""),
            Rect {
                x: number(value, "x"),
                y: number(value, "y"),
                width: number(value, "width"),
                height: number(value, "height"),
            },
        );
        frame.radius = value["radius"]
            .as_f64()
            .or_else(|| value["radius"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0.0);
        if value["shape"] == "line" {
            let point = |key: &str, x: &str, y: &str| {
                value[key]
                    .as_array()
                    .map(|p| Point {
                        x: p.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
                        y: p.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
                    })
                    .unwrap_or(Point {
                        x: number(value, x),
                        y: number(value, y),
                    })
            };
            frame.line = Some((point("start", "x1", "y1"), point("end", "x2", "y2")));
        }
        frame
    }
}

fn number(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or(f64::NAN)
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= EPSILON
}
fn component(point: Point, axis: usize) -> f64 {
    if axis == 0 {
        point.x
    } else {
        point.y
    }
}
fn valid_relation(relation: &Value) -> bool {
    relation["from"].is_string() && relation["to"].is_string()
}
fn share_endpoint(a: &Value, b: &Value) -> bool {
    [text(a, "from"), text(a, "to")]
        .iter()
        .any(|id| *id == text(b, "from") || *id == text(b, "to"))
}
fn same_junction(a: &Value, b: &Value) -> bool {
    truthy(&a["junction"]) && a["junction"] == b["junction"]
}
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64().is_some_and(|n| n != 0.0),
        Value::String(v) => !v.is_empty(),
        _ => true,
    }
}

pub type RelationPairPredicate<'a> = &'a dyn Fn(&Value, &Value) -> bool;

pub struct CorridorOptions<'a> {
    pub contract: InputContract,
    pub min_overlap_px: f64,
    pub include_shared_endpoints: Option<RelationPairPredicate<'a>>,
    pub include_shared_endpoint_counterflow: Option<RelationPairPredicate<'a>>,
    pub allow_short_workflow_trunks: bool,
}

impl Default for CorridorOptions<'_> {
    fn default() -> Self {
        Self {
            contract: InputContract::PhoenixV1,
            min_overlap_px: 8.0,
            include_shared_endpoints: None,
            include_shared_endpoint_counterflow: None,
            allow_short_workflow_trunks: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorridorHit {
    pub left_relation_index: usize,
    pub right_relation_index: usize,
    pub left_segment: usize,
    pub right_segment: usize,
    pub overlap_length: f64,
    pub overlap_start: Point,
    pub overlap_end: Point,
}

#[derive(Clone, Copy)]
struct AxisOverlap {
    length: f64,
    start: Point,
    end: Point,
}

fn collinear_axis_overlap(a: Point, b: Point, c: Point, d: Point) -> Option<AxisOverlap> {
    let horizontal = (a.y - b.y).abs() <= EPSILON
        && (c.y - d.y).abs() <= EPSILON
        && (a.y - c.y).abs() <= EPSILON;
    let vertical = (a.x - b.x).abs() <= EPSILON
        && (c.x - d.x).abs() <= EPSILON
        && (a.x - c.x).abs() <= EPSILON;
    if !horizontal && !vertical {
        return None;
    }
    let axis = if horizontal { 0 } else { 1 };
    let low = component(a, axis)
        .min(component(b, axis))
        .max(component(c, axis).min(component(d, axis)));
    let high = component(a, axis)
        .max(component(b, axis))
        .min(component(c, axis).max(component(d, axis)));
    if high - low <= EPSILON {
        return None;
    }
    Some(AxisOverlap {
        length: high - low,
        start: if horizontal {
            Point { x: low, y: a.y }
        } else {
            Point { x: a.x, y: low }
        },
        end: if horizontal {
            Point { x: high, y: a.y }
        } else {
            Point { x: a.x, y: high }
        },
    })
}

fn short_workflow_trunk(
    left: &RoutedRelation,
    right: &RoutedRelation,
    a: &[Point],
    b: &[Point],
    i: usize,
    j: usize,
    length: f64,
) -> bool {
    if length > 24.0 + EPSILON {
        return false;
    }
    let variant = |r: &Value| {
        if truthy(&r["variant"]) {
            r["variant"].clone()
        } else {
            Value::String("default".into())
        }
    };
    let width = |r: &Value| {
        r["width"]
            .as_f64()
            .filter(|n| *n != 0.0)
            .unwrap_or(if variant(r) == "emphasis" { 1.8 } else { 1.4 })
    };
    if variant(&left.relation) != variant(&right.relation)
        || width(&left.relation) != width(&right.relation)
        || text(&left.relation, "role") != text(&right.relation, "role")
    {
        return false;
    }
    let same = |p: Point, q: Point| (p.x - q.x).abs() < EPSILON && (p.y - q.y).abs() < EPSILON;
    let source = left.source_endpoint
        && right.source_endpoint
        && left.relation["from"] == right.relation["from"]
        && i == 0
        && j == 0
        && same(a[0], b[0]);
    let target = left.target_endpoint
        && right.target_endpoint
        && left.relation["to"] == right.relation["to"]
        && i == a.len() - 2
        && j == b.len() - 2
        && same(*a.last().unwrap(), *b.last().unwrap());
    (source || target)
        && (a[i + 1].x - a[i].x) * (b[j + 1].x - b[j].x)
            + (a[i + 1].y - a[i].y) * (b[j + 1].y - b[j].y)
            > 0.0
}

pub fn collect_ambiguous_corridors(
    routed: &[RoutedRelation],
    options: &CorridorOptions<'_>,
) -> Vec<CorridorHit> {
    let normalized: Vec<_> = routed
        .iter()
        .filter(|r| valid_relation(&r.relation))
        .map(|r| (r, normalize_route_points(&r.points)))
        .filter(|(_, p)| p.len() >= 2)
        .collect();
    let mut hits = Vec::new();
    for (left_index, (left, a)) in normalized.iter().enumerate() {
        for (right, b) in normalized.iter().skip(left_index + 1) {
            let shared = share_endpoint(&left.relation, &right.relation);
            let legacy = options.contract == InputContract::Legacy217;
            if legacy && shared {
                continue;
            }
            let counterflow_only = shared
                && !options
                    .include_shared_endpoints
                    .is_some_and(|f| f(&left.relation, &right.relation));
            if !legacy
                && counterflow_only
                && !options
                    .include_shared_endpoint_counterflow
                    .is_some_and(|f| f(&left.relation, &right.relation))
            {
                continue;
            }
            let mut longest: Option<CorridorHit> = None;
            for i in 0..a.len() - 1 {
                for j in 0..b.len() - 1 {
                    let Some(overlap) = collinear_axis_overlap(a[i], a[i + 1], b[j], b[j + 1])
                    else {
                        continue;
                    };
                    if overlap.length + EPSILON < options.min_overlap_px {
                        continue;
                    }
                    if !legacy {
                        if options.allow_short_workflow_trunks
                            && short_workflow_trunk(left, right, a, b, i, j, overlap.length)
                        {
                            continue;
                        }
                        let shared_source_tick = left.source_endpoint
                            && right.source_endpoint
                            && left.relation["from"] == right.relation["from"]
                            && i == 0
                            && j == 0
                            && (a[0].x - b[0].x).abs() < EPSILON
                            && (a[0].y - b[0].y).abs() < EPSILON;
                        if same_junction(&left.relation, &right.relation)
                            && ((overlap.start.y - overlap.end.y).abs() < EPSILON
                                || left.relation["to"] == right.relation["to"]
                                || shared_source_tick)
                        {
                            continue;
                        }
                        if counterflow_only
                            && (a[i + 1].x - a[i].x) * (b[j + 1].x - b[j].x)
                                + (a[i + 1].y - a[i].y) * (b[j + 1].y - b[j].y)
                                >= 0.0
                        {
                            continue;
                        }
                    }
                    if longest
                        .as_ref()
                        .is_none_or(|hit| overlap.length > hit.overlap_length + EPSILON)
                    {
                        longest = Some(CorridorHit {
                            left_relation_index: left.relation_index,
                            right_relation_index: right.relation_index,
                            left_segment: i,
                            right_segment: j,
                            overlap_length: overlap.length,
                            overlap_start: overlap.start,
                            overlap_end: overlap.end,
                        });
                    }
                }
            }
            if let Some(hit) = longest {
                hits.push(hit);
            }
        }
    }
    hits
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrowheadCollision {
    pub left_relation_index: usize,
    pub right_relation_index: usize,
    pub left_tip: Point,
    pub right_tip: Point,
    pub distance: f64,
    pub minimum: f64,
}

// This predicate exists only in the PhoenixV1 frozen source. It is an explicit
// new-contract check, not evidence that legacy had an arrowhead collision rule.
pub fn collect_arrowhead_collisions(
    routed: &[RoutedRelation],
    allow_short_workflow_trunks: bool,
) -> Vec<ArrowheadCollision> {
    let mut incoming: Vec<(&RoutedRelation, Vec<Point>, usize, f64, f64)> = Vec::new();
    let mut hits = Vec::new();
    for entry in routed {
        let points = normalize_route_points(&entry.points);
        if points.len() < 2 || !truthy(&entry.relation["to"]) {
            continue;
        }
        let tip = points[points.len() - 1];
        let previous = points[points.len() - 2];
        let axis = if (tip.x - previous.x).abs() < EPSILON {
            0
        } else {
            1
        };
        let delta = component(tip, 1 - axis) - component(previous, 1 - axis);
        let direction = if delta == 0.0 { 0.0 } else { delta.signum() };
        let half_width = 3.5
            * entry.relation["width"]
                .as_f64()
                .filter(|n| *n != 0.0)
                .unwrap_or(if entry.relation["variant"] == "emphasis" {
                    1.8
                } else {
                    1.5
                });
        for (sibling, sibling_points, sibling_axis, sibling_direction, sibling_half) in &incoming {
            if sibling.relation["to"] != entry.relation["to"]
                || *sibling_axis != axis
                || *sibling_direction != direction
            {
                continue;
            }
            if same_junction(&sibling.relation, &entry.relation) {
                continue;
            }
            let sibling_tip = *sibling_points.last().unwrap();
            if (component(tip, 1 - axis) - component(sibling_tip, 1 - axis)).abs() > EPSILON {
                continue;
            }
            let distance = (component(tip, axis) - component(sibling_tip, axis)).abs();
            let minimum = half_width + sibling_half;
            if allow_short_workflow_trunks {
                if let Some(overlap) = collinear_axis_overlap(
                    sibling_points[sibling_points.len() - 2],
                    sibling_tip,
                    points[points.len() - 2],
                    tip,
                ) {
                    if short_workflow_trunk(
                        sibling,
                        entry,
                        sibling_points,
                        &points,
                        sibling_points.len() - 2,
                        points.len() - 2,
                        overlap.length,
                    ) {
                        continue;
                    }
                }
            }
            if distance < minimum - EPSILON {
                hits.push(ArrowheadCollision {
                    left_relation_index: sibling.relation_index,
                    right_relation_index: entry.relation_index,
                    left_tip: sibling_tip,
                    right_tip: tip,
                    distance,
                    minimum,
                });
            }
        }
        incoming.push((entry, points, axis, direction, half_width));
    }
    hits
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorderSegment {
    pub side: &'static str,
    pub start: Point,
    pub end: Point,
}

pub fn frame_border_segments(frame: &Frame) -> Vec<BorderSegment> {
    if let Some((start, end)) = frame.line {
        return if finite_point(start) && finite_point(end) {
            vec![BorderSegment {
                side: "line",
                start,
                end,
            }]
        } else {
            vec![]
        };
    }
    let r = frame.bounds;
    if !finite_rect(&r) || r.width <= 0.0 || r.height <= 0.0 {
        return vec![];
    }
    let radius = if frame.radius.is_nan() {
        0.0
    } else {
        frame.radius
    }
    .min(r.width / 2.0)
    .min(r.height / 2.0)
    .max(0.0);
    let (left, right, top, bottom) = (r.x, r.x + r.width, r.y, r.y + r.height);
    [
        BorderSegment {
            side: "top",
            start: Point {
                x: left + radius,
                y: top,
            },
            end: Point {
                x: right - radius,
                y: top,
            },
        },
        BorderSegment {
            side: "right",
            start: Point {
                x: right,
                y: top + radius,
            },
            end: Point {
                x: right,
                y: bottom - radius,
            },
        },
        BorderSegment {
            side: "bottom",
            start: Point {
                x: right - radius,
                y: bottom,
            },
            end: Point {
                x: left + radius,
                y: bottom,
            },
        },
        BorderSegment {
            side: "left",
            start: Point {
                x: left,
                y: bottom - radius,
            },
            end: Point {
                x: left,
                y: top + radius,
            },
        },
    ]
    .into_iter()
    .filter(|s| (s.end.x - s.start.x).hypot(s.end.y - s.start.y) > EPSILON)
    .collect()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorderRun {
    pub relation_index: usize,
    pub frame_index: usize,
    pub side: &'static str,
    pub segment_index: usize,
    pub overlap_length: f64,
    pub overlap_start: Point,
    pub overlap_end: Point,
}

pub fn collect_border_runs(routed: &[RoutedRelation], frames: &[Frame]) -> Vec<BorderRun> {
    let mut hits = Vec::new();
    for route in routed {
        let segments = route.segments.clone().unwrap_or_else(|| {
            route
                .points
                .windows(2)
                .map(|p| Segment {
                    start: p[0],
                    end: p[1],
                })
                .collect()
        });
        if segments.is_empty()
            || !segments
                .iter()
                .all(|s| finite_point(s.start) && finite_point(s.end))
        {
            continue;
        }
        for (frame_index, frame) in frames.iter().enumerate() {
            for border in frame_border_segments(frame) {
                let horizontal = (border.start.y - border.end.y).abs() <= EPSILON;
                let axis = if horizontal { 0 } else { 1 };
                let mut intervals = Vec::new();
                let mut first = usize::MAX;
                for (index, segment) in segments.iter().enumerate() {
                    if let Some(overlap) =
                        collinear_axis_overlap(segment.start, segment.end, border.start, border.end)
                    {
                        if overlap.length <= EPSILON {
                            continue;
                        }
                        first = first.min(index);
                        intervals.push((
                            component(overlap.start, axis).min(component(overlap.end, axis)),
                            component(overlap.start, axis).max(component(overlap.end, axis)),
                        ));
                    }
                }
                intervals.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
                let mut merged: Vec<(f64, f64)> = Vec::new();
                for (low, high) in intervals {
                    if let Some(previous) = merged.last_mut().filter(|p| low <= p.1 + EPSILON) {
                        previous.1 = previous.1.max(high);
                    } else {
                        merged.push((low, high));
                    }
                }
                if merged.is_empty() {
                    continue;
                }
                // Stable tie-break: the lowest interval wins among equal lengths.
                let longest = merged
                    .iter()
                    .reduce(|a, b| if b.1 - b.0 > a.1 - a.0 { b } else { a })
                    .unwrap();
                let fixed = component(border.start, 1 - axis);
                hits.push(BorderRun {
                    relation_index: route.relation_index,
                    frame_index,
                    side: border.side,
                    segment_index: first,
                    overlap_length: merged.iter().map(|p| p.1 - p.0).sum(),
                    overlap_start: if horizontal {
                        Point {
                            x: longest.0,
                            y: fixed,
                        }
                    } else {
                        Point {
                            x: fixed,
                            y: longest.0,
                        }
                    },
                    overlap_end: if horizontal {
                        Point {
                            x: longest.1,
                            y: fixed,
                        }
                    } else {
                        Point {
                            x: fixed,
                            y: longest.1,
                        }
                    },
                });
            }
        }
    }
    hits
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteBudgetOptions {
    pub bends_per_relationship: usize,
    pub stretch: f64,
    pub segment_px: f64,
    pub micro_segment_px: f64,
}
impl Default for RouteBudgetOptions {
    fn default() -> Self {
        Self {
            bends_per_relationship: 2,
            stretch: 1.35,
            segment_px: 16.0,
            micro_segment_px: 8.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteBudgetMetrics {
    pub max_bends: usize,
    pub routes_over_suggested_bends: usize,
    pub max_stretch: Option<f64>,
    pub routes_over_suggested_stretch: usize,
    pub min_segment_px: Option<f64>,
    pub min_interior_segment_px: Option<f64>,
    pub short_segment_count: usize,
    pub short_endpoint_segment_count: usize,
    pub short_interior_segment_count: usize,
    pub micro_segment_count: usize,
}

pub fn segment_position(index: usize, segment_count: usize) -> &'static str {
    if index == 0 {
        "source-stub"
    } else if index == segment_count - 1 {
        "target-stub"
    } else {
        "interior"
    }
}

fn manhattan(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

pub fn route_budget_metrics(
    routed: &[RoutedRelation],
    options: &RouteBudgetOptions,
) -> RouteBudgetMetrics {
    let mut metrics = RouteBudgetMetrics::default();
    for route in routed {
        let points = normalize_route_points(&route.points);
        if points.len() < 2 {
            continue;
        }
        let bends = points.len() - 2;
        metrics.max_bends = metrics.max_bends.max(bends);
        if bends > options.bends_per_relationship {
            metrics.routes_over_suggested_bends += 1;
        }
        let mut route_length = 0.0;
        for (index, pair) in points.windows(2).enumerate() {
            let length = manhattan(pair[0], pair[1]);
            if length <= EPSILON {
                continue;
            }
            let interior = segment_position(index, points.len() - 1) == "interior";
            route_length += length;
            metrics.min_segment_px = Some(metrics.min_segment_px.map_or(length, |p| p.min(length)));
            if interior {
                metrics.min_interior_segment_px = Some(
                    metrics
                        .min_interior_segment_px
                        .map_or(length, |p| p.min(length)),
                );
            }
            if length < options.segment_px {
                metrics.short_segment_count += 1;
                if interior {
                    metrics.short_interior_segment_count += 1;
                } else {
                    metrics.short_endpoint_segment_count += 1;
                }
            }
            if length < options.micro_segment_px {
                metrics.micro_segment_count += 1;
            }
        }
        let direct = manhattan(points[0], *points.last().unwrap());
        if direct > EPSILON {
            let stretch = route_length / direct;
            metrics.max_stretch = Some(metrics.max_stretch.map_or(stretch, |p| p.max(stretch)));
            if stretch > options.stretch + EPSILON {
                metrics.routes_over_suggested_stretch += 1;
            }
        }
    }
    metrics
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteRhythmIssue {
    pub code: &'static str,
    pub relation_index: usize,
    pub segment_index: usize,
    pub position: &'static str,
    pub length: f64,
    pub start: Point,
    pub end: Point,
}

pub fn collect_route_rhythm_issues(
    routed: &[RoutedRelation],
    interior_segment_px: f64,
    micro_segment_px: f64,
) -> Vec<RouteRhythmIssue> {
    let mut issues = Vec::new();
    for route in routed {
        let points = normalize_route_points(&route.points);
        for (index, pair) in points.windows(2).enumerate() {
            let length = manhattan(pair[0], pair[1]);
            if length <= EPSILON {
                continue;
            }
            let position = segment_position(index, points.len() - 1);
            let code = if length < micro_segment_px - EPSILON {
                "composition/micro-segment"
            } else if position == "interior" && length < interior_segment_px - EPSILON {
                "composition/short-interior-segment"
            } else {
                continue;
            };
            issues.push(RouteRhythmIssue {
                code,
                relation_index: route.relation_index,
                segment_index: index,
                position,
                length,
                start: pair[0],
                end: pair[1],
            });
        }
    }
    issues
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelRect {
    pub relation_index: isize,
    pub relation: Value,
    pub bounds: Rect,
    pub label: String,
}

impl LabelRect {
    pub fn new(relation_index: isize, relation: Value, bounds: Rect) -> Self {
        let label = text(&relation, "label").to_owned();
        Self {
            relation_index,
            relation,
            bounds,
            label,
        }
    }
}

fn relationship_identity(relation: &Value, index: isize) -> String {
    if let Some(key) = relation.get("key") {
        return format!("key:{}", js_string(key));
    }
    if truthy(&relation["id"]) {
        return format!(
            "id:{}\0{}\0{}",
            text(relation, "from"),
            text(relation, "to"),
            js_string(&relation["id"])
        );
    }
    format!("index:{index}")
}

fn js_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn same_relationship(a: &Value, b: &Value) -> bool {
    if a.is_null() || b.is_null() {
        return false;
    }
    if let (Some(left), Some(right)) = (a.get("key"), b.get("key")) {
        return left == right;
    }
    truthy(&a["id"])
        && truthy(&b["id"])
        && a["id"] == b["id"]
        && a["from"] == b["from"]
        && a["to"] == b["to"]
}

pub fn segment_rect_intersection_length(start: Point, end: Point, rect: &Rect) -> Option<f64> {
    if !finite_point(start)
        || !finite_point(end)
        || !finite_rect(rect)
        || rect.width < 0.0
        || rect.height < 0.0
    {
        return None;
    }
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = dx.hypot(dy);
    if length <= 0.0000001 {
        return Some(0.0);
    }
    let mut enter: f64 = 0.0;
    let mut leave: f64 = 1.0;
    for (direction, distance) in [
        (-dx, start.x - rect.x),
        (dx, rect.x + rect.width - start.x),
        (-dy, start.y - rect.y),
        (dy, rect.y + rect.height - start.y),
    ] {
        if direction.abs() <= 0.0000001 {
            if distance < -0.0000001 {
                return Some(0.0);
            }
            continue;
        }
        let ratio = distance / direction;
        if direction < 0.0 {
            enter = enter.max(ratio);
        } else {
            leave = leave.min(ratio);
        }
        if enter > leave + 0.0000001 {
            return Some(0.0);
        }
    }
    Some(length * (leave - enter).max(0.0))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelRouteClearance {
    pub label_index: usize,
    pub label_relation_index: isize,
    pub other_relation_index: usize,
    pub rect: Rect,
    pub clearance: f64,
    pub intersection_length: f64,
    pub segment_index: usize,
    pub start: Point,
    pub end: Point,
    pub threshold: f64,
}

pub fn collect_label_route_clearance(
    labels: &[LabelRect],
    routed: &[RoutedRelation],
    threshold: f64,
) -> Vec<LabelRouteClearance> {
    if !threshold.is_finite() || threshold <= 0.0 {
        return vec![];
    }
    let mut seen_routes = HashSet::new();
    let routes: Vec<_> = routed
        .iter()
        .map(|r| (r, normalize_route_points(&r.points)))
        .filter(|(r, p)| {
            p.len() >= 2
                && seen_routes.insert(relationship_identity(
                    &r.relation,
                    r.relation_index as isize,
                ))
        })
        .collect();
    let mut seen_labels = HashSet::new();
    let mut hits = Vec::new();
    for (label_index, label) in labels.iter().enumerate() {
        let rect = label.bounds;
        if !finite_rect(&rect)
            || rect.width < 0.0
            || rect.height < 0.0
            || !seen_labels.insert(relationship_identity(&label.relation, label.relation_index))
        {
            continue;
        }
        for (route, points) in &routes {
            if label.relation_index == route.relation_index as isize
                || same_relationship(&label.relation, &route.relation)
            {
                continue;
            }
            let mut nearest: Option<LabelRouteClearance> = None;
            for (index, pair) in points.windows(2).enumerate() {
                let clearance = segment_rect_clearance(pair[0], pair[1], &rect);
                if !clearance.is_finite() {
                    continue;
                }
                if nearest.as_ref().is_none_or(|hit| clearance < hit.clearance) {
                    nearest = Some(LabelRouteClearance {
                        label_index,
                        label_relation_index: label.relation_index,
                        other_relation_index: route.relation_index,
                        rect,
                        clearance,
                        intersection_length: segment_rect_intersection_length(
                            pair[0], pair[1], &rect,
                        )
                        .unwrap_or(0.0),
                        segment_index: index,
                        start: pair[0],
                        end: pair[1],
                        threshold,
                    });
                }
            }
            if let Some(hit) = nearest.filter(|hit| hit.clearance + EPSILON < threshold) {
                hits.push(hit);
            }
        }
    }
    hits
}

pub fn minimum_label_route_clearance(hits: &[LabelRouteClearance]) -> Option<f64> {
    hits.iter()
        .map(|hit| hit.clearance)
        .reduce(f64::min)
        .map(|n| (n * 10.0 + 0.5).floor() / 10.0)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelCanvasOverflow {
    pub label_index: usize,
    pub relation_index: isize,
    pub rect: Rect,
    pub view_box: [f64; 2],
    pub view_box_origin: [f64; 2],
    pub sides: Vec<&'static str>,
    pub overflow_px: Vec<(&'static str, f64)>,
    pub tolerance: f64,
}

// Like arrowhead collisions, containment was added in the PhoenixV1 baseline.
pub fn collect_label_canvas_overflow(
    labels: &[LabelRect],
    view_box: &[f64],
    tolerance: f64,
) -> Vec<LabelCanvasOverflow> {
    let (x, y, width, height) = match view_box {
        [x, y, w, h] => (*x, *y, *w, *h),
        [w, h] => (0.0, 0.0, *w, *h),
        _ => return vec![],
    };
    if ![x, y, width, height].iter().all(|v| v.is_finite()) {
        return vec![];
    }
    let mut hits = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        let r = label.bounds;
        if !finite_rect(&r) || r.width < 0.0 || r.height < 0.0 {
            continue;
        }
        let overflow: Vec<_> = [
            ("left", x - r.x),
            ("right", r.x + r.width - x - width),
            ("top", y - r.y),
            ("bottom", r.y + r.height - y - height),
        ]
        .into_iter()
        .filter(|(_, n)| *n > tolerance)
        .map(|(side, n)| (side, (n * 10.0 + 0.5).floor() / 10.0))
        .collect();
        if overflow.is_empty() {
            continue;
        }
        hits.push(LabelCanvasOverflow {
            label_index: index,
            relation_index: label.relation_index,
            rect: r,
            view_box: [width, height],
            view_box_origin: [x, y],
            sides: overflow.iter().map(|(side, _)| *side).collect(),
            overflow_px: overflow,
            tolerance,
        });
    }
    hits
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSideIssue {
    pub endpoint: &'static str,
    pub side: String,
    pub segment_index: usize,
    pub start: Point,
    pub end: Point,
    pub expected_axis: &'static str,
    pub expected_direction: &'static str,
}

// Resolve authored versus inferred sides before calling: a named/via route
// without authored sides must not acquire an invented direction contract.
pub fn collect_endpoint_side_issues(
    points: &[Point],
    from_side: &str,
    to_side: &str,
) -> Vec<EndpointSideIssue> {
    let points = normalize_route_points(points);
    if points.len() < 2 {
        return vec![];
    }
    let mut issues = Vec::new();
    for (endpoint, side) in [("source", from_side), ("target", to_side)] {
        let source = endpoint == "source";
        let (horizontal, source_sign, source_direction, target_direction) = match side {
            "left" => (true, -1.0, "leftward", "rightward from the left"),
            "right" => (true, 1.0, "rightward", "leftward from the right"),
            "top" => (false, -1.0, "upward", "downward from above"),
            "bottom" => (false, 1.0, "downward", "upward from below"),
            _ => continue,
        };
        let segment_index = if source { 0 } else { points.len() - 2 };
        let (start, end) = (points[segment_index], points[segment_index + 1]);
        let (along, across) = if horizontal {
            (end.x - start.x, end.y - start.y)
        } else {
            (end.y - start.y, end.x - start.x)
        };
        let expected_sign = if source { source_sign } else { -source_sign };
        if across.abs() <= EPSILON && along * expected_sign > EPSILON {
            continue;
        }
        issues.push(EndpointSideIssue {
            endpoint,
            side: side.into(),
            segment_index,
            start,
            end,
            expected_axis: if horizontal { "horizontal" } else { "vertical" },
            expected_direction: if source {
                source_direction
            } else {
                target_direction
            },
        });
    }
    issues
}

pub fn route_honors_endpoint_sides(points: &[Point], from_side: &str, to_side: &str) -> bool {
    collect_endpoint_side_issues(points, from_side, to_side).is_empty()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSegment {
    pub start: Point,
    pub end: Point,
    pub segment_index: usize,
    pub source_segments: Vec<(usize, Segment)>,
}

fn segments_continue_forward(a: Point, b: Point, c: Point, d: Point) -> bool {
    if ![a, b, c, d].into_iter().all(finite_point)
        || (b.x - c.x).abs() > EPSILON
        || (b.y - c.y).abs() > EPSILON
    {
        return false;
    }
    let (ax, ay, bx, by) = (b.x - a.x, b.y - a.y, d.x - c.x, d.y - c.y);
    ax.hypot(ay) > EPSILON
        && bx.hypot(by) > EPSILON
        && (ax * by - ay * bx).abs() <= EPSILON
        && ax * bx + ay * by > EPSILON
}

fn authored_analysis_segments(points: &[Point]) -> Vec<AnalysisSegment> {
    points
        .windows(2)
        .enumerate()
        .map(|(index, p)| AnalysisSegment {
            start: p[0],
            end: p[1],
            segment_index: index,
            source_segments: vec![(
                index,
                Segment {
                    start: p[0],
                    end: p[1],
                },
            )],
        })
        .collect()
}

// Unlike normalize_route_points, this analysis does not drop authored zero
// segments or mutate output geometry. It retains source indexes at a waypoint.
pub fn forward_collinear_analysis_segments(points: &[Point]) -> Vec<AnalysisSegment> {
    let mut segments: Vec<AnalysisSegment> = Vec::new();
    for segment in authored_analysis_segments(points) {
        if let Some(previous) = segments
            .last_mut()
            .filter(|p| segments_continue_forward(p.start, p.end, segment.start, segment.end))
        {
            previous.end = segment.end;
            previous.source_segments.extend(segment.source_segments);
        } else {
            segments.push(segment);
        }
    }
    segments
}

fn point_lies_on_segment(point: Point, start: Point, end: Point) -> bool {
    if ![point, start, end].into_iter().all(finite_point) {
        return false;
    }
    let length = (end.x - start.x).hypot(end.y - start.y);
    if length <= EPSILON {
        return near(point, start);
    }
    cross_product(start, end, point).abs() <= EPSILON * length
        && point.x >= start.x.min(end.x) - EPSILON
        && point.x <= start.x.max(end.x) + EPSILON
        && point.y >= start.y.min(end.y) - EPSILON
        && point.y <= start.y.max(end.y) + EPSILON
}

pub fn source_segment_index_at_point(segment: &AnalysisSegment, point: Point) -> usize {
    segment
        .source_segments
        .iter()
        .find(|(_, s)| point_lies_on_segment(point, s.start, s.end))
        .map_or(segment.segment_index, |(index, _)| *index)
}

pub fn polyline_bend_contacts(left: &[Point], right: &[Point]) -> Vec<Point> {
    let mut contacts = Vec::new();
    for (points, other) in [(left, right), (right, left)] {
        if other.len() < 2 {
            continue;
        }
        for triple in points.windows(3) {
            let point = triple[1];
            if cross_product(triple[0], point, triple[2]).abs() <= EPSILON
                || near(point, other[0])
                || near(point, *other.last().unwrap())
            {
                continue;
            }
            if other
                .windows(2)
                .any(|pair| point_lies_on_segment(point, pair[0], pair[1]))
                && !contacts.iter().any(|&p| near(point, p))
            {
                contacts.push(point);
            }
        }
    }
    contacts
}

pub struct CrossingOptions<'a> {
    pub contract: InputContract,
    pub merge_forward_collinear_waypoints: bool,
    pub include_shared_endpoints: Option<RelationPairPredicate<'a>>,
    pub crossing_resolved: Option<&'a dyn Fn(&Value, &Value, &ProperCrossing) -> bool>,
}
impl Default for CrossingOptions<'_> {
    fn default() -> Self {
        Self {
            contract: InputContract::PhoenixV1,
            merge_forward_collinear_waypoints: false,
            include_shared_endpoints: None,
            crossing_resolved: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProperCrossing {
    pub left_relation_index: usize,
    pub right_relation_index: usize,
    pub point: Point,
    pub left_segment: usize,
    pub right_segment: usize,
}

pub fn collect_proper_crossings(
    routed: &[RoutedRelation],
    options: &CrossingOptions<'_>,
) -> Vec<ProperCrossing> {
    let routes: Vec<_> = routed
        .iter()
        .filter(|r| {
            valid_relation(&r.relation)
                && r.points.len() >= 2
                && r.points.iter().copied().all(finite_point)
        })
        .map(|r| {
            (
                r,
                if options.merge_forward_collinear_waypoints {
                    forward_collinear_analysis_segments(&r.points)
                } else {
                    authored_analysis_segments(&r.points)
                },
            )
        })
        .collect();
    let mut hits = Vec::new();
    for (index, (left, a)) in routes.iter().enumerate() {
        for (right, b) in routes.iter().skip(index + 1) {
            if share_endpoint(&left.relation, &right.relation)
                && (options.contract == InputContract::Legacy217
                    || !options
                        .include_shared_endpoints
                        .is_some_and(|f| f(&left.relation, &right.relation)))
            {
                continue;
            }
            let mut hit = None;
            'segments: for l in a {
                for r in b {
                    if let Some(point) = proper_segment_intersection(l.start, l.end, r.start, r.end)
                    {
                        hit = Some(ProperCrossing {
                            left_relation_index: left.relation_index,
                            right_relation_index: right.relation_index,
                            point,
                            left_segment: source_segment_index_at_point(l, point),
                            right_segment: source_segment_index_at_point(r, point),
                        });
                        break 'segments;
                    }
                }
            }
            if let Some(hit) = hit {
                if options.contract != InputContract::Legacy217
                    && options
                        .crossing_resolved
                        .is_some_and(|f| f(&left.relation, &right.relation, &hit))
                {
                    continue;
                }
                hits.push(hit);
            }
        }
    }
    hits
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Obstacle {
    pub id: String,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteObstacleHit {
    pub relation_index: usize,
    pub obstacle_index: usize,
    pub obstacle_id: String,
    pub segment_index: usize,
    pub start: Point,
    pub end: Point,
    pub clearance: f64,
}

// Supply opaque semantic entities only, not frames/lifelines that allow passage.
// As in cleanFlowProblems, both relationship endpoints must be known obstacles.
pub fn collect_route_obstacle_hits(
    routed: &[RoutedRelation],
    obstacles: &[Obstacle],
    clearance: f64,
) -> Vec<RouteObstacleHit> {
    let ids: HashSet<_> = obstacles.iter().map(|o| o.id.as_str()).collect();
    let mut hits = Vec::new();
    for route in routed {
        let relation = &route.relation;
        if !valid_relation(relation)
            || !ids.contains(text(relation, "from"))
            || !ids.contains(text(relation, "to"))
            || route.points.len() < 2
            || !route.points.iter().copied().all(finite_point)
        {
            continue;
        }
        for (obstacle_index, obstacle) in obstacles.iter().enumerate() {
            if obstacle.id == text(relation, "from")
                || obstacle.id == text(relation, "to")
                || !finite_rect(&obstacle.bounds)
            {
                continue;
            }
            if let Some((segment_index, pair)) =
                route.points.windows(2).enumerate().find(|(_, pair)| {
                    segment_intersects_rect(pair[0], pair[1], &obstacle.bounds, clearance)
                })
            {
                hits.push(RouteObstacleHit {
                    relation_index: route.relation_index,
                    obstacle_index,
                    obstacle_id: obstacle.id.clone(),
                    segment_index,
                    start: pair[0],
                    end: pair[1],
                    clearance,
                });
            }
        }
    }
    hits
}
