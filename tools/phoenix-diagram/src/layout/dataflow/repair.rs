// Source: dataflow/joint-label-routing.mjs; proposals commit only when all routes clear.
use super::*;
use crate::geometry::route_quality::{
    shortest_orthogonal_grid_route_accept, GridRouteOptions, Segment,
};

fn clear_route(
    context: &Context<'_>,
    index: usize,
    points: &[Point],
    labels: &[Plate],
    routes: &[Vec<Point>],
    bottom: f64,
) -> bool {
    sides_ok(points, &context.flows[index].sides)
        && points.iter().all(|point| {
            point.x >= 0.0
                && point.y >= 0.0
                && point.x <= context.view_box.width
                && point.y <= bottom
        })
        && !context.intrudes(index, points)
        && rhythm(points)
        && !context.border_runs(points, &context.frames, true)
        && labels.iter().all(|plate| {
            if plate.index() == index {
                route_distance(points, &plate.label.bounds) <= 36.0
            } else {
                !masks(points, &plate.label.bounds)
            }
        })
        && routes.iter().enumerate().all(|(other, route)| {
            other == index
                || !geometry_pinned(&context.flows[other].raw)
                || intersections(points, route) == 0
        })
        && routes.iter().enumerate().all(|(other, route)| {
            other == index
                || !context.corridors(index, points, other, route, true)
                || context.corridors(
                    index,
                    &context.flows[index].points,
                    other,
                    &context.flows[other].points,
                    true,
                )
        })
}

struct Candidate {
    plate: Plate,
    points: Vec<Point>,
    blocked: Vec<usize>,
    own_distance: f64,
    movement: f64,
}

pub(super) fn joint(context: &mut Context<'_>) {
    if !context.automatic_showcase
        || context.nodes.len() + context.frames.len() + context.labels.len().saturating_sub(1) > 80
    {
        return;
    }
    let rectangles: Vec<_> = context
        .nodes
        .iter()
        .map(|node| node.bounds)
        .chain(context.titles())
        .collect();
    let borders: Vec<_> = context
        .frames
        .iter()
        .flat_map(composition::frame_border_segments)
        .map(|segment| Segment {
            start: segment.start,
            end: segment.end,
        })
        .collect();
    let mut searches = 0;
    let mut changed = false;
    for label_index in 0..context.labels.len() {
        if searches >= 32 {
            break;
        }
        let label = context.labels[label_index].clone();
        let index = label.index();
        let own = &context.flows[index];
        if geometry_pinned(&own.raw)
            || (!context
                .flows
                .iter()
                .enumerate()
                .any(|(other, flow)| other != index && masks(&flow.points, &label.label.bounds))
                && !context
                    .nodes
                    .iter()
                    .any(|node| geometry::rects_overlap(&label.label.bounds, &node.bounds, -2.0)))
        {
            continue;
        }
        let mut own_options = vec![own.points.clone()];
        for stub in [24.0, 16.0] {
            for points in
                routing::bridges(own.points[0], *own.points.last().unwrap(), &own.sides, stub)
            {
                unique(&mut own_options, points);
            }
        }
        let mut xs = vec![label.label.bounds.x];
        let mut ys = vec![label.label.bounds.y];
        for points in &own_options {
            for pair in points.windows(2) {
                for x in [pair[0].x, pair[1].x, (pair[0].x + pair[1].x) / 2.0] {
                    unique(&mut xs, x - label.label.bounds.width / 2.0);
                }
                for y in [pair[0].y, pair[1].y, (pair[0].y + pair[1].y) / 2.0] {
                    unique(&mut ys, y - label.label.bounds.height - 4.0);
                    unique(&mut ys, y + 4.0);
                }
            }
        }
        for rect in rectangles.iter().copied().chain(
            context
                .labels
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != label_index)
                .map(|(_, plate)| plate.label.bounds),
        ) {
            unique(&mut xs, rect.x - label.label.bounds.width - 2.0);
            unique(&mut xs, rect.x + rect.width + 2.0);
            unique(&mut ys, rect.y - label.label.bounds.height - 2.0);
            unique(&mut ys, rect.y + rect.height + 2.0);
        }
        for flow in &context.flows {
            for pair in flow.points.windows(2) {
                if pair[0].x == pair[1].x {
                    unique(&mut xs, pair[0].x - label.label.bounds.width - 8.0);
                    unique(&mut xs, pair[0].x + 8.0);
                }
                if pair[0].y == pair[1].y {
                    unique(&mut ys, pair[0].y - label.label.bounds.height - 8.0);
                    unique(&mut ys, pair[0].y + 8.0);
                }
            }
        }
        let (left, right, top, bottom) = own_options.iter().flat_map(|points| points.iter()).fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |(left, right, top, bottom), point| {
                (
                    left.min(point.x),
                    right.max(point.x),
                    top.min(point.y),
                    bottom.max(point.y),
                )
            },
        );
        let routes: Vec<_> = context
            .flows
            .iter()
            .map(|flow| flow.points.clone())
            .collect();
        let mut candidates = Vec::new();
        for x in xs {
            if x + label.label.bounds.width < left - 36.0 || x > right + 36.0 {
                continue;
            }
            for &y in &ys {
                if y + label.label.bounds.height < top - 36.0 || y > bottom + 36.0 {
                    continue;
                }
                let plate = Plate {
                    label: labels::Label {
                        bounds: Rect {
                            x,
                            y,
                            ..label.label.bounds
                        },
                        position: p(
                            x + label.label.bounds.width / 2.0,
                            y + (label.label.position.y - label.label.bounds.y),
                        ),
                        ..label.label.clone()
                    },
                    lines: label.lines.clone(),
                };
                if !context.clear_label(&plate.label, label_index, &context.labels) {
                    continue;
                }
                for points in &own_options {
                    let own_distance = route_distance(points, &plate.label.bounds);
                    if own_distance < 4.0 || own_distance > 36.0 {
                        continue;
                    }
                    let mut next_labels = context.labels.clone();
                    next_labels[label_index] = plate.clone();
                    if !clear_route(
                        context,
                        index,
                        points,
                        &next_labels,
                        &routes,
                        context.view_box.height - 74.0,
                    ) {
                        continue;
                    }
                    let blocked: Vec<_> = context
                        .flows
                        .iter()
                        .enumerate()
                        .filter(|(other, flow)| {
                            *other != index && masks(&flow.points, &plate.label.bounds)
                        })
                        .map(|(index, _)| index)
                        .collect();
                    if blocked.len() > 4
                        || blocked
                            .iter()
                            .any(|&index| geometry_pinned(&context.flows[index].raw))
                    {
                        continue;
                    }
                    let stable_distance = context
                        .flows
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != index && !blocked.contains(other))
                        .map(|(_, flow)| route_distance(&flow.points, &plate.label.bounds))
                        .fold(f64::INFINITY, f64::min);
                    if own_distance > stable_distance + geometry::EPSILON {
                        continue;
                    }
                    candidates.push(Candidate {
                        plate: plate.clone(),
                        points: points.clone(),
                        blocked,
                        own_distance,
                        movement: (x - label.label.bounds.x).hypot(y - label.label.bounds.y),
                    });
                }
            }
        }
        candidates.sort_by(|a, b| {
            a.blocked
                .len()
                .cmp(&b.blocked.len())
                .then_with(|| a.own_distance.total_cmp(&b.own_distance))
                .then_with(|| a.movement.total_cmp(&b.movement))
        });
        let mut chosen: Vec<Candidate> = Vec::new();
        for candidate in candidates {
            if chosen.iter().any(|other| {
                (candidate.plate.label.bounds.x - other.plate.label.bounds.x)
                    .hypot(candidate.plate.label.bounds.y - other.plate.label.bounds.y)
                    < 8.0
                    && candidate.points == other.points
            }) {
                continue;
            }
            chosen.push(candidate);
            if chosen.len() >= 16 {
                break;
            }
        }
        for candidate in chosen {
            if searches >= 32 {
                break;
            }
            let mut next_labels = context.labels.clone();
            next_labels[label_index] = candidate.plate.clone();
            let mut next_routes = routes.clone();
            next_routes[index] = candidate.points;
            let mut blocked = candidate.blocked;
            blocked.sort_by(|&a, &b| {
                length(&context.flows[b].points).total_cmp(&length(&context.flows[a].points))
            });
            let mut failed = false;
            for old in blocked {
                if searches >= 32 {
                    failed = true;
                    break;
                }
                searches += 1;
                let flow = &context.flows[old];
                let endpoints = [flow.points[0], *flow.points.last().unwrap()];
                let obstacles: Vec<_> =
                    context
                        .nodes
                        .iter()
                        .enumerate()
                        .map(|(node_index, node)| {
                            if node_index == flow.from || node_index == flow.to {
                                node.bounds
                            } else {
                                Rect {
                                    x: node.bounds.x - 12.0,
                                    y: node.bounds.y - 12.0,
                                    width: node.bounds.width + 24.0,
                                    height: node.bounds.height + 24.0,
                                }
                            }
                        })
                        .chain(context.titles())
                        .chain(next_labels.iter().filter(|plate| plate.index() != old).map(
                            |plate| {
                                let rect = plate.label.bounds;
                                Rect {
                                    x: rect.x - 2.0,
                                    y: rect.y - 2.0,
                                    width: rect.width + 4.0,
                                    height: rect.height + 4.0,
                                }
                            },
                        ))
                        .collect();
                let avoided: Vec<_> = next_routes
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != old && !related(flow, &context.flows[*other]))
                    .flat_map(|(_, points)| {
                        points.windows(2).map(|pair| Segment {
                            start: pair[0],
                            end: pair[1],
                        })
                    })
                    .collect();
                let mut options = GridRouteOptions::new(
                    endpoints[0],
                    endpoints[1],
                    &endpoints,
                    &obstacles,
                    &flow.sides.0,
                    &flow.sides.1,
                );
                options.endpoint_stub_px = Some(24.0);
                options.maximum_grid_nodes = 4096.0;
                options.avoided_segments = &avoided;
                options.allow_avoided_crossings = true;
                options.minimum_segment_px = 16.0;
                options.bend_penalty_px = 48.0;
                options.border_segments = &borders;
                let found = shortest_orthogonal_grid_route_accept(&options, |points| {
                    clear_route(
                        context,
                        old,
                        points,
                        &next_labels,
                        &next_routes,
                        context.view_box.height - 74.0 + label.label.bounds.height,
                    )
                });
                let Some(found) = found.route else {
                    failed = true;
                    break;
                };
                next_routes[old] = geometry::normalize_route_points(&found.points);
            }
            if failed {
                continue;
            }
            let foreign_distance = next_routes
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, points)| route_distance(points, &candidate.plate.label.bounds))
                .fold(f64::INFINITY, f64::min);
            if candidate.own_distance > foreign_distance + geometry::EPSILON {
                continue;
            }
            context.labels = next_labels;
            for (flow, points) in context.flows.iter_mut().zip(next_routes) {
                flow.points = points;
            }
            changed = true;
            break;
        }
    }
    if changed {
        capacity::fit_height(context);
        context.mark_crossovers();
    }
}
