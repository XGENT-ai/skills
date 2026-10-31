use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::geometry::{
    fitted_node_font_size_for_contract, rects_overlap_for_contract, segment_intersects_rect,
    text_units_for_contract,
};
use crate::input::{CompileOptions, DiagramInput, DiagramType, InputContract};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Relation, Sigil,
    Text,
};
use serde_json::{json, Value};

#[path = "architecture/presentation.rs"]
mod presentation;
use super::router;

fn string(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap_or("").to_owned()
}
fn number(value: &Value, field: &str, fallback: f64) -> f64 {
    value[field].as_f64().unwrap_or(fallback)
}
fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
fn point(value: &Value) -> Option<Point> {
    Some(Point {
        x: value.get(0)?.as_f64()?,
        y: value.get(1)?.as_f64()?,
    })
}
fn text(
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
fn authored(
    raw: &Value,
    path: String,
    id: String,
    kind: String,
    provenance: Vec<String>,
) -> Authored {
    Authored {
        id,
        source_path: path,
        kind,
        facts: raw.clone(),
        geometry_provenance: provenance,
    }
}
fn diagnostic(code: &str, path: &str, message: impl Into<String>, evidence: Value) -> Diagnostic {
    let mut result = Diagnostic::error(Some("architecture"), code, path, message);
    result.stage = "layout".into();
    result.evidence = evidence;
    result.supported_fixes=vec!["Adjust the identified geometry while preserving component responsibilities, relationships, labels and boundary membership".into()];
    result
}
fn contains(outer: &Rect, inner: &Rect) -> bool {
    outer.x <= inner.x + 1e-9
        && outer.y <= inner.y + 1e-9
        && outer.x + outer.width + 1e-9 >= inner.x + inner.width
        && outer.y + outer.height + 1e-9 >= inner.y + inner.height
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    if input.diagram_type != DiagramType::Architecture {
        return Err(vec![diagnostic(
            "architecture/wrong-type",
            "/diagram_type",
            "Architecture compiler requires architecture input",
            Value::Null,
        )]);
    }
    let contract = input.input_contract;
    let legacy = contract == InputContract::Legacy217;
    let document = &input.document;
    let meta = &document["meta"];
    let mut geometry_meta = meta.clone();
    if input.quality == crate::input::Quality::Showcase {
        geometry_meta["quality_profile"] = json!("showcase");
    }
    let grid = document["layout"]["mode"] == "grid";
    let mut errors = vec![];
    engineering_profile(document, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut nodes = vec![];
    let mut occupied = std::collections::HashMap::new();
    for (index, raw) in list(&document["components"]).iter().enumerate() {
        let path = format!("/components/{index}");
        let id = string(raw, "id");
        let position = point(&raw["pos"]).or_else(|| {
            if !grid {
                return None;
            }
            // JSON Schema integers include integral decimal numbers. Geometry
            // uses the same floating-point number domain as the source renderer.
            let row = raw["row"].as_f64()?;
            let col = raw["col"].as_f64()?;
            let columns = number(&document["layout"], "cols", 4.0);
            if col >= columns {
                errors.push(diagnostic(
                    "architecture/grid-column",
                    &format!("{path}/col"),
                    "Grid column exceeds layout.cols",
                    json!({"column":col,"columns":columns}),
                ));
            }
            let key = |value: f64| if value == 0.0 { 0 } else { value.to_bits() };
            if let Some(previous) = occupied.insert((key(row), key(col)), index) {
                errors.push(diagnostic(
                    "architecture/grid-cell",
                    &path,
                    "Components share a grid cell",
                    json!({"otherComponent":previous,"row":row,"col":col}),
                ));
            }
            let origin = point(&document["layout"]["origin"]).unwrap_or(Point { x: 40.0, y: 80.0 });
            Some(Point {
                x: origin.x
                    + col
                        * (number(&document["layout"], "cellW", 130.0)
                            + number(&document["layout"], "gapX", 30.0)),
                y: origin.y
                    + row
                        * (number(&document["layout"], "cellH", 64.0)
                            + number(&document["layout"], "gapY", 40.0)),
            })
        });
        let Some(position) = position else {
            errors.push(diagnostic(
                "architecture/component-position",
                &path,
                "Component requires pos or grid row/col",
                Value::Null,
            ));
            continue;
        };
        let size = point(&raw["size"]).unwrap_or_else(|| Point {
            x: if raw.get("brand").is_some() {
                120.0_f64.max(
                    text_units_for_contract(&string(raw, "label"), contract) * 11.0 * 0.6 + 8.0,
                ) + 48.0
            } else {
                120.0
            },
            y: 60.0,
        });
        let bounds = Rect {
            x: position.x,
            y: position.y,
            width: size.x,
            height: size.y,
        };
        let label = string(raw, "label");
        let sub = string(raw, "sublabel");
        let tag = string(raw, "tag");
        let center = bounds.x + bounds.width / 2.0;
        let font = |value: &str, preferred: f64, minimum: f64| {
            fitted_node_font_size_for_contract(value, bounds.width, preferred, minimum, contract)
        };
        let mut texts = vec![text(
            format!("{path}/label"),
            &label,
            center,
            bounds.y + bounds.height / 2.0 + if sub.is_empty() { 4.0 } else { -2.0 },
            if raw.get("brand").is_some() {
                11.0
            } else {
                font(&label, 11.0, 8.0)
            },
            "middle",
        )];
        if !sub.is_empty() {
            texts.push(text(
                format!("{path}/sublabel"),
                &sub,
                center,
                bounds.y + bounds.height / 2.0 + 14.0,
                font(&sub, 9.0, 6.0),
                "middle",
            ));
        }
        if !tag.is_empty() {
            texts.push(text(
                format!("{path}/tag"),
                &tag,
                center,
                bounds.y + bounds.height - 8.0,
                font(&tag, 7.0, 6.0),
                "middle",
            ));
        }
        let mut provenance = if raw.get("pos").is_some() {
            vec![format!("{path}/pos")]
        } else {
            vec![
                format!("{path}/row"),
                format!("{path}/col"),
                "/layout".into(),
            ]
        };
        provenance.push(if raw.get("size").is_some() {
            format!("{path}/size")
        } else {
            format!("default-size:{}x60", size.x)
        });
        nodes.push(Node {
            brand: None,
            authored: authored(raw, path, id, string(raw, "type"), provenance),
            bounds,
            label,
            text: texts,
            sigil: {
                let kind = if legacy {
                    raw["type"].as_str()
                } else {
                    raw["icon"].as_str().or(raw["type"].as_str())
                }
                .unwrap_or("neutral");
                (kind != "none").then(|| Sigil {
                    kind: kind.into(),
                    position: Point {
                        x: bounds.x + 6.0,
                        y: bounds.y + 6.0,
                    },
                    size: 11.0,
                })
            },
            detail_rows: Vec::new(),
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let raw_boundaries = presentation::measure_boundaries(document, &nodes, contract);
    let edges = router::connections(document, &nodes);
    let frames: Vec<_> = raw_boundaries
        .iter()
        .map(|boundary| router::Frame {
            bounds: boundary.bounds,
            radius: if boundary.raw["kind"] == "security-group" {
                8.0
            } else {
                12.0
            },
        })
        .collect();
    let routes = router::route(
        &nodes,
        &edges,
        &frames,
        contract,
        meta.get("viewBox").is_none(),
    );
    let mut labels: Vec<router::Label> = edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            router::Label::new(index, &edge.raw, &routes[index].points, contract)
        })
        .collect();
    let extra: Vec<Rect> = if legacy {
        vec![]
    } else {
        labels
            .iter()
            .map(|label| label.rect)
            .chain(routes.iter().flat_map(|route| {
                route.points.iter().map(|point| Rect {
                    x: point.x,
                    y: point.y,
                    width: 0.0,
                    height: 0.0,
                })
            }))
            .collect()
    };
    let entries = presentation::legend_entries(meta, &nodes, contract);
    let authored_canvas = point(&meta["viewBox"]).map(|size| Rect {
        x: 0.0,
        y: 0.0,
        width: size.x,
        height: size.y,
    });
    let boundaries = presentation::resolve_titles(
        &raw_boundaries,
        &nodes,
        &routes,
        &edges,
        &labels,
        &extra,
        &entries,
        authored_canvas,
        geometry_meta.get("quality_profile").is_some(),
        contract,
        &mut errors,
    );
    let view_box = authored_canvas
        .unwrap_or_else(|| presentation::auto_canvas(&nodes, &boundaries, &extra, &entries));
    if !legacy && input.quality == crate::input::Quality::Showcase {
        let rows = presentation::legend_rows(&entries, view_box.width - 80.0);
        let bottom = if entries.is_empty() {
            view_box.height
        } else {
            view_box.height - 48.0 - rows.len().saturating_sub(1) as f64 * 22.0
        };
        router::place_labels(
            &mut labels,
            &routes,
            &nodes,
            &boundaries
                .iter()
                .map(|boundary| boundary.title)
                .collect::<Vec<_>>(),
            view_box,
            bottom,
        );
    }
    validate(
        &nodes,
        &edges,
        &routes,
        &boundaries,
        &labels,
        view_box,
        &geometry_meta,
        contract,
        &mut errors,
    );
    let legends = presentation::legends(
        &entries,
        &nodes,
        &boundaries,
        &routes,
        &labels,
        view_box,
        meta,
        contract,
        &mut errors,
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    let relations = edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            let label = string(&edge.raw, "label");
            let plate = labels.iter().find(|label| label.edge == index);
            let mut provenance = vec![
                format!("/connections/{}/from", edge.index),
                format!("/connections/{}/to", edge.index),
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
                if edge.raw.get(field).is_some() {
                    provenance.push(format!("/connections/{}/{field}", edge.index));
                }
            }
            if !edge.explicit() {
                provenance.push(if legacy {
                    "legacy-automatic-routing".into()
                } else {
                    "obstacle-grid-and-readable-routing".into()
                });
            }
            Relation {
                authored: authored(
                    &edge.raw,
                    format!("/connections/{}", edge.index),
                    edge.raw["id"]
                        .as_str()
                        .map_or_else(|| format!("connection-{}", edge.index), str::to_owned),
                    "connection".into(),
                    provenance,
                ),
                from: nodes[edge.from].authored.id.clone(),
                to: nodes[edge.to].authored.id.clone(),
                variant: edge.raw["variant"].as_str().unwrap_or("default").into(),
                label: label.clone(),
                note: None,
                note_bounds: None,
                note_backdrop: None,
                junction: None,
                label_owner: None,
                points: routes[index].points.clone(),
                crossover_halo: !legacy && !edge.explicit(),
                independent_ports: !legacy && !edge.explicit() && edge.raw.get("labelAt").is_none(),
                label_plate: plate.map(|label| label.rect),
                text: plate
                    .map(|plate| {
                        vec![text(
                            format!("/connections/{}/label", edge.index),
                            label,
                            plate.anchor.x,
                            plate.anchor.y,
                            8.0,
                            "middle",
                        )]
                    })
                    .unwrap_or_default(),
                marker_start: None,
                marker_end: None,
            }
        })
        .collect();
    let containers = boundaries
        .iter()
        .map(|boundary| Container {
            authored: authored(
                &boundary.raw,
                format!("/boundaries/{}", boundary.index),
                format!("boundary-{}", boundary.index),
                string(&boundary.raw, "kind"),
                vec![
                    format!("/boundaries/{}/wraps", boundary.index),
                    if boundary.raw.get("pad").is_some() {
                        format!("/boundaries/{}/pad", boundary.index)
                    } else {
                        "default-boundary-pad:30".into()
                    },
                    "measured-title-rail".into(),
                ],
            ),
            bounds: boundary.bounds,
            rule: None,
            label_plate: Some(boundary.title),
            members: list(&boundary.raw["wraps"])
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            text: vec![text(
                format!("/boundaries/{}/label", boundary.index),
                string(&boundary.raw, "label"),
                boundary.title.x + 4.0,
                boundary.title.y + boundary.font + 4.0,
                boundary.font,
                "start",
            )],
        })
        .collect();
    let cards = list(&document["cards"])
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
                authored: authored(
                    raw,
                    path,
                    format!("card-{index}"),
                    "card".into(),
                    vec!["reader-card-flow".into()],
                ),
                bounds: Rect::default(),
                text: texts,
            }
        })
        .collect();
    Ok(CompiledDiagram {
        diagram_type: input.diagram_type,
        schema_version: document["schema_version"]
            .as_f64()
            .expect("validated schema version") as u32,
        input_contract: contract,
        quality: input.quality,
        meta: meta.clone(),
        view_box,
        reader_fit: if legacy {
            "legacy"
        } else if authored_canvas.is_some() {
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
        cards,
        facts: DiagramFacts::Architecture {
            layout: document.get("layout").cloned(),
            components: list(&document["components"]).to_vec(),
            boundaries: list(&document["boundaries"]).to_vec(),
        },
        diagnostics: vec![],
    })
}

fn engineering_profile(document: &Value, errors: &mut Diagnostics) {
    if document["meta"]["engineering_profile"] != "deployment-ownership" {
        return;
    }
    let boundaries = list(&document["boundaries"]);
    let memberships = |id: &str, kind: &str| {
        boundaries
            .iter()
            .enumerate()
            .filter(|(_, boundary)| {
                boundary["kind"] == kind
                    && list(&boundary["wraps"])
                        .iter()
                        .any(|member| member.as_str() == Some(id))
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };
    let mut emit = |code: &str, path: &str, message: &str, evidence: Value, fix: String| {
        let mut error = diagnostic(code, path, message, evidence);
        error.stage = "input".into();
        error.supported_fixes = vec![fix];
        errors.push(error);
    };
    for kind in ["region", "security-group"] {
        if !boundaries.iter().any(|boundary| boundary["kind"] == kind) {
            emit(
                "engineering/deployment-boundary-kind",
                "/boundaries",
                "Deployment ownership requires both region and security-group scopes",
                json!({"requiredKind":kind,"found":0}),
                format!("Add a {kind} boundary with its truthful wraps list"),
            );
        }
    }
    for (index, component) in list(&document["components"]).iter().enumerate() {
        if component["type"] == "external" {
            continue;
        }
        let id = string(component, "id");
        let path = format!("/components/{index}");
        if string(component, "tag").trim().is_empty() {
            emit(
                "engineering/deployment-owner-missing",
                &format!("{path}/tag"),
                "Deployment component must name its responsible owner in tag",
                json!({"component":id,"ownerField":"tag"}),
                format!("Set {path}/tag to the responsible team or owner"),
            );
        }
        let regions = memberships(&id, "region");
        if regions.is_empty() {
            emit(
                "engineering/deployment-region-scope",
                &path,
                "Deployment component requires a region boundary",
                json!({"component":id,"regionMemberships":0}),
                "Add the component to its real region boundary wraps list".into(),
            );
        } else if regions.len() > 1 {
            emit(
                "engineering/deployment-region-ambiguous",
                &path,
                "Deployment component belongs to more than one region",
                json!({"component":id,"regions":regions}),
                "Keep the component in exactly one real region wraps list".into(),
            );
        }
        if component["type"] == "database" && memberships(&id, "security-group").is_empty() {
            emit(
                "engineering/deployment-private-state",
                &path,
                "Stateful deployment component requires a private security-group scope",
                json!({"component":id,"privateMemberships":0}),
                "Add the database to its real private security-group wraps list".into(),
            );
        }
    }
    for (index, boundary) in boundaries.iter().enumerate() {
        if boundary["kind"] != "security-group" {
            continue;
        }
        let members: Vec<_> = list(&boundary["wraps"])
            .iter()
            .filter_map(Value::as_str)
            .map(|id| (id, memberships(id, "region")))
            .collect();
        let unique: std::collections::HashSet<_> = members
            .iter()
            .flat_map(|(_, regions)| regions.iter().copied())
            .collect();
        if members.is_empty()
            || !members.iter().all(|(_, regions)| regions.len() == 1)
            || unique.len() != 1
        {
            emit(
                "engineering/deployment-private-region-consistency",
                &format!("/boundaries/{index}/wraps"),
                "Private scope members require exactly one shared region",
                json!({"members":members}),
                "Assign every private-scope component to exactly one shared region".into(),
            );
        }
    }
    for (index, edge) in list(&document["connections"]).iter().enumerate() {
        let from = string(edge, "from");
        let to = string(edge, "to");
        let crossed:Vec<_>=boundaries.iter().enumerate().filter(|(_,boundary)|{let members=list(&boundary["wraps"]);members.iter().any(|id|id.as_str()==Some(&from))!=members.iter().any(|id|id.as_str()==Some(&to))}).map(|(index,boundary)|json!({"boundaryIndex":index,"kind":boundary["kind"],"label":boundary["label"]})).collect();
        if !crossed.is_empty() && string(edge, "label").trim().is_empty() {
            emit(
                "engineering/deployment-crossing-mechanism",
                &format!("/connections/{index}/label"),
                "Cross-boundary deployment relationship must name its mechanism",
                json!({"from":from,"to":to,"crossedBoundaries":crossed}),
                format!("Set /connections/{index}/label to the real crossing mechanism"),
            );
        }
    }
}

// Validate the distinct compiled geometry collections without hiding inputs.
#[allow(clippy::too_many_arguments)]
fn validate(
    nodes: &[Node],
    edges: &[router::Edge],
    routes: &[router::Route],
    boundaries: &[presentation::Boundary],
    labels: &[router::Label],
    canvas: Rect,
    meta: &Value,
    contract: InputContract,
    errors: &mut Diagnostics,
) {
    for (index, node) in nodes.iter().enumerate() {
        let raw = &node.authored.facts;
        let bounds = node.bounds;
        let path = format!("/components/{index}");
        if ![bounds.x, bounds.y, bounds.width, bounds.height]
            .iter()
            .all(|value| value.is_finite())
        {
            errors.push(diagnostic(
                "architecture/non-finite-component-geometry",
                &path,
                "Component geometry must be finite",
                json!({"bounds":bounds}),
            ));
            continue;
        }
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            errors.push(diagnostic(
                "architecture/invalid-component-size",
                &format!("{path}/size"),
                "Component dimensions must be positive",
                json!({"bounds":bounds}),
            ));
            continue;
        }
        if !contains(&canvas, &bounds) {
            errors.push(diagnostic(
                "layout/component-out-of-bounds",
                &path,
                "Component falls outside the canvas",
                json!({"bounds":bounds,"viewBox":canvas}),
            ));
        }
        let estimate = text_units_for_contract(&node.label, contract) * 6.6;
        if estimate > bounds.width + 8.0 {
            errors.push(diagnostic("architecture/component-label-overflow",&format!("{path}/label"),"Component label exceeds its fixed width",json!({"text":node.label,"estimatedWidthPx":estimate,"availableWidthPx":bounds.width})));
        }
        for field in ["sublabel", "tag"] {
            let value = string(raw, field);
            let required = text_units_for_contract(&value, contract) * 6.0 * 0.6;
            if required > bounds.width - 8.0 {
                errors.push(diagnostic(&format!("architecture/component-{field}-overflow"),&format!("{path}/{field}"),"Text cannot fit at its legible font minimum",json!({"text":value,"requiredWidthPx":required,"availableWidthPx":bounds.width-8.0,"minimumFontPx":6})));
            }
        }
        for other in nodes.iter().take(index) {
            if rects_overlap_for_contract(&bounds, &other.bounds, 8.0, contract) {
                errors.push(diagnostic("layout/component-overlap",&path,"Components must remain at least 8px apart",json!({"other":other.authored.id,"bounds":bounds,"otherBounds":other.bounds,"minimumGapPx":8})));
            }
        }
    }
    for boundary in boundaries {
        let path = format!("/boundaries/{}", boundary.index);
        if !contains(&canvas, &boundary.bounds) {
            errors.push(diagnostic(
                "layout/boundary-out-of-bounds",
                &path,
                "Boundary extends outside the canvas; preserve wraps when repairing",
                json!({"bounds":boundary.bounds,"viewBox":canvas,"members":boundary.raw["wraps"]}),
            ));
        }
        if meta.get("quality_profile").is_some() {
            if !contains(&boundary.bounds, &boundary.title) || !contains(&canvas, &boundary.title) {
                errors.push(diagnostic(
                    "layout/boundary-title-out-of-bounds",
                    &format!("{path}/label"),
                    "Boundary title must fit its frame and canvas",
                    json!({"title":boundary.title,"bounds":boundary.bounds}),
                ));
            }
            if nodes.iter().any(|node| {
                rects_overlap_for_contract(&boundary.title, &node.bounds, 0.0, contract)
            }) {
                errors.push(diagnostic(
                    "layout/boundary-title-overlap",
                    &format!("{path}/label"),
                    "Boundary title overlaps a component",
                    json!({"title":boundary.title}),
                ));
            }
        }
        if contract == InputContract::PhoenixV1 && meta["quality_profile"] == "showcase" {
            for node in nodes {
                if !list(&boundary.raw["wraps"])
                    .iter()
                    .any(|id| id.as_str() == Some(&node.authored.id))
                    && contains(&boundary.bounds, &node.bounds)
                {
                    errors.push(diagnostic(
                        "layout/boundary-encloses-non-member",
                        &format!("{path}/wraps"),
                        "Boundary frame encloses a component outside its wraps set",
                        json!({"component":node.authored.id,"bounds":boundary.bounds}),
                    ));
                }
            }
        }
    }
    for (index, left) in boundaries.iter().enumerate() {
        for right in boundaries.iter().skip(index + 1) {
            let left_ids = list(&left.raw["wraps"]);
            let right_ids = list(&right.raw["wraps"]);
            if meta.get("quality_profile").is_some()
                && rects_overlap_for_contract(&left.title, &right.title, 0.0, contract)
            {
                errors.push(diagnostic(
                    "layout/boundary-title-overlap",
                    &format!("/boundaries/{}/label", left.index),
                    "Boundary titles overlap",
                    json!({"otherBoundary":right.index}),
                ));
            }
            if meta["engineering_profile"] != "deployment-ownership" {
                continue;
            }
            let shared = left_ids.iter().any(|id| right_ids.contains(id));
            let left_nested = left_ids.iter().all(|id| right_ids.contains(id));
            let right_nested = right_ids.iter().all(|id| left_ids.contains(id));
            let overlap = rects_overlap_for_contract(&left.bounds, &right.bounds, 0.0, contract);
            let correct = (left_nested && contains(&right.bounds, &left.bounds))
                || (right_nested && contains(&left.bounds, &right.bounds));
            if (shared && !left_nested && !right_nested) || (overlap && (!shared || !correct)) {
                errors.push(diagnostic("architecture/boundary-membership",&format!("/boundaries/{}/wraps",left.index),"Deployment boundary frames must agree with nested wraps membership",json!({"otherBoundary":right.index,"left":left.raw["wraps"],"right":right.raw["wraps"]})));
            }
        }
    }
    for (index, edge) in edges.iter().enumerate() {
        let route = &routes[index];
        let path = format!("/connections/{}", edge.index);
        if !router::authored_sides_honored(edge, route) {
            errors.push(diagnostic("clean-flow/endpoint-side-direction",&path,"Route must leave and enter perpendicular to its selected endpoint sides",json!({"points":route.points,"fromSide":route.sides.from.name(),"toSide":route.sides.to.name()})));
        }
        for (other, node) in nodes.iter().enumerate() {
            for (segment, pair) in route.points.windows(2).enumerate() {
                if (other == edge.from && segment == 0)
                    || (other == edge.to && segment == route.points.len() - 2)
                {
                    continue;
                }
                if segment_intersects_rect(pair[0], pair[1], &node.bounds, 0.0) {
                    errors.push(diagnostic("clean-flow/node-intersection",&path,"Relationship crosses a component",json!({"component":node.authored.id,"segment":segment,"points":route.points})));
                    break;
                }
            }
        }
        let start = route.points[0];
        let end = *route.points.last().unwrap();
        let distance = (start.x - end.x).hypot(start.y - end.y);
        if distance < 24.0 {
            errors.push(diagnostic(
                if edge.from == edge.to {
                    "layout/self-loop-ports"
                } else {
                    "architecture/short-connection"
                },
                &path,
                "Connection ports must remain at least 24px apart",
                json!({"distancePx":distance,"minimumPx":24}),
            ));
        }
        if contract == InputContract::PhoenixV1
            && meta["quality_profile"] == "showcase"
            && route
                .points
                .iter()
                .any(|p| p.x < 0.0 || p.y < 0.0 || p.x > canvas.width || p.y > canvas.height)
        {
            errors.push(diagnostic(
                "layout/route-out-of-bounds",
                &path,
                "Relationship route extends outside the canvas",
                json!({"points":route.points,"viewBox":canvas}),
            ));
        }
    }
    for label in labels {
        let path = format!("/connections/{}/label", edges[label.edge].index);
        for node in nodes {
            if rects_overlap_for_contract(&label.rect, &node.bounds, -2.0, contract) {
                errors.push(diagnostic(
                    "layout/label-component-overlap",
                    &path,
                    "Relationship label overlaps a component",
                    json!({"component":node.authored.id,"labelPlate":label.rect}),
                ));
            }
        }
        if meta.get("quality_profile").is_some()
            && boundaries.iter().any(|boundary| {
                rects_overlap_for_contract(&label.rect, &boundary.title, 0.0, contract)
            })
        {
            errors.push(diagnostic(
                "layout/label-title-overlap",
                &path,
                "Relationship label overlaps a boundary title",
                json!({"labelPlate":label.rect}),
            ));
        }
    }
}
