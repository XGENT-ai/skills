use super::{escape_text, TypeRenderer};
use crate::format::number::js_to_string;
use crate::geometry::format_number as number;
use crate::i18n::Locale;
use crate::scene::{Card, CompiledDiagram, Container, DiagramFacts, Node};
use serde_json::Value;
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

const TONES: [&str; 5] = ["frontend", "backend", "database", "cloud", "messagebus"];

fn root_attributes(scene: &CompiledDiagram) -> String {
    let DiagramFacts::Chart {
        form, orientation, ..
    } = &scene.facts
    else {
        return String::new();
    };
    let mut attributes = format!(
        " data-chart-ui=\"\" data-chart-form=\"{}\" data-reader-min-text=\"7.5\" data-node-kind-derived=\"\"",
        escape_text(form)
    );
    if form == "bar" {
        attributes.push_str(&format!(
            " data-chart-orientation=\"{}\"",
            escape_text(orientation)
        ));
    }
    attributes
}

fn defs(_scene: &CompiledDiagram) -> String {
    let mut style = String::from(concat!(
        "<style>",
        "svg[data-chart-ui] .ch-axis { stroke: var(--text-dim); }",
        "svg[data-chart-ui] .ch-grid { stroke: var(--lane-stroke); stroke-dasharray: 2 4; opacity: .7; }",
        "svg[data-chart-ui] .ch-zero { stroke: var(--text-dim); stroke-dasharray: none; opacity: .9; }",
        "svg[data-chart-ui] .ch-band { fill: var(--lane-fill); fill-opacity: 1; opacity: .45; }",
        "svg[data-chart-ui] .ch-band-alt { fill: var(--lane-fill); fill-opacity: 1; opacity: .25; }",
        "svg[data-chart-ui] .ch-bar { stroke: none; fill-opacity: .9; }",
        "svg[data-chart-ui] .ch-point { fill: var(--bg); stroke-width: 2; }",
        "svg[data-chart-ui] .ch-line { fill: none; stroke-width: 2; stroke-linejoin: round; }",
        "svg[data-chart-ui] .ch-slice { stroke: var(--bg); stroke-width: 1.5; }",
        "svg[data-chart-ui] .ch-track { fill: var(--lane-fill); stroke: var(--lane-stroke); stroke-width: 1; }",
        "svg[data-chart-ui] .ch-fill { stroke: none; fill-opacity: .9; }",
        // Dark themes' reds are lighter than their cyans, so an over-limit fill only
        // outweighs a normal one when the normal fill is quieter. Pie list swatches
        // share `.ch-fill` and must keep their sector's colour.
        "svg[data-chart-ui] .ch-fill.ch-limit-fill { fill-opacity: .55; }",
        "svg[data-chart-ui] .ch-fill.c-security { fill-opacity: .9; }",
        "svg[data-chart-ui] .ch-span { fill: none; stroke: var(--text-dim); stroke-width: 1.5; stroke-dasharray: 4 3; }",
        "svg[data-chart-ui] .ch-limit { stroke: var(--text-dim); stroke-width: 1.5; }",
        "svg[data-chart-ui] .ch-num { font-variant-numeric: tabular-nums; }",
        "svg[data-chart-ui] .ch-dim { opacity: .45; }",
        // Thin bars stay crisp: the classic preset's card shadow blurs their edges.
        "svg[data-chart-ui] [data-node-id] > rect { filter: none !important; }",
    ));
    for tone in TONES.into_iter().chain(["security", "external"]) {
        write!(
            style,
            "svg[data-chart-ui] .ch-bar.c-{tone} {{ fill: var(--{tone}-stroke); }}\
             svg[data-chart-ui] .ch-point.t-{tone} {{ stroke: var(--{tone}-stroke); }}\
             svg[data-chart-ui] .ch-line.t-{tone} {{ stroke: var(--{tone}-stroke); }}\
             svg[data-chart-ui] .ch-slice.c-{tone} {{ fill: var(--{tone}-stroke); }}\
             svg[data-chart-ui] .ch-fill.c-{tone} {{ fill: var(--{tone}-stroke); }}"
        )
        .unwrap();
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

fn body(scene: &CompiledDiagram, out: &mut String) {
    let DiagramFacts::Chart { evidence, .. } = &scene.facts else {
        return;
    };
    let locale = crate::i18n::resolve(&scene.meta);
    for badge in of_kind(scene, "chart-evidence") {
        let b = badge.bounds;
        write!(out, "<g data-chart-evidence=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"11\" class=\"c-messagebus\" stroke-width=\"1\"/><circle cx=\"{}\" cy=\"{}\" r=\"3\" class=\"t-messagebus\"/>",
            escape_text(evidence), number(b.x), number(b.y), number(b.width), number(b.height),
            number(b.x + 11.0), number(b.y + 11.0)).unwrap();
        for text in &badge.text {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-messagebus\" font-size=\"{}\" font-weight=\"700\" text-anchor=\"middle\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
        }
        out.push_str("</g>");
    }
    // The pie order note doubles as the non-colour key for reading the ring.
    for caption in of_kind(scene, "chart-caption") {
        for text in &caption.text {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-muted\" font-size=\"{}\" text-anchor=\"{}\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size),
                escape_text(&text.anchor), escape_text(&text.content)).unwrap();
        }
    }
    for axis in of_kind(scene, "chart-axis") {
        if let Some([start, end]) = axis.rule {
            write!(out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"ch-axis\" stroke-width=\"1\"/>",
                number(start.x), number(start.y), number(end.x), number(end.y)).unwrap();
        }
    }
    for tick in of_kind(scene, "chart-tick") {
        let (Some([start, end]), Some(label)) = (tick.rule, tick.text.first()) else {
            continue;
        };
        write!(out, "<line data-chart-tick=\"{0}\" x1=\"{1}\" y1=\"{2}\" x2=\"{3}\" y2=\"{4}\" class=\"ch-grid{5}\" stroke-width=\"1\"/><text x=\"{6}\" y=\"{7}\" class=\"t-muted ch-num\" font-size=\"{8}\" text-anchor=\"{9}\">{10}</text>",
            js_to_string(tick.authored.facts["value"].as_f64().unwrap_or_default()),
            number(start.x), number(start.y), number(end.x), number(end.y),
            if tick.authored.facts["zero"] == true { " ch-zero" } else { "" },
            number(label.position.x), number(label.position.y), number(label.font_size),
            escape_text(&label.anchor), escape_text(&label.content)).unwrap();
    }
    // Series polylines read before the rows so points sit on top.
    for line in of_kind(scene, "chart-series") {
        let points = line.authored.facts["points"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        if points.len() < 2 {
            continue;
        }
        let tone = line.authored.facts["tone"].as_str().unwrap_or("external");
        let mut d = String::new();
        for (index, point) in points.iter().enumerate() {
            let (x, y) = (
                number(point[0].as_f64().unwrap_or_default()),
                number(point[1].as_f64().unwrap_or_default()),
            );
            if index == 0 {
                write!(d, "M {x} {y}").unwrap();
            } else {
                write!(d, " L {x} {y}").unwrap();
            }
        }
        write!(
            out,
            "<path d=\"{d}\" class=\"t-{tone} ch-line\" data-chart-series=\"{}\"/>",
            line.authored.facts["series"].as_u64().unwrap_or_default()
        )
        .unwrap();
    }
    for (order, node) in scene.nodes.iter().enumerate() {
        if let Some(row) =
            of_kind(scene, "chart-row").find(|row| row.authored.id == node.authored.id)
        {
            write_row(&locale, order, node, row, out);
        }
    }
}

fn title_of(node: &Node, sublabel: &str) -> String {
    [node.label.as_str(), sublabel]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

fn write_row(locale: &Locale, order: usize, node: &Node, row: &Container, out: &mut String) {
    let facts = &row.authored.facts;
    let tone = facts["tone"].as_str().unwrap_or_default();
    let highlight = facts["highlight"] == true;
    let dim = facts["dim"] == true;
    let sublabel = facts["sublabel"].as_str().unwrap_or_default();
    let aria = locale.text(
        "node.focus.detail",
        &[("label", &node.label), ("detail", sublabel)],
    );
    write!(out, "<g id=\"node-{0}\" data-node-id=\"{0}\" data-node-label=\"{1}\" tabindex=\"0\" role=\"button\" aria-label=\"{2}\" aria-pressed=\"false\" data-node-kind=\"{3}\" data-node-sublabel=\"{4}\" data-source-path=\"{5}\"",
        escape_text(&node.authored.id), escape_text(&node.label), escape_text(&aria), escape_text(tone), escape_text(sublabel), escape_text(&node.authored.source_path)).unwrap();
    // The numeric attributes carry the row's values for the Reader and the
    // artifact checks; limits add value/limit/over in M2.
    if let Some(values) = node.authored.facts["values"].as_array() {
        let joined: Vec<String> = values
            .iter()
            .map(|value| js_to_string(value.as_f64().unwrap_or_default()))
            .collect();
        write!(
            out,
            " data-chart-values=\"{}\"",
            escape_text(&joined.join(","))
        )
        .unwrap();
    } else if facts["limit"].as_f64().is_some() {
        // limits rows carry value/limit/over for the Reader and the checks.
        write!(
            out,
            " data-chart-value=\"{}\" data-chart-limit=\"{}\" data-chart-over=\"{}\"",
            escape_text(
                &facts["value"]
                    .as_f64()
                    .map(js_to_string)
                    .unwrap_or_default()
            ),
            escape_text(&js_to_string(facts["limit"].as_f64().unwrap_or_default())),
            facts["over"] == true
        )
        .unwrap();
    }
    write!(
        out,
        "><title>{}</title>",
        escape_text(&title_of(node, sublabel))
    )
    .unwrap();
    let b = row.bounds;
    write!(out, "<rect data-graph-role=\"node-body\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" class=\"ch-band{}\"/>",
        number(b.x), number(b.y), number(b.width), number(b.height),
        if order % 2 == 1 { " ch-band-alt" } else { "" }).unwrap();
    let dim_class = if dim { " ch-dim" } else { "" };
    let form_name = facts["form"].as_str().unwrap_or("bar");
    let fill_class = if form_name == "limits" {
        " ch-limit-fill"
    } else {
        ""
    };
    for mark in facts["marks"].as_array().map_or(&[][..], Vec::as_slice) {
        let mark_tone = mark["tone"].as_str().unwrap_or(tone);
        match mark["kind"].as_str().unwrap_or("bar") {
            "point" => write!(out, "<circle cx=\"{}\" cy=\"{}\" r=\"3.5\" class=\"t-{mark_tone} ch-point{dim_class}\" data-chart-series=\"{}\"/>",
                number(mark["cx"].as_f64().unwrap_or_default()), number(mark["cy"].as_f64().unwrap_or_default()),
                mark["series"].as_u64().unwrap_or_default()).unwrap(),
            "slice" => write!(out, "<path d=\"{}\" class=\"c-{mark_tone} ch-slice{dim_class}\" fill-rule=\"evenodd\" data-chart-sweep=\"{}\"/>",
                escape_text(mark["path"].as_str().unwrap_or_default()),
                js_to_string(mark["sweep"].as_f64().unwrap_or_default())).unwrap(),
            "track" => write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"ch-track\"/>",
                number(mark["x"].as_f64().unwrap_or_default()), number(mark["y"].as_f64().unwrap_or_default()),
                number(mark["width"].as_f64().unwrap_or_default()), number(mark["height"].as_f64().unwrap_or_default())).unwrap(),
            "fill" => write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"c-{mark_tone} ch-fill{fill_class}{dim_class}\"/>",
                number(mark["x"].as_f64().unwrap_or_default()), number(mark["y"].as_f64().unwrap_or_default()),
                number(mark["width"].as_f64().unwrap_or_default()), number(mark["height"].as_f64().unwrap_or_default())).unwrap(),
            "span" => write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"ch-span{dim_class}\"/>",
                number(mark["x"].as_f64().unwrap_or_default()), number(mark["y"].as_f64().unwrap_or_default()),
                number(mark["width"].as_f64().unwrap_or_default()), number(mark["height"].as_f64().unwrap_or_default())).unwrap(),
            "limit-line" => write!(out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"ch-limit{dim_class}\" stroke-width=\"1.5\"/>",
                number(mark["x"].as_f64().unwrap_or_default()), number(mark["y1"].as_f64().unwrap_or_default()),
                number(mark["x"].as_f64().unwrap_or_default()), number(mark["y2"].as_f64().unwrap_or_default())).unwrap(),
            _ => write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"c-{mark_tone} ch-bar{dim_class}\" data-chart-series=\"{}\"/>",
                number(mark["x"].as_f64().unwrap_or_default()), number(mark["y"].as_f64().unwrap_or_default()),
                number(mark["width"].as_f64().unwrap_or_default()), number(mark["height"].as_f64().unwrap_or_default()),
                mark["series"].as_u64().unwrap_or_default()).unwrap(),
        }
    }
    let note = facts["note"].as_str().unwrap_or_default();
    for (index, text) in node.text.iter().enumerate() {
        if index == 0 {
            write!(out, "<text data-node-label=\"\" x=\"{}\" y=\"{}\" class=\"t-primary\" font-size=\"{}\" font-weight=\"{}\" text-anchor=\"{}\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size),
                if highlight { 750 } else { 500 },
                escape_text(&text.anchor), escape_text(&text.content)).unwrap();
            continue;
        }
        // Ordinal numbers and limits notes are context text, not annotations.
        let is_context = (form_name == "pie" && index == 1)
            || (form_name == "limits" && !note.is_empty() && index == 1);
        if is_context {
            write!(out, "<text x=\"{}\" y=\"{}\" class=\"t-muted\" font-size=\"{}\" text-anchor=\"{}\">{}</text>",
                number(text.position.x), number(text.position.y), number(text.font_size),
                escape_text(&text.anchor), escape_text(&text.content)).unwrap();
            continue;
        }
        let mut ordinal = index - 1;
        if form_name == "pie" {
            ordinal -= 1;
        }
        if form_name == "limits" && !note.is_empty() {
            ordinal -= 1;
        }
        let class = if form_name == "pie" && ordinal == 1 {
            "t-muted"
        } else {
            "t-primary"
        };
        let weight = if form_name == "limits" && ordinal == 1 {
            750
        } else {
            600
        };
        write!(out, "<text data-chart-value-label=\"{}\" x=\"{}\" y=\"{}\" class=\"{class} ch-num{}\" font-size=\"{}\" font-weight=\"{weight}\" text-anchor=\"{}\">{}</text>",
            ordinal, number(text.position.x), number(text.position.y), dim_class,
            number(text.font_size), escape_text(&text.anchor), escape_text(&text.content)).unwrap();
    }
    out.push_str("</g>");
}

fn legend_glyph(_scene: &CompiledDiagram, legend: &Card, baseline: f64, out: &mut String) {
    let facts = &legend.authored.facts;
    let kind = legend.authored.kind.as_str();
    if let Some(_series) = kind.strip_prefix("series:") {
        // A series entry pairs its colour block with the series key, never the
        // authored id, so long ids cannot spill out of the block.
        write!(out, "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"10\" rx=\"2.5\" class=\"c-{} ch-bar\" stroke-width=\"1.2\"/><text x=\"{}\" y=\"{}\" class=\"t-primary\" font-size=\"9\" font-weight=\"700\" text-anchor=\"middle\">{}</text>",
            number(legend.bounds.x), number(baseline - 10.0), facts["tone"].as_str().unwrap_or("external"),
            number(legend.bounds.x + 7.0), number(baseline - 1.5), escape_text(facts["key"].as_str().unwrap_or_default())).unwrap();
        return;
    }
    if kind == "limits-limit-only" {
        // Limit-only rows read as an outlined span, matching the row mark.
        write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"10\" rx=\"2.5\" class=\"ch-span\"/>",
            number(legend.bounds.x),
            number(baseline - 10.0)
        )
        .unwrap();
        return;
    }
    if kind == "limits-limit" {
        // The limit reads as a vertical stroke over the track.
        write!(
            out,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"ch-limit\" stroke-width=\"2\"/>",
            number(legend.bounds.x + 7.0),
            number(baseline - 12.0),
            number(legend.bounds.x + 7.0),
            number(baseline + 2.0)
        )
        .unwrap();
        return;
    }
    if kind == "limits-current" || kind == "limits-over" {
        write!(out, "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"10\" rx=\"2.5\" class=\"c-{} ch-fill ch-limit-fill\" stroke-width=\"1.2\"/>",
            number(legend.bounds.x), number(baseline - 10.0), facts["tone"].as_str().unwrap_or("external")).unwrap();
    }
}

// The reader's data table: the same cells the static figure ships as data.
pub(super) fn data_table(locale: &Locale, table: &Value) -> String {
    let mut html = format!(
        "<details class=\"chart-data\"><summary>{}</summary><div class=\"chart-data-scroll\"><table><thead><tr>",
        escape_text(&locale.text("chart.table.summary", &[]))
    );
    for column in table["columns"].as_array().map_or(&[][..], Vec::as_slice) {
        html.push_str(&format!(
            "<th scope=\"col\">{}</th>",
            escape_text(column.as_str().unwrap_or_default())
        ));
    }
    html.push_str("</tr></thead><tbody>");
    for row in table["rows"].as_array().map_or(&[][..], Vec::as_slice) {
        html.push_str("<tr>");
        for (index, cell) in row
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .enumerate()
        {
            let content = escape_text(cell.as_str().unwrap_or_default());
            if index == 0 {
                html.push_str(&format!("<th scope=\"row\">{content}</th>"));
            } else {
                html.push_str(&format!("<td>{content}</td>"));
            }
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table></div></details>");
    html
}
