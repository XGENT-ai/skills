// Entity-relationship SVG body, ported from the frozen render-erd.mjs markup.
use super::{escape_text, sigil_shape, TypeRenderer};
use crate::geometry::format_number as number;
use crate::i18n::Locale;
use crate::layout::erd::trunks;
use crate::scene::{Card, CompiledDiagram, DiagramFacts, Marker, Point, Rect, Relation, Text};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

const HUES: [&str; 6] = [
    "database",
    "frontend",
    "backend",
    "cloud",
    "messagebus",
    "security",
];
const KEY_BADGE_W: f64 = 22.0;
const MARGIN: f64 = 24.0;

fn root_attributes(scene: &CompiledDiagram) -> String {
    // An automatic canvas is intrinsic-height: the reader scrolls a tall schema
    // instead of shrinking its fields past this floor.
    let mut attributes = String::from(" data-erd-ui=\"cards\"");
    if scene.meta.get("viewBox").is_none() {
        attributes.push_str(" data-reader-min-text=\"7.5\"");
    }
    attributes
}

fn defs(_scene: &CompiledDiagram) -> String {
    let mut style = String::from(include_str!("../../assets/svg-relationship-markers.svg"));
    style.push_str("<style>");
    for hue in HUES {
        write!(style, "svg[data-erd-ui=\"cards\"] .er-hue-{hue} {{ --er-hue: var(--{hue}-stroke); --er-wash: var(--{hue}-fill); }}").unwrap();
    }
    style.push_str(STYLE);
    for (kind, ring) in [
        ("pk", "var(--database-stroke)"),
        ("fk", "var(--messagebus-stroke)"),
        ("uk", "var(--text-muted)"),
    ] {
        write!(style, "svg[data-erd-ui=\"cards\"]:has([data-legend-semantic-kind=\"{kind}\"]:is(:hover, :focus-within)) [data-er-row]:not(:has([data-er-badge=\"{kind}\"])) {{ opacity: .22; }} svg[data-erd-ui=\"cards\"]:has([data-legend-semantic-kind=\"{kind}\"]:is(:hover, :focus-within)) [data-er-row] [data-er-badge=\"{kind}\"] {{ stroke: {ring}; stroke-width: 1.25; }}").unwrap();
    }
    // One and many sit on most routes, so the matching glyph itself takes the accent.
    for (kind, marker) in [
        ("one", "[id^=\"er-one\"]"),
        ("many", "[id^=\"er-many\"]"),
        ("optional", "[id*=\"-optional\"]"),
    ] {
        write!(style, "svg[data-erd-ui=\"cards\"]:has([data-legend-semantic-kind=\"{kind}\"]:is(:hover, :focus-within)) [data-er-ends]:not([data-er-ends~=\"{kind}\"]) {{ opacity: .12; }} svg[data-erd-ui=\"cards\"]:has([data-legend-semantic-kind=\"{kind}\"]:is(:hover, :focus-within)) marker{marker} {{ stroke: var(--database-stroke) !important; stroke-width: 2.25; }}").unwrap();
    }
    style.push_str("@media (prefers-reduced-motion: reduce) { svg[data-erd-ui=\"cards\"] * { transition: none !important; } }</style>");
    style
}

const STYLE: &str = r#"svg[data-erd-ui="cards"] .er-band { fill: color-mix(in srgb, var(--er-hue) 4%, transparent); stroke: color-mix(in srgb, var(--er-hue) 20%, transparent); stroke-dasharray: none; }
svg[data-erd-ui="cards"] .er-band-pill { fill: var(--er-wash); stroke: color-mix(in srgb, var(--er-hue) 30%, transparent); stroke-width: .8; }
svg[data-erd-ui="cards"] .er-band-label { fill: var(--er-hue); letter-spacing: .08em; text-transform: uppercase; }
svg[data-erd-ui="cards"] .er-table { fill: var(--mask); stroke: color-mix(in srgb, var(--er-hue) 34%, var(--lane-stroke)); }
svg[data-erd-ui="cards"] .er-table-header { fill: var(--er-wash); stroke: none; }
svg[data-erd-ui="cards"] .er-table-cap { stroke: var(--er-hue); pointer-events: none; }
svg[data-erd-ui="cards"] .er-header-rule { stroke: color-mix(in srgb, var(--er-hue) 30%, var(--lane-stroke)); }
svg[data-erd-ui="cards"] [class*="er-hue-"] .semantic-sigil { color: var(--er-hue); }
svg[data-erd-ui="cards"] .er-row-rule { stroke-opacity: 0; }
svg[data-erd-ui="cards"] .er-zebra { fill: color-mix(in srgb, var(--er-hue) 6%, transparent); }
svg[data-erd-ui="cards"] .er-name-pk { font-weight: 700; }
svg[data-erd-ui="cards"] .er-type { fill: var(--text-muted); opacity: .86; }
svg[data-erd-ui="cards"] .er-key-badge { fill: var(--database-fill); stroke: none; }
svg[data-erd-ui="cards"] .er-key-badge[data-er-badge="fk"] { fill: var(--messagebus-fill); }
svg[data-erd-ui="cards"] .er-key-badge[data-er-badge="uk"] { fill: none; stroke: var(--lane-stroke); }
svg[data-erd-ui="cards"] .er-key-badge + text { text-anchor: middle; }
svg[data-erd-ui="cards"] [data-legend] text.t-primary { fill: var(--text); }
svg[data-erd-ui="cards"] [data-legend] > text.t-primary { fill: var(--text-muted); font-size: 10px; letter-spacing: .08em; text-transform: uppercase; }
svg[data-erd-ui="cards"] .er-legend-hit { fill: transparent; transition: fill .15s; }
svg[data-erd-ui="cards"] .er-legend-hit:focus { outline: none; }
svg[data-erd-ui="cards"] .er-legend-hit:focus-visible { stroke: var(--database-stroke); stroke-width: 1.5; }
svg[data-erd-ui="cards"] [data-legend-semantic-kind]:is(:hover, :focus-within) .er-legend-hit { fill: var(--database-fill); }
svg[data-erd-ui="cards"] [data-legend-semantic-kind]:is(:hover, :focus-within) .er-legend-glyph { stroke: var(--database-stroke) !important; }
svg[data-erd-ui="cards"] .er-legend-route { stroke: var(--text-muted); stroke-opacity: .55; }
svg[data-erd-ui="cards"] .er-legend-edge { stroke: var(--lane-stroke); stroke-linecap: round; }
svg[data-erd-ui="cards"] .er-legend-divider { stroke: var(--lane-stroke); }
svg[data-erd-ui="cards"] [data-er-row], svg[data-erd-ui="cards"] [data-er-ends] { transition: opacity .15s; }
"#;

fn rect(rect: &Rect) -> String {
    format!(
        "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        number(rect.x),
        number(rect.y),
        number(rect.width),
        number(rect.height)
    )
}

/// The Reader's focus hooks for one node, shared with the class renderer.
pub(super) fn node_attributes(
    locale: &Locale,
    id: &str,
    label: &str,
    source_path: &str,
    fields: &[(&str, Option<&str>)],
    detail: &[Option<&str>],
) -> String {
    fn present<'a>(value: &Option<&'a str>) -> Option<&'a str> {
        value.filter(|value| !value.trim().is_empty())
    }
    let detail = detail
        .iter()
        .filter_map(present)
        .collect::<Vec<_>>()
        .join(", ");
    let aria = if detail.is_empty() {
        locale.text("node.focus", &[("label", label)])
    } else {
        locale.text(
            "node.focus.detail",
            &[("label", label), ("detail", &detail)],
        )
    };
    let mut attributes = format!(
        "id=\"node-{}\" data-node-id=\"{}\" data-node-label=\"{}\" tabindex=\"0\" role=\"button\" aria-label=\"{}\" aria-pressed=\"false\" data-source-path=\"{}\"",
        escape_text(id),
        escape_text(id),
        escape_text(label),
        escape_text(&aria),
        escape_text(source_path)
    );
    for (name, value) in fields {
        if let Some(value) = present(value) {
            write!(attributes, " data-node-{name}=\"{}\"", escape_text(value)).unwrap();
        }
    }
    attributes
}

/// The native SVG title that keeps details on demand outside the Reader.
pub(super) fn node_title(parts: &[Option<&str>]) -> String {
    let parts: Vec<_> = parts
        .iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .copied()
        .collect();
    format!("<title>{}</title>", escape_text(&parts.join(" · ")))
}

/// Ambient trace order: a node animates with the first relationship that reaches it.
pub(super) fn steps(
    nodes: &[Value],
    relations: &[Value],
    target_first: bool,
) -> HashMap<String, usize> {
    let mut steps = HashMap::new();
    for (index, relation) in relations.iter().enumerate() {
        let (first, second) = if target_first {
            ("to", "from")
        } else {
            ("from", "to")
        };
        for (field, step) in [(first, index), (second, index + 1)] {
            if let Some(id) = relation[field].as_str() {
                steps.entry(id.to_owned()).or_insert(step);
            }
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        if let Some(id) = node["id"].as_str() {
            steps.entry(id.to_owned()).or_insert(index);
        }
    }
    steps
}

pub(super) fn edge_attributes(relation: &Relation, key: usize) -> String {
    let mut attributes = format!(
        "data-edge-from=\"{}\" data-edge-to=\"{}\"",
        escape_text(&relation.from),
        escape_text(&relation.to)
    );
    if !relation.label.is_empty() {
        write!(
            attributes,
            " data-edge-label=\"{}\"",
            escape_text(&relation.label)
        )
        .unwrap();
    }
    write!(attributes, " data-edge-key=\"{key}\"").unwrap();
    if let Some(id) = relation.authored.facts["id"]
        .as_str()
        .filter(|id| !id.trim().is_empty())
    {
        write!(attributes, " data-edge-id=\"{}\"", escape_text(id)).unwrap();
    }
    attributes
}

pub(super) fn points_value(points: &[Point]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", number(point.x), number(point.y)))
        .collect::<Vec<_>>()
        .join(";")
}

pub(super) fn text_element(text: &Text, attributes: &str, anchor: bool, content: &str) -> String {
    let mut markup = format!(
        "<text {attributes} x=\"{}\" y=\"{}\" font-size=\"{}\"",
        number(text.position.x),
        number(text.position.y),
        number(text.font_size)
    );
    if anchor && text.anchor != "start" {
        write!(markup, " text-anchor=\"{}\"", escape_text(&text.anchor)).unwrap();
    }
    write!(
        markup,
        " data-source-path=\"{}\">{content}</text>",
        escape_text(&text.source_path)
    )
    .unwrap();
    markup
}

fn marker_id(marker: Option<Marker>, end: &str) -> String {
    let name = match marker {
        Some(Marker::ErOne) => "er-one",
        Some(Marker::ErOneOptional) => "er-one-optional",
        Some(Marker::ErMany) => "er-many",
        _ => "er-many-optional",
    };
    format!("{name}-{end}")
}

fn ends(raw: &Value) -> String {
    crate::layout::erd::relationship_ends(raw).join(" ")
}

fn variant_class(variant: &str) -> &'static str {
    match variant {
        "emphasis" => "a-emphasis",
        "security" => "a-security",
        "dashed" => "a-dashed",
        _ => "a-default",
    }
}

fn hue_class(scene: &CompiledDiagram, tag: Option<&str>) -> &'static str {
    let DiagramFacts::Erd { entities, .. } = &scene.facts else {
        return "database";
    };
    let mut tags: Vec<&str> = vec![];
    for entity in entities {
        if let Some(candidate) = entity["tag"].as_str().filter(|tag| !tag.is_empty()) {
            if !tags.contains(&candidate) {
                tags.push(candidate);
            }
        }
    }
    tag.and_then(|tag| tags.iter().position(|candidate| *candidate == tag))
        .map_or("database", |index| HUES[index % HUES.len()])
}

/// The drawn route of a bundled member skips its trunk stretches.
fn relation_path(relation: &Relation) -> String {
    let runs = relation
        .junction
        .as_deref()
        .and_then(trunks::parse_junction)
        .map(|(axis, coordinate)| trunks::trunk_runs(&relation.points, axis, coordinate))
        .unwrap_or_default();
    if runs.is_empty() {
        return crate::geometry::rounded_path(&relation.points, 8.0);
    }
    trunks::drawn_pieces(&relation.points, &runs)
        .iter()
        .map(|piece| crate::geometry::rounded_path(piece, 8.0))
        .collect::<Vec<_>>()
        .join(" ")
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let locale = crate::i18n::resolve(&scene.meta);
    // Domain bands sit behind the routes so a channel is never washed out.
    for band in scene
        .containers
        .iter()
        .filter(|container| container.authored.kind == "erd-domain")
    {
        let tag = band.authored.facts["tag"].as_str().unwrap_or("");
        let pill = band.label_plate.unwrap_or_default();
        write!(out, "<g data-domain-band=\"{}\" class=\"er-hue-{}\" data-source-path=\"{}\"><rect {} rx=\"12\" class=\"c-lane er-band\" stroke-width=\"1\"/><rect {} rx=\"7.5\" class=\"c-mask\"/><rect {} rx=\"7.5\" class=\"er-band-pill\"/>",
            escape_text(tag), hue_class(scene, Some(tag)), escape_text(&band.authored.source_path), rect(&band.bounds), rect(&pill), rect(&pill)).unwrap();
        for caption in &band.text {
            out.push_str(&text_element(
                caption,
                "data-domain-label=\"\" class=\"t-muted er-band-label\" font-weight=\"700\"",
                true,
                &escape_text(&caption.content),
            ));
        }
        out.push_str("</g>");
    }
    // Trunks run under the branches so a glyph that reaches past one stays readable.
    for trunk in scene
        .containers
        .iter()
        .filter(|container| container.authored.kind == "erd-trunk")
    {
        let Some(rule) = trunk.rule else {
            continue;
        };
        write!(out, "<path data-er-trunk=\"\" data-composition-points=\"{}\" d=\"{}\" class=\"{}\" stroke-width=\"1.5\"/>",
            points_value(&rule), crate::geometry::polyline_path(&rule),
            variant_class(trunk.authored.facts["variant"].as_str().unwrap_or("default"))).unwrap();
    }
    for (index, relation) in scene.relations.iter().enumerate() {
        let facts = &relation.authored.facts;
        let width = facts["width"].as_f64().unwrap_or(1.5);
        write!(out, "<path {} data-er-ends=\"{}\" data-graph-role=\"edge\" data-composition-points=\"{}\" d=\"{}\" class=\"{}\" data-animate=\"edge\" style=\"--step:{}\" stroke-width=\"{}\" marker-start=\"url(#{})\" marker-end=\"url(#{})\"/>",
            edge_attributes(relation, index), ends(facts), points_value(&relation.points), relation_path(relation),
            variant_class(&relation.variant), index.min(12), number(width),
            marker_id(relation.marker_start, "start"), marker_id(relation.marker_end, "end")).unwrap();
    }
    let (entities, relationships) = match &scene.facts {
        DiagramFacts::Erd {
            entities,
            relationships,
        } => (entities.as_slice(), relationships.as_slice()),
        _ => (&[][..], &[][..]),
    };
    let steps = steps(entities, relationships, false);
    let entity_context = locale.text("node.context.erd", &[]);
    for node in &scene.nodes {
        let facts = &node.authored.facts;
        let (sublabel, tag) = (facts["sublabel"].as_str(), facts["tag"].as_str());
        // A key badge cannot say which column a foreign key references; the
        // table detail names it, after the segment the node index groups by.
        let targets: Vec<String> = facts["attributes"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|attribute| {
                let target = attribute["references"].as_str()?;
                let name = attribute["name"].as_str().unwrap_or_default();
                Some(format!("{name} → {target}"))
            })
            .collect();
        let context = if targets.is_empty() {
            entity_context.clone()
        } else {
            format!("{entity_context} \u{203a} {}", targets.join(", "))
        };
        let b = node.bounds;
        let r = 8.0;
        let header = b.y
            + node
                .detail_rows
                .first()
                .map_or(30.0, |row| row.bounds.y - b.y);
        write!(out, "<g {} class=\"er-hue-{}\">{}<rect {} rx=\"8\" class=\"c-mask\"/><rect data-graph-role=\"node-body\" {} rx=\"8\" class=\"c-database er-table\" data-animate=\"node\" style=\"--step:{}\" stroke-width=\"1.2\"/>",
            node_attributes(&locale, &node.authored.id, &node.label, &node.authored.source_path,
                &[("kind", Some("database")), ("sublabel", sublabel), ("tag", tag), ("context", Some(&context))],
                &[sublabel, Some(&context)]),
            hue_class(scene, tag), node_title(&[Some(&node.label), sublabel, Some(&context), tag]),
            rect(&b), rect(&b), steps.get(&node.authored.id).copied().unwrap_or(0).min(12)).unwrap();
        write!(out, "<path class=\"er-table-header\" d=\"M {} {} H {} A 8 8 0 0 1 {} {} V {} H {} V {} A 8 8 0 0 1 {} {} Z\"/><path class=\"er-table-cap\" d=\"M {} {} H {}\" stroke-width=\"2.6\" stroke-linecap=\"round\" fill=\"none\"/>",
            number(b.x + r), number(b.y), number(b.x + b.width - r), number(b.x + b.width), number(b.y + r), number(header),
            number(b.x), number(b.y + r), number(b.x + r), number(b.y), number(b.x + 12.0), number(b.y + 1.4), number(b.x + b.width - 12.0)).unwrap();
        if let Some(sigil) = &node.sigil {
            let (selected, shape) = sigil_shape(&sigil.kind);
            write!(out, "<g aria-hidden=\"true\" data-semantic-sigil=\"{selected}\" class=\"semantic-sigil s-database\" transform=\"translate({} {}) scale({})\">{shape}</g>",
                number(sigil.position.x), number(sigil.position.y), number(sigil.size / 16.0)).unwrap();
        }
        for title in &node.text {
            out.push_str(&text_element(
                title,
                "data-node-label=\"\" class=\"t-primary er-title\" font-weight=\"700\"",
                true,
                &escape_text(&title.content),
            ));
        }
        write!(out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"c-grid er-header-rule\" stroke-width=\"1\"/>",
            number(b.x), number(header), number(b.x + b.width), number(header)).unwrap();
        for (index, row) in node.detail_rows.iter().enumerate() {
            let top = row.bounds.y;
            write!(out, "<g data-detail=\"context\" data-er-row=\"{index}\" data-source-path=\"{}\"><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"c-grid er-row-rule\" stroke-width=\"0.5\"/>",
                escape_text(&row.source_path), number(b.x), number(top), number(b.x + b.width), number(top)).unwrap();
            if index % 2 == 1 {
                write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" class=\"er-zebra\"/>",
                    number(b.x + 4.0), number(top + 1.0), number(b.width - 8.0), number(row.bounds.height - 2.0)).unwrap();
            }
            let primary = row
                .text
                .iter()
                .any(|text| text.content == "PK" && text.source_path.contains("/key"));
            for text in &row.text {
                let content = escape_text(&text.content);
                if text.source_path.contains("/key") {
                    let role = text.content.to_lowercase();
                    let accent = match role.as_str() {
                        "pk" => "t-database",
                        "fk" => "t-messagebus",
                        _ => "t-muted",
                    };
                    write!(out, "<rect x=\"{}\" y=\"{}\" width=\"22\" height=\"13\" rx=\"3\" class=\"er-key-badge\" data-er-badge=\"{}\"/>",
                        number(text.position.x - KEY_BADGE_W / 2.0), number(text.position.y - 10.0), escape_text(&role)).unwrap();
                    // The stylesheet centres the badge letters.
                    out.push_str(&text_element(
                        text,
                        &format!("data-detail=\"context\" class=\"{accent}\" font-weight=\"600\""),
                        false,
                        &content,
                    ));
                } else if text.source_path.ends_with("/type") {
                    out.push_str(&text_element(
                        text,
                        "data-detail=\"context\" class=\"er-type\"",
                        true,
                        &content,
                    ));
                } else {
                    let class = if primary {
                        "t-primary er-name-pk"
                    } else {
                        "t-primary"
                    };
                    out.push_str(&text_element(
                        text,
                        &format!("data-detail=\"context\" class=\"{class}\""),
                        true,
                        &content,
                    ));
                }
            }
            out.push_str("</g>");
        }
        out.push_str("</g>");
    }
    for (index, relation) in scene.relations.iter().enumerate() {
        let (Some(plate), Some(label)) = (relation.label_plate, relation.text.first()) else {
            continue;
        };
        let accent = match relation.authored.facts["variant"].as_str() {
            Some("security") => "t-security",
            Some("emphasis") => "t-backend",
            Some("dashed") => "t-messagebus",
            _ => "t-muted",
        };
        write!(out, "<g data-detail=\"context\" {} data-er-ends=\"{}\"><rect {} rx=\"3\" class=\"c-mask\"/>",
            edge_attributes(relation, index), ends(&relation.authored.facts), rect(&plate)).unwrap();
        out.push_str(&text_element(
            label,
            &format!("class=\"{accent}\""),
            true,
            &escape_text(&label.content),
        ));
        out.push_str("</g>");
    }
}

// A key swatch is the badge a field row draws; a relationship swatch is a short
// route meeting a table edge with the marker's own glyph.
fn legend_glyph(_scene: &CompiledDiagram, legend: &Card, baseline: f64, out: &mut String) {
    let kind = legend.authored.kind.as_str();
    let lead = legend.authored.facts["lead"].as_f64().unwrap_or(0.0);
    let x = legend.bounds.x + lead;
    let center = baseline - 4.0;
    let label = legend.text.first().map_or("", |text| text.content.as_str());
    let hit = format!("<rect class=\"er-legend-hit\" tabindex=\"0\" role=\"img\" aria-label=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"22\" rx=\"11\"/>",
        escape_text(label), number(x - 6.0), number(baseline - 15.0), number(legend.bounds.width - lead + 12.0));
    if crate::layout::erd::KEY_KINDS.contains(&kind) {
        let accent = match kind {
            "pk" => "t-database",
            "fk" => "t-messagebus",
            _ => "t-muted",
        };
        write!(out, "{hit}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"13\" rx=\"3\" class=\"er-key-badge\" data-er-badge=\"{kind}\"/><text x=\"{}\" y=\"{}\" class=\"{accent}\" text-anchor=\"middle\" font-size=\"9\" font-weight=\"600\">{}</text>",
            number(x), number(baseline - 10.0), number(crate::layout::erd::LEGEND_KEY_SWATCH),
            number(x + crate::layout::erd::LEGEND_KEY_SWATCH / 2.0), number(baseline), kind.to_uppercase()).unwrap();
        return;
    }
    if lead > 0.0 && legend.bounds.x > MARGIN {
        write!(out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"er-legend-divider\" stroke-width=\"1\"/>",
            number(legend.bounds.x - 4.0), number(center - 8.0), number(legend.bounds.x - 4.0), number(center + 8.0)).unwrap();
    }
    let edge = x + crate::layout::erd::LEGEND_END_SWATCH - 2.0;
    let glyph = match kind {
        "many" => "<path d=\"M 8 8 L 22 1 M 8 8 L 22 8 M 8 8 L 22 15\"/>",
        "one" => "<path d=\"M 17 1 L 17 15\"/>",
        _ => "<circle cx=\"3.5\" cy=\"8\" r=\"2.6\" class=\"c-mask\"/><circle cx=\"3.5\" cy=\"8\" r=\"2.6\"/>",
    };
    write!(out, "{hit}<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"er-legend-route\" stroke-width=\"1.5\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"er-legend-edge\" stroke-width=\"2\"/><g class=\"er-legend-glyph\" transform=\"translate({} {})\" style=\"stroke: var(--text-muted)\" stroke-width=\"1.75\" fill=\"none\">{glyph}</g>",
        number(x), number(center), number(edge), number(center), number(edge), number(center - 7.0), number(edge), number(center + 7.0),
        number(edge - 22.0), number(center - 8.0)).unwrap();
}
