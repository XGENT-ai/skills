// Frozen legacy-2.17 lifecycle v1: no migration to the v3 main-path layout.
use super::*;
use crate::geometry::composition::*;
use crate::geometry::{anchor, rects_overlap_for_contract};
use crate::input::Quality;
use std::collections::BTreeMap;

fn band(lane: &str) -> usize {
    if lane == "main" {
        0
    } else if lane == "terminal" {
        2
    } else {
        1
    }
}
fn sides(raw: &Value, a: Rect, b: Rect) -> (String, String) {
    let (ax, ay, bx, by) = (
        a.x + a.width / 2.0,
        a.y + a.height / 2.0,
        b.x + b.width / 2.0,
        b.y + b.height / 2.0,
    );
    let (from, to) = if bx < ax {
        ("left", "right")
    } else if bx > ax {
        ("right", "left")
    } else if by > ay {
        ("bottom", "top")
    } else {
        ("top", "bottom")
    };
    let chosen = |key: &str, default: &str| {
        raw[key]
            .as_str()
            .filter(|s| *s != "auto" && !s.is_empty())
            .unwrap_or(default)
            .to_owned()
    };
    (chosen("fromSide", from), chosen("toSide", to))
}
// Preserve the source router's explicit boxes, anchors, sides and spread mode.
#[allow(clippy::too_many_arguments)]
fn route(
    raw: &Value,
    a: Rect,
    b: Rect,
    start: Point,
    end: Point,
    from_side: &str,
    to_side: &str,
    spread: bool,
) -> Vec<Point> {
    let mut via = if raw["via"].is_array() {
        list(&raw["via"]).iter().filter_map(point).collect()
    } else {
        match raw["route"].as_str().unwrap_or("auto") {
            "straight" => vec![],
            "drop" => {
                let y = number(raw, "channelY", (start.y + end.y) / 2.0);
                vec![p(start.x, y), p(end.x, y)]
            }
            "bottom-channel" => {
                let y = number(raw, "channelY", (a.y + a.height).max(b.y + b.height) + 34.0);
                vec![p(start.x, y), p(end.x, y)]
            }
            "top-channel" => {
                let y = number(raw, "channelY", a.y.min(b.y) - 28.0);
                vec![p(start.x, y), p(end.x, y)]
            }
            "right-channel" => {
                let x = number(raw, "channelX", (a.x + a.width).max(b.x + b.width) + 36.0);
                vec![p(x, start.y), p(x, end.y)]
            }
            "left-channel" => {
                let x = number(raw, "channelX", a.x.min(b.x) - 36.0);
                vec![p(x, start.y), p(x, end.y)]
            }
            _ => {
                if start.x == end.x || start.y == end.y {
                    vec![]
                } else {
                    let from_vertical = matches!(from_side, "top" | "bottom");
                    let to_vertical = matches!(to_side, "top" | "bottom");
                    if from_vertical != to_vertical {
                        vec![if from_vertical {
                            p(start.x, end.y)
                        } else {
                            p(end.x, start.y)
                        }]
                    } else if from_vertical {
                        let y = number(raw, "channelY", (start.y + end.y) / 2.0);
                        vec![p(start.x, y), p(end.x, y)]
                    } else {
                        let x = number(raw, "channelX", (start.x + end.x) / 2.0);
                        vec![p(x, start.y), p(x, end.y)]
                    }
                }
            }
        }
    };
    if spread && via.is_empty() && (start.x - end.x).abs() >= 4.0 && (start.y - end.y).abs() >= 4.0
    {
        let x = (start.x + end.x) / 2.0;
        via = vec![p(x, start.y), p(x, end.y)];
    }
    let mut points = vec![start];
    points.extend(via);
    points.push(end);
    points
}

pub(super) fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let view_box = Rect {
        x: 0.0,
        y: 0.0,
        width: document["meta"]["viewBox"][0].as_f64().unwrap_or(980.0),
        height: document["meta"]["viewBox"][1].as_f64().unwrap_or(660.0),
    };
    let mut errors = vec![];
    let mut nodes = vec![];
    for (index, raw) in list(&document["states"]).iter().enumerate() {
        let lane = band(string(raw, "lane"));
        let xs: &[f64] = if lane == 0 {
            &[94.0, 248.0, 402.0, 556.0, 710.0]
        } else {
            &[402.0, 556.0, 710.0]
        };
        let col = number(raw, "col", f64::NAN);
        if !col.is_finite() || col.fract() != 0.0 || col < 0.0 || col >= xs.len() as f64 {
            errors.push(failure(
                "lifecycle/invalid-column",
                &format!("/states/{index}/col"),
                "State column exceeds its fixed lifecycle band",
                json!({"column":col,"columns":xs.len()}),
            ));
            continue;
        }
        let width = number(raw, "width", if lane == 1 { 126.0 } else { 118.0 });
        let height = number(raw, "height", if lane == 0 { 62.0 } else { 58.0 });
        let x = xs[col as usize] - width / 2.0;
        let y = [126.0, 278.0, 450.0][lane] + number(raw, "yOffset", 0.0);
        let bounds = Rect {
            x,
            y,
            width,
            height,
        };
        if x < 32.0
            || x + width > view_box.width - 32.0
            || y < 64.0
            || y + height > view_box.height - 122.0
        {
            errors.push(failure(
                "lifecycle/state-bounds",
                &format!("/states/{index}"),
                "State exceeds the fixed lifecycle area",
                json!({"bounds":bounds,"viewBox":view_box}),
            ));
        }
        if units(input, string(raw, "label")) * 6.2 > width + 6.0 {
            errors.push(failure(
                "lifecycle/state-text-too-long",
                &format!("/states/{index}/label"),
                "State label exceeds its fixed width",
                json!({"width":width}),
            ));
        }
        for field in ["sublabel", "tag"] {
            if units(input, string(raw, field)) * 6.0 * 0.6 > width - 8.0 {
                errors.push(failure(
                    "lifecycle/state-text-too-long",
                    &format!("/states/{index}/{field}"),
                    "State text cannot fit at the legible minimum",
                    json!({"availableWidth":width-8.0}),
                ));
            }
        }
        let mut texts = vec![text(
            format!("/states/{index}/label"),
            string(raw, "label"),
            xs[col as usize],
            y + 21.0,
            font(input, string(raw, "label"), width, 10.0, 8.0),
            "middle",
        )];
        for (field, baseline) in [("sublabel", y + 37.0), ("tag", y + height - 11.0)] {
            if !string(raw, field).is_empty() {
                texts.push(text(
                    format!("/states/{index}/{field}"),
                    string(raw, field),
                    xs[col as usize],
                    baseline,
                    font(input, string(raw, field), width, 7.0, 6.0),
                    "middle",
                ));
            }
        }
        if !string(raw, "step").is_empty() {
            texts.insert(
                0,
                text(
                    format!("/states/{index}/step"),
                    string(raw, "step"),
                    x + 10.0,
                    y + 14.0,
                    7.0,
                    "start",
                ),
            );
        }
        nodes.push(Node {
            authored: authored(
                raw,
                "states",
                index,
                string(raw, "type"),
                &["lane", "col", "yOffset", "width", "height"],
            ),
            bounds,
            sigil: Some(Sigil {
                kind: string(raw, "type").into(),
                position: p(x + width - 17.0, y + 6.0),
                size: 11.0,
            }),
            label: string(raw, "label").into(),
            text: texts,
        });
    }
    if view_box.height - 118.0 < 448.0 {
        errors.push(failure(
            "lifecycle/fixed-band-canvas",
            "/meta/viewBox",
            "Canvas is too short for the fixed lifecycle bands",
            json!({"minimumHeight":566}),
        ));
    }
    for (index, a) in nodes.iter().enumerate() {
        for b in nodes.iter().skip(index + 1) {
            if rects_overlap_for_contract(&a.bounds, &b.bounds, 10.0, input.input_contract) {
                errors.push(failure(
                    "lifecycle/state-overlap",
                    &a.authored.source_path,
                    "States must remain at least 10px apart across all lifecycle bands",
                    json!({"otherState":b.authored.id}),
                ));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let node_index: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.authored.id.as_str(), i))
        .collect();
    let transitions = list(&document["transitions"]);
    let mut endpoints = vec![];
    for (index, raw) in transitions.iter().enumerate() {
        let (Some(&a), Some(&b)) = (
            node_index.get(string(raw, "from")),
            node_index.get(string(raw, "to")),
        ) else {
            errors.push(failure(
                "lifecycle/unknown-endpoint",
                &format!("/transitions/{index}"),
                "Transition endpoint is not a lifecycle state",
                Value::Null,
            ));
            continue;
        };
        let (from, to) = sides(raw, nodes[a].bounds, nodes[b].bounds);
        endpoints.push((a, b, from, to));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    // Members retain transition index, source-end flag and counterpart node.
    type PortMember = (usize, bool, usize);
    type PortGroup = ((usize, String), Vec<PortMember>);
    let mut groups: Vec<PortGroup> = vec![];
    for (index, raw) in transitions.iter().enumerate() {
        if raw.get("via").is_some()
            || raw.get("channelX").is_some()
            || raw.get("channelY").is_some()
            || raw.get("labelAt").is_some()
            || raw["route"]
                .as_str()
                .is_some_and(|s| s != "auto" && !s.is_empty())
        {
            continue;
        }
        let (a, b, from, to) = &endpoints[index];
        for (node, side, source, other) in
            [(*a, from.clone(), true, *b), (*b, to.clone(), false, *a)]
        {
            if let Some((_, items)) = groups
                .iter_mut()
                .find(|(key, _)| key == &(node, side.clone()))
            {
                items.push((index, source, other));
            } else {
                groups.push(((node, side), vec![(index, source, other)]));
            }
        }
    }
    let mut ports: Vec<(Option<Point>, Option<Point>)> = vec![(None, None); transitions.len()];
    for ((node, side), mut items) in groups {
        if items.len() < 2 {
            continue;
        }
        let vertical = side == "left" || side == "right";
        items.sort_by(|a, b| {
            let rect_a = nodes[a.2].bounds;
            let rect_b = nodes[b.2].bounds;
            let ca = if vertical {
                rect_a.y + rect_a.height / 2.0
            } else {
                rect_a.x + rect_a.width / 2.0
            };
            let cb = if vertical {
                rect_b.y + rect_b.height / 2.0
            } else {
                rect_b.x + rect_b.width / 2.0
            };
            ca.total_cmp(&cb).then_with(|| {
                let key = |i: usize| {
                    format!(
                        "{}\0{}\0{}\0{}",
                        string(&transitions[i], "id"),
                        string(&transitions[i], "from"),
                        string(&transitions[i], "to"),
                        string(&transitions[i], "label")
                    )
                };
                key(a.0).encode_utf16().cmp(key(b.0).encode_utf16())
            })
        });
        let rect = nodes[node].bounds;
        let extent = if vertical { rect.height } else { rect.width };
        let spacing = 14.0_f64.min((extent - 32.0).max(0.0) / (items.len() - 1) as f64);
        if spacing <= 0.0 {
            continue;
        }
        for (slot, (edge, source, _)) in items.iter().enumerate() {
            let mut port = anchor(&rect, &side);
            let offset = (slot as f64 - (items.len() - 1) as f64 / 2.0) * spacing;
            if vertical {
                port.y += offset;
            } else {
                port.x += offset;
            }
            if *source {
                ports[*edge].0 = Some(port);
            } else {
                ports[*edge].1 = Some(port);
            }
        }
    }
    let mut relations = vec![];
    let mut routed = vec![];
    let mut label_rects = vec![];
    for (index, raw) in transitions.iter().enumerate() {
        let (a, b, from, to) = &endpoints[index];
        let start = ports[index]
            .0
            .unwrap_or_else(|| anchor(&nodes[*a].bounds, from));
        let end = ports[index]
            .1
            .unwrap_or_else(|| anchor(&nodes[*b].bounds, to));
        let points = route(
            raw,
            nodes[*a].bounds,
            nodes[*b].bounds,
            start,
            end,
            from,
            to,
            ports[index].0.is_some() || ports[index].1.is_some(),
        );
        if (start.x - end.x).hypot(start.y - end.y) < 32.0 {
            errors.push(failure(
                "lifecycle/transition-too-short",
                &format!("/transitions/{index}"),
                "Transition endpoint distance is below 32px",
                json!({"start":start,"end":end}),
            ));
        }
        if !raw["via"].is_array() {
            let has_named = raw["route"].as_str().is_some_and(|s| s != "auto");
            let check_from = raw["fromSide"]
                .as_str()
                .filter(|s| *s != "auto")
                .or(if has_named { None } else { Some(from.as_str()) })
                .unwrap_or("");
            let check_to = raw["toSide"]
                .as_str()
                .filter(|s| *s != "auto")
                .or(if has_named { None } else { Some(to.as_str()) })
                .unwrap_or("");
            for hit in collect_endpoint_side_issues(&points, check_from, check_to) {
                errors.push(failure(
                    "clean-flow/endpoint-side-direction",
                    &format!("/transitions/{index}"),
                    "Transition does not honor its endpoint side",
                    json!(hit),
                ));
            }
        }
        let label = string(raw, "label");
        let note = string(raw, "note");
        let mut texts = vec![];
        let mut plate = None;
        if !label.is_empty() {
            let anchor = point(&raw["labelAt"]).unwrap_or_else(|| {
                let segment =
                    (number(raw, "labelSegment", 1.0).max(0.0) as usize).min(points.len() - 2);
                if points.len() == 2 {
                    p(
                        (start.x + end.x) / 2.0 + number(raw, "labelDx", 0.0),
                        start.y - 10.0 + number(raw, "labelDy", 0.0),
                    )
                } else {
                    p(
                        (points[segment].x + points[segment + 1].x) / 2.0
                            + number(raw, "labelDx", 0.0),
                        (points[segment].y + points[segment + 1].y) / 2.0 - 10.0
                            + number(raw, "labelDy", 0.0),
                    )
                }
            });
            let width = 32.0_f64.max(units(input, label).max(units(input, note)) * 4.9 + 12.0);
            let rect = Rect {
                x: anchor.x - width / 2.0,
                y: anchor.y - 11.0,
                width,
                height: if note.is_empty() { 16.0 } else { 27.0 },
            };
            plate = Some(rect);
            texts.push(text(
                format!("/transitions/{index}/label"),
                label,
                anchor.x,
                anchor.y,
                8.0,
                "middle",
            ));
            if !note.is_empty() {
                texts.push(text(
                    format!("/transitions/{index}/note"),
                    note,
                    anchor.x,
                    anchor.y + 11.0,
                    7.0,
                    "middle",
                ));
            }
            label_rects.push(LabelRect::new(index as isize, raw.clone(), rect));
        }
        routed.push(RoutedRelation::new(index, raw.clone(), points.clone()));
        relations.push(Relation {
            authored: authored(
                raw,
                "transitions",
                index,
                "transition",
                &[
                    "route",
                    "via",
                    "fromSide",
                    "toSide",
                    "channelX",
                    "channelY",
                    "labelAt",
                    "labelDx",
                    "labelDy",
                    "labelSegment",
                    "cornerRadius",
                ],
            ),
            from: string(raw, "from").into(),
            to: string(raw, "to").into(),
            variant: raw["variant"].as_str().unwrap_or("default").into(),
            label: label.into(),
            note: (!note.is_empty()).then(|| note.into()),
            points,
            crossover_halo: false,
            independent_ports: false,
            junction: None,
            label_owner: None,
            note_bounds: None,
            note_backdrop: None,
            label_plate: plate,
            text: texts,
        });
    }
    let obstacles: Vec<_> = nodes
        .iter()
        .map(|n| Obstacle {
            id: n.authored.id.clone(),
            bounds: n.bounds,
        })
        .collect();
    for hit in collect_route_obstacle_hits(&routed, &obstacles, 2.0) {
        errors.push(failure(
            "clean-flow/edge-through-node",
            &format!("/transitions/{}", hit.relation_index),
            "Transition crosses an unrelated state",
            json!(hit),
        ));
    }
    if input.quality == Quality::Showcase {
        for hit in collect_proper_crossings(
            &routed,
            &CrossingOptions {
                contract: input.input_contract,
                ..Default::default()
            },
        ) {
            errors.push(failure(
                "composition/proper-crossing",
                &format!("/transitions/{}", hit.left_relation_index),
                "Unrelated lifecycle transitions form a proper crossing",
                json!(hit),
            ));
        }
        for hit in collect_ambiguous_corridors(
            &routed,
            &CorridorOptions {
                contract: input.input_contract,
                ..Default::default()
            },
        ) {
            errors.push(failure(
                "composition/ambiguous-corridor",
                &format!("/transitions/{}", hit.left_relation_index),
                "Unrelated lifecycle transitions share a corridor",
                json!(hit),
            ));
        }
        for hit in collect_route_rhythm_issues(&routed, 16.0, 8.0) {
            errors.push(failure(
                hit.code,
                &format!("/transitions/{}", hit.relation_index),
                "Lifecycle transition has a cramped run",
                json!(hit),
            ));
        }
        for hit in collect_label_route_clearance(&label_rects, &routed, 4.0) {
            errors.push(failure(
                "composition/label-route-clearance",
                &format!("/transitions/{}", hit.label_relation_index),
                "Lifecycle label is too close to another route",
                json!(hit),
            ));
        }
    }
    for (index, a) in label_rects.iter().enumerate() {
        for node in &nodes {
            if rects_overlap_for_contract(&a.bounds, &node.bounds, -2.0, input.input_contract) {
                errors.push(failure(
                    "lifecycle/label-state-overlap",
                    &format!("/transitions/{}", a.relation_index),
                    "Transition label overlaps a lifecycle state",
                    json!({"state":node.authored.id}),
                ));
            }
        }
        for b in label_rects.iter().skip(index + 1) {
            if rects_overlap_for_contract(&a.bounds, &b.bounds, -2.0, input.input_contract) {
                errors.push(failure(
                    "lifecycle/label-overlap",
                    &format!("/transitions/{}", a.relation_index),
                    "Transition labels overlap",
                    json!({"otherTransition":b.relation_index}),
                ));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let lanes = list(&document["lanes"]);
    let title = |kind: usize| {
        if kind == 0 {
            lanes
                .iter()
                .find(|l| string(l, "id") == "main")
                .map(|l| string(l, "label").to_owned())
                .unwrap_or_else(|| "Lifecycle phases".into())
        } else if kind == 2 {
            lanes
                .iter()
                .find(|l| string(l, "id") == "terminal")
                .map(|l| string(l, "label").to_owned())
                .unwrap_or_else(|| "Outcomes".into())
        } else {
            let labels = lanes
                .iter()
                .filter(|l| !matches!(string(l, "id"), "main" | "terminal"))
                .map(|l| string(l, "label"))
                .collect::<Vec<_>>();
            if labels.is_empty() {
                "Interruptions + recovery".into()
            } else {
                labels.join(" + ")
            }
        }
    };
    let mut containers = vec![];
    for index in 0..3 {
        let y = [112.0, 264.0, 436.0][index];
        containers.push(Container {
            authored: generated(
                "lifecycle-band",
                format!("lifecycle-band-{index}"),
                "/lanes",
            ),
            bounds: Rect {
                x: 72.0,
                y,
                width: view_box.width - 144.0,
                height: 0.0,
            },
            label_plate: None,
            rule: Some([p(72.0, y), p(view_box.width - 72.0, y)]),
            members: nodes
                .iter()
                .filter(|n| band(string(&n.authored.facts, "lane")) == index)
                .map(|n| n.authored.id.clone())
                .collect(),
            text: vec![text(
                "/lanes",
                format!("{:02} / {}", index + 1, title(index)),
                72.0,
                y - 12.0,
                10.0,
                "start",
            )],
        });
    }
    if let Some(col) = nodes
        .iter()
        .filter(|n| string(&n.authored.facts, "lane") == "main")
        .map(|n| number(&n.authored.facts, "col", 0.0) as usize)
        .max()
    {
        let x = [94.0, 248.0, 402.0, 556.0, 710.0][col] + 38.0;
        containers.push(Container {
            authored: generated("lifecycle-rail", "lifecycle-main-rail".into(), "/states"),
            bounds: Rect {
                x: 154.0,
                y: 157.0,
                width: x - 154.0,
                height: 0.0,
            },
            label_plate: None,
            rule: Some([p(154.0, 157.0), p(x, 157.0)]),
            members: vec![],
            text: vec![],
        });
    }
    let entries = presentation::legend_entries(input, &nodes, false);
    let legends = presentation::legends(
        input,
        &entries,
        view_box,
        view_box.height - 36.0,
        view_box.height - 114.0,
    )?;
    Ok(scene(
        input, view_box, nodes, relations, containers, legends, "",
    ))
}
