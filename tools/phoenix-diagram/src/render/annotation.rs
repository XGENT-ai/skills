// Word-by-word annotation figure: sentence lines rebuild their text from the
// recorded runs, and every numbered span gets a card whose quoted text, fix and
// note come from the authored annotation.
use super::{escape_text, TypeRenderer};
use crate::format::number::js_to_string;
use crate::geometry::format_number as number;
use crate::i18n::Locale;
use crate::scene::{Card, CompiledDiagram, Container, DiagramFacts, Node, Text};
use serde_json::Value;
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

const MARGIN: f64 = 28.0;
const TEXT_FONT: f64 = 15.0;
const CARD_PAD: f64 = 10.0;

fn root_attributes(scene: &CompiledDiagram) -> String {
    let DiagramFacts::Annotation { .. } = &scene.facts else {
        return String::new();
    };
    " data-annotation-ui=\"\" data-reader-min-text=\"7.5\" data-node-kind-derived=\"\"".into()
}

fn defs(_scene: &CompiledDiagram) -> String {
    // Every rule starts with the annotation root, so the figure stylesheet only
    // carries these declarations and the static figure keeps the same markup.
    concat!(
        "<style>",
        "svg[data-annotation-ui] .an-sentence { white-space: pre; }",
        "svg[data-annotation-ui] .an-span { fill: var(--backend-stroke); font-weight: 700; }",
        "svg[data-annotation-ui] .an-span.an-issue { fill: var(--security-stroke); }",
        "svg[data-annotation-ui] .an-marker { fill: var(--text-dim); }",
        "svg[data-annotation-ui] .an-number { fill: var(--text-dim); font-weight: 700; }",
        "svg[data-annotation-ui] .an-quote { font-weight: 650; }",
        "svg[data-annotation-ui] .an-fix { fill: var(--text-dim); }",
        "svg[data-annotation-ui] .an-card { stroke-width: 1; }",
        "svg[data-annotation-ui] .an-card-issue { stroke-dasharray: 4 3; }",
        "svg[data-annotation-ui] .an-issue-tag { fill: var(--security-stroke); font-weight: 700; }",
        // A soft card shadow blurs the dashed issue border and the small numbers.
        "svg[data-annotation-ui] [data-node-id] > rect { filter: none !important; }",
        "</style>",
    )
    .into()
}

fn legend_glyph(_scene: &CompiledDiagram, _card: &Card, _size: f64, _out: &mut String) {}

fn text_element(text: &Text, class: &str, extra: &str, out: &mut String) {
    write!(
        out,
        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{}\" class=\"{}\" data-source-path=\"{}\"{}>{}</text>",
        number(text.position.x),
        number(text.position.y),
        number(text.font_size),
        escape_text(&text.anchor),
        class,
        escape_text(&text.source_path),
        extra,
        escape_text(&text.content)
    )
    .unwrap();
}

fn heading(out: &mut String, container: &Container) {
    for text in &container.text {
        if text.source_path.ends_with("/title") {
            text_element(text, "t-primary", " font-weight=\"700\"", out);
        } else {
            text_element(text, "t-muted", "", out);
        }
    }
}

// The recorded runs index the authored sentence; the renderer only emits the
// slices, never re-finds a span.
fn sentence_line(sentence: &Value, index: usize, line: &Value, out: &mut String) {
    let characters: Vec<char> = sentence["text"]
        .as_str()
        .unwrap_or_default()
        .chars()
        .collect();
    let id = sentence["id"].as_str().unwrap_or_default();
    write!(
        out,
        "<text class=\"t-primary an-sentence\" data-annotation-line=\"{}\" data-source-path=\"/sentences/{}/text\" x=\"{}\" y=\"{}\" font-size=\"{}\">",
        escape_text(id),
        index,
        number(MARGIN),
        number(line["baseline"].as_f64().unwrap_or_default()),
        number(TEXT_FONT)
    )
    .unwrap();
    let mut restore = false;
    for run in line["runs"].as_array().map_or(&[][..], Vec::as_slice) {
        let start = run["start"].as_u64().unwrap_or_default() as usize;
        let end = run["end"].as_u64().unwrap_or_default() as usize;
        let number_text = run["number"]
            .as_u64()
            .map(|value| js_to_string(value as f64))
            .unwrap_or_default();
        let issue = run["issue"] == true;
        let dy = if restore { " dy=\"5\"" } else { "" };
        match run["kind"].as_str().unwrap_or_default() {
            "marker" => {
                write!(
                    out,
                    "<tspan class=\"an-marker\" data-annotation-marker=\"{}\" font-size=\"10\" dy=\"-5\">{}{}</tspan>",
                    number_text,
                    number_text,
                    if issue { "!" } else { "" }
                )
                .unwrap();
                restore = true;
                continue;
            }
            "span" => {
                write!(
                    out,
                    "<tspan class=\"an-span{}\" data-annotation-ref=\"{}\" font-weight=\"700\"{}>{}</tspan>",
                    if issue { " an-issue" } else { "" },
                    number_text,
                    dy,
                    escape_text(&characters[start..end].iter().collect::<String>())
                )
                .unwrap();
            }
            _ => {
                write!(
                    out,
                    "<tspan{}>{}</tspan>",
                    dy,
                    escape_text(&characters[start..end].iter().collect::<String>())
                )
                .unwrap();
            }
        }
        restore = false;
    }
    out.push_str("</text>");
}

fn number_of(facts: &DiagramFacts, id: &str) -> Option<u64> {
    let DiagramFacts::Annotation { sentences } = facts else {
        return None;
    };
    sentences
        .iter()
        .flat_map(|sentence| {
            sentence["annotations"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
        })
        .find(|annotation| annotation["node"].as_str() == Some(id))
        .and_then(|annotation| annotation["number"].as_u64())
}

fn card(locale: &Locale, scene: &CompiledDiagram, node: &Node, out: &mut String) {
    let facts = &node.authored.facts;
    let issue = facts["issue"] == true;
    let note = facts["note"].as_str().unwrap_or_default();
    let fix = facts["fix"].as_str().unwrap_or_default();
    let fix_label = locale.text("annotation.fix", &[]);
    let sublabel = if !note.is_empty() {
        note.to_owned()
    } else if !fix.is_empty() {
        format!("{fix_label} {fix}")
    } else {
        String::new()
    };
    let context = locale.text(
        if issue {
            "node.context.annotation.issue"
        } else {
            "node.context.annotation.note"
        },
        &[],
    );
    let aria = locale.text(
        "node.focus.detail",
        &[("label", &node.label), ("detail", &sublabel)],
    );
    let number_text = number_of(&scene.facts, &node.authored.id)
        .map(|value| js_to_string(value as f64))
        .unwrap_or_default();
    let id = escape_text(&node.authored.id);
    write!(
        out,
        "<g id=\"node-{id}\" data-node-id=\"{id}\" data-node-label=\"{}\" tabindex=\"0\" role=\"button\" aria-label=\"{}\" aria-pressed=\"false\" data-node-kind=\"{}\" data-node-sublabel=\"{}\" data-node-context=\"{}\" data-source-path=\"{}\" data-annotation-number=\"{}\"{}><title>{}</title>",
        escape_text(&node.label),
        escape_text(&aria),
        if issue { "security" } else { "backend" },
        escape_text(&sublabel),
        escape_text(&context),
        escape_text(&node.authored.source_path),
        number_text,
        if issue { " data-annotation-issue=\"\"" } else { "" },
        escape_text(&node.label)
    )
    .unwrap();
    let b = node.bounds;
    write!(
        out,
        "<rect data-graph-role=\"node-body\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" class=\"c-{} an-card{}\" stroke-width=\"1.5\"/>",
        number(b.x),
        number(b.y),
        number(b.width),
        number(b.height),
        if issue { "security" } else { "backend" },
        if issue { " an-card-issue" } else { "" }
    )
    .unwrap();
    let baseline = node
        .text
        .first()
        .map(|text| text.position.y)
        .unwrap_or(b.y + CARD_PAD + 11.0);
    write!(
        out,
        "<text class=\"an-number\" x=\"{}\" y=\"{}\" font-size=\"11\" font-weight=\"700\" text-anchor=\"start\">{}</text>",
        number(b.x + CARD_PAD),
        number(baseline),
        number_text
    )
    .unwrap();
    for text in &node.text {
        let path = text.source_path.as_str();
        if path.ends_with("/span") {
            text_element(
                text,
                "an-quote",
                " data-node-label=\"\" font-weight=\"650\"",
                out,
            );
        } else if path.ends_with("/fix") {
            text_element(text, "an-fix", "", out);
        } else {
            text_element(text, "t-muted", " data-detail=\"context\"", out);
        }
    }
    if issue {
        write!(
            out,
            "<text class=\"an-issue-tag\" x=\"{}\" y=\"{}\" font-size=\"10\" font-weight=\"700\" text-anchor=\"end\">{}</text>",
            number(b.x + b.width - CARD_PAD),
            number(baseline),
            escape_text(&locale.text("annotation.issue", &[]))
        )
        .unwrap();
    }
    out.push_str("</g>");
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let DiagramFacts::Annotation { sentences } = &scene.facts else {
        return;
    };
    let locale = crate::i18n::resolve(&scene.meta);
    for (index, sentence) in sentences.iter().enumerate() {
        let id = sentence["id"].as_str().unwrap_or_default();
        if let Some(container) = scene
            .containers
            .iter()
            .find(|container| container.authored.id == id)
        {
            heading(out, container);
        }
        for line in sentence["lines"].as_array().map_or(&[][..], Vec::as_slice) {
            sentence_line(sentence, index, line, out);
        }
    }
    for node in &scene.nodes {
        card(&locale, scene, node, out);
    }
}
