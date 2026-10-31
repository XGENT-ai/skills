// Shared comparison of a rendered Phoenix SVG with an element list captured
// from the frozen renderer (tests/fixtures/{erd,class}/generate.mjs).
#![allow(dead_code)]

use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, InputContract, Quality};
use phoenix_diagram::render::{render, EmbeddedAssets};
use phoenix_diagram::scene::CompiledDiagram;
use phoenix_diagram::{diagnostic, layout};
use scraper::{ElementRef, Html, Node, Selector};
use serde_json::{Map, Value};

pub const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

/// One SVG element: tag, attributes, parent index, own text and, for a
/// <text> element, its full text including every <tspan> in order.
#[derive(Clone, Debug)]
pub struct Element {
    pub tag: String,
    pub attrs: Map<String, Value>,
    pub parent: isize,
    pub text: String,
    pub full: String,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).and_then(Value::as_str)
    }
    pub fn number(&self, name: &str) -> f64 {
        self.attr(name)
            .and_then(|value| value.parse().ok())
            .unwrap_or(f64::NAN)
    }
}

pub fn quality(case: &Value) -> Quality {
    if case["quality"] == "showcase" {
        Quality::Showcase
    } else {
        Quality::Standard
    }
}

pub fn compile(
    document: &Value,
    kind: DiagramType,
    quality: Quality,
) -> Result<CompiledDiagram, diagnostic::Diagnostics> {
    let options = CompileOptions {
        input_contract: Some(InputContract::PhoenixV1),
        quality: Some(quality),
        ..Default::default()
    };
    let input = parse_input(&serde_json::to_vec(document).unwrap(), kind, &options)?;
    layout::compile(&input, &options)
}

pub fn html(scene: &CompiledDiagram) -> String {
    String::from_utf8(render(scene, &EmbeddedAssets { template: TEMPLATE }).unwrap()).unwrap()
}

pub fn oracle_elements(case: &Value) -> Vec<Element> {
    case["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| Element {
            tag: item[0].as_str().unwrap().into(),
            attrs: item[1].as_object().unwrap().clone(),
            parent: item[2].as_i64().unwrap() as isize,
            text: item[3].as_str().unwrap().into(),
            full: item[4].as_str().unwrap().into(),
        })
        .collect()
}

/// The main SVG flattened like the oracle: document order, without <defs>/<style>.
pub fn rendered_elements(html: &str) -> Vec<Element> {
    let document = Html::parse_document(html);
    let svg = document
        .select(&Selector::parse("svg").unwrap())
        .next()
        .expect("main SVG");
    let mut result = vec![];
    fn walk(element: ElementRef<'_>, parent: isize, result: &mut Vec<Element>) {
        let tag = element.value().name();
        if tag == "defs" || tag == "style" {
            return;
        }
        let text: String = element
            .children()
            .filter_map(|child| match child.value() {
                Node::Text(text) if !text.trim().is_empty() => Some(text.to_string()),
                _ => None,
            })
            .collect();
        let attrs = element
            .value()
            .attrs()
            .map(|(key, value)| (key.to_owned(), Value::String(value.to_owned())))
            .collect();
        let full = if tag == "text" {
            element.text().collect()
        } else {
            String::new()
        };
        result.push(Element {
            tag: tag.into(),
            attrs,
            parent,
            text,
            full,
        });
        let index = result.len() as isize - 1;
        for child in element.children().filter_map(ElementRef::wrap) {
            walk(child, index, result);
        }
    }
    walk(svg, -1, &mut result);
    result
}

pub fn descendants(elements: &[Element], root: usize) -> Vec<usize> {
    let mut result = vec![];
    let mut inside = vec![root as isize];
    for (index, element) in elements.iter().enumerate().skip(root + 1) {
        while let Some(last) = inside.last() {
            if *last == element.parent {
                break;
            }
            inside.pop();
        }
        if inside.is_empty() {
            break;
        }
        result.push(index);
        inside.push(index as isize);
    }
    result
}

pub fn find(elements: &[Element], predicate: impl Fn(&Element) -> bool) -> Vec<usize> {
    elements
        .iter()
        .enumerate()
        .filter(|(_, element)| predicate(element))
        .map(|(index, _)| index)
        .collect()
}

pub fn numbers(value: &str) -> Vec<f64> {
    let mut result = vec![];
    let mut token = String::new();
    let flush = |token: &mut String, result: &mut Vec<f64>| {
        if !token.is_empty() {
            result.push(token.parse::<f64>().unwrap_or(f64::NAN));
            token.clear();
        }
    };
    for character in value.chars() {
        if character.is_ascii_digit()
            || character == '.'
            || (character == 'e' && token.chars().any(|c| c.is_ascii_digit()))
        {
            token.push(character);
        } else if character == '-' {
            flush(&mut token, &mut result);
            token.push(character);
        } else {
            flush(&mut token, &mut result);
        }
    }
    flush(&mut token, &mut result);
    result
}

const GEOMETRY: [&str; 16] = [
    "x",
    "y",
    "width",
    "height",
    "rx",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "font-size",
    "stroke-width",
    "data-legend-x",
    "data-legend-baseline",
];
const NUMERIC_LISTS: [&str; 3] = ["d", "transform", "data-composition-points"];

/// Compare the attributes `keys` of two elements: geometry within `tolerance`.
pub fn same_attributes(
    expected: &Element,
    actual: &Element,
    keys: &[&str],
    tolerance: f64,
    context: &str,
) {
    for key in keys {
        let (left, right) = (expected.attr(key), actual.attr(key));
        if GEOMETRY.contains(key) || *key == "data-legend-width" {
            match (left, right) {
                (None, None) => {}
                (Some(left), Some(right)) => {
                    let (a, b) = (left.parse::<f64>().unwrap(), right.parse::<f64>().unwrap());
                    assert!(
                        (a - b).abs() <= tolerance,
                        "{context} {key}: {left} != {right}"
                    );
                }
                _ => panic!("{context} {key}: {left:?} != {right:?}"),
            }
        } else if NUMERIC_LISTS.contains(key) {
            let (a, b) = (
                left.map(numbers).unwrap_or_default(),
                right.map(numbers).unwrap_or_default(),
            );
            assert_eq!(a.len(), b.len(), "{context} {key}: {left:?} != {right:?}");
            assert_eq!(
                left.map(|v| v
                    .chars()
                    .filter(char::is_ascii_alphabetic)
                    .collect::<String>()),
                right.map(|v| v
                    .chars()
                    .filter(char::is_ascii_alphabetic)
                    .collect::<String>()),
                "{context} {key} commands"
            );
            for (x, y) in a.iter().zip(&b) {
                assert!(
                    (x - y).abs() <= tolerance,
                    "{context} {key}: {left:?} != {right:?}"
                );
            }
        } else {
            assert_eq!(left, right, "{context} {key}");
        }
    }
}

/// Element-by-element comparison of two subtrees on the given attributes and own text.
pub fn same_subtree(
    expected: &[Element],
    expected_root: usize,
    actual: &[Element],
    actual_root: usize,
    keys: &[&str],
    context: &str,
) {
    let left = descendants(expected, expected_root);
    let right = descendants(actual, actual_root);
    let describe = |elements: &[Element], items: &[usize]| {
        items
            .iter()
            .map(|index| format!("{}:{}", elements[*index].tag, elements[*index].text))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        left.len(),
        right.len(),
        "{context}: {:?} != {:?}",
        describe(expected, &left),
        describe(actual, &right)
    );
    for (position, (a, b)) in left.iter().zip(&right).enumerate() {
        let (a, b) = (&expected[*a], &actual[*b]);
        let label = format!("{context}/{position}<{}>", a.tag);
        assert_eq!(a.tag, b.tag, "{label}");
        assert_eq!(a.text.trim(), b.text.trim(), "{label} text");
        assert_eq!(a.full, b.full, "{label} full text");
        same_attributes(a, b, keys, 1e-5, &label);
    }
}

/// Every element that carries a Reader key, projected onto those keys.
pub fn reader_projection(
    elements: &[Element],
    keys: &[&str],
    skip: impl Fn(&Element) -> bool,
) -> Vec<String> {
    let mut result: Vec<String> = elements
        .iter()
        .filter(|element| !skip(element))
        .filter_map(|element| {
            let present: Vec<_> = keys
                .iter()
                .filter_map(|key| element.attr(key).map(|value| format!("{key}={value:?}")))
                .collect();
            (!present.is_empty()).then(|| format!("{} {}", element.tag, present.join(" ")))
        })
        .collect();
    result.sort();
    result
}

pub fn near(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: {actual} != {expected}"
    );
}
