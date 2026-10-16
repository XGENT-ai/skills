use labels::{Label, Options, Route};
use phoenix_diagram::geometry::{labels, route_quality, Point, Rect};
use route_quality::{DetourOptions, DetourPolicy, DetourRelation, GridRouteOptions, Segment};
use serde_json::{json, Value};
use std::collections::HashSet;

fn corpus() -> Value {
    serde_json::from_str(include_str!("fixtures/geometry-quality/oracle.json")).unwrap()
}
fn point(value: &Value) -> Point {
    Point {
        x: value[0].as_f64().unwrap(),
        y: value[1].as_f64().unwrap(),
    }
}
fn points(value: &Value) -> Vec<Point> {
    value.as_array().unwrap().iter().map(point).collect()
}
fn rect(value: &Value) -> Rect {
    serde_json::from_value(value.clone()).unwrap()
}
fn rects(value: &Value) -> Vec<Rect> {
    value.as_array().unwrap().iter().map(rect).collect()
}
fn label(value: &Value) -> Label {
    Label {
        relation_index: value["relationIndex"].as_i64().unwrap() as isize,
        bounds: rect(value),
        position: Point {
            x: value["lx"].as_f64().unwrap(),
            y: value["ly"].as_f64().unwrap(),
        },
        pinned: ["labelAt", "labelDx", "labelDy", "labelSegment"]
            .iter()
            .any(|field| value["relation"].get(field).is_some()),
    }
}
fn routes(value: &Value) -> Vec<Route> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| Route {
            relation_index: entry["relationIndex"].as_i64().unwrap() as isize,
            points: points(&entry["points"]),
        })
        .collect()
}
fn segments(value: &Value) -> Vec<Segment> {
    value
        .as_array()
        .map(|values| {
            values
                .iter()
                .map(|entry| Segment {
                    start: point(&entry["start"]),
                    end: point(&entry["end"]),
                })
                .collect()
        })
        .unwrap_or_default()
}
fn number(value: &Value, key: &str, fallback: f64) -> f64 {
    value[key].as_f64().unwrap_or(fallback)
}
fn flag(value: &Value, key: &str, fallback: bool) -> bool {
    value[key].as_bool().unwrap_or(fallback)
}
fn point_json(values: &[Point]) -> Value {
    json!(values.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>())
}
fn assert_json_same(actual: &Value, expected: &Value, context: &str) {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => {
            assert_eq!(left.as_f64(), right.as_f64(), "{context}")
        }
        (Value::Array(left), Value::Array(right)) => {
            assert_eq!(left.len(), right.len(), "{context}");
            for (index, (a, b)) in left.iter().zip(right).enumerate() {
                assert_json_same(a, b, &format!("{context}[{index}]"));
            }
        }
        (Value::Object(left), Value::Object(right)) => {
            assert_eq!(left.len(), right.len(), "{context}");
            for (key, value) in left {
                assert!(right.contains_key(key), "{context}: missing key {key}");
                assert_json_same(value, &right[key], &format!("{context}.{key}"));
            }
        }
        _ => assert_eq!(actual, expected, "{context}"),
    }
}

#[test]
fn automatic_labels_match_fixed_source_oracle_including_pins_retries_and_grid_ranking() {
    for case in corpus()["labelCases"].as_array().unwrap() {
        let source = &case["input"];
        let original: Vec<_> = source["labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(label)
            .collect();
        let original_routes = routes(&source["routes"]);
        let components = rects(&source["components"]);
        let titles = rects(&source["titles"]);
        let view_box = Rect {
            x: 0.0,
            y: 0.0,
            width: source["viewBox"][0].as_f64().unwrap(),
            height: source["viewBox"][1].as_f64().unwrap(),
        };
        let mut options = Options::new(view_box);
        options.placement_bottom = number(source, "placementBottom", view_box.height);
        options.fallback_ring = flag(source, "fallbackRing", true);
        options.keep_fallback_near_route = flag(source, "keepFallbackNearRoute", false);
        options.grid_sweep = flag(source, "gridSweep", false);
        options.max_own_route_distance = number(source, "maxOwnRouteDistance", f64::INFINITY);
        let before = (
            original.clone(),
            original_routes.clone(),
            components.clone(),
            titles.clone(),
        );
        let actual = labels::place_automatic_labels(
            &original,
            &original_routes,
            &components,
            &titles,
            &options,
        );
        let expected: Vec<_> = case["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(label)
            .collect();
        assert_eq!(actual, expected, "{}", case["name"]);
        assert_eq!(
            before,
            (original, original_routes, components, titles),
            "{}: helper changed authored inputs",
            case["name"]
        );
    }
}

#[test]
fn reserved_label_rect_keeps_the_reference_owner_and_failure_rules() {
    for case in corpus()["reservationCases"].as_array().unwrap() {
        let source = &case["input"];
        let value = label(&source["label"]);
        let route_points = points(&source["points"]);
        let context_routes = routes(&source["routes"]);
        let context_labels: Vec<_> = source["labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(label)
            .collect();
        let components = rects(&source["components"]);
        let view_box = Rect {
            x: 0.0,
            y: 0.0,
            width: source["viewBox"][0].as_f64().unwrap(),
            height: source["viewBox"][1].as_f64().unwrap(),
        };
        let actual = labels::reserved_label_rect(
            &value,
            &route_points,
            &context_routes,
            &context_labels,
            &components,
            view_box,
            number(source, "placementBottom", f64::INFINITY),
        );
        let expected = (!case["expected"].is_null()).then(|| label(&case["expected"]));
        assert_eq!(actual, expected, "{}", case["name"]);
    }
}

#[test]
fn shortest_grid_routes_and_actual_search_metrics_match_the_fixed_source_oracle() {
    for case in corpus()["gridCases"].as_array().unwrap() {
        let source = &case["input"];
        let route_points = points(&source["points"]);
        let obstacles = rects(&source["obstacles"]);
        let avoided = segments(&source["avoidedSegments"]);
        let borders = segments(&source["borderSegments"]);
        let mut options = GridRouteOptions::new(
            point(&source["start"]),
            point(&source["end"]),
            &route_points,
            &obstacles,
            source["fromSide"].as_str().unwrap(),
            source["toSide"].as_str().unwrap(),
        );
        options.clearance = number(source, "clearance", 2.0);
        options.maximum_obstacle_count = number(source, "maximumObstacleCount", 80.0) as usize;
        options.endpoint_stub_px = source["endpointStubPx"].as_f64();
        options.maximum_grid_nodes = number(source, "maximumGridNodes", f64::INFINITY);
        options.avoided_segments = &avoided;
        options.allow_avoided_crossings = flag(source, "allowAvoidedCrossings", false);
        options.minimum_avoided_overlap_px = number(source, "minimumAvoidedOverlapPx", 8.0);
        options.route_separation_px = number(source, "routeSeparationPx", 8.0);
        options.minimum_segment_px = number(source, "minimumSegmentPx", 8.0);
        options.border_segments = &borders;
        options.bend_penalty_px = number(source, "bendPenaltyPx", 0.0);
        let mut accept_calls = 0;
        let search = route_quality::shortest_orthogonal_grid_route_accept(&options, |_| {
            accept_calls += 1;
            case["acceptMode"] == "all"
                || (case["acceptMode"] == "reject-first" && accept_calls > 1)
        });
        assert_eq!(
            accept_calls,
            case["acceptCalls"].as_u64().unwrap(),
            "{}: callback count",
            case["name"]
        );
        let actual = search.route.map(|route| json!({ "points": point_json(&route.points), "length": route.length, "obstacleCount": route.obstacle_count }));
        let expected = (!case["expected"].is_null()).then(|| case["expected"].clone());
        assert_json_same(
            &json!(actual),
            &json!(expected),
            &format!("{}: shortest route", case["name"]),
        );
        assert_json_same(
            &serde_json::to_value(&search.metrics).unwrap(),
            &case["metrics"],
            &format!(
                "{}: metrics must describe the executed search",
                case["name"]
            ),
        );
    }
}

#[test]
fn authored_detour_issues_match_the_source_evidence_and_shared_corridor_boundaries() {
    for case in corpus()["detourCases"].as_array().unwrap() {
        let source = &case["input"];
        let relations: Vec<_> = source["relations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|relation| DetourRelation {
                id: relation["id"].as_str().map(str::to_owned),
                from: relation["from"].as_str().unwrap().to_owned(),
                to: relation["to"].as_str().unwrap().to_owned(),
                authored_via: relation["via"]
                    .as_array()
                    .is_some_and(|via| !via.is_empty()),
                points: points(&relation["points"]),
                from_side: relation["fromSide"].as_str().map(str::to_owned),
                to_side: relation["toSide"].as_str().map(str::to_owned),
            })
            .collect();
        let obstacles = rects(&source["obstacles"]);
        let content_rects = rects(&source["contentRects"]);
        let endpoint_ids: HashSet<_> = source["endpointIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        let thresholds = &source["thresholds"];
        let mut policy = DetourPolicy::default();
        policy.clearance = number(thresholds, "clearance", policy.clearance);
        policy.minimum_detour_ratio = number(
            thresholds,
            "minimumDetourRatio",
            policy.minimum_detour_ratio,
        );
        policy.minimum_excess_length_px = number(
            thresholds,
            "minimumExcessLengthPx",
            policy.minimum_excess_length_px,
        );
        policy.minimum_empty_excursion_px = number(
            thresholds,
            "minimumEmptyExcursionPx",
            policy.minimum_empty_excursion_px,
        );
        policy.maximum_obstacle_count = number(
            thresholds,
            "maximumObstacleCount",
            policy.maximum_obstacle_count as f64,
        ) as usize;
        policy.shared_corridor_minimum_px = number(
            thresholds,
            "sharedCorridorMinimumPx",
            policy.shared_corridor_minimum_px,
        );
        let report = route_quality::clean_route_detour_problems(&DetourOptions {
            relations: &relations,
            obstacles: &obstacles,
            content_rects: &content_rects,
            endpoint_ids: &endpoint_ids,
            diagram_type: source["diagramType"].as_str().unwrap(),
            relation_collection: source["relationCollection"].as_str().unwrap(),
            profile: source["profile"].as_str().unwrap(),
            policy,
        });
        assert_eq!(report.scope, "showcase-authored-orthogonal-route-detours");
        for (index, evaluation) in report.evaluations.iter().enumerate() {
            assert_eq!(evaluation.relation_index, index);
        }
        let messages: Vec<_> = report
            .problems
            .iter()
            .map(|problem| &problem.message)
            .collect();
        assert_eq!(
            json!(messages),
            case["expected"]["messages"],
            "{}: issues",
            case["name"]
        );
        for (actual, expected) in report
            .problems
            .iter()
            .zip(case["expected"]["diagnostics"].as_array().unwrap())
        {
            assert_eq!(
                actual.code,
                expected["code"].as_str().unwrap(),
                "{}",
                case["name"]
            );
            assert_json_same(
                &actual.evidence,
                &expected["evidence"],
                &format!("{}: evidence", case["name"]),
            );
            assert_eq!(
                json!(actual.supported_fixes),
                expected["supportedFixes"],
                "{}",
                case["name"]
            );
            assert_eq!(
                actual.subject.path,
                format!(
                    "/connections/{}",
                    expected["subject"]["index"].as_u64().unwrap()
                ),
                "{}: source subject",
                case["name"]
            );
        }
        if case["name"] == "detour-obstacle-budget-leaves-evaluation-incomplete" {
            assert!(
                !report.evaluated,
                "a skipped shortest-route search cannot prove composition passed"
            );
            assert_eq!(report.evaluations[0].reason, "obstacle-budget-exceeded");
        } else if case["name"] == "explicit-unsupported-side-cannot-prove-detour-check" {
            assert!(!report.evaluated);
            assert_eq!(report.evaluations[0].reason, "unsupported-endpoint-side");
        } else if case["name"] == "standard-profile-does-not-run-showcase-detour-check" {
            assert!(!report.applicable);
            assert!(!report.evaluated);
        } else {
            assert!(
                report.evaluated,
                "{}: applicable predicates/search were actually evaluated",
                case["name"]
            );
        }
    }
}

#[test]
fn fixtures_lock_distinguishing_retry_sliding_grid_and_failure_examples() {
    let oracle = corpus();
    let lookup = |name: &str| {
        oracle["labelCases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap()
    };
    assert_eq!(
        lookup("later-label-vacates-earlier-label-only-slot")["expected"][0]["lx"],
        110
    );
    assert_eq!(
        lookup("sliding-finds-unsampled-obstacle-edge-slot")["expected"][0]["lx"],
        195
    );
    assert_eq!(
        lookup("grid-reciprocal-horizontal-low-neighbour")["expected"][0]["ly"],
        170
    );
    assert_eq!(
        lookup("grid-reciprocal-horizontal-high-neighbour")["expected"][0]["ly"],
        140
    );
    assert_eq!(
        lookup("no-feasible-position-keeps-original-failure")["expected"],
        lookup("no-feasible-position-keeps-original-failure")["input"]["labels"]
    );
}
