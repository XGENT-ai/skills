use super::erd::{edge_attributes, node_attributes, node_title, points_value, text_element};
use super::{escape_text, initial_marker, TypeRenderer};
use crate::geometry::{format_number as number, rounded_path};
use crate::scene::{Card, CompiledDiagram, Container, DiagramFacts, Node, Rect};
use serde_json::Value;
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

const STYLE: &str = concat!(
    "<style>",
    "svg[data-statechart-ui] .sc-parent { fill: var(--lane-fill); stroke: var(--lane-stroke); }",
    "svg[data-statechart-ui] .sc-region { fill: none; stroke: var(--lane-stroke); stroke-dasharray: 5 4; }",
    "svg[data-statechart-ui] .sc-title { fill: var(--mask); stroke: var(--lane-stroke); }",
    "svg[data-statechart-ui] [data-statechart-mark] rect[data-graph-role=node-body] { stroke: var(--security-stroke); stroke-width: 2.5px; }",
    "svg[data-statechart-ui] [data-statechart-mark=\"may-break\"] rect[data-graph-role=node-body] { stroke-dasharray: 6 3; }",
    "svg[data-statechart-ui] [data-statechart-mark=\"breaks\"] [data-node-label], svg[data-statechart-ui] [data-statechart-mark] .sc-tag { fill: var(--security-stroke); }",
    "svg[data-statechart-ui] .sc-bug-ring { fill: none; stroke: var(--arrow-emphasis); stroke-width: 2.5px; }",
    "svg[data-statechart-ui] .sc-legend-mark { fill: var(--mask); stroke: var(--security-stroke); stroke-width: 2px; }",
    "svg[data-statechart-ui] .sc-legend-may { stroke-dasharray: 4 2; }",
    "</style>",
);

fn root_attributes(_scene: &CompiledDiagram) -> String {
    // Chart states are derived from the model, not authored types.
    " data-statechart-ui=\"\" data-node-kind-derived=\"\" data-reader-min-text=\"7.5\"".into()
}

fn defs(_scene: &CompiledDiagram) -> String {
    STYLE.into()
}

fn rect(rect: &Rect) -> String {
    format!(
        "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        number(rect.x),
        number(rect.y),
        number(rect.width),
        number(rect.height)
    )
}

fn states(scene: &CompiledDiagram) -> &[Value] {
    match &scene.facts {
        DiagramFacts::Statechart { states, .. } => states,
        _ => &[],
    }
}

fn write_frame(container: &Container, out: &mut String) {
    let (class, radius) = if container.authored.kind == "statechart-region" {
        ("sc-region", 8)
    } else {
        ("sc-parent", 12)
    };
    write!(out, "<rect data-graph-role=\"structural-frame\" data-composition-frame-id=\"{}\" data-composition-frame-kind=\"{}\" {} rx=\"{radius}\" class=\"{class}\" stroke-width=\"1\"/>",
        escape_text(&container.authored.id), escape_text(&container.authored.kind), rect(&container.bounds)).unwrap();
}

fn write_node(
    locale: &crate::i18n::Locale,
    node: &Node,
    facts: &Value,
    step: usize,
    out: &mut String,
) {
    let parent = node.authored.kind == "parent";
    // The index groups chart states by where they sit; the member count and
    // the rules they break are detail.
    let members = facts["members"].as_u64().unwrap_or(0).to_string();
    let mut detail = locale.text("node.context.statechart.members", &[("count", &members)]);
    let tag = node.authored.facts["tag"].as_str();
    if let Some(tag) = tag {
        detail = format!("{detail} · {tag}");
    }
    let location = node.authored.facts["location"]
        .as_str()
        .filter(|location| !location.is_empty());
    let tone = facts["tone"].as_str().unwrap_or("external");
    write!(
        out,
        "<g {}",
        node_attributes(
            locale,
            &node.authored.id,
            &node.label,
            &node.authored.source_path,
            &[
                ("kind", Some(&node.authored.kind)),
                ("sublabel", Some(&detail)),
                ("tag", tag),
                ("context", location)
            ],
            &[location, Some(&detail)]
        )
    )
    .unwrap();
    if let Some(mark) = facts["mark"].as_str() {
        write!(out, " data-statechart-mark=\"{}\"", escape_text(mark)).unwrap();
    }
    if facts["bugPath"] == true {
        out.push_str(" data-statechart-bug-path=\"\"");
    }
    if facts["start"] == true {
        out.push_str(" data-statechart-start=\"\"");
    }
    out.push('>');
    out.push_str(&node_title(&[Some(&node.label), location, Some(&detail)]));
    let b = node.bounds;
    if facts["bugPath"] == true {
        let ring = Rect {
            x: b.x - 4.0,
            y: b.y - 4.0,
            width: b.width + 8.0,
            height: b.height + 8.0,
        };
        write!(
            out,
            "<rect data-statechart-bug-ring=\"\" {} rx=\"13\" class=\"sc-bug-ring\"/>",
            rect(&ring)
        )
        .unwrap();
    }
    let (radius, class) = if parent {
        (8, "sc-title".to_owned())
    } else {
        (10, format!("c-{tone}"))
    };
    write!(out, "<rect {} rx=\"{radius}\" class=\"c-mask\"/><rect data-graph-role=\"node-body\" {} rx=\"{radius}\" class=\"{class}\" data-animate=\"node\" style=\"--step:{}\" stroke-width=\"1.2\"/>",
        rect(&b), rect(&b), step.min(12)).unwrap();
    for text in &node.text {
        let attributes = if text.source_path.ends_with("/tag") {
            "data-detail=\"context\" class=\"t-muted sc-tag\" font-weight=\"600\"".to_owned()
        } else {
            format!(
                "data-node-label=\"\" class=\"t-primary\" font-weight=\"{}\"",
                if parent { 650 } else { 700 }
            )
        };
        out.push_str(&text_element(
            text,
            &attributes,
            true,
            &escape_text(&text.content),
        ));
    }
    out.push_str("</g>");
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let locale = crate::i18n::resolve(&scene.meta);
    for container in &scene.containers {
        write_frame(container, out);
    }
    for (index, relation) in scene.relations.iter().enumerate() {
        let emphasis = relation.variant == "emphasis";
        let width = if emphasis { 1.8 } else { 1.4 };
        let route = rounded_path(&relation.points, 8.0);
        if relation.crossover_halo {
            write!(out, "<g data-graph-role=\"automatic-crossover\"><path data-graph-role=\"automatic-crossover-underlay\" d=\"{route}\" fill=\"none\" stroke=\"var(--mask)\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" pointer-events=\"none\"/>", number(width + 4.0)).unwrap();
        }
        write!(out, "<path {} data-graph-role=\"edge\" data-composition-points=\"{}\" d=\"{route}\" class=\"{}\" data-animate=\"edge\" style=\"--step:{}\" stroke-width=\"{}\" marker-end=\"url(#{})\"",
            edge_attributes(relation, index), points_value(&relation.points),
            if emphasis { "a-emphasis" } else { "a-default" }, index.min(12), number(width),
            if emphasis { "arrowhead-emphasis" } else { "arrowhead" }).unwrap();
        if relation.crossover_halo {
            out.push_str(" data-composition-crossover=\"halo\"");
            if relation.independent_ports {
                out.push_str(" data-composition-independent=\"true\"");
            }
            out.push_str("/></g>");
        } else {
            out.push_str("/>");
        }
    }
    let facts = states(scene);
    for (order, node) in scene.nodes.iter().enumerate() {
        let fact = facts
            .iter()
            .find(|fact| fact["id"] == node.authored.id.as_str())
            .unwrap_or(&Value::Null);
        write_node(&locale, node, fact, order, out);
        if fact["start"] == true {
            let b = node.bounds;
            initial_marker(out, b.x - 30.0, b.x - 1.0, b.y + b.height / 2.0);
        }
    }
    for container in scene
        .containers
        .iter()
        .filter(|container| container.label_plate.is_some())
    {
        let plate = container.label_plate.unwrap_or_default();
        write!(
            out,
            "<g data-statechart-region=\"{}\"><rect {} rx=\"6\" class=\"c-mask\"/>",
            escape_text(&container.authored.id),
            rect(&plate)
        )
        .unwrap();
        for text in &container.text {
            out.push_str(&text_element(
                text,
                "class=\"t-primary\" font-weight=\"700\"",
                true,
                &escape_text(&text.content),
            ));
        }
        out.push_str("</g>");
    }
    for (index, relation) in scene.relations.iter().enumerate() {
        let (Some(plate), Some(label)) = (relation.label_plate, relation.text.first()) else {
            continue;
        };
        let accent = if relation.variant == "emphasis" {
            "t-edge-emphasis"
        } else {
            "t-muted"
        };
        write!(
            out,
            "<g data-detail=\"context\" {}><rect {} rx=\"3\" class=\"c-mask\"/>",
            edge_attributes(relation, index),
            rect(&plate)
        )
        .unwrap();
        out.push_str(&text_element(
            label,
            &format!("class=\"{accent}\""),
            true,
            &escape_text(&label.content),
        ));
        out.push_str("</g>");
    }
}

fn legend_glyph(_scene: &CompiledDiagram, legend: &Card, baseline: f64, out: &mut String) {
    let x = legend.bounds.x;
    let swatch = |class: &str, out: &mut String| {
        write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"10\" rx=\"2.5\" class=\"{class}\"/>",
            number(x),
            number(baseline - 9.0)
        )
        .unwrap();
    };
    match legend.authored.facts["glyph"].as_str() {
        Some("start") => initial_marker(out, x + 4.5, x + 24.0, baseline - 4.0),
        Some("breaks") => swatch("sc-legend-mark", out),
        Some("may-break") => swatch("sc-legend-mark sc-legend-may", out),
        Some("bug-path") => swatch("c-mask sc-bug-ring", out),
        Some("tone") => {
            let tone = legend.authored.facts["tone"].as_str().unwrap_or("external");
            write!(out, "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"10\" rx=\"2.5\" class=\"c-{}\" stroke-width=\"1\"/>", number(x), number(baseline - 9.0), escape_text(tone)).unwrap();
        }
        _ => {}
    }
}
