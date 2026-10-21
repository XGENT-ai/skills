use crate::input::{DiagramInput, DiagramType};
use crate::scene::DiagramFacts;
use scraper::{Html, Selector};
use serde_json::{json, Value};

fn array<'a>(document: &'a Value, key: &str) -> &'a [Value] {
    document
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn same_members(actual: &Value, expected: &[Value]) -> bool {
    let Some(actual) = actual.as_array() else {
        return false;
    };
    if actual.len() != expected.len() {
        return false;
    }
    let mut remaining: Vec<_> = actual.iter().collect();
    for fact in expected {
        let Some(index) = remaining.iter().position(|entry| *entry == fact) else {
            return false;
        };
        remaining.remove(index);
    }
    true
}

fn dom_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "\u{fffd}")
}

/// The single embedded metadata object of an artifact.
pub(super) fn embedded(document: &Html) -> Option<Value> {
    let selector = Selector::parse("script#phoenix-diagram-metadata[type='application/json']")
        .expect("static selector");
    let mut scripts = document.select(&selector);
    let script = scripts.next()?;
    if scripts.next().is_some() {
        return None;
    }
    serde_json::from_str::<Value>(&script.text().collect::<String>()).ok()
}

/// Bind authored facts without running layout or serializing another artifact.
pub(super) fn matches(bytes: &[u8], input: &DiagramInput) -> bool {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return false;
    };
    let document = Html::parse_document(source);
    let Some(metadata) = embedded(&document) else {
        return false;
    };
    if metadata["tool"] != "phoenix-diagram"
        || metadata["meta"] != input.document["meta"]
        || metadata["diagramType"] != json!(input.diagram_type)
        || metadata["inputContract"] != json!(input.input_contract)
        || metadata["quality"] != json!(input.quality)
        || metadata["schemaVersion"].as_f64() != input.document["schema_version"].as_f64()
    {
        return false;
    }
    let svg_selector = Selector::parse("svg").expect("static selector");
    let Some(svg) = document.select(&svg_selector).next() else {
        return false;
    };
    let contract = match input.input_contract {
        crate::input::InputContract::Legacy217 => "legacy-2.17",
        crate::input::InputContract::PhoenixV1 => "phoenix-v1",
    };
    let quality = match input.quality {
        crate::input::Quality::Standard => "standard",
        crate::input::Quality::Showcase => "showcase",
    };
    if svg.value().attr("data-diagram-type") != Some(input.diagram_type.as_str())
        || svg.value().attr("data-input-contract") != Some(contract)
        || svg.value().attr("data-quality-profile") != Some(quality)
    {
        return false;
    }
    for selector in ["head > title", "h1"] {
        let selector = Selector::parse(selector).expect("static selector");
        let headings: Vec<_> = document
            .select(&selector)
            .map(|element| element.text().collect::<String>())
            .collect();
        if headings.len() != 1
            || headings[0] != dom_text(input.document["meta"]["title"].as_str().unwrap_or_default())
        {
            return false;
        }
    }
    let (nodes, relations) = match input.diagram_type {
        DiagramType::Architecture => ("components", "connections"),
        DiagramType::Workflow => ("nodes", "edges"),
        DiagramType::Sequence => ("participants", "messages"),
        DiagramType::Dataflow => ("nodes", "flows"),
        DiagramType::Lifecycle => ("states", "transitions"),
        DiagramType::Erd | DiagramType::Class => (
            if input.diagram_type == DiagramType::Erd {
                "entities"
            } else {
                "types"
            },
            "relationships",
        ),
        DiagramType::Tree => ("nodes", ""),
        DiagramType::Timeline => ("events", ""),
        DiagramType::Waterfall => ("spans", ""),
    };
    // Tree edges are derived from each node's declared parent.
    let relation_facts: Vec<Value> = if input.diagram_type == DiagramType::Tree {
        array(&input.document, "nodes")
            .iter()
            .filter(|node| node.get("parent").is_some())
            .map(|node| json!({"from":node["parent"],"to":node["id"]}))
            .collect()
    } else {
        array(&input.document, relations).to_vec()
    };
    if !same_members(&metadata["graph"]["nodes"], array(&input.document, nodes))
        || !same_members(&metadata["graph"]["relations"], &relation_facts)
    {
        return false;
    }
    let Ok(facts) = serde_json::from_value::<DiagramFacts>(metadata["facts"].clone()) else {
        return false;
    };
    let facts_match = match facts {
        DiagramFacts::Architecture {
            layout,
            components,
            boundaries,
        } => {
            input.diagram_type == DiagramType::Architecture
                && layout.as_ref() == input.document.get("layout")
                && components == array(&input.document, "components")
                && boundaries == array(&input.document, "boundaries")
        }
        DiagramFacts::Workflow {
            semantic_checks,
            lanes,
            phases,
            groups,
            main_path,
        } => {
            input.diagram_type == DiagramType::Workflow
                && semantic_checks.as_ref() == input.document.get("semanticChecks")
                && lanes == array(&input.document, "lanes")
                && phases == array(&input.document, "phases")
                && groups == array(&input.document, "groups")
                && json!(main_path)
                    == input
                        .document
                        .get("mainPath")
                        .cloned()
                        .unwrap_or_else(|| json!([]))
        }
        DiagramFacts::Sequence {
            participants,
            messages,
            activations,
            segments,
        } => {
            input.diagram_type == DiagramType::Sequence
                && participants == array(&input.document, "participants")
                && messages == array(&input.document, "messages")
                && activations == array(&input.document, "activations")
                && segments == array(&input.document, "segments")
        }
        DiagramFacts::Dataflow {
            stages,
            nodes,
            flows,
        } => {
            input.diagram_type == DiagramType::Dataflow
                && stages == array(&input.document, "stages")
                && nodes == array(&input.document, "nodes")
                && flows == array(&input.document, "flows")
        }
        DiagramFacts::Lifecycle {
            states,
            transitions,
            lanes,
            main_path,
            ..
        } => {
            input.diagram_type == DiagramType::Lifecycle
                && states == array(&input.document, "states")
                && transitions == array(&input.document, "transitions")
                && lanes == array(&input.document, "lanes")
                && json!(main_path)
                    == input
                        .document
                        .get("mainPath")
                        .cloned()
                        .unwrap_or_else(|| json!([]))
        }
        DiagramFacts::Erd {
            entities,
            relationships,
        } => {
            input.diagram_type == DiagramType::Erd
                && entities == array(&input.document, "entities")
                && relationships == array(&input.document, "relationships")
        }
        DiagramFacts::Class {
            types,
            relationships,
        } => {
            input.diagram_type == DiagramType::Class
                && types == array(&input.document, "types")
                && relationships == array(&input.document, "relationships")
        }
        DiagramFacts::Tree {
            root,
            nodes,
            direction,
        } => {
            input.diagram_type == DiagramType::Tree
                && nodes == array(&input.document, "nodes")
                && nodes
                    .iter()
                    .filter(|node| node.get("parent").is_none())
                    .map(|node| &node["id"])
                    .eq([&json!(root)])
                && direction
                    == input.document["layout"]["direction"]
                        .as_str()
                        .unwrap_or("down")
        }
        DiagramFacts::Timeline {
            timezone,
            evidence,
            lanes,
            events,
            ..
        } => {
            let expected: Vec<Value> = array(&input.document, "events")
                .iter()
                .map(|event| {
                    json!({"id":event["id"],"lane":event["lane"],
                        "atMs":event["at"].as_str().and_then(crate::format::time::parse_timestamp),
                        "precision":event["precision"].as_str().unwrap_or("time"),
                        "kind":event["kind"].as_str().unwrap_or("default")})
                })
                .collect();
            input.diagram_type == DiagramType::Timeline
                && timezone == input.document["meta"]["timezone"].as_str().unwrap_or("UTC")
                && json!(evidence) == input.document["meta"]["evidence"]
                && lanes == array(&input.document, "lanes")
                && same_members(&json!(events), &expected)
        }
        DiagramFacts::Waterfall {
            unit,
            evidence,
            spans,
        } => {
            let expected: Vec<Value> = array(&input.document, "spans")
                .iter()
                .map(|span| {
                    let start = span["start"].as_f64();
                    let end = span["end"]
                        .as_f64()
                        .or_else(|| Some(start? + span["duration"].as_f64()?));
                    json!({"id":span["id"],"parent":span["parent"],"start":start,"end":end,
                        "status":span["status"].as_str().unwrap_or("ok"),"service":span["service"]})
                })
                .collect();
            input.diagram_type == DiagramType::Waterfall
                && unit == input.document["meta"]["unit"].as_str().unwrap_or("ms")
                && json!(evidence) == input.document["meta"]["evidence"]
                && same_members(&json!(spans), &expected)
        }
    };
    if !facts_match {
        return false;
    }
    let cards_selector = Selector::parse(".cards > .card").expect("static selector");
    let title_selector = Selector::parse("h3").expect("static selector");
    let items_selector = Selector::parse("li").expect("static selector");
    let cards: Vec<_> = document.select(&cards_selector).collect();
    let expected = array(&input.document, "cards");
    cards.len() == expected.len()
        && cards.into_iter().zip(expected).all(|(card, fact)| {
            let titles: Vec<_> = card
                .select(&title_selector)
                .map(|title| title.text().collect::<String>())
                .collect();
            let items: Vec<_> = card
                .select(&items_selector)
                .map(|item| item.text().collect::<String>())
                .collect();
            let expected_items: Vec<_> = array(fact, "items")
                .iter()
                .filter_map(Value::as_str)
                .map(dom_text)
                .collect();
            titles.len() == 1
                && titles[0] == dom_text(fact["title"].as_str().unwrap_or_default())
                && items == expected_items
        })
}
