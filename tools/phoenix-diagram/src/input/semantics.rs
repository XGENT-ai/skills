// Structural rules ported from the frozen Archify 2.17 / 3.1 renderers.
// Geometry-dependent checks run after layout; these checks never reorder facts.
use super::{DiagramType, InputContract};
use crate::diagnostic::{Diagnostic, Diagnostics};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};

fn array<'a>(document: &'a Value, field: &str) -> &'a [Value] {
    document
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn ids(items: &[Value]) -> HashSet<&str> {
    items
        .iter()
        .filter_map(|item| item["id"].as_str())
        .collect()
}

struct Checks {
    kind: DiagramType,
    errors: Diagnostics,
}

impl Checks {
    fn error(&mut self, code: &str, path: &str, message: impl Into<String>) {
        self.errors.push(Diagnostic::error(
            Some(self.kind.as_str()),
            code,
            path,
            message,
        ));
    }

    fn reference(&mut self, value: &Value, path: &str, known: &HashSet<&str>) {
        if let Some(id) = value.as_str() {
            if !known.contains(id) {
                self.error(
                    "input/unknown-reference",
                    path,
                    format!("Unknown reference: {id}"),
                );
                let error = self.errors.last_mut().expect("just inserted diagnostic");
                error.subject.identity = Some(id.into());
                let mut candidates: Vec<_> = known.iter().copied().collect();
                candidates.sort_unstable();
                error.evidence = json!({"knownIdentities":candidates});
                error.supported_fixes = candidates
                    .iter()
                    .take(3)
                    .map(|candidate| format!("Set {path} to existing identity {candidate}"))
                    .collect();
            }
        }
    }

    fn unique<'a>(
        &mut self,
        items: &'a [Value],
        collection: &str,
        seen: &mut HashMap<&'a str, String>,
    ) {
        for (index, item) in items.iter().enumerate() {
            if let Some(id) = item["id"].as_str() {
                let path = format!("{collection}/{index}/id");
                if let Some(previous) = seen.get(id) {
                    self.error(
                        "input/duplicate-id",
                        &path,
                        format!("Identity {id} already occurs at {previous}"),
                    );
                    let error = self.errors.last_mut().expect("just inserted diagnostic");
                    error.subject.identity = Some(id.into());
                    error.evidence = json!({"firstOccurrence":previous});
                } else {
                    seen.insert(id, path);
                }
            }
        }
    }

    fn ordered_range(&mut self, item: &Value, path: &str, from: &str, to: &str, strict: bool) {
        if let (Some(start), Some(end)) = (item[from].as_f64(), item[to].as_f64()) {
            if end < start || (strict && end == start) {
                self.error(
                    "input/range",
                    &format!("{path}/{to}"),
                    format!(
                        "{to} must be {} {from}",
                        if strict { "greater than" } else { "at least" }
                    ),
                );
            }
        }
    }
}

pub(super) fn validate(
    document: &Value,
    kind: DiagramType,
    contract: InputContract,
) -> Result<(), Diagnostics> {
    let (node_collection, relation_collection) = match kind {
        DiagramType::Architecture => ("components", "connections"),
        DiagramType::Workflow => ("nodes", "edges"),
        DiagramType::Sequence => ("participants", "messages"),
        DiagramType::Dataflow => ("nodes", "flows"),
        DiagramType::Lifecycle => ("states", "transitions"),
    };
    let nodes = array(document, node_collection);
    let relations = array(document, relation_collection);
    let known_nodes = ids(nodes);
    let lanes = ids(array(document, "lanes"));
    let mut checks = Checks {
        kind,
        errors: Vec::new(),
    };
    let mut identities = HashMap::new();
    checks.unique(nodes, &format!("/{node_collection}"), &mut identities);
    checks.unique(
        relations,
        &format!("/{relation_collection}"),
        &mut identities,
    );
    for collection in ["lanes", "phases", "groups"] {
        checks.unique(
            array(document, collection),
            &format!("/{collection}"),
            &mut HashMap::new(),
        );
    }
    let views = array(&document["meta"], "views");
    checks.unique(views, "/meta/views", &mut HashMap::new());
    let focus_ids: HashSet<&str> = identities.keys().copied().collect();
    for (index, view) in views.iter().enumerate() {
        for (focus_index, value) in array(view, "focus").iter().enumerate() {
            checks.reference(
                value,
                &format!("/meta/views/{index}/focus/{focus_index}"),
                &focus_ids,
            );
        }
    }
    for (index, relation) in relations.iter().enumerate() {
        for endpoint in ["from", "to"] {
            checks.reference(
                &relation[endpoint],
                &format!("/{relation_collection}/{index}/{endpoint}"),
                &known_nodes,
            );
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        let path = format!("/{node_collection}/{index}");
        if kind == DiagramType::Workflow
            || (kind == DiagramType::Lifecycle && contract == InputContract::Legacy217)
        {
            checks.reference(&node["lane"], &format!("{path}/lane"), &lanes);
        }
        if kind == DiagramType::Dataflow
            && node["stage"]
                .as_f64()
                .is_some_and(|stage| stage >= array(document, "stages").len() as f64)
        {
            checks.error(
                "input/unknown-reference",
                &format!("{path}/stage"),
                "Stage index is outside the authored stages array",
            );
        }
        for (source_index, source) in array(node, "sources").iter().enumerate() {
            checks.ordered_range(
                source,
                &format!("{path}/sources/{source_index}"),
                "line",
                "end_line",
                false,
            );
        }
    }
    if kind == DiagramType::Architecture {
        for (index, boundary) in array(document, "boundaries").iter().enumerate() {
            for (wrap_index, value) in array(boundary, "wraps").iter().enumerate() {
                checks.reference(
                    value,
                    &format!("/boundaries/{index}/wraps/{wrap_index}"),
                    &known_nodes,
                );
            }
        }
    }
    if kind == DiagramType::Sequence {
        for collection in ["activations", "segments"] {
            for (index, item) in array(document, collection).iter().enumerate() {
                let path = format!("/{collection}/{index}");
                checks.ordered_range(item, &path, "from", "to", true);
                if collection == "activations" {
                    checks.reference(
                        &item["participant"],
                        &format!("{path}/participant"),
                        &known_nodes,
                    );
                }
            }
        }
    }
    if kind == DiagramType::Workflow {
        for collection in ["phases", "groups"] {
            for (index, item) in array(document, collection).iter().enumerate() {
                checks.ordered_range(
                    item,
                    &format!("/{collection}/{index}"),
                    "fromCol",
                    "toCol",
                    false,
                );
                if collection == "groups" {
                    checks.reference(&item["lane"], &format!("/groups/{index}/lane"), &lanes);
                }
            }
        }
        workflow_checks(document, nodes, relations, &known_nodes, &mut checks);
    }
    if kind == DiagramType::Lifecycle
        && contract == InputContract::Legacy217
        && !lanes.contains("main")
    {
        checks.error(
            "input/unknown-reference",
            "/lanes",
            "Legacy lifecycle requires the main lane",
        );
    }
    if kind == DiagramType::Workflow || kind == DiagramType::Lifecycle {
        main_path(document, nodes, relations, &known_nodes, &mut checks);
    }
    if checks.errors.is_empty() {
        Ok(())
    } else {
        Err(checks.errors)
    }
}

fn main_path(
    document: &Value,
    nodes: &[Value],
    relations: &[Value],
    known: &HashSet<&str>,
    checks: &mut Checks,
) {
    let path = array(document, "mainPath");
    let mut seen = HashSet::new();
    for (index, id) in path.iter().enumerate() {
        checks.reference(id, &format!("/mainPath/{index}"), known);
        if !seen.insert(id.as_str()) {
            checks.error(
                "input/main-path",
                &format!("/mainPath/{index}"),
                "Main path repeats a state; keep loops as relations",
            );
        }
    }
    for (index, pair) in path.windows(2).enumerate() {
        if !relations
            .iter()
            .any(|edge| edge["from"] == pair[0] && edge["to"] == pair[1])
        {
            checks.error(
                "input/main-path",
                &format!("/mainPath/{index}"),
                "Main path step has no directed relation",
            );
        }
        if checks.kind == DiagramType::Workflow {
            let from = nodes
                .iter()
                .find(|node| node["id"] == pair[0])
                .and_then(|node| node["col"].as_f64());
            let to = nodes
                .iter()
                .find(|node| node["id"] == pair[1])
                .and_then(|node| node["col"].as_f64());
            if matches!((from, to), (Some(from), Some(to)) if to < from) {
                checks.error(
                    "input/main-path",
                    &format!("/mainPath/{}", index + 1),
                    "Main path moves backward through columns",
                );
            }
        }
    }
}

fn workflow_checks(
    document: &Value,
    nodes: &[Value],
    relations: &[Value],
    known: &HashSet<&str>,
    checks: &mut Checks,
) {
    let Some(contract) = document.get("semanticChecks") else {
        return;
    };
    for collection in ["allowedRoots", "allowedTerminals"] {
        for (index, id) in array(contract, collection).iter().enumerate() {
            checks.reference(id, &format!("/semanticChecks/{collection}/{index}"), known);
        }
    }
    for collection in ["requiredEdges", "requiredPaths"] {
        for (index, relation) in array(contract, collection).iter().enumerate() {
            for endpoint in ["from", "to"] {
                checks.reference(
                    &relation[endpoint],
                    &format!("/semanticChecks/{collection}/{index}/{endpoint}"),
                    known,
                );
            }
        }
    }
    for (collection, endpoint) in [("allowedRoots", "to"), ("allowedTerminals", "from")] {
        if contract.get(collection).is_none() {
            continue;
        }
        for (index, node) in nodes.iter().enumerate() {
            if !array(contract, collection).contains(&node["id"])
                && !relations.iter().any(|edge| edge[endpoint] == node["id"])
            {
                checks.error(
                    "input/semantic-contract",
                    &format!("/nodes/{index}/id"),
                    format!("Node has no {endpoint} relation and is not declared in {collection}"),
                );
            }
        }
    }
    for collection in ["requiredEdges", "requiredPaths"] {
        for (index, relation) in array(contract, collection).iter().enumerate() {
            let from = &relation["from"];
            let to = &relation["to"];
            let matches = if collection == "requiredEdges" {
                relations
                    .iter()
                    .any(|edge| edge["from"] == *from && edge["to"] == *to)
            } else {
                let mut seen = HashSet::new();
                let mut pending = VecDeque::from([from.as_str().expect("schema-checked endpoint")]);
                while let Some(current) = pending.pop_front() {
                    if !seen.insert(current) {
                        continue;
                    }
                    for edge in relations.iter().filter(|edge| edge["from"] == current) {
                        pending.push_back(edge["to"].as_str().expect("schema-checked endpoint"));
                    }
                }
                seen.contains(to.as_str().expect("schema-checked endpoint"))
            };
            if !matches {
                checks.error(
                    "input/semantic-contract",
                    &format!("/semanticChecks/{collection}/{index}"),
                    "Required directed relationship is absent",
                );
            }
        }
    }
}
