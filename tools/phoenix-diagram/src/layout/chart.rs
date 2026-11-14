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
const PLOT_H: f64 = 240.0;
const BAR_W: f64 = 22.0;
const BAR_GAP: f64 = 4.0;
const SLOT_PAD: f64 = 24.0;
const ANNOT_ROOM: f64 = 16.0;
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
    let value_text_w =
        |k: usize, v: f64| text_units(&number_text(k, v)) * VALUE_FONT * ADVANCE;
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

    if form != "bar" {
        // line, pie and limits join in M2.
        let mut diagnostic = Diagnostic::error(
            Some("chart"),
            "chart/unsupported-form",
            "/form",
            format!("The form {form:?} has no layout yet"),
        );
        diagnostic.stage = "layout".into();
        diagnostic.supported_fixes = vec!["Author form \"bar\" in this milestone".into()];
        return Err(vec![diagnostic]);
    }

    let mut containers: Vec<Container> = Vec::new();
    let mut nodes = Vec::new();
    let mut rows_facts: Vec<Value> = Vec::new();
    let width;
    let content_bottom;

    if orientation != "horizontal" {
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
            .fold(f64::NEG_INFINITY, f64::max);
        let plot_w = slot_w * rows.len() as f64;
        let (lo, hi, step) = {
            let (min_v, max_v) = value_range(&rows, n);
            axis_scale(min_v.min(0.0), max_v.max(0.0), plot_w, VERTICAL_MIN_TICK)?
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
        let y_at =
            |v: f64| js_round((plot_bottom - (v - lo) / (hi - lo) * PLOT_H) * 100.0) / 100.0;
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
        let x_at = |v: f64| {
            js_round((axis_x0 + (v - lo) / (hi - lo) * HORIZONTAL_AXIS_W) * 100.0) / 100.0
        };
        let zero_x = x_at(0.0);
        width = axis_x1 + pos_room + MARGIN;
        content_bottom =
            rows_top + rows.len() as f64 * (row_h + ROW_BAND_GAP) - ROW_BAND_GAP + 4.0;
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
                        zero_x,
                        center_y - 6.0,
                        VALUE_FONT,
                        "middle",
                    ));
                    continue;
                }
                let (bar_x, bar_len, annot_x, anchor) = if v > 0.0 {
                    let len = (x_at(v) - zero_x).abs().max(1.0);
                    (zero_x, len, zero_x + len + 4.0, "start")
                } else {
                    let len = (zero_x - x_at(v)).abs().max(1.0);
                    (x_at(v), len, x_at(v) - 4.0, "end")
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

    // Legends: series keys once two or more series compete.
    let mut entries: Vec<LegendEntry> = Vec::new();
    if series.len() >= 2 {
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
        + if entries.is_empty() { 0.0 } else { 34.0 + extra })
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
            cells.push(json!(number_text(k, row.values[k].as_f64().unwrap_or_default())));
        }
        table_rows.push(Value::from(cells));
    }
    let table = json!({"columns": columns, "rows": table_rows});

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
