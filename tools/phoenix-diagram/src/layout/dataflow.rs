// Native port of the frozen legacy dataflow renderer and reference compiler; see NOTICE.md.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{self, composition, labels, Point, Rect};
use crate::input::{CompileOptions, DiagramInput, InputContract, Quality};
use crate::scene::{Authored, CompiledDiagram, Text};
use serde_json::{json, Value};

mod capacity;
mod measure;
mod repair;
mod routing;
mod validation;

fn list(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}
fn number(value: &Value, key: &str, default: f64) -> f64 {
    value[key].as_f64().unwrap_or(default)
}
fn has(value: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| value.get(*key).is_some())
}
fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn point(value: &Value) -> Point {
    p(value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}
fn component(point: Point, axis: usize) -> f64 {
    if axis == 0 {
        point.x
    } else {
        point.y
    }
}
fn set_component(point: &mut Point, axis: usize, value: f64) {
    if axis == 0 {
        point.x = value;
    } else {
        point.y = value;
    }
}
fn label_pinned(value: &Value) -> bool {
    has(value, &["labelAt", "labelDx", "labelDy", "labelSegment"])
}
fn geometry_pinned(value: &Value) -> bool {
    value.get("via").is_some()
        || (!string(value, "route").is_empty() && string(value, "route") != "auto")
        || has(
            value,
            &[
                "fromSide",
                "toSide",
                "channelX",
                "channelY",
                "labelAt",
                "labelDx",
                "labelDy",
                "labelSegment",
            ],
        )
}
fn failure(
    code: &str,
    path: impl Into<String>,
    message: impl Into<String>,
    evidence: Value,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(Some("dataflow"), code, &path.into(), message);
    diagnostic.stage = "layout".into();
    diagnostic.evidence = evidence;
    diagnostic
}
fn authored(value: &Value, collection: &str, index: usize, kind: &str, pins: &[&str]) -> Authored {
    let path = format!("/{collection}/{index}");
    let mut geometry_provenance: Vec<_> = pins
        .iter()
        .filter(|key| value.get(**key).is_some())
        .map(|key| format!("{path}/{key}"))
        .collect();
    if geometry_provenance.is_empty() {
        geometry_provenance.push(format!("dataflow-{collection}-defaults"));
    }
    Authored {
        id: value["id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{collection}-{index}")),
        source_path: path,
        kind: kind.into(),
        facts: value.clone(),
        geometry_provenance,
    }
}
fn text(path: String, content: &str, position: Point, font_size: f64, anchor: &str) -> Text {
    Text {
        source_path: path,
        content: content.into(),
        position,
        font_size,
        anchor: anchor.into(),
    }
}
fn units(input: &DiagramInput, value: &str) -> f64 {
    geometry::text_units_for_contract(value, input.input_contract)
}
fn fitted(input: &DiagramInput, value: &str, width: f64, preferred: f64, minimum: f64) -> f64 {
    geometry::fitted_node_font_size_for_contract(
        value,
        width,
        preferred,
        minimum,
        input.input_contract,
    )
}
fn unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}
fn length(points: &[Point]) -> f64 {
    points.windows(2).fold(0.0, |sum, pair| {
        sum + (pair[1].x - pair[0].x).abs() + (pair[1].y - pair[0].y).abs()
    })
}
fn related(a: &Flow, b: &Flow) -> bool {
    [a.from, a.to].iter().any(|id| *id == b.from || *id == b.to)
}
fn route_distance(points: &[Point], rect: &Rect) -> f64 {
    points
        .windows(2)
        .map(|pair| geometry::segment_rect_clearance(pair[0], pair[1], rect))
        .fold(f64::INFINITY, f64::min)
}
fn masks(points: &[Point], rect: &Rect) -> bool {
    route_distance(points, rect) + geometry::EPSILON < 4.0
}
fn intersections(a: &[Point], b: &[Point]) -> usize {
    a.windows(2)
        .map(|left| {
            b.windows(2)
                .filter(|right| {
                    geometry::proper_segment_intersection(left[0], left[1], right[0], right[1])
                        .is_some()
                })
                .count()
        })
        .sum()
}
fn rhythm(points: &[Point]) -> bool {
    composition::collect_route_rhythm_issues(
        &[composition::RoutedRelation::new(
            0,
            Value::Null,
            points.to_vec(),
        )],
        16.0,
        8.0,
    )
    .is_empty()
}
fn sides_ok(points: &[Point], sides: &(String, String)) -> bool {
    composition::route_honors_endpoint_sides(points, &sides.0, &sides.1)
}
fn folds_back(points: &[Point]) -> bool {
    points.windows(2).enumerate().any(|(index, a)| {
        points.windows(2).skip(index + 2).any(|b| {
            geometry::segment_intersects_rect(
                a[0],
                a[1],
                &Rect {
                    x: b[0].x.min(b[1].x),
                    y: b[0].y.min(b[1].y),
                    width: (b[1].x - b[0].x).abs(),
                    height: (b[1].y - b[0].y).abs(),
                },
                0.0,
            )
        })
    })
}

#[derive(Clone)]
struct MeasuredNode {
    raw: Value,
    bounds: Rect,
    cx: f64,
    cy: f64,
    inset: f64,
}
#[derive(Clone, Default)]
struct Ports {
    from: Option<Point>,
    to: Option<Point>,
}
#[derive(Clone)]
struct Flow {
    raw: Value,
    from: usize,
    to: usize,
    sides: (String, String),
    ports: Ports,
    points: Vec<Point>,
    halo: bool,
}
#[derive(Clone, PartialEq)]
struct Plate {
    label: labels::Label,
    lines: Vec<String>,
}
impl Plate {
    fn index(&self) -> usize {
        self.label.relation_index as usize
    }
}

struct Context<'a> {
    input: &'a DiagramInput,
    nodes: Vec<MeasuredNode>,
    flows: Vec<Flow>,
    labels: Vec<Plate>,
    view_box: Rect,
    frames: Vec<composition::Frame>,
    automatic_showcase: bool,
    context_minimum: f64,
}
impl Context<'_> {
    fn legacy(&self) -> bool {
        self.input.input_contract == InputContract::Legacy217
    }
    fn overlaps(&self, a: &Rect, b: &Rect, gap: f64) -> bool {
        geometry::rects_overlap_for_contract(a, b, gap, self.input.input_contract)
    }
    fn supports_crossover(&self, index: usize) -> bool {
        let raw = &self.flows[index].raw;
        self.automatic_showcase
            && raw.get("via").is_none()
            && (string(raw, "route").is_empty() || string(raw, "route") == "auto")
            && !has(raw, &["channelX", "channelY"])
    }
    fn composition_route(&self, index: usize, points: &[Point]) -> composition::RoutedRelation {
        composition::RoutedRelation::new(index, self.flows[index].raw.clone(), points.to_vec())
    }
    fn routed(&self) -> Vec<composition::RoutedRelation> {
        self.flows
            .iter()
            .enumerate()
            .map(|(index, flow)| self.composition_route(index, &flow.points))
            .collect()
    }
    fn intrudes(&self, index: usize, points: &[Point]) -> bool {
        let flow = &self.flows[index];
        self.nodes.iter().enumerate().any(|(node_index, node)| {
            points.windows(2).enumerate().any(|(segment, pair)| {
                !((node_index == flow.from && segment == 0)
                    || (node_index == flow.to && segment == points.len() - 2))
                    && geometry::segment_intersects_rect(pair[0], pair[1], &node.bounds, 2.0)
            })
        })
    }
    fn corridors(
        &self,
        index: usize,
        points: &[Point],
        other: usize,
        other_points: &[Point],
        shared: bool,
    ) -> bool {
        let include = |_: &Value, _: &Value| true;
        let options = composition::CorridorOptions {
            contract: self.input.input_contract,
            include_shared_endpoints: shared
                .then_some(&include as composition::RelationPairPredicate<'_>),
            ..Default::default()
        };
        !composition::collect_ambiguous_corridors(
            &[
                self.composition_route(index, points),
                self.composition_route(other, other_points),
            ],
            &options,
        )
        .is_empty()
    }
    fn border_runs(
        &self,
        points: &[Point],
        frames: &[composition::Frame],
        exclude_bottom: bool,
    ) -> bool {
        composition::collect_border_runs(
            &[composition::RoutedRelation::new(
                0,
                Value::Null,
                points.to_vec(),
            )],
            frames,
        )
        .iter()
        .any(|hit| !exclude_bottom || hit.side != "bottom")
    }
    fn crossing_flows(&self, index: usize, points: &[Point]) -> Vec<usize> {
        self.flows
            .iter()
            .enumerate()
            .filter(|(other, flow)| {
                *other != index
                    && !related(&self.flows[index], flow)
                    && intersections(points, &flow.points) > 0
            })
            .map(|(other, _)| other)
            .collect()
    }
    fn cost(&self, index: usize, points: &[Point]) -> f64 {
        length(points)
            + points.len().saturating_sub(2) as f64 * 48.0
            + self.crossing_flows(index, points).len() as f64 * 160.0
    }
    fn clear_label(&self, label: &labels::Label, index: usize, plates: &[Plate]) -> bool {
        let rect = &label.bounds;
        rect.x >= 0.0
            && rect.y >= 0.0
            && rect.x + rect.width <= self.view_box.width
            && rect.y + rect.height <= self.view_box.height - 74.0
            && !self
                .nodes
                .iter()
                .map(|node| node.bounds)
                .chain(self.titles())
                .any(|other| geometry::rects_overlap(rect, &other, 2.0))
            && !plates.iter().enumerate().any(|(i, other)| {
                i != index && geometry::rects_overlap(rect, &other.label.bounds, 2.0)
            })
    }
    fn titles(&self) -> Vec<Rect> {
        self.frames
            .iter()
            .map(|frame| Rect {
                height: 36.0,
                ..frame.bounds
            })
            .collect()
    }
    fn mark_crossovers(&mut self) {
        for flow in &mut self.flows {
            flow.halo = false;
        }
        if !self.automatic_showcase {
            return;
        }
        for left in 0..self.flows.len() {
            for right in left + 1..self.flows.len() {
                if !related(&self.flows[left], &self.flows[right])
                    && self.supports_crossover(left)
                    && self.supports_crossover(right)
                    && intersections(&self.flows[left].points, &self.flows[right].points) > 0
                {
                    self.flows[left].halo = true;
                    self.flows[right].halo = true;
                }
            }
        }
    }
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    fn attempt(
        input: &DiagramInput,
        rebalance: bool,
    ) -> Result<Option<CompiledDiagram>, Diagnostics> {
        let mut context = measure::create(input)?;
        if rebalance && !routing::rebalance(&mut context) {
            return Ok(None);
        }
        measure::expand_heights(&mut context);
        routing::spread_ports(&mut context);
        routing::initial_routes(&mut context);
        routing::repair_routes(&mut context);
        capacity::initial(&mut context);
        capacity::settle(&mut context);
        repair::joint(&mut context);
        validation::check(&context)?;
        measure::scene(context).map(Some)
    }
    match attempt(input, false) {
        Ok(Some(scene)) => Ok(scene),
        Ok(None) => unreachable!("The initial layout attempt never requests port rebalancing"),
        Err(initial) => {
            if !initial
                .iter()
                .any(|diagnostic| diagnostic.code.starts_with("legend/"))
            {
                if let Ok(Some(scene)) = attempt(input, true) {
                    return Ok(scene);
                }
            }
            Err(initial)
        }
    }
}
