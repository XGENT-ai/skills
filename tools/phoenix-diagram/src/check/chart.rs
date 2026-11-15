use super::{number, selector, visible_text, ContentFinding};
use crate::format::number::{js_round, js_to_string};
use scraper::ElementRef;
use serde_json::{json, Value};

fn finding(code: &'static str, path: &str, message: &str, evidence: Value) -> ContentFinding {
    (code, path.into(), message.into(), evidence)
}

fn text_of(element: ElementRef<'_>) -> String {
    visible_text(&element.text().collect::<String>())
}

// Unit text right after a number, matching the layout's written form.
fn unit_suffix(unit: &str) -> String {
    if unit.is_empty() {
        String::new()
    } else if unit == "%" {
        "%".into()
    } else {
        format!(" {unit}")
    }
}

// Rows keep their label, annotation texts and numeric attributes; bars and
// ticks must agree with the authored values on one linear scale.
pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings = Vec::new();
    let facts = &metadata["facts"];
    let form = facts["form"].as_str().unwrap_or_default();
    let rows = facts["rows"].as_array().map_or(&[][..], Vec::as_slice);
    let series = facts["series"].as_array().map_or(&[][..], Vec::as_slice);
    let authored = metadata
        .pointer("/graph/nodes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    for (index, row) in rows.iter().enumerate() {
        let id = row["id"].as_str().unwrap_or_default();
        let path = format!("/metadata/facts/rows/{index}");
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A row node is missing from the figure",
                json!({"id": id}),
            ));
            continue;
        };
        let label = authored
            .iter()
            .find(|raw| raw["id"].as_str() == Some(id))
            .and_then(|raw| raw["label"].as_str())
            .unwrap_or_default();
        if group
            .select(&selector("text[data-node-label]"))
            .next()
            .is_none_or(|text| text_of(text) != visible_text(label))
        {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A row label is missing or changed",
                json!({"id": id}),
            ));
        }
        if form == "limits" {
            continue; // limits rows carry value/limit/over, checked in limits_checks
        }
        let suffix = unit_suffix(facts["unit"].as_str().unwrap_or_default());
        let expected: Vec<String> = if form == "pie" {
            let total: f64 = rows
                .iter()
                .map(|row| row["values"][0].as_f64().unwrap_or_default())
                .sum();
            let v = row["values"][0].as_f64().unwrap_or_default();
            let mut expected = vec![format!("{}{}", js_to_string(v), suffix)];
            if facts["unit"].as_str() != Some("%") {
                expected.push(format!(
                    "{}%",
                    js_to_string(js_round(v / total * 100.0 * 10.0) / 10.0)
                ));
            }
            expected
        } else {
            let values = row["values"].as_array().map_or(&[][..], Vec::as_slice);
            values
                .iter()
                .enumerate()
                .map(|(k, value)| {
                    let text = js_to_string(value.as_f64().unwrap_or_default());
                    if series.len() >= 2 {
                        format!("{} {}", series[k]["key"].as_str().unwrap_or_default(), text)
                    } else {
                        text
                    }
                })
                .collect()
        };
        let actual: Vec<String> = (0..expected.len())
            .map(|k| {
                group
                    .select(&selector(&format!("text[data-chart-value-label=\"{k}\"]")))
                    .next()
                    .map(text_of)
                    .unwrap_or_default()
            })
            .collect();
        if expected
            .iter()
            .zip(&actual)
            .any(|(expected, actual)| *actual != visible_text(expected))
        {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A row's value annotations differ from its facts",
                json!({"expected": expected}),
            ));
        }
        let expected_attr: String = row["values"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|value| js_to_string(value.as_f64().unwrap_or_default()))
            .collect::<Vec<_>>()
            .join(",");
        if group.value().attr("data-chart-values").unwrap_or_default() != expected_attr {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A row's numeric attributes differ from its facts",
                json!({"id": id}),
            ));
        }
    }
    match form {
        "bar" => bar_scale(svg, facts, &mut findings),
        "line" => line_scale(svg, facts, &mut findings),
        "pie" => pie_checks(svg, facts, &mut findings),
        "limits" => limits_checks(svg, metadata, &mut findings),
        _ => {}
    }
    let badge = svg
        .select(&selector("g[data-chart-evidence]"))
        .next()
        .and_then(|badge| badge.value().attr("data-chart-evidence"));
    // The badge appears exactly when the author declared illustrative values;
    // sourced numbers draw no badge at all.
    let expected_badge = if facts["evidence"].as_str() == Some("illustrative") {
        Some("illustrative")
    } else {
        None
    };
    if badge != expected_badge {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/evidence",
            "The evidence badge differs from the declared values",
            json!({"evidence": facts["evidence"]}),
        ));
    }
    // Series legend entries pair every series with its key, and the colour
    // block itself carries that key rather than the authored id.
    if series.len() >= 2 {
        for entry in series {
            let id = entry["id"].as_str().unwrap_or_default();
            let key = entry["key"].as_str().unwrap_or_default();
            let expected = format!(
                "{} \u{b7} {}",
                key,
                entry["name"].as_str().unwrap_or_default()
            );
            let legend = svg
                .select(&selector("g[data-graph-role='legend']"))
                .find(|legend| {
                    legend.value().attr("data-legend-semantic-kind")
                        == Some(format!("series:{id}").as_str())
                });
            let glyph_key = legend
                .and_then(|legend| legend.select(&selector("text")).next())
                .is_some_and(|text| text_of(text) == visible_text(key));
            let found = legend.is_some_and(|legend| {
                text_of(legend).contains(&visible_text(&expected))
            });
            if !found || !glyph_key {
                findings.push(finding(
                    "artifact/semantic-text",
                    "/metadata/facts/series",
                    "A series legend entry is missing, changed or shows the series id",
                    json!({"id": id, "expected": expected, "key": key}),
                ));
            }
        }
    }
    findings
}

// Bars read against the tick scale: one edge on the zero line, the other at
// the value's position, negative bars on the far side of zero.
fn bar_scale(svg: ElementRef<'_>, facts: &Value, findings: &mut Vec<ContentFinding>) {
    let vertical = facts["orientation"].as_str() != Some("horizontal");
    let mut ticks: Vec<(f64, f64)> = Vec::new();
    for tick in svg.select(&selector("line[data-chart-tick]")) {
        let value = tick
            .value()
            .attr("data-chart-tick")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        let position = if vertical {
            number(tick, "y1", f64::NAN)
        } else {
            number(tick, "x1", f64::NAN)
        };
        ticks.push((value, position));
    }
    let Some(&(_, zero)) = ticks.iter().find(|(value, _)| *value == 0.0) else {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/rows",
            "A bar axis has no zero tick",
            Value::Null,
        ));
        return;
    };
    // The farthest tick fixes the scale so coordinate rounding near zero is
    // not multiplied across the rest of the axis.
    let scale = ticks
        .iter()
        .filter(|(value, position)| ((position - zero) / value).is_finite())
        .max_by(|a, b| a.0.abs().total_cmp(&b.0.abs()))
        .map(|(value, position)| (position - zero) / value);
    let Some(scale) = scale else {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/rows",
            "A bar axis needs a second tick to fix its scale",
            Value::Null,
        ));
        return;
    };
    for (value, position) in &ticks {
        if (zero + value * scale - position).abs() > 0.05 {
            findings.push(finding(
                "artifact/semantic-relation",
                "/metadata/facts/rows",
                "A tick is not on the shared linear scale",
                json!({"value": value}),
            ));
        }
    }
    for (index, row) in facts["rows"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
    {
        let id = row["id"].as_str().unwrap_or_default();
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        for (k, value) in row["values"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .enumerate()
        {
            let v = value.as_f64().unwrap_or_default();
            let path = format!("/metadata/facts/rows/{index}/values/{k}");
            let mark = group.children().filter_map(ElementRef::wrap).find(|child| {
                child.value().name() == "rect"
                    && child.value().attr("data-chart-series") == Some(k.to_string().as_str())
            });
            if v == 0.0 {
                if mark.is_some() {
                    findings.push(finding(
                        "artifact/semantic-relation",
                        &path,
                        "A zero value must not draw a bar",
                        json!({"id": id, "series": k}),
                    ));
                }
                continue;
            }
            let Some(mark) = mark else {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A non-zero value is missing its bar",
                    json!({"id": id, "series": k}),
                ));
                continue;
            };
            let (start, length) = if vertical {
                (
                    number(mark, "y", f64::NAN),
                    number(mark, "height", f64::NAN),
                )
            } else {
                (number(mark, "x", f64::NAN), number(mark, "width", f64::NAN))
            };
            // Layout keeps every non-zero bar at least 1 px long so it stays visible.
            let exact = v * scale;
            let outer = zero + exact.signum() * exact.abs().max(1.0);
            let (near, far) = match (vertical, v > 0.0) {
                (true, true) | (false, false) => (start + length, start),
                (true, false) | (false, true) => (start, start + length),
            };
            if (near - zero).abs() > 0.05 || (far - outer).abs() > 0.05 {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A bar is not placed on the shared linear scale from zero",
                    json!({"value": v, "zero": zero, "outer": outer}),
                ));
            }
        }
    }
}

// Line points read against the tick scale; the axis need not include zero.
fn line_scale(svg: ElementRef<'_>, facts: &Value, findings: &mut Vec<ContentFinding>) {
    let vertical = facts["orientation"].as_str() != Some("horizontal");
    let mut ticks: Vec<(f64, f64)> = Vec::new();
    for tick in svg.select(&selector("line[data-chart-tick]")) {
        let value = tick
            .value()
            .attr("data-chart-tick")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        let position = if vertical {
            number(tick, "y1", f64::NAN)
        } else {
            number(tick, "x1", f64::NAN)
        };
        ticks.push((value, position));
    }
    // Measure against the farthest tick so coordinate rounding is not multiplied.
    let scale = ticks.first().and_then(|(value, position)| {
        ticks
            .iter()
            .filter_map(|(other_value, other_position)| {
                let candidate = (other_position - position) / (other_value - value);
                candidate
                    .is_finite()
                    .then_some(((other_value - value).abs(), candidate))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, candidate)| candidate)
    });
    let Some(scale) = scale else {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/rows",
            "A line axis needs two ticks to fix its scale",
            Value::Null,
        ));
        return;
    };
    let (anchor_value, anchor_position) = ticks[0];
    for (index, row) in facts["rows"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
    {
        let id = row["id"].as_str().unwrap_or_default();
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        for (k, value) in row["values"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .enumerate()
        {
            let v = value.as_f64().unwrap_or_default();
            let path = format!("/metadata/facts/rows/{index}/values/{k}");
            let Some(point) = group.children().filter_map(ElementRef::wrap).find(|child| {
                child.value().name() == "circle"
                    && child.value().attr("data-chart-series") == Some(k.to_string().as_str())
            }) else {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A line point is missing for its value",
                    json!({"id": id, "series": k}),
                ));
                continue;
            };
            let position = if vertical {
                number(point, "cy", f64::NAN)
            } else {
                number(point, "cx", f64::NAN)
            };
            let expected = anchor_position + (v - anchor_value) * scale;
            if (position - expected).abs() > 0.05 {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A line point is not on the shared linear scale",
                    json!({"value": v, "expected": expected}),
                ));
            }
        }
    }
}

// Pie sectors and the list must both read from the same values.
fn pie_checks(svg: ElementRef<'_>, facts: &Value, findings: &mut Vec<ContentFinding>) {
    let rows = facts["rows"].as_array().map_or(&[][..], Vec::as_slice);
    let unit = facts["unit"].as_str().unwrap_or_default();
    let suffix = unit_suffix(unit);
    let total: f64 = rows
        .iter()
        .map(|row| row["values"][0].as_f64().unwrap_or_default())
        .sum();
    let mut sweep_sum = 0.0f64;
    for (index, row) in rows.iter().enumerate() {
        let id = row["id"].as_str().unwrap_or_default();
        let path = format!("/metadata/facts/rows/{index}");
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        let v = row["values"][0].as_f64().unwrap_or_default();
        let sector = group
            .children()
            .filter_map(ElementRef::wrap)
            .find(|child| child.value().attr("data-chart-sweep").is_some());
        if v > 0.0 {
            let expected = v / total * 360.0;
            let sweep = sector
                .and_then(|sector| sector.value().attr("data-chart-sweep"))
                .and_then(|sweep| sweep.parse::<f64>().ok());
            match sweep {
                Some(sweep) if (sweep - expected).abs() <= 0.5 => {}
                _ => findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A sector sweep differs from its share of the whole",
                    json!({"value": v, "expected": expected}),
                )),
            }
            if let Some(sweep) = sweep {
                sweep_sum += sweep;
            }
        } else if sector.is_some() {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A zero value must not draw a sector",
                json!({"id": id}),
            ));
        }
        let expected_value = format!("{}{}", js_to_string(v), suffix);
        let actual_value = group
            .select(&selector("text[data-chart-value-label='0']"))
            .next()
            .map(text_of)
            .unwrap_or_default();
        if actual_value != visible_text(&expected_value) {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A pie list value differs from its facts",
                json!({"expected": expected_value}),
            ));
        }
        if unit != "%" {
            let expected_share = format!(
                "{}%",
                js_to_string(js_round(v / total * 100.0 * 10.0) / 10.0)
            );
            let actual_share = group
                .select(&selector("text[data-chart-value-label='1']"))
                .next()
                .map(text_of)
                .unwrap_or_default();
            if actual_share != visible_text(&expected_share) {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A pie list share differs from its facts",
                    json!({"expected": expected_share}),
                ));
            }
        }
    }
    if total > 0.0 && (sweep_sum - 360.0).abs() > 1.0 {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/rows",
            "The sector sweeps must cover the whole ring",
            json!({"sum": sweep_sum}),
        ));
    }
}

// Limits read fill-to-track ratios, the limit line and the over note.
fn limits_checks(svg: ElementRef<'_>, metadata: &Value, findings: &mut Vec<ContentFinding>) {
    let facts = &metadata["facts"];
    let locale = crate::i18n::resolve(&metadata["meta"]);
    let over_text = visible_text(&locale.text("chart.limit.over", &[]));
    let rows = facts["rows"].as_array().map_or(&[][..], Vec::as_slice);
    for (index, row) in rows.iter().enumerate() {
        let id = row["id"].as_str().unwrap_or_default();
        let path = format!("/metadata/facts/rows/{index}");
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        let value = row["value"].as_f64();
        let limit = row["limit"].as_f64().unwrap_or_default();
        let expected_value = value.map(js_to_string).unwrap_or_default();
        let actual_value = group.value().attr("data-chart-value").unwrap_or_default();
        let actual_limit = group.value().attr("data-chart-limit").unwrap_or_default();
        let actual_over = group.value().attr("data-chart-over").unwrap_or_default();
        if actual_value != expected_value
            || actual_limit != js_to_string(limit)
            || actual_over != row["over"].as_bool().unwrap_or_default().to_string()
        {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A limits row's numeric attributes differ from its facts",
                json!({"id": id}),
            ));
        }
        let track = group.select(&selector("rect.ch-track")).next();
        let fill = group.select(&selector("rect.ch-fill")).next();
        let span = group.select(&selector("rect.ch-span")).next();
        let line = group.select(&selector("line.ch-limit")).next();
        let (Some(track), Some(line)) = (track, line) else {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A limits row is missing its track or limit line",
                json!({"id": id}),
            ));
            continue;
        };
        // A current value fills the track; a limit-only row shows an outlined
        // span instead, so the two cases never read as the same mark.
        let bar = match (value, fill, span) {
            (Some(_), Some(fill), None) => fill,
            (None, None, Some(span)) => span,
            (value, fill, span) => {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A limits row needs a fill for its current value or a span for a limit alone",
                    json!({"id": id, "value": value, "fill": fill.is_some(), "span": span.is_some()}),
                ));
                continue;
            }
        };
        let full = value.unwrap_or(limit).max(limit) * 1.25;
        let expected_ratio = value.unwrap_or(limit) / full;
        let track_width = number(track, "width", f64::NAN);
        let bar_width = number(bar, "width", f64::NAN);
        let actual_ratio = bar_width / track_width;
        if !actual_ratio.is_finite() || (actual_ratio - expected_ratio).abs() > 0.01 {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A limits bar is not proportional to its track scale",
                json!({"expected": expected_ratio, "actual": actual_ratio}),
            ));
        }
        let expected_x = number(track, "x", f64::NAN) + limit / full * track_width;
        let line_x = number(line, "x1", f64::NAN);
        if (line_x - expected_x).abs() > 0.05 {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "The limit line is not at the authored limit",
                json!({"expected": expected_x, "actual": line_x}),
            ));
        }
        let has_over_text = group
            .select(&selector("text"))
            .any(|text| text_of(text).contains(&over_text));
        let over_expected = value.is_some_and(|v| v > limit);
        if has_over_text != over_expected {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "The over note must appear exactly when the value exceeds the limit",
                json!({"over": over_expected}),
            ));
        }
    }
    // The limit-only entry explains the outlined span exactly when one exists.
    let any_limit_only = rows.iter().any(|row| row["value"].is_null());
    let legend = svg
        .select(&selector("g[data-graph-role='legend']"))
        .any(|legend| {
            legend.value().attr("data-legend-semantic-kind") == Some("limits-limit-only")
        });
    if legend != any_limit_only {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/rows",
            "The limit-only legend entry must appear exactly when a row has only a limit",
            json!({"expected": any_limit_only}),
        ));
    }
}

// The reader's data table must mirror facts.table cell for cell.
pub(super) fn table(document: &scraper::Html, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings = Vec::new();
    let spec = &metadata["facts"]["table"];
    let details: Vec<_> = document.select(&selector("details.chart-data")).collect();
    if details.len() != 1 {
        findings.push(finding(
            "artifact/semantic-text",
            "/metadata/facts/table",
            "The data table must appear exactly once",
            json!({"actual": details.len()}),
        ));
        return findings;
    }
    let headers: Vec<String> = details[0]
        .select(&selector("thead th"))
        .map(text_of)
        .collect();
    let expected: Vec<String> = spec["columns"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|column| visible_text(column.as_str().unwrap_or_default()))
        .collect();
    if headers != expected {
        findings.push(finding(
            "artifact/semantic-text",
            "/metadata/facts/table/columns",
            "The data table headers differ from the facts",
            json!({"expected": expected, "actual": headers}),
        ));
    }
    let body_rows: Vec<_> = details[0].select(&selector("tbody tr")).collect();
    let spec_rows = spec["rows"].as_array().map_or(&[][..], Vec::as_slice);
    if body_rows.len() != spec_rows.len() {
        findings.push(finding(
            "artifact/semantic-text",
            "/metadata/facts/table/rows",
            "The data table row count differs from the facts",
            json!({"expected": spec_rows.len(), "actual": body_rows.len()}),
        ));
        return findings;
    }
    for (index, (row, spec_row)) in body_rows.iter().zip(spec_rows).enumerate() {
        let cells: Vec<String> = row.select(&selector("th, td")).map(text_of).collect();
        let expected_cells: Vec<String> = spec_row
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|cell| visible_text(cell.as_str().unwrap_or_default()))
            .collect();
        if cells != expected_cells {
            findings.push(finding(
                "artifact/semantic-text",
                &format!("/metadata/facts/table/rows/{index}"),
                "A data table row differs from the facts",
                json!({"expected": expected_cells, "actual": cells}),
            ));
        }
    }
    findings
}
