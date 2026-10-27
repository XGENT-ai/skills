use super::measure::Measurement;
use super::*;
use crate::diagnostic::Severity;
use crate::geometry::composition::*;
use crate::geometry::{anchor, rects_overlap_for_contract};

fn relation_index(relation: &crate::scene::Relation) -> usize {
    relation
        .authored
        .source_path
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
fn relation_diagnostic(
    code: &str,
    index: usize,
    scene: &CompiledDiagram,
    message: &str,
    evidence: Value,
) -> Diagnostic {
    let mut diagnostic = failure(code, &format!("/edges/{index}"), message, evidence);
    if let Some(relation) = scene
        .relations
        .iter()
        .find(|relation| relation_index(relation) == index)
    {
        diagnostic.subject.identity = Some(relation.authored.id.clone());
    }
    diagnostic
}

pub(super) fn validate(
    input: &DiagramInput,
    measured: &Measurement,
    scene: &mut CompiledDiagram,
) -> Result<(), Diagnostics> {
    let showcase = input.quality == crate::input::Quality::Showcase;
    let phoenix = input.input_contract == InputContract::PhoenixV1;
    let readable = scene.schema_version == 2;
    let overlap =
        |a: &Rect, b: &Rect, gap: f64| rects_overlap_for_contract(a, b, gap, input.input_contract);
    let mut diagnostics = measured.layout_diagnostics.clone();
    let routed = scene
        .relations
        .iter()
        .map(|relation| {
            let mut facts = relation.authored.facts.clone();
            if relation.crossover_halo {
                facts["verifiedHalo"] = Value::Bool(true);
            }
            RoutedRelation::new(relation_index(relation), facts, relation.points.clone())
        })
        .collect::<Vec<_>>();
    let labels = scene
        .relations
        .iter()
        .filter_map(|relation| {
            relation.label_plate.map(|plate| {
                LabelRect::new(
                    relation_index(relation) as isize,
                    relation.authored.facts.clone(),
                    plate,
                )
            })
        })
        .collect::<Vec<_>>();
    let obstacles = scene
        .nodes
        .iter()
        .map(|node| Obstacle {
            id: node.authored.id.clone(),
            bounds: node.bounds,
        })
        .collect::<Vec<_>>();
    for hit in collect_route_obstacle_hits(&routed, &obstacles, 2.0) {
        diagnostics.push(relation_diagnostic(
            "clean-flow/edge-through-node",
            hit.relation_index,
            scene,
            "Relationship crosses an unrelated opaque node",
            serde_json::to_value(hit).unwrap(),
        ));
    }
    for relation in &scene.relations {
        let index = relation_index(relation);
        let from = scene
            .nodes
            .iter()
            .find(|node| node.authored.id == relation.from)
            .unwrap();
        let to = scene
            .nodes
            .iter()
            .find(|node| node.authored.id == relation.to)
            .unwrap();
        let side_at = |point: Point, node: &crate::scene::Node| {
            ["left", "right", "top", "bottom"]
                .into_iter()
                .find(|side| {
                    let test = anchor(&node.bounds, side);
                    if ["left", "right"].contains(side) {
                        (test.x - point.x).abs() <= 0.0001
                    } else {
                        (test.y - point.y).abs() <= 0.0001
                    }
                })
                .unwrap_or("right")
        };
        let authored_from = string(&relation.authored.facts, "fromSide");
        let authored_to = string(&relation.authored.facts, "toSide");
        let from_side = if authored_from.is_empty() {
            side_at(relation.points[0], from)
        } else {
            authored_from
        };
        let to_side = if authored_to.is_empty() {
            side_at(*relation.points.last().unwrap(), to)
        } else {
            authored_to
        };
        for issue in collect_endpoint_side_issues(&relation.points, from_side, to_side) {
            diagnostics.push(relation_diagnostic(
                "clean-flow/endpoint-side-direction",
                index,
                scene,
                "Route endpoints must cross node borders perpendicularly",
                serde_json::to_value(issue).unwrap(),
            ));
        }
        if relation.points.len() == 2
            && (relation.points[1].x - relation.points[0].x)
                .hypot(relation.points[1].y - relation.points[0].y)
                < 28.0
        {
            diagnostics.push(relation_diagnostic(
                "layout/constraint",
                index,
                scene,
                "Direct edge is shorter than 28px",
                json!({"minimumPx":28}),
            ));
        }
        if let Some(plate) = relation.label_plate {
            for node in &scene.nodes {
                if overlap(&plate, &node.bounds, -2.0) {
                    diagnostics.push(relation_diagnostic(
                        if readable && relation.authored.facts.get("labelAt").is_some() {
                            "workflow/explicit-pin-conflict"
                        } else {
                            "layout/constraint"
                        },
                        index,
                        scene,
                        "Relationship label overlaps a node",
                        json!({"labelRect":plate,"node":node.authored.id}),
                    ));
                }
            }
        }
    }
    for (index, left) in labels.iter().enumerate() {
        for right in &labels[index + 1..] {
            if overlap(&left.bounds, &right.bounds, -2.0) {
                diagnostics.push(relation_diagnostic("layout/constraint",left.relation_index as usize,scene,"Relationship labels overlap",json!({"labelRect":left.bounds,"otherLabelRect":right.bounds,"otherRelationship":{"index":right.relation_index}})));
            }
        }
    }
    let frames = scene
        .containers
        .iter()
        .filter(|container| container.authored.kind != "phase")
        .map(|container| {
            let mut frame = Frame::new(
                &container.authored.kind,
                &container.authored.id,
                container.bounds,
            );
            frame.radius = if container.authored.kind == "lane" {
                10.0
            } else if container.authored.kind == "group" {
                9.0
            } else {
                8.0
            };
            frame
        })
        .collect::<Vec<_>>();
    for hit in collect_border_runs(&routed, &frames) {
        diagnostics.push(relation_diagnostic("composition/container-border-run",hit.relation_index,scene,"Relationship follows a structural frame border",json!({"frameIndex":hit.frame_index,"side":hit.side,"segmentIndex":hit.segment_index,"overlapLengthPx":hit.overlap_length})));
    }
    if showcase || phoenix && readable {
        for hit in collect_proper_crossings(
            &routed,
            &CrossingOptions {
                contract: input.input_contract,
                merge_forward_collinear_waypoints: readable,
                include_shared_endpoints: Some(&|_, _| phoenix && readable),
                // Same rule as the artifact check: a crossing is drawn
                // legibly when both routes carry a crossover halo.
                crossing_resolved: Some(&|left, right, _| {
                    left["verifiedHalo"] == true && right["verifiedHalo"] == true
                }),
            },
        ) {
            let mut diagnostic = relation_diagnostic(
                "composition/proper-crossing",
                hit.left_relation_index,
                scene,
                "Unrelated relationship interiors cross",
                json!({"relationIndex":hit.left_relation_index,"otherRelationship":{"index":hit.right_relation_index},"point":hit.point,"leftSegment":hit.left_segment,"rightSegment":hit.right_segment}),
            );
            if !showcase {
                diagnostic.severity = Severity::Warning;
            }
            diagnostics.push(diagnostic);
        }
        for hit in collect_ambiguous_corridors(
            &routed,
            &CorridorOptions {
                contract: input.input_contract,
                include_shared_endpoints: Some(&|_, _| phoenix && readable),
                allow_short_workflow_trunks: phoenix && readable,
                ..Default::default()
            },
        ) {
            let mut diagnostic = relation_diagnostic(
                "composition/ambiguous-corridor",
                hit.left_relation_index,
                scene,
                "Relationships share an ambiguous corridor",
                json!({"relationIndex":hit.left_relation_index,"otherRelationship":{"index":hit.right_relation_index},"overlapLengthPx":hit.overlap_length,"segmentIndex":hit.left_segment,"otherSegmentIndex":hit.right_segment}),
            );
            if !showcase {
                diagnostic.severity = Severity::Warning;
            }
            diagnostics.push(diagnostic);
        }
    }
    if phoenix && readable {
        let arrows = routed
            .iter()
            .map(|relation| {
                let mut relation = relation.clone();
                if relation.relation.get("width").is_none() {
                    relation.relation["width"] =
                        json!(if string(&relation.relation, "variant") == "emphasis" {
                            1.8
                        } else {
                            1.4
                        });
                }
                relation
            })
            .collect::<Vec<_>>();
        for hit in collect_arrowhead_collisions(&arrows, true) {
            let mut diagnostic = relation_diagnostic(
                "composition/arrowhead-collision",
                hit.left_relation_index,
                scene,
                "Arrowhead footprints collide at their destinations",
                json!({"otherRelationship":{"index":hit.right_relation_index},"distancePx":hit.distance,"minimumPx":hit.minimum}),
            );
            if !showcase {
                diagnostic.severity = Severity::Warning;
            }
            diagnostics.push(diagnostic);
        }
    }
    if showcase {
        for hit in collect_route_rhythm_issues(&routed, 16.0, 8.0) {
            diagnostics.push(relation_diagnostic(
                hit.code,
                hit.relation_index,
                scene,
                "Route segment is below the readable rhythm floor",
                serde_json::to_value(&hit).unwrap(),
            ));
        }
        for hit in collect_label_route_clearance(&labels, &routed, 4.0) {
            diagnostics.push(relation_diagnostic(
                "composition/label-route-clearance",
                hit.label_relation_index as usize,
                scene,
                "Relationship label is too close to another route",
                serde_json::to_value(&hit).unwrap(),
            ));
        }
        if phoenix && readable {
            // Author-pinned label positions are kept as authored.
            let automatic = scene
                .relations
                .iter()
                .filter(|relation| {
                    !["labelAt", "labelDx", "labelDy", "labelSegment"]
                        .iter()
                        .any(|key| relation.authored.facts.get(*key).is_some())
                })
                .filter_map(|relation| {
                    relation.label_plate.map(|plate| {
                        LabelRect::new(
                            relation_index(relation) as isize,
                            relation.authored.facts.clone(),
                            plate,
                        )
                    })
                })
                .collect::<Vec<_>>();
            let arrows = scene
                .relations
                .iter()
                .map(|relation| {
                    let mut facts = relation.authored.facts.clone();
                    facts["variant"] = Value::String(relation.variant.clone());
                    RoutedRelation::new(relation_index(relation), facts, relation.points.clone())
                })
                .collect::<Vec<_>>();
            for hit in collect_label_arrowhead_overlaps(&automatic, &arrows) {
                diagnostics.push(relation_diagnostic(
                    "composition/label-arrowhead",
                    hit.label_relation_index as usize,
                    scene,
                    "Relationship label covers an arrowhead",
                    serde_json::to_value(&hit).unwrap(),
                ));
            }
        }
        if phoenix && !readable {
            for hit in collect_label_canvas_overflow(
                &labels,
                &[scene.view_box.width, scene.view_box.height],
                0.5,
            ) {
                diagnostics.push(relation_diagnostic(
                    "composition/label-canvas-containment",
                    hit.relation_index as usize,
                    scene,
                    "Relationship label extends beyond the fixed canvas",
                    serde_json::to_value(&hit).unwrap(),
                ));
            }
        }
    }
    if !readable {
        let minimum_height =
            measured.lane_tops.last().unwrap() + measured.lane_heights.last().unwrap() + 62.0;
        if scene.view_box.width < 696.0 || scene.view_box.height < minimum_height {
            diagnostics.push(failure(
                "layout/constraint",
                "/meta/viewBox",
                "Fixed-v1 canvas clips lanes or legend reserve",
                json!({"minimumWidthPx":696,"minimumHeightPx":minimum_height}),
            ));
        }
    }
    let mut hidden_legend = false;
    for legend in &scene.legends {
        if scene.relations.iter().any(|relation| {
            relation.points.windows(2).any(|pair| {
                crate::geometry::segment_intersects_rect(pair[0], pair[1], &legend.bounds, 0.0)
            }) || relation
                .label_plate
                .is_some_and(|plate| overlap(&plate, &legend.bounds, 0.0))
        }) {
            if input.document.pointer("/meta/legend").is_some() {
                diagnostics.push(failure(
                    "legend/content-overlap",
                    "/meta/legend",
                    "Legend overlaps authored relationship geometry",
                    json!({"legendRect":legend.bounds}),
                ));
            } else {
                hidden_legend = true;
            }
        }
    }
    if hidden_legend {
        scene.legends.clear();
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        for diagnostic in &mut diagnostics {
            diagnostic.evidence["measuredScene"] =
                serde_json::to_value(&scene).expect("measured workflow JSON");
        }
        return Err(diagnostics);
    }
    scene.diagnostics.extend(diagnostics);
    Ok(())
}
