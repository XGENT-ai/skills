// Ported from the frozen Archify 3.1.0 renderers/tree/render-tree.mjs; see NOTICE.md.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::format::number::js_round;
use crate::geometry::text::is_js_whitespace;
use crate::geometry::text_units;
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Relation, Text,
};
use serde_json::{json, Value};

const MARGIN: f64 = 32.0;
const TOP: f64 = 64.0;
const PAD_X: f64 = 14.0;
const PAD_Y: f64 = 12.0;
const LABEL_FONT: f64 = 13.0;
const LABEL_LINE: f64 = 17.0;
const SUBLABEL_FONT: f64 = 11.0;
const SUBLABEL_LINE: f64 = 14.0;
const ROOT_LABEL_FONT: f64 = 14.5;
pub(crate) const TOGGLE_R: f64 = 7.5;
const ADVANCE: f64 = 0.6;
const STACK_MIN: usize = 3;
const STACK_GAP: f64 = 10.0;
// The desktop reading contract: node text projected into a 930px diagram
// stays at or above 6px.
const READER_WIDTH: f64 = 930.0;
const READABLE_PX: f64 = 6.0;
const NODE_MAX_W_FLOOR: f64 = 120.0;
const NODE_MAX_W_STEP: f64 = 20.0;

/// Provenance of a leaf placed in a stacked list rather than a generation band.
pub(crate) const STACKED: &str = "tree-stacked-leaf-run";
const BANDED: &str = "tree-generation-band";

fn units(text: &str) -> f64 {
    text_units(text)
}

fn trim_end(text: &str) -> &str {
    text.trim_end_matches(is_js_whitespace)
}

fn trim_start(text: &str) -> &str {
    text.trim_start_matches(is_js_whitespace)
}

// CJK ideographs, Hangul and full-width forms wrap as single tokens; other
// words stay together with their trailing white space.
fn wraps_alone(character: char) -> bool {
    matches!(character as u32, 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

fn tokens(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let start = index;
        if wraps_alone(characters[index]) {
            index += 1;
        } else {
            let word = !is_js_whitespace(characters[index]);
            while word
                && index < characters.len()
                && !is_js_whitespace(characters[index])
                && !wraps_alone(characters[index])
            {
                index += 1;
            }
            while index < characters.len() && is_js_whitespace(characters[index]) {
                index += 1;
            }
        }
        result.push(characters[start..index].iter().collect());
    }
    result
}

fn wrap_text(text: &str, font: f64, max_width: f64) -> Vec<String> {
    let limit = 4f64.max((max_width / (font * ADVANCE)).floor());
    if units(text) <= limit {
        return vec![text.to_owned()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for token in tokens(text) {
        let joined = format!("{current}{token}");
        if units(trim_end(&joined)) <= limit {
            current = joined;
            continue;
        }
        if !trim_start(&current).is_empty() {
            lines.push(trim_end(&current).to_owned());
        }
        current = trim_start(&token).to_owned();
        while units(trim_end(&current)) > limit {
            let characters: Vec<char> = current.chars().collect();
            let (mut cut, mut used) = (0, 0.0);
            while cut < characters.len() && used + units(&characters[cut].to_string()) <= limit {
                used += units(&characters[cut].to_string());
                cut += 1;
            }
            let cut = cut.max(1);
            lines.push(characters[..cut].iter().collect());
            current = characters[cut..].iter().collect();
        }
    }
    if !trim_start(&current).is_empty() {
        lines.push(trim_end(&current).to_owned());
    }
    lines
}

#[derive(Clone, Copy)]
struct Settings {
    down: bool,
    gap_x: f64,
    gap_y: f64,
    node_w: f64,
    node_max_w: f64,
}

impl Settings {
    fn breadth_gap(&self) -> f64 {
        if self.down {
            self.gap_x
        } else {
            self.gap_y
        }
    }

    fn depth_gap(&self) -> f64 {
        if self.down {
            self.gap_y
        } else {
            self.gap_x
        }
    }

    fn breadth(&self, node: &Measured) -> f64 {
        if self.down {
            node.width
        } else {
            node.height
        }
    }
}

enum Group {
    Single(usize),
    Stack {
        members: Vec<usize>,
        spine_offset: f64,
        indent: f64,
        width: f64,
        spine: f64,
    },
}

struct Measured {
    id: String,
    label: String,
    parent: Option<usize>,
    children: Vec<usize>,
    depth: usize,
    label_font: f64,
    label_lines: Vec<String>,
    sublabel_lines: Vec<String>,
    width: f64,
    height: f64,
    content_height: f64,
    // Index of the stack group, within the parent's groups, holding this leaf.
    stack: Option<usize>,
    x: f64,
    y: f64,
    toggle_at: Option<f64>,
}

impl Measured {
    fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

struct Solved {
    root: usize,
    nodes: Vec<Measured>,
    groups: Vec<Vec<Group>>,
    band_start: Vec<f64>,
    view_box: [f64; 2],
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn group_span(group: &Group, span: &[f64]) -> f64 {
    match group {
        Group::Stack { indent, width, .. } => indent + width,
        Group::Single(child) => span[*child],
    }
}

fn groups_span(groups: &[Group], span: &[f64], gap: f64) -> f64 {
    groups.iter().enumerate().fold(0.0, |sum, (index, group)| {
        sum + group_span(group, span) + if index > 0 { gap } else { 0.0 }
    })
}

// The hierarchy is exactly the authored one; three or more consecutive leaf
// children of one parent become a stacked list when the tree grows down.
fn structure(nodes: &[Value], down: bool) -> (usize, Vec<Measured>, Vec<Vec<Group>>) {
    let ids: Vec<&str> = nodes.iter().map(|node| string(node, "id")).collect();
    let mut measured: Vec<Measured> = nodes
        .iter()
        .map(|node| Measured {
            id: string(node, "id").into(),
            label: string(node, "label").into(),
            parent: node["parent"]
                .as_str()
                .and_then(|parent| ids.iter().position(|id| *id == parent)),
            children: vec![],
            depth: 0,
            label_font: LABEL_FONT,
            label_lines: vec![],
            sublabel_lines: vec![],
            width: 0.0,
            height: 0.0,
            content_height: 0.0,
            stack: None,
            x: 0.0,
            y: 0.0,
            toggle_at: None,
        })
        .collect();
    for index in 0..measured.len() {
        if let Some(parent) = measured[index].parent {
            measured[parent].children.push(index);
        }
    }
    let root = measured
        .iter()
        .position(|node| node.parent.is_none())
        .expect("the input gate admits exactly one root");
    let mut pending = vec![(root, 0)];
    while let Some((index, depth)) = pending.pop() {
        measured[index].depth = depth;
        pending.extend(
            measured[index]
                .children
                .iter()
                .map(|child| (*child, depth + 1)),
        );
    }
    let mut groups = Vec::with_capacity(measured.len());
    for index in 0..measured.len() {
        let mut owned = Vec::new();
        let mut run = Vec::new();
        let flush = |run: &mut Vec<usize>, owned: &mut Vec<Group>| {
            if down && run.len() >= STACK_MIN {
                owned.push(Group::Stack {
                    members: std::mem::take(run),
                    spine_offset: 0.0,
                    indent: 0.0,
                    width: 0.0,
                    spine: 0.0,
                });
            } else {
                owned.extend(run.drain(..).map(Group::Single));
            }
        };
        for &child in &measured[index].children {
            if measured[child].children.is_empty() {
                run.push(child);
            } else {
                flush(&mut run, &mut owned);
                owned.push(Group::Single(child));
            }
        }
        flush(&mut run, &mut owned);
        // A list that is its parent's only child group drops its spine from
        // inside the parent's box, clear of the rounded corner.
        let only = owned.len() == 1;
        for (group_index, group) in owned.iter_mut().enumerate() {
            if let Group::Stack {
                members,
                spine_offset,
                indent,
                ..
            } = group
            {
                *spine_offset = if only { 22.0 } else { 12.0 };
                *indent = *spine_offset + 16.0;
                for &member in members.iter() {
                    measured[member].stack = Some(group_index);
                }
            }
        }
        groups.push(owned);
    }
    (root, measured, groups)
}

// Text wraps only when it must, and then at the narrowest width keeping the
// same number of lines.
fn measure(raw: &Value, node: &mut Measured, is_root: bool, settings: Settings) {
    node.label_font = if is_root { ROOT_LABEL_FONT } else { LABEL_FONT };
    let label_advance = node.label_font * ADVANCE * 1.06;
    let sublabel = string(raw, "sublabel");
    let longest_word = node
        .label
        .split(is_js_whitespace)
        .map(|word| units(word) * label_advance)
        .fold(f64::NEG_INFINITY, f64::max);
    let floor = settings
        .node_w
        .max((longest_word + PAD_X * 2.0 + 2.0).ceil());
    let cap = (settings.node_max_w * 1.5)
        .min(floor.max(settings.node_max_w))
        .ceil();
    let (label, font) = (node.label.clone(), node.label_font);
    let lines_at = |width: f64| {
        (
            wrap_text(&label, font * 1.06, width - PAD_X * 2.0),
            if sublabel.is_empty() {
                vec![]
            } else {
                wrap_text(sublabel, SUBLABEL_FONT, width - PAD_X * 2.0)
            },
        )
    };
    let sublabel_width = if sublabel.is_empty() {
        0.0
    } else {
        units(sublabel) * SUBLABEL_FONT * ADVANCE
    };
    let natural = ((units(&label) * label_advance).max(sublabel_width) + PAD_X * 2.0 + 2.0).ceil();
    let mut width = cap.min(floor.max(natural));
    let target = lines_at(width);
    let mut candidate = floor;
    while node.stack.is_none() && candidate < width {
        let lines = lines_at(candidate);
        if lines.0.len() <= target.0.len() && lines.1.len() <= target.1.len() {
            width = candidate;
            break;
        }
        candidate += 4.0;
    }
    let (label_lines, sublabel_lines) = lines_at(width);
    node.content_height = label_lines.len() as f64 * LABEL_LINE
        + if sublabel_lines.is_empty() {
            0.0
        } else {
            4.0 + sublabel_lines.len() as f64 * SUBLABEL_LINE
        };
    node.height = (PAD_Y * 2.0 + node.content_height).ceil();
    node.width = width;
    node.label_lines = label_lines;
    node.sublabel_lines = sublabel_lines;
}

struct Placement<'a> {
    settings: Settings,
    band_size: &'a [f64],
    band_start: &'a [f64],
    span: &'a [f64],
}

fn measure_span(
    id: usize,
    nodes: &[Measured],
    groups: &[Vec<Group>],
    span: &mut [f64],
    settings: Settings,
) {
    for &child in &nodes[id].children {
        measure_span(child, nodes, groups, span, settings);
    }
    span[id] =
        settings
            .breadth(&nodes[id])
            .max(groups_span(&groups[id], span, settings.breadth_gap()));
}

// Each subtree owns a contiguous span; leaves keep their order and a parent is
// centred over its children. Positions never depend on collapse state.
fn place(
    id: usize,
    start: f64,
    context: &Placement<'_>,
    nodes: &mut [Measured],
    groups: &mut [Vec<Group>],
) {
    let settings = context.settings;
    let down = settings.down;
    let gap = settings.breadth_gap();
    let only_stack = groups[id].len() == 1 && matches!(groups[id][0], Group::Stack { .. });
    let mut cursor = if only_stack {
        start
    } else {
        start + (context.span[id] - groups_span(&groups[id], context.span, gap)) / 2.0
    };
    let depth = nodes[id].depth;
    for index in 0..groups[id].len() {
        let extent = group_span(&groups[id][index], context.span);
        let single = match &mut groups[id][index] {
            Group::Stack {
                members,
                spine_offset,
                indent,
                spine,
                ..
            } => {
                *spine = cursor + *spine_offset;
                let mut y = context.band_start[depth + 1];
                for &member in members.iter() {
                    let leaf = &mut nodes[member];
                    leaf.x = js_round(cursor + *indent);
                    leaf.y = js_round(y);
                    y += leaf.height + STACK_GAP;
                }
                None
            }
            Group::Single(child) => Some(*child),
        };
        if let Some(child) = single {
            place(child, cursor, context, nodes, groups);
        }
        cursor += extent + gap;
    }
    let center_of = |node: &Measured| {
        if down {
            node.x + node.width / 2.0
        } else {
            node.y + node.height / 2.0
        }
    };
    let attach = |group: &Group| match group {
        Group::Stack { spine, .. } => *spine,
        Group::Single(child) => center_of(&nodes[*child]),
    };
    let owned = &groups[id];
    let mut center = match (owned.first(), owned.last()) {
        (Some(first), Some(last)) => (attach(first) + attach(last)) / 2.0,
        _ => start + context.span[id] / 2.0,
    };
    let half = settings.breadth(&nodes[id]) / 2.0;
    // A middle child a few px off the midpoint would put a small jog in the
    // stem; the parent sits exactly over it instead.
    if owned.len() > 2 {
        if let Some(middle) = owned[1..owned.len() - 1].iter().map(attach).find(|attach| {
            (attach - center).abs() < 24.0
                && attach - half >= start
                && attach + half <= start + context.span[id]
        }) {
            center = middle;
        }
    }
    let toggle_at = match owned.first() {
        Some(Group::Stack { spine, .. }) if only_stack => Some(js_round(*spine)),
        _ => None,
    };
    let along = if only_stack { start } else { center - half };
    let node = &mut nodes[id];
    let across = context.band_start[depth]
        + if down {
            (context.band_size[depth] - node.height) / 2.0
        } else {
            0.0
        };
    let (x, y) = if down {
        (along, across)
    } else {
        (across, along)
    };
    node.x = js_round(x);
    node.y = js_round(y);
    node.toggle_at = toggle_at;
}

fn solve(nodes: &[Value], settings: Settings, authored_view_box: Option<[f64; 2]>) -> Solved {
    let down = settings.down;
    let (root, mut measured, mut groups) = structure(nodes, down);
    for (index, raw) in nodes.iter().enumerate() {
        measure(raw, &mut measured[index], index == root, settings);
    }
    // A stacked run reads as one list: one width, one row height.
    for owned in &mut groups {
        for group in owned.iter_mut() {
            if let Group::Stack { members, width, .. } = group {
                *width = members
                    .iter()
                    .map(|&member| measured[member].width)
                    .fold(f64::NEG_INFINITY, f64::max);
                let row_height = members
                    .iter()
                    .map(|&member| measured[member].height)
                    .fold(f64::NEG_INFINITY, f64::max);
                for &member in members.iter() {
                    measured[member].width = *width;
                    measured[member].height = row_height;
                }
            }
        }
    }
    // Every generation shares one band sized by its largest node.
    let depth_size = |node: &Measured| if down { node.height } else { node.width };
    let max_depth = measured.iter().map(|node| node.depth).max().unwrap_or(0);
    let mut band_size = vec![0.0f64; max_depth + 1];
    for node in measured.iter().filter(|node| node.stack.is_none()) {
        band_size[node.depth] = band_size[node.depth].max(depth_size(node));
    }
    let mut band_start = Vec::with_capacity(band_size.len());
    let mut offset = if down { TOP } else { MARGIN };
    for size in &band_size {
        band_start.push(offset);
        offset += size + settings.depth_gap();
    }
    for node in measured.iter_mut().filter(|node| node.stack.is_none()) {
        if down {
            node.height = band_size[node.depth];
        } else {
            node.width = band_size[node.depth];
        }
    }
    let mut span = vec![0.0; measured.len()];
    measure_span(root, &measured, &groups, &mut span, settings);
    let context = Placement {
        settings,
        band_size: &band_size,
        band_start: &band_start,
        span: &span,
    };
    place(
        root,
        if down { MARGIN } else { TOP },
        &context,
        &mut measured,
        &mut groups,
    );
    let view_box = authored_view_box.unwrap_or_else(|| {
        let right = measured
            .iter()
            .map(|node| node.x + node.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = measured
            .iter()
            .map(|node| node.y + node.height)
            .fold(f64::NEG_INFINITY, f64::max);
        [
            320f64.max((right + MARGIN + if down { 0.0 } else { TOGGLE_R + 4.0 }).ceil()),
            240f64.max((bottom + MARGIN + if down { TOGGLE_R + 4.0 } else { 0.0 }).ceil()),
        ]
    });
    Solved {
        root,
        nodes: measured,
        groups,
        band_start,
        view_box,
    }
}

/// Where a parent's links leave it, which is also where its branch toggle sits.
pub(crate) fn edge_start(bounds: Rect, toggle_at: Option<f64>, down: bool) -> Point {
    if down {
        point(
            toggle_at.unwrap_or(bounds.x + bounds.width / 2.0),
            bounds.y + bounds.height,
        )
    } else {
        point(bounds.x + bounds.width, bounds.y + bounds.height / 2.0)
    }
}

// One elbow per link: out of the parent's toggle side, across at the middle
// of the generation gap, into the child. A stacked child is reached along its
// run's spine and entered from the left.
fn edge_points(solved: &Solved, settings: Settings, parent: usize, child: usize) -> Vec<Point> {
    let (from, to) = (&solved.nodes[parent], &solved.nodes[child]);
    let start = edge_start(from.rect(), from.toggle_at, settings.down);
    let middle = solved.band_start[to.depth] - settings.depth_gap() / 2.0;
    if !settings.down {
        let end = point(to.x, to.y + to.height / 2.0);
        if (start.y - end.y).abs() < 2.0 {
            let y = js_round((start.y + end.y) / 2.0);
            return vec![point(start.x, y), point(end.x, y)];
        }
        return vec![start, point(middle, start.y), point(middle, end.y), end];
    }
    if let Some(group) = to.stack {
        let Group::Stack { spine, .. } = solved.groups[parent][group] else {
            unreachable!("stacked leaves belong to a stack group");
        };
        let end = point(to.x, to.y + to.height / 2.0);
        // The reference starts an only-child list at the rounded toggle x but
        // runs its spine at the unrounded x; keep the spine on the toggle x
        // so the link stays orthogonal.
        if let Some(toggle_at) = from.toggle_at {
            return vec![start, point(toggle_at, end.y), end];
        }
        return vec![
            start,
            point(start.x, middle),
            point(spine, middle),
            point(spine, end.y),
            end,
        ];
    }
    let end = point(to.x + to.width / 2.0, to.y);
    // Rounded parent and child centres can differ by about 1px; a shared x
    // keeps the stem straight.
    if (start.x - end.x).abs() < 2.0 {
        let x = js_round((start.x + end.x) / 2.0);
        return vec![point(x, start.y), point(x, end.y)];
    }
    vec![start, point(start.x, middle), point(end.x, middle), end]
}

fn readable(solved: &Solved) -> bool {
    let smallest = solved
        .nodes
        .iter()
        .map(|node| {
            if node.sublabel_lines.is_empty() {
                node.label_font
            } else {
                SUBLABEL_FONT
            }
        })
        .fold(f64::INFINITY, f64::min);
    smallest * (READER_WIDTH / solved.view_box[0]).min(1.0) >= READABLE_PX
}

fn text(path: String, content: &str, x: f64, y: f64, font_size: f64, anchor: &str) -> Text {
    Text {
        source_path: path,
        content: content.into(),
        position: point(x, y),
        font_size,
        anchor: anchor.into(),
    }
}

/// HTML note cards; their text is rendered from the authored facts.
pub(crate) fn cards(document: &Value) -> Vec<Card> {
    document["cards"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .enumerate()
        .map(|(index, raw)| Card {
            authored: Authored {
                id: format!("cards-{index}"),
                source_path: format!("/cards/{index}"),
                kind: "card".into(),
                facts: raw.clone(),
                geometry_provenance: vec![],
            },
            bounds: Rect::default(),
            text: vec![],
        })
        .collect()
}

fn viewbox_failures(solved: &Solved) -> Diagnostics {
    let [width, height] = solved.view_box;
    solved
        .nodes
        .iter()
        .filter(|node| {
            node.x + node.width + MARGIN / 2.0 > width
                || node.y + node.height + MARGIN / 2.0 > height
        })
        .map(|node| {
            let mut diagnostic = Diagnostic::error(
                Some("tree"),
                "layout/constraint",
                "/meta/viewBox",
                format!(
                    "Node \"{}\" lies outside the authored meta.viewBox {width}x{height}",
                    node.id
                ),
            );
            diagnostic.stage = "layout".into();
            diagnostic.subject.identity = Some(node.id.clone());
            diagnostic.evidence = json!({"bounds":node.rect(),"viewBox":[width,height]});
            diagnostic.supported_fixes =
                vec!["Remove meta.viewBox or enlarge it; every node and branch is kept".into()];
            diagnostic
        })
        .collect()
}

fn scene_node(raw: &Value, index: usize, node: &Measured) -> Node {
    let role = if node.parent.is_none() {
        "tree-root"
    } else if node.children.is_empty() {
        "tree-leaf"
    } else {
        "tree-branch"
    };
    let stacked = node.stack.is_some();
    // A stacked list reads left-aligned, like a directory listing.
    let (x, anchor) = if stacked {
        (node.x + PAD_X + 2.0, "start")
    } else {
        (node.x + node.width / 2.0, "middle")
    };
    let mut y = node.y + (node.height - node.content_height) / 2.0;
    let mut texts = Vec::new();
    for line in &node.label_lines {
        y += LABEL_LINE;
        texts.push(text(
            format!("/nodes/{index}/label"),
            line,
            x,
            y - 4.5,
            node.label_font,
            anchor,
        ));
    }
    if !node.sublabel_lines.is_empty() {
        y += 4.0;
    }
    for line in &node.sublabel_lines {
        y += SUBLABEL_LINE;
        texts.push(text(
            format!("/nodes/{index}/sublabel"),
            line,
            x,
            y - 3.5,
            SUBLABEL_FONT,
            anchor,
        ));
    }
    Node {
        brand: None,
        authored: Authored {
            id: node.id.clone(),
            source_path: format!("/nodes/{index}"),
            kind: role.into(),
            facts: raw.clone(),
            geometry_provenance: vec![if stacked { STACKED } else { BANDED }.into()],
        },
        bounds: node.rect(),
        sigil: None,
        label: node.label.clone(),
        text: texts,
        detail_rows: vec![],
    }
}

fn toggle(raw: &Value, index: usize, node: &Measured, count: usize, down: bool) -> Container {
    let center = edge_start(node.rect(), node.toggle_at, down);
    let pill = Rect {
        x: center.x + TOGGLE_R + 3.0,
        y: center.y - 7.5,
        width: count.to_string().len() as f64 * 6.2 + 11.0,
        height: 15.0,
    };
    Container {
        authored: Authored {
            id: node.id.clone(),
            source_path: format!("/nodes/{index}"),
            kind: "tree-toggle".into(),
            facts: json!({"count":count,"initiallyCollapsed":raw["collapsed"] == true}),
            geometry_provenance: vec!["tree-edge-exit".into()],
        },
        bounds: Rect {
            x: center.x - TOGGLE_R,
            y: center.y - TOGGLE_R,
            width: TOGGLE_R * 2.0,
            height: TOGGLE_R * 2.0,
        },
        label_plate: Some(pill),
        rule: None,
        members: vec![node.id.clone()],
        text: vec![text(
            format!("/nodes/{index}"),
            &count.to_string(),
            pill.x + pill.width / 2.0,
            center.y + 3.6,
            10.0,
            "middle",
        )],
    }
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let nodes = document["nodes"].as_array().map_or(&[][..], Vec::as_slice);
    let layout = &document["layout"];
    let down = layout["direction"].as_str() != Some("right");
    let settings = Settings {
        down,
        gap_x: layout["gapX"]
            .as_f64()
            .unwrap_or(if down { 28.0 } else { 48.0 }),
        gap_y: layout["gapY"]
            .as_f64()
            .unwrap_or(if down { 56.0 } else { 18.0 }),
        node_w: layout["nodeW"].as_f64().unwrap_or(140.0),
        node_max_w: layout["nodeMaxW"]
            .as_f64()
            .unwrap_or(if down { 220.0 } else { 200.0 }),
    };
    let authored_view_box = meta["viewBox"]
        .as_array()
        .and_then(|values| Some([values.first()?.as_f64()?, values.get(1)?.as_f64()?]));
    let mut solved = solve(nodes, settings, authored_view_box);
    // When the default node width cap makes the tree too wide to read on the
    // desktop, narrow only that cap (wrapping text inside nodes) until it
    // fits. An authored canvas or cap is kept as written.
    if authored_view_box.is_none() && layout["nodeMaxW"].is_null() && !readable(&solved) {
        let mut node_max_w = settings.node_max_w - NODE_MAX_W_STEP;
        while node_max_w >= NODE_MAX_W_FLOOR {
            let candidate = solve(
                nodes,
                Settings {
                    node_max_w,
                    ..settings
                },
                None,
            );
            if readable(&candidate) {
                solved = candidate;
                break;
            }
            node_max_w -= NODE_MAX_W_STEP;
        }
    }
    if authored_view_box.is_some() {
        let failures = viewbox_failures(&solved);
        if !failures.is_empty() {
            return Err(failures);
        }
    }
    let scene_nodes = solved
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| scene_node(&nodes[index], index, node))
        .collect();
    let relations = solved
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let parent = node.parent?;
            let from = solved.nodes[parent].id.clone();
            Some(Relation {
                authored: Authored {
                    id: node.id.clone(),
                    source_path: format!("/nodes/{index}/parent"),
                    kind: "tree-edge".into(),
                    facts: json!({"from":from,"to":node.id}),
                    geometry_provenance: vec!["tree-elbow".into()],
                },
                crossover_halo: false,
                independent_ports: false,
                junction: None,
                label_owner: None,
                from,
                to: node.id.clone(),
                variant: "default".into(),
                label: String::new(),
                note: None,
                note_bounds: None,
                note_backdrop: None,
                points: edge_points(&solved, settings, parent, index),
                label_plate: None,
                text: vec![],
                marker_start: None,
                marker_end: None,
            })
        })
        .collect();
    let descendants = |index: usize| {
        let (mut count, mut pending) = (0, solved.nodes[index].children.clone());
        while let Some(child) = pending.pop() {
            count += 1;
            pending.extend(&solved.nodes[child].children);
        }
        count
    };
    let mut containers: Vec<Container> = solved
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| !node.children.is_empty())
        .map(|(index, node)| toggle(&nodes[index], index, node, descendants(index), down))
        .collect();
    let expand_all = crate::i18n::resolve(meta).text("tree.expandAll", &[]);
    containers.push(Container {
        authored: Authored {
            id: "tree-expand-all".into(),
            source_path: "/meta".into(),
            kind: "tree-expand-all".into(),
            facts: Value::Null,
            geometry_provenance: vec!["renderer-owned-control".into()],
        },
        bounds: Rect {
            x: MARGIN,
            y: 18.0,
            width: (units(&expand_all) * 10.5 * ADVANCE + 24.0).ceil(),
            height: 24.0,
        },
        label_plate: None,
        rule: None,
        members: vec![],
        text: vec![text(
            "/meta".into(),
            &expand_all,
            MARGIN + 12.0,
            34.0,
            10.5,
            "start",
        )],
    });
    let [width, height] = solved.view_box;
    Ok(CompiledDiagram {
        diagram_type: DiagramType::Tree,
        schema_version: document["schema_version"].as_u64().unwrap_or(1) as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: meta.clone(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        },
        reader_fit: if authored_view_box.is_some() {
            "contain"
        } else {
            "intrinsic-height"
        }
        .into(),
        nodes: scene_nodes,
        relations,
        containers,
        text: vec![],
        legends: vec![],
        cards: cards(document),
        facts: DiagramFacts::Tree {
            root: solved.nodes[solved.root].id.clone(),
            nodes: nodes.to_vec(),
            direction: if down { "down" } else { "right" }.into(),
        },
        diagnostics: vec![],
    })
}
