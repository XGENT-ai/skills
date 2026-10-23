use super::*;

#[derive(Clone)]
pub(super) struct Boundary {
    pub raw: Value,
    pub index: usize,
    pub bounds: Rect,
    pub member_top: f64,
    pub title: Rect,
    pub font: f64,
}
#[derive(Clone)]
pub(super) struct LegendEntry {
    kind: String,
    label: String,
    width: f64,
}

pub(super) fn measure_boundaries(
    document: &Value,
    nodes: &[Node],
    contract: InputContract,
) -> Vec<Boundary> {
    let mut boundaries = vec![];
    let enforce = document["meta"].get("quality_profile").is_some();
    for (index, raw) in list(&document["boundaries"]).iter().enumerate() {
        let members: Vec<&Node> = nodes
            .iter()
            .filter(|node| {
                list(&raw["wraps"])
                    .iter()
                    .any(|id| id.as_str() == Some(&node.authored.id))
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let min_x = members
            .iter()
            .map(|node| node.bounds.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = members
            .iter()
            .map(|node| node.bounds.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = members
            .iter()
            .map(|node| node.bounds.x + node.bounds.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = members
            .iter()
            .map(|node| node.bounds.y + node.bounds.height)
            .fold(f64::NEG_INFINITY, f64::max);
        let pad = number(raw, "pad", 30.0);
        let top = pad.max(if contract == InputContract::PhoenixV1 && enforce {
            24.0
        } else {
            22.0
        });
        boundaries.push(Boundary {
            raw: raw.clone(),
            index,
            bounds: Rect {
                x: min_x - pad,
                y: min_y - top,
                width: max_x - min_x + pad * 2.0,
                height: max_y - min_y + top + 20.0,
            },
            member_top: min_y,
            title: Rect::default(),
            font: 6.0,
        });
    }
    boundaries
}

pub(super) fn legend_entries(
    meta: &Value,
    nodes: &[Node],
    contract: InputContract,
) -> Vec<LegendEntry> {
    let config = &meta["legend"];
    let mode = config["mode"].as_str().unwrap_or("auto");
    if mode == "hidden" {
        return vec![];
    }
    let locale = meta["locale"].as_str().unwrap_or("en").to_lowercase();
    let labels = match locale.as_str() {
        "zh-cn" => [
            "前端",
            "后端",
            "数据库",
            "云服务",
            "安全",
            "消息总线",
            "外部系统",
        ],
        "zh-tw" if contract == InputContract::PhoenixV1 => [
            "前端",
            "後端",
            "資料庫",
            "雲端服務",
            "安全",
            "訊息匯流排",
            "外部系統",
        ],
        "es" if contract == InputContract::PhoenixV1 => [
            "Frontend",
            "Backend",
            "Base de datos",
            "Nube",
            "Seguridad",
            "Bus de mensajes",
            "Externo",
        ],
        "ko" if contract == InputContract::PhoenixV1 => [
            "프런트엔드",
            "백엔드",
            "데이터베이스",
            "클라우드",
            "보안",
            "메시지 버스",
            "외부",
        ],
        _ => [
            "Frontend",
            "Backend",
            "Database",
            "Cloud",
            "Security",
            "Message bus",
            "External",
        ],
    };
    [
        "frontend",
        "backend",
        "database",
        "cloud",
        "security",
        "messagebus",
        "external",
    ]
    .iter()
    .enumerate()
    .filter_map(|(index, kind)| {
        let raw = &config["entries"][kind];
        let present = nodes.iter().any(|node| node.authored.kind == *kind);
        if raw["visible"] != true && (!(mode == "all" || present) || raw["visible"] == false) {
            return None;
        }
        let key = format!("legend.architecture.{kind}");
        let translated = translation(meta, &key, labels[index], contract);
        let label = raw["label"]
            .as_str()
            .filter(|label| !label.is_empty())
            .unwrap_or(&translated)
            .to_owned();
        let measured_font = if contract == InputContract::Legacy217 {
            8.0
        } else {
            10.0
        };
        let width = (14.0
            + 8.0
            + text_units_for_contract(&label, contract) * measured_font * 0.62
            + if present { 21.0 } else { 0.0 })
        .ceil();
        Some(LegendEntry {
            kind: (*kind).into(),
            label,
            width,
        })
    })
    .collect()
}

pub(super) fn legend_rows(entries: &[LegendEntry], width: f64) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return vec![];
    }
    let mut rows = vec![vec![]];
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        let row = rows.last_mut().unwrap();
        let required = entry.width + if row.is_empty() { 0.0 } else { 22.0 };
        if !row.is_empty() && cursor + required > width {
            rows.push(vec![index]);
            cursor = entry.width;
        } else {
            row.push(index);
            cursor += required;
        }
    }
    rows
}

pub(super) fn auto_canvas(
    nodes: &[Node],
    boundaries: &[Boundary],
    extra: &[Rect],
    entries: &[LegendEntry],
) -> Rect {
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for rect in nodes
        .iter()
        .map(|node| &node.bounds)
        .chain(boundaries.iter().map(|boundary| &boundary.bounds))
        .chain(extra.iter())
    {
        max_x = max_x.max(rect.x + rect.width);
        max_y = max_y.max(rect.y + rect.height);
    }
    let mut width = (max_x + 40.0).ceil();
    let minimum = entries.iter().map(|entry| entry.width).fold(0.0, f64::max);
    if minimum > width - 80.0 {
        width = (minimum + 80.0).ceil();
    }
    let rows = legend_rows(entries, (width - 80.0).max(1.0));
    Rect {
        x: 0.0,
        y: 0.0,
        width,
        height: (max_y + 68.0 + rows.len().saturating_sub(1) as f64 * 22.0).ceil(),
    }
}

fn title_width(label: &str, font: f64, contract: InputContract) -> f64 {
    (text_units_for_contract(label, contract) * font * 0.6 + 10.0).max(30.0)
}

fn marker_bounds(route: &router::Route, edge: &router::Edge) -> Option<Rect> {
    let pair = route
        .points
        .windows(2)
        .rev()
        .find(|pair| pair[0] != pair[1])?;
    let end = pair[1];
    let length = (end.x - pair[0].x).hypot(end.y - pair[0].y);
    let (dx, dy) = ((end.x - pair[0].x) / length, (end.y - pair[0].y) / length);
    let stroke = number(
        &edge.raw,
        "width",
        if edge.raw["variant"] == "emphasis" {
            1.8
        } else {
            1.5
        },
    );
    // The shared SVG marker is 10x7, referenced at (9, 3.5), in stroke units.
    let points = [(-9.0, -3.5), (1.0, 0.0), (-9.0, 3.5)].map(|(x, y)| Point {
        x: end.x + (dx * x - dy * y) * stroke,
        y: end.y + (dy * x + dx * y) * stroke,
    });
    let x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    Some(Rect {
        x,
        y,
        width: points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max)
            - x,
        height: points
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max)
            - y,
    })
}

#[allow(clippy::too_many_arguments)]
fn titles(
    raw: &[Boundary],
    nodes: &[Node],
    routes: &[router::Route],
    labels: &[router::Label],
    minimum: f64,
    enforce: bool,
    contract: InputContract,
    markers: &[Rect],
) -> Vec<Boundary> {
    let mut order: Vec<usize> = (0..raw.len()).collect();
    order.sort_by(|a, b| {
        (raw[*a].bounds.width * raw[*a].bounds.height)
            .total_cmp(&(raw[*b].bounds.width * raw[*b].bounds.height))
            .then(a.cmp(b))
    });
    let mut measured = raw.to_vec();
    let mut placed: Vec<Rect> = vec![];
    let mut placed_frames: Vec<Rect> = vec![];
    for index in order {
        let boundary = &mut measured[index];
        let label = string(&boundary.raw, "label");
        if enforce {
            let extra =
                (title_width(&label, minimum, contract) + 8.0 - boundary.bounds.width).max(0.0);
            boundary.bounds.x -= extra / 2.0;
            boundary.bounds.width += extra;
        }
        let available = (boundary.bounds.width - 8.0).max(0.0);
        let units = text_units_for_contract(&label, contract);
        let fitted = if units > 0.0 {
            (available - 10.0) / (units * 0.6)
        } else {
            9.0
        };
        let font = minimum.max(fitted.min(9.0_f64.max(minimum)));
        let height = (font + 7.0).ceil().max(16.0);
        let mut title = Rect {
            x: boundary.bounds.x + 4.0,
            y: boundary.member_top - 4.0 - height,
            width: available.min(title_width(&label, font, contract)),
            height,
        };
        for _ in 0..raw.len() + nodes.len() + 1 {
            let blockers: Vec<&Rect> = placed
                .iter()
                .chain(nodes.iter().map(|node| &node.bounds))
                .filter(|rect| rects_overlap_for_contract(&title, rect, 0.0, contract))
                .collect();
            if blockers.is_empty() {
                break;
            }
            title.y = blockers
                .iter()
                .map(|rect| rect.y - 2.0 - title.height)
                .fold(f64::INFINITY, f64::min);
        }
        if enforce && contract == InputContract::PhoenixV1 {
            let clear = |candidate: Rect| {
                !routes
                    .iter()
                    .flat_map(|route| route.points.windows(2))
                    .any(|pair| segment_intersects_rect(pair[0], pair[1], &candidate, 2.0))
                    && !placed
                        .iter()
                        .chain(nodes.iter().map(|node| &node.bounds))
                        .chain(labels.iter().map(|label| &label.rect))
                        .chain(markers)
                        .any(|other| rects_overlap_for_contract(&candidate, other, 0.0, contract))
            };
            if !clear(title) {
                let left = boundary.bounds.x + 4.0;
                let right = boundary.bounds.x + boundary.bounds.width - 4.0 - title.width;
                let mut positions = vec![right];
                for segment in routes
                    .iter()
                    .flat_map(|route| route.points.windows(2))
                    .filter(|pair| {
                        pair[0].y.max(pair[1].y) >= title.y - 2.0
                            && pair[0].y.min(pair[1].y) <= title.y + title.height + 2.0
                    })
                {
                    positions.push(segment[0].x.max(segment[1].x) + 8.0);
                    positions.push(segment[0].x.min(segment[1].x) - 8.0 - title.width);
                }
                for label in labels.iter().filter(|label| {
                    label.rect.y < title.y + title.height
                        && label.rect.y + label.rect.height > title.y
                }) {
                    positions.push(label.rect.x + label.rect.width + 8.0);
                    positions.push(label.rect.x - 8.0 - title.width);
                }
                positions.sort_by(f64::total_cmp);
                if let Some(x) = positions
                    .into_iter()
                    .find(|x| *x >= left && *x <= right && clear(Rect { x: *x, ..title }))
                {
                    title.x = x;
                } else {
                    // A narrow wrap can have no clear top rail. Keep authored
                    // routes and nodes; put its measured title below the members.
                    let bottom = boundary.bounds.y + boundary.bounds.height;
                    boundary.bounds.y = boundary.bounds.y.min(title.y - 4.0);
                    boundary.bounds.height = bottom - boundary.bounds.y;
                    let first = bottom - title.height - 2.0;
                    let mut rows = vec![first];
                    rows.extend(
                        placed
                            .iter()
                            .chain(nodes.iter().map(|node| &node.bounds))
                            .chain(labels.iter().map(|label| &label.rect))
                            .chain(markers)
                            .chain(&placed_frames)
                            .map(|rect| rect.y + rect.height + 2.0),
                    );
                    rows.extend(
                        routes
                            .iter()
                            .flat_map(|route| route.points.iter().map(|point| point.y + 4.0)),
                    );
                    rows.sort_by(f64::total_cmp);
                    if let Some(candidate) = rows
                        .into_iter()
                        .filter(|y| *y >= first)
                        .map(|y| Rect { y, ..title })
                        .find(|candidate| {
                            clear(*candidate)
                                && !placed_frames.iter().any(|frame| {
                                    rects_overlap_for_contract(candidate, frame, 2.0, contract)
                                })
                        })
                    {
                        title = candidate;
                        boundary.bounds.height =
                            bottom.max(title.y + title.height + 2.0) - boundary.bounds.y;
                    }
                }
            }
        }
        placed.push(title);
        let bottom = boundary.bounds.y + boundary.bounds.height;
        if enforce {
            boundary.bounds.y = boundary.bounds.y.min(title.y - 4.0);
            boundary.bounds.height = bottom - boundary.bounds.y;
        }
        boundary.title = title;
        boundary.font = font;
        placed_frames.push(boundary.bounds);
    }
    measured
}

// Keep source geometry and independent title/readability constraints explicit.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_titles(
    raw: &[Boundary],
    nodes: &[Node],
    routes: &[router::Route],
    edges: &[router::Edge],
    labels: &[router::Label],
    extra: &[Rect],
    entries: &[LegendEntry],
    canvas: Option<Rect>,
    enforce: bool,
    contract: InputContract,
    errors: &mut Diagnostics,
) -> Vec<Boundary> {
    let markers: Vec<_> = routes
        .iter()
        .zip(edges)
        .filter_map(|(route, edge)| marker_bounds(route, edge))
        .collect();
    let mut boundaries = titles(raw, nodes, routes, labels, 6.0, enforce, contract, &markers);
    if !enforce || boundaries.is_empty() {
        return boundaries;
    }
    for _ in 0..32 {
        let width = canvas
            .unwrap_or_else(|| auto_canvas(nodes, &boundaries, extra, entries))
            .width;
        let minimum = 6.0_f64.max(6.0 / (930.0 / width).min(1.0) + 1e-6);
        boundaries = titles(
            raw, nodes, routes, labels, minimum, enforce, contract, &markers,
        );
        let final_width = canvas
            .unwrap_or_else(|| auto_canvas(nodes, &boundaries, extra, entries))
            .width;
        if minimum >= 6.0_f64.max(6.0 / (930.0 / final_width).min(1.0)) {
            return boundaries;
        }
    }
    errors.push(diagnostic(
        "composition/desktop-readability",
        "/boundaries",
        "Boundary title readability did not converge after 32 iterations",
        Value::Null,
    ));
    boundaries
}

// Legend placement checks each source geometry collection independently.
#[allow(clippy::too_many_arguments)]
pub(super) fn legends(
    entries: &[LegendEntry],
    nodes: &[Node],
    boundaries: &[Boundary],
    routes: &[router::Route],
    labels: &[router::Label],
    canvas: Rect,
    meta: &Value,
    contract: InputContract,
    errors: &mut Diagnostics,
) -> Vec<Card> {
    if entries.is_empty() {
        return vec![];
    }
    let rows = legend_rows(entries, canvas.width - 80.0);
    let extra_height = rows.len().saturating_sub(1) as f64 * 22.0;
    let title_y = canvas.height - 36.0 - extra_height;
    let content_bottom = nodes
        .iter()
        .map(|node| node.bounds.y + node.bounds.height)
        .chain(
            boundaries
                .iter()
                .map(|boundary| boundary.bounds.y + boundary.bounds.height),
        )
        .fold(0.0, f64::max);
    let too_wide = entries
        .iter()
        .any(|entry| entry.width > canvas.width - 80.0);
    let too_high = title_y - 10.0 < content_bottom + 8.0;
    if too_wide || too_high {
        if meta.get("legend").is_some() {
            errors.push(diagnostic(
                if too_wide {
                    "legend/label-too-wide"
                } else {
                    "legend/vertical-overflow"
                },
                "/meta/legend",
                "Legend does not fit the measured canvas",
                json!({"viewBox":canvas,"rowCount":rows.len()}),
            ));
        }
        return vec![];
    }
    let title = match meta["locale"]
        .as_str()
        .unwrap_or("en")
        .to_lowercase()
        .as_str()
    {
        "zh-cn" => "图例",
        "zh-tw" if contract == InputContract::PhoenixV1 => "圖例",
        "es" if contract == InputContract::PhoenixV1 => "Leyenda",
        "ko" if contract == InputContract::PhoenixV1 => "범례",
        _ => "Legend",
    };
    let title = translation(meta, "legend.title", title, contract);
    let (title_source, title_facts) = legend_source(meta, nodes, "title", &title);
    let mut cards = vec![Card {
        authored: authored(
            &title_facts,
            title_source.clone(),
            "legend-title".into(),
            "legend-title".into(),
            vec!["resolved-legend-footprint".into()],
        ),
        bounds: Rect {
            x: 40.0,
            y: title_y - 10.0,
            width: 48.0,
            height: 14.0,
        },
        text: vec![text(title_source, title, 40.0, title_y, 12.0, "start")],
    }];
    for (row_index, row) in rows.iter().enumerate() {
        let mut x = 40.0;
        let baseline = canvas.height - 16.0 - (rows.len() - row_index - 1) as f64 * 22.0;
        for index in row {
            let entry = &entries[*index];
            let (source, facts) = legend_source(meta, nodes, &entry.kind, &entry.label);
            cards.push(Card {
                authored: authored(
                    &facts,
                    source.clone(),
                    format!("legend-{}", entry.kind),
                    entry.kind.clone(),
                    vec!["resolved-legend-footprint".into()],
                ),
                bounds: Rect {
                    x,
                    y: baseline - 10.0,
                    width: entry.width,
                    height: 14.0,
                },
                text: vec![text(
                    if facts.get("label").is_some() {
                        format!("{source}/label")
                    } else {
                        source
                    },
                    &entry.label,
                    x + 22.0,
                    baseline,
                    10.0,
                    "start",
                )],
            });
            x += entry.width + 22.0;
        }
    }
    let collision = cards.iter().any(|card| {
        routes
            .iter()
            .flat_map(|route| route.points.windows(2))
            .any(|segment| segment_intersects_rect(segment[0], segment[1], &card.bounds, 0.0))
            || labels
                .iter()
                .any(|label| rects_overlap_for_contract(&label.rect, &card.bounds, 0.0, contract))
    });
    if collision {
        if meta.get("legend").is_some() {
            errors.push(diagnostic(
                "legend/content-overlap",
                "/meta/legend",
                "Legend overlaps relationship geometry",
                Value::Null,
            ));
        }
        vec![]
    } else {
        cards
    }
}

fn translation(meta: &Value, key: &str, fallback: &str, contract: InputContract) -> String {
    if contract == InputContract::PhoenixV1 && meta["locale"].is_string() {
        if let Some(value) = meta["translations"][key]
            .as_str()
            .filter(|value| !value.is_empty())
        {
            let has_placeholder = value.split('{').skip(1).any(|part| {
                part.split_once('}').is_some_and(|(name, _)| {
                    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
            });
            if !has_placeholder {
                return value.into();
            }
        }
    }
    fallback.into()
}

fn legend_source(meta: &Value, nodes: &[Node], kind: &str, label: &str) -> (String, Value) {
    if let Some(raw) = meta.pointer(&format!("/legend/entries/{kind}")) {
        return (format!("/meta/legend/entries/{kind}"), raw.clone());
    }
    let key = if kind == "title" {
        "legend.title".into()
    } else {
        format!("legend.architecture.{kind}")
    };
    if meta["locale"].is_string() && meta["translations"][&key].as_str() == Some(label) {
        return (
            format!("/meta/translations/{key}"),
            meta["translations"][&key].clone(),
        );
    }
    if let Some(node) = nodes.iter().find(|node| node.authored.kind == kind) {
        return (
            format!("{}/type", node.authored.source_path),
            node.authored.facts["type"].clone(),
        );
    }
    if let Some(raw) = meta.get("legend") {
        ("/meta/legend".into(), raw.clone())
    } else {
        ("/meta".into(), meta.clone())
    }
}
