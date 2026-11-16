// Artifact checks for annotated prose: the drawn sentence must rebuild the
// authored text character for character, each numbered span must sit at its
// recorded range, and each card must quote its span, fix and note.
use super::{selector, visible_text, ContentFinding};
use scraper::{CaseSensitivity, ElementRef};
use serde_json::{json, Value};
use std::collections::HashMap;

fn finding(code: &'static str, path: &str, message: &str, evidence: Value) -> ContentFinding {
    (code, path.into(), message.into(), evidence)
}

struct Located {
    start: usize,
    text: String,
    issue: bool,
    markers: usize,
    marker_text: String,
}

pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings = Vec::new();
    let facts = &metadata["facts"];
    let Some(sentences) = facts["sentences"].as_array() else {
        return findings;
    };
    let mut lines: HashMap<String, Vec<ElementRef<'_>>> = HashMap::new();
    for element in svg.select(&selector("text[data-annotation-line]")) {
        if let Some(id) = element.value().attr("data-annotation-line") {
            lines.entry(id.to_owned()).or_default().push(element);
        }
    }
    let mut groups: HashMap<&str, ElementRef<'_>> = HashMap::new();
    for group in svg.select(&selector("g[data-node-id]")) {
        if let Some(id) = group.value().attr("data-node-id") {
            groups.insert(id, group);
        }
    }
    // Expected card text comes from the authored annotations, not from facts.
    // The graph lists one entry per node in drawing order, the same order this
    // loop walks, so the running slot binds a card to its authored object.
    let raw_nodes = metadata
        .pointer("/graph/nodes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);

    let mut located: HashMap<u64, Located> = HashMap::new();
    for (index, sentence) in sentences.iter().enumerate() {
        let path = format!("/metadata/facts/sentences/{index}");
        let id = sentence["id"].as_str().unwrap_or_default();
        let expected = sentence["text"].as_str().unwrap_or_default();
        let Some(sentence_lines) = lines.get(id) else {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A sentence has no drawn text lines",
                json!({"id": id}),
            ));
            continue;
        };
        let mut actual = String::new();
        for line in sentence_lines {
            let tspans: Vec<_> = line.select(&selector("tspan")).collect();
            for (position, tspan) in tspans.iter().enumerate() {
                let text = tspan.text().collect::<String>();
                if let Some(marker) = tspan.value().attr("data-annotation-marker") {
                    let previous = position
                        .checked_sub(1)
                        .and_then(|index| tspans.get(index))
                        .and_then(|previous| previous.value().attr("data-annotation-ref"));
                    if previous != Some(marker) {
                        findings.push(finding(
                            "artifact/semantic-text",
                            &path,
                            "A span marker does not immediately follow its span",
                            json!({"marker": marker}),
                        ));
                    }
                    if let Ok(number) = marker.parse::<u64>() {
                        if let Some(entry) = located.get_mut(&number) {
                            entry.markers += 1;
                            entry.marker_text = text;
                        }
                    }
                    continue;
                }
                if let Some(reference) = tspan.value().attr("data-annotation-ref") {
                    let Ok(number) = reference.parse::<u64>() else {
                        findings.push(finding(
                            "artifact/semantic-text",
                            &path,
                            "A span reference is not a number",
                            json!({"reference": reference}),
                        ));
                        actual.push_str(&text);
                        continue;
                    };
                    match located.get_mut(&number) {
                        Some(entry) => entry.text.push_str(&text),
                        None => {
                            located.insert(
                                number,
                                Located {
                                    start: actual.chars().count(),
                                    text: text.clone(),
                                    issue: tspan
                                        .value()
                                        .has_class("an-issue", CaseSensitivity::CaseSensitive),
                                    markers: 0,
                                    marker_text: String::new(),
                                },
                            );
                        }
                    }
                }
                actual.push_str(&text);
            }
        }
        if actual != expected {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "The drawn sentence differs from the authored text",
                json!({"length": actual.chars().count(), "expected": expected.chars().count()}),
            ));
        }
    }

    let mut expected_number = 1;
    let mut order = Vec::new();
    let mut slot = 0;
    for (index, sentence) in sentences.iter().enumerate() {
        let annotations = sentence["annotations"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        for (position, annotation) in annotations.iter().enumerate() {
            let path = format!("/metadata/facts/sentences/{index}/annotations/{position}");
            let number = annotation["number"].as_u64().unwrap_or_default();
            if number != expected_number {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "Annotation numbers are not continuous from 1",
                    json!({"number": number, "expected": expected_number}),
                ));
            }
            expected_number += 1;
            order.push(number.to_string());
            let node = annotation["node"].as_str().unwrap_or_default();
            let Some(raw) = raw_nodes.get(slot) else {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "An annotation card has no authored facts",
                    json!({"node": node}),
                ));
                break;
            };
            slot += 1;
            let issue = raw["issue"] == true;
            let span = raw["span"].as_str().unwrap_or_default();
            let input_index = node
                .rsplit_once('-')
                .map(|(_, tail)| tail)
                .unwrap_or_default();
            let expected_path = format!("/sentences/{index}/annotations/{input_index}");
            if let Some(group) = groups.get(node) {
                if group.value().attr("data-source-path") != Some(expected_path.as_str()) {
                    findings.push(finding(
                        "artifact/semantic-text",
                        &path,
                        "A card is bound to a different annotation than its facts",
                        json!({"node": node, "path": expected_path}),
                    ));
                }
                if group.value().attr("data-node-label") != Some(span) {
                    findings.push(finding(
                        "artifact/semantic-text",
                        &path,
                        "A card's label differs from its authored span",
                        json!({"node": node, "span": span}),
                    ));
                }
            }
            let Some(entry) = located.get(&number) else {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A numbered span is missing from the drawn text",
                    json!({"number": number}),
                ));
                continue;
            };
            if entry.text != span {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A span's drawn text differs from the authored span",
                    json!({"number": number, "drawn": entry.text, "span": span}),
                ));
            }
            if entry.start as u64 != annotation["start"].as_u64().unwrap_or_default() {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A span starts at a different character than its facts",
                    json!({"number": number, "start": entry.start, "recorded": annotation["start"]}),
                ));
            }
            if entry.issue != issue || annotation["issue"] != issue {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A span's issue style differs from its facts",
                    json!({"number": number}),
                ));
            }
            let expected_marker = if issue {
                format!("{number}!")
            } else {
                number.to_string()
            };
            if entry.markers != 1 || entry.marker_text != expected_marker {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A span needs exactly one marker carrying its number",
                    json!({"number": number, "markers": entry.markers, "text": entry.marker_text}),
                ));
            }
            let expected = number.to_string();
            let Some(group) = groups.get(node) else {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "An annotation card is missing from the figure",
                    json!({"node": node}),
                ));
                continue;
            };
            if group.value().attr("data-annotation-number") != Some(expected.as_str()) {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A card carries a different number than its span",
                    json!({"node": node, "number": number}),
                ));
            }
            let drawn = visible_text(&group.text().collect::<String>());
            for field in ["span", "fix", "note"] {
                if let Some(value) = raw[field].as_str().filter(|value| !value.is_empty()) {
                    if !drawn.contains(&visible_text(value)) {
                        findings.push(finding(
                            "artifact/semantic-text",
                            &path,
                            "A card is missing authored text",
                            json!({"node": node, "field": field}),
                        ));
                    }
                }
            }
            if issue != group.value().attr("data-annotation-issue").is_some() {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A card's issue state differs from its span",
                    json!({"node": node}),
                ));
            }
            let tag = group.select(&selector(".an-issue-tag")).next().is_some();
            if issue != tag {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "An issue card must show its tag and a plain card must not",
                    json!({"node": node}),
                ));
            }
        }
    }
    // Cards read in number order, the same order the annotations are drawn in.
    let drawn_order: Vec<&str> = svg
        .select(&selector("g[data-annotation-number]"))
        .filter_map(|group| group.value().attr("data-annotation-number"))
        .collect();
    if drawn_order != order.iter().map(String::as_str).collect::<Vec<_>>() {
        findings.push(finding(
            "artifact/semantic-text",
            "/metadata/facts/sentences",
            "Card order differs from the annotation order",
            json!({"drawn": drawn_order}),
        ));
    }
    findings
}
