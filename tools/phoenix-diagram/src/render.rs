use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::evidence::SourceEvidence;
use crate::geometry::{
    finite_point, finite_rect, format_number as number, lifecycle_v3_path, polyline_path,
    rounded_path, workflow_v2_path,
};
use crate::input::{DiagramType, InputContract};
use crate::scene::{Card, CompiledDiagram, DiagramFacts, Point, Rect, Text};
use serde_json::{json, Value};
use std::fmt::Write;

mod class;
mod erd;
mod timeline;
mod tree;
mod waterfall;

type LegendGlyph = fn(&CompiledDiagram, &Card, f64, &mut String);

/// What a type outside the original five contributes to the shared SVG frame:
/// root attributes, extra definitions, its body and its legend glyphs.
struct TypeRenderer {
    root_attributes: fn(&CompiledDiagram) -> String,
    defs: fn(&CompiledDiagram) -> String,
    body: fn(&CompiledDiagram, &mut String),
    legend_glyph: LegendGlyph,
}

fn type_renderer(kind: DiagramType) -> Option<TypeRenderer> {
    match kind {
        DiagramType::Erd => Some(erd::RENDERER),
        DiagramType::Class => Some(class::RENDERER),
        DiagramType::Tree => Some(tree::RENDERER),
        DiagramType::Timeline => Some(timeline::RENDERER),
        DiagramType::Waterfall => Some(waterfall::RENDERER),
        _ => None,
    }
}

pub struct EmbeddedAssets<'a> {
    pub template: &'a str,
}

impl EmbeddedAssets<'static> {
    pub fn builtin() -> Self {
        Self {
            template: include_str!("../assets/template.html"),
        }
    }
}

fn failure(
    scene: &CompiledDiagram,
    code: &str,
    path: &str,
    message: impl Into<String>,
) -> Diagnostics {
    let mut diagnostic = Diagnostic::error(Some(scene.diagram_type.as_str()), code, path, message);
    diagnostic.stage = "render".into();
    vec![diagnostic]
}

pub fn escape_text(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(character),
        }
    }
    result
}

fn attribute(value: &Value, key: &str) -> String {
    value[key].as_str().map(escape_text).unwrap_or_default()
}

fn rect_attributes(rect: &Rect) -> String {
    format!(
        "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        number(rect.x),
        number(rect.y),
        number(rect.width),
        number(rect.height)
    )
}

fn text_markup(text: &Text, class: &str, attributes: &str) -> String {
    format!("<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{}\" class=\"{}\" data-source-path=\"{}\" {}>{}</text>",
        number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.anchor), class,
        escape_text(&text.source_path), attributes, escape_text(&text.content))
}

fn tone(kind: &str) -> &'static str {
    match kind {
        "frontend" | "start" | "active" => "frontend",
        "backend" | "success" => "backend",
        "database" | "decision" => "database",
        "cloud" | "waiting" => "cloud",
        "security" | "failure" => "security",
        "messagebus" => "messagebus",
        _ => "external",
    }
}

fn node_tone(scene: &CompiledDiagram, kind: &str) -> &'static str {
    if scene.diagram_type == DiagramType::Lifecycle && scene.schema_version == 1 {
        match kind {
            "active" => "backend",
            "decision" => "security",
            "success" => "database",
            _ => tone(kind),
        }
    } else {
        tone(kind)
    }
}

fn initial_marker(svg: &mut String, dot_x: f64, tip_x: f64, y: f64) {
    write!(svg,"<circle cx=\"{}\" cy=\"{}\" r=\"4.5\" style=\"fill: var(--arrow-emphasis)\"/><path d=\"M {} {} L {} {}\" class=\"a-emphasis\" stroke-width=\"1.4\"/><path d=\"M {} {} L {} {} L {} {} Z\" style=\"fill: var(--arrow-emphasis)\"/>",number(dot_x),number(y),number(dot_x+4.5),number(y),number(tip_x-6.0),number(y),number(tip_x-7.0),number(y-3.5),number(tip_x),number(y),number(tip_x-7.0),number(y+3.5)).unwrap();
}

fn edge_label_class(scene: &CompiledDiagram, variant: &str) -> &'static str {
    if scene.input_contract == InputContract::Legacy217 {
        match variant {
            "security" => "t-security",
            "emphasis" => "t-backend",
            "dashed" => "t-messagebus",
            _ => "t-muted",
        }
    } else {
        match variant {
            "security" => "t-edge-security",
            "emphasis" => "t-edge-emphasis",
            "dashed" => "t-edge-dashed",
            _ => "t-muted",
        }
    }
}

fn sigil_shape(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "calendar" => (
            "calendar",
            r#"<rect x="2" y="3.5" width="12" height="10.5" rx="2"/><path d="M5 2v3M11 2v3M2 7h12M5 10h2M9 10h2"/>"#,
        ),
        "clock" => (
            "clock",
            r#"<circle cx="8" cy="8" r="6"/><path d="M8 4v4l3 2"/>"#,
        ),
        "person" => (
            "person",
            r#"<circle cx="8" cy="4.5" r="2.5"/><path d="M3 14v-2a5 5 0 0 1 10 0v2"/>"#,
        ),
        "briefcase" => (
            "briefcase",
            r#"<rect x="2" y="5" width="12" height="9" rx="2"/><path d="M5 5V2h6v3M2 9h12M7 9v2h2V9"/>"#,
        ),
        "flag" => ("flag", r#"<path d="M3 14V2h10l-2 3 2 3H3"/>"#),
        "moon" => (
            "moon",
            r#"<path d="M13.5 10A6 6 0 0 1 6 2.5 6 6 0 1 0 13.5 10Z"/>"#,
        ),
        "frontend" => (
            "frontend",
            r#"<rect x="2" y="3" width="12" height="10" rx="2"/><path d="M2 6.5h12"/><circle cx="4.1" cy="4.8" r=".7" class="sigil-fill"/><circle cx="6.3" cy="4.8" r=".7" class="sigil-fill"/>"#,
        ),
        "backend" => ("backend", r#"<path d="M6 3 3 8l3 5M10 3l3 5-3 5"/>"#),
        "database" => (
            "database",
            r#"<ellipse cx="8" cy="4" rx="5" ry="2"/><path d="M3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4M3 8c0 1.1 2.2 2 5 2s5-.9 5-2"/>"#,
        ),
        "cloud" => (
            "cloud",
            r#"<path d="M4.3 12.5h7.3a2.4 2.4 0 0 0 .2-4.8 4 4 0 0 0-7.5-1.3A3.1 3.1 0 0 0 4.3 12.5Z"/>"#,
        ),
        "security" => (
            "security",
            r#"<path d="M8 2.2 13 4v3.5c0 3.1-1.8 5.4-5 6.5-3.2-1.1-5-3.4-5-6.5V4Z"/><path d="m5.8 8 1.5 1.5 3-3"/>"#,
        ),
        "messagebus" => (
            "messagebus",
            r#"<path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11"/><circle cx="5" cy="4.5" r="1" class="sigil-fill"/><circle cx="10.5" cy="8" r="1" class="sigil-fill"/><circle cx="7" cy="11.5" r="1" class="sigil-fill"/>"#,
        ),
        "external" => (
            "external",
            r#"<rect x="2.5" y="5" width="8.5" height="8" rx="1.5"/><path d="M8 2.5h5.5V8M13.5 2.5 7.5 8.5"/>"#,
        ),
        "start" => (
            "start",
            r#"<circle cx="8" cy="8" r="5"/><path d="m7 5.4 3.6 2.6L7 10.6Z" class="sigil-fill"/>"#,
        ),
        "active" => ("active", r#"<path d="M2 8h3l1.5-3.5L9 12l1.6-4H14"/>"#),
        "waiting" => (
            "waiting",
            r#"<path d="M4 2.5h8M4 13.5h8M5 3c0 2.8 2 3.2 3 5-1 1.8-3 2.2-3 5M11 3c0 2.8-2 3.2-3 5 1 1.8 3 2.2 3 5"/>"#,
        ),
        "decision" => ("decision", r#"<path d="M8 2.2 13.8 8 8 13.8 2.2 8Z"/>"#),
        "success" => (
            "success",
            r#"<circle cx="8" cy="8" r="5.3"/><path d="m5.2 8 1.8 1.8 3.8-4"/>"#,
        ),
        "failure" => (
            "failure",
            r#"<circle cx="8" cy="8" r="5.3"/><path d="m5.7 5.7 4.6 4.6m0-4.6-4.6 4.6"/>"#,
        ),
        "stop" => (
            "stop",
            r#"<circle cx="8" cy="8" r="5.3"/><rect x="6.1" y="6.1" width="3.8" height="3.8" rx=".5" class="sigil-fill"/>"#,
        ),
        _ => (
            "neutral",
            r#"<rect x="3" y="3" width="10" height="10" rx="2"/><circle cx="8" cy="8" r="1.2" class="sigil-fill"/>"#,
        ),
    }
}

fn edge_attributes(from: &str, to: &str, label: &str, key: usize, facts: &Value) -> String {
    let id = facts["id"]
        .as_str()
        .map(|id| format!(" data-edge-id=\"{}\"", escape_text(id)))
        .unwrap_or_default();
    format!("data-edge-from=\"{}\" data-edge-to=\"{}\" data-edge-label=\"{}\" data-edge-key=\"{key}\"{id}",escape_text(from),escape_text(to),escape_text(label))
}

fn validate_geometry(scene: &CompiledDiagram) -> Result<(), Diagnostics> {
    if !finite_rect(&scene.view_box) || scene.view_box.width <= 0.0 || scene.view_box.height <= 0.0
    {
        return Err(failure(
            scene,
            "render/non-finite",
            "/viewBox",
            "Canvas must have finite positive dimensions",
        ));
    }
    for node in &scene.nodes {
        if !finite_rect(&node.bounds) || node.bounds.width <= 0.0 || node.bounds.height <= 0.0 {
            return Err(failure(
                scene,
                "render/non-finite",
                &node.authored.source_path,
                "Node must have finite positive geometry",
            ));
        }
        if node.sigil.as_ref().is_some_and(|sigil| {
            !finite_point(sigil.position) || !sigil.size.is_finite() || sigil.size <= 0.0
        }) {
            return Err(failure(
                scene,
                "render/non-finite",
                &node.authored.source_path,
                "Node sigil geometry must be finite and positive",
            ));
        }
    }
    for relation in &scene.relations {
        if relation.points.len() < 2
            || relation.points.iter().any(|point| !finite_point(*point))
            || [
                relation.label_plate,
                relation.note_bounds,
                relation.note_backdrop,
            ]
            .iter()
            .flatten()
            .any(|rect| !finite_rect(rect))
        {
            return Err(failure(
                scene,
                "render/non-finite",
                &relation.authored.source_path,
                "Relation must have finite resolved geometry",
            ));
        }
    }
    for container in &scene.containers {
        if !finite_rect(&container.bounds)
            || container
                .label_plate
                .as_ref()
                .is_some_and(|plate| !finite_rect(plate))
            || container
                .rule
                .is_some_and(|points| points.iter().any(|point| !finite_point(*point)))
        {
            return Err(failure(
                scene,
                "render/non-finite",
                &container.authored.source_path,
                "Container geometry is not finite",
            ));
        }
    }
    let text = scene
        .text
        .iter()
        .chain(scene.nodes.iter().flat_map(|node| &node.text))
        .chain(scene.relations.iter().flat_map(|relation| &relation.text))
        .chain(
            scene
                .containers
                .iter()
                .flat_map(|container| &container.text),
        )
        .chain(scene.legends.iter().flat_map(|legend| &legend.text));
    for text in text {
        if !finite_point(text.position) || !text.font_size.is_finite() || text.font_size <= 0.0 {
            return Err(failure(
                scene,
                "render/non-finite",
                &text.source_path,
                "Text must have a finite position and positive font size",
            ));
        }
    }
    for legend in &scene.legends {
        if !finite_rect(&legend.bounds) {
            return Err(failure(
                scene,
                "render/non-finite",
                &legend.authored.source_path,
                "Legend geometry is not finite",
            ));
        }
    }
    Ok(())
}

// A pinned label can cover the terminal arrow without covering its route. Add
// one redundant glyph on that same straight path, without changing any pins.
fn architecture_direction_cue_ref_x(
    scene: &CompiledDiagram,
    index: usize,
    width: f64,
) -> Option<f64> {
    if scene.diagram_type != DiagramType::Architecture {
        return None;
    }
    let [start, end] = scene.relations[index].points.as_slice() else {
        return None;
    };
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = dx.hypot(dy);
    let stroke = number(width).parse::<f64>().ok()?;
    if (dx != 0.0 && dy != 0.0)
        || !length.is_finite()
        || length <= 0.0
        || stroke <= 0.0
        || !(stroke * 10.0).is_finite()
    {
        return None;
    }
    let axis = Point {
        x: dx / length,
        y: dy / length,
    };
    let normal = Point {
        x: -axis.y,
        y: axis.x,
    };
    let project = |rect: Rect, direction: Point| {
        let (a, extent) = if direction.x == 0.0 {
            ((rect.y - start.y) * direction.y, rect.height * direction.y)
        } else {
            ((rect.x - start.x) * direction.x, rect.width * direction.x)
        };
        (a.min(a + extent), a.max(a + extent))
    };
    let plates: Vec<_> = scene
        .relations
        .iter()
        .filter_map(|r| r.label_plate)
        .chain(scene.containers.iter().filter_map(|c| c.label_plate))
        .collect();
    let covered = plates.iter().any(|plate| {
        let (a, b) = project(*plate, axis);
        let (left, right) = project(*plate, normal);
        let first = a.max(length - 9.0 * stroke);
        let last = b.min(length + stroke);
        let half = (length + stroke - first) * 0.35;
        first < last && left < half && right > -half
    });
    if !covered {
        return None;
    }

    let (canvas_start, canvas_end) = project(scene.view_box, axis);
    let (canvas_left, canvas_right) = project(scene.view_box, normal);
    let half = 3.5 * stroke;
    let minimum = (9.0 * stroke).max(canvas_start + 9.0 * stroke);
    let maximum = (length - stroke).min(canvas_end - stroke);
    if minimum >= maximum || canvas_left > -half || canvas_right < half {
        return None;
    }
    let mut forbidden = Vec::new();
    let mut block = |rect: Rect, ink: f64| {
        let (left, right) = project(rect, normal);
        if left - ink < half && right + ink > -half {
            let (a, b) = project(rect, axis);
            forbidden.push((a - ink - stroke, b + ink + 9.0 * stroke));
        }
    };
    for node in &scene.nodes {
        block(node.bounds, 0.75);
    }
    for plate in plates {
        block(plate, 0.0);
    }
    for legend in &scene.legends {
        block(legend.bounds, 0.0);
    }
    for container in &scene.containers {
        let r = container.bounds;
        for border in [
            Rect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: 0.0,
            },
            Rect {
                x: r.x,
                y: r.y + r.height,
                width: r.width,
                height: 0.0,
            },
            Rect {
                x: r.x,
                y: r.y,
                width: 0.0,
                height: r.height,
            },
            Rect {
                x: r.x + r.width,
                y: r.y,
                width: 0.0,
                height: r.height,
            },
        ] {
            block(border, 0.5);
        }
    }
    for (other_index, other) in scene.relations.iter().enumerate() {
        if other_index == index {
            continue;
        }
        let ink =
            other.authored.facts["width"]
                .as_f64()
                .unwrap_or(if other.variant == "emphasis" {
                    1.8
                } else {
                    1.5
                })
                / 2.0;
        for pair in other.points.windows(2) {
            block(
                Rect {
                    x: pair[0].x.min(pair[1].x),
                    y: pair[0].y.min(pair[1].y),
                    width: (pair[1].x - pair[0].x).abs(),
                    height: (pair[1].y - pair[0].y).abs(),
                },
                ink,
            );
        }
    }
    forbidden.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut cursor = minimum;
    let mut clear = Vec::new();
    for (a, b) in forbidden {
        if b <= cursor {
            continue;
        }
        if a >= maximum {
            break;
        }
        if a > cursor {
            clear.push((cursor, a));
        }
        cursor = cursor.max(b);
        if cursor >= maximum {
            break;
        }
    }
    if cursor < maximum {
        clear.push((cursor, maximum));
    }
    clear.into_iter().rev().find_map(|(a, b)| {
        // The emitted SVG uses the existing fixed numeric precision. Recheck
        // that exact reference point rather than relying on unprinted digits.
        let reference = number(9.0 * stroke - (a + b) / 2.0).parse::<f64>().ok()?;
        let offset = 9.0 * stroke - reference;
        (offset > a && offset < b).then_some(reference)
    })
}

#[derive(Clone)]
struct DeltaMark {
    state: String,
    classifications: Vec<String>,
    origin: Option<&'static str>,
}
struct DeltaLayer {
    marks: std::collections::BTreeMap<String, DeltaMark>,
    ghost: bool,
}
struct DeltaLayers {
    head: DeltaLayer,
    ghost: DeltaLayer,
    ghost_scene: CompiledDiagram,
}
fn boundary_identity(container: &crate::scene::Container) -> String {
    format!(
        "{}:{}",
        container.authored.kind,
        container.authored.facts["label"]
            .as_str()
            .unwrap_or_default()
    )
}
impl DeltaLayer {
    fn mark(&self, kind: &str, id: &str) -> Option<&DeltaMark> {
        self.marks.get(&format!("{kind}:{id}"))
    }
    fn attributes(&self, kind: &str, id: &str) -> String {
        let mark = self.mark(kind, id);
        let state = mark.map(|mark| mark.state.as_str()).unwrap_or("same");
        let classifications = mark
            .map(|mark| mark.classifications.join(" "))
            .unwrap_or_default();
        let mut attrs=format!(" data-delta-kind=\"{}\" data-delta-key=\"{}:{}\" data-delta-state=\"{}\" data-delta-classifications=\"{}\"",escape_text(kind),escape_text(kind),escape_text(id),escape_text(state),escape_text(&classifications));
        if self.ghost {
            attrs.push_str(" data-delta-ghost=\"true\"");
        }
        if let Some(origin) = mark.and_then(|mark| mark.origin) {
            write!(attrs, " data-delta-origin-state=\"{origin}\"").unwrap();
        }
        attrs
    }
}
impl DeltaLayers {
    fn new(base: &CompiledDiagram, ir: &Value) -> Self {
        let mut head = DeltaLayer {
            marks: Default::default(),
            ghost: false,
        };
        let mut ghost = DeltaLayer {
            marks: Default::default(),
            ghost: true,
        };
        for (collection, kind) in [
            ("components", "component"),
            ("connections", "relationship"),
            ("boundaries", "boundary"),
        ] {
            for change in ir["changes"][collection].as_array().into_iter().flatten() {
                let id = change[if kind == "boundary" { "key" } else { "id" }]
                    .as_str()
                    .unwrap_or_default();
                let state = change["status"].as_str().unwrap_or("same");
                let classifications: Vec<_> = change["classifications"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                let geometry = classifications.iter().any(|value| value == "geometry");
                let topology = classifications.iter().any(|value| value == "topology");
                let origin = if state == "removed" || (kind == "relationship" && topology) {
                    Some("removed")
                } else if (kind == "component" && geometry)
                    || (kind == "relationship" && geometry)
                    || (kind == "boundary" && matches!(state, "changed" | "geometry-changed"))
                {
                    Some("moved-from")
                } else {
                    None
                };
                let mark = DeltaMark {
                    state: state.into(),
                    classifications,
                    origin: None,
                };
                head.marks.insert(format!("{kind}:{id}"), mark.clone());
                if let Some(origin) = origin {
                    ghost.marks.insert(
                        format!("{kind}:{id}"),
                        DeltaMark {
                            origin: Some(origin),
                            ..mark
                        },
                    );
                }
            }
        }
        let ghost_scene = base.clone();
        Self {
            head,
            ghost,
            ghost_scene,
        }
    }
}
fn comparison_edge_attributes(
    relation: &crate::scene::Relation,
    index: usize,
    delta: Option<&DeltaLayer>,
) -> String {
    let mut attrs = if delta.is_some_and(|layer| layer.ghost) {
        format!("data-delta-edge-from=\"{}\" data-delta-edge-to=\"{}\" data-delta-edge-label=\"{}\" data-delta-edge-key=\"{index}\" data-delta-edge-id=\"{}\"",escape_text(&relation.from),escape_text(&relation.to),escape_text(&relation.label),escape_text(&relation.authored.id))
    } else {
        edge_attributes(
            &relation.from,
            &relation.to,
            &relation.label,
            index,
            &relation.authored.facts,
        )
    };
    if let Some(delta) = delta {
        attrs.push_str(&delta.attributes("relationship", &relation.authored.id));
    }
    attrs
}
fn delta_symbol(state: &str) -> &'static str {
    match state {
        "added" => "+",
        "removed" => "−",
        "changed" => "~",
        "moved" | "moved-from" | "rerouted" | "geometry-changed" => "↔",
        "evidence-changed" => "E",
        _ => "",
    }
}
fn delta_node_marker(node: &crate::scene::Node, delta: &DeltaLayer, svg: &mut String) {
    let Some(mark) = delta.mark("component", &node.authored.id) else {
        return;
    };
    let symbol = delta_symbol(mark.origin.unwrap_or(&mark.state));
    if symbol.is_empty() {
        return;
    }
    write!(svg,"<g class=\"delta-node-marker\" aria-hidden=\"true\" transform=\"translate({} {})\"><circle r=\"8\"/><text y=\"3\" text-anchor=\"middle\">{symbol}</text></g>",number(node.bounds.x+node.bounds.width-9.0),number(node.bounds.y+9.0)).unwrap();
}
fn write_delta_markers(scene: &CompiledDiagram, delta: &DeltaLayer, svg: &mut String) {
    for relation in &scene.relations {
        let Some(mark) = delta.mark("relationship", &relation.authored.id) else {
            continue;
        };
        let Some(point) = relation.points.first() else {
            continue;
        };
        let visual = mark.origin.unwrap_or(
            if mark.state == "changed" && mark.classifications.iter().any(|v| v == "topology") {
                "added"
            } else {
                &mark.state
            },
        );
        write!(
            svg,
            "<text class=\"delta-edge-marker\"{} x=\"{}\" y=\"{}\" aria-hidden=\"true\">{}</text>",
            delta.attributes("relationship", &relation.authored.id),
            number(point.x + 9.0),
            number(point.y - 7.0),
            delta_symbol(visual)
        )
        .unwrap();
    }
    for container in &scene.containers {
        let id = boundary_identity(container);
        let Some(mark) = delta.mark("boundary", &id) else {
            continue;
        };
        if delta.ghost && mark.origin != Some("removed") {
            continue;
        }
        write!(svg,"<text class=\"delta-boundary-marker\"{} x=\"{}\" y=\"{}\" text-anchor=\"middle\" aria-hidden=\"true\">{}</text>",delta.attributes("boundary",&id),number(container.bounds.x+container.bounds.width-12.0),number(container.bounds.y+16.0),delta_symbol(mark.origin.unwrap_or(&mark.state))).unwrap();
    }
}

// Change only generated SVG attribute values. Authored text may itself contain
// strings such as `url(#grid)` and must remain literal.
fn namespace_svg(svg: &str, prefix: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        if rest.starts_with("<!--") {
            let end = rest.find("-->").map(|end| end + 3).unwrap_or(rest.len());
            result.push_str(&rest[..end]);
            rest = &rest[end..];
            continue;
        }
        let mut quote = 0u8;
        let mut end = rest.len();
        for (index, byte) in rest.bytes().enumerate() {
            if quote != 0 {
                if byte == quote {
                    quote = 0;
                }
            } else if matches!(byte, b'\'' | b'"') {
                quote = byte;
            } else if byte == b'>' {
                end = index + 1;
                break;
            }
        }
        let tag = &rest[..end];
        let bytes = tag.as_bytes();
        let mut at = 1;
        while at < bytes.len() && !bytes[at].is_ascii_whitespace() && bytes[at] != b'>' {
            at += 1;
        }
        let mut cursor = 0;
        while at < bytes.len() {
            while at < bytes.len()
                && (bytes[at].is_ascii_whitespace() || matches!(bytes[at], b'/' | b'>'))
            {
                at += 1;
            }
            let name_start = at;
            while at < bytes.len()
                && !bytes[at].is_ascii_whitespace()
                && !matches!(bytes[at], b'=' | b'/' | b'>')
            {
                at += 1;
            }
            if at == name_start {
                break;
            }
            let name = &tag[name_start..at];
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            if bytes.get(at) != Some(&b'=') {
                continue;
            }
            at += 1;
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            let Some(&quote) = bytes
                .get(at)
                .filter(|quote| matches!(**quote, b'\'' | b'"'))
            else {
                break;
            };
            at += 1;
            let value_start = at;
            while at < bytes.len() && bytes[at] != quote {
                at += 1;
            }
            let value = &tag[value_start..at];
            let replacement = match name {
                "id" => Some(format!("{prefix}-{value}")),
                "href" | "xlink:href" if value.starts_with('#') => {
                    Some(format!("#{prefix}-{}", &value[1..]))
                }
                "aria-labelledby" => Some(
                    value
                        .split_ascii_whitespace()
                        .map(|id| format!("{prefix}-{id}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                "fill" | "stroke" | "marker-start" | "marker-mid" | "marker-end" | "filter"
                | "clip-path" | "mask" | "style" => {
                    Some(value.replace("url(#", &format!("url(#{prefix}-")))
                }
                _ => None,
            };
            if let Some(replacement) = replacement {
                result.push_str(&tag[cursor..value_start]);
                result.push_str(&replacement);
                cursor = at;
            }
            at += 1;
        }
        result.push_str(&tag[cursor..]);
        rest = &rest[end..];
    }
    result.push_str(rest);
    result
}

pub(crate) fn render_comparison_delta(
    base: &CompiledDiagram,
    head: &CompiledDiagram,
    ir: &Value,
    evidence: &SourceEvidence,
) -> Result<Vec<u8>, Diagnostics> {
    validate_geometry(base)?;
    validate_geometry(head)?;
    let mut scene = head.clone();
    scene.view_box.x = base.view_box.x.min(head.view_box.x);
    scene.view_box.y = base.view_box.y.min(head.view_box.y);
    scene.view_box.width = (base.view_box.x + base.view_box.width)
        .max(head.view_box.x + head.view_box.width)
        - scene.view_box.x
        + 24.0;
    scene.view_box.height = (base.view_box.y + base.view_box.height)
        .max(head.view_box.y + head.view_box.height)
        - scene.view_box.y
        + 24.0;
    let layers = DeltaLayers::new(base, ir);
    let template = EmbeddedAssets::builtin()
        .template
        .replace("</head>", &format!("<style>{DELTA_STYLE}</style></head>"));
    render_with_layers(
        &scene,
        &EmbeddedAssets {
            template: &template,
        },
        evidence,
        Some(&layers),
    )
}

const DELTA_STYLE: &str = r#"
:root{--delta-add:#34d399;--delta-remove:#fb7185;--delta-change:#fbbf24;--delta-move:#7dd3fc}
html[data-theme="light"]{--delta-add:#08784b;--delta-remove:#b82c49;--delta-change:#966000;--delta-move:#006c91}
[data-delta-ghost="true"]{pointer-events:none;opacity:.55}
[data-delta-kind="component"][data-delta-state="added"]>[data-graph-role="node-body"],path[data-delta-state="added"],rect[data-delta-kind="boundary"][data-delta-state="added"]{stroke:var(--delta-add)!important;stroke-width:3!important}
[data-delta-kind="component"][data-delta-state="changed"]>[data-graph-role="node-body"],path[data-delta-state="changed"],rect[data-delta-kind="boundary"][data-delta-state="changed"]{stroke:var(--delta-change)!important;stroke-width:3!important;stroke-dasharray:2 3!important}
[data-delta-kind="component"][data-delta-state="evidence-changed"]>[data-graph-role="node-body"]{stroke:var(--delta-change)!important;stroke-width:3!important}
[data-delta-kind="component"][data-delta-state="moved"]>[data-graph-role="node-body"],path[data-delta-state="rerouted"],rect[data-delta-kind="boundary"][data-delta-state="geometry-changed"]{stroke:var(--delta-move)!important;stroke-width:2.5!important;stroke-dasharray:8 3 2 3!important}
[data-delta-kind="component"][data-delta-origin-state="removed"]>[data-graph-role="node-body"],path[data-delta-origin-state="removed"],rect[data-delta-origin-state="removed"]{stroke:var(--delta-remove)!important;stroke-width:3!important;stroke-dasharray:7 5!important}
[data-delta-kind="component"][data-delta-origin-state="moved-from"]>[data-graph-role="node-body"],path[data-delta-origin-state="moved-from"],rect[data-delta-origin-state="moved-from"]{stroke:var(--delta-move)!important;stroke-width:2.5!important;stroke-dasharray:8 3 2 3!important}
.delta-node-marker circle{fill:var(--mask);stroke:var(--delta-move);stroke-width:1.5}.delta-node-marker text,.delta-edge-marker,.delta-boundary-marker{fill:var(--text);font:800 9px ui-monospace,monospace;paint-order:stroke;stroke:var(--mask);stroke-width:2px}
[data-delta-review-current]{filter:drop-shadow(0 0 4px var(--delta-move))}
"#;

fn render_svg(scene: &CompiledDiagram) -> String {
    render_svg_layers(scene, None)
}

fn render_svg_layers(scene: &CompiledDiagram, layers: Option<&DeltaLayers>) -> String {
    let mut svg = String::new();
    let profile = match scene.quality {
        crate::input::Quality::Standard => "standard",
        crate::input::Quality::Showcase => "showcase",
    };
    let contract = if scene.diagram_type == DiagramType::Workflow && scene.schema_version == 2 {
        "readable-v2"
    } else {
        "fixed-v1"
    };
    let input_contract = match scene.input_contract {
        crate::input::InputContract::Legacy217 => "legacy-2.17",
        crate::input::InputContract::PhoenixV1 => "phoenix-v1",
    };
    write!(svg,"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" role=\"img\" aria-labelledby=\"phoenix-diagram-title phoenix-diagram-description\" lang=\"{}\" data-diagram-type=\"{}\" data-layout-contract=\"{contract}\" data-quality-profile=\"{profile}\" data-reader-fit=\"{}\" data-animation=\"trace\" data-preset=\"{}\">",
        number(scene.view_box.x),number(scene.view_box.y),number(scene.view_box.width),number(scene.view_box.height),escape_text(&crate::i18n::resolve(&scene.meta).resolved),scene.diagram_type.as_str(),escape_text(&scene.reader_fit),scene.meta["visual_preset"].as_str().map(escape_text).unwrap_or_else(||"classic".into())).unwrap();
    // Append metadata to the root tag before any authored content is inserted.
    svg.pop();
    write!(svg, " data-input-contract=\"{input_contract}\"").unwrap();
    let node_ui = scene.input_contract == crate::input::InputContract::PhoenixV1
        && matches!(
            scene.diagram_type,
            DiagramType::Architecture | DiagramType::Workflow
        );
    if node_ui {
        svg.push_str(" data-node-ui=\"\"");
    }
    if scene.input_contract == crate::input::InputContract::PhoenixV1 {
        match scene.diagram_type {
            DiagramType::Architecture if scene.meta.get("viewBox").is_none() => {
                svg.push_str(" data-reader-min-text=\"7.5\" data-reader-primary-text=\"14\"")
            }
            DiagramType::Sequence => svg.push_str(" data-motion-choreography=\"sequence\""),
            DiagramType::Lifecycle => svg.push_str(
                " data-motion-choreography=\"walk\" data-lifecycle-layout=\"main-path-v3\"",
            ),
            _ => {}
        }
    }
    let extended = type_renderer(scene.diagram_type);
    if let Some(renderer) = &extended {
        svg.push_str(&(renderer.root_attributes)(scene));
    }
    svg.push('>');
    write!(svg,"<title id=\"phoenix-diagram-title\">{}</title><desc id=\"phoenix-diagram-description\">{}</desc>",attribute(&scene.meta,"title"),attribute(&scene.meta,"subtitle")).unwrap();
    svg.push_str(include_str!("../assets/svg-definitions.svg"));
    if node_ui {
        svg.push_str(include_str!("../assets/node-card-style.svg"));
    }
    if let Some(renderer) = &extended {
        svg.push_str(&(renderer.defs)(scene));
    }
    if layers.is_some() {
        svg.push_str(&namespace_svg(
            include_str!("../assets/svg-definitions.svg"),
            "base",
        ));
    }
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"url(#grid)\"/>");
    if let Some(renderer) = &extended {
        (renderer.body)(scene, &mut svg);
        write_legends(scene, &mut svg, Some(renderer.legend_glyph));
        return svg;
    }
    if let Some(layers) = layers {
        let mut fragment = String::new();

        write_container_frames(&layers.ghost_scene, &mut fragment, Some(&layers.ghost));
        svg.push_str(&namespace_svg(&fragment, "base"));
    }
    write_container_frames(scene, &mut svg, layers.map(|layers| &layers.head));
    if let Some(layers) = layers {
        let mut fragment = String::new();
        write_relation_labels(&layers.ghost_scene, &mut fragment, Some(&layers.ghost));
        write_relation_paths(&layers.ghost_scene, &mut fragment, Some(&layers.ghost));
        svg.push_str(&namespace_svg(&fragment, "base"));
    }
    write_relation_paths(scene, &mut svg, layers.map(|layers| &layers.head));
    if let DiagramFacts::Lifecycle { presentation, .. } = &scene.facts {
        for point in &presentation.junctions {
            write!(svg,"<circle cx=\"{}\" cy=\"{}\" r=\"2.6\" data-lifecycle-junction=\"\" style=\"fill: var(--arrow)\" aria-hidden=\"true\"/>",number(point.x),number(point.y)).unwrap();
        }
    }
    if let Some(layers) = layers {
        let mut fragment = String::new();
        write_container_labels(&layers.ghost_scene, &mut fragment, Some(&layers.ghost));
        write_nodes(&layers.ghost_scene, &mut fragment, Some(&layers.ghost));
        svg.push_str(&namespace_svg(&fragment, "base"));
    }
    write_nodes(scene, &mut svg, layers.map(|layers| &layers.head));
    write_container_labels(scene, &mut svg, layers.map(|layers| &layers.head));
    write_relation_labels(scene, &mut svg, layers.map(|layers| &layers.head));
    for text in &scene.text {
        svg.push_str(&text_markup(text, "t-muted", ""));
    }
    if let Some(layers) = layers {
        let mut fragment = String::new();
        write_delta_markers(&layers.ghost_scene, &layers.ghost, &mut fragment);
        svg.push_str(&namespace_svg(&fragment, "base"));
        write_delta_markers(scene, &layers.head, &mut svg);
    }
    write_legends(scene, &mut svg, None);
    svg
}

fn write_container_frames(scene: &CompiledDiagram, svg: &mut String, delta: Option<&DeltaLayer>) {
    for container in &scene.containers {
        if delta
            .is_some_and(|d| d.ghost && d.mark("boundary", &boundary_identity(container)).is_none())
        {
            continue;
        }
        let delta_attrs = delta
            .map(|d| d.attributes("boundary", &boundary_identity(container)))
            .unwrap_or_default();
        let kind = escape_text(&container.authored.kind);
        let class = match container.authored.kind.as_str() {
            "region" => "c-region",
            "security-group" | "exception-lane" => "c-security-group",
            _ => "c-lane",
        };
        if let Some([start, end]) = container.rule {
            let (width, dash, marker) = match container.authored.kind.as_str() {
                "lifeline" => (0.8, "3,7", ""),
                "lifecycle-band" => (0.8, "3,8", ""),
                "lifecycle-rail" => (2.2, "", " marker-end=\"url(#arrowhead-emphasis)\""),
                _ => (1.0, "", ""),
            };
            let class = if container.authored.kind == "lifecycle-rail" {
                "a-emphasis"
            } else if container.authored.kind == "phase" {
                "a-default phase-rule"
            } else {
                "a-default"
            };
            write!(svg,"<line data-graph-role=\"reading-guide\" data-composition-frame-id=\"{}\" data-composition-frame-kind=\"{kind}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"{class}\" stroke-width=\"{}\"{}",escape_text(&container.authored.id),number(start.x),number(start.y),number(end.x),number(end.y),number(width),marker).unwrap();
            if !dash.is_empty() {
                write!(svg, " stroke-dasharray=\"{dash}\"").unwrap();
            }
            svg.push_str("/>");
        } else if container.authored.kind == "activation" {
            let participant = scene
                .nodes
                .iter()
                .find(|n| container.members.contains(&n.authored.id));
            let fill = container.authored.facts["type"]
                .as_str()
                .or_else(|| participant.and_then(|n| n.authored.facts["type"].as_str()))
                .unwrap_or("external");
            write!(svg,"<rect {} rx=\"3\" class=\"c-mask\"/><rect data-graph-role=\"activation\" {} rx=\"3\" class=\"c-{}\" stroke-width=\"1\"/>",rect_attributes(&container.bounds),rect_attributes(&container.bounds),tone(fill)).unwrap();
        } else if container.authored.kind == "lifecycle-frame" {
            write!(svg,"<rect data-graph-role=\"structural-frame\" data-composition-frame-id=\"{}\" data-composition-frame-kind=\"{kind}\" data-lifecycle-frame=\"\" {} rx=\"14\" fill=\"none\" stroke=\"var(--arrow)\" stroke-opacity=\"0.55\" stroke-width=\"1\" stroke-dasharray=\"1.5 3.5\" stroke-linecap=\"round\" aria-hidden=\"true\"/>",escape_text(&container.authored.id),rect_attributes(&container.bounds)).unwrap();
        } else {
            let radius = match container.authored.kind.as_str() {
                "lane" | "stage" | "segment" => 10.0,
                "region" => 12.0,
                "group" => 9.0,
                _ => 8.0,
            };
            write!(svg,"<rect data-graph-role=\"structural-frame\" data-composition-frame-id=\"{}\" data-composition-frame-kind=\"{kind}\" {} rx=\"{}\" class=\"{class}\" stroke-width=\"1\"{delta_attrs}/>",escape_text(&container.authored.id),rect_attributes(&container.bounds),number(radius)).unwrap();
        }
    }
}

fn write_relation_paths(scene: &CompiledDiagram, svg: &mut String, delta: Option<&DeltaLayer>) {
    for (index, relation) in scene.relations.iter().enumerate() {
        if delta.is_some_and(|d| d.ghost && d.mark("relationship", &relation.authored.id).is_none())
        {
            continue;
        }
        let facts = &relation.authored.facts;
        let attrs = comparison_edge_attributes(relation, index, delta);
        let animation_attr = if delta.is_some_and(|d| d.ghost) {
            "data-delta-animate"
        } else {
            "data-animate"
        };
        let variant = match relation.variant.as_str() {
            "emphasis" => "emphasis",
            "security" => "security",
            "dashed" => "dashed",
            _ => "default",
        };
        let marker = if variant == "default" {
            "arrowhead".into()
        } else {
            format!("arrowhead-{variant}")
        };
        let legacy_lifecycle =
            scene.diagram_type == DiagramType::Lifecycle && scene.schema_version == 1;
        let width = facts["width"]
            .as_f64()
            .filter(|v| !legacy_lifecycle || *v != 0.0)
            .unwrap_or(if variant == "emphasis" {
                if legacy_lifecycle {
                    2.0
                } else {
                    1.8
                }
            } else if scene.diagram_type == DiagramType::Lifecycle {
                if legacy_lifecycle {
                    1.1
                } else {
                    1.2
                }
            } else if scene.diagram_type == DiagramType::Architecture {
                1.5
            } else {
                1.4
            });
        let direction_cue = architecture_direction_cue_ref_x(scene, index, width);
        if let Some(reference) = direction_cue {
            let stroke = number(width).parse::<f64>().expect("finite stroke width");
            write!(svg,"<defs><marker id=\"phoenix-direction-cue-{index}\" markerUnits=\"userSpaceOnUse\" markerWidth=\"{}\" markerHeight=\"{}\" refX=\"{}\" refY=\"{}\" orient=\"auto\"><polygon points=\"0 0,{} {},0 {}\" class=\"m-{variant}\"/></marker></defs>",number(stroke*10.0),number(stroke*7.0),number(reference),number(stroke*3.5),number(stroke*10.0),number(stroke*3.5),number(stroke*7.0)).unwrap();
        }
        let route = if scene.diagram_type == DiagramType::Workflow && scene.schema_version == 2 {
            workflow_v2_path(&relation.points)
        } else if scene.diagram_type == DiagramType::Architecture {
            rounded_path(&relation.points, 8.0)
        } else if scene.diagram_type == DiagramType::Lifecycle {
            if legacy_lifecycle {
                rounded_path(
                    &relation.points,
                    facts["cornerRadius"].as_f64().unwrap_or(10.0),
                )
            } else {
                lifecycle_v3_path(&relation.points)
            }
        } else {
            polyline_path(&relation.points)
        };
        let points = relation
            .points
            .iter()
            .map(|point| format!("{},{}", number(point.x), number(point.y)))
            .collect::<Vec<_>>()
            .join(";");
        if relation.crossover_halo {
            write!(svg,"<g data-graph-role=\"automatic-crossover\"><path data-graph-role=\"automatic-crossover-underlay\" d=\"{route}\" fill=\"none\" stroke=\"var(--mask)\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" pointer-events=\"none\"/>",number(width+4.0)).unwrap();
        }
        write!(svg,"<path {attrs} data-graph-role=\"edge\" data-composition-points=\"{points}\" d=\"{route}\" class=\"a-{variant}\" {animation_attr}=\"edge\" style=\"--step:{}\" stroke-width=\"{}\" marker-end=\"url(#{marker})\"",index.min(12),number(width)).unwrap();
        if direction_cue.is_some() {
            write!(svg, " marker-start=\"url(#phoenix-direction-cue-{index})\"").unwrap();
        }
        if scene.diagram_type == DiagramType::Sequence && relation.variant == "return" {
            svg.push_str(" stroke-dasharray=\"3,5\"");
        }
        if let Some(junction) = &relation.junction {
            write!(
                svg,
                " data-composition-junction=\"{}\"",
                escape_text(&junction.replace('\0', "+"))
            )
            .unwrap();
        }
        if relation.crossover_halo {
            svg.push_str(" data-composition-crossover=\"halo\"");
            if relation.independent_ports {
                svg.push_str(" data-composition-independent=\"true\"");
            }
        }
        for key in ["fromSide", "toSide", "route", "role"] {
            if let Some(value) = facts[key].as_str() {
                write!(svg, " data-authored-{key}=\"{}\"", escape_text(value)).unwrap();
            }
        }
        if facts["via"].is_array() {
            svg.push_str(" data-authored-via=\"true\"");
        }
        if facts["route"] == "straight"
            && relation.points.len() == 2
            && facts["via"].as_array().is_none_or(Vec::is_empty)
        {
            svg.push_str(" data-composition-route=\"straight\"");
        }
        if scene.diagram_type == DiagramType::Workflow
            && scene.schema_version == 2
            && facts["via"].is_null()
            && facts["route"].as_str().is_none_or(|route| route == "auto")
        {
            svg.push_str(" data-composition-routing=\"workflow-v2-auto\"");
        }
        svg.push_str("/>");
        if relation.crossover_halo {
            svg.push_str("</g>");
        }
    }
}

fn write_nodes(scene: &CompiledDiagram, svg: &mut String, delta: Option<&DeltaLayer>) {
    for (index, node) in scene.nodes.iter().enumerate() {
        if delta.is_some_and(|d| d.ghost && d.mark("component", &node.authored.id).is_none()) {
            continue;
        }
        let kind = node.authored.facts["type"]
            .as_str()
            .unwrap_or(&node.authored.kind);
        let node_attr = if delta.is_some_and(|d| d.ghost) {
            "data-delta-node-id"
        } else {
            "data-node-id"
        };
        let role = if delta.is_some_and(|d| d.ghost) {
            "group"
        } else {
            "button"
        };
        let tab = if delta.is_some_and(|d| d.ghost) {
            "-1"
        } else {
            "0"
        };
        write!(svg,"<g id=\"node-{}\" {node_attr}=\"{}\" data-node-label=\"{}\" data-node-kind=\"{}\" tabindex=\"{tab}\" role=\"{role}\" aria-label=\"{}\" aria-pressed=\"false\" data-source-path=\"{}\"",escape_text(&node.authored.id),escape_text(&node.authored.id),escape_text(&node.label),escape_text(kind),escape_text(&node.label),escape_text(&node.authored.source_path)).unwrap();
        for field in ["sublabel", "tag"] {
            if let Some(value) = node.authored.facts[field]
                .as_str()
                .filter(|v| !v.trim().is_empty())
            {
                write!(svg, " data-node-{field}=\"{}\"", escape_text(value)).unwrap();
            }
        }
        if let Some(delta) = delta {
            svg.push_str(&delta.attributes("component", &node.authored.id));
        }
        svg.push('>');
        let radius = match scene.diagram_type {
            DiagramType::Sequence | DiagramType::Dataflow => 6.0,
            DiagramType::Lifecycle if scene.schema_version == 1 => 7.0,
            _ => 8.0,
        };
        let fill = node_tone(scene, kind);
        let animation_attr = if delta.is_some_and(|d| d.ghost) {
            "data-delta-animate"
        } else {
            "data-animate"
        };
        write!(svg,"<title>{}</title><rect {} rx=\"{}\" class=\"c-mask\"/><rect data-graph-role=\"node-body\" {} rx=\"{}\" class=\"c-{fill}\" stroke-width=\"1.5\" {animation_attr}=\"node\" style=\"--step:{}\"/>",escape_text(&node.label),rect_attributes(&node.bounds),number(radius),rect_attributes(&node.bounds),number(radius),index.min(12)).unwrap();
        if let DiagramFacts::Lifecycle { presentation, .. } = &scene.facts {
            if presentation.final_states.contains(&node.authored.id) {
                let inner = Rect {
                    x: node.bounds.x + 3.0,
                    y: node.bounds.y + 3.0,
                    width: node.bounds.width - 6.0,
                    height: node.bounds.height - 6.0,
                };
                write!(svg,"<rect {} rx=\"5\" class=\"c-{fill}\" style=\"fill: none\" stroke-width=\"1\"/>",rect_attributes(&inner)).unwrap();
            }
            if presentation.initial_state.as_ref() == Some(&node.authored.id) {
                svg.push_str("<g aria-hidden=\"true\" data-lifecycle-initial-marker=\"\">");
                initial_marker(
                    svg,
                    node.bounds.x - 26.0,
                    node.bounds.x - 1.0,
                    node.bounds.y + node.bounds.height / 2.0,
                );
                svg.push_str("</g>");
            }
        }
        if let Some(sigil) = &node.sigil {
            let (selected, shape) = sigil_shape(&sigil.kind);
            write!(svg,"<g aria-hidden=\"true\" data-semantic-sigil=\"{selected}\" class=\"semantic-sigil s-{}\" transform=\"translate({} {}) scale({})\">{shape}</g>",tone(kind),number(sigil.position.x),number(sigil.position.y),number(sigil.size/16.0)).unwrap();
        }
        if let Some(brand) = &node.brand {
            let r = &brand.resource;
            write!(svg,"<g aria-hidden=\"true\" data-brand-mark=\"{}\" data-brand-title=\"{}\" data-brand-source=\"{}\" data-brand-sha256=\"{}\" transform=\"translate({} {})\"><rect width=\"16\" height=\"16\" rx=\"4\" fill=\"#fff\" stroke=\"#cbd5e1\"/>",escape_text(&r.resource.id),escape_text(&r.title),escape_text(&r.resource.source_url),escape_text(&r.resource.sha256),number(brand.bounds.x),number(brand.bounds.y)).unwrap();
            if let Some(mark) = &r.mark {
                write!(
                    svg,
                    "<path d=\"{}\" fill=\"#{}\" transform=\"translate(2 2) scale({})\"/>",
                    escape_text(&mark.path),
                    escape_text(&mark.hex),
                    number(12.0 / mark.view_box)
                )
                .unwrap();
            }
            if let Some(data) = &r.data_base64 {
                write!(svg,"<image href=\"data:{};base64,{}\" x=\"2\" y=\"2\" width=\"12\" height=\"12\" preserveAspectRatio=\"xMidYMid meet\"/>",escape_text(&r.resource.media_type),escape_text(data)).unwrap();
            }
            svg.push_str("</g>");
        }
        for text in &node.text {
            let primary = text.source_path.ends_with("/label");
            let fine = text.source_path.ends_with("/tag") || text.source_path.ends_with("/step");
            let attributes = if primary {
                "data-node-label=\"\" font-weight=\"600\""
            } else if text.source_path.ends_with("/step") {
                "data-detail=\"fine\" font-weight=\"700\""
            } else if fine {
                "data-detail=\"fine\""
            } else {
                "data-detail=\"context\""
            };
            svg.push_str(&text_markup(
                text,
                &if primary {
                    "t-primary".into()
                } else if fine && !matches!(kind, "neutral" | "external") {
                    format!("t-{fill}")
                } else {
                    "t-muted".into()
                },
                attributes,
            ));
        }
        if let Some(delta) = delta {
            delta_node_marker(node, delta, svg);
        }
        svg.push_str("</g>");
    }
}

fn write_container_labels(scene: &CompiledDiagram, svg: &mut String, delta: Option<&DeltaLayer>) {
    for container in &scene.containers {
        if delta
            .is_some_and(|d| d.ghost && d.mark("boundary", &boundary_identity(container)).is_none())
        {
            continue;
        }
        write!(svg,"<g data-graph-role=\"structural-frame-label\" data-composition-frame-id=\"{}\" data-composition-frame-kind=\"{}\"{}>",escape_text(&container.authored.id),escape_text(&container.authored.kind),delta.map(|d|d.attributes("boundary", &boundary_identity(container))).unwrap_or_default()).unwrap();
        if let Some(plate) = container.label_plate {
            let class = if container.rule.is_some() {
                "c-mask phase-chip"
            } else {
                "c-mask"
            };
            let radius = if container.authored.kind == "phase" {
                8.0
            } else {
                3.0
            };
            write!(svg,"<rect data-graph-role=\"structural-frame-label-mask\" {} class=\"{class}\" rx=\"{}\"/>",rect_attributes(&plate),number(radius)).unwrap();
        }
        for text in &container.text {
            svg.push_str(&text_markup(text, "t-muted", ""));
        }
        svg.push_str("</g>");
    }
}

fn write_relation_labels(scene: &CompiledDiagram, svg: &mut String, delta: Option<&DeltaLayer>) {
    for (index, relation) in scene.relations.iter().enumerate() {
        if delta.is_some_and(|d| d.ghost && d.mark("relationship", &relation.authored.id).is_none())
        {
            continue;
        }
        let attrs = comparison_edge_attributes(relation, index, delta);
        write!(
            svg,
            "<g data-detail=\"{}\" {attrs}>",
            if scene.diagram_type == DiagramType::Lifecycle && relation.label.is_empty() {
                "fine"
            } else {
                "context"
            }
        )
        .unwrap();
        if let Some(plate) = relation.label_plate {
            write!(
                svg,
                "<rect {} rx=\"{}\" class=\"c-mask\"/>",
                rect_attributes(&plate),
                if matches!(
                    scene.diagram_type,
                    DiagramType::Architecture | DiagramType::Sequence | DiagramType::Workflow
                ) {
                    "3"
                } else {
                    "4"
                }
            )
            .unwrap();
        }
        if let Some(backdrop) = relation.note_backdrop {
            write!(svg,"<rect data-detail=\"context\" data-graph-role=\"message-note-background\" {} rx=\"2\" class=\"c-mask\" pointer-events=\"none\"/>",rect_attributes(&backdrop)).unwrap();
        }
        for text in &relation.text {
            let fine =
                scene.diagram_type == DiagramType::Lifecycle && text.source_path.ends_with("/note");
            let class = if text.source_path.ends_with("/label") {
                edge_label_class(scene, &relation.variant)
            } else if fine && scene.schema_version == 1 {
                "t-dim"
            } else {
                "t-muted"
            };
            svg.push_str(&text_markup(
                text,
                class,
                if fine { "data-detail=\"fine\"" } else { "" },
            ));
        }
        svg.push_str("</g>");
    }
}

fn write_legends(scene: &CompiledDiagram, svg: &mut String, glyph: Option<LegendGlyph>) {
    svg.push_str("<!-- Legend -->");
    svg.push_str("<g data-legend=\"\" data-legend-bridge=\"\">");
    for legend in &scene.legends {
        let kind = if scene.diagram_type == DiagramType::Lifecycle
            && legend.authored.kind != "legend-title"
        {
            legend
                .authored
                .kind
                .strip_prefix("legend-")
                .unwrap_or(&legend.authored.kind)
        } else {
            &legend.authored.kind
        };
        let baseline = legend
            .text
            .first()
            .map_or(legend.bounds.y + 10.0, |t| t.position.y);
        let arrow_legend = matches!(
            scene.diagram_type,
            DiagramType::Sequence | DiagramType::Dataflow
        ) && kind != "database";
        let interactive = !arrow_legend
            && scene.nodes.iter().any(|node| {
                node.authored.facts["type"].as_str() == Some(kind) || node.authored.kind == kind
            });
        write!(svg,"<g data-graph-role=\"legend\" data-legend-semantic-kind=\"{}\" data-legend-x=\"{}\" data-legend-y=\"{}\" data-legend-baseline=\"{}\" data-legend-width=\"{}\" data-legend-height=\"{}\"",escape_text(kind),number(legend.bounds.x),number(legend.bounds.y),number(baseline),number(legend.bounds.width),number(legend.bounds.height)).unwrap();
        if interactive {
            write!(
                svg,
                " data-legend-kind=\"{}\" data-legend-label=\"{}\"",
                escape_text(kind),
                legend
                    .text
                    .first()
                    .map(|text| escape_text(&text.content))
                    .unwrap_or_default()
            )
            .unwrap();
        }
        svg.push('>');
        if kind != "legend-title" {
            if let Some(glyph) = glyph {
                glyph(scene, legend, baseline, svg);
            } else if arrow_legend {
                let variant = if matches!(kind, "emphasis" | "security" | "dashed") {
                    kind
                } else {
                    "default"
                };
                let marker = if variant == "default" {
                    "arrowhead".into()
                } else {
                    format!("arrowhead-{variant}")
                };
                write!(svg,"<path d=\"M {} {} L {} {}\" class=\"a-{variant}\" stroke-width=\"{}\" marker-end=\"url(#{marker})\"{} />",number(legend.bounds.x),number(baseline-3.0),number(legend.bounds.x+34.0),number(baseline-3.0),if variant=="emphasis" {"1.8"} else {"1.4"},if kind=="return" {" stroke-dasharray=\"3,5\""} else {""}).unwrap();
            } else if scene.diagram_type == DiagramType::Lifecycle
                && scene.schema_version == 3
                && kind == "start"
            {
                initial_marker(
                    svg,
                    legend.bounds.x + 4.5,
                    legend.bounds.x + 24.0,
                    baseline - 3.5,
                );
            } else {
                let (width, height, radius, offset) =
                    if scene.diagram_type == DiagramType::Architecture {
                        (16.0, 10.0, 2.5, 9.0)
                    } else {
                        (14.0, 9.0, 2.0, 8.0)
                    };
                write!(svg,"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" class=\"c-{}\" stroke-width=\"1\"/>",number(legend.bounds.x),number(baseline-offset),number(width),number(height),number(radius),node_tone(scene,kind)).unwrap();
                if kind == "final"
                    && scene.diagram_type == DiagramType::Lifecycle
                    && scene.schema_version == 3
                {
                    write!(svg,"<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"4\" rx=\"1\" class=\"c-external\" style=\"fill: none\" stroke-width=\"0.8\"/>",number(legend.bounds.x+2.5),number(baseline-5.5)).unwrap();
                }
            }
        }
        for text in &legend.text {
            svg.push_str(&text_markup(
                text,
                if kind == "legend-title" {
                    "t-primary"
                } else {
                    "t-muted"
                },
                if kind == "legend-title" {
                    "font-weight=\"650\""
                } else {
                    "font-weight=\"500\""
                },
            ));
        }
        svg.push_str("</g>");
    }
    svg.push_str("</g></svg>");
}

pub fn render(
    scene: &CompiledDiagram,
    assets: &EmbeddedAssets<'_>,
) -> Result<Vec<u8>, Diagnostics> {
    render_with_evidence(scene, assets, &SourceEvidence::none())
}

/// The artifact with the source evidence observed for its candidate.
pub fn render_with_evidence(
    scene: &CompiledDiagram,
    assets: &EmbeddedAssets<'_>,
    evidence: &SourceEvidence,
) -> Result<Vec<u8>, Diagnostics> {
    render_with_layers(scene, assets, evidence, None)
}

fn render_with_layers(
    scene: &CompiledDiagram,
    assets: &EmbeddedAssets<'_>,
    evidence: &SourceEvidence,
    layers: Option<&DeltaLayers>,
) -> Result<Vec<u8>, Diagnostics> {
    validate_geometry(scene)?;
    let mut cards = String::from("<div class=\"cards\">");
    for card in &scene.cards {
        write!(cards,"<div class=\"card\"><div class=\"card-header\"><div class=\"card-dot {}\"></div><h3>{}</h3></div><ul>",attribute(&card.authored.facts,"dot"),attribute(&card.authored.facts,"title")).unwrap();
        if let Some(items) = card.authored.facts["items"].as_array() {
            for item in items {
                write!(
                    cards,
                    "<li>{}</li>",
                    item.as_str().map(escape_text).unwrap_or_default()
                )
                .unwrap();
            }
        }
        cards.push_str("</ul></div>");
    }
    cards.push_str("</div>");
    let mut public_metadata = json!({"tool":"phoenix-diagram","meta":scene.meta,"diagramType":scene.diagram_type,"schemaVersion":scene.schema_version,"inputContract":scene.input_contract,"quality":scene.quality,"readerFit":scene.reader_fit,
        "graph":{"nodes":scene.nodes.iter().map(|node|&node.authored.facts).collect::<Vec<_>>(),"relations":scene.relations.iter().map(|relation|&relation.authored.facts).collect::<Vec<_>>()},"cards":scene.cards.iter().map(|card|&card.authored.facts).collect::<Vec<_>>(),"presentation":{"relations":scene.relations.iter().map(|r|json!({"variant":r.variant,"labelOwner":r.label_owner,"junction":r.junction})).collect::<Vec<_>>()},"facts":scene.facts,"sourceEvidence":evidence,"brandResources":scene.nodes.iter().filter_map(|n|n.brand.as_ref().map(|b|(format!("{}/brand",n.authored.source_path),&b.resource))).collect::<std::collections::BTreeMap<_,_>>()});
    if scene.nodes.iter().all(|n| n.brand.is_none()) {
        public_metadata
            .as_object_mut()
            .expect("metadata object")
            .remove("brandResources");
    }
    let metadata = serde_json::to_string(&public_metadata)
        .expect("serializable metadata")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let title = attribute(&scene.meta, "title");
    let subtitle = scene.meta["subtitle"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("<p class=\"subtitle\">{}</p>", escape_text(value)))
        .unwrap_or_default();
    let replacements = [
        (
            "LOCALE",
            escape_text(&crate::i18n::resolve(&scene.meta).resolved),
        ),
        (
            "PRESET",
            scene.meta["visual_preset"]
                .as_str()
                .map(escape_text)
                .unwrap_or_else(|| "classic".into()),
        ),
        ("TITLE", title.clone()),
        ("HEADING", title),
        ("SUBTITLE", subtitle),
        (
            "SVG",
            if layers.is_some() {
                namespace_svg(&render_svg_layers(scene, layers), "delta")
            } else {
                render_svg(scene)
            },
        ),
        ("CARDS", cards),
        ("METADATA", metadata),
    ];
    for (name, _) in &replacements {
        let marker = format!("{{{{PHOENIX:{name}}}}}");
        if assets.template.matches(&marker).count() != 1 {
            return Err(failure(
                scene,
                "render/template",
                "",
                format!("Template must contain exactly one {marker}"),
            ));
        }
    }
    let mut result = String::new();
    let mut remaining = assets.template;
    while let Some(start) = remaining.find("{{PHOENIX:") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + 10..];
        let Some(end) = remaining.find("}}") else {
            return Err(failure(
                scene,
                "render/template",
                "",
                "Unterminated template slot",
            ));
        };
        let name = &remaining[..end];
        let Some((_, value)) = replacements
            .iter()
            .find(|(candidate, _)| *candidate == name)
        else {
            return Err(failure(
                scene,
                "render/template",
                "",
                format!("Unknown template slot {name}"),
            ));
        };
        result.push_str(value);
        remaining = &remaining[end + 2..];
    }
    result.push_str(remaining);
    Ok(result.into_bytes())
}
