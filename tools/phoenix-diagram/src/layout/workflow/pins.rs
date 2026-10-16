use super::*;
use crate::geometry::composition::{
    collect_ambiguous_corridors, collect_label_route_clearance,
    forward_collinear_analysis_segments, source_segment_index_at_point, CorridorOptions, LabelRect,
    LabelRouteClearance, RoutedRelation,
};
use crate::geometry::rects_overlap_for_contract;
use crate::scene::Relation;

fn index(relation: &Relation) -> usize {
    relation
        .authored
        .source_path
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
fn name(edge: &Value) -> String {
    edge.get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}->{}", string(edge, "from"), string(edge, "to")))
}
pub(super) fn route_fields(edge: &Value) -> Vec<&'static str> {
    ["via", "channelX", "channelY", "route", "fromSide", "toSide"]
        .into_iter()
        .filter(|field| {
            edge.get(*field).is_some()
                && (!["route", "fromSide", "toSide"].contains(field)
                    || !["", "auto"].contains(&string(edge, field)))
        })
        .collect()
}
pub(super) fn absolute_route(edge: &Value) -> bool {
    ["via", "channelX", "channelY"]
        .iter()
        .any(|field| edge.get(*field).is_some())
}
fn point(point: Point) -> [f64; 2] {
    [point.x, point.y]
}
fn points(points: &[Point]) -> Vec<[f64; 2]> {
    points.iter().copied().map(point).collect()
}
fn pin(input: &DiagramInput, edge: usize, field: &str) -> Value {
    let value = &input.document["edges"][edge];
    json!({"edge":name(value),"field":field,"path":format!("/edges/{edge}/{field}"),"value":value[field]})
}
fn assertions(input: &DiagramInput, left: &Relation, right: &Relation) -> Vec<Value> {
    [left, right]
        .into_iter()
        .flat_map(|relation| {
            route_fields(&relation.authored.facts)
                .into_iter()
                .map(|field| pin(input, index(relation), field))
        })
        .collect()
}
fn diagnostic(
    input: &DiagramInput,
    relation: &Relation,
    invariant: &str,
    pins: Vec<Value>,
    mut evidence: Value,
) -> Diagnostic {
    let edge = index(relation);
    let path = if pins.len() == 1 {
        pins[0]["path"].as_str().unwrap().to_owned()
    } else {
        format!("/edges/{edge}")
    };
    evidence["invariant"] = json!(invariant);
    evidence["conflictingPins"] = json!(pins);
    let mut result = failure(
        "workflow/explicit-pin-conflict",
        &path,
        format!(
            "Workflow edge \"{}\" has explicit geometry that violates {invariant}.",
            name(&relation.authored.facts)
        ),
        evidence,
    );
    result.subject.identity = input.document["edges"][edge]["id"]
        .as_str()
        .map(str::to_owned);
    result
}
fn labels(relations: &[Relation]) -> Vec<LabelRect> {
    relations
        .iter()
        .filter_map(|relation| {
            relation.label_plate.map(|rect| {
                LabelRect::new(
                    index(relation) as isize,
                    relation.authored.facts.clone(),
                    rect,
                )
            })
        })
        .collect()
}
fn routes(relations: &[Relation]) -> Vec<RoutedRelation> {
    relations
        .iter()
        .map(|relation| {
            RoutedRelation::new(
                index(relation),
                relation.authored.facts.clone(),
                relation.points.clone(),
            )
        })
        .collect()
}
fn label_label(input: &DiagramInput, left: &Relation, right: &Relation) -> Option<Diagnostic> {
    let pins = [left, right]
        .into_iter()
        .filter(|relation| relation.authored.facts.get("labelAt").is_some())
        .map(|relation| pin(input, index(relation), "labelAt"))
        .collect::<Vec<_>>();
    if pins.is_empty() {
        return None;
    }
    let edge = if left.authored.facts.get("labelAt").is_some() {
        left
    } else {
        right
    };
    let rects=[left,right].map(|relation|{
        let rect=relation.label_plate.unwrap();
        json!({"edge":relation.authored.facts.get("id").unwrap_or(&Value::Null),"x":rect.x,"y":rect.y,"width":rect.width,"height":rect.height})
    });
    Some(diagnostic(
        input,
        edge,
        "explicit label-label clearance",
        pins,
        json!({"labelRects":rects,"minimumGapPx":-2}),
    ))
}
fn label_route(
    input: &DiagramInput,
    label: &Relation,
    route: &Relation,
    hit: &LabelRouteClearance,
) -> Option<Diagnostic> {
    let label_pinned = label.authored.facts.get("labelAt").is_some();
    let mut pins = Vec::new();
    if label_pinned {
        pins.push(pin(input, index(label), "labelAt"));
    }
    pins.extend(
        route_fields(&route.authored.facts)
            .into_iter()
            .map(|field| pin(input, index(route), field)),
    );
    if pins.is_empty() {
        return None;
    }
    let mut evidence = json!({"labelRect":hit.rect,"collidedRoute":{"edge":name(&route.authored.facts),"from":route.from,"to":route.to,"points":points(&route.points)},"routeSegmentIndex":hit.segment_index,"routeSegment":{"from":point(hit.start),"to":point(hit.end)},"clearancePx":(hit.clearance*10.0+0.5).floor()/10.0,"minimumPx":hit.threshold});
    if label_pinned {
        evidence["labelAt"] = label.authored.facts["labelAt"].clone();
    }
    Some(diagnostic(
        input,
        if label_pinned { label } else { route },
        "explicit label-route clearance",
        pins,
        evidence,
    ))
}

// Inspect rejected raw families only after their normal feasibility and
// expanded-corridor attempts have failed. Their order is part of the cause.
pub(super) fn failed_candidate(
    input: &DiagramInput,
    candidate: &Relation,
    prior: &[Relation],
) -> Option<Diagnostic> {
    if let Some(rect) = candidate.label_plate {
        for other in prior {
            if other.label_plate.is_some_and(|other_rect| {
                rects_overlap_for_contract(&rect, &other_rect, -2.0, input.input_contract)
            }) {
                if let Some(diagnostic) = label_label(input, candidate, other) {
                    return Some(diagnostic);
                }
            }
        }
        for hit in collect_label_route_clearance(
            &labels(std::slice::from_ref(candidate)),
            &routes(prior),
            4.0,
        ) {
            let route = prior
                .iter()
                .find(|relation| index(relation) == hit.other_relation_index)
                .unwrap();
            if candidate.authored.facts.get("labelAt").is_some()
                || absolute_route(&route.authored.facts)
            {
                if let Some(diagnostic) = label_route(input, candidate, route, &hit) {
                    return Some(diagnostic);
                }
            }
        }
    }
    for hit in collect_label_route_clearance(
        &labels(prior),
        &routes(std::slice::from_ref(candidate)),
        4.0,
    ) {
        let label = prior
            .iter()
            .find(|relation| index(relation) == hit.label_relation_index as usize)
            .unwrap();
        if label.authored.facts.get("labelAt").is_some() {
            if let Some(diagnostic) = label_route(input, label, candidate, &hit) {
                return Some(diagnostic);
            }
        }
    }
    None
}

fn orthogonal_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let horizontal = |a: Point, b: Point| (a.y - b.y).abs() <= 0.0001;
    let vertical = |a: Point, b: Point| (a.x - b.x).abs() <= 0.0001;
    let (a, b, c, d) = if horizontal(a, b) && vertical(c, d) {
        (a, b, c, d)
    } else if vertical(a, b) && horizontal(c, d) {
        (c, d, a, b)
    } else {
        return None;
    };
    let point = Point { x: c.x, y: a.y };
    (point.x > a.x.min(b.x) + 0.0001
        && point.x < a.x.max(b.x) - 0.0001
        && point.y > c.y.min(d.y) + 0.0001
        && point.y < c.y.max(d.y) - 0.0001)
        .then_some(point)
}
fn segment(relation: &Relation, start: Point, end: Point) -> Value {
    json!({"edge":relation.authored.facts.get("id").unwrap_or(&Value::Null),"from":point(start),"to":point(end)})
}

pub(super) fn pairwise(input: &DiagramInput, relations: &[Relation]) -> Option<Diagnostic> {
    for hit in collect_label_route_clearance(&labels(relations), &routes(relations), 4.0) {
        let label = relations
            .iter()
            .find(|relation| index(relation) == hit.label_relation_index as usize)
            .unwrap();
        let route = relations
            .iter()
            .find(|relation| index(relation) == hit.other_relation_index)
            .unwrap();
        if let Some(diagnostic) = label_route(input, label, route, &hit) {
            return Some(diagnostic);
        }
    }
    for (i, left) in relations.iter().enumerate() {
        for right in &relations[i + 1..] {
            if left
                .label_plate
                .zip(right.label_plate)
                .is_some_and(|(a, b)| {
                    rects_overlap_for_contract(&a, &b, -2.0, input.input_contract)
                })
            {
                if let Some(diagnostic) = label_label(input, left, right) {
                    return Some(diagnostic);
                }
            }
        }
    }
    if input.quality != crate::input::Quality::Showcase {
        return None;
    }
    for (i, left) in relations.iter().enumerate() {
        for right in &relations[i + 1..] {
            let left_pinned = !route_fields(&left.authored.facts).is_empty();
            if !left_pinned && route_fields(&right.authored.facts).is_empty() {
                continue;
            }
            if [&left.from, &left.to]
                .into_iter()
                .any(|id| id == &right.from || id == &right.to)
            {
                continue;
            }
            for a in forward_collinear_analysis_segments(&left.points) {
                for b in forward_collinear_analysis_segments(&right.points) {
                    if let Some(at) = orthogonal_intersection(a.start, a.end, b.start, b.end) {
                        let ai = source_segment_index_at_point(&a, at);
                        let bi = source_segment_index_at_point(&b, at);
                        return Some(diagnostic(
                            input,
                            if left_pinned { left } else { right },
                            "explicit route-route crossing",
                            assertions(input, left, right),
                            json!({"point":point(at),"segmentIndex":ai,"otherSegmentIndex":bi,"routeSegments":[segment(left,a.start,a.end),segment(right,b.start,b.end)],"sourceRouteSegments":[segment(left,left.points[ai],left.points[ai+1]),segment(right,right.points[bi],right.points[bi+1])]}),
                        ));
                    }
                }
            }
        }
    }
    for hit in collect_ambiguous_corridors(
        &routes(relations),
        &CorridorOptions {
            contract: input.input_contract,
            ..Default::default()
        },
    ) {
        let left = relations
            .iter()
            .find(|relation| index(relation) == hit.left_relation_index)
            .unwrap();
        let right = relations
            .iter()
            .find(|relation| index(relation) == hit.right_relation_index)
            .unwrap();
        let left_pinned = !route_fields(&left.authored.facts).is_empty();
        if !left_pinned && route_fields(&right.authored.facts).is_empty() {
            continue;
        }
        return Some(diagnostic(
            input,
            if left_pinned { left } else { right },
            "explicit route-route corridor clearance",
            assertions(input, left, right),
            json!({"segmentIndex":hit.left_segment,"otherSegmentIndex":hit.right_segment,"routeSegments":[segment(left,left.points[hit.left_segment],left.points[hit.left_segment+1]),segment(right,right.points[hit.right_segment],right.points[hit.right_segment+1])],"overlapStart":point(hit.overlap_start),"overlapEnd":point(hit.overlap_end),"overlapLengthPx":hit.overlap_length,"minimumClearancePx":8}),
        ));
    }
    None
}

fn changes_for(pins: &[Value]) -> Vec<Value> {
    pins.iter()
        .map(|pin| json!({"op":"remove","path":pin["path"]}))
        .collect()
}
fn changed_document(input: &DiagramInput, changes: &[Value]) -> Value {
    let mut candidate = input.document.clone();
    for change in changes {
        let (parent, field) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
        let object = candidate
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap();
        if change["op"] == "set" {
            object.insert(field.into(), change["value"].clone());
        } else {
            object.remove(field);
        }
    }
    candidate
}
fn verified(input: &DiagramInput, options: &CompileOptions, changes: &[Value]) -> bool {
    candidate_passes(input, options, &changed_document(input, changes))
}
fn nudges(input: &DiagramInput, options: &CompileOptions, pin: &Value) -> Vec<Vec<Value>> {
    let at = pin["value"].as_array().unwrap();
    let (x, y) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap());
    [
        (0.0, 24.0),
        (0.0, -24.0),
        (24.0, 0.0),
        (-24.0, 0.0),
        (0.0, 48.0),
        (0.0, -48.0),
        (48.0, 0.0),
        (-48.0, 0.0),
    ]
    .into_iter()
    .map(|(dx, dy)| vec![json!({"op":"set","path":pin["path"],"value":[x+dx,y+dy]})])
    .filter(|changes| verified(input, options, changes))
    .collect()
}
fn edge_at_path<'a>(input: &'a DiagramInput, path: &str) -> &'a Value {
    let (parent, _) = path.rsplit_once('/').unwrap();
    input.document.pointer(parent).unwrap()
}
fn fix_message(input: &DiagramInput, changes: &[Value], reason: &str) -> String {
    if changes[0]["op"] == "set" {
        let path = changes[0]["path"].as_str().unwrap();
        let name = name(edge_at_path(input, path));
        if path.ends_with("/labelAt") {
            let p = &changes[0]["value"];
            return format!(
                "set labelAt on edge \"{name}\" to [{}, {}]",
                p[0].as_f64().unwrap(),
                p[1].as_f64().unwrap()
            );
        }
        if path.ends_with("/labelSegment") {
            return format!(
                "set labelSegment on edge \"{name}\" to {}",
                changes[0]["value"]
            );
        }
        return format!(
            "set edge \"{name}\" route to verified preset \"{}\"",
            changes[0]["value"].as_str().unwrap()
        );
    }
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    for change in changes {
        let (parent, field) = change["path"].as_str().unwrap().rsplit_once('/').unwrap();
        if let Some((_, fields)) = groups.iter_mut().find(|(path, _)| *path == parent) {
            fields.push(field);
        } else {
            groups.push((parent, vec![field]));
        }
    }
    format!(
        "{} {reason}",
        groups
            .into_iter()
            .map(|(path, fields)| format!(
                "remove {} from edge \"{}\"",
                fields.join(" and "),
                name(input.document.pointer(path).unwrap())
            ))
            .collect::<Vec<_>>()
            .join(" and ")
    )
}

// Proof search is deliberately outside routing: recursive candidates compile
// the complete diagram without discovering another set of proposed repairs.
pub(super) fn verify(
    input: &DiagramInput,
    options: &CompileOptions,
    diagnostic: &mut Diagnostic,
) -> bool {
    let invariant = string(&diagnostic.evidence, "invariant").to_owned();
    if ![
        "explicit label-route clearance",
        "explicit label-label clearance",
        "explicit route-route crossing",
        "explicit route-route corridor clearance",
    ]
    .contains(&invariant.as_str())
    {
        return false;
    }
    let initial = array(&diagnostic.evidence, "conflictingPins").to_vec();
    let removals = changes_for(&initial);
    let mut minimal = Vec::new();
    for size in 1..=removals.len() {
        minimal = removal_combinations(&removals, size)
            .into_iter()
            .filter(|changes| verified(input, options, changes))
            .collect();
        if !minimal.is_empty() {
            break;
        }
    }
    let mut causal = Vec::new();
    for change in minimal.iter().flatten() {
        if causal
            .iter()
            .any(|pin: &Value| pin["path"] == change["path"])
        {
            continue;
        }
        causal.push(
            initial
                .iter()
                .find(|pin| pin["path"] == change["path"])
                .unwrap()
                .clone(),
        );
    }
    if minimal.is_empty() {
        causal = initial.clone();
    }
    let label_route = invariant == "explicit label-route clearance";
    let phoenix_label_route = label_route && input.input_contract == InputContract::PhoenixV1;
    let label_pin = initial.iter().find(|pin| pin["field"] == "labelAt");
    let label_nudges = if phoenix_label_route {
        label_pin
            .map(|pin| nudges(input, options, pin))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if !label_nudges.is_empty() {
        let pin = label_pin.unwrap();
        if !causal.iter().any(|causal| causal["path"] == pin["path"]) {
            causal.insert(0, pin.clone());
        }
    }
    let mut repairs = label_nudges.clone();
    for removal in minimal {
        if removal.len() == 1 && removal[0]["path"].as_str().unwrap().ends_with("/labelAt") {
            if phoenix_label_route {
                if !label_nudges.is_empty() {
                    continue;
                }
            } else {
                let pin = initial
                    .iter()
                    .find(|pin| pin["path"] == removal[0]["path"])
                    .unwrap();
                let replacements = nudges(input, options, pin);
                if !replacements.is_empty() {
                    repairs.extend(replacements);
                    continue;
                }
            }
        }
        repairs.push(removal);
    }
    let reason = if label_route && label_pin.is_some() {
        "so readable-v2 can replan the remaining authored label-route pins"
    } else if invariant == "explicit label-label clearance" {
        "so readable-v2 can replan the remaining authored label pins"
    } else {
        "so readable-v2 can replan the remaining authored route assertions"
    };
    diagnostic.supported_fixes = repairs
        .iter()
        .map(|changes| fix_message(input, changes, reason))
        .collect();
    if label_route || invariant == "explicit label-label clearance" {
        if let Some(pin) = causal.first() {
            let edge = edge_at_path(input, pin["path"].as_str().unwrap());
            diagnostic.subject.identity = edge["id"].as_str().map(str::to_owned);
            diagnostic.subject.path = pin["path"]
                .as_str()
                .unwrap()
                .rsplit_once('/')
                .unwrap()
                .0
                .into();
            diagnostic.message = format!(
                "Workflow edge \"{}\" has explicit geometry that violates {invariant}.",
                name(edge)
            );
        }
    }
    if causal.len() == 1 {
        diagnostic.subject.path = causal[0]["path"].as_str().unwrap().into();
    }
    diagnostic.evidence["conflictingPins"] = json!(causal);
    if !repairs.is_empty() {
        diagnostic.evidence["verifiedRepairs"] = json!(repairs
            .into_iter()
            .map(|changes| json!({"scope":"complete-workflow-compile","changes":changes}))
            .collect::<Vec<_>>());
    }
    true
}

pub(super) fn verify_preset(
    input: &DiagramInput,
    options: &CompileOptions,
    diagnostic: &mut Diagnostic,
) -> bool {
    if diagnostic.code != "workflow/route-preset-conflict" {
        return false;
    }
    let Some(edge_index) = diagnostic
        .subject
        .path
        .split('/')
        .nth(2)
        .and_then(|index| index.parse::<usize>().ok())
    else {
        return false;
    };
    let edge = &input.document["edges"][edge_index];
    let path = format!("/edges/{edge_index}/route");
    let mut repairs = Vec::new();
    if input.input_contract == InputContract::PhoenixV1
        && string(&diagnostic.evidence, "invariant") == "edge-label node clearance"
        && !string(edge, "label").is_empty()
        && ["labelAt", "labelDx", "labelDy", "labelSegment"]
            .iter()
            .all(|field| edge.get(*field).is_none())
    {
        for segment in 0..array(&diagnostic.evidence, "points")
            .len()
            .saturating_sub(1)
            .min(3)
        {
            let changes = vec![
                json!({"op":"set","path":format!("/edges/{edge_index}/labelSegment"),"value":segment}),
            ];
            if verified(input, options, &changes) {
                repairs.push(changes);
                break;
            }
        }
    }
    let mut proposals = [
        "straight",
        "drop",
        "outside-right",
        "return-left",
        "bottom-channel",
        "up-channel",
    ]
    .into_iter()
    .filter(|preset| *preset != string(edge, "route"))
    .map(|preset| vec![json!({"op":"set","path":path,"value":preset})])
    .collect::<Vec<_>>();
    proposals.push(vec![json!({"op":"remove","path":path})]);
    repairs.extend(
        proposals
            .into_iter()
            .filter(|changes| verified(input, options, changes)),
    );
    diagnostic.supported_fixes = repairs
        .iter()
        .map(|changes| {
            fix_message(
                input,
                changes,
                "so readable-v2 can use its verified automatic candidate",
            )
        })
        .collect();
    if !repairs.is_empty() {
        diagnostic.evidence["verifiedRepairs"] = json!(repairs
            .into_iter()
            .map(|changes| json!({"scope":"complete-workflow-compile","changes":changes}))
            .collect::<Vec<_>>());
    }
    true
}
