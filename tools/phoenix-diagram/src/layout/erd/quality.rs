// The shared relationship gates the frozen erd and class renderers run after
// layout (geometry.mjs clean*Problems), as compile diagnostics.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::composition::{
    collect_ambiguous_corridors, collect_endpoint_side_issues, collect_label_route_clearance,
    collect_proper_crossings, collect_route_obstacle_hits, collect_route_rhythm_issues,
    CorridorOptions, CrossingOptions, LabelRect, Obstacle, RoutedRelation,
};
use crate::geometry::rects_overlap;
use crate::scene::{Point, Rect};
use serde_json::{json, Value};

/// One routed relationship: its index in the authored collection, raw facts,
/// logical route points and resolved endpoint sides.
pub(crate) struct Routed<'a> {
    pub index: usize,
    pub raw: &'a Value,
    pub points: &'a [Point],
    pub from_side: &'a str,
    pub to_side: &'a str,
}

pub(crate) struct Gate<'a> {
    pub kind: &'static str,
    /// What a relationship must not cross ("entity", "type").
    pub obstacle_kind: &'static str,
    pub nodes: &'a [(String, Rect)],
    pub showcase: bool,
}

pub(crate) fn diagnostic(
    kind: &str,
    code: &str,
    path: &str,
    message: impl Into<String>,
    evidence: Value,
) -> Diagnostic {
    let mut result = Diagnostic::error(Some(kind), code, path, message);
    result.stage = "layout".into();
    result.evidence = evidence;
    result
}

fn routed_relations(relations: &[Routed<'_>]) -> Vec<RoutedRelation> {
    relations
        .iter()
        .map(|relation| {
            RoutedRelation::new(
                relation.index,
                relation.raw.clone(),
                relation.points.to_vec(),
            )
        })
        .collect()
}

impl Gate<'_> {
    fn error(&self, code: &str, index: usize, message: &str, evidence: Value) -> Diagnostic {
        diagnostic(
            self.kind,
            code,
            &format!("/relationships/{index}"),
            message,
            evidence,
        )
    }

    /// Endpoint sides, opaque nodes, crossings, shared corridors and rhythm.
    pub fn routes(&self, relations: &[Routed<'_>], errors: &mut Diagnostics) {
        for relation in relations {
            let raw = relation.raw;
            let authored = |field: &str| {
                raw[field]
                    .as_str()
                    .filter(|side| !side.is_empty() && *side != "auto")
            };
            // A named route or authored via carries its own geometry; it does not
            // acquire an inferred side contract.
            let automatic = !(raw["route"].as_str().is_some_and(|route| route != "auto")
                || raw["via"].is_array());
            let from = authored("fromSide").or(automatic.then_some(relation.from_side));
            let to = authored("toSide").or(automatic.then_some(relation.to_side));
            for issue in
                collect_endpoint_side_issues(relation.points, from.unwrap_or(""), to.unwrap_or(""))
            {
                errors.push(self.error(
                    "clean-flow/endpoint-side-direction",
                    relation.index,
                    "Relationship does not leave or enter its endpoint side perpendicularly",
                    json!(issue),
                ));
            }
        }
        let routed = routed_relations(relations);
        let obstacles: Vec<_> = self
            .nodes
            .iter()
            .map(|(id, bounds)| Obstacle {
                id: id.clone(),
                bounds: *bounds,
            })
            .collect();
        for hit in collect_route_obstacle_hits(&routed, &obstacles, 2.0) {
            errors.push(self.error(
                "clean-flow/edge-through-node",
                hit.relation_index,
                &format!("Relationship crosses an unrelated {}", self.obstacle_kind),
                json!(hit),
            ));
        }
        if !self.showcase {
            return;
        }
        for hit in collect_proper_crossings(&routed, &CrossingOptions::default()) {
            errors.push(self.error(
                "composition/proper-crossing",
                hit.left_relation_index,
                "Unrelated relationships cross",
                json!(hit),
            ));
        }
        for hit in collect_ambiguous_corridors(&routed, &CorridorOptions::default()) {
            errors.push(self.error(
                "composition/ambiguous-corridor",
                hit.left_relation_index,
                "Unrelated relationships share one corridor",
                json!(hit),
            ));
        }
        for issue in collect_route_rhythm_issues(&routed, 16.0, 8.0) {
            errors.push(self.error(
                issue.code,
                issue.relation_index,
                "Relationship has a segment below the route rhythm floor",
                json!(issue),
            ));
        }
    }

    /// Label rectangles must clear every node and, in showcase, every other route.
    pub fn labels(&self, labels: &[LabelRect], relations: &[Routed<'_>], errors: &mut Diagnostics) {
        for label in labels {
            for (id, bounds) in self.nodes {
                if rects_overlap(&label.bounds, bounds, -2.0) {
                    errors.push(self.error(
                        "layout/constraint",
                        label.relation_index as usize,
                        &format!("Relationship label overlaps {} {id}", self.obstacle_kind),
                        json!({"label":label.label,"labelRect":label.bounds,"node":id}),
                    ));
                }
            }
        }
        if !self.showcase {
            return;
        }
        for hit in collect_label_route_clearance(labels, &routed_relations(relations), 4.0) {
            errors.push(self.error(
                "composition/label-route-clearance",
                hit.label_relation_index as usize,
                "Relationship label is closer than 4px to another route",
                json!(hit),
            ));
        }
    }
}
