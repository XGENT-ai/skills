// Ported from the frozen Archify geometry, utils and text-fit modules; see NOTICE.md.
use crate::input::InputContract;
pub use crate::scene::{Point, Rect};
use serde_json::Value;

pub mod composition;
pub mod labels;
pub mod route_quality;
pub mod text;

pub const EPSILON: f64 = 0.0001;

pub fn text_units(text: &str) -> f64 {
    text_units_for_contract(text, InputContract::PhoenixV1)
}

pub fn text_units_for_contract(text: &str, contract: InputContract) -> f64 {
    let mut characters = text.chars().peekable();
    let mut units = 0.0;
    while let Some(character) = characters.next() {
        if ('\u{fe00}'..='\u{fe0f}').contains(&character) {
            continue;
        }
        units += if characters.peek() == Some(&'\u{fe0f}') {
            2.0
        } else if contract == InputContract::Legacy217 && characters.peek() == Some(&'\u{fe0e}') {
            1.0
        } else if fullwidth(character) {
            2.0
        } else {
            1.0
        };
    }
    units
}

fn fullwidth(character: char) -> bool {
    matches!(character as u32,
        0x1100..=0x115f | 0x231a..=0x231b | 0x2329..=0x232a | 0x23e9..=0x23ec |
        0x23f0 | 0x23f3 | 0x25fd..=0x25fe | 0x2614..=0x2615 | 0x2630..=0x2637 |
        0x2648..=0x2653 | 0x267f | 0x268a..=0x268f | 0x2693 | 0x26a1 |
        0x26aa..=0x26ab | 0x26bd..=0x26be | 0x26c4..=0x26c5 | 0x26ce | 0x26d4 |
        0x26ea | 0x26f2..=0x26f3 | 0x26f5 | 0x26fa | 0x26fd | 0x2705 |
        0x270a..=0x270b | 0x2728 | 0x274c | 0x274e | 0x2753..=0x2755 | 0x2757 |
        0x2795..=0x2797 | 0x27b0 | 0x27bf | 0x2b1b..=0x2b1c | 0x2b50 | 0x2b55 |
        0x2e80..=0xa4cf | 0xa960..=0xa97c | 0xac00..=0xd7a3 | 0xf900..=0xfaff |
        0xfe10..=0xfe19 | 0xfe30..=0xfe6f | 0xff01..=0xff60 | 0xffe0..=0xffe6 |
        0x16fe0..=0x18dff | 0x1aff0..=0x1afff | 0x1b000..=0x1b2ff |
        0x1f000..=0x1faff | 0x20000..=0x3fffd
    )
}

pub fn fitted_node_font_size(text: &str, width: f64, preferred: f64, minimum: f64) -> f64 {
    fitted_node_font_size_for_contract(text, width, preferred, minimum, InputContract::PhoenixV1)
}

pub fn fitted_node_font_size_for_contract(
    text: &str,
    width: f64,
    preferred: f64,
    minimum: f64,
    contract: InputContract,
) -> f64 {
    let available = (width - 8.0).max(1.0);
    let fitted =
        preferred.min(available / (text_units_for_contract(text, contract).max(1.0) * 0.6));
    minimum.max((fitted * 10.0).floor() / 10.0)
}

pub fn minimum_node_text_width(text: &str, minimum: f64) -> f64 {
    text_units(text) * minimum * 0.6
}

pub fn finite_rect(rect: &Rect) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|value| value.is_finite())
}

pub fn finite_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

pub fn rects_overlap(a: &Rect, b: &Rect, gap: f64) -> bool {
    rects_overlap_for_contract(a, b, gap, InputContract::PhoenixV1)
}

pub fn rects_overlap_for_contract(a: &Rect, b: &Rect, gap: f64, contract: InputContract) -> bool {
    if !finite_rect(a) || !finite_rect(b) {
        return false;
    }
    let tolerance = if contract == InputContract::Legacy217 {
        0.0
    } else {
        EPSILON
    };
    !(a.x + a.width + gap <= b.x + tolerance
        || b.x + b.width + gap <= a.x + tolerance
        || a.y + a.height + gap <= b.y + tolerance
        || b.y + b.height + gap <= a.y + tolerance)
}

pub fn point_in_rect(point: Point, rect: &Rect) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn orientation(a: Point, b: Point, c: Point) -> u8 {
    let value = (b.y - a.y) * (c.x - b.x) - (b.x - a.x) * (c.y - b.y);
    if value.abs() < EPSILON {
        0
    } else if value > 0.0 {
        1
    } else {
        2
    }
}

fn on_segment(a: Point, b: Point, c: Point) -> bool {
    b.x <= a.x.max(c.x) && b.x >= a.x.min(c.x) && b.y <= a.y.max(c.y) && b.y >= a.y.min(c.y)
}

pub fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (o1, o2, o3, o4) = (
        orientation(a, b, c),
        orientation(a, b, d),
        orientation(c, d, a),
        orientation(c, d, b),
    );
    (o1 == 0 && on_segment(a, c, b))
        || (o2 == 0 && on_segment(a, d, b))
        || (o3 == 0 && on_segment(c, a, d))
        || (o4 == 0 && on_segment(c, b, d))
        || (o1 != o2 && o3 != o4)
}

pub fn segment_intersects_rect(start: Point, end: Point, rect: &Rect, gap: f64) -> bool {
    if !finite_point(start) || !finite_point(end) || !finite_rect(rect) {
        return false;
    }
    let expanded = Rect {
        x: rect.x - gap,
        y: rect.y - gap,
        width: rect.width + gap * 2.0,
        height: rect.height + gap * 2.0,
    };
    if point_in_rect(start, &expanded) || point_in_rect(end, &expanded) {
        return true;
    }
    let corners = [
        Point {
            x: expanded.x,
            y: expanded.y,
        },
        Point {
            x: expanded.x + expanded.width,
            y: expanded.y,
        },
        Point {
            x: expanded.x + expanded.width,
            y: expanded.y + expanded.height,
        },
        Point {
            x: expanded.x,
            y: expanded.y + expanded.height,
        },
    ];
    (0..4).any(|index| segments_intersect(start, end, corners[index], corners[(index + 1) % 4]))
}

pub fn point_segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let squared = dx * dx + dy * dy;
    if squared <= 0.0000001 {
        return (point.x - start.x).hypot(point.y - start.y);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / squared).clamp(0.0, 1.0);
    (point.x - (start.x + t * dx)).hypot(point.y - (start.y + t * dy))
}

fn point_rect_distance(point: Point, rect: &Rect) -> f64 {
    let dx = (rect.x - point.x)
        .max(0.0)
        .max(point.x - (rect.x + rect.width));
    let dy = (rect.y - point.y)
        .max(0.0)
        .max(point.y - (rect.y + rect.height));
    dx.hypot(dy)
}

pub fn segment_rect_clearance(start: Point, end: Point, rect: &Rect) -> f64 {
    if !finite_point(start)
        || !finite_point(end)
        || !finite_rect(rect)
        || rect.width < 0.0
        || rect.height < 0.0
    {
        return f64::NAN;
    }
    if segment_intersects_rect(start, end, rect, 0.0) {
        return 0.0;
    }
    let mut clearance = point_rect_distance(start, rect).min(point_rect_distance(end, rect));
    for point in [
        Point {
            x: rect.x,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
        Point {
            x: rect.x,
            y: rect.y + rect.height,
        },
    ] {
        clearance = clearance.min(point_segment_distance(point, start, end));
    }
    clearance
}

pub fn polyline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .sum()
}

pub fn cross_product(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

pub fn normalize_route_points(points: &[Point]) -> Vec<Point> {
    let mut normalized: Vec<Point> = Vec::new();
    for &point in points.iter().filter(|point| finite_point(**point)) {
        if normalized.last().is_some_and(|previous| {
            (point.x - previous.x).abs() <= EPSILON && (point.y - previous.y).abs() <= EPSILON
        }) {
            continue;
        }
        while normalized.len() >= 2 {
            let a = normalized[normalized.len() - 2];
            let b = normalized[normalized.len() - 1];
            let collinear_forward = cross_product(a, b, point).abs() <= EPSILON
                && (b.x - a.x) * (point.x - b.x) + (b.y - a.y) * (point.y - b.y) >= -EPSILON;
            if !collinear_forward {
                break;
            }
            normalized.pop();
        }
        normalized.push(point);
    }
    normalized
}

pub fn proper_segment_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let opposite = |left: f64, right: f64| {
        (left > EPSILON && right < -EPSILON) || (left < -EPSILON && right > EPSILON)
    };
    if !opposite(cross_product(a, b, c), cross_product(a, b, d))
        || !opposite(cross_product(c, d, a), cross_product(c, d, b))
    {
        return None;
    }
    let denominator = (a.x - b.x) * (c.y - d.y) - (a.y - b.y) * (c.x - d.x);
    if denominator.abs() < EPSILON {
        return None;
    }
    let ab = a.x * b.y - a.y * b.x;
    let cd = c.x * d.y - c.y * d.x;
    Some(Point {
        x: (ab * (c.x - d.x) - (a.x - b.x) * cd) / denominator,
        y: (ab * (c.y - d.y) - (a.y - b.y) * cd) / denominator,
    })
}

pub fn anchor(rect: &Rect, side: &str) -> Point {
    match side {
        "left" => Point {
            x: rect.x,
            y: rect.y + rect.height / 2.0,
        },
        "top" => Point {
            x: rect.x + rect.width / 2.0,
            y: rect.y,
        },
        "bottom" => Point {
            x: rect.x + rect.width / 2.0,
            y: rect.y + rect.height,
        },
        _ => Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height / 2.0,
        },
    }
}

pub fn label_point(item: &Value, points: &[Point]) -> Point {
    if let Some(pair) = item["labelAt"].as_array() {
        if let (Some(x), Some(y)) = (
            pair.first().and_then(Value::as_f64),
            pair.get(1).and_then(Value::as_f64),
        ) {
            return Point { x, y };
        }
    }
    assert!(
        points.len() >= 2,
        "A resolved relation must have two endpoints"
    );
    let dx = item["labelDx"].as_f64().unwrap_or(0.0);
    let dy = item["labelDy"].as_f64().unwrap_or(0.0);
    if points.len() == 2 {
        return Point {
            x: (points[0].x + points[1].x) / 2.0 + dx,
            y: points[0].y - 10.0 + dy,
        };
    }
    let index = item["labelSegment"]
        .as_f64()
        .unwrap_or(1.0)
        .max(0.0)
        .min((points.len() - 2) as f64) as usize;
    Point {
        x: (points[index].x + points[index + 1].x) / 2.0 + dx,
        y: (points[index].y + points[index + 1].y) / 2.0 - 10.0 + dy,
    }
}

pub fn format_number(value: f64) -> String {
    let formatted = format!("{value:.6}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".into()
    } else {
        trimmed.into()
    }
}

pub fn polyline_path(points: &[Point]) -> String {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            format!(
                "{} {} {}",
                if index == 0 { "M" } else { "L" },
                format_number(point.x),
                format_number(point.y)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn rounded_path(points: &[Point], radius: f64) -> String {
    if points.len() < 3 || radius <= 0.0 {
        return polyline_path(points);
    }
    let mut commands = vec![format!(
        "M {} {}",
        format_number(points[0].x),
        format_number(points[0].y)
    )];
    for triple in points.windows(3) {
        let (previous, current, next) = (triple[0], triple[1], triple[2]);
        let previous_length = (current.x - previous.x).hypot(current.y - previous.y);
        let next_length = (next.x - current.x).hypot(next.y - current.y);
        let r = radius.min(previous_length / 2.0).min(next_length / 2.0);
        if r < 1.0 {
            commands.push(format!(
                "L {} {}",
                format_number(current.x),
                format_number(current.y)
            ));
            continue;
        }
        let before = Point {
            x: current.x - (current.x - previous.x) / previous_length * r,
            y: current.y - (current.y - previous.y) / previous_length * r,
        };
        let after = Point {
            x: current.x + (next.x - current.x) / next_length * r,
            y: current.y + (next.y - current.y) / next_length * r,
        };
        commands.push(format!(
            "L {} {}",
            format_number(before.x),
            format_number(before.y)
        ));
        commands.push(format!(
            "Q {} {} {} {}",
            format_number(current.x),
            format_number(current.y),
            format_number(after.x),
            format_number(after.y)
        ));
    }
    let end = points.last().expect("at least three points");
    commands.push(format!(
        "L {} {}",
        format_number(end.x),
        format_number(end.y)
    ));
    commands.join(" ")
}

pub fn workflow_v2_path(points: &[Point]) -> String {
    let mut radius: f64 = 8.0;
    for (i, pair) in points.windows(2).enumerate() {
        let corners = usize::from(i > 0) + usize::from(i + 2 < points.len());
        if corners > 0 {
            radius = radius.min(
                (((pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y) - 8.0) / corners as f64)
                    .max(0.0),
            );
        }
    }
    rounded_path(points, radius)
}

pub fn lifecycle_v3_path(points: &[Point]) -> String {
    let mut radius: f64 = 12.0;
    let lengths: Vec<_> = points
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .collect();
    let trims: Vec<_> = lengths
        .windows(2)
        .map(|pair| {
            let trim = radius.min(pair[0] / 2.0).min(pair[1] / 2.0);
            if trim < 1.0 {
                0.0
            } else {
                trim
            }
        })
        .collect();
    let has_micro_run = lengths.iter().enumerate().any(|(index, length)| {
        let before = if index > 0 { trims[index - 1] } else { 0.0 };
        let after = trims.get(index).copied().unwrap_or(0.0);
        let remaining = length - before - after;
        remaining > EPSILON && remaining < 8.0 - EPSILON
    });
    if has_micro_run {
        // Keep valid rounded paths unchanged and retain a visible run at each turn.
        for (index, length) in lengths.iter().enumerate() {
            let corners = usize::from(index > 0) + usize::from(index + 1 < lengths.len());
            if corners > 0 {
                radius = radius.min(((length - 8.0) / corners as f64).max(0.0));
            }
        }
    }
    rounded_path(points, radius)
}
