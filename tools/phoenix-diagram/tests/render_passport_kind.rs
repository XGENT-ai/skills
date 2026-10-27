// The Reader's Passport shows `data-node-kind` as the node's type only when the
// author provided it. Where the renderer derives that value as a colour slot
// (per span service, tree position, event tone or the fixed table fill), the
// root carries `data-node-kind-derived`.
#[path = "support/svg_oracle.rs"]
mod svg_oracle;

use phoenix_diagram::input::{DiagramType, Quality};
use scraper::{Html, Selector};
use serde_json::{json, Value};

/// Each node id with the type its Passport may present.
fn passport_types(document: &Value, kind: DiagramType) -> Vec<(String, Option<String>)> {
    let quality = if document["meta"]["quality_profile"] == "showcase" {
        Quality::Showcase
    } else {
        Quality::Standard
    };
    let scene =
        svg_oracle::compile(document, kind, quality).unwrap_or_else(|errors| panic!("{errors:?}"));
    let html = Html::parse_document(&svg_oracle::html(&scene));
    let svg = html
        .select(&Selector::parse(".diagram-container > svg").unwrap())
        .next()
        .unwrap();
    let root_derived = svg.value().attr("data-node-kind-derived").is_some();
    svg.select(&Selector::parse("g[data-node-id]").unwrap())
        .map(|node| {
            let value = node.value();
            let kind = value.attr("data-node-kind").unwrap_or_default();
            (
                value.attr("data-node-id").unwrap().to_owned(),
                (!root_derived && !kind.is_empty()).then(|| kind.to_owned()),
            )
        })
        .collect()
}

fn untyped(types: &[(String, Option<String>)]) -> bool {
    !types.is_empty() && types.iter().all(|(_, kind)| kind.is_none())
}

#[test]
fn untyped_spans_tree_items_tables_and_event_tones_present_no_type() {
    // The reference checkout request: its spans name services, never a type.
    let checkout = json!({"schema_version": 1, "diagram_type": "waterfall",
        "meta": {"title": "Checkout request execution", "evidence": "illustrative", "unit": "ms"},
        "spans": [
            {"id": "request", "name": "POST /checkout", "service": "checkout-api", "start": 0, "end": 1200, "detail": "Parent of every step below; its duration is not added to theirs."},
            {"id": "auth", "name": "Authenticate caller", "service": "identity", "start": 0, "duration": 100, "parent": "request"},
            {"id": "inventory", "name": "Look up inventory", "service": "inventory", "start": 100, "duration": 300, "parent": "request"},
            {"id": "discount", "name": "Calculate discount", "service": "checkout-api", "start": 100, "duration": 200, "parent": "request"},
            {"id": "payment", "name": "Charge payment", "service": "payments", "start": 400, "duration": 650, "parent": "request"},
            {"id": "save", "name": "Write order", "service": "checkout-api", "start": 1050, "duration": 150, "parent": "request"}]});
    let rebuild = json!({"schema_version": 1, "diagram_type": "waterfall",
        "meta": {"title": "Rebuilding the bundled examples", "evidence": "measured", "unit": "ms"},
        "spans": [
            {"id": "batch", "name": "Rebuild bundled examples (3 workers)", "start": 0, "end": 1286.2},
            {"id": "ex0", "name": "web-app.architecture", "start": 0.2, "end": 429.7, "parent": "batch", "service": "worker 1"}]});
    // The repository-evidence tree: folders and files have no authored type.
    let tree = json!({"schema_version": 1, "diagram_type": "tree",
        "meta": {"title": "Phoenix Diagram skill files"},
        "nodes": [
            {"id": "skill", "label": "skills/phoenix-diagram"},
            {"id": "schemas", "label": "schemas", "parent": "skill"},
            {"id": "scripts", "label": "scripts", "parent": "skill"}]});
    let erd: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.erd.json"
    ))
    .unwrap();
    let timeline: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.timeline.json"
    ))
    .unwrap();
    let presented: Vec<_> = [
        ("checkout", &checkout, DiagramType::Waterfall),
        ("rebuild", &rebuild, DiagramType::Waterfall),
        ("tree", &tree, DiagramType::Tree),
        ("erd", &erd, DiagramType::Erd),
        ("timeline", &timeline, DiagramType::Timeline),
    ]
    .into_iter()
    .map(|(name, document, kind)| (name, passport_types(document, kind)))
    .filter(|(_, types)| !untyped(types))
    .collect();
    assert!(presented.is_empty(), "{presented:?}");
}

#[test]
fn authored_types_still_reach_the_passport() {
    let architecture: Value = serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.architecture.json"
    ))
    .unwrap();
    assert_eq!(
        passport_types(&architecture, DiagramType::Architecture),
        [("client", "frontend"), ("service", "backend")]
            .map(|(id, kind)| (id.to_owned(), Some(kind.to_owned())))
    );
    // Every class type names its kind.
    let class = json!({"schema_version": 1, "diagram_type": "class",
        "meta": {"title": "Payment types", "output": "types.html"}, "layout": {"mode": "grid"},
        "types": [
            {"id": "gateway", "label": "PaymentGateway", "kind": "interface", "row": 0, "col": 0},
            {"id": "card", "label": "CardGateway", "kind": "class", "row": 1, "col": 0}],
        "relationships": [{"id": "card_implements", "from": "card", "to": "gateway", "kind": "realization"}]});
    assert_eq!(
        passport_types(&class, DiagramType::Class),
        [
            ("gateway".to_owned(), Some("interface".to_owned())),
            ("card".to_owned(), Some("class".to_owned()))
        ]
    );
}
