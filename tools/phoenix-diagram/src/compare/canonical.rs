//! Authored comparison semantics ported from the fixed Architecture comparator.
use crate::{
    diagnostic::{Diagnostic, Diagnostics},
    evidence::location::parse_remote,
    format::number::js_to_string,
};
use serde_json::{json, Map, Value};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

fn order(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(v)) => v.as_f64().is_some_and(|v| v != 0.0),
        Some(Value::String(v)) => !v.is_empty(),
        _ => true,
    }
}
fn string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => js_to_string(value.as_f64().unwrap()),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else {
                    string(Some(v))
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}
fn default_string(value: Option<&Value>) -> String {
    if truthy(value) {
        string(value)
    } else {
        String::new()
    }
}
fn sorted_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort_by(|a, b| order(a, b));
    values
}
fn values(value: Option<&Value>) -> &[Value] {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn serialize(value: &Value, output: &mut String) {
    match value {
        Value::Object(object) => {
            output.push('{');
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort_by(|a, b| order(a, b));
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).expect("JSON string"));
                output.push(':');
                serialize(&object[key], output);
            }
            output.push('}');
        }
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                serialize(value, output);
            }
            output.push(']');
        }
        Value::Number(number) => {
            output.push_str(&js_to_string(number.as_f64().expect("finite JSON number")))
        }
        _ => output.push_str(&serde_json::to_string(value).expect("JSON value")),
    }
}
fn serialized(value: &Value) -> String {
    let mut output = String::new();
    serialize(value, &mut output);
    output
}
fn equal(left: Option<&Value>, right: Option<&Value>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => serialized(left) == serialized(right),
        _ => false,
    }
}
fn sorted_objects(values: &[Value]) -> Vec<Value> {
    let mut values = values.to_vec();
    values.sort_by(|a, b| order(&serialized(a), &serialized(b)));
    values
}
fn sorted_values(values: &[Value]) -> Vec<Value> {
    let mut values = values.to_vec();
    values.sort_by(|a, b| order(&string(Some(a)), &string(Some(b))));
    values
}
fn copy(object: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        object.insert(key.into(), value.clone());
    }
}
fn object(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn normalize_repository(value: Option<&Value>) -> Option<Value> {
    if !truthy(value) {
        return None;
    }
    let repository = value.unwrap();
    let authored_url = default_string(repository.get("url"));
    let location = parse_remote(&authored_url, true, repository["provider"].as_str()).ok();
    let mut url = location
        .as_ref()
        .map(|l| l.url.clone())
        .unwrap_or(authored_url);
    if location
        .as_ref()
        .is_some_and(|location| matches!(location.provider, Some("github" | "gitlab")))
    {
        url = url.to_lowercase();
    }
    let mut result = Map::new();
    result.insert("url".into(), json!(url));
    result.insert(
        "revision".into(),
        json!(default_string(repository.get("revision")).to_lowercase()),
    );
    for field in ["provider", "link_mode"] {
        copy(&mut result, field, repository.get(field));
    }
    Some(Value::Object(result))
}
fn repository_identity(repository: &Value) -> Value {
    match parse_remote(
        repository["url"].as_str().unwrap_or_default(),
        true,
        repository["provider"].as_str(),
    ) {
        Ok(location) => json!(location.identity),
        Err(_) => repository["url"].clone(),
    }
}
fn boundary_key(boundary: &Value) -> String {
    format!(
        "{}\u{1f}{}",
        string(boundary.get("kind")),
        string(boundary.get("label"))
    )
}

pub fn canonical_architecture(diagram: &Value) -> Value {
    let mut meta = object(diagram.get("meta"));
    meta.remove("output");
    if let Some(repository) = normalize_repository(meta.get("repository")) {
        meta.insert("repository".into(), repository);
    }
    let mut components: Vec<Value> = values(diagram.get("components"))
        .iter()
        .map(|component| {
            let mut component = component.clone();
            if let Some(sources) = component.get("sources").and_then(Value::as_array) {
                component["sources"] = json!(sorted_objects(sources));
            }
            component
        })
        .collect();
    components.sort_by(|a, b| order(&string(a.get("id")), &string(b.get("id"))));
    let mut boundaries: Vec<Value> = values(diagram.get("boundaries"))
        .iter()
        .map(|boundary| {
            let mut boundary = boundary.clone();
            boundary["wraps"] = json!(sorted_values(values(boundary.get("wraps"))));
            boundary
        })
        .collect();
    boundaries.sort_by(|a, b| order(&boundary_key(a), &boundary_key(b)));
    let mut connections = values(diagram.get("connections")).to_vec();
    connections.sort_by(|a, b| order(&default_string(a.get("id")), &default_string(b.get("id"))));
    let mut result = Map::new();
    for key in ["schema_version", "diagram_type"] {
        copy(&mut result, key, diagram.get(key));
    }
    result.insert("meta".into(), Value::Object(meta));
    if truthy(diagram.get("layout")) {
        copy(&mut result, "layout", diagram.get("layout"));
    }
    result.insert("components".into(), json!(components));
    result.insert("boundaries".into(), json!(boundaries));
    result.insert("connections".into(), json!(connections));
    if truthy(diagram.get("cards")) {
        copy(&mut result, "cards", diagram.get("cards"));
    }
    Value::Object(result)
}
pub fn canonical_json(diagram: &Value) -> Vec<u8> {
    serialized(&canonical_architecture(diagram)).into_bytes()
}

fn fail(code: &str, message: String, details: Value) -> Diagnostics {
    let mut diagnostic = Diagnostic::error(
        Some("architecture"),
        code,
        details["path"].as_str().unwrap_or_default(),
        message,
    );
    diagnostic.stage = "compare".into();
    diagnostic.supported_fixes = values(details.get("supportedFixes"))
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    diagnostic.evidence = details;
    vec![diagnostic]
}
fn comparable_shape(diagram: &Value, side: &str) -> Result<(), Diagnostics> {
    for (field, expected, code, message) in [
        (
            "schema_version",
            json!(1),
            "delta/schema-version-mismatch",
            format!("{side} must use schema_version 1."),
        ),
        (
            "diagram_type",
            json!("architecture"),
            "delta/type-mismatch",
            format!("{side} must use diagram_type architecture."),
        ),
    ] {
        if !equal(diagram.get(field), Some(&expected)) {
            let mut details = json!({"side":side,"path":format!("/{field}")});
            copy(
                details.as_object_mut().unwrap(),
                "actual",
                diagram.get(field),
            );
            return Err(fail(code, message, details));
        }
    }
    Ok(())
}
type Index<'a> = BTreeMap<String, &'a Value>;
fn stable_index<'a>(
    diagram: &'a Value,
    collection: &str,
    side: &str,
    missing_code: &str,
) -> Result<Index<'a>, Diagnostics> {
    let mut index = BTreeMap::new();
    let mut missing = vec![];
    let mut duplicates = BTreeSet::new();
    for (position, item) in values(diagram.get(collection)).iter().enumerate() {
        if !truthy(item.get("id")) {
            missing.push(format!("/{collection}/{position}/id"));
        } else {
            let id = string(item.get("id"));
            match index.entry(id) {
                std::collections::btree_map::Entry::Occupied(entry) => {
                    duplicates.insert(entry.key().clone());
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(item);
                }
            }
        }
    }
    if !missing.is_empty() {
        return Err(fail(
            missing_code,
            format!("{side} {collection} require authored stable ids for comparison."),
            json!({"side":side,"paths":sorted_strings(missing),"supportedFixes":[format!("add a unique id to every {collection} item")]}),
        ));
    }
    if !duplicates.is_empty() {
        return Err(fail(
            "delta/duplicate-stable-id",
            format!("{side} {collection} contain duplicate ids."),
            json!({"side":side,"collection":collection,"ids":sorted_strings(duplicates.into_iter().collect()),"supportedFixes":[format!("make every {collection} id unique")]}),
        ));
    }
    Ok(index)
}
fn boundary_index<'a>(diagram: &'a Value, side: &str) -> Result<Index<'a>, Diagnostics> {
    let mut index = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for boundary in values(diagram.get("boundaries")) {
        let key = boundary_key(boundary);
        if let std::collections::btree_map::Entry::Vacant(entry) = index.entry(key) {
            entry.insert(boundary);
        } else {
            ambiguous.insert(format!(
                "{}:{}",
                string(boundary.get("kind")),
                string(boundary.get("label"))
            ));
        }
    }
    if !ambiguous.is_empty() {
        return Err(fail(
            "delta/boundary-key-ambiguous",
            format!("{side} boundary kind + label keys must be unique."),
            json!({"side":side,"boundaries":sorted_strings(ambiguous.into_iter().collect()),"supportedFixes":["rename one duplicate boundary or add stable boundary ids in a future schema version"]}),
        ));
    }
    Ok(index)
}

const COMPONENT_FIELDS: &[(&str, &[&str])] = &[
    (
        "semantic",
        &["type", "label", "sublabel", "tag", "brand", "icon"],
    ),
    ("evidence", &["sources"]),
    ("geometry", &["row", "col", "pos", "size"]),
];
const CONNECTION_FIELDS: &[(&str, &[&str])] = &[
    ("topology", &["from", "to"]),
    ("semantic", &["label", "variant"]),
    (
        "geometry",
        &[
            "fromSide",
            "toSide",
            "route",
            "via",
            "labelAt",
            "labelDx",
            "labelDy",
            "labelSegment",
            "width",
        ],
    ),
];
const BOUNDARY_FIELDS: &[(&str, &[&str])] = &[("scope", &["wraps"]), ("geometry", &["pad"])];
fn normalized_field(item: &Value, field: &str) -> Option<Value> {
    let value = item.get(field)?;
    Some(match (field, value.as_array()) {
        ("sources", Some(values)) => json!(sorted_objects(values)),
        ("wraps", Some(values)) => json!(sorted_values(values)),
        _ => value.clone(),
    })
}
fn describe(
    id: &str,
    before: Option<&Value>,
    after: Option<&Value>,
    kind: &str,
) -> Map<String, Value> {
    let mut result = Map::new();
    match kind {
        "component" => {
            result.insert("id".into(), json!(id));
            copy(
                &mut result,
                "baseLabel",
                before.and_then(|v| v.get("label")),
            );
            copy(&mut result, "headLabel", after.and_then(|v| v.get("label")));
        }
        "connection" => {
            result.insert("id".into(), json!(id));
            for (side, item) in [("base", before), ("head", after)] {
                if let Some(item) = item {
                    let mut detail = Map::new();
                    for field in ["from", "to"] {
                        copy(&mut detail, field, item.get(field));
                    }
                    detail.insert(
                        "label".into(),
                        if truthy(item.get("label")) {
                            item["label"].clone()
                        } else {
                            json!("")
                        },
                    );
                    result.insert(side.into(), Value::Object(detail));
                }
            }
        }
        _ => {
            let item = after.or(before).unwrap();
            result.insert(
                "key".into(),
                json!(format!(
                    "{}:{}",
                    string(item.get("kind")),
                    string(item.get("label"))
                )),
            );
            for field in ["kind", "label"] {
                copy(&mut result, field, item.get(field));
            }
        }
    }
    result
}
fn compare_entities(
    base: &Index<'_>,
    head: &Index<'_>,
    kind: &str,
    groups: &[(&str, &[&str])],
) -> Vec<Value> {
    let ids = sorted_strings(
        base.keys()
            .chain(head.keys())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    );
    let identity = match kind {
        "connection" => "topology",
        "boundary" => "scope",
        _ => "semantic",
    };
    let mut changes = vec![];
    for id in ids {
        let before = base.get(&id).copied();
        let after = head.get(&id).copied();
        let mut classifications: Vec<String> = vec![];
        let mut changed_fields = vec![];
        let status = match (before, after) {
            (None, Some(_)) => {
                classifications.push(identity.into());
                "added"
            }
            (Some(_), None) => {
                classifications.push(identity.into());
                "removed"
            }
            (Some(before), Some(after)) => {
                for (classification, fields) in groups {
                    let mut changed = false;
                    for field in *fields {
                        if !equal(
                            normalized_field(before, field).as_ref(),
                            normalized_field(after, field).as_ref(),
                        ) {
                            changed = true;
                            changed_fields.push(format!("/{field}"));
                        }
                    }
                    if changed {
                        classifications.push((*classification).into());
                    }
                }
                if classifications
                    .iter()
                    .any(|c| matches!(c.as_str(), "topology" | "semantic" | "scope"))
                {
                    "changed"
                } else if classifications.iter().any(|c| c == "evidence") {
                    "evidence-changed"
                } else if classifications.iter().any(|c| c == "geometry") {
                    match kind {
                        "connection" => "rerouted",
                        "component" => "moved",
                        _ => "geometry-changed",
                    }
                } else {
                    "same"
                }
            }
            _ => unreachable!(),
        };
        if status == "same" {
            continue;
        }
        let mut change = describe(&id, before, after, kind);
        change.insert("status".into(), json!(status));
        change.insert(
            "classifications".into(),
            json!(sorted_strings(classifications)),
        );
        change.insert(
            "changedFields".into(),
            json!(sorted_strings(changed_fields)),
        );
        changes.push(Value::Object(change));
    }
    changes
}
fn summary(changes: &[Value], shape: &[&str]) -> Value {
    let mut result: Map<String, Value> =
        shape.iter().map(|key| ((*key).into(), json!(0))).collect();
    for change in changes {
        let key = match change["status"].as_str().unwrap_or_default() {
            "evidence-changed" => "evidenceChanged",
            "geometry-changed" => "geometryChanged",
            other => other,
        };
        if let Some(count) = result.get_mut(key) {
            *count = json!(count.as_u64().unwrap() + 1);
        }
    }
    Value::Object(result)
}
fn presentation(diagram: &Value) -> Value {
    let mut result = Map::new();
    for field in [
        "title",
        "locale",
        "subtitle",
        "animation",
        "visual_preset",
        "quality_profile",
        "engineering_profile",
        "legend",
        "viewBox",
    ] {
        copy(&mut result, field, diagram["meta"].get(field));
    }
    for field in ["layout", "cards"] {
        copy(&mut result, field, diagram.get(field));
    }
    Value::Object(result)
}
fn provenance_side(repository: Option<&Value>) -> Value {
    let Some(repository) = repository else {
        return Value::Null;
    };
    let mut side = Map::new();
    copy(&mut side, "revision", repository.get("revision"));
    copy(&mut side, "provider", repository.get("provider"));
    copy(&mut side, "linkMode", repository.get("link_mode"));
    Value::Object(side)
}
fn provenance(base: Option<&Value>, head: Option<&Value>) -> Option<Value> {
    if equal(base, head) {
        return None;
    }
    let fields = match (base, head) {
        (Some(base), Some(head)) => ["url", "revision", "provider", "link_mode"]
            .into_iter()
            .filter(|field| !equal(base.get(field), head.get(field)))
            .map(|field| format!("/{field}"))
            .collect(),
        _ => vec!["/repository".into()],
    };
    Some(
        json!({"changedFields":sorted_strings(fields),"base":provenance_side(base),"head":provenance_side(head)}),
    )
}
fn side(diagram: &Value, repository: Option<&Value>, evidence: &Value, prefix: &str) -> Value {
    let mut result = Map::new();
    result.insert(
        "title".into(),
        if truthy(diagram["meta"].get("title")) {
            diagram["meta"]["title"].clone()
        } else {
            json!("")
        },
    );
    for (field, key) in [
        ("RawSha256", "rawSha256"),
        ("SemanticSha256", "semanticSha256"),
    ] {
        let value = evidence.get(format!("{prefix}{field}"));
        if truthy(value) {
            copy(&mut result, key, value);
        }
    }
    if let Some(value) = evidence.get(format!("{prefix}Bytes")).filter(|value| {
        value
            .as_f64()
            .is_some_and(|number| number.is_finite() && number.fract() == 0.0)
    }) {
        copy(&mut result, "bytes", Some(value));
    }
    if let Some(repository) = repository {
        if truthy(repository.get("revision")) {
            copy(&mut result, "revision", repository.get("revision"));
        }
    }
    Value::Object(result)
}
fn pinned(repository: Option<&Value>) -> bool {
    repository
        .and_then(|v| v["revision"].as_str())
        .is_some_and(|revision| {
            revision.len() == 40
                && revision
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

pub fn compare_architecture(
    base: &Value,
    head: &Value,
    evidence: &Value,
) -> Result<Value, Diagnostics> {
    comparable_shape(base, "base")?;
    comparable_shape(head, "head")?;
    let base_components = stable_index(base, "components", "base", "delta/stable-id-required")?;
    let head_components = stable_index(head, "components", "head", "delta/stable-id-required")?;
    if !base_components
        .keys()
        .any(|id| head_components.contains_key(id))
    {
        return Err(fail("delta/no-shared-component-id","The snapshots share no component id, so Archify cannot prove that they describe the same system.".into(),json!({"supportedFixes":["preserve at least one authored component id across snapshots"]})));
    }
    let base_connections = stable_index(
        base,
        "connections",
        "base",
        "delta/relationship-id-required",
    )?;
    let head_connections = stable_index(
        head,
        "connections",
        "head",
        "delta/relationship-id-required",
    )?;
    let base_boundaries = boundary_index(base, "base")?;
    let head_boundaries = boundary_index(head, "head")?;
    let base_repository = normalize_repository(base["meta"].get("repository"));
    let head_repository = normalize_repository(head["meta"].get("repository"));
    if let (Some(base), Some(head)) = (&base_repository, &head_repository) {
        if repository_identity(base) != repository_identity(head) {
            return Err(fail(
                "delta/repository-mismatch",
                "The snapshots name different repositories.".into(),
                json!({"baseRepository":base["url"],"headRepository":head["url"],"supportedFixes":["compare snapshots from the same repository or remove repository evidence from both inputs"]}),
            ));
        }
    }
    let proof_level = if pinned(base_repository.as_ref())
        && pinned(head_repository.as_ref())
        && truthy(evidence.get("baseVerified"))
        && truthy(evidence.get("headVerified"))
    {
        "revision-pinned"
    } else {
        "authored"
    };
    let components = compare_entities(
        &base_components,
        &head_components,
        "component",
        COMPONENT_FIELDS,
    );
    let connections = compare_entities(
        &base_connections,
        &head_connections,
        "connection",
        CONNECTION_FIELDS,
    );
    let boundaries = compare_entities(
        &base_boundaries,
        &head_boundaries,
        "boundary",
        BOUNDARY_FIELDS,
    );
    let provenance = provenance(base_repository.as_ref(), head_repository.as_ref());
    let mut result = json!({
        "schemaVersion":1,"ok":true,"command":"compare","type":"architecture","comparatorVersion":1,"canonicalVersion":1,"completeness":"complete","proofLevel":proof_level,
        "base":side(base,base_repository.as_ref(),evidence,"base"),"head":side(head,head_repository.as_ref(),evidence,"head"),
        "summary":{"components":summary(&components,&["added","changed","evidenceChanged","removed","moved"]),"connections":summary(&connections,&["added","changed","removed","rerouted"]),"boundaries":summary(&boundaries,&["added","changed","removed","geometryChanged"]),"presentationChanged":!equal(Some(&presentation(base)),Some(&presentation(head))),"provenanceChanged":provenance.is_some()},
        "changes":{"components":components,"connections":connections,"boundaries":boundaries},
        "identity":{"components":"components[].id","connections":"connections[].id (required)","boundaries":"boundaries[].kind + boundaries[].label (derived)"},
        "view":{"visualPreset":if truthy(head["meta"].get("visual_preset")){head["meta"]["visual_preset"].clone()}else{json!("classic")}},
        "limitations":["Authored Architecture IR only; no runtime impact, causality, risk, or mergeability is inferred.","Boundary identity is conservatively derived from kind + label."]
    });
    if let Some(provenance) = provenance {
        result["provenance"] = provenance;
    }
    Ok(result)
}
