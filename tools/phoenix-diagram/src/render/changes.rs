//! SVG surface for single-input change marks. Every function reads
//! `scene.changes` and emits nothing when it is absent, so inputs without
//! change marks keep byte-identical output.

use super::escape_text;
use crate::geometry::format_number as number;
use crate::i18n;
use crate::scene::{ChangeCounts, CompiledDiagram, Node, Point};
use std::fmt::Write;

pub(super) fn root_attributes(scene: &CompiledDiagram) -> String {
    if scene.changes.is_none() {
        return String::new();
    }
    " data-change-ui=\"\" data-change-view=\"changes\"".to_owned()
}

pub(super) fn attribute(mark: &str) -> String {
    format!(" data-change=\"{mark}\"")
}

/// `data-change` for a node `<g>` from its effective mark.
pub(super) fn node_attribute(scene: &CompiledDiagram, node: &Node) -> String {
    scene
        .changes
        .as_ref()
        .and_then(|changes| changes.nodes.get(node.authored.id.as_str()))
        .map(|mark| attribute(mark))
        .unwrap_or_default()
}

/// `data-change` for a laid-out relation from its mark, aligned by index.
pub(super) fn relation_attribute(scene: &CompiledDiagram, index: usize) -> String {
    scene
        .changes
        .as_ref()
        .and_then(|changes| changes.relations.get(index))
        .and_then(Option::as_deref)
        .map(attribute)
        .unwrap_or_default()
}

/// `data-change-hidden-in` for an erd trunk container.
pub(super) fn trunk_attribute(scene: &CompiledDiagram, trunk_id: &str) -> String {
    scene
        .changes
        .as_ref()
        .and_then(|changes| changes.trunks.get(trunk_id))
        .map(|view| format!(" data-change-hidden-in=\"{view}\""))
        .unwrap_or_default()
}

/// `data-change` for an erd field row from its effective mark.
pub(super) fn field_attribute(scene: &CompiledDiagram, source_path: &str) -> String {
    scene
        .changes
        .as_ref()
        .and_then(|changes| changes.fields.get(source_path))
        .map(|mark| attribute(mark))
        .unwrap_or_default()
}

/// A bare field-row mark symbol (no disc): the row tint carries the color.
pub(super) fn field_mark(scene: &CompiledDiagram, mark: &str, cx: f64, cy: f64) -> String {
    let label = i18n::resolve(&scene.meta).text(&format!("change.mark.{mark}"), &[]);
    let symbol = match mark {
        "added" => format!("M {} 0 H {} M 0 {} V {}", -3.0, 3.0, -3.0, 3.0),
        "removed" => "M -3 0 H 3".to_owned(),
        _ => "M -3 0 C -1 -1.95, 1 1.95, 3 0".to_owned(),
    };
    let mut out = String::new();
    write!(
        out,
        "<g data-change-mark=\"{mark}\" role=\"img\" aria-label=\"{}\" transform=\"translate({} {})\"><path d=\"{symbol}\"/><title>{}</title></g>",
        escape_text(&label),
        number(cx),
        number(cy),
        escape_text(&label)
    )
    .unwrap();
    out
}

pub(super) fn defs(scene: &CompiledDiagram) -> String {
    if scene.changes.is_none() {
        return String::new();
    }
    format!("<style>{CHANGE_STYLES}</style>")
}

/// One `<style>` block; every rule is prefixed with the change root so the
/// rules only apply to figures that carry change marks. Declarations stay
/// inside the supported figure list; removed and hidden elements are
/// `display: none` in the views where they do not exist.
const CHANGE_STYLES: &str = r#"
svg[data-change-ui] [data-change-mark="added"] circle { fill: var(--backend-stroke); stroke: none; }
svg[data-change-ui] [data-change-mark="removed"] circle { fill: var(--security-stroke); stroke: none; }
svg[data-change-ui] [data-change-mark="changed"] circle { fill: var(--cloud-stroke); stroke: none; }
svg[data-change-ui] [data-change-mark] path { fill: none; stroke: var(--mask); stroke-width: 2px; stroke-linecap: round; }
svg[data-change-ui] [data-change-band] text { fill: var(--text); }
svg[data-change-ui] .er-change-row-added { fill: var(--backend-stroke); fill-opacity: 0.14; }
svg[data-change-ui] .er-change-row-removed { fill: var(--security-stroke); fill-opacity: 0.14; }
svg[data-change-ui] .er-change-row-changed { fill: var(--cloud-stroke); fill-opacity: 0.14; }
svg[data-change-ui][data-change-view="changes"] g[data-node-id][data-change="added"] > rect[data-graph-role="node-body"] { stroke: var(--backend-stroke); stroke-width: 2.5px; }
svg[data-change-ui][data-change-view="changes"] g[data-node-id][data-change="changed"] > rect[data-graph-role="node-body"] { stroke: var(--cloud-stroke); stroke-width: 2.5px; stroke-dasharray: 2px 3px; }
svg[data-change-ui][data-change-view="changes"] g[data-node-id][data-change="removed"] > rect[data-graph-role="node-body"] { stroke: var(--security-stroke); stroke-width: 2.5px; stroke-dasharray: 6px 4px; }
svg[data-change-ui][data-change-view="changes"] path[data-graph-role="edge"][data-change="added"] { stroke: var(--backend-stroke); stroke-width: 2.5px; }
svg[data-change-ui][data-change-view="changes"] path[data-graph-role="edge"][data-change="removed"] { stroke: var(--security-stroke); stroke-width: 2.5px; stroke-dasharray: 6px 4px; }
svg[data-change-ui][data-change-view="changes"] [data-change="removed"] { opacity: 0.62; }
svg[data-change-ui][data-change-view="before"] [data-change="added"] { display: none; }
svg[data-change-ui][data-change-view="before"] [data-change-hidden-in~="before"] { display: none; }
svg[data-change-ui][data-change-view="before"] [data-change-mark] { display: none; }
svg[data-change-ui][data-change-view="before"] [data-change-when="changes"] { display: none; }
svg[data-change-ui][data-change-view="before"] [data-change-when="after"] { display: none; }
svg[data-change-ui][data-change-view="after"] [data-change="removed"] { display: none; }
svg[data-change-ui][data-change-view="after"] [data-change-hidden-in~="after"] { display: none; }
svg[data-change-ui][data-change-view="after"] [data-change-mark] { display: none; }
svg[data-change-ui][data-change-view="after"] [data-change-when="changes"] { display: none; }
svg[data-change-ui][data-change-view="after"] [data-change-when="before"] { display: none; }
svg[data-change-ui][data-change-view="changes"] [data-change-when="before"] { display: none; }
svg[data-change-ui][data-change-view="changes"] [data-change-when="after"] { display: none; }
"#;

/// The node badge sits in the node's top-right corner, mirroring the delta
/// marker position.
pub(super) fn node_mark(scene: &CompiledDiagram, node: &Node) -> String {
    let Some(changes) = scene.changes.as_ref() else {
        return String::new();
    };
    let Some(mark) = changes.nodes.get(node.authored.id.as_str()) else {
        return String::new();
    };
    badge(
        scene,
        mark,
        node.bounds.x + node.bounds.width - 9.0,
        node.bounds.y + 9.0,
        8.0,
        4.0,
    )
}

/// Relation badges along the first route point. Tree connections follow the
/// child node's mark and share the child's badge; the reparent connections
/// added with the tree layout work carry their own badges there.
pub(super) fn edge_marks(scene: &CompiledDiagram) -> String {
    let Some(changes) = scene.changes.as_ref() else {
        return String::new();
    };
    let mut out = String::new();
    if scene.diagram_type == crate::input::DiagramType::Tree {
        // Only the reparented node's two connections carry badges — the
        // child-following links share the child node's own badge. Both sit
        // beside the last route segment's midpoint, where the two links are
        // distinguishable.
        let reparented: std::collections::BTreeSet<&str> = scene
            .relations
            .iter()
            .filter(|relation| relation.authored.kind == "tree-edge-before")
            .map(|relation| relation.to.as_str())
            .collect();
        for (index, relation) in scene.relations.iter().enumerate() {
            let carries_badge = relation.authored.kind == "tree-edge-before"
                || (relation.authored.kind == "tree-edge"
                    && reparented.contains(relation.to.as_str()));
            if !carries_badge {
                continue;
            }
            let Some(mark) = changes.relations.get(index).and_then(Option::as_deref) else {
                continue;
            };
            if relation.points.len() < 2 {
                continue;
            }
            let end = relation.points[relation.points.len() - 1];
            let start = relation.points[relation.points.len() - 2];
            let mid = Point {
                x: (start.x + end.x) / 2.0,
                y: (start.y + end.y) / 2.0,
            };
            out.push_str(&badge(scene, mark, mid.x + 9.0, mid.y - 7.0, 6.0, 3.0));
        }
        return out;
    }
    for (index, relation) in scene.relations.iter().enumerate() {
        let Some(mark) = changes.relations.get(index).and_then(Option::as_deref) else {
            continue;
        };
        let Some(point) = relation.points.first() else {
            continue;
        };
        out.push_str(&badge(scene, mark, point.x + 9.0, point.y - 7.0, 6.0, 3.0));
    }
    out
}

fn badge(scene: &CompiledDiagram, mark: &str, cx: f64, cy: f64, radius: f64, arm: f64) -> String {
    let label = i18n::resolve(&scene.meta).text(&format!("change.mark.{mark}"), &[]);
    let symbol = match mark {
        "added" => format!("M {} 0 H {} M 0 {} V {}", -arm, arm, -arm, arm),
        "removed" => format!("M {} 0 H {}", -arm, arm),
        _ => format!(
            "M {} 0 C {} {} {} {} {} 0",
            -arm,
            -arm / 3.0,
            -arm * 0.65,
            arm / 3.0,
            arm * 0.65,
            arm
        ),
    };
    let mut out = String::new();
    write!(
        out,
        "<g data-change-mark=\"{mark}\" role=\"img\" aria-label=\"{}\" transform=\"translate({} {})\"><circle r=\"{}\"/><path d=\"{symbol}\"/><title>{}</title></g>",
        escape_text(&label),
        number(cx),
        number(cy),
        number(radius),
        escape_text(&label)
    )
    .unwrap();
    out
}

/// The description band under the canvas: three view groups whose text
/// lines and positions the layout stage computed. Count runs keep
/// `data-change-count` so checks and readers can read the numbers.
pub(super) fn band(scene: &CompiledDiagram) -> String {
    let Some(changes) = scene.changes.as_ref() else {
        return String::new();
    };
    let band = &changes.band;
    let mut out = String::new();
    out.push_str("<g data-change-band=\"\">");
    band_group(&mut out, "changes", &band.changes, true, &changes.counts);
    band_group(&mut out, "before", &band.before, false, &changes.counts);
    band_group(&mut out, "after", &band.after, false, &changes.counts);
    out.push_str("</g>");
    out
}

fn band_group(
    out: &mut String,
    when: &str,
    lines: &[crate::scene::Text],
    with_counts: bool,
    counts: &ChangeCounts,
) {
    write!(out, "<g data-change-when=\"{when}\">").unwrap();
    let mut next_run = 0usize;
    for (index, line) in lines.iter().enumerate() {
        let mut attributes = String::new();
        if index == 0 {
            attributes.push_str(" font-weight=\"700\"");
        }
        if !with_counts && index > 0 {
            attributes.push_str(" data-change-note=\"\"");
        }
        write!(
            out,
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{}\"{attributes}>",
            number(line.position.x),
            number(line.position.y),
            number(line.font_size),
            escape_text(&line.anchor)
        )
        .unwrap();
        if with_counts && index > 0 {
            count_runs(out, &line.content, counts, &mut next_run);
        } else {
            out.push_str(&escape_text(&line.content));
        }
        out.push_str("</text>");
    }
    out.push_str("</g>");
}

/// Split a packed counts line back into its " · " items and emit each item
/// as a label run plus a `data-change-count` run, in counts order. The mark
/// index carries over to the next line, so a counts row that wraps keeps
/// every run on its own mark; a line without a trailing count (a wrapped
/// view name) consumes none.
fn count_runs(out: &mut String, content: &str, counts: &ChangeCounts, next: &mut usize) {
    let marks: [(&str, usize); 3] = [
        ("added", counts.added),
        ("removed", counts.removed),
        ("changed", counts.changed),
    ];
    let marks: Vec<(&str, usize)> = marks.into_iter().filter(|(_, count)| *count > 0).collect();
    for (index, piece) in content.split(" \u{b7} ").enumerate() {
        if index > 0 {
            out.push_str("<tspan> \u{b7} </tspan>");
        }
        let digits: String = piece
            .chars()
            .rev()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        let Some((mark, _)) = marks.get(*next).filter(|_| !digits.is_empty()) else {
            out.push_str(&escape_text(piece));
            continue;
        };
        *next += 1;
        let split = piece.len() - digits.len();
        write!(
            out,
            "<tspan>{}</tspan><tspan data-change-count=\"{mark}\">{}</tspan>",
            escape_text(&piece[..split]),
            &piece[split..]
        )
        .unwrap();
    }
}
