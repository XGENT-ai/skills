// Word-by-word annotation: each sentence keeps its authored text in one column
// and every marked span gets a numbered card. The original text is wrapped into
// lines whose runs record exact character ranges, so the renderer emits tspans
// without re-locating spans, and the content check can rebuild the sentence.
use crate::diagnostic::Diagnostics;
use crate::format::number::js_round;
use crate::geometry::text_units;
use crate::input::semantics::{annotation_spans, AnnotationSpan};
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::{Authored, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Text};
use serde_json::{json, Value};

use super::tree::{tokens, wrap_text};

const MARGIN: f64 = 28.0;
const WIDTH: f64 = 760.0;
const BODY_W: f64 = WIDTH - MARGIN * 2.0;
const TEXT_FONT: f64 = 15.0;
const TEXT_LINE: f64 = 26.0;
// First baseline below the sentence text block's top.
const TEXT_TOP: f64 = 20.0;
const TITLE_FONT: f64 = 13.0;
const ASIDE_FONT: f64 = 11.0;
const TITLE_ROW: f64 = 22.0;
const TITLE_TOP: f64 = 15.0;
const FOOTNOTE_FONT: f64 = 11.0;
const NUMBER_FONT: f64 = 11.0;
const SPAN_FONT: f64 = 12.5;
const FIX_FONT: f64 = 11.5;
const NOTE_FONT: f64 = 11.5;
const TAG_FONT: f64 = 10.0;
const MARKER_FONT: f64 = 10.0;
const CARD_LINE: f64 = 17.0;
const CARD_INSET: f64 = 16.0;
const CARD_PAD: f64 = 10.0;
const CARD_W: f64 = BODY_W - CARD_INSET;
const CARD_GAP: f64 = 10.0;
const INTRO_GAP: f64 = 12.0;
const SENTENCE_GAP: f64 = 30.0;
const ADVANCE: f64 = 0.6;

fn round(value: f64) -> f64 {
    js_round(value * 100.0) / 100.0
}

fn text(
    source_path: String,
    content: String,
    x: f64,
    y: f64,
    font_size: f64,
    anchor: &str,
) -> Text {
    Text {
        source_path,
        content,
        position: Point {
            x: round(x),
            y: round(y),
        },
        font_size,
        anchor: anchor.into(),
    }
}

fn container(authored: Authored, bounds: Rect, members: Vec<String>, text: Vec<Text>) -> Container {
    Container {
        authored,
        bounds,
        label_plate: None,
        rule: None,
        members,
        text,
    }
}

// What one character range of a line is drawn as.
#[derive(Clone, Copy, PartialEq)]
enum Run {
    Plain,
    Span { number: usize, issue: bool },
    Marker { number: usize, issue: bool },
}

#[derive(Clone, Copy)]
struct Piece {
    start: usize,
    end: usize,
    kind: Run,
    token_start: bool,
}

fn marker_text(number: usize, issue: bool) -> String {
    if issue {
        format!("{number}!")
    } else {
        number.to_string()
    }
}

fn slice(text: &[char], start: usize, end: usize) -> String {
    text[start..end].iter().collect()
}

fn width(piece: &Piece, text: &[char]) -> f64 {
    match piece.kind {
        Run::Marker { number, issue } => {
            text_units(&marker_text(number, issue)) * MARKER_FONT * ADVANCE + 4.0
        }
        _ => text_units(&slice(text, piece.start, piece.end)) * TEXT_FONT * ADVANCE,
    }
}

// One piece per authored token, split again at span edges so a span may begin
// or end inside a word; a span's marker follows its last character directly.
fn sentence_pieces(text: &str, spans: &[&AnnotationSpan]) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut offset = 0;
    for token in tokens(text) {
        let start = offset;
        let end = start + token.chars().count();
        offset = end;
        let mut bounds = vec![start, end];
        for span in spans {
            for edge in [span.start, span.end] {
                if edge > start && edge < end {
                    bounds.push(edge);
                }
            }
        }
        bounds.sort_unstable();
        bounds.dedup();
        for window in bounds.windows(2) {
            let (first, last) = (window[0], window[1]);
            let kind = spans
                .iter()
                .find(|span| span.start <= first && last <= span.end)
                .map(|span| Run::Span {
                    number: span.number,
                    issue: span.issue,
                })
                .unwrap_or(Run::Plain);
            pieces.push(Piece {
                start: first,
                end: last,
                kind,
                token_start: first == start,
            });
            if let Some(span) = spans.iter().find(|span| span.end == last) {
                pieces.push(Piece {
                    start: last,
                    end: last,
                    kind: Run::Marker {
                        number: span.number,
                        issue: span.issue,
                    },
                    token_start: false,
                });
            }
        }
    }
    pieces
}

// Greedy wrap before the token that would overflow; a single token wider than
// the column is split by characters. Trailing whitespace stays on its line.
fn wrap_lines(pieces: Vec<Piece>, text: &[char]) -> Vec<Vec<Piece>> {
    let mut lines: Vec<Vec<Piece>> = Vec::new();
    let mut current: Vec<Piece> = Vec::new();
    let mut used = 0.0;
    let mut index = 0;
    while index < pieces.len() {
        let start = index;
        index += 1;
        while index < pieces.len() && !pieces[index].token_start {
            index += 1;
        }
        let group = &pieces[start..index];
        let group_width: f64 = group.iter().map(|piece| width(piece, text)).sum();
        if !current.is_empty() && used + group_width > BODY_W {
            lines.push(std::mem::take(&mut current));
            used = 0.0;
        }
        if group_width <= BODY_W {
            current.extend_from_slice(group);
            used += group_width;
            continue;
        }
        for piece in group {
            if piece.end.saturating_sub(piece.start) <= 1
                || matches!(piece.kind, Run::Marker { .. })
            {
                let atom_width = width(piece, text);
                if !current.is_empty() && used + atom_width > BODY_W {
                    lines.push(std::mem::take(&mut current));
                    used = 0.0;
                }
                current.push(*piece);
                used += atom_width;
                continue;
            }
            for position in piece.start..piece.end {
                let atom = Piece {
                    start: position,
                    end: position + 1,
                    kind: piece.kind,
                    token_start: false,
                };
                let atom_width = width(&atom, text);
                if !current.is_empty() && used + atom_width > BODY_W {
                    lines.push(std::mem::take(&mut current));
                    used = 0.0;
                }
                current.push(atom);
                used += atom_width;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn runs_of(line: &[Piece]) -> Vec<Value> {
    let mut runs: Vec<(Run, usize, usize)> = Vec::new();
    for piece in line {
        if matches!(piece.kind, Run::Marker { .. }) {
            runs.push((piece.kind, piece.start, piece.end));
            continue;
        }
        if let Some(last) = runs.last_mut() {
            if last.0 == piece.kind {
                last.2 = piece.end;
                continue;
            }
        }
        runs.push((piece.kind, piece.start, piece.end));
    }
    runs.into_iter()
        .map(|(kind, start, end)| match kind {
            Run::Plain => json!({"kind": "plain", "start": start, "end": end}),
            Run::Span { number, issue } => {
                json!({"kind": "span", "start": start, "end": end, "number": number, "issue": issue})
            }
            Run::Marker { number, issue } => {
                json!({"kind": "marker", "start": start, "end": end, "number": number, "issue": issue})
            }
        })
        .collect()
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let locale = crate::i18n::resolve(meta);
    let spans = annotation_spans(document)?;
    let sentences = document["sentences"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let card_x = MARGIN + CARD_INSET;
    let fix_label = locale.text("annotation.fix", &[]);
    let issue_label = locale.text("annotation.issue", &[]);

    let mut nodes = Vec::new();
    let mut containers = Vec::new();
    let mut sentence_facts = Vec::new();
    let mut cursor = MARGIN;

    for (index, sentence) in sentences.iter().enumerate() {
        let id = sentence["id"].as_str().unwrap_or_default();
        let source = sentence["text"].as_str().unwrap_or_default();
        let characters: Vec<char> = source.chars().collect();
        let own: Vec<&AnnotationSpan> =
            spans.iter().filter(|span| span.sentence == index).collect();
        let lines = wrap_lines(sentence_pieces(source, &own), &characters);
        let block_top = cursor;
        let mut body_top = block_top;

        let mut heading = Vec::new();
        let title = sentence["title"].as_str();
        let aside = sentence["aside"].as_str();
        if let Some(title) = title {
            heading.push(text(
                format!("/sentences/{index}/title"),
                title.into(),
                MARGIN,
                block_top + TITLE_TOP,
                TITLE_FONT,
                "start",
            ));
        }
        if let Some(aside) = aside {
            // Both share one baseline unless their estimated widths collide.
            let title_w = title
                .map(|value| text_units(value) * TITLE_FONT * ADVANCE)
                .unwrap_or_default();
            let apart = title_w + text_units(aside) * ASIDE_FONT * ADVANCE + 24.0 > BODY_W;
            let baseline = block_top + TITLE_TOP + if apart { TITLE_ROW } else { 0.0 };
            heading.push(text(
                format!("/sentences/{index}/aside"),
                aside.into(),
                MARGIN + BODY_W,
                baseline,
                ASIDE_FONT,
                "end",
            ));
            if apart {
                body_top += TITLE_ROW;
            }
        }
        if title.is_some() || aside.is_some() {
            body_top += TITLE_ROW;
        }

        let mut line_facts = Vec::new();
        for (line_index, line) in lines.iter().enumerate() {
            let baseline = body_top + TEXT_TOP + line_index as f64 * TEXT_LINE;
            line_facts.push(json!({"baseline": round(baseline), "runs": runs_of(line)}));
        }
        let text_bottom = if lines.is_empty() {
            body_top
        } else {
            body_top + TEXT_TOP + (lines.len() - 1) as f64 * TEXT_LINE + 6.0
        };

        let mut card_top = text_bottom + INTRO_GAP;
        let mut members = Vec::new();
        let mut annotations = Vec::new();
        for span in &own {
            let raw = &sentence["annotations"][span.index];
            let mark = raw["span"].as_str().unwrap_or_default();
            let note = raw["note"].as_str().filter(|value| !value.is_empty());
            let fix = raw["fix"].as_str().filter(|value| !value.is_empty());
            let fix_line = fix.map(|value| format!("→ {fix_label} {value}"));
            let number_text = span.number.to_string();
            let number_w = text_units(&number_text) * NUMBER_FONT * ADVANCE;
            let span_x = card_x + CARD_PAD + number_w + 6.0;
            let tag_w = if span.issue {
                text_units(&issue_label) * TAG_FONT * ADVANCE + 8.0
            } else {
                0.0
            };
            let span_width = (card_x + CARD_W - CARD_PAD - span_x - tag_w).max(40.0);
            let span_lines = wrap_text(mark, SPAN_FONT, span_width);
            let fix_lines = fix_line
                .as_ref()
                .map(|line| wrap_text(line, FIX_FONT, CARD_W - 2.0 * CARD_PAD))
                .unwrap_or_default();
            let note_lines = note
                .map(|value| wrap_text(value, NOTE_FONT, CARD_W - 2.0 * CARD_PAD))
                .unwrap_or_default();

            let mut texts = Vec::new();
            let mut baseline = card_top + CARD_PAD + 11.0;
            for (line_index, line) in span_lines.iter().enumerate() {
                if line_index > 0 {
                    baseline += CARD_LINE;
                }
                texts.push(text(
                    format!("/sentences/{index}/annotations/{}/span", span.index),
                    line.clone(),
                    span_x,
                    baseline,
                    SPAN_FONT,
                    "start",
                ));
            }
            for line in &fix_lines {
                baseline += CARD_LINE;
                texts.push(text(
                    format!("/sentences/{index}/annotations/{}/fix", span.index),
                    line.clone(),
                    card_x + CARD_PAD,
                    baseline,
                    FIX_FONT,
                    "start",
                ));
            }
            for line in &note_lines {
                baseline += CARD_LINE;
                texts.push(text(
                    format!("/sentences/{index}/annotations/{}/note", span.index),
                    line.clone(),
                    card_x + CARD_PAD,
                    baseline,
                    NOTE_FONT,
                    "start",
                ));
            }
            let card_bottom = baseline + 6.0 + CARD_PAD;
            let node_id = format!("{id}-{}", span.index);
            members.push(node_id.clone());
            annotations.push(json!({
                "number": span.number,
                "node": node_id,
                "start": span.start,
                "end": span.end,
                "issue": span.issue,
            }));
            // The metadata graph lists one entry per node and the Reader keys
            // focus, search and detail by `id`; every other type authors it.
            let mut facts = raw.as_object().cloned().unwrap_or_default();
            facts.insert("id".to_owned(), Value::String(node_id.clone()));
            nodes.push(Node {
                brand: None,
                authored: Authored {
                    id: node_id,
                    source_path: format!("/sentences/{index}/annotations/{}", span.index),
                    kind: if span.issue {
                        "annotation-issue".into()
                    } else {
                        "annotation-note".into()
                    },
                    facts: Value::Object(facts),
                    geometry_provenance: vec!["authored-span".into()],
                },
                bounds: Rect {
                    x: card_x,
                    y: round(card_top),
                    width: CARD_W,
                    height: round(card_bottom - card_top),
                },
                sigil: None,
                label: mark.into(),
                text: texts,
                detail_rows: vec![],
            });
            card_top = card_bottom + CARD_GAP;
        }
        let cards_bottom = if own.is_empty() {
            text_bottom
        } else {
            card_top - CARD_GAP
        };

        let mut footnote_top = cards_bottom;
        if let Some(footnote) = sentence["footnote"].as_str() {
            footnote_top += 10.0;
            for (line_index, line) in wrap_text(footnote, FOOTNOTE_FONT, BODY_W)
                .iter()
                .enumerate()
            {
                heading.push(text(
                    format!("/sentences/{index}/footnote"),
                    line.clone(),
                    MARGIN,
                    footnote_top + 10.0 + line_index as f64 * CARD_LINE,
                    FOOTNOTE_FONT,
                    "start",
                ));
            }
            let count = wrap_text(footnote, FOOTNOTE_FONT, BODY_W).len();
            footnote_top += 10.0 + (count.max(1) - 1) as f64 * CARD_LINE + 6.0;
        }

        let block_bottom = footnote_top;
        containers.push(container(
            Authored {
                id: id.into(),
                source_path: format!("/sentences/{index}"),
                kind: "annotation-sentence".into(),
                facts: json!({"id": id}),
                geometry_provenance: vec!["authored-text".into()],
            },
            Rect {
                x: MARGIN,
                y: round(block_top),
                width: BODY_W,
                height: round(block_bottom - block_top),
            },
            members,
            heading,
        ));
        sentence_facts.push(json!({
            "id": id,
            "text": source,
            "lines": line_facts,
            "annotations": annotations,
        }));
        cursor = block_bottom + SENTENCE_GAP;
    }

    let height = if sentences.is_empty() {
        MARGIN * 2.0
    } else {
        cursor - SENTENCE_GAP + MARGIN
    };
    Ok(CompiledDiagram {
        diagram_type: DiagramType::Annotation,
        schema_version: document["schema_version"].as_u64().unwrap_or(1) as u32,
        quality: input.quality,
        meta: meta.clone(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width: WIDTH,
            height: round(height),
        },
        reader_fit: "width-first".into(),
        nodes,
        relations: vec![],
        containers,
        text: vec![],
        legends: vec![],
        cards: super::tree::cards(document),
        facts: DiagramFacts::Annotation {
            sentences: sentence_facts,
        },
        changes: None,
        diagnostics: vec![],
    })
}
