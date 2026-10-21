// Entity-relationship layout ported from the frozen renderers/erd/render-erd.mjs:
// banded grid → measured tables → domain bands → routes with bundled trunks →
// relationship labels → canvas and legend.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::composition::LabelRect;
use crate::geometry::{
    fitted_node_font_size, label_point, labels as shared_labels, rects_overlap,
    segment_intersects_rect, text_units,
};
use crate::input::{CompileOptions, DiagramInput, InputContract, Quality};
use crate::layout::router::{self, CandidateContext, Edge, Endpoint, RouterOptions, Side, Sides};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DetailRow, DiagramFacts, Marker, Node, Point, Rect,
    Relation, Sigil, Text,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) mod grid;
pub(crate) mod legend;
pub(crate) mod quality;
pub(crate) mod trunks;

use grid::{Cell, Grid};
use quality::{diagnostic, Gate, Routed};

const MARGIN: f64 = 24.0;
const LEGEND_H: f64 = 34.0;
const PAD_X: f64 = 12.0;
const KEY_WIDTH: f64 = 30.0;
pub(crate) const KEY_FONT: f64 = 9.0;
const HEADER_FONT: f64 = 13.0;
const HEADER_FONT_MINIMUM: f64 = 10.0;
const ROW_FONT: f64 = 11.0;
const ROW_FONT_MINIMUM: f64 = 9.0;
const TYPE_FONT: f64 = 10.5;
const MIN_RELATIONSHIP_LENGTH: f64 = 24.0;
pub(crate) const KEY_BADGE_W: f64 = 22.0;
const KEY_BADGE_GAP: f64 = 3.0;
const KEY_BADGE_INSET: f64 = 2.0;
const KEY_ROLE_GAP: f64 = 5.0;
const RELATIONSHIP_LABEL_FONT: f64 = 9.0;
const LABEL_GLYPH_CLEARANCE: f64 = 24.0 + 4.0;
const DOMAIN_PAD_X: f64 = 12.0;
const DOMAIN_PAD_TOP: f64 = 18.0;
const DOMAIN_PAD_BOTTOM: f64 = 8.0;
const DOMAIN_HUES: [&str; 6] = [
    "database",
    "frontend",
    "backend",
    "cloud",
    "messagebus",
    "security",
];
// A cardinality glyph is 14 units tall, so two ports on one side must sit at
// least this far apart.
const ERD_PORT_SPACING: f64 = 24.0;
const MARKER_GLYPH_WIDTH: f64 = 24.0;
const MARKER_HEIGHT: f64 = 14.0;
const GLYPH_HALF_HEIGHT: f64 = 10.0;
const LEGEND_BASELINE_GAP: f64 = 16.0;
const LEGEND_FONT_SIZE: f64 = 10.0;
pub(crate) const LEGEND_KEY_SWATCH: f64 = 22.0;
pub(crate) const LEGEND_END_SWATCH: f64 = 30.0;
const LEGEND_GROUP_LEAD: f64 = 10.0;
pub(crate) const KEY_KINDS: [&str; 3] = ["pk", "fk", "uk"];

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
fn string(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap_or("").to_owned()
}
pub(crate) fn text(
    path: impl Into<String>,
    content: impl Into<String>,
    x: f64,
    y: f64,
    font: f64,
    anchor: &str,
) -> Text {
    Text {
        source_path: path.into(),
        content: content.into(),
        position: Point { x, y },
        font_size: font,
        anchor: anchor.into(),
    }
}
pub(crate) fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// A column's key roles, each drawn once in authored order, with the index of
/// the authored list item (none for a single role).
pub(crate) fn key_roles(attribute: &Value) -> Vec<(Option<usize>, String)> {
    let roles: Vec<(Option<usize>, &str)> = match &attribute["key"] {
        Value::String(role) => vec![(None, role.as_str())],
        Value::Array(items) => items
            .iter()
            .enumerate()
            .filter_map(|(index, role)| Some((Some(index), role.as_str()?)))
            .collect(),
        _ => vec![],
    };
    let mut seen = HashSet::new();
    roles
        .into_iter()
        .filter(|(_, role)| seen.insert(*role))
        .map(|(index, role)| (index, role.to_owned()))
        .collect()
}

fn key_glyph_width(count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        count as f64 * KEY_BADGE_W + (count - 1) as f64 * KEY_BADGE_GAP - KEY_BADGE_INSET
    }
}

// One key column per table, floored at the shared width so a table that states
// a single role keeps the established alignment.
fn key_column_width(entity: &Value) -> f64 {
    let widest = list(&entity["attributes"])
        .iter()
        .map(|attribute| key_glyph_width(key_roles(attribute).len()))
        .fold(0.0, f64::max);
    KEY_WIDTH.max(if widest > 0.0 {
        widest + KEY_ROLE_GAP
    } else {
        0.0
    })
}

fn relationship_label_width(label: &str) -> f64 {
    (text_units(label) * RELATIONSHIP_LABEL_FONT * 0.62 + 10.0).max(30.0)
}

fn cardinality(raw: &Value, from: bool) -> &'static str {
    match raw[if from {
        "fromCardinality"
    } else {
        "toCardinality"
    }]
    .as_str()
    {
        Some("one") => "one",
        Some("many") => "many",
        _ if from => "many",
        _ => "one",
    }
}
fn optional(raw: &Value, from: bool) -> bool {
    raw[if from { "fromOptional" } else { "toOptional" }] == true
}
fn marker(raw: &Value, from: bool) -> Marker {
    match (cardinality(raw, from), optional(raw, from)) {
        ("one", false) => Marker::ErOne,
        ("one", true) => Marker::ErOneOptional,
        ("many", false) => Marker::ErMany,
        _ => Marker::ErManyOptional,
    }
}
/// `identifying: false` is a fact about the key and the dash draws it.
pub(crate) fn marker_style(raw: &Value) -> String {
    if raw["identifying"] == false {
        "dashed".into()
    } else {
        raw["variant"].as_str().unwrap_or("default").into()
    }
}
/// The kinds one relationship shows at its two ends.
pub(crate) fn relationship_ends(raw: &Value) -> Vec<&'static str> {
    let mut ends = vec![cardinality(raw, true)];
    if cardinality(raw, false) != ends[0] {
        ends.push(cardinality(raw, false));
    }
    if optional(raw, true) || optional(raw, false) {
        ends.push("optional");
    }
    ends
}

struct Entity<'a> {
    raw: &'a Value,
    index: usize,
    id: String,
    bounds: Rect,
    cell: Cell,
}

impl Entity<'_> {
    // A band follows only a table its grid cells place.
    fn placed_by_grid(&self) -> bool {
        self.cell.row.is_some_and(|row| row.fract() == 0.0)
            && self.cell.col.is_some_and(|col| col.fract() == 0.0)
            && !self.raw["pos"].as_array().is_some_and(|pos| pos.len() == 2)
    }
    fn tag(&self) -> Option<&str> {
        self.raw["tag"].as_str().filter(|tag| !tag.is_empty())
    }
    fn cell_key(&self) -> (i64, i64) {
        (
            self.cell.row.unwrap_or(f64::NAN) as i64,
            self.cell.col.unwrap_or(f64::NAN) as i64,
        )
    }
}

struct Domain {
    tag: String,
    members: Vec<usize>,
}

fn band_box(entities: &[Entity<'_>], members: &[usize]) -> Rect {
    let bounds = || members.iter().map(|index| entities[*index].bounds);
    let min_x = bounds().map(|r| r.x).fold(f64::INFINITY, f64::min);
    let min_y = bounds().map(|r| r.y).fold(f64::INFINITY, f64::min);
    let max_x = bounds()
        .map(|r| r.x + r.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = bounds()
        .map(|r| r.y + r.height)
        .fold(f64::NEG_INFINITY, f64::max);
    Rect {
        x: min_x - DOMAIN_PAD_X,
        y: min_y - DOMAIN_PAD_TOP,
        width: max_x - min_x + DOMAIN_PAD_X * 2.0,
        height: max_y - min_y + DOMAIN_PAD_TOP + DOMAIN_PAD_BOTTOM,
    }
}

fn caption(band: Rect) -> Point {
    Point {
        x: band.x + 10.0,
        y: band.y + 13.0,
    }
}

fn glyph_box(point: Point, side: Side) -> Rect {
    let (reach, half) = (MARKER_GLYPH_WIDTH, GLYPH_HALF_HEIGHT);
    match side {
        Side::Left => Rect {
            x: point.x - reach,
            y: point.y - half,
            width: reach,
            height: half * 2.0,
        },
        Side::Right => Rect {
            x: point.x,
            y: point.y - half,
            width: reach,
            height: half * 2.0,
        },
        Side::Top => Rect {
            x: point.x - half,
            y: point.y - reach,
            width: half * 2.0,
            height: reach,
        },
        Side::Bottom => Rect {
            x: point.x - half,
            y: point.y,
            width: half * 2.0,
            height: reach,
        },
    }
}

/// The horizontal-first endpoint inference every non-architecture type keeps.
pub(crate) fn legacy_side(from: Rect, to: Rect, endpoint: Endpoint) -> Side {
    let (a, b) = (
        (from.x + from.width / 2.0, from.y + from.height / 2.0),
        (to.x + to.width / 2.0, to.y + to.height / 2.0),
    );
    let sides = if b.0 < a.0 {
        Sides {
            from: Side::Left,
            to: Side::Right,
        }
    } else if b.0 > a.0 {
        Sides {
            from: Side::Right,
            to: Side::Left,
        }
    } else if b.1 > a.1 {
        Sides {
            from: Side::Bottom,
            to: Side::Top,
        }
    } else {
        Sides {
            from: Side::Top,
            to: Side::Bottom,
        }
    };
    match endpoint {
        Endpoint::Source => sides.from,
        Endpoint::Target => sides.to,
    }
}

/// The side an endpoint actually uses: an authored side, else the route's.
pub(crate) fn endpoint_side(raw: &Value, route: &router::Route, source: bool) -> Side {
    Side::parse(&raw[if source { "fromSide" } else { "toSide" }]).unwrap_or(if source {
        route.sides.from
    } else {
        route.sides.to
    })
}

pub(crate) fn legend_cards(
    meta: &Value,
    entries: &[legend::Entry],
    placement: &legend::Placement,
    title: String,
    x: f64,
) -> Vec<Card> {
    let card =
        |path: String, id: String, kind: &str, facts: Value, bounds: Rect, label: Text| Card {
            authored: Authored {
                id,
                source_path: path,
                kind: kind.into(),
                facts,
                geometry_provenance: vec!["resolved-legend-footprint".into()],
            },
            bounds,
            text: vec![label],
        };
    let source = if meta.get("legend").is_some() {
        "/meta/legend"
    } else {
        "/meta"
    };
    let mut cards = vec![card(
        source.into(),
        "legend-title".into(),
        "legend-title",
        json!({ "label": title }),
        Rect {
            x,
            y: placement.title_y - 10.0,
            width: 48.0,
            height: 14.0,
        },
        text(source, title.clone(), x, placement.title_y, 12.0, "start"),
    )];
    for positioned in &placement.entries {
        let entry = &entries[positioned.index];
        let path = entry.source.clone().unwrap_or_else(|| source.into());
        cards.push(card(
            path.clone(),
            format!("legend-{}", entry.kind),
            &entry.kind,
            json!({"kind":entry.kind,"label":entry.label,"lead":entry.lead,"swatchWidth":entry.swatch_width}),
            Rect {
                x: positioned.x,
                y: positioned.baseline - 10.0,
                width: positioned.width,
                height: 14.0,
            },
            text(
                path,
                entry.label.clone(),
                positioned.x + entry.swatch_width + legend::SWATCH_GAP,
                positioned.baseline,
                placement.rendered_font(),
                "start",
            ),
        ));
    }
    cards
}

pub(crate) fn legend_failure(kind: &str, unfit: legend::Unfit) -> Diagnostic {
    let (code, path, evidence) = match unfit {
        legend::Unfit::TooWide {
            kind: entry,
            width,
            available,
        } => (
            "legend/label-too-wide",
            format!("/meta/legend/entries/{entry}/label"),
            json!({"kind":entry,"measuredWidthPx":width,"availableWidthPx":available}),
        ),
        legend::Unfit::VerticalOverflow {
            rows,
            top,
            available,
        } => (
            "legend/vertical-overflow",
            "/meta/legend".into(),
            json!({"rowCount":rows,"requiredTopY":top,"availableTopY":available}),
        ),
        legend::Unfit::Collision { kind: entry, rect } => (
            "legend/content-overlap",
            "/meta/legend".into(),
            json!({"legendKind":entry,"legendRect":rect}),
        ),
    };
    diagnostic(
        kind,
        code,
        &path,
        "The legend does not fit its reserved band",
        evidence,
    )
}

pub(crate) fn cards(document: &Value) -> Vec<Card> {
    list(&document["cards"])
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let path = format!("/cards/{index}");
            let mut texts = vec![text(
                format!("{path}/title"),
                string(raw, "title"),
                0.0,
                0.0,
                14.0,
                "start",
            )];
            texts.extend(list(&raw["items"]).iter().enumerate().map(|(item, value)| {
                text(
                    format!("{path}/items/{item}"),
                    value.as_str().unwrap_or(""),
                    0.0,
                    0.0,
                    12.0,
                    "start",
                )
            }));
            Card {
                authored: Authored {
                    id: format!("card-{index}"),
                    source_path: path,
                    kind: "card".into(),
                    facts: raw.clone(),
                    geometry_provenance: vec!["reader-card-flow".into()],
                },
                bounds: Rect::default(),
                text: texts,
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
    let grid = Grid::erd(document);
    let metrics = grid.unwrap_or(Grid::ERD_DEFAULT);
    let raw_entities = list(&document["entities"]);
    let relationships = list(&document["relationships"]);
    let mut entities: Vec<Entity<'_>> = raw_entities
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let width = raw["width"].as_f64().unwrap_or(metrics.entity_w);
            let rows = list(&raw["attributes"]).len().max(1) as f64;
            let height = metrics.header_h + rows * metrics.row_h + 6.0;
            Entity {
                raw,
                index,
                id: string(raw, "id"),
                bounds: Rect {
                    x: f64::NAN,
                    y: f64::NAN,
                    width,
                    height,
                },
                cell: Cell::of(raw, width, height),
            }
        })
        .collect();
    let by_id: HashMap<&str, usize> = raw_entities
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| Some((raw["id"].as_str()?, index)))
        .collect();

    // A label between two neighbouring columns is drawn in the gap between them.
    // When the grid owns every coordinate, each such gap widens just enough for
    // its widest label to clear both cardinality glyphs.
    let column_gaps = grid.filter(|_| !meta["viewBox"].is_array()).and_then(|_| {
        let absolute = raw_entities.iter().any(|entity| entity["pos"].is_array())
            || relationships.iter().any(|relation| {
                relation["via"]
                    .as_array()
                    .is_some_and(|via| !via.is_empty())
                    || relation.get("labelAt").is_some()
                    || relation["route"]
                        .as_str()
                        .is_some_and(|route| route != "auto")
                    || relation.get("channelX").is_some()
                    || relation.get("channelY").is_some()
            });
        if absolute {
            return None;
        }
        let mut gaps: BTreeMap<i64, f64> = BTreeMap::new();
        for relation in relationships {
            let label = string(relation, "label");
            if label.is_empty() {
                continue;
            }
            let column = |field: &str| {
                by_id
                    .get(relation[field].as_str().unwrap_or(""))
                    .and_then(|index| entities[*index].cell.col)
                    .filter(|col| col.fract() == 0.0)
            };
            let (Some(from), Some(to)) = (column("from"), column("to")) else {
                continue;
            };
            if (from - to).abs() != 1.0 {
                continue;
            }
            let demand = (relationship_label_width(&label) + LABEL_GLYPH_CLEARANCE * 2.0).ceil();
            let gap = gaps.entry(from.min(to) as i64).or_insert(0.0);
            *gap = gap.max(demand);
        }
        (!gaps.is_empty()).then_some(gaps)
    });
    let cells: Vec<_> = entities.iter().map(|entity| entity.cell).collect();
    let bands = grid.map(|grid| grid::banded_layout(&cells, &grid, column_gaps.as_ref()));
    for entity in &mut entities {
        let position = grid::resolve_position(entity.raw, &entity.cell, bands.as_ref());
        entity.bounds.x = position.x;
        entity.bounds.y = position.y;
    }

    // ---- Domain bands ----
    // A tag names a domain, and a domain has to read as one block: only members
    // that fill a solid rectangle of grid cells earn a band.
    let mut tags: Vec<(String, Vec<usize>)> = vec![];
    for (index, entity) in entities.iter().enumerate() {
        if let Some(tag) = entity.tag() {
            match tags.iter_mut().find(|(candidate, _)| candidate == tag) {
                Some((_, members)) => members.push(index),
                None => tags.push((tag.into(), vec![index])),
            }
        }
    }
    let distinct_cells = |members: &[usize]| {
        members
            .iter()
            .map(|m| entities[*m].cell_key())
            .collect::<HashSet<_>>()
    };
    let contiguous = |members: &[usize]| {
        let cells = distinct_cells(members);
        if cells.len() != members.len() {
            return false;
        }
        let (rows, cols): (Vec<i64>, Vec<i64>) = cells.iter().copied().unzip();
        let span = |values: &[i64]| values.iter().max().unwrap() - values.iter().min().unwrap() + 1;
        cells.len() as i64 == span(&rows) * span(&cols)
    };
    let mut domains = vec![];
    let mut unbanded = vec![];
    for (tag, members) in &tags {
        let grid_placed = members.iter().all(|m| entities[*m].placed_by_grid());
        if grid_placed && contiguous(members) {
            domains.push(Domain {
                tag: tag.clone(),
                members: members.clone(),
            });
        } else {
            unbanded.push((tag.clone(), members.clone(), grid_placed));
        }
    }
    let hue_of = |tag: &str| {
        tags.iter()
            .position(|(candidate, _)| candidate == tag)
            .map_or("database", |index| DOMAIN_HUES[index % DOMAIN_HUES.len()])
    };

    // ---- Legend and canvas ----
    let mut present: HashSet<String> = HashSet::new();
    for entity in &entities {
        for attribute in list(&entity.raw["attributes"]) {
            for (_, role) in key_roles(attribute) {
                present.insert(role);
            }
        }
    }
    for relation in relationships {
        for end in relationship_ends(relation) {
            present.insert(end.into());
        }
    }
    let catalog: Vec<(&str, String)> = ["pk", "fk", "uk", "one", "many", "optional"]
        .into_iter()
        .map(|kind| (kind, locale.text(&format!("legend.erd.{kind}"), &[])))
        .collect();
    let mut entries = legend::resolve(&meta["legend"], &catalog, &present);
    for index in 0..entries.len() {
        let key = KEY_KINDS.contains(&entries[index].kind.as_str());
        let lead = if !key && index > 0 && KEY_KINDS.contains(&entries[index - 1].kind.as_str()) {
            LEGEND_GROUP_LEAD
        } else {
            0.0
        };
        let entry = &mut entries[index];
        entry.lead = lead;
        entry.swatch_width = if key {
            LEGEND_KEY_SWATCH
        } else {
            LEGEND_END_SWATCH
        } + lead;
        entry.trailing_width = (text_units(&entry.label) * 2.0 * 0.62).ceil();
    }
    let content_bottom = entities
        .iter()
        .map(|entity| entity.bounds.y + entity.bounds.height)
        .fold(0.0, f64::max);
    let band = |width: f64, height: f64| legend::Band {
        x: MARGIN,
        baseline: height - LEGEND_BASELINE_GAP,
        width: width - MARGIN * 2.0,
        font: LEGEND_FONT_SIZE,
        min_title_y: content_bottom + 8.0,
    };
    let authored_canvas = meta["viewBox"]
        .as_array()
        .and_then(|size| Some((size.first()?.as_f64()?, size.get(1)?.as_f64()?)));
    let (width, height) = authored_canvas.unwrap_or_else(|| {
        let max_x = entities
            .iter()
            .map(|entity| entity.bounds.x + entity.bounds.width)
            .fold(0.0, f64::max);
        let mut width = (max_x + MARGIN).ceil();
        let mut footprint = legend::sizing_footprint(&entries, (width - MARGIN * 2.0).max(1.0));
        if footprint.min_width > width - MARGIN * 2.0 {
            width = (footprint.min_width + MARGIN * 2.0).ceil();
            footprint = legend::sizing_footprint(&entries, width - MARGIN * 2.0);
        }
        // The legend is generated content, so a strip a few units short grows
        // the canvas by whole bands instead of dropping the key.
        let mut height = (content_bottom + MARGIN + LEGEND_H + footprint.extra_height).ceil();
        for _ in 0..4 {
            if legend::measure(&entries, &band(width, height), &[]).is_ok() {
                break;
            }
            height += LEGEND_H;
        }
        (width, height)
    });
    let canvas = Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };

    // ---- Routing ----
    let nodes: Vec<Node> = entities
        .iter()
        .map(|entity| entity_node(entity, &metrics, hue_of))
        .collect();
    let edges: Vec<Edge> = relationships
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| {
            Some(Edge {
                raw: raw.clone(),
                index,
                from: *by_id.get(raw["from"].as_str()?)?,
                to: *by_id.get(raw["to"].as_str()?)?,
            })
        })
        .collect();
    // Horizontal-first, like every other non-architecture type: a relationship
    // leaves sideways when the tables sit side by side.
    let side_for = |edge: &Edge, endpoint: Endpoint| {
        Some(legacy_side(
            entities[edge.from].bounds,
            entities[edge.to].bounds,
            endpoint,
        ))
    };
    let base = RouterOptions {
        side_for: Some(&side_for),
        max_port_spacing: ERD_PORT_SPACING,
        // The drawn route removes each bundled trunk stretch, so the planned
        // route's rhythm floors do not describe what is drawn; the artifact gate
        // measures the drawn result.
        composition_floors: false,
        crossing_free_grid_first: true,
        component_gap_px: 10.0,
        // Corners are drawn with a radius; an end segment must outlast it plus
        // the micro-segment floor.
        minimum_terminal_segment_px: 14.0,
        distinct_automatic_ports: false,
        label_reservations: false,
        ..RouterOptions::default()
    };
    let (inferred, ports) = router::inferred_ports(&nodes, &edges, base);
    let tables: Vec<_> = entities
        .iter()
        .map(|entity| trunks::Table {
            row: entity.cell.row,
            col: entity.cell.col,
            bounds: entity.bounds,
            id: &entity.id,
        })
        .collect();
    let lanes = trunks::lane_offsets(&edges, &tables, &inferred);
    let styles: Vec<String> = edges.iter().map(|edge| marker_style(&edge.raw)).collect();
    let groups = trunks::trunk_groups(&edges, &tables, &inferred, &ports, &styles);
    let assignment: HashMap<usize, usize> = groups
        .iter()
        .enumerate()
        .flat_map(|(group, trunk)| trunk.members.iter().map(move |member| (*member, group)))
        .collect();
    let edge_position: HashMap<usize, usize> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| (edge.index, position))
        .collect();
    let preferred = |context: &CandidateContext<'_>| {
        let position = edge_position[&context.edge.index];
        let mut candidates = vec![];
        if let Some(candidate) = assignment
            .get(&position)
            .and_then(|group| trunks::trunk_candidate(&groups[*group], context))
        {
            candidates.push(candidate);
        }
        if let Some(offset) = lanes.get(&position) {
            candidates.push(trunks::lane_candidate(*offset, context));
        }
        candidates
    };
    let routes = router::route_with(
        &nodes,
        &edges,
        &[],
        InputContract::PhoenixV1,
        false,
        RouterOptions {
            preferred_candidates: Some(&preferred),
            ..base
        },
    );
    // The drawn route skips each trunk stretch; one shared trunk path carries
    // it. A member moved to another side after grouping is drawn whole.
    let runs: Vec<Vec<trunks::Run>> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| {
            let Some(group) = assignment.get(&position).map(|group| &groups[*group]) else {
                return vec![];
            };
            if endpoint_side(&edge.raw, &routes[position], !group.inbound) != group.side {
                return vec![];
            }
            trunks::trunk_runs(&routes[position].points, group.axis, group.coordinate)
        })
        .collect();

    // ---- Relationship labels ----
    // A bundled label defaults to the source-side stub so it never annotates
    // the shared trunk.
    let default_label_point = |position: usize| {
        let edge = &edges[position];
        if assignment.contains_key(&position) && edge.raw.get("labelSegment").is_none() {
            let mut raw = edge.raw.clone();
            raw["labelSegment"] = json!(0);
            label_point(&raw, &routes[position].points)
        } else {
            label_point(&edge.raw, &routes[position].points)
        }
    };
    let mut label_points: Vec<Option<Point>> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| {
            (!string(&edge.raw, "label").is_empty()).then(|| default_label_point(position))
        })
        .collect();
    let label_rect = |position: usize, at: Point| {
        let width = relationship_label_width(&string(&edges[position].raw, "label"));
        Rect {
            x: at.x - width / 2.0,
            y: at.y - 10.0,
            width,
            height: 14.0,
        }
    };
    if label_points.iter().any(Option::is_some) {
        let labels: Vec<_> = label_points
            .iter()
            .enumerate()
            .filter_map(|(position, at)| {
                let at = (*at)?;
                let raw = &edges[position].raw;
                Some(shared_labels::Label {
                    relation_index: edges[position].index as isize,
                    bounds: label_rect(position, at),
                    position: at,
                    pinned: ["labelAt", "labelDx", "labelDy", "labelSegment"]
                        .iter()
                        .any(|field| raw.get(*field).is_some()),
                })
            })
            .collect();
        // Each end reserves its cardinality glyph, and a bundled member's shared
        // trunk counts as another route.
        let mut route_list: Vec<shared_labels::Route> = vec![];
        let mut obstacles: Vec<Rect> = entities.iter().map(|entity| entity.bounds).collect();
        for (position, edge) in edges.iter().enumerate() {
            let points = &routes[position].points;
            obstacles.push(glyph_box(
                points[0],
                endpoint_side(&edge.raw, &routes[position], true),
            ));
            obstacles.push(glyph_box(
                *points.last().unwrap(),
                endpoint_side(&edge.raw, &routes[position], false),
            ));
            let relation_index = edge.index as isize;
            if runs[position].is_empty() {
                route_list.push(shared_labels::Route {
                    relation_index,
                    points: points.clone(),
                });
                continue;
            }
            let in_run = |index: usize| {
                runs[position]
                    .iter()
                    .any(|run| index >= run.start_index && index < run.end_index)
            };
            let mut piece = vec![points[0]];
            for index in 0..points.len() - 1 {
                if in_run(index) {
                    if piece.len() > 1 {
                        route_list.push(shared_labels::Route {
                            relation_index,
                            points: piece.clone(),
                        });
                    }
                    route_list.push(shared_labels::Route {
                        relation_index: -1 - route_list.len() as isize,
                        points: vec![points[index], points[index + 1]],
                    });
                    piece = vec![points[index + 1]];
                } else {
                    piece.push(points[index + 1]);
                }
            }
            if piece.len() > 1 {
                route_list.push(shared_labels::Route {
                    relation_index,
                    points: piece,
                });
            }
        }
        let titles: Vec<Rect> = domains
            .iter()
            .map(|domain| {
                let at = caption(band_box(&entities, &domain.members));
                Rect {
                    x: at.x - 2.0,
                    y: at.y - 10.0,
                    width: text_units(&domain.tag) * 9.0 * 0.65 + 4.0,
                    height: 13.0,
                }
            })
            .collect();
        let footprint = legend::sizing_footprint(&entries, (width - MARGIN * 2.0).max(1.0));
        let mut options = shared_labels::Options::new(canvas);
        options.placement_bottom = height - LEGEND_H - footprint.extra_height;
        options.keep_fallback_near_route = true;
        let placed = shared_labels::place_automatic_labels(
            &labels,
            &route_list,
            &obstacles,
            &titles,
            &options,
        );
        for (label, resolved) in labels.iter().zip(placed) {
            let position = edge_position[&(label.relation_index as usize)];
            label_points[position] = Some(resolved.position);
        }
    }

    // ---- Validation ----
    let mut errors: Diagnostics = vec![];
    validate_entities(&entities, &mut errors);
    for (tag, members, grid_placed) in &unbanded {
        // A shared cell is reported by the input gate with the pair that has to move.
        if *grid_placed && distinct_cells(members).len() != members.len() {
            continue;
        }
        let off_grid: Vec<_> = members
            .iter()
            .filter(|m| !entities[**m].placed_by_grid())
            .map(|m| entities[*m].id.clone())
            .collect();
        errors.push(diagnostic(
            "erd",
            "erd/domain-not-drawn",
            &format!("/entities/{}/tag", entities[members[0]].index),
            format!("Domain {tag} is named but its tables do not fill one solid block of grid cells"),
            json!({"domain":tag,"entities":members.iter().map(|m| &entities[*m].id).collect::<Vec<_>>(),"reason":if *grid_placed {"scattered"} else {"no-grid"},"offGrid":off_grid}),
        ));
    }
    for domain in &domains {
        let band = band_box(&entities, &domain.members);
        let at = caption(band);
        let path = format!("/entities/{}/tag", entities[domain.members[0]].index);
        let member_ids: Vec<_> = domain.members.iter().map(|m| &entities[*m].id).collect();
        if !(at.x >= 4.0 && at.y >= 4.0 && at.y <= height - 4.0 && at.x <= width - 4.0) {
            errors.push(diagnostic(
                "erd",
                "erd/domain-not-drawn",
                &path,
                format!("Domain {} caption falls outside the canvas", domain.tag),
                json!({"domain":domain.tag,"entities":member_ids,"reason":"off-canvas","caption":at}),
            ));
        }
        // A band claims the box it covers.
        if let Some((_, intruder)) = entities.iter().enumerate().find(|(index, entity)| {
            !domain.members.contains(index)
                && entity.bounds.x < band.x + band.width
                && entity.bounds.x + entity.bounds.width > band.x
                && entity.bounds.y < band.y + band.height
                && entity.bounds.y + entity.bounds.height > band.y
        }) {
            errors.push(diagnostic(
                "erd",
                "erd/domain-not-drawn",
                &path,
                format!("Domain {} band would cover the unrelated table {}", domain.tag, intruder.id),
                json!({"domain":domain.tag,"entities":member_ids,"reason":"band-covers-entity","band":band,"coveredEntity":intruder.id}),
            ));
        }
    }
    for (position, edge) in edges.iter().enumerate() {
        let points = &routes[position].points;
        let (start, end) = (points[0], *points.last().unwrap());
        let distance = (end.x - start.x).hypot(end.y - start.y);
        if distance < MIN_RELATIONSHIP_LENGTH {
            errors.push(diagnostic(
                "erd",
                "layout/constraint",
                &format!("/relationships/{}", edge.index),
                "Relationship is too short; place the entities farther apart",
                json!({"distancePx":distance,"minimumPx":MIN_RELATIONSHIP_LENGTH}),
            ));
        }
    }
    // The port spread is a cap, not a floor: a short side with several ends
    // draws overlapping glyphs, which the route gate cannot see.
    for (index, entity) in entities.iter().enumerate() {
        let mut sides: Vec<(Side, usize)> = vec![];
        for (position, edge) in edges.iter().enumerate() {
            if !trunks::auto_routed(edge) {
                continue;
            }
            for (node, side) in [
                (edge.from, inferred[position].from),
                (edge.to, inferred[position].to),
            ] {
                if node != index {
                    continue;
                }
                match sides.iter_mut().find(|(candidate, _)| *candidate == side) {
                    Some((_, count)) => *count += 1,
                    None => sides.push((side, 1)),
                }
            }
        }
        for (side, count) in sides {
            if count < 2 {
                continue;
            }
            let extent = if side.horizontal() {
                entity.bounds.height
            } else {
                entity.bounds.width
            };
            let spacing = ERD_PORT_SPACING.min((extent - 32.0).max(0.0) / (count - 1) as f64);
            if spacing >= MARKER_HEIGHT {
                continue;
            }
            errors.push(diagnostic(
                "erd",
                "layout/marker-capacity",
                &format!("/entities/{}", entity.index),
                format!(
                    "Entity {} shares its {} side with {count} relationship ends whose glyphs would overlap",
                    entity.id,
                    side.name()
                ),
                json!({"entity":entity.id,"side":side.name(),"relationshipEnds":count,"spacingPx":js_round(spacing),"markerHeightPx":MARKER_HEIGHT,"sideExtentPx":extent}),
            ));
        }
    }
    let node_boxes: Vec<(String, Rect)> = entities
        .iter()
        .map(|entity| (entity.id.clone(), entity.bounds))
        .collect();
    let gate = Gate {
        kind: "erd",
        obstacle_kind: "entity",
        nodes: &node_boxes,
        showcase: input.quality == Quality::Showcase,
    };
    let sides: Vec<(&str, &str)> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| {
            (
                endpoint_side(&edge.raw, &routes[position], true).name(),
                endpoint_side(&edge.raw, &routes[position], false).name(),
            )
        })
        .collect();
    let routed: Vec<Routed<'_>> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| Routed {
            index: edge.index,
            raw: &edge.raw,
            points: &routes[position].points,
            from_side: sides[position].0,
            to_side: sides[position].1,
        })
        .collect();
    gate.routes(&routed, &mut errors);
    let label_rects: Vec<LabelRect> = label_points
        .iter()
        .enumerate()
        .filter_map(|(position, at)| {
            Some(LabelRect::new(
                edges[position].index as isize,
                edges[position].raw.clone(),
                label_rect(position, (*at)?),
            ))
        })
        .collect();
    gate.labels(&label_rects, &routed, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    // ---- Legend ----
    let mut obstacles: Vec<legend::Obstacle> = vec![];
    for (position, route) in routes.iter().enumerate() {
        for pair in route.points.windows(2) {
            obstacles.push(legend::Obstacle::Segment(pair[0], pair[1]));
        }
        if let Some(at) = label_points[position] {
            obstacles.push(legend::Obstacle::Rect(label_rect(position, at)));
        }
    }
    let legends = match legend::measure(&entries, &band(width, height), &obstacles) {
        Ok(Some(placement)) => legend_cards(
            meta,
            &entries,
            &placement,
            locale.text("legend.title", &[]),
            MARGIN,
        ),
        Ok(None) => vec![],
        Err(unfit) => return Err(vec![legend_failure("erd", unfit)]),
    };

    // ---- Scene ----
    let relations: Vec<Relation> = edges
        .iter()
        .enumerate()
        .map(|(position, edge)| {
            let raw = &edge.raw;
            let label = string(raw, "label");
            let at = label_points[position];
            let mut provenance = vec![
                format!("/relationships/{}/from", edge.index),
                format!("/relationships/{}/to", edge.index),
            ];
            for field in [
                "fromSide",
                "toSide",
                "via",
                "route",
                "labelAt",
                "labelDx",
                "labelDy",
                "labelSegment",
            ] {
                if raw.get(field).is_some() {
                    provenance.push(format!("/relationships/{}/{field}", edge.index));
                }
            }
            if trunks::auto_routed(edge) {
                provenance.push(if runs[position].is_empty() {
                    "obstacle-grid-and-readable-routing".into()
                } else {
                    "bundled-relationship-trunk".into()
                });
            }
            Relation {
                authored: Authored {
                    id: raw["id"]
                        .as_str()
                        .map_or_else(|| format!("relationship-{}", edge.index), str::to_owned),
                    source_path: format!("/relationships/{}", edge.index),
                    kind: "relationship".into(),
                    facts: relationships[edge.index].clone(),
                    geometry_provenance: provenance,
                },
                crossover_halo: false,
                independent_ports: false,
                junction: assignment
                    .get(&position)
                    .filter(|_| !runs[position].is_empty())
                    .map(|group| trunks::junction(groups[*group].axis, groups[*group].coordinate)),
                label_owner: None,
                from: entities[edge.from].id.clone(),
                to: entities[edge.to].id.clone(),
                variant: styles[position].clone(),
                label: label.clone(),
                note: None,
                note_bounds: None,
                note_backdrop: None,
                points: routes[position].points.clone(),
                label_plate: at.map(|at| label_rect(position, at)),
                text: at
                    .map(|at| {
                        vec![text(
                            format!("/relationships/{}/label", edge.index),
                            label,
                            at.x,
                            at.y,
                            RELATIONSHIP_LABEL_FONT,
                            "middle",
                        )]
                    })
                    .unwrap_or_default(),
                marker_start: Some(marker(raw, true)),
                marker_end: Some(marker(raw, false)),
            }
        })
        .collect();
    let mut containers: Vec<Container> = domains
        .iter()
        .map(|domain| {
            let band = band_box(&entities, &domain.members);
            let at = caption(band);
            let pill = (text_units(&domain.tag) * (9.0 * 0.6 + 0.8) + 14.0).ceil();
            let first = &entities[domain.members[0]];
            Container {
                authored: Authored {
                    id: format!("domain-{}", domain.tag),
                    source_path: format!("/entities/{}/tag", first.index),
                    kind: "erd-domain".into(),
                    facts: json!({"tag":domain.tag,"hue":hue_of(&domain.tag)}),
                    geometry_provenance: domain
                        .members
                        .iter()
                        .map(|m| format!("/entities/{}/tag", entities[*m].index))
                        .chain(["measured-domain-band".into()])
                        .collect(),
                },
                bounds: band,
                label_plate: Some(Rect {
                    x: at.x - 2.0,
                    y: band.y - 7.0,
                    width: pill,
                    height: 15.0,
                }),
                rule: None,
                members: domain
                    .members
                    .iter()
                    .map(|m| entities[*m].id.clone())
                    .collect(),
                text: vec![text(
                    format!("/entities/{}/tag", first.index),
                    domain.tag.clone(),
                    at.x + 5.0,
                    band.y + 3.6,
                    9.0,
                    "start",
                )],
            }
        })
        .collect();
    containers.extend(trunk_containers(
        &edges,
        &entities,
        &groups,
        &assignment,
        &runs,
        &styles,
    ));
    Ok(CompiledDiagram {
        diagram_type: input.diagram_type,
        schema_version: document["schema_version"]
            .as_f64()
            .expect("validated schema version") as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: meta.clone(),
        view_box: canvas,
        reader_fit: if authored_canvas.is_some() {
            "authored-height"
        } else {
            "intrinsic-height"
        }
        .into(),
        nodes,
        relations,
        containers,
        text: vec![],
        legends,
        cards: cards(document),
        facts: DiagramFacts::Erd {
            entities: raw_entities.to_vec(),
            relationships: relationships.to_vec(),
        },
        diagnostics: vec![],
    })
}

fn entity_node(entity: &Entity<'_>, metrics: &Grid, hue_of: impl Fn(&str) -> &'static str) -> Node {
    let raw = entity.raw;
    let path = format!("/entities/{}", entity.index);
    let bounds = entity.bounds;
    let label = string(raw, "label");
    let header = header_text(&label, bounds);
    let key_column = key_column_width(raw);
    let detail_rows = list(&raw["attributes"])
        .iter()
        .enumerate()
        .map(|(index, attribute)| {
            let row_path = format!("{path}/attributes/{index}");
            let top = bounds.y + metrics.header_h + index as f64 * metrics.row_h;
            let baseline = top + metrics.row_h - 5.0;
            let mut texts: Vec<Text> = key_roles(attribute)
                .into_iter()
                .enumerate()
                .map(|(slot, (source, role))| {
                    let x = bounds.x + PAD_X - KEY_BADGE_INSET
                        + slot as f64 * (KEY_BADGE_W + KEY_BADGE_GAP);
                    text(
                        source.map_or_else(
                            || format!("{row_path}/key"),
                            |source| format!("{row_path}/key/{source}"),
                        ),
                        role.to_uppercase(),
                        x + KEY_BADGE_W / 2.0,
                        baseline,
                        KEY_FONT,
                        "middle",
                    )
                })
                .collect();
            let name = string(attribute, "name");
            let kind = string(attribute, "type");
            let name_x = bounds.x + PAD_X + key_column;
            let type_space = if kind.is_empty() {
                0.0
            } else {
                96.0_f64.min(text_units(&kind) * TYPE_FONT * 0.6)
            };
            let name_font = fitted_node_font_size(
                &name,
                40.0_f64.max(bounds.x + bounds.width - PAD_X - type_space - name_x),
                ROW_FONT,
                ROW_FONT_MINIMUM,
            );
            texts.push(text(
                format!("{row_path}/name"),
                name,
                name_x,
                baseline,
                name_font,
                "start",
            ));
            if !kind.is_empty() {
                texts.push(text(
                    format!("{row_path}/type"),
                    kind,
                    bounds.x + bounds.width - PAD_X,
                    baseline,
                    TYPE_FONT,
                    "end",
                ));
            }
            DetailRow {
                source_path: row_path,
                bounds: Rect {
                    x: bounds.x,
                    y: top,
                    width: bounds.width,
                    height: metrics.row_h,
                },
                text: texts,
                role: "attribute".into(),
            }
        })
        .collect();
    let mut provenance = if raw["pos"].is_array() {
        vec![format!("{path}/pos")]
    } else {
        vec![
            format!("{path}/row"),
            format!("{path}/col"),
            "banded-grid".into(),
        ]
    };
    provenance.push(if raw.get("width").is_some() {
        format!("{path}/width")
    } else {
        "grid-entity-width".into()
    });
    // Each domain owns one hue that its band and its tables share.
    provenance.push(format!(
        "domain-hue:{}",
        entity.tag().map_or("database", &hue_of)
    ));
    Node {
        authored: Authored {
            id: entity.id.clone(),
            source_path: path.clone(),
            kind: "entity".into(),
            facts: raw.clone(),
            geometry_provenance: provenance,
        },
        bounds,
        sigil: Some(Sigil {
            kind: "database".into(),
            position: Point {
                x: bounds.x + 8.0,
                y: bounds.y + 7.0,
            },
            size: 14.0,
        }),
        label: label.clone(),
        text: vec![text(
            format!("{path}/label"),
            label,
            header.map_or(bounds.x + bounds.width / 2.0, |(x, _)| x),
            bounds.y + 19.0,
            header.map_or(HEADER_FONT_MINIMUM, |(_, font)| font),
            "middle",
        )],
        detail_rows,
    }
}

/// The centred title position and font, or none when the title cannot fit the
/// header interval between the sigil porch and the padding.
fn header_text(label: &str, bounds: Rect) -> Option<(f64, f64)> {
    let inset = 30.0;
    let title_width = bounds.width - inset - PAD_X;
    let font = fitted_node_font_size(label, title_width + 8.0, HEADER_FONT, HEADER_FONT_MINIMUM);
    let label_width = text_units(label) * font * 0.6;
    if label_width > title_width {
        return None;
    }
    let left = bounds.x + inset;
    let right = left + title_width;
    let x = (bounds.x + bounds.width / 2.0)
        .max(left + label_width / 2.0)
        .min(right - label_width / 2.0);
    Some((js_round(x * 10.0) / 10.0, font))
}

fn validate_entities(entities: &[Entity<'_>], errors: &mut Diagnostics) {
    for entity in entities {
        let label = string(entity.raw, "label");
        if header_text(&label, entity.bounds).is_none() {
            errors.push(diagnostic(
                "erd",
                "erd/header-text-capacity",
                &format!("/entities/{}/label", entity.index),
                format!(
                    "Entity {} title does not fit the header at a readable size; widen the entity or shorten the title",
                    entity.id
                ),
                json!({"label":label,"availableWidth":entity.bounds.width - 30.0 - PAD_X}),
            ));
        }
    }
    for entity in entities {
        let key_column = key_column_width(entity.raw);
        let available = entity.bounds.width - PAD_X * 2.0 - key_column;
        for (index, attribute) in list(&entity.raw["attributes"]).iter().enumerate() {
            let name = text_units(&string(attribute, "name")) * 0.6;
            let kind = text_units(&string(attribute, "type")) * 0.6;
            let needed = name * ROW_FONT
                + if kind > 0.0 {
                    kind * TYPE_FONT + 8.0
                } else {
                    0.0
                };
            if needed > available {
                let required = (needed + PAD_X * 2.0 + key_column).ceil();
                errors.push(diagnostic(
                    "erd",
                    "erd/attribute-text-capacity",
                    &format!("/entities/{}/attributes/{index}", entity.index),
                    format!(
                        "Entity {} attribute {index} needs {}px of text but only {}px is available; set its width to at least {required}px",
                        entity.id,
                        needed.ceil(),
                        available.floor()
                    ),
                    json!({"entityWidth":entity.bounds.width,"requiredEntityWidth":required,"availableTextWidth":available,"requiredTextWidth":needed,"paddingWidth":PAD_X*2.0,"keyColumnWidth":key_column,"nameFontSize":ROW_FONT,"typeFontSize":TYPE_FONT}),
                ));
            }
        }
    }
    for (index, entity) in entities.iter().enumerate() {
        for other in &entities[index + 1..] {
            if rects_overlap(&entity.bounds, &other.bounds, 0.0) {
                errors.push(diagnostic(
                    "erd",
                    "layout/constraint",
                    &format!("/entities/{}", entity.index),
                    format!("Entity {} overlaps entity {}", entity.id, other.id),
                    json!({"other":other.id,"bounds":entity.bounds,"otherBounds":other.bounds}),
                ));
            }
        }
    }
}

fn trunk_containers(
    edges: &[Edge],
    entities: &[Entity<'_>],
    groups: &[trunks::Trunk],
    assignment: &HashMap<usize, usize>,
    runs: &[Vec<trunks::Run>],
    styles: &[String],
) -> Vec<Container> {
    // Style is part of the cluster key so differently styled groups never share
    // one trunk path even if their coordinates coincide.
    type Cluster = ((usize, u64, String), Vec<(f64, f64, usize)>);
    let mut clusters: Vec<Cluster> = vec![];
    for (position, member_runs) in runs.iter().enumerate() {
        if member_runs.is_empty() {
            continue;
        }
        let group = &groups[assignment[&position]];
        let key = (
            group.axis,
            group.coordinate.to_bits(),
            styles[position].clone(),
        );
        let index = match clusters.iter().position(|(candidate, _)| *candidate == key) {
            Some(index) => index,
            None => {
                clusters.push((key, vec![]));
                clusters.len() - 1
            }
        };
        for run in member_runs {
            clusters[index]
                .1
                .push((run.low, run.high, edges[position].index));
        }
    }
    let point = |axis: usize, coordinate: f64, along: f64| {
        if axis == 0 {
            Point {
                x: coordinate,
                y: along,
            }
        } else {
            Point {
                x: along,
                y: coordinate,
            }
        }
    };
    let mut containers = vec![];
    for ((axis, bits, style), mut cluster) in clusters {
        let coordinate = f64::from_bits(bits);
        cluster.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64, Vec<usize>)> = vec![];
        for (low, high, member) in cluster {
            match merged.last_mut() {
                Some(last) if low <= last.1 + 0.5 => {
                    last.1 = last.1.max(high);
                    last.2.push(member);
                }
                _ => merged.push((low, high, vec![member])),
            }
        }
        // Interleaved branches leave short uncovered stretches on an otherwise
        // continuous bus; bridge them while the bridge clears every table.
        let mut segments: Vec<(f64, f64, Vec<usize>)> = vec![];
        for segment in merged {
            if let Some(last) = segments.last_mut() {
                let gap = segment.0 - last.1;
                if gap > 0.5 && gap <= trunks::TRUNK_BRIDGE {
                    let (start, end) = (
                        point(axis, coordinate, last.1),
                        point(axis, coordinate, segment.0),
                    );
                    if !entities
                        .iter()
                        .any(|entity| segment_intersects_rect(start, end, &entity.bounds, 0.0))
                    {
                        last.1 = segment.1;
                        last.2.extend(segment.2);
                        continue;
                    }
                }
            }
            segments.push(segment);
        }
        for (low, high, members) in segments {
            let (start, end) = (point(axis, coordinate, low), point(axis, coordinate, high));
            let index = containers.len();
            containers.push(Container {
                authored: Authored {
                    id: format!("trunk-{index}"),
                    source_path: "/relationships".into(),
                    kind: "erd-trunk".into(),
                    facts: json!({"variant":style,"axis":axis,"coordinate":coordinate}),
                    geometry_provenance: members
                        .iter()
                        .map(|member| format!("/relationships/{member}"))
                        .chain(["bundled-relationship-trunk".into()])
                        .collect(),
                },
                bounds: Rect {
                    x: start.x.min(end.x),
                    y: start.y.min(end.y),
                    width: (end.x - start.x).abs(),
                    height: (end.y - start.y).abs(),
                },
                label_plate: None,
                rule: Some([start, end]),
                members: members
                    .iter()
                    .map(|member| {
                        edges
                            .iter()
                            .find(|edge| edge.index == *member)
                            .and_then(|edge| edge.raw["id"].as_str())
                            .map_or_else(|| format!("relationship-{member}"), str::to_owned)
                    })
                    .collect(),
                text: vec![],
            });
        }
    }
    containers
}
