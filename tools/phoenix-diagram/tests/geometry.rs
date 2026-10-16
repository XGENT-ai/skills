use phoenix_diagram::geometry::*;
use phoenix_diagram::input::InputContract;
use serde_json::json;

fn p(x: f64, y: f64) -> Point { Point { x, y } }

#[test]
fn unicode_units_preserve_both_contracts_and_scalar_order() {
    assert_eq!(text_units("A中🦀"), 5.0);
    assert_eq!(text_units("ｶﾀｶﾅ"), 4.0);
    assert_eq!(text_units("⌚\u{fe0e}"), 2.0);
    assert_eq!(text_units_for_contract("⌚\u{fe0e}", InputContract::Legacy217), 1.0);
    assert_eq!(text_units("A\u{fe0f}"), 2.0);
    assert_eq!(text_units("\u{fe00}"), 0.0);
    assert_eq!(text_units("👨‍👩‍👧"), 8.0);
    assert_eq!(text_units("\u{a97c}\u{a97d}"), 3.0);
}

#[test]
fn text_fitting_respects_readable_floor_and_tenth_pixel_quantization() {
    assert_eq!(fitted_node_font_size("abcdefghij", 60.0, 12.0, 7.0), 8.6);
    assert_eq!(fitted_node_font_size("abcdefghijklmnopqrstuvwxyz", 60.0, 12.0, 7.0), 7.0);
    assert_eq!(fitted_node_font_size("short", 200.0, 12.0, 7.0), 12.0);
    assert_eq!(minimum_node_text_width("中文", 8.0), 19.2);
}

#[test]
fn overlap_keeps_exact_legacy_comparisons_and_reference_clearance_tolerance() {
    let a = Rect { x: 793.6, y: 0.0, width: 160.0, height: 30.0 };
    let b = Rect { x: 961.5999999999999, y: 0.0, width: 160.0, height: 30.0 };
    assert!(rects_overlap_for_contract(&a, &b, 8.0, InputContract::Legacy217));
    assert!(!rects_overlap(&a, &b, 8.0));
    assert!(rects_overlap(&a, &Rect { x: 960.0, ..b }, 8.0));
    assert!(!rects_overlap(&a, &Rect { x: f64::NAN, ..b }, 8.0));
}

#[test]
fn segment_collision_includes_borders_and_clearance_uses_diagonal_distance() {
    let rect = Rect { x: 10.0, y: 0.0, width: 2.0, height: 2.0 };
    assert!(segment_intersects_rect(p(0.0, 0.0), p(20.0, 0.0), &rect, 0.0));
    assert!(!segment_intersects_rect(p(0.0, 4.0), p(20.0, 4.0), &rect, 0.0));
    assert!(segment_intersects_rect(p(0.0, 4.0), p(20.0, 4.0), &rect, 2.0));
    assert_eq!(segment_rect_clearance(p(0.0, 4.0), p(20.0, 4.0), &rect), 2.0);
    assert!((segment_rect_clearance(p(0.0, 0.0), p(10.0, 10.0), &rect) - 32.0_f64.sqrt()).abs() < 1e-9);
}

#[test]
fn normalization_removes_duplicates_and_forward_collinearity_but_keeps_reversals() {
    assert_eq!(normalize_route_points(&[p(0.0,0.0), p(0.0,0.00001), p(0.0,10.0), p(0.0,20.0), p(0.0,10.0), p(10.0,10.0)]), vec![p(0.0,0.0),p(0.0,20.0),p(0.0,10.0),p(10.0,10.0)]);
    assert_eq!(polyline_length(&[p(0.0,0.0),p(3.0,4.0),p(6.0,8.0)]), 10.0);
    assert_eq!(rounded_path(&[p(0.0,0.0),p(10.0,0.0),p(10.0,10.0)],4.0), "M 0 0 L 6 0 Q 10 0 10 4 L 10 10");
}

#[test]
fn proper_crossings_exclude_shared_endpoints_and_collinear_corridors() {
    assert_eq!(proper_segment_intersection(p(0.0,0.0),p(10.0,10.0),p(0.0,10.0),p(10.0,0.0)),Some(p(5.0,5.0)));
    assert_eq!(proper_segment_intersection(p(0.0,0.0),p(10.0,0.0),p(10.0,0.0),p(10.0,10.0)),None);
    assert_eq!(proper_segment_intersection(p(0.0,0.0),p(10.0,0.0),p(5.0,0.0),p(15.0,0.0)),None);
}

#[test]
fn ports_and_explicit_labels_preserve_the_authored_coordinates() {
    let rect = Rect { x: 20.0,y: 30.0,width: 100.0,height: 60.0 };
    assert_eq!(anchor(&rect,"right"),p(120.0,60.0));
    assert_eq!(anchor(&rect,"left"),p(20.0,60.0));
    assert_eq!(anchor(&rect,"top"),p(70.0,30.0));
    assert_eq!(anchor(&rect,"bottom"),p(70.0,90.0));
    let points = [p(0.0,0.0),p(10.0,0.0),p(10.0,20.0)];
    assert_eq!(label_point(&json!({"labelAt":[7.0,9.0],"labelDx":100}),&points),p(7.0,9.0));
    assert_eq!(label_point(&json!({"labelSegment":1,"labelDx":2,"labelDy":3}),&points),p(12.0,3.0));
    assert_eq!(label_point(&json!({"labelSegment":0.0}),&points),p(5.0,-10.0));
}
