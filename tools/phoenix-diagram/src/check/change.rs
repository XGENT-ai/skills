//! Artifact check for single-input change marks. Runs for every delivered
//! figure: the root `data-change-ui` must match the metadata's change facts,
//! and when facts exist every mark, badge count and band number must agree.

use super::{selector, ContentFinding};
use scraper::ElementRef;
use serde_json::{json, Value};

fn finding(path: String, message: String, evidence: Value) -> ContentFinding {
    ("artifact/change-mark", path, message, evidence)
}

pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings = Vec::new();
    let marked = metadata.get("changes").is_some();
    let ui = svg.value().attr("data-change-ui").is_some();
    if marked != ui {
        findings.push(finding(
            "/metadata/changes".to_owned(),
            format!(
                "The figure {} data-change-ui while metadata {} change facts",
                if ui { "has" } else { "lacks" },
                if marked { "has" } else { "lacks" }
            ),
            Value::Null,
        ));
    }
    let Some(changes) = metadata.get("changes") else {
        return findings;
    };

    for (id, mark) in changes["nodes"]
        .as_object()
        .map_or(Default::default(), |m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect::<Vec<_>>()
        })
    {
        let path = format!("/metadata/changes/nodes/{id}");
        match svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id.as_str()))
        {
            Some(group) => {
                if group.value().attr("data-change") != Some(mark.as_str()) {
                    findings.push(finding(
                        path,
                        format!("Node {id} does not carry data-change={mark}"),
                        json!({"id": id, "expected": mark, "actual": group.value().attr("data-change")}),
                    ));
                }
            }
            None => findings.push(finding(
                path,
                format!("Marked node {id} is missing from the figure"),
                json!({"id": id}),
            )),
        }
    }

    let relations = changes["relations"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    for (index, mark) in relations.iter().enumerate() {
        let path = format!("/metadata/changes/relations/{index}");
        let expected = mark.as_str();
        match svg
            .select(&selector("path[data-edge-key]"))
            .find(|path| path.value().attr("data-edge-key") == Some(index.to_string().as_str()))
        {
            Some(path) => {
                if path.value().attr("data-change") != expected {
                    findings.push(finding(
                        path_name(&path),
                        format!(
                            "Relationship {index} does not carry the expected data-change"
                        ),
                        json!({"index": index, "expected": expected, "actual": path.value().attr("data-change")}),
                    ));
                }
            }
            None => {
                if expected.is_some() {
                    findings.push(finding(
                        path,
                        format!("Marked relationship {index} is missing from the figure"),
                        json!({"index": index}),
                    ));
                }
            }
        }
    }

    for (source_path, mark) in changes["fields"]
        .as_object()
        .map_or(Default::default(), |m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect::<Vec<_>>()
        })
    {
        let path = format!("/metadata/changes/fields/{source_path}");
        match svg
            .select(&selector("g[data-er-row]"))
            .find(|row| row.value().attr("data-source-path") == Some(source_path.as_str()))
        {
            Some(row) => {
                if row.value().attr("data-change") != Some(mark.as_str()) {
                    findings.push(finding(
                        path,
                        format!("Field row {source_path} does not carry data-change={mark}"),
                        json!({"sourcePath": source_path, "expected": mark}),
                    ));
                }
            }
            None => findings.push(finding(
                path,
                format!("Marked field row {source_path} is missing from the figure"),
                json!({"sourcePath": source_path}),
            )),
        }
    }

    let counts = &changes["counts"];
    for (mark, count) in [
        ("added", counts["added"].as_u64()),
        ("removed", counts["removed"].as_u64()),
        ("changed", counts["changed"].as_u64()),
    ] {
        let Some(count) = count else {
            continue;
        };
        let actual = svg
            .select(&selector(&format!("[data-change-mark=\"{mark}\"]")))
            .count() as u64;
        if actual != count {
            findings.push(finding(
                format!("/metadata/changes/counts/{mark}"),
                format!("The figure shows {actual} {mark} change badges instead of {count}"),
                json!({"expected": count, "actual": actual}),
            ));
        }
        let counted: u64 = svg
            .select(&selector(&format!("[data-change-count=\"{mark}\"]")))
            .map(|tspan| {
                tspan
                    .text()
                    .collect::<String>()
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .sum();
        if counted != count {
            findings.push(finding(
                format!("/metadata/changes/counts/{mark}"),
                format!("The description band reports {counted} for {mark} instead of {count}"),
                json!({"expected": count, "actual": counted}),
            ));
        }
    }

    let bands: Vec<ElementRef<'_>> = svg.select(&selector("g[data-change-band]")).collect();
    if bands.len() != 1 {
        findings.push(finding(
            "/metadata/changes/band".to_owned(),
            format!(
                "The figure carries {} change bands instead of 1",
                bands.len()
            ),
            Value::Null,
        ));
        return findings;
    }
    let band = bands[0];
    for when in ["changes", "before", "after"] {
        if band
            .select(&selector(&format!("g[data-change-when=\"{when}\"]")))
            .count()
            != 1
        {
            findings.push(finding(
                "/metadata/changes/band".to_owned(),
                format!("The change band lacks exactly one {when} group"),
                json!({"when": when}),
            ));
        }
    }
    let changed = counts["changed"].as_u64().unwrap_or(0);
    for when in ["before", "after"] {
        let notes = band
            .select(&selector(&format!(
                "g[data-change-when=\"{when}\"] [data-change-note]"
            )))
            .count();
        if (changed > 0 && notes == 0) || (changed == 0 && notes > 0) {
            findings.push(finding(
                "/metadata/changes/counts/changed".to_string(),
                format!(
                    "The {when} band group {} the changed-items note while the metadata marks {} changed items",
                    if notes > 0 { "has" } else { "lacks" },
                    changed
                ),
                json!({"changed": changed, "notes": notes}),
            ));
        }
    }
    findings
}

fn path_name(path: &ElementRef<'_>) -> String {
    format!(
        "/metadata/changes/relations/{}",
        path.value().attr("data-edge-key").unwrap_or_default()
    )
}
