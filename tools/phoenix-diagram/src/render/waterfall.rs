use super::{escape_text, TypeRenderer};
use crate::format::number::js_to_string;
use crate::geometry::format_number as number;
use crate::layout::waterfall::BAR_H;
use crate::scene::{Card, CompiledDiagram, Container, DiagramFacts, Node};
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

const SOLID_TONES: [&str; 7] = [
    "frontend",
    "backend",
    "database",
    "cloud",
    "messagebus",
    "security",
    "external",
];

fn unit(scene: &CompiledDiagram) -> &str {
    match &scene.facts {
        DiagramFacts::Waterfall { unit, .. } => unit,
        _ => "ms",
    }
}

fn root_attributes(scene: &CompiledDiagram) -> String {
    format!(
        " data-motion-choreography=\"replay\" data-waterfall-ui=\"\" data-waterfall-unit=\"{}\" data-reader-min-text=\"7.5\"",
        escape_text(unit(scene))
    )
}

fn defs(_scene: &CompiledDiagram) -> String {
    let mut style = String::from(concat!(
        "<style>",
        "svg[data-waterfall-ui] .wf-axis { stroke: var(--text-dim); }",
        "svg[data-waterfall-ui] .wf-grid { stroke: var(--lane-stroke); stroke-dasharray: 2 4; opacity: .7; }",
        "svg[data-waterfall-ui] .wf-grid-origin { stroke-dasharray: none; opacity: .9; }",
        "svg[data-waterfall-ui] .wf-row { fill: transparent; }",
        "svg[data-waterfall-ui] .wf-row-alt { fill: var(--lane-fill); fill-opacity: 1; opacity: .7; }",
        "svg[data-waterfall-ui] .wf-guide { stroke: var(--lane-stroke); }",
        "svg[data-waterfall-ui] .wf-span { fill-opacity: .9; }",
        "svg[data-waterfall-ui] .wf-open { stroke-dasharray: 3 2.5; fill-opacity: .6; }",
        "svg[data-waterfall-ui] .wf-open-edge { fill: none !important; }",
        "svg[data-waterfall-ui] .wf-num { font-variant-numeric: tabular-nums; }",
        "svg[data-waterfall-ui] .wf-caps { letter-spacing: .06em; }",
        "svg[data-waterfall-ui] .wf-solid { stroke: none; }",
        // Thin bars stay crisp: the classic preset's card shadow blurs their edges.
        "svg[data-waterfall-ui] [data-node-id] > rect { filter: none !important; }",
    ));
    for tone in SOLID_TONES {
        write!(style, "svg[data-waterfall-ui] .wf-solid.c-{tone} {{ fill: var(--{tone}-stroke); }}svg[data-waterfall-ui] .wf-open-edge.t-{tone} {{ stroke: var(--{tone}-stroke); }}").unwrap();
    }
    style.push_str("</style>");
    style
}

fn of_kind<'a>(scene: &'a CompiledDiagram, kind: &'a str) -> impl Iterator<Item = &'a Container> {
    scene
        .containers
        .iter()
        .filter(move |container| container.authored.kind == kind)
}

fn write_row(
    locale: &crate::i18n::Locale,
    order: usize,
    node: &Node,
    row: &Container,
    out: &mut String,
) {
    let facts = &row.authored.facts;
    let text = |key: &str| facts[key].as_str().unwrap_or_default();
    let value = |key: &str| js_to_string(facts[key].as_f64().unwrap_or_default());
    let (tone, paint) = (text("tone"), text("paint"));
    let (sublabel, context) = (text("sublabel"), text("context"));
    let detail: Vec<&str> = [sublabel, context]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect();
    let aria = locale.text(
        "node.focus.detail",
        &[("label", &node.label), ("detail", &detail.join(", "))],
    );
    let open = facts["open"] == true;
    let parent_bar = facts["parentBar"] == true;
    let depth = facts["depth"].as_u64().unwrap_or_default();
    write!(out, "<g id=\"node-{0}\" data-node-id=\"{0}\" data-node-label=\"{1}\" tabindex=\"0\" role=\"button\" aria-label=\"{2}\" aria-pressed=\"false\" data-node-kind=\"{3}\" data-node-sublabel=\"{4}\"",
        escape_text(&node.authored.id), escape_text(&node.label), escape_text(&aria), escape_text(tone), escape_text(sublabel)).unwrap();
    if !context.is_empty() {
        write!(out, " data-node-context=\"{}\"", escape_text(context)).unwrap();
    }
    write!(out, " data-source-path=\"{}\" data-waterfall-start=\"{}\" data-waterfall-end=\"{}\" data-waterfall-duration=\"{}\" data-waterfall-depth=\"{depth}\" data-waterfall-status=\"{}\"",
        escape_text(&node.authored.source_path), value("start"), if open { String::new() } else { value("end") }, value("duration"), escape_text(text("status"))).unwrap();
    if let Some(service) = facts["service"].as_str() {
        write!(out, " data-waterfall-service=\"{}\"", escape_text(service)).unwrap();
    }
    let title: Vec<&str> = [node.label.as_str()].into_iter().chain(detail).collect();
    write!(out, "><title>{}</title>", escape_text(&title.join(" · "))).unwrap();
    let r = row.bounds;
    write!(
        out,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" class=\"c-mask wf-row{}\"/>",
        number(r.x),
        number(r.y),
        number(r.width),
        number(r.height),
        if order % 2 == 1 { " wf-row-alt" } else { "" }
    )
    .unwrap();
    let b = node.bounds;
    write!(out, "<rect data-graph-role=\"node-body\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" class=\"c-{tone} {paint}\" data-animate=\"node\" style=\"--step:{}\" stroke-width=\"1.2\"/>",
        number(b.x), number(b.y), number(b.width), number(b.height), number((b.width / 2.0).min(4.0)), order.min(12)).unwrap();
    let Some([dot, end]) = row.rule else {
        out.push_str("</g>");
        return;
    };
    // An open bar ends in a chevron: the trace recorded no end for it.
    if open {
        write!(out, "<path d=\"M {} {} L {} {} L {} {}\" class=\"wf-open-edge t-{tone}\" stroke-width=\"1.6\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
            number(end.x - 8.0), number(b.y + 2.0), number(end.x - 2.0), number(end.y), number(end.x - 8.0), number(b.y + BAR_H - 2.0)).unwrap();
    }
    write!(
        out,
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"t-{tone} wf-dot\"/>",
        number(dot.x),
        number(dot.y),
        if depth > 0 { "3.5" } else { "4.5" }
    )
    .unwrap();
    for (index, text) in node.text.iter().enumerate() {
        let (class, weight) = if index == 0 {
            (
                "t-primary",
                if depth == 0 {
                    750
                } else if parent_bar {
                    650
                } else {
                    500
                },
            )
        } else if parent_bar {
            ("t-primary wf-num", 700)
        } else {
            ("t-muted wf-num", 600)
        };
        write!(out, "<text {}x=\"{}\" y=\"{}\" class=\"{class}\" font-size=\"{}\" font-weight=\"{weight}\">{}</text>",
            if index == 0 { "data-node-label=\"\" " } else { "" }, number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
    }
    out.push_str("</g>");
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let DiagramFacts::Waterfall { evidence, .. } = &scene.facts else {
        return;
    };
    let locale = crate::i18n::resolve(&scene.meta);
    let tone = if evidence == "measured" {
        "backend"
    } else {
        "messagebus"
    };
    for badge in of_kind(scene, "waterfall-evidence") {
        let b = badge.bounds;
        write!(out, "<g data-waterfall-evidence=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"11\" class=\"c-{tone}\" stroke-width=\"1\"/><circle cx=\"{}\" cy=\"{}\" r=\"3\" class=\"t-{tone}\"/>",
            escape_text(evidence), number(b.x), number(b.y), number(b.width), number(b.height), number(b.x + 11.0), number(b.y + 11.0)).unwrap();
        for text in &badge.text {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-{tone}\" font-size=\"{}\" font-weight=\"700\" text-anchor=\"middle\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
        }
        out.push_str("</g>");
    }
    for total in of_kind(scene, "waterfall-total") {
        if let [wall, range] = total.text.as_slice() {
            write!(out, "<text data-waterfall-total=\"{}\" x=\"{}\" y=\"{}\" class=\"t-primary wf-num\" font-size=\"{}\" font-weight=\"700\">{}<tspan class=\"t-muted\" font-weight=\"500\" dx=\"10\">{}</tspan></text>",
                js_to_string(total.authored.facts["wall"].as_f64().unwrap_or_default()), number(wall.position.x), number(wall.position.y), number(wall.font_size), escape_text(&wall.content), escape_text(&range.content)).unwrap();
        }
    }
    for axis in of_kind(scene, "waterfall-axis") {
        if let (Some([start, end]), Some(caption)) = (axis.rule, axis.text.first()) {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-muted wf-caps\" font-size=\"{}\" font-weight=\"700\">{}</text><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"wf-axis\" stroke-width=\"1\"/>",
                number(caption.position.x), number(caption.position.y), number(caption.font_size), escape_text(&caption.content),
                number(start.x), number(start.y), number(end.x), number(end.y)).unwrap();
        }
    }
    for tick in of_kind(scene, "waterfall-tick") {
        let (Some([top, bottom]), Some(label)) = (tick.rule, tick.text.first()) else {
            continue;
        };
        write!(out, "<line x1=\"{0}\" y1=\"{1}\" x2=\"{0}\" y2=\"{2}\" class=\"wf-grid{3}\" stroke-width=\"1\"/><line x1=\"{0}\" y1=\"{4}\" x2=\"{0}\" y2=\"{1}\" class=\"wf-axis\" stroke-width=\"1\"/><text x=\"{5}\" y=\"{6}\" class=\"t-muted wf-num\" font-size=\"{7}\" text-anchor=\"middle\">{8}</text>",
            number(top.x), number(top.y), number(bottom.y), if tick.authored.facts["origin"] == true { " wf-grid-origin" } else { "" },
            number(top.y - 4.0), number(label.position.x), number(label.position.y), number(label.font_size), escape_text(&label.content)).unwrap();
    }
    // Ownership guides carry no relationship identity: a waterfall states
    // time, never runtime causality between spans.
    for guide in of_kind(scene, "waterfall-guide") {
        if let Some([from, to]) = guide.rule {
            write!(out, "<path d=\"M {} {} V {} Q {} {} {} {} H {}\" class=\"wf-guide\" stroke-width=\"1\" fill=\"none\"/>",
                number(from.x), number(from.y), number(to.y - 4.0), number(from.x), number(to.y), number(from.x + 4.0), number(to.y), number(to.x)).unwrap();
        }
    }
    for (order, node) in scene.nodes.iter().enumerate() {
        if let Some(row) =
            of_kind(scene, "waterfall-row").find(|row| row.authored.id == node.authored.id)
        {
            write_row(&locale, order, node, row, out);
        }
    }
}

fn legend_glyph(_scene: &CompiledDiagram, legend: &Card, baseline: f64, out: &mut String) {
    let facts = &legend.authored.facts;
    write!(out, "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"10\" rx=\"2.5\" class=\"c-{} {}\" stroke-width=\"1.2\"/>",
        number(legend.bounds.x), number(baseline - 10.0), facts["tone"].as_str().unwrap_or("external"), facts["paint"].as_str().unwrap_or("wf-solid")).unwrap();
}
