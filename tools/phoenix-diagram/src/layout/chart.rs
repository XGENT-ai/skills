// Numeric charts: one band per authored row on a shared linear value axis;
// every value is labelled as authored and mirrored in facts.table. This file
// lays out the two bar orientations; line, pie and limits join in M2.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::format::number::{js_round, js_to_string};
use crate::geometry::{format_number as number, text_units};
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Text,
};
use serde_json::{json, Value};

use super::waterfall::tick_step;

const MARGIN: f64 = 28.0;
// The illustrative badge lives in the same header band as the waterfall's.
const HEADER_H: f64 = 52.0;
// Also the minimum width of vertical plots: narrow screens scale the canvas to
// full width, and a portrait canvas would then overflow the page.
const PLOT_H: f64 = 240.0;
const BAR_W: f64 = 22.0;
const BAR_GAP: f64 = 4.0;
const SLOT_PAD: f64 = 24.0;
const ANNOT_ROOM: f64 = 16.0;
// Line annotations hang 8 above their point (bar ones 4), so the line needs
// more headroom above the plot before the band top.
const LINE_ANNOT_ROOM: f64 = 24.0;
const LABEL_BASE_GAP: f64 = 24.0;
const LABEL_FONT: f64 = 12.5;
const VALUE_FONT: f64 = 11.0;
const TICK_FONT: f64 = 10.5;
const LEGEND_FONT: f64 = 13.0;
const ADVANCE: f64 = 0.6;
const VERTICAL_MIN_TICK: f64 = 40.0;
const HORIZONTAL_AXIS_W: f64 = 560.0;
const HORIZONTAL_MIN_TICK: f64 = 80.0;
const SERIES_BAR_H: f64 = 14.0;
const SERIES_BAR_GAP: f64 = 4.0;
const ROW_BAND_GAP: f64 = 10.0;
const TONES: [&str; 5] = ["frontend", "backend", "database", "cloud", "messagebus"];

struct RowInput<'a> {
    index: usize,
    id: &'a str,
    label: &'a str,
    values: &'a [Value],
    highlight: bool,
    // limits rows carry value/limit/note/row-level unit instead of values.
    value: Option<f64>,
    limit: Option<f64>,
    note: &'a str,
    row_unit: &'a str,
}

struct LegendEntry {
    kind: String,
    key: String,
    label: String,
    tone: &'static str,
    width: f64,
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

// Series keys run A..Z, then AA, AB, …; they stay independent of the palette.
fn series_key(index: usize) -> String {
    let mut key = String::new();
    let mut rest = index;
    loop {
        key.insert(0, (b'A' + (rest % 26) as u8) as char);
        if rest < 26 {
            return key;
        }
        rest = rest / 26 - 1;
    }
}

// Unit text right after a number: `%` joins directly, other units take a space.
fn unit_suffix(unit: &str) -> String {
    if unit.is_empty() {
        String::new()
    } else if unit == "%" {
        "%".into()
    } else {
        format!(" {unit}")
    }
}

fn scale_failure() -> Diagnostics {
    let mut diagnostic = Diagnostic::error(
        Some("chart"),
        "chart/invalid-scale",
        "/rows",
        "The authored values cannot form a finite axis scale",
    );
    diagnostic.stage = "layout".into();
    diagnostic.supported_fixes =
        vec!["Rescale the values and unit to a representable range".into()];
    vec![diagnostic]
}

// Rounds the axis bounds outward to the resolved step; the finite check comes
// before `tick_step` because its power-of-ten parse panics on infinities.
fn axis_scale(
    lo: f64,
    hi: f64,
    length: f64,
    min_tick_px: f64,
) -> Result<(f64, f64, f64), Diagnostics> {
    let span = hi - lo;
    if !span.is_finite() || span < 0.0 || !length.is_finite() || !length.is_normal() {
        return Err(scale_failure());
    }
    if span == 0.0 {
        return Ok((lo, lo + 1.0, 1.0));
    }
    let step = tick_step(span, length, min_tick_px);
    if !step.is_finite() || step <= 0.0 {
        return Err(scale_failure());
    }
    let lo = (lo / step).floor() * step;
    let hi = (hi / step).ceil() * step;
    if !hi.is_finite() || !lo.is_finite() || hi <= lo {
        return Err(scale_failure());
    }
    Ok((lo, hi, step))
}

// Ticks walk from the rounded lower bound by whole steps; iterating by index
// keeps the loop bounded even for huge magnitudes.
fn ticks(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let count = (((hi - lo) / step) + 1e-9).floor() as usize + 1;
    let mut result = Vec::new();
    for index in 0..count {
        let t = lo + index as f64 * step;
        if result.last().is_none_or(|last| *last != t) {
            result.push(t);
        }
    }
    result
}

// A whole donut reads as two opposite arcs; a sector as an annular wedge.
fn full_donut_path(cx: f64, cy: f64, r_out: f64, r_in: f64) -> String {
    format!(
        "M {} {} A {} {} 0 1 1 {} {} A {} {} 0 1 1 {} {} Z M {} {} A {} {} 0 1 0 {} {} A {} {} 0 1 0 {} {} Z",
        number(cx + r_out), number(cy), number(r_out), number(r_out), number(cx - r_out), number(cy),
        number(r_out), number(r_out), number(cx + r_out), number(cy),
        number(cx + r_in), number(cy), number(r_in), number(r_in), number(cx - r_in), number(cy),
        number(r_in), number(r_in), number(cx + r_in), number(cy)
    )
}

fn sector_path(cx: f64, cy: f64, r_out: f64, r_in: f64, a0: f64, a1: f64) -> String {
    let at = |r: f64, a: f64| (cx + r * a.cos(), cy + r * a.sin());
    let (x0, y0) = at(r_out, a0);
    let (x1, y1) = at(r_out, a1);
    let (x2, y2) = at(r_in, a1);
    let (x3, y3) = at(r_in, a0);
    let large = if a1 - a0 > std::f64::consts::PI { 1 } else { 0 };
    format!(
        "M {} {} A {} {} 0 {} 1 {} {} L {} {} A {} {} 0 {} 0 {} {} Z",
        number(x0),
        number(y0),
        number(r_out),
        number(r_out),
        large,
        number(x1),
        number(y1),
        number(x2),
        number(y2),
        number(r_in),
        number(r_in),
        large,
        number(x3),
        number(y3)
    )
}

fn value_range(rows: &[RowInput<'_>], n: usize) -> (f64, f64) {
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for row in rows {
        for k in 0..n {
            let v = row.values[k].as_f64().unwrap_or_default();
            min_v = min_v.min(v);
            max_v = max_v.max(v);
        }
    }
    (min_v, max_v)
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let locale = crate::i18n::resolve(meta);
    let form = document["form"].as_str().unwrap_or_default();
    let orientation = if form == "bar" {
        document["orientation"].as_str().unwrap_or("vertical")
    } else {
        ""
    };
    let unit = meta["unit"].as_str().unwrap_or_default();
    let evidence = meta["evidence"].as_str().unwrap_or_default();
    let raw_series = document["series"].as_array().map_or(&[][..], Vec::as_slice);
    let series: Vec<(&str, &str, String)> = raw_series
        .iter()
        .enumerate()
        .map(|(index, item)| {
            (
                item["id"].as_str().unwrap_or_default(),
                item["name"].as_str().unwrap_or_default(),
                series_key(index),
            )
        })
        .collect();
    let rows: Vec<RowInput<'_>> = document["rows"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
        .map(|(index, row)| RowInput {
            index,
            id: row["id"].as_str().unwrap_or_default(),
            label: row["label"].as_str().unwrap_or_default(),
            values: row["values"].as_array().map_or(&[][..], Vec::as_slice),
            highlight: row["highlight"] == true,
            value: row["value"].as_f64(),
            limit: row["limit"].as_f64(),
            note: row["note"].as_str().unwrap_or_default(),
            row_unit: row["unit"].as_str().unwrap_or_default(),
        })
        .collect();
    let n = series.len().max(1);
    let has_highlight = rows.iter().any(|row| row.highlight);
    let suffix = unit_suffix(unit);
    let tone_of = |k: usize| TONES[k % TONES.len()];
    // The annotation text: the series key leads once two or more compete.
    let number_text = |k: usize, value: f64| -> String {
        if series.len() >= 2 {
            format!("{} {}", series[k].2, js_to_string(value))
        } else {
            js_to_string(value)
        }
    };
    let value_text_w = |k: usize, v: f64| text_units(&number_text(k, v)) * VALUE_FONT * ADVANCE;
    let sublabel_of = |row: &RowInput<'_>| -> String {
        let parts: Vec<String> = (0..n)
            .map(|k| {
                let v = row.values[k].as_f64().unwrap_or_default();
                if series.len() >= 2 {
                    format!("{} {}{}", series[k].1, js_to_string(v), suffix)
                } else {
                    format!("{}{}", js_to_string(v), suffix)
                }
            })
            .collect();
        parts.join(" \u{b7} ")
    };
    let header_top = if evidence == "illustrative" {
        HEADER_H
    } else {
        MARGIN
    };

    let mut containers: Vec<Container> = Vec::new();
    let mut nodes = Vec::new();
    let mut rows_facts: Vec<Value> = Vec::new();
    let mut any_over = false;
    let width;
    let content_bottom;

    if form == "line" {
        // --- line: equidistant slots on a shared linear value axis ----------
        let slot_w = rows
            .iter()
            .map(|row| {
                let label_w = text_units(row.label) * LABEL_FONT * ADVANCE * 1.04;
                let annot_w = (0..n)
                    .map(|k| value_text_w(k, row.values[k].as_f64().unwrap_or_default()))
                    .fold(0.0, f64::max);
                label_w.max(annot_w) + 16.0
            })
            .fold(PLOT_H / rows.len() as f64, f64::max);
        let plot_w = slot_w * rows.len() as f64;
        let (lo, hi, step) = {
            let (min_v, max_v) = value_range(&rows, n);
            axis_scale(min_v, max_v, PLOT_H, VERTICAL_MIN_TICK)?
        };
        let tick_values = ticks(lo, hi, step);
        let tick_room = tick_values
            .iter()
            .map(|t| text_units(&format!("{}{}", number(*t), suffix)) * TICK_FONT * ADVANCE)
            .fold(0.0, f64::max);
        let plot_top = header_top + ANNOT_ROOM;
        let plot_bottom = plot_top + PLOT_H;
        let plot_left = MARGIN + tick_room + 8.0;
        let plot_right = plot_left + plot_w;
        let label_y = plot_bottom + LABEL_BASE_GAP;
        content_bottom = label_y + 4.0;
        width = plot_right + MARGIN;
        let y_at = |v: f64| js_round((plot_bottom - (v - lo) / (hi - lo) * PLOT_H) * 100.0) / 100.0;
        containers.push(container(
            authored(
                "axis",
                "/meta/unit".into(),
                "chart-axis",
                Value::Null,
                "linear-value-axis",
            ),
            Rect {
                x: plot_left,
                y: plot_top,
                width: 0.0,
                height: PLOT_H,
            },
            line(plot_left, plot_top, plot_left, plot_bottom),
            vec![],
        ));
        for t in &tick_values {
            let y = y_at(*t);
            containers.push(container(
                authored(
                    &js_to_string(*t),
                    "/meta/unit".into(),
                    "chart-tick",
                    json!({"value": t, "zero": *t == 0.0}),
                    "nice-step-tick",
                ),
                Rect {
                    x: plot_left,
                    y,
                    width: plot_w,
                    height: 0.0,
                },
                line(plot_left, y, plot_right, y),
                vec![text(
                    "/meta/unit".into(),
                    &format!("{}{}", number(*t), suffix),
                    plot_left - 8.0,
                    y + 3.5,
                    TICK_FONT,
                    "end",
                )],
            ));
        }
        // One polyline per series, drawn before the rows; a single row has no
        // segments at all.
        let centers: Vec<f64> = (0..rows.len())
            .map(|index| plot_left + index as f64 * slot_w + slot_w / 2.0)
            .collect();
        if rows.len() >= 2 {
            for k in 0..n {
                let points: Vec<Vec<f64>> = rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        vec![
                            js_round(centers[index] * 100.0) / 100.0,
                            y_at(row.values[k].as_f64().unwrap_or_default()),
                        ]
                    })
                    .collect();
                let series_id = series
                    .get(k)
                    .map(|(id, _, _)| id.to_string())
                    .unwrap_or_else(|| format!("series-{k}"));
                containers.push(container(
                    authored(
                        &series_id,
                        "/series".into(),
                        "chart-series",
                        json!({"series": k, "tone": tone_of(k), "points": points}),
                        "authored-values",
                    ),
                    Rect {
                        x: plot_left,
                        y: plot_top,
                        width: plot_w,
                        height: PLOT_H,
                    },
                    None,
                    vec![],
                ));
            }
        }
        let mut x_cursor = plot_left;
        for row in &rows {
            let slot_x = x_cursor;
            x_cursor += slot_w;
            let center = slot_x + slot_w / 2.0;
            let band_x = slot_x + 5.0;
            let band_w = slot_w - 10.0;
            let band_y = plot_top - LINE_ANNOT_ROOM;
            let band_h = content_bottom - band_y;
            // Annotations sit 8 above their point; collisions resolve downward
            // with a 13 px minimum and the stack shifts up when it would run
            // past the band bottom.
            let mut placed = vec![0.0f64; n];
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|&a, &b| {
                let ya = y_at(row.values[a].as_f64().unwrap_or_default()) - 8.0;
                let yb = y_at(row.values[b].as_f64().unwrap_or_default()) - 8.0;
                ya.total_cmp(&yb)
            });
            let mut stack = f64::NEG_INFINITY;
            for &i in &order {
                let desired = y_at(row.values[i].as_f64().unwrap_or_default()) - 8.0;
                let p = desired.max(stack + 13.0);
                placed[i] = p;
                stack = p;
            }
            let bottom_limit = plot_bottom + 14.0;
            let last = order[n - 1];
            if placed[last] > bottom_limit {
                let shift = placed[last] - bottom_limit;
                for value in placed.iter_mut() {
                    *value -= shift;
                }
            }
            if placed[order[0]] < band_y + 9.0 {
                let mut diagnostic = Diagnostic::error(
                    Some("chart"),
                    "chart/label-room",
                    &format!("/rows/{}", row.index),
                    "This row's value annotations do not fit its band",
                );
                diagnostic.stage = "layout".into();
                diagnostic.supported_fixes = vec!["减少系列，或拆成多张图".into()];
                return Err(vec![diagnostic]);
            }
            let mut marks = Vec::new();
            let mut annots = Vec::new();
            for (k, placed_y) in placed.iter().enumerate().take(n) {
                let v = row.values[k].as_f64().unwrap_or_default();
                marks.push(json!({
                    "kind": "point",
                    "series": k,
                    "tone": tone_of(k),
                    "cx": center,
                    "cy": y_at(v),
                }));
                annots.push(text(
                    format!("/rows/{}/values/{}", row.index, k),
                    &number_text(k, v),
                    center,
                    *placed_y,
                    VALUE_FONT,
                    "middle",
                ));
            }
            let mut row_texts = vec![text(
                format!("/rows/{}/label", row.index),
                row.label,
                center,
                label_y,
                LABEL_FONT,
                "middle",
            )];
            row_texts.extend(annots);
            containers.push(Container {
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    json!({
                        "form": "line",
                        "tone": tone_of(0),
                        "highlight": row.highlight,
                        "dim": has_highlight && !row.highlight,
                        "sublabel": sublabel_of(row),
                        "marks": marks,
                    }),
                    "authored-values",
                ),
                bounds: Rect {
                    x: band_x,
                    y: band_y,
                    width: band_w,
                    height: band_h,
                },
                label_plate: None,
                rule: None,
                members: vec![row.id.into()],
                text: vec![],
            });
            nodes.push(Node {
                brand: None,
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    document["rows"][row.index].clone(),
                    "authored-values",
                ),
                bounds: Rect {
                    x: band_x,
                    y: band_y,
                    width: band_w,
                    height: band_h,
                },
                sigil: None,
                label: row.label.into(),
                text: row_texts,
                detail_rows: vec![],
            });
            rows_facts.push(json!({"id": row.id, "values": row.values}));
        }
    } else if form == "pie" {
        // --- pie: one ring plus the list that doubles as the legend ---------
        let ring_r_out = 110.0;
        let ring_r_in = 62.0;
        let ring_cx = MARGIN + ring_r_out;
        let ring_cy = header_top + ring_r_out + 8.0;
        let total: f64 = rows
            .iter()
            .map(|row| row.values[0].as_f64().unwrap_or_default())
            .sum();
        let row_tone = |index: usize| -> &'static str {
            if rows.len() > 1 && rows.len() % 5 == 1 && index == rows.len() - 1 {
                "external"
            } else {
                TONES[index % TONES.len()]
            }
        };
        let value_pie_text = |v: f64| format!("{}{}", js_to_string(v), suffix);
        let share_text = |v: f64| {
            format!(
                "{}%",
                js_to_string(js_round(v / total * 100.0 * 10.0) / 10.0)
            )
        };
        let with_share = unit != "%";
        let num_w = 16.0;
        let swatch_gap = 20.0;
        let label_w = rows
            .iter()
            .map(|row| text_units(row.label) * LABEL_FONT * ADVANCE * 1.04)
            .fold(0.0, f64::max);
        let value_w = rows
            .iter()
            .map(|row| {
                text_units(&value_pie_text(row.values[0].as_f64().unwrap_or_default()))
                    * VALUE_FONT
                    * ADVANCE
            })
            .fold(0.0, f64::max);
        let share_w = if with_share {
            rows.iter()
                .map(|row| {
                    text_units(&share_text(row.values[0].as_f64().unwrap_or_default()))
                        * VALUE_FONT
                        * ADVANCE
                })
                .fold(0.0, f64::max)
        } else {
            0.0
        };
        let list_x = MARGIN + ring_r_out * 2.0 + 32.0;
        let swatch_x = list_x + num_w;
        let col_label = swatch_x + swatch_gap;
        let col_value = col_label + label_w + 14.0;
        let col_share = col_value + value_w + 12.0;
        let list_w = if with_share {
            col_share + share_w + 10.0 - list_x
        } else {
            col_value + value_w + 10.0 - list_x
        };
        let list_top = header_top + 4.0;
        let pitch = 28.0;
        let band_h = 18.0;
        let caption_y = ring_cy + ring_r_out + 24.0;
        let caption = locale.text("chart.pie.order", &[]);
        let caption_w = text_units(&caption) * TICK_FONT * ADVANCE * 1.04;
        content_bottom = caption_y.max(list_top + rows.len() as f64 * pitch) + 4.0;
        width = (list_x + list_w + MARGIN)
            .ceil()
            .max(ring_cx + ring_r_out + MARGIN)
            .max((MARGIN * 2.0 + caption_w).ceil());
        let mut angle = -std::f64::consts::FRAC_PI_2;
        for row in &rows {
            let v = row.values[0].as_f64().unwrap_or_default();
            let band_y = list_top + row.index as f64 * pitch;
            let tone = row_tone(row.index);
            let mut marks = Vec::new();
            if v > 0.0 {
                let sweep = v / total * 360.0;
                let a0 = angle;
                let a1 = angle + sweep.to_radians();
                angle = a1;
                let path = if sweep >= 359.999 {
                    full_donut_path(ring_cx, ring_cy, ring_r_out, ring_r_in)
                } else {
                    sector_path(ring_cx, ring_cy, ring_r_out, ring_r_in, a0, a1)
                };
                marks.push(json!({
                    "kind": "slice",
                    "tone": tone,
                    "path": path,
                    "sweep": js_round(sweep * 100.0) / 100.0,
                }));
            }
            marks.push(json!({
                "kind": "fill",
                "tone": tone,
                "x": swatch_x,
                "y": band_y + 4.0,
                "width": 14.0,
                "height": 10.0,
            }));
            let mut row_texts = vec![
                text(
                    format!("/rows/{}/label", row.index),
                    row.label,
                    col_label,
                    band_y + 13.0,
                    LABEL_FONT,
                    "start",
                ),
                text(
                    format!("/rows/{}/index", row.index),
                    &(row.index + 1).to_string(),
                    list_x + 2.0,
                    band_y + 13.0,
                    TICK_FONT,
                    "start",
                ),
                text(
                    format!("/rows/{}/values/0", row.index),
                    &value_pie_text(v),
                    col_value,
                    band_y + 13.0,
                    VALUE_FONT,
                    "start",
                ),
            ];
            if with_share {
                row_texts.push(text(
                    format!("/rows/{}/share", row.index),
                    &share_text(v),
                    col_share,
                    band_y + 13.0,
                    VALUE_FONT,
                    "start",
                ));
            }
            containers.push(Container {
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    json!({
                        "form": "pie",
                        "tone": tone,
                        "highlight": row.highlight,
                        "dim": has_highlight && !row.highlight,
                        "sublabel": value_pie_text(v),
                        "marks": marks,
                    }),
                    "authored-values",
                ),
                bounds: Rect {
                    x: list_x,
                    y: band_y,
                    width: list_w,
                    height: band_h,
                },
                label_plate: None,
                rule: None,
                members: vec![row.id.into()],
                text: vec![],
            });
            nodes.push(Node {
                brand: None,
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    document["rows"][row.index].clone(),
                    "authored-values",
                ),
                bounds: Rect {
                    x: list_x,
                    y: band_y,
                    width: list_w,
                    height: band_h,
                },
                sigil: None,
                label: row.label.into(),
                text: row_texts,
                detail_rows: vec![],
            });
            rows_facts.push(json!({"id": row.id, "values": row.values}));
        }
        containers.push(container(
            authored(
                "caption",
                "/meta".into(),
                "chart-caption",
                Value::Null,
                "pie-order-note",
            ),
            Rect {
                x: MARGIN,
                y: caption_y - 14.0,
                width: width - MARGIN * 2.0,
                height: 16.0,
            },
            None,
            vec![text(
                "/meta".into(),
                &caption,
                MARGIN,
                caption_y,
                TICK_FONT,
                "start",
            )],
        ));
    } else if form == "limits" {
        // --- limits: one track per row on its own scale ----------------------
        let label_w = rows
            .iter()
            .map(|row| text_units(row.label) * LABEL_FONT * ADVANCE * 1.04)
            .fold(140.0, f64::max)
            .ceil();
        let track_w = 420.0;
        let track_x = MARGIN + label_w + 24.0;
        let rows_top = header_top + 4.0;
        let pitch = 44.0;
        let band_h = 34.0;
        // Room for the widest right-hand text plus the over note.
        let mut text_room = 0.0f64;
        for row in &rows {
            let limit = row.limit.unwrap_or_default();
            let row_unit = if row.row_unit.is_empty() {
                unit
            } else {
                row.row_unit
            };
            let main = if let Some(v) = row.value {
                format!(
                    "{}{}",
                    locale.text(
                        "chart.limit.pair",
                        &[("value", &js_to_string(v)), ("limit", &js_to_string(limit))],
                    ),
                    unit_suffix(row_unit)
                )
            } else {
                format!(
                    "{}{}",
                    locale.text("chart.limit.only", &[("limit", &js_to_string(limit))]),
                    unit_suffix(row_unit)
                )
            };
            let mut w = text_units(&main) * VALUE_FONT * ADVANCE;
            if row.value.is_some_and(|v| v > limit) {
                w += 10.0
                    + text_units(&format!("\u{b7} {}", locale.text("chart.limit.over", &[])))
                        * VALUE_FONT
                        * ADVANCE;
            }
            text_room = text_room.max(w);
        }
        width = track_x + track_w + 12.0 + text_room + MARGIN;
        content_bottom = rows_top + rows.len() as f64 * pitch - (pitch - band_h) + 4.0;
        for (position, row) in rows.iter().enumerate() {
            let band_y = rows_top + position as f64 * pitch;
            let limit = row.limit.unwrap_or_default();
            let over = row.value.is_some_and(|v| v > limit);
            any_over = any_over || over;
            let row_unit = if row.row_unit.is_empty() {
                unit
            } else {
                row.row_unit
            };
            let full = row.value.unwrap_or(limit).max(limit) * 1.25;
            let fill_tone = if over { "security" } else { tone_of(0) };
            let fill_len =
                js_round((row.value.unwrap_or(limit) / full * track_w).max(1.0) * 100.0) / 100.0;
            let marks = vec![
                json!({
                    "kind": "track",
                    "tone": fill_tone,
                    "x": track_x,
                    "y": band_y + 8.0,
                    "width": track_w,
                    "height": 18.0,
                }),
                json!({
                    "kind": "fill",
                    "tone": fill_tone,
                    "x": track_x,
                    "y": band_y + 8.0,
                    "width": fill_len,
                    "height": 18.0,
                }),
                json!({
                    "kind": "limit-line",
                    "tone": fill_tone,
                    "x": js_round((track_x + limit / full * track_w) * 100.0) / 100.0,
                    "y1": band_y + 4.0,
                    "y2": band_y + 30.0,
                }),
            ];
            let with_note = !row.note.is_empty();
            let main = if let Some(v) = row.value {
                format!(
                    "{}{}",
                    locale.text(
                        "chart.limit.pair",
                        &[("value", &js_to_string(v)), ("limit", &js_to_string(limit))],
                    ),
                    unit_suffix(row_unit)
                )
            } else {
                format!(
                    "{}{}",
                    locale.text("chart.limit.only", &[("limit", &js_to_string(limit))]),
                    unit_suffix(row_unit)
                )
            };
            let main_x = track_x + track_w + 12.0;
            let mut row_texts = vec![text(
                format!("/rows/{}/label", row.index),
                row.label,
                MARGIN,
                band_y + if with_note { 12.0 } else { 20.0 },
                LABEL_FONT,
                "start",
            )];
            if with_note {
                row_texts.push(text(
                    format!("/rows/{}/note", row.index),
                    row.note,
                    MARGIN,
                    band_y + 26.0,
                    TICK_FONT,
                    "start",
                ));
            }
            row_texts.push(text(
                format!("/rows/{}/limit", row.index),
                &main,
                main_x,
                band_y + 21.0,
                VALUE_FONT,
                "start",
            ));
            if over {
                let main_w = text_units(&main) * VALUE_FONT * ADVANCE;
                row_texts.push(text(
                    format!("/rows/{}/over", row.index),
                    &format!("\u{b7} {}", locale.text("chart.limit.over", &[])),
                    main_x + main_w + 6.0,
                    band_y + 21.0,
                    VALUE_FONT,
                    "start",
                ));
            }
            containers.push(Container {
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    json!({
                        "form": "limits",
                        "tone": fill_tone,
                        "highlight": row.highlight,
                        "dim": has_highlight && !row.highlight,
                        "note": row.note,
                        "value": row.value,
                        "limit": limit,
                        "over": over,
                        "sublabel": main,
                        "marks": marks,
                    }),
                    "authored-values",
                ),
                bounds: Rect {
                    x: MARGIN,
                    y: band_y,
                    width: width - 2.0 * MARGIN,
                    height: band_h,
                },
                label_plate: None,
                rule: None,
                members: vec![row.id.into()],
                text: vec![],
            });
            nodes.push(Node {
                brand: None,
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    document["rows"][row.index].clone(),
                    "authored-values",
                ),
                bounds: Rect {
                    x: MARGIN,
                    y: band_y,
                    width: width - 2.0 * MARGIN,
                    height: band_h,
                },
                sigil: None,
                label: row.label.into(),
                text: row_texts,
                detail_rows: vec![],
            });
            rows_facts.push(json!({
                "id": row.id,
                "value": row.value,
                "limit": limit,
                "unit": row_unit,
                "over": over,
            }));
        }
    } else if orientation != "horizontal" {
        // --- vertical: one equal-width slot per row, bars hang from zero ---
        let bar_w = rows
            .iter()
            .flat_map(|row| (0..n).map(|k| (k, row.values[k].as_f64().unwrap_or_default())))
            .map(|(k, v)| value_text_w(k, v) + 6.0)
            .fold(BAR_W, f64::max);
        let slot_w = rows
            .iter()
            .map(|row| {
                let bars = n as f64 * bar_w + (n - 1) as f64 * BAR_GAP;
                let label_w = text_units(row.label) * LABEL_FONT * ADVANCE * 1.04;
                bars.max(label_w) + SLOT_PAD
            })
            .fold(PLOT_H / rows.len() as f64, f64::max);
        let plot_w = slot_w * rows.len() as f64;
        let (lo, hi, step) = {
            let (min_v, max_v) = value_range(&rows, n);
            axis_scale(min_v.min(0.0), max_v.max(0.0), PLOT_H, VERTICAL_MIN_TICK)?
        };
        let tick_values = ticks(lo, hi, step);
        let tick_room = tick_values
            .iter()
            .map(|t| text_units(&format!("{}{}", number(*t), suffix)) * TICK_FONT * ADVANCE)
            .fold(0.0, f64::max);
        let plot_top = header_top + ANNOT_ROOM;
        let plot_bottom = plot_top + PLOT_H;
        let plot_left = MARGIN + tick_room + 8.0;
        let plot_right = plot_left + plot_w;
        let label_y = plot_bottom + LABEL_BASE_GAP;
        content_bottom = label_y + 4.0;
        width = plot_right + MARGIN;
        let y_at = |v: f64| js_round((plot_bottom - (v - lo) / (hi - lo) * PLOT_H) * 100.0) / 100.0;
        let zero_y = y_at(0.0);
        // The value axis is the vertical spine left of the plot.
        containers.push(container(
            authored(
                "axis",
                "/meta/unit".into(),
                "chart-axis",
                Value::Null,
                "linear-value-axis",
            ),
            Rect {
                x: plot_left,
                y: plot_top,
                width: 0.0,
                height: PLOT_H,
            },
            line(plot_left, plot_top, plot_left, plot_bottom),
            vec![],
        ));
        for t in &tick_values {
            let y = y_at(*t);
            containers.push(container(
                authored(
                    &js_to_string(*t),
                    "/meta/unit".into(),
                    "chart-tick",
                    json!({"value": t, "zero": *t == 0.0}),
                    "nice-step-tick",
                ),
                Rect {
                    x: plot_left,
                    y,
                    width: plot_w,
                    height: 0.0,
                },
                line(plot_left, y, plot_right, y),
                vec![text(
                    "/meta/unit".into(),
                    &format!("{}{}", number(*t), suffix),
                    plot_left - 8.0,
                    y + 3.5,
                    TICK_FONT,
                    "end",
                )],
            ));
        }
        let mut x_cursor = plot_left;
        for row in &rows {
            let slot_x = x_cursor;
            x_cursor += slot_w;
            let band_x = slot_x + 5.0;
            let band_w = slot_w - 10.0;
            let band_y = plot_top - ANNOT_ROOM;
            let band_h = content_bottom - band_y;
            let total_bars = n as f64 * bar_w + (n - 1) as f64 * BAR_GAP;
            let bars_x0 = slot_x + (slot_w - total_bars) / 2.0;
            let mut marks = Vec::new();
            let mut annots = Vec::new();
            for k in 0..n {
                let v = row.values[k].as_f64().unwrap_or_default();
                let bar_x = bars_x0 + k as f64 * (bar_w + BAR_GAP);
                let bar_center_x = bar_x + bar_w / 2.0;
                let annot_path = format!("/rows/{}/values/{}", row.index, k);
                if v == 0.0 {
                    // A zero still reads its number, right above the zero line.
                    annots.push(text(
                        annot_path,
                        &number_text(k, v),
                        bar_center_x,
                        zero_y - 4.0,
                        VALUE_FONT,
                        "middle",
                    ));
                    continue;
                }
                let len = (zero_y - y_at(v)).abs().max(1.0);
                let (y, h) = if v > 0.0 {
                    (zero_y - len, len)
                } else {
                    (zero_y, len)
                };
                marks.push(json!({
                    "series": k,
                    "tone": tone_of(k),
                    "x": js_round(bar_x * 100.0) / 100.0,
                    "y": y,
                    "width": js_round(bar_w * 100.0) / 100.0,
                    "height": h,
                }));
                let baseline = if v > 0.0 { y - 4.0 } else { y + h + 10.0 };
                annots.push(text(
                    annot_path,
                    &number_text(k, v),
                    bar_center_x,
                    baseline,
                    VALUE_FONT,
                    "middle",
                ));
            }
            let mut row_texts = vec![text(
                format!("/rows/{}/label", row.index),
                row.label,
                slot_x + slot_w / 2.0,
                label_y,
                LABEL_FONT,
                "middle",
            )];
            row_texts.extend(annots);
            containers.push(Container {
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    json!({
                        "tone": tone_of(0),
                        "highlight": row.highlight,
                        "dim": has_highlight && !row.highlight,
                        "sublabel": sublabel_of(row),
                        "marks": marks,
                    }),
                    "authored-values",
                ),
                bounds: Rect {
                    x: band_x,
                    y: band_y,
                    width: band_w,
                    height: band_h,
                },
                label_plate: None,
                rule: None,
                members: vec![row.id.into()],
                text: vec![],
            });
            nodes.push(Node {
                brand: None,
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    document["rows"][row.index].clone(),
                    "authored-values",
                ),
                bounds: Rect {
                    x: band_x,
                    y: band_y,
                    width: band_w,
                    height: band_h,
                },
                sigil: None,
                label: row.label.into(),
                text: row_texts,
                detail_rows: vec![],
            });
            rows_facts.push(json!({"id": row.id, "values": row.values}));
        }
    } else {
        // --- horizontal: rows stack downward, bars run from zero ----------
        let label_w = rows
            .iter()
            .map(|row| text_units(row.label) * LABEL_FONT * ADVANCE * 1.04)
            .fold(140.0, f64::max)
            .ceil();
        let row_h = n as f64 * SERIES_BAR_H + (n - 1) as f64 * SERIES_BAR_GAP + 12.0;
        let (lo, hi, step) = {
            let (min_v, max_v) = value_range(&rows, n);
            axis_scale(
                min_v.min(0.0),
                max_v.max(0.0),
                HORIZONTAL_AXIS_W,
                HORIZONTAL_MIN_TICK,
            )?
        };
        let tick_values = ticks(lo, hi, step);
        let mut neg_room = 0.0f64;
        let mut pos_room = 0.0f64;
        for row in &rows {
            for k in 0..n {
                let v = row.values[k].as_f64().unwrap_or_default();
                let w = value_text_w(k, v) + 6.0;
                if v < 0.0 {
                    neg_room = neg_room.max(w);
                } else {
                    pos_room = pos_room.max(w);
                }
            }
        }
        let rows_top = header_top + 26.0;
        let axis_x0 = MARGIN + label_w + 12.0 + neg_room;
        let axis_x1 = axis_x0 + HORIZONTAL_AXIS_W;
        let x_at =
            |v: f64| js_round((axis_x0 + (v - lo) / (hi - lo) * HORIZONTAL_AXIS_W) * 100.0) / 100.0;
        let zero_x = x_at(0.0);
        width = axis_x1 + pos_room + MARGIN;
        content_bottom = rows_top + rows.len() as f64 * (row_h + ROW_BAND_GAP) - ROW_BAND_GAP + 4.0;
        // The value axis is the horizontal spine above the rows.
        containers.push(container(
            authored(
                "axis",
                "/meta/unit".into(),
                "chart-axis",
                Value::Null,
                "linear-value-axis",
            ),
            Rect {
                x: axis_x0,
                y: rows_top - 6.0,
                width: HORIZONTAL_AXIS_W,
                height: 0.0,
            },
            line(axis_x0, rows_top - 6.0, axis_x1, rows_top - 6.0),
            vec![],
        ));
        for t in &tick_values {
            let x = x_at(*t);
            containers.push(container(
                authored(
                    &js_to_string(*t),
                    "/meta/unit".into(),
                    "chart-tick",
                    json!({"value": t, "zero": *t == 0.0}),
                    "nice-step-tick",
                ),
                Rect {
                    x,
                    y: rows_top - 4.0,
                    width: 0.0,
                    height: content_bottom - rows_top + 4.0,
                },
                line(x, rows_top - 4.0, x, content_bottom),
                vec![text(
                    "/meta/unit".into(),
                    &format!("{}{}", number(*t), suffix),
                    x,
                    rows_top - 12.0,
                    TICK_FONT,
                    "middle",
                )],
            ));
        }
        let band_w = width - 2.0 * MARGIN;
        for (position, row) in rows.iter().enumerate() {
            let band_y = rows_top + position as f64 * (row_h + ROW_BAND_GAP);
            let mut marks = Vec::new();
            let mut annots = Vec::new();
            for k in 0..n {
                let v = row.values[k].as_f64().unwrap_or_default();
                let bar_y = band_y + 6.0 + k as f64 * (SERIES_BAR_H + SERIES_BAR_GAP);
                let center_y = bar_y + SERIES_BAR_H / 2.0;
                let annot_path = format!("/rows/{}/values/{}", row.index, k);
                if v == 0.0 {
                    annots.push(text(
                        annot_path,
                        &number_text(k, v),
                        zero_x + 4.0,
                        center_y + 3.8,
                        VALUE_FONT,
                        "start",
                    ));
                    continue;
                }
                let (bar_x, bar_len, annot_x, anchor) = if v > 0.0 {
                    let len = (x_at(v) - zero_x).abs().max(1.0);
                    (zero_x, len, zero_x + len + 4.0, "start")
                } else {
                    // Anchor at zero so the 1 px minimum grows leftward, not across zero.
                    let len = (zero_x - x_at(v)).abs().max(1.0);
                    (zero_x - len, len, zero_x - len - 4.0, "end")
                };
                marks.push(json!({
                    "series": k,
                    "tone": tone_of(k),
                    "x": bar_x,
                    "y": bar_y,
                    "width": bar_len,
                    "height": SERIES_BAR_H,
                }));
                annots.push(text(
                    annot_path,
                    &number_text(k, v),
                    annot_x,
                    center_y + 3.8,
                    VALUE_FONT,
                    anchor,
                ));
            }
            let mut row_texts = vec![text(
                format!("/rows/{}/label", row.index),
                row.label,
                MARGIN,
                band_y + row_h / 2.0 + 4.3,
                LABEL_FONT,
                "start",
            )];
            row_texts.extend(annots);
            containers.push(Container {
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    json!({
                        "tone": tone_of(0),
                        "highlight": row.highlight,
                        "dim": has_highlight && !row.highlight,
                        "sublabel": sublabel_of(row),
                        "marks": marks,
                    }),
                    "authored-values",
                ),
                bounds: Rect {
                    x: MARGIN,
                    y: band_y,
                    width: band_w,
                    height: row_h,
                },
                label_plate: None,
                rule: None,
                members: vec![row.id.into()],
                text: vec![],
            });
            nodes.push(Node {
                brand: None,
                authored: authored(
                    row.id,
                    format!("/rows/{}", row.index),
                    "chart-row",
                    document["rows"][row.index].clone(),
                    "authored-values",
                ),
                bounds: Rect {
                    x: MARGIN,
                    y: band_y,
                    width: band_w,
                    height: row_h,
                },
                sigil: None,
                label: row.label.into(),
                text: row_texts,
                detail_rows: vec![],
            });
            rows_facts.push(json!({"id": row.id, "values": row.values}));
        }
    }

    // The illustrative badge shares the waterfall's header band.
    if evidence == "illustrative" {
        let badge_text = locale.text("chart.evidence.illustrative", &[]);
        let badge_w = (text_units(&badge_text) * 10.5 * ADVANCE + 24.0).ceil();
        containers.push(container(
            authored(
                "evidence",
                "/meta/evidence".into(),
                "chart-evidence",
                json!({"evidence": evidence}),
                "renderer-owned-header",
            ),
            Rect {
                x: MARGIN,
                y: 14.0,
                width: badge_w,
                height: 22.0,
            },
            None,
            vec![text(
                "/meta/evidence".into(),
                &badge_text,
                MARGIN + badge_w / 2.0 + 5.0,
                29.0,
                10.5,
                "middle",
            )],
        ));
    }

    // Legends: series keys once two or more series compete; limits carry
    // fixed entries for the fill, the limit line and any over row.
    let mut entries: Vec<LegendEntry> = Vec::new();
    if form == "limits" {
        entries.push(LegendEntry {
            kind: "limits-current".into(),
            key: String::new(),
            label: locale.text("legend.chart.current", &[]),
            tone: "frontend",
            width: 0.0,
        });
        entries.push(LegendEntry {
            kind: "limits-limit".into(),
            key: String::new(),
            label: locale.text("legend.chart.limit", &[]),
            tone: "external",
            width: 0.0,
        });
        if any_over {
            entries.push(LegendEntry {
                kind: "limits-over".into(),
                key: String::new(),
                label: locale.text("legend.chart.over", &[]),
                tone: "security",
                width: 0.0,
            });
        }
    } else if series.len() >= 2 {
        for (k, (id, name, key)) in series.iter().enumerate() {
            entries.push(LegendEntry {
                kind: format!("series:{id}"),
                key: key.clone(),
                label: format!("{key} \u{b7} {name}"),
                tone: tone_of(k),
                width: 0.0,
            });
        }
    }
    for entry in &mut entries {
        entry.width = (22.0 + text_units(&entry.label) * LEGEND_FONT * 0.62).ceil();
    }
    let max_entry = entries.iter().map(|entry| entry.width).fold(0.0, f64::max);
    let width = width.max(MARGIN * 2.0 + max_entry).ceil();
    let mut wrapped: Vec<Vec<usize>> = vec![];
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        match wrapped.last_mut() {
            Some(row) if cursor + 22.0 + entry.width <= width - MARGIN * 2.0 => {
                row.push(index);
                cursor += 22.0 + entry.width;
            }
            _ => {
                wrapped.push(vec![index]);
                cursor = entry.width;
            }
        }
    }
    let extra = (wrapped.len().saturating_sub(1)) as f64 * 22.0;
    let height = (content_bottom
        + MARGIN
        + if entries.is_empty() {
            0.0
        } else {
            34.0 + extra
        })
    .ceil();
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
        for (row_index, row) in wrapped.iter().enumerate() {
            let row_baseline = baseline - (wrapped.len() - row_index - 1) as f64 * 22.0;
            let mut x = MARGIN;
            for &index in row {
                let entry = &entries[index];
                legends.push(Card {
                    authored: authored(
                        &entry.kind,
                        "/series".into(),
                        &entry.kind,
                        json!({"tone": entry.tone, "key": entry.key}),
                        "resolved-legend-footprint",
                    ),
                    bounds: Rect {
                        x,
                        y: row_baseline - 10.0,
                        width: entry.width,
                        height: 14.0,
                    },
                    text: vec![text(
                        "/series".into(),
                        &entry.label,
                        x + 22.0,
                        row_baseline,
                        LEGEND_FONT,
                        "start",
                    )],
                });
                x += entry.width + 22.0;
            }
        }
    }

    // The table carries every value once, written the way the figure writes it.
    let table = if form == "pie" {
        let total: f64 = rows
            .iter()
            .map(|row| row.values[0].as_f64().unwrap_or_default())
            .sum();
        let mut columns = vec![
            json!(locale.text("chart.table.item", &[])),
            json!(locale.text("chart.table.value", &[])),
        ];
        if unit != "%" {
            columns.push(json!(locale.text("chart.table.share", &[])));
        }
        let mut table_rows = Vec::new();
        for row in &rows {
            let v = row.values[0].as_f64().unwrap_or_default();
            let mut cells = vec![
                json!(row.label),
                json!(format!("{}{}", js_to_string(v), suffix)),
            ];
            if unit != "%" {
                cells.push(json!(format!(
                    "{}%",
                    js_to_string(js_round(v / total * 100.0 * 10.0) / 10.0)
                )));
            }
            table_rows.push(Value::from(cells));
        }
        json!({"columns": columns, "rows": table_rows})
    } else if form == "limits" {
        let with_note = rows.iter().any(|row| !row.note.is_empty());
        let mut columns = vec![
            json!(locale.text("chart.table.item", &[])),
            json!(locale.text("chart.table.current", &[])),
            json!(locale.text("chart.table.limit", &[])),
            json!(locale.text("chart.table.unit", &[])),
            json!(locale.text("chart.table.status", &[])),
        ];
        if with_note {
            columns.push(json!(locale.text("chart.table.note", &[])));
        }
        let mut table_rows = Vec::new();
        for row in &rows {
            let limit = row.limit.unwrap_or_default();
            let row_unit = if row.row_unit.is_empty() {
                unit
            } else {
                row.row_unit
            };
            let status = if row.value.is_none() {
                "\u{2014}".to_owned()
            } else if row.value.is_some_and(|v| v > limit) {
                locale.text("chart.limit.over", &[])
            } else {
                locale.text("chart.limit.within", &[])
            };
            let mut cells = vec![
                json!(row.label),
                json!(row
                    .value
                    .map(js_to_string)
                    .unwrap_or_else(|| "\u{2014}".into())),
                json!(js_to_string(limit)),
                json!(row_unit),
                json!(status),
            ];
            if with_note {
                cells.push(json!(row.note));
            }
            table_rows.push(Value::from(cells));
        }
        json!({"columns": columns, "rows": table_rows})
    } else {
        let mut columns = vec![json!(locale.text("chart.table.item", &[]))];
        if series.is_empty() {
            let name = locale.text("chart.table.value", &[]);
            columns.push(json!(if unit.is_empty() {
                name
            } else {
                format!("{name}\u{ff08}{unit}\u{ff09}")
            }));
        } else {
            for (_, name, _) in &series {
                columns.push(json!(if unit.is_empty() {
                    (*name).to_owned()
                } else {
                    format!("{name}\u{ff08}{unit}\u{ff09}")
                }));
            }
        }
        let mut table_rows = Vec::new();
        for row in &rows {
            let mut cells = vec![json!(row.label)];
            for k in 0..n {
                cells.push(json!(number_text(
                    k,
                    row.values[k].as_f64().unwrap_or_default()
                )));
            }
            table_rows.push(Value::from(cells));
        }
        json!({"columns": columns, "rows": table_rows})
    };

    Ok(CompiledDiagram {
        diagram_type: DiagramType::Chart,
        schema_version: document["schema_version"].as_u64().unwrap_or(1) as u32,
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
        facts: DiagramFacts::Chart {
            form: form.into(),
            orientation: orientation.into(),
            unit: unit.into(),
            evidence: evidence.into(),
            series: series
                .iter()
                .map(|(id, name, key)| json!({"id": id, "name": name, "key": key}))
                .collect(),
            rows: rows_facts,
            table,
        },
        diagnostics: vec![],
    })
}
