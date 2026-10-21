use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{
    fitted_node_font_size_for_contract, rects_overlap_for_contract, text_units_for_contract,
};
use crate::input::{CompileOptions, DiagramInput, DiagramType, InputContract, Quality};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Relation, Sigil,
    Text,
};
use serde_json::{json, Value};

#[path = "sequence/legend.rs"]
mod legend;
#[path = "sequence/notes.rs"]
mod notes;
#[path = "sequence/segments.rs"]
mod segments;
#[path = "sequence/validation.rs"]
mod validation;

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
fn number(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or(f64::NAN)
}
fn point(value: &Value) -> Option<Point> {
    Some(Point {
        x: value.get(0)?.as_f64()?,
        y: value.get(1)?.as_f64()?,
    })
}
fn authored(
    value: &Value,
    path: String,
    id: String,
    kind: &str,
    provenance: Vec<String>,
) -> Authored {
    Authored {
        id,
        source_path: path,
        kind: kind.into(),
        facts: value.clone(),
        geometry_provenance: provenance,
    }
}
fn text(path: String, content: &str, x: f64, y: f64, font: f64, anchor: &str) -> Text {
    Text {
        source_path: path,
        content: content.into(),
        position: Point { x, y },
        font_size: font,
        anchor: anchor.into(),
    }
}
fn diagnostic(
    code: &str,
    path: &str,
    message: impl Into<String>,
    evidence: Value,
    fixes: Vec<String>,
) -> Diagnostic {
    let mut error = Diagnostic::error(Some("sequence"), code, path, message);
    error.stage = "layout".into();
    error.evidence = evidence;
    error.supported_fixes = fixes;
    error
}

#[derive(Clone, Copy)]
struct MessageGeometry {
    start: f64,
    end: f64,
    center: f64,
}

struct Layout<'a> {
    document: &'a Value,
    contract: InputContract,
    quality: Quality,
    legacy: bool,
    automatic: bool,
    spread: bool,
    width: f64,
    height: f64,
    participant_width: f64,
    centers: Vec<f64>,
    readable: bool,
    sublabel_minimum: f64,
    sublabel_preferred: f64,
    notes: Vec<Option<notes::Note>>,
    content_bottom: f64,
    legend_entries: Vec<legend::Entry>,
}

fn spread_width(width: f64, count: usize) -> f64 {
    (((width - 124.0) / count.max(1) as f64).round() - 24.0).clamp(86.0, 190.0)
}

fn spread_columns(width: f64, count: usize, card: f64) -> (f64, f64) {
    let count = count.max(1);
    let span = count as f64 * card + (count - 1) as f64 * 16.0;
    let margin = (width - 40.0 - span).clamp(40.0, 62.0);
    let gap = if count > 1 {
        (card + 16.0).max((width - 40.0 - margin - card) / (count - 1) as f64)
    } else {
        108.0
    };
    (margin, gap)
}

impl<'a> Layout<'a> {
    fn new(document: &'a Value, contract: InputContract, quality: Quality) -> Self {
        let legacy = contract == InputContract::Legacy217;
        let meta = &document["meta"];
        let automatic = point(&meta["viewBox"]).is_none();
        let spread = if legacy {
            meta["column_fit"] == "spread"
        } else {
            meta["column_fit"] != "fixed"
        };
        let count = list(&document["participants"]).len().max(1);
        let readable = !legacy && quality == Quality::Showcase;
        let mut layout = Self {
            document,
            contract,
            quality,
            legacy,
            automatic,
            spread,
            width: 920.0,
            height: 760.0,
            participant_width: 86.0,
            centers: vec![],
            readable,
            sublabel_minimum: 6.0,
            sublabel_preferred: 7.0,
            notes: vec![],
            content_bottom: 0.0,
            legend_entries: vec![],
        };
        layout.width = point(&meta["viewBox"]).map_or_else(
            || {
                if legacy {
                    920.0
                } else {
                    layout.automatic_width()
                }
            },
            |size| size.x,
        );
        if !legacy && readable && automatic && spread {
            layout.sublabel_minimum =
                6.0_f64.max((6.0 / (930.0 / layout.width).min(1.0) * 10.0).ceil() / 10.0);
            layout.sublabel_preferred = 7.0_f64.max(layout.sublabel_minimum);
        }
        let (width, margin, gap) = if !spread {
            (86.0, 62.0, 108.0)
        } else if legacy {
            let card = (((layout.width - 124.0) / count as f64).round() - 24.0).clamp(86.0, 190.0);
            (
                card,
                62.0,
                if count > 1 {
                    108.0_f64.max((layout.width - 40.0 - 62.0 - card) / (count - 1) as f64)
                } else {
                    108.0
                },
            )
        } else {
            let nominal = spread_width(layout.width, count);
            let mut card = nominal;
            if automatic
                && list(&document["participants"]).iter().any(|participant| {
                    layout.units(string(participant, "label")) * 6.8 > nominal + 6.0
                })
            {
                let required = list(&document["participants"])
                    .iter()
                    .map(|participant| {
                        let primary = layout.units(string(participant, "label")) * 6.8 - 6.0;
                        let sub = if string(participant, "sublabel").is_empty() {
                            0.0
                        } else {
                            layout.units(string(participant, "sublabel"))
                                * layout.sublabel_minimum
                                * 0.6
                                + 8.0
                        };
                        primary.max(sub)
                    })
                    .fold(nominal, f64::max)
                    .ceil();
                let capacity = ((layout.width - 80.0 - (count - 1) as f64 * 16.0) / count as f64)
                    .min(190.0)
                    .floor();
                if required <= capacity {
                    card = required;
                }
            }
            let (margin, gap) = spread_columns(layout.width, count, card);
            (card, margin, gap)
        };
        layout.participant_width = width;
        let left = if spread { margin + width / 2.0 } else { margin };
        layout.centers = (0..count).map(|index| left + index as f64 * gap).collect();
        layout.notes = list(&document["messages"])
            .iter()
            .map(|message| notes::measure(&layout, message))
            .collect();
        for (index, message) in list(&document["messages"]).iter().enumerate() {
            let y = number(message, "y");
            let bottom = if !string(message, "note").is_empty() {
                layout.notes[index]
                    .as_ref()
                    .map_or(y + 22.0, |note| note.bottom)
            } else {
                y + 6.0
            };
            layout.content_bottom = layout.content_bottom.max(bottom);
        }
        for value in list(&document["activations"])
            .iter()
            .chain(list(&document["segments"]))
        {
            layout.content_bottom = layout.content_bottom.max(number(value, "to"));
        }
        layout.legend_entries = legend::entries(&layout);
        layout.height = point(&meta["viewBox"]).map_or_else(
            || {
                if legacy {
                    760.0
                } else {
                    let timeline = list(&document["messages"])
                        .iter()
                        .map(|message| number(message, "y") + 83.0)
                        .fold(layout.content_bottom + 65.0, f64::max);
                    327.0_f64
                        .max(timeline)
                        .max(legend::required_height(&layout, layout.width))
                }
            },
            |size| size.y,
        );
        layout
    }
    fn units(&self, text: &str) -> f64 {
        text_units_for_contract(text, self.contract)
    }
    fn participant_index(&self, id: &str) -> Option<usize> {
        list(&self.document["participants"])
            .iter()
            .position(|participant| string(participant, "id") == id)
    }
    fn header_height(&self) -> f64 {
        if self.legacy {
            54.0
        } else {
            60.0
        }
    }
    fn participant_bounds(&self, index: usize) -> Rect {
        Rect {
            x: self.centers[index] - self.participant_width / 2.0,
            y: 72.0,
            width: self.participant_width,
            height: self.header_height(),
        }
    }
    fn geometry(&self, message: &Value) -> Option<MessageGeometry> {
        let from = self.participant_index(string(message, "from"))?;
        let to = self.participant_index(string(message, "to"))?;
        let direction = if self.centers[to] > self.centers[from] {
            1.0
        } else {
            -1.0
        };
        let start = self.centers[from] + direction * 7.0;
        let end = self.centers[to] - direction * 7.0;
        Some(MessageGeometry {
            start,
            end,
            center: (start + end) / 2.0,
        })
    }
    fn label_rect(&self, message: &Value) -> Option<Rect> {
        let geometry = self.geometry(message)?;
        let width = (self.units(string(message, "label")) * if self.readable { 6.6 } else { 5.2 }
            + 12.0)
            .max(34.0);
        Some(Rect {
            x: geometry.center - width / 2.0,
            y: number(message, "y") - 20.0,
            width,
            height: if self.readable { 18.0 } else { 16.0 },
        })
    }
    fn route_rect(&self, message: &Value) -> Option<Rect> {
        let geometry = self.geometry(message)?;
        Some(Rect {
            x: geometry.start.min(geometry.end),
            y: number(message, "y") - 2.0,
            width: (geometry.end - geometry.start).abs(),
            height: 4.0,
        })
    }
    fn overlaps(&self, a: &Rect, b: &Rect, gap: f64) -> bool {
        rects_overlap_for_contract(a, b, gap, self.contract)
    }
    fn timeline_labels_fit(&self, width: f64) -> bool {
        let count = list(&self.document["participants"]).len().max(1);
        let card = spread_width(width, count);
        let (margin, gap) = spread_columns(width, count, card);
        let center = |index: usize| margin + card / 2.0 + index as f64 * gap;
        list(&self.document["messages"]).iter().all(|message| {
            let Some(from) = self.participant_index(string(message, "from")) else {
                return true;
            };
            let Some(to) = self.participant_index(string(message, "to")) else {
                return true;
            };
            let center = (center(from) + center(to)) / 2.0;
            let half = (self.units(string(message, "label"))
                * if self.readable { 6.6 } else { 5.2 }
                + 12.0)
                .max(34.0)
                / 2.0;
            center - half >= 0.0 && center + half <= width
        }) && list(&self.document["segments"]).iter().all(|segment| {
            56.0 + (self.units(string(segment, "label")) * 5.2 + 14.0).max(42.0) <= width - 48.0
        })
    }
    fn automatic_width(&self) -> f64 {
        if !self.spread {
            return 920.0;
        }
        let participants = list(&self.document["participants"]);
        let count = participants.len().max(1);
        let fits = |width: f64| {
            participants.iter().all(|participant| {
                self.units(string(participant, "label")) * 6.8 <= width + 6.0
                    && (string(participant, "sublabel").is_empty()
                        || self.units(string(participant, "sublabel")) * 6.0 * 0.6 <= width - 8.0)
            })
        };
        if fits(spread_width(920.0, count)) {
            if count <= 3 {
                for candidate in [560.0, 640.0, 720.0, 800.0, 920.0] {
                    let card = spread_width(candidate, count);
                    let span = count as f64 * card + (count - 1) as f64 * 16.0;
                    if fits(card) && candidate - 80.0 >= span && self.timeline_labels_fit(candidate)
                    {
                        return candidate;
                    }
                }
            }
            return 920.0;
        }
        let needed = participants
            .iter()
            .map(|participant| {
                (self.units(string(participant, "label")) * 6.8 - 6.0).max(
                    if string(participant, "sublabel").is_empty() {
                        0.0
                    } else {
                        self.units(string(participant, "sublabel")) * 7.0 * 0.6 + 8.0
                    },
                )
            })
            .fold(f64::NEG_INFINITY, f64::max);
        1085.0_f64
            .min(920.0_f64.max(((190.0_f64.min(needed) + 25.0) * count as f64 + 124.0).ceil()))
    }
    fn note_rect_at(&self, index: usize, y: f64) -> Option<Rect> {
        self.notes[index].as_ref().and_then(|note| note.rect_at(y))
    }
    fn note_backdrop(&self, index: usize) -> Option<Rect> {
        if self.legacy {
            return None;
        }
        let note = self.notes[index].as_ref()?.rect()?;
        let bounds = Rect {
            x: note.x - 2.0,
            y: note.y - 2.0,
            width: note.width + 4.0,
            height: note.height + 4.0,
        };
        if !list(&self.document["segments"]).iter().any(|segment| {
            [number(segment, "from"), number(segment, "to")]
                .iter()
                .any(|border| *border >= bounds.y && *border <= bounds.y + bounds.height)
        }) {
            return None;
        }
        let covered =
            list(&self.document["messages"])
                .iter()
                .enumerate()
                .any(|(other, message)| {
                    other != index
                        && [
                            self.label_rect(message),
                            self.route_rect(message),
                            self.notes[other].as_ref().and_then(notes::Note::rect),
                        ]
                        .into_iter()
                        .flatten()
                        .any(|rect| self.overlaps(&bounds, &rect, 0.0))
                });
        (!covered).then_some(bounds)
    }
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    if input.diagram_type != DiagramType::Sequence {
        return Err(vec![diagnostic(
            "sequence/wrong-type",
            "/diagram_type",
            "Sequence compiler requires sequence input",
            Value::Null,
            vec![],
        )]);
    }
    let layout = Layout::new(&input.document, input.input_contract, input.quality);
    let labels: Vec<_> = list(&input.document["segments"])
        .iter()
        .enumerate()
        .map(|(index, segment)| segments::label(&layout, segment, Some(index)))
        .collect();
    let (mut legend, legend_errors) = legend::measure(&layout);
    let mut errors = validation::validate(&layout, &labels, !legend.is_empty());
    errors.extend(legend_errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    let document = &input.document;
    let mut nodes = vec![];
    for (index, participant) in list(&document["participants"]).iter().enumerate() {
        let path = format!("/participants/{index}");
        let bounds = layout.participant_bounds(index);
        let center = layout.centers[index];
        let label = string(participant, "label");
        let mut texts = vec![text(
            format!("{path}/label"),
            label,
            center,
            72.0 + if layout.legacy { 22.0 } else { 36.0 },
            fitted_node_font_size_for_contract(
                label,
                layout.participant_width,
                11.0,
                8.0,
                input.input_contract,
            ),
            "middle",
        )];
        let sub = string(participant, "sublabel");
        if !sub.is_empty() {
            texts.push(text(
                format!("{path}/sublabel"),
                sub,
                center,
                72.0 + if layout.legacy { 39.0 } else { 50.0 },
                fitted_node_font_size_for_contract(
                    sub,
                    layout.participant_width,
                    layout.sublabel_preferred,
                    layout.sublabel_minimum,
                    input.input_contract,
                ),
                "middle",
            ));
        }
        let kind = if layout.legacy {
            string(participant, "type")
        } else {
            participant["icon"]
                .as_str()
                .unwrap_or(string(participant, "type"))
        };
        nodes.push(Node {
            authored: authored(
                participant,
                path.clone(),
                string(participant, "id").into(),
                string(participant, "type"),
                vec![
                    path,
                    if layout.automatic {
                        "automatic-sequence-canvas".into()
                    } else {
                        "/meta/viewBox".into()
                    },
                    if document["meta"].get("column_fit").is_some() {
                        "/meta/column_fit".into()
                    } else if layout.legacy {
                        "default-column-fit:fixed".into()
                    } else {
                        "default-column-fit:spread".into()
                    },
                ],
            ),
            bounds,
            sigil: (kind != "none").then(|| Sigil {
                kind: kind.into(),
                position: Point {
                    x: bounds.x + 6.0,
                    y: 78.0,
                },
                size: 11.0,
            }),
            label: label.into(),
            text: texts,
            detail_rows: Vec::new(),
        });
    }
    let title_y = legend
        .iter()
        .find(|item| item.authored.kind == "legend-title")
        .map(|item| item.text[0].position.y);
    let lifeline_bottom = layout.height - 65.0;
    let lifeline_end = if layout.legacy {
        lifeline_bottom
    } else {
        title_y.map_or(lifeline_bottom, |title| lifeline_bottom.min(title - 22.0))
    };
    let mut containers = vec![];
    for (index, participant) in list(&document["participants"]).iter().enumerate() {
        let x = layout.centers[index];
        containers.push(Container {
            authored: authored(
                participant,
                format!("/participants/{index}"),
                format!("lifeline-{}", string(participant, "id")),
                "lifeline",
                vec![
                    format!("/participants/{index}"),
                    "resolved-timeline-and-legend-band".into(),
                ],
            ),
            bounds: Rect {
                x,
                y: 142.0,
                width: 0.0,
                height: lifeline_end - 142.0,
            },
            label_plate: None,
            rule: Some([Point { x, y: 142.0 }, Point { x, y: lifeline_end }]),
            members: vec![string(participant, "id").into()],
            text: vec![],
        });
    }
    for (index, activation) in list(&document["activations"]).iter().enumerate() {
        let participant = layout
            .participant_index(string(activation, "participant"))
            .expect("validated activation participant");
        containers.push(Container {
            authored: authored(
                activation,
                format!("/activations/{index}"),
                format!("activation-{index}"),
                "activation",
                vec![
                    format!("/activations/{index}/participant"),
                    format!("/activations/{index}/from"),
                    format!("/activations/{index}/to"),
                ],
            ),
            bounds: Rect {
                x: layout.centers[participant] - 5.0,
                y: number(activation, "from"),
                width: 10.0,
                height: number(activation, "to") - number(activation, "from"),
            },
            label_plate: None,
            rule: None,
            members: vec![string(activation, "participant").into()],
            text: vec![],
        });
    }
    for (index, segment) in list(&document["segments"]).iter().enumerate() {
        containers.push(Container {
            authored: authored(
                segment,
                format!("/segments/{index}"),
                format!("segment-{index}"),
                "segment",
                vec![
                    format!("/segments/{index}/from"),
                    format!("/segments/{index}/to"),
                    "measured-segment-title".into(),
                ],
            ),
            bounds: Rect {
                x: 48.0,
                y: number(segment, "from"),
                width: layout.width - 96.0,
                height: number(segment, "to") - number(segment, "from"),
            },
            label_plate: Some(labels[index]),
            rule: None,
            members: list(&document["participants"])
                .iter()
                .map(|participant| string(participant, "id").into())
                .collect(),
            text: vec![text(
                format!("/segments/{index}/label"),
                string(segment, "label"),
                labels[index].x + 6.0,
                labels[index].y + 13.0,
                9.0,
                "start",
            )],
        });
    }
    let relations = list(&document["messages"])
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let geometry = layout
                .geometry(message)
                .expect("validated message endpoints");
            let y = number(message, "y");
            let label = string(message, "label");
            let mut texts = vec![text(
                format!("/messages/{index}/label"),
                label,
                geometry.center,
                y - 10.0,
                if layout.readable { 11.0 } else { 9.0 },
                "middle",
            )];
            if let Some(note) = &layout.notes[index] {
                let lines = if note.lines.is_empty() {
                    vec![string(message, "note").to_owned()]
                } else {
                    note.lines.clone()
                };
                for (line, value) in lines.iter().enumerate() {
                    texts.push(text(
                        format!("/messages/{index}/note"),
                        value,
                        note.x,
                        y + 18.0 + line as f64 * 11.0,
                        7.0,
                        "start",
                    ));
                }
            }
            Relation {
                authored: authored(
                    message,
                    format!("/messages/{index}"),
                    message["id"]
                        .as_str()
                        .map_or_else(|| format!("message-{index}"), str::to_owned),
                    "message",
                    vec![
                        format!("/messages/{index}/from"),
                        format!("/messages/{index}/to"),
                        format!("/messages/{index}/y"),
                        "participant-order".into(),
                    ],
                ),
                crossover_halo: false,
                independent_ports: false,
                junction: None,
                label_owner: None,
                from: string(message, "from").into(),
                to: string(message, "to").into(),
                variant: message["variant"].as_str().unwrap_or("default").into(),
                label: label.into(),
                note: message["note"].as_str().map(str::to_owned),
                note_bounds: layout.notes[index].as_ref().and_then(notes::Note::rect),
                note_backdrop: layout.note_backdrop(index),
                points: vec![
                    Point {
                        x: geometry.start,
                        y,
                    },
                    Point { x: geometry.end, y },
                ],
                label_plate: layout.label_rect(message),
                text: texts,
                marker_start: None,
                marker_end: None,
            }
        })
        .collect();
    let cards = list(&document["cards"])
        .iter()
        .enumerate()
        .map(|(index, value)| Card {
            authored: authored(
                value,
                format!("/cards/{index}"),
                format!("card-{index}"),
                "card",
                vec!["reader-card-flow".into()],
            ),
            bounds: Rect::default(),
            text: std::iter::once(text(
                format!("/cards/{index}/title"),
                string(value, "title"),
                0.0,
                0.0,
                14.0,
                "start",
            ))
            .chain(
                list(&value["items"])
                    .iter()
                    .enumerate()
                    .map(|(item, value)| {
                        text(
                            format!("/cards/{index}/items/{item}"),
                            value.as_str().unwrap_or(""),
                            0.0,
                            0.0,
                            12.0,
                            "start",
                        )
                    }),
            )
            .collect(),
        })
        .collect();
    Ok(CompiledDiagram {
        diagram_type: DiagramType::Sequence,
        schema_version: document["schema_version"]
            .as_f64()
            .expect("validated schema version") as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: document["meta"].clone(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width: layout.width,
            height: layout.height,
        },
        reader_fit: if !layout.legacy && layout.automatic {
            "width-first"
        } else {
            "legacy"
        }
        .into(),
        nodes,
        relations,
        containers,
        text: vec![],
        legends: std::mem::take(&mut legend),
        cards,
        facts: DiagramFacts::Sequence {
            participants: list(&document["participants"]).to_vec(),
            messages: list(&document["messages"]).to_vec(),
            activations: list(&document["activations"]).to_vec(),
            segments: list(&document["segments"]).to_vec(),
        },
        diagnostics: vec![],
    })
}
