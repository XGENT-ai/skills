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
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Chart,
        &options,
    )
    .unwrap_or_else(|errors| panic!("parse failed: {errors:?}"));
    layout::compile(&input, &options).unwrap_or_else(|errors| panic!("layout failed: {errors:?}"))
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
        bar(json!([{"id":"a","label":"A","values":[-1]}])).tap_form_pie(),
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
    let zero_tick = svg
        .select(&select("line[data-chart-tick='0']"))
        .next()
        .unwrap();
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
    let zero_tick = svg
        .select(&select("line[data-chart-tick='0']"))
        .next()
        .unwrap();
    let zero_y = number(&zero_tick, "y1");
    let bars: Vec<_> = svg.select(&select("rect[data-chart-series='0']")).collect();
    assert_eq!(bars.len(), 2);
    let positive = bars.iter().find(|bar| number(bar, "y") < zero_y).unwrap();
    let negative = bars.iter().find(|bar| number(bar, "y") >= zero_y).unwrap();
    assert!((number(positive, "y") + number(positive, "height") - zero_y).abs() < 0.06);
    assert!((number(negative, "y") - zero_y).abs() < 0.06);
    // All zero: axis becomes [0, 1] and no bar is drawn.
    let scene = compile(bar(json!([{"id":"a","label":"A","values":[0]}])));
    let document = Html::parse_document(&html(&scene));
    let svg = document.select(&select("svg")).next().unwrap();
    assert!(svg
        .select(&select("rect[data-chart-series]"))
        .next()
        .is_none());
    let ticks: Vec<String> = svg
        .select(&select("line[data-chart-tick]"))
        .map(|tick| tick.value().attr("data-chart-tick").unwrap().to_owned())
        .collect();
    assert_eq!(ticks, vec!["0", "1"]);
}

#[test]
fn wide_and_mixed_magnitude_charts_keep_spaced_ticks_and_pass_the_artifact_check() {
    let tick_gap = |html: &str| {
        let document = Html::parse_document(html);
        let mut ys: Vec<f64> = document
            .select(&select("line[data-chart-tick]"))
            .map(|tick| number(&tick, "y1"))
            .collect();
        ys.sort_by(f64::total_cmp);
        ys.windows(2)
            .map(|pair| pair[1] - pair[0])
            .fold(f64::INFINITY, f64::min)
    };
    let wide_bar = json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title": "T", "evidence": "sourced", "unit": "ms"},
        "form": "bar",
        "series": [{"id":"a","name":"A"},{"id":"b","name":"B"},{"id":"c","name":"C"}],
        "rows": (0..6).map(|i| json!({
            "id": format!("r{i}"), "label": format!("Row {i}"),
            "values": [100000 + i * 41000, 120000 + i * 37000, 305000 - i * 20000],
        })).collect::<Vec<_>>(),
    });
    let wide_line = json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title": "T", "evidence": "sourced"},
        "form": "line",
        "rows": (0..20).map(|i| json!({
            "id": format!("p{i}"), "label": format!("P{i}"), "values": [100 + i * 37],
        })).collect::<Vec<_>>(),
    });
    for document in [wide_bar, wide_line] {
        let scene = compile(document);
        let html = html(&scene);
        assert!(tick_gap(&html) >= 40.0 - 0.01, "gap {}", tick_gap(&html));
        let result = check::check_html(html.as_bytes(), scene.quality);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    }
    // A value far below one pixel of the axis still draws the 1 px minimum bar,
    // on its own side of the zero line in every direction.
    for (small, orientation) in [
        (1.0, "vertical"),
        (0.334971, "vertical"),
        (-0.334971, "vertical"),
        (1.0, "horizontal"),
        (-0.334971, "horizontal"),
        (-2.0, "horizontal"),
    ] {
        let mut document = bar(json!([
            {"id":"a","label":"Almost none","values":[small]},
            {"id":"b","label":"All of it","values":[1000]},
        ]));
        document["orientation"] = json!(orientation);
        let scene = compile(document);
        let result = check::check_html(html(&scene).as_bytes(), scene.quality);
        assert!(
            result.diagnostics.is_empty(),
            "{small} {orientation}: {:?}",
            result.diagnostics
        );
    }
    // The minimum does not excuse a bar drawn longer than both rules allow.
    for (small, orientation) in [(1.0, "vertical"), (-1.0, "horizontal")] {
        let mut document = bar(json!([
            {"id":"a","label":"Almost none","values":[small]},
            {"id":"b","label":"All of it","values":[1000]},
        ]));
        document["orientation"] = json!(orientation);
        let mut scene = compile(document);
        let row = scene
            .containers
            .iter_mut()
            .find(|container| {
                container.authored.id == "a" && container.authored.kind == "chart-row"
            })
            .unwrap();
        let mark = &mut row.authored.facts["marks"][0];
        // Keep the zero end fixed and stretch the bar to 4 px.
        let (start, length) = if orientation == "vertical" {
            ("y", "height")
        } else {
            ("x", "width")
        };
        let zero_end = mark[start].as_f64().unwrap() + mark[length].as_f64().unwrap();
        mark[start] = json!(zero_end - 4.0);
        mark[length] = json!(4.0);
        let result = check::check_html(html(&scene).as_bytes(), scene.quality);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|error| error.message.contains("shared linear scale from zero")),
            "{orientation}: {:?}",
            result.diagnostics
        );
    }
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
    for (id, expected) in [
        ("a", "12"),
        ("b", "4.5"),
        ("c", "0.1"),
        ("d", "1234567"),
        ("e", "-3"),
    ] {
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
    let group = document
        .select(&select("g[data-node-id='d']"))
        .next()
        .unwrap();
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
        text_of(
            document
                .select(&select("text[data-chart-value-label='0']"))
                .next()
                .unwrap()
        ),
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
        text_of(
            document
                .select(&select("text[data-chart-value-label='26']"))
                .next()
                .unwrap()
        ),
        "AA 27"
    );
}

#[test]
fn the_minimal_example_passes_the_artifact_check() {
    let raw = include_str!("../../../skills/phoenix-diagram/examples/minimal.chart.json");
    let scene = compile(serde_json::from_str(raw).unwrap());
    let result = check::check_html(html(&scene).as_bytes(), scene.quality);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
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
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn line_axis_follows_data_and_points_stay_on_scale() {
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced"},
        "form": "line",
        "rows": [
            {"id":"a","label":"A","values":[10]},
            {"id":"b","label":"B","values":[20]},
        ],
    }));
    let document = Html::parse_document(&html(&scene));
    let svg = document.select(&select("svg")).next().unwrap();
    // The axis follows the data: all-positive rows need no zero tick.
    assert!(svg
        .select(&select("line[data-chart-tick='0']"))
        .next()
        .is_none());
    let ticks: Vec<(f64, f64)> = svg
        .select(&select("line[data-chart-tick]"))
        .map(|tick| {
            (
                tick.value()
                    .attr("data-chart-tick")
                    .unwrap()
                    .parse()
                    .unwrap(),
                tick.value().attr("y1").unwrap().parse().unwrap(),
            )
        })
        .collect();
    let (v0, y0) = ticks[0];
    let (v1, y1) = ticks[1];
    let scale = (y1 - y0) / (v1 - v0);
    for (id, value) in [("a", 10.0), ("b", 20.0)] {
        let group = document
            .select(&select(&format!("g[data-node-id='{id}']")))
            .next()
            .unwrap();
        let cy: f64 = group
            .select(&select("circle[data-chart-series='0']"))
            .next()
            .unwrap()
            .value()
            .attr("cy")
            .unwrap()
            .parse()
            .unwrap();
        let expected = y0 + (value - v0) * scale;
        assert!((cy - expected).abs() < 0.06, "{id}: {cy} vs {expected}");
    }
}

#[test]
fn line_annotations_stack_with_minimum_spacing() {
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced"},
        "form": "line",
        "series": [
            {"id":"x","name":"X"},
            {"id":"y","name":"Y"},
            {"id":"z","name":"Z"},
        ],
        "rows": [{"id":"a","label":"A","values":[10, 10.02, 10.04]}],
    }));
    let document = Html::parse_document(&html(&scene));
    // A single row draws points only, no polylines.
    assert!(document.select(&select("path.ch-line")).next().is_none());
    let mut ys: Vec<f64> = (0..3)
        .map(|k| {
            document
                .select(&select(&format!("text[data-chart-value-label='{k}']")))
                .next()
                .unwrap()
                .value()
                .attr("y")
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect();
    ys.sort_by(|a, b| a.total_cmp(b));
    assert!(ys[1] - ys[0] >= 13.0 - 0.01, "{ys:?}");
    assert!(ys[2] - ys[1] >= 13.0 - 0.01, "{ys:?}");
}

#[test]
fn line_label_room_is_reported_for_a_row_that_cannot_fit() {
    let series: Vec<Value> = (0..27)
        .map(|index| json!({"id": format!("s{index}"), "name": format!("S{index}")}))
        .collect();
    let values: Vec<f64> = vec![10.0; 27];
    // label-room is a layout-stage error: run the compile, not just the parse.
    let options = CompileOptions::default();
    let input = parse_input(
        &serde_json::to_vec(&json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"sourced"},
            "form": "line",
            "series": series,
            "rows": [{"id":"a","label":"A","values": values}],
        }))
        .unwrap(),
        DiagramType::Chart,
        &options,
    )
    .unwrap();
    let errors = layout::compile(&input, &options).unwrap_err();
    assert!(
        errors.iter().any(|error| error.code == "chart/label-room"
            && error.subject.path == "/rows/0"
            && error.stage == "layout"),
        "{errors:?}"
    );
}

#[test]
fn pie_sectors_and_list_match_facts() {
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced"},
        "form": "pie",
        "rows": [
            {"id":"a","label":"A","values":[2]},
            {"id":"b","label":"B","values":[1]},
            {"id":"c","label":"C","values":[1]},
            {"id":"z","label":"Zero","values":[0]},
        ],
    }));
    let document = Html::parse_document(&html(&scene));
    let svg = document.select(&select("svg")).next().unwrap();
    // The order note is the non-colour key for reading the ring.
    assert!(svg
        .select(&select("text"))
        .any(|text| text.text().collect::<String>().contains("clockwise")));
    for (id, expected_sweep, expected_share) in [
        ("a", Some(180.0), "50%"),
        ("b", Some(90.0), "25%"),
        ("c", Some(90.0), "25%"),
        ("z", None, "0%"),
    ] {
        let group = document
            .select(&select(&format!("g[data-node-id='{id}']")))
            .next()
            .unwrap();
        let sector = group.select(&select("path[data-chart-sweep]")).next();
        match expected_sweep {
            Some(expected) => {
                let sweep: f64 = sector
                    .unwrap()
                    .value()
                    .attr("data-chart-sweep")
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!((sweep - expected).abs() <= 0.5, "{id}: {sweep}");
                let share = group
                    .select(&select("text[data-chart-value-label='1']"))
                    .next()
                    .unwrap();
                assert_eq!(text_of(share), expected_share, "{id}");
            }
            None => assert!(sector.is_none(), "{id} must not draw a sector"),
        }
    }
}

#[test]
fn pie_percent_unit_omits_share() {
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced","unit":"%"},
        "form": "pie",
        "rows": [
            {"id":"a","label":"A","values":[45]},
            {"id":"b","label":"B","values":[55]},
        ],
    }));
    let document = Html::parse_document(&html(&scene));
    assert!(document
        .select(&select("text[data-chart-value-label='1']"))
        .next()
        .is_none());
    assert_eq!(
        text_of(
            document
                .select(&select("text[data-chart-value-label='0']"))
                .next()
                .unwrap()
        ),
        "45%"
    );
}

#[test]
fn limits_fill_ratio_limit_line_and_over_mark() {
    let scene = compile(json!({
        "schema_version": 1, "diagram_type": "chart",
        "meta": {"title":"T","evidence":"sourced","unit":"req/s"},
        "form": "limits",
        "rows": [
            {"id":"ok","label":"OK","value":13,"limit":20},
            {"id":"over","label":"Over","value":25,"limit":20},
            {"id":"only","label":"Only","limit":20},
        ],
    }));
    let document = Html::parse_document(&html(&scene));
    // Within limit: fill ratio 13/25, limit line at 20/25 of the track.
    let ok = document
        .select(&select("g[data-node-id='ok']"))
        .next()
        .unwrap();
    let track = ok.select(&select("rect.ch-track")).next().unwrap();
    let track_w: f64 = track.value().attr("width").unwrap().parse().unwrap();
    let track_x: f64 = track.value().attr("x").unwrap().parse().unwrap();
    let fill_w: f64 = ok
        .select(&select("rect.ch-fill"))
        .next()
        .unwrap()
        .value()
        .attr("width")
        .unwrap()
        .parse()
        .unwrap();
    assert!((fill_w / track_w - 13.0 / 25.0).abs() < 0.01);
    let limit_x: f64 = ok
        .select(&select("line.ch-limit"))
        .next()
        .unwrap()
        .value()
        .attr("x1")
        .unwrap()
        .parse()
        .unwrap();
    assert!((limit_x - (track_x + 20.0 / 25.0 * track_w)).abs() < 0.05);
    assert!(ok
        .select(&select("text"))
        .all(|text| !text.text().collect::<String>().contains("Over limit")));
    // Over limit: security fill and the over note, exactly once.
    let over = document
        .select(&select("g[data-node-id='over']"))
        .next()
        .unwrap();
    assert!(over
        .select(&select("rect.ch-fill"))
        .next()
        .unwrap()
        .value()
        .attr("class")
        .unwrap()
        .contains("c-security"));
    assert_eq!(over.value().attr("data-chart-over"), Some("true"));
    assert!(over
        .select(&select("text"))
        .any(|text| text.text().collect::<String>().contains("Over limit")));
    // Limit-only row fills to the limit with an empty value attribute.
    let only = document
        .select(&select("g[data-node-id='only']"))
        .next()
        .unwrap();
    assert_eq!(only.value().attr("data-chart-value"), Some(""));
    let only_fill_w: f64 = only
        .select(&select("rect.ch-fill"))
        .next()
        .unwrap()
        .value()
        .attr("width")
        .unwrap()
        .parse()
        .unwrap();
    let only_track_w: f64 = only
        .select(&select("rect.ch-track"))
        .next()
        .unwrap()
        .value()
        .attr("width")
        .unwrap()
        .parse()
        .unwrap();
    assert!((only_fill_w / only_track_w - 0.8).abs() < 0.01);
}

#[test]
fn three_forms_pass_the_artifact_check() {
    for (form, rows) in [
        (
            "line",
            json!([
                {"id":"a","label":"A","values":[3]},
                {"id":"b","label":"B","values":[5]},
            ]),
        ),
        (
            "pie",
            json!([
                {"id":"a","label":"A","values":[2]},
                {"id":"b","label":"B","values":[1]},
            ]),
        ),
        (
            "limits",
            json!([
                {"id":"a","label":"A","value":13,"limit":20},
                {"id":"b","label":"B","limit":20},
            ]),
        ),
    ] {
        let scene = compile(json!({
            "schema_version": 1, "diagram_type": "chart",
            "meta": {"title":"T","evidence":"illustrative","locale":"en","unit":"ms"},
            "form": form,
            "rows": rows,
        }));
        let result = check::check_html(html(&scene).as_bytes(), scene.quality);
        assert!(
            result.diagnostics.is_empty(),
            "{form}: {:?}",
            result.diagnostics
        );
    }
}
