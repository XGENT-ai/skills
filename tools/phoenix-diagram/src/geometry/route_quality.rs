// Ported from the frozen shared/route-quality.mjs; see NOTICE.md.
use crate::diagnostic::{Diagnostic, Severity, Subject};
use crate::geometry::{
    finite_rect, normalize_route_points, proper_segment_intersection, segment_intersects_rect,
    Point, Rect, EPSILON,
};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start: Point,
    pub end: Point,
}

pub struct GridRouteOptions<'a> {
    pub start: Point,
    pub end: Point,
    pub points: &'a [Point],
    pub obstacles: &'a [Rect],
    pub from_side: &'a str,
    pub to_side: &'a str,
    pub clearance: f64,
    pub maximum_obstacle_count: usize,
    pub endpoint_stub_px: Option<f64>,
    pub maximum_grid_nodes: f64,
    pub avoided_segments: &'a [Segment],
    pub allow_avoided_crossings: bool,
    pub minimum_avoided_overlap_px: f64,
    pub route_separation_px: f64,
    pub minimum_segment_px: f64,
    pub border_segments: &'a [Segment],
    pub bend_penalty_px: f64,
}

impl<'a> GridRouteOptions<'a> {
    pub fn new(
        start: Point,
        end: Point,
        points: &'a [Point],
        obstacles: &'a [Rect],
        from_side: &'a str,
        to_side: &'a str,
    ) -> Self {
        Self {
            start,
            end,
            points,
            obstacles,
            from_side,
            to_side,
            clearance: 2.0,
            maximum_obstacle_count: 80,
            endpoint_stub_px: None,
            maximum_grid_nodes: f64::INFINITY,
            avoided_segments: &[],
            allow_avoided_crossings: false,
            minimum_avoided_overlap_px: 8.0,
            route_separation_px: 8.0,
            minimum_segment_px: 8.0,
            border_segments: &[],
            bend_penalty_px: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridMetrics {
    pub status: String,
    pub maximum_grid_nodes: f64,
    pub obstacle_count: usize,
    pub avoided_segment_count: usize,
    pub coordinate_count: usize,
    pub candidate_node_count: usize,
    pub usable_node_count: usize,
    pub graph_edge_count: usize,
    pub visited_node_count: usize,
    pub attempt_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridRoute {
    pub points: Vec<Point>,
    pub length: f64,
    pub obstacle_count: usize,
}
#[derive(Clone, Debug)]
pub struct GridSearch {
    pub route: Option<GridRoute>,
    pub metrics: GridMetrics,
}

pub fn shortest_orthogonal_grid_route(options: &GridRouteOptions<'_>) -> GridSearch {
    shortest_orthogonal_grid_route_accept(options, |_| true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Right = 0,
    Left = 1,
    Down = 2,
    Up = 3,
}

impl Direction {
    fn for_side(side: &str) -> Option<Self> {
        match side {
            "right" => Some(Self::Right),
            "left" => Some(Self::Left),
            "bottom" => Some(Self::Down),
            "top" => Some(Self::Up),
            _ => None,
        }
    }
    fn opposite(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Down => Self::Up,
            Self::Up => Self::Down,
        }
    }
    fn for_state(state: usize) -> Self {
        match state % 4 {
            0 => Self::Right,
            1 => Self::Left,
            2 => Self::Down,
            _ => Self::Up,
        }
    }
    fn outward(self, point: Point, distance: f64) -> Point {
        match self {
            Self::Right => Point {
                x: point.x + distance,
                ..point
            },
            Self::Left => Point {
                x: point.x - distance,
                ..point
            },
            Self::Down => Point {
                y: point.y + distance,
                ..point
            },
            Self::Up => Point {
                y: point.y - distance,
                ..point
            },
        }
    }
}

#[derive(Clone, Copy)]
struct QueueEntry {
    key: usize,
    distance: f64,
}

// Equal distances retain the source heap's left-child and insertion tie-breaks.
struct MinHeap {
    entries: Vec<QueueEntry>,
}
impl MinHeap {
    fn push(&mut self, key: usize, distance: f64) {
        let entry = QueueEntry { key, distance };
        self.entries.push(entry);
        let mut index = self.entries.len() - 1;
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.entries[parent].distance <= distance {
                break;
            }
            self.entries[index] = self.entries[parent];
            index = parent;
        }
        self.entries[index] = entry;
    }

    fn pop(&mut self) -> Option<QueueEntry> {
        let first = *self.entries.first()?;
        let last = self.entries.pop().expect("a first entry exists");
        if self.entries.is_empty() {
            return Some(first);
        }
        let mut index = 0;
        loop {
            let left = index * 2 + 1;
            let right = left + 1;
            if left >= self.entries.len() {
                break;
            }
            let child = if right < self.entries.len()
                && self.entries[right].distance < self.entries[left].distance
            {
                right
            } else {
                left
            };
            if self.entries[child].distance >= last.distance {
                break;
            }
            self.entries[index] = self.entries[child];
            index = child;
        }
        self.entries[index] = last;
        Some(first)
    }
}

fn orthogonal_length(points: &[Point]) -> Option<f64> {
    let mut length = 0.0;
    for pair in points.windows(2) {
        if pair[0].x != pair[1].x && pair[0].y != pair[1].y {
            return None;
        }
        length += (pair[1].x - pair[0].x).abs() + (pair[1].y - pair[0].y).abs();
    }
    Some(length)
}

fn segment_blocked(start: Point, end: Point, obstacles: &[Rect]) -> bool {
    let (min_x, max_x, min_y, max_y) = (
        start.x.min(end.x),
        start.x.max(end.x),
        start.y.min(end.y),
        start.y.max(end.y),
    );
    obstacles.iter().any(|rect| {
        if max_x < rect.x.min(rect.x + rect.width) - EPSILON
            || min_x > rect.x.max(rect.x + rect.width) + EPSILON
            || max_y < rect.y.min(rect.y + rect.height) - EPSILON
            || min_y > rect.y.max(rect.y + rect.height) + EPSILON
        {
            return false;
        }
        segment_intersects_rect(start, end, rect, 0.0)
    })
}

fn orthogonal_touch_on_avoided_interior(start: Point, end: Point, avoided: Segment) -> bool {
    if (start.y - end.y).abs() <= EPSILON && (avoided.start.x - avoided.end.x).abs() <= EPSILON {
        avoided.start.x >= start.x.min(end.x) - EPSILON
            && avoided.start.x <= start.x.max(end.x) + EPSILON
            && start.y > avoided.start.y.min(avoided.end.y) + EPSILON
            && start.y < avoided.start.y.max(avoided.end.y) - EPSILON
    } else if (start.x - end.x).abs() <= EPSILON
        && (avoided.start.y - avoided.end.y).abs() <= EPSILON
    {
        avoided.start.y >= start.y.min(end.y) - EPSILON
            && avoided.start.y <= start.y.max(end.y) + EPSILON
            && start.x > avoided.start.x.min(avoided.end.x) + EPSILON
            && start.x < avoided.start.x.max(avoided.end.x) - EPSILON
    } else {
        false
    }
}

fn collinear_overlap(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if a.x == b.x && c.x == d.x && a.x == c.x {
        return (a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y))).max(0.0);
    }
    if a.y == b.y && c.y == d.y && a.y == c.y {
        return (a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x))).max(0.0);
    }
    0.0
}

fn segment_conflicts_with_avoided(
    start: Point,
    end: Point,
    avoided: &[Segment],
    minimum_overlap: f64,
    allow_crossings: bool,
) -> bool {
    let (min_x, max_x, min_y, max_y) = (
        start.x.min(end.x),
        start.x.max(end.x),
        start.y.min(end.y),
        start.y.max(end.y),
    );
    avoided.iter().any(|other| {
        if minimum_overlap > 0.0
            && (max_x < other.start.x.min(other.end.x) - EPSILON
                || min_x > other.start.x.max(other.end.x) + EPSILON
                || max_y < other.start.y.min(other.end.y) - EPSILON
                || min_y > other.start.y.max(other.end.y) + EPSILON)
        {
            return false;
        }
        (!allow_crossings
            && (proper_segment_intersection(start, end, other.start, other.end).is_some()
                || orthogonal_touch_on_avoided_interior(start, end, *other)))
            || collinear_overlap(start, end, other.start, other.end) >= minimum_overlap
    })
}

fn point_on_avoided_interior(point: Point, avoided: &[Segment]) -> bool {
    avoided.iter().any(|segment| {
        let cross = (segment.end.x - segment.start.x) * (point.y - segment.start.y)
            - (segment.end.y - segment.start.y) * (point.x - segment.start.x);
        if cross.abs() > EPSILON {
            return false;
        }
        (point.x - segment.start.x) * (point.x - segment.end.x)
            + (point.y - segment.start.y) * (point.y - segment.end.y)
            < -EPSILON
    })
}

fn add_coordinate(values: &mut Vec<f64>, value: f64) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn border_coordinates(xs: &mut Vec<f64>, ys: &mut Vec<f64>, borders: &[Segment], distance: f64) {
    for segment in borders {
        if (segment.start.x - segment.end.x).abs() <= EPSILON {
            add_coordinate(xs, segment.start.x - distance);
            add_coordinate(xs, segment.start.x + distance);
        }
        if (segment.start.y - segment.end.y).abs() <= EPSILON {
            add_coordinate(ys, segment.start.y - distance);
            add_coordinate(ys, segment.start.y + distance);
        }
    }
}

fn coalesce(
    mut values: Vec<f64>,
    keep: [f64; 2],
    minimum_segment: f64,
    fill_gaps: bool,
) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    if !fill_gaps {
        return values
            .iter()
            .enumerate()
            .filter(|(index, value)| {
                *index == 0
                    || keep.contains(value)
                    || **value - values[*index - 1] >= minimum_segment
            })
            .map(|(_, value)| *value)
            .collect();
    }
    let mut retained = Vec::new();
    for value in values {
        if keep.contains(&value)
            || (retained
                .last()
                .is_none_or(|last| value - last >= minimum_segment)
                && keep
                    .iter()
                    .all(|endpoint| (value - endpoint).abs() >= minimum_segment))
        {
            retained.push(value);
        }
    }
    retained
}

#[derive(Clone, Copy)]
struct Edge {
    node: usize,
    distance: f64,
    direction: Direction,
}

fn connect_line(
    line: &[usize],
    horizontal: bool,
    nodes: &[Option<Point>],
    adjacency: &mut [Vec<Edge>],
    expanded: &[Rect],
    options: &GridRouteOptions<'_>,
) -> usize {
    let mut count = 0;
    for pair in line.windows(2) {
        let (left, right) = (
            nodes[pair[0]].expect("line contains usable nodes"),
            nodes[pair[1]].expect("line contains usable nodes"),
        );
        if segment_blocked(left, right, expanded)
            || segment_conflicts_with_avoided(
                left,
                right,
                options.avoided_segments,
                options.minimum_avoided_overlap_px,
                options.allow_avoided_crossings,
            )
            || options
                .border_segments
                .iter()
                .any(|segment| collinear_overlap(left, right, segment.start, segment.end) > EPSILON)
        {
            continue;
        }
        let distance = (right.x - left.x).abs() + (right.y - left.y).abs();
        adjacency[pair[0]].push(Edge {
            node: pair[1],
            distance,
            direction: if horizontal {
                Direction::Right
            } else {
                Direction::Down
            },
        });
        adjacency[pair[1]].push(Edge {
            node: pair[0],
            distance,
            direction: if horizontal {
                Direction::Left
            } else {
                Direction::Up
            },
        });
        count += 1;
    }
    count
}

pub fn shortest_orthogonal_grid_route_accept(
    options: &GridRouteOptions<'_>,
    mut accept: impl FnMut(&[Point]) -> bool,
) -> GridSearch {
    let mut search = GridSearch {
        route: None,
        metrics: GridMetrics {
            status: "initializing".into(),
            maximum_grid_nodes: options.maximum_grid_nodes,
            obstacle_count: 0,
            avoided_segment_count: 0,
            coordinate_count: 0,
            candidate_node_count: 0,
            usable_node_count: 0,
            graph_edge_count: 0,
            visited_node_count: 0,
            attempt_count: 0,
        },
    };
    let (Some(source_direction), Some(end_direction)) = (
        Direction::for_side(options.from_side),
        Direction::for_side(options.to_side),
    ) else {
        search.metrics.status = "unsupported-endpoint-side".into();
        return search;
    };
    let start_stub = source_direction.outward(
        options.start,
        options.endpoint_stub_px.unwrap_or(options.clearance + 2.0),
    );
    let end_stub = end_direction.outward(
        options.end,
        options.endpoint_stub_px.unwrap_or(options.clearance + 2.0),
    );
    let expanded: Vec<_> = options
        .obstacles
        .iter()
        .filter(|rect| finite_rect(rect))
        .map(|rect| Rect {
            x: rect.x - options.clearance,
            y: rect.y - options.clearance,
            width: rect.width + options.clearance * 2.0,
            height: rect.height + options.clearance * 2.0,
        })
        .collect();
    search.metrics.obstacle_count = expanded.len();
    search.metrics.avoided_segment_count = options.avoided_segments.len();
    if expanded.len() > options.maximum_obstacle_count {
        search.metrics.status = "obstacle-budget-exceeded".into();
        return search;
    }
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for point in [start_stub, end_stub].iter().chain(options.points) {
        add_coordinate(&mut xs, point.x);
        add_coordinate(&mut ys, point.y);
    }
    for rect in &expanded {
        add_coordinate(&mut xs, rect.x - 1.0);
        add_coordinate(&mut xs, rect.x + rect.width + 1.0);
        add_coordinate(&mut ys, rect.y - 1.0);
        add_coordinate(&mut ys, rect.y + rect.height + 1.0);
    }
    for segment in options.avoided_segments {
        for point in [segment.start, segment.end] {
            add_coordinate(&mut xs, point.x);
            add_coordinate(&mut ys, point.y);
        }
        if (segment.start.x - segment.end.x).abs() <= EPSILON {
            add_coordinate(&mut xs, segment.start.x - options.route_separation_px);
            add_coordinate(&mut xs, segment.start.x + options.route_separation_px);
        }
        if (segment.start.y - segment.end.y).abs() <= EPSILON {
            add_coordinate(&mut ys, segment.start.y - options.route_separation_px);
            add_coordinate(&mut ys, segment.start.y + options.route_separation_px);
        }
    }
    border_coordinates(
        &mut xs,
        &mut ys,
        options.border_segments,
        options.route_separation_px,
    );
    let mut remaining_nodes = options.maximum_grid_nodes;
    for attempt in 0..2 {
        if attempt > 0 && options.route_separation_px < options.minimum_segment_px {
            border_coordinates(
                &mut xs,
                &mut ys,
                options.border_segments,
                options.minimum_segment_px,
            );
        }
        let ordered_x = coalesce(
            xs.clone(),
            [start_stub.x, end_stub.x],
            options.minimum_segment_px,
            attempt > 0,
        );
        let ordered_y = coalesce(
            ys.clone(),
            [start_stub.y, end_stub.y],
            options.minimum_segment_px,
            attempt > 0,
        );
        let candidate_count = ordered_x.len() * ordered_y.len();
        search.metrics.coordinate_count += ordered_x.len() + ordered_y.len();
        search.metrics.candidate_node_count += candidate_count;
        search.metrics.attempt_count = attempt + 1;
        if candidate_count as f64 > remaining_nodes {
            search.metrics.status = "node-budget-exceeded".into();
            return search;
        }
        remaining_nodes -= candidate_count as f64;
        let mut nodes = Vec::with_capacity(candidate_count);
        for &x in &ordered_x {
            for &y in &ordered_y {
                let point = Point { x, y };
                let blocked = expanded.iter().any(|rect| {
                    point.x >= rect.x
                        && point.x <= rect.x + rect.width
                        && point.y >= rect.y
                        && point.y <= rect.y + rect.height
                });
                nodes.push(
                    (!blocked
                        && (options.allow_avoided_crossings
                            || !point_on_avoided_interior(point, options.avoided_segments)))
                    .then_some(point),
                );
            }
        }
        search.metrics.usable_node_count += nodes.iter().filter(|node| node.is_some()).count();
        let find_node = |point: Point| nodes.iter().position(|node| *node == Some(point));
        let (Some(source), Some(target)) = (find_node(start_stub), find_node(end_stub)) else {
            search.metrics.status = "endpoint-blocked".into();
            return search;
        };
        let mut adjacency = vec![Vec::new(); candidate_count];
        for y in 0..ordered_y.len() {
            let line: Vec<_> = (0..ordered_x.len())
                .map(|x| x * ordered_y.len() + y)
                .filter(|&index| nodes[index].is_some())
                .collect();
            search.metrics.graph_edge_count +=
                connect_line(&line, true, &nodes, &mut adjacency, &expanded, options);
        }
        for x in 0..ordered_x.len() {
            let line: Vec<_> = (0..ordered_y.len())
                .map(|y| x * ordered_y.len() + y)
                .filter(|&index| nodes[index].is_some())
                .collect();
            search.metrics.graph_edge_count +=
                connect_line(&line, false, &nodes, &mut adjacency, &expanded, options);
        }
        let target_direction = end_direction.opposite();
        let source_state = source * 4 + source_direction as usize;
        let mut distances = vec![f64::INFINITY; candidate_count * 4];
        let mut previous = vec![None; candidate_count * 4];
        distances[source_state] = 0.0;
        let mut queue = MinHeap {
            entries: Vec::new(),
        };
        queue.push(source_state, 0.0);
        let mut target_state: Option<QueueEntry> = None;
        while let Some(current) = queue.pop() {
            if current.distance != distances[current.key] {
                continue;
            }
            search.metrics.visited_node_count += 1;
            let current_node = current.key / 4;
            let current_direction = Direction::for_state(current.key);
            if current_node == target {
                if current_direction == target_direction.opposite() {
                    continue;
                }
                let arrival = current.distance
                    + if current_direction == target_direction {
                        0.0
                    } else {
                        options.bend_penalty_px
                    };
                if target_state.is_none_or(|state| arrival < state.distance) {
                    target_state = Some(QueueEntry {
                        key: current.key,
                        distance: arrival,
                    });
                }
                if current_direction == target_direction || options.bend_penalty_px == 0.0 {
                    break;
                }
                continue;
            }
            if target_state.is_some_and(|state| current.distance >= state.distance) {
                break;
            }
            for neighbor in &adjacency[current_node] {
                if neighbor.direction == current_direction.opposite() {
                    continue;
                }
                let candidate = current.distance
                    + neighbor.distance
                    + if neighbor.direction == current_direction {
                        0.0
                    } else {
                        options.bend_penalty_px
                    };
                let neighbor_state = neighbor.node * 4 + neighbor.direction as usize;
                if candidate >= distances[neighbor_state] {
                    continue;
                }
                distances[neighbor_state] = candidate;
                previous[neighbor_state] = Some(current.key);
                queue.push(neighbor_state, candidate);
            }
        }
        let Some(target_state) = target_state else {
            search.metrics.status = "no-route".into();
            continue;
        };
        let mut reversed = Vec::new();
        let mut key = Some(target_state.key);
        while let Some(state) = key {
            reversed.push(nodes[state / 4].expect("a search state has a usable node"));
            if state == source_state {
                break;
            }
            key = previous[state];
        }
        if reversed.last() != Some(&start_stub) {
            search.metrics.status = "broken-predecessor-chain".into();
            return search;
        }
        reversed.reverse();
        let mut shortest = vec![options.start];
        shortest.extend(reversed);
        shortest.push(options.end);
        let shortest = normalize_route_points(&shortest);
        if !accept(&shortest) {
            search.metrics.status = "rejected".into();
            continue;
        }
        search.metrics.status = "routed".into();
        search.route = Some(GridRoute {
            length: orthogonal_length(&shortest).unwrap_or(f64::NAN),
            points: shortest,
            obstacle_count: expanded.len(),
        });
        return search;
    }
    search
}

#[derive(Clone, Debug)]
pub struct DetourRelation {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub authored_via: bool,
    pub points: Vec<Point>,
    pub from_side: Option<String>,
    pub to_side: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DetourPolicy {
    pub clearance: f64,
    pub minimum_detour_ratio: f64,
    pub minimum_excess_length_px: f64,
    pub minimum_empty_excursion_px: f64,
    pub maximum_obstacle_count: usize,
    pub shared_corridor_minimum_px: f64,
}

impl Default for DetourPolicy {
    fn default() -> Self {
        Self {
            clearance: 2.0,
            minimum_detour_ratio: 2.5,
            minimum_excess_length_px: 200.0,
            minimum_empty_excursion_px: 96.0,
            maximum_obstacle_count: 80,
            shared_corridor_minimum_px: 32.0,
        }
    }
}

pub struct DetourOptions<'a> {
    pub relations: &'a [DetourRelation],
    pub obstacles: &'a [Rect],
    pub content_rects: &'a [Rect],
    pub endpoint_ids: &'a HashSet<String>,
    pub diagram_type: &'a str,
    pub relation_collection: &'a str,
    pub profile: &'a str,
    pub policy: DetourPolicy,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetourEvaluation {
    pub relation_index: usize,
    pub applicable: bool,
    pub evaluated: bool,
    pub reason: String,
    pub grid_metrics: Option<GridMetrics>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetourReport {
    pub scope: String,
    pub reason: String,
    pub applicable: bool,
    pub evaluated: bool,
    pub problems: Vec<Diagnostic>,
    pub evaluations: Vec<DetourEvaluation>,
}

#[derive(Clone, Copy, Serialize)]
struct Bounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, Serialize)]
struct Excursion {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    maximum: f64,
}

fn usable_rect(rect: &&Rect) -> bool {
    finite_rect(rect) && rect.width >= 0.0 && rect.height >= 0.0
}

fn bounds_for_points(points: &[Point]) -> Option<Bounds> {
    let first = *points.first()?;
    let (mut left, mut right, mut top, mut bottom) = (first.x, first.x, first.y, first.y);
    for point in &points[1..] {
        left = left.min(point.x);
        right = right.max(point.x);
        top = top.min(point.y);
        bottom = bottom.max(point.y);
    }
    Some(Bounds {
        left,
        top,
        right,
        bottom,
        width: right - left,
        height: bottom - top,
    })
}

fn bounds_for_rects(rects: &[Rect]) -> Option<Bounds> {
    let mut usable = rects.iter().filter(usable_rect);
    let first = usable.next()?;
    let (mut left, mut right, mut top, mut bottom) = (
        first.x,
        first.x + first.width,
        first.y,
        first.y + first.height,
    );
    for rect in usable {
        left = left.min(rect.x);
        right = right.max(rect.x + rect.width);
        top = top.min(rect.y);
        bottom = bottom.max(rect.y + rect.height);
    }
    Some(Bounds {
        left,
        top,
        right,
        bottom,
        width: right - left,
        height: bottom - top,
    })
}

fn outside_excursion(route: Bounds, content: Option<Bounds>) -> Option<Excursion> {
    let content = content?;
    let left = (content.left - route.left).max(0.0);
    let top = (content.top - route.top).max(0.0);
    let right = (route.right - content.right).max(0.0);
    let bottom = (route.bottom - content.bottom).max(0.0);
    Some(Excursion {
        left,
        top,
        right,
        bottom,
        maximum: left.max(top).max(right).max(bottom),
    })
}

fn empty_control_point_clearance(points: &[Point], rects: &[Rect]) -> Option<(f64, Point)> {
    let usable: Vec<_> = rects.iter().filter(usable_rect).collect();
    if points.len() < 3 || usable.is_empty() {
        return None;
    }
    let mut maximum: Option<(f64, Point)> = None;
    for &point in &points[1..points.len() - 1] {
        let distance = usable
            .iter()
            .map(|rect| {
                (rect.x - point.x)
                    .max(0.0)
                    .max(point.x - (rect.x + rect.width))
                    + (rect.y - point.y)
                        .max(0.0)
                        .max(point.y - (rect.y + rect.height))
            })
            .fold(f64::INFINITY, f64::min);
        if maximum.is_none_or(|(previous, _)| distance > previous) {
            maximum = Some((distance, point));
        }
    }
    maximum
}

fn shares_outer_corridor(
    index: usize,
    relations: &[DetourRelation],
    points: &[Point],
    content: Option<Bounds>,
    minimum_overlap: f64,
) -> bool {
    let Some(content) = content else {
        return false;
    };
    let relation = &relations[index];
    for (other_index, other) in relations.iter().enumerate() {
        if other_index == index
            || !(relation.from == other.from
                || relation.from == other.to
                || relation.to == other.from
                || relation.to == other.to)
        {
            continue;
        }
        let other_points = normalize_route_points(&other.points);
        for pair in points.windows(2) {
            let midpoint = Point {
                x: (pair[0].x + pair[1].x) / 2.0,
                y: (pair[0].y + pair[1].y) / 2.0,
            };
            if midpoint.x >= content.left
                && midpoint.x <= content.right
                && midpoint.y >= content.top
                && midpoint.y <= content.bottom
            {
                continue;
            }
            if other_points.windows(2).any(|other_pair| {
                collinear_overlap(pair[0], pair[1], other_pair[0], other_pair[1]) >= minimum_overlap
            }) {
                return true;
            }
        }
    }
    false
}

fn inferred_side(points: &[Point], source: bool) -> Option<&'static str> {
    if points.len() < 2 {
        return None;
    }
    let (start, end) = if source {
        (points[0], points[1])
    } else {
        (points[points.len() - 2], points[points.len() - 1])
    };
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    if dx > 0.0 && dy == 0.0 {
        Some(if source { "right" } else { "left" })
    } else if dx < 0.0 && dy == 0.0 {
        Some(if source { "left" } else { "right" })
    } else if dy > 0.0 && dx == 0.0 {
        Some(if source { "bottom" } else { "top" })
    } else if dy < 0.0 && dx == 0.0 {
        Some(if source { "top" } else { "bottom" })
    } else {
        None
    }
}

fn rounded(value: f64) -> f64 {
    (value * 100.0 + 0.5).floor() / 100.0
}
fn point_arrays(points: &[Point]) -> Vec<[f64; 2]> {
    points.iter().map(|point| [point.x, point.y]).collect()
}

pub fn clean_route_detour_problems(options: &DetourOptions<'_>) -> DetourReport {
    let mut report = DetourReport {
        scope: "showcase-authored-orthogonal-route-detours".into(),
        reason: "evaluated".into(),
        applicable: options.profile == "showcase",
        evaluated: options.profile == "showcase",
        problems: Vec::new(),
        evaluations: Vec::new(),
    };
    if !report.applicable {
        report.reason = "showcase-only".into();
        return report;
    }
    let policy = &options.policy;
    let content_bounds = bounds_for_rects(options.content_rects);
    for (index, relation) in options.relations.iter().enumerate() {
        let mut evaluation = DetourEvaluation {
            relation_index: index,
            applicable: true,
            evaluated: true,
            reason: String::new(),
            grid_metrics: None,
        };
        if !options.endpoint_ids.contains(&relation.from)
            || !options.endpoint_ids.contains(&relation.to)
        {
            evaluation.applicable = false;
            evaluation.reason = "unknown-endpoint".into();
            report.evaluations.push(evaluation);
            continue;
        }
        if !relation.authored_via {
            evaluation.applicable = false;
            evaluation.reason = "no-authored-via".into();
            report.evaluations.push(evaluation);
            continue;
        }
        let points = normalize_route_points(&relation.points);
        if points.len() < 3 {
            evaluation.applicable = false;
            evaluation.reason = "fewer-than-three-normalized-points".into();
            report.evaluations.push(evaluation);
            continue;
        }
        let Some(actual_length) = orthogonal_length(&points).filter(|length| length.is_finite())
        else {
            evaluation.applicable = false;
            evaluation.reason = "non-orthogonal-route".into();
            report.evaluations.push(evaluation);
            continue;
        };
        let (start, end) = (
            points[0],
            *points.last().expect("at least three normalized points"),
        );
        let manhattan = (end.x - start.x).abs() + (end.y - start.y).abs();
        if actual_length < manhattan * policy.minimum_detour_ratio
            || actual_length - manhattan < policy.minimum_excess_length_px
        {
            evaluation.reason = "below-geometric-detour-thresholds".into();
            report.evaluations.push(evaluation);
            continue;
        }
        let route_bounds = bounds_for_points(&points).expect("at least three normalized points");
        let excursion = outside_excursion(route_bounds, content_bounds);
        let empty_clearance = empty_control_point_clearance(&points, options.obstacles);
        if excursion
            .map_or(0.0, |value| value.maximum)
            .max(empty_clearance.map_or(0.0, |value| value.0))
            < policy.minimum_empty_excursion_px
        {
            evaluation.reason = "below-empty-excursion-threshold".into();
            report.evaluations.push(evaluation);
            continue;
        }
        if shares_outer_corridor(
            index,
            options.relations,
            &points,
            content_bounds,
            policy.shared_corridor_minimum_px,
        ) {
            evaluation.reason = "related-shared-outer-corridor".into();
            report.evaluations.push(evaluation);
            continue;
        }
        let from_side = relation
            .from_side
            .as_deref()
            .filter(|side| !side.is_empty())
            .or_else(|| inferred_side(&points, true))
            .unwrap_or("");
        let to_side = relation
            .to_side
            .as_deref()
            .filter(|side| !side.is_empty())
            .or_else(|| inferred_side(&points, false))
            .unwrap_or("");
        let mut grid_options =
            GridRouteOptions::new(start, end, &points, options.obstacles, from_side, to_side);
        grid_options.clearance = policy.clearance;
        grid_options.maximum_obstacle_count = policy.maximum_obstacle_count;
        let search = shortest_orthogonal_grid_route(&grid_options);
        evaluation.grid_metrics = Some(search.metrics.clone());
        let Some(shortest) = search
            .route
            .filter(|route| route.length.is_finite() && route.length > 0.0)
        else {
            evaluation.evaluated = false;
            evaluation.reason = search.metrics.status;
            report.evaluated = false;
            report.reason = "route-search-incomplete".into();
            report.evaluations.push(evaluation);
            continue;
        };
        let detour_ratio = actual_length / shortest.length;
        let excess_length = actual_length - shortest.length;
        if detour_ratio < policy.minimum_detour_ratio
            || excess_length < policy.minimum_excess_length_px
        {
            evaluation.reason = "obstacles-explain-route-length".into();
            report.evaluations.push(evaluation);
            continue;
        }
        let relation_id = relation
            .id
            .as_ref()
            .filter(|id| !id.is_empty())
            .map(|id| format!(" id \"{id}\""))
            .unwrap_or_default();
        let message = format!("[composition/excessive-route-detour] {} {}[{}]{} \"{}\" -> \"{}\" travels {}px, {}x the {}px shortest obstacle-clearing orthogonal route, and reaches {}px beyond the content bounds — remove the distant via corridor or move it close to the connected content.",
            options.diagram_type, options.relation_collection, index, relation_id, relation.from, relation.to,
            crate::geometry::format_number((actual_length + 0.5).floor()), crate::geometry::format_number(rounded(detour_ratio)),
            crate::geometry::format_number((shortest.length + 0.5).floor()), crate::geometry::format_number((excursion.map_or(0.0, |value| value.maximum) + 0.5).floor()));
        report.problems.push(Diagnostic {
            stage: "composition".into(), code: "composition/excessive-route-detour".into(), severity: Severity::Error, message,
            subject: Subject { diagram_type: Some(options.diagram_type.into()), path: format!("/{}/{}", options.relation_collection, index), identity: relation.id.clone().filter(|id| !id.is_empty()) },
            evidence: json!({
                "points": point_arrays(&points), "actualLengthPx": rounded(actual_length), "shortestLegalPoints": point_arrays(&shortest.points),
                "shortestLegalLengthPx": rounded(shortest.length), "detourRatio": rounded(detour_ratio), "excessLengthPx": rounded(excess_length),
                "routeBounds": route_bounds, "contentBounds": content_bounds, "emptyExcursionPx": excursion,
                "emptyControlPointClearancePx": empty_clearance.map(|(maximum, point)| json!({ "maximum": maximum, "point": [point.x, point.y] })),
                "obstacleCount": shortest.obstacle_count,
                "thresholds": { "minimumDetourRatio": policy.minimum_detour_ratio, "minimumExcessLengthPx": policy.minimum_excess_length_px, "minimumEmptyExcursionPx": policy.minimum_empty_excursion_px },
            }),
            supported_fixes: vec!["remove the distant via points and retry automatic routing, or keep the endpoint sides and move the via corridor near the connected nodes while preserving labels and direction".into()],
        });
        evaluation.reason = "excessive-route-detour".into();
        report.evaluations.push(evaluation);
    }
    if report.evaluated && !report.problems.is_empty() {
        report.reason = "problems-found".into();
    }
    report
}
