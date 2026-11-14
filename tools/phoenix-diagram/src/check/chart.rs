use super::{number, selector, visible_text, ContentFinding};
use crate::format::number::js_to_string;
use scraper::ElementRef;
use serde_json::{json, Value};

fn finding(code: &'static str, path: &str, message: &str, evidence: Value) -> ContentFinding {
    (code, path.into(), message.into(), evidence)
}

fn text_of(element: ElementRef<'_>) -> String {
    visible_text(&element.text().collect::<String>())
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
            continue; // limits attributes join in M2
        }
        let values = row["values"].as_array().map_or(&[][..], Vec::as_slice);
        let expected: Vec<String> = values
            .iter()
            .enumerate()
            .map(|(k, value)| {
                let text = js_to_string(value.as_f64().unwrap_or_default());
                if series.len() >= 2 {
                    format!(
                        "{} {}",
                        series[k]["key"].as_str().unwrap_or_default(),
                        text
                    )
                } else {
                    text
                }
            })
            .collect();
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
        let expected_attr: String = values
            .iter()
            .map(|value| js_to_string(value.as_f64().unwrap_or_default()))
            .collect::<Vec<_>>()
            .join(",");
        if group
            .value()
            .attr("data-chart-values")
            .unwrap_or_default()
            != expected_attr
        {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A row's numeric attributes differ from its facts",
                json!({"id": id}),
            ));
        }
    }
    if form == "bar" {
        bar_scale(svg, facts, &mut findings);
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
    // Series legend entries pair every series with its key.
    if series.len() >= 2 {
        for entry in series {
            let id = entry["id"].as_str().unwrap_or_default();
            let expected = format!(
                "{} \u{b7} {}",
                entry["key"].as_str().unwrap_or_default(),
                entry["name"].as_str().unwrap_or_default()
            );
            let found = svg
                .select(&selector("g[data-graph-role='legend']"))
                .any(|legend| {
                    legend.value().attr("data-legend-semantic-kind")
                        == Some(format!("series:{id}").as_str())
                        && text_of(legend).contains(&visible_text(&expected))
                });
            if !found {
                findings.push(finding(
                    "artifact/semantic-text",
                    "/metadata/facts/series",
                    "A series legend entry is missing or changed",
                    json!({"id": id, "expected": expected}),
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
    let scale = ticks.iter().find_map(|(value, position)| {
        let candidate = (position - zero) / value;
        candidate.is_finite().then_some(candidate)
    });
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
            let mark = group
                .children()
                .filter_map(ElementRef::wrap)
                .find(|child| {
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
                (
                    number(mark, "x", f64::NAN),
                    number(mark, "width", f64::NAN),
                )
            };
            let outer = zero + v * scale;
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
    let headers: Vec<String> = details[0].select(&selector("thead th")).map(text_of).collect();
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
