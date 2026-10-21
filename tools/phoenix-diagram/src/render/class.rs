// Class and interface SVG body, ported from the frozen render-class.mjs markup.
use super::erd::{edge_attributes, node_attributes, node_title, points_value, steps, text_element};
use super::{escape_text, TypeRenderer};
use crate::geometry::format_number as number;
use crate::geometry::text::is_js_whitespace;
use crate::layout::class::{notation, stereotype, KINDS, VIS_CHIP};
use crate::scene::{Card, CompiledDiagram, DiagramFacts, Marker, Relation, Text};
use serde_json::Value;
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

const BADGE_SIZE: f64 = 16.0;

fn root_attributes(scene: &CompiledDiagram) -> String {
    // Like the erd, an automatic canvas scrolls a tall model instead of
    // shrinking member rows past legibility.
    let mut attributes = String::from(" data-class-ui=\"\"");
    if scene.meta.get("viewBox").is_none() {
        attributes.push_str(" data-reader-min-text=\"7.5\"");
    }
    attributes
}

fn defs(_scene: &CompiledDiagram) -> String {
    let mut style = String::from(include_str!("../../assets/svg-relationship-markers.svg"));
    style.push_str("<style>");
    style.push_str(STYLE);
    for tone in ["backend", "frontend", "messagebus", "database"] {
        write!(
            style,
            "svg[data-class-ui] .cl-hue-{tone} {{ --cl-hue: var(--{tone}-stroke); }}"
        )
        .unwrap();
    }
    for kind in KINDS {
        write!(style, "svg[data-class-ui]:has([data-legend-semantic-kind=\"{kind}\"]:is(:hover, :focus-within)) [data-class-relationship]:not([data-class-relationship=\"{kind}\"]) {{ opacity: .15; }}").unwrap();
    }
    style.push_str("@media (prefers-reduced-motion: reduce) { svg[data-class-ui] * { transition: none !important; } }</style>");
    style
}

const STYLE: &str = r#"svg[data-class-ui] .cl-zebra { fill: color-mix(in srgb, var(--cl-hue) 6%, transparent); }
svg[data-class-ui] .cl-param { fill: color-mix(in srgb, var(--text) 72%, var(--text-muted)); }
svg[data-class-ui] .cl-type { fill: var(--mask); stroke: color-mix(in srgb, var(--cl-hue) 46%, var(--lane-stroke)); }
svg[data-class-ui] [data-class-kind="abstract"] .cl-type { stroke-dasharray: 5 3; stroke: color-mix(in srgb, var(--cl-hue) 64%, var(--lane-stroke)); }
svg[data-class-ui] .cl-header { stroke: none; }
svg[data-class-ui] .cl-cap { stroke: var(--cl-hue); pointer-events: none; }
svg[data-class-ui] .cl-badge-chip { fill: var(--cl-hue); }
svg[data-class-ui] .cl-badge-letter { fill: var(--mask); }
svg[data-class-ui] .cl-stereotype { fill: var(--text-muted); letter-spacing: .04em; }
svg[data-class-ui] .cl-rule { stroke: color-mix(in srgb, var(--cl-hue) 22%, var(--lane-stroke)); }
svg[data-class-ui] .cl-vis { font-weight: 700; }
svg[data-class-ui] .cl-vis-chip { fill: color-mix(in srgb, var(--text) 6%, transparent); }
svg[data-class-ui] .cl-type-ref { fill: var(--text-muted); }
svg[data-class-ui] .cl-punct { fill: var(--text-muted); opacity: .8; }
svg[data-class-ui] .cl-label-box { stroke: var(--lane-stroke); stroke-width: .8; }
svg[data-class-ui] .cl-member { font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, 'DejaVu Sans Mono', 'Liberation Mono', 'Noto Sans Mono CJK SC', 'PingFang SC', 'Microsoft YaHei', monospace; }
svg[data-class-ui] .cl-dashed { stroke-dasharray: 6 4; }
svg[data-class-ui] .cl-marker { stroke: var(--arrow); }
svg[data-class-ui] .cl-marker-hollow { fill: var(--mask); }
svg[data-class-ui] .cl-marker-filled { fill: var(--arrow); }
svg[data-class-ui] .cl-legend-hit { fill: transparent; transition: fill .15s; }
svg[data-class-ui] .cl-legend-hit:focus { outline: none; }
svg[data-class-ui] .cl-legend-hit:focus-visible { stroke: var(--arrow); stroke-width: 1.5; }
svg[data-class-ui] [data-legend-semantic-kind]:is(:hover, :focus-within) .cl-legend-hit { fill: var(--lane-fill); }
svg[data-class-ui] [data-class-relationship] { transition: opacity .15s; }
"#;

/// The colour family a kind owns.
pub(crate) fn tone(kind: &str) -> &'static str {
    match kind {
        "interface" => "frontend",
        "enum" => "messagebus",
        "record" => "database",
        _ => "backend",
    }
}

fn visibility_class(glyph: &str) -> &'static str {
    match glyph {
        "+" => "public",
        "#" => "protected",
        "~" => "package",
        _ => "private",
    }
}

fn marker_name(marker: Marker) -> &'static str {
    match marker {
        Marker::DiamondFilled => "diamond-filled",
        Marker::DiamondHollow => "diamond-hollow",
        Marker::Triangle => "triangle",
        _ => "open",
    }
}

fn relationship_class(kind: &str) -> String {
    format!(
        "a-default cl-relationship{}",
        if notation(kind).2 { " cl-dashed" } else { "" }
    )
}

fn marker_attributes(kind: &str, reversed: bool) -> String {
    let (start, end, _) = notation(kind);
    if reversed {
        return format!(
            " marker-start=\"url(#cl-{}-start)\"",
            end.map_or("triangle", marker_name)
        );
    }
    let mut attributes = String::new();
    if let Some(start) = start {
        write!(
            attributes,
            " marker-start=\"url(#cl-{})\"",
            marker_name(start)
        )
        .unwrap();
    }
    if let Some(end) = end {
        write!(attributes, " marker-end=\"url(#cl-{})\"", marker_name(end)).unwrap();
    }
    attributes
}

const PUNCT: &str = "<tspan class=\"t-muted cl-punct\">";

fn punctuated(text: &str) -> String {
    let mut result = String::new();
    for character in escape_text(text).chars() {
        if matches!(character, '(' | ')' | ',') {
            write!(result, "{PUNCT}{character}</tspan>").unwrap();
        } else {
            result.push(character);
        }
    }
    result
}

/// A member line set like a signature in an editor: the visibility glyph and
/// every type annotation are quiet, so the names carry the row.
fn member_spans(text: &str, first: bool) -> String {
    if !first && !text.contains(':') && !text.starts_with(')') {
        return format!(
            "<tspan class=\"t-muted cl-type-ref\">{}</tspan>",
            escape_text(text)
        );
    }
    let mut out = String::new();
    let mut rest = text;
    if first {
        let bytes = rest.as_bytes();
        if bytes.len() >= 2 && matches!(bytes[0], b'+' | b'#' | b'~' | b'-') && bytes[1] == b' ' {
            let glyph = &rest[..1];
            write!(
                out,
                "<tspan class=\"t-muted cl-vis cl-vis-{}\">{}</tspan> ",
                visibility_class(glyph),
                escape_text(glyph)
            )
            .unwrap();
            rest = &rest[2..];
        }
    }
    // Every `:(\s*)([^,()]+)` annotation becomes a quiet type reference.
    let mut last = 0;
    let mut search = 0;
    while let Some(offset) = rest[search..].find(':') {
        let colon = search + offset;
        let start = colon + 1;
        let spaces = rest[start..]
            .char_indices()
            .find(|(_, c)| !is_js_whitespace(*c))
            .map_or(rest.len(), |(index, _)| start + index);
        let words = rest[spaces..]
            .char_indices()
            .find(|(_, c)| matches!(c, ',' | '(' | ')'))
            .map_or(rest.len(), |(index, _)| spaces + index);
        let (gap_end, end) = if words > spaces {
            (spaces, words)
        } else if spaces > start {
            // `[^,()]+` takes back the last space.
            let back = rest[..spaces]
                .char_indices()
                .last()
                .map_or(start, |(index, _)| index);
            (back, spaces)
        } else {
            search = start;
            continue;
        };
        write!(
            out,
            "{}{PUNCT}:</tspan>{}<tspan class=\"t-muted cl-type-ref\">{}</tspan>",
            punctuated(&rest[last..colon]),
            escape_text(&rest[start..gap_end]),
            escape_text(&rest[gap_end..end])
        )
        .unwrap();
        last = end;
        search = end;
    }
    out + &punctuated(&rest[last..])
}

fn parameter_spans(parameters: &str) -> String {
    // `split(/(,\s*)/)`: parts with their separators kept.
    let mut parts: Vec<(&str, bool)> = vec![];
    let mut start = 0;
    let mut cursor = 0;
    while let Some(offset) = parameters[cursor..].find(',') {
        let comma = cursor + offset;
        let end = parameters[comma + 1..]
            .char_indices()
            .find(|(_, c)| !is_js_whitespace(*c))
            .map_or(parameters.len(), |(index, _)| comma + 1 + index);
        parts.push((&parameters[start..comma], false));
        parts.push((&parameters[comma..end], true));
        start = end;
        cursor = end;
    }
    parts.push((&parameters[start..], false));
    parts
        .into_iter()
        .map(|(part, separator)| {
            if separator {
                return format!("{PUNCT},</tspan> ");
            }
            match part.split_once(':') {
                Some((name, annotation)) => format!(
                    "<tspan class=\"cl-param\">{}</tspan>{PUNCT}:</tspan><tspan class=\"t-muted cl-type-ref\">{}</tspan>",
                    escape_text(name),
                    escape_text(annotation)
                ),
                None => format!("<tspan class=\"cl-param\">{}</tspan>", escape_text(part)),
            }
        })
        .collect()
}

fn member<'a>(scene: &'a CompiledDiagram, path: &str) -> &'a Value {
    static NULL: Value = Value::Null;
    let DiagramFacts::Class { types, .. } = &scene.facts else {
        return &NULL;
    };
    let mut parts = path.trim_start_matches("/types/").splitn(2, '/');
    let index: usize = parts
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(usize::MAX);
    types
        .get(index)
        .and_then(|item| item.pointer(&format!("/{}", parts.next().unwrap_or(""))))
        .unwrap_or(&NULL)
}

fn bus(relation: &Relation) -> bool {
    relation
        .junction
        .as_deref()
        .is_some_and(|junction| junction.starts_with("class-bus:"))
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let locale = crate::i18n::resolve(&scene.meta);
    for (index, relation) in scene.relations.iter().enumerate() {
        let kind = relation.authored.facts["kind"]
            .as_str()
            .unwrap_or("association");
        let forward = crate::geometry::rounded_path(&relation.points, 8.0);
        let reversed = bus(relation);
        // A bus member is drawn from the supertype outward so the shared
        // stretches start at one point and their dashes coincide.
        let (d, extra) = if reversed {
            let points: Vec<_> = relation.points.iter().rev().copied().collect();
            (
                crate::geometry::rounded_path(&points, 8.0),
                format!(
                    " data-class-bus=\"{}\" data-motion-path=\"{}\" data-composition-junction=\"{}\"",
                    escape_text(&relation.to),
                    escape_text(&forward),
                    escape_text(relation.junction.as_deref().unwrap_or(""))
                ),
            )
        } else {
            (forward, String::new())
        };
        write!(out, "<path {} data-class-relationship=\"{}\"{extra} data-graph-role=\"edge\" data-composition-points=\"{}\" d=\"{d}\" class=\"{}\" data-animate=\"edge\" style=\"--step:{}\" stroke-width=\"1.5\"{}/>",
            edge_attributes(relation, index), escape_text(kind), points_value(&relation.points),
            relationship_class(kind), index.min(12), marker_attributes(kind, reversed)).unwrap();
    }
    let (types, relationships) = match &scene.facts {
        DiagramFacts::Class {
            types,
            relationships,
        } => (types.as_slice(), relationships.as_slice()),
        _ => (&[][..], &[][..]),
    };
    let steps = steps(types, relationships, true);
    for node in &scene.nodes {
        let kind = node.authored.kind.as_str();
        let tone = tone(kind);
        let b = node.bounds;
        let sublabel = node.authored.facts["sublabel"].as_str();
        let context = locale.text(&format!("node.context.class.{kind}"), &[]);
        let header = if stereotype(kind).is_some() {
            44.0
        } else {
            34.0
        };
        write!(out, "<g {} data-class-kind=\"{}\" class=\"cl-hue-{tone}\">{}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" class=\"c-mask\"/><rect data-graph-role=\"node-body\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" class=\"c-{tone} cl-type\" data-animate=\"node\" style=\"--step:{}\" stroke-width=\"1.2\"/>",
            node_attributes(&locale, &node.authored.id, &node.label, &node.authored.source_path,
                &[("kind", Some(kind)), ("sublabel", sublabel), ("context", Some(&context))],
                &[sublabel, Some(&context)]),
            escape_text(kind), node_title(&[Some(&node.label), sublabel, Some(&context)]),
            number(b.x), number(b.y), number(b.width), number(b.height),
            number(b.x), number(b.y), number(b.width), number(b.height),
            steps.get(&node.authored.id).copied().unwrap_or(0).min(12)).unwrap();
        write!(out, "<path class=\"cl-header c-{tone}\" stroke=\"none\" d=\"M {} {} H {} A 8 8 0 0 1 {} {} V {} H {} V {} A 8 8 0 0 1 {} {} Z\"/><path class=\"cl-cap\" d=\"M {} {} H {}\" stroke-width=\"2.6\" stroke-linecap=\"round\" fill=\"none\"/>",
            number(b.x + 8.0), number(b.y), number(b.x + b.width - 8.0), number(b.x + b.width), number(b.y + 8.0), number(b.y + header),
            number(b.x), number(b.y + 8.0), number(b.x + 8.0), number(b.y), number(b.x + 12.0), number(b.y + 1.4), number(b.x + b.width - 12.0)).unwrap();
        for text in &node.text {
            let content = escape_text(&text.content);
            if text.source_path.ends_with("/label") {
                let italic = if kind == "abstract" {
                    " font-style=\"italic\""
                } else {
                    ""
                };
                out.push_str(&text_element(
                    text,
                    &format!(
                        "data-node-label=\"\" class=\"t-primary\" font-weight=\"700\"{italic}"
                    ),
                    true,
                    &content,
                ));
            } else if text.content.starts_with('«') {
                out.push_str(&text_element(
                    text,
                    "class=\"t-muted cl-stereotype\"",
                    true,
                    &content,
                ));
            } else {
                write!(out, "<g aria-hidden=\"true\" class=\"cl-badge\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" class=\"cl-badge-chip\"/>",
                    number(text.position.x - BADGE_SIZE / 2.0), number(text.position.y - BADGE_SIZE / 2.0 - 3.5),
                    number(BADGE_SIZE), number(BADGE_SIZE), number(BADGE_SIZE * 0.3)).unwrap();
                out.push_str(&text_element(
                    text,
                    &format!("class=\"t-{tone} cl-badge-letter\" font-weight=\"800\""),
                    true,
                    &content,
                ));
                out.push_str("</g>");
            }
        }
        members(scene, node, out);
        out.push_str("</g>");
    }
    for (index, relation) in scene.relations.iter().enumerate() {
        let (Some(plate), Some(label)) = (relation.label_plate, relation.text.first()) else {
            continue;
        };
        write!(out, "<g data-detail=\"context\" {} data-class-relationship=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" class=\"c-mask cl-label-box\"/>",
            edge_attributes(relation, index), escape_text(relation.authored.facts["kind"].as_str().unwrap_or("")),
            number(plate.x), number(plate.y), number(plate.width), number(plate.height), number(plate.height / 2.0)).unwrap();
        out.push_str(&text_element(
            label,
            "class=\"t-muted\"",
            true,
            &escape_text(&label.content),
        ));
        out.push_str("</g>");
    }
}

fn members(scene: &CompiledDiagram, node: &crate::scene::Node, out: &mut String) {
    let b = node.bounds;
    let mut open: Option<&str> = None;
    // Zebra stripes alternate across both compartments of one type.
    for (stripe, row) in node.detail_rows.iter().enumerate() {
        if open != Some(row.role.as_str()) {
            if open.is_some() {
                out.push_str("</g>");
            }
            let top = row.bounds.y - 5.0;
            write!(out, "<g data-detail=\"context\" data-class-compartment=\"{}\"><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"cl-rule\" stroke-width=\"1\"/>",
                escape_text(&row.role), number(b.x), number(top), number(b.x + b.width), number(top)).unwrap();
            open = Some(row.role.as_str());
        }
        let facts = member(scene, &row.source_path);
        let style: Vec<&str> = [
            (facts["static"] == true).then_some("text-decoration: underline"),
            (facts["abstract"] == true).then_some("font-style: italic"),
        ]
        .into_iter()
        .flatten()
        .collect();
        let style = if style.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", style.join("; "))
        };
        if stripe % 2 == 1 {
            write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" class=\"cl-zebra\"/>",
                number(b.x + 4.0),
                number(row.bounds.y + 1.0),
                number(b.width - 8.0),
                number(row.bounds.height - 2.0)
            )
            .unwrap();
        }
        let member_attributes = format!(
            "data-detail=\"context\" data-class-member=\"{}\"",
            escape_text(&row.role)
        );
        for (position, text) in row.text.iter().enumerate() {
            let path = text.source_path.as_str();
            if path.ends_with("/visibility") {
                let kind = visibility_class(&text.content);
                write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" class=\"cl-vis-chip cl-vis-{kind}\"/>",
                    number(text.position.x - VIS_CHIP / 2.0), number(text.position.y - 10.5), number(VIS_CHIP), number(VIS_CHIP)).unwrap();
                out.push_str(&text_element(
                    text,
                    &format!("class=\"t-muted cl-vis cl-vis-{kind}\""),
                    true,
                    &escape_text(&text.content),
                ));
            } else if path.ends_with("/name") {
                let name = facts["name"].as_str().unwrap_or("");
                let content = if row.role == "methods" {
                    format!(
                        "{}{PUNCT}(</tspan>{}{PUNCT})</tspan>",
                        escape_text(name),
                        facts["parameters"]
                            .as_str()
                            .filter(|parameters| !parameters.is_empty())
                            .map(parameter_spans)
                            .unwrap_or_default()
                    )
                } else {
                    punctuated(name)
                };
                out.push_str(&styled(
                    text,
                    &format!("{member_attributes} class=\"t-primary cl-member\""),
                    &style,
                    &content,
                ));
            } else if path.ends_with("/type") || path.ends_with("/returns") {
                out.push_str(&styled(
                    text,
                    "data-detail=\"context\" class=\"t-muted cl-member cl-type-ref\"",
                    &style,
                    &escape_text(&text.content),
                ));
            } else {
                let first = !row.text[..position]
                    .iter()
                    .any(|text| text.source_path.contains("/lines/"));
                let attributes = if first {
                    format!("{member_attributes} class=\"t-primary cl-member\"")
                } else {
                    format!("{member_attributes} data-class-continuation=\"\" class=\"t-primary cl-member\"")
                };
                out.push_str(&styled(
                    text,
                    &attributes,
                    &style,
                    &member_spans(&text.content, first),
                ));
            }
        }
    }
    if open.is_some() {
        out.push_str("</g>");
    }
}

fn styled(text: &Text, attributes: &str, style: &str, content: &str) -> String {
    text_element(text, &format!("{attributes}{style}"), true, content)
}

// A swatch is a short relationship drawn in the same notation.
fn legend_glyph(_scene: &CompiledDiagram, legend: &Card, baseline: f64, out: &mut String) {
    let kind = legend.authored.kind.as_str();
    let y = baseline - 4.0;
    let label = legend.text.first().map_or("", |text| text.content.as_str());
    write!(out, "<rect class=\"cl-legend-hit\" tabindex=\"0\" role=\"img\" aria-label=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"22\" rx=\"11\"/><path d=\"M {} {} L {} {}\" class=\"{}\" stroke-width=\"1.5\"{}/>",
        escape_text(label), number(legend.bounds.x - 6.0), number(baseline - 15.0), number(legend.bounds.width + 12.0),
        number(legend.bounds.x + 2.0), number(y), number(legend.bounds.x + 32.0), number(y),
        relationship_class(kind), marker_attributes(kind, false)).unwrap();
}
