use super::*;
use crate::geometry::rects_overlap_for_contract;
use crate::scene::{Container, Node, Sigil};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const LEGACY_COLUMNS: [f64; 6] = [88.0, 220.0, 300.0, 430.0, 500.0, 625.0];

#[derive(Clone)]
pub(super) struct Measurement {
    pub readable: bool,
    pub columns: Vec<f64>,
    pub lane_width: f64,
    pub lane_tops: Vec<f64>,
    pub lane_heights: Vec<f64>,
    pub lane_gap: f64,
    pub lane_ids: Vec<String>,
    pub view_box: Rect,
    pub auto_height: f64,
    pub minimum_width: f64,
    pub nodes: Vec<Node>,
    pub containers: Vec<Container>,
    pub text: Vec<Text>,
    pub legends: Vec<Card>,
    pub label_obstacles: Vec<Rect>,
    pub channel_labels: BTreeSet<usize>,
    pub layout_diagnostics: Diagnostics,
}
pub(super) fn absolute(document: &Value) -> bool {
    document.pointer("/meta/viewBox").is_some()
        || array(document, "edges").iter().any(|edge| {
            ["via", "labelAt", "channelX", "channelY"]
                .iter()
                .any(|field| edge.get(field).is_some())
        })
}
fn node_height(node: &Value) -> f64 {
    number(
        node,
        "height",
        if string(node, "tag").is_empty() {
            52.0
        } else {
            68.0
        },
    )
}

fn automatic_width(input: &DiagramInput, node: &Value) -> f64 {
    let label_width =
        92.0_f64.max(((units(input, string(node, "label")) * 6.8 - 6.0) / 4.0).ceil() * 4.0);
    let mut width = label_width;
    for (field, preferred) in [("sublabel", 8.0), ("tag", 7.0)] {
        let content = string(node, field);
        if !content.is_empty() && units(input, content) * 6.0 * 0.6 > label_width - 8.0 {
            width = width.max(((units(input, content) * preferred * 0.6 + 8.0) / 4.0).ceil() * 4.0);
        }
    }
    width.min(200.0).max(label_width)
}
fn group_span(
    input: &DiagramInput,
    group: &Value,
    columns: &[f64],
    nodes: &[Entry],
    widths: &BTreeMap<String, f64>,
    readable: bool,
) -> (f64, f64) {
    let from = number(group, "fromCol", 0.0) as usize;
    let to = number(group, "toCol", 0.0) as usize;
    let start = columns[from] - 50.0;
    let end = columns[to] + 50.0;
    if !readable {
        return (start, end - start);
    }
    let width = (end - start).max(units(input, string(group, "label")) * 5.6 + 20.0);
    let mut left = if from == to && width > end - start {
        start
    } else {
        (start + end - width) / 2.0
    };
    let mut right = left + width;
    for node in nodes {
        let col = number(&node.value, "col", 0.0) as usize;
        if node.value["lane"] == group["lane"] && col >= from && col <= to {
            let half = widths[string(&node.value, "id")] / 2.0;
            left = left.min(columns[col] - half - 4.0);
            right = right.max(columns[col] + half + 4.0);
        }
    }
    (left, right - left)
}

// Source collections and independent rank/width overrides remain explicit.
#[allow(clippy::too_many_arguments)]
pub(super) fn measure(
    input: &DiagramInput,
    lanes: &[Entry],
    entries: &[Entry],
    edges: &[Entry],
    phases: &[Entry],
    groups: &[Entry],
    feedback_gap: f64,
    rank_minimums: &BTreeMap<(usize, usize), f64>,
    width_overrides: &BTreeMap<String, f64>,
) -> Result<Measurement, Diagnostics> {
    let document = &input.document;
    let readable = number(document, "schema_version", 1.0) == 2.0;
    let phoenix = input.input_contract == InputContract::PhoenixV1;
    let preserves = absolute(document);
    let mut widths = BTreeMap::new();
    for node in entries {
        let mut width = number(&node.value, "width", 92.0);
        if phoenix && readable && !preserves && node.value.get("width").is_none() {
            width = automatic_width(input, &node.value);
            width = width.max(
                width_overrides
                    .get(string(&node.value, "id"))
                    .copied()
                    .unwrap_or(0.0),
            );
        }
        widths.insert(string(&node.value, "id").to_owned(), width);
    }
    let mut columns = if readable {
        (0..6)
            .map(|col| 94.0 + col as f64 * 120.0)
            .collect::<Vec<_>>()
    } else {
        LEGACY_COLUMNS.to_vec()
    };
    let mut channel_labels = BTreeSet::new();
    if readable {
        let mut constraints: Vec<(usize, usize, f64)> =
            (0..5).map(|col| (col, col + 1, 120.0)).collect();
        constraints.extend(
            rank_minimums
                .iter()
                .map(|(&(from, to), &minimum)| (from, to, minimum)),
        );
        for (i, left) in entries.iter().enumerate() {
            for right in &entries[i + 1..] {
                let lc = number(&left.value, "col", 0.0) as usize;
                let rc = number(&right.value, "col", 0.0) as usize;
                if left.value["lane"] != right.value["lane"] || lc == rc {
                    continue;
                }
                if (number(&left.value, "yOffset", 0.0) - number(&right.value, "yOffset", 0.0))
                    .abs()
                    >= node_height(&left.value) / 2.0 + node_height(&right.value) / 2.0 + 8.0
                {
                    continue;
                }
                let gap = if phoenix && !preserves { 32.0 } else { 8.0 };
                constraints.push((
                    lc.min(rc),
                    lc.max(rc),
                    widths[string(&left.value, "id")] / 2.0
                        + gap
                        + widths[string(&right.value, "id")] / 2.0,
                ));
            }
        }
        for edge in edges {
            let from = entries
                .iter()
                .find(|node| node.value["id"] == edge.value["from"])
                .unwrap();
            let to = entries
                .iter()
                .find(|node| node.value["id"] == edge.value["to"])
                .unwrap();
            let fc = number(&from.value, "col", 0.0) as usize;
            let tc = number(&to.value, "col", 0.0) as usize;
            if from.value["lane"] != to.value["lane"]
                || fc == tc
                || ["via", "channelX", "channelY"]
                    .iter()
                    .any(|field| edge.value.get(field).is_some())
                || !["", "auto", "straight"].contains(&string(&edge.value, "route"))
                || number(&from.value, "yOffset", 0.0) != number(&to.value, "yOffset", 0.0)
            {
                continue;
            }
            let label = string(&edge.value, "label");
            let labeled = if !label.is_empty() && edge.value.get("labelAt").is_none() {
                28.0_f64.max(30.0_f64.max(units(input, label) * 4.8 + 10.0) + 8.0)
            } else {
                28.0
            };
            let channel = !label.is_empty()
                && edge.value.get("labelAt").is_none()
                && ["", "auto"].contains(&string(&edge.value, "route"))
                && edge.value.get("fromSide").is_none()
                && edge.value.get("toSide").is_none()
                && labeled - 28.0 > 112.0;
            if channel {
                channel_labels.insert(edge.index);
            }
            let half =
                widths[string(&from.value, "id")] / 2.0 + widths[string(&to.value, "id")] / 2.0;
            constraints.push((
                fc.min(tc),
                fc.max(tc),
                half + if channel { 28.0 } else { labeled },
            ));
        }
        for (collection, offset) in [(phases, 92.0), (groups, 100.0)] {
            for entry in collection {
                let from = number(&entry.value, "fromCol", 0.0) as usize;
                let to = number(&entry.value, "toCol", 0.0) as usize;
                let minimum = units(input, string(&entry.value, "label")) * 5.6
                    + if offset == 92.0 { 8.0 } else { 20.0 };
                if from == to {
                    if to < 5 {
                        constraints.push((to, to + 1, 120.0 + (minimum - offset).max(0.0)));
                    }
                } else {
                    constraints.push((from, to, (minimum - offset).max(0.0)));
                }
            }
        }
        constraints.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then(a.0.cmp(&b.0))
                .then_with(|| a.2.total_cmp(&b.2))
        });
        for to in 1..6 {
            for &(from, end, min) in &constraints {
                if end == to {
                    columns[to] = columns[to].max(columns[from] + min);
                }
            }
        }
        let first = entries
            .iter()
            .filter(|node| number(&node.value, "col", 0.0) == 0.0)
            .fold(46.0_f64, |max, node| {
                max.max(widths[string(&node.value, "id")] / 2.0)
            });
        let shift = (48.0 + first - columns[0]).max(0.0);
        for column in &mut columns {
            *column += shift;
        }
        let mut header_shift = 0.0_f64;
        for edge in edges {
            if edge.value.get("via").is_some() || edge.value.get("channelX").is_some() {
                continue;
            }
            for (side, endpoint) in [("fromSide", "from"), ("toSide", "to")] {
                if string(&edge.value, side) != "top" {
                    continue;
                }
                let node = entries
                    .iter()
                    .find(|node| node.value["id"] == edge.value[endpoint])
                    .unwrap();
                let index = lanes
                    .iter()
                    .position(|lane| lane.value["id"] == node.value["lane"])
                    .unwrap();
                let prefix = if string(&lanes[index].value, "variant") == "exception" {
                    "EX".into()
                } else {
                    format!("{:02}", index + 1)
                };
                let right = 54.0
                    + units(
                        input,
                        &format!("{prefix} / {}", string(&lanes[index].value, "label")),
                    ) * 6.2;
                header_shift = header_shift
                    .max(right + 2.0 - columns[number(&node.value, "col", 0.0) as usize]);
            }
        }
        for column in &mut columns {
            *column += header_shift;
        }
        let mut shift = 0.0_f64;
        for edge in edges
            .iter()
            .filter(|edge| channel_labels.contains(&edge.index))
        {
            let from = entries
                .iter()
                .find(|node| node.value["id"] == edge.value["from"])
                .unwrap();
            let to = entries
                .iter()
                .find(|node| node.value["id"] == edge.value["to"])
                .unwrap();
            let center = (columns[number(&from.value, "col", 0.0) as usize]
                + columns[number(&to.value, "col", 0.0) as usize])
                / 2.0;
            shift = shift.max(
                16.0 - center
                    + 30.0_f64.max(units(input, string(&edge.value, "label")) * 4.8 + 10.0) / 2.0,
            );
        }
        for phase in phases {
            let from = number(&phase.value, "fromCol", 0.0) as usize;
            let to = number(&phase.value, "toCol", 0.0) as usize;
            let width = (columns[to] - columns[from] + 92.0)
                .max(units(input, string(&phase.value, "label")) * 5.6 + 8.0);
            let left = if from == to {
                columns[from] - 46.0
            } else {
                (columns[from] + columns[to] - width) / 2.0
            };
            shift = shift.max(16.0 - left);
        }
        for group in groups {
            shift = shift
                .max(44.0 - group_span(input, &group.value, &columns, entries, &widths, true).0);
        }
        for column in &mut columns {
            *column += shift;
        }
    }
    let mut lane_width = 640.0_f64;
    if readable {
        let mut right = columns[5] + 50.0;
        for node in entries {
            right = right.max(
                columns[number(&node.value, "col", 0.0) as usize]
                    + widths[string(&node.value, "id")] / 2.0,
            );
        }
        for group in groups {
            let (left, width) = group_span(input, &group.value, &columns, entries, &widths, true);
            right = right.max(left + width);
        }
        lane_width = lane_width.max((right - 32.0).ceil());
        for (index, lane) in lanes.iter().enumerate() {
            lane_width = lane_width.max(
                (units(
                    input,
                    &format!("{:02} / {}", index + 1, string(&lane.value, "label")),
                ) * 6.2
                    + 30.0)
                    .ceil(),
            );
        }
    }
    let extent = |lane: Option<&Value>| {
        entries
            .iter()
            .filter(|node| lane.is_none_or(|lane| node.value["lane"] == lane["id"]))
            .fold(0.0_f64, |max, node| {
                max.max(node_height(&node.value) / 2.0 + number(&node.value, "yOffset", 0.0).abs())
            })
    };
    let stack = entries.iter().enumerate().any(|(index, a)| {
        entries[index + 1..].iter().any(|b| {
            a.value["lane"] == b.value["lane"]
                && a.value["col"] == b.value["col"]
                && number(&a.value, "yOffset", 0.0) != number(&b.value, "yOffset", 0.0)
        })
    });
    let independent = phoenix
        && stack
        && document.pointer("/meta/viewBox").is_none()
        && !array(document, "edges").iter().any(|edge| {
            ["via", "labelAt", "channelX", "channelY"]
                .iter()
                .any(|key| edge.get(key).is_some())
        });
    let base_height = if readable {
        30.0 + 74.0_f64.max((extent(None) * 2.0 + 8.0).ceil())
    } else {
        104.0
    };
    let mut heights = lanes
        .iter()
        .map(|lane| {
            if readable && independent {
                30.0 + 74.0_f64.max((extent(Some(&lane.value)) * 2.0 + 8.0).ceil())
            } else {
                base_height
            }
        })
        .collect::<Vec<_>>();
    let mut headers = vec![0.0_f64; lanes.len()];
    let mut footers = vec![0.0_f64; lanes.len()];
    if readable {
        for (index, lane) in lanes.iter().enumerate() {
            let content = heights[index] - 30.0;
            for group in groups
                .iter()
                .filter(|group| group.value["lane"] == lane.value["id"])
            {
                let (left, _) = group_span(input, &group.value, &columns, entries, &widths, true);
                let label_left = left + 10.0;
                let label_right = label_left + units(input, string(&group.value, "label")) * 5.6;
                for node in entries.iter().filter(|node| {
                    node.value["lane"] == group.value["lane"]
                        && number(&node.value, "col", 0.0) >= number(&group.value, "fromCol", 0.0)
                        && number(&node.value, "col", 0.0) <= number(&group.value, "toCol", 0.0)
                }) {
                    let half = widths[string(&node.value, "id")] / 2.0;
                    let center = columns[number(&node.value, "col", 0.0) as usize];
                    let overlap = center + half > label_left && center - half < label_right;
                    let top = (content - node_height(&node.value)) / 2.0
                        + number(&node.value, "yOffset", 0.0);
                    headers[index] =
                        headers[index].max(((if overlap { 11.0 } else { 9.0 }) - top).ceil());
                    footers[index] = footers[index]
                        .max((1.0 - (content - 4.0 - top - node_height(&node.value))).ceil());
                }
            }
            heights[index] += headers[index] + footers[index];
        }
    }
    let gap = 20.0_f64.max(feedback_gap.ceil());
    let mut cursor = 52.0;
    let tops = heights
        .iter()
        .map(|height| {
            let top = cursor;
            cursor += height + gap;
            top
        })
        .collect::<Vec<_>>();
    let last_bottom = cursor - gap;
    let mut min_width = if readable { 56.0 + lane_width } else { 720.0 };
    let (legends, extra) = measure_legends(input, entries, last_bottom, &mut min_width, readable)?;
    let authored_box = document.pointer("/meta/viewBox").and_then(Value::as_array);
    let auto_height = last_bottom + 124.0 + extra;
    let view_box = Rect {
        x: 0.0,
        y: 0.0,
        width: authored_box
            .map(|value| value[0].as_f64().unwrap())
            .unwrap_or(min_width),
        height: authored_box
            .map(|value| value[1].as_f64().unwrap())
            .unwrap_or(auto_height),
    };
    if ![view_box.width, view_box.height, min_width, auto_height]
        .iter()
        .all(|value| value.is_finite())
    {
        let authored_geometry=entries.iter().flat_map(|node|["width","height","yOffset"].into_iter().filter_map(move|field|node.value.get(field).map(|value|json!({"path":format!("/nodes/{}/{field}",node.index),"value":value})))).collect::<Vec<_>>();
        return Err(vec![failure("layout/constraint","/meta/viewBox","Authored dimensions or offsets overflow the measured canvas; reduce those geometry assertions",json!({"finiteCanvas":false,"authoredGeometry":authored_geometry}))]);
    }
    let mut nodes = Vec::new();
    let mut diagnostics = Vec::new();
    for entry in entries {
        let value = &entry.value;
        let index = lanes
            .iter()
            .position(|lane| lane.value["id"] == value["lane"])
            .unwrap();
        let height = node_height(value);
        let width = widths[string(value, "id")];
        let center = columns[number(value, "col", 0.0) as usize];
        let content = heights[index] - 30.0 - headers[index] - footers[index];
        let bounds = Rect {
            x: center - width / 2.0,
            y: tops[index]
                + 30.0
                + headers[index]
                + (content - height) / 2.0
                + number(value, "yOffset", 0.0),
            width,
            height,
        };
        if ![bounds.x, bounds.y, bounds.width, bounds.height]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(vec![failure(
                if readable {
                    "workflow/non-finite-node-geometry"
                } else {
                    "layout/constraint"
                },
                &format!("/nodes/{}", entry.index),
                "Node geometry became non-finite; reduce authored dimensions or offsets",
                json!({"node":value["id"],"finite":false}),
            )]);
        }
        let author = authored(
            entry,
            "nodes",
            string(value, "type"),
            &["col", "yOffset", "width", "height"],
        );
        let mut node_text = Vec::new();
        for (field, preferred, minimum, offset) in [
            ("label", 11.0, 9.0, 21.0),
            ("sublabel", 8.0, 6.0, 38.0),
            ("tag", 7.0, 6.0, height - 12.0),
        ] {
            let content = string(value, field);
            if content.is_empty() {
                continue;
            }
            let font = fitted_node_font_size_for_contract(
                content,
                width,
                preferred,
                minimum,
                input.input_contract,
            );
            node_text.push(text(
                format!("{}/{field}", author.source_path),
                content,
                center,
                bounds.y + offset,
                font,
                "middle",
            ));
            if (field == "label" && units(input, content) * 6.8 > width + 6.0)
                || (field != "label" && units(input, content) * minimum * 0.6 > width - 8.0)
            {
                diagnostics.push(failure(
                    "layout/constraint",
                    &format!("{}/{field}", author.source_path),
                    format!("Node {} {field} does not fit its authored width", author.id),
                    json!({"node":author.id,"availableWidthPx":width-8.0,"minimumFontPx":minimum}),
                ));
            }
        }
        if bounds.x < 40.0
            || bounds.x + width > 40.0 + lane_width
            || bounds.y < tops[index] + 30.0 + headers[index]
            || bounds.y + height > tops[index] + heights[index]
        {
            diagnostics.push(failure(
                "layout/constraint",
                &author.source_path,
                format!("Node {} exceeds its lane content bounds", author.id),
                json!({"bounds":bounds,"lane":value["lane"]}),
            ));
        }
        let mut sigil_y = 6.0;
        let mut sigil_size = 11.0;
        if phoenix && !node_text.is_empty() {
            let label_width = units(input, &node_text[0].content) * node_text[0].font_size * 0.6;
            let left = 19.0;
            let right = width - 4.0;
            if width / 2.0 - label_width / 2.0 < left || width / 2.0 + label_width / 2.0 > right {
                if label_width <= right - left {
                    node_text[0].position.x = bounds.x
                        + ((right - label_width / 2.0) * 10.0).floor().min(
                            (width / 2.0 * 10.0).max(((left + label_width / 2.0) * 10.0).ceil()),
                        ) / 10.0;
                } else {
                    let mut bottom = 17.0_f64;
                    let mut ys = Vec::new();
                    for row in &node_text {
                        let y = (row.position.y - bounds.y)
                            .max(((bottom + 2.0 + row.font_size * 1.2) * 10.0).ceil() / 10.0);
                        bottom = y + row.font_size * 0.3;
                        ys.push(y);
                    }
                    if bottom <= height - 2.0 {
                        for (row, y) in node_text.iter_mut().zip(ys) {
                            row.position.y = bounds.y + y;
                        }
                    } else {
                        sigil_y = 1.0;
                        sigil_size = 1.0_f64
                            .max(11.0_f64.min((21.0 - node_text[0].font_size * 1.2 - 3.0).floor()));
                    }
                }
            }
        }
        nodes.push(Node {
            sigil: if string(value, "icon") == "none" {
                None
            } else {
                Some(Sigil {
                    kind: if string(value, "icon").is_empty() {
                        string(value, "type").into()
                    } else {
                        string(value, "icon").into()
                    },
                    position: Point {
                        x: bounds.x + 6.0,
                        y: bounds.y + sigil_y,
                    },
                    size: sigil_size,
                })
            },
            authored: author,
            bounds,
            label: string(value, "label").into(),
            text: node_text,
            detail_rows: Vec::new(),
        });
    }
    if !readable {
        for edge in edges {
            let from = nodes
                .iter()
                .find(|node| node.authored.id == string(&edge.value, "from"))
                .unwrap();
            let to = nodes
                .iter()
                .find(|node| node.authored.id == string(&edge.value, "to"))
                .unwrap();
            if from.authored.facts["lane"] != to.authored.facts["lane"]
                || from.authored.facts["col"] == to.authored.facts["col"]
                || (number(&from.authored.facts, "yOffset", 0.0)
                    - number(&to.authored.facts, "yOffset", 0.0))
                .abs()
                    >= from.bounds.height / 2.0 + to.bounds.height / 2.0 + 8.0
            {
                continue;
            }
            let distance = ((from.bounds.x + from.bounds.width / 2.0)
                - (to.bounds.x + to.bounds.width / 2.0))
                .abs();
            let clearance = distance - from.bounds.width / 2.0 - to.bounds.width / 2.0;
            let direct = edge.value.get("via").is_none()
                && ["", "auto", "straight"].contains(&string(&edge.value, "route"))
                && ((from.bounds.y + from.bounds.height / 2.0)
                    - (to.bounds.y + to.bounds.height / 2.0))
                    .abs()
                    < 0.0001;
            let required = if direct { 28.0 } else { 8.0 };
            if clearance < required {
                return Err(vec![failure(
                    "workflow/column-capacity",
                    &format!("/edges/{}", edge.index),
                    "Fixed-v1 columns do not provide the required route clearance",
                    json!({"centerDistancePx":distance,"nodeWidthsPx":[from.bounds.width,to.bounds.width],"actualSignedClearancePx":clearance,"requiredDirectClearancePx":required}),
                )]);
            }
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        for other in &nodes[..index] {
            if rects_overlap_for_contract(&node.bounds, &other.bounds, 8.0, input.input_contract) {
                diagnostics.push(failure(
                    if readable {
                        "workflow/node-overlap"
                    } else {
                        "layout/constraint"
                    },
                    &node.authored.source_path,
                    format!("Node {} overlaps {}", node.authored.id, other.authored.id),
                    json!({"node":node.authored.id,"otherNode":other.authored.id,"gapPx":8}),
                ));
            }
        }
    }
    let (containers, obstacles) = measure_containers(
        input, lanes, phases, groups, entries, &nodes, &widths, &columns, &tops, &heights,
        lane_width, readable,
    );
    for (index, phase) in phases.iter().enumerate() {
        let from = number(&phase.value, "fromCol", 0.0) as usize;
        let to = number(&phase.value, "toCol", 0.0) as usize;
        let span = containers
            .iter()
            .find(|container| container.authored.source_path == format!("/phases/{}", phase.index))
            .unwrap();
        if units(input, string(&phase.value, "label")) * 5.6 > span.bounds.width + 8.0 {
            diagnostics.push(failure(
                "layout/constraint",
                &format!("/phases/{}/label", phase.index),
                "Phase label exceeds its authored column span",
                json!({"span":span.bounds}),
            ));
        }
        for other in &phases[..index] {
            if from as f64 <= number(&other.value, "toCol", 0.0)
                && to as f64 >= number(&other.value, "fromCol", 0.0)
            {
                diagnostics.push(failure(
                    "layout/constraint",
                    &format!("/phases/{}", phase.index),
                    "Phase column ranges overlap",
                    json!({"otherPhasePath":format!("/phases/{}",other.index)}),
                ));
            }
        }
    }
    for group in containers
        .iter()
        .filter(|container| container.authored.kind == "group")
    {
        if group.members.is_empty() {
            diagnostics.push(failure(
                "layout/constraint",
                &group.authored.source_path,
                "Group does not contain any node in its lane and column range",
                json!({"group":group.authored.facts}),
            ));
        }
    }
    if readable {
        let overlaps = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "workflow/node-overlap")
            .cloned()
            .collect::<Vec<_>>();
        if !overlaps.is_empty() {
            return Err(overlaps);
        }
    }
    Ok(Measurement {
        readable,
        columns,
        lane_width,
        lane_tops: tops,
        lane_heights: heights,
        lane_gap: gap,
        lane_ids: lanes
            .iter()
            .map(|lane| string(&lane.value, "id").into())
            .collect(),
        view_box,
        auto_height,
        minimum_width: min_width,
        nodes,
        containers,
        text: Vec::new(),
        legends,
        label_obstacles: obstacles,
        channel_labels,
        layout_diagnostics: diagnostics,
    })
}

fn legend_message<'a>(input: &'a DiagramInput, kind: &str) -> &'a str {
    let key = if kind == "title" {
        "legend.title".to_owned()
    } else {
        format!("legend.workflow.{kind}")
    };
    if let Some(value) = input
        .document
        .pointer("/meta/translations")
        .and_then(|translations| translations.get(&key))
        .and_then(Value::as_str)
    {
        let has_placeholder = value.split('{').skip(1).any(|suffix| {
            suffix.split_once('}').is_some_and(|(name, _)| {
                !name.is_empty()
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            })
        });
        if !value.is_empty() && !has_placeholder {
            return value;
        }
    }
    let locale = string(&input.document["meta"], "locale").to_ascii_lowercase();
    match (locale.as_str(), kind) {
        ("zh-cn", "title") => "图例",
        ("zh-cn", "frontend") => "用户界面",
        ("zh-cn", "backend") => "Agent 逻辑",
        ("zh-cn", "security") => "策略",
        ("zh-cn", "messagebus") => "工具操作",
        ("zh-cn", "database") => "上下文 / 追踪",
        ("zh-cn", "cloud") => "云服务",
        ("zh-cn", "external") => "外部系统",
        ("zh-tw", "title") => "圖例",
        ("zh-tw", "frontend") => "使用者介面",
        ("zh-tw", "backend") => "Agent 邏輯",
        ("zh-tw", "security") => "政策",
        ("zh-tw", "messagebus") => "工具操作",
        ("zh-tw", "database") => "脈絡 / 追蹤",
        ("zh-tw", "cloud") => "雲端服務",
        ("zh-tw", "external") => "外部系統",
        ("es", "title") => "Leyenda",
        ("es", "frontend") => "Interfaz de usuario",
        ("es", "backend") => "Lógica del agente",
        ("es", "security") => "Política",
        ("es", "messagebus") => "Acción de herramienta",
        ("es", "database") => "Contexto / traza",
        ("es", "cloud") => "Servicio en la nube",
        ("es", "external") => "Sistema externo",
        ("ko", "title") => "범례",
        ("ko", "frontend") => "사용자 UI",
        ("ko", "backend") => "Agent 로직",
        ("ko", "security") => "정책",
        ("ko", "messagebus") => "도구 실행",
        ("ko", "database") => "컨텍스트 / 추적",
        ("ko", "cloud") => "클라우드 서비스",
        ("ko", "external") => "외부 시스템",
        (_, "title") => "Legend",
        (_, "frontend") => "User UI",
        (_, "backend") => "Agent logic",
        (_, "security") => "Policy",
        (_, "messagebus") => "Tool action",
        (_, "database") => "Context / trace",
        (_, "cloud") => "Cloud service",
        _ => "External system",
    }
}

fn legend_source(input: &DiagramInput, entries: &[Entry], kind: &str) -> String {
    let explicit = format!("/meta/legend/entries/{kind}");
    if input.document.pointer(&explicit).is_some() {
        return explicit;
    }
    let key = if kind == "title" {
        "legend.title".to_owned()
    } else {
        format!("legend.workflow.{kind}")
    };
    let translated = format!("/meta/translations/{key}");
    if input.document.pointer(&translated).and_then(Value::as_str)
        == Some(legend_message(input, kind))
    {
        return translated;
    }
    if let Some(node) = entries
        .iter()
        .find(|node| string(&node.value, "type") == kind)
    {
        return format!("/nodes/{}/type", node.index);
    }
    if input.document.pointer("/meta/legend").is_some() {
        "/meta/legend".into()
    } else if input.document.pointer("/meta/locale").is_some() {
        "/meta/locale".into()
    } else {
        "/meta".into()
    }
}

fn measure_legends(
    input: &DiagramInput,
    entries: &[Entry],
    last_bottom: f64,
    min_width: &mut f64,
    readable: bool,
) -> Result<(Vec<Card>, f64), Diagnostics> {
    let document = &input.document;
    let catalog = [
        "frontend",
        "backend",
        "security",
        "messagebus",
        "database",
        "cloud",
        "external",
    ];
    let legend = &document["meta"]["legend"];
    let mode = string(legend, "mode");
    let mut items = Vec::new();
    if mode != "hidden" {
        for kind in catalog {
            let present = entries
                .iter()
                .any(|node| string(&node.value, "type") == kind);
            let override_value = &legend["entries"][kind];
            if override_value["visible"] == true
                || ((mode == "all" || present) && override_value["visible"] != false)
            {
                let label = if string(override_value, "label").is_empty() {
                    legend_message(input, kind)
                } else {
                    string(override_value, "label")
                };
                // Legacy packed at 7px while drawing at 7.5px. Preserve its
                // authored geometry; the newer contract measures the drawn size.
                let measured_font = if input.input_contract == InputContract::Legacy217 {
                    7.0
                } else {
                    7.5
                };
                let width = (22.0
                    + units(input, label) * measured_font * 0.62
                    + if present { 21.0 } else { 0.0 })
                .ceil();
                items.push((kind.to_owned(), label.to_owned(), width));
            }
        }
    }
    if readable {
        for (_, _, width) in &items {
            *min_width = min_width.max(width + 40.0);
        }
    }
    let authored_box = document.pointer("/meta/viewBox").and_then(Value::as_array);
    let packing_width = if readable {
        *min_width
    } else {
        authored_box
            .map(|value| value[0].as_f64().unwrap())
            .unwrap_or(*min_width)
    } - 40.0;
    let mut rows: Vec<Vec<(String, String, f64)>> = Vec::new();
    let mut row_width = 0.0;
    for item in items {
        if rows.is_empty() {
            rows.push(Vec::new());
        }
        if !rows.last().unwrap().is_empty() && row_width + 7.0 + item.2 > packing_width {
            rows.push(Vec::new());
            row_width = 0.0;
        }
        row_width += if row_width > 0.0 { 7.0 } else { 0.0 };
        row_width += item.2;
        rows.last_mut().unwrap().push(item);
    }
    let extra = if readable {
        rows.len().saturating_sub(1) as f64 * 22.0
    } else {
        0.0
    };
    if let Some((kind, _, width)) = rows
        .iter()
        .flatten()
        .find(|(_, _, width)| *width > packing_width)
    {
        if input.document.pointer("/meta/legend").is_none() {
            return Ok((Vec::new(), extra));
        }
        return Err(vec![failure(
            "legend/label-too-wide",
            &format!("/meta/legend/entries/{kind}/label"),
            "Legend label exceeds the available packing width",
            json!({"kind":kind,"measuredWidthPx":width,"availableWidthPx":packing_width}),
        )]);
    }
    let legend_y = last_bottom + 44.0 + extra;
    let mut legends = Vec::new();
    let row_count = rows.len();
    let legend_top = legend_y - row_count.saturating_sub(1) as f64 * 22.0 - 30.0;
    if row_count > 0 && legend_top < last_bottom + 8.0 {
        if input.document.pointer("/meta/legend").is_none() {
            return Ok((Vec::new(), extra));
        }
        return Err(vec![failure(
            "legend/vertical-overflow",
            "/meta/legend",
            "Legend rows exceed the available legend band",
            json!({"rowCount":row_count,"requiredTopY":legend_top,"availableTopY":last_bottom+8.0}),
        )]);
    }
    if row_count > 0 {
        let title_y = legend_y - (row_count - 1) as f64 * 22.0 - 20.0;
        let source = legend_source(input, entries, "title");
        legends.push(Card {
            authored: Authored {
                id: "legend-title".into(),
                source_path: source.clone(),
                kind: "legend-title".into(),
                facts: input.document.pointer(&source).unwrap().clone(),
                geometry_provenance: Vec::new(),
            },
            bounds: Rect {
                x: 20.0,
                y: title_y - 10.0,
                width: 48.0,
                height: 14.0,
            },
            text: vec![text(
                source,
                legend_message(input, "title"),
                20.0,
                title_y,
                12.0,
                "start",
            )],
        });
    }
    for (row_index, row) in rows.into_iter().enumerate() {
        let mut x = 20.0;
        let baseline = legend_y - (row_count - row_index - 1) as f64 * 22.0;
        for (kind, label, width) in row {
            let source = legend_source(input, entries, &kind);
            let label_source = if input.document.pointer(&format!("{source}/label")).is_some() {
                format!("{source}/label")
            } else {
                source.clone()
            };
            legends.push(Card {
                authored: Authored {
                    id: format!("legend-{kind}"),
                    source_path: source.clone(),
                    kind: kind.clone(),
                    facts: input.document.pointer(&source).unwrap().clone(),
                    geometry_provenance: Vec::new(),
                },
                bounds: Rect {
                    x,
                    y: baseline - 10.0,
                    width,
                    height: 14.0,
                },
                text: vec![text(label_source, &label, x + 22.0, baseline, 7.5, "start")],
            });
            x += width + 7.0;
        }
    }
    Ok((legends, extra))
}

#[allow(clippy::too_many_arguments)]
fn measure_containers(
    input: &DiagramInput,
    lanes: &[Entry],
    phases: &[Entry],
    groups: &[Entry],
    entries: &[Entry],
    nodes: &[Node],
    widths: &BTreeMap<String, f64>,
    columns: &[f64],
    tops: &[f64],
    heights: &[f64],
    lane_width: f64,
    readable: bool,
) -> (Vec<Container>, Vec<Rect>) {
    let mut containers = Vec::new();
    let mut obstacles = Vec::new();
    for (index, lane) in lanes.iter().enumerate() {
        let bounds = Rect {
            x: 40.0,
            y: tops[index],
            width: lane_width,
            height: heights[index],
        };
        let prefix = if string(&lane.value, "variant") == "exception" {
            "EX".into()
        } else {
            format!("{:02}", index + 1)
        };
        let label = format!("{prefix} / {}", string(&lane.value, "label"));
        let lane_text = text(
            format!("/lanes/{}/label", lane.index),
            &label,
            54.0,
            tops[index] + 22.0,
            10.0,
            "start",
        );
        obstacles.push(Rect {
            x: 54.0,
            y: tops[index] + 12.0,
            width: units(input, &label) * 6.2,
            height: 14.0,
        });
        containers.push(Container {
            authored: authored(lane, "lanes", "lane", &[]),
            bounds,
            label_plate: None,
            rule: None,
            members: nodes
                .iter()
                .filter(|node| node.authored.facts["lane"] == lane.value["id"])
                .map(|node| node.authored.id.clone())
                .collect(),
            text: vec![lane_text],
        });
        if string(&lane.value, "variant") == "exception" {
            let mut author = authored(lane, "lanes", "exception-lane", &[]);
            author.id = format!("{}-exception", author.id);
            containers.push(Container {
                authored: author,
                label_plate: None,
                rule: None,
                bounds: Rect {
                    x: 46.0,
                    y: tops[index] + 6.0,
                    width: lane_width - 12.0,
                    height: heights[index] - 12.0,
                },
                members: Vec::new(),
                text: Vec::new(),
            });
        }
    }
    for phase in phases {
        let from = number(&phase.value, "fromCol", 0.0) as usize;
        let to = number(&phase.value, "toCol", 0.0) as usize;
        let natural = columns[to] - columns[from] + 92.0;
        let width = if readable {
            natural.max(units(input, string(&phase.value, "label")) * 5.6 + 8.0)
        } else {
            natural
        };
        let x = if from == to && width > natural {
            columns[from] - 46.0
        } else {
            (columns[from] + columns[to] - width) / 2.0
        };
        let bounds = Rect {
            x,
            y: 27.0,
            width,
            height: 16.0,
        };
        obstacles.push(bounds);
        containers.push(Container {
            authored: authored(phase, "phases", "phase", &["fromCol", "toCol"]),
            bounds,
            rule: Some([
                Point {
                    x: x + 8.0,
                    y: 35.0,
                },
                Point {
                    x: x + width - 8.0,
                    y: 35.0,
                },
            ]),
            label_plate: Some(Rect {
                x: x + width / 2.0
                    - (width - 12.0).min(
                        30.0_f64.max(units(input, string(&phase.value, "label")) * 4.8 + 10.0)
                            + 10.0,
                    ) / 2.0,
                y: 27.0,
                width: (width - 12.0).min(
                    30.0_f64.max(units(input, string(&phase.value, "label")) * 4.8 + 10.0) + 10.0,
                ),
                height: 16.0,
            }),
            members: nodes
                .iter()
                .filter(|node| {
                    number(&node.authored.facts, "col", 0.0) >= from as f64
                        && number(&node.authored.facts, "col", 0.0) <= to as f64
                })
                .map(|node| node.authored.id.clone())
                .collect(),
            text: vec![text(
                format!("/phases/{}/label", phase.index),
                string(&phase.value, "label"),
                x + width / 2.0,
                38.5,
                8.0,
                "middle",
            )],
        });
    }
    for group in groups {
        let index = lanes
            .iter()
            .position(|lane| lane.value["id"] == group.value["lane"])
            .unwrap();
        let (x, width) = group_span(input, &group.value, columns, entries, widths, readable);
        let y = tops[index] + 38.0;
        let bounds = Rect {
            x,
            y,
            width,
            height: if readable {
                heights[index] - 42.0
            } else {
                58.0
            },
        };
        let label_y = if readable { y - 2.0 } else { y + 14.0 };
        obstacles.push(Rect {
            x: x + 10.0,
            y: label_y - 10.0,
            width: units(input, string(&group.value, "label")) * 5.6,
            height: 14.0,
        });
        containers.push(Container {
            authored: authored(group, "groups", "group", &["lane", "fromCol", "toCol"]),
            bounds,
            rule: None,
            label_plate: None,
            members: nodes
                .iter()
                .filter(|node| {
                    node.authored.facts["lane"] == group.value["lane"]
                        && number(&node.authored.facts, "col", 0.0)
                            >= number(&group.value, "fromCol", 0.0)
                        && number(&node.authored.facts, "col", 0.0)
                            <= number(&group.value, "toCol", 0.0)
                })
                .map(|node| node.authored.id.clone())
                .collect(),
            text: vec![text(
                format!("/groups/{}/label", group.index),
                string(&group.value, "label"),
                x + 10.0,
                label_y,
                7.0,
                "start",
            )],
        });
    }
    (containers, obstacles)
}

pub(super) fn finalize(
    input: &DiagramInput,
    measured: &Measurement,
    scene: &mut CompiledDiagram,
) -> Result<(), Diagnostics> {
    if !measured.readable {
        return Ok(());
    }
    let mut left = 40.0_f64;
    let mut top = 27.0_f64;
    let mut right = 40.0 + measured.lane_width;
    let mut bottom =
        measured.lane_tops.last().unwrap() + measured.lane_heights.last().unwrap() + 62.0;
    let mut include = |rect: Rect| {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    };
    for node in &scene.nodes {
        include(node.bounds);
    }
    for container in &scene.containers {
        include(container.bounds);
    }
    for legend in &scene.legends {
        include(legend.bounds);
    }
    for relation in &scene.relations {
        for point in &relation.points {
            include(Rect {
                x: point.x,
                y: point.y,
                width: 0.0,
                height: 0.0,
            });
        }
        if let Some(plate) = relation.label_plate {
            include(plate);
        }
    }
    let required = [
        measured.minimum_width.max((right + 16.0).ceil()),
        measured.auto_height.max((bottom + 18.0).ceil()),
    ];
    let authored = input.document.pointer("/meta/viewBox").is_some();
    if left < 0.0
        || top < 0.0
        || authored && (scene.view_box.width < required[0] || scene.view_box.height < required[1])
    {
        let mut diagnostic = failure(
            if left < 0.0 || top < 0.0 {
                if absolute(&input.document) {
                    "workflow/explicit-pin-conflict"
                } else {
                    "workflow/solver-budget-exhausted"
                }
            } else {
                "workflow/viewbox-capacity"
            },
            "/meta/viewBox",
            "Authored viewBox cannot contain the measured workflow",
            json!({"actualViewBox":[scene.view_box.width,scene.view_box.height],"requiredViewBox":required,"contentBounds":[left,top,right,bottom],"measuredScene":scene}),
        );
        diagnostic.supported_fixes.push(format!(
            "Set /meta/viewBox to at least [{}, {}]",
            required[0], required[1]
        ));
        return Err(vec![diagnostic]);
    }
    if !authored {
        scene.view_box.width = required[0];
        scene.view_box.height = required[1];
    }
    Ok(())
}

pub(super) fn validate_readable_text(
    input: &DiagramInput,
    scene: &mut CompiledDiagram,
) -> Result<(), Diagnostics> {
    let readable = scene.schema_version == 2;
    let implicit = input.document.pointer("/meta/viewBox").is_none();
    let phoenix = input.input_contract == InputContract::PhoenixV1;
    if phoenix && readable && implicit {
        let stack = scene.nodes.iter().enumerate().any(|(index, a)| {
            scene.nodes[index + 1..].iter().any(|b| {
                a.authored.facts["lane"] == b.authored.facts["lane"]
                    && a.authored.facts["col"] == b.authored.facts["col"]
                    && number(&a.authored.facts, "yOffset", 0.0)
                        != number(&b.authored.facts, "yOffset", 0.0)
            })
        });
        let tall_lane = scene
            .containers
            .iter()
            .any(|container| container.authored.kind == "lane" && container.bounds.height > 104.0);
        let desktop_overflow = scene.view_box.width / scene.view_box.height < 1.55
            && (1346.0 * scene.view_box.height / scene.view_box.width).round() + 126.0 > 900.0;
        if stack && tall_lane || desktop_overflow {
            scene.reader_fit = "width-first".into();
        }
    }
    if !phoenix || !readable {
        return Ok(());
    }
    let minimum = if input.quality == crate::input::Quality::Showcase && implicit {
        6.0_f64.max((6.0 / (930.0 / scene.view_box.width).min(1.0) * 10.0).ceil() / 10.0)
    } else {
        6.0
    };
    let mut diagnostics = Vec::new();
    for node in &mut scene.nodes {
        let content = string(&node.authored.facts, "sublabel");
        if content.is_empty() {
            continue;
        }
        let needed = units(input, content) * minimum * 0.6;
        if minimum > 8.0 || needed > node.bounds.width - 8.0 {
            diagnostics.push(failure("workflow/sublabel-readability",&format!("{}/sublabel",node.authored.source_path),"Workflow sublabel does not fit the final desktop readability floor",json!({"requiredFontPx":minimum,"maximumSlotFontPx":8,"canvasWidthPx":scene.view_box.width,"maximumCanvasWidthPx":1240,"requiredTextWidthPx":needed,"availableTextWidthPx":node.bounds.width-8.0})));
        }
        if let Some(row) = node
            .text
            .iter_mut()
            .find(|row| row.source_path.ends_with("/sublabel"))
        {
            row.font_size = fitted_node_font_size_for_contract(
                content,
                node.bounds.width,
                8.0,
                minimum.min(8.0),
                input.input_contract,
            );
        }
    }
    if diagnostics.is_empty() {
        return Ok(());
    }
    for diagnostic in &mut diagnostics {
        diagnostic.evidence["measuredScene"] =
            serde_json::to_value(&scene).expect("measured scene JSON");
    }
    Err(diagnostics)
}

pub(super) fn absolute_sizing_warning(
    input: &DiagramInput,
    measured: &Measurement,
) -> Option<Diagnostic> {
    if input.input_contract != InputContract::PhoenixV1
        || !measured.readable
        || !absolute(&input.document)
    {
        return None;
    }
    let mut affected = Vec::new();
    for node in &measured.nodes {
        if node.authored.facts.get("width").is_some() {
            continue;
        }
        let auto = automatic_width(input, &node.authored.facts);
        if auto <= node.bounds.width {
            continue;
        }
        let fits = |width: f64| {
            units(input, string(&node.authored.facts, "label")) * 6.8 <= width + 6.0
                && ["sublabel", "tag"].iter().all(|field| {
                    units(input, string(&node.authored.facts, field)) * 6.0 * 0.6 <= width - 8.0
                })
        };
        if fits(node.bounds.width) || !fits(auto) {
            continue;
        }
        affected.push(json!({"node":node.authored.id,"path":format!("{}/width",node.authored.source_path),"currentWidthPx":node.bounds.width,"automaticWidthPx":auto}));
    }
    if affected.is_empty() {
        return None;
    }
    let mut constraints = Vec::new();
    if let Some(view_box) = input.document.pointer("/meta/viewBox") {
        constraints.push(json!({"path":"/meta/viewBox","value":view_box}));
    }
    for (index, edge) in array(&input.document, "edges").iter().enumerate() {
        for field in ["via", "labelAt", "channelX", "channelY"] {
            if let Some(value) = edge.get(field) {
                constraints.push(json!({"path":format!("/edges/{index}/{field}"),"value":value}));
            }
        }
    }
    let mut diagnostic=failure("workflow/absolute-geometry-sizing","/meta","Preserving absolute geometry disables automatic node widening; explicit width changes also require remeasuring route endpoints",json!({"absoluteConstraints":constraints,"affectedNodes":affected}));
    diagnostic.severity = crate::diagnostic::Severity::Warning;
    Some(diagnostic)
}
