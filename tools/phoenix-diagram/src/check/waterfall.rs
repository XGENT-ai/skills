use super::{number, selector, visible_text, ContentFinding};
use crate::format::number::js_to_string;
use crate::layout::waterfall::duration_label;
use scraper::ElementRef;
use serde_json::{json, Value};

fn finding(code: &'static str, path: &str, message: &str, evidence: Value) -> ContentFinding {
    (code, path.into(), message.into(), evidence)
}

fn text_of(element: ElementRef<'_>) -> String {
    visible_text(&element.text().collect::<String>())
}

// Every span keeps its name and its duration label; bar positions and
// lengths come from recorded timing on one linear scale.
pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings: Vec<ContentFinding> =
        super::timeline::relationships(svg).into_iter().collect();
    let facts = &metadata["facts"];
    let unit = facts["unit"].as_str().unwrap_or("ms");
    let spans = facts["spans"].as_array().map_or(&[][..], Vec::as_slice);
    let authored = metadata
        .pointer("/graph/nodes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let locale = crate::i18n::resolve(&metadata["meta"]);
    let start = |span: &Value| span["start"].as_f64().unwrap_or_default();
    let t0 = spans.iter().map(start).fold(f64::INFINITY, f64::min);
    let t1 = spans
        .iter()
        .map(|span| span["end"].as_f64().unwrap_or_else(|| start(span)))
        .fold(f64::NEG_INFINITY, f64::max);
    let mut bars = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        let id = span["id"].as_str().unwrap_or_default();
        let path = format!("/metadata/facts/spans/{index}");
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        let name = authored
            .iter()
            .find(|raw| raw["id"].as_str() == Some(id))
            .and_then(|raw| raw["name"].as_str())
            .unwrap_or_default();
        let texts: Vec<_> = group.select(&selector("text")).collect();
        if texts
            .iter()
            .find(|text| text.value().attr("data-node-label").is_some())
            .is_none_or(|label| text_of(*label) != visible_text(name))
        {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A span name is missing or changed",
                json!({"id":id}),
            ));
        }
        let open = span["end"].is_null();
        let end = span["end"].as_f64().unwrap_or(t1);
        let duration = end - start(span);
        let expected = duration_label(&locale, unit, duration, open);
        if !texts.iter().any(|text| {
            text.value().attr("data-node-label").is_none()
                && text_of(*text) == visible_text(&expected)
        }) {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A span's duration label differs from its recorded timing",
                json!({"id":id,"expected":expected}),
            ));
        }
        let attribute = |name: &str| group.value().attr(name).unwrap_or_default().to_owned();
        if attribute("data-waterfall-start") != js_to_string(start(span))
            || attribute("data-waterfall-end")
                != if open {
                    String::new()
                } else {
                    js_to_string(end)
                }
            || attribute("data-waterfall-duration") != js_to_string(duration)
            || attribute("data-waterfall-status") != span["status"].as_str().unwrap_or("ok")
        {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A span's Reader timing attributes differ from its facts",
                json!({"id":id}),
            ));
        }
        if let Some(bar) = group
            .children()
            .filter_map(ElementRef::wrap)
            .find(|child| child.value().attr("data-graph-role") == Some("node-body"))
        {
            bars.push((
                index,
                start(span),
                end,
                number(bar, "x", f64::NAN),
                number(bar, "width", f64::NAN),
            ));
        }
    }
    // One linear scale: the earliest start and the latest end span the axis.
    let left = bars.iter().find(|bar| bar.1 == t0).map(|bar| bar.3);
    let right = bars
        .iter()
        .find(|bar| bar.2 == t1 && bar.4 > 2.0)
        .map(|bar| bar.3 + bar.4);
    if let (Some(left), Some(right)) = (left, right) {
        let scale = (right - left) / (t1 - t0);
        for (index, start, end, x, width) in &bars {
            let (x0, x1) = (left + (start - t0) * scale, left + (end - t0) * scale);
            let off = (x - x0).abs() > 0.02 || (width - (x1 - x0).max(2.0)).abs() > 0.03;
            if !scale.is_finite() || off {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &format!("/metadata/facts/spans/{index}"),
                    "A span bar is not placed on the shared linear time scale",
                    json!({"x":x,"width":width,"expectedX":x0,"expectedEnd":x1}),
                ));
            }
        }
    }
    if svg
        .select(&selector("g[data-waterfall-evidence]"))
        .next()
        .and_then(|badge| badge.value().attr("data-waterfall-evidence"))
        != facts["evidence"].as_str()
        || svg.value().attr("data-waterfall-ui").is_none()
        || svg.value().attr("data-waterfall-unit") != Some(unit)
    {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/evidence",
            "The evidence badge or unit differs from the declared timing",
            json!({"evidence":facts["evidence"],"unit":unit}),
        ));
    }
    findings
}
