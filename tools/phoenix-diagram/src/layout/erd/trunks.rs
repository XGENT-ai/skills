// Corridor lanes and bundled fan-in/fan-out trunks, ported from the frozen
// render-erd.mjs. Both are only preferred candidates: a candidate that breaks
// the endpoint contract or meets an obstacle falls back to the shared families.
use crate::geometry::{anchor, segment_intersects_rect};
use crate::layout::router::{CandidateContext, Edge, Side, Sides};
use crate::scene::{Point, Rect};
use std::collections::HashMap;

const LANE_STEP: f64 = 12.0;
const TRUNK_OFFSETS: [f64; 4] = [16.0, 22.0, 12.0, 28.0];
const TRUNK_SNAP: f64 = LANE_STEP;
const TRUNK_CAP: f64 = 10.0;
const TRUNK_LEG: f64 = 12.0;
// One port step of uncovered bus is a bus with a gap in it; two steps belong to
// different groups and stay separate lines.
pub(crate) const TRUNK_BRIDGE: f64 = 24.0 + 8.0;

/// One entity as the trunk planner sees it.
pub(crate) struct Table<'a> {
    pub row: Option<f64>,
    pub col: Option<f64>,
    pub bounds: Rect,
    pub id: &'a str,
}

fn centre(rect: Rect) -> Point {
    Point {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    }
}

fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

fn integral(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.fract() == 0.0)
}

fn band_key(table: &Table<'_>) -> String {
    match (integral(table.row), integral(table.col)) {
        (Some(row), Some(col)) => format!("i{row},{col}"),
        _ => format!("p{},{}", js_round(table.bounds.x), js_round(table.bounds.y)),
    }
}

fn utf16_key(edge: &Edge) -> Vec<u16> {
    let text = |field: &str| edge.raw[field].as_str().unwrap_or("").to_owned();
    format!("{}\0{}\0{}", text("from"), text("to"), text("id"))
        .encode_utf16()
        .collect()
}

/// Relationships that would share one corridor get a deterministic lane each.
pub(crate) fn lane_offsets(
    edges: &[Edge],
    tables: &[Table<'_>],
    sides: &[Sides],
) -> HashMap<usize, f64> {
    let mut groups: Vec<(String, Vec<usize>)> = vec![];
    for (index, edge) in edges.iter().enumerate() {
        let horizontal = sides[index].from.horizontal();
        let (from, to) = (&tables[edge.from], &tables[edge.to]);
        let mut ordered = [from, to];
        let along = |table: &Table<'_>| {
            let point = centre(table.bounds);
            if horizontal {
                point.x
            } else {
                point.y
            }
        };
        if along(ordered[1]) < along(ordered[0]) {
            ordered.swap(0, 1);
        }
        let key = format!(
            "{}:{}|{}",
            if horizontal { "h" } else { "v" },
            band_key(ordered[0]),
            band_key(ordered[1])
        );
        match groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            Some((_, list)) => list.push(index),
            None => groups.push((key, vec![index])),
        }
    }
    let mut offsets = HashMap::new();
    for (_, mut list) in groups {
        if list.len() < 2 {
            continue;
        }
        list.sort_by(|a, b| utf16_key(&edges[*a]).cmp(&utf16_key(&edges[*b])));
        let middle = (list.len() - 1) as f64 / 2.0;
        for (slot, index) in list.into_iter().enumerate() {
            offsets.insert(index, slot as f64 - middle);
        }
    }
    offsets
}

#[derive(Clone, Debug)]
pub(crate) struct Trunk {
    /// `in` groups share their target side, `out` groups their source side.
    pub inbound: bool,
    pub side: Side,
    pub style: String,
    pub members: Vec<usize>,
    /// 0 when the bus runs vertically (a left/right side), 1 when horizontally.
    pub axis: usize,
    pub coordinate: f64,
}

/// A relationship that names no route, waypoints or label point.
pub(crate) fn auto_routed(edge: &Edge) -> bool {
    edge.raw.get("via").is_none()
        && edge.raw["route"]
            .as_str()
            .is_none_or(|route| route == "auto")
        && edge.raw.get("channelX").is_none()
        && edge.raw.get("channelY").is_none()
        && edge.raw.get("labelAt").is_none()
}

fn component(point: Point, axis: usize) -> f64 {
    if axis == 0 {
        point.x
    } else {
        point.y
    }
}

/// Relationships sharing one entity side and marker style read as one trunk
/// with short branches. Returns the groups that found a clear coordinate.
pub(crate) fn trunk_groups(
    edges: &[Edge],
    tables: &[Table<'_>],
    sides: &[Sides],
    ports: &[[Option<Point>; 2]],
    styles: &[String],
) -> Vec<Trunk> {
    type Groups = Vec<(String, Vec<usize>)>;
    let mut fan_in: Groups = vec![];
    let mut fan_out: Groups = vec![];
    let push = |groups: &mut Groups, key: String, index: usize| match groups
        .iter_mut()
        .find(|(candidate, _)| *candidate == key)
    {
        Some((_, list)) => list.push(index),
        None => groups.push((key, vec![index])),
    };
    for (index, edge) in edges.iter().enumerate() {
        if !auto_routed(edge) {
            continue;
        }
        let style = &styles[index];
        let to = tables[edge.to].id;
        let from = tables[edge.from].id;
        push(
            &mut fan_in,
            format!("{to}\0{}\0{style}", sides[index].to.name()),
            index,
        );
        push(
            &mut fan_out,
            format!("{from}\0{}\0{style}", sides[index].from.name()),
            index,
        );
    }
    let mut taken = vec![false; edges.len()];
    let mut groups: Vec<Trunk> = vec![];
    for (inbound, map) in [(true, fan_in), (false, fan_out)] {
        let mut lists: Vec<_> = map
            .into_iter()
            .map(|(_, list)| list)
            .filter(|list| list.len() >= 2)
            .collect();
        lists.sort_by_key(|list| list[0]);
        for list in lists {
            let members: Vec<_> = list.into_iter().filter(|index| !taken[*index]).collect();
            if members.len() < 2 {
                continue;
            }
            let head = members[0];
            let side = if inbound {
                sides[head].to
            } else {
                sides[head].from
            };
            for member in &members {
                taken[*member] = true;
            }
            groups.push(Trunk {
                inbound,
                side,
                style: styles[head].clone(),
                members,
                axis: if side.horizontal() { 0 } else { 1 },
                coordinate: f64::NAN,
            });
        }
    }
    // One coordinate per group just outside the shared side. A coordinate near
    // an already chosen trunk of the same style snaps onto it, so fan-in and
    // fan-out groups sharing one channel read as one bus.
    let mut chosen: Vec<(usize, f64, String)> = vec![];
    for group in &mut groups {
        let port_axis = 1 - group.axis;
        let entity = if group.inbound {
            edges[group.members[0]].to
        } else {
            edges[group.members[0]].from
        };
        let rect = tables[entity].bounds;
        let coordinates: Vec<f64> = group
            .members
            .iter()
            .map(|member| {
                let port = ports[*member][if group.inbound { 1 } else { 0 }]
                    .unwrap_or_else(|| anchor(&rect, group.side.name()));
                component(port, port_axis)
            })
            .collect();
        let low = coordinates.iter().copied().fold(f64::INFINITY, f64::min) - TRUNK_CAP;
        let high = coordinates
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            + TRUNK_CAP;
        let axis = group.axis;
        let clears_span = |coordinate: f64| {
            let (start, end) = if axis == 0 {
                (
                    Point {
                        x: coordinate,
                        y: low,
                    },
                    Point {
                        x: coordinate,
                        y: high,
                    },
                )
            } else {
                (
                    Point {
                        x: low,
                        y: coordinate,
                    },
                    Point {
                        x: high,
                        y: coordinate,
                    },
                )
            };
            tables
                .iter()
                .all(|table| !segment_intersects_rect(start, end, &table.bounds, 0.0))
        };
        let base = match group.side {
            Side::Left => rect.x,
            Side::Right => rect.x + rect.width,
            Side::Top => rect.y,
            Side::Bottom => rect.y + rect.height,
        };
        let outward = if matches!(group.side, Side::Left | Side::Top) {
            -1.0
        } else {
            1.0
        };
        for offset in TRUNK_OFFSETS {
            let candidate = base + outward * offset;
            if !clears_span(candidate) {
                continue;
            }
            let near = chosen.iter().find(|(line_axis, coordinate, style)| {
                *line_axis == axis
                    && *style == group.style
                    && (coordinate - candidate).abs() < TRUNK_SNAP
            });
            // A snapped coordinate serves another group's span; it must clear this one too.
            if near.is_some_and(|(_, coordinate, _)| !clears_span(*coordinate)) {
                continue;
            }
            // A differently styled trunk never shares a coordinate.
            if chosen.iter().any(|(line_axis, coordinate, style)| {
                *line_axis == axis
                    && *style != group.style
                    && (coordinate - candidate).abs() < TRUNK_SNAP
            }) {
                continue;
            }
            group.coordinate = near.map_or(candidate, |(_, coordinate, _)| *coordinate);
            break;
        }
        if group.coordinate.is_finite() {
            chosen.push((axis, group.coordinate, group.style.clone()));
        }
    }
    groups.retain(|group| group.coordinate.is_finite());
    groups
}

/// The trunk-shaped interior points for one member, or none.
pub(crate) fn trunk_candidate(trunk: &Trunk, context: &CandidateContext<'_>) -> Option<Vec<Point>> {
    let (start, end, sides) = (context.start, context.end, context.sides);
    let c = trunk.coordinate;
    let side_offset = |side: Side| {
        if matches!(side, Side::Right | Side::Bottom) {
            1.0
        } else {
            -1.0
        }
    };
    let (start_horizontal, end_horizontal) = (sides.from.horizontal(), sides.to.horizontal());
    if trunk.axis == 0 {
        if start_horizontal && end_horizontal {
            return Some(vec![Point { x: c, y: start.y }, Point { x: c, y: end.y }]);
        }
        if start_horizontal {
            let approach = end.y + side_offset(sides.to) * TRUNK_LEG;
            return Some(vec![
                Point { x: c, y: start.y },
                Point { x: c, y: approach },
                Point {
                    x: end.x,
                    y: approach,
                },
            ]);
        }
        if end_horizontal {
            let exit = start.y + side_offset(sides.from) * TRUNK_LEG;
            return Some(vec![
                Point {
                    x: start.x,
                    y: exit,
                },
                Point { x: c, y: exit },
                Point { x: c, y: end.y },
            ]);
        }
        return None;
    }
    if !start_horizontal && !end_horizontal {
        return Some(vec![Point { x: start.x, y: c }, Point { x: end.x, y: c }]);
    }
    if !start_horizontal {
        let approach = end.x + side_offset(sides.to) * TRUNK_LEG;
        return Some(vec![
            Point { x: start.x, y: c },
            Point { x: approach, y: c },
            Point {
                x: approach,
                y: end.y,
            },
        ]);
    }
    if !end_horizontal {
        let exit = start.x + side_offset(sides.from) * TRUNK_LEG;
        return Some(vec![
            Point {
                x: exit,
                y: start.y,
            },
            Point { x: exit, y: c },
            Point { x: end.x, y: c },
        ]);
    }
    None
}

/// The corridor lane candidate for a relationship with a lane offset.
pub(crate) fn lane_candidate(offset: f64, context: &CandidateContext<'_>) -> Vec<Point> {
    let (start, end) = (context.start, context.end);
    if context.sides.from.horizontal() {
        let channel = (start.x + end.x) / 2.0 + offset * LANE_STEP;
        vec![
            Point {
                x: channel,
                y: start.y,
            },
            Point {
                x: channel,
                y: end.y,
            },
        ]
    } else {
        let channel = (start.y + end.y) / 2.0 + offset * LANE_STEP;
        vec![
            Point {
                x: start.x,
                y: channel,
            },
            Point {
                x: end.x,
                y: channel,
            },
        ]
    }
}

/// A stretch of a member route that lies on its trunk line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Run {
    pub start_index: usize,
    pub end_index: usize,
    pub low: f64,
    pub high: f64,
}

/// The stretches the shared trunk path draws instead of the member itself.
pub(crate) fn trunk_runs(points: &[Point], axis: usize, coordinate: f64) -> Vec<Run> {
    let mut runs: Vec<Run> = vec![];
    for (index, pair) in points.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        if (component(a, axis) - coordinate).abs() > 0.5
            || (component(b, axis) - coordinate).abs() > 0.5
        {
            continue;
        }
        let (a, b) = (component(a, 1 - axis), component(b, 1 - axis));
        let (low, high) = (a.min(b), a.max(b));
        if high - low < 1.0 {
            continue;
        }
        match runs.last_mut() {
            Some(last) if last.end_index == index => {
                last.end_index = index + 1;
                last.high = high;
            }
            _ => runs.push(Run {
                start_index: index,
                end_index: index + 1,
                low,
                high,
            }),
        }
    }
    runs
}

/// The junction tag a bundled member carries, and its inverse.
pub(crate) fn junction(axis: usize, coordinate: f64) -> String {
    format!("er-trunk:{axis}:{coordinate}")
}

pub(crate) fn parse_junction(value: &str) -> Option<(usize, f64)> {
    let mut parts = value.strip_prefix("er-trunk:")?.splitn(2, ':');
    let axis = parts.next()?.parse().ok().filter(|axis| *axis < 2)?;
    let coordinate: f64 = parts.next()?.parse().ok()?;
    coordinate.is_finite().then_some((axis, coordinate))
}

/// The drawn pieces of a member route once its trunk stretches are removed.
pub(crate) fn drawn_pieces(points: &[Point], runs: &[Run]) -> Vec<Vec<Point>> {
    let in_run = |index: usize| {
        runs.iter()
            .any(|run| index >= run.start_index && index < run.end_index)
    };
    let mut pieces = vec![];
    let mut current = Some(vec![points[0]]);
    for index in 0..points.len().saturating_sub(1) {
        if in_run(index) {
            if let Some(piece) = current.take() {
                pieces.push(piece);
            }
            continue;
        }
        current
            .get_or_insert_with(|| vec![points[index]])
            .push(points[index + 1]);
    }
    if let Some(piece) = current {
        pieces.push(piece);
    }
    pieces.retain(|piece| piece.len() >= 2);
    pieces
}
