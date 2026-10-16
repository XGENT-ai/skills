// Source: resolvedLabelRects pipeline and dataflow/label-capacity.mjs.
use super::*;

pub(super) fn size(input: &DiagramInput, raw: &Value, lines: &[String]) -> Point {
    let longest = if lines.is_empty() {
        units(input, string(raw, "label"))
    } else {
        lines
            .iter()
            .map(|line| units(input, line))
            .fold(0.0, f64::max)
    }
    .max(units(input, string(raw, "classification")));
    p(
        (34.0_f64.max(longest * 4.9 + 12.0) * 10.0 + 0.5).floor() / 10.0,
        16.0 + 11.0
            * (lines.len().max(1) - 1 + usize::from(!string(raw, "classification").is_empty()))
                as f64,
    )
}
pub(super) fn label_point(context: &Context<'_>, index: usize, points: &[Point]) -> Point {
    let raw = &context.flows[index].raw;
    if context.automatic_showcase
        && string(raw, "classification").is_empty()
        && points.len() == 2
        && points[0].x == points[1].x
        && !label_pinned(raw)
    {
        let at = p(points[0].x, (points[0].y + points[1].y) / 2.0 - 10.0);
        let size = size(context.input, raw, &[]);
        let rect = Rect {
            x: at.x - size.x / 2.0,
            y: at.y - 11.0,
            width: size.x,
            height: size.y,
        };
        if context.flows.iter().enumerate().all(|(other, flow)| {
            other == index || flow.from != context.flows[index].from || !masks(&flow.points, &rect)
        }) {
            return at;
        }
    }
    if raw["labelAt"].is_array() {
        return point(&raw["labelAt"]);
    }
    let dx = number(raw, "labelDx", 0.0);
    let dy = number(raw, "labelDy", 0.0);
    if points.len() == 2 {
        return p(
            (points[0].x + points[1].x) / 2.0 + dx,
            points[0].y - 10.0 + dy,
        );
    }
    let segment = (number(raw, "labelSegment", 1.0) as usize).min(points.len() - 2);
    p(
        (points[segment].x + points[segment + 1].x) / 2.0 + dx,
        (points[segment].y + points[segment + 1].y) / 2.0 - 10.0 + dy,
    )
}
fn extra_line_masks(plate: &Plate, points: &[Point]) -> bool {
    if plate.label.bounds.height <= 16.0 {
        return false;
    }
    points
        .windows(2)
        .any(|pair| geometry::segment_intersects_rect(pair[0], pair[1], &plate.label.bounds, 0.0))
        && !points.windows(2).any(|pair| {
            geometry::segment_intersects_rect(
                pair[0],
                pair[1],
                &Rect {
                    height: 16.0,
                    ..plate.label.bounds
                },
                0.0,
            )
        })
}
fn routes(context: &Context<'_>) -> Vec<labels::Route> {
    context
        .flows
        .iter()
        .enumerate()
        .map(|(index, flow)| labels::Route {
            relation_index: index as isize,
            points: flow.points.clone(),
        })
        .collect()
}
fn place(
    context: &Context<'_>,
    plates: &[Plate],
    routes: &[labels::Route],
    components: &[Rect],
    keep_near: bool,
) -> Vec<Plate> {
    let labels: Vec<_> = plates
        .iter()
        .map(|plate| {
            let mut label = plate.label.clone();
            label.position.y -= 1.0;
            label
        })
        .collect();
    let mut options = labels::Options::new(context.view_box);
    options.placement_bottom = context.view_box.height - 74.0;
    options.keep_fallback_near_route = keep_near;
    options.max_own_route_distance = 36.0;
    labels::place_automatic_labels(&labels, routes, components, &context.titles(), &options)
        .into_iter()
        .zip(plates)
        .map(|(mut label, original)| {
            label.position.y += 1.0;
            Plate {
                label,
                lines: original.lines.clone(),
            }
        })
        .collect()
}
pub(super) fn fit_height(context: &mut Context<'_>) {
    if !context.automatic_showcase {
        return;
    }
    let bottom = context
        .nodes
        .iter()
        .map(|node| node.bounds.y + node.bounds.height)
        .chain(
            context
                .flows
                .iter()
                .flat_map(|flow| flow.points.iter().map(|point| point.y)),
        )
        .chain(
            context
                .labels
                .iter()
                .map(|plate| plate.label.bounds.y + plate.label.bounds.height),
        )
        .fold(0.0, f64::max);
    context.view_box.height = 360.0_f64.max((bottom + 98.0).ceil());
    context.frames = measure::frames(context);
}
fn fit_width(context: &mut Context<'_>) {
    if context.input.document.pointer("/meta/viewBox").is_none() {
        let right = context
            .labels
            .iter()
            .map(|plate| plate.label.bounds.x + plate.label.bounds.width)
            .fold(0.0, f64::max);
        context.view_box.width = context.view_box.width.max((right + 24.0).ceil());
    }
}

pub(super) fn initial(context: &mut Context<'_>) {
    let initial: Vec<_> = context
        .flows
        .iter()
        .enumerate()
        .map(|(index, flow)| {
            let position = label_point(context, index, &flow.points);
            let size = size(context.input, &flow.raw, &[]);
            Plate {
                label: labels::Label {
                    relation_index: index as isize,
                    bounds: Rect {
                        x: position.x - size.x / 2.0,
                        y: position.y - 11.0,
                        width: size.x,
                        height: size.y,
                    },
                    position,
                    pinned: label_pinned(&flow.raw),
                },
                lines: Vec::new(),
            }
        })
        .collect();
    context.labels = initial.clone();
    if context.legacy() {
        return;
    }
    let occlusions: Vec<_> = initial
        .iter()
        .filter(|plate| {
            !string(&context.flows[plate.index()].raw, "classification").is_empty()
                && !plate.label.pinned
                && extra_line_masks(plate, &context.flows[plate.index()].points)
        })
        .map(Plate::index)
        .collect();
    fit_width(context);
    if context.automatic_showcase {
        fit_height(context);
        context.view_box.height += initial
            .iter()
            .map(|plate| plate.label.bounds.height - 16.0)
            .fold(0.0, f64::max);
        context.frames = measure::frames(context);
    }
    let components: Vec<_> = context.nodes.iter().map(|node| node.bounds).collect();
    if context.automatic_showcase {
        context.labels = place(context, &initial, &routes(context), &components, false);
    }
    for (index, plate) in context.labels.iter_mut().enumerate() {
        if route_distance(&context.flows[plate.index()].points, &plate.label.bounds) > 36.0 {
            *plate = initial[index].clone();
        }
    }
    for index in 0..context.labels.len() {
        let current = context.labels[index].clone();
        if !occlusions.contains(&current.index()) {
            continue;
        }
        let points = &context.flows[current.index()].points;
        if !points.windows(2).any(|pair| {
            geometry::segment_intersects_rect(pair[0], pair[1], &current.label.bounds, 0.0)
        }) {
            continue;
        }
        let mut preferred = initial[index].clone();
        let extra = preferred.label.bounds.height - 16.0;
        preferred.label.bounds.y -= extra;
        preferred.label.position.y -= extra;
        let mut proposals = context.labels.clone();
        for (i, plate) in proposals.iter_mut().enumerate() {
            if i == index {
                *plate = preferred.clone();
            } else {
                plate.label.pinned = true;
            }
        }
        let mut bands = components.clone();
        bands.extend(points.windows(2).map(|pair| Rect {
            x: pair[0].x.min(pair[1].x) - 2.0,
            y: pair[0].y.min(pair[1].y) - 2.0,
            width: (pair[1].x - pair[0].x).abs() + 4.0,
            height: (pair[1].y - pair[0].y).abs() + 4.0,
        }));
        let candidate = place(context, &proposals, &routes(context), &bands, true).remove(index);
        if context.clear_label(&candidate.label, index, &context.labels)
            && context
                .flows
                .iter()
                .all(|flow| route_distance(&flow.points, &candidate.label.bounds) >= 4.0)
            && route_distance(points, &candidate.label.bounds) <= 36.0
        {
            context.labels[index] = candidate;
        }
    }
}

fn node_overlap(context: &Context<'_>, plate: &Plate) -> bool {
    context
        .nodes
        .iter()
        .any(|node| geometry::rects_overlap(&plate.label.bounds, &node.bounds, -2.0))
}
fn move_corridors(context: &mut Context<'_>) {
    let components: Vec<_> = context.nodes.iter().map(|node| node.bounds).collect();
    let mut slots = Vec::new();
    let mut probes = 0;
    for index in 0..context.labels.len() {
        let current = &context.labels[index];
        if probes >= 16 || current.label.pinned || !node_overlap(context, current) {
            continue;
        }
        probes += 1;
        let mut labels = context.labels.clone();
        for (other, plate) in labels.iter_mut().enumerate() {
            if other != index {
                plate.label.pinned = true;
            }
        }
        let route = labels::Route {
            relation_index: current.index() as isize,
            points: context.flows[current.index()].points.clone(),
        };
        let probe = place(context, &labels, &[route], &components, false).remove(index);
        if probe != labels[index] {
            slots.push(probe);
        }
    }
    let mut attempts = 0;
    for index in 0..context.flows.len() {
        if attempts >= 16 {
            break;
        }
        if !context.supports_crossover(index) || label_pinned(&context.flows[index].raw) {
            continue;
        }
        let current = geometry::normalize_route_points(&context.flows[index].points);
        let pending: Vec<_> = slots
            .iter()
            .filter(|slot| {
                node_overlap(
                    context,
                    context
                        .labels
                        .iter()
                        .find(|plate| plate.index() == slot.index())
                        .unwrap(),
                )
            })
            .collect();
        let blocked: Vec<_> = context
            .labels
            .iter()
            .chain(pending.iter().copied())
            .filter(|plate| plate.index() != index && masks(&current, &plate.label.bounds))
            .collect();
        if blocked.is_empty() {
            continue;
        }
        let mut candidates = Vec::new();
        for plate in blocked {
            for segment in 1..current.len().saturating_sub(2) {
                let (start, end) = (current[segment], current[segment + 1]);
                if !masks(&[start, end], &plate.label.bounds) {
                    continue;
                }
                let axis = if start.x == end.x {
                    0
                } else if start.y == end.y {
                    1
                } else {
                    continue;
                };
                let (low, size) = if axis == 0 {
                    (plate.label.bounds.x, plate.label.bounds.width)
                } else {
                    (plate.label.bounds.y, plate.label.bounds.height)
                };
                for coordinate in [low - 4.0, low + size + 4.0] {
                    let mut points = current.clone();
                    set_component(&mut points[segment], axis, coordinate);
                    set_component(&mut points[segment + 1], axis, coordinate);
                    if [segment - 1, segment + 1].iter().any(|&k| {
                        (current[k + 1].x - current[k].x) * (points[k + 1].x - points[k].x)
                            + (current[k + 1].y - current[k].y) * (points[k + 1].y - points[k].y)
                            < 0.0
                    }) {
                        continue;
                    }
                    unique(&mut candidates, geometry::normalize_route_points(&points));
                }
            }
        }
        let mut ranked: Vec<_> = candidates
            .into_iter()
            .map(|points| {
                let cost = context.cost(index, &points);
                (points, cost)
            })
            .collect();
        ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
        let old_cost = context.cost(index, &current);
        for (points, cost) in ranked {
            if attempts >= 16 {
                break;
            }
            attempts += 1;
            if points.len() > current.len()
                || cost > old_cost + 96.0
                || folds_back(&points)
                || !sides_ok(&points, &context.flows[index].sides)
                || context.intrudes(index, &points)
                || !rhythm(&points)
                || context.border_runs(&points, &context.frames, false)
                || points.iter().any(|point| {
                    point.x < 0.0
                        || point.y < 0.0
                        || point.x > context.view_box.width
                        || point.y > context.view_box.height - 74.0
                })
                || context.flows.iter().enumerate().any(|(other, flow)| {
                    other != index
                        && !related(&context.flows[index], flow)
                        && (context.corridors(index, &points, other, &flow.points, false) || {
                            let count = intersections(&points, &flow.points);
                            count > intersections(&current, &flow.points)
                                || (count > 0 && !context.supports_crossover(other))
                        })
                })
            {
                continue;
            }
            let mut routes = routes(context);
            routes[index].points = points.clone();
            let placed = place(context, &context.labels, &routes, &components, false);
            // A proposal clears the pending slots this route obstructed. Other
            // unresolved labels remain available for later corridor repairs.
            if placed
                .iter()
                .any(|plate| plate.index() != index && masks(&points, &plate.label.bounds))
                || pending
                    .iter()
                    .filter(|slot| slot.index() != index && masks(&current, &slot.label.bounds))
                    .any(|slot| {
                        node_overlap(
                            context,
                            placed
                                .iter()
                                .find(|plate| plate.index() == slot.index())
                                .unwrap(),
                        )
                    })
            {
                continue;
            }
            if placed.iter().enumerate().any(|(label_index, plate)| {
                let old_points = &context.flows[plate.index()].points;
                let own_points = if plate.index() == index {
                    &points
                } else {
                    old_points
                };
                ((plate.index() == index || *plate != context.labels[label_index])
                    && route_distance(own_points, &plate.label.bounds) > 36.0)
                    || (extra_line_masks(plate, own_points)
                        && !extra_line_masks(&context.labels[label_index], old_points))
            }) {
                continue;
            }
            context.flows[index].points = points;
            context.labels = placed;
            break;
        }
    }
}

fn align_settled_ports(context: &mut Context<'_>) {
    let components: Vec<_> = context.nodes.iter().map(|node| node.bounds).collect();
    let mut probes = 0;
    for index in 0..context.flows.len() {
        if probes >= 16 {
            break;
        }
        if !context.supports_crossover(index) || label_pinned(&context.flows[index].raw) {
            continue;
        }
        let current = geometry::normalize_route_points(&context.flows[index].points);
        if current.len() <= 2 {
            continue;
        }
        let Some(label_index) = context
            .labels
            .iter()
            .position(|plate| plate.index() == index)
        else {
            continue;
        };
        for points in
            routing::alignment_candidates(context, index, current[0], *current.last().unwrap())
        {
            if probes >= 16
                || context.intrudes(index, &points)
                || context.flows.iter().enumerate().any(|(other, flow)| {
                    other != index
                        && !related(&context.flows[index], flow)
                        && intersections(&points, &flow.points)
                            > intersections(&current, &flow.points)
                })
                || context
                    .labels
                    .iter()
                    .any(|plate| plate.index() != index && masks(&points, &plate.label.bounds))
            {
                continue;
            }
            let flow = &context.flows[index];
            let mut occupied = false;
            for (node, side, point, original) in [
                (flow.from, &flow.sides.0, points[0], current[0]),
                (flow.to, &flow.sides.1, points[1], *current.last().unwrap()),
            ] {
                if (point.x - original.x).hypot(point.y - original.y) < geometry::EPSILON {
                    continue;
                }
                for (other, route) in context.flows.iter().enumerate() {
                    if other == index {
                        continue;
                    }
                    for (endpoint, other_side, at) in [
                        (route.from, &route.sides.0, route.points[0]),
                        (route.to, &route.sides.1, *route.points.last().unwrap()),
                    ] {
                        if endpoint == node
                            && other_side == side
                            && (point.x - at.x).hypot(point.y - at.y) < 18.0 - geometry::EPSILON
                        {
                            occupied = true;
                        }
                    }
                }
            }
            if occupied
                || context.flows.iter().enumerate().any(|(other, flow)| {
                    other != index && context.corridors(index, &points, other, &flow.points, true)
                })
            {
                continue;
            }
            probes += 1;
            let position = label_point(context, index, &points);
            let mut plates = context.labels.clone();
            for (i, plate) in plates.iter_mut().enumerate() {
                if i == label_index {
                    plate.label.position = position;
                    plate.label.bounds.x = position.x - plate.label.bounds.width / 2.0;
                    plate.label.bounds.y = position.y - 11.0;
                } else {
                    plate.label.pinned = true;
                }
            }
            let mut routes = routes(context);
            routes[index].points = points.to_vec();
            let candidate =
                place(context, &plates, &routes, &components, false).remove(label_index);
            if !context.clear_label(&candidate.label, label_index, &context.labels)
                || routes.iter().any(|route| {
                    route.relation_index != index as isize
                        && masks(&route.points, &candidate.label.bounds)
                })
                || route_distance(&points, &candidate.label.bounds) > 36.0
                || (extra_line_masks(&candidate, &points)
                    && !extra_line_masks(&context.labels[label_index], &current))
            {
                continue;
            }
            context.flows[index].points = points.to_vec();
            context.labels[label_index] = candidate;
            break;
        }
    }
}

fn wrap(context: &mut Context<'_>) {
    let obstacles: Vec<_> = context
        .nodes
        .iter()
        .map(|node| node.bounds)
        .chain(context.titles())
        .collect();
    let segments: Vec<_> = context
        .flows
        .iter()
        .enumerate()
        .flat_map(|(index, flow)| {
            flow.points
                .windows(2)
                .map(move |pair| (index, pair[0], pair[1]))
        })
        .collect();
    for index in 0..context.labels.len() {
        let label = context.labels[index].clone();
        if label.label.pinned {
            continue;
        }
        let rect = label.label.bounds;
        let inside = |rect: &Rect| {
            rect.x >= 0.0
                && rect.y >= 0.0
                && rect.x + rect.width <= context.view_box.width
                && rect.y + rect.height <= context.view_box.height - 74.0
        };
        if inside(&rect)
            && !node_overlap(context, &label)
            && !context
                .titles()
                .iter()
                .any(|title| geometry::rects_overlap(&rect, title, 0.0))
            && !context.labels.iter().enumerate().any(|(other, plate)| {
                other != index && geometry::rects_overlap(&rect, &plate.label.bounds, 0.0)
            })
            && !segments
                .iter()
                .any(|(owner, a, b)| *owner != label.index() && masks(&[*a, *b], &rect))
        {
            continue;
        }
        let raw = &context.flows[label.index()].raw;
        let value = string(raw, "label");
        let mut shapes = Vec::new();
        for split in crate::geometry::text::word_boundaries(value) {
            if split == 0 || split >= value.len() {
                continue;
            }
            let lines = vec![value[..split].to_owned(), value[split..].to_owned()];
            if lines
                .iter()
                .any(|line| line.chars().all(crate::geometry::text::is_js_whitespace))
            {
                continue;
            }
            let size = size(context.input, raw, &lines);
            if size.x < rect.width {
                shapes.push((lines, size));
            }
        }
        shapes.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
        shapes.truncate(8);
        let mut blockers: Vec<_> = obstacles.iter().copied().map(|rect| (rect, 2.0)).collect();
        blockers.extend(
            context
                .labels
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, plate)| (plate.label.bounds, 2.0)),
        );
        blockers.extend(segments.iter().map(|(_, a, b)| {
            (
                Rect {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                    width: (b.x - a.x).abs(),
                    height: (b.y - a.y).abs(),
                },
                4.0,
            )
        }));
        let mut probes = 0;
        let mut seen = Vec::new();
        let mut replacement = None;
        'search: for (lines, size) in shapes {
            for &(owner, a, b) in &segments {
                if owner != label.index() || (a.y - b.y).abs() > geometry::EPSILON {
                    continue;
                }
                let low = a.x.min(b.x) + 8.0;
                let high = a.x.max(b.x) - size.x - 8.0;
                if high < low {
                    continue;
                }
                let mut ys = [a.y - size.y - 4.0, a.y + 4.0];
                if rect.y > a.y {
                    ys.reverse();
                }
                for y in ys {
                    let mut positions = vec![
                        low.max(high.min(label.label.position.x - size.x / 2.0)),
                        low,
                        high,
                    ];
                    for (blocker, gap) in &blockers {
                        if y + size.y + gap <= blocker.y || blocker.y + blocker.height + gap <= y {
                            continue;
                        }
                        positions
                            .extend([blocker.x - gap - size.x, blocker.x + blocker.width + gap]);
                    }
                    let mut xs = Vec::new();
                    for x in positions {
                        if x >= low && x <= high {
                            unique(&mut xs, x);
                        }
                    }
                    xs.sort_by(|a, b| {
                        (a + size.x / 2.0 - label.label.position.x)
                            .abs()
                            .total_cmp(&(b + size.x / 2.0 - label.label.position.x).abs())
                    });
                    for x in xs {
                        let key = (x, y, size.x, size.y);
                        if seen.contains(&key) {
                            continue;
                        }
                        seen.push(key);
                        if probes >= 64 {
                            break 'search;
                        }
                        probes += 1;
                        let bounds = Rect {
                            x,
                            y,
                            width: size.x,
                            height: size.y,
                        };
                        if !inside(&bounds)
                            || obstacles
                                .iter()
                                .any(|other| geometry::rects_overlap(&bounds, other, 2.0))
                            || context.labels.iter().enumerate().any(|(other, plate)| {
                                other != index
                                    && geometry::rects_overlap(&bounds, &plate.label.bounds, 2.0)
                            })
                            || segments.iter().any(|(_, a, b)| masks(&[*a, *b], &bounds))
                        {
                            continue;
                        }
                        replacement = Some(Plate {
                            label: labels::Label {
                                bounds,
                                position: p(x + size.x / 2.0, y + 11.0),
                                ..label.label.clone()
                            },
                            lines: lines.clone(),
                        });
                        break 'search;
                    }
                }
            }
        }
        if let Some(replacement) = replacement {
            context.labels[index] = replacement;
        }
    }
}

pub(super) fn settle(context: &mut Context<'_>) {
    if context.legacy() {
        return;
    }
    if context.automatic_showcase {
        move_corridors(context);
        align_settled_ports(context);
        context.mark_crossovers();
        wrap(context);
    }
    fit_width(context);
    fit_height(context);
}
