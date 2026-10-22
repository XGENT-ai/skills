use super::*;
use crate::geometry::composition::collect_arrowhead_collisions;
use crate::geometry::route_quality::{shortest_orthogonal_grid_route_accept, GridRouteOptions};

fn stroke_width(edge: &Value) -> f64 {
    number(
        edge,
        "width",
        if string(edge, "variant") == "emphasis" {
            1.8
        } else {
            1.4
        },
    )
}
fn arrow_fact(edge: &Value) -> Value {
    let mut fact = edge.clone();
    fact["width"] = json!(stroke_width(edge));
    fact
}
fn coordinate(point: Point, vertical: bool) -> f64 {
    if vertical {
        point.y
    } else {
        point.x
    }
}
fn shift(point: &mut Point, vertical: bool, offset: f64) {
    if vertical {
        point.y += offset;
    } else {
        point.x += offset;
    }
}

impl Router<'_> {
    pub(super) fn later_arrival_blocks(&self, edge: &Value, points: &[Point]) -> f64 {
        if !independent(edge)
            || ["labelDx", "labelDy", "labelSegment"]
                .iter()
                .any(|field| edge.get(*field).is_some())
        {
            return 0.0;
        }
        let from = self.node(string(edge, "from"));
        let to = self.node(string(edge, "to"));
        if from.authored.facts["col"] != to.authored.facts["col"] || center(to).y >= center(from).y
        {
            return 0.0;
        }
        let eligible = self
            .edges
            .iter()
            .filter(|incoming| {
                incoming.value != *edge
                    && !self.placed.iter().any(|placed| {
                        placed.authored.source_path == format!("/edges/{}", incoming.index)
                    })
                    && independent(&incoming.value)
                    && ["labelDx", "labelDy", "labelSegment"]
                        .iter()
                        .all(|field| incoming.value.get(*field).is_none())
            })
            .filter(|incoming| {
                let source = self.node(string(&incoming.value, "from"));
                let target = self.node(string(&incoming.value, "to"));
                target.authored.facts["col"] == from.authored.facts["col"]
                    && source.authored.facts["col"] != target.authored.facts["col"]
                    && center(target).y > center(to).y
                    && center(target).y < center(from).y
            })
            .collect::<Vec<_>>();
        if eligible.len() > 8 {
            return 0.0;
        }
        let mut blocked = 0.0;
        for incoming in eligible {
            let source = self.node(string(&incoming.value, "from"));
            let target = self.node(string(&incoming.value, "to"));
            let a = if center(source).x < center(target).x {
                "right"
            } else {
                "left"
            };
            let b = if center(source).x < center(target).x {
                "left"
            } else {
                "right"
            };
            let start = anchor(&source.bounds, a);
            let end = anchor(&target.bounds, b);
            let prospective = join(
                start,
                corridor_x(start, end, a, b, (start.x + end.x) / 2.0),
                end,
            );
            if self
                .rejection(&incoming.value, &prospective, a, b)
                .is_some()
            {
                continue;
            }
            let cost = self.cost(&incoming.value, &prospective, 0.0, (a, b));
            if cost[0] != 0.0 || cost[1] != 0.0 || cost[2] != 0.0 || cost[4] != 0.0 {
                continue;
            }
            let arrival = stub(end, b);
            if points
                .windows(2)
                .enumerate()
                .skip(1)
                .take(points.len().saturating_sub(3))
                .any(|(_, pair)| {
                    proper_segment_intersection(pair[0], pair[1], arrival, end).is_some()
                })
            {
                blocked += 1.0;
            }
        }
        blocked
    }
    fn corridor_hits(&self, edge: &Value, points: &[Point]) -> Vec<(usize, usize)> {
        self.placed
            .iter()
            .enumerate()
            .flat_map(|(index, other)| {
                let routed = [
                    RoutedRelation::new(0, edge.clone(), points.to_vec()),
                    RoutedRelation::new(1, other.authored.facts.clone(), other.points.clone()),
                ];
                collect_ambiguous_corridors(
                    &routed,
                    &CorridorOptions {
                        contract: self.input.input_contract,
                        include_shared_endpoints: Some(&|_, _| true),
                        allow_short_workflow_trunks: true,
                        ..Default::default()
                    },
                )
                .into_iter()
                .map(move |hit| (index, hit.left_segment))
            })
            .collect()
    }
    pub(super) fn endpoint_repairs(
        &self,
        edge: &Value,
        selected: &Candidate,
        natural: (&str, &str),
        narrow_slots: bool,
    ) -> Vec<Candidate> {
        if !independent(edge)
            || ["labelDx", "labelDy", "labelSegment", "bias"]
                .iter()
                .any(|field| edge.get(*field).is_some())
            || selected.cost[0] != 0.0
            || (selected.cost[1] == 0.0 && selected.cost[2] == 0.0)
            || selected.points.len() < 3
        {
            return Vec::new();
        }
        let original_hits = self.corridor_hits(edge, &selected.points);
        let original_neighbours = original_hits
            .iter()
            .map(|hit| hit.0)
            .collect::<BTreeSet<_>>();
        let contacts = if selected.cost[1] > 0.0 {
            self.placed
                .iter()
                .filter(|other| {
                    ![string(edge, "from"), string(edge, "to")]
                        .iter()
                        .any(|id| *id == other.from || *id == other.to)
                })
                .flat_map(|other| bend_contact_points(&selected.points, &other.points))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut repairs = Vec::new();
        for first in [true, false] {
            let endpoint_index = if first { 0 } else { selected.points.len() - 1 };
            let segment_index = if first { 0 } else { selected.points.len() - 2 };
            let corner_index = if first { 1 } else { selected.points.len() - 2 };
            let corner = selected.points[corner_index];
            if !contacts
                .iter()
                .any(|point| (point.x - corner.x).hypot(point.y - corner.y) <= 0.0001)
                && !original_hits.iter().any(|hit| hit.1 == segment_index)
            {
                continue;
            }
            let node = self.node(string(edge, if first { "from" } else { "to" }));
            let side = if first {
                &selected.from_side
            } else {
                &selected.to_side
            };
            let vertical = ["left", "right"].contains(&side.as_str());
            let minimum = if vertical {
                node.bounds.y
            } else {
                node.bounds.x
            } + 16.0;
            let maximum = if vertical {
                node.bounds.y + node.bounds.height
            } else {
                node.bounds.x + node.bounds.width
            } - 16.0;
            let mut offsets = vec![-8.0, 8.0, -10.0, 10.0];
            if narrow_slots {
                let footprint = if first { 0.5 } else { 3.5 } * stroke_width(edge);
                let mut slots = vec![(minimum, maximum)];
                for other in &self.placed {
                    for (other_first, id, point) in [
                        (true, &other.from, other.points[0]),
                        (false, &other.to, *other.points.last().unwrap()),
                    ] {
                        if id != &node.authored.id
                            || (coordinate(point, !vertical)
                                - coordinate(selected.points[endpoint_index], !vertical))
                            .abs()
                                > 0.0001
                        {
                            continue;
                        }
                        let radius = footprint
                            + (if other_first { 0.5 } else { 3.5 })
                                * stroke_width(&other.authored.facts);
                        let low = coordinate(point, vertical) - radius;
                        let high = coordinate(point, vertical) + radius;
                        slots = slots
                            .into_iter()
                            .flat_map(|(start, end)| {
                                if high <= start || low >= end {
                                    vec![(start, end)]
                                } else {
                                    vec![(start, end.min(low)), (start.max(high), end)]
                                        .into_iter()
                                        .filter(|(a, b)| b > a)
                                        .collect()
                                }
                            })
                            .collect();
                    }
                }
                let mut gaps = slots
                    .into_iter()
                    .map(|(start, end)| {
                        (start + end) / 2.0 - coordinate(selected.points[endpoint_index], vertical)
                    })
                    .collect::<Vec<_>>();
                gaps.sort_by(|a, b| a.abs().total_cmp(&b.abs()).then_with(|| a.total_cmp(b)));
                gaps.truncate(4);
                offsets.extend(gaps);
            }
            for offset in offsets {
                let mut points = selected.points.clone();
                shift(&mut points[endpoint_index], vertical, offset);
                shift(&mut points[corner_index], vertical, offset);
                let port = coordinate(points[endpoint_index], vertical);
                if port < minimum || port > maximum {
                    continue;
                }
                let footprint = if first { 0.5 } else { 3.5 } * stroke_width(edge);
                let clear = self.placed.iter().all(|other| {
                    [
                        (true, &other.from, other.points[0]),
                        (false, &other.to, *other.points.last().unwrap()),
                    ]
                    .iter()
                    .all(|(other_first, id, point)| {
                        *id != &node.authored.id
                            || (coordinate(*point, !vertical)
                                - coordinate(points[endpoint_index], !vertical))
                            .abs()
                                > 0.0001
                            || (coordinate(*point, vertical) - port).abs() + 0.0001
                                >= footprint
                                    + (if *other_first { 0.5 } else { 3.5 })
                                        * stroke_width(&other.authored.facts)
                    })
                });
                if !clear
                    || self
                        .rejection(edge, &points, &selected.from_side, &selected.to_side)
                        .is_some()
                {
                    continue;
                }
                let cost = self.cost(edge, &points, *selected.cost.last().unwrap(), natural);
                if cost[0] != 0.0
                    || cost[1] > selected.cost[1]
                    || cost[2] > selected.cost[2]
                    || (cost[1] == selected.cost[1] && cost[2] == selected.cost[2])
                {
                    continue;
                }
                if self
                    .corridor_hits(edge, &points)
                    .iter()
                    .any(|hit| !original_neighbours.contains(&hit.0))
                {
                    continue;
                }
                let mut arrows = vec![RoutedRelation::new(0, arrow_fact(edge), points.clone())];
                for (index, other) in self.placed.iter().enumerate() {
                    if other.from == string(edge, "to") || other.to == string(edge, "to") {
                        arrows.push(RoutedRelation::new(
                            index + 1,
                            arrow_fact(&other.authored.facts),
                            other.points.clone(),
                        ));
                    }
                }
                if collect_arrowhead_collisions(&arrows, true)
                    .iter()
                    .any(|hit| hit.left_relation_index == 0 || hit.right_relation_index == 0)
                {
                    continue;
                }
                repairs.push(Candidate {
                    points,
                    cost,
                    ..selected.clone()
                });
            }
        }
        repairs
    }
    pub(super) fn internal_detours(
        &self,
        edge: &Value,
        selected: &Candidate,
        natural: (&str, &str),
    ) -> Vec<Candidate> {
        let from = self.node(string(edge, "from"));
        let to = self.node(string(edge, "to"));
        if !independent(edge) || from.authored.facts["col"] != to.authored.facts["col"] {
            return Vec::new();
        }
        let length = polyline_length(&selected.points);
        let direct = distance(center(from), center(to));
        if length <= direct * 2.0 || length - direct <= 64.0 {
            return Vec::new();
        }
        let down = center(to).y > center(from).y;
        let a = if down { "bottom" } else { "top" };
        let b = if down { "top" } else { "bottom" };
        let starts = self.endpoint_candidates(edge, from, a, anchor(&from.bounds, a), to);
        let ends = self.endpoint_candidates(edge, to, b, anchor(&to.bounds, b), from);
        let low = center(from).y.min(center(to).y);
        let high = center(from).y.max(center(to).y);
        let mid = (center(from).x + center(to).x) / 2.0;
        let mut tracks = self
            .measured
            .nodes
            .iter()
            .filter(|node| node.bounds.y <= high && node.bounds.y + node.bounds.height >= low)
            .flat_map(|node| [node.bounds.x - 8.0, node.bounds.x + node.bounds.width + 8.0])
            .filter(|x| *x >= 44.0 && *x <= self.measured.lane_width + 36.0)
            .collect::<Vec<_>>();
        tracks.sort_by(|a, b| a.total_cmp(b));
        tracks.dedup();
        tracks.sort_by(|a, b| {
            (a - mid)
                .abs()
                .total_cmp(&(b - mid).abs())
                .then_with(|| a.total_cmp(b))
        });
        tracks.truncate(24);
        let mut plans = Vec::new();
        for start in starts {
            for end in &ends {
                for x in &tracks {
                    let points = join(start, corridor_x(start, *end, a, b, *x), *end);
                    if self.rejection(edge, &points, a, b).is_none() {
                        let cost = self.cost(edge, &points, 10000.0 + plans.len() as f64, natural);
                        plans.push(Candidate {
                            points,
                            from_side: a.into(),
                            to_side: b.into(),
                            cost,
                        });
                    }
                }
            }
        }
        plans
    }
    pub(super) fn free_space_plans(
        &self,
        edge: &Value,
        selected: &Candidate,
        natural: (&str, &str),
    ) -> Vec<Candidate> {
        let from = self.node(string(edge, "from"));
        let to = self.node(string(edge, "to"));
        if !independent(edge)
            || from.authored.facts["lane"] == to.authored.facts["lane"]
            || ["labelDx", "labelDy", "labelSegment", "bias"]
                .iter()
                .any(|field| edge.get(*field).is_some())
            || (selected.cost[0] == 0.0 && selected.cost[1] == 0.0 && selected.cost[2] == 0.0)
        {
            return Vec::new();
        }
        let mut obstacles = Vec::new();
        let mut add = |rect: Rect, padding: f64| {
            obstacles.push(Rect {
                x: rect.x - padding,
                y: rect.y - padding,
                width: rect.width + padding * 2.0,
                height: rect.height + padding * 2.0,
            })
        };
        for node in &self.measured.nodes {
            add(node.bounds, 8.0);
        }
        for obstacle in &self.measured.label_obstacles {
            add(*obstacle, 4.0);
        }
        for other in &self.placed {
            if let Some(plate) = other.label_plate {
                add(plate, 4.0);
            }
            for pair in other.points.windows(2) {
                add(
                    Rect {
                        x: pair[0].x.min(pair[1].x),
                        y: pair[0].y.min(pair[1].y),
                        width: (pair[0].x - pair[1].x).abs(),
                        height: (pair[0].y - pair[1].y).abs(),
                    },
                    8.0,
                );
            }
        }
        let down = center(to).y > center(from).y;
        let pairs = [
            (
                if down { "bottom" } else { "top" },
                if down { "top" } else { "bottom" },
            ),
            ("left", "left"),
            ("right", "right"),
            (&selected.from_side, &selected.to_side),
        ];
        let mut seen = BTreeSet::new();
        let mut plans = Vec::new();
        for (ordinal, (a, b)) in pairs.into_iter().enumerate() {
            if !seen.insert((a, b)) {
                continue;
            }
            let start = self.endpoint_candidates(edge, from, a, anchor(&from.bounds, a), to)[0];
            let end = self.endpoint_candidates(edge, to, b, anchor(&to.bounds, b), from)[0];
            let mut options = GridRouteOptions::new(start, end, &selected.points, &obstacles, a, b);
            options.clearance = 0.0;
            options.endpoint_stub_px = Some(16.0);
            options.bend_penalty_px = 24.0;
            options.maximum_obstacle_count = 200;
            options.maximum_grid_nodes = 25600.0;
            options.minimum_segment_px = 0.0;
            let searched = shortest_orthogonal_grid_route_accept(&options, |points| {
                if self.rejection(edge, points, a, b).is_some() {
                    return false;
                }
                let cost = self.cost(edge, points, 20000.0 + ordinal as f64, natural);
                cost[0] == 0.0 && cost[1] == 0.0 && cost[2] == 0.0 && cost[4] == 0.0
            });
            if let Some(route) = searched.route {
                let cost = self.cost(edge, &route.points, 20000.0 + ordinal as f64, natural);
                plans.push(Candidate {
                    points: route.points,
                    from_side: a.into(),
                    to_side: b.into(),
                    cost,
                });
            }
        }
        plans
    }
    pub(super) fn repair_plans(
        &self,
        edge: &Value,
        plans: &mut Vec<Candidate>,
        natural: (&str, &str),
    ) {
        if self.input.input_contract != InputContract::PhoenixV1 || plans.is_empty() {
            return;
        }
        let touching = plans
            .iter()
            .filter(|plan| plan.cost[1] > 0.0 && plan.cost[0] == 0.0)
            .take(4)
            .cloned()
            .collect::<Vec<_>>();
        for selected in touching {
            plans.extend(self.endpoint_repairs(edge, &selected, natural, false));
        }
        plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        plans.extend(self.internal_detours(edge, &plans[0], natural));
        plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        let endpoint_repairs = self.endpoint_repairs(edge, &plans[0], natural, false);
        let mut seeds = Vec::new();
        if plans[0].cost[2] > 0.0 {
            for plan in plans.iter() {
                if !seeds.iter().any(|other: &Candidate| {
                    other.from_side == plan.from_side
                        && other.to_side == plan.to_side
                        && other.points[0] == plan.points[0]
                        && other.points.last() == plan.points.last()
                }) {
                    seeds.push(plan.clone());
                    if seeds.len() >= 8 {
                        break;
                    }
                }
            }
        }
        for selected in seeds {
            if selected.cost[2] == 0.0 {
                continue;
            }
            let mut alternatives = Vec::new();
            for segment in 1..selected.points.len() - 2 {
                let vertical = selected.points[segment].x != selected.points[segment + 1].x;
                for offset in [-16.0, 16.0, -32.0, 32.0] {
                    let mut points = selected.points.clone();
                    shift(&mut points[segment], vertical, offset);
                    shift(&mut points[segment + 1], vertical, offset);
                    if self
                        .rejection(edge, &points, &selected.from_side, &selected.to_side)
                        .is_some()
                    {
                        continue;
                    }
                    let cost = self.cost(edge, &points, *selected.cost.last().unwrap(), natural);
                    if compare_cost(&cost, &selected.cost) == Ordering::Less {
                        alternatives.push(Candidate {
                            points,
                            cost,
                            ..selected.clone()
                        });
                    }
                }
            }
            alternatives.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
            if let Some(plan) = alternatives.into_iter().next() {
                plans.push(plan);
            }
        }
        plans.extend(endpoint_repairs);
        plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        if plans[0].cost[2] > 0.0 {
            plans.extend(self.endpoint_repairs(edge, &plans[0], natural, true));
            plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
        }
        plans.extend(self.free_space_plans(edge, &plans[0], natural));
        plans.sort_by(|a, b| compare_cost(&a.cost, &b.cost));
    }
}
