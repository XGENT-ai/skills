use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn coordinate_value(value: f64) -> Value {
    serde_json::from_str(&value.to_string()).expect("finite placement coordinate")
}

// Optional advice only. A proposal is never applied to the authored document;
// its complete native compilation is verified before it becomes a supported fix.
pub(super) fn propose(input: &Value) -> Option<(Value, Vec<Value>)> {
    if number(input, "schema_version", 1.0) != 2.0
        || measure::absolute(input)
        || !array(input, "groups").is_empty()
        || !array(input, "phases").is_empty()
        || array(input, "mainPath").is_empty()
        || array(input, "nodes").len() > 64
    {
        return None;
    }
    let nodes = array(input, "nodes");
    let edges = array(input, "edges");
    let main = array(input, "mainPath")
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect::<Vec<_>>();
    let by_id = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (string(node, "id"), index))
        .collect::<BTreeMap<_, _>>();
    let main_index = main
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect::<BTreeMap<_, _>>();
    let lane_index = array(input, "lanes")
        .iter()
        .enumerate()
        .map(|(index, lane)| (string(lane, "id"), index))
        .collect::<BTreeMap<_, _>>();
    if main_index.len() != main.len() || main.iter().any(|id| !by_id.contains_key(id)) {
        return None;
    }
    let mut reachable = BTreeMap::new();
    for node in nodes {
        let mut found = BTreeSet::new();
        let mut pending = edges
            .iter()
            .filter(|edge| edge["from"] == node["id"])
            .map(|edge| string(edge, "to"))
            .collect::<Vec<_>>();
        while let Some(next) = pending.pop() {
            if !by_id.contains_key(next) || !found.insert(next) {
                continue;
            }
            pending.extend(
                edges
                    .iter()
                    .filter(|edge| string(edge, "from") == next)
                    .map(|edge| string(edge, "to")),
            );
        }
        reachable.insert(string(node, "id"), found);
    }
    let off = nodes
        .iter()
        .filter(|node| !main_index.contains_key(string(node, "id")))
        .map(|node| string(node, "id"))
        .collect::<BTreeSet<_>>();
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for node in nodes.iter().filter(|node| off.contains(string(node, "id"))) {
        let id = string(node, "id");
        if assigned.contains(id) {
            continue;
        }
        let mut component = Vec::new();
        let mut pending = vec![id];
        while let Some(id) = pending.pop() {
            if !assigned.insert(id) {
                continue;
            }
            component.push(id);
            for edge in edges {
                if string(edge, "from") == id && off.contains(string(edge, "to")) {
                    pending.push(string(edge, "to"));
                }
                if string(edge, "to") == id && off.contains(string(edge, "from")) {
                    pending.push(string(edge, "from"));
                }
            }
        }
        let cyclic = component.iter().any(|id| {
            main.iter()
                .any(|main| reachable[id].contains(main) && reachable[main].contains(id))
        });
        components.push((component, cyclic));
    }
    if !components.iter().any(|(_, cyclic)| *cyclic) {
        return None;
    }
    let right = components
        .iter()
        .any(|(ids, cyclic)| !*cyclic && ids.len() > 1);
    let end = if right { 4.0 } else { 5.0 };
    let start = 1.0;
    let mut proposed = input.clone();
    let mut col = start;
    for (index, id) in main.iter().enumerate() {
        let current = by_id[id];
        if index > 0 {
            let previous = by_id[main[index - 1]];
            if lane_index[string(&nodes[current], "lane")]
                <= lane_index[string(&nodes[previous], "lane")]
            {
                col = (col + 1.0_f64).min(end);
            }
        }
        proposed["nodes"][current]["col"] = json!(col as u64);
    }
    for (component, cyclic) in &components {
        for id in component {
            let index = by_id[id];
            let parents = edges
                .iter()
                .filter(|edge| {
                    string(edge, "to") == *id && main_index.contains_key(string(edge, "from"))
                })
                .map(|edge| by_id[string(edge, "from")])
                .collect::<Vec<_>>();
            let column = if *cyclic {
                0.0
            } else if component.len() > 1 {
                5.0
            } else {
                parents
                    .iter()
                    .fold(start, |column, index| {
                        column.max(number(&proposed["nodes"][*index], "col", 0.0) + 1.0)
                    })
                    .min(end)
            };
            proposed["nodes"][index]["col"] = json!(column as u64);
            if *cyclic
                && parents
                    .iter()
                    .any(|parent| nodes[*parent]["lane"] == nodes[index]["lane"])
            {
                let maximum = main
                    .iter()
                    .map(|id| by_id[id])
                    .filter(|main| nodes[*main]["lane"] == nodes[index]["lane"])
                    .fold(f64::NEG_INFINITY, |maximum, main| {
                        maximum.max(
                            number(&proposed["nodes"][main], "yOffset", 0.0)
                                + number(&proposed["nodes"][main], "height", 52.0) / 2.0,
                        )
                    });
                proposed["nodes"][index]["yOffset"] =
                    coordinate_value(maximum + number(&nodes[index], "height", 52.0) / 2.0 + 40.0);
            }
        }
    }
    let mut groups: BTreeMap<(String, u64), Vec<usize>> = BTreeMap::new();
    for (index, node) in array(&proposed, "nodes").iter().enumerate() {
        groups
            .entry((string(node, "lane").into(), number(node, "col", 0.0) as u64))
            .or_default()
            .push(index);
    }
    let order = |index: usize| {
        let id = string(&nodes[index], "id");
        if let Some(index) = main_index.get(id) {
            return *index as f64;
        }
        edges
            .iter()
            .filter(|edge| string(edge, "to") == id)
            .filter_map(|edge| main_index.get(string(edge, "from")))
            .map(|index| *index as f64 + 0.5)
            .reduce(f64::min)
            .unwrap_or(main.len() as f64)
    };
    for group in groups.values_mut() {
        if group.len() < 2 {
            continue;
        }
        group.sort_by(|a, b| {
            order(*a).total_cmp(&order(*b)).then_with(|| {
                let a = string(&nodes[*a], "id");
                let b = string(&nodes[*b], "id");
                a.to_ascii_lowercase()
                    .cmp(&b.to_ascii_lowercase())
                    .then_with(|| b.cmp(a))
            })
        });
        let total = group
            .iter()
            .map(|index| number(&proposed["nodes"][*index], "height", 52.0))
            .sum::<f64>()
            + 40.0 * (group.len() - 1) as f64;
        let mut top = -total / 2.0;
        for index in group {
            let height = number(&proposed["nodes"][*index], "height", 52.0);
            proposed["nodes"][*index]["yOffset"] = coordinate_value(top + height / 2.0);
            top += height + 40.0;
        }
    }
    let mut changes = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        for field in ["col", "yOffset"] {
            let original = node.get(field);
            let updated = proposed["nodes"][index].get(field);
            let same = match (
                original.and_then(Value::as_f64),
                updated.and_then(Value::as_f64),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => original == updated,
            };
            if !same {
                if let Some(value) = updated {
                    changes.push(json!({"op":if original.is_some(){"replace"}else{"add"},"path":format!("/nodes/{index}/{field}"),"value":value}));
                }
            }
        }
    }
    if changes.is_empty() {
        None
    } else {
        Some((proposed, changes))
    }
}
