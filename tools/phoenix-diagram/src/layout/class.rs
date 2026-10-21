// Class and interface layout ported from the frozen renderers/class/render-class.mjs:
// measured types on the erd banded grid → hierarchy buses → routes → labels →
// canvas and legend.
use crate::diagnostic::Diagnostics;
use crate::geometry::composition::{
    collect_ambiguous_corridors, CorridorOptions, LabelRect, RoutedRelation,
};
use crate::geometry::{
    labels as shared_labels, proper_segment_intersection, rects_overlap, segment_intersects_rect,
    text_units,
};
use crate::input::{CompileOptions, DiagramInput, InputContract, Quality};
use crate::layout::erd::grid::{self, Cell, Grid};
use crate::layout::erd::quality::{diagnostic, Gate, Routed};
use crate::layout::erd::{self, js_round, legend, text};
use crate::layout::router::{self, Edge, Endpoint, RouterOptions, Side};
use crate::scene::{
    Authored, CompiledDiagram, DetailRow, DiagramFacts, Marker, Node, Point, Rect, Relation,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub(crate) mod members;

use members::MEMBER_ADVANCE;

const MARGIN: f64 = 24.0;
const LEGEND_H: f64 = 34.0;
const PAD_X: f64 = 12.0;
const HEADER_H: f64 = 44.0;
const HEADER_PLAIN_H: f64 = 34.0;
const NAME_FONT: f64 = 13.5;
const STEREOTYPE_FONT: f64 = 9.5;
const MEMBER_FONT: f64 = 11.0;
const ROW_H: f64 = 20.0;
const COMPARTMENT_PAD: f64 = 5.0;
const MIN_RELATIONSHIP_LENGTH: f64 = 28.0;
// The visibility chip is wider than the two-character prefix the member text is
// measured with; every box reserves the difference.
pub(crate) const VIS_CHIP: f64 = 14.0;
pub(crate) const VIS_GAP: f64 = 6.0;
const BADGE_SIZE: f64 = 16.0;
const BADGE_PORCH: f64 = BADGE_SIZE + 8.0;
// The tallest marker; ports on one side closer than this draw one shape.
const MARKER_HEIGHT: f64 = 16.0;
const PORT_SPACING: f64 = 26.0;
const BUS_MIN_GAP: f64 = 40.0;
const RELATIONSHIP_LABEL_FONT: f64 = 9.5;
const MARKER_REACH: f64 = 20.0;
const MARKER_HALF: f64 = 10.0;
const LEGEND_SWATCH: f64 = 34.0;
const LEGEND_FONT_SIZE: f64 = 10.0;
const LEGEND_BASELINE_GAP: f64 = 16.0;
// The artifact gate projects node text onto a 930px desktop diagram and needs
// 6px; the kind badge and keyword are raised to that floor on wider canvases.
const DESKTOP_WIDTH: f64 = 930.0;
const DESKTOP_MINIMUM_TEXT: f64 = 6.0;
pub(crate) const KINDS: [&str; 6] = [
    "dependency",
    "association",
    "inheritance",
    "realization",
    "composition",
    "aggregation",
];

fn vis_extra() -> f64 {
    (VIS_CHIP + VIS_GAP - 2.0 * MEMBER_ADVANCE).ceil()
}
fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
fn string(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap_or("").to_owned()
}

/// The keyword a kind states in its header; a plain class states none.
pub(crate) fn stereotype(kind: &str) -> Option<&'static str> {
    match kind {
        "abstract" => Some("«abstract»"),
        "interface" => Some("«interface»"),
        "enum" => Some("«enumeration»"),
        "record" => Some("«record»"),
        _ => None,
    }
}

/// The end symbols of one relationship kind and whether its line is dashed.
pub(crate) fn notation(kind: &str) -> (Option<Marker>, Option<Marker>, bool) {
    match kind {
        "dependency" => (None, Some(Marker::Open), true),
        "association" => (None, Some(Marker::Open), false),
        "inheritance" => (None, Some(Marker::Triangle), false),
        "realization" => (None, Some(Marker::Triangle), true),
        "composition" => (Some(Marker::DiamondFilled), None, false),
        _ => (Some(Marker::DiamondHollow), None, false),
    }
}

fn generalization(kind: &str) -> bool {
    matches!(kind, "inheritance" | "realization")
}

fn header_height(kind: &str) -> f64 {
    if stereotype(kind).is_some() {
        HEADER_H
    } else {
        HEADER_PLAIN_H
    }
}

fn header_width(raw: &Value) -> f64 {
    let name = text_units(&string(raw, "label")) * NAME_FONT * 0.6 * 1.08;
    let keyword = stereotype(raw["kind"].as_str().unwrap_or("class"))
        .map_or(0.0, |keyword| text_units(keyword) * STEREOTYPE_FONT * 0.6);
    (PAD_X * 2.0 + BADGE_PORCH * 2.0 + name.max(keyword)).ceil()
}

/// Attribute rows, then method rows, each with its one-line signature.
fn member_rows(raw: &Value) -> Vec<(&'static str, usize, &Value, String)> {
    let mut rows = vec![];
    for (role, method) in [("attributes", false), ("methods", true)] {
        for (index, member) in list(&raw[role]).iter().enumerate() {
            rows.push((role, index, member, members::signature(member, method)));
        }
    }
    rows
}

struct Type<'a> {
    raw: &'a Value,
    index: usize,
    id: String,
    kind: String,
    bounds: Rect,
    cell: Cell,
}

fn centre(rect: Rect) -> Point {
    Point {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    }
}

fn marker_box(point: Point, side: Side) -> Rect {
    match side {
        Side::Left => Rect {
            x: point.x - MARKER_REACH,
            y: point.y - MARKER_HALF,
            width: MARKER_REACH,
            height: MARKER_HALF * 2.0,
        },
        Side::Right => Rect {
            x: point.x,
            y: point.y - MARKER_HALF,
            width: MARKER_REACH,
            height: MARKER_HALF * 2.0,
        },
        Side::Top => Rect {
            x: point.x - MARKER_HALF,
            y: point.y - MARKER_REACH,
            width: MARKER_HALF * 2.0,
            height: MARKER_REACH,
        },
        Side::Bottom => Rect {
            x: point.x - MARKER_HALF,
            y: point.y,
            width: MARKER_HALF * 2.0,
            height: MARKER_REACH,
        },
    }
}

fn label_width(label: &str) -> f64 {
    (text_units(label) * RELATIONSHIP_LABEL_FONT * 0.62 + 10.0).max(30.0)
}

// A label sits on the roomiest segment unless the author picks one: above a
// horizontal run, beside a vertical one.
fn default_label_point(raw: &Value, points: &[Point]) -> Point {
    if let (Some(x), Some(y)) = (raw["labelAt"][0].as_f64(), raw["labelAt"][1].as_f64()) {
        return Point { x, y };
    }
    let width = label_width(&string(raw, "label"));
    let vertical = |index: usize| {
        (points[index].x - points[index + 1].x).abs()
            < (points[index].y - points[index + 1].y).abs()
    };
    let room = |index: usize| {
        (points[index + 1].x - points[index].x).hypot(points[index + 1].y - points[index].y)
            - if vertical(index) { 22.0 } else { width + 8.0 }
    };
    let index = match raw["labelSegment"]
        .as_f64()
        .filter(|value| value.fract() == 0.0)
    {
        Some(segment) => (segment as usize).min(points.len() - 2),
        None => (1..points.len() - 1).fold(0, |best, candidate| {
            if room(candidate) > room(best) {
                candidate
            } else {
                best
            }
        }),
    };
    let (a, b) = (points[index], points[index + 1]);
    let dx = raw["labelDx"].as_f64().unwrap_or(0.0);
    let dy = raw["labelDy"].as_f64().unwrap_or(0.0);
    if vertical(index) {
        Point {
            x: a.x + width / 2.0 + 6.0 + dx,
            y: (a.y + b.y) / 2.0 + 4.0 + dy,
        }
    } else {
        Point {
            x: (a.x + b.x) / 2.0 + dx,
            y: a.y - 6.0 + dy,
        }
    }
}

fn label_rect(raw: &Value, at: Point) -> Rect {
    let width = label_width(&string(raw, "label"));
    Rect {
        x: at.x - width / 2.0,
        y: at.y - 10.0,
        width,
        height: 14.0,
    }
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let locale = crate::i18n::resolve(meta);
    let raw_types = list(&document["types"]);
    let raw_relationships = list(&document["relationships"]);
    // An empty via is the same as an omitted one; the authored document stays intact.
    let relationships: Vec<Value> = raw_relationships
        .iter()
        .map(|raw| {
            let mut relation = raw.clone();
            if relation["via"].as_array().is_some_and(Vec::is_empty) {
                relation.as_object_mut().unwrap().remove("via");
            }
            relation
        })
        .collect();
    // The grid is always on; gaps are wider than the erd's because a relationship
    // carries a marker at one end and usually a label.
    let layout = &document["layout"];
    let grid = Grid {
        origin: Point { x: 32.0, y: 40.0 },
        gap_x: 128.0,
        gap_y: 84.0,
        entity_w: 180.0,
        header_h: 0.0,
        row_h: 0.0,
    }
    .merged(&json!({
        "origin": layout["origin"],
        "gapX": layout["gapX"],
        "gapY": layout["gapY"],
        "entityW": layout["typeW"],
    }));
    let type_max_w = layout["typeMaxW"].as_f64().unwrap_or(300.0);
    let mut types: Vec<Type<'_>> = raw_types
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let kind = raw["kind"].as_str().unwrap_or("class").to_owned();
            // An authored width is authoritative; an omitted one grows to the
            // widest member up to typeMaxW, past which members wrap.
            let width = raw["width"].as_f64().unwrap_or_else(|| {
                let widest = member_rows(raw)
                    .iter()
                    .map(|row| text_units(&row.3) * MEMBER_ADVANCE)
                    .fold(0.0, f64::max);
                let wanted = (widest + PAD_X * 2.0 + vis_extra()).ceil();
                header_width(raw).max(type_max_w.min(grid.entity_w.max(wanted)))
            });
            let height = type_height(raw, &kind, width);
            Type {
                raw,
                index,
                id: string(raw, "id"),
                kind,
                bounds: Rect {
                    x: f64::NAN,
                    y: f64::NAN,
                    width,
                    height,
                },
                cell: Cell::of(raw, width, height),
            }
        })
        .collect();
    let cells: Vec<_> = types.iter().map(|item| item.cell).collect();
    let bands = grid::banded_layout(&cells, &grid, None);
    // Types are centred in their row band, so a relationship between two types
    // of one row runs straight across.
    for item in &mut types {
        let position = grid::resolve_position(item.raw, &item.cell, Some(&bands));
        let absolute = item.raw["pos"].as_array().is_some_and(|pos| pos.len() == 2);
        item.bounds.x = position.x;
        item.bounds.y = if absolute {
            position.y
        } else {
            let row = item.cell.row.unwrap_or(f64::NAN) as i64;
            position.y
                + (bands
                    .heights
                    .get(&row)
                    .copied()
                    .unwrap_or(item.bounds.height)
                    - item.bounds.height)
                    / 2.0
        };
    }
    let by_id: HashMap<&str, usize> = raw_types
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| Some((raw["id"].as_str()?, index)))
        .collect();
    let routable: Vec<usize> = relationships
        .iter()
        .enumerate()
        .filter(|(_, raw)| {
            let (from, to) = (raw["from"].as_str(), raw["to"].as_str());
            from.is_some_and(|id| by_id.contains_key(id))
                && to.is_some_and(|id| by_id.contains_key(id))
                && from != to
        })
        .map(|(index, _)| index)
        .collect();
    let endpoint = |index: usize, field: &str| by_id[relationships[index][field].as_str().unwrap()];

    // ---- Legend and canvas ----
    let present: HashSet<String> = relationships
        .iter()
        .filter_map(|raw| raw["kind"].as_str().map(str::to_owned))
        .collect();
    let catalog: Vec<(&str, String)> = KINDS
        .into_iter()
        .map(|kind| (kind, locale.text(&format!("legend.class.{kind}"), &[])))
        .collect();
    let mut entries = legend::resolve(&meta["legend"], &catalog, &present);
    for entry in &mut entries {
        entry.swatch_width = LEGEND_SWATCH;
    }
    let max_y = types
        .iter()
        .filter(|item| item.bounds.x.is_finite() && item.bounds.y.is_finite())
        .map(|item| item.bounds.y + item.bounds.height)
        .fold(0.0, f64::max);
    let band = |width: f64, height: f64| legend::Band {
        x: MARGIN,
        baseline: height - LEGEND_BASELINE_GAP,
        width: width - MARGIN * 2.0,
        font: LEGEND_FONT_SIZE,
        min_title_y: max_y + 8.0,
    };
    let authored_canvas = meta["viewBox"]
        .as_array()
        .and_then(|size| Some((size.first()?.as_f64()?, size.get(1)?.as_f64()?)));
    let (width, height) = authored_canvas.unwrap_or_else(|| {
        let max_x = types
            .iter()
            .filter(|item| item.bounds.x.is_finite() && item.bounds.y.is_finite())
            .map(|item| item.bounds.x + item.bounds.width)
            .fold(0.0, f64::max);
        let mut width = (max_x + MARGIN).ceil();
        let mut footprint = legend::sizing_footprint(&entries, (width - MARGIN * 2.0).max(1.0));
        if footprint.min_width > width - MARGIN * 2.0 {
            width = (footprint.min_width + MARGIN * 2.0).ceil();
            footprint = legend::sizing_footprint(&entries, width - MARGIN * 2.0);
        }
        let mut height = (max_y
            + MARGIN
            + if entries.is_empty() {
                0.0
            } else {
                LEGEND_H + footprint.extra_height
            })
        .ceil();
        if !entries.is_empty() {
            for _ in 0..4 {
                if legend::measure(&entries, &band(width, height), &[]).is_ok() {
                    break;
                }
                height += LEGEND_H;
            }
        }
        (width.max(320.0), height.max(240.0))
    });
    let canvas = Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };

    // ---- Hierarchy buses ----
    // Two or more automatic generalizations of one kind into one supertype, all
    // from subtypes placed below it, draw as one tree. A bus that would cross
    // another type is not drawn; its members keep ordinary routes.
    let auto_routed = |raw: &Value| {
        raw["via"].as_array().is_none_or(Vec::is_empty)
            && raw["route"].as_str().is_none_or(|route| route == "auto")
            && raw.get("fromSide").is_none()
            && raw.get("toSide").is_none()
            && raw.get("labelAt").is_none()
    };
    let mut candidates: Vec<(String, Vec<usize>)> = vec![];
    for index in &routable {
        let raw = &relationships[*index];
        if !generalization(raw["kind"].as_str().unwrap_or("")) || !auto_routed(raw) {
            continue;
        }
        let key = format!("{}\0{}", string(raw, "to"), string(raw, "kind"));
        match candidates
            .iter_mut()
            .find(|(candidate, _)| *candidate == key)
        {
            Some((_, members)) => members.push(*index),
            None => candidates.push((key, vec![*index])),
        }
    }
    let mut bus: HashMap<usize, Vec<Point>> = HashMap::new();
    let mut bus_group: HashMap<usize, usize> = HashMap::new();
    for (group, (_, members)) in candidates.iter().enumerate() {
        if members.len() < 2 {
            continue;
        }
        let target = types[endpoint(members[0], "to")].bounds;
        let bottom = target.y + target.height;
        let sources: Vec<Rect> = members
            .iter()
            .map(|member| types[endpoint(*member, "from")].bounds)
            .collect();
        let first_top = sources
            .iter()
            .map(|source| source.y)
            .fold(f64::INFINITY, f64::min);
        if first_top - bottom < BUS_MIN_GAP {
            continue;
        }
        let bus_y = js_round(bottom + (first_top - bottom) / 2.0);
        let target_x = centre(target).x;
        let paths: Vec<Vec<Point>> = sources
            .iter()
            .map(|source| {
                let source_x = centre(*source).x;
                if (source_x - target_x).abs() < 0.5 {
                    vec![
                        Point {
                            x: source_x,
                            y: source.y,
                        },
                        Point {
                            x: target_x,
                            y: bottom,
                        },
                    ]
                } else {
                    vec![
                        Point {
                            x: source_x,
                            y: source.y,
                        },
                        Point {
                            x: source_x,
                            y: bus_y,
                        },
                        Point {
                            x: target_x,
                            y: bus_y,
                        },
                        Point {
                            x: target_x,
                            y: bottom,
                        },
                    ]
                }
            })
            .collect();
        // Each drop may touch only its own subtype and the supertype.
        let target_index = endpoint(members[0], "to");
        let clear = paths.iter().zip(members).all(|(points, member)| {
            let source_index = endpoint(*member, "from");
            points.windows(2).all(|pair| {
                types
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != target_index && *index != source_index)
                    .all(|(_, other)| {
                        !segment_intersects_rect(pair[0], pair[1], &other.bounds, 0.0)
                    })
            })
        });
        if !clear {
            continue;
        }
        for (member, points) in members.iter().zip(paths) {
            bus.insert(*member, points);
            bus_group.insert(*member, group);
        }
    }

    // ---- Routing ----
    let header_floor = if width > DESKTOP_WIDTH {
        (DESKTOP_MINIMUM_TEXT * width / DESKTOP_WIDTH * 10.0).ceil() / 10.0
    } else {
        0.0
    };
    let nodes: Vec<Node> = types
        .iter()
        .map(|item| type_node(item, header_floor))
        .collect();
    // A generalization stands vertically whenever the two types are not side by
    // side; other relationships keep the horizontal-first inference.
    let side_for = |edge: &Edge, end: Endpoint| {
        let (from, to) = (types[edge.from].bounds, types[edge.to].bounds);
        let overlap_x = from.x < to.x + to.width && to.x < from.x + from.width;
        let overlap_y = from.y < to.y + to.height && to.y < from.y + from.height;
        let vertical =
            overlap_x || (!overlap_y && generalization(edge.raw["kind"].as_str().unwrap_or("")));
        if !vertical {
            return Some(erd::legacy_side(from, to, end));
        }
        let down = centre(to).y > centre(from).y;
        Some(match (end, down) {
            (Endpoint::Source, true) | (Endpoint::Target, false) => Side::Bottom,
            _ => Side::Top,
        })
    };
    let mut distinct = false;
    let (edges, routes, options) = loop {
        let edges: Vec<Edge> = routable
            .iter()
            .filter(|index| !bus.contains_key(index))
            .map(|index| Edge {
                raw: relationships[*index].clone(),
                index: *index,
                from: endpoint(*index, "from"),
                to: endpoint(*index, "to"),
            })
            .collect();
        let options = RouterOptions {
            side_for: Some(&side_for),
            max_port_spacing: PORT_SPACING,
            crossing_free_grid_first: true,
            component_gap_px: 10.0,
            distinct_automatic_ports: distinct,
            label_reservations: false,
            ..RouterOptions::default()
        };
        let routes = router::route_with(
            &nodes,
            &edges,
            &[],
            InputContract::PhoenixV1,
            false,
            options,
        );
        if bus.is_empty() {
            break (edges, routes, options);
        }
        let points_of = |index: usize| -> Vec<Point> {
            bus.get(&index).cloned().unwrap_or_else(|| {
                let position = edges.iter().position(|edge| edge.index == index).unwrap();
                routes[position].points.clone()
            })
        };
        let all: Vec<(usize, Vec<Point>)> = routable
            .iter()
            .map(|index| (*index, points_of(*index)))
            .collect();
        let mut rejected: HashSet<usize> = HashSet::new();
        for (left_position, (left, left_points)) in all.iter().enumerate() {
            for (right, right_points) in &all[left_position + 1..] {
                let (left_group, right_group) = (bus_group.get(left), bus_group.get(right));
                // Only one bus group may share ports and stretches.
                if (left_group.is_none() && right_group.is_none()) || left_group == right_group {
                    continue;
                }
                if bus_conflict(&relationships, *left, left_points, *right, right_points) {
                    rejected.extend(left_group);
                    rejected.extend(right_group);
                }
            }
        }
        if rejected.is_empty() {
            break (edges, routes, options);
        }
        // Fallback routes stay distinct even when they share a target.
        distinct = true;
        bus.retain(|index, _| !rejected.contains(&bus_group[index]));
        bus_group.retain(|_, group| !rejected.contains(group));
    };
    let (inferred, _) = router::inferred_ports(&nodes, &edges, options);
    let position_of: HashMap<usize, usize> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| (edge.index, position))
        .collect();
    // Every routable relationship: points and the sides it actually uses.
    let resolved: HashMap<usize, (Vec<Point>, Side, Side)> = routable
        .iter()
        .map(|index| {
            if let Some(points) = bus.get(index) {
                return (*index, (points.clone(), Side::Top, Side::Bottom));
            }
            let position = position_of[index];
            let raw = &relationships[*index];
            (
                *index,
                (
                    routes[position].points.clone(),
                    erd::endpoint_side(raw, &routes[position], true),
                    erd::endpoint_side(raw, &routes[position], false),
                ),
            )
        })
        .collect();

    // ---- Relationship labels ----
    let mut label_points: HashMap<usize, Point> = routable
        .iter()
        .filter(|index| !string(&relationships[**index], "label").is_empty())
        .map(|index| {
            let raw = &relationships[*index];
            (*index, default_label_point(raw, &resolved[index].0))
        })
        .collect();
    if !label_points.is_empty() {
        let labels: Vec<_> = routable
            .iter()
            .filter_map(|index| {
                let at = *label_points.get(index)?;
                let raw = &relationships[*index];
                Some(shared_labels::Label {
                    relation_index: *index as isize,
                    bounds: label_rect(raw, at),
                    position: at,
                    pinned: ["labelAt", "labelDx", "labelDy", "labelSegment"]
                        .iter()
                        .any(|field| raw.get(*field).is_some()),
                })
            })
            .collect();
        // Every end marker is reserved, so a moved label never covers one.
        let mut obstacles: Vec<Rect> = types.iter().map(|item| item.bounds).collect();
        for index in &routable {
            let (points, from, to) = &resolved[index];
            obstacles.push(marker_box(points[0], *from));
            obstacles.push(marker_box(*points.last().unwrap(), *to));
        }
        let route_list: Vec<_> = routable
            .iter()
            .map(|index| shared_labels::Route {
                relation_index: *index as isize,
                points: resolved[index].0.clone(),
            })
            .collect();
        let footprint = if entries.is_empty() {
            0.0
        } else {
            LEGEND_H
                + legend::sizing_footprint(&entries, (width - MARGIN * 2.0).max(1.0)).extra_height
        };
        let mut options = shared_labels::Options::new(canvas);
        options.placement_bottom = height - footprint;
        options.keep_fallback_near_route = true;
        let placed =
            shared_labels::place_automatic_labels(&labels, &route_list, &obstacles, &[], &options);
        for (label, resolved) in labels.iter().zip(placed) {
            label_points.insert(label.relation_index as usize, resolved.position);
        }
    }

    // ---- Validation ----
    let mut errors: Diagnostics = vec![];
    for item in &types {
        let needed = header_width(item.raw);
        if needed > item.bounds.width {
            errors.push(diagnostic(
                "class",
                "class/title-text-capacity",
                &format!("/types/{}/width", item.index),
                format!(
                    "Type {} title needs {needed}px but the type is {}px wide; remove or raise the authored width, or shorten the title",
                    item.id, item.bounds.width
                ),
                json!({"label":string(item.raw, "label"),"width":item.bounds.width,"requiredWidth":needed}),
            ));
        }
    }
    for (index, item) in types.iter().enumerate() {
        for other in &types[index + 1..] {
            if rects_overlap(&item.bounds, &other.bounds, 0.0) {
                errors.push(diagnostic(
                    "class",
                    "layout/constraint",
                    &format!("/types/{}", item.index),
                    format!("Type {} overlaps type {}", item.id, other.id),
                    json!({"other":other.id,"bounds":item.bounds,"otherBounds":other.bounds}),
                ));
            }
        }
    }
    for index in &routable {
        let points = &resolved[index].0;
        let (start, end) = (points[0], *points.last().unwrap());
        let distance = (end.x - start.x).hypot(end.y - start.y);
        if distance < MIN_RELATIONSHIP_LENGTH {
            errors.push(diagnostic(
                "class",
                "layout/constraint",
                &format!("/relationships/{index}"),
                "Relationship is too short; place the types farther apart",
                json!({"distancePx":distance,"minimumPx":MIN_RELATIONSHIP_LENGTH}),
            ));
        }
    }
    // A bus enters its supertype through one shared trunk, which is one port.
    for (type_index, item) in types.iter().enumerate() {
        let mut sides: Vec<(Side, usize)> = vec![];
        for (position, edge) in edges.iter().enumerate() {
            for (node, side) in [
                (edge.from, inferred[position].from),
                (edge.to, inferred[position].to),
            ] {
                if node != type_index {
                    continue;
                }
                match sides.iter_mut().find(|(candidate, _)| *candidate == side) {
                    Some((_, count)) => *count += 1,
                    None => sides.push((side, 1)),
                }
            }
        }
        for (side, count) in sides {
            if count < 2 {
                continue;
            }
            let extent = if side.horizontal() {
                item.bounds.height
            } else {
                item.bounds.width
            };
            let spacing = PORT_SPACING.min((extent - 32.0).max(0.0) / (count - 1) as f64);
            if spacing >= MARKER_HEIGHT {
                continue;
            }
            errors.push(diagnostic(
                "class",
                "layout/marker-capacity",
                &format!("/types/{}", item.index),
                format!(
                    "Type {} shares its {} side with {count} relationship ends whose markers would overlap",
                    item.id,
                    side.name()
                ),
                json!({"type":item.id,"side":side.name(),"relationshipEnds":count,"spacingPx":js_round(spacing),"markerHeightPx":MARKER_HEIGHT,"sideExtentPx":extent}),
            ));
        }
    }
    let node_boxes: Vec<(String, Rect)> = types
        .iter()
        .map(|item| (item.id.clone(), item.bounds))
        .collect();
    let gate = Gate {
        kind: "class",
        obstacle_kind: "type",
        nodes: &node_boxes,
        showcase: input.quality == Quality::Showcase,
    };
    let routed: Vec<Routed<'_>> = routable
        .iter()
        .map(|index| {
            let (points, from, to) = &resolved[index];
            Routed {
                index: *index,
                raw: &relationships[*index],
                points,
                from_side: from.name(),
                to_side: to.name(),
            }
        })
        .collect();
    gate.routes(&routed, &mut errors);
    let label_rects: Vec<LabelRect> = routable
        .iter()
        .filter_map(|index| {
            let raw = &relationships[*index];
            Some(LabelRect::new(
                *index as isize,
                raw.clone(),
                label_rect(raw, *label_points.get(index)?),
            ))
        })
        .collect();
    gate.labels(&label_rects, &routed, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    // ---- Legend ----
    let mut obstacles: Vec<legend::Obstacle> = vec![];
    for index in &routable {
        for pair in resolved[index].0.windows(2) {
            obstacles.push(legend::Obstacle::Segment(pair[0], pair[1]));
        }
        if let Some(at) = label_points.get(index) {
            obstacles.push(legend::Obstacle::Rect(label_rect(
                &relationships[*index],
                *at,
            )));
        }
    }
    let legends = match legend::measure(&entries, &band(width, height), &obstacles) {
        Ok(Some(placement)) => erd::legend_cards(
            meta,
            &entries,
            &placement,
            locale.text("legend.title", &[]),
            MARGIN,
        ),
        Ok(None) => vec![],
        Err(unfit) => return Err(vec![erd::legend_failure("class", unfit)]),
    };

    // ---- Scene ----
    let relations: Vec<Relation> = routable
        .iter()
        .map(|index| {
            let raw = &relationships[*index];
            let kind = raw["kind"].as_str().unwrap_or("association");
            let (start, end, _) = notation(kind);
            let label = string(raw, "label");
            let at = label_points.get(index).copied();
            let mut provenance = vec![
                format!("/relationships/{index}/from"),
                format!("/relationships/{index}/to"),
            ];
            for field in [
                "fromSide",
                "toSide",
                "via",
                "route",
                "labelAt",
                "labelDx",
                "labelDy",
                "labelSegment",
            ] {
                if raw.get(field).is_some() {
                    provenance.push(format!("/relationships/{index}/{field}"));
                }
            }
            provenance.push(if bus.contains_key(index) {
                "hierarchy-bus".into()
            } else {
                "obstacle-grid-and-readable-routing".into()
            });
            Relation {
                authored: Authored {
                    id: raw["id"]
                        .as_str()
                        .map_or_else(|| format!("relationship-{index}"), str::to_owned),
                    source_path: format!("/relationships/{index}"),
                    kind: kind.into(),
                    facts: raw_relationships[*index].clone(),
                    geometry_provenance: provenance,
                },
                crossover_halo: false,
                independent_ports: false,
                // A hierarchy bus is one junction: its members share the trunk and
                // the triangle under the supertype.
                junction: bus
                    .contains_key(index)
                    .then(|| format!("class-bus:{}:{kind}", string(raw, "to"))),
                label_owner: None,
                from: string(raw, "from"),
                to: string(raw, "to"),
                variant: "default".into(),
                label: label.clone(),
                note: None,
                note_bounds: None,
                note_backdrop: None,
                points: resolved[index].0.clone(),
                label_plate: at.map(|at| label_rect(raw, at)),
                text: at
                    .map(|at| {
                        vec![text(
                            format!("/relationships/{index}/label"),
                            label,
                            at.x,
                            at.y,
                            RELATIONSHIP_LABEL_FONT,
                            "middle",
                        )]
                    })
                    .unwrap_or_default(),
                marker_start: start,
                marker_end: end,
            }
        })
        .collect();
    Ok(CompiledDiagram {
        diagram_type: input.diagram_type,
        schema_version: document["schema_version"]
            .as_f64()
            .expect("validated schema version") as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: meta.clone(),
        view_box: canvas,
        reader_fit: if authored_canvas.is_some() {
            "authored-height"
        } else {
            "intrinsic-height"
        }
        .into(),
        nodes,
        relations,
        containers: vec![],
        text: vec![],
        legends,
        cards: erd::cards(document),
        facts: DiagramFacts::Class {
            types: raw_types.to_vec(),
            relationships: raw_relationships.to_vec(),
        },
        diagnostics: vec![],
    })
}

// Two routes that would leave a bus without a semantic owner: ends of one node
// closer than a marker, a shared corridor, or a proper crossing.
fn bus_conflict(
    relationships: &[Value],
    left: usize,
    left_points: &[Point],
    right: usize,
    right_points: &[Point],
) -> bool {
    let ends = |points: &[Point]| [("from", points[0]), ("to", *points.last().unwrap())];
    for (left_field, left_point) in ends(left_points) {
        for (right_field, right_point) in ends(right_points) {
            if relationships[left][left_field] == relationships[right][right_field]
                && (left_point.x - right_point.x).hypot(left_point.y - right_point.y)
                    < MARKER_HEIGHT - 0.0001
            {
                return true;
            }
        }
    }
    let all = |_: &Value, _: &Value| true;
    let routed = [
        RoutedRelation::new(0, relationships[left].clone(), left_points.to_vec()),
        RoutedRelation::new(1, relationships[right].clone(), right_points.to_vec()),
    ];
    if !collect_ambiguous_corridors(
        &routed,
        &CorridorOptions {
            include_shared_endpoints: Some(&all),
            ..CorridorOptions::default()
        },
    )
    .is_empty()
    {
        return true;
    }
    left_points.windows(2).any(|a| {
        right_points
            .windows(2)
            .any(|b| proper_segment_intersection(a[0], a[1], b[0], b[1]).is_some())
    })
}

fn type_height(raw: &Value, kind: &str, width: f64) -> f64 {
    let available = width - PAD_X * 2.0 - vis_extra();
    let mut height = header_height(kind);
    for role in ["attributes", "methods"] {
        let rows: Vec<_> = member_rows(raw)
            .into_iter()
            .filter(|row| row.0 == role)
            .collect();
        if rows.is_empty() {
            continue;
        }
        let lines: usize = rows
            .iter()
            .map(|row| members::wrap(&row.3, available).len())
            .sum();
        height += lines as f64 * ROW_H + COMPARTMENT_PAD * 2.0;
    }
    height
}

fn type_node(item: &Type<'_>, header_floor: f64) -> Node {
    let raw = item.raw;
    let path = format!("/types/{}", item.index);
    let b = item.bounds;
    let label = string(raw, "label");
    let keyword = stereotype(&item.kind);
    let cx = b.x + b.width / 2.0;
    let badge = Point {
        x: b.x + PAD_X,
        y: b.y + if keyword.is_some() { 14.0 } else { 9.0 },
    };
    let mut texts = vec![text(
        format!("{path}/kind"),
        item.kind
            .chars()
            .next()
            .map_or_else(String::new, |c| c.to_uppercase().collect()),
        badge.x + BADGE_SIZE / 2.0,
        badge.y + BADGE_SIZE / 2.0 + 3.5,
        header_floor.max(10.0),
        "middle",
    )];
    if let Some(keyword) = keyword {
        texts.push(text(
            format!("{path}/kind"),
            keyword,
            cx,
            b.y + 16.0,
            header_floor.max(STEREOTYPE_FONT),
            "middle",
        ));
    }
    texts.push(text(
        format!("{path}/label"),
        label.clone(),
        cx,
        b.y + if keyword.is_some() { 34.0 } else { 22.0 },
        NAME_FONT,
        "middle",
    ));
    let available = b.width - PAD_X * 2.0 - vis_extra();
    let mut top = b.y + header_height(&item.kind);
    let mut detail_rows = vec![];
    for role in ["attributes", "methods"] {
        let rows: Vec<_> = member_rows(raw)
            .into_iter()
            .filter(|row| row.0 == role)
            .collect();
        if rows.is_empty() {
            continue;
        }
        let mut line = 0.0;
        for (_, index, member, signature) in rows {
            let member_path = format!("{path}/{role}/{index}");
            let lines = members::wrap(&signature, available);
            let row_top = top + COMPARTMENT_PAD + line * ROW_H;
            let x0 = b.x + PAD_X;
            let glyph = member["visibility"].as_str().and_then(members::glyph);
            let mut row_texts = vec![];
            if let Some(glyph) = glyph {
                row_texts.push(text(
                    format!("{member_path}/visibility"),
                    glyph,
                    x0 + VIS_CHIP / 2.0,
                    row_top + ROW_H - 5.0,
                    MEMBER_FONT,
                    "middle",
                ));
            }
            let text_x = if glyph.is_some() {
                x0 + VIS_CHIP + VIS_GAP
            } else {
                x0
            };
            if lines.len() == 1 {
                line += 1.0;
                let baseline = top + COMPARTMENT_PAD + line * ROW_H - 5.0;
                let name = string(member, "name");
                let (left, right, right_field) = if role == "attributes" {
                    (name, member["type"].as_str(), "type")
                } else {
                    (
                        format!("{name}({})", member["parameters"].as_str().unwrap_or("")),
                        member["returns"].as_str(),
                        "returns",
                    )
                };
                row_texts.push(text(
                    format!("{member_path}/name"),
                    left,
                    text_x,
                    baseline,
                    MEMBER_FONT,
                    "start",
                ));
                if let Some(right) = right.filter(|value| !value.is_empty()) {
                    row_texts.push(text(
                        format!("{member_path}/{right_field}"),
                        right,
                        b.x + b.width - PAD_X,
                        baseline,
                        MEMBER_FONT,
                        "end",
                    ));
                }
            } else {
                for (number, entry) in lines.iter().enumerate() {
                    line += 1.0;
                    let baseline = top + COMPARTMENT_PAD + line * ROW_H - 5.0;
                    let (content, x) = if number == 0 && glyph.is_some() {
                        (entry.text.chars().skip(2).collect::<String>(), text_x)
                    } else {
                        (
                            entry.text.clone(),
                            x0 + if glyph.is_some() {
                                VIS_CHIP + VIS_GAP - 2.0 * MEMBER_ADVANCE
                            } else {
                                0.0
                            } + entry.indent as f64 * MEMBER_ADVANCE,
                        )
                    };
                    row_texts.push(text(
                        format!("{member_path}/lines/{number}"),
                        content,
                        x,
                        baseline,
                        MEMBER_FONT,
                        "start",
                    ));
                }
            }
            detail_rows.push(DetailRow {
                source_path: member_path,
                bounds: Rect {
                    x: b.x,
                    y: row_top,
                    width: b.width,
                    height: lines.len() as f64 * ROW_H,
                },
                text: row_texts,
                role: role.into(),
            });
        }
        top += line * ROW_H + COMPARTMENT_PAD * 2.0;
    }
    let mut provenance = if raw["pos"].is_array() {
        vec![format!("{path}/pos")]
    } else {
        vec![
            format!("{path}/row"),
            format!("{path}/col"),
            "banded-grid".into(),
        ]
    };
    provenance.push(if raw.get("width").is_some() {
        format!("{path}/width")
    } else {
        "measured-member-width".into()
    });
    Node {
        authored: Authored {
            id: item.id.clone(),
            source_path: path,
            kind: item.kind.clone(),
            facts: raw.clone(),
            geometry_provenance: provenance,
        },
        bounds: b,
        sigil: None,
        label,
        text: texts,
        detail_rows,
    }
}
