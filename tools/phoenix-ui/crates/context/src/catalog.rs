//! JS: lib/concept-catalog.mjs + lib/composition-catalog.mjs (the parts the
//! seeder uses: read + merge, validate concept catalog, pool revision).

use crate::util::{js_trim, safe_read, utf16_len};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

pub const WELL_TIERS: [&str; 3] = ["graphic", "interaction", "atmosphere"];
pub const SYSTEM_PREFIXES: [&str; 5] =
    ["Palette/material:", "Type/composition:", "Topology/navigation:", "Controls/state:", "Responsive/motion:"];
const CONCEPT_STRENGTHS: [&str; 3] = ["world", "composition", "dual"];
const CONCEPT_STATUSES: [&str; 2] = ["approved", "rejected"];
const CONCEPT_BREADTHS: [&str; 2] = ["general", "niche"];
pub const SEED_MODES: [&str; 4] = ["persuade", "operate", "read", "experience"];

fn local_asset_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn validate_original_source(entry: &Value) -> Vec<String> {
    let id = s(entry.get("id")).unwrap_or("(unknown)");
    let mut errors = Vec::new();
    for key in ["applicability", "thesis"] {
        if !s(entry.get(key)).is_some_and(|text| !js_trim(text).is_empty()) {
            errors.push(format!("entry {id} needs {key}"));
        }
    }
    let source = entry.get("source");
    for key in ["kind", "title", "license"] {
        if !source
            .and_then(|value| value.get(key))
            .and_then(Value::as_str)
            .is_some_and(|text| !js_trim(text).is_empty())
        {
            errors.push(format!("entry {id} needs source.{key}"));
        }
    }
    errors
}

static ID_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap());
static WRAPPER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?-u:\b)(?:live digital system|shared participatory system) modeled on(?-u:\b)").unwrap());
static IMITATION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(?-u:\b)(?:in the style of|styled like|copy of)(?-u:\b)").unwrap());

pub fn sha256_hex(input: &str) -> String {
    let d = Sha256::digest(input.as_bytes());
    d.iter().map(|b| format!("{:02x}", b)).collect()
}

fn s(v: Option<&Value>) -> Option<&str> {
    v.and_then(|x| x.as_str())
}

fn str_or_empty(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => crate::critique_storage::js_string_value(other),
    }
}

/// `x ?? ''` for hashing payloads: undefined/null -> '', else String(x)
fn nullish_or_string(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(x)) => x.clone(),
        Some(other) => crate::critique_storage::js_string_value(other),
    }
}

/// `JSON.stringify(x ?? [])`
fn json_or_empty_array(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "[]".to_string(),
        Some(x) => serde_json::to_string(x).unwrap_or_else(|_| "null".into()),
    }
}

/// JS: normalizeConceptForm
pub fn normalize_concept_form(value: Option<&Value>) -> String {
    let raw = str_or_empty(value);
    let nfkd: String = raw.nfkd().collect();
    let lower = nfkd.to_lowercase().replace(['\u{2019}', '\u{2018}'], "'");
    let mut out = String::new();
    let mut in_bad = false;
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            in_bad = false;
            out.push(c);
        } else if !in_bad {
            out.push(' ');
            in_bad = true;
        }
    }
    js_trim(&out).to_string()
}

fn trimmed_len(v: Option<&Value>) -> Option<usize> {
    s(v).map(|x| utf16_len(js_trim(x)))
}

fn string_len_between(v: Option<&Value>, min: usize, max: usize) -> bool {
    matches!(trimmed_len(v), Some(n) if n >= min && n <= max)
}

fn is_string_array_of(v: Option<&Value>, len: usize) -> bool {
    match v.and_then(|x| x.as_array()) {
        Some(a) => a.len() == len && a.iter().all(|t| t.as_str().map(|x| !js_trim(x).is_empty()).unwrap_or(false)),
        None => false,
    }
}

/// JS: validateConceptEntry (error strings; only emptiness matters to the seeder)
pub fn validate_concept_entry(concept: &Value, existing_forms: &HashMap<String, String>) -> Vec<String> {
    let mut errors = Vec::new();
    let id = s(concept.get("id")).filter(|x| !x.is_empty()).unwrap_or("(unknown)").to_string();
    if let Some(axes) = concept.get("axes") {
        if !axes.is_null() && !axes.is_object() {
            errors.push(format!("concept {} axes must be an object of axis id to value id", id));
        }
    }
    if !ID_RE.is_match(s(concept.get("id")).unwrap_or("")) {
        errors.push(format!("invalid concept id: {}", nullish_or_string(concept.get("id"))));
    }
    let normalized = normalize_concept_form(concept.get("form"));
    if normalized.is_empty() {
        errors.push(format!("concept {} needs a form", id));
    } else if let Some(other) = existing_forms.get(&normalized) {
        errors.push(format!("duplicate concept form: {} and {}", id, other));
    }
    let form = s(concept.get("form"));
    if !string_len_between(concept.get("form"), 40, 360) || !form.map(|f| f.contains(',')).unwrap_or(false) {
        errors.push(format!("concept {} must name a form and inherited structure after a comma", id));
    }
    if !string_len_between(concept.get("lineage"), 12, 200) {
        errors.push(format!("concept {} needs specific lineage metadata of 12–200 characters", id));
    }
    if !s(concept.get("strength")).map(|x| CONCEPT_STRENGTHS.contains(&x)).unwrap_or(false) {
        errors.push(format!("concept {} needs a strength", id));
    }
    if !is_string_array_of(concept.get("tags"), 3) {
        errors.push(format!("concept {} must have exactly three structural tags", id));
    }
    if let Some(avoid) = concept.get("avoid") {
        let ok = avoid
            .as_array()
            .map(|a| a.len() >= 2 && a.len() <= 3 && a.iter().all(|i| string_len_between(Some(i), 12, 160)))
            .unwrap_or(false);
        if !ok {
            errors.push(format!("concept {} avoid must be two or three negations of 12–160 characters", id));
        }
    }
    let system_ok = concept
        .get("system")
        .and_then(|x| x.as_array())
        .map(|a| a.len() == SYSTEM_PREFIXES.len() && a.iter().all(|r| string_len_between(Some(r), 12, 180)))
        .unwrap_or(false);
    if !system_ok {
        errors.push(format!("concept {} needs system grammar with exactly five rules of 12–180 characters", id));
    } else {
        let rules = concept.get("system").unwrap().as_array().unwrap();
        let unique: HashSet<String> = rules.iter().map(|r| normalize_concept_form(Some(r))).collect();
        if unique.len() != SYSTEM_PREFIXES.len() {
            errors.push(format!("concept {} has duplicate system grammar rules", id));
        }
        if rules.iter().enumerate().any(|(i, r)| !r.as_str().unwrap_or("").starts_with(SYSTEM_PREFIXES[i])) {
            errors.push(format!("concept {} system grammar must use palette, type, topology, controls, and responsive prefixes in order", id));
        }
    }
    if !string_len_between(concept.get("spark"), 80, 320) {
        errors.push(format!("concept {} needs a vivid creative spark of 80–320 characters", id));
    }
    if !string_len_between(concept.get("webLeverage"), 20, 240) {
        errors.push(format!("concept {} needs web leverage of 20–240 characters", id));
    }
    let form_s = form.unwrap_or("");
    if WRAPPER_RE.is_match(form_s) {
        errors.push(format!("concept {} is a generic wrapper around another artifact", id));
    }
    if IMITATION_RE.is_match(form_s) {
        errors.push(format!("concept {} contains imitation language", id));
    }
    errors
}

/// JS: conceptContentHash
pub fn concept_content_hash(c: &Value) -> String {
    let payload = [
        nullish_or_string(c.get("form")),
        nullish_or_string(c.get("lineage")),
        json_or_empty_array(c.get("tags")),
        json_or_empty_array(c.get("system")),
        nullish_or_string(c.get("spark")),
        nullish_or_string(c.get("webLeverage")),
    ]
    .join("\n");
    sha256_hex(&payload)[..12].to_string()
}

pub struct ConceptCatalog {
    pub catalog: Value,
    pub review_data: Value,
    pub concepts: Vec<Value>,
}

fn obj_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// JS: readConceptCatalog
pub fn read_concept_catalog(catalog_path: &str, reviews_path: &str) -> Option<ConceptCatalog> {
    let catalog: Value = serde_json::from_str(&safe_read(catalog_path)?).ok()?;
    let review_data: Value = serde_json::from_str(&safe_read(reviews_path)?).ok()?;
    concept_catalog_from_values(catalog, review_data)
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

fn is_int(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    if let Some(i) = v.as_i64() {
        return Some(i);
    }
    let f = v.as_f64()?;
    if f.fract() == 0.0 && f.is_finite() {
        Some(f as i64)
    } else {
        None
    }
}

/// JS: validateConceptCatalog(catalog, reviewData) with defaults -> errors
pub fn validate_concept_catalog(catalog: &Value, review_data: &Value) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let mut family_ids: HashSet<String> = HashSet::new();
    let mut concept_ids: HashSet<String> = HashSet::new();
    let mut normalized_forms: HashMap<String, String> = HashMap::new();
    let mut concepts: Vec<Value> = Vec::new();

    if !matches!(is_int(obj_get(catalog, "schemaVersion")), Some(v) if v >= 7) {
        errors.push("catalog.schemaVersion must be 7 or newer".into());
    }
    if !s(obj_get(catalog, "catalogVersion")).map(|x| !js_trim(x).is_empty()).unwrap_or(false) {
        errors.push("catalog.catalogVersion must be a non-empty string".into());
    }
    let qb = obj_get(catalog, "qualityBar");
    if !string_len_between(qb.and_then(|q| q.get("principle")), 80, usize::MAX) {
        errors.push("catalog.qualityBar.principle must define the universal creative bar".into());
    }
    if qb
        .and_then(|q| q.get("rejectIf"))
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
        < 5
    {
        errors.push("catalog.qualityBar.rejectIf must define at least five rejection gates".into());
    }
    if qb
        .and_then(|q| q.get("reviewAxes"))
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
        < 8
    {
        errors.push("catalog.qualityBar.reviewAxes must define at least eight review axes".into());
    }
    let families: Vec<Value> = obj_get(catalog, "families")
        .and_then(|f| f.as_array())
        .cloned()
        .unwrap_or_default();
    if obj_get(catalog, "families")
        .and_then(Value::as_array)
        .is_none()
    {
        errors.push("catalog.families must be an array".into());
    }
    let wells: Vec<Value> = obj_get(catalog, "wells")
        .and_then(|w| w.as_array())
        .cloned()
        .unwrap_or_default();
    let mut well_ids: HashSet<String> = HashSet::new();
    if obj_get(catalog, "wells")
        .and_then(Value::as_array)
        .is_none()
    {
        errors.push("catalog.wells must be an array".into());
    }
    for well in &wells {
        let wid = s(well.get("id")).unwrap_or("");
        if !ID_RE.is_match(wid) {
            errors.push(format!("invalid well id: {}", wid));
        } else if well_ids.contains(wid) {
            errors.push(format!("duplicate well id: {}", wid));
        }
        well_ids.insert(wid.to_string());
        if !s(well.get("label")).map(|x| !js_trim(x).is_empty()).unwrap_or(false) {
            errors.push("well needs a label".into());
        }
        if !string_len_between(well.get("description"), 40, usize::MAX) {
            errors.push("well needs a description".into());
        }
        if !s(well.get("tier")).map(|t| WELL_TIERS.contains(&t)).unwrap_or(false) {
            errors.push("well needs a tier".into());
        }
    }
    for family in &families {
        let fid = s(family.get("id")).unwrap_or("");
        if !ID_RE.is_match(fid) {
            errors.push(format!("invalid family id: {}", fid));
        } else if family_ids.contains(fid) {
            errors.push(format!("duplicate family id: {}", fid));
        }
        family_ids.insert(fid.to_string());
        if !s(family.get("label")).map(|x| !js_trim(x).is_empty()).unwrap_or(false) {
            errors.push("family needs a label".into());
        }
        let well = s(family.get("well")).unwrap_or("");
        if !well_ids.contains(well) || family.get("well").map(|w| !w.is_string()).unwrap_or(true) {
            errors.push("family must belong to a declared well".into());
        }
        let Some(fc) = family.get("concepts").and_then(|c| c.as_array()) else {
            errors.push("family concepts must be an array".into());
            continue;
        };
        for concept in fc {
            concepts.push(concept.clone());
            let cid = s(concept.get("id")).unwrap_or("").to_string();
            if concept_ids.contains(&cid) {
                errors.push(format!("duplicate concept id: {}", cid));
            }
            errors.extend(validate_concept_entry(concept, &normalized_forms));
            if is_int(obj_get(catalog, "schemaVersion")).unwrap_or(0) >= 8 {
                errors.extend(validate_original_source(concept));
            }
            for key in ["cardBoard", "cardHero"] {
                if let Some(image) = concept.get(key) {
                    if !image.as_str().is_some_and(local_asset_path) {
                        errors.push(format!(
                            "concept {cid} {key} must be a relative local asset path"
                        ));
                    }
                }
            }
            concept_ids.insert(cid.clone());
            let normalized = normalize_concept_form(concept.get("form"));
            if !normalized.is_empty() {
                normalized_forms.insert(normalized, cid);
            }
        }
    }
    if !matches!(is_int(obj_get(review_data, "schemaVersion")), Some(v) if v >= 2) {
        errors.push("reviews.schemaVersion must be 2 or newer".into());
    }
    let concepts_by_id: HashMap<String, &Value> = concepts
        .iter()
        .filter_map(|c| s(c.get("id")).map(|i| (i.to_string(), c)))
        .collect();
    if !obj_get(review_data, "reviews").is_some_and(Value::is_object) {
        errors.push("reviews.reviews must be an object".into());
    }
    let reviews: Map<String, Value> = obj_get(review_data, "reviews")
        .and_then(|r| r.as_object())
        .cloned()
        .unwrap_or_default();
    for (id, review) in &reviews {
        if !concept_ids.contains(id) {
            errors.push(format!("review references missing concept: {}", id));
        }
        if !s(review.get("status")).map(|x| CONCEPT_STATUSES.contains(&x)).unwrap_or(false) {
            errors.push(format!("invalid review status for {}", id));
        }
        if !s(review.get("reviewedBy")).map(|x| !js_trim(x).is_empty()).unwrap_or(false) {
            errors.push(format!("review {} needs reviewedBy", id));
        }
        match s(review.get("reviewedAt")) {
            Some(x) if js_date_parse_ok(x) => {}
            _ => errors.push(format!("review {} needs an ISO reviewedAt timestamp", id)),
        }
        match s(review.get("formHash")).filter(|x| !js_trim(x).is_empty()) {
            None => errors.push(format!("review {} needs a formHash of the reviewed content", id)),
            Some(h) => {
                if let Some(c) = concepts_by_id.get(id) {
                    if h != concept_content_hash(c) {
                        errors.push(format!("review {} is stale", id));
                    }
                }
            }
        }
        if let Some(note) = review.get("note") {
            let ok = note.as_str().map(|n| !js_trim(n).is_empty() && utf16_len(n) <= 500).unwrap_or(false);
            if !ok {
                errors.push(format!("review {} note must be a non-empty string of 500 characters or fewer", id));
            }
        }
        if let Some(rating) = review.get("rating") {
            let r = rating.as_f64();
            if !matches!(r, Some(x) if x == 1.0 || x == 2.0 || x == 3.0) {
                errors.push(format!("review {} rating must be 1, 2, or 3", id));
            } else if s(review.get("status")) != Some("approved") {
                errors.push(format!("review {} rating only applies to approved concepts", id));
            }
        }
        if let Some(b) = review.get("breadth") {
            if !b.as_str().map(|x| CONCEPT_BREADTHS.contains(&x)).unwrap_or(false) {
                errors.push(format!("review {} breadth must be one of general, niche", id));
            }
        }
        if let Some(am) = review.get("allowedModes") {
            match am.as_array() {
                None => errors.push(format!("review {} allowedModes must be a non-empty array", id)),
                Some(a) if a.is_empty() => errors.push(format!("review {} allowedModes must be a non-empty array", id)),
                Some(a) => {
                    if a.iter().any(|m| !m.as_str().map(|x| SEED_MODES.contains(&x)).unwrap_or(false)) {
                        errors.push(format!("review {} allowedModes may only contain modes", id));
                    } else {
                        let set: HashSet<String> = a.iter().map(|m| serde_json::to_string(m).unwrap()).collect();
                        if set.len() != a.len() {
                            errors.push(format!("review {} allowedModes must not repeat a mode", id));
                        } else if a.len() == SEED_MODES.len() {
                            errors.push(format!("review {} allowedModes lists every mode; omit the field instead", id));
                        }
                    }
                }
            }
        }
    }
    errors
}

/// Catalog reviews use calendar-valid ISO dates or timestamps.
fn js_date_parse_ok(x: &str) -> bool {
    static DATE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^([0-9]{4})-([0-9]{2})-([0-9]{2})(?:T([0-9]{2}):([0-9]{2})(?::([0-9]{2})(?:\.[0-9]+)?)?(?:Z|[+-]([0-9]{2}):([0-9]{2}))?)?$").unwrap()
    });
    let Some(parts) = DATE.captures(js_trim(x)) else {
        return false;
    };
    let number = |i| {
        parts
            .get(i)
            .map(|part| part.as_str().parse::<u32>().unwrap())
            .unwrap_or(0)
    };
    let (year, month, day) = (number(1), number(2), number(3));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1
        && day <= days
        && number(4) <= 23
        && number(5) <= 59
        && number(6) <= 59
        && number(7) <= 23
        && number(8) <= 59
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

pub fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

pub struct CompositionCatalog {
    pub compositions: Vec<Value>,
}

pub fn composition_content_hash(c: &Value) -> String {
    let payload = [
        nullish_or_string(c.get("form")),
        nullish_or_string(c.get("lineage")),
        json_or_empty_array(c.get("tags")),
        json_or_empty_array(c.get("grammar")),
        nullish_or_string(c.get("spark")),
        nullish_or_string(c.get("webLeverage")),
    ]
    .join("\n");
    sha256_hex(&payload)[..12].to_string()
}

pub fn validate_composition_catalog(catalog: &Value, review_data: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let entries = obj_get(catalog, "compositions").and_then(Value::as_array);
    if is_int(obj_get(catalog, "schemaVersion")).is_none_or(|version| version < 1)
        || !s(obj_get(catalog, "catalogVersion"))
            .is_some_and(|version| !js_trim(version).is_empty())
        || obj_get(catalog, "families")
            .and_then(Value::as_array)
            .is_none()
        || entries.is_none()
        || obj_get(review_data, "reviews")
            .and_then(Value::as_object)
            .is_none()
        || is_int(obj_get(review_data, "schemaVersion")).is_none_or(|version| version < 2)
    {
        return vec!["composition catalog needs schemaVersion, catalogVersion, families, compositions and reviews".into()];
    }
    let mut families = HashSet::new();
    for family in obj_get(catalog, "families")
        .and_then(Value::as_array)
        .unwrap()
    {
        let id = s(family.get("id")).unwrap_or("(unknown)");
        if !ID_RE.is_match(id)
            || !families.insert(id)
            || !s(family.get("label")).is_some_and(|label| !js_trim(label).is_empty())
        {
            errors.push(format!("invalid or duplicate composition family: {id}"));
        }
    }
    let mut ids = HashSet::new();
    for entry in entries.unwrap() {
        let id = s(entry.get("id")).unwrap_or("(unknown)");
        if !ID_RE.is_match(id) || !ids.insert(id) {
            errors.push(format!("invalid or duplicate composition id: {id}"));
        }
        if !s(entry.get("familyId")).is_some_and(|family| families.contains(family)) {
            errors.push(format!("composition {id} needs a declared family"));
        }
        if !s(entry.get("surface")).is_some_and(|mode| SEED_MODES.contains(&mode)) {
            errors.push(format!("composition {id} needs a valid surface mode"));
        }
        for key in ["form", "lineage", "spark", "webLeverage"] {
            if !s(entry.get(key)).is_some_and(|text| !js_trim(text).is_empty()) {
                errors.push(format!("composition {id} needs {key}"));
            }
        }
        let prefixes = [
            "Staging/hierarchy:",
            "Sequence/attention:",
            "Controls/state:",
            "Adaptation:",
        ];
        if !entry
            .get("grammar")
            .and_then(Value::as_array)
            .is_some_and(|rules| {
                rules.len() == prefixes.len()
                    && rules.iter().zip(prefixes).all(|(rule, prefix)| {
                        rule.as_str()
                            .is_some_and(|text| text.starts_with(prefix) && utf16_len(text) >= 12)
                    })
            })
        {
            errors.push(format!(
                "composition {id} needs the four ordered grammar prefixes"
            ));
        }
        if is_int(obj_get(catalog, "schemaVersion")).unwrap_or(0) >= 2 {
            errors.extend(validate_original_source(entry));
        }
    }
    for (id, review) in obj_get(review_data, "reviews")
        .and_then(Value::as_object)
        .unwrap()
    {
        if !ids.contains(id.as_str())
            || !s(review.get("status")).is_some_and(|status| CONCEPT_STATUSES.contains(&status))
        {
            errors.push(format!("invalid composition review: {id}"));
        }
        if is_int(obj_get(catalog, "schemaVersion")).unwrap_or(0) >= 2 {
            if !s(review.get("reviewedBy")).is_some_and(|text| !js_trim(text).is_empty()) {
                errors.push(format!("composition review {id} needs reviewedBy"));
            }
            if !s(review.get("reviewedAt")).is_some_and(js_date_parse_ok) {
                errors.push(format!(
                    "composition review {id} needs an ISO reviewedAt timestamp"
                ));
            }
            let entry = entries
                .unwrap()
                .iter()
                .find(|entry| s(entry.get("id")) == Some(id));
            if !entry.is_some_and(|entry| {
                s(review.get("formHash")) == Some(composition_content_hash(entry).as_str())
            }) {
                errors.push(format!("composition review {id} needs a current formHash"));
            }
            if review.get("note").is_some_and(|note| {
                !note
                    .as_str()
                    .is_some_and(|text| !js_trim(text).is_empty() && utf16_len(text) <= 500)
            }) {
                errors.push(format!("composition review {id} note must be a non-empty string of 500 characters or fewer"));
            }
            if review.get("rating").is_some_and(|rating| !matches!(rating.as_f64(), Some(value) if value == 1.0 || value == 2.0 || value == 3.0) || s(review.get("status")) != Some("approved")) {
                errors.push(format!("composition review {id} rating must be 1, 2, or 3 and applies only to approved entries"));
            }
            if review.get("breadth").is_some_and(|breadth| {
                !breadth
                    .as_str()
                    .is_some_and(|value| CONCEPT_BREADTHS.contains(&value))
            }) {
                errors.push(format!(
                    "composition review {id} breadth must be general or niche"
                ));
            }
            if review.get("allowedModes").is_some_and(|modes| {
                !modes.as_array().is_some_and(|values| {
                    !values.is_empty()
                        && values.iter().all(|mode| {
                            mode.as_str()
                                .is_some_and(|value| SEED_MODES.contains(&value))
                        })
                        && entry.is_some_and(|entry| {
                            values.iter().any(|mode| mode == &entry["surface"])
                        })
                })
            }) {
                errors.push(format!(
                    "composition review {id} allowedModes must include its surface mode"
                ));
            }
        }
    }
    errors
}

/// JS: readCompositionCatalog
pub fn read_composition_catalog(catalog_path: &str, reviews_path: &str) -> Option<CompositionCatalog> {
    let catalog: Value = serde_json::from_str(&safe_read(catalog_path)?).ok()?;
    let review_data: Value = serde_json::from_str(&safe_read(reviews_path)?).ok()?;
    composition_catalog_from_values(catalog, review_data)
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
