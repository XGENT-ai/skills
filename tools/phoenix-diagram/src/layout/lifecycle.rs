// Native ports of the two frozen lifecycle renderers; see NOTICE.md.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{fitted_node_font_size_for_contract, text_units_for_contract};
use crate::input::{CompileOptions, DiagramInput, DiagramType, InputContract};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Relation, Sigil,
    Text,
};
use serde_json::{json, Value};

#[path = "legacy.rs"]
mod legacy;
#[path = "lifecycle/presentation.rs"]
mod presentation;
#[path = "lifecycle/v3.rs"]
mod v3;

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
fn number(value: &Value, key: &str, default: f64) -> f64 {
    value[key].as_f64().unwrap_or(default)
}
fn point(value: &Value) -> Option<Point> {
    Some(Point {
        x: value.get(0)?.as_f64()?,
        y: value.get(1)?.as_f64()?,
    })
}
fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn text(
    path: impl Into<String>,
    content: impl Into<String>,
    x: f64,
    y: f64,
    font_size: f64,
    anchor: &str,
) -> Text {
    Text {
        source_path: path.into(),
        content: content.into(),
        position: p(x, y),
        font_size,
        anchor: anchor.into(),
    }
}
fn authored(raw: &Value, collection: &str, index: usize, kind: &str, pins: &[&str]) -> Authored {
    Authored {
        id: raw["id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{collection}-{index}")),
        source_path: format!("/{collection}/{index}"),
        kind: kind.into(),
        facts: raw.clone(),
        geometry_provenance: pins
            .iter()
            .filter(|pin| raw.get(**pin).is_some())
            .map(|pin| format!("/{collection}/{index}/{pin}"))
            .collect(),
    }
}
fn failure(code: &str, path: &str, message: impl Into<String>, evidence: Value) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(Some("lifecycle"), code, path, message);
    diagnostic.stage = "layout".into();
    diagnostic.evidence = evidence;
    diagnostic.supported_fixes=vec!["Preserve all lifecycle states, conditions and transitions while adjusting the identified layout constraint".into()];
    diagnostic
}
fn units(input: &DiagramInput, value: &str) -> f64 {
    text_units_for_contract(value, input.input_contract)
}
fn font(input: &DiagramInput, value: &str, width: f64, preferred: f64, minimum: f64) -> f64 {
    fitted_node_font_size_for_contract(value, width, preferred, minimum, input.input_contract)
}
fn generated(kind: &str, id: String, path: &str) -> Authored {
    Authored {
        id,
        source_path: path.into(),
        kind: kind.into(),
        facts: Value::Null,
        geometry_provenance: vec!["renderer-owned-lifecycle-presentation".into()],
    }
}
fn cards(input: &DiagramInput) -> Vec<Card> {
    list(&input.document["cards"])
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let mut texts = vec![text(
                format!("/cards/{index}/title"),
                string(raw, "title"),
                0.0,
                0.0,
                14.0,
                "start",
            )];
            texts.extend(
                list(&raw["items"])
                    .iter()
                    .enumerate()
                    .filter_map(|(item, value)| {
                        value.as_str().map(|content| {
                            text(
                                format!("/cards/{index}/items/{item}"),
                                content,
                                0.0,
                                item as f64 * 20.0 + 24.0,
                                12.0,
                                "start",
                            )
                        })
                    }),
            );
            Card {
                authored: authored(raw, "cards", index, "card", &[]),
                bounds: Rect::default(),
                text: texts,
            }
        })
        .collect()
}
fn scene(
    input: &DiagramInput,
    view_box: Rect,
    nodes: Vec<Node>,
    relations: Vec<Relation>,
    containers: Vec<Container>,
    legends: Vec<Card>,
    reader_fit: &str,
) -> CompiledDiagram {
    CompiledDiagram {
        diagram_type: DiagramType::Lifecycle,
        schema_version: number(&input.document, "schema_version", 1.0) as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: input.document["meta"].clone(),
        view_box,
        reader_fit: reader_fit.into(),
        nodes,
        relations,
        containers,
        text: vec![],
        legends,
        cards: cards(input),
        facts: DiagramFacts::Lifecycle {
            states: list(&input.document["states"]).to_vec(),
            transitions: list(&input.document["transitions"]).to_vec(),
            lanes: list(&input.document["lanes"]).to_vec(),
            main_path: list(&input.document["mainPath"])
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            presentation: Default::default(),
        },
        diagnostics: vec![],
    }
}

pub fn compile(
    input: &DiagramInput,
    options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    if input.diagram_type != DiagramType::Lifecycle {
        return Err(vec![failure(
            "lifecycle/wrong-type",
            "/diagram_type",
            "Lifecycle compiler requires lifecycle input",
            Value::Null,
        )]);
    }
    match input.input_contract {
        InputContract::Legacy217 => legacy::compile(input, options),
        InputContract::PhoenixV1 => v3::compile(input, options),
    }
}
