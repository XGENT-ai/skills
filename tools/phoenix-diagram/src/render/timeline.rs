use super::{escape_text, TypeRenderer};
use crate::geometry::format_number as number;
use crate::layout::timeline::{tone, Clock, KINDS};
use crate::scene::{Card, CompiledDiagram, Container, DiagramFacts, Node};
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

fn root_attributes(_scene: &CompiledDiagram) -> String {
    // Node kinds are the tones of the event kinds, not the authored kinds.
    " data-motion-choreography=\"sweep\" data-timeline-ui=\"\" data-reader-min-text=\"7.5\" data-node-kind-derived=\"\"".into()
}

fn of_kind<'a>(scene: &'a CompiledDiagram, kind: &'a str) -> impl Iterator<Item = &'a Container> {
    scene
        .containers
        .iter()
        .filter(move |container| container.authored.kind == kind)
}

fn defs(scene: &CompiledDiagram) -> String {
    let mut defs = String::new();
    if of_kind(scene, "timeline-break").next().is_some() {
        defs.push_str("<defs><pattern id=\"tl-break-hatch\" width=\"6\" height=\"6\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate(45)\"><line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"6\" class=\"tl-hatch\" stroke-width=\"1\"/></pattern></defs>");
    }
    defs.push_str(concat!(
        "<style>",
        "svg[data-timeline-ui] .tl-grid { stroke: var(--lane-stroke); stroke-dasharray: 2 4; opacity: .7; }",
        "svg[data-timeline-ui] .tl-axis { stroke: var(--text-dim); }",
        "svg[data-timeline-ui] .tl-lane { fill: var(--lane-fill); stroke: none; opacity: .75; }",
        "svg[data-timeline-ui] .tl-lane-alt { opacity: 0; }",
        "svg[data-timeline-ui] .tl-lane-line { stroke: var(--lane-stroke); }",
        "svg[data-timeline-ui] .tl-stem { stroke: var(--text-dim); }",
        "svg[data-timeline-ui] .tl-card { fill-opacity: .55; stroke-opacity: .5; }",
        "svg[data-timeline-ui] .tl-card.c-external { fill: var(--mask); stroke: var(--lane-stroke); stroke-opacity: 1; }",
        "svg[data-timeline-ui] .tl-dot { stroke: var(--mask); }",
        "svg[data-timeline-ui] .tl-halo { fill: var(--mask); }",
        "svg[data-timeline-ui] .tl-break-band { opacity: .55; }",
        "svg[data-timeline-ui] .tl-break-chip { fill: var(--mask); stroke: var(--lane-stroke); stroke-width: 1; }",
        "svg[data-timeline-ui] .tl-hatch { stroke: var(--lane-stroke); }",
        "svg[data-timeline-ui] .tl-break-zig { stroke: var(--text-dim); }",
    ));
    for (_, tone) in KINDS {
        write!(defs, "svg[data-timeline-ui] .tl-accent-{tone} {{ stroke: var(--{tone}-stroke); }}svg[data-timeline-ui] .tl-stem-{tone} {{ stroke: color-mix(in srgb, var(--{tone}-stroke) 45%, var(--lane-stroke)); }}svg[data-timeline-ui] .tl-pin-{tone} {{ fill: var(--{tone}-fill); stroke: color-mix(in srgb, var(--{tone}-stroke) 35%, transparent); stroke-width: 1; }}").unwrap();
    }
    defs.push_str("svg[data-timeline-ui] .tl-tick, svg[data-timeline-ui] .tl-time { font-variant-numeric: tabular-nums; }</style>");
    defs
}

fn evidence_tone(evidence: &str) -> &'static str {
    if evidence == "observed" {
        "backend"
    } else {
        "messagebus"
    }
}

fn write_event(
    scene: &CompiledDiagram,
    clock: &Clock,
    locale: &crate::i18n::Locale,
    order: usize,
    node: &Node,
    out: &mut String,
) {
    let id = node.authored.id.as_str();
    let facts = &node.authored.facts;
    let tone = tone(facts["kind"].as_str().unwrap_or("default"));
    let Some([dot, _]) = of_kind(scene, "timeline-stem")
        .find(|stem| stem.authored.id == id)
        .and_then(|stem| stem.rule)
    else {
        return;
    };
    let lane = of_kind(scene, "timeline-lane")
        .find(|lane| lane.members.iter().any(|member| member == id))
        .and_then(|lane| lane.text.first())
        .map(|text| text.content.as_str());
    let at = facts["at"].as_str().unwrap_or_default();
    let ms = facts["at"]
        .as_str()
        .and_then(crate::format::time::parse_timestamp)
        .unwrap_or_default();
    let instant = if facts["precision"] == "day" {
        clock.event_label(at, ms, "day", locale)
    } else {
        format!("{} {}", clock.label(ms, true, true), clock.offset_label(ms))
    };
    let context = lane
        .into_iter()
        .chain([instant.as_str()])
        .collect::<Vec<_>>()
        .join(" \u{203a} ");
    let detail = facts["detail"]
        .as_str()
        .filter(|value| !value.trim().is_empty());
    let summary: Vec<&str> = detail.into_iter().chain([context.as_str()]).collect();
    let aria = locale.text(
        "node.focus.detail",
        &[("label", &node.label), ("detail", &summary.join(", "))],
    );
    write!(out, "<g id=\"node-{0}\" data-node-id=\"{0}\" data-node-label=\"{1}\" tabindex=\"0\" role=\"button\" aria-label=\"{2}\" aria-pressed=\"false\" data-node-kind=\"{tone}\"",
        escape_text(id), escape_text(&node.label), escape_text(&aria)).unwrap();
    if let Some(detail) = detail {
        write!(out, " data-node-sublabel=\"{}\"", escape_text(detail)).unwrap();
    }
    let title: Vec<&str> = [node.label.as_str()].into_iter().chain(summary).collect();
    write!(out, " data-node-context=\"{}\" data-source-path=\"{}\" data-timeline-at=\"{}\" data-timeline-ms=\"{ms}\" data-timeline-x=\"{}\"><title>{}</title>",
        escape_text(&context), escape_text(&node.authored.source_path), escape_text(at), number(dot.x), escape_text(&title.join(" · "))).unwrap();
    let b = node.bounds;
    let (x, y, w, h) = (number(b.x), number(b.y), number(b.width), number(b.height));
    write!(out, "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"7\" class=\"c-mask\"/><rect data-graph-role=\"node-body\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"7\" class=\"c-{tone} tl-card\" data-animate=\"node\" style=\"--step:{}\" stroke-width=\"1\"/>", order.min(12)).unwrap();
    // A tone accent down the leading edge carries the event kind, so the
    // card body stays a quiet surface for the text.
    write!(out, "<line x1=\"{0}\" y1=\"{1}\" x2=\"{0}\" y2=\"{2}\" class=\"tl-accent tl-accent-{tone}\" stroke-width=\"3\" stroke-linecap=\"round\"/>",
        number(b.x + 6.0), number(b.y + 7.0), number(b.y + b.height - 7.0)).unwrap();
    for text in node
        .text
        .iter()
        .filter(|text| text.source_path.ends_with("/at"))
    {
        write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-{tone} tl-time\" font-size=\"{}\" font-weight=\"600\">{}</text>",
            number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
    }
    let lines: Vec<_> = node
        .text
        .iter()
        .filter(|text| text.source_path.ends_with("/title"))
        .collect();
    if let Some(first) = lines.first() {
        write!(
            out,
            "<text data-node-label=\"\" class=\"t-primary\" font-size=\"{}\" font-weight=\"650\">",
            number(first.font_size)
        )
        .unwrap();
        for line in lines {
            write!(
                out,
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                number(line.position.x),
                number(line.position.y),
                escape_text(&line.content)
            )
            .unwrap();
        }
        out.push_str("</text>");
    }
    write!(
        out,
        "<circle cx=\"{}\" cy=\"{}\" r=\"4.75\" class=\"t-{tone}\"/></g>",
        number(dot.x),
        number(dot.y)
    )
    .unwrap();
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let DiagramFacts::Timeline {
        timezone,
        evidence,
        events,
        ..
    } = &scene.facts
    else {
        return;
    };
    let locale = crate::i18n::resolve(&scene.meta);
    let instants: Vec<i64> = events.iter().map(|event| event.at_ms).collect();
    let Some(clock) = Clock::of(timezone, &instants) else {
        return;
    };
    let tone = evidence_tone(evidence);
    for badge in of_kind(scene, "timeline-evidence") {
        let b = badge.bounds;
        write!(out, "<g data-timeline-evidence=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"11\" class=\"c-{tone}\" stroke-width=\"1\"/><circle cx=\"{}\" cy=\"{}\" r=\"3\" class=\"t-{tone}\"/>",
            escape_text(evidence), number(b.x), number(b.y), number(b.width), number(b.height), number(b.x + 11.0), number(b.y + 11.0)).unwrap();
        for text in &badge.text {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-{tone}\" font-size=\"{}\" font-weight=\"700\" text-anchor=\"middle\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
        }
        out.push_str("</g>");
    }
    for caption in &scene.text {
        write!(out, "<text data-timeline-axis-caption=\"\" x=\"{}\" y=\"{}\" class=\"t-muted\" font-size=\"{}\">{}</text>",
            number(caption.position.x), number(caption.position.y), number(caption.font_size), escape_text(&caption.content)).unwrap();
    }
    for (index, lane) in of_kind(scene, "timeline-lane").enumerate() {
        let b = lane.bounds;
        write!(out, "<g data-timeline-lane=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"10\" class=\"tl-lane{}\"/>",
            escape_text(&lane.authored.id), number(b.x), number(b.y), number(b.width), number(b.height), if index % 2 == 1 { " tl-lane-alt" } else { "" }).unwrap();
        if let Some([start, end]) = lane.rule {
            write!(out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"tl-lane-line\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>",
                number(start.x), number(start.y), number(end.x), number(end.y)).unwrap();
        }
        for text in &lane.text {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-primary\" font-size=\"{}\" font-weight=\"700\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
        }
        out.push_str("</g>");
    }
    for tick in of_kind(scene, "timeline-tick") {
        let (Some([top, bottom]), Some(text)) = (tick.rule, tick.text.first()) else {
            continue;
        };
        write!(out, "<line x1=\"{0}\" y1=\"{1}\" x2=\"{0}\" y2=\"{2}\" class=\"tl-grid\" stroke-width=\"1\"/><line x1=\"{0}\" y1=\"{3}\" x2=\"{0}\" y2=\"{1}\" class=\"tl-axis\" stroke-width=\"1\"/><text x=\"{4}\" y=\"{5}\" class=\"t-muted tl-tick\" font-size=\"{6}\" text-anchor=\"middle\">{7}</text>",
            number(top.x), number(top.y), number(bottom.y), number(top.y - 4.0), number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
    }
    for segment in of_kind(scene, "timeline-axis") {
        if let Some([start, end]) = segment.rule {
            write!(out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"tl-axis\" stroke-width=\"1\"/>",
                number(start.x), number(start.y), number(end.x), number(end.y)).unwrap();
        }
    }
    for gap in of_kind(scene, "timeline-break") {
        let (Some([from, to]), Some(chip), Some(text)) =
            (gap.rule, gap.label_plate, gap.text.first())
        else {
            continue;
        };
        let y = from.y;
        let zig = |x: f64| {
            format!(
                "M {} {} L {} {}",
                number(x - 2.5),
                number(y - 6.0),
                number(x + 2.5),
                number(y + 6.0)
            )
        };
        let omitted = locale.text(
            "timeline.break",
            &[(
                "duration",
                gap.authored.facts["duration"].as_str().unwrap_or_default(),
            )],
        );
        let b = gap.bounds;
        write!(out, "<g data-timeline-break=\"\" data-break-ms=\"{}\" aria-label=\"{}\" role=\"img\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#tl-break-hatch)\" class=\"tl-break-band\"/><path d=\"{} {} {} {}\" class=\"tl-break-zig\" stroke-width=\"1.3\" stroke-linecap=\"round\" fill=\"none\"/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8.5\" class=\"tl-break-chip\"/><text x=\"{}\" y=\"{}\" class=\"t-muted\" font-size=\"{}\" font-weight=\"700\" text-anchor=\"middle\">{}</text></g>",
            gap.authored.facts["length"], escape_text(&omitted), number(b.x), number(b.y), number(b.width), number(b.height),
            zig(from.x + 6.0), zig(from.x + 10.0), zig(to.x - 10.0), zig(to.x - 6.0),
            number(chip.x), number(chip.y), number(chip.width), number(chip.height),
            number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
    }
    // Stems sit behind every card, and one halo layer sits under every dot,
    // so events seconds apart merge into one outlined capsule.
    let stems: Vec<_> = of_kind(scene, "timeline-stem").collect();
    for stem in &stems {
        if let Some([dot, end]) = stem.rule {
            write!(out, "<line data-detail=\"context\" data-timeline-stem=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"tl-stem tl-stem-{}\" stroke-width=\"1\"/>",
                escape_text(&stem.authored.id), number(dot.x), number(dot.y), number(end.x), number(end.y), tone_of(stem)).unwrap();
        }
    }
    for stem in &stems {
        if let Some([dot, _]) = stem.rule {
            write!(
                out,
                "<circle data-detail=\"context\" cx=\"{}\" cy=\"{}\" r=\"7.5\" class=\"tl-halo\"/>",
                number(dot.x),
                number(dot.y)
            )
            .unwrap();
        }
    }
    for stem in &stems {
        if let Some([dot, _]) = stem.rule {
            write!(out, "<circle data-detail=\"context\" cx=\"{}\" cy=\"{}\" r=\"6.5\" class=\"tl-pin tl-pin-{}\"/>", number(dot.x), number(dot.y), tone_of(stem)).unwrap();
        }
    }
    for (order, node) in scene.nodes.iter().enumerate() {
        write_event(scene, &clock, &locale, order, node, out);
    }
}

fn tone_of(stem: &Container) -> &'static str {
    tone(stem.authored.facts["kind"].as_str().unwrap_or("default"))
}

fn legend_glyph(_scene: &CompiledDiagram, legend: &Card, baseline: f64, out: &mut String) {
    write!(
        out,
        "<circle cx=\"{}\" cy=\"{}\" r=\"4.5\" class=\"t-{} tl-dot\" stroke-width=\"2\"/>",
        number(legend.bounds.x + 6.0),
        number(baseline - 4.0),
        tone(&legend.authored.kind)
    )
    .unwrap();
}
