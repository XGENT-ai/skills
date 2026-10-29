// Ported from the frozen Archify 3.1.0 renderers/waterfall/render-waterfall.mjs; see NOTICE.md.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::format::number::{
    en_us_max_fraction, en_us_max_significant, js_round, js_to_precision, js_to_string,
};
use crate::geometry::text_units;
use crate::i18n::Locale;
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Text,
    WaterfallSpan,
};
use serde_json::{json, Value};

const MARGIN: f64 = 28.0;
const TOP: f64 = 52.0;
const AXIS_H: f64 = 36.0;
const ROW_H: f64 = 25.0;
pub(crate) const BAR_H: f64 = 14.0;
const INDENT: f64 = 18.0;
const NAME_FONT: f64 = 12.5;
const DURATION_FONT: f64 = 11.0;
const COLUMN_GAP: f64 = 28.0;
const MIN_TICK_PX: f64 = 80.0;
const ADVANCE: f64 = 0.6;
const NAME_DOT: f64 = 14.0;
const LABEL_GAP: f64 = 7.0;
const NICE: [f64; 4] = [1.0, 2.0, 2.5, 5.0];
// Red is never a service colour: it is reserved for status "error".
const SERVICE_TONES: [&str; 5] = ["frontend", "backend", "database", "cloud", "messagebus"];

/// The reference's duration and axis number: three significant digits below
/// 0.01, else two fraction digits after `Math.round(value * 100) / 100`.
pub fn number(value: f64) -> String {
    if value != 0.0 && value.abs() < 0.01 {
        return if value.abs() < 1e-6 {
            js_to_string(
                js_to_precision(value, 3)
                    .parse()
                    .expect("toPrecision yields a number"),
            )
        } else {
            en_us_max_significant(value, 3)
        };
    }
    let rounded = if value.abs() > f64::MAX / 100.0 {
        value
    } else {
        js_round(value * 100.0) / 100.0
    };
    en_us_max_fraction(rounded, 2)
}

/// The schema spells microseconds `us` for ASCII input; the canvas uses µs.
pub(crate) fn unit_label(unit: &str) -> &str {
    if unit == "us" {
        "µs"
    } else {
        unit
    }
}

/// A span's visible duration; an open span is only a lower bound.
pub(crate) fn duration_label(locale: &Locale, unit: &str, duration: f64, open: bool) -> String {
    let duration = format!("{} {}", number(duration), unit_label(unit));
    if open {
        locale.text("waterfall.incomplete", &[("duration", &duration)])
    } else {
        duration
    }
}

fn status_tone(status: &str) -> Option<&'static str> {
    match status {
        "error" => Some("security"),
        "cancelled" => Some("external"),
        _ => None,
    }
}

struct Span<'a> {
    index: usize,
    raw: &'a Value,
    id: &'a str,
    name: &'a str,
    parent: Option<&'a str>,
    service: Option<&'a str>,
    status: &'a str,
    start: f64,
    end: f64,
    open: bool,
    duration: f64,
}

struct Row<'a> {
    span: &'a Span<'a>,
    depth: usize,
    service: Option<&'a str>,
    tone: &'static str,
    parent_bar: bool,
    y: f64,
    x0: f64,
    x1: f64,
    label: String,
    label_width: f64,
}

fn text(path: String, content: &str, x: f64, y: f64, font_size: f64, anchor: &str) -> Text {
    Text {
        source_path: path,
        content: content.into(),
        position: Point { x, y },
        font_size,
        anchor: anchor.into(),
    }
}

fn authored(id: &str, path: String, kind: &str, facts: Value, provenance: &str) -> Authored {
    Authored {
        id: id.into(),
        source_path: path,
        kind: kind.into(),
        facts,
        geometry_provenance: vec![provenance.into()],
    }
}

fn container(
    authored: Authored,
    bounds: Rect,
    rule: Option<[Point; 2]>,
    text: Vec<Text>,
) -> Container {
    Container {
        authored,
        bounds,
        label_plate: None,
        rule,
        members: vec![],
        text,
    }
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Option<[Point; 2]> {
    Some([Point { x: x1, y: y1 }, Point { x: x2, y: y2 }])
}

fn timing_failure(evidence: Value) -> Diagnostics {
    let mut diagnostic = Diagnostic::error(
        Some("waterfall"),
        "waterfall/invalid-timing",
        "/spans",
        "The recorded range cannot form a finite axis scale",
    );
    diagnostic.stage = "layout".into();
    diagnostic.evidence = evidence;
    diagnostic.supported_fixes =
        vec!["Rescale the recorded timing and unit to a representable range".into()];
    vec![diagnostic]
}

fn tick_step(wall: f64, axis_width: f64) -> f64 {
    if wall == 0.0 {
        return 1.0;
    }
    let target = (MIN_TICK_PX / axis_width) * wall;
    let mut magnitude: f64 = format!("1e{}", target.log10().floor())
        .parse()
        .expect("a power of ten");
    loop {
        if let Some(factor) = NICE.iter().find(|factor| *factor * magnitude >= target) {
            return factor * magnitude;
        }
        magnitude *= 10.0;
    }
}

struct LegendEntry {
    kind: String,
    label: String,
    tone: &'static str,
    width: f64,
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let locale = crate::i18n::resolve(meta);
    let unit = meta["unit"].as_str().unwrap_or("ms");
    let evidence = meta["evidence"].as_str().unwrap_or_default();
    let axis_width = document["layout"]["width"].as_f64().unwrap_or(760.0);
    let raw = document["spans"].as_array().map_or(&[][..], Vec::as_slice);
    let recorded_end = |span: &Value| {
        span["end"]
            .as_f64()
            .or_else(|| Some(span["start"].as_f64()? + span["duration"].as_f64()?))
    };
    let t0 = raw
        .iter()
        .filter_map(|span| span["start"].as_f64())
        .fold(f64::INFINITY, f64::min);
    let t1 = raw
        .iter()
        .map(|span| recorded_end(span).unwrap_or_else(|| span["start"].as_f64().unwrap_or(t0)))
        .fold(f64::NEG_INFINITY, f64::max);
    let wall = t1 - t0;
    // An incomplete span with no end is drawn to the last recorded instant
    // and labelled as a lower bound, never as a measured duration.
    let spans: Vec<Span<'_>> = raw
        .iter()
        .enumerate()
        .map(|(index, span)| {
            let start = span["start"].as_f64().unwrap_or_default();
            let end = recorded_end(span);
            Span {
                index,
                raw: span,
                id: span["id"].as_str().unwrap_or_default(),
                name: span["name"].as_str().unwrap_or_default(),
                parent: span["parent"].as_str(),
                service: span["service"].as_str(),
                status: span["status"].as_str().unwrap_or("ok"),
                start,
                end: end.unwrap_or(t1),
                open: end.is_none(),
                duration: end.unwrap_or(t1) - start,
            }
        })
        .collect();
    // Rows run depth first, each sibling list by start time, then input order.
    let mut order: Vec<(usize, usize)> = Vec::new();
    fn visit(
        spans: &[Span<'_>],
        parent: Option<&str>,
        depth: usize,
        order: &mut Vec<(usize, usize)>,
    ) {
        let mut children: Vec<usize> = (0..spans.len())
            .filter(|index| spans[*index].parent == parent)
            .collect();
        children.sort_by(|a, b| {
            spans[*a]
                .start
                .total_cmp(&spans[*b].start)
                .then(spans[*a].index.cmp(&spans[*b].index))
        });
        for child in children {
            order.push((child, depth));
            visit(spans, Some(spans[child].id), depth + 1, order);
        }
    }
    visit(&spans, None, 0, &mut order);

    // Colour states who did the work: each service in order of first
    // appearance, inherited by spans without one.
    let mut services: Vec<&str> = Vec::new();
    let mut service_of: Vec<(&str, Option<&str>)> = Vec::new();
    for (index, _) in &order {
        let span = &spans[*index];
        if let Some(service) = span.service.filter(|service| !service.is_empty()) {
            if !services.contains(&service) {
                services.push(service);
            }
        }
        let inherited = span.service.or_else(|| {
            span.parent.and_then(|parent| {
                service_of
                    .iter()
                    .find(|(id, _)| *id == parent)
                    .and_then(|(_, service)| *service)
            })
        });
        service_of.push((span.id, inherited));
    }
    let service_tone = |service: Option<&str>| match service {
        None => "external",
        Some(service) => {
            let index = services
                .iter()
                .position(|item| *item == service)
                .unwrap_or(0);
            SERVICE_TONES[index % SERVICE_TONES.len()]
        }
    };

    let name_width = |depth: usize, name: &str| {
        depth as f64 * INDENT + NAME_DOT + text_units(name) * NAME_FONT * ADVANCE * 1.04
    };
    let label_w = order
        .iter()
        .map(|(index, depth)| name_width(*depth, spans[*index].name))
        .fold(140.0, f64::max)
        .ceil();
    let axis_x0 = MARGIN + label_w + COLUMN_GAP;
    let axis_x1 = axis_x0 + axis_width;
    let scale = if wall > 0.0 { axis_width / wall } else { 0.0 };
    if !scale.is_finite() {
        return Err(timing_failure(json!({"start":t0,"end":t1,"wall":wall})));
    }
    let x_of = |t: f64| js_round((axis_x0 + (t - t0) * scale) * 100.0) / 100.0;
    let rows_top = TOP + AXIS_H;
    let step = tick_step(wall, axis_width);
    let first_tick = (t0 / step).ceil() * step;
    let tick_count = (((t1 - first_tick) / step + 1e-9).floor() + 1.0).max(0.0) as usize;
    // Large timestamps can round adjacent ticks to the same double; iterate
    // by index so the loop stays bounded.
    let mut ticks: Vec<(f64, f64)> = Vec::new();
    for index in 0..tick_count {
        let t = first_tick + index as f64 * step;
        if ticks.last().is_none_or(|(last, _)| *last != t) {
            ticks.push((t, x_of(t)));
        }
    }

    let parents: Vec<&str> = spans.iter().filter_map(|span| span.parent).collect();
    let rows: Vec<Row<'_>> = order
        .iter()
        .enumerate()
        .map(|(position, (index, depth))| {
            let span = &spans[*index];
            let service = service_of[position].1;
            let label = duration_label(&locale, unit, span.duration, span.open);
            Row {
                span,
                depth: *depth,
                service,
                tone: status_tone(span.status).unwrap_or_else(|| service_tone(service)),
                parent_bar: parents.contains(&span.id),
                y: rows_top + position as f64 * ROW_H,
                x0: x_of(span.start),
                x1: x_of(span.end),
                label_width: text_units(&label) * DURATION_FONT * ADVANCE + 4.0,
                label,
            }
        })
        .collect();

    // The legend names the services when more than one does work, and any
    // status that changes a bar's paint; colours shared past five services
    // share one entry.
    let mut entries: Vec<LegendEntry> = Vec::new();
    if services.len() > 1 {
        let mut by_tone: Vec<(&'static str, Vec<&str>)> = Vec::new();
        for service in &services {
            let tone = service_tone(Some(service));
            match by_tone.iter_mut().find(|(candidate, _)| *candidate == tone) {
                Some((_, owners)) => owners.push(service),
                None => by_tone.push((tone, vec![service])),
            }
        }
        for (tone, owners) in by_tone {
            entries.push(LegendEntry {
                kind: format!("service:{}", owners[0]),
                label: owners.join(" · "),
                tone,
                width: 0.0,
            });
        }
    }
    for kind in ["error", "cancelled", "incomplete"] {
        if rows.iter().any(|row| row.span.status == kind) {
            entries.push(LegendEntry {
                kind: kind.into(),
                label: locale.text(&format!("legend.waterfall.{kind}"), &[]),
                tone: status_tone(kind).unwrap_or("external"),
                width: 0.0,
            });
        }
    }
    for entry in &mut entries {
        entry.width = (14.0 + 8.0 + text_units(&entry.label) * 13.0 * 0.62).ceil();
    }
    let content_bottom = rows_top + rows.len() as f64 * ROW_H + 8.0;
    // Every duration reads just after its bar; the canvas reserves room past
    // the axis for the widest one.
    let label_room = (rows
        .iter()
        .map(|row| row.x1 + LABEL_GAP + row.label_width)
        .fold(f64::NEG_INFINITY, f64::max)
        - axis_x1)
        .ceil();
    let width = (axis_x1 + 24f64.max(label_room) + MARGIN).ceil();
    let available = width - MARGIN * 2.0;
    if let Some(entry) = entries.iter().find(|entry| entry.width > available) {
        let mut diagnostic = Diagnostic::error(
            Some("waterfall"),
            "legend/label-too-wide",
            "/spans",
            format!(
                "waterfall legend label for \"{}\" needs {}px but only {available}px is available",
                entry.kind, entry.width
            ),
        );
        diagnostic.stage = "layout".into();
        diagnostic.evidence =
            json!({"kind":entry.kind,"measuredWidthPx":entry.width,"availableWidthPx":available});
        diagnostic.supported_fixes =
            vec!["Shorten service names that share a colour, or widen layout.width".into()];
        return Err(vec![diagnostic]);
    }
    let mut legend_rows: Vec<Vec<usize>> = vec![];
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        match legend_rows.last_mut() {
            Some(row) if cursor + 22.0 + entry.width <= available => {
                row.push(index);
                cursor += 22.0 + entry.width;
            }
            _ => {
                legend_rows.push(vec![index]);
                cursor = entry.width;
            }
        }
    }
    let extra = legend_rows.len().saturating_sub(1) as f64 * 22.0;
    let height = (content_bottom
        + MARGIN
        + if entries.is_empty() {
            0.0
        } else {
            34.0 + extra
        })
    .ceil();

    let evidence_text = locale.text(&format!("waterfall.evidence.{evidence}"), &[]);
    let evidence_w = (text_units(&evidence_text) * 10.5 * ADVANCE + 24.0).ceil();
    let total = format!("{} {}", number(wall), unit_label(unit));
    let wall_text = locale.text("waterfall.wall", &[("duration", &total)]);
    let range = format!("{} → {} {}", number(t0), number(t1), unit_label(unit));
    let wall_x = MARGIN + evidence_w + 14.0;
    let axis_y = rows_top - 6.0;
    let mut containers = vec![
        container(
            authored(
                evidence,
                "/meta/evidence".into(),
                "waterfall-evidence",
                json!({"evidence":evidence}),
                "renderer-owned-header",
            ),
            Rect {
                x: MARGIN,
                y: 14.0,
                width: evidence_w,
                height: 22.0,
            },
            None,
            vec![text(
                "/meta/evidence".into(),
                &evidence_text,
                MARGIN + evidence_w / 2.0 + 5.0,
                29.0,
                10.5,
                "middle",
            )],
        ),
        container(
            authored(
                "total",
                "/spans".into(),
                "waterfall-total",
                json!({"wall":wall}),
                "recorded-range",
            ),
            Rect {
                x: wall_x,
                y: 16.0,
                width: 0.0,
                height: 16.0,
            },
            None,
            vec![
                text("/spans".into(), &wall_text, wall_x, 29.5, 13.0, "start"),
                text("/spans".into(), &range, wall_x, 29.5, 13.0, "start"),
            ],
        ),
        container(
            authored(
                "axis",
                "/meta/unit".into(),
                "waterfall-axis",
                Value::Null,
                "linear-time-axis",
            ),
            Rect {
                x: axis_x0,
                y: axis_y,
                width: axis_width,
                height: 0.0,
            },
            line(axis_x0, axis_y, axis_x1, axis_y),
            vec![text(
                "/meta/unit".into(),
                &locale.text("waterfall.operation", &[]),
                MARGIN,
                axis_y - 10.0,
                10.0,
                "start",
            )],
        ),
    ];
    for (index, (t, x)) in ticks.iter().enumerate() {
        containers.push(container(
            authored(
                &js_to_string(*t),
                "/meta/unit".into(),
                "waterfall-tick",
                json!({"t":t,"origin":index == 0}),
                "nice-step-tick",
            ),
            Rect {
                x: *x,
                y: axis_y - 4.0,
                width: 0.0,
                height: content_bottom - axis_y + 4.0,
            },
            line(*x, axis_y, *x, content_bottom),
            vec![text(
                "/meta/unit".into(),
                &format!("{} {}", number(*t), unit_label(unit)),
                *x,
                axis_y - 10.0,
                10.5,
                "middle",
            )],
        ));
    }
    // Tree guides in the name column state the parent/child structure; the
    // bars themselves only ever state time.
    let dot_x = |depth: usize| MARGIN + depth as f64 * INDENT + 4.0;
    for row in &rows {
        let Some(parent) = row
            .span
            .parent
            .and_then(|parent| rows.iter().find(|candidate| candidate.span.id == parent))
        else {
            continue;
        };
        let (x, parent_y, y) = (
            dot_x(parent.depth),
            parent.y + ROW_H / 2.0 + 6.0,
            row.y + ROW_H / 2.0,
        );
        containers.push(container(
            authored(
                row.span.id,
                format!("/spans/{}/parent", row.span.index),
                "waterfall-guide",
                json!({"parent":parent.span.id,"child":row.span.id}),
                "name-column-guide",
            ),
            Rect {
                x,
                y: parent_y,
                width: dot_x(row.depth) - 7.0 - x,
                height: y - parent_y,
            },
            line(x, parent_y, dot_x(row.depth) - 7.0, y),
            vec![],
        ));
    }
    let percent = |value: f64| {
        if wall == 0.0 {
            return "100%".to_owned();
        }
        let share = value / wall * 100.0;
        let rounded = if share >= 10.0 {
            js_round(share)
        } else {
            js_round(share * 10.0) / 10.0
        };
        format!("{}%", js_to_string(rounded))
    };
    let mut nodes = Vec::with_capacity(rows.len());
    for row in &rows {
        let span = row.span;
        let mid_y = row.y + ROW_H / 2.0;
        let parent = span
            .parent
            .and_then(|parent| spans.iter().find(|candidate| candidate.id == parent));
        let timing = format!(
            "{}–{} {} · {} · {}",
            number(span.start),
            if span.open {
                "…".into()
            } else {
                number(span.end)
            },
            unit_label(unit),
            row.label,
            locale.text(
                "waterfall.share",
                &[("percent", &percent(span.duration)), ("total", &total)]
            )
        );
        let sublabel: Vec<&str> = [timing.as_str()]
            .into_iter()
            .chain(
                span.raw["detail"]
                    .as_str()
                    .filter(|detail| !detail.trim().is_empty()),
            )
            .collect();
        let context: Vec<&str> = row
            .service
            .into_iter()
            .chain(parent.map(|parent| parent.name))
            .filter(|value| !value.trim().is_empty())
            .collect();
        let paint = if span.open {
            "wf-open"
        } else if row.parent_bar {
            "wf-span"
        } else {
            "wf-solid"
        };
        containers.push(Container {
            authored: authored(
                span.id,
                format!("/spans/{}", span.index),
                "waterfall-row",
                json!({"depth":row.depth,"tone":row.tone,"paint":paint,"service":row.service,"status":span.status,
                    "open":span.open,"parentBar":row.parent_bar,"start":span.start,"end":span.end,"duration":span.duration,
                    "sublabel":sublabel.join(" — "),"context":context.join(" \u{203a} ")}),
                "waterfall-recorded-timing",
            ),
            bounds: Rect {
                x: MARGIN - 8.0,
                y: row.y + 1.0,
                width: width - MARGIN * 2.0 + 16.0,
                height: ROW_H - 2.0,
            },
            label_plate: None,
            rule: line(dot_x(row.depth), mid_y, row.x1, mid_y),
            members: vec![span.id.into()],
            text: vec![],
        });
        nodes.push(Node {
            brand: None,
            authored: authored(
                span.id,
                format!("/spans/{}", span.index),
                "waterfall-span",
                span.raw.clone(),
                "waterfall-recorded-timing",
            ),
            bounds: Rect {
                x: row.x0,
                y: mid_y - BAR_H / 2.0,
                width: 2f64.max(row.x1 - row.x0),
                height: BAR_H,
            },
            sigil: None,
            label: span.name.into(),
            text: vec![
                text(
                    format!("/spans/{}/name", span.index),
                    span.name,
                    dot_x(row.depth) + 10.0,
                    mid_y + 4.3,
                    NAME_FONT,
                    "start",
                ),
                text(
                    format!("/spans/{}/duration", span.index),
                    &row.label,
                    row.x1 + LABEL_GAP,
                    mid_y + 3.8,
                    DURATION_FONT,
                    "start",
                ),
            ],
            detail_rows: vec![],
        });
    }

    let mut legends = Vec::new();
    if !entries.is_empty() {
        let baseline = height - 16.0;
        let title_y = baseline - extra - 20.0;
        legends.push(Card {
            authored: authored(
                "legend-title",
                "/meta".into(),
                "legend-title",
                Value::Null,
                "resolved-legend-footprint",
            ),
            bounds: Rect {
                x: MARGIN,
                y: title_y - 10.0,
                width: 48.0,
                height: 14.0,
            },
            text: vec![text(
                "/meta".into(),
                &locale.text("legend.title", &[]),
                MARGIN,
                title_y,
                12.0,
                "start",
            )],
        });
        for (row_index, row) in legend_rows.iter().enumerate() {
            let row_baseline = baseline - (legend_rows.len() - row_index - 1) as f64 * 22.0;
            let mut x = MARGIN;
            for &index in row {
                let entry = &entries[index];
                legends.push(Card {
                    authored: authored(
                        &entry.kind,
                        "/spans".into(),
                        &entry.kind,
                        json!({"tone":entry.tone,"paint":if entry.kind == "incomplete" { "wf-open" } else { "wf-solid" }}),
                        "resolved-legend-footprint",
                    ),
                    bounds: Rect {
                        x,
                        y: row_baseline - 10.0,
                        width: entry.width,
                        height: 14.0,
                    },
                    text: vec![text("/spans".into(), &entry.label, x + 22.0, row_baseline, 13.0, "start")],
                });
                x += entry.width + 22.0;
            }
        }
    }

    Ok(CompiledDiagram {
        diagram_type: DiagramType::Waterfall,
        schema_version: document["schema_version"].as_u64().unwrap_or(1) as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: meta.clone(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        },
        reader_fit: "width-first".into(),
        nodes,
        relations: vec![],
        containers,
        text: vec![],
        legends,
        cards: super::tree::cards(document),
        facts: DiagramFacts::Waterfall {
            unit: unit.into(),
            evidence: evidence.into(),
            spans: raw
                .iter()
                .map(|span| WaterfallSpan {
                    id: span["id"].as_str().unwrap_or_default().into(),
                    parent: span["parent"].as_str().map(str::to_owned),
                    start: span["start"].as_f64().unwrap_or_default(),
                    end: recorded_end(span),
                    status: span["status"].as_str().unwrap_or("ok").into(),
                    service: span["service"].as_str().map(str::to_owned),
                })
                .collect(),
        },
        diagnostics: vec![],
    })
}
