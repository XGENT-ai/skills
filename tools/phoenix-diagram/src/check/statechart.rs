use super::{selector, ContentFinding};
use scraper::ElementRef;
use serde_json::{json, Value};

fn mismatch(path: String, message: &str, evidence: Value) -> ContentFinding {
    (
        "statechart/artifact-mismatch",
        path,
        message.into(),
        evidence,
    )
}

fn list<'a>(metadata: &'a Value, pointer: &str) -> &'a [Value] {
    metadata
        .pointer(pointer)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// The mark a chart state must show: every member breaks a rule, or only
/// some do; unmarked states list no rules.
fn derived_mark(state: &Value) -> Option<&'static str> {
    let listed = |key: &str| state[key].as_array().is_some_and(|rules| !rules.is_empty());
    if listed("breaks") {
        Some("breaks")
    } else if listed("mayBreak") {
        Some("may-break")
    } else {
        None
    }
}

// The chart is derived from the model; the drawn states, marks, start markers
// and arrows must restate the facts the projection produced.
pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings = Vec::new();
    let states = list(metadata, "/facts/states");
    let arrows = list(metadata, "/facts/arrows");
    let groups: Vec<ElementRef<'_>> = svg.select(&selector("g[data-node-id]")).collect();
    if groups.len() != states.len() {
        findings.push(mismatch(
            "/facts/states".into(),
            "The drawn chart states differ from the projected states",
            json!({"expected":states.len(),"actual":groups.len()}),
        ));
    }
    for (index, state) in states.iter().enumerate() {
        let id = state["id"].as_str().unwrap_or_default();
        let path = format!("/facts/states/{index}");
        let Some(group) = groups
            .iter()
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            findings.push(mismatch(
                path,
                "A projected chart state is not drawn",
                json!({"id":id}),
            ));
            continue;
        };
        let attribute = |name: &str| group.value().attr(name);
        let drawn = (
            attribute("data-node-kind"),
            attribute("data-statechart-mark"),
            attribute("data-statechart-bug-path").is_some(),
            attribute("data-statechart-start").is_some(),
        );
        let expected = (
            state["kind"].as_str(),
            derived_mark(state),
            state["bugPath"] == true,
            state["start"] == true,
        );
        if drawn != expected || state["mark"].as_str() != expected.1 {
            findings.push(mismatch(
                path,
                "A chart state's kind, mark, bug-path or start marker differs from the projection",
                json!({"id":id,"expected":[expected.0,expected.1,expected.2,expected.3],"actual":[drawn.0,drawn.1,drawn.2,drawn.3]}),
            ));
        }
    }
    let edges: Vec<ElementRef<'_>> = svg.select(&selector("path[data-edge-from]")).collect();
    if edges.len() != arrows.len() {
        findings.push(mismatch(
            "/facts/arrows".into(),
            "The drawn arrows differ from the projected arrows",
            json!({"expected":arrows.len(),"actual":edges.len()}),
        ));
    }
    for (index, (arrow, edge)) in arrows.iter().zip(&edges).enumerate() {
        let attribute = |name: &str| edge.value().attr(name).unwrap_or_default();
        if ["from", "to", "label", "id"].iter().any(|field| {
            arrow[field].as_str().unwrap_or_default() != attribute(&format!("data-edge-{field}"))
        }) {
            findings.push(mismatch(
                format!("/facts/arrows/{index}"),
                "An arrow's ends or label differ from the projection",
                json!({"id":arrow["id"]}),
            ));
        }
    }
    findings
}
