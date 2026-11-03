//! The concept and composition catalogs the seeder deals from: merge each
//! entry with its family, well and review, and hash the approved pool. The
//! catalogs ship with the skill; `validation` (tests only) guards their shape.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const CONCEPTS_JSON: &str = include_str!(
    "../../../../../skills/phoenix-ui/src/scripts/data/catalog/concept-ingredients.json"
);
pub const CONCEPT_REVIEWS_JSON: &str =
    include_str!("../../../../../skills/phoenix-ui/src/scripts/data/catalog/concept-reviews.json");
pub const COMPOSITIONS_JSON: &str = include_str!(
    "../../../../../skills/phoenix-ui/src/scripts/data/catalog/composition-ingredients.json"
);
pub const COMPOSITION_REVIEWS_JSON: &str = include_str!(
    "../../../../../skills/phoenix-ui/src/scripts/data/catalog/composition-reviews.json"
);

pub const WELL_TIERS: [&str; 3] = ["graphic", "interaction", "atmosphere"];

pub const SEED_MODES: [&str; 4] = ["persuade", "operate", "read", "experience"];

pub fn sha256_hex(input: &str) -> String {
    let d = Sha256::digest(input.as_bytes());
    d.iter().map(|b| format!("{:02x}", b)).collect()
}

fn s(v: Option<&Value>) -> Option<&str> {
    v.and_then(|x| x.as_str())
}

pub struct ConceptCatalog {
    pub catalog: Value,
    pub review_data: Value,
    pub concepts: Vec<Value>,
}

fn obj_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

pub fn concept_catalog_from_values(catalog: Value, review_data: Value) -> Option<ConceptCatalog> {
    // reviewData.reviews || {}  (throws if reviewData is null -> caught -> None)
    if review_data.is_null() {
        return None;
    }
    let reviews = obj_get(&review_data, "reviews").cloned().filter(|r| crate::staleness::js_truthy(r)).unwrap_or(Value::Object(Map::new()));
    let wells: HashMap<String, Value> = obj_get(&catalog, "wells")
        .and_then(|w| w.as_array())
        .map(|a| a.iter().filter_map(|w| s(w.get("id")).map(|id| (id.to_string(), w.clone()))).collect())
        .unwrap_or_default();
    let mut concepts = Vec::new();
    for family in obj_get(&catalog, "families").and_then(|f| f.as_array()).cloned().unwrap_or_default() {
        for concept in family.get("concepts").and_then(|c| c.as_array()).cloned().unwrap_or_default() {
            let mut m = concept.as_object().cloned().unwrap_or_default();
            let well_key = family.get("well").and_then(|w| w.as_str()).unwrap_or("");
            let well = wells.get(well_key);
            let cid = s(concept.get("id")).unwrap_or("");
            let review = reviews.get(cid).cloned().filter(|r| !r.is_null());
            m.insert("familyId".into(), family.get("id").cloned().unwrap_or(Value::Null));
            m.insert("familyLabel".into(), family.get("label").cloned().unwrap_or(Value::Null));
            m.insert("wellId".into(), family.get("well").cloned().filter(|v| crate::staleness::js_truthy(v)).unwrap_or(Value::Null));
            m.insert("wellLabel".into(), well.and_then(|w| w.get("label")).cloned().filter(|v| crate::staleness::js_truthy(v)).unwrap_or(Value::Null));
            m.insert("wellTier".into(), well.and_then(|w| w.get("tier")).cloned().filter(|v| crate::staleness::js_truthy(v)).unwrap_or(Value::Null));
            m.insert(
                "status".into(),
                review.as_ref().and_then(|r| r.get("status")).cloned().filter(|v| crate::staleness::js_truthy(v)).unwrap_or(Value::String("pending".into())),
            );
            m.insert("review".into(), review.unwrap_or(Value::Null));
            concepts.push(Value::Object(m));
        }
    }
    Some(ConceptCatalog { catalog, review_data, concepts })
}

/// JS: approvedPoolRevision(concepts)
pub fn approved_pool_revision(concepts: &[Value]) -> String {
    let mut lines: Vec<String> = concepts
        .iter()
        .filter(|c| s(c.get("status")) == Some("approved"))
        .map(|c| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                str_or_empty_undefined(c.get("familyId")),
                str_or_empty_undefined(c.get("id")),
                str_or_empty_undefined(c.get("strength")),
                str_or_empty_undefined(c.get("form")),
                str_or_empty_undefined(c.get("spark")),
                match c.get("system") {
                    None => "undefined".to_string(),
                    Some(v) => serde_json::to_string(v).unwrap_or_default(),
                },
                str_or_empty_undefined(c.get("webLeverage")),
            )
        })
        .collect();
    // JS default sort: by UTF-16 code units
    lines.sort_by(|a, b| utf16_cmp(a, b));
    sha256_hex(&lines.join("\n"))[..12].to_string()
}

/// Template-literal coercion: undefined -> "undefined", null -> "null".
fn str_or_empty_undefined(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(x)) => x.clone(),
        Some(other) => crate::critique_storage::js_string_value(other),
    }
}

fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

pub struct CompositionCatalog {
    pub compositions: Vec<Value>,
}

pub fn composition_catalog_from_values(
    catalog: Value,
    review_data: Value,
) -> Option<CompositionCatalog> {
    if review_data.is_null() || catalog.is_null() {
        return None;
    }
    let reviews = obj_get(&review_data, "reviews").cloned().filter(|r| crate::staleness::js_truthy(r)).unwrap_or(Value::Object(Map::new()));
    let families: HashMap<String, Value> = obj_get(&catalog, "families")
        .and_then(|f| f.as_array())
        .map(|a| a.iter().filter_map(|f| s(f.get("id")).map(|id| (id.to_string(), f.clone()))).collect())
        .unwrap_or_default();
    let compositions = obj_get(&catalog, "compositions")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|c| {
            let mut m = c.as_object().cloned().unwrap_or_default();
            let fam = families.get(s(c.get("familyId")).unwrap_or(""));
            let cid = s(c.get("id")).unwrap_or("");
            let review = reviews.get(cid).cloned().filter(|r| !r.is_null());
            m.insert("familyLabel".into(), fam.and_then(|f| f.get("label")).cloned().filter(|v| crate::staleness::js_truthy(v)).unwrap_or(Value::Null));
            m.insert(
                "status".into(),
                review.as_ref().and_then(|r| r.get("status")).cloned().filter(|v| crate::staleness::js_truthy(v)).unwrap_or(Value::String("pending".into())),
            );
            m.insert("review".into(), review.unwrap_or(Value::Null));
            Value::Object(m)
        })
        .collect();
    Some(CompositionCatalog { compositions })
}

#[cfg(test)]
mod validation;
