use super::*;
use crate::geometry::{rects_overlap_for_contract, segment_rect_clearance};
use std::cmp::Ordering;

#[derive(Clone)]
struct State {
    raw: Value,
    bounds: Rect,
}
#[derive(Clone)]
struct Edge {
    raw: Value,
    from: usize,
    to: usize,
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Spine,
    Arc,
    Same,
    Between,
}
#[derive(Clone)]
struct Bracket {
    sources: Vec<usize>,
    targets: Vec<usize>,
    edges: Vec<usize>,
    frame: Option<(usize, usize)>,
}
struct Model {
    states: Vec<State>,
    edges: Vec<Edge>,
    main: Vec<usize>,
    spine: Vec<Option<usize>>,
    spine_edges: Vec<Option<usize>>,
    depth: Vec<usize>,
    rows: Vec<Vec<usize>>,
    kinds: Vec<Kind>,
    brackets: Vec<Bracket>,
    bracket_of: Vec<Option<usize>>,
    minimum_gaps: Vec<f64>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PortKey {
    Connector(usize),
    LoopOut(usize),
    LoopIn(usize),
    ArcOut(usize),
    ArcIn(usize),
}
#[derive(Clone)]
struct Port {
    state: usize,
    top: bool,
    key: PortKey,
    other: f64,
    order: usize,
    x: f64,
}
#[derive(Clone, Copy)]
enum ConnectorKind {
    Single {
        edge: usize,
        upper: usize,
        lower: usize,
        down: bool,
    },
    Frame {
        bracket: usize,
        target: usize,
        rect: Rect,
        exit: f64,
    },
    Bracket(usize),
    Loop(usize),
}
#[derive(Clone)]
struct Connector {
    kind: ConnectorKind,
    gap: usize,
    order: usize,
}
#[derive(Clone)]
struct Run {
    connector: usize,
    tops: Vec<f64>,
    bottoms: Vec<f64>,
    lo: f64,
    hi: f64,
    straight: bool,
    track: Option<usize>,
}
#[derive(Clone)]
struct Arc {
    edge: usize,
    lo: usize,
    hi: usize,
    index: usize,
    level: usize,
}
#[derive(Clone, Copy, PartialEq)]
enum LabelMode {
    Above,
    On,
    Beside,
    None,
}
#[derive(Clone)]
struct Route {
    points: Vec<Point>,
    segment: usize,
    label_mode: LabelMode,
    bracket: Option<usize>,
}
#[derive(Clone, Default)]
struct Geometry {
    routes: Vec<Option<Route>>,
    route_order: Vec<usize>,
    junctions: Vec<Point>,
    frames: Vec<(usize, Rect)>,
    labels: Vec<Option<Rect>>,
    unplaced: Vec<usize>,
    content_right: f64,
    content_bottom: f64,
}
#[derive(Clone, Copy)]
struct Spacing {
    spine: f64,
    row: f64,
    drop: f64,
    first: f64,
}
fn sign(value: f64) -> i8 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}
fn compare(a: f64, b: f64) -> Ordering {
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}
fn cx(state: &State) -> f64 {
    state.bounds.x + state.bounds.width / 2.0
}
fn cy(state: &State) -> f64 {
    state.bounds.y + state.bounds.height / 2.0
}
fn has_label(edge: &Edge) -> bool {
    !string(&edge.raw, "label").is_empty() || !string(&edge.raw, "note").is_empty()
}
fn label_box(input: &DiagramInput, edge: &Edge) -> (f64, f64) {
    let label = string(&edge.raw, "label");
    let note = string(&edge.raw, "note");
    (
        ((units(input, label) * 10.0 * 0.6).max(units(input, note) * 9.0 * 0.6) + 12.0).ceil(),
        if !label.is_empty() && !note.is_empty() {
            28.0
        } else {
            16.0
        },
    )
}
fn variant(model: &Model, index: usize) -> &str {
    model.edges[index].raw["variant"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(if model.kinds[index] == Kind::Spine {
            "emphasis"
        } else {
            "default"
        })
}
fn preferred_width(input: &DiagramInput, raw: &Value) -> f64 {
    let need = |value: &str, font: f64| {
        if value.is_empty() {
            0.0
        } else {
            units(input, value) * font * 0.6 + 44.0
        }
    };
    let step = string(raw, "step");
    let left = 19.0_f64.max(if step.is_empty() {
        0.0
    } else {
        26.0 + units(input, step) * 8.0 * 0.6
    });
    // Text units count string scalars; fixed coefficients keep widths finite.
    (units(input, string(raw, "label")) * 12.0 * 0.6
        + left
        + 8.0
        + if raw.get("brand").is_some() {
            48.0
        } else {
            0.0
        })
    .max(need(string(raw, "sublabel"), 9.0))
    .max(need(string(raw, "tag"), 8.0))
    .clamp(140.0, 220.0)
    .ceil()
}

impl Model {
    fn new(input: &DiagramInput) -> Result<Self, Diagnostics> {
        let raw_states = list(&input.document["states"]);
        let raw_edges = list(&input.document["transitions"]);
        let mut errors = vec![];
        let states: Vec<_> = raw_states
            .iter()
            .map(|raw| State {
                raw: raw.clone(),
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: preferred_width(input, raw),
                    height: 64.0,
                },
            })
            .collect();
        let find = |id: &str| states.iter().position(|s| string(&s.raw, "id") == id);
        let mut main = vec![];
        for (index, id) in list(&input.document["mainPath"]).iter().enumerate() {
            if let Some(state) = id.as_str().and_then(find) {
                if main.contains(&state) {
                    errors.push(failure(
                        "lifecycle/repeated-main-path-state",
                        &format!("/mainPath/{index}"),
                        "Main path repeats a state",
                        Value::Null,
                    ));
                }
                main.push(state);
            } else {
                errors.push(failure(
                    "lifecycle/unknown-main-path-state",
                    &format!("/mainPath/{index}"),
                    "Main path references an unknown state",
                    Value::Null,
                ));
            }
        }
        if main.len() < 2 {
            errors.push(failure(
                "lifecycle/main-path-gap",
                "/mainPath",
                "Lifecycle v3 needs at least two main-path states",
                Value::Null,
            ));
        }
        let mut edges = vec![];
        for (index, raw) in raw_edges.iter().enumerate() {
            if let (Some(from), Some(to)) = (find(string(raw, "from")), find(string(raw, "to"))) {
                edges.push(Edge {
                    raw: raw.clone(),
                    from,
                    to,
                });
            } else {
                errors.push(failure(
                    "lifecycle/unknown-endpoint",
                    &format!("/transitions/{index}"),
                    "Transition endpoint is not a lifecycle state",
                    Value::Null,
                ));
            }
        }
        for (index, pair) in main.windows(2).enumerate() {
            if !edges.iter().any(|e| e.from == pair[0] && e.to == pair[1]) {
                errors.push(failure(
                    "lifecycle/main-path-gap",
                    &format!("/mainPath/{index}"),
                    "Consecutive main-path states need a real transition",
                    Value::Null,
                ));
            }
        }
        for (index, state) in states.iter().enumerate() {
            for (field, minimum) in [("label", 10.0), ("sublabel", 8.0), ("tag", 7.0)] {
                let needed = (units(input, string(&state.raw, field)) * minimum * 0.6).ceil();
                if needed > 212.0 {
                    errors.push(failure(
                        "lifecycle/state-text-too-long",
                        &format!("/states/{index}/{field}"),
                        "State text is wider than the maximum 220px state",
                        json!({"neededWidth":needed,"availableWidth":212}),
                    ));
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut spine = vec![None; states.len()];
        for (index, &state) in main.iter().enumerate() {
            spine[state] = Some(index);
        }
        let mut spine_edges = vec![None; main.len()];
        for (index, edge) in edges.iter().enumerate() {
            if let (Some(a), Some(b)) = (spine[edge.from], spine[edge.to]) {
                if b == a + 1 && spine_edges[a].is_none() {
                    spine_edges[a] = Some(index);
                }
            }
        }
        let minimum_gaps = main
            .iter()
            .take(main.len() - 1)
            .enumerate()
            .map(|(index, _)| {
                spine_edges[index].map_or(0.0, |edge| {
                    if has_label(&edges[edge]) {
                        label_box(input, &edges[edge]).0
                    } else {
                        0.0
                    }
                }) + 26.0
            })
            .collect();
        let mut depth = vec![usize::MAX; states.len()];
        for &id in &main {
            depth[id] = 0;
        }
        let mut frontier = main.clone();
        while !frontier.is_empty() {
            let mut next = vec![];
            for &id in &frontier {
                for edge in &edges {
                    let other = if edge.from == id {
                        Some(edge.to)
                    } else if edge.to == id {
                        Some(edge.from)
                    } else {
                        None
                    };
                    if let Some(other) = other.filter(|&other| depth[other] == usize::MAX) {
                        depth[other] = depth[id] + 1;
                        next.push(other);
                    }
                }
            }
            frontier = next;
        }
        for depth in &mut depth {
            if *depth == usize::MAX {
                *depth = 1;
            }
        }
        let row_count = depth.iter().copied().max().unwrap_or(0) + 1;
        let rows = (0..row_count)
            .map(|row| {
                if row == 0 {
                    main.clone()
                } else {
                    (0..states.len()).filter(|&s| depth[s] == row).collect()
                }
            })
            .collect();
        let kinds = edges
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                if depth[edge.from] == 0 && depth[edge.to] == 0 {
                    if spine_edges[spine[edge.from].unwrap()] == Some(index) {
                        Kind::Spine
                    } else {
                        Kind::Arc
                    }
                } else if depth[edge.from] == depth[edge.to] {
                    Kind::Same
                } else {
                    Kind::Between
                }
            })
            .collect();
        let mut model = Self {
            states,
            edges,
            main,
            spine,
            spine_edges,
            depth,
            rows,
            kinds,
            brackets: vec![],
            bracket_of: vec![None; raw_edges.len()],
            minimum_gaps,
        };
        model.classify_brackets();
        Ok(model)
    }
    fn classify_brackets(&mut self) {
        let mut by_target: Vec<(usize, Vec<usize>)> = vec![];
        for (index, edge) in self.edges.iter().enumerate() {
            if self.kinds[index] != Kind::Between || self.depth[edge.from] != 0 {
                continue;
            }
            if let Some((_, list)) = by_target.iter_mut().find(|(to, _)| *to == edge.to) {
                list.push(index);
            } else {
                by_target.push((edge.to, vec![index]));
            }
        }
        for (target, list) in by_target {
            let mut buckets: Vec<(String, Vec<usize>)> = vec![];
            for edge in list {
                let raw = &self.edges[edge].raw;
                let signature = format!(
                    "{}\0{}\0{}",
                    string(raw, "label"),
                    string(raw, "note"),
                    variant(self, edge)
                );
                if let Some((_, list)) = buckets.iter_mut().find(|(key, _)| *key == signature) {
                    list.push(edge);
                } else {
                    buckets.push((signature, vec![edge]));
                }
            }
            let eligible: Vec<_> = buckets
                .into_iter()
                .filter(|(_, edges)| {
                    let mut sources = vec![];
                    for &edge in edges {
                        if !sources.contains(&self.edges[edge].from) {
                            sources.push(self.edges[edge].from);
                        }
                    }
                    sources.len() >= 2
                })
                .collect();
            if eligible.len() != 1 {
                continue;
            }
            let group = &eligible[0].1;
            let mut sources = vec![];
            for &edge in group {
                if !sources.contains(&self.edges[edge].from) {
                    sources.push(self.edges[edge].from);
                }
            }
            sources.sort_by_key(|&s| self.spine[s].unwrap());
            if let Some(bracket) = self.brackets.iter_mut().find(|b| b.sources == sources) {
                bracket.targets.push(target);
                bracket.edges.extend(group);
            } else {
                self.brackets.push(Bracket {
                    sources,
                    targets: vec![target],
                    edges: group.clone(),
                    frame: None,
                });
            }
        }
        let mut spans = vec![];
        for (index, bracket) in self.brackets.iter_mut().enumerate() {
            for &edge in &bracket.edges {
                self.bracket_of[edge] = Some(index);
            }
            let lo = bracket
                .sources
                .iter()
                .map(|&s| self.spine[s].unwrap())
                .min()
                .unwrap();
            let hi = bracket
                .sources
                .iter()
                .map(|&s| self.spine[s].unwrap())
                .max()
                .unwrap();
            if hi - lo + 1 == bracket.sources.len()
                && !spans.iter().any(|&(a, b)| a <= hi && lo <= b)
            {
                bracket.frame = Some((lo, hi));
                spans.push((lo, hi));
            }
        }
    }
}

fn request(ports: &mut Vec<Port>, state: usize, top: bool, key: PortKey, other: f64, order: usize) {
    if !ports
        .iter()
        .any(|p| p.state == state && p.top == top && p.key == key)
    {
        ports.push(Port {
            state,
            top,
            key,
            other,
            order,
            x: 0.0,
        });
    }
}
fn port_index(ports: &[Port], state: usize, top: bool, key: PortKey) -> usize {
    ports
        .iter()
        .position(|p| p.state == state && p.top == top && p.key == key)
        .expect("a classified connector requested this port")
}
fn port_x(ports: &[Port], state: usize, top: bool, key: PortKey) -> f64 {
    ports[port_index(ports, state, top, key)].x
}
fn assign_arcs(model: &Model) -> Vec<Arc> {
    let mut arcs: Vec<_> = model
        .edges
        .iter()
        .enumerate()
        .filter(|(i, _)| model.kinds[*i] == Kind::Arc)
        .enumerate()
        .map(|(index, (edge, e))| Arc {
            edge,
            lo: model.spine[e.from].unwrap().min(model.spine[e.to].unwrap()),
            hi: model.spine[e.from].unwrap().max(model.spine[e.to].unwrap()),
            index,
            level: 0,
        })
        .collect();
    let mut sorted: Vec<_> = (0..arcs.len()).collect();
    sorted.sort_by_key(|&i| (arcs[i].hi - arcs[i].lo, arcs[i].lo, arcs[i].index));
    for index in sorted {
        let arc = &arcs[index];
        let level = arcs
            .iter()
            .enumerate()
            .filter(|(i, other)| {
                *i != index
                    && other.level > 0
                    && ((other.lo < arc.hi && arc.lo < other.hi)
                        || (other.lo == other.hi && arc.lo < other.lo && other.lo < arc.hi)
                        || (other.lo == arc.lo && other.hi == arc.hi))
            })
            .map(|(_, a)| a.level)
            .max()
            .unwrap_or(0)
            + 1;
        arcs[index].level = level;
    }
    arcs
}

fn gap_between(
    model: &Model,
    input: &DiagramInput,
    left: usize,
    right: usize,
    row_gap: f64,
) -> f64 {
    model
        .edges
        .iter()
        .filter(|edge| {
            has_label(edge)
                && ((edge.from == left && edge.to == right)
                    || (edge.from == right && edge.to == left))
        })
        .map(|edge| label_box(input, edge).0 + 32.0)
        .fold(row_gap, f64::max)
}
#[derive(Clone)]
struct Block {
    members: Vec<usize>,
    ideal: f64,
    weight: f64,
    order: usize,
}
fn arrange(
    model: &Model,
    input: &DiagramInput,
    blocks: &[Block],
    gap: f64,
) -> (f64, Vec<f64>, Vec<(usize, f64)>) {
    let mut offsets = vec![];
    let mut positions = vec![];
    let mut x = 0.0;
    let mut previous = None;
    for block in blocks {
        let start = x;
        for &member in &block.members {
            if let Some(previous) = previous {
                x += gap_between(model, input, previous, member, gap);
            }
            positions.push((member, x));
            x += model.states[member].bounds.width;
            previous = Some(member);
        }
        offsets.push((start + x) / 2.0);
    }
    let width = x;
    (
        width,
        offsets.iter().map(|x| x - width / 2.0).collect(),
        positions,
    )
}
fn centre(model: &Model, input: &DiagramInput, blocks: &[Block], gap: f64) -> (f64, f64) {
    let (width, offsets, _) = arrange(model, input, blocks, gap);
    let weight: f64 = blocks.iter().map(|b| b.weight).sum();
    (
        width,
        blocks
            .iter()
            .zip(offsets)
            .map(|(b, offset)| b.weight * (b.ideal - offset))
            .sum::<f64>()
            / weight,
    )
}
fn place_row(model: &mut Model, input: &DiagramInput, row: usize, gap: f64) {
    let members = model.rows[row].clone();
    let mut ideals = vec![None; model.states.len()];
    for &state in &members {
        let xs: Vec<_> = model
            .edges
            .iter()
            .filter_map(|edge| {
                let other = if edge.to == state {
                    Some(edge.from)
                } else if edge.from == state {
                    Some(edge.to)
                } else {
                    None
                };
                other
                    .filter(|&s| model.depth[s] == row - 1)
                    .map(|s| cx(&model.states[s]))
            })
            .collect();
        if !xs.is_empty() {
            ideals[state] = Some(xs.iter().sum::<f64>() / xs.len() as f64);
        }
    }
    if row == 1 {
        for bracket in &model.brackets {
            let center = bracket
                .sources
                .iter()
                .map(|&s| cx(&model.states[s]))
                .sum::<f64>()
                / bracket.sources.len() as f64;
            for &target in &bracket.targets {
                ideals[target] = Some(center);
            }
        }
    }
    let fallback = model
        .main
        .iter()
        .map(|&s| cx(&model.states[s]))
        .fold(f64::NEG_INFINITY, f64::max)
        + 1.0;
    let mut grouped = vec![false; model.states.len()];
    let mut blocks = vec![];
    for (order, &state) in members.iter().enumerate() {
        if grouped[state] {
            continue;
        }
        let bracket = if row == 1 {
            model
                .brackets
                .iter()
                .rev()
                .find(|b| b.targets.contains(&state))
        } else {
            None
        };
        let list = bracket.map_or_else(|| vec![state], |b| b.targets.clone());
        for &member in &list {
            grouped[member] = true;
        }
        blocks.push(Block {
            weight: if ideals[state].is_some() {
                list.len() as f64
            } else {
                0.05
            },
            members: list,
            ideal: ideals[state].unwrap_or(fallback),
            order,
        });
    }
    blocks.sort_by(|a, b| compare(a.ideal, b.ideal).then(a.order.cmp(&b.order)));
    let mut clusters: Vec<Vec<Block>> = vec![];
    for block in blocks {
        clusters.push(vec![block]);
        while clusters.len() > 1 {
            let last = &clusters[clusters.len() - 1];
            let previous = &clusters[clusters.len() - 2];
            let (aw, ax) = centre(model, input, previous, gap);
            let (bw, bx) = centre(model, input, last, gap);
            if ax
                + aw / 2.0
                + gap_between(
                    model,
                    input,
                    *previous.last().unwrap().members.last().unwrap(),
                    last[0].members[0],
                    gap,
                )
                <= bx - bw / 2.0
            {
                break;
            }
            let last = clusters.pop().unwrap();
            clusters.last_mut().unwrap().extend(last);
        }
    }
    for cluster in clusters {
        let (width, x) = centre(model, input, &cluster, gap);
        let (_, _, positions) = arrange(model, input, &cluster, gap);
        for (state, offset) in positions {
            model.states[state].bounds.x = x - width / 2.0 + offset;
        }
    }
}

fn port_slots(ports: &[Port]) -> Vec<(usize, bool, Vec<usize>)> {
    let mut slots: Vec<(usize, bool, Vec<usize>)> = vec![];
    for (index, port) in ports.iter().enumerate() {
        if let Some((_, _, list)) = slots
            .iter_mut()
            .find(|(state, top, _)| *state == port.state && *top == port.top)
        {
            list.push(index);
        } else {
            slots.push((port.state, port.top, vec![index]));
        }
    }
    slots
}
fn fit_ports(wanted: &[f64], step: f64, low: f64, high: f64) -> Vec<f64> {
    let top = high - step * wanted.len().saturating_sub(1) as f64;
    let mut pools: Vec<(f64, usize)> = vec![];
    for (index, &x) in wanted.iter().enumerate() {
        pools.push((
            (if x.is_finite() {
                x
            } else if x > 0.0 {
                high
            } else {
                low
            }) - index as f64 * step,
            1,
        ));
        while pools.len() > 1 {
            let a = pools[pools.len() - 2];
            let b = pools[pools.len() - 1];
            if a.0 / a.1 as f64 <= b.0 / b.1 as f64 {
                break;
            }
            pools.pop();
            let last = pools.last_mut().unwrap();
            last.0 += b.0;
            last.1 += b.1;
        }
    }
    let mut values = vec![];
    for (sum, count) in pools {
        let x = (sum / count as f64).max(low).min(low.max(top));
        values.extend(std::iter::repeat_n(x, count));
    }
    values
        .iter()
        .enumerate()
        .map(|(i, x)| x + i as f64 * step)
        .collect()
}
fn fits(model: &Model, ports: &[Port], moving: usize, x: f64) -> bool {
    let port = &ports[moving];
    let state = &model.states[port.state];
    x >= state.bounds.x + 14.0
        && x <= state.bounds.x + state.bounds.width - 14.0
        && ports.iter().enumerate().all(|(i, other)| {
            i == moving
                || other.state != port.state
                || other.top != port.top
                || (other.x - x).abs() >= 12.0
        })
}
fn distribute_ports(
    model: &Model,
    ports: &mut [Port],
    arcs: &[Arc],
    connectors: &[Connector],
    even: bool,
) {
    let mut slots = port_slots(ports);
    slots.sort_by_key(|(state, top, _)| *top && model.spine[*state].is_none());
    for (state, top, mut list) in slots {
        let lower = top && model.spine[state].is_none();
        if lower {
            for &index in &list {
                if let PortKey::Connector(connector) = ports[index].key {
                    if let ConnectorKind::Single { upper, .. } = connectors[connector].kind {
                        ports[index].other = port_x(ports, upper, false, ports[index].key);
                    }
                }
            }
        }
        let center = cx(&model.states[state]);
        let level = |key| match key {
            PortKey::ArcOut(edge) | PortKey::ArcIn(edge) => {
                arcs.iter().find(|a| a.edge == edge).map(|a| a.level)
            }
            _ => None,
        };
        list.sort_by(|&a, &b| {
            let left = &ports[a];
            let right = &ports[b];
            let ld = sign(left.other - center);
            let rd = sign(right.other - center);
            if ld != rd {
                return ld.cmp(&rd);
            }
            if let (Some(a), Some(b)) = (level(left.key), level(right.key)) {
                if a != b {
                    return if ld < 0 { a.cmp(&b) } else { b.cmp(&a) };
                }
            }
            compare(left.other, right.other).then(left.order.cmp(&right.order))
        });
        let usable = model.states[state].bounds.width - 44.0;
        let step = if list.len() > 1 {
            26.0_f64.min(usable / (list.len() - 1) as f64)
        } else {
            0.0
        };
        let xs = if even || (model.spine[state].is_some() && top) {
            (0..list.len())
                .map(|index| center - step * (list.len() - 1) as f64 / 2.0 + index as f64 * step)
                .collect()
        } else {
            fit_ports(
                &list.iter().map(|&i| ports[i].other).collect::<Vec<_>>(),
                step,
                model.states[state].bounds.x + 22.0,
                model.states[state].bounds.x + model.states[state].bounds.width - 22.0,
            )
        };
        for (index, x) in list.into_iter().zip(xs) {
            ports[index].x = (x * 10.0 + 0.5).floor() / 10.0;
        }
    }
    for (state, top, entries) in port_slots(ports) {
        if !top || model.spine[state].is_some() {
            continue;
        }
        let mut group = entries.clone();
        group.sort_by(|&a, &b| compare(ports[a].x, ports[b].x));
        let targets: Vec<_> = group
            .iter()
            .map(|&index| {
                if let PortKey::Connector(c) = ports[index].key {
                    if let ConnectorKind::Single { upper, .. } = connectors[c].kind {
                        Some(port_x(ports, upper, false, ports[index].key))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        let rect = model.states[state].bounds;
        if targets.iter().enumerate().all(|(i, x)| {
            x.is_some_and(|x| {
                x >= rect.x + 14.0
                    && x <= rect.x + rect.width - 14.0
                    && (i == 0 || x - targets[i - 1].unwrap_or(f64::INFINITY) >= 12.0)
            })
        }) {
            for (&index, x) in group.iter().zip(targets) {
                ports[index].x = x.unwrap();
            }
            continue;
        }
        for index in entries {
            let PortKey::Connector(c) = ports[index].key else {
                continue;
            };
            match connectors[c].kind {
                ConnectorKind::Single { upper, .. } => {
                    let upper_index = port_index(ports, upper, false, ports[index].key);
                    let delta = (ports[upper_index].x - ports[index].x).abs();
                    if delta < 0.5 || (delta >= 16.0 && even) {
                        continue;
                    }
                    if fits(model, ports, index, ports[upper_index].x) {
                        ports[index].x = ports[upper_index].x;
                    } else if delta >= 16.0 {
                        continue;
                    } else if fits(model, ports, upper_index, ports[index].x) {
                        ports[upper_index].x = ports[index].x;
                    } else {
                        let away = match sign(ports[index].x - ports[upper_index].x) {
                            0 => 1,
                            s => s,
                        } as f64;
                        if let Some(x) = [
                            ports[upper_index].x + away * 16.0,
                            ports[upper_index].x - away * 16.0,
                        ]
                        .into_iter()
                        .find(|&x| fits(model, ports, index, x))
                        {
                            ports[index].x = x;
                        }
                    }
                }
                ConnectorKind::Bracket(bracket) => {
                    let ticks: Vec<_> = model.brackets[bracket]
                        .sources
                        .iter()
                        .map(|&s| port_x(ports, s, false, ports[index].key))
                        .collect();
                    if let Some(x) = ticks.into_iter().find(|x| {
                        (*x - ports[index].x).abs() >= 0.5 && (*x - ports[index].x).abs() < 16.0
                    }) {
                        if fits(model, ports, index, x) {
                            ports[index].x = x;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn frame_exits(model: &Model, connectors: &mut [Connector], ports: &mut [Port]) {
    for index in 0..connectors.len() {
        let ConnectorKind::Frame {
            bracket,
            target,
            rect,
            ..
        } = connectors[index].kind
        else {
            continue;
        };
        let (lo, hi) = model.brackets[bracket].frame.unwrap();
        let mut taken: Vec<_> = model.main[lo..=hi]
            .iter()
            .flat_map(|&s| {
                ports
                    .iter()
                    .filter(move |p| p.state == s && !p.top)
                    .map(|p| p.x)
            })
            .collect();
        taken.extend(connectors.iter().filter_map(|c| {
            if let ConnectorKind::Frame {
                bracket: b, exit, ..
            } = c.kind
            {
                (b == bracket && exit.is_finite()).then_some(exit)
            } else {
                None
            }
        }));
        let port = port_index(ports, target, true, PortKey::Connector(index));
        let low = rect.x + 16.0;
        let high = rect.x + rect.width - 16.0;
        let wanted = ports[port].x.clamp(low, high);
        let free =
            |x: f64| x >= low && x <= high && taken.iter().all(|other| (other - x).abs() >= 14.0);
        let mut exit = wanted;
        for step in 0..=40 {
            if free(exit) {
                break;
            }
            if let Some(x) = [wanted + step as f64 * 7.0, wanted - step as f64 * 7.0]
                .into_iter()
                .find(|&x| free(x))
            {
                exit = x;
            }
        }
        let delta = (exit - ports[port].x).abs();
        if (0.5..24.0).contains(&delta) {
            let r = model.states[target].bounds;
            if exit >= r.x + 12.0
                && exit <= r.x + r.width - 12.0
                && ports.iter().enumerate().all(|(i, p)| {
                    i == port || p.state != target || !p.top || (p.x - exit).abs() >= 12.0
                })
            {
                ports[port].x = exit;
            } else if delta < 16.0 {
                let away = if sign(exit - ports[port].x) == 0 {
                    1.0
                } else {
                    sign(exit - ports[port].x) as f64
                };
                if let Some(x) = [ports[port].x, exit + away * (16.0 - delta)]
                    .into_iter()
                    .find(|&x| free(x))
                {
                    exit = x;
                }
            }
        }
        if let ConnectorKind::Frame { exit: value, .. } = &mut connectors[index].kind {
            *value = exit;
        }
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}
fn crossing_count(runs: &[Run], assignment: &[Option<usize>]) -> usize {
    let mut count = 0;
    for (a, left) in runs.iter().enumerate() {
        let verticals: Vec<_> = if let Some(track) = assignment[a] {
            let y = 10.0 + track as f64;
            left.tops
                .iter()
                .map(|&x| (x, 0.0, y))
                .chain(left.bottoms.iter().map(|&x| (x, y, 1000.0)))
                .collect()
        } else {
            vec![(left.tops[0], 0.0, 1000.0)]
        };
        for (b, right) in runs.iter().enumerate() {
            if a == b {
                continue;
            }
            let Some(track) = assignment[b] else {
                continue;
            };
            let y = 10.0 + track as f64;
            for &(x, y1, y2) in &verticals {
                if x > right.lo + 0.5 && x < right.hi - 0.5 && y > y1 && y < y2 {
                    count += 1;
                }
            }
        }
    }
    count
}
fn assign_tracks(runs: &mut [Run]) -> usize {
    let needs: Vec<_> = (0..runs.len()).filter(|&i| !runs[i].straight).collect();
    if needs.is_empty() {
        return 0;
    }
    let direction = |r: &Run| {
        if r.bottoms.is_empty() {
            0
        } else {
            sign(mean(&r.bottoms) - mean(&r.tops))
        }
    };
    let by_start = |a: &Run, b: &Run| {
        if direction(a) >= 0 {
            compare(mean(&b.tops), mean(&a.tops))
        } else {
            compare(mean(&a.tops), mean(&b.tops))
        }
    };
    let mut best: Option<(usize, usize, Vec<Option<usize>>)> = None;
    for ordering in 0..5 {
        let mut indices = needs.clone();
        indices.sort_by(|&i, &j| {
            let (a, b) = (&runs[i], &runs[j]);
            let span = || compare(a.hi - a.lo, b.hi - b.lo);
            match ordering {
                0 => direction(b).cmp(&direction(a)).then_with(|| by_start(a, b)),
                1 => direction(a).cmp(&direction(b)).then_with(|| by_start(a, b)),
                2 => span(),
                3 => a
                    .bottoms
                    .is_empty()
                    .cmp(&b.bottoms.is_empty())
                    .reverse()
                    .then_with(span),
                _ => a
                    .bottoms
                    .is_empty()
                    .cmp(&b.bottoms.is_empty())
                    .then_with(span),
            }
        });
        let mut tracks: Vec<Vec<usize>> = vec![];
        let mut assignment = vec![None; runs.len()];
        for i in indices {
            let mut track = 0;
            while tracks.get(track).is_some_and(|list| {
                list.iter()
                    .any(|&j| runs[j].lo - 10.0 < runs[i].hi && runs[i].lo < runs[j].hi + 10.0)
            }) {
                track += 1;
            }
            if track == tracks.len() {
                tracks.push(vec![]);
            }
            tracks[track].push(i);
            assignment[i] = Some(track);
        }
        let crossings = crossing_count(runs, &assignment);
        if best
            .as_ref()
            .is_none_or(|(c, n, _)| crossings < *c || (crossings == *c && tracks.len() < *n))
        {
            best = Some((crossings, tracks.len(), assignment));
        }
    }
    let (_, count, assignment) = best.unwrap();
    for (index, run) in runs.iter_mut().enumerate() {
        if !run.straight {
            run.track = assignment[index];
        }
    }
    count
}

fn set_route(geometry: &mut Geometry, index: usize, route: Route) {
    if geometry.routes[index].is_none() {
        geometry.route_order.push(index);
    }
    geometry.routes[index] = Some(route);
}
fn route_cost(geometry: &Geometry) -> f64 {
    let mut cost = 0.0;
    for &index in &geometry.route_order {
        let points = &geometry.routes[index].as_ref().unwrap().points;
        cost += points.len().saturating_sub(2) as f64 * 10.0;
        for pair in points.windows(2) {
            let run = (pair[1].x - pair[0].x).abs();
            if points.len() > 2 && (pair[1].y - pair[0].y).abs() < 0.5 {
                cost += run * 0.1 + if run < 30.0 { 40.0 } else { 0.0 };
            }
        }
    }
    cost
}

fn layout(
    model: &mut Model,
    input: &DiagramInput,
    spacing: Spacing,
    even: bool,
) -> Result<Geometry, Diagnostics> {
    let mut cursor = 56.0;
    for (index, &state) in model.main.iter().enumerate() {
        model.states[state].bounds.x = cursor;
        cursor += model.states[state].bounds.width
            + spacing
                .spine
                .max(model.minimum_gaps.get(index).copied().unwrap_or(26.0));
    }
    for row in 1..model.rows.len() {
        place_row(model, input, row, spacing.row);
    }
    let dx = 56.0
        - model
            .states
            .iter()
            .map(|s| s.bounds.x)
            .fold(f64::INFINITY, f64::min);
    for state in &mut model.states {
        state.bounds.x += dx;
    }
    let arcs = assign_arcs(model);
    let max_level = arcs.iter().map(|a| a.level).max().unwrap_or(0);
    let reserve = if arcs.iter().any(|a| has_label(&model.edges[a.edge])) {
        12.0
    } else {
        0.0
    };
    let spine_top = 28.0
        + if max_level > 0 {
            30.0 + (max_level - 1) as f64 * 24.0 + reserve
        } else {
            0.0
        };
    let mut connectors = vec![];
    for (index, edge) in model.edges.iter().enumerate() {
        if model.kinds[index] != Kind::Between || model.bracket_of[index].is_some() {
            continue;
        }
        let down = model.depth[edge.from] < model.depth[edge.to];
        let (upper, lower) = if down {
            (edge.from, edge.to)
        } else {
            (edge.to, edge.from)
        };
        connectors.push(Connector {
            kind: ConnectorKind::Single {
                edge: index,
                upper,
                lower,
                down,
            },
            gap: model.depth[upper],
            order: index,
        });
    }
    let mut geometry = Geometry {
        routes: vec![None; model.edges.len()],
        labels: vec![None; model.edges.len()],
        ..Default::default()
    };
    for (index, bracket) in model.brackets.iter().enumerate() {
        let order = bracket.edges[0];
        if let Some((lo, hi)) = bracket.frame {
            let first = model.states[model.main[lo]].bounds;
            let last = model.states[model.main[hi]].bounds;
            let rect = Rect {
                x: first.x - 10.0,
                y: spine_top - 10.0,
                width: last.x + last.width - first.x + 20.0,
                height: 84.0,
            };
            geometry.frames.push((index, rect));
            for &target in &bracket.targets {
                connectors.push(Connector {
                    kind: ConnectorKind::Frame {
                        bracket: index,
                        target,
                        rect,
                        exit: f64::NAN,
                    },
                    gap: 0,
                    order,
                });
            }
        } else {
            connectors.push(Connector {
                kind: ConnectorKind::Bracket(index),
                gap: 0,
                order,
            });
        }
    }
    let row_order: Vec<_> = model
        .rows
        .iter()
        .map(|row| {
            let mut states = row.clone();
            states.sort_by(|&a, &b| compare(model.states[a].bounds.x, model.states[b].bounds.x));
            states
        })
        .collect();
    let adjacent = |edge: &Edge| {
        edge.from != edge.to
            && row_order[model.depth[edge.from]]
                .iter()
                .position(|&s| s == edge.from)
                .unwrap()
                .abs_diff(
                    row_order[model.depth[edge.from]]
                        .iter()
                        .position(|&s| s == edge.to)
                        .unwrap(),
                )
                == 1
    };
    let mut side_groups: Vec<((usize, usize), Vec<usize>)> = vec![];
    let mut loops = vec![];
    for (index, edge) in model.edges.iter().enumerate() {
        if model.kinds[index] != Kind::Same {
            continue;
        }
        if !adjacent(edge) {
            loops.push(index);
            continue;
        }
        let key = if string(&model.states[edge.from].raw, "id")
            < string(&model.states[edge.to].raw, "id")
        {
            (edge.from, edge.to)
        } else {
            (edge.to, edge.from)
        };
        if let Some((_, list)) = side_groups.iter_mut().find(|(k, _)| *k == key) {
            list.push(index);
        } else {
            side_groups.push((key, vec![index]));
        }
    }
    let mut side_lanes = vec![];
    let mut row_heights = vec![64.0_f64; model.rows.len()];
    for (_, group) in side_groups {
        let mut ordered: Vec<_> = group
            .iter()
            .copied()
            .filter(|&i| {
                model.states[model.edges[i].from].bounds.x
                    < model.states[model.edges[i].to].bounds.x
            })
            .collect();
        ordered.extend(group.iter().copied().filter(|&i| {
            model.states[model.edges[i].from].bounds.x >= model.states[model.edges[i].to].bounds.x
        }));
        if ordered.len() > 5 {
            return Err(vec![failure(
                "lifecycle/crowded-side-transitions",
                &format!("/transitions/{}", ordered[0]),
                "At most five transitions between adjacent lower-row states remain legible",
                json!({"count":ordered.len()}),
            )]);
        }
        let mut step = 20.0_f64.min(48.0 / ordered.len().saturating_sub(1) as f64);
        if ordered.len() > 2 {
            let heights: Vec<_> = ordered
                .iter()
                .map(|&i| {
                    if has_label(&model.edges[i]) {
                        label_box(input, &model.edges[i]).1
                    } else {
                        0.0
                    }
                })
                .collect();
            for &height in &heights {
                if height > 0.0 {
                    step = step.max(height / 2.0 + 5.0);
                }
            }
            for pair in heights.windows(2) {
                if pair[0] > 0.0 && pair[1] > 0.0 {
                    step = step.max((pair[0] + pair[1]) / 2.0 + 4.0);
                }
            }
            let row = model.depth[model.edges[ordered[0]].from];
            row_heights[row] = row_heights[row].max(step * (ordered.len() - 1) as f64 + 16.0);
        }
        side_lanes.push((ordered, step));
    }
    for edge in loops {
        connectors.push(Connector {
            kind: ConnectorKind::Loop(edge),
            gap: model.depth[model.edges[edge].from],
            order: edge,
        });
    }
    let mut ports = vec![];
    for (index, connector) in connectors.iter().enumerate() {
        let key = PortKey::Connector(index);
        match connector.kind {
            ConnectorKind::Single { upper, lower, .. } => {
                request(
                    &mut ports,
                    upper,
                    false,
                    key,
                    cx(&model.states[lower]),
                    connector.order,
                );
                request(
                    &mut ports,
                    lower,
                    true,
                    key,
                    cx(&model.states[upper]),
                    connector.order,
                );
            }
            ConnectorKind::Frame { target, rect, .. } => request(
                &mut ports,
                target,
                true,
                key,
                rect.x + rect.width / 2.0,
                connector.order,
            ),
            ConnectorKind::Loop(edge) => {
                let e = &model.edges[edge];
                let (other_a, other_b, order_a, order_b) = if e.from == e.to {
                    (f64::INFINITY, f64::INFINITY, 0, 1)
                } else {
                    (
                        cx(&model.states[e.to]),
                        cx(&model.states[e.from]),
                        connector.order,
                        connector.order,
                    )
                };
                request(
                    &mut ports,
                    e.from,
                    false,
                    PortKey::LoopOut(edge),
                    other_a,
                    order_a,
                );
                request(
                    &mut ports,
                    e.to,
                    false,
                    PortKey::LoopIn(edge),
                    other_b,
                    order_b,
                );
            }
            ConnectorKind::Bracket(bracket) => {
                let b = &model.brackets[bracket];
                let drops = b.targets.iter().map(|&s| cx(&model.states[s])).sum::<f64>()
                    / b.targets.len() as f64;
                let sources = b.sources.iter().map(|&s| cx(&model.states[s])).sum::<f64>()
                    / b.sources.len() as f64;
                for &s in &b.sources {
                    request(&mut ports, s, false, key, drops, connector.order);
                }
                for &s in &b.targets {
                    request(&mut ports, s, true, key, sources, connector.order);
                }
            }
        }
    }
    for arc in &arcs {
        let edge = &model.edges[arc.edge];
        let (a, b, oa, ob) = if edge.from == edge.to {
            (f64::INFINITY, f64::INFINITY, 0, 1)
        } else {
            (
                cx(&model.states[edge.to]),
                cx(&model.states[edge.from]),
                0,
                0,
            )
        };
        request(
            &mut ports,
            edge.from,
            true,
            PortKey::ArcOut(arc.edge),
            a,
            oa,
        );
        request(&mut ports, edge.to, true, PortKey::ArcIn(arc.edge), b, ob);
    }
    distribute_ports(model, &mut ports, &arcs, &connectors, even);
    frame_exits(model, &mut connectors, &mut ports);
    let mut runs = vec![];
    for (index, connector) in connectors.iter().enumerate() {
        let key = PortKey::Connector(index);
        let (tops, bottoms, straight) = match connector.kind {
            ConnectorKind::Loop(edge) => {
                let e = &model.edges[edge];
                (
                    vec![
                        port_x(&ports, e.from, false, PortKey::LoopOut(edge)),
                        port_x(&ports, e.to, false, PortKey::LoopIn(edge)),
                    ],
                    vec![],
                    false,
                )
            }
            ConnectorKind::Bracket(bracket) => {
                let b = &model.brackets[bracket];
                (
                    b.sources
                        .iter()
                        .map(|&s| port_x(&ports, s, false, key))
                        .collect(),
                    b.targets
                        .iter()
                        .map(|&s| port_x(&ports, s, true, key))
                        .collect(),
                    false,
                )
            }
            ConnectorKind::Single { upper, lower, .. } => {
                let top = port_x(&ports, upper, false, key);
                let bottom = port_x(&ports, lower, true, key);
                (vec![top], vec![bottom], (top - bottom).abs() < 0.5)
            }
            ConnectorKind::Frame { target, exit, .. } => {
                let bottom = port_x(&ports, target, true, key);
                (vec![exit], vec![bottom], (exit - bottom).abs() < 0.5)
            }
        };
        let lo = tops
            .iter()
            .chain(&bottoms)
            .copied()
            .fold(f64::INFINITY, f64::min);
        let hi = tops
            .iter()
            .chain(&bottoms)
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        runs.push(Run {
            connector: index,
            tops,
            bottoms,
            lo,
            hi,
            straight,
            track: None,
        });
    }
    let mut row_top = vec![spine_top];
    let mut track_base = vec![];
    for row in 0..model.rows.len() {
        for &state in &model.rows[row] {
            model.states[state].bounds.y = row_top[row];
            model.states[state].bounds.height = row_heights[row];
        }
        let bottom = row_top[row] + row_heights[row];
        let indices: Vec<_> = runs
            .iter()
            .enumerate()
            .filter(|(_, run)| connectors[run.connector].gap == row)
            .map(|(i, _)| i)
            .collect();
        let mut row_runs: Vec<_> = indices.iter().map(|&i| runs[i].clone()).collect();
        let count = assign_tracks(&mut row_runs);
        for (i, run) in indices.into_iter().zip(row_runs) {
            runs[i] = run;
        }
        track_base.push(bottom + spacing.first);
        if row + 1 < model.rows.len() {
            row_top.push(
                bottom + spacing.first + count.saturating_sub(1) as f64 * 18.0 + spacing.drop,
            );
        }
    }
    for (index, &edge) in model.spine_edges.iter().enumerate() {
        let Some(edge) = edge else {
            continue;
        };
        let a = &model.states[model.main[index]];
        let b = &model.states[model.main[index + 1]];
        set_route(
            &mut geometry,
            edge,
            Route {
                points: vec![p(a.bounds.x + a.bounds.width, cy(a)), p(b.bounds.x, cy(b))],
                segment: 0,
                label_mode: LabelMode::Above,
                bracket: None,
            },
        );
    }
    for arc in &arcs {
        let e = &model.edges[arc.edge];
        let a = port_x(&ports, e.from, true, PortKey::ArcOut(arc.edge));
        let b = port_x(&ports, e.to, true, PortKey::ArcIn(arc.edge));
        let y = spine_top - 30.0 - (arc.level - 1) as f64 * 24.0;
        set_route(
            &mut geometry,
            arc.edge,
            Route {
                points: vec![p(a, spine_top), p(a, y), p(b, y), p(b, spine_top)],
                segment: 1,
                label_mode: LabelMode::On,
                bracket: None,
            },
        );
    }
    for run in &runs {
        let connector = &connectors[run.connector];
        let y = track_base[connector.gap] + run.track.unwrap_or(0) as f64 * 18.0;
        let upper = row_top[connector.gap] + row_heights[connector.gap];
        match connector.kind {
            ConnectorKind::Loop(edge) => {
                set_route(
                    &mut geometry,
                    edge,
                    Route {
                        points: vec![
                            p(run.tops[0], upper),
                            p(run.tops[0], y),
                            p(run.tops[1], y),
                            p(run.tops[1], upper),
                        ],
                        segment: 1,
                        label_mode: LabelMode::On,
                        bracket: None,
                    },
                );
            }
            ConnectorKind::Single { edge, .. } | ConnectorKind::Frame { bracket: edge, .. } => {
                let lower = row_top[connector.gap + 1];
                let start = if let ConnectorKind::Frame { rect, .. } = connector.kind {
                    rect.y + rect.height
                } else {
                    upper
                };
                let mut points = if run.straight {
                    vec![p(run.tops[0], start), p(run.tops[0], lower)]
                } else {
                    vec![
                        p(run.tops[0], start),
                        p(run.tops[0], y),
                        p(run.bottoms[0], y),
                        p(run.bottoms[0], lower),
                    ]
                };
                if let ConnectorKind::Single { down, .. } = connector.kind {
                    if !down {
                        points.reverse();
                    }
                    let segment = if down { points.len() - 2 } else { 0 };
                    set_route(
                        &mut geometry,
                        edge,
                        Route {
                            points,
                            segment,
                            label_mode: LabelMode::Beside,
                            bracket: None,
                        },
                    );
                } else if let ConnectorKind::Frame { target, .. } = connector.kind {
                    for (index, &edge_index) in model.brackets[edge]
                        .edges
                        .iter()
                        .filter(|&&i| model.edges[i].to == target)
                        .enumerate()
                    {
                        set_route(
                            &mut geometry,
                            edge_index,
                            Route {
                                segment: points.len() - 2,
                                points: points.clone(),
                                label_mode: if index == 0 {
                                    LabelMode::Beside
                                } else {
                                    LabelMode::None
                                },
                                bracket: Some(edge),
                            },
                        );
                    }
                }
            }
            ConnectorKind::Bracket(bracket) => {
                let lower = row_top[connector.gap + 1];
                let mut labelled = vec![];
                for &edge in &model.brackets[bracket].edges {
                    let e = &model.edges[edge];
                    let tick = port_x(&ports, e.from, false, PortKey::Connector(run.connector));
                    let drop = port_x(&ports, e.to, true, PortKey::Connector(run.connector));
                    let points = if (tick - drop).abs() < 0.5 {
                        vec![p(tick, upper), p(tick, lower)]
                    } else {
                        vec![p(tick, upper), p(tick, y), p(drop, y), p(drop, lower)]
                    };
                    let owner = !labelled.contains(&e.to);
                    labelled.push(e.to);
                    set_route(
                        &mut geometry,
                        edge,
                        Route {
                            segment: points.len() - 2,
                            points,
                            label_mode: if owner {
                                LabelMode::Beside
                            } else {
                                LabelMode::None
                            },
                            bracket: Some(bracket),
                        },
                    );
                }
                for &x in run.tops.iter().chain(&run.bottoms) {
                    if x > run.lo + 0.5 && x < run.hi - 0.5 {
                        geometry.junctions.push(p(x, y));
                    }
                }
            }
        }
    }
    for (ordered, step) in side_lanes {
        for (index, &edge) in ordered.iter().enumerate() {
            let e = &model.edges[edge];
            let a = &model.states[e.from];
            let b = &model.states[e.to];
            let y = cy(a) + (index as f64 - (ordered.len() - 1) as f64 / 2.0) * step;
            let points = if a.bounds.x < b.bounds.x {
                vec![p(a.bounds.x + a.bounds.width, y), p(b.bounds.x, y)]
            } else {
                vec![p(a.bounds.x, y), p(b.bounds.x + b.bounds.width, y)]
            };
            set_route(
                &mut geometry,
                edge,
                Route {
                    points,
                    segment: 0,
                    label_mode: if ordered.len() > 2 {
                        LabelMode::On
                    } else {
                        LabelMode::Above
                    },
                    bracket: None,
                },
            );
        }
    }
    place_labels(model, input, &mut geometry);
    Ok(geometry)
}

fn siblings(model: &Model, geometry: &Geometry, edge: usize) -> Vec<usize> {
    let route = geometry.routes[edge].as_ref().unwrap();
    if let Some(bracket) = route.bracket {
        model.brackets[bracket]
            .edges
            .iter()
            .copied()
            .filter(|&other| model.edges[other].to == model.edges[edge].to)
            .collect()
    } else {
        vec![edge]
    }
}
fn place_labels(model: &Model, input: &DiagramInput, geometry: &mut Geometry) {
    let state_right = model
        .states
        .iter()
        .map(|s| s.bounds.x + s.bounds.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut placed: Vec<Rect> = vec![];
    for (index, edge) in model.edges.iter().enumerate() {
        let route = geometry.routes[index]
            .as_ref()
            .expect("every classified transition has a route");
        if route.label_mode == LabelMode::None || !has_label(edge) {
            continue;
        }
        let (width, height) = label_box(input, edge);
        let start = route.points[route.segment];
        let end = route.points[route.segment + 1];
        let mut candidates = vec![];
        let at = |x: f64, y: f64| Rect {
            x: x - width / 2.0,
            y: y - height / 2.0,
            width,
            height,
        };
        match route.label_mode {
            LabelMode::Above => {
                let x = (start.x + end.x) / 2.0;
                candidates.push(at(x, start.y - 6.0 - height / 2.0));
                candidates.push(at(x, start.y + 6.0 + height / 2.0));
            }
            LabelMode::On => {
                for fraction in [0.5, 0.35, 0.65, 0.2, 0.8] {
                    candidates.push(at(start.x + (end.x - start.x) * fraction, start.y));
                }
            }
            LabelMode::Beside => {
                let mut preferred = vec![(start, end)];
                preferred.extend(
                    route
                        .points
                        .windows(2)
                        .enumerate()
                        .filter(|(i, p)| *i != route.segment && (p[0].x - p[1].x).abs() < 0.5)
                        .map(|(_, p)| (p[0], p[1])),
                );
                let row_state = if model.depth[edge.to] < model.depth[edge.from] {
                    edge.from
                } else {
                    edge.to
                };
                for (a, b) in preferred {
                    let sides = if a.x <= cx(&model.states[row_state]) {
                        [-1.0, 1.0]
                    } else {
                        [1.0, -1.0]
                    };
                    for side in sides {
                        for shift in [0.0, -10.0, 10.0] {
                            candidates.push(at(
                                a.x + side * (6.0 + width / 2.0),
                                (a.y + b.y) / 2.0 + shift,
                            ));
                        }
                    }
                }
                if let Some(pair) = route.points.windows(2).find(|p| {
                    (p[0].y - p[1].y).abs() < 0.5 && (p[0].x - p[1].x).abs() > width + 8.0
                }) {
                    candidates.push(at((pair[0].x + pair[1].x) / 2.0, pair[0].y));
                }
            }
            LabelMode::None => {}
        }
        let siblings = siblings(model, geometry, index);
        let clear = |rect: &Rect| {
            rect.x >= 8.0
                && rect.x + rect.width <= state_right + 48.0
                && rect.y >= 4.0
                && model.states.iter().all(|s| {
                    !rects_overlap_for_contract(rect, &s.bounds, 4.0, input.input_contract)
                })
                && placed.iter().all(|other| {
                    !rects_overlap_for_contract(rect, other, 4.0, input.input_contract)
                })
                && model.edges.iter().enumerate().all(|(other, _)| {
                    siblings.contains(&other)
                        || geometry.routes[other]
                            .as_ref()
                            .unwrap()
                            .points
                            .windows(2)
                            .all(|p| segment_rect_clearance(p[0], p[1], rect) >= 5.0)
                })
                && (route.label_mode == LabelMode::On
                    || siblings.iter().all(|&other| {
                        geometry.routes[other]
                            .as_ref()
                            .unwrap()
                            .points
                            .windows(2)
                            .all(|p| segment_rect_clearance(p[0], p[1], rect) >= 5.0)
                    }))
        };
        let chosen = candidates.iter().copied().find(clear);
        if chosen.is_none() {
            geometry.unplaced.push(index);
        }
        let rect = chosen.unwrap_or(candidates[0]);
        geometry.labels[index] = Some(rect);
        placed.push(rect);
    }
    geometry.content_bottom = model
        .states
        .iter()
        .map(|s| s.bounds.y + s.bounds.height)
        .chain(placed.iter().map(|r| r.y + r.height))
        .chain(
            geometry
                .routes
                .iter()
                .filter_map(Option::as_ref)
                .flat_map(|r| r.points.iter().map(|p| p.y)),
        )
        .fold(f64::NEG_INFINITY, f64::max);
    geometry.content_right = model
        .states
        .iter()
        .map(|s| s.bounds.x + s.bounds.width)
        .chain(placed.iter().map(|r| r.x + r.width))
        .fold(f64::NEG_INFINITY, f64::max);
}

pub(super) fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let mut model = Model::new(input)?;
    let smallest = model.states.iter().fold(10.0_f64, |smallest, state| {
        smallest
            .min(font(
                input,
                string(&state.raw, "label"),
                state.bounds.width,
                12.0,
                10.0,
            ))
            .min(if string(&state.raw, "sublabel").is_empty() {
                f64::INFINITY
            } else {
                font(
                    input,
                    string(&state.raw, "sublabel"),
                    state.bounds.width,
                    9.0,
                    8.0,
                )
            })
    });
    let budget = (930.0 * smallest / 6.0).floor();
    let state_width: f64 = model
        .main
        .iter()
        .map(|&s| model.states[s].bounds.width)
        .sum();
    let mut spacing = Spacing {
        spine: 72.0,
        row: 44.0,
        drop: 58.0,
        first: 34.0,
    };
    let mut geometry = Geometry::default();
    let mut wideners = vec![];
    for round in 0..5 {
        let partner = layout(&mut model, input, spacing, false)?;
        let even = layout(&mut model, input, spacing, true)?;
        let partner_placed = partner.unplaced.is_empty();
        let even_placed = even.unplaced.is_empty();
        let choose_even =
            even_placed && (!partner_placed || route_cost(&even) < route_cost(&partner));
        geometry = if choose_even {
            even.clone()
        } else {
            layout(&mut model, input, spacing, false)?
        };
        if partner_placed || even_placed {
            break;
        }
        if round == 0 {
            wideners = if even.unplaced.len() < partner.unplaced.len() {
                even.unplaced
            } else {
                partner.unplaced
            };
        }
        let next = spacing.spine + 28.0;
        let next_width = 112.0
            + state_width
            + model
                .minimum_gaps
                .iter()
                .map(|&minimum| next.max(minimum))
                .sum::<f64>();
        spacing = Spacing {
            spine: if next_width > budget {
                spacing.spine
            } else {
                next
            },
            row: spacing.row + 28.0,
            drop: spacing.drop + 14.0,
            first: spacing.first + 6.0,
        };
    }
    let outgoing: Vec<_> = model.edges.iter().map(|e| e.from).collect();
    let finals: Vec<_> = model
        .states
        .iter()
        .enumerate()
        .filter(|(i, _)| !outgoing.contains(i))
        .map(|(_, s)| string(&s.raw, "id").to_owned())
        .collect();
    let mut nodes = vec![];
    for (index, state) in model.states.iter().enumerate() {
        let raw = &state.raw;
        let r = state.bounds;
        let label = string(raw, "label");
        let sub = string(raw, "sublabel");
        let tag = string(raw, "tag");
        let step = string(raw, "step");
        let lf = if raw.get("brand").is_some() {
            12.0
        } else {
            font(input, label, r.width, 12.0, 10.0)
        };
        let sf = font(input, sub, r.width, 9.0, 8.0);
        let tf = font(input, tag, r.width, 8.0, 7.0);
        let (label_y, sub_y, tag_y) = if !sub.is_empty() && !tag.is_empty() {
            (23.0, 40.0, 55.0)
        } else if !sub.is_empty() {
            (27.0, 45.0, 0.0)
        } else if !tag.is_empty() {
            (29.0, 0.0, 50.0)
        } else {
            (37.0, 0.0, 0.0)
        };
        let offset = (r.height - 64.0) / 2.0;
        let mut rows = vec![(label_y + offset, lf)];
        if !sub.is_empty() {
            rows.push((sub_y + offset, sf));
        }
        if !tag.is_empty() {
            rows.push((tag_y + offset, tf));
        }
        let (x, ys, sigil_y, sigil_size) =
            presentation::node_text_layout(label, lf, r.width, r.height, &rows, step, input);
        let mut texts = vec![];
        if !step.is_empty() {
            texts.push(text(
                format!("/states/{index}/step"),
                step,
                r.x + 23.0,
                r.y + 14.0,
                8.0,
                "start",
            ));
        }
        texts.push(text(
            format!("/states/{index}/label"),
            label,
            r.x + x,
            r.y + ys[0],
            lf,
            "middle",
        ));
        if !sub.is_empty() {
            texts.push(text(
                format!("/states/{index}/sublabel"),
                sub,
                cx(state),
                r.y + ys[1],
                sf,
                "middle",
            ));
        }
        if !tag.is_empty() {
            texts.push(text(
                format!("/states/{index}/tag"),
                tag,
                cx(state),
                r.y + ys[if sub.is_empty() { 1 } else { 2 }],
                tf,
                "middle",
            ));
        }
        let kind = string(raw, "type");
        let icon =
            raw["icon"]
                .as_str()
                .unwrap_or(if matches!(kind, "start" | "active" | "neutral") {
                    if !outgoing.contains(&index) {
                        "stop"
                    } else {
                        "none"
                    }
                } else {
                    kind
                });
        nodes.push(Node {
            brand: None,
            authored: authored(raw, "states", index, kind, &[]),
            bounds: r,
            sigil: (icon != "none").then(|| Sigil {
                kind: icon.into(),
                position: p(r.x + 6.0, r.y + sigil_y),
                size: sigil_size,
            }),
            label: label.into(),
            text: texts,
            detail_rows: Vec::new(),
        });
    }
    let entries = presentation::legend_entries(input, &nodes, !finals.is_empty());
    let canvas_width = (geometry.content_right + 56.0).ceil().max(640.0);
    let view_box = Rect {
        x: 0.0,
        y: 0.0,
        width: canvas_width,
        height: (geometry.content_bottom
            + if entries.is_empty() {
                32.0
            } else {
                78.0 + presentation::legend_extra_height(&entries, canvas_width - 80.0)
            })
        .ceil(),
    };
    let mut errors = vec![];
    if view_box.width > budget {
        errors.push(failure("lifecycle/too-wide","/mainPath","Lifecycle exceeds the desktop text readability width budget",json!({"viewBoxWidth":view_box.width,"budgetPx":budget,"mainPathStates":model.main.len(),"lowerRowStates":model.states.len()-model.main.len(),"spacingWideners":wideners})));
    }
    for &edge in &geometry.unplaced {
        errors.push(failure("lifecycle/label-unplaced",&format!("/transitions/{edge}"),"Transition label and note cannot be placed clear of nearby routes",json!({"from":string(&model.edges[edge].raw,"from"),"to":string(&model.edges[edge].raw,"to")})));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut relations = vec![];
    for (index, edge) in model.edges.iter().enumerate() {
        let route = geometry.routes[index].as_ref().unwrap();
        let raw = &edge.raw;
        let label = string(raw, "label");
        let note = string(raw, "note");
        let plate = geometry.labels[index];
        let mut texts = vec![];
        if let Some(rect) = plate {
            let x = rect.x + rect.width / 2.0;
            let baseline = rect.y + if label.is_empty() { 11.0 } else { 11.5 };
            if !label.is_empty() {
                texts.push(text(
                    format!("/transitions/{index}/label"),
                    label,
                    x,
                    baseline,
                    10.0,
                    "middle",
                ));
            }
            if !note.is_empty() {
                texts.push(text(
                    format!("/transitions/{index}/note"),
                    note,
                    x,
                    baseline + if label.is_empty() { 0.0 } else { 12.0 },
                    9.0,
                    "middle",
                ));
            }
        }
        let label_owner = if route.label_mode == LabelMode::None {
            route.bracket.and_then(|b| {
                model.brackets[b].edges.iter().copied().find(|&other| {
                    model.edges[other].to == edge.to
                        && geometry.routes[other].as_ref().unwrap().label_mode != LabelMode::None
                })
            })
        } else {
            None
        };
        let junction = route.bracket.map(|b| {
            model.brackets[b]
                .sources
                .iter()
                .map(|&s| string(&model.states[s].raw, "id"))
                .collect::<Vec<_>>()
                .join("+")
        });
        relations.push(Relation {
            authored: authored(raw, "transitions", index, "transition", &[]),
            from: string(raw, "from").into(),
            to: string(raw, "to").into(),
            variant: variant(&model, index).into(),
            label: label.into(),
            note: (!note.is_empty()).then(|| note.into()),
            points: route.points.clone(),
            crossover_halo: true,
            independent_ports: true,
            junction,
            label_owner,
            note_bounds: None,
            note_backdrop: None,
            label_plate: plate,
            text: texts,
            marker_start: None,
            marker_end: None,
        });
    }
    let containers = geometry
        .frames
        .iter()
        .map(|&(bracket, bounds)| Container {
            authored: generated(
                "lifecycle-frame",
                format!("lifecycle-composite-{bracket}"),
                "/mainPath",
            ),
            bounds,
            label_plate: None,
            rule: None,
            members: model.brackets[bracket]
                .sources
                .iter()
                .map(|&s| string(&model.states[s].raw, "id").into())
                .collect(),
            text: vec![],
        })
        .collect();
    let legends = presentation::legends(
        input,
        &entries,
        view_box,
        view_box.height - 32.0,
        geometry.content_bottom + 12.0,
    )?;
    let mut scene = scene(
        input,
        view_box,
        nodes,
        relations,
        containers,
        legends,
        "intrinsic-height",
    );
    if let DiagramFacts::Lifecycle { presentation, .. } = &mut scene.facts {
        *presentation = crate::scene::LifecyclePresentation {
            initial_state: Some(string(&model.states[model.main[0]].raw, "id").into()),
            final_states: finals,
            junctions: geometry.junctions,
        };
    }
    Ok(scene)
}
