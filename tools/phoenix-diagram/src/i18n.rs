// Catalog selection ported from the frozen Archify 3.1.0 renderers/shared/i18n.mjs.
// The assembler writes the one normalized catalog file the Reader also embeds.
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

const DEFAULT_LOCALE: &str = "en";

type Catalogs = Map<String, Value>;

fn catalogs() -> &'static Catalogs {
    static CATALOGS: OnceLock<Catalogs> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/catalogs.json"))
            .expect("assembled catalogs are JSON objects")
    })
}

fn english() -> &'static Map<String, Value> {
    catalogs()[DEFAULT_LOCALE]
        .as_object()
        .expect("the English source catalog is bundled")
}

fn placeholders(message: &str) -> BTreeSet<&str> {
    let mut result = BTreeSet::new();
    let mut rest = message;
    while let Some(start) = rest.find('{') {
        rest = &rest[start + 1..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if end > 0 && rest[end..].starts_with('}') {
            result.insert(&rest[..end]);
        }
    }
    result
}

/// The interface language of one document.
#[derive(Clone, Debug)]
pub struct Locale {
    /// The authored `meta.locale`, if any.
    pub requested: Option<String>,
    /// The tag written to `lang`: the bundled tag, the authored tag when only
    /// overrides support it, or `en`.
    pub resolved: String,
    /// True when an authored tag has neither a bundled catalog nor a usable override.
    pub fallback: bool,
    messages: HashMap<String, String>,
}

/// Explicit `meta.translations` → bundled catalog → English. An override whose
/// placeholders differ from the English source is ignored; translations
/// without a `meta.locale` are not registered.
pub fn resolve(meta: &Value) -> Locale {
    let requested = meta["locale"].as_str().map(str::to_owned);
    let english = english();
    let bundled = requested.as_deref().and_then(|tag| {
        catalogs()
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(tag))
    });
    let overrides: HashMap<&str, &str> = match (&requested, meta["translations"].as_object()) {
        (Some(_), Some(translations)) => translations
            .iter()
            .filter_map(|(key, value)| {
                let (value, source) = (value.as_str()?, english.get(key)?.as_str()?);
                (!value.is_empty() && placeholders(value) == placeholders(source))
                    .then_some((key.as_str(), value))
            })
            .collect(),
        _ => HashMap::new(),
    };
    let fallback = requested.is_some() && bundled.is_none() && overrides.is_empty();
    let resolved = match (&requested, bundled) {
        (Some(_), Some((tag, _))) => tag.clone(),
        (Some(tag), None) if !fallback => tag.clone(),
        _ => DEFAULT_LOCALE.into(),
    };
    let messages = english
        .iter()
        .map(|(key, source)| {
            let message = overrides
                .get(key.as_str())
                .copied()
                .or_else(|| bundled.and_then(|(_, catalog)| catalog[key].as_str()))
                .or_else(|| source.as_str())
                .expect("catalog messages are strings");
            (key.clone(), message.to_owned())
        })
        .collect();
    Locale {
        requested,
        resolved,
        fallback,
        messages,
    }
}

impl Locale {
    /// A catalog message with `{name}` placeholders filled; unknown names stay literal.
    pub fn text(&self, key: &str, params: &[(&str, &str)]) -> String {
        let template = self
            .messages
            .get(key)
            .unwrap_or_else(|| panic!("Missing catalog message {key}"));
        let mut result = String::new();
        let mut rest = template.as_str();
        while let Some(start) = rest.find('{') {
            result.push_str(&rest[..start]);
            rest = &rest[start..];
            let value = rest[1..].find('}').and_then(|end| {
                let name = &rest[1..end + 1];
                params
                    .iter()
                    .find(|(candidate, _)| *candidate == name)
                    .map(|(_, value)| (*value, end + 2))
            });
            match value {
                Some((value, consumed)) => {
                    result.push_str(value);
                    rest = &rest[consumed..];
                }
                None => {
                    result.push('{');
                    rest = &rest[1..];
                }
            }
        }
        result.push_str(rest);
        result
    }
}
