//! Statechart layout: leaves and parent title nodes placed by `place`, frames
//! for parents and regions, routes from the shared router, self-loops drawn
//! by hand, then the legend and the engine's report cards.
mod place;

use super::router::{self, Edge, Frame, Label, RouterOptions};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::text_units;
use crate::i18n::Locale;
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Relation, Text,
};
use crate::statechart::chart::{Child, Mark, Region, State};
use crate::statechart::{self, report};
use place::{text_width, Sizes};
use serde_json::{json, Value};
use std::collections::HashMap;

const MARGIN: f64 = 32.0;
const LEAF_H: f64 = 64.0;
const LEAF_MIN_W: f64 = 140.0;
const LEAF_MAX_W: f64 = 220.0;
const TEXT_MAX: f64 = 212.0;
const LABEL_FONT: f64 = 13.0;
const TAG_FONT: f64 = 12.0;
const TITLE_FONT: f64 = 12.0;
const EDGE_FONT: f64 = 10.0;
const LOOP_ROOM: f64 = 40.0;
/// Port spacing that keeps a 15px label between two parallel arrows.
const PORT_PITCH: f64 = 26.0;
/// Parallel runs closer than this leave no room for a label between them.
const CLEAR_APART: f64 = 24.0;
const LEGEND_ROW: f64 = 22.0;
/// Widest content the reader shows with its smallest text at the reading floor.
const READABLE_WIDTH: f64 = place::MAX_WIDTH;
/// Charts larger than this cannot be drawn readably; checked before placement.
const CHART_STATES: usize = 60;
const CHART_ARROWS: usize = 40;

/// The tone variable behind each phase colour; `rose` is kept for breaks.
pub(crate) fn tone(color: Option<&str>) -> &'static str {
    match color {
        Some("cyan") => "frontend",
        Some("emerald") => "backend",
        Some("violet") => "database",
        Some("amber") => "cloud",
        Some("orange") => "messagebus",
        _ => "external",
    }
}

pub(crate) fn mark_name(mark: Option<Mark>) -> Option<&'static str> {
    mark.map(|mark| match mark {
        Mark::Breaks => "breaks",
        Mark::MayBreak => "may-break",
    })
}

fn text(path: &str, content: &str, x: f64, y: f64, font_size: f64, anchor: &str) -> Text {
    Text {
        source_path: path.into(),
        content: content.into(),
        position: Point { x, y },
        font_size,
        anchor: anchor.into(),
    }
}

fn authored(id: String, source_path: &str, kind: &str, facts: Value, provenance: &str) -> Authored {
    Authored {
        id,
        source_path: source_path.into(),
        kind: kind.into(),
        facts,
        geometry_provenance: vec![provenance.into()],
    }
}

/// The second line of a marked leaf: rules always broken, then rules that
/// may break.
pub(crate) fn tag(state: &State, locale: &Locale) -> Option<String> {
    let join = locale.text("statechart.join.list", &[]);
    let mut parts = Vec::new();
    if !state.breaks.is_empty() {
        parts.push(locale.text(
            "node.context.statechart.breaks",
            &[("rules", &state.breaks.join(&join))],
        ));
    }
    if !state.may_break.is_empty() {
        parts.push(locale.text(
            "node.context.statechart.may-break",
            &[("rules", &state.may_break.join(&join))],
        ));
    }
    (!parts.is_empty()).then(|| parts.join(&join))
}

fn leaf_label(state: &State) -> &str {
    state.path.last().map_or("", String::as_str)
}

/// Where a chart state sits: its region, then its parents, outermost first.
fn location(region: &Region, path: &[String]) -> String {
    region
        .name
        .iter()
        .chain(&path[..path.len() - 1])
        .cloned()
        .collect::<Vec<_>>()
        .join(" \u{203a} ")
}

fn region_pointer(region: &Region) -> &'static str {
    if region.name.is_some() {
        "/view/regions"
    } else {
        "/view/state"
    }
}

fn layout_error(code: &str, path: &str, message: String, evidence: Value, fix: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(Some("statechart"), code, path, message);
    diagnostic.stage = "layout".into();
    diagnostic.evidence = evidence;
    diagnostic.supported_fixes = vec![fix.into()];
    diagnostic
}

/// A self-loop over the leaf's top-right corner, ending on its right side.
fn loop_points(leaf: Rect) -> Vec<Point> {
    let right = leaf.x + leaf.width;
    vec![
        Point {
            x: right - 24.0,
            y: leaf.y,
        },
        Point {
            x: right - 24.0,
            y: leaf.y - 20.0,
        },
        Point {
            x: right + 20.0,
            y: leaf.y - 20.0,
        },
        Point {
            x: right + 20.0,
            y: leaf.y + 16.0,
        },
        Point {
            x: right,
            y: leaf.y + 16.0,
        },
    ]
}

/// The label of a self-loop, centred over the loop's top segment.
fn loop_plate(leaf: Rect, raw: &Value) -> (Rect, Point) {
    let width = text_width(raw["label"].as_str().unwrap_or_default(), EDGE_FONT);
    let anchor = Point {
        x: leaf.x + leaf.width - 2.0,
        y: leaf.y - 25.0,
    };
    (
        Rect {
            x: anchor.x - width / 2.0,
            y: anchor.y - 11.0,
            width,
            height: 15.0,
        },
        anchor,
    )
}

/// The last stretch of a route, where its arrowhead is drawn.
fn arrowhead(points: &[Point]) -> Option<Rect> {
    let [.., a, b] = points else {
        return None;
    };
    let (dx, dy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let (tail_x, tail_y) = (b.x - dx * 16.0, b.y - dy * 16.0);
    Some(Rect {
        x: b.x.min(tail_x) - 5.0 * dy.abs(),
        y: b.y.min(tail_y) - 5.0 * dx.abs(),
        width: (b.x - tail_x).abs() + 10.0 * dy.abs(),
        height: (b.y - tail_y).abs() + 10.0 * dx.abs(),
    })
}

struct LegendEntry {
    kind: String,
    label: String,
    facts: Value,
    lead: f64,
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let locale = crate::i18n::resolve(meta);
    let analysis = statechart::analyze(document)?;
    let chart = &analysis.chart;
    let regions = &chart.regions;
    let count = regions.len();

    if chart.state_count() > CHART_STATES || chart.arrow_count() > CHART_ARROWS {
        return Err(vec![layout_error(
            "statechart/chart-too-large",
            "/view",
            format!(
                "The chart has {} states and {} arrows; at most {CHART_STATES} states and {CHART_ARROWS} arrows can be drawn readably",
                chart.state_count(),
                chart.arrow_count()
            ),
            json!({"states":chart.state_count(),"arrows":chart.arrow_count(),"stateLimit":CHART_STATES,"arrowLimit":CHART_ARROWS}),
            "Merge model states into fewer chart states with view.state labels, hide steps that do not matter with a null view.steps label, or shrink the model's domains",
        )]);
    }

    // ---- Measure ----
    let mut errors = Vec::new();
    let mut sizes = Sizes {
        leaves: Vec::new(),
        loop_room: Vec::new(),
        titles: Vec::new(),
        plates: Vec::new(),
        labels: Vec::new(),
    };
    let mut tags: Vec<Vec<Option<String>>> = Vec::new();
    let mut arrow_text: Vec<Vec<String>> = Vec::new();
    for region in regions {
        let mut leaves = Vec::new();
        let mut region_tags = Vec::new();
        for state in &region.states {
            let tag = tag(state, &locale);
            let needed = (text_units(leaf_label(state)) * LABEL_FONT * 0.6).max(
                tag.as_deref()
                    .map_or(0.0, |tag| text_units(tag) * TAG_FONT * 0.6),
            );
            if needed > TEXT_MAX {
                errors.push(layout_error(
                    "statechart/state-text-too-long",
                    region_pointer(region),
                    format!(
                        "The state {} needs {}px of text; a state holds at most {TEXT_MAX}px",
                        state.path.join(" / "),
                        needed.ceil()
                    ),
                    json!({"path":state.path,"widthPx":needed.ceil(),"limitPx":TEXT_MAX}),
                    "Shorten the state label in view.state, or move detail into a parent path",
                ));
            }
            // Arrows between the same two states leave side by side; each
            // needs a port far enough from the next for its label.
            let mut pairs: HashMap<&[String], usize> = HashMap::new();
            for arrow in region.arrows.iter().filter(|arrow| arrow.from != arrow.to) {
                if arrow.from == state.path {
                    *pairs.entry(&arrow.to).or_default() += 1;
                } else if arrow.to == state.path {
                    *pairs.entry(&arrow.from).or_default() += 1;
                }
            }
            let parallel = pairs.values().copied().max().unwrap_or(0) as f64;
            leaves.push((
                (needed + 28.0).clamp(LEAF_MIN_W, LEAF_MAX_W).ceil(),
                LEAF_H.max(PORT_PITCH * (parallel + 1.0)),
            ));
            region_tags.push(tag);
        }
        let loops = region
            .states
            .iter()
            .map(|state| {
                if region
                    .arrows
                    .iter()
                    .any(|arrow| arrow.from == state.path && arrow.to == state.path)
                {
                    LOOP_ROOM
                } else {
                    0.0
                }
            })
            .collect();
        let titles = region
            .parents
            .iter()
            .map(|parent| {
                text_width(parent.path.last().map_or("", String::as_str), TITLE_FONT).max(72.0)
                    + 12.0
            })
            .collect();
        let texts: Vec<String> = region
            .arrows
            .iter()
            .map(|arrow| report::arrow_label(arrow, count, &locale))
            .collect();
        sizes.labels.push(
            texts
                .iter()
                .map(|text| text_width(text, EDGE_FONT))
                .collect(),
        );
        sizes.leaves.push(leaves);
        sizes.loop_room.push(loops);
        sizes.titles.push(titles);
        sizes.plates.push(
            region
                .name
                .as_deref()
                .map_or(0.0, |name| text_width(name, TITLE_FONT) + 8.0),
        );
        tags.push(region_tags);
        arrow_text.push(texts);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let placement = place::place(regions, &sizes, (MARGIN, MARGIN));
    if placement.bounds.height > place::MAX_TALL * placement.bounds.width {
        return Err(vec![layout_error(
            "statechart/chart-too-tall",
            "/view",
            format!(
                "The chart is {}px tall and {}px wide; taller than {} times its width it cannot be read on a phone",
                placement.bounds.height.ceil(),
                placement.bounds.width.ceil(),
                place::MAX_TALL
            ),
            json!({"widthPx":placement.bounds.width.ceil(),"heightPx":placement.bounds.height.ceil(),"ratioLimit":place::MAX_TALL}),
            "Merge states with shared labels in view.state, or hide steps that do not matter to the reader",
        )]);
    }
    if placement.bounds.width > READABLE_WIDTH {
        return Err(vec![layout_error(
            "statechart/chart-too-wide",
            "/view",
            format!(
                "The chart needs {}px of width; at most {READABLE_WIDTH}px stays readable",
                placement.bounds.width.ceil()
            ),
            json!({"widthPx":placement.bounds.width.ceil(),"limitPx":READABLE_WIDTH,"states":chart.state_count(),"arrows":chart.arrow_count()}),
            "Merge states with shared labels in view.state, or hide steps that do not matter to the reader",
        )]);
    }

    // ---- Nodes ----
    let mut nodes: Vec<Node> = Vec::new();
    let mut node_of: HashMap<(usize, &[String]), usize> = HashMap::new();
    let mut facts_states = Vec::new();
    for (r, region) in regions.iter().enumerate() {
        for (leaf, state) in region.states.iter().enumerate() {
            let id = format!("state-{r}-{leaf}");
            let bounds = placement.leaves[r][leaf];
            let label = leaf_label(state);
            let tag = tags[r][leaf].clone();
            let mut texts = Vec::new();
            let center = bounds.x + bounds.width / 2.0;
            let source = region_pointer(region);
            match &tag {
                Some(tag) => {
                    texts.push(text(
                        &format!("{source}/label"),
                        label,
                        center,
                        bounds.y + 27.0,
                        LABEL_FONT,
                        "middle",
                    ));
                    texts.push(text(
                        &format!("{source}/tag"),
                        tag,
                        center,
                        bounds.y + 47.0,
                        TAG_FONT,
                        "middle",
                    ));
                }
                None => texts.push(text(
                    &format!("{source}/label"),
                    label,
                    center,
                    bounds.y + bounds.height / 2.0 + 4.5,
                    LABEL_FONT,
                    "middle",
                )),
            }
            let mut facts = json!({"id":id,"label":label,"path":state.path,"members":state.members,"location":location(region, &state.path)});
            if let Some(tag) = &tag {
                facts["tag"] = json!(tag);
            }
            facts_states.push(json!({"id":id,"region":r,"path":state.path,"kind":"state","mark":mark_name(state.mark),
                "bugPath":state.bug_path,"tone":tone(state.color.as_deref()),"start":state.start,"members":state.members,
                "breaks":state.breaks,"mayBreak":state.may_break}));
            node_of.insert((r, state.path.as_slice()), nodes.len());
            nodes.push(Node {
                brand: None,
                authored: authored(
                    id.clone(),
                    source,
                    "state",
                    facts,
                    "statechart-compound-layers",
                ),
                bounds,
                sigil: None,
                label: label.into(),
                text: texts,
                detail_rows: vec![],
            });
        }
        for (p, parent) in region.parents.iter().enumerate() {
            let id = format!("parent-{r}-{p}");
            let bounds = placement.titles[r][p];
            let label = parent.path.last().map_or("", String::as_str);
            let members: usize = leaves_below(region, Child::Parent(p))
                .into_iter()
                .map(|leaf| region.states[leaf].members)
                .sum();
            facts_states.push(json!({"id":id,"region":r,"path":parent.path,"kind":"parent","mark":null,"bugPath":false,
                "tone":"external","start":false,"members":members}));
            node_of.insert((r, parent.path.as_slice()), nodes.len());
            nodes.push(Node {
                brand: None,
                authored: authored(
                    id.clone(),
                    region_pointer(region),
                    "parent",
                    json!({"id":id,"label":label,"path":parent.path,"location":location(region, &parent.path)}),
                    "statechart-frame-title",
                ),
                bounds,
                sigil: None,
                label: label.into(),
                text: vec![text(
                    &format!("{}/label", region_pointer(region)),
                    label,
                    bounds.x + 12.0,
                    bounds.y + 18.5,
                    TITLE_FONT,
                    "start",
                )],
                detail_rows: vec![],
            });
        }
    }

    // ---- Routes ----
    let mut edges = Vec::new();
    let mut loops = Vec::new();
    let mut order: Vec<(usize, usize)> = Vec::new();
    for (r, region) in regions.iter().enumerate() {
        for (a, arrow) in region.arrows.iter().enumerate() {
            order.push((r, a));
            let id = format!("arrow-{r}-{a}");
            let variant = if arrow.bug_path {
                "emphasis"
            } else {
                "default"
            };
            let raw = json!({"id":id,"label":arrow_text[r][a],"variant":variant});
            let from = node_of[&(r, arrow.from.as_slice())];
            let to = node_of[&(r, arrow.to.as_slice())];
            if arrow.from == arrow.to {
                loops.push((r, a, from, raw));
            } else {
                edges.push(Edge {
                    raw,
                    index: edges.len(),
                    from,
                    to,
                });
            }
        }
    }
    let frames: Vec<Frame> = placement
        .frames
        .iter()
        .flatten()
        .map(|bounds| Frame {
            bounds: *bounds,
            radius: 12.0,
        })
        .chain(placement.regions.iter().map(|bounds| Frame {
            bounds: *bounds,
            radius: 8.0,
        }))
        .collect();
    // A self-loop and its label are drawn by hand; the router keeps clear of them.
    let loop_area = |node: usize, raw: &Value| {
        let leaf = nodes[node].bounds;
        let (plate, _) = loop_plate(leaf, raw);
        let left = plate.x.min(leaf.x + leaf.width - 72.0) - 4.0;
        let right = (plate.x + plate.width).max(leaf.x + leaf.width + 24.0) + 4.0;
        [
            Rect {
                x: left,
                y: leaf.y - LOOP_ROOM,
                width: right - left,
                height: LOOP_ROOM - 2.0,
            },
            Rect {
                x: leaf.x + leaf.width + 2.0,
                y: leaf.y - LOOP_ROOM,
                width: 22.0,
                height: 40.0 + LOOP_ROOM - 20.0,
            },
        ]
    };
    let mut obstacles = nodes.clone();
    for (_, _, node, raw) in &loops {
        for (k, bounds) in loop_area(*node, raw).into_iter().enumerate() {
            let mut blocker = nodes[*node].clone();
            blocker.authored.id = format!("{}-loop-{k}", blocker.authored.id);
            blocker.bounds = bounds;
            obstacles.push(blocker);
        }
    }
    // The router may pack two arrows a few pixels apart or end one where
    // another continues on the same line; either reads as one arrow. Route
    // with a few spacing preferences and keep the clearest result.
    let base = RouterOptions {
        component_gap_px: 10.0,
        max_port_spacing: PORT_PITCH,
        minimum_terminal_segment_px: 16.0,
        ..RouterOptions::default()
    };
    let alternatives = [
        base,
        RouterOptions {
            crossing_free_grid_first: true,
            ..base
        },
        RouterOptions {
            component_gap_px: 18.0,
            ..base
        },
        RouterOptions {
            crossing_free_grid_first: true,
            component_gap_px: 18.0,
            ..base
        },
    ];
    let mut routes = Vec::new();
    let mut best = usize::MAX;
    for options in alternatives {
        let candidate = router::route_with(&obstacles, &edges, &frames, false, options);
        let points: Vec<&[Point]> = candidate
            .iter()
            .map(|route| route.points.as_slice())
            .collect();
        let score = 1000 * through_nodes(&points, &edges, &nodes) + crowding(&points);
        if score < best {
            best = score;
            routes = candidate;
        }
        if best == 0 {
            break;
        }
    }
    spread(&mut routes, &edges, &nodes);
    let mut labels: Vec<Label> = edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let mut label = Label::new(index, &edge.raw, &routes[index].points)?;
            let width = text_width(edge.raw["label"].as_str().unwrap_or_default(), EDGE_FONT);
            label.rect = Rect {
                x: label.anchor.x - width / 2.0,
                y: label.anchor.y - 11.0,
                width,
                height: 15.0,
            };
            Some(label)
        })
        .collect();
    let plates: Vec<Rect> = placement
        .regions
        .iter()
        .zip(&sizes.plates)
        .map(|(frame, width)| Rect {
            x: frame.x + place::TITLE_INSET.0,
            y: frame.y + place::TITLE_INSET.1,
            width: *width,
            height: 22.0,
        })
        .collect();
    let mut extent = placement.bounds;
    for route in &routes {
        for point in &route.points {
            grow(
                &mut extent,
                Rect {
                    x: point.x,
                    y: point.y,
                    width: 0.0,
                    height: 0.0,
                },
            );
        }
    }
    for (r, a, _, _) in &loops {
        let leaf = placement.leaves[*r][regions[*r]
            .states
            .iter()
            .position(|s| s.path == regions[*r].arrows[*a].from)
            .expect("loop leaf")];
        grow(
            &mut extent,
            Rect {
                x: leaf.x,
                y: leaf.y - LOOP_ROOM,
                width: leaf.width + 24.0,
                height: leaf.height + LOOP_ROOM,
            },
        );
    }
    let canvas = Rect {
        x: 0.0,
        y: 0.0,
        width: extent.x + extent.width + MARGIN,
        height: extent.y + extent.height + MARGIN,
    };
    let titles: Vec<Rect> = plates
        .iter()
        .copied()
        .chain(placement.titles.iter().flatten().copied())
        .collect();
    let mut drawn = routes.clone();
    let mut reserved = titles.clone();
    for (_, _, node, raw) in &loops {
        let leaf = nodes[*node].bounds;
        drawn.push(router::Route {
            points: loop_points(leaf),
            sides: router::Sides {
                from: router::Side::Top,
                to: router::Side::Right,
            },
        });
        reserved.push(loop_plate(leaf, raw).0);
    }
    // A label flush against a state reads as part of it and hides its bug-path
    // ring; one over an arrow's last stretch hides the head that gives it a
    // direction. Keep labels off both.
    reserved.extend(routes.iter().filter_map(|route| arrowhead(&route.points)));
    let mut spaced = obstacles.clone();
    for node in &mut spaced {
        let b = node.bounds;
        node.bounds = Rect {
            x: b.x - CLEARANCE,
            y: b.y - CLEARANCE,
            width: b.width + 2.0 * CLEARANCE,
            height: b.height + 2.0 * CLEARANCE,
        };
    }
    router::place_labels(
        &mut labels,
        &drawn,
        &spaced,
        &reserved,
        canvas,
        canvas.height,
    );
    for label in &labels {
        grow(&mut extent, label.rect);
    }

    // ---- Relations ----
    let mut relations: Vec<Option<Relation>> = vec![None; order.len()];
    let position = |r: usize, a: usize| {
        order
            .iter()
            .position(|item| *item == (r, a))
            .expect("ordered arrow")
    };
    let mut facts_arrows = vec![Value::Null; order.len()];
    let relation = |raw: &Value,
                    from: usize,
                    to: usize,
                    points: Vec<Point>,
                    plate: Option<(Rect, Point)>,
                    routed: bool| Relation {
        authored: authored(
            raw["id"].as_str().unwrap_or_default().into(),
            "/view/steps",
            "statechart-arrow",
            json!({"id":raw["id"],"from":nodes[from].authored.id,"to":nodes[to].authored.id,"label":raw["label"],"variant":raw["variant"]}),
            "statechart-router",
        ),
        // Routed arrows may cross; the halo keeps each one traceable.
        crossover_halo: routed,
        independent_ports: true,
        junction: None,
        label_owner: None,
        from: nodes[from].authored.id.clone(),
        to: nodes[to].authored.id.clone(),
        variant: raw["variant"].as_str().unwrap_or("default").into(),
        label: raw["label"].as_str().unwrap_or_default().into(),
        note: None,
        note_bounds: None,
        note_backdrop: None,
        points,
        label_plate: plate.map(|(rect, _)| rect),
        text: plate
            .map(|(_, anchor)| {
                vec![text(
                    "/view/steps",
                    raw["label"].as_str().unwrap_or_default(),
                    anchor.x,
                    anchor.y,
                    EDGE_FONT,
                    "middle",
                )]
            })
            .unwrap_or_default(),
        marker_start: None,
        marker_end: None,
    };
    let mut routed = 0;
    for (r, region) in regions.iter().enumerate() {
        for (a, arrow) in region.arrows.iter().enumerate() {
            if arrow.from == arrow.to {
                continue;
            }
            let edge = &edges[routed];
            let plate = labels
                .iter()
                .find(|label| label.edge == routed)
                .map(|label| {
                    (
                        label.rect,
                        Point {
                            x: label.rect.x + label.rect.width / 2.0,
                            y: label.rect.y + 11.0,
                        },
                    )
                });
            relations[position(r, a)] = Some(relation(
                &edge.raw,
                edge.from,
                edge.to,
                routes[routed].points.clone(),
                plate,
                true,
            ));
            routed += 1;
        }
    }
    for (r, a, node, raw) in &loops {
        let leaf = nodes[*node].bounds;
        let (plate, anchor) = loop_plate(leaf, raw);
        grow(&mut extent, plate);
        relations[position(*r, *a)] = Some(relation(
            raw,
            *node,
            *node,
            loop_points(leaf),
            Some((plate, anchor)),
            false,
        ));
    }
    let relations: Vec<Relation> = relations
        .into_iter()
        .map(|relation| relation.expect("every arrow is drawn"))
        .collect();
    for (index, relation) in relations.iter().enumerate() {
        facts_arrows[index] = json!({"id":relation.authored.id,"from":relation.from,"to":relation.to,"label":relation.label});
    }

    // ---- Frames ----
    let mut containers = Vec::new();
    for (r, region) in regions.iter().enumerate() {
        for (p, parent) in region.parents.iter().enumerate() {
            let members = leaves_below(region, Child::Parent(p))
                .into_iter()
                .map(|leaf| format!("state-{r}-{leaf}"))
                .chain([format!("parent-{r}-{p}")])
                .collect();
            containers.push(Container {
                authored: authored(
                    format!("frame-{r}-{p}"),
                    region_pointer(region),
                    "statechart-parent",
                    json!({"path":parent.path}),
                    "statechart-compound-layers",
                ),
                bounds: placement.frames[r][p],
                label_plate: None,
                rule: None,
                members,
                text: vec![],
            });
        }
    }
    for (r, (frame, plate)) in placement.regions.iter().zip(&plates).enumerate() {
        let name = regions[r].name.clone().unwrap_or_default();
        containers.push(Container {
            authored: authored(
                format!("region-{r}"),
                &format!("/view/regions/{r}"),
                "statechart-region",
                json!({"name":name}),
                "statechart-region-stack",
            ),
            bounds: *frame,
            label_plate: Some(*plate),
            rule: None,
            members: (0..regions[r].states.len())
                .map(|leaf| format!("state-{r}-{leaf}"))
                .collect(),
            text: vec![text(
                &format!("/view/regions/{r}/name"),
                &name,
                plate.x + 8.0,
                plate.y + 15.5,
                TITLE_FONT,
                "start",
            )],
        });
    }

    // ---- Legend ----
    let states = || regions.iter().flat_map(|region| region.states.iter());
    let mut entries = Vec::new();
    let mut entry = |kind: &str, label: String, facts: Value, lead: f64| {
        entries.push(LegendEntry {
            kind: kind.into(),
            label,
            facts,
            lead,
        })
    };
    if states().any(|state| state.start) {
        entry(
            "statechart-start",
            locale.text("legend.statechart.start", &[]),
            json!({"glyph":"start"}),
            30.0,
        );
    }
    for (mark, key) in [(Mark::Breaks, "breaks"), (Mark::MayBreak, "may-break")] {
        if states().any(|state| state.mark == Some(mark)) {
            entry(
                &format!("statechart-{key}"),
                locale.text(&format!("legend.statechart.{key}"), &[]),
                json!({"glyph":key}),
                22.0,
            );
        }
    }
    if states().any(|state| state.bug_path) {
        entry(
            "statechart-bug-path",
            locale.text("legend.statechart.bug-path", &[]),
            json!({"glyph":"bug-path"}),
            22.0,
        );
    }
    let model = &analysis.model;
    let mut phases: Vec<(&String, &String)> = model
        .view
        .colors
        .iter()
        .filter(|(_, color)| states().any(|state| state.color.as_ref() == Some(color)))
        .collect();
    phases.sort_by_key(|(group, _)| {
        model
            .view
            .groups
            .iter()
            .position(|rule| &&rule.group == group)
    });
    for (group, color) in phases {
        entry(
            &format!("statechart-phase-{group}"),
            group.clone(),
            json!({"glyph":"tone","tone":tone(Some(color))}),
            22.0,
        );
    }
    let available = (canvas.width.max(extent.x + extent.width + MARGIN) - MARGIN * 2.0).max(1.0);
    let widths: Vec<f64> = entries
        .iter()
        .map(|entry| (entry.lead + text_units(&entry.label) * 13.0 * 0.62).ceil())
        .collect();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut cursor = 0.0;
    for (index, width) in widths.iter().enumerate() {
        match rows.last_mut() {
            Some(row) if cursor + 22.0 + width <= available => {
                row.push(index);
                cursor += 22.0 + width;
            }
            _ => {
                rows.push(vec![index]);
                cursor = *width;
            }
        }
    }
    let top = extent.y + extent.height + 28.0;
    let mut legends = Vec::new();
    if !entries.is_empty() {
        legends.push(Card {
            authored: authored(
                "legend-title".into(),
                "/meta",
                "legend-title",
                Value::Null,
                "resolved-legend-footprint",
            ),
            bounds: Rect {
                x: MARGIN,
                y: top,
                width: 48.0,
                height: 14.0,
            },
            text: vec![text(
                "/meta",
                &locale.text("legend.title", &[]),
                MARGIN,
                top + 10.0,
                12.0,
                "start",
            )],
        });
        for (row_index, row) in rows.iter().enumerate() {
            let baseline = top + 32.0 + row_index as f64 * LEGEND_ROW;
            let mut x = MARGIN;
            for index in row {
                let item = &entries[*index];
                let mut facts = item.facts.clone();
                facts["lead"] = json!(item.lead);
                legends.push(Card {
                    authored: authored(
                        item.kind.clone(),
                        "/view",
                        &item.kind,
                        facts,
                        "resolved-legend-footprint",
                    ),
                    bounds: Rect {
                        x,
                        y: baseline - 10.0,
                        width: widths[*index],
                        height: 14.0,
                    },
                    text: vec![text(
                        "/view",
                        &item.label,
                        x + item.lead,
                        baseline,
                        13.0,
                        "start",
                    )],
                });
                x += widths[*index] + 22.0;
            }
        }
    }
    let height = if entries.is_empty() {
        extent.y + extent.height + MARGIN
    } else {
        top + 32.0 + (rows.len() - 1) as f64 * LEGEND_ROW + 20.0
    };
    let width = (extent.x + extent.width + MARGIN).max(canvas.width);

    // ---- Cards ----
    let mut cards: Vec<Card> = report::cards(&analysis, &locale)
        .into_iter()
        .enumerate()
        .map(|(index, facts)| Card {
            authored: authored(
                format!("statechart-card-{index}"),
                "/model",
                "card",
                facts,
                "statechart-report",
            ),
            bounds: Rect::default(),
            text: vec![],
        })
        .collect();
    cards.extend(super::tree::cards(document));

    Ok(CompiledDiagram {
        diagram_type: DiagramType::Statechart,
        schema_version: document["schema_version"].as_u64().unwrap_or(1) as u32,
        quality: input.quality,
        meta: meta.clone(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width: width.ceil(),
            height: height.ceil(),
        },
        reader_fit: "intrinsic-height".into(),
        nodes,
        relations,
        containers,
        text: vec![],
        legends,
        cards,
        facts: DiagramFacts::Statechart {
            regions: chart.region_names.clone(),
            states: facts_states,
            arrows: facts_arrows,
            model: report::receipt(&analysis, &locale),
        },
        diagnostics: analysis.warnings.clone(),
    })
}

/// One axis-aligned piece of a route: the fixed coordinate and its span.
struct Run {
    horizontal: bool,
    at: f64,
    from: f64,
    to: f64,
}

fn runs(points: &[Point]) -> Vec<Run> {
    points
        .windows(2)
        .filter_map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            if (a.y - b.y).abs() < 0.5 && (a.x - b.x).abs() >= 0.5 {
                Some(Run {
                    horizontal: true,
                    at: a.y,
                    from: a.x.min(b.x),
                    to: a.x.max(b.x),
                })
            } else if (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() >= 0.5 {
                Some(Run {
                    horizontal: false,
                    at: a.x,
                    from: a.y.min(b.y),
                    to: a.y.max(b.y),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Two runs read as one arrow when they run side by side for 40px with no
/// room for a label between them, or sit on one line with a short gap.
fn crowds(apart: f64, shared: f64) -> bool {
    (apart < CLEAR_APART && shared >= 40.0) || (apart <= 6.0 && shared < 0.0 && -shared < 100.0)
}

/// Pairs of runs from different arrows that read as one (see `crowds`).
fn crowding(routes: &[&[Point]]) -> usize {
    let runs: Vec<Vec<Run>> = routes.iter().map(|points| runs(points)).collect();
    let mut count = 0;
    for (i, left) in runs.iter().enumerate() {
        for right in &runs[i + 1..] {
            for a in left {
                for b in right.iter().filter(|b| b.horizontal == a.horizontal) {
                    let apart = (a.at - b.at).abs();
                    let shared = a.to.min(b.to) - a.from.max(b.from);
                    if crowds(apart, shared) {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Every crowded pair of runs, as (route, segment) for both sides.
fn crowded_pairs(routes: &[&[Point]]) -> Vec<([(usize, usize); 2], f64)> {
    let indexed: Vec<Vec<(usize, Run)>> = routes
        .iter()
        .map(|points| {
            points
                .windows(2)
                .enumerate()
                .filter_map(|(segment, pair)| runs(pair).pop().map(|run| (segment, run)))
                .collect()
        })
        .collect();
    let mut pairs = Vec::new();
    for (i, left) in indexed.iter().enumerate() {
        for (j, right) in indexed.iter().enumerate().skip(i + 1) {
            for (s, a) in left {
                for (t, b) in right.iter().filter(|(_, b)| b.horizontal == a.horizontal) {
                    let apart = (a.at - b.at).abs();
                    let shared = a.to.min(b.to) - a.from.max(b.from);
                    if crowds(apart, shared) {
                        pairs.push(([(i, *s), (j, *t)], b.at - a.at));
                    }
                }
            }
        }
    }
    pairs
}

/// Move one segment sideways; an end segment slides its port along the
/// node side and must stay 8px clear of the corners.
fn shifted(points: &[Point], segment: usize, by: f64, ends: [Rect; 2]) -> Option<Vec<Point>> {
    let mut moved = points.to_vec();
    let horizontal = (points[segment].y - points[segment + 1].y).abs() < 0.5;
    for at in [segment, segment + 1] {
        if horizontal {
            moved[at].y += by;
        } else {
            moved[at].x += by;
        }
    }
    let last = points.len() - 1;
    for (at, node) in [(0, ends[0]), (last, ends[1])] {
        if at != segment && at != segment + 1 {
            continue;
        }
        let ok = if horizontal {
            moved[at].y >= node.y + 8.0 && moved[at].y <= node.y + node.height - 8.0
        } else {
            moved[at].x >= node.x + 8.0 && moved[at].x <= node.x + node.width - 8.0
        };
        if !ok {
            return None;
        }
    }
    // The drawn route loses 8px to each rounded corner; what remains must
    // meet the rhythm floors (8px at an end, 16px between corners).
    let segments = moved.len() - 1;
    let long_enough = moved.windows(2).enumerate().all(|(index, pair)| {
        let length = (pair[0].x - pair[1].x).abs() + (pair[0].y - pair[1].y).abs();
        let corners = usize::from(index > 0) + usize::from(index + 1 < segments);
        let floor = if corners == 2 { 32.0 } else { 16.0 };
        corners == 0 || length >= floor
    });
    long_enough.then_some(moved)
}

/// Pull apart runs of different arrows that would read as one arrow.
fn spread(routes: &mut [router::Route], edges: &[Edge], nodes: &[Node]) {
    for _ in 0..64 {
        let points: Vec<Vec<Point>> = routes.iter().map(|route| route.points.clone()).collect();
        let view: Vec<&[Point]> = points.iter().map(Vec::as_slice).collect();
        let current = crowding(&view);
        if current == 0 {
            return;
        }
        let mut improved = false;
        'pairs: for (pair, offset) in crowded_pairs(&view) {
            // Move the second run away from the first, or the first away from
            // the second, far enough to leave label room; then try past it.
            for ((route, segment), away) in [(pair[1], offset), (pair[0], -offset)] {
                let edge = &edges[route];
                let ends = [nodes[edge.from].bounds, nodes[edge.to].bounds];
                let side = if away < 0.0 { -1.0 } else { 1.0 };
                let gap = PORT_PITCH - away.abs();
                for by in [
                    side * gap,
                    -side * (PORT_PITCH + away.abs()),
                    14.0,
                    -14.0,
                    28.0,
                    -28.0,
                    42.0,
                    -42.0,
                ] {
                    let Some(candidate) = shifted(&points[route], segment, by, ends) else {
                        continue;
                    };
                    let mut trial = view.clone();
                    trial[route] = &candidate;
                    // Arrow ends stay a full arrowhead apart.
                    let ends_clear =
                        [candidate[0], candidate[candidate.len() - 1]]
                            .iter()
                            .all(|end| {
                                view.iter()
                                    .enumerate()
                                    .filter(|(other, _)| *other != route)
                                    .all(|(_, points)| {
                                        [points[0], points[points.len() - 1]].iter().all(|other| {
                                            (end.x - other.x).hypot(end.y - other.y) >= 12.0
                                        })
                                    })
                            });
                    if ends_clear
                        && crowding(&trial) < current
                        && through_nodes(&trial[route..=route], &edges[route..=route], nodes) == 0
                    {
                        routes[route].points = candidate;
                        improved = true;
                        break 'pairs;
                    }
                }
            }
        }
        if !improved {
            return;
        }
    }
}

/// How close a route may run past a state it does not connect, and an arrow
/// label may come to any state.
const CLEARANCE: f64 = 8.0;

/// Routes that pass through, or hug, a state other than their own ends.
fn through_nodes(routes: &[&[Point]], edges: &[Edge], nodes: &[Node]) -> usize {
    routes
        .iter()
        .zip(edges)
        .map(|(points, edge)| {
            nodes
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != edge.from && *index != edge.to)
                .filter(|(_, node)| {
                    runs(points).iter().any(|run| {
                        let b = node.bounds;
                        let (low, high, from, to) = if run.horizontal {
                            (b.y, b.y + b.height, b.x, b.x + b.width)
                        } else {
                            (b.x, b.x + b.width, b.y, b.y + b.height)
                        };
                        run.at > low - CLEARANCE
                            && run.at < high + CLEARANCE
                            && run.from < to + CLEARANCE
                            && run.to > from - CLEARANCE
                    })
                })
                .count()
        })
        .sum()
}

/// Widen `extent` to cover `rect`.
fn grow(extent: &mut Rect, rect: Rect) {
    let right = (extent.x + extent.width).max(rect.x + rect.width);
    let bottom = (extent.y + extent.height).max(rect.y + rect.height);
    extent.x = extent.x.min(rect.x);
    extent.y = extent.y.min(rect.y);
    extent.width = right - extent.x;
    extent.height = bottom - extent.y;
}

fn leaves_below(region: &Region, child: Child) -> Vec<usize> {
    match child {
        Child::State(leaf) => vec![leaf],
        Child::Parent(parent) => region.parents[parent]
            .children
            .iter()
            .flat_map(|kid| leaves_below(region, *kid))
            .collect(),
    }
}
