// Source: routeVia/pathFor, automatic port helpers and the compiler's bounded route repairs.
use super::*;

pub(super) fn default_sides(
    from: &MeasuredNode,
    to: &MeasuredNode,
) -> (&'static str, &'static str) {
    if to.cx < from.cx {
        ("left", "right")
    } else if to.cx > from.cx {
        ("right", "left")
    } else if to.cy > from.cy {
        ("bottom", "top")
    } else {
        ("top", "bottom")
    }
}
/// Row of an authored horizontal channel; `via` points take precedence.
fn channel_y(raw: &Value, from: &MeasuredNode, to: &MeasuredNode) -> Option<f64> {
    if raw["via"].is_array() {
        return None;
    }
    match string(raw, "route") {
        "bottom-channel" => Some(number(
            raw,
            "channelY",
            (from.bounds.y + from.bounds.height).max(to.bounds.y + to.bounds.height) + 26.0,
        )),
        "top-channel" => Some(number(
            raw,
            "channelY",
            from.bounds.y.min(to.bounds.y) - 24.0,
        )),
        _ => None,
    }
}
/// Sides used where the author left fromSide/toSide open. A horizontal
/// channel is reached through the face toward it; a lateral face would put
/// the channel riser on the node border and the arrowhead along it.
pub(super) fn inferred_sides(
    raw: &Value,
    from: &MeasuredNode,
    to: &MeasuredNode,
) -> (&'static str, &'static str) {
    match channel_y(raw, from, to) {
        Some(y) => {
            let face = |node: &MeasuredNode| if y < node.cy { "top" } else { "bottom" };
            (face(from), face(to))
        }
        None => default_sides(from, to),
    }
}
pub(super) fn stroke_width(raw: &Value) -> f64 {
    number(
        raw,
        "width",
        if raw["variant"] == "emphasis" {
            1.8
        } else {
            1.4
        },
    )
}
pub(super) fn port_span(items: &[(bool, f64)]) -> f64 {
    32.0 + items.len().saturating_sub(1) as f64
        * 18.0_f64.min(
            items
                .iter()
                .map(|(incoming, width)| width * if *incoming { 7.0 } else { 1.0 })
                .fold(0.0, f64::max)
                .ceil()
                + 2.0,
        )
}
pub(super) fn outward(point: Point, side: &str, distance: f64) -> Point {
    match side {
        "left" => p(point.x - distance, point.y),
        "right" => p(point.x + distance, point.y),
        "top" => p(point.x, point.y - distance),
        _ => p(point.x, point.y + distance),
    }
}
fn anchor(node: &MeasuredNode, side: &str) -> Point {
    geometry::anchor(&node.bounds, side)
}

pub(super) fn rebalance(context: &mut Context<'_>) -> bool {
    if !context.automatic_showcase {
        return false;
    }
    let mut changed = false;
    for node_index in 0..context.nodes.len() {
        let node = &context.nodes[node_index];
        if has(&node.raw, &["height", "yOffset"]) {
            continue;
        }
        let incident: Vec<_> = context
            .flows
            .iter()
            .enumerate()
            .filter(|(_, flow)| flow.from == node_index || flow.to == node_index)
            .map(|(i, _)| i)
            .collect();
        if incident
            .iter()
            .any(|&index| geometry_pinned(&context.flows[index].raw))
        {
            continue;
        }
        let mut sides: [Vec<(usize, bool, f64)>; 4] =
            [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for index in incident {
            let flow = &context.flows[index];
            let incoming = flow.to == node_index;
            let side = if incoming {
                flow.sides.1.as_str()
            } else {
                flow.sides.0.as_str()
            };
            let side_index = match side {
                "left" => 0,
                "right" => 1,
                "top" => 2,
                _ => 3,
            };
            sides[side_index].push((index, incoming, stroke_width(&flow.raw)));
        }
        let mut choices = Vec::new();
        for side in 0..2 {
            let items = &sides[side];
            let span: Vec<_> = items
                .iter()
                .map(|(_, incoming, width)| (*incoming, *width))
                .collect();
            if !items.iter().any(|item| item.1) || port_span(&span) <= node.bounds.height {
                continue;
            }
            for &(index, incoming, width) in items {
                if incoming {
                    continue;
                }
                let flow = &context.flows[index];
                let target = &context.nodes[flow.to];
                let dy = target.cy - node.cy;
                if dy.abs() < (node.bounds.height + target.bounds.height) / 2.0 + 16.0 {
                    continue;
                }
                let alternate = if dy > 0.0 { "bottom" } else { "top" };
                let slot = if dy > 0.0 { 3 } else { 2 };
                let mut span: Vec<_> = sides[slot]
                    .iter()
                    .map(|(_, incoming, width)| (*incoming, *width))
                    .collect();
                span.push((false, width));
                if port_span(&span) > node.bounds.width {
                    continue;
                }
                let to_side = default_sides(node, target).1;
                if to_side != "left" && to_side != "right" {
                    continue;
                }
                let start = anchor(node, alternate);
                let end = anchor(target, to_side);
                let points = geometry::normalize_route_points(&[start, p(start.x, end.y), end]);
                if !sides_ok(&points, &(alternate.into(), to_side.into()))
                    || context.intrudes(index, &points)
                {
                    continue;
                }
                choices.push((index, alternate, length(&[start, end])));
            }
        }
        choices.sort_by(|a, b| a.2.total_cmp(&b.2));
        if let Some(&(index, alternate, _)) = choices.first() {
            context.flows[index].sides.0 = alternate.into();
            changed = true;
        }
    }
    changed
}

pub(super) fn spread_ports(context: &mut Context<'_>) {
    // Members retain flow index, source-end flag and counterpart node.
    type PortMember = (usize, bool, usize);
    type PortGroup = (usize, String, Vec<PortMember>);
    let mut groups: Vec<PortGroup> = Vec::new();
    for (index, flow) in context.flows.iter().enumerate() {
        let raw = &flow.raw;
        if (!string(raw, "route").is_empty() && raw["route"] != "auto")
            || raw.get("via").is_some()
            || has(raw, &["channelX", "channelY", "labelAt"])
        {
            continue;
        }
        for (node_index, side, counterpart, from) in [
            (flow.from, &flow.sides.0, flow.to, true),
            (flow.to, &flow.sides.1, flow.from, false),
        ] {
            if let Some(group) = groups
                .iter_mut()
                .find(|group| group.0 == node_index && group.1 == *side)
            {
                group.2.push((index, from, counterpart));
            } else {
                groups.push((node_index, side.clone(), vec![(index, from, counterpart)]));
            }
        }
    }
    for (node_index, side, mut items) in groups {
        if items.len() < 2 {
            continue;
        }
        let vertical = side == "left" || side == "right";
        items.sort_by(|a, b| {
            let coordinate = |item: &(usize, bool, usize)| {
                let node = &context.nodes[item.2];
                if vertical {
                    node.cy
                } else {
                    node.cx
                }
            };
            coordinate(a).total_cmp(&coordinate(b)).then_with(|| {
                let key = |index: usize| {
                    let raw = &context.flows[index].raw;
                    format!(
                        "{}\0{}\0{}\0{}",
                        string(raw, "id"),
                        string(raw, "from"),
                        string(raw, "to"),
                        string(raw, "label")
                    )
                };
                key(a.0).encode_utf16().cmp(key(b.0).encode_utf16())
            })
        });
        let node = &context.nodes[node_index];
        let extent = if vertical {
            node.bounds.height
        } else {
            node.bounds.width
        };
        let spacing = (if context.legacy() { 14.0_f64 } else { 18.0_f64 })
            .min((extent - 32.0).max(0.0) / (items.len() - 1) as f64);
        if spacing <= 0.0 {
            continue;
        }
        let count = items.len();
        for (order, (index, from, _)) in items.into_iter().enumerate() {
            let offset = (order as f64 - (count - 1) as f64 / 2.0) * spacing;
            let mut point = anchor(node, &side);
            if vertical {
                point.y += offset;
            } else {
                point.x += offset;
            }
            if from {
                context.flows[index].ports.from = Some(point);
            } else {
                context.flows[index].ports.to = Some(point);
            }
        }
    }
}

fn corner_clear(node: &MeasuredNode, side: &str, point: Point) -> bool {
    match side {
        "left" | "right" => {
            let inset = 16.0_f64.min(node.bounds.height / 2.0);
            point.y >= node.bounds.y + inset
                && point.y <= node.bounds.y + node.bounds.height - inset
        }
        "top" | "bottom" => {
            let inset = 16.0_f64.min(node.bounds.width / 2.0);
            point.x >= node.bounds.x + inset && point.x <= node.bounds.x + node.bounds.width - inset
        }
        _ => false,
    }
}

pub(super) fn alignment_candidates(
    context: &Context<'_>,
    index: usize,
    start: Point,
    end: Point,
) -> Vec<[Point; 2]> {
    let flow = &context.flows[index];
    let raw = &flow.raw;
    let sides = &flow.sides;
    if raw.get("via").is_some()
        || (!string(raw, "route").is_empty() && raw["route"] != "auto")
        || has(raw, &["channelX", "channelY", "labelAt"])
    {
        return Vec::new();
    }
    let horizontal =
        (sides.0 == "right" && sides.1 == "left") || (sides.0 == "left" && sides.1 == "right");
    let vertical =
        (sides.0 == "bottom" && sides.1 == "top") || (sides.0 == "top" && sides.1 == "bottom");
    if !horizontal && !vertical {
        return Vec::new();
    }
    let from_spread = flow.ports.from.is_some();
    let to_spread = flow.ports.to.is_some();
    if from_spread && to_spread {
        return Vec::new();
    }
    if !from_spread
        && !to_spread
        && ["fromSide", "toSide"]
            .iter()
            .any(|key| !string(raw, key).is_empty() && raw[*key] != "auto")
    {
        return Vec::new();
    }
    if (if horizontal {
        (start.y - end.y).abs()
    } else {
        (start.x - end.x).abs()
    }) >= 16.0
    {
        return Vec::new();
    }
    let end_to_start = if horizontal {
        [start, p(end.x, start.y)]
    } else {
        [start, p(start.x, end.y)]
    };
    let start_to_end = if horizontal {
        [p(start.x, end.y), end]
    } else {
        [p(end.x, start.y), end]
    };
    let candidates = if from_spread {
        vec![end_to_start]
    } else if to_spread {
        vec![start_to_end]
    } else {
        vec![end_to_start, start_to_end]
    };
    candidates
        .into_iter()
        .filter(|points| {
            corner_clear(&context.nodes[flow.from], &sides.0, points[0])
                && corner_clear(&context.nodes[flow.to], &sides.1, points[1])
                && sides_ok(points, sides)
        })
        .collect()
}

pub(super) fn bridges(
    start: Point,
    end: Point,
    sides: &(String, String),
    stub: f64,
) -> Vec<Vec<Point>> {
    let horizontal = matches!(sides.0.as_str(), "left" | "right")
        && matches!(sides.1.as_str(), "left" | "right");
    let vertical = matches!(sides.0.as_str(), "top" | "bottom")
        && matches!(sides.1.as_str(), "top" | "bottom");
    let gap = if (sides.0 == "right" && sides.1 == "left" && end.x > start.x)
        || (sides.0 == "left" && sides.1 == "right" && start.x > end.x)
    {
        Some((end.x - start.x).abs())
    } else if (sides.0 == "bottom" && sides.1 == "top" && end.y > start.y)
        || (sides.0 == "top" && sides.1 == "bottom" && start.y > end.y)
    {
        Some((end.y - start.y).abs())
    } else {
        None
    };
    let mut stubs = vec![stub];
    if let Some(gap) = gap {
        if gap - stub * 2.0 < 16.0 {
            let bounded = 8.0_f64.max(stub.min((gap - 16.0) / 2.0));
            if bounded != stub {
                stubs.push(bounded);
            }
        }
    }
    let mut candidates = Vec::new();
    for stub in stubs {
        let a = outward(start, &sides.0, stub);
        let b = outward(end, &sides.1, stub);
        if vertical && (start.x - end.x).abs() < 16.0 {
            for x in [start.x.max(end.x) + 16.0, start.x.min(end.x) - 16.0] {
                candidates.push(vec![start, a, p(x, a.y), p(x, b.y), b, end]);
            }
        }
        if horizontal && (start.y - end.y).abs() < 16.0 {
            for y in [start.y.max(end.y) + 16.0, start.y.min(end.y) - 16.0] {
                candidates.push(vec![start, a, p(a.x, y), p(b.x, y), b, end]);
            }
        }
    }
    candidates
        .into_iter()
        .map(|points| geometry::normalize_route_points(&points))
        .filter(|points| sides_ok(points, sides) && rhythm(points))
        .collect()
}

fn route_via(context: &Context<'_>, index: usize, start: Point, end: Point) -> Vec<Point> {
    let flow = &context.flows[index];
    let raw = &flow.raw;
    if raw["via"].is_array() {
        return list(&raw["via"]).iter().map(point).collect();
    }
    if let Some(y) = channel_y(raw, &context.nodes[flow.from], &context.nodes[flow.to]) {
        return vec![p(start.x, y), p(end.x, y)];
    }
    match string(raw, "route") {
        "straight" => Vec::new(),
        "vertical-channel" => {
            let x = number(
                raw,
                "channelX",
                start.x + if end.x > start.x { 44.0 } else { -44.0 },
            );
            vec![p(x, start.y), p(x, end.y)]
        }
        _ => {
            if !context.legacy() {
                let from_vertical = matches!(flow.sides.0.as_str(), "top" | "bottom");
                let to_vertical = matches!(flow.sides.1.as_str(), "top" | "bottom");
                let across = if from_vertical && to_vertical {
                    (start.x - end.x).abs()
                } else {
                    (start.y - end.y).abs()
                };
                if context.automatic_showcase && across > geometry::EPSILON && across < 16.0 {
                    if let Some(points) =
                        bridges(start, end, &flow.sides, 24.0)
                            .into_iter()
                            .find(|points| {
                                !context.nodes.iter().enumerate().any(|(node_index, node)| {
                                    node_index != flow.from
                                        && node_index != flow.to
                                        && points.windows(2).any(|pair| {
                                            geometry::segment_intersects_rect(
                                                pair[0],
                                                pair[1],
                                                &node.bounds,
                                                2.0,
                                            )
                                        })
                                })
                            })
                    {
                        return points[1..points.len() - 1].to_vec();
                    }
                }
                if from_vertical != to_vertical {
                    let a = outward(start, &flow.sides.0, 24.0);
                    let b = outward(end, &flow.sides.1, 24.0);
                    for via in [
                        vec![if from_vertical {
                            p(start.x, end.y)
                        } else {
                            p(end.x, start.y)
                        }],
                        vec![a, p(a.x, b.y), b],
                        vec![a, p(b.x, a.y), b],
                    ] {
                        let mut points = vec![start];
                        points.extend_from_slice(&via);
                        points.push(end);
                        let points = geometry::normalize_route_points(&points);
                        if sides_ok(&points, &flow.sides) && !context.intrudes(index, &points) {
                            return via;
                        }
                    }
                }
                if context.automatic_showcase && from_vertical && to_vertical {
                    if (start.x - end.x).abs() < geometry::EPSILON {
                        return Vec::new();
                    }
                    let y = (start.y + end.y) / 2.0;
                    return vec![p(start.x, y), p(end.x, y)];
                }
            }
            if (start.y - end.y).abs() < 4.0 {
                return Vec::new();
            }
            let middle = start.x + (end.x - start.x) / 2.0;
            let mut turn = middle;
            if !context.legacy() {
                let mut gaps: Vec<_> = (0..list(&context.input.document["stages"]).len() - 1)
                    .map(|index| (measure::stage_x(index) + measure::stage_x(index + 1)) / 2.0)
                    .filter(|x| *x > start.x.min(end.x) && *x < start.x.max(end.x))
                    .collect();
                gaps.sort_by(|a, b| (a - middle).abs().total_cmp(&(b - middle).abs()));
                for x in std::iter::once(middle).chain(gaps) {
                    let points = [start, p(x, start.y), p(x, end.y), end];
                    if !context.nodes.iter().enumerate().any(|(node_index, node)| {
                        node_index != flow.from
                            && node_index != flow.to
                            && points.windows(2).any(|pair| {
                                geometry::segment_intersects_rect(
                                    pair[0],
                                    pair[1],
                                    &node.bounds,
                                    2.0,
                                )
                            })
                    }) {
                        turn = x;
                        break;
                    }
                }
            }
            vec![p(turn, start.y), p(turn, end.y)]
        }
    }
}

pub(super) fn initial_routes(context: &mut Context<'_>) {
    for index in 0..context.flows.len() {
        let flow = &context.flows[index];
        let mut start = flow
            .ports
            .from
            .unwrap_or_else(|| anchor(&context.nodes[flow.from], &flow.sides.0));
        let mut end = flow
            .ports
            .to
            .unwrap_or_else(|| anchor(&context.nodes[flow.to], &flow.sides.1));
        if context.automatic_showcase && !has(&flow.raw, &["labelDx", "labelDy", "labelSegment"]) {
            for candidate in alignment_candidates(context, index, start, end) {
                if context.nodes.iter().enumerate().any(|(node_index, node)| {
                    node_index != flow.from
                        && node_index != flow.to
                        && geometry::segment_intersects_rect(
                            candidate[0],
                            candidate[1],
                            &node.bounds,
                            2.0,
                        )
                }) {
                    continue;
                }
                let mut occupied = false;
                for (node_index, side, point) in [
                    (flow.from, &flow.sides.0, candidate[0]),
                    (flow.to, &flow.sides.1, candidate[1]),
                ] {
                    for (other_index, other) in context.flows.iter().enumerate() {
                        if other_index == index {
                            continue;
                        }
                        for (endpoint, other_side, port) in [
                            (other.from, &other.sides.0, other.ports.from),
                            (other.to, &other.sides.1, other.ports.to),
                        ] {
                            if endpoint == node_index && other_side == side {
                                let at = port
                                    .unwrap_or_else(|| anchor(&context.nodes[node_index], side));
                                if (point.x - at.x).hypot(point.y - at.y) < 18.0 - geometry::EPSILON
                                {
                                    occupied = true;
                                }
                            }
                        }
                    }
                }
                if !occupied {
                    start = candidate[0];
                    end = candidate[1];
                    break;
                }
            }
        }
        let mut raw_points = vec![start];
        raw_points.extend(route_via(context, index, start, end));
        raw_points.push(end);
        let mut points: Vec<Point> = Vec::new();
        for point in raw_points {
            if points.last().is_none_or(|previous| {
                (point.x - previous.x).abs() > geometry::EPSILON
                    || (point.y - previous.y).abs() > geometry::EPSILON
            }) {
                points.push(point);
            }
        }
        if points.len() < 2 {
            points.push(end);
        }
        context.flows[index].points = points;
    }
}

fn endpoint_stubs(context: &Context<'_>, index: usize) -> Vec<[Point; 2]> {
    let flow = &context.flows[index];
    let first = flow.points[0];
    let last = *flow.points.last().unwrap();
    vec![
        [first, outward(first, &flow.sides.0, 24.0)],
        [last, outward(last, &flow.sides.1, 24.0)],
    ]
}
fn shares_corridor(
    context: &Context<'_>,
    index: usize,
    points: &[Point],
    deferred: &[usize],
) -> bool {
    context.flows.iter().enumerate().any(|(other, flow)| {
        other != index && !related(&context.flows[index], flow) && {
            if deferred.contains(&other) {
                endpoint_stubs(context, other)
                    .iter()
                    .any(|stub| context.corridors(index, points, other, stub, false))
            } else {
                context.corridors(index, points, other, &flow.points, false)
            }
        }
    })
}
fn grid_detour(
    context: &Context<'_>,
    index: usize,
    allow_crossings: bool,
    deferred: &[usize],
    frames: &[composition::Frame],
) -> Option<Vec<Point>> {
    use crate::geometry::route_quality::{
        shortest_orthogonal_grid_route, GridRouteOptions, Segment,
    };
    let flow = &context.flows[index];
    let endpoints = [flow.points[0], *flow.points.last().unwrap()];
    let obstacles: Vec<_> = context
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
        .collect();
    let mut avoided = Vec::new();
    for (other, other_flow) in context.flows.iter().enumerate() {
        if other == index || related(flow, other_flow) {
            continue;
        }
        if deferred.contains(&other) {
            avoided.extend(endpoint_stubs(context, other).iter().map(|pair| Segment {
                start: pair[0],
                end: pair[1],
            }));
        } else {
            avoided.extend(other_flow.points.windows(2).map(|pair| Segment {
                start: pair[0],
                end: pair[1],
            }));
        }
    }
    let borders: Vec<_> = frames
        .iter()
        .flat_map(composition::frame_border_segments)
        .filter(|segment| segment.side != "bottom")
        .map(|segment| Segment {
            start: segment.start,
            end: segment.end,
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
    options.allow_avoided_crossings = allow_crossings;
    options.minimum_segment_px = 16.0;
    options.bend_penalty_px = 48.0;
    let mut detour = shortest_orthogonal_grid_route(&options).route;
    if detour
        .as_ref()
        .is_some_and(|route| context.border_runs(&route.points, frames, true))
    {
        options.border_segments = &borders;
        detour = shortest_orthogonal_grid_route(&options).route;
    }
    let points = geometry::normalize_route_points(&detour?.points);
    rhythm(&points).then_some(points)
}
fn compact_corridors(
    context: &Context<'_>,
    index: usize,
    frames: &[composition::Frame],
) -> Vec<Vec<Point>> {
    let flow = &context.flows[index];
    let axis = match (flow.sides.0.as_str(), flow.sides.1.as_str()) {
        ("right", "left") | ("left", "right") => 0,
        ("bottom", "top") | ("top", "bottom") => 1,
        _ => return Vec::new(),
    };
    let (start, end) = (flow.points[0], *flow.points.last().unwrap());
    let low = component(start, axis).min(component(end, axis)) + 24.0;
    let high = component(start, axis).max(component(end, axis)) - 24.0;
    if low > high {
        return Vec::new();
    }
    let middle = (low + high) / 2.0;
    let mut coordinates = vec![low, middle, high];
    for (other, route) in context.flows.iter().enumerate() {
        if other == index {
            continue;
        }
        for pair in route.points.windows(2) {
            if component(pair[0], axis) == component(pair[1], axis) {
                unique(&mut coordinates, component(pair[0], axis) - 8.0);
                unique(&mut coordinates, component(pair[0], axis) + 8.0);
            }
        }
    }
    coordinates.retain(|coordinate| *coordinate >= low && *coordinate <= high);
    coordinates.sort_by(|a, b| (a - middle).abs().total_cmp(&(b - middle).abs()));
    coordinates
        .into_iter()
        .take(16)
        .filter_map(|coordinate| {
            let (mut a, mut b) = (start, end);
            set_component(&mut a, axis, coordinate);
            set_component(&mut b, axis, coordinate);
            let points = geometry::normalize_route_points(&[start, a, b, end]);
            (sides_ok(&points, &flow.sides)
                && !context.intrudes(index, &points)
                && rhythm(&points)
                && !context.border_runs(&points, frames, true)
                && context
                    .crossing_flows(index, &points)
                    .iter()
                    .all(|&other| context.supports_crossover(other))
                && context.flows.iter().enumerate().all(|(other, route)| {
                    other == index || !context.corridors(index, &points, other, &route.points, true)
                }))
            .then_some(points)
        })
        .collect()
}

pub(super) fn repair_routes(context: &mut Context<'_>) {
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
        .fold(context.view_box.height, f64::max)
        + 36.0;
    let frames: Vec<_> = context
        .frames
        .iter()
        .cloned()
        .map(|mut frame| {
            frame.bounds.height = bottom - 46.0;
            frame
        })
        .collect();
    for _ in 0..context.flows.len() {
        let mut repaired = false;
        for index in 0..context.flows.len() {
            if !context.supports_crossover(index) {
                continue;
            }
            let current = context.flows[index].points.clone();
            if !context.intrudes(index, &current) && !shares_corridor(context, index, &current, &[])
            {
                continue;
            }
            let candidates_for = |context: &Context<'_>, deferred: &[usize]| {
                [
                    grid_detour(context, index, false, deferred, &frames),
                    grid_detour(context, index, true, deferred, &frames),
                ]
                .into_iter()
                .flatten()
                .filter(|points| {
                    sides_ok(points, &context.flows[index].sides)
                        && !context.intrudes(index, points)
                        && !shares_corridor(context, index, points, deferred)
                        && context
                            .crossing_flows(index, points)
                            .iter()
                            .all(|&other| context.supports_crossover(other))
                })
                .collect::<Vec<_>>()
            };
            let mut candidates = candidates_for(context, &[]);
            if candidates.is_empty() {
                let deferred: Vec<_> = context
                    .flows
                    .iter()
                    .enumerate()
                    .filter(|(other, flow)| {
                        *other != index
                            && !related(&context.flows[index], flow)
                            && context.supports_crossover(*other)
                            && context.intrudes(*other, &flow.points)
                    })
                    .map(|(index, _)| index)
                    .collect();
                if !deferred.is_empty() {
                    candidates = candidates_for(context, &deferred);
                }
            }
            candidates.extend(compact_corridors(context, index, &frames));
            candidates.sort_by(|a, b| context.cost(index, a).total_cmp(&context.cost(index, b)));
            if let Some(chosen) = candidates.into_iter().next() {
                if chosen != current {
                    context.flows[index].points = chosen;
                    repaired = true;
                }
            }
        }
        if !repaired {
            break;
        }
    }
    for _ in 0..context.flows.len() {
        let mut options = Vec::new();
        for index in 0..context.flows.len() {
            if !context.supports_crossover(index)
                || context
                    .crossing_flows(index, &context.flows[index].points)
                    .is_empty()
            {
                continue;
            }
            let Some(points) = grid_detour(context, index, false, &[], &frames) else {
                continue;
            };
            let old = &context.flows[index].points;
            let old_cost = context.cost(index, old);
            let new_cost = length(&points) + points.len().saturating_sub(2) as f64 * 48.0;
            let resolvable = context
                .crossing_flows(index, old)
                .iter()
                .all(|&other| context.supports_crossover(other))
                && !shares_corridor(context, index, old, &[])
                && !context.intrudes(index, old);
            if resolvable && new_cost >= old_cost {
                continue;
            }
            options.push((index, points, new_cost - old_cost));
        }
        options.sort_by(|a, b| a.2.total_cmp(&b.2));
        let before = context
            .flows
            .iter()
            .enumerate()
            .filter(|(index, flow)| !context.crossing_flows(*index, &flow.points).is_empty())
            .count();
        let mut accepted = false;
        for (index, points, _) in options {
            let previous = std::mem::replace(&mut context.flows[index].points, points);
            let after = context
                .flows
                .iter()
                .enumerate()
                .filter(|(index, flow)| !context.crossing_flows(*index, &flow.points).is_empty())
                .count();
            if after < before {
                accepted = true;
                break;
            }
            context.flows[index].points = previous;
        }
        if !accepted {
            break;
        }
    }
    reorder_ports(context);
}

fn reorder_ports(context: &mut Context<'_>) {
    type PortEndpoint = (usize, bool);
    type PortGroup = (usize, String, Vec<PortEndpoint>);
    let mut groups: Vec<PortGroup> = Vec::new();
    for (index, flow) in context.flows.iter().enumerate() {
        for (node, side, from) in [
            (flow.from, &flow.sides.0, true),
            (flow.to, &flow.sides.1, false),
        ] {
            if let Some(group) = groups
                .iter_mut()
                .find(|group| group.0 == node && group.1 == *side)
            {
                group.2.push((index, from));
            } else {
                groups.push((node, side.clone(), vec![(index, from)]));
            }
        }
    }
    let mut probes = 0;
    for (_, side, group) in groups {
        if probes >= 16 {
            break;
        }
        if group.len() < 2
            || group.len() > 8
            || group.iter().any(|&(index, from)| {
                !context.supports_crossover(index)
                    || has(
                        &context.flows[index].raw,
                        &[
                            "fromSide",
                            "toSide",
                            "labelAt",
                            "labelDx",
                            "labelDy",
                            "labelSegment",
                        ],
                    )
                    || if from {
                        context.flows[index].ports.from.is_none()
                    } else {
                        context.flows[index].ports.to.is_none()
                    }
            })
        {
            continue;
        }
        let axis = if side == "left" || side == "right" {
            1
        } else {
            0
        };
        let mut items = Vec::new();
        for (index, from) in group {
            let current = geometry::normalize_route_points(&context.flows[index].points);
            let mut outward = current.clone();
            if !from {
                outward.reverse();
            }
            let coordinate = component(outward[0], axis);
            let turn = outward
                .iter()
                .find(|point| (component(**point, axis) - coordinate).abs() > geometry::EPSILON)
                .map_or(coordinate, |point| component(*point, axis));
            items.push((index, from, current, outward, coordinate, turn));
        }
        let mut slots: Vec<_> = items.iter().map(|item| item.4).collect();
        slots.sort_by(f64::total_cmp);
        let mut ordered = items.clone();
        ordered.sort_by(|a, b| a.5.total_cmp(&b.5).then_with(|| a.4.total_cmp(&b.4)));
        if ordered
            .iter()
            .enumerate()
            .all(|(index, item)| item.4 == slots[index])
        {
            continue;
        }
        probes += 1;
        let mut proposals: Vec<Option<Vec<Point>>> = vec![None; context.flows.len()];
        let mut straightened = false;
        for (index, item) in ordered.iter().enumerate() {
            let mut points = item.3.clone();
            set_component(&mut points[0], axis, slots[index]);
            if points.len() > 2 {
                set_component(&mut points[1], axis, slots[index]);
            }
            if !item.1 {
                points.reverse();
            }
            let direct = [points[0], *points.last().unwrap()];
            if sides_ok(&direct, &context.flows[item.0].sides)
                && (direct[0].x == direct[1].x || direct[0].y == direct[1].y)
            {
                points = direct.to_vec();
            }
            let points = geometry::normalize_route_points(&points);
            if points.len() == 2 && item.2.len() > 2 {
                straightened = true;
            }
            proposals[item.0] = Some(points);
        }
        if !straightened
            || items.iter().any(|item| {
                let index = item.0;
                let points = proposals[index].as_ref().unwrap();
                let current = &item.2;
                points.len() > current.len()
                    || length(points) > length(current) + geometry::EPSILON
                    || folds_back(points)
                    || !sides_ok(points, &context.flows[index].sides)
                    || context.intrudes(index, points)
                    || !rhythm(points)
                    || context.border_runs(points, &context.frames, false)
                    || points.iter().any(|point| {
                        point.x < 0.0
                            || point.y < 0.0
                            || point.x > context.view_box.width
                            || point.y > context.view_box.height - 74.0
                    })
                    || context.flows.iter().enumerate().any(|(other, flow)| {
                        other != index && {
                            let other_points = proposals[other].as_ref().unwrap_or(&flow.points);
                            intersections(points, other_points)
                                > intersections(current, &flow.points)
                                || context.corridors(index, points, other, other_points, true)
                        }
                    })
            })
        {
            continue;
        }
        for (index, from, _, _, _, _) in items {
            let points = proposals[index].take().unwrap();
            let endpoint = if from {
                points[0]
            } else {
                *points.last().unwrap()
            };
            context.flows[index].points = points;
            if from {
                context.flows[index].ports.from = Some(endpoint);
            } else {
                context.flows[index].ports.to = Some(endpoint);
            }
        }
    }
}
