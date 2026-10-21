// Entity-relationship facts in the actual SVG: every column's name, type and
// key badges, each domain band, and both cardinality ends of every relationship.
use super::{route_points, selector, visible_text, ContentFinding};
use crate::geometry;
use crate::layout::erd::{key_roles, relationship_ends, trunks};
use crate::scene::{Point, Rect};
use scraper::ElementRef;
use serde_json::{json, Value};

/// Shared trunk paths as straight segments.
pub(super) fn trunks(svg: ElementRef<'_>) -> Vec<(Point, Point)> {
    svg.select(&selector("path[data-er-trunk]"))
        .filter_map(|path| {
            let points = route_points(path.value().attr("data-composition-points")?)?;
            match points.as_slice() {
                [start, end] => Some((*start, *end)),
                _ => None,
            }
        })
        .collect()
}

/// The drawn route of a bundled member: the pieces around its trunk stretches,
/// each of which a shared trunk path has to cover.
pub(super) fn bundled_route(
    points: &[Point],
    junction: &str,
    trunk_paths: &[(Point, Point)],
) -> Option<String> {
    let (axis, coordinate) = trunks::parse_junction(junction)?;
    let runs = trunks::trunk_runs(points, axis, coordinate);
    let along = |point: Point| if axis == 0 { point.y } else { point.x };
    let across = |point: Point| if axis == 0 { point.x } else { point.y };
    let mut covering: Vec<(f64, f64)> = trunk_paths
        .iter()
        .filter(|(start, end)| {
            (across(*start) - coordinate).abs() <= 0.5 && (across(*end) - coordinate).abs() <= 0.5
        })
        .map(|(start, end)| {
            let (a, b) = (along(*start), along(*end));
            (a.min(b), a.max(b))
        })
        .collect();
    covering.sort_by(|a, b| a.0.total_cmp(&b.0));
    let covered = |low: f64, high: f64| {
        let mut reach = low;
        for (start, end) in &covering {
            if *start > reach + 0.5 {
                break;
            }
            reach = reach.max(*end);
        }
        reach + 0.5 >= high
    };
    if !runs.iter().all(|run| covered(run.low, run.high)) {
        return None;
    }
    if runs.is_empty() {
        return Some(geometry::rounded_path(points, 8.0));
    }
    Some(
        trunks::drawn_pieces(points, &runs)
            .iter()
            .map(|piece| geometry::rounded_path(piece, 8.0))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn texts(element: ElementRef<'_>) -> Vec<String> {
    element
        .select(&selector("text"))
        .map(|text| visible_text(&text.text().collect::<String>()))
        .collect()
}

fn number(element: ElementRef<'_>, name: &str) -> f64 {
    element
        .value()
        .attr(name)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(f64::NAN)
}

fn body(node: ElementRef<'_>) -> Option<Rect> {
    let body = node
        .children()
        .filter_map(ElementRef::wrap)
        .find(|element| element.value().attr("data-graph-role") == Some("node-body"))?;
    Some(Rect {
        x: number(body, "x"),
        y: number(body, "y"),
        width: number(body, "width"),
        height: number(body, "height"),
    })
}

fn marker_id(cardinality: &str, optional: bool, end: &str) -> String {
    format!(
        "url(#er-{cardinality}{}-{end})",
        if optional { "-optional" } else { "" }
    )
}

fn cardinality(relationship: &Value, from: bool) -> &'static str {
    match relationship[if from {
        "fromCardinality"
    } else {
        "toCardinality"
    }]
    .as_str()
    {
        Some("one") => "one",
        Some("many") => "many",
        _ if from => "many",
        _ => "one",
    }
}

// A domain is named only by its band, which must enclose the table.
fn domain_drawn(bands: &[ElementRef<'_>], tag: &str, table: Option<Rect>) -> bool {
    bands.iter().any(|band| {
        band.value().attr("data-domain-band") == Some(tag)
            && band
                .select(&selector("text[data-domain-label]"))
                .any(|text| visible_text(&text.text().collect::<String>()) == visible_text(tag))
            && band
                .children()
                .filter_map(ElementRef::wrap)
                .find(|element| element.value().name() == "rect")
                .zip(table)
                .is_some_and(|(frame, table)| {
                    let (x, y) = (number(frame, "x"), number(frame, "y"));
                    let (width, height) = (number(frame, "width"), number(frame, "height"));
                    table.x >= x - geometry::EPSILON
                        && table.y >= y - geometry::EPSILON
                        && table.x + table.width <= x + width + geometry::EPSILON
                        && table.y + table.height <= y + height + geometry::EPSILON
                })
    })
}

pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings: Vec<ContentFinding> = vec![];
    let empty = vec![];
    let entities = metadata["facts"]["entities"].as_array().unwrap_or(&empty);
    let relationships = metadata["facts"]["relationships"]
        .as_array()
        .unwrap_or(&empty);
    let nodes: Vec<_> = svg.select(&selector("g[data-node-id]")).collect();
    let bands: Vec<_> = svg.select(&selector("g[data-domain-band]")).collect();
    for (index, entity) in entities.iter().enumerate() {
        let id = entity["id"].as_str().unwrap_or("");
        let path = format!("/metadata/facts/entities/{index}");
        let Some(node) = nodes
            .iter()
            .find(|node| node.value().attr("data-node-id") == Some(id))
        else {
            findings.push((
                "artifact/semantic-count",
                path,
                "Authored entity is missing from the actual SVG".into(),
                json!({ "id": id }),
            ));
            continue;
        };
        let rows: Vec<_> = node.select(&selector("g[data-er-row]")).collect();
        let attributes = entity["attributes"].as_array().unwrap_or(&empty);
        if rows.len() != attributes.len() {
            findings.push((
                "artifact/semantic-count",
                format!("{path}/attributes"),
                "Rendered column rows differ from the authored columns".into(),
                json!({"id":id,"authored":attributes.len(),"actual":rows.len()}),
            ));
        }
        for (column, (attribute, row)) in attributes.iter().zip(&rows).enumerate() {
            let actual = texts(*row);
            let roles: Vec<String> = key_roles(attribute)
                .into_iter()
                .map(|(_, role)| role)
                .collect();
            let badges: Vec<String> = row
                .select(&selector("[data-er-badge]"))
                .filter_map(|badge| badge.value().attr("data-er-badge").map(str::to_owned))
                .collect();
            let mut expected: Vec<String> = roles.iter().map(|role| role.to_uppercase()).collect();
            expected.push(visible_text(attribute["name"].as_str().unwrap_or("")));
            if let Some(kind) = attribute["type"].as_str().filter(|kind| !kind.is_empty()) {
                expected.push(visible_text(kind));
            }
            if actual != expected || badges != roles {
                findings.push((
                    "artifact/semantic-text",
                    format!("{path}/attributes/{column}"),
                    "Column name, type or key roles differ from the actual SVG row".into(),
                    json!({"id":id,"expected":expected,"actual":actual,"keys":roles,"badges":badges}),
                ));
            }
        }
        if let Some(sublabel) = entity["sublabel"]
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
                    "Entity sublabel is missing from its actual details".into(),
                    json!({ "id": id }),
                ));
            }
        }
        if let Some(tag) = entity["tag"].as_str().filter(|tag| !tag.is_empty()) {
            if !domain_drawn(&bands, tag, body(*node)) {
                findings.push((
                    "artifact/semantic-text",
                    format!("{path}/tag"),
                    "Entity domain is not drawn as a band around the table".into(),
                    json!({"id":id,"tag":tag}),
                ));
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
        let optional =
            |from: bool| relationship[if from { "fromOptional" } else { "toOptional" }] == true;
        let expected = [
            marker_id(cardinality(relationship, true), optional(true), "start"),
            marker_id(cardinality(relationship, false), optional(false), "end"),
            relationship_ends(relationship).join(" "),
        ];
        let actual = ["marker-start", "marker-end", "data-er-ends"]
            .map(|name| edge.value().attr(name).unwrap_or("").to_owned());
        if actual != expected {
            findings.push((
                "artifact/semantic-relation",
                format!("/metadata/facts/relationships/{index}"),
                "Relationship cardinality ends differ from the actual SVG markers".into(),
                json!({"expected":expected,"actual":actual}),
            ));
        }
    }
    findings
}
