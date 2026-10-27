// Source: validateDataflow; the shared collectors retain both geometry contracts.
use super::*;

fn problem(
    context: &Context<'_>,
    collection: &str,
    index: usize,
    code: &str,
    message: impl Into<String>,
    evidence: Value,
) -> Diagnostic {
    let mut result = failure(code, format!("/{collection}/{index}"), message, evidence);
    result.subject.identity = context.input.document[collection][index]["id"]
        .as_str()
        .map(str::to_owned);
    result
}
fn raised_canvas_page(width: f64, height: f64) -> Option<f64> {
    if width / height >= 1.55 {
        return None;
    }
    let page = (1346.0 * height / width + 0.5).floor() + 126.0;
    (page > 900.0).then_some(page)
}

pub(super) fn check(context: &Context<'_>) -> Result<(), Diagnostics> {
    let mut errors = Vec::new();
    let nodes = &context.nodes;
    for (index, node) in nodes.iter().enumerate() {
        let raw = &node.raw;
        let rect = node.bounds;
        if !geometry::finite_rect(&rect)
            || ![node.cx, node.cy].iter().all(|value| value.is_finite())
        {
            errors.push(problem(
                context,
                "nodes",
                index,
                "layout/constraint",
                "Node produced non-finite coordinates",
                json!({"bounds":rect}),
            ));
            continue;
        }
        if rect.x < 24.0 || rect.x + rect.width > context.view_box.width - 24.0 {
            errors.push(problem(
                context,
                "nodes",
                index,
                "layout/constraint",
                "Node exceeds the horizontal bounds of the viewBox",
                json!({"bounds":rect,"viewBox":context.view_box}),
            ));
        }
        let area_bottom = context.view_box.height - 74.0;
        if context.legacy() {
            if rect.y < 104.0 || rect.y + rect.height > area_bottom {
                errors.push(problem(
                    context,
                    "nodes",
                    index,
                    "layout/constraint",
                    "Node exceeds the readable diagram area",
                    json!({"bounds":rect,"areaTop":104,"areaBottom":area_bottom}),
                ));
            }
        } else {
            if rect.y < 104.0 {
                let mut diagnostic = problem(
                    context,
                    "nodes",
                    index,
                    "dataflow/node-above-area",
                    "Node starts above the readable diagram area; keep y at or above 104",
                    json!({"y":rect.y,"areaTop":104,"yOffset":number(raw,"yOffset",0.0)}),
                );
                diagnostic.supported_fixes.push(format!(
                    "raise or remove the negative yOffset on node \"{}\"",
                    string(raw, "id")
                ));
                errors.push(diagnostic);
            }
            if rect.y + rect.height > area_bottom {
                let required = rect.y + rect.height + 74.0;
                let room = area_bottom - 104.0;
                let height_cap = room.min(area_bottom - rect.y);
                let overflow = raised_canvas_page(context.view_box.width, required);
                let mut maximum = (context.view_box.width / 1.55).floor();
                while maximum > 1.0 && raised_canvas_page(context.view_box.width, maximum).is_some()
                {
                    maximum -= 1.0;
                }
                let mut evidence = json!({"y":rect.y,"height":rect.height,"areaBottom":area_bottom,"requiredViewBoxHeight":required,"requiredViewBoxFitsViewport":overflow.is_none(),"maximumViewBoxHeight":maximum,"viewBoxHeight":context.view_box.height});
                if let Some(page) = overflow {
                    evidence["requiredViewBoxPageHeightPx"] = json!(page);
                }
                let mut diagnostic=problem(context,"nodes",index,"dataflow/node-below-area","Node bottom exceeds the readable diagram area; preserve its authored geometry and adjust its height, yOffset, or canvas together",evidence);
                if rect.height <= room {
                    diagnostic
                        .supported_fixes
                        .push(format!("lower yOffset on node \"{}\"", string(raw, "id")));
                }
                if height_cap >= 36.0 {
                    diagnostic.supported_fixes.push(format!(
                        "reduce node \"{}\" height to at most {}",
                        string(raw, "id"),
                        geometry::format_number(height_cap)
                    ));
                }
                if overflow.is_none() {
                    diagnostic.supported_fixes.push(if maximum>required{format!("raise meta.viewBox[1] to at least {} and at most {}, then rerun deliver and visual-check",geometry::format_number(required),geometry::format_number(maximum))}else{format!("raise meta.viewBox[1] to {}, then rerun deliver and visual-check",geometry::format_number(required))});
                }
                errors.push(diagnostic);
            }
        }
        if units(context.input, string(raw, "label")) * 6.2 > rect.width + 6.0 {
            errors.push(problem(context,"nodes",index,"layout/constraint","Node label is wider than its fixed component",json!({"label":string(raw,"label"),"estimatedWidth":units(context.input,string(raw,"label"))*6.2,"width":rect.width})));
        }
        for field in ["sublabel", "tag"] {
            let value = string(raw, field);
            let minimum = context.context_minimum;
            let required = units(context.input, value) * minimum * 0.6;
            if required > rect.width - 8.0 {
                errors.push(problem(context,"nodes",index,"layout/constraint",format!("Node {field} cannot fit at the legible minimum; preserve the complete text and widen the node"),json!({"field":field,"requiredWidth":required,"availableWidth":rect.width-8.0,"minimumFont":minimum})));
            }
        }
    }
    for left in 0..nodes.len() {
        for right in left + 1..nodes.len() {
            if context.overlaps(&nodes[left].bounds, &nodes[right].bounds, 10.0) {
                errors.push(problem(
                    context,
                    "nodes",
                    left,
                    "layout/constraint",
                    "Nodes are less than 10px apart",
                    json!({"otherNode":string(&nodes[right].raw,"id"),"gap":10}),
                ));
            }
        }
    }
    let routed = context.routed();
    for (index, flow) in context.flows.iter().enumerate() {
        let points = &flow.points;
        let first = points[0];
        let last = *points.last().unwrap();
        let distance = (last.x - first.x).hypot(last.y - first.y);
        if distance < 34.0 {
            errors.push(problem(
                context,
                "flows",
                index,
                "layout/constraint",
                "Flow endpoints are closer than the 34px minimum",
                json!({"distance":distance,"minimum":34}),
            ));
        }
        if flow.raw["via"].is_array() {
            for (segment, pair) in points.windows(2).enumerate() {
                if (pair[1].x - pair[0].x).abs() > 0.01 && (pair[1].y - pair[0].y).abs() > 0.01 {
                    errors.push(problem(context,"flows",index,"layout/constraint","Authored via points produce a diagonal segment; align adjacent coordinates",json!({"segmentIndex":segment,"start":pair[0],"end":pair[1]})));
                }
            }
        }
        let named = (!string(&flow.raw, "route").is_empty() && flow.raw["route"] != "auto")
            || flow.raw["via"].is_array();
        let side = |key, fallback: &str| {
            let value = string(&flow.raw, key);
            if !value.is_empty() && value != "auto" {
                value.to_owned()
            } else if !named {
                fallback.to_owned()
            } else {
                String::new()
            }
        };
        for issue in composition::collect_endpoint_side_issues(
            points,
            &side("fromSide", &flow.sides.0),
            &side("toSide", &flow.sides.1),
        ) {
            let mut diagnostic = problem(
                context,
                "flows",
                index,
                "clean-flow/endpoint-side-direction",
                "Flow endpoint segment does not honor the authored or inferred side",
                serde_json::to_value(issue).unwrap(),
            );
            diagnostic.supported_fixes.push("align the first/final via segment with the endpoint side or remove unneeded authored routing controls".into());
            errors.push(diagnostic);
        }
    }
    let obstacles: Vec<_> = nodes
        .iter()
        .map(|node| composition::Obstacle {
            id: string(&node.raw, "id").into(),
            bounds: node.bounds,
        })
        .collect();
    for hit in composition::collect_route_obstacle_hits(&routed, &obstacles, 2.0) {
        errors.push(problem(
            context,
            "flows",
            hit.relation_index,
            "clean-flow/edge-through-node",
            "Flow crosses an unrelated opaque node",
            serde_json::to_value(hit).unwrap(),
        ));
    }
    for (index, label) in context.labels.iter().enumerate() {
        for (node_index, node) in nodes.iter().enumerate() {
            if context.overlaps(&label.label.bounds, &node.bounds, -2.0) {
                errors.push(problem(context,"flows",label.index(),"layout/constraint","Flow label overlaps a node; preserve its text and adjust the label controls or add space",json!({"node":node_index,"labelBounds":label.label.bounds,"nodeBounds":node.bounds})));
            }
        }
        for other in context.labels.iter().skip(index + 1) {
            if context.overlaps(&label.label.bounds, &other.label.bounds, -2.0) {
                errors.push(problem(context,"flows",label.index(),"layout/constraint","Flow labels overlap; move an unpinned label or add space",json!({"otherFlow":other.index(),"left":label.label.bounds,"right":other.label.bounds})));
            }
        }
    }
    if context.input.quality == Quality::Showcase {
        let resolved = |_: &Value, _: &Value, hit: &composition::ProperCrossing| {
            context.flows[hit.left_relation_index].halo
                && context.flows[hit.right_relation_index].halo
        };
        for hit in composition::collect_proper_crossings(
            &routed,
            &composition::CrossingOptions {
                contract: context.input.input_contract,
                crossing_resolved: Some(&resolved),
                ..Default::default()
            },
        ) {
            errors.push(problem(
                context,
                "flows",
                hit.left_relation_index,
                "composition/proper-crossing",
                "Unresolved flows cross in their interiors",
                serde_json::to_value(hit).unwrap(),
            ));
        }
        for hit in composition::collect_ambiguous_corridors(
            &routed,
            &composition::CorridorOptions {
                contract: context.input.input_contract,
                ..Default::default()
            },
        ) {
            errors.push(problem(
                context,
                "flows",
                hit.left_relation_index,
                "composition/ambiguous-corridor",
                "Unrelated flows share an ambiguous corridor",
                serde_json::to_value(hit).unwrap(),
            ));
        }
        for hit in composition::collect_border_runs(&routed, &context.frames) {
            errors.push(problem(
                context,
                "flows",
                hit.relation_index,
                "composition/container-border-run",
                "Flow follows a stage border",
                serde_json::to_value(hit).unwrap(),
            ));
        }
        for hit in composition::collect_route_rhythm_issues(&routed, 16.0, 8.0) {
            errors.push(problem(
                context,
                "flows",
                hit.relation_index,
                hit.code,
                "Flow contains a short segment",
                serde_json::to_value(&hit).unwrap(),
            ));
        }
        let labels: Vec<_> = context
            .labels
            .iter()
            .map(|plate| {
                composition::LabelRect::new(
                    plate.index() as isize,
                    context.flows[plate.index()].raw.clone(),
                    plate.label.bounds,
                )
            })
            .collect();
        for hit in composition::collect_label_route_clearance(&labels, &routed, 4.0) {
            errors.push(problem(
                context,
                "flows",
                hit.label_relation_index as usize,
                "composition/label-route-clearance",
                "Flow label masks a foreign route",
                serde_json::to_value(hit).unwrap(),
            ));
        }
        if !context.legacy() {
            // Author-pinned label positions are kept as authored.
            let automatic: Vec<_> = labels
                .iter()
                .filter(|label| !label_pinned(&label.relation))
                .cloned()
                .collect();
            for hit in composition::collect_label_arrowhead_overlaps(&automatic, &routed) {
                errors.push(problem(
                    context,
                    "flows",
                    hit.label_relation_index as usize,
                    "composition/label-arrowhead",
                    "Flow label covers an arrowhead",
                    serde_json::to_value(hit).unwrap(),
                ));
            }
        }
        if !context.legacy() {
            for hit in composition::collect_label_canvas_overflow(
                &labels,
                &[context.view_box.width, context.view_box.height],
                0.5,
            ) {
                errors.push(problem(
                    context,
                    "flows",
                    hit.relation_index as usize,
                    "composition/label-canvas-containment",
                    "Flow label is outside the canvas",
                    serde_json::to_value(hit).unwrap(),
                ));
            }
        }
    }
    let right = measure::stage_x(list(&context.input.document["stages"]).len() - 1) + 84.0;
    if right > context.view_box.width - 24.0 {
        errors.push(failure(
            "layout/constraint",
            "/stages",
            "Stages exceed the viewBox width",
            json!({"requiredWidth":(right+24.0).ceil(),"actualWidth":context.view_box.width}),
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
