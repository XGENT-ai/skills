// Word-by-word annotation acceptance: authored text survives wrapping exactly,
// numbers follow reading order, cards quote their span and controls catch a
// figure that drifts from its facts.
use phoenix_diagram::{
    check,
    input::{parse_input, CompileOptions, DiagramType},
    layout,
    render::{render, render_static, EmbeddedAssets, FontPolicy},
    scene::DiagramFacts,
};
use scraper::{Html, Selector};
use serde_json::{json, Value};

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

fn select(css: &str) -> Selector {
    Selector::parse(css).unwrap()
}

fn example() -> Value {
    serde_json::from_str(include_str!(
        "../../../skills/phoenix-diagram/examples/minimal.annotation.json"
    ))
    .unwrap()
}

fn compile(document: Value) -> phoenix_diagram::scene::CompiledDiagram {
    let options = CompileOptions::default();
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Annotation,
        &options,
    )
    .unwrap_or_else(|errors| panic!("parse failed: {errors:?}"));
    layout::compile(&input, &options).unwrap_or_else(|errors| panic!("layout failed: {errors:?}"))
}

fn html(scene: &phoenix_diagram::scene::CompiledDiagram) -> String {
    String::from_utf8(render(scene, &EmbeddedAssets { template: TEMPLATE }).unwrap()).unwrap()
}

fn sentence(text: &str, annotations: Value) -> Value {
    json!({
        "schema_version": 1,
        "diagram_type": "annotation",
        "meta": {"title": "T", "output": "t.html"},
        "sentences": [{"id": "s", "text": text, "annotations": annotations}],
    })
}

fn errors_of(document: &Value) -> phoenix_diagram::diagnostic::Diagnostics {
    parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Annotation,
        &CompileOptions::default(),
    )
    .unwrap_err()
}

fn drawn_sentence(document: &Html, id: &str) -> String {
    let css = format!("text[data-annotation-line=\"{id}\"]");
    let tspans = select("tspan");
    let mut text = String::new();
    for line in document.select(&select(&css)) {
        for tspan in line.select(&tspans) {
            if tspan.value().attr("data-annotation-marker").is_none() {
                text.push_str(&tspan.text().collect::<String>());
            }
        }
    }
    text
}

fn span_text(document: &Html, number: u64) -> String {
    let css = format!("tspan[data-annotation-ref=\"{number}\"]");
    document
        .select(&select(&css))
        .map(|tspan| tspan.text().collect::<String>())
        .collect()
}

#[test]
fn the_minimal_example_numbers_spans_in_reading_order_and_rebuilds_its_prose() {
    let scene = compile(example());
    let DiagramFacts::Annotation { sentences } = &scene.facts else {
        panic!("annotation facts expected");
    };
    assert_eq!(sentences.len(), 2);
    let first = &sentences[0];
    assert_eq!(
        first["text"],
        "The dashboard renders faster after the cache change."
    );
    let numbers: Vec<u64> = first["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|annotation| annotation["number"].as_u64().unwrap())
        .collect();
    // Authored order is not reading order: "dashboard" comes first in the text.
    assert_eq!(numbers, [1, 2, 3]);
    let dashboard = &first["annotations"][0];
    assert_eq!(dashboard["node"], "headline-1");
    assert_eq!(dashboard["issue"], true);
    assert_eq!(dashboard["start"], 4);
    assert_eq!(dashboard["end"], 13);
    assert_eq!(sentences[1]["annotations"][0]["number"], 4);

    let page = Html::parse_document(&html(&scene));
    assert_eq!(
        drawn_sentence(&page, "headline"),
        "The dashboard renders faster after the cache change."
    );
    assert_eq!(drawn_sentence(&page, "closing"), "Nobody waits anymore.");
    assert_eq!(span_text(&page, 1), "dashboard");
    assert_eq!(span_text(&page, 2), "renders faster");
    assert_eq!(span_text(&page, 4), "Nobody");
    // The cards read in number order and the issue card keeps its tag.
    let order: Vec<&str> = page
        .select(&select("g[data-annotation-number]"))
        .filter_map(|group| group.value().attr("data-annotation-number"))
        .collect();
    assert_eq!(order, ["1", "2", "3", "4"]);
    let issue = page
        .select(&select("g[data-node-id='headline-1']"))
        .next()
        .unwrap();
    assert!(issue.value().attr("data-annotation-issue").is_some());
    assert!(issue.select(&select(".an-issue-tag")).next().is_some());
    let card: String = issue.text().collect();
    assert!(card.contains("Which dashboard? Three ship in this product."));
    assert!(card.contains("Fix the usage dashboard"), "{card}");
    let plain = page
        .select(&select("g[data-node-id='headline-0']"))
        .next()
        .unwrap();
    assert!(plain.value().attr("data-annotation-issue").is_none());
    assert!(plain.select(&select(".an-issue-tag")).next().is_none());
}

#[test]
fn a_span_that_crosses_a_line_keeps_every_character_and_one_marker() {
    let words: Vec<String> = (0..40).map(|index| format!("word{index:02}")).collect();
    let text = words.join(" ");
    let span: String = words[..22].join(" ");
    let document = sentence(
        &text,
        json!([{"span": span, "note": "Twenty-two words at once."}]),
    );
    let scene = compile(document);
    let page = Html::parse_document(&html(&scene));
    assert_eq!(drawn_sentence(&page, "s"), text);
    assert_eq!(span_text(&page, 1), span);
    let pieces = page
        .select(&select("tspan[data-annotation-ref='1']"))
        .count();
    assert!(pieces >= 2, "the span must wrap, got {pieces} pieces");
    // Every marker directly follows the span's current piece.
    for marker in page.select(&select("tspan[data-annotation-marker]")) {
        let previous = marker
            .prev_siblings()
            .filter_map(scraper::ElementRef::wrap)
            .find(|element| element.value().name() == "tspan")
            .and_then(|element| element.value().attr("data-annotation-ref"));
        assert_eq!(
            previous,
            marker.value().attr("data-annotation-marker"),
            "marker must follow its span"
        );
    }
}

#[test]
fn an_unbreakable_token_wider_than_the_column_splits_by_characters() {
    let text = "x".repeat(200);
    let document = sentence(
        &text,
        json!([{"span": text.clone(), "note": "One long token."}]),
    );
    let scene = compile(document);
    let page = Html::parse_document(&html(&scene));
    assert_eq!(drawn_sentence(&page, "s"), text);
    assert_eq!(span_text(&page, 1), text);
    assert!(page
        .select(&select("g[data-node-id='s-0']"))
        .next()
        .is_some());
}

#[test]
fn overlapping_spans_are_rejected_at_the_later_authored_annotation() {
    let document = sentence(
        "cache warmer",
        json!([{"span": "cache warm", "note": "first"}, {"span": "warm", "note": "second"}]),
    );
    let errors = errors_of(&document);
    assert!(
        errors
            .iter()
            .any(|error| error.code == "annotation/span-overlap"
                && error.subject.path == "/sentences/0/annotations/1"),
        "{errors:?}"
    );
    let evidence = errors
        .iter()
        .find(|error| error.code == "annotation/span-overlap")
        .map(|error| &error.evidence)
        .unwrap();
    assert_eq!(evidence["other"], 0);
    assert_eq!(evidence["start"], 6);
    assert_eq!(evidence["end"], 10);
}

#[test]
fn identical_and_contained_spans_are_rejected_at_the_later_annotation() {
    for annotations in [
        json!([{"span": "cache", "note": "a"}, {"span": "cache", "note": "b"}]),
        json!([{"span": "cache warm", "note": "a"}, {"span": "warm", "note": "b"}]),
        json!([{"span": "warm", "note": "a"}, {"span": "cache warm", "note": "b"}]),
    ] {
        let document = sentence("cache warmer", annotations);
        let errors = errors_of(&document);
        assert!(
            errors
                .iter()
                .any(|error| error.code == "annotation/span-overlap"
                    && error.subject.path == "/sentences/0/annotations/1"),
            "{errors:?}"
        );
    }
}

#[test]
fn unknown_spans_and_impossible_occurrences_fail_validation() {
    let missing = sentence("the dashboard", json!([{"span": "cache", "note": "n"}]));
    let errors = errors_of(&missing);
    assert!(
        errors
            .iter()
            .any(|error| error.code == "annotation/span-missing"
                && error.subject.path == "/sentences/0/annotations/0/span"),
        "{errors:?}"
    );
    let exceeded = sentence(
        "cache cache",
        json!([{"span": "cache", "occurrence": 3, "note": "n"}]),
    );
    let errors = errors_of(&exceeded);
    assert!(
        errors
            .iter()
            .any(|error| error.code == "annotation/span-missing"
                && error.subject.path == "/sentences/0/annotations/0/occurrence"),
        "{errors:?}"
    );
}

#[test]
fn an_annotation_without_a_span_is_a_schema_error() {
    let document = sentence("cache warmer", json!([{"note": "no span is not enough"}]));
    let errors = errors_of(&document);
    assert!(
        errors.iter().any(|error| error.code == "input/schema"
            && error.subject.path == "/sentences/0/annotations/0"),
        "{errors:?}"
    );
}

#[test]
fn occurrence_selects_the_requested_match() {
    let document = sentence(
        "cache cache",
        json!([{"span": "cache", "occurrence": 2, "note": "second"}]),
    );
    let scene = compile(document);
    let DiagramFacts::Annotation { sentences } = &scene.facts else {
        panic!("annotation facts expected");
    };
    assert_eq!(sentences[0]["annotations"][0]["start"], 6);
    assert_eq!(sentences[0]["annotations"][0]["end"], 11);
    let page = Html::parse_document(&html(&scene));
    let marked: Vec<String> = page
        .select(&select("tspan[data-annotation-ref]"))
        .map(|tspan| tspan.text().collect::<String>())
        .collect();
    assert_eq!(marked, ["cache"]);
    assert_eq!(
        page.select(&select("tspan[data-annotation-ref]")).count(),
        1
    );
}

#[test]
fn chinese_and_mixed_prose_rebuild_character_for_character_after_wrapping() {
    let chinese = "缓存让仪表盘渲染更快，但读者不知道是哪个仪表盘。".repeat(4);
    let mixed = "The 缓存 lets the dashboard render faster after the 切换 and repeat. ".repeat(4);
    for (text, span) in [(&chinese, "仪表盘"), (&mixed, "缓存")] {
        let document = sentence(text, json!([{"span": span, "note": "n"}]));
        let scene = compile(document);
        let page = Html::parse_document(&html(&scene));
        assert_eq!(drawn_sentence(&page, "s"), *text);
        assert_eq!(span_text(&page, 1), span);
        let lines = page
            .select(&select("text[data-annotation-line='s']"))
            .count();
        assert!(lines >= 2, "the prose must wrap, got {lines} lines");
    }
}

#[test]
fn the_first_occurrence_is_the_default_choice() {
    let document = sentence("cache cache", json!([{"span": "cache", "note": "n"}]));
    let scene = compile(document);
    let DiagramFacts::Annotation { sentences } = &scene.facts else {
        panic!("annotation facts expected");
    };
    assert_eq!(sentences[0]["annotations"][0]["start"], 0);
    assert_eq!(sentences[0]["annotations"][0]["end"], 5);
}

#[test]
fn artifact_checking_catches_drawn_text_that_drifts_from_the_facts() {
    let scene = compile(example());
    let page = html(&scene);
    let marker = "<tspan class=\"an-marker\" data-annotation-marker=\"1\" font-size=\"10\" dy=\"-5\">1!</tspan>";
    assert!(page.contains(marker));
    let without_marker = page.replacen(marker, "", 1);
    let result = check::check_html(without_marker.as_bytes(), scene.quality);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|error| error.code == "artifact/semantic-text"),
        "{:?}",
        result.diagnostics
    );

    let changed_span = page.replacen(
        "data-annotation-ref=\"2\" font-weight=\"700\">renders faster",
        "data-annotation-ref=\"2\" font-weight=\"700\">renders slow",
        1,
    );
    assert_ne!(changed_span, page);
    let result = check::check_html(changed_span.as_bytes(), scene.quality);
    assert!(result
        .diagnostics
        .iter()
        .any(|error| error.code == "artifact/semantic-text"));

    let wrong_card = page.replacen(
        "data-node-id=\"headline-0\" data-node-label=\"renders faster\"",
        "data-node-id=\"headline-0\" data-node-label=\"renders slower\"",
        1,
    );
    let result = check::check_html(wrong_card.as_bytes(), scene.quality);
    assert!(result
        .diagnostics
        .iter()
        .any(|error| error.code == "artifact/semantic-text"));
}

#[test]
fn a_wide_title_moves_the_aside_onto_its_own_line() {
    let aside = "marketing owns this line";
    let short = compile(json!({
        "schema_version": 1, "diagram_type": "annotation",
        "meta": {"title": "T", "output": "t.html"},
        "sentences": [{"id": "s", "title": "Short", "aside": aside,
            "text": "Cache warms the page.", "annotations": [{"span": "warms", "note": "n"}]}],
    }));
    let long = compile(json!({
        "schema_version": 1, "diagram_type": "annotation",
        "meta": {"title": "T", "output": "t.html"},
        "sentences": [{"id": "s", "title": "A title long enough to push the aside across the column ".repeat(2),
            "aside": aside, "text": "Cache warms the page.",
            "annotations": [{"span": "warms", "note": "n"}]}],
    }));
    let baseline = |scene: &phoenix_diagram::scene::CompiledDiagram, suffix: &str| {
        scene.containers[0]
            .text
            .iter()
            .find(|text| text.source_path.ends_with(suffix))
            .map(|text| text.position.y)
            .unwrap()
    };
    assert_eq!(baseline(&short, "/title"), baseline(&short, "/aside"));
    assert!(baseline(&long, "/aside") > baseline(&long, "/title"));
}

#[test]
fn the_static_figure_keeps_annotation_text_markers_and_issue_styles() {
    let scene = compile(example());
    let svg = String::from_utf8(render_static(&scene, "minimal", FontPolicy::Embedded).unwrap())
        .expect("utf8 figure");
    assert!(svg.contains("data-annotation-marker=\"1\""));
    assert!(svg.contains("an-issue-tag"));
    assert!(svg.contains("white-space:pre"));
    assert!(svg.contains("data-annotation-line=\"headline\""));
    let page = Html::parse_document(&svg);
    assert_eq!(page.select(&select("g[data-annotation-number]")).count(), 4);
}
