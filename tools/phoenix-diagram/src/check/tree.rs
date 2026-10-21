use super::{selector, visible_text, ContentFinding};
use scraper::ElementRef;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

fn finding(code: &'static str, path: &str, message: &str, evidence: Value) -> ContentFinding {
    (code, path.into(), message.into(), evidence)
}

fn text_of(element: ElementRef<'_>) -> String {
    visible_text(&element.text().collect::<String>())
}

// The hierarchy is the authored parent links; the static artifact draws all
// of it, and the Reader's collapse attributes must restate the same chain.
pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings = Vec::new();
    let nodes = metadata
        .pointer("/facts/nodes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let parent: HashMap<&str, &str> = nodes
        .iter()
        .filter_map(|node| Some((node["id"].as_str()?, node["parent"].as_str()?)))
        .collect();
    let ancestors = |id: &str| {
        let mut chain = Vec::new();
        let mut current = parent.get(id).copied();
        while let Some(next) = current {
            if chain.contains(&next) {
                break;
            }
            chain.push(next);
            current = parent.get(next).copied();
        }
        chain
    };
    let mut descendants: HashMap<&str, usize> = HashMap::new();
    for node in nodes.iter().filter_map(|node| node["id"].as_str()) {
        for ancestor in ancestors(node) {
            *descendants.entry(ancestor).or_default() += 1;
        }
    }

    let expected: Vec<Value> = nodes
        .iter()
        .filter(|node| node.get("parent").is_some())
        .map(|node| json!({"from":node["parent"],"to":node["id"]}))
        .collect();
    let mut declared: Vec<Value> = metadata
        .pointer("/graph/relations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let key = |value: &Value| value.to_string();
    declared.sort_by_key(key);
    let mut derived = expected.clone();
    derived.sort_by_key(key);
    if declared != derived {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/graph/relations",
            "Tree relationships differ from the authored parent links",
            json!({"expected":expected}),
        ));
    }
    let mut drawn: BTreeMap<(String, String), usize> = BTreeMap::new();
    for edge in svg.select(&selector("[data-edge-from]")) {
        let (from, to) = (
            edge.value().attr("data-edge-from").unwrap_or_default(),
            edge.value().attr("data-edge-to").unwrap_or_default(),
        );
        *drawn.entry((from.into(), to.into())).or_default() += 1;
        if edge.value().attr("data-tree-ancestors").unwrap_or_default() != ancestors(to).join(" ") {
            findings.push(finding(
                "artifact/semantic-relation",
                "/svg/relationships",
                "A link's Reader ancestry differs from the authored hierarchy",
                json!({"from":from,"to":to}),
            ));
        }
    }
    let links: BTreeMap<(String, String), usize> = parent
        .iter()
        .map(|(child, parent)| (((*parent).to_owned(), (*child).to_owned()), 1))
        .collect();
    if drawn != links {
        findings.push(finding(
            "artifact/semantic-count",
            "/svg/relationships",
            "Drawn tree links differ from one link per authored parent",
            json!({"links":links.len(),"drawn":drawn.values().sum::<usize>()}),
        ));
    }

    for (index, node) in nodes.iter().enumerate() {
        let id = node["id"].as_str().unwrap_or_default();
        let path = format!("/metadata/facts/nodes/{index}");
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        let texts: Vec<_> = group.select(&selector("text")).collect();
        let label = texts
            .iter()
            .find(|text| text.value().attr("data-node-label").is_some());
        if label.is_none_or(|label| {
            text_of(*label) != visible_text(node["label"].as_str().unwrap_or_default())
        }) {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "A tree node label is missing or changed in its label text",
                json!({"id":id}),
            ));
        }
        if let Some(sublabel) = node["sublabel"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
        {
            if !texts.iter().any(|text| {
                text.value().attr("data-node-label").is_none()
                    && text_of(*text) == visible_text(sublabel)
            }) {
                findings.push(finding(
                    "artifact/semantic-text",
                    &path,
                    "A tree node sublabel is missing or changed",
                    json!({"id":id}),
                ));
            }
        }
        let chain = ancestors(id);
        if group.value().attr("data-tree-depth") != Some(chain.len().to_string().as_str())
            || group
                .value()
                .attr("data-tree-ancestors")
                .unwrap_or_default()
                != chain.join(" ")
        {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "A node's Reader depth or ancestry differs from the authored hierarchy",
                json!({"id":id,"ancestors":chain}),
            ));
        }
        let toggles: Vec<_> = svg
            .select(&selector("g[data-tree-toggle]"))
            .filter(|toggle| toggle.value().attr("data-tree-toggle") == Some(id))
            .collect();
        let count = descendants.get(id).copied().unwrap_or(0);
        let expected_toggles = usize::from(count > 0);
        if toggles.len() != expected_toggles
            || toggles.iter().any(|toggle| {
                toggle.value().attr("data-tree-count") != Some(count.to_string().as_str())
            })
        {
            findings.push(finding(
                "artifact/semantic-count",
                &path,
                "A branch toggle is missing, extra, or counts a different subtree",
                json!({"id":id,"descendants":count,"toggles":toggles.len()}),
            ));
        }
        for toggle in toggles {
            let collapsed = toggle
                .value()
                .attr("data-tree-initially-collapsed")
                .is_some();
            if collapsed != (node["collapsed"] == true)
                || toggle
                    .value()
                    .attr("data-tree-ancestors")
                    .unwrap_or_default()
                    != chain.join(" ")
            {
                findings.push(finding(
                    "artifact/semantic-relation",
                    &path,
                    "A branch toggle's initial state or ancestry differs from the authored node",
                    json!({"id":id,"collapsed":node["collapsed"]}),
                ));
            }
        }
    }
    let toggles = svg.select(&selector("g[data-tree-toggle]")).count();
    if toggles != descendants.len() {
        findings.push(finding(
            "artifact/semantic-count",
            "/svg/tree-toggles",
            "Branch toggles do not correspond one to one with authored branches",
            json!({"branches":descendants.len(),"toggles":toggles}),
        ));
    }
    // Collapse is a Reader state; a delivered artifact always shows every node.
    let hidden = svg
        .select(&selector("[data-tree-hidden], [data-tree-collapsed]"))
        .count();
    if hidden > 0 {
        findings.push(finding(
            "artifact/semantic-count",
            "/svg/tree",
            "The static tree hides part of the authored hierarchy",
            json!({"hidden":hidden}),
        ));
    }
    let direction = metadata
        .pointer("/facts/direction")
        .and_then(Value::as_str)
        .unwrap_or("down");
    if svg.value().attr("data-tree-ui").is_none()
        || svg.value().attr("data-tree-direction") != Some(direction)
    {
        findings.push(finding(
            "artifact/semantic-relation",
            "/svg/data-tree-direction",
            "The tree Reader contract differs from the authored direction",
            json!({"direction":direction}),
        ));
    }
    findings
}
