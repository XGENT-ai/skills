// Source: measureNode, stageHeaderText/renderStageHeader, renderNode and legend layout.
use super::*;
use crate::scene::{Card, Container, DiagramFacts, Node, Relation, Sigil};

pub(super) fn stage_x(index: usize) -> f64 {
    100.0 + index as f64 * 215.0
}
pub(super) fn frames(context: &Context<'_>) -> Vec<composition::Frame> {
    list(&context.input.document["stages"])
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let mut frame = composition::Frame::new(
                "stage",
                index.to_string(),
                Rect {
                    x: stage_x(index) - 84.0,
                    y: 46.0,
                    width: 168.0,
                    height: context.view_box.height - 120.0,
                },
            );
            frame.radius = 10.0;
            frame
        })
        .collect()
}

pub(super) fn create(input: &DiagramInput) -> Result<Context<'_>, Diagnostics> {
    let raw_nodes = list(&input.document["nodes"]);
    let stages = list(&input.document["stages"]);
    let legacy = input.input_contract == InputContract::Legacy217;
    let automatic_showcase = !legacy
        && input.quality == Quality::Showcase
        && input.document.pointer("/meta/viewBox").is_none()
        && !raw_nodes.iter().any(|node| node.get("width").is_some());
    let authored_view = input
        .document
        .pointer("/meta/viewBox")
        .map(|v| p(v[0].as_f64().unwrap(), v[1].as_f64().unwrap()));
    let mut content_right = stage_x(stages.len() - 1) + 84.0;
    for node in raw_nodes {
        content_right = content_right
            .max(100.0 + number(node, "stage", 0.0) * 215.0 + number(node, "width", 112.0) / 2.0);
    }
    for flow in list(&input.document["flows"]) {
        if flow["labelAt"].is_array() {
            content_right = content_right
                .max(point(&flow["labelAt"]).x + capacity::size(input, flow, &[]).x / 2.0);
        }
    }
    let view = authored_view.unwrap_or_else(|| {
        p(
            if legacy {
                940.0
            } else {
                480.0_f64.max((content_right + 24.0).ceil())
            },
            720.0,
        )
    });
    let context_minimum = if automatic_showcase {
        6.0_f64.max(((6.0 / (930.0 / view.x).min(1.0)) * 10.0).ceil() / 10.0)
    } else {
        6.0
    };
    let mut nodes = Vec::new();
    for (index, raw) in raw_nodes.iter().enumerate() {
        let stage = number(raw, "stage", f64::NAN);
        let row = number(raw, "row", f64::NAN);
        if !stage.is_finite()
            || stage < 0.0
            || stage >= stages.len() as f64
            || !row.is_finite()
            || row < 0.0
            || row >= 5.0
        {
            return Err(vec![failure(
                "layout/constraint",
                format!("/nodes/{index}"),
                "Node stage or row is outside the dataflow layout",
                json!({"stage":stage,"row":row}),
            )]);
        }
        let cx = 100.0 + stage * 215.0;
        let context_width = ["sublabel", "tag"]
            .iter()
            .map(|key| {
                let value = string(raw, key);
                if value.is_empty() {
                    0.0
                } else {
                    units(input, value) * 7.0_f64.max(context_minimum) * 0.6 + 8.0
                }
            })
            .fold(0.0, f64::max);
        let title_width = if raw.get("brand").is_some() {
            units(input, string(raw, "label")) * 10.0 * 0.6 + 8.0 + 48.0
        } else {
            0.0
        };
        let automatic_width = 112.0_f64.max(
            168.0_f64
                .min((cx - 24.0) * 2.0)
                .min((view.x - 24.0 - cx) * 2.0)
                .min(context_width.max(title_width).ceil()),
        );
        let width = number(
            raw,
            "width",
            if automatic_showcase {
                automatic_width
            } else {
                112.0
            },
        );
        let height = number(raw, "height", 58.0);
        let y = [128.0, 242.0, 356.0, 470.0, 584.0][row as usize] + number(raw, "yOffset", 0.0);
        nodes.push(MeasuredNode {
            raw: raw.clone(),
            bounds: Rect {
                x: cx - width / 2.0,
                y,
                width,
                height,
            },
            cx,
            cy: y + height / 2.0,
            inset: 0.0,
        });
    }
    let mut flows = Vec::new();
    for (index, raw) in list(&input.document["flows"]).iter().enumerate() {
        let find = |field| {
            nodes
                .iter()
                .position(|node| string(&node.raw, "id") == string(raw, field))
        };
        let (Some(from), Some(to)) = (find("from"), find("to")) else {
            return Err(vec![failure(
                "dataflow/unknown-endpoint",
                format!("/flows/{index}"),
                "Flow endpoint must refer to an existing node",
                Value::Null,
            )]);
        };
        let inferred = routing::inferred_sides(raw, &nodes[from], &nodes[to]);
        let side = |key, fallback: &str| {
            let value = string(raw, key);
            if value.is_empty() || value == "auto" {
                fallback.to_owned()
            } else {
                value.to_owned()
            }
        };
        flows.push(Flow {
            raw: raw.clone(),
            from,
            to,
            sides: (side("fromSide", inferred.0), side("toSide", inferred.1)),
            ports: Ports::default(),
            points: Vec::new(),
            halo: false,
        });
    }
    let mut context = Context {
        input,
        nodes,
        flows,
        labels: Vec::new(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width: view.x,
            height: view.y,
        },
        frames: Vec::new(),
        automatic_showcase,
        context_minimum,
    };
    context.frames = frames(&context);
    Ok(context)
}

pub(super) fn expand_heights(context: &mut Context<'_>) {
    if !context.automatic_showcase {
        return;
    }
    let originals = context.nodes.clone();
    for index in 0..context.nodes.len() {
        if has(&context.nodes[index].raw, &["height", "yOffset"]) {
            continue;
        }
        let incident: Vec<_> = context
            .flows
            .iter()
            .filter(|flow| flow.from == index || flow.to == index)
            .collect();
        if incident.iter().any(|flow| geometry_pinned(&flow.raw)) {
            continue;
        }
        let mut sides: [Vec<(bool, f64)>; 2] = [Vec::new(), Vec::new()];
        for flow in incident {
            for (endpoint, side, incoming) in [
                (flow.from, flow.sides.0.as_str(), false),
                (flow.to, flow.sides.1.as_str(), true),
            ] {
                if endpoint == index {
                    if let Some(slot) = match side {
                        "left" => Some(0),
                        "right" => Some(1),
                        _ => None,
                    } {
                        sides[slot].push((incoming, routing::stroke_width(&flow.raw)));
                    }
                }
            }
        }
        let mut desired = context.nodes[index].bounds.height;
        for items in &sides {
            if items.iter().any(|item| item.0) {
                desired = desired.max(routing::port_span(items));
            }
        }
        let node = &context.nodes[index];
        let mut height = desired.min(104.0).min(2.0 * (node.cy - 104.0));
        for (other_index, other) in originals.iter().enumerate() {
            if other_index == index
                || other.bounds.x >= node.bounds.x + node.bounds.width + 10.0
                || other.bounds.x + other.bounds.width + 10.0 <= node.bounds.x
            {
                continue;
            }
            let distance = (other.cy - node.cy).abs();
            height = height.min((distance - 10.0).min(2.0 * distance - other.bounds.height - 20.0));
        }
        if height > node.bounds.height {
            let inset = (height - node.bounds.height) / 2.0;
            context.nodes[index].inset = inset;
            context.nodes[index].bounds.y -= inset;
            context.nodes[index].bounds.height = height;
        }
    }
}

fn label_layout(
    input: &DiagramInput,
    width: f64,
    height: f64,
    rows: &[(String, f64, f64)],
) -> (f64, Vec<f64>, f64, f64) {
    let mut x = width / 2.0;
    let mut ys: Vec<_> = rows.iter().map(|row| row.2).collect();
    let label_width = units(input, &rows[0].0) * rows[0].1 * 0.6;
    let left = 19.0;
    let right = width - 4.0;
    if x - label_width / 2.0 >= left && x + label_width / 2.0 <= right {
        return (x, ys, 6.0, 11.0);
    }
    if label_width <= right - left {
        x = ((right - label_width / 2.0) * 10.0).floor() / 10.0;
        x = x.min((width / 2.0).max(((left + label_width / 2.0) * 10.0).ceil() / 10.0));
        return (x, ys, 6.0, 11.0);
    }
    let mut bottom = 17.0_f64;
    let mut shifted = Vec::new();
    for row in rows {
        let y = row
            .2
            .max(((bottom + 2.0 + row.1 * 1.2) * 10.0).ceil() / 10.0);
        shifted.push(y);
        bottom = y + row.1 * 0.3;
    }
    if bottom <= height - 2.0 {
        ys = shifted;
        return (x, ys, 6.0, 11.0);
    }
    (
        x,
        ys,
        1.0,
        11.0_f64
            .min((rows[0].2 - rows[0].1 * 1.2 - 3.0).floor())
            .max(1.0),
    )
}

fn stage_text(context: &Context<'_>, raw: &Value, index: usize) -> Vec<Text> {
    let content = format!("{:02} / {}", index + 1, string(raw, "label"));
    let x = stage_x(index);
    let path = format!("/stages/{index}/label");
    if context.legacy() {
        return vec![text(path, &content, p(x, 68.0), 9.0, "middle")];
    }
    let font = fitted(context.input, &content, 168.0, 9.0, 7.0);
    if units(context.input, &content) * font * 0.6 <= 160.0 {
        return vec![text(path, &content, p(x, 68.0), font, "middle")];
    }
    let mut words = Vec::new();
    let mut start = 0;
    let mut whitespace = None;
    for (offset, character) in content.char_indices() {
        let current = crate::geometry::text::is_js_whitespace(character);
        if whitespace.is_some_and(|previous| previous != current) {
            words.push(&content[start..offset]);
            start = offset;
        }
        whitespace = Some(current);
    }
    words.push(&content[start..]);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in words {
        if !line.is_empty() && units(context.input, &format!("{line}{word}")) * font * 0.6 > 160.0 {
            lines.push(std::mem::take(&mut line));
        }
        for cluster in crate::geometry::text::graphemes(word) {
            if !line.is_empty()
                && units(context.input, &format!("{line}{cluster}")) * font * 0.6 > 160.0
            {
                lines.push(std::mem::take(&mut line));
            }
            line.push_str(cluster);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    let first_node = context
        .nodes
        .iter()
        .filter(|node| number(&node.raw, "stage", 0.0) == index as f64)
        .map(|node| node.bounds.y)
        .fold(context.view_box.height - 74.0, f64::min);
    if 68.0 + lines.len().saturating_sub(1) as f64 * (font + 4.0) + font * 0.3 > first_node - 4.0 {
        return vec![text(path, &content, p(x, 68.0), font, "middle")];
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(line, content)| {
            text(
                path.clone(),
                &content,
                p(x, 68.0 + line as f64 * (font + 4.0)),
                font,
                "middle",
            )
        })
        .collect()
}

fn legend_label(input: &DiagramInput, index: usize) -> String {
    let kinds = [
        "title", "emphasis", "security", "dashed", "database", "default",
    ];
    let key = if index == 0 {
        "legend.title".into()
    } else {
        format!("legend.dataflow.{}", kinds[index])
    };
    if input.input_contract == InputContract::PhoenixV1 {
        if let Some(value) = input
            .document
            .pointer("/meta/translations")
            .and_then(|v| v.get(&key))
            .and_then(Value::as_str)
        {
            if !value.is_empty()
                && !value.split('{').skip(1).any(|suffix| {
                    suffix.split_once('}').is_some_and(|(key, _)| {
                        !key.is_empty()
                            && key
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    })
                })
            {
                return value.into();
            }
        }
    }
    let locale = string(&input.document["meta"], "locale").to_ascii_lowercase();
    let labels = match locale.as_str() {
        "zh-cn" => [
            "图例",
            "主要数据",
            "策略 / PII",
            "异步批处理",
            "数据存储",
            "数据流",
        ],
        "zh-tw" if input.input_contract == InputContract::PhoenixV1 => [
            "圖例",
            "主要資料",
            "政策 / 個資",
            "非同步批次處理",
            "資料儲存",
            "資料流",
        ],
        "es" if input.input_contract == InputContract::PhoenixV1 => [
            "Leyenda",
            "datos primarios",
            "política / PII",
            "lote asíncrono",
            "almacén de datos",
            "flujo de datos",
        ],
        "ko" if input.input_contract == InputContract::PhoenixV1 => [
            "범례",
            "주요 데이터",
            "정책 / PII",
            "비동기 배치",
            "데이터 저장소",
            "데이터 흐름",
        ],
        _ => [
            "Legend",
            "primary data",
            "policy / PII",
            "async batch",
            "data store",
            "data flow",
        ],
    };
    labels[index].into()
}

struct LegendEntry {
    kind: &'static str,
    label: String,
    width: f64,
    swatch: f64,
}
fn legend_entries(context: &Context<'_>) -> Vec<LegendEntry> {
    let config = &context.input.document["meta"]["legend"];
    if config["mode"] == "hidden" {
        return Vec::new();
    }
    ["emphasis", "security", "dashed", "database", "default"]
        .iter()
        .enumerate()
        .filter_map(|(index, &kind)| {
            let present = if kind == "database" {
                context.nodes.iter().any(|node| node.raw["type"] == kind)
            } else {
                context.flows.iter().any(|flow| {
                    let variant = string(&flow.raw, "variant");
                    if variant.is_empty() {
                        kind == "default"
                    } else {
                        variant == kind
                    }
                })
            };
            let entry = &config["entries"][kind];
            if entry["visible"] != true
                && (!(config["mode"] == "all" || present) || entry["visible"] == false)
            {
                return None;
            }
            let label = if string(entry, "label").is_empty() {
                legend_label(context.input, index + 1)
            } else {
                string(entry, "label").into()
            };
            let swatch = if kind == "database" { 14.0 } else { 34.0 };
            let gap = if kind == "database" { 8.0 } else { 9.0 };
            let badge = if kind == "database" && present {
                21.0
            } else {
                0.0
            };
            let measured_font = if context.legacy() { 8.0 } else { 10.0 };
            let width =
                (swatch + gap + units(context.input, &label) * measured_font * 0.62 + badge).ceil();
            Some(LegendEntry {
                kind,
                label,
                width,
                swatch: swatch + gap,
            })
        })
        .collect()
}
fn legend_rows(entries: &[LegendEntry], width: f64) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return Vec::new();
    }
    let mut rows = vec![Vec::new()];
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        let row = rows.last_mut().unwrap();
        let needed = entry.width + if row.is_empty() { 0.0 } else { 22.0 };
        if !row.is_empty() && cursor + needed > width {
            rows.push(vec![index]);
            cursor = entry.width;
        } else {
            row.push(index);
            cursor += needed;
        }
    }
    rows
}

pub(super) fn scene(mut context: Context<'_>) -> Result<CompiledDiagram, Diagnostics> {
    let entries = legend_entries(&context);
    if !context.legacy() && context.input.document.pointer("/meta/viewBox").is_none() {
        while context.view_box.width < 1600.0
            && legend_rows(&entries, (context.view_box.width - 80.0).max(1.0)).len() > 1
        {
            context.view_box.width += 40.0;
        }
    }
    let rows = legend_rows(&entries, context.view_box.width - 80.0);
    let explicit_legend = context.input.document.pointer("/meta/legend").is_some();
    let title_y = context.view_box.height - 56.0 - rows.len().saturating_sub(1) as f64 * 22.0;
    let too_wide = entries
        .iter()
        .find(|entry| entry.width > context.view_box.width - 80.0);
    let hidden =
        too_wide.is_some() || (!rows.is_empty() && title_y - 10.0 < context.view_box.height - 66.0);
    if hidden && explicit_legend {
        return Err(vec![failure(
            if too_wide.is_some() {
                "legend/label-too-wide"
            } else {
                "legend/vertical-overflow"
            },
            "/meta/legend",
            "Legend does not fit the authored dataflow canvas",
            json!({"rowCount":rows.len()}),
        )]);
    }
    let mut legends = Vec::new();
    if !hidden && !rows.is_empty() {
        legends.push(Card {
            authored: Authored {
                id: "legend-title".into(),
                source_path: "/meta/legend".into(),
                kind: "legend-title".into(),
                facts: context.input.document["meta"]["legend"].clone(),
                geometry_provenance: Vec::new(),
            },
            bounds: Rect {
                x: 40.0,
                y: title_y - 10.0,
                width: 48.0,
                height: 14.0,
            },
            text: vec![text(
                "/meta/legend".into(),
                &legend_label(context.input, 0),
                p(40.0, title_y),
                12.0,
                "start",
            )],
        });
        for (row_index, row) in rows.iter().enumerate() {
            let baseline =
                context.view_box.height - 36.0 - (rows.len() - row_index - 1) as f64 * 22.0;
            let mut x = 40.0;
            for &index in row {
                let entry = &entries[index];
                let path = format!("/meta/legend/entries/{}", entry.kind);
                legends.push(Card {
                    authored: Authored {
                        id: format!("legend-{}", entry.kind),
                        source_path: path.clone(),
                        kind: entry.kind.into(),
                        facts: context
                            .input
                            .document
                            .pointer(&path)
                            .cloned()
                            .unwrap_or(Value::Null),
                        geometry_provenance: Vec::new(),
                    },
                    bounds: Rect {
                        x,
                        y: baseline - 10.0,
                        width: entry.width,
                        height: 14.0,
                    },
                    text: vec![text(
                        format!("{path}/label"),
                        &entry.label,
                        p(x + entry.swatch, baseline),
                        10.0,
                        "start",
                    )],
                });
                x += entry.width + 22.0;
            }
        }
    }
    let input = context.input;
    let nodes = context
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let raw = &node.raw;
            let label = string(raw, "label");
            let has_sub = !string(raw, "sublabel").is_empty();
            let label_font = if raw.get("brand").is_some() {
                10.0
            } else {
                fitted(input, label, node.bounds.width, 10.0, 8.0)
            };
            let sub_font = fitted(
                input,
                string(raw, "sublabel"),
                node.bounds.width,
                7.0_f64.max(context.context_minimum),
                context.context_minimum,
            );
            let tag_font = fitted(
                input,
                string(raw, "tag"),
                node.bounds.width,
                7.0_f64.max(context.context_minimum),
                context.context_minimum,
            );
            let mut rows = vec![(label.to_owned(), label_font, 21.0 + node.inset)];
            if has_sub {
                rows.push((string(raw, "sublabel").into(), sub_font, 37.0 + node.inset));
            }
            if !string(raw, "tag").is_empty() {
                rows.push((
                    string(raw, "tag").into(),
                    tag_font,
                    node.bounds.height - 11.0 - node.inset,
                ));
            }
            let (label_x, ys, sigil_y, sigil_size) = if context.legacy() {
                (
                    node.bounds.width / 2.0,
                    rows.iter().map(|row| row.2).collect(),
                    6.0,
                    11.0,
                )
            } else {
                label_layout(input, node.bounds.width, node.bounds.height, &rows)
            };
            let path = format!("/nodes/{index}");
            let mut texts = vec![text(
                format!("{path}/label"),
                label,
                p(node.bounds.x + label_x, node.bounds.y + ys[0]),
                label_font,
                "middle",
            )];
            if has_sub {
                texts.push(text(
                    format!("{path}/sublabel"),
                    string(raw, "sublabel"),
                    p(node.cx, node.bounds.y + ys[1]),
                    sub_font,
                    "middle",
                ));
            }
            if !string(raw, "tag").is_empty() {
                texts.push(text(
                    format!("{path}/tag"),
                    string(raw, "tag"),
                    p(node.cx, node.bounds.y + ys[if has_sub { 2 } else { 1 }]),
                    tag_font,
                    "middle",
                ));
            }
            let kind = if context.legacy() || string(raw, "icon").is_empty() {
                string(raw, "type")
            } else {
                string(raw, "icon")
            };
            Node {
                brand: None,
                authored: authored(
                    raw,
                    "nodes",
                    index,
                    string(raw, "type"),
                    &["stage", "row", "width", "height", "yOffset"],
                ),
                bounds: node.bounds,
                label: label.into(),
                sigil: (kind != "none").then(|| Sigil {
                    kind: kind.into(),
                    position: p(node.bounds.x + 6.0, node.bounds.y + sigil_y),
                    size: sigil_size,
                }),
                text: texts,
                detail_rows: Vec::new(),
            }
        })
        .collect();
    let relations = context
        .flows
        .iter()
        .enumerate()
        .map(|(index, flow)| {
            let raw = &flow.raw;
            let plate = context
                .labels
                .iter()
                .find(|plate| plate.index() == index)
                .unwrap();
            let position = plate.label.position;
            let mut texts = Vec::new();
            let lines = if plate.lines.is_empty() {
                vec![string(raw, "label").to_owned()]
            } else {
                plate.lines.clone()
            };
            for (line, value) in lines.iter().enumerate() {
                texts.push(text(
                    format!("/flows/{index}/label"),
                    value,
                    p(position.x, position.y + 11.0 * line as f64),
                    8.0,
                    "middle",
                ));
            }
            if !string(raw, "classification").is_empty() {
                texts.push(text(
                    format!("/flows/{index}/classification"),
                    string(raw, "classification"),
                    p(position.x, position.y + 11.0 * lines.len() as f64),
                    7.0,
                    "middle",
                ));
            }
            Relation {
                authored: authored(
                    raw,
                    "flows",
                    index,
                    "flow",
                    &[
                        "fromSide",
                        "toSide",
                        "route",
                        "via",
                        "channelX",
                        "channelY",
                        "labelAt",
                        "labelDx",
                        "labelDy",
                        "labelSegment",
                        "width",
                    ],
                ),
                crossover_halo: flow.halo,
                independent_ports: false,
                junction: None,
                label_owner: None,
                from: string(raw, "from").into(),
                to: string(raw, "to").into(),
                variant: if string(raw, "variant").is_empty() {
                    "default".into()
                } else {
                    string(raw, "variant").into()
                },
                label: string(raw, "label").into(),
                note: raw["classification"].as_str().map(str::to_owned),
                note_bounds: None,
                note_backdrop: None,
                points: flow.points.clone(),
                label_plate: Some(plate.label.bounds),
                text: texts,
                marker_start: None,
                marker_end: None,
            }
        })
        .collect();
    let containers = list(&input.document["stages"])
        .iter()
        .enumerate()
        .map(|(index, raw)| Container {
            authored: authored(raw, "stages", index, "stage", &[]),
            bounds: context.frames[index].bounds,
            label_plate: None,
            rule: None,
            members: context
                .nodes
                .iter()
                .filter(|node| number(&node.raw, "stage", 0.0) == index as f64)
                .map(|node| string(&node.raw, "id").to_owned())
                .collect(),
            text: stage_text(&context, raw, index),
        })
        .collect();
    let cards = list(&input.document["cards"])
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let path = format!("/cards/{index}");
            let mut texts = vec![text(
                format!("{path}/title"),
                string(raw, "title"),
                p(0.0, 0.0),
                14.0,
                "start",
            )];
            texts.extend(list(&raw["items"]).iter().enumerate().map(|(i, value)| {
                text(
                    format!("{path}/items/{i}"),
                    value.as_str().unwrap(),
                    p(0.0, 0.0),
                    12.0,
                    "start",
                )
            }));
            Card {
                authored: authored(raw, "cards", index, string(raw, "dot"), &[]),
                bounds: Rect::default(),
                text: texts,
            }
        })
        .collect();
    Ok(CompiledDiagram {
        diagram_type: input.diagram_type,
        schema_version: input.document["schema_version"].as_f64().unwrap() as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: input.document["meta"].clone(),
        view_box: context.view_box,
        reader_fit: if !context.legacy() && input.document.pointer("/meta/viewBox").is_none() {
            "intrinsic-height".into()
        } else {
            String::new()
        },
        nodes,
        relations,
        containers,
        text: Vec::new(),
        legends,
        cards,
        facts: DiagramFacts::Dataflow {
            stages: list(&input.document["stages"]).to_vec(),
            nodes: list(&input.document["nodes"]).to_vec(),
            flows: list(&input.document["flows"]).to_vec(),
        },
        diagnostics: Vec::new(),
    })
}
