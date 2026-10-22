use crate::diagnostic::Diagnostics;
use crate::input::{DiagramType, InputContract, Quality};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Authored {
    pub id: String,
    pub source_path: String,
    pub kind: String,
    // Retain all source facts, including explicit pins, alongside resolved geometry.
    pub facts: Value,
    pub geometry_provenance: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Sigil {
    pub kind: String,
    pub position: Point,
    pub size: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Node {
    pub authored: Authored,
    pub bounds: Rect,
    pub sigil: Option<Sigil>,
    pub label: String,
    pub text: Vec<Text>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    pub authored: Authored,
    pub crossover_halo: bool,
    pub independent_ports: bool,
    pub junction: Option<String>,
    pub label_owner: Option<usize>,
    pub from: String,
    pub to: String,
    pub variant: String,
    pub label: String,
    pub note: Option<String>,
    pub note_bounds: Option<Rect>,
    pub note_backdrop: Option<Rect>,
    pub points: Vec<Point>,
    pub label_plate: Option<Rect>,
    pub text: Vec<Text>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Container {
    pub authored: Authored,
    pub bounds: Rect,
    pub label_plate: Option<Rect>,
    pub rule: Option<[Point; 2]>,
    pub members: Vec<String>,
    pub text: Vec<Text>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Text {
    pub source_path: String,
    pub content: String,
    pub position: Point,
    pub font_size: f64,
    pub anchor: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Card {
    pub authored: Authored,
    pub bounds: Rect,
    pub text: Vec<Text>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecyclePresentation {
    pub initial_state: Option<String>,
    pub final_states: Vec<String>,
    pub junctions: Vec<Point>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DiagramFacts {
    Architecture {
        layout: Option<Value>,
        components: Vec<Value>,
        boundaries: Vec<Value>,
    },
    Workflow {
        semantic_checks: Option<Value>,
        lanes: Vec<Value>,
        phases: Vec<Value>,
        groups: Vec<Value>,
        main_path: Vec<String>,
    },
    Sequence {
        participants: Vec<Value>,
        messages: Vec<Value>,
        activations: Vec<Value>,
        segments: Vec<Value>,
    },
    Dataflow {
        stages: Vec<Value>,
        nodes: Vec<Value>,
        flows: Vec<Value>,
    },
    Lifecycle {
        presentation: LifecyclePresentation,
        states: Vec<Value>,
        transitions: Vec<Value>,
        lanes: Vec<Value>,
        main_path: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledDiagram {
    pub diagram_type: DiagramType,
    pub schema_version: u32,
    pub input_contract: InputContract,
    pub quality: Quality,
    pub meta: Value,
    pub view_box: Rect,
    pub reader_fit: String,
    pub nodes: Vec<Node>,
    pub relations: Vec<Relation>,
    pub containers: Vec<Container>,
    pub text: Vec<Text>,
    pub legends: Vec<Card>,
    pub cards: Vec<Card>,
    pub facts: DiagramFacts,
    pub diagnostics: Diagnostics,
}
