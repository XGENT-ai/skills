//! Single-input change marking for the structural types (workflow, lifecycle,
//! tree, erd). `resolve` validates authored marks and computes effective
//! per-element marks and counts; `attach` maps them onto the laid-out scene
//! and appends the description band. A `None` resolution leaves the scene
//! untouched, so inputs without change marks keep byte-identical output.

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::i18n;
use crate::input::{DiagramInput, DiagramType};
use crate::layout::tree::wrap_text;
use crate::scene::{ChangeBand, ChangeCounts, ChangeFacts, CompiledDiagram, DiagramFacts, Point, Rect, Text};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

const BAND_SIDE_MARGIN: f64 = 24.0;
const BAND_TITLE_FONT: f64 = 12.5;
const BAND_BODY_FONT: f64 = 11.5;
const BAND_LINE_HEIGHT: f64 = 18.0;
const BAND_TOP_PADDING: f64 = 16.0;
const BAND_BOTTOM_PADDING: f64 = 12.0;
/// The shared reading width the readability checks scale from.
const BAND_REFERENCE_WIDTH: f64 = 930.0;

/// Effective marks per element, keyed by authored source path so `attach`
/// can map them onto laid-out relations without re-deriving.
pub(crate) struct Resolved {
    counts: ChangeCounts,
    nodes: BTreeMap<String, String>,
    relations: BTreeMap<String, String>,
    fields: BTreeMap<String, String>,
}

pub(crate) fn resolve(
    document: &Value,
    kind: DiagramType,
) -> Result<Option<Resolved>, Diagnostics> {
    let (nodes_key, relations_key) = match kind {
        DiagramType::Workflow => ("nodes", Some("edges")),
        DiagramType::Lifecycle => ("states", Some("transitions")),
        DiagramType::Erd => ("entities", Some("relationships")),
        DiagramType::Tree => ("nodes", None),
        _ => return Ok(None),
    };
    let empty: Vec<Value> = Vec::new();
    let nodes = document
        .get(nodes_key)
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let relations = relations_key
        .and_then(|key| document.get(key))
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let tree = kind == DiagramType::Tree;
    // No authored marks anywhere: leave the scene untouched so unmarked
    // inputs keep byte-identical output.
    let has_marks = nodes.iter().any(|item| item.get("change").is_some())
        || relations.iter().any(|item| item.get("change").is_some())
        || (kind == DiagramType::Erd
            && nodes.iter().any(|entity| {
                entity
                    .get("attributes")
                    .and_then(Value::as_array)
                    .is_some_and(|fields| {
                        fields.iter().any(|field| field.get("change").is_some())
                    })
            }))
        || (tree && nodes.iter().any(|item| item.get("parent_before").is_some()));
    if !has_marks {
        return Ok(None);
    }
    let mut errors: Diagnostics = Vec::new();

    let mut node_marks: BTreeMap<String, String> = BTreeMap::new();
    for item in nodes {
        if let Some(mark) = item["change"].as_str() {
            node_marks.insert(node_id(item), mark.to_owned());
        }
    }
    // Authored relation marks (tree connections are derived from nodes below).
    let mut relation_marks: BTreeMap<String, String> = BTreeMap::new();
    let mut authored_marks: BTreeMap<String, String> = BTreeMap::new();
    for (index, item) in relations.iter().enumerate() {
        if let Some(mark) = item["change"].as_str() {
            let path = format!("/{}/{}", relations_key.unwrap_or_default(), index);
            authored_marks.insert(path.clone(), mark.to_owned());
            relation_marks.insert(path, mark.to_owned());
        }
    }
    let mut field_marks: BTreeMap<String, String> = BTreeMap::new();

    if let Some(relations_key) = relations_key {
        // Endpoint rule: a removed endpoint takes the relation out of the
        // after view, an added endpoint keeps it out of the before view, so
        // the relation must carry the same mark or it dangles there.
        for (index, item) in relations.iter().enumerate() {
            let relation_mark = item["change"].as_str();
            for endpoint_key in ["from", "to"] {
                let Some(endpoint) = item[endpoint_key].as_str() else {
                    continue;
                };
                let Some(node_mark) = node_marks.get(endpoint) else {
                    continue;
                };
                // A changed endpoint keeps both views; only added and removed
                // endpoints constrain the relation's views.
                if node_mark == "changed" {
                    continue;
                }
                let (expected, view) = if node_mark == "removed" {
                    ("removed", "after")
                } else {
                    ("added", "before")
                };
                if relation_mark != Some(expected) {
                    let path = format!("/{}/{index}", relations_key);
                    let label = relation_identity(item);
                    errors.push(Diagnostic::error(
                        Some(kind.as_str()),
                        "change/dangling-relation",
                        &path,
                        format!(
                            "Relationship {label} dangles in the {view} view: endpoint {endpoint} is marked {node_mark} but the relationship is not"
                        ),
                    ));
                    let error = errors.last_mut().expect("just inserted diagnostic");
                    error.subject.identity = Some(endpoint.to_owned());
                    error.evidence = json!({"endpoint": endpoint, "endpointMark": node_mark, "view": view});
                    error.supported_fixes =
                        vec![format!("Set {path}/change to {expected}")];
                }
            }
        }
    }

    if kind == DiagramType::Erd {
        // An entity mark applies to every field; a field may not disagree.
        for (entity_index, entity) in nodes.iter().enumerate() {
            let entity_mark = entity["change"].as_str();
            let attributes = entity["attributes"].as_array();
            for (field_index, field) in attributes.map(Vec::as_slice).unwrap_or(&[]).iter().enumerate()
            {
                let field_mark = field["change"].as_str();
                match (entity_mark, field_mark) {
                    (Some(entity_mark), Some(field_mark)) if field_mark != entity_mark => {
                        let path =
                            format!("/entities/{entity_index}/attributes/{field_index}/change");
                        errors.push(Diagnostic::error(
                            Some(kind.as_str()),
                            "change/conflict",
                            &path,
                            format!(
                                "Field {} cannot carry {field_mark} while its entity {} is marked {entity_mark}",
                                field_identity(field, field_index),
                                node_id(entity),
                            ),
                        ));
                        let error = errors.last_mut().expect("just inserted diagnostic");
                        error.evidence =
                            json!({"entityMark": entity_mark, "fieldMark": field_mark});
                        error.supported_fixes = vec![
                            format!("Set {path} to {entity_mark}"),
                            format!("Remove {path} so the field inherits {entity_mark} from the entity"),
                        ];
                    }
                    (Some(entity_mark), _) => {
                        field_marks.insert(field_path(entity_index, field_index), entity_mark.to_owned());
                    }
                    (None, Some(field_mark)) => {
                        field_marks.insert(field_path(entity_index, field_index), field_mark.to_owned());
                    }
                    (None, None) => {}
                }
            }
        }
    }

    let mut reparents = 0usize;
    if tree {
        let identities: HashSet<&str> =
            nodes.iter().filter_map(|item| item["id"].as_str()).collect();
        // parent_before is only meaningful for a kept node moving between
        // two different existing parents.
        for (index, item) in nodes.iter().enumerate() {
            let Some(old_parent) = item["parent_before"].as_str() else {
                continue;
            };
            reparents += 1;
            let path = format!("/nodes/{index}/parent_before");
            let id = node_id(item);
            if matches!(item["change"].as_str(), Some("added") | Some("removed")) {
                errors.push(Diagnostic::error(
                    Some(kind.as_str()),
                    "change/conflict",
                    &path,
                    format!(
                        "Node {id} is marked {}; a node with parent_before must be kept in both views",
                        item["change"].as_str().expect("matched above")
                    ),
                ));
                let error = errors.last_mut().expect("just inserted diagnostic");
                error.subject.identity = Some(id.clone());
                error.supported_fixes = vec![
                    format!("Remove /nodes/{index}/change so the node can move with parent_before"),
                    format!("Remove /nodes/{index}/parent_before and keep the node's mark"),
                ];
            }
            if item["parent"].as_str() == Some(old_parent) {
                errors.push(Diagnostic::error(
                    Some(kind.as_str()),
                    "change/conflict",
                    &path,
                    format!(
                        "Node {id} has parent_before equal to its parent; the node has not changed parent"
                    ),
                ));
                let error = errors.last_mut().expect("just inserted diagnostic");
                error.subject.identity = Some(id.clone());
                error.supported_fixes =
                    vec![format!("Remove /nodes/{index}/parent_before")];
            }
            if !identities.contains(old_parent) {
                let mut candidates: Vec<_> = identities.iter().copied().collect();
                candidates.sort_unstable();
                let mut error = Diagnostic::error(
                    Some(kind.as_str()),
                    "input/unknown-reference",
                    &path,
                    format!("Unknown reference: {old_parent}"),
                );
                error.subject.identity = Some(old_parent.to_owned());
                error.evidence = json!({"knownIdentities": candidates});
                error.supported_fixes = vec![format!("Set {path} to an existing identity")];
                errors.push(error);
            }
        }
        // Connection marks follow the child node; parent_before marks the old
        // connection as removed.
        for (index, item) in nodes.iter().enumerate() {
            let Some(parent) = item["parent"].as_str() else {
                continue;
            };
            let mark = match item["change"].as_str() {
                Some(mark @ ("added" | "removed")) => Some(mark),
                _ if item.get("parent_before").is_some() => Some("added"),
                _ => None,
            };
            if let Some(mark) = mark {
                relation_marks
                    .insert(format!("/nodes/{index}/parent"), mark.to_owned());
            }
            if item.get("parent_before").is_some() {
                relation_marks
                    .insert(format!("/nodes/{index}/parent_before"), "removed".to_owned());
            }
            // Dangling: the connection only exists in the views where both
            // endpoints exist.
            let parent_mark = node_marks.get(parent);
            let child_mark = item["change"].as_str();
            let mut dangling_view: Option<(&str, &str)> = None;
            if child_mark == Some("removed") && parent_mark == Some(&"added".to_owned()) {
                dangling_view = Some(("before", "the child only exists in the after view"));
            } else if child_mark == Some("added") && parent_mark == Some(&"removed".to_owned()) {
                dangling_view = Some(("after", "the parent only exists in the before view"));
            } else if child_mark.is_none() {
                if parent_mark == Some(&"removed".to_owned()) {
                    dangling_view = Some(("after", "the parent is removed"));
                } else if parent_mark == Some(&"added".to_owned()) {
                    dangling_view = Some(("before", "the parent is added"));
                }
            }
            if let Some((view, reason)) = dangling_view {
                let path = format!("/nodes/{index}/parent");
                let fixes: Vec<String> = if child_mark.is_none() && view == "after" {
                    vec![
                        format!("Set /nodes/{index}/change to removed"),
                        format!(
                            "Set /nodes/{index}/parent_before to {parent} and change /nodes/{index}/parent to the new parent"
                        ),
                    ]
                } else {
                    vec![format!("Set /nodes/{index}/parent to a node that exists in the {view} view")]
                };
                errors.push(Diagnostic::error(
                    Some(kind.as_str()),
                    "change/dangling-relation",
                    &path,
                    format!(
                        "Connection {parent} -> {} dangles in the {view} view: {reason}",
                        node_id(item)
                    ),
                ));
                let error = errors.last_mut().expect("just inserted diagnostic");
                error.subject.identity = Some(node_id(item));
                error.evidence = json!({"parent": parent, "view": view});
                error.supported_fixes = fixes;
            }
            if let Some(old_parent) = item["parent_before"].as_str() {
                if matches!(node_marks.get(old_parent).map(String::as_str), Some("added" | "removed")) {
                    let path = format!("/nodes/{index}/parent_before");
                    errors.push(Diagnostic::error(
                        Some(kind.as_str()),
                        "change/dangling-relation",
                        &path,
                        format!(
                            "Connection {old_parent} -> {} dangles in the before view: the old parent does not exist there",
                            node_id(item)
                        ),
                    ));
                    let error = errors.last_mut().expect("just inserted diagnostic");
                    error.subject.identity = Some(node_id(item));
                    error.evidence = json!({"oldParent": old_parent, "view": "before"});
                    error.supported_fixes =
                        vec![format!("Set /nodes/{index}/parent_before to a node that exists in the before view")];
                }
            }
        }
        // The before view keeps its own acyclic parent structure.
        check_before_cycle(nodes, &node_marks, kind.as_str(), &mut errors);
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut counts = ChangeCounts::default();
    for mark in node_marks.values() {
        bump(&mut counts, mark);
    }
    if !tree {
        for mark in authored_marks.values() {
            bump(&mut counts, mark);
        }
    }
    for mark in field_marks.values() {
        bump(&mut counts, mark);
    }
    counts.added += reparents;
    counts.removed += reparents;

    Ok(Some(Resolved {
        counts,
        nodes: node_marks,
        relations: relation_marks,
        fields: field_marks,
    }))
}

fn node_id(item: &Value) -> String {
    item["id"].as_str().unwrap_or_default().to_owned()
}

fn relation_identity(item: &Value) -> String {
    if let Some(label) = item["label"].as_str().filter(|label| !label.trim().is_empty()) {
        return label.to_owned();
    }
    format!(
        "{} -> {}",
        item["from"].as_str().unwrap_or_default(),
        item["to"].as_str().unwrap_or_default()
    )
}

fn field_identity(field: &Value, index: usize) -> String {
    field["name"]
        .as_str()
        .or(field["label"].as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("#{index}"))
}

fn field_path(entity_index: usize, field_index: usize) -> String {
    format!("/entities/{entity_index}/attributes/{field_index}")
}

fn bump(counts: &mut ChangeCounts, mark: &str) {
    match mark {
        "added" => counts.added += 1,
        "removed" => counts.removed += 1,
        "changed" => counts.changed += 1,
        _ => {}
    }
}

/// The before view derives every parent from `parent_before` when present;
/// a cycle there must involve a reparented node, reported at the first
/// `parent_before`-bearing member of the cycle.
fn check_before_cycle(
    nodes: &[Value],
    node_marks: &BTreeMap<String, String>,
    kind: &str,
    errors: &mut Diagnostics,
) {
    let id_of: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item["id"].as_str().map(|id| (id, index)))
        .collect();
    let before_parent = |index: usize| -> Option<usize> {
        let item = &nodes[index];
        if node_marks.get(node_id(item).as_str()).map(String::as_str) == Some("added") {
            return None;
        }
        let parent = item["parent_before"]
            .as_str()
            .or(item["parent"].as_str())?;
        id_of.get(parent).copied()
    };
    let mut state: HashMap<usize, usize> = HashMap::new();
    for start in 0..nodes.len() {
        if state.contains_key(&start) {
            continue;
        }
        let mut path: Vec<usize> = Vec::new();
        let mut current = Some(start);
        while let Some(node) = current {
            if let Some(&first) = state.get(&node) {
                if first == start {
                    let cycle: Vec<String> =
                        path[first..].iter().map(|&index| node_id(&nodes[index])).collect();
                    let mut member = None;
                    for &index in &path[first..] {
                        if nodes[index].get("parent_before").is_some() {
                            member = Some(index);
                            break;
                        }
                    }
                    if let Some(member) = member {
                        errors.push(Diagnostic::error(
                            Some(kind),
                            "tree/cycle",
                            &format!("/nodes/{member}/parent_before"),
                            format!(
                                "Parent links form a cycle in the before view: {}",
                                cycle.join(" -> ")
                            ),
                        ));
                        let error = errors.last_mut().expect("just inserted diagnostic");
                        error.evidence = json!({"cycle": cycle});
                    }
                    return;
                }
                break;
            }
            state.insert(node, path.len());
            path.push(node);
            current = before_parent(node);
        }
    }
}

/// Map resolved marks onto the laid-out scene and add the description band.
/// Trunk and junction views are derived with the erd/lifecycle rendering in
/// their own milestone; this attach covers node, relation and field marks.
pub(crate) fn attach(diagram: &mut CompiledDiagram, input: &DiagramInput) -> Result<(), Diagnostics> {
    let Some(resolved) = resolve(&input.document, diagram.diagram_type)? else {
        return Ok(());
    };
    let mut relations: Vec<Option<String>> = Vec::with_capacity(diagram.relations.len());
    for relation in &diagram.relations {
        relations.push(
            resolved
                .relations
                .get(relation.authored.source_path.as_str())
                .cloned(),
        );
    }
    let trunks = trunk_views(diagram, &resolved);
    let junctions = junction_views(diagram, &relations);
    let band = build_band(diagram, &resolved);
    diagram.view_box.height += band.bounds.height;
    diagram.changes = Some(ChangeFacts {
        counts: resolved.counts,
        nodes: resolved.nodes,
        relations,
        fields: resolved.fields,
        trunks,
        junctions,
        band,
    });
    Ok(())
}

/// An erd trunk exists only in the view where none of its member relations
/// exist: every member added hides it in the before view, every member
/// removed hides it in the after view. Mixed or unmarked trunks stay in both.
fn trunk_views(diagram: &CompiledDiagram, resolved: &Resolved) -> BTreeMap<String, String> {
    let mut trunks = BTreeMap::new();
    for container in &diagram.containers {
        if container.authored.kind != "erd-trunk" {
            continue;
        }
        let marks: Vec<Option<&str>> = container
            .authored
            .geometry_provenance
            .iter()
            .filter_map(|path| resolved.relations.get(path.as_str()).map(|mark| Some(mark.as_str())))
            .collect();
        if marks.is_empty() {
            continue;
        }
        if marks.iter().all(|mark| *mark == Some("added")) {
            trunks.insert(container.authored.id.clone(), "before".to_owned());
        } else if marks.iter().all(|mark| *mark == Some("removed")) {
            trunks.insert(container.authored.id.clone(), "after".to_owned());
        }
    }
    trunks
}

/// A lifecycle junction disappears from a view when fewer than two visible
/// relations pass through it (within 0.5 of the polyline).
fn junction_views(diagram: &CompiledDiagram, relations: &[Option<String>]) -> Vec<Vec<String>> {
    let DiagramFacts::Lifecycle { presentation, .. } = &diagram.facts else {
        return Vec::new();
    };
    let visible_in = |mark: Option<&str>, view: &str| match mark {
        Some("added") => view == "after",
        Some("removed") => view == "before",
        _ => true,
    };
    presentation
        .junctions
        .iter()
        .map(|junction| {
            let mut hidden = Vec::new();
            for view in ["before", "after"] {
                let visible = diagram
                    .relations
                    .iter()
                    .zip(relations.iter())
                    .filter(|(relation, mark)| {
                        visible_in(mark.as_deref(), view)
                            && relation.points.iter().zip(relation.points.iter().skip(1)).any(
                                |(start, end)| {
                                    crate::geometry::point_segment_distance(*junction, *start, *end)
                                        <= 0.5
                                },
                            )
                    })
                    .count();
                if visible < 2 {
                    hidden.push(view.to_owned());
                }
            }
            hidden
        })
        .collect()
}

fn build_band(diagram: &CompiledDiagram, resolved: &Resolved) -> ChangeBand {
    let locale = i18n::resolve(&diagram.meta);
    let scale = (diagram.view_box.width / BAND_REFERENCE_WIDTH).max(1.0);
    let band_width = diagram.view_box.width - 2.0 * BAND_SIDE_MARGIN;
    let x = diagram.view_box.x + BAND_SIDE_MARGIN;
    let y = diagram.view_box.y + diagram.view_box.height;
    let title_font = BAND_TITLE_FONT * scale;
    let body_font = BAND_BODY_FONT * scale;

    let mut changes_lines = wrap_text(&locale.text("change.view.changes", &[]), title_font, band_width);
    let items: Vec<String> = [
        ("+", "change.count.added", resolved.counts.added),
        ("\u{2212}", "change.count.removed", resolved.counts.removed),
        ("~", "change.count.changed", resolved.counts.changed),
    ]
    .into_iter()
    .filter(|(_, _, count)| *count > 0)
    .map(|(symbol, key, count)| {
        format!("{symbol} {} {count}", locale.text(key, &[]))
    })
    .collect();
    changes_lines.extend(pack_items(&items, body_font, band_width));
    let note = (resolved.counts.changed > 0).then(|| {
        locale.text(
            "change.note.changed",
            &[("count", &resolved.counts.changed.to_string())],
        )
    });
    let before_lines = group_lines(
        &locale.text("change.view.before", &[]),
        note.as_deref(),
        title_font,
        body_font,
        band_width,
    );
    let after_lines = group_lines(
        &locale.text("change.view.after", &[]),
        note.as_deref(),
        title_font,
        body_font,
        band_width,
    );

    let lines = changes_lines
        .len()
        .max(before_lines.len())
        .max(after_lines.len());
    let height =
        (BAND_TOP_PADDING + lines as f64 * BAND_LINE_HEIGHT + BAND_BOTTOM_PADDING) * scale;
    let band = |group_lines: &[String]| -> Vec<Text> {
        group_lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let font = if index == 0 { title_font } else { body_font };
                Text {
                    source_path: String::new(),
                    content: line.clone(),
                    position: Point {
                        x,
                        y: y
                            + (BAND_TOP_PADDING
                                + index as f64 * BAND_LINE_HEIGHT
                                + font * 0.8)
                                * scale,
                    },
                    font_size: font,
                    anchor: "start".to_owned(),
                }
            })
            .collect()
    };
    ChangeBand {
        bounds: Rect {
            x,
            y,
            width: band_width,
            height,
        },
        changes: band(&changes_lines),
        before: band(&before_lines),
        after: band(&after_lines),
    }
}

/// Join count items with " · ", starting a new line when the joined string
/// would wrap, so every count stays inside one tspan run.
fn pack_items(items: &[String], font: f64, max_width: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for item in items {
        let joined = if current.is_empty() {
            item.clone()
        } else {
            format!("{current} \u{b7} {item}")
        };
        if wrap_text(&joined, font, max_width).len() == 1 || current.is_empty() {
            current = joined;
        } else {
            lines.push(std::mem::take(&mut current));
            current = item.clone();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn group_lines(
    title: &str,
    note: Option<&str>,
    title_font: f64,
    body_font: f64,
    max_width: f64,
) -> Vec<String> {
    let mut lines = wrap_text(title, title_font, max_width);
    if let Some(note) = note {
        lines.extend(wrap_text(note, body_font, max_width));
    }
    lines
}
