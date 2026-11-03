use scraper::{Html, Selector};
use serde_json::Value;

/// The single embedded metadata object of an artifact.
pub(crate) fn embedded(document: &Html) -> Option<Value> {
    let selector = Selector::parse("script#phoenix-diagram-metadata[type='application/json']")
        .expect("static selector");
    let mut scripts = document.select(&selector);
    let script = scripts.next()?;
    if scripts.next().is_some() {
        return None;
    }
    serde_json::from_str::<Value>(&script.text().collect::<String>()).ok()
}
