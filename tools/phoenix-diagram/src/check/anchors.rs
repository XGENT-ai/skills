// Independent SVG endpoint checks. Route samples come from parsed d commands;
// authored constraints come from native metadata, never SVG route annotations.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{
    self,
    composition::{collect_endpoint_side_issues, Frame, RoutedRelation},
    EPSILON,
};
use crate::input::DiagramType;
use crate::scene::{Point, Rect};
use scraper::{ElementRef, Selector};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static selector")
}

fn number(element: ElementRef<'_>, name: &str) -> f64 {
    element
        .value()
        .attr(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(f64::NAN)
}

fn transformed(element: ElementRef<'_>) -> bool {
    std::iter::once(element)
        .chain(element.ancestors().filter_map(ElementRef::wrap))
        .any(|element| {
            element
                .value()
                .attr("transform")
                .is_some_and(|value| !value.trim().is_empty())
        })
}

fn same(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= EPSILON && (a.y - b.y).abs() <= EPSILON
}

fn between(value: f64, start: f64, end: f64) -> bool {
    value >= start - EPSILON && value <= end + EPSILON
}

// Native node ports use the straight sides of the visible rounded body.
// A point on its enclosing rectangle's cut-away corner is not a body anchor.
fn body_side(point: Point, bounds: Rect, radius: (f64, f64)) -> Option<&'static str> {
    if !geometry::finite_point(point)
        || !geometry::finite_rect(&bounds)
        || bounds.width <= 0.0
        || bounds.height <= 0.0
    {
        return None;
    }
    let rx = radius.0.max(0.0).min(bounds.width / 2.0);
    let ry = radius.1.max(0.0).min(bounds.height / 2.0);
    if between(point.y, bounds.y + ry, bounds.y + bounds.height - ry) {
        if (point.x - bounds.x).abs() <= EPSILON {
            return Some("left");
        }
        if (point.x - bounds.x - bounds.width).abs() <= EPSILON {
            return Some("right");
        }
    }
    if between(point.x, bounds.x + rx, bounds.x + bounds.width - rx) {
        if (point.y - bounds.y).abs() <= EPSILON {
            return Some("top");
        }
        if (point.y - bounds.y - bounds.height).abs() <= EPSILON {
            return Some("bottom");
        }
    }
    None
}

fn contains(outer: Rect, inner: Rect) -> bool {
    geometry::finite_rect(&outer)
        && geometry::finite_rect(&inner)
        && outer.width > 0.0
        && outer.height > 0.0
        && between(inner.x, outer.x, outer.x + outer.width)
        && between(inner.x + inner.width, outer.x, outer.x + outer.width)
        && between(inner.y, outer.y, outer.y + outer.height)
        && between(inner.y + inner.height, outer.y, outer.y + outer.height)
}

fn error(
    kind: DiagramType,
    index: usize,
    raw: &Value,
    code: &str,
    message: &str,
    evidence: Value,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(
        Some(kind.as_str()),
        code,
        &format!("/svg/relationships/{index}"),
        message,
    );
    diagnostic.stage = "artifact".into();
    diagnostic.subject.identity = raw["id"].as_str().map(str::to_owned);
    diagnostic.evidence = evidence;
    diagnostic.supported_fixes = vec!["Restore the actual SVG geometry from the authored input; preserve its endpoints and explicit route pins".into()];
    diagnostic
}

struct ExitGroup<'a> {
    sources: Vec<&'a str>,
    edges: Vec<usize>,
}

// Mirror only the frozen v3 semantic rule that permits an endpoint to leave a
// composite frame rather than its original from-state. Junction/presentation
// attributes do not authorize an exception. Nonconsecutive/overlapping spans
// remain ordinary node-boundary endpoints on a shared bus.
fn frame_groups<'a>(relations: &'a [Value], main: &[&'a str]) -> Vec<ExitGroup<'a>> {
    let mut targets: Vec<(&str, Vec<usize>)> = Vec::new();
    for (index, raw) in relations.iter().enumerate() {
        let (Some(from), Some(to)) = (raw["from"].as_str(), raw["to"].as_str()) else {
            continue;
        };
        if !main.contains(&from) || main.contains(&to) {
            continue;
        }
        if let Some((_, edges)) = targets.iter_mut().find(|(target, _)| *target == to) {
            edges.push(index);
        } else {
            targets.push((to, vec![index]));
        }
    }
    let mut groups: Vec<ExitGroup<'a>> = Vec::new();
    for (_, edges) in targets {
        let mut buckets: Vec<(String, Vec<usize>)> = Vec::new();
        for index in edges {
            let raw = &relations[index];
            let signature = format!(
                "{}\0{}\0{}",
                raw["label"].as_str().unwrap_or(""),
                raw["note"].as_str().unwrap_or(""),
                raw["variant"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("default")
            );
            if let Some((_, edges)) = buckets.iter_mut().find(|(key, _)| *key == signature) {
                edges.push(index);
            } else {
                buckets.push((signature, vec![index]));
            }
        }
        let eligible: Vec<_> = buckets
            .into_iter()
            .filter(|(_, edges)| {
                edges
                    .iter()
                    .filter_map(|&index| relations[index]["from"].as_str())
                    .collect::<HashSet<_>>()
                    .len()
                    >= 2
            })
            .collect();
        if eligible.len() != 1 {
            continue;
        }
        let edges = eligible.into_iter().next().unwrap().1;
        let mut sources: Vec<_> = edges
            .iter()
            .filter_map(|&index| relations[index]["from"].as_str())
            .collect();
        sources.sort_by_key(|source| main.iter().position(|id| id == source));
        sources.dedup();
        if let Some(group) = groups.iter_mut().find(|group| group.sources == sources) {
            group.edges.extend(edges);
        } else {
            groups.push(ExitGroup { sources, edges });
        }
    }
    let mut spans = Vec::new();
    groups
        .into_iter()
        .filter(|group| {
            let lo = main
                .iter()
                .position(|id| id == group.sources.first().unwrap())
                .unwrap();
            let hi = main
                .iter()
                .position(|id| id == group.sources.last().unwrap())
                .unwrap();
            if hi - lo + 1 != group.sources.len() || spans.iter().any(|&(a, b)| a <= hi && lo <= b)
            {
                return false;
            }
            spans.push((lo, hi));
            true
        })
        .collect()
}

fn sequence_anchor(
    svg: ElementRef<'_>,
    id: &str,
    center: f64,
    point: Point,
    expected_x: f64,
    y: f64,
) -> bool {
    let id = format!("lifeline-{id}");
    let lines: Vec<_> = svg
        .select(&selector(
            "line[data-graph-role='reading-guide'][data-composition-frame-kind='lifeline']",
        ))
        .filter(|line| line.value().attr("data-composition-frame-id") == Some(id.as_str()))
        .collect();
    if lines.len() != 1 || transformed(lines[0]) {
        return false;
    }
    let line = lines[0];
    let (x1, x2, y1, y2) = (
        number(line, "x1"),
        number(line, "x2"),
        number(line, "y1"),
        number(line, "y2"),
    );
    [x1, x2, y1, y2, y, expected_x]
        .iter()
        .all(|value| value.is_finite())
        && (x1 - center).abs() <= EPSILON
        && (x2 - center).abs() <= EPSILON
        && y2 > y1
        && between(y, y1, y2)
        && same(point, Point { x: expected_x, y })
}

fn point(value: &Value) -> Option<Point> {
    let values = value.as_array()?;
    let point = Point {
        x: values.first()?.as_f64()?,
        y: values.get(1)?.as_f64()?,
    };
    geometry::finite_point(point).then_some(point)
}

fn explicit_side<'a>(raw: &'a Value, field: &str) -> Option<&'a str> {
    raw[field]
        .as_str()
        .filter(|side| matches!(*side, "left" | "right" | "top" | "bottom"))
}

fn sampled_path(
    kind: DiagramType,
    version: u64,
    raw: &Value,
    points: &[Point],
) -> Option<crate::check::path::PathAnalysis> {
    let path = match kind {
        DiagramType::Architecture | DiagramType::Erd | DiagramType::Class => {
            geometry::rounded_path(points, 8.0)
        }
        DiagramType::Workflow if version == 2 => {
            let mut radius: f64 = 8.0;
            for (index, pair) in points.windows(2).enumerate() {
                let corners = usize::from(index > 0) + usize::from(index + 2 < points.len());
                if corners > 0 {
                    radius = radius.min(
                        (((pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y) - 8.0)
                            / corners as f64)
                            .max(0.0),
                    );
                }
            }
            geometry::rounded_path(points, radius)
        }
        DiagramType::Tree => geometry::rounded_path(points, 10.0),
        DiagramType::Lifecycle => geometry::rounded_path(
            points,
            if version == 1 {
                raw["cornerRadius"].as_f64().unwrap_or(10.0)
            } else {
                12.0
            },
        ),
        _ => geometry::polyline_path(points),
    };
    crate::check::path::analyze_svg_path(&path).ok()
}

fn matches_pins(
    kind: DiagramType,
    version: u64,
    raw: &Value,
    parts: &[&RoutedRelation],
    points: Vec<Point>,
) -> bool {
    // Dataflow removes repeated neighboring coordinates before serialization.
    let mut points = points;
    if kind == DiagramType::Dataflow {
        points.dedup_by(|a, b| same(*a, *b));
    }
    let Some(expected) = sampled_path(kind, version, raw, &points) else {
        return false;
    };
    parts.len() == expected.subpaths.len()
        && parts.iter().zip(expected.subpaths).all(|(part, expected)| {
            part.points.len() == expected.points.len()
                && part
                    .points
                    .iter()
                    .zip(expected.points)
                    .all(|(&a, b)| same(a, b))
        })
}

// Only fields actually consumed by these typed route presets are pins. The
// source ignores channel coordinates on other presets; do not invent a gate.
fn channel_points(
    kind: DiagramType,
    version: u64,
    raw: &Value,
    start: Point,
    end: Point,
    from_side: Option<&str>,
    to_side: Option<&str>,
) -> Option<Vec<Point>> {
    if raw["via"].is_array() {
        return None;
    }
    let route = raw["route"].as_str().unwrap_or("auto");
    let (x, y) = (raw["channelX"].as_f64(), raw["channelY"].as_f64());
    let vertical = match kind {
        DiagramType::Workflow => matches!(route, "outside-right" | "return-left"),
        DiagramType::Dataflow => route == "vertical-channel",
        DiagramType::Lifecycle if version == 1 => matches!(route, "right-channel" | "left-channel"),
        _ => false,
    };
    let horizontal = match kind {
        DiagramType::Workflow => matches!(route, "drop" | "bottom-channel" | "up-channel"),
        DiagramType::Dataflow => matches!(route, "bottom-channel" | "top-channel"),
        DiagramType::Lifecycle if version == 1 => {
            matches!(route, "drop" | "bottom-channel" | "top-channel")
        }
        _ => false,
    };
    let via = if vertical {
        let x = x?;
        vec![Point { x, y: start.y }, Point { x, y: end.y }]
    } else if horizontal {
        let y = y?;
        vec![Point { x: start.x, y }, Point { x: end.x, y }]
    } else if kind == DiagramType::Workflow && version == 2 && matches!(route, "" | "auto") {
        match (x, y) {
            (Some(x), Some(y)) => vec![
                Point { x, y: start.y },
                Point { x, y },
                Point { x: end.x, y },
            ],
            (Some(x), None) => vec![Point { x, y: start.y }, Point { x, y: end.y }],
            (None, Some(y)) => vec![Point { x: start.x, y }, Point { x: end.x, y }],
            _ => return None,
        }
    } else if kind == DiagramType::Lifecycle
        && version == 1
        && matches!(route, "" | "auto")
        && start.x != end.x
        && start.y != end.y
    {
        let from_vertical = matches!(from_side?, "top" | "bottom");
        let to_vertical = matches!(to_side?, "top" | "bottom");
        if from_vertical != to_vertical {
            return None;
        }
        if from_vertical {
            let y = y?;
            vec![Point { x: start.x, y }, Point { x: end.x, y }]
        } else {
            let x = x?;
            vec![Point { x, y: start.y }, Point { x, y: end.y }]
        }
    } else {
        return None;
    };
    Some(
        std::iter::once(start)
            .chain(via)
            .chain(std::iter::once(end))
            .collect(),
    )
}

/// Check resolved endpoints against actual SVG bodies and authored route pins.
pub fn check_anchors(
    kind: DiagramType,
    svg: ElementRef<'_>,
    nodes: &[(String, Rect)],
    routes: &[RoutedRelation],
    frames: &[Frame],
    metadata: &Value,
) -> Diagnostics {
    let mut diagnostics = Vec::new();
    let Some(relations) = metadata
        .pointer("/graph/relations")
        .and_then(Value::as_array)
    else {
        return diagnostics;
    };
    let version = metadata["schemaVersion"].as_u64().unwrap_or(0);
    let bounds = |id: &str| {
        nodes
            .iter()
            .find(|(node, _)| node == id)
            .map(|(_, bounds)| *bounds)
    };
    let radii: HashMap<_, _> = svg
        .select(&selector("g[data-node-id]"))
        .filter_map(|group| {
            let id = group.value().attr("data-node-id")?;
            let body = group
                .children()
                .filter_map(ElementRef::wrap)
                .find(|element| {
                    element.value().name() == "rect"
                        && element.value().attr("data-graph-role") == Some("node-body")
                        && !element
                            .value()
                            .has_class("c-mask", scraper::CaseSensitivity::CaseSensitive)
                })?;
            let rx = body.value().attr("rx").map(|_| number(body, "rx"));
            let ry = body.value().attr("ry").map(|_| number(body, "ry"));
            Some((
                id.to_owned(),
                (rx.or(ry).unwrap_or(0.0), ry.or(rx).unwrap_or(0.0)),
            ))
        })
        .collect();
    let mut frame_exits = HashMap::new();
    if kind == DiagramType::Lifecycle && version == 3 {
        let main: Vec<_> = metadata
            .pointer("/facts/main_path")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        for group in frame_groups(relations, &main) {
            let candidates: Vec<_> = frames
                .iter()
                .filter(|frame| {
                    frame.kind == "lifecycle-frame"
                        && frame.line.is_none()
                        && group
                            .sources
                            .iter()
                            .all(|id| bounds(id).is_some_and(|node| contains(frame.bounds, node)))
                        && !svg
                            .select(&selector("[data-graph-role='structural-frame']"))
                            .any(|element| {
                                element.value().attr("data-composition-frame-id")
                                    == Some(frame.id.as_str())
                                    && transformed(element)
                            })
                })
                .collect();
            let frame = (candidates.len() == 1).then(|| candidates[0]);
            for index in group.edges {
                frame_exits.insert(index, frame);
            }
        }
    }
    let mut checked = HashSet::new();
    for route in routes {
        let index = route.relation_index;
        if !checked.insert(index) {
            continue;
        }
        let Some(raw) = relations.get(index) else {
            continue;
        };
        let parts: Vec<_> = routes
            .iter()
            .filter(|part| part.relation_index == index)
            .collect();
        let source = parts.iter().find(|part| part.source_endpoint).copied();
        let target = parts.iter().find(|part| part.target_endpoint).copied();
        let (Some(source), Some(target)) = (source, target) else {
            continue;
        };
        let (Some(&start), Some(&end)) = (source.points.first(), target.points.last()) else {
            continue;
        };
        let (from, to) = (
            raw["from"].as_str().unwrap_or(""),
            raw["to"].as_str().unwrap_or(""),
        );
        let (Some(from_bounds), Some(to_bounds)) = (bounds(from), bounds(to)) else {
            continue;
        };
        if kind == DiagramType::Sequence {
            let (from_center, to_center) = (
                from_bounds.x + from_bounds.width / 2.0,
                to_bounds.x + to_bounds.width / 2.0,
            );
            let direction = if to_center > from_center { 1.0 } else { -1.0 };
            let y = raw["y"].as_f64().unwrap_or(f64::NAN);
            if parts.len() != 1
                || geometry::normalize_route_points(&source.points).len() != 2
                || !sequence_anchor(
                    svg,
                    from,
                    from_center,
                    start,
                    from_center + direction * 7.0,
                    y,
                )
                || !sequence_anchor(svg, to, to_center, end, to_center - direction * 7.0, y)
            {
                diagnostics.push(error(kind, index, raw, "artifact/sequence-anchor", "Actual message must connect participant lifelines at their 7px offsets and authored timeline y", json!({"from":from,"to":to,"start":start,"end":end,"authoredY":raw["y"],"fromBounds":from_bounds,"toBounds":to_bounds})));
            }
            continue;
        }
        // The class source reads an empty via as an omitted one.
        let via_pins = raw["via"]
            .as_array()
            .filter(|via| kind != DiagramType::Class || !via.is_empty());
        let has_route_geometry = via_pins.is_some()
            || raw["route"]
                .as_str()
                .is_some_and(|route| !matches!(route, "" | "auto"));
        let legacy_via = kind == DiagramType::Lifecycle && version == 1 && raw["via"].is_array();
        for (source_endpoint, id, node, part, at, field) in [
            (true, from, from_bounds, source, start, "fromSide"),
            (false, to, to_bounds, target, end, "toSide"),
        ] {
            let side = if source_endpoint && frame_exits.contains_key(&index) {
                let frame = frame_exits[&index];
                if frame.is_some_and(|frame| {
                    body_side(at, frame.bounds, (frame.radius, frame.radius)) == Some("bottom")
                        && !contains(frame.bounds, to_bounds)
                }) {
                    Some("bottom")
                } else {
                    diagnostics.push(error(kind, index, raw, "artifact/lifecycle-frame-exit", "Composite transition must leave the bottom of its actual frame containing its consecutive source states", json!({"from":from,"to":to,"point":at,"frame":frame})));
                    None
                }
            } else {
                let side = body_side(at, node, radii.get(id).copied().unwrap_or((0.0, 0.0)));
                if side.is_none() {
                    diagnostics.push(error(kind, index, raw, "artifact/endpoint-binding", "Actual relationship endpoint does not touch its referenced node body", json!({"endpoint":if source_endpoint {"source"} else {"target"},"node":id,"point":at,"bounds":node})));
                }
                side
            };
            let authored_side = explicit_side(raw, field);
            if let Some(authored_side) = authored_side {
                if side.is_some_and(|side| side != authored_side) {
                    diagnostics.push(error(kind, index, raw, "artifact/endpoint-side-position", "Actual endpoint touches a different body side than its authored side", json!({"field":field,"side":authored_side,"actualSide":side,"point":at,"node":id})));
                }
            }
            let checked_side = authored_side.or(if has_route_geometry { None } else { side });
            if legacy_via {
                continue;
            }
            if let Some(side) = checked_side {
                let issues = collect_endpoint_side_issues(
                    &part.points,
                    if source_endpoint { side } else { "" },
                    if source_endpoint { "" } else { side },
                );
                if !issues.is_empty() || geometry::normalize_route_points(&part.points).len() < 2 {
                    diagnostics.push(error(kind, index, raw, "artifact/endpoint-side-direction", "Actual endpoint tangent does not leave or enter its selected body side perpendicularly", json!({"field":field,"side":side,"sideOrigin":if authored_side.is_some() {"authored"} else {"actual-boundary"},"issues":issues})));
                }
            }
        }
        if let Some(via) = via_pins {
            let via: Option<Vec<_>> = via.iter().map(point).collect();
            if via.is_none_or(|via| {
                !matches_pins(
                    kind,
                    version,
                    raw,
                    &parts,
                    std::iter::once(start)
                        .chain(via)
                        .chain(std::iter::once(end))
                        .collect(),
                )
            }) {
                diagnostics.push(error(
                    kind,
                    index,
                    raw,
                    "artifact/route-pin",
                    "Actual path differs from the authored via geometry",
                    json!({"field":"via","via":raw["via"],"actualParts":parts}),
                ));
            }
        } else if let Some(points) = channel_points(
            kind,
            version,
            raw,
            start,
            end,
            explicit_side(raw, "fromSide").or_else(|| {
                body_side(
                    start,
                    from_bounds,
                    radii.get(from).copied().unwrap_or((0.0, 0.0)),
                )
            }),
            explicit_side(raw, "toSide").or_else(|| {
                body_side(end, to_bounds, radii.get(to).copied().unwrap_or((0.0, 0.0)))
            }),
        ) {
            if !matches_pins(kind, version, raw, &parts, points) {
                diagnostics.push(error(kind, index, raw, "artifact/route-pin", "Actual path differs from its authored channel coordinates", json!({"route":raw["route"],"channelX":raw["channelX"],"channelY":raw["channelY"],"actualParts":parts})));
            }
        }
        // Readable-v2 keeps channel assertions even when via supplies the
        // coordinates. Its radius buffer leaves a real straight pin segment.
        if kind == DiagramType::Workflow && version == 2 {
            for field in ["channelX", "channelY"] {
                let Some(coordinate) = raw[field].as_f64() else {
                    continue;
                };
                let present = parts.iter().any(|part| {
                    part.points.windows(2).any(|pair| {
                        if field == "channelX" {
                            (pair[0].x - coordinate).abs() <= EPSILON
                                && (pair[1].x - coordinate).abs() <= EPSILON
                                && (pair[1].y - pair[0].y).abs() > EPSILON
                        } else {
                            (pair[0].y - coordinate).abs() <= EPSILON
                                && (pair[1].y - coordinate).abs() <= EPSILON
                                && (pair[1].x - pair[0].x).abs() > EPSILON
                        }
                    })
                });
                if !present {
                    diagnostics.push(error(
                        kind,
                        index,
                        raw,
                        "artifact/route-pin",
                        "Actual workflow path has no straight segment at its authored channel pin",
                        json!({"field":field,"coordinate":coordinate,"actualParts":parts}),
                    ));
                }
            }
        }
    }
    diagnostics
}
