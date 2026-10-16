use super::*;
use crate::geometry::{anchor, label_point, normalize_route_points, proper_segment_intersection};
use crate::geometry::{labels as shared_labels, route_quality};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Right,
    Bottom,
    Left,
    Top,
}
impl Side {
    const ORDER: [Self; 4] = [Self::Right, Self::Bottom, Self::Left, Self::Top];
    pub fn name(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Top => "top",
        }
    }
    fn parse(value: &Value) -> Option<Self> {
        match value.as_str() {
            Some("right") => Some(Self::Right),
            Some("bottom") => Some(Self::Bottom),
            Some("left") => Some(Self::Left),
            Some("top") => Some(Self::Top),
            _ => None,
        }
    }
    fn vector(self) -> Point {
        match self {
            Self::Right => Point { x: 1.0, y: 0.0 },
            Self::Bottom => Point { x: 0.0, y: 1.0 },
            Self::Left => Point { x: -1.0, y: 0.0 },
            Self::Top => Point { x: 0.0, y: -1.0 },
        }
    }
    fn opposite(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
        }
    }
    fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Sides {
    pub from: Side,
    pub to: Side,
}
#[derive(Clone)]
pub(super) struct Edge {
    pub raw: Value,
    pub index: usize,
    pub from: usize,
    pub to: usize,
}
impl Edge {
    pub fn explicit(&self) -> bool {
        self.raw.get("via").is_some()
            || self.raw["route"]
                .as_str()
                .is_some_and(|value| value != "auto")
            || self.raw.get("channelX").is_some()
            || self.raw.get("channelY").is_some()
    }
    fn label_pinned(&self) -> bool {
        ["labelAt", "labelDx", "labelDy", "labelSegment"]
            .iter()
            .any(|key| self.raw.get(key).is_some())
    }
    fn stroke(&self) -> f64 {
        number(
            &self.raw,
            "width",
            if self.raw["variant"] == "emphasis" {
                1.8
            } else {
                1.5
            },
        )
    }
}
#[derive(Clone, Debug)]
pub(super) struct Route {
    pub points: Vec<Point>,
    pub sides: Sides,
}
#[derive(Clone)]
pub(super) struct Label {
    pub edge: usize,
    pub rect: Rect,
    pub anchor: Point,
    pinned: bool,
}
impl Label {
    pub fn new(
        edge: usize,
        raw: &Value,
        points: &[Point],
        contract: InputContract,
    ) -> Option<Self> {
        let label = string(raw, "label");
        if label.is_empty() {
            return None;
        }
        let width = (text_units_for_contract(&label, contract) * 4.8 + 10.0).max(30.0);
        let anchor = label_point(raw, points);
        Some(Self {
            edge,
            rect: Rect {
                x: anchor.x - width / 2.0,
                y: anchor.y - 10.0,
                width,
                height: 14.0,
            },
            anchor,
            pinned: ["labelAt", "labelDx", "labelDy", "labelSegment"]
                .iter()
                .any(|key| raw.get(key).is_some()),
        })
    }
    fn shared(&self) -> shared_labels::Label {
        shared_labels::Label {
            relation_index: self.edge as isize,
            bounds: self.rect,
            position: self.anchor,
            pinned: self.pinned,
        }
    }
}

pub(super) fn place_labels(
    labels: &mut [Label],
    routes: &[Route],
    nodes: &[Node],
    titles: &[Rect],
    canvas: Rect,
    bottom: f64,
) {
    let originals: Vec<_> = labels.iter().map(Label::shared).collect();
    let route_list: Vec<_> = routes
        .iter()
        .enumerate()
        .map(|(index, route)| shared_labels::Route {
            relation_index: index as isize,
            points: route.points.clone(),
        })
        .collect();
    let components: Vec<_> = nodes.iter().map(|node| node.bounds).collect();
    let mut options = shared_labels::Options::new(canvas);
    options.placement_bottom = bottom;
    options.keep_fallback_near_route = true;
    let placed = shared_labels::place_automatic_labels(
        &originals,
        &route_list,
        &components,
        titles,
        &options,
    );
    for (label, resolved) in labels.iter_mut().zip(placed) {
        label.rect = resolved.bounds;
        label.anchor = resolved.position;
    }
}
pub(super) fn connections(document: &Value, nodes: &[Node]) -> Vec<Edge> {
    list(&document["connections"])
        .iter()
        .enumerate()
        .map(|(index, raw)| Edge {
            index,
            raw: raw.clone(),
            from: nodes
                .iter()
                .position(|node| node.authored.id == string(raw, "from"))
                .expect("validated source endpoint"),
            to: nodes
                .iter()
                .position(|node| node.authored.id == string(raw, "to"))
                .expect("validated target endpoint"),
        })
        .collect()
}
fn center(rect: Rect) -> Point {
    Point {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    }
}
fn span(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
fn coordinate(point: Point, side: Side) -> f64 {
    if side.horizontal() {
        point.y
    } else {
        point.x
    }
}
fn with_coordinate(mut point: Point, side: Side, value: f64) -> Point {
    if side.horizontal() {
        point.y = value;
    } else {
        point.x = value;
    }
    point
}
fn outward(point: Point, side: Side, distance: f64) -> Point {
    let v = side.vector();
    Point {
        x: point.x + v.x * distance,
        y: point.y + v.y * distance,
    }
}
fn corner_clear(rect: Rect, side: Side, point: Point) -> bool {
    let (origin, size, value) = if side.horizontal() {
        (rect.y, rect.height, point.y)
    } else {
        (rect.x, rect.width, point.x)
    };
    let inset = 16.0_f64.min(size / 2.0);
    value >= origin + inset && value <= origin + size - inset
}
fn default_sides(from: Rect, to: Rect, legacy: bool) -> Sides {
    let a = center(from);
    let b = center(to);
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let side = if dx != 0.0 && (legacy || dx.abs() >= dy.abs()) {
        if dx < 0.0 {
            Side::Left
        } else {
            Side::Right
        }
    } else if dy > 0.0 {
        Side::Bottom
    } else {
        Side::Top
    };
    Sides {
        from: side,
        to: side.opposite(),
    }
}
fn endpoint_honors(a: Point, b: Point, side: Side) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    match side {
        Side::Right => dy.abs() <= 0.0001 && dx > 0.0001,
        Side::Left => dy.abs() <= 0.0001 && dx < -0.0001,
        Side::Bottom => dx.abs() <= 0.0001 && dy > 0.0001,
        Side::Top => dx.abs() <= 0.0001 && dy < -0.0001,
    }
}
pub(super) fn honors(points: &[Point], sides: Sides) -> bool {
    let normalized = normalize_route_points(points);
    normalized.len() >= 2
        && endpoint_honors(normalized[0], normalized[1], sides.from)
        && endpoint_honors(
            *normalized.last().unwrap(),
            normalized[normalized.len() - 2],
            sides.to,
        )
}
pub(super) fn authored_sides_honored(edge: &Edge, route: &Route) -> bool {
    if !edge.explicit() {
        return honors(&route.points, route.sides);
    }
    let points = normalize_route_points(&route.points);
    if points.len() < 2 {
        return true;
    }
    Side::parse(&edge.raw["fromSide"])
        .is_none_or(|side| endpoint_honors(points[0], points[1], side))
        && Side::parse(&edge.raw["toSide"]).is_none_or(|side| {
            endpoint_honors(*points.last().unwrap(), points[points.len() - 2], side)
        })
}
fn backtrack(a: Point, b: Point, c: Point) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let ex = c.x - b.x;
    let ey = c.y - b.y;
    (dx * ey - dy * ex).abs() <= 0.0001 && dx * ex + dy * ey < -0.0001
}
fn join(start: Point, via: &[Point], end: Point) -> Vec<Point> {
    let mut points = vec![start];
    points.extend_from_slice(via);
    points.push(end);
    points
}
fn overlap(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if (a.x - b.x).abs() <= 0.0001 && (c.x - d.x).abs() <= 0.0001 && (a.x - c.x).abs() <= 0.0001 {
        a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y))
    } else if (a.y - b.y).abs() <= 0.0001
        && (c.y - d.y).abs() <= 0.0001
        && (a.y - c.y).abs() <= 0.0001
    {
        a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x))
    } else {
        0.0
    }
    .max(0.0)
}
fn touch(a: Point, b: Point, c: Point, d: Point) -> bool {
    if (a.y - b.y).abs() <= 0.0001 && (c.x - d.x).abs() <= 0.0001 {
        c.x >= a.x.min(b.x) - 0.0001
            && c.x <= a.x.max(b.x) + 0.0001
            && a.y > c.y.min(d.y) + 0.0001
            && a.y < c.y.max(d.y) - 0.0001
    } else if (a.x - b.x).abs() <= 0.0001 && (c.y - d.y).abs() <= 0.0001 {
        c.y >= a.y.min(b.y) - 0.0001
            && c.y <= a.y.max(b.y) + 0.0001
            && a.x > c.x.min(d.x) + 0.0001
            && a.x < c.x.max(d.x) - 0.0001
    } else {
        false
    }
}
fn rhythm(points: &[Point]) -> bool {
    let points = normalize_route_points(points);
    points.windows(2).enumerate().all(|(index, pair)| {
        let length = span(pair[0], pair[1]);
        length <= 0.0001
            || (length >= 8.0 - 0.0001
                && (index == 0 || index == points.len() - 2 || length >= 16.0 - 0.0001))
    })
}
fn shares(a: &Edge, b: &Edge) -> bool {
    a.from == b.from || a.from == b.to || a.to == b.from || a.to == b.to
}
fn port_spacing(a: &Edge, b: &Edge) -> f64 {
    14.0_f64.max(3.5 * (a.stroke() + b.stroke()) + 3.5)
}
#[derive(Clone, Copy)]
struct Geometry {
    start: Point,
    end: Point,
    sides: Sides,
    crowded: bool,
}

struct Router<'a> {
    nodes: &'a [Node],
    edges: &'a [Edge],
    frames: Vec<route_quality::Segment>,
    legacy: bool,
    inferred: Vec<Sides>,
    ports: Vec<[Option<Point>; 2]>,
    cache: Vec<Option<Route>>,
    reserved: Vec<Label>,
    honor_labels: bool,
    allow_grid: bool,
    grid_count: usize,
}

pub(super) fn route(
    nodes: &[Node],
    edges: &[Edge],
    boundaries: &[presentation::Boundary],
    contract: InputContract,
    grow_bottom: bool,
) -> Vec<Route> {
    let legacy = contract == InputContract::Legacy217;
    let mut router = Router {
        nodes,
        edges,
        frames: vec![],
        legacy,
        inferred: vec![],
        ports: vec![[None, None]; edges.len()],
        cache: vec![None; edges.len()],
        reserved: vec![],
        honor_labels: true,
        allow_grid: true,
        grid_count: 0,
    };
    for boundary in boundaries {
        let r = boundary.bounds;
        let radius = if boundary.raw["kind"] == "security-group" {
            8.0_f64
        } else {
            12.0_f64
        }
        .min(r.width / 2.0)
        .min(r.height / 2.0);
        let left = r.x;
        let right = r.x + r.width;
        let top = r.y;
        let bottom = r.y + r.height;
        for (a, b) in [
            (
                Point {
                    x: left + radius,
                    y: top,
                },
                Point {
                    x: right - radius,
                    y: top,
                },
            ),
            (
                Point {
                    x: right,
                    y: top + radius,
                },
                Point {
                    x: right,
                    y: bottom - radius,
                },
            ),
            (
                Point {
                    x: right - radius,
                    y: bottom,
                },
                Point {
                    x: left + radius,
                    y: bottom,
                },
            ),
            (
                Point {
                    x: left,
                    y: bottom - radius,
                },
                Point {
                    x: left,
                    y: top + radius,
                },
            ),
        ] {
            if span(a, b) > 0.0001 {
                router
                    .frames
                    .push(route_quality::Segment { start: a, end: b });
            }
        }
    }
    router.inferred = edges
        .iter()
        .map(|edge| {
            let mut sides = default_sides(nodes[edge.from].bounds, nodes[edge.to].bounds, legacy);
            if !legacy {
                if let Some(fanout) = router.fanout(edge) {
                    sides = fanout;
                }
            }
            Sides {
                from: Side::parse(&edge.raw["fromSide"]).unwrap_or(sides.from),
                to: Side::parse(&edge.raw["toSide"]).unwrap_or(sides.to),
            }
        })
        .collect();
    router.spread_ports();
    let mut order: Vec<usize> = (0..edges.len()).collect();
    if !legacy {
        order.sort_by(|a, b| {
            let left = &edges[*a];
            let right = &edges[*b];
            (!left.explicit())
                .cmp(&(!right.explicit()))
                .then_with(|| {
                    if left.explicit() && right.explicit() {
                        a.cmp(b)
                    } else {
                        let ld = span(
                            center(nodes[left.from].bounds),
                            center(nodes[left.to].bounds),
                        );
                        let rd = span(
                            center(nodes[right.from].bounds),
                            center(nodes[right.to].bounds),
                        );
                        ld.total_cmp(&rd)
                    }
                })
                .then(a.cmp(b))
        });
    }
    let mut resolved = vec![];
    for index in order {
        let routed = router.compute(index, &resolved);
        router.cache[index] = Some(routed);
        resolved.push(index);
        if !legacy {
            if let Some(label) = router.reserve(
                index,
                &router.cache[index].as_ref().unwrap().points,
                &resolved,
                &router.reserved,
            ) {
                router.reserved.push(label);
            }
        }
    }
    if !legacy {
        router.improve(&resolved, grow_bottom);
    }
    router.cache.into_iter().map(Option::unwrap).collect()
}

impl Router<'_> {
    fn fanout(&self, edge: &Edge) -> Option<Sides> {
        let vertical = |node: usize, other: usize| -> Option<Side> {
            let a = self.nodes[node].bounds;
            let b = self.nodes[other].bounds;
            let below = b.y >= a.y + a.height;
            if !below && b.y + b.height > a.y {
                return None;
            }
            let default = default_sides(a, b, false).from;
            if !default.horizontal() {
                return None;
            }
            let neighbors: Vec<usize> = self
                .edges
                .iter()
                .filter_map(|edge| {
                    if edge.from == node && edge.to != node {
                        Some(edge.to)
                    } else if edge.to == node && edge.from != node {
                        Some(edge.from)
                    } else {
                        None
                    }
                })
                .collect();
            let wanted = if below { Side::Bottom } else { Side::Top };
            let row_sibling = neighbors.iter().any(|sibling| {
                *sibling != other
                    && (center(self.nodes[*sibling].bounds).y - center(b).y).abs() < 1.0
                    && default_sides(a, self.nodes[*sibling].bounds, false).from == wanted
            });
            if !row_sibling {
                return None;
            }
            let blocked = neighbors
                .iter()
                .filter(|sibling| {
                    (center(self.nodes[**sibling].bounds).y - center(b).y).abs() < 1.0
                        && default_sides(a, self.nodes[**sibling].bounds, false).from == default
                })
                .any(|target| {
                    let t = self.nodes[*target].bounds;
                    let (low, high) = if center(t).x < center(a).x {
                        (t.x + t.width, a.x)
                    } else {
                        (a.x + a.width, t.x)
                    };
                    self.nodes.iter().enumerate().any(|(index, component)| {
                        index != node
                            && index != *target
                            && component.bounds.y < t.y + t.height
                            && component.bounds.y + component.bounds.height > t.y
                            && component.bounds.x < high
                            && component.bounds.x + component.bounds.width > low
                    })
                });
            blocked.then_some(wanted)
        };
        if let Some(side) = vertical(edge.from, edge.to) {
            Some(Sides {
                from: side,
                to: side.opposite(),
            })
        } else {
            vertical(edge.to, edge.from).map(|side| Sides {
                from: side.opposite(),
                to: side,
            })
        }
    }
    fn spread_ports(&mut self) {
        let mut groups: Vec<(usize, Side, Vec<(usize, usize, usize)>)> = vec![];
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.explicit() || edge.raw.get("labelAt").is_some() {
                continue;
            }
            for (endpoint, node, other, side) in [
                (0, edge.from, edge.to, self.inferred[index].from),
                (1, edge.to, edge.from, self.inferred[index].to),
            ] {
                if let Some(group) = groups
                    .iter_mut()
                    .find(|(id, s, _)| *id == node && *s == side)
                {
                    group.2.push((index, endpoint, other));
                } else {
                    groups.push((node, side, vec![(index, endpoint, other)]));
                }
            }
        }
        for (node, side, mut items) in groups {
            if items.len() < 2 {
                continue;
            }
            items.sort_by(|a, b| {
                coordinate(center(self.nodes[a.2].bounds), side)
                    .total_cmp(&coordinate(center(self.nodes[b.2].bounds), side))
                    .then_with(|| {
                        let key = |index: usize| {
                            let edge = &self.edges[index];
                            format!(
                                "{}\0{}\0{}\0{}",
                                string(&edge.raw, "id"),
                                string(&edge.raw, "from"),
                                string(&edge.raw, "to"),
                                string(&edge.raw, "label")
                            )
                        };
                        key(a.0).encode_utf16().cmp(key(b.0).encode_utf16())
                    })
            });
            let rect = self.nodes[node].bounds;
            let extent = if side.horizontal() {
                rect.height
            } else {
                rect.width
            };
            let usable = (extent - 32.0).max(0.0);
            let spacing = 14.0_f64.min(usable / (items.len() - 1) as f64);
            if spacing <= 0.0 {
                continue;
            }
            let gaps: Vec<f64> = items
                .windows(2)
                .map(|pair| {
                    let a = &self.edges[pair[0].0];
                    let b = &self.edges[pair[1].0];
                    if !self.legacy && 3.5 * (a.stroke() + b.stroke()) > 14.0 {
                        port_spacing(a, b)
                    } else {
                        14.0
                    }
                })
                .collect();
            let total: f64 = gaps.iter().sum();
            let wide = !self.legacy && total <= usable && gaps.iter().any(|gap| *gap > 14.0);
            let mut offset = -total / 2.0;
            for (slot, (index, endpoint, _)) in items.iter().enumerate() {
                let shift = if wide {
                    if slot > 0 {
                        offset += gaps[slot - 1];
                    }
                    offset
                } else {
                    (slot as f64 - (items.len() - 1) as f64 / 2.0) * spacing
                };
                let base = anchor(&rect, side.name());
                self.ports[*index][*endpoint] =
                    Some(with_coordinate(base, side, coordinate(base, side) + shift));
            }
        }
    }
    fn clears_components(&self, index: usize, points: &[Point]) -> bool {
        let edge = &self.edges[index];
        self.nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != edge.from && *i != edge.to)
            .all(|(_, node)| {
                points
                    .windows(2)
                    .all(|pair| !segment_intersects_rect(pair[0], pair[1], &node.bounds, 2.0))
            })
            && (!self.honor_labels
                || self
                    .reserved
                    .iter()
                    .filter(|label| label.edge != index)
                    .all(|label| {
                        points.windows(2).all(|pair| {
                            !segment_intersects_rect(pair[0], pair[1], &label.rect, 4.0)
                        })
                    }))
    }
    fn clears_endpoints(&self, index: usize, points: &[Point]) -> bool {
        let edge = &self.edges[index];
        points.windows(2).enumerate().all(|(segment, pair)| {
            (segment == 0
                || !segment_intersects_rect(pair[0], pair[1], &self.nodes[edge.from].bounds, 0.0))
                && (segment == points.len() - 2
                    || !segment_intersects_rect(pair[0], pair[1], &self.nodes[edge.to].bounds, 0.0))
        })
    }
    fn floors(&self, points: &[Point]) -> bool {
        rhythm(points)
            && !self.frames.iter().any(|border| {
                points
                    .windows(2)
                    .any(|pair| overlap(pair[0], pair[1], border.start, border.end) > 0.0001)
            })
    }
    fn conflicts(&self, index: usize, points: &[Point], resolved: &[usize]) -> bool {
        resolved
            .iter()
            .filter(|other| !shares(&self.edges[index], &self.edges[**other]))
            .any(|other| {
                points.windows(2).any(|a| {
                    self.cache[*other]
                        .as_ref()
                        .unwrap()
                        .points
                        .windows(2)
                        .any(|b| {
                            proper_segment_intersection(a[0], a[1], b[0], b[1]).is_some()
                                || touch(a[0], a[1], b[0], b[1])
                                || overlap(a[0], a[1], b[0], b[1]) >= 8.0
                        })
                })
            })
    }
    fn independent(&self, index: usize, other: usize) -> bool {
        !shares(&self.edges[index], &self.edges[other])
            || (!self.edges[other].explicit()
                && self.edges[other].raw.get("labelAt").is_none()
                && self.edges[index].raw.get("labelAt").is_none())
    }
    fn overlaps(&self, index: usize, points: &[Point], resolved: &[usize]) -> bool {
        resolved
            .iter()
            .filter(|other| self.independent(index, **other))
            .any(|other| {
                points.windows(2).any(|a| {
                    self.cache[*other]
                        .as_ref()
                        .unwrap()
                        .points
                        .windows(2)
                        .any(|b| overlap(a[0], a[1], b[0], b[1]) >= 8.0)
                })
            })
    }
    fn crossing_count(&self, index: usize, points: &[Point], resolved: &[usize]) -> usize {
        resolved
            .iter()
            .filter(|other| self.independent(index, **other))
            .filter(|other| {
                points.windows(2).any(|a| {
                    self.cache[**other]
                        .as_ref()
                        .unwrap()
                        .points
                        .windows(2)
                        .any(|b| proper_segment_intersection(a[0], a[1], b[0], b[1]).is_some())
                })
            })
            .count()
    }
    fn reciprocal(&self, index: usize) -> bool {
        let edge = &self.edges[index];
        self.edges
            .iter()
            .any(|other| other.from == edge.to && other.to == edge.from)
    }
    fn sibling_crossings(&self, index: usize, points: &[Point], resolved: &[usize]) -> usize {
        if self.reciprocal(index) {
            0
        } else {
            self.crossing_count(
                index,
                points,
                &resolved
                    .iter()
                    .copied()
                    .filter(|other| !self.reciprocal(*other))
                    .collect::<Vec<_>>(),
            )
        }
    }
    fn endpoint(&self, index: usize, endpoint: usize, side: Side) -> (Point, bool, bool) {
        let edge = &self.edges[index];
        let inferred = if endpoint == 0 {
            self.inferred[index].from
        } else {
            self.inferred[index].to
        };
        let node = if endpoint == 0 { edge.from } else { edge.to };
        let rect = self.nodes[node].bounds;
        let initial = if side == inferred {
            self.ports[index][endpoint]
        } else {
            None
        };
        let preferred = initial.unwrap_or_else(|| anchor(&rect, side.name()));
        if self.legacy || edge.explicit() || edge.raw.get("labelAt").is_some() {
            return (preferred, initial.is_some(), false);
        }
        let mut occupied = vec![];
        for (other_index, other) in self.edges.iter().enumerate() {
            if other_index == index || other.explicit() || other.raw.get("labelAt").is_some() {
                continue;
            }
            let sides = self.cache[other_index]
                .as_ref()
                .map_or(self.inferred[other_index], |route| route.sides);
            for (other_endpoint, id, other_side) in
                [(0, other.from, sides.from), (1, other.to, sides.to)]
            {
                if id != node || other_side != side {
                    continue;
                }
                let point = self.cache[other_index]
                    .as_ref()
                    .map(|route| {
                        if other_endpoint == 0 {
                            route.points[0]
                        } else {
                            *route.points.last().unwrap()
                        }
                    })
                    .or(self.ports[other_index][other_endpoint])
                    .unwrap_or_else(|| anchor(&rect, side.name()));
                occupied.push((coordinate(point, side), port_spacing(edge, other)));
            }
        }
        if occupied.is_empty() {
            return (preferred, initial.is_some(), false);
        }
        let preferred_value = coordinate(preferred, side);
        let other = if endpoint == 0 { edge.to } else { edge.from };
        let toward = if side == inferred {
            preferred_value
        } else {
            coordinate(center(self.nodes[other].bounds), side)
        };
        let mut candidates = vec![preferred_value];
        for (value, spacing) in &occupied {
            candidates.extend([value - spacing, value + spacing]);
        }
        candidates.sort_by(|a, b| {
            (a - preferred_value)
                .abs()
                .total_cmp(&(b - preferred_value).abs())
                .then_with(|| (a - toward).abs().total_cmp(&(b - toward).abs()))
                .then(a.total_cmp(b))
        });
        for value in candidates {
            let point = with_coordinate(preferred, side, value);
            if corner_clear(rect, side, point)
                && occupied
                    .iter()
                    .all(|(other, spacing)| (value - other).abs() >= spacing - 0.0001)
            {
                return (point, true, false);
            }
        }
        (preferred, true, true)
    }
    fn geometry(&self, index: usize, sides: Sides) -> Geometry {
        let (mut start, from_spread, from_crowded) = self.endpoint(index, 0, sides.from);
        let (mut end, to_spread, to_crowded) = self.endpoint(index, 1, sides.to);
        let edge = &self.edges[index];
        let opposite = sides.from.opposite() == sides.to;
        if !edge.explicit()
            && edge.raw.get("labelAt").is_none()
            && opposite
            && !(from_spread && to_spread)
            && !(!from_spread
                && !to_spread
                && (Side::parse(&edge.raw["fromSide"]).is_some()
                    || Side::parse(&edge.raw["toSide"]).is_some()))
            && (coordinate(start, sides.from) - coordinate(end, sides.from)).abs() < 16.0
        {
            let candidates = if from_spread {
                vec![(
                    start,
                    with_coordinate(end, sides.from, coordinate(start, sides.from)),
                )]
            } else if to_spread {
                vec![(
                    with_coordinate(start, sides.from, coordinate(end, sides.from)),
                    end,
                )]
            } else {
                vec![
                    (
                        start,
                        with_coordinate(end, sides.from, coordinate(start, sides.from)),
                    ),
                    (
                        with_coordinate(start, sides.from, coordinate(end, sides.from)),
                        end,
                    ),
                ]
            };
            for (a, b) in candidates {
                let points = [a, b];
                if corner_clear(self.nodes[edge.from].bounds, sides.from, a)
                    && corner_clear(self.nodes[edge.to].bounds, sides.to, b)
                    && honors(&points, sides)
                    && self.clears_endpoints(index, &points)
                    && self.clears_components(index, &points)
                {
                    start = a;
                    end = b;
                    break;
                }
            }
        }
        Geometry {
            start,
            end,
            sides,
            crowded: from_crowded || to_crowded,
        }
    }
    fn candidate_sides(&self, index: usize) -> Vec<Sides> {
        let inferred = self.inferred[index];
        let edge = &self.edges[index];
        let from = Side::parse(&edge.raw["fromSide"]).map_or_else(
            || {
                std::iter::once(inferred.from)
                    .chain(
                        Side::ORDER
                            .into_iter()
                            .filter(|side| *side != inferred.from),
                    )
                    .collect()
            },
            |side| vec![side],
        );
        let to = Side::parse(&edge.raw["toSide"]).map_or_else(
            || {
                std::iter::once(inferred.to)
                    .chain(Side::ORDER.into_iter().filter(|side| *side != inferred.to))
                    .collect::<Vec<_>>()
            },
            |side| vec![side],
        );
        let mut pairs: Vec<_> = from
            .into_iter()
            .flat_map(|from| to.iter().map(move |to| Sides { from, to: *to }))
            .collect();
        pairs.sort_by(|a, b| {
            let deviation = |sides: Sides| {
                usize::from(sides.from != inferred.from) + usize::from(sides.to != inferred.to)
            };
            deviation(*a).cmp(&deviation(*b)).then_with(|| {
                let ga = self.geometry(index, *a);
                let gb = self.geometry(index, *b);
                span(ga.start, ga.end).total_cmp(&span(gb.start, gb.end))
            })
        });
        pairs
    }
    fn side_bridges(&self, start: Point, end: Point, sides: Sides) -> Vec<Vec<Point>> {
        let a = outward(start, sides.from, 24.0);
        let b = outward(end, sides.to, 24.0);
        let mut candidates = vec![];
        if !sides.from.horizontal() && !sides.to.horizontal() && (start.x - end.x).abs() < 16.0 {
            for x in [start.x.max(end.x) + 16.0, start.x.min(end.x) - 16.0] {
                candidates.push(vec![a, Point { x, y: a.y }, Point { x, y: b.y }, b]);
            }
        }
        if sides.from.horizontal() && sides.to.horizontal() && (start.y - end.y).abs() < 16.0 {
            for y in [start.y.max(end.y) + 16.0, start.y.min(end.y) - 16.0] {
                candidates.push(vec![a, Point { x: a.x, y }, Point { x: b.x, y }, b]);
            }
        }
        candidates.extend([
            vec![a, Point { x: b.x, y: a.y }, b],
            vec![a, Point { x: a.x, y: b.y }, b],
        ]);
        let mut full: Vec<_> = candidates
            .iter()
            .map(|via| normalize_route_points(&join(start, via, end)))
            .filter(|p| {
                p.len() >= 2
                    && !backtrack(p[0], p[1], *p.get(2).unwrap_or(&p[1]))
                    && !backtrack(
                        *p.get(p.len().saturating_sub(3)).unwrap_or(&p[p.len() - 2]),
                        p[p.len() - 2],
                        p[p.len() - 1],
                    )
                    && honors(p, sides)
            })
            .collect();
        if !self.legacy {
            full.sort_by_key(Vec::len);
        }
        full.into_iter()
            .map(|points| points[1..points.len() - 1].to_vec())
            .collect()
    }
    fn rhythm_bridge(
        &self,
        index: usize,
        geometry: Geometry,
        resolved: &[usize],
    ) -> Option<Vec<Point>> {
        let Geometry {
            start, end, sides, ..
        } = geometry;
        let vector = sides.from.vector();
        let delta = Point {
            x: end.x - start.x,
            y: end.y - start.y,
        };
        let gap =
            if sides.from.opposite() == sides.to && delta.x * vector.x + delta.y * vector.y > 0.0 {
                Some(if sides.from.horizontal() {
                    delta.x.abs()
                } else {
                    delta.y.abs()
                })
            } else {
                None
            };
        let mut distances = vec![24.0];
        if let Some(gap) = gap {
            if gap - 48.0 < 16.0 {
                let bounded = 8.0_f64.max(24.0_f64.min((gap - 16.0) / 2.0));
                if bounded != 24.0 {
                    distances.push(bounded);
                }
            }
        }
        for stub in distances {
            let a = outward(start, sides.from, stub);
            let b = outward(end, sides.to, stub);
            let mut candidates = vec![];
            if !sides.from.horizontal() && !sides.to.horizontal() && (start.x - end.x).abs() < 16.0
            {
                for x in [start.x.max(end.x) + 16.0, start.x.min(end.x) - 16.0] {
                    candidates.push(vec![
                        start,
                        a,
                        Point { x, y: a.y },
                        Point { x, y: b.y },
                        b,
                        end,
                    ]);
                }
            }
            if sides.from.horizontal() && sides.to.horizontal() && (start.y - end.y).abs() < 16.0 {
                for y in [start.y.max(end.y) + 16.0, start.y.min(end.y) - 16.0] {
                    candidates.push(vec![
                        start,
                        a,
                        Point { x: a.x, y },
                        Point { x: b.x, y },
                        b,
                        end,
                    ]);
                }
            }
            for candidate in candidates {
                let points = normalize_route_points(&candidate);
                if honors(&points, sides)
                    && rhythm(&points)
                    && self.clears_endpoints(index, &points)
                    && self.clears_components(index, &points)
                    && (self.legacy
                        || (self.floors(&points) && !self.conflicts(index, &points, resolved)))
                {
                    return Some(points[1..points.len() - 1].to_vec());
                }
            }
        }
        None
    }
    fn route_via(&mut self, index: usize, geometry: Geometry, resolved: &[usize]) -> Vec<Point> {
        let edge = &self.edges[index];
        let Geometry {
            start, end, sides, ..
        } = geometry;
        if let Some(via) = edge.raw.get("via") {
            return list(via).iter().filter_map(point).collect();
        }
        let mid_x = (start.x + end.x) / 2.0;
        let mid_y = (start.y + end.y) / 2.0;
        let horizontal = vec![
            Point {
                x: mid_x,
                y: start.y,
            },
            Point { x: mid_x, y: end.y },
        ];
        let vertical = vec![
            Point {
                x: start.x,
                y: mid_y,
            },
            Point { x: end.x, y: mid_y },
        ];
        match edge.raw["route"].as_str().unwrap_or("auto") {
            "straight" => return vec![],
            "orthogonal-h" => return horizontal,
            "orthogonal-v" => return vertical,
            _ => {}
        }
        let dx = (start.x - end.x).abs();
        let dy = (start.y - end.y).abs();
        let direct = [start, end];
        if (dx < 4.0 || dy < 4.0)
            && honors(&direct, sides)
            && (self.legacy
                || (self.clears_endpoints(index, &direct)
                    && self.clears_components(index, &direct)
                    && self.floors(&direct)
                    && !self.conflicts(index, &direct, resolved)))
        {
            return vec![];
        }
        if let Some(via) = self.rhythm_bridge(index, geometry, resolved) {
            return via;
        }
        let from = self.nodes[edge.from].bounds;
        let to = self.nodes[edge.to].bounds;
        let mut outside = vec![];
        if (start.y == from.y || start.y == from.y + from.height)
            && (end.y == to.y || end.y == to.y + to.height)
            && dx < 16.0
        {
            for x in [start.x.max(end.x) + 16.0, start.x.min(end.x) - 16.0] {
                outside.push(vec![Point { x, y: start.y }, Point { x, y: end.y }]);
            }
        }
        if (start.x == from.x || start.x == from.x + from.width)
            && (end.x == to.x || end.x == to.x + to.width)
            && dy < 16.0
        {
            for y in [start.y.max(end.y) + 16.0, start.y.min(end.y) - 16.0] {
                outside.push(vec![Point { x: start.x, y }, Point { x: end.x, y }]);
            }
        }
        for via in outside {
            let points = join(start, &via, end);
            if honors(&points, sides)
                && self.clears_components(index, &points)
                && (self.legacy
                    || (self.floors(&points) && !self.conflicts(index, &points, resolved)))
            {
                return via;
            }
        }
        let candidates = [horizontal.clone(), vertical.clone()];
        let safe: Vec<_> = candidates
            .iter()
            .filter(|via| honors(&join(start, via, end), sides))
            .cloned()
            .collect();
        let bridges = self.side_bridges(start, end, sides);
        let near_parallel = (!sides.from.horizontal() && !sides.to.horizontal() && dx < 16.0)
            || (sides.from.horizontal() && sides.to.horizontal() && dy < 16.0);
        let fallback = safe
            .first()
            .or(bridges.first())
            .cloned()
            .unwrap_or(horizontal.clone());
        let mut ordered = if near_parallel {
            bridges
                .iter()
                .chain(safe.iter())
                .cloned()
                .collect::<Vec<_>>()
        } else {
            safe.iter()
                .chain(bridges.iter())
                .cloned()
                .collect::<Vec<_>>()
        };
        if self.legacy {
            ordered.extend(candidates.into_iter().filter(|via| !safe.contains(via)));
            for via in ordered {
                let points = join(start, &via, end);
                if self.clears_endpoints(index, &points) && self.clears_components(index, &points) {
                    return via;
                }
            }
            return fallback;
        }
        let mut first_feasible = None;
        for via in ordered {
            let points = join(start, &via, end);
            if self.clears_endpoints(index, &points)
                && self.clears_components(index, &points)
                && self.floors(&points)
                && !self.conflicts(index, &points, resolved)
                && !self.overlaps(index, &points, resolved)
                && self.sibling_crossings(index, &points, resolved) == 0
            {
                if self.edges[index].raw["label"]
                    .as_str()
                    .unwrap_or("")
                    .is_empty()
                    || self
                        .reserve(index, &points, resolved, &self.reserved)
                        .is_some()
                {
                    return via;
                }
                if first_feasible.is_none() {
                    first_feasible = Some(via);
                }
            }
        }
        if let Some(via) = first_feasible {
            return via;
        }
        let channels = |a: f64, b: f64, mid: f64| {
            let low = a.min(b) + 24.0;
            let high = a.max(b) - 24.0;
            let mut values = vec![];
            let mut offset = 16.0;
            while mid - offset >= low || mid + offset <= high {
                for value in [mid + offset, mid - offset] {
                    if value >= low && value <= high {
                        values.push(value);
                    }
                }
                offset += 16.0;
            }
            values
        };
        let alternatives = channels(start.x, end.x, mid_x)
            .into_iter()
            .map(|x| vec![Point { x, y: start.y }, Point { x, y: end.y }])
            .chain(
                channels(start.y, end.y, mid_y)
                    .into_iter()
                    .map(|y| vec![Point { x: start.x, y }, Point { x: end.x, y }]),
            );
        for via in alternatives {
            let points = join(start, &via, end);
            if honors(&points, sides)
                && self.clears_endpoints(index, &points)
                && self.clears_components(index, &points)
                && self.floors(&points)
                && !self.conflicts(index, &points, resolved)
                && !self.overlaps(index, &points, resolved)
                && self.sibling_crossings(index, &points, resolved) == 0
            {
                return via;
            }
        }
        if !self.allow_grid || self.grid_count >= 64 {
            return fallback;
        }
        self.grid_count += 1;
        let mut obstacles: Vec<_> = self.nodes.iter().map(|node| node.bounds).collect();
        if self.honor_labels {
            obstacles.extend(self.reserved.iter().map(|label| Rect {
                x: label.rect.x - 2.0,
                y: label.rect.y - 2.0,
                width: label.rect.width + 4.0,
                height: label.rect.height + 4.0,
            }));
        }
        let avoided: Vec<_> = resolved
            .iter()
            .filter(|other| {
                shares(&self.edges[index], &self.edges[**other])
                    && !self.edges[**other].explicit()
                    && self.edges[**other].raw.get("labelAt").is_none()
                    && self.edges[index].raw.get("labelAt").is_none()
            })
            .flat_map(|other| {
                self.cache[*other]
                    .as_ref()
                    .unwrap()
                    .points
                    .windows(2)
                    .map(|pair| route_quality::Segment {
                        start: pair[0],
                        end: pair[1],
                    })
            })
            .collect();
        let points = [start, end];
        let mut options = route_quality::GridRouteOptions::new(
            start,
            end,
            &points,
            &obstacles,
            sides.from.name(),
            sides.to.name(),
        );
        options.clearance = 2.0;
        options.maximum_obstacle_count = 80;
        options.endpoint_stub_px = Some(24.0);
        options.maximum_grid_nodes = 4096.0;
        options.avoided_segments = &avoided;
        options.allow_avoided_crossings = true;
        options.minimum_avoided_overlap_px = 8.0;
        options.route_separation_px = 8.0;
        options.minimum_segment_px = 16.0;
        options.border_segments = &self.frames;
        options.bend_penalty_px = 48.0;
        if let Some(searched) = route_quality::shortest_orthogonal_grid_route(&options).route {
            if self.clears_endpoints(index, &searched.points)
                && self.clears_components(index, &searched.points)
                && !self.overlaps(index, &searched.points, resolved)
                && self.floors(&searched.points)
            {
                return searched.points[1..searched.points.len() - 1].to_vec();
            }
        }
        fallback
    }
    fn routed(&mut self, index: usize, geometry: Geometry, resolved: &[usize]) -> Route {
        let via = self.route_via(index, geometry, resolved);
        let points = join(geometry.start, &via, geometry.end);
        Route {
            points: if self.legacy || self.edges[index].explicit() {
                points
            } else {
                normalize_route_points(&points)
            },
            sides: geometry.sides,
        }
    }
    fn is_clear(
        &self,
        index: usize,
        route: &Route,
        geometry: Geometry,
        resolved: &[usize],
    ) -> bool {
        !geometry.crowded
            && route.points.len() >= 2
            && honors(&route.points, geometry.sides)
            && self.clears_endpoints(index, &route.points)
            && self.clears_components(index, &route.points)
            && self.floors(&route.points)
            && !self.overlaps(index, &route.points, resolved)
    }
    fn first_clear(
        &mut self,
        index: usize,
        resolved: &[usize],
        fallback: &mut Option<Route>,
    ) -> Option<Route> {
        for sides in self.candidate_sides(index) {
            let geometry = self.geometry(index, sides);
            let route = self.routed(index, geometry, resolved);
            if fallback.is_none() {
                *fallback = Some(route.clone());
            }
            if self.is_clear(index, &route, geometry, resolved) {
                return Some(route);
            }
        }
        None
    }
    fn compute(&mut self, index: usize, resolved: &[usize]) -> Route {
        if self.legacy || self.edges[index].explicit() {
            let geometry = self.geometry(index, self.inferred[index]);
            return self.routed(index, geometry, resolved);
        }
        let mut fallback = None;
        self.honor_labels = true;
        let mut chosen = self.first_clear(index, resolved, &mut fallback);
        if !self.reserved.is_empty() {
            self.honor_labels = false;
            if chosen.is_none() {
                chosen = self.first_clear(index, resolved, &mut fallback);
            } else {
                let current = chosen.as_ref().unwrap();
                let direct = span(current.points[0], *current.points.last().unwrap());
                if length(&current.points) > direct * 1.25 + 64.0 {
                    if let Some(plain) = self.first_clear(index, resolved, &mut fallback) {
                        if length(&plain.points) * 1.25 + 64.0 < length(&current.points) {
                            chosen = Some(plain);
                        }
                    }
                }
            }
            self.honor_labels = true;
        }
        chosen
            .or(fallback)
            .expect("at least one deterministic endpoint pair")
    }
    fn reserve(
        &self,
        index: usize,
        points: &[Point],
        resolved: &[usize],
        labels: &[Label],
    ) -> Option<Label> {
        let label = Label::new(
            index,
            &self.edges[index].raw,
            points,
            InputContract::PhoenixV1,
        )?;
        let index_owner = index;
        let routes: Vec<_> = resolved
            .iter()
            .enumerate()
            .map(|(order, index)| shared_labels::Route {
                relation_index: order as isize,
                points: if *index == index_owner {
                    points.to_vec()
                } else {
                    self.cache[*index].as_ref().unwrap().points.clone()
                },
            })
            .collect();
        let existing: Vec<_> = labels.iter().map(Label::shared).collect();
        let components: Vec<_> = self.nodes.iter().map(|node| node.bounds).collect();
        let placed = shared_labels::reserved_label_rect(
            &label.shared(),
            points,
            &routes,
            &existing,
            &components,
            Rect {
                x: 0.0,
                y: 0.0,
                width: f64::INFINITY,
                height: f64::INFINITY,
            },
            f64::INFINITY,
        )?;
        Some(Label {
            rect: placed.bounds,
            anchor: placed.position,
            ..label
        })
    }
    fn cost(&self, index: usize, route: &Route, resolved: &[usize]) -> f64 {
        length(&route.points)
            + route.points.len().saturating_sub(2) as f64 * 48.0
            + self.crossing_count(index, &route.points, resolved) as f64 * 160.0
    }
    fn slots_clear(&self, candidates: &[(usize, Route)], others: &[usize]) -> bool {
        for (index, route) in candidates {
            let edge = &self.edges[*index];
            for (id, side, point) in [
                (edge.from, route.sides.from, route.points[0]),
                (edge.to, route.sides.to, *route.points.last().unwrap()),
            ] {
                for (other, other_route) in others
                    .iter()
                    .map(|index| (*index, self.cache[*index].as_ref().unwrap()))
                    .chain(candidates.iter().map(|(index, route)| (*index, route)))
                {
                    if other == *index {
                        continue;
                    }
                    let other_edge = &self.edges[other];
                    for (other_id, other_side, other_point) in [
                        (
                            other_edge.from,
                            other_route.sides.from,
                            other_route.points[0],
                        ),
                        (
                            other_edge.to,
                            other_route.sides.to,
                            *other_route.points.last().unwrap(),
                        ),
                    ] {
                        if id == other_id
                            && side == other_side
                            && (coordinate(point, side) - coordinate(other_point, side)).abs()
                                < port_spacing(edge, other_edge) - 0.0001
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
    fn label_clear(
        &self,
        label: Option<&Label>,
        owner: usize,
        resolved: &[usize],
        labels: &[Label],
        bounds: Rect,
    ) -> bool {
        let Some(label) = label else {
            return self.edges[owner].raw["label"]
                .as_str()
                .unwrap_or("")
                .is_empty();
        };
        in_bounds(label.rect, bounds)
            && !labels
                .iter()
                .any(|other| crate::geometry::rects_overlap(&label.rect, &other.rect, 2.0))
            && resolved
                .iter()
                .filter(|other| **other != owner)
                .all(|other| {
                    self.cache[*other]
                        .as_ref()
                        .unwrap()
                        .points
                        .windows(2)
                        .all(|segment| {
                            !segment_intersects_rect(segment[0], segment[1], &label.rect, 4.0)
                        })
                })
    }
    fn scene_bounds(&self, resolved: &[usize]) -> Rect {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut add = |point: Point| {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        };
        for point in resolved
            .iter()
            .flat_map(|index| self.cache[*index].as_ref().unwrap().points.iter())
        {
            add(*point);
        }
        for rect in self
            .nodes
            .iter()
            .map(|node| node.bounds)
            .chain(self.reserved.iter().map(|label| label.rect))
        {
            add(Point {
                x: rect.x,
                y: rect.y,
            });
            add(Point {
                x: rect.x + rect.width,
                y: rect.y + rect.height,
            });
        }
        for frame in &self.frames {
            add(frame.start);
            add(frame.end);
        }
        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
    fn improve_reciprocals(
        &mut self,
        resolved: &[usize],
        bounds: Rect,
    ) -> std::collections::HashSet<usize> {
        let mut paired = std::collections::HashSet::new();
        let mut improved = std::collections::HashSet::new();
        for first in resolved {
            let edge = &self.edges[*first];
            if paired.contains(first)
                || edge.explicit()
                || edge.label_pinned()
                || Side::parse(&edge.raw["fromSide"]).is_some()
                || Side::parse(&edge.raw["toSide"]).is_some()
            {
                continue;
            }
            let Some(second) = resolved.iter().copied().find(|index| {
                *index != *first
                    && self.edges[*index].from == edge.to
                    && self.edges[*index].to == edge.from
                    && !self.edges[*index].explicit()
                    && !self.edges[*index].label_pinned()
                    && Side::parse(&self.edges[*index].raw["fromSide"]).is_none()
                    && Side::parse(&self.edges[*index].raw["toSide"]).is_none()
            }) else {
                continue;
            };
            paired.insert(*first);
            paired.insert(second);
            let fs = self.inferred[*first];
            let ss = self.inferred[second];
            if fs.from.opposite() != fs.to
                || ss.from.opposite() != ss.to
                || fs.from != ss.to
                || fs.to != ss.from
            {
                continue;
            }
            let first_route = self.cache[*first].as_ref().unwrap().clone();
            let second_route = self.cache[second].as_ref().unwrap().clone();
            if first_route.points.len() <= 2 && second_route.points.len() <= 2 {
                continue;
            }
            let fg = self.geometry(*first, fs);
            let sg = self.geometry(second, ss);
            if fg.crowded || sg.crowded {
                continue;
            }
            let others: Vec<_> = resolved
                .iter()
                .copied()
                .filter(|index| *index != *first && *index != second)
                .collect();
            let other_labels: Vec<_> = self
                .reserved
                .iter()
                .filter(|label| label.edge != *first && label.edge != second)
                .cloned()
                .collect();
            let previous_labels = std::mem::replace(&mut self.reserved, other_labels.clone());
            let mut best = None;
            let mut best_cost = self.cost(
                *first,
                &first_route,
                &resolved
                    .iter()
                    .copied()
                    .filter(|index| *index != *first)
                    .collect::<Vec<_>>(),
            ) + self.cost(
                second,
                &second_route,
                &resolved
                    .iter()
                    .copied()
                    .filter(|index| *index != second)
                    .collect::<Vec<_>>(),
            );
            for fl in unique(vec![
                coordinate(fg.start, fs.from),
                coordinate(fg.end, fs.from),
            ]) {
                for sl in unique(vec![
                    coordinate(sg.start, ss.from),
                    coordinate(sg.end, ss.from),
                ]) {
                    let fr = Route {
                        points: vec![
                            with_coordinate(fg.start, fs.from, fl),
                            with_coordinate(fg.end, fs.from, fl),
                        ],
                        sides: fs,
                    };
                    let sr = Route {
                        points: vec![
                            with_coordinate(sg.start, ss.from, sl),
                            with_coordinate(sg.end, ss.from, sl),
                        ],
                        sides: ss,
                    };
                    let candidates = vec![(*first, fr.clone()), (second, sr.clone())];
                    if !candidates.iter().all(|(_, route)| {
                        route
                            .points
                            .iter()
                            .all(|point| point_in_bounds(*point, bounds))
                    }) || !self.slots_clear(&candidates, &others)
                        || !corner_clear(self.nodes[edge.from].bounds, fs.from, fr.points[0])
                        || !corner_clear(self.nodes[edge.to].bounds, fs.to, fr.points[1])
                        || !corner_clear(
                            self.nodes[self.edges[second].from].bounds,
                            ss.from,
                            sr.points[0],
                        )
                        || !corner_clear(
                            self.nodes[self.edges[second].to].bounds,
                            ss.to,
                            sr.points[1],
                        )
                    {
                        continue;
                    }
                    self.cache[*first] = Some(fr.clone());
                    self.cache[second] = Some(sr.clone());
                    let all: Vec<_> = others.iter().copied().chain([*first, second]).collect();
                    if !self.is_clear(
                        *first,
                        &fr,
                        fg,
                        &others.iter().copied().chain([second]).collect::<Vec<_>>(),
                    ) || !self.is_clear(
                        second,
                        &sr,
                        sg,
                        &others.iter().copied().chain([*first]).collect::<Vec<_>>(),
                    ) {
                        self.cache[*first] = Some(first_route.clone());
                        self.cache[second] = Some(second_route.clone());
                        continue;
                    }
                    let first_label = self.reserve(*first, &fr.points, &all, &other_labels);
                    let mut labels = other_labels.clone();
                    let first_ok =
                        self.label_clear(first_label.as_ref(), *first, &all, &labels, bounds);
                    if let Some(label) = &first_label {
                        labels.push(label.clone());
                    }
                    let second_label = self.reserve(second, &sr.points, &all, &labels);
                    let second_ok =
                        self.label_clear(second_label.as_ref(), second, &all, &labels, bounds);
                    let cost = self.cost(
                        *first,
                        &fr,
                        &others.iter().copied().chain([second]).collect::<Vec<_>>(),
                    ) + self.cost(
                        second,
                        &sr,
                        &others.iter().copied().chain([*first]).collect::<Vec<_>>(),
                    );
                    self.cache[*first] = Some(first_route.clone());
                    self.cache[second] = Some(second_route.clone());
                    if first_ok && second_ok && cost < best_cost {
                        best = Some((fr, sr, first_label, second_label));
                        best_cost = cost;
                    }
                }
            }
            if let Some((fr, sr, fl, sl)) = best {
                self.cache[*first] = Some(fr);
                self.cache[second] = Some(sr);
                self.reserved = other_labels;
                self.reserved.extend(fl);
                self.reserved.extend(sl);
                improved.insert(*first);
                improved.insert(second);
            } else {
                self.reserved = previous_labels;
            }
        }
        improved
    }
    fn improve_complex(
        &mut self,
        resolved: &[usize],
        bounds: Rect,
        joint: &std::collections::HashSet<usize>,
        only_crossings: bool,
    ) {
        for index in resolved {
            let edge = &self.edges[*index];
            if joint.contains(index) || edge.explicit() || edge.label_pinned() {
                continue;
            }
            let others: Vec<_> = resolved
                .iter()
                .copied()
                .filter(|other| other != index)
                .collect();
            let route = self.cache[*index].as_ref().unwrap().clone();
            let crossings = self.crossing_count(*index, &route.points, &others);
            let labels: Vec<_> = self
                .reserved
                .iter()
                .filter(|label| label.edge != *index)
                .cloned()
                .collect();
            let current_label = self.reserve(*index, &route.points, &others, &labels);
            let blocked = !string(&edge.raw, "label").is_empty() && current_label.is_none();
            if !blocked && crossings == 0 && (only_crossings || route.points.len() <= 4) {
                continue;
            }
            let mut best = None;
            let mut cost = if blocked {
                f64::INFINITY
            } else {
                self.cost(*index, &route, &others)
            };
            for sides in self.candidate_sides(*index) {
                let geometry = self.geometry(*index, sides);
                let candidate = self.routed(*index, geometry, &others);
                if !candidate
                    .points
                    .iter()
                    .all(|point| point_in_bounds(*point, bounds))
                    || !self.is_clear(*index, &candidate, geometry, &others)
                    || (only_crossings
                        && self.crossing_count(*index, &candidate.points, &others) >= crossings)
                {
                    continue;
                }
                let candidate_cost = self.cost(*index, &candidate, &others);
                if candidate_cost >= cost {
                    continue;
                }
                let label = self.reserve(
                    *index,
                    &candidate.points,
                    &others.iter().copied().chain([*index]).collect::<Vec<_>>(),
                    &labels,
                );
                if !self.label_clear(label.as_ref(), *index, &others, &labels, bounds) {
                    continue;
                }
                if only_crossings
                    && (!self.slots_clear(&[(*index, candidate.clone())], &others)
                        || label.as_ref().is_some_and(|label| {
                            self.nodes.iter().any(|node| {
                                crate::geometry::rects_overlap(&label.rect, &node.bounds, -2.0)
                            })
                        }))
                {
                    continue;
                }
                cost = candidate_cost;
                best = Some((candidate, label));
            }
            if let Some((candidate, label)) = best {
                self.cache[*index] = Some(candidate);
                self.reserved = labels;
                self.reserved.extend(label);
            }
        }
    }
    fn straighten(
        &mut self,
        resolved: &[usize],
        bounds: Rect,
        joint: &std::collections::HashSet<usize>,
    ) {
        for index in resolved {
            let edge = &self.edges[*index];
            let original = self.cache[*index].as_ref().unwrap().clone();
            if original.points.len() < 4
                || joint.contains(index)
                || edge.explicit()
                || edge.label_pinned()
            {
                continue;
            }
            let from = self.nodes[edge.from].bounds;
            let to = self.nodes[edge.to].bounds;
            let facing = if from.x + from.width <= to.x {
                Some(Side::Right)
            } else if to.x + to.width <= from.x {
                Some(Side::Left)
            } else if from.y + from.height <= to.y {
                Some(Side::Bottom)
            } else if to.y + to.height <= from.y {
                Some(Side::Top)
            } else {
                None
            };
            let Some(side) = facing else {
                continue;
            };
            let sides = Sides {
                from: side,
                to: side.opposite(),
            };
            let (low, high) = if side.horizontal() {
                (
                    from.y.max(to.y) + 16.0,
                    (from.y + from.height).min(to.y + to.height) - 16.0,
                )
            } else {
                (
                    from.x.max(to.x) + 16.0,
                    (from.x + from.width).min(to.x + to.width) - 16.0,
                )
            };
            if high < low
                || Side::parse(&edge.raw["fromSide"]).is_some_and(|side| side != sides.from)
                || Side::parse(&edge.raw["toSide"]).is_some_and(|side| side != sides.to)
            {
                continue;
            }
            let geometry = self.geometry(*index, sides);
            let others: Vec<_> = resolved
                .iter()
                .copied()
                .filter(|other| other != index)
                .collect();
            let labels: Vec<_> = self
                .reserved
                .iter()
                .filter(|label| label.edge != *index)
                .cloned()
                .collect();
            let before = self.crossing_count(*index, &original.points, &others);
            let a = coordinate(geometry.start, side);
            let b = coordinate(geometry.end, side);
            for lane in unique(
                vec![a, b, (a + b) / 2.0, (low + high) / 2.0]
                    .into_iter()
                    .map(|value| value.clamp(low, high))
                    .collect(),
            ) {
                let candidate = Route {
                    points: vec![
                        with_coordinate(geometry.start, side, lane),
                        with_coordinate(geometry.end, side, lane),
                    ],
                    sides,
                };
                if !candidate
                    .points
                    .iter()
                    .all(|point| point_in_bounds(*point, bounds))
                    || !self.slots_clear(&[(*index, candidate.clone())], &others)
                    || !self.is_clear(*index, &candidate, geometry, &others)
                    || self.crossing_count(*index, &candidate.points, &others) > before
                {
                    continue;
                }
                let label = self.reserve(
                    *index,
                    &candidate.points,
                    &others.iter().copied().chain([*index]).collect::<Vec<_>>(),
                    &labels,
                );
                if !self.label_clear(label.as_ref(), *index, &others, &labels, bounds) {
                    continue;
                }
                self.cache[*index] = Some(candidate);
                self.reserved = labels;
                self.reserved.extend(label);
                break;
            }
        }
    }
    fn relocated(
        &mut self,
        index: usize,
        candidate: &Route,
        others: &[usize],
        bounds: Rect,
    ) -> Option<Vec<Label>> {
        let displaced: Vec<_> = self
            .reserved
            .iter()
            .filter(|label| {
                label.edge != index
                    && candidate
                        .points
                        .windows(2)
                        .any(|pair| segment_intersects_rect(pair[0], pair[1], &label.rect, 4.0))
            })
            .cloned()
            .collect();
        if displaced.len() != 1 {
            return None;
        }
        let owner = displaced[0].edge;
        if self.edges[owner].explicit() || self.edges[owner].label_pinned() {
            return None;
        }
        let mut labels: Vec<_> = self
            .reserved
            .iter()
            .filter(|label| label.edge != index && label.edge != owner)
            .cloned()
            .collect();
        let all: Vec<_> = others.iter().copied().chain([index]).collect();
        let previous = self.cache[index].replace(candidate.clone());
        let mut valid = true;
        for entry in [owner, index] {
            let points = self.cache[entry].as_ref().unwrap().points.clone();
            let label = self.reserve(entry, &points, &all, &labels);
            if !self.label_clear(label.as_ref(), entry, &all, &labels, bounds)
                || label.as_ref().is_some_and(|label| {
                    self.nodes
                        .iter()
                        .any(|node| crate::geometry::rects_overlap(&label.rect, &node.bounds, -2.0))
                })
            {
                valid = false;
                break;
            }
            labels.extend(label);
        }
        self.cache[index] = previous;
        valid.then_some(labels)
    }
    fn relocate_labels(
        &mut self,
        resolved: &[usize],
        bounds: Rect,
        joint: &std::collections::HashSet<usize>,
    ) {
        for index in resolved {
            let edge = &self.edges[*index];
            if joint.contains(index) || edge.explicit() || edge.label_pinned() {
                continue;
            }
            let others: Vec<_> = resolved
                .iter()
                .copied()
                .filter(|other| other != index)
                .collect();
            let original = self.cache[*index].as_ref().unwrap().clone();
            let before = self.crossing_count(*index, &original.points, &others);
            if original.points.len() <= 4 && before == 0 {
                continue;
            }
            let mut best = None;
            let mut cost = self.cost(*index, &original, &others);
            if self.reserved.iter().any(|label| label.edge != *index) {
                for sides in self.candidate_sides(*index) {
                    let geometry = self.geometry(*index, sides);
                    self.honor_labels = false;
                    let candidate = self.routed(*index, geometry, &others);
                    let clear = self.is_clear(*index, &candidate, geometry, &others);
                    self.honor_labels = true;
                    let candidate_cost = self.cost(*index, &candidate, &others);
                    if candidate_cost >= cost
                        || !clear
                        || !candidate
                            .points
                            .iter()
                            .all(|point| point_in_bounds(*point, bounds))
                        || self.crossing_count(*index, &candidate.points, &others) > before
                        || !self.slots_clear(&[(*index, candidate.clone())], &others)
                    {
                        continue;
                    }
                    if let Some(labels) = self.relocated(*index, &candidate, &others, bounds) {
                        cost = candidate_cost;
                        best = Some((candidate, labels));
                    }
                }
            }
            if let Some((candidate, labels)) = best {
                self.cache[*index] = Some(candidate);
                self.reserved = labels;
            }
        }
    }
    fn corner_routes(
        &mut self,
        resolved: &[usize],
        bounds: Rect,
        joint: &std::collections::HashSet<usize>,
    ) {
        for index in resolved {
            let edge = &self.edges[*index];
            let original = self.cache[*index].as_ref().unwrap().clone();
            if original.points.len() < 4
                || joint.contains(index)
                || edge.explicit()
                || edge.label_pinned()
            {
                continue;
            }
            let others: Vec<_> = resolved
                .iter()
                .copied()
                .filter(|other| other != index)
                .collect();
            let labels: Vec<_> = self
                .reserved
                .iter()
                .filter(|label| label.edge != *index)
                .cloned()
                .collect();
            let before = self.crossing_count(*index, &original.points, &others);
            let mut best = None;
            let mut cost = self.cost(*index, &original, &others);
            for sides in self.candidate_sides(*index) {
                if sides.from.horizontal() == sides.to.horizontal() {
                    continue;
                }
                let geometry = self.geometry(*index, sides);
                let slots = |rect: Rect, side: Side, preferred: Point| {
                    let (origin, size) = if side.horizontal() {
                        (rect.y, rect.height)
                    } else {
                        (rect.x, rect.width)
                    };
                    let inset = 16.0_f64.min(size / 2.0);
                    unique(vec![
                        coordinate(preferred, side),
                        origin + inset,
                        origin + size - inset,
                    ])
                    .into_iter()
                    .map(|value| with_coordinate(preferred, side, value))
                    .filter(|point| corner_clear(rect, side, *point))
                    .collect::<Vec<_>>()
                };
                for start in slots(self.nodes[edge.from].bounds, sides.from, geometry.start) {
                    for end in slots(self.nodes[edge.to].bounds, sides.to, geometry.end) {
                        let corner = if sides.from.horizontal() {
                            Point {
                                x: end.x,
                                y: start.y,
                            }
                        } else {
                            Point {
                                x: start.x,
                                y: end.y,
                            }
                        };
                        let candidate = Route {
                            points: normalize_route_points(&[start, corner, end]),
                            sides,
                        };
                        let candidate_cost = self.cost(*index, &candidate, &others);
                        if candidate_cost >= cost
                            || candidate.points.len() != 3
                            || !candidate
                                .points
                                .iter()
                                .all(|point| point_in_bounds(*point, bounds))
                            || !self.slots_clear(&[(*index, candidate.clone())], &others)
                            || !self.is_clear(
                                *index,
                                &candidate,
                                Geometry {
                                    start,
                                    end,
                                    crowded: false,
                                    ..geometry
                                },
                                &others,
                            )
                            || self.crossing_count(*index, &candidate.points, &others) > before
                        {
                            continue;
                        }
                        let label = self.reserve(
                            *index,
                            &candidate.points,
                            &others.iter().copied().chain([*index]).collect::<Vec<_>>(),
                            &labels,
                        );
                        if !self.label_clear(label.as_ref(), *index, &others, &labels, bounds)
                            || label.as_ref().is_some_and(|label| {
                                self.nodes.iter().any(|node| {
                                    crate::geometry::rects_overlap(&label.rect, &node.bounds, -2.0)
                                })
                            })
                        {
                            continue;
                        }
                        cost = candidate_cost;
                        best = Some((candidate, label));
                    }
                }
            }
            if let Some((candidate, label)) = best {
                self.cache[*index] = Some(candidate);
                self.reserved = labels;
                self.reserved.extend(label);
            }
        }
    }
    fn improve(&mut self, resolved: &[usize], grow_bottom: bool) {
        if resolved.is_empty() {
            return;
        }
        let mut bounds = self.scene_bounds(resolved);
        self.allow_grid = false;
        let joint = self.improve_reciprocals(resolved, bounds);
        self.improve_complex(resolved, bounds, &joint, false);
        self.straighten(resolved, bounds, &joint);
        self.relocate_labels(resolved, bounds, &joint);
        self.corner_routes(resolved, bounds, &joint);
        if grow_bottom {
            bounds.height += 24.0;
            self.improve_complex(resolved, bounds, &joint, true);
        }
        self.allow_grid = true;
    }
}

fn length(points: &[Point]) -> f64 {
    points.windows(2).map(|pair| span(pair[0], pair[1])).sum()
}
fn unique(values: Vec<f64>) -> Vec<f64> {
    let mut result = vec![];
    for value in values {
        if !result.contains(&value) {
            result.push(value);
        }
    }
    result
}
fn point_in_bounds(point: Point, bounds: Rect) -> bool {
    point.x >= bounds.x
        && point.y >= bounds.y
        && point.x <= bounds.x + bounds.width
        && point.y <= bounds.y + bounds.height
}
fn in_bounds(rect: Rect, bounds: Rect) -> bool {
    point_in_bounds(
        Point {
            x: rect.x,
            y: rect.y,
        },
        bounds,
    ) && point_in_bounds(
        Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
        bounds,
    )
}
