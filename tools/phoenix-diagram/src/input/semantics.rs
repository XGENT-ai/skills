// Structural rules derived from Archify (MIT); see NOTICE.md.
// Geometry-dependent checks run after layout; these checks never reorder facts.
use super::DiagramType;
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

pub(super) fn validate(document: &Value, kind: DiagramType) -> Result<(), Diagnostics> {
    let (node_collection, relation_collection) = match kind {
        DiagramType::Architecture => ("components", "connections"),
        DiagramType::Workflow => ("nodes", "edges"),
        DiagramType::Sequence => ("participants", "messages"),
        DiagramType::Dataflow => ("nodes", "flows"),
        DiagramType::Lifecycle => ("states", "transitions"),
        DiagramType::Erd => ("entities", "relationships"),
        DiagramType::Class => ("types", "relationships"),
        DiagramType::Tree => ("nodes", "relationships"),
        DiagramType::Timeline => ("events", "relationships"),
        DiagramType::Waterfall => ("spans", "relationships"),
        DiagramType::Chart => ("rows", "relationships"),
        DiagramType::Annotation => ("sentences", "relationships"),
        // A statechart has no authored entity collections; its model rules
        // are checked by the statechart module.
        DiagramType::Statechart => {
            let errors = crate::statechart::check_static(document);
            return if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            };
        }
    };
    let nodes = array(document, node_collection);
    let relations = array(document, relation_collection);
    let known_nodes = ids(nodes);
    let lanes = ids(array(document, "lanes"));
    let mut checks = Checks {
        kind,
        errors: Vec::new(),
    };
    // Tree, timeline and waterfall keep the frozen renderers' own identity codes.
    let typed_identity = matches!(
        kind,
        DiagramType::Tree | DiagramType::Timeline | DiagramType::Waterfall
    );
    let mut identities = HashMap::new();
    if !typed_identity {
        checks.unique(nodes, &format!("/{node_collection}"), &mut identities);
        checks.unique(
            relations,
            &format!("/{relation_collection}"),
            &mut identities,
        );
    }
    if kind.is_original_five() {
        for collection in ["lanes", "phases", "groups"] {
            checks.unique(
                array(document, collection),
                &format!("/{collection}"),
                &mut HashMap::new(),
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
        if kind == DiagramType::Workflow {
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
    if kind == DiagramType::Workflow || kind == DiagramType::Lifecycle {
        main_path(document, nodes, relations, &known_nodes, &mut checks);
    }
    match kind {
        DiagramType::Erd => erd_checks(document, nodes, relations, &mut checks),
        DiagramType::Class => class_checks(nodes, relations, &mut checks),
        DiagramType::Tree => parent_checks(nodes, "nodes", "tree", true, &mut checks),
        DiagramType::Timeline => timeline_checks(document, nodes, &mut checks),
        DiagramType::Waterfall => waterfall_checks(nodes, &mut checks),
        DiagramType::Chart => chart_checks(document, nodes, &mut checks),
        DiagramType::Annotation => annotation_checks(document, &mut checks),
        _ => {}
    }
    if let Err(change_errors) = crate::changes::resolve(document, kind) {
        checks.errors.extend(change_errors);
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
    // The fewest column changes that keep every main-path step at or after
    // its predecessor: the longest non-decreasing run of columns stays, and
    // each other step takes the column of the kept step before it (or after
    // it, at the start), so one round fixes all backward steps together.
    let steps = path
        .iter()
        .filter_map(|id| {
            let index = nodes.iter().position(|node| node["id"] == *id)?;
            Some((index, nodes[index]["col"].as_f64()?))
        })
        .collect::<Vec<_>>();
    let mut length = vec![1; steps.len()];
    let mut previous = vec![None; steps.len()];
    for i in 0..steps.len() {
        for j in 0..i {
            if steps[j].1 <= steps[i].1 && length[j] + 1 > length[i] {
                length[i] = length[j] + 1;
                previous[i] = Some(j);
            }
        }
    }
    let mut kept = vec![false; steps.len()];
    let mut cursor = (0..steps.len()).rev().max_by_key(|i| length[*i]);
    while let Some(i) = cursor {
        kept[i] = true;
        cursor = previous[i];
    }
    let mut forward = Vec::new();
    let mut running = None;
    for (i, (index, col)) in steps.iter().enumerate() {
        if kept[i] {
            running = Some(*col);
            continue;
        }
        let target = running.unwrap_or_else(|| {
            (i..steps.len())
                .find(|j| kept[*j])
                .map_or(*col, |j| steps[j].1)
        });
        if target != *col {
            forward.push(format!("/nodes/{index}/col to {target}"));
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
            if let (Some(from), Some(to)) = (from, to) {
                if to < from {
                    checks.error(
                        "input/main-path",
                        &format!("/mainPath/{}", index + 1),
                        "Main path moves backward through columns",
                    );
                    let error = checks.errors.last_mut().expect("just inserted diagnostic");
                    error.evidence = json!({"from":pair[0],"fromCol":from,"to":pair[1],"toCol":to});
                    error.supported_fixes = vec![
                        format!("Set {} so every mainPath step stays in or after its predecessor's column", forward.join(", ")),
                        format!("If {} is a return or retry, end mainPath at /mainPath/{index} and keep that step as its relation", pair[1].as_str().unwrap_or_default()),
                    ];
                }
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

fn self_relationships(relations: &[Value], checks: &mut Checks) {
    for (index, relation) in relations.iter().enumerate() {
        if relation["from"].is_string() && relation["from"] == relation["to"] {
            let kind = checks.kind.as_str();
            checks.error(
                &format!("{kind}/self-relationship"),
                &format!("/relationships/{index}"),
                "A relationship connects a node to itself; describe the self-reference in a card",
            );
        }
    }
}

fn duplicate_members(items: &[Value], collection: &str, field: &str, checks: &mut Checks) {
    for (index, item) in items.iter().enumerate() {
        let mut seen = HashSet::new();
        for (member, value) in array(item, field).iter().enumerate() {
            if let Some(name) = value["name"].as_str() {
                if !seen.insert(name) {
                    let kind = checks.kind.as_str();
                    checks.error(
                        &format!("{kind}/duplicate-attribute"),
                        &format!("/{collection}/{index}/{field}/{member}/name"),
                        format!("Attribute {name} is declared twice"),
                    );
                }
            }
        }
    }
}

// Grid cells are facts the author places; geometry-dependent checks run after layout.
fn erd_checks(document: &Value, entities: &[Value], relations: &[Value], checks: &mut Checks) {
    duplicate_members(entities, "entities", "attributes", checks);
    let by_id: HashMap<&str, &Value> = entities
        .iter()
        .filter_map(|entity| Some((entity["id"].as_str()?, entity)))
        .collect();
    for (index, entity) in entities.iter().enumerate() {
        for (member, attribute) in array(entity, "attributes").iter().enumerate() {
            let Some((table, column)) = attribute["references"]
                .as_str()
                .and_then(|value| value.split_once('.'))
            else {
                continue;
            };
            let path = format!("/entities/{index}/attributes/{member}/references");
            match by_id.get(table) {
                None => checks.error(
                    "input/unknown-reference",
                    &path,
                    format!("Unknown referenced entity: {table}"),
                ),
                Some(target)
                    if !array(target, "attributes")
                        .iter()
                        .any(|candidate| candidate["name"] == column) =>
                {
                    checks.error(
                        "input/unknown-reference",
                        &path,
                        format!("Entity {table} has no attribute {column}"),
                    )
                }
                Some(_) => {}
            }
        }
    }
    self_relationships(relations, checks);
    let grid = document.get("layout").is_some()
        || entities
            .iter()
            .any(|entity| entity["row"].is_number() && entity["col"].is_number());
    let mut cells = HashMap::new();
    for (index, entity) in entities.iter().enumerate() {
        if entity["pos"].is_array() {
            continue;
        }
        let path = format!("/entities/{index}");
        let (Some(row), Some(col)) = (entity["row"].as_u64(), entity["col"].as_u64()) else {
            checks.error(
                "erd/grid-placement",
                &path,
                if grid {
                    "Entity needs pos [x,y] or grid row/col"
                } else {
                    "Entity needs pos [x,y] or a grid layout with row/col"
                },
            );
            continue;
        };
        if let Some(previous) = cells.insert((row, col), index) {
            checks.error(
                "erd/grid-cell-conflict",
                &path,
                format!("Entities {previous} and {index} share grid cell row {row} col {col}"),
            );
        }
    }
}

fn class_checks(types: &[Value], relations: &[Value], checks: &mut Checks) {
    duplicate_members(types, "types", "attributes", checks);
    let mut cells = HashMap::new();
    for (index, item) in types.iter().enumerate() {
        if item["pos"].is_array() {
            continue;
        }
        let path = format!("/types/{index}");
        let (Some(row), Some(col)) = (item["row"].as_u64(), item["col"].as_f64()) else {
            checks.error(
                "class/grid-placement",
                &path,
                "Type needs grid row/col (col may be a half step) or an absolute pos [x,y]",
            );
            continue;
        };
        // A half column straddles the two columns beside it.
        let (left, right) = (col.floor() as u64, col.ceil() as u64);
        for cell in if left == right {
            vec![left]
        } else {
            vec![left, right]
        } {
            if let Some(previous) = cells.insert((row, cell), index) {
                checks.error(
                    "class/grid-cell-conflict",
                    &path,
                    format!("Types {previous} and {index} share grid cell row {row} col {cell}"),
                );
            }
        }
    }
    self_relationships(relations, checks);
    let kinds: HashMap<&str, &str> = types
        .iter()
        .filter_map(|item| {
            Some((
                item["id"].as_str()?,
                item["kind"].as_str().unwrap_or("class"),
            ))
        })
        .collect();
    let mut parents: Vec<(&str, &str)> = Vec::new();
    for (index, relation) in relations.iter().enumerate() {
        let (Some(from), Some(to)) = (relation["from"].as_str(), relation["to"].as_str()) else {
            continue;
        };
        let (Some(&from_kind), Some(&to_kind)) = (kinds.get(from), kinds.get(to)) else {
            continue;
        };
        if from == to {
            continue;
        }
        let path = format!("/relationships/{index}");
        match relation["kind"].as_str() {
            Some("realization") if to_kind != "interface" || from_kind == "interface" => {
                checks.error(
                    "class/realization-target",
                    &path,
                    format!("Realization runs from a non-interface type to an interface, not {from_kind} -> {to_kind}"),
                )
            }
            Some("inheritance") => {
                if (from_kind == "interface") != (to_kind == "interface") {
                    checks.error(
                        "class/inheritance-kind",
                        &path,
                        format!("Inheritance crosses an interface boundary ({from_kind} -> {to_kind}); use realization"),
                    );
                }
                parents.push((from, to));
            }
            _ => {}
        }
    }
    // Depth-first walk in first-declaration order, as the frozen renderer reports one cycle.
    let mut order: Vec<&str> = Vec::new();
    for (from, _) in &parents {
        if !order.contains(from) {
            order.push(from);
        }
    }
    fn visit<'a>(
        id: &'a str,
        parents: &[(&'a str, &'a str)],
        state: &mut HashMap<&'a str, bool>,
        stack: &mut Vec<&'a str>,
    ) -> Option<Vec<&'a str>> {
        match state.get(id) {
            Some(true) => return None,
            Some(false) => {
                let start = stack.iter().position(|item| *item == id)?;
                let mut cycle = stack[start..].to_vec();
                cycle.push(id);
                return Some(cycle);
            }
            None => {}
        }
        state.insert(id, false);
        stack.push(id);
        for (_, parent) in parents.iter().filter(|(from, _)| *from == id) {
            if let Some(cycle) = visit(parent, parents, state, stack) {
                return Some(cycle);
            }
        }
        stack.pop();
        state.insert(id, true);
        None
    }
    let mut state = HashMap::new();
    for id in order {
        if let Some(cycle) = visit(id, &parents, &mut state, &mut Vec::new()) {
            checks.error(
                "class/inheritance-cycle",
                "/relationships",
                format!("Inheritance forms a cycle: {}", cycle.join(" -> ")),
            );
            checks
                .errors
                .last_mut()
                .expect("just inserted diagnostic")
                .evidence = json!({"cycle":cycle});
            break;
        }
    }
}

// One declared parent per item; nothing is inferred, so a missing parent, a
// cycle, or (for trees) a second root is refused.
fn parent_checks(
    items: &[Value],
    collection: &str,
    prefix: &str,
    single_root: bool,
    checks: &mut Checks,
) {
    let mut by_id: HashMap<&str, &Value> = HashMap::new();
    for (index, item) in items.iter().enumerate() {
        let Some(id) = item["id"].as_str() else {
            continue;
        };
        if by_id.insert(id, item).is_some() {
            checks.error(
                &format!("{prefix}/duplicate-id"),
                &format!("/{collection}/{index}/id"),
                format!("Identity {id} is declared twice"),
            );
            checks
                .errors
                .last_mut()
                .expect("just inserted diagnostic")
                .subject
                .identity = Some(id.into());
        }
    }
    for (index, item) in items.iter().enumerate() {
        if let Some(parent) = item["parent"].as_str() {
            if !by_id.contains_key(parent) {
                checks.error(
                    &format!("{prefix}/missing-parent"),
                    &format!("/{collection}/{index}/parent"),
                    format!("Parent {parent} is not declared"),
                );
            }
        }
    }
    let roots: Vec<&str> = items
        .iter()
        .filter(|item| item.get("parent").is_none())
        .filter_map(|item| item["id"].as_str())
        .collect();
    if single_root && roots.len() != 1 {
        checks.error(
            &format!("{prefix}/root-count"),
            &format!("/{collection}"),
            format!(
                "A tree has one root, but {} nodes have no parent",
                roots.len()
            ),
        );
        checks
            .errors
            .last_mut()
            .expect("just inserted diagnostic")
            .evidence = json!({"roots":roots});
    }
    let mut reported = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let mut chain: Vec<&str> = Vec::new();
        let mut current = Some(item);
        while let Some(node) = current {
            let (Some(id), Some(parent)) = (node["id"].as_str(), node["parent"].as_str()) else {
                break;
            };
            if chain.contains(&id) {
                break;
            }
            chain.push(id);
            current = by_id.get(parent).copied();
        }
        let Some(id) = current.and_then(|node| node["id"].as_str()) else {
            continue;
        };
        if let Some(start) = chain.iter().position(|item| *item == id) {
            let cycle = chain[start..].to_vec();
            let mut key = cycle.clone();
            key.sort_unstable();
            if reported.insert(key) {
                checks.error(
                    &format!("{prefix}/cycle"),
                    &format!("/{collection}/{index}/parent"),
                    format!("Parent links form a cycle: {}", cycle.join(" -> ")),
                );
                checks
                    .errors
                    .last_mut()
                    .expect("just inserted diagnostic")
                    .evidence = json!({"cycle":cycle});
            }
        }
    }
}

fn timeline_checks(document: &Value, events: &[Value], checks: &mut Checks) {
    use crate::format::time;
    let timezone = document["meta"]["timezone"].as_str().unwrap_or("UTC");
    if time::zone(timezone).is_none() {
        checks.error(
            "timeline/invalid-timezone",
            "/meta/timezone",
            format!(
                "{timezone:?} is neither a recognised time zone nor a UTC offset such as +05:30"
            ),
        );
    }
    let mut lanes = HashSet::new();
    for (index, lane) in array(document, "lanes").iter().enumerate() {
        if let Some(id) = lane["id"].as_str() {
            if !lanes.insert(id) {
                checks.error(
                    "timeline/duplicate-lane-id",
                    &format!("/lanes/{index}/id"),
                    format!("Lane {id} is declared twice"),
                );
            }
        }
    }
    let mut seen = HashSet::new();
    for (index, event) in events.iter().enumerate() {
        let path = format!("/events/{index}");
        if let Some(id) = event["id"].as_str() {
            if !seen.insert(id) {
                checks.error(
                    "timeline/duplicate-id",
                    &format!("{path}/id"),
                    format!("Event {id} is declared twice"),
                );
            }
        }
        let at = event["at"].as_str().unwrap_or_default();
        let valid = time::valid_calendar_date(at) && time::parse_timestamp(at).is_some();
        if !valid {
            checks.error(
                "timeline/invalid-timestamp",
                &format!("{path}/at"),
                format!("{at:?} is not a valid date and time"),
            );
        }
        if !lanes.is_empty()
            && !event["lane"]
                .as_str()
                .is_some_and(|lane| lanes.contains(lane))
        {
            checks.error(
                "timeline/unknown-lane",
                &format!("{path}/lane"),
                "Event lane is not one of the declared lanes",
            );
        }
        // A date-only record is placed at noon of the authored offset; the
        // authored clock must say so rather than claim a recorded time.
        if valid && event["precision"] == "day" {
            let clock = &at[11..at.len() - if at.ends_with('Z') { 1 } else { 6 }];
            let noon = clock == "12:00"
                || clock.strip_prefix("12:00:00").is_some_and(|rest| {
                    rest.is_empty() || rest[1..].bytes().all(|byte| byte == b'0')
                });
            if !noon {
                checks.error(
                    "timeline/day-precision-time",
                    &format!("{path}/precision"),
                    "A day-precision event must be authored at 12:00 of its offset",
                );
            }
        }
    }
}

// Bars come only from recorded timing: end or duration (agreeing when both),
// and only an incomplete span may state neither.
fn waterfall_checks(spans: &[Value], checks: &mut Checks) {
    for (index, span) in spans.iter().enumerate() {
        let path = format!("/spans/{index}");
        let start = span["start"].as_f64().unwrap_or_default();
        let (end, duration) = (span["end"].as_f64(), span["duration"].as_f64());
        let mut timing = |message: String| {
            checks.error("waterfall/invalid-timing", &path, message);
        };
        if duration.is_some_and(|duration| !(start + duration).is_finite()) {
            timing("The derived end is outside the finite numeric range".into());
        }
        if let Some(end) = end.filter(|end| *end < start) {
            timing(format!("Span ends ({end}) before it starts ({start})"));
        }
        if let (Some(end), Some(duration)) = (end, duration) {
            if (end - start - duration).abs() > 1e-9 {
                timing(format!(
                    "End {end} and duration {duration} disagree with start {start}"
                ));
            }
        }
        if end.is_none() && duration.is_none() && span["status"] != "incomplete" {
            timing("A span without a recorded end or duration must be marked incomplete".into());
        }
    }
    parent_checks(spans, "spans", "waterfall", false, checks);
}

// The form decides which value fields a row may carry, and every row states
// exactly as many numbers as there are series.
fn chart_checks(document: &Value, rows: &[Value], checks: &mut Checks) {
    let form = document["form"].as_str().unwrap_or_default();
    let series = array(document, "series");
    let mut seen_series = HashMap::new();
    checks.unique(series, "/series", &mut seen_series);
    let expected = series.len().max(1);
    if form != "bar" && !document["orientation"].is_null() {
        checks.error(
            "chart/form-fields",
            "/orientation",
            format!("Only bar takes orientation, but the form is {form}"),
        );
    }
    if form == "limits" {
        if !series.is_empty() {
            checks.error(
                "chart/form-fields",
                "/series",
                format!(
                    "A limits row carries no series, but {} series are declared",
                    series.len()
                ),
            );
        }
        for (index, row) in rows.iter().enumerate() {
            let path = format!("/rows/{index}");
            if row["limit"].as_f64().is_none() {
                checks.error(
                    "chart/form-fields",
                    &path,
                    "A limits row states limit (the upper bound); none was found",
                );
            }
            if !row["values"].is_null() {
                checks.error(
                    "chart/form-fields",
                    &format!("{path}/values"),
                    "A limits row states value and limit, not a values array",
                );
            }
        }
    } else {
        for (index, row) in rows.iter().enumerate() {
            let path = format!("/rows/{index}");
            let Some(values) = row["values"].as_array() else {
                checks.error(
                    "chart/form-fields",
                    &path,
                    format!("A {form} row states values (one number per series); none were found"),
                );
                continue;
            };
            if values.len() != expected {
                checks.error(
                    "chart/series-count",
                    &format!("{path}/values"),
                    format!(
                        "Row {} states {} values but {} series are declared",
                        index + 1,
                        values.len(),
                        expected
                    ),
                );
            }
            for field in ["value", "limit", "unit", "note"] {
                if !row[field].is_null() {
                    checks.error(
                        "chart/form-fields",
                        &format!("{path}/{field}"),
                        format!("A {form} row carries no {field}"),
                    );
                }
            }
        }
    }
    if form == "pie" {
        if series.len() > 1 {
            checks.error(
                "chart/pie-value",
                "/series",
                format!(
                    "A pie compares one value per row, but {} series are declared",
                    series.len()
                ),
            );
        }
        let mut all_zero = true;
        for (index, row) in rows.iter().enumerate() {
            let value = row["values"][0].as_f64().unwrap_or_default();
            if value < 0.0 {
                checks.error(
                    "chart/pie-value",
                    &format!("/rows/{index}/values/0"),
                    format!(
                        "A pie sector cannot be negative, but row {} states {value}",
                        index + 1
                    ),
                );
            }
            if value != 0.0 {
                all_zero = false;
            }
        }
        if all_zero {
            checks.error(
                "chart/pie-value",
                "/rows",
                "Every pie value is 0; a pie needs at least one non-zero sector",
            );
        }
    }
}

/// One resolved annotation: the character range it marks in its sentence and
/// the number it takes in the figure.
pub(crate) struct AnnotationSpan {
    pub sentence: usize,
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub number: usize,
    pub issue: bool,
}

// Left-to-right, non-overlapping occurrences of `span` in `text`.
fn occurrences(text: &[char], span: &str) -> Vec<(usize, usize)> {
    let needle: Vec<char> = span.chars().collect();
    let mut found = Vec::new();
    if needle.is_empty() {
        return found;
    }
    let mut index = 0;
    while index + needle.len() <= text.len() {
        if text[index..index + needle.len()] == needle[..] {
            found.push((index, index + needle.len()));
            index += needle.len();
        } else {
            index += 1;
        }
    }
    found
}

fn span_error(code: &str, path: &str, message: String, fix: &str, evidence: Value) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(Some("annotation"), code, path, message);
    diagnostic.supported_fixes = vec![fix.into()];
    diagnostic.evidence = evidence;
    diagnostic
}

/// Locates every authored span in its sentence and numbers the annotations in
/// reading order. Both the semantic check and the layout read this, so the
/// drawn ranges never disagree with the validated ones.
pub(crate) fn annotation_spans(document: &Value) -> Result<Vec<AnnotationSpan>, Diagnostics> {
    let mut spans = Vec::new();
    let mut errors = Diagnostics::new();
    let mut number = 0;
    for (sentence_index, sentence) in array(document, "sentences").iter().enumerate() {
        let text: Vec<char> = sentence["text"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .collect();
        let annotations = array(sentence, "annotations");
        let mut located: Vec<(usize, usize, usize, bool)> = Vec::new();
        for (index, annotation) in annotations.iter().enumerate() {
            let span = annotation["span"].as_str().unwrap_or_default();
            let occurrence = annotation["occurrence"].as_u64().unwrap_or(1).max(1) as usize;
            let path = format!("/sentences/{sentence_index}/annotations/{index}");
            let found = occurrences(&text, span);
            if found.is_empty() {
                errors.push(span_error(
                    "annotation/span-missing",
                    &format!("{path}/span"),
                    format!("Span {span:?} does not occur in the sentence text"),
                    "Pick a span that occurs in the sentence text",
                    json!({"span": span}),
                ));
                continue;
            }
            if occurrence > found.len() {
                errors.push(span_error(
                    "annotation/span-missing",
                    &format!("{path}/occurrence"),
                    format!(
                        "Occurrence {occurrence} exceeds the {} matches of span {span:?}",
                        found.len()
                    ),
                    "Use an occurrence between 1 and the number of matches",
                    json!({"occurrences": found.len(), "span": span}),
                ));
                continue;
            }
            let (start, end) = found[occurrence - 1];
            located.push((start, end, index, annotation["issue"] == true));
        }
        // Two spans cannot mark the same character: report the later authored
        // one, keeping the earlier span's range as the evidence.
        for later in 0..located.len() {
            let (start, end, index, _) = located[later];
            for (other_start, other_end, other_index, _) in located[..later].iter() {
                if start < *other_end && *other_start < end {
                    errors.push(span_error(
                        "annotation/span-overlap",
                        &format!("/sentences/{sentence_index}/annotations/{index}"),
                        format!(
                            "Span at characters {start}..{end} overlaps annotation {other_index} at {other_start}..{other_end}"
                        ),
                        "Shorten one span so each character is marked once",
                        json!({"other": other_index, "start": start, "end": end,
                               "otherStart": other_start, "otherEnd": other_end}),
                    ));
                    break;
                }
            }
        }
        let mut ordered = located;
        ordered.sort_by_key(|(start, _, index, _)| (*start, *index));
        for (start, end, index, issue) in ordered {
            number += 1;
            spans.push(AnnotationSpan {
                sentence: sentence_index,
                index,
                start,
                end,
                number,
                issue,
            });
        }
    }
    if errors.is_empty() {
        Ok(spans)
    } else {
        Err(errors)
    }
}

fn annotation_checks(document: &Value, checks: &mut Checks) {
    // Sentence identities use the shared unique() check; the span rules live in
    // annotation_spans so the layout reads exactly the validated ranges.
    if let Err(errors) = annotation_spans(document) {
        checks.errors.extend(errors);
    }
}
