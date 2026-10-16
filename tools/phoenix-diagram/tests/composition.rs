use phoenix_diagram::geometry::{self, composition::*, Point};
use phoenix_diagram::input::InputContract;
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn point(value: &Value) -> Point {
    p(value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}
fn points(value: &Value) -> Vec<Point> {
    value.as_array().unwrap().iter().map(point).collect()
}
fn xy(point: Point) -> Value {
    json!([point.x, point.y])
}
fn rect(value: &Value) -> geometry::Rect {
    serde_json::from_value(value.clone()).unwrap()
}
fn numeric(value: &Value, key: &str, fallback: f64) -> f64 {
    value[key].as_f64().unwrap_or(fallback)
}

fn routed(value: &Value) -> Vec<RoutedRelation> {
    value
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let mut entry = RoutedRelation::new(
                        value["relationIndex"]
                            .as_u64()
                            .map_or(index, |v| v as usize),
                        value["relation"].clone(),
                        points(&value["points"]),
                    );
                    entry.source_endpoint = value["sourceEndpoint"] != false;
                    entry.target_endpoint = value["targetEndpoint"] != false;
                    entry.segments = value["segments"].as_array().map(|segments| {
                        segments
                            .iter()
                            .map(|s| Segment {
                                start: point(&s["start"]),
                                end: point(&s["end"]),
                            })
                            .collect()
                    });
                    entry
                })
                .collect()
        })
        .unwrap_or_default()
}

fn labels(value: &Value) -> Vec<LabelRect> {
    value
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    LabelRect::new(
                        value["relationIndex"]
                            .as_i64()
                            .map_or(index as isize, |v| v as isize),
                        value["relation"].clone(),
                        rect(&value["rect"]),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn result(test_case: &Value) -> Value {
    let input = &test_case["input"];
    let routes = routed(&input["routedRelations"]);
    let contract = if test_case["contract"] == "legacy-2.17" {
        InputContract::Legacy217
    } else {
        InputContract::PhoenixV1
    };
    let include = |_: &Value, _: &Value| input["includeSharedEndpoints"] == true;
    let counterflow = |_: &Value, _: &Value| input["includeSharedEndpointCounterflow"] == true;
    match test_case["operation"].as_str().unwrap() {
        "corridors" => json!(collect_ambiguous_corridors(&routes,&CorridorOptions {contract,
            min_overlap_px:numeric(input,"minOverlapPx",8.0),include_shared_endpoints:Some(&include),
            include_shared_endpoint_counterflow:Some(&counterflow),allow_short_workflow_trunks:input["allowShortWorkflowTrunks"]==true,
        }).iter().map(|h|json!({"left":h.left_relation_index,"right":h.right_relation_index,"leftSegment":h.left_segment,
            "rightSegment":h.right_segment,"overlapLength":h.overlap_length,"start":xy(h.overlap_start),"end":xy(h.overlap_end)})).collect::<Vec<_>>()),
        "border" => {
            let frames=input["frames"].as_array().unwrap().iter().map(Frame::from_value).collect::<Vec<_>>();
            json!(collect_border_runs(&routes,&frames).iter().map(|h|json!({"relation":h.relation_index,"frame":h.frame_index,"side":h.side,
                "segment":h.segment_index,"overlapLength":h.overlap_length,"start":xy(h.overlap_start),"end":xy(h.overlap_end)})).collect::<Vec<_>>())
        }
        "rhythm" => json!(collect_route_rhythm_issues(&routes,numeric(input,"interiorSegmentPx",16.0),numeric(input,"microSegmentPx",8.0))
            .iter().map(|h|json!({"code":h.code,"relation":h.relation_index,"segment":h.segment_index,"position":h.position,
                "length":h.length,"start":xy(h.start),"end":xy(h.end)})).collect::<Vec<_>>()),
        "budget" => serde_json::to_value(route_budget_metrics(&routes,&RouteBudgetOptions {
            bends_per_relationship:input["bendsPerRelationship"].as_u64().unwrap_or(2) as usize,stretch:numeric(input,"stretch",1.35),
            segment_px:numeric(input,"segmentPx",16.0),micro_segment_px:numeric(input,"microSegmentPx",8.0),
        })).unwrap(),
        "arrows" => json!(collect_arrowhead_collisions(&routes,input["allowShortWorkflowTrunks"]==true).iter()
            .map(|h|json!({"left":h.left_relation_index,"right":h.right_relation_index,"leftTip":xy(h.left_tip),"rightTip":xy(h.right_tip),
                "distance":h.distance,"minimum":h.minimum})).collect::<Vec<_>>()),
        "label-route" => json!(collect_label_route_clearance(&labels(&input["labels"]),&routes,numeric(input,"threshold",4.0)).iter()
            .map(|h|json!({"labelRelation":h.label_relation_index,"otherRelation":h.other_relation_index,"segment":h.segment_index,
                "clearance":h.clearance,"intersectionLength":h.intersection_length,"start":xy(h.start),"end":xy(h.end)})).collect::<Vec<_>>()),
        "canvas" => json!(collect_label_canvas_overflow(&labels(&input["labels"]),&input["viewBox"].as_array().unwrap().iter().map(|v|v.as_f64().unwrap()).collect::<Vec<_>>(),
            numeric(input,"tolerance",0.5)).iter().map(|h|json!({"relation":h.relation_index,"viewBox":h.view_box,"origin":h.view_box_origin,
                "sides":h.sides,"overflowPx":h.overflow_px.iter().map(|(s,n)|(s.to_string(),json!(n))).collect::<serde_json::Map<_,_>>()})).collect::<Vec<_>>()),
        "endpoint-sides" => json!(route_honors_endpoint_sides(&points(&input["points"]),input["fromSide"].as_str().unwrap(),input["toSide"].as_str().unwrap())),
        "bend-contacts" => json!(polyline_bend_contacts(&points(&input["left"]),&points(&input["right"])).into_iter().map(xy).collect::<Vec<_>>()),
        "crossings" => {
            let resolved=|_: &Value,_: &Value,_: &ProperCrossing|input["crossingResolved"]==true;
            json!(collect_proper_crossings(&routes,&CrossingOptions {contract,merge_forward_collinear_waypoints:input["mergeForwardCollinearWaypoints"]==true,
                include_shared_endpoints:Some(&include),crossing_resolved:Some(&resolved)}).iter().map(|h|json!({"left":h.left_relation_index,
                    "right":h.right_relation_index,"point":xy(h.point),"leftSegment":h.left_segment,"rightSegment":h.right_segment})).collect::<Vec<_>>())
        }
        "obstacles" => {
            let obstacles=input["obstacles"].as_array().unwrap().iter().map(|o|Obstacle {id:o["id"].as_str().unwrap().into(),bounds:rect(o)}).collect::<Vec<_>>();
            json!(collect_route_obstacle_hits(&routes,&obstacles,numeric(input,"clearance",2.0)).iter().map(|h|json!({"relation":h.relation_index,
                "obstacle":h.obstacle_id,"segment":h.segment_index})).collect::<Vec<_>>())
        }
        other => panic!("Unknown oracle operation {other}"),
    }
}

fn assert_same(actual: &Value, expected: &Value, context: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
                "{context}: actual {a}, source {b}"
            );
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(
                a.len(),
                b.len(),
                "{context}: actual {actual}, source {expected}"
            );
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                assert_same(a, b, &format!("{context}/{index}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "{context}: object fields differ");
            for (key, b) in b {
                assert_same(
                    a.get(key)
                        .unwrap_or_else(|| panic!("{context}: missing {key}")),
                    b,
                    &format!("{context}/{key}"),
                );
            }
        }
        _ => assert_eq!(actual, expected, "{context}"),
    }
}

fn compare_oracle(operation: &str) {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/composition/oracle.json")).unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    let mut count = 0;
    for test_case in cases.iter().filter(|c| c["operation"] == operation) {
        assert_same(
            &result(test_case),
            &test_case["expected"],
            &format!("{}/{}", test_case["contract"], test_case["name"]),
        );
        count += 1;
    }
    assert!(count > 0, "No oracle cases for {operation}");
}

#[test]
fn frozen_corridor_oracle() {
    compare_oracle("corridors");
}
#[test]
fn frozen_border_run_oracle() {
    compare_oracle("border");
}
#[test]
fn frozen_rhythm_oracle() {
    compare_oracle("rhythm");
}
#[test]
fn frozen_route_budget_oracle() {
    compare_oracle("budget");
}
#[test]
fn frozen_arrowhead_oracle() {
    compare_oracle("arrows");
}
#[test]
fn frozen_label_route_oracle() {
    compare_oracle("label-route");
}
#[test]
fn frozen_canvas_containment_oracle() {
    compare_oracle("canvas");
}
#[test]
fn frozen_endpoint_direction_oracle() {
    compare_oracle("endpoint-sides");
}
#[test]
fn frozen_proper_crossing_oracle() {
    compare_oracle("crossings");
}
#[test]
fn frozen_bend_contact_oracle() {
    compare_oracle("bend-contacts");
}
#[test]
fn frozen_opaque_obstacle_oracle() {
    compare_oracle("obstacles");
}

#[test]
fn oracle_source_hashes_match_both_pinned_manifests() {
    let legacy_bytes = include_bytes!("../upstream/legacy-2.17-manifest.json");
    let reference_bytes = include_bytes!("../upstream/archify-3.1.0-manifest.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(legacy_bytes)),
        "9b02b58082c1872f611fff050b225026e108ca53efa738da429965db9966d273"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(reference_bytes)),
        "4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395"
    );
    let legacy: Value = serde_json::from_slice(legacy_bytes).unwrap();
    let reference: Value = serde_json::from_slice(reference_bytes).unwrap();
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/composition/oracle.json")).unwrap();
    assert_eq!(oracle["format"], 1);
    for case in oracle["cases"].as_array().unwrap() {
        let manifest = if case["contract"] == "legacy-2.17" {
            &legacy
        } else {
            &reference
        };
        let row = manifest
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r[0].as_str()
                    .unwrap()
                    .ends_with("renderers/shared/geometry.mjs")
            })
            .unwrap();
        assert_eq!(case["sourceSha256"], row[2], "{}", case["source"]);
    }
}

#[test]
fn malformed_geometry_is_unknown_and_does_not_report_composition_pass() {
    let frame = Frame::new(
        "boundary",
        "box",
        geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 40.0,
        },
    );
    let route = RoutedRelation::new(
        0,
        json!({"from":"a","to":"b"}),
        vec![p(f64::NAN, 0.0), p(40.0, 0.0)],
    );
    assert!(collect_border_runs(&[route.clone()], &[frame]).is_empty());
    assert!(collect_proper_crossings(&[route], &Default::default()).is_empty());
    let labels = [LabelRect::new(
        7,
        json!({"id":"label"}),
        geometry::Rect {
            x: f64::NAN,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
    )];
    assert!(collect_label_canvas_overflow(&labels, &[100.0, 80.0], 0.5).is_empty());
    assert_eq!(minimum_label_route_clearance(&[]), None);
    assert_eq!(
        segment_rect_intersection_length(
            p(0.0, 0.0),
            p(40.0, 0.0),
            &geometry::Rect {
                x: 10.0,
                y: -2.0,
                width: -10.0,
                height: 10.0
            }
        ),
        None
    );
}

#[test]
fn endpoint_and_crossing_indices_name_authored_or_normalized_geometry_explicitly() {
    let issues = collect_endpoint_side_issues(
        &[p(0.0, 0.0), p(20.0, 0.0), p(40.0, 0.0), p(40.0, 40.0)],
        "right",
        "left",
    );
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].endpoint, "target");
    assert_eq!(issues[0].segment_index, 1);
    assert_eq!(issues[0].expected_axis, "horizontal");
    let segments = forward_collinear_analysis_segments(&[p(0.0, 0.0), p(20.0, 0.0), p(40.0, 0.0)]);
    assert_eq!(segments.len(), 1);
    assert_eq!(source_segment_index_at_point(&segments[0], p(20.0, 0.0)), 0);
    assert_eq!(source_segment_index_at_point(&segments[0], p(30.0, 0.0)), 1);
}

#[test]
fn workflow_short_trunk_exemption_preserves_style_and_contract_boundaries() {
    let routed = vec![
        RoutedRelation::new(
            0,
            json!({"id":"ab","from":"a","to":"b"}),
            vec![p(0.0, 0.0), p(24.0, 0.0), p(24.0, 40.0)],
        ),
        RoutedRelation::new(
            1,
            json!({"id":"ac","from":"a","to":"c"}),
            vec![p(0.0, 0.0), p(24.0, 0.0), p(24.0, -40.0)],
        ),
    ];
    let include = |_: &serde_json::Value, _: &serde_json::Value| true;
    let mut options = CorridorOptions {
        include_shared_endpoints: Some(&include),
        ..Default::default()
    };
    assert_eq!(collect_ambiguous_corridors(&routed, &options).len(), 1);
    options.allow_short_workflow_trunks = true;
    assert!(collect_ambiguous_corridors(&routed, &options).is_empty());
    let mut mismatched = routed.clone();
    mismatched[1].relation["width"] = json!(3);
    assert_eq!(collect_ambiguous_corridors(&mismatched, &options).len(), 1);
    options.contract = InputContract::Legacy217;
    assert!(collect_ambiguous_corridors(&mismatched, &options).is_empty());
}
