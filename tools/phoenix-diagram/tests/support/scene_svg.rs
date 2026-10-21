// Compares a Phoenix SVG with the frozen renderer's SVG element by element.
use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType};
use phoenix_diagram::render::{render, EmbeddedAssets};
use phoenix_diagram::{diagnostic, layout, scene};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;

pub const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

/// Scene positions must match the reference layout report to this tolerance.
pub const SCENE: f64 = 1e-8;
/// Serialized SVG numbers use six decimals.
pub const RENDERED: f64 = 1e-5;

pub fn select(css: &str) -> Selector {
    Selector::parse(css).unwrap()
}

pub fn near(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: {actual} != {expected}"
    );
}

pub fn compile(
    document: &Value,
    kind: DiagramType,
) -> Result<scene::CompiledDiagram, diagnostic::Diagnostics> {
    let options = CompileOptions::default();
    let input = parse_input(&serde_json::to_vec(document).unwrap(), kind, &options)?;
    layout::compile(&input, &options)
}

pub fn html(scene: &scene::CompiledDiagram) -> String {
    String::from_utf8(render(scene, &EmbeddedAssets { template: TEMPLATE }).unwrap()).unwrap()
}

pub fn svg_of(html: &str) -> String {
    let start = html.find("<svg").unwrap();
    let end = html.rfind("</svg>").unwrap() + "</svg>".len();
    html[start..end].to_owned()
}

/// Every numeric token of an attribute such as `d` or `points`.
pub fn numbers(value: &str) -> Vec<f64> {
    value
        .split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | 'e' | '+')))
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse().ok())
        .collect()
}

fn same_value(reference: &str, actual: &str) -> bool {
    if reference == actual {
        return true;
    }
    let (a, b) = (numbers(reference), numbers(actual));
    let skeleton = |value: &str| {
        value
            .chars()
            .filter(|c| c.is_ascii_alphabetic() && *c != 'e')
            .collect::<String>()
    };
    !a.is_empty()
        && a.len() == b.len()
        && skeleton(reference) == skeleton(actual)
        && a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= RENDERED)
}

pub struct Rules<'a> {
    /// Attributes Phoenix adds to the reference markup.
    pub extra: &'a [&'a str],
    /// Attributes compared separately by the caller.
    pub skip: &'a [&'a str],
}

/// Tag, attributes, direct text and child elements must agree recursively.
pub fn same_element(
    reference: ElementRef<'_>,
    actual: ElementRef<'_>,
    rules: &Rules<'_>,
    context: &str,
) {
    let (r, a) = (reference.value(), actual.value());
    assert_eq!(r.name(), a.name(), "{context}: element");
    let context = format!("{context}/{}", r.name());
    for (name, value) in r.attrs() {
        if rules.skip.contains(&name) {
            continue;
        }
        let actual_value = a
            .attr(name)
            .unwrap_or_else(|| panic!("{context}: missing {name}={value:?}"));
        assert!(
            same_value(value, actual_value),
            "{context}: {name} {actual_value:?} != {value:?}"
        );
    }
    for (name, value) in a.attrs() {
        assert!(
            r.attr(name).is_some() || rules.extra.contains(&name) || rules.skip.contains(&name),
            "{context}: unexpected {name}={value:?}"
        );
    }
    let direct = |element: ElementRef<'_>| {
        element
            .children()
            .filter_map(|child| child.value().as_text().map(|text| text.trim().to_owned()))
            .collect::<String>()
    };
    assert_eq!(direct(reference), direct(actual), "{context}: text");
    let left: Vec<_> = reference.children().filter_map(ElementRef::wrap).collect();
    let right: Vec<_> = actual.children().filter_map(ElementRef::wrap).collect();
    assert_eq!(left.len(), right.len(), "{context}: child count");
    for (index, (left, right)) in left.into_iter().zip(right).enumerate() {
        same_element(left, right, rules, &format!("{context}[{index}]"));
    }
}

/// Elements matched by `css` agree pairwise in document order.
pub fn same_selection(
    reference: &Html,
    actual: &Html,
    css: &str,
    rules: &Rules<'_>,
    context: &str,
) -> usize {
    let selector = select(css);
    let left: Vec<_> = reference.select(&selector).collect();
    let right: Vec<_> = actual.select(&selector).collect();
    assert_eq!(left.len(), right.len(), "{context}: {css} count");
    let count = right.len();
    for (index, (left, right)) in left.into_iter().zip(right).enumerate() {
        same_element(left, right, rules, &format!("{context} {css}[{index}]"));
    }
    count
}

/// The reference's rejected-case codes, as a sorted set.
pub fn codes(diagnostics: &[Value]) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap().to_owned())
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

pub fn actual_codes(diagnostics: &diagnostic::Diagnostics) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics.iter().map(|d| d.code.clone()).collect();
    codes.sort();
    codes.dedup();
    codes
}
