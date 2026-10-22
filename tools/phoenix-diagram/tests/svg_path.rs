pub use phoenix_diagram::{geometry, scene};
#[path = "../src/check/path.rs"]
mod path;

use path::{analyze_svg_path, PathAnalysis, PathError, Segment};
use scene::{Point, Rect};
use serde_json::{json, Value};

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn point_json(point: &Point) -> Value {
    json!([point.x, point.y])
}
fn segment_json(segment: &Segment) -> Value {
    json!({"start": point_json(&segment.start), "end": point_json(&segment.end)})
}

// JSON has one number type; serde_json distinguishes integer/floating storage.
fn normalize_numbers(value: &mut Value) {
    match value {
        Value::Number(number) => *value = json!(number.as_f64().unwrap()),
        Value::Array(items) => items.iter_mut().for_each(normalize_numbers),
        Value::Object(fields) => fields.values_mut().for_each(normalize_numbers),
        _ => (),
    }
}

fn result_json(result: &Result<PathAnalysis, PathError>) -> Value {
    match result {
        Ok(analysis) => json!({
            "ok": true,
            "subpaths": analysis.subpaths.iter().map(|subpath| json!({
                "points": subpath.points.iter().map(point_json).collect::<Vec<_>>(),
                "segments": subpath.segments.iter().map(segment_json).collect::<Vec<_>>(),
                "straightSegments": subpath.straight_segments.iter().map(segment_json).collect::<Vec<_>>(),
                "closed": subpath.closed,
            })).collect::<Vec<_>>(),
        }),
        Err(error) => {
            let mut evidence = json!({
                "code": error.code, "tokenOffset": error.token_offset, "reason": error.reason,
            });
            if let Some(command) = error.command {
                evidence["command"] = json!(command.to_string());
            }
            json!({"ok": false, "error": evidence})
        }
    }
}

#[test]
fn native_path_analysis_matches_the_frozen_source_oracle() {
    let mut oracle: Value =
        serde_json::from_str(include_str!("fixtures/svg-path/oracle.json")).unwrap();
    assert_eq!(oracle["format"], 1);
    normalize_numbers(&mut oracle);
    let cases = oracle["cases"].as_array().unwrap();
    assert!(cases.len() >= 90);
    for case in cases {
        let result = analyze_svg_path(case["d"].as_str().unwrap());
        let mut actual = result_json(&result);
        normalize_numbers(&mut actual);
        assert_eq!(actual, case["expected"], "{}: {}", case["name"], case["d"]);
        if let Ok(analysis) = result {
            let route_parts = analysis
                .route_subpaths()
                .iter()
                .map(|part| part.iter().map(point_json).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            assert_eq!(
                json!(route_parts),
                case["routeSubpaths"],
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn bounds_follow_actual_sampled_geometry_and_keep_subpaths_separate() {
    let analysis = analyze_svg_path("M 0 0 Q 10 10 20 5 M 100 100 L 110 100").unwrap();
    assert_eq!(analysis.subpaths.len(), 2);
    assert_eq!(
        analysis.subpaths[0].bounds,
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 6.640625
        })
    );
    assert_eq!(
        analysis.subpaths[1].bounds,
        Some(Rect {
            x: 100.0,
            y: 100.0,
            width: 10.0,
            height: 0.0
        })
    );
    assert_eq!(
        analysis.bounds,
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 110.0,
            height: 100.0
        })
    );
    assert_eq!(analysis.subpaths[0].segments.len(), 8);
    assert!(analysis.subpaths[0].straight_segments.is_empty());
    assert_eq!(
        analysis.subpaths[1].segments,
        vec![Segment {
            start: p(100.0, 100.0),
            end: p(110.0, 100.0)
        }]
    );
    assert_eq!(
        analysis.route_subpaths(),
        vec![vec![], vec![p(100.0, 100.0), p(110.0, 100.0)]]
    );
}

#[test]
fn route_geometry_does_not_bridge_curves_or_moveto_boundaries() {
    let analysis = analyze_svg_path("M 0 0 L 10 0 Q 20 0 20 10 L 20 20 M 20 20 L 30 20").unwrap();
    assert_eq!(
        analysis.route_subpaths(),
        vec![
            vec![p(0.0, 0.0), p(10.0, 0.0)],
            vec![p(20.0, 10.0), p(20.0, 20.0)],
            vec![p(20.0, 20.0), p(30.0, 20.0)],
        ]
    );
}

#[test]
fn malformed_and_unsupported_commands_return_no_partial_geometry() {
    let error = analyze_svg_path("M 0 0 L 10 10 C 1 2 3 4 5 6").unwrap_err();
    assert_eq!(error.code, "artifact/svg-path-unsupported");
    assert_eq!(error.token_offset, 14);
    assert_eq!(error.command, Some('C'));
    assert_eq!(
        error.to_string(),
        "path command is not supported by artifact analysis at d offset 14."
    );
    let error = analyze_svg_path("M 0 0 Q 1 2 3").unwrap_err();
    assert_eq!(error.code, "artifact/svg-path-malformed");
    assert_eq!(error.token_offset, 13);
    assert_eq!(error.command, Some('Q'));
}

#[test]
fn empty_paths_and_move_only_paths_have_distinct_bounds() {
    assert_eq!(analyze_svg_path("").unwrap().bounds, None);
    let analysis = analyze_svg_path("M -5 7").unwrap();
    assert_eq!(
        analysis.bounds,
        Some(Rect {
            x: -5.0,
            y: 7.0,
            width: 0.0,
            height: 0.0
        })
    );
    assert_eq!(analysis.route_subpaths(), vec![vec![]]);
}

#[test]
fn finite_coordinates_do_not_create_nonfinite_bounds() {
    let analysis = analyze_svg_path("M -1e308 0 L 1e308 0").unwrap();
    assert_eq!(analysis.subpaths[0].segments.len(), 1);
    assert_eq!(analysis.bounds, None);
    assert_eq!(analysis.subpaths[0].bounds, None);
}
