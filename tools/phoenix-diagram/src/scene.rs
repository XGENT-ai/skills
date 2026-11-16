use crate::diagnostic::Diagnostics;
use crate::input::{DiagramType, Quality};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
#[serde(rename_all = "camelCase")]
pub struct BrandPlacement {
    pub bounds: Rect,
    pub resource: crate::brands::ResolvedBrand,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<BrandPlacement>,
    pub authored: Authored,
    pub bounds: Rect,
    pub sigil: Option<Sigil>,
    pub label: String,
    pub text: Vec<Text>,
    /// ERD columns and class members: context rows inside the node body.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detail_rows: Vec<DetailRow>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailRow {
    pub source_path: String,
    pub bounds: Rect,
    pub text: Vec<Text>,
    pub role: String,
}

/// Relationship end symbols. ERD ends are crow's-foot cardinalities; class ends
/// are UML notation. The end a marker sits on fixes its SVG orientation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Marker {
    ErOne,
    ErOneOptional,
    ErMany,
    ErManyOptional,
    Open,
    Triangle,
    DiamondFilled,
    DiamondHollow,
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
    // Absent for the original five types, which keep their variant arrowheads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_start: Option<Marker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_end: Option<Marker>,
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
    Erd {
        entities: Vec<Value>,
        relationships: Vec<Value>,
    },
    Class {
        types: Vec<Value>,
        relationships: Vec<Value>,
    },
    Tree {
        root: String,
        nodes: Vec<Value>,
        direction: String,
    },
    Timeline {
        timezone: String,
        evidence: String,
        lanes: Vec<Value>,
        events: Vec<TimelineEvent>,
        /// Compressed quiet periods as [from, to] instants in milliseconds.
        breaks: Vec<[i64; 2]>,
    },
    Waterfall {
        unit: String,
        evidence: String,
        spans: Vec<WaterfallSpan>,
    },
    /// The drawn projection of an explored model: chart states (leaves and
    /// parent titles), arrows, and the model verdict the receipt reports.
    Statechart {
        regions: Vec<String>,
        states: Vec<Value>,
        arrows: Vec<Value>,
        model: Value,
    },
    /// Numeric rows as authored, the resolved series (id, name, key) and the
    /// one table shared by the reader details and the static figure data.
    Chart {
        form: String,
        orientation: String,
        unit: String,
        evidence: String,
        series: Vec<Value>,
        rows: Vec<Value>,
        table: Value,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: String,
    pub lane: Option<String>,
    pub at_ms: i64,
    pub precision: String,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaterfallSpan {
    pub id: String,
    pub parent: Option<String>,
    pub start: f64,
    /// The recorded end; an incomplete span without one is open.
    pub end: Option<f64>,
    pub status: String,
    pub service: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCounts {
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
}

/// Single-input change marking: effective per-element marks, counts and the
/// description band added below the canvas. Present only when the input
/// marks changes; rendering and checks read these facts instead of
/// re-deriving them from the authored document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeFacts {
    pub counts: ChangeCounts,
    /// Effective node marks by node id.
    pub nodes: BTreeMap<String, String>,
    /// Relation marks aligned with `relations` (`None` keeps a relation
    /// unmarked, including tree connections that follow the child node).
    pub relations: Vec<Option<String>>,
    /// Effective erd field marks by field source path.
    pub fields: BTreeMap<String, String>,
    /// erd trunk container id → the view where the trunk does not exist.
    pub trunks: BTreeMap<String, String>,
    /// Lifecycle junction points in scene order; each entry lists the views
    /// where the point is hidden.
    pub junctions: Vec<Vec<String>>,
    /// The description band added below the canvas.
    pub band: ChangeBand,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeBand {
    pub bounds: Rect,
    pub changes: Vec<Text>,
    pub before: Vec<Text>,
    pub after: Vec<Text>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledDiagram {
    pub diagram_type: DiagramType,
    pub schema_version: u32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<ChangeFacts>,
    pub diagnostics: Diagnostics,
}
