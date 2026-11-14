use phoenix_diagram::{
    check,
    input::{parse_input, CompileOptions, DiagramType},
    layout,
    render::{render, EmbeddedAssets},
};
use scraper::{ElementRef, Html, Selector};
use serde_json::{json, Value};

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

fn select(css: &str) -> Selector {
    Selector::parse(css).unwrap()
}

fn number(element: &ElementRef<'_>, key: &str) -> f64 {
    element
        .value()
        .attr(key)
        .unwrap_or_default()
        .parse()
        .unwrap_or(f64::NAN)
}

fn text_of(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}

fn compile(document: Value) -> phoenix_diagram::scene::CompiledDiagram {
    let options = CompileOptions::default();
    let input =
        parse_input(&serde_json::to_vec(&document).unwrap(), DiagramType::Chart, &options).unwrap();
    layout::compile(&input, &options).unwrap()
}

fn html(scene: &phoenix_diagram::scene::CompiledDiagram) -> String {
    String::from_utf8(render(scene, &EmbeddedAssets { template: TEMPLATE }).unwrap()).unwrap()
}

fn parse_errors(document: &Value) -> phoenix_diagram::diagnostic::Diagnostics {
    parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Chart,
        &CompileOptions::default(),
    )
    .unwrap_err()
}

fn assert_code_path(document: Value, code: &str, path: &str) {
    let errors = parse_errors(&document);
    assert!(
        errors
            .iter()
            .any(|error| error.code == code && error.subject.path == path),
        "{code} at {path}: {errors:?}"
    );
}

fn bar(rows: Value) -> Value {
    json!({
        "schema_version": 1,
        "diagram_type": "chart",
        "meta": {"title": "T", "evidence": "sourced"},
        "form": "bar",
        "rows": rows,
    })
}

#[test]
fn form_field_pairings_and_series_count_point_at_the_offending_field() {
    // bar/line/pie rows need values; extra limit fields land on the field.
    let missing = json!({"id":"a","label":"A"});
    assert_code_path(bar(json!([missing])), "chart/form-fields", "/rows/0");
    assert_code_path(
        bar(json!([{"id":"a","label":"A","values":[1],"value":2}])),
        "chart/form-fields",
        "/rows/0/value",
    );
    assert_code_path(
        json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "bar",
            "series": [{"id":"x","name":"X"},{"id":"y","name":"Y"}],
            "rows": [{"id":"a","label":"A","values":[1]}],
        }),
        "chart/series-count",
        "/rows/0/values",
    );
    // limits rows need limit and refuse values; orientation and series refuse forms.
    assert_code_path(
        json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "limits",
            "rows": [{"id":"a","label":"A"}],
        }),
        "chart/form-fields",
        "/rows/0",
    );
    assert_code_path(
        json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "limits",
            "rows": [{"id":"a","label":"A","values":[1],"limit":2}],
        }),
        "chart/form-fields",
        "/rows/0/values",
    );
    assert_code_path(
        json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "line", "orientation": "vertical",
            "rows": [{"id":"a","label":"A","values":[1]}],
        }),
        "chart/form-fields",
        "/orientation",
    );
    assert_code_path(
        json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "limits",
            "series": [{"id":"x","name":"X"}],
            "rows": [{"id":"a","label":"A","limit":2}],
        }),
        "chart/form-fields",
        "/series",
    );
    // Pie: one series per row, no negatives, not everything zero.
    assert_code_path(
        json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "pie",
            "series": [{"id":"x","name":"X"},{"id":"y","name":"Y"}],
            "rows": [{"id":"a","label":"A","values":[1,2]}],
        }),
        "chart/pie-value",
        "/series",
    );
    assert_code_path(
        bar(json!([{"id":"a","label":"A","values":[-1]}]))
        .tap_form_pie(),
        "chart/pie-value",
        "/rows/0/values/0",
    );
    assert_code_path(
        bar(json!([{"id":"a","label":"A","values":[0]}])).tap_form_pie(),
        "chart/pie-value",
        "/rows",
    );
}

trait TapFormPie {
    fn tap_form_pie(self) -> Value;
}

impl TapFormPie for Value {
    fn tap_form_pie(mut self) -> Value {
        self["form"] = json!("pie");
        self
    }
}

#[test]
fn bar_axis_covers_zero_and_bars_follow_the_sign() {
    // All positive: the zero tick is the bars' common bottom edge.
    let scene = compile(bar(json!([
        {"id":"a","label":"A","values":[3]},
        {"id":"b","label":"B","values":[5]},
    ])));
    let document = Html::parse_document(&html(&scene));
    let svg = document.select(&select("svg")).next().unwrap();
    let zero_tick = svg.select(&select("line[data-chart-tick='0']")).next().unwrap();
    let zero_y = number(&zero_tick, "y1");
    for group in svg.select(&select("g[data-node-id]")) {
        let bar = group
            .select(&select("rect[data-chart-series='0']"))
            .next()
            .unwrap();
        assert!((number(&bar, "y") + number(&bar, "height") - zero_y).abs() < 0.06);
    }
    // Cross zero: the negative bar grows to the other side of the same line.
    let scene = compile(bar(json!([
        {"id":"a","label":"A","values":[3]},
        {"id":"b","label":"B","values":[-3]},
    ])));
    let document = Html::parse_document(&html(&scene));
    let svg = document.select(&select("svg")).next().unwrap();
    let zero_tick = svg.select(&select("line[data-chart-tick='0']")).next().unwrap();
    let zero_y = number(&zero_tick, "y1");
    let bars: Vec<_> = svg
        .select(&select("rect[data-chart-series='0']"))
        .collect();
    assert_eq!(bars.len(), 2);
    let positive = bars.iter().find(|bar| number(*bar, "y") < zero_y).unwrap();
    let negative = bars.iter().find(|bar| number(*bar, "y") >= zero_y).unwrap();
    assert!((number(positive, "y") + number(positive, "height") - zero_y).abs() < 0.06);
    assert!((number(negative, "y") - zero_y).abs() < 0.06);
    // All zero: axis becomes [0, 1] and no bar is drawn.
    let scene = compile(bar(json!([{"id":"a","label":"A","values":[0]}])));
    let document = Html::parse_document(&html(&scene));
    let svg = document.select(&select("svg")).next().unwrap();
    assert!(svg.select(&select("rect[data-chart-series]")).next().is_none());
    let ticks: Vec<String> = svg
        .select(&select("line[data-chart-tick]"))
        .map(|tick| tick.value().attr("data-chart-tick").unwrap().to_owned())
        .collect();
    assert_eq!(ticks, vec!["0", "1"]);
}

#[test]
fn values_are_written_exactly_as_authored() {
    let scene = compile(bar(json!([
        {"id":"a","label":"A","values":[12]},
        {"id":"b","label":"B","values":[4.5]},
        {"id":"c","label":"C","values":[0.1]},
        {"id":"d","label":"D","values":[1234567]},
        {"id":"e","label":"E","values":[-3]},
    ])));
    let document = Html::parse_document(&html(&scene));
    for (id, expected) in
        [("a", "12"), ("b", "4.5"), ("c", "0.1"), ("d", "1234567"), ("e", "-3")]
    {
        let group = document
            .select(&select(&format!("g[data-node-id='{id}']")))
            .next()
            .unwrap();
        let label = group
            .select(&select("text[data-chart-value-label='0']"))
            .next()
            .unwrap();
        assert_eq!(text_of(label), expected, "row {id}");
    }
    let group = document.select(&select("g[data-node-id='d']")).next().unwrap();
    assert_eq!(group.value().attr("data-chart-values"), Some("1234567"));
}

#[test]
fn equal_values_of_two_series_keep_separate_annotations() {
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced"},
        "form": "bar",
        "series": [{"id":"x","name":"X"},{"id":"y","name":"Y"}],
        "rows": [{"id":"a","label":"A","values":[7,7]}],
    }));
    let document = Html::parse_document(&html(&scene));
    let first = number(
        &document
            .select(&select("text[data-chart-value-label='0']"))
            .next()
            .unwrap(),
        "x",
    );
    let second = number(
        &document
            .select(&select("text[data-chart-value-label='1']"))
            .next()
            .unwrap(),
        "x",
    );
    assert!((first - second).abs() >= 20.0, "{first} vs {second}");
    // Each annotation carries its series key.
    assert_eq!(
        text_of(document.select(&select("text[data-chart-value-label='0']")).next().unwrap()),
        "A 7"
    );
}

#[test]
fn the_27th_series_key_is_aa() {
    let series: Vec<Value> = (0..27)
        .map(|index| json!({"id": format!("s{index}"), "name": format!("S{index}")}))
        .collect();
    let values: Vec<f64> = (0..27).map(|index| (index + 1) as f64).collect();
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced"},
        "form": "bar",
        "series": series,
        "rows": [{"id":"a","label":"A","values": values}],
    }));
    let document = Html::parse_document(&html(&scene));
    assert_eq!(
        text_of(document.select(&select("text[data-chart-value-label='26']")).next().unwrap()),
        "AA 27"
    );
}

#[test]
fn the_minimal_example_passes_the_artifact_check() {
    let raw = include_str!("../../../skills/phoenix-diagram/examples/minimal.chart.json");
    let scene = compile(serde_json::from_str(raw).unwrap());
    let result = check::check_html(html(&scene).as_bytes(), scene.quality);
    assert!(
        result.diagnostics.is_empty(),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn sourced_evidence_draws_no_badge_and_still_passes_the_artifact_check() {
    let scene = compile(bar(json!([
        {"id":"a","label":"A","values":[1]},
        {"id":"b","label":"B","values":[2]},
    ])));
    let document = Html::parse_document(&html(&scene));
    assert!(document
        .select(&select("g[data-chart-evidence]"))
        .next()
        .is_none());
    let result = check::check_html(html(&scene).as_bytes(), scene.quality);
    assert!(
        result.diagnostics.is_empty(),
        "{:?}",
        result.diagnostics
    );
}
