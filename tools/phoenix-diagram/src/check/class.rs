// Class facts in the actual SVG: each type's kind, every member's complete
// signature (joined across wrapped lines) and the notation of every relationship.
use super::{selector, ContentFinding};
use crate::layout::class::{members, notation, stereotype};
use crate::scene::Marker;
use scraper::ElementRef;
use serde_json::{json, Value};

// Wrapping and the right-hand type column move whitespace and the `:` before
// an annotation; every other character of a signature must be drawn in order.
fn significant(value: &str) -> String {
    value
        .chars()
        .filter(|c| !crate::geometry::text::is_js_whitespace(*c) && *c != ':')
        .collect()
}

fn marker_name(marker: Marker) -> &'static str {
    match marker {
        Marker::DiamondFilled => "diamond-filled",
        Marker::DiamondHollow => "diamond-hollow",
        Marker::Triangle => "triangle",
        _ => "open",
    }
}

pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings: Vec<ContentFinding> = vec![];
    let empty = vec![];
    let types = metadata["facts"]["types"].as_array().unwrap_or(&empty);
    let relationships = metadata["facts"]["relationships"]
        .as_array()
        .unwrap_or(&empty);
    let nodes: Vec<_> = svg.select(&selector("g[data-node-id]")).collect();
    for (index, item) in types.iter().enumerate() {
        let id = item["id"].as_str().unwrap_or("");
        let kind = item["kind"].as_str().unwrap_or("class");
        let path = format!("/metadata/facts/types/{index}");
        let Some(node) = nodes
            .iter()
            .find(|node| node.value().attr("data-node-id") == Some(id))
        else {
            findings.push((
                "artifact/semantic-count",
                path,
                "Authored type is missing from the actual SVG".into(),
                json!({ "id": id }),
            ));
            continue;
        };
        let texts: Vec<_> = node.select(&selector("text")).collect();
        let drawn = |content: &str| {
            texts
                .iter()
                .any(|text| text.text().collect::<String>() == content)
        };
        if node.value().attr("data-class-kind") != Some(kind)
            || stereotype(kind).is_some_and(|keyword| !drawn(keyword))
        {
            findings.push((
                "artifact/semantic-text",
                format!("{path}/kind"),
                "Type kind differs from its actual SVG keyword".into(),
                json!({"id":id,"kind":kind}),
            ));
        }
        if let Some(sublabel) = item["sublabel"]
            .as_str()
            .filter(|text| !text.trim().is_empty())
        {
            let title = node
                .select(&selector("title"))
                .next()
                .map(|title| title.text().collect::<String>())
                .unwrap_or_default();
            if node.value().attr("data-node-sublabel") != Some(sublabel)
                || !title.contains(sublabel)
            {
                findings.push((
                    "artifact/semantic-text",
                    format!("{path}/sublabel"),
                    "Type sublabel is missing from its actual details".into(),
                    json!({ "id": id }),
                ));
            }
        }
        let compartments: Vec<String> = node
            .select(&selector("g[data-class-compartment]"))
            .filter_map(|group| {
                group
                    .value()
                    .attr("data-class-compartment")
                    .map(str::to_owned)
            })
            .collect();
        let authored: Vec<String> = ["attributes", "methods"]
            .into_iter()
            .filter(|role| item[*role].as_array().is_some_and(|list| !list.is_empty()))
            .map(str::to_owned)
            .collect();
        if compartments != authored {
            findings.push((
                "artifact/semantic-count",
                format!("{path}/members"),
                "Rendered compartments differ from the authored members".into(),
                json!({"id":id,"authored":authored,"actual":compartments}),
            ));
        }
        for (role, method) in [("attributes", false), ("methods", true)] {
            for (member_index, member) in item[role].as_array().unwrap_or(&empty).iter().enumerate()
            {
                let prefix = format!("/types/{index}/{role}/{member_index}/");
                let visible: String = node
                    .select(&selector(&format!(
                        "g[data-class-compartment='{role}'] text"
                    )))
                    .filter(|text| {
                        text.value()
                            .attr("data-source-path")
                            .is_some_and(|source| source.starts_with(&prefix))
                    })
                    .map(|text| text.text().collect::<String>())
                    .collect();
                let expected = members::signature(member, method);
                if significant(&visible) != significant(&expected) {
                    findings.push((
                        "artifact/semantic-text",
                        format!("{path}/{role}/{member_index}"),
                        "Member signature differs from its actual SVG lines".into(),
                        json!({"id":id,"expected":expected,"actual":visible}),
                    ));
                }
            }
        }
    }
    let edges: Vec<_> = svg.select(&selector("path[data-edge-from]")).collect();
    for (index, relationship) in relationships.iter().enumerate() {
        let key = index.to_string();
        let Some(edge) = edges
            .iter()
            .find(|edge| edge.value().attr("data-edge-key") == Some(key.as_str()))
        else {
            continue;
        };
        let kind = relationship["kind"].as_str().unwrap_or("");
        let (start, end, dashed) = notation(kind);
        // A hierarchy bus draws its triangle as a reversed start marker.
        let expected = if edge.value().attr("data-class-bus").is_some() {
            (
                end.map(|end| format!("url(#cl-{}-start)", marker_name(end))),
                None,
            )
        } else {
            (
                start.map(|start| format!("url(#cl-{})", marker_name(start))),
                end.map(|end| format!("url(#cl-{})", marker_name(end))),
            )
        };
        let actual = (
            edge.value().attr("marker-start").map(str::to_owned),
            edge.value().attr("marker-end").map(str::to_owned),
        );
        let classes: Vec<_> = edge
            .value()
            .attr("class")
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if actual != expected
            || edge.value().attr("data-class-relationship") != Some(kind)
            || classes.contains(&"cl-dashed") != dashed
        {
            findings.push((
                "artifact/semantic-relation",
                format!("/metadata/facts/relationships/{index}"),
                "Relationship notation differs from the actual SVG line".into(),
                json!({"kind":kind,"expected":[expected.0, expected.1],"actual":[actual.0, actual.1],"class":classes}),
            ));
        }
    }
    findings
}
