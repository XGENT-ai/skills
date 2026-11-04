use phoenix_diagram::check::check_html;
use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType};
use phoenix_diagram::layout;
use phoenix_diagram::render::{render, EmbeddedAssets};
use phoenix_diagram::scene::{CompiledDiagram, Rect};
use scraper::{Html, Selector};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/statechart/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn example() -> Value {
    let path = format!(
        "{}/../../skills/phoenix-diagram/examples/minimal.statechart.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn compile(document: &Value) -> Result<CompiledDiagram, phoenix_diagram::diagnostic::Diagnostics> {
    let options = CompileOptions::default();
    let input = parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Statechart,
        &options,
    )
    .unwrap();
    layout::compile(&input, &options)
}

fn html(scene: &CompiledDiagram) -> String {
    String::from_utf8(render(scene, &EmbeddedAssets::builtin()).unwrap()).unwrap()
}

fn findings(html: &str, scene: &CompiledDiagram) -> Vec<String> {
    check_html(html.as_bytes(), scene.quality)
        .diagnostics
        .iter()
        .map(|d| format!("{} {} {}", d.code, d.subject.path, d.evidence))
        .collect()
}

/// Pairs of straight runs from different arrows that a reader would take for
/// one arrow: parallel with no room for a label between them for 40px or
/// more, or on one line with less than 100px between them.
fn crowded(scene: &CompiledDiagram) -> usize {
    let runs: Vec<Vec<(bool, f64, f64, f64)>> = scene
        .relations
        .iter()
        .map(|relation| {
            relation
                .points
                .windows(2)
                .filter_map(|pair| {
                    let (a, b) = (pair[0], pair[1]);
                    if (a.y - b.y).abs() < 0.5 && (a.x - b.x).abs() >= 0.5 {
                        Some((true, a.y, a.x.min(b.x), a.x.max(b.x)))
                    } else if (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() >= 0.5 {
                        Some((false, a.x, a.y.min(b.y), a.y.max(b.y)))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    let mut count = 0;
    for (i, left) in runs.iter().enumerate() {
        for right in &runs[i + 1..] {
            for a in left {
                for b in right.iter().filter(|b| b.0 == a.0) {
                    let apart = (a.1 - b.1).abs();
                    let shared = a.3.min(b.3) - a.2.max(b.2);
                    if (apart < 12.0 && shared >= 40.0)
                        || (apart <= 3.0 && shared < 0.0 && -shared < 100.0)
                    {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Arrows whose straight runs pass within 8px of a state they do not connect.
fn hugging(scene: &CompiledDiagram) -> Vec<String> {
    let mut found = Vec::new();
    for relation in &scene.relations {
        for pair in relation.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let run = Rect {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
                width: (a.x - b.x).abs(),
                height: (a.y - b.y).abs(),
            };
            for node in scene
                .nodes
                .iter()
                .filter(|node| node.authored.id != relation.from && node.authored.id != relation.to)
            {
                let b = node.bounds;
                if run.x < b.x + b.width + 8.0
                    && run.x + run.width > b.x - 8.0
                    && run.y < b.y + b.height + 8.0
                    && run.y + run.height > b.y - 8.0
                {
                    found.push(format!(
                        "{} near {}",
                        relation.authored.id, node.authored.id
                    ));
                }
            }
        }
    }
    found
}

fn overlaps(a: Rect, b: Rect, gap: f64) -> bool {
    a.x < b.x + b.width + gap
        && a.x + a.width > b.x - gap
        && a.y < b.y + b.height + gap
        && a.y + a.height > b.y - gap
}

/// Arrow labels that sit against a state, where they read as belonging to it
/// or hide its bug-path ring, or that cover the last 14px of an arrow, where
/// the arrowhead shows which way it points.
fn misplaced_labels(scene: &CompiledDiagram) -> Vec<String> {
    let heads: Vec<(String, Rect)> = scene
        .relations
        .iter()
        .filter_map(|relation| {
            let [.., a, b] = relation.points.as_slice() else {
                return None;
            };
            let (dx, dy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
            let tail = (b.x - dx * 14.0, b.y - dy * 14.0);
            Some((
                relation.authored.id.clone(),
                Rect {
                    x: b.x.min(tail.0) - 4.0 * dy.abs(),
                    y: b.y.min(tail.1) - 4.0 * dx.abs(),
                    width: (b.x - tail.0).abs() + 8.0 * dy.abs(),
                    height: (b.y - tail.1).abs() + 8.0 * dx.abs(),
                },
            ))
        })
        .collect();
    let mut found = Vec::new();
    for relation in scene.relations.iter().filter(|r| r.from != r.to) {
        let Some(plate) = relation.label_plate else {
            continue;
        };
        for node in &scene.nodes {
            if overlaps(plate, node.bounds, 6.0) {
                found.push(format!(
                    "{} against {}",
                    relation.authored.id, node.authored.id
                ));
            }
        }
        for (id, head) in &heads {
            if overlaps(plate, *head, 0.0) {
                found.push(format!("{} covers the head of {id}", relation.authored.id));
            }
        }
    }
    found
}

fn inside(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

#[test]
fn every_drawable_fixture_passes_the_artifact_gates() {
    for (name, document) in [
        "checkout",
        "login-parents",
        "login-regions",
        "two-phase",
        "deadlock",
        "mixed-groups",
        "retry",
        "retry-merged",
        "retry-split",
    ]
    .map(|name| (name, fixture(name)))
    .into_iter()
    .chain([("minimal", example())])
    {
        let scene = compile(&document).unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        let html = html(&scene);
        assert_eq!(findings(&html, &scene), Vec::<String>::new(), "{name}");
        assert_eq!(crowded(&scene), 0, "{name}: arrows that read as one");
        assert_eq!(
            hugging(&scene),
            Vec::<String>::new(),
            "{name}: arrows along other states"
        );
        assert_eq!(
            misplaced_labels(&scene),
            Vec::<String>::new(),
            "{name}: labels against states or arrowheads"
        );
        // A phone shows the chart at its own width; it must not be far taller.
        assert!(
            scene.view_box.height <= 1.75 * scene.view_box.width,
            "{name}: {:?}",
            scene.view_box
        );
        // Every chart state sits inside the frames that own it.
        for frame in &scene.containers {
            for member in &frame.members {
                let node = scene
                    .nodes
                    .iter()
                    .find(|node| &node.authored.id == member)
                    .unwrap();
                assert!(
                    inside(node.bounds, frame.bounds),
                    "{name}: {member} leaves {}",
                    frame.authored.id
                );
            }
        }
    }
}

#[test]
fn a_chart_too_dense_to_read_asks_for_a_smaller_view() {
    for name in ["diehard", "limit"] {
        let errors = compile(&fixture(name)).expect_err(name);
        assert_eq!(errors[0].code, "statechart/chart-too-large", "{name}");
        assert!(!errors[0].supported_fixes.is_empty());
    }
}

#[test]
fn marks_frames_and_start_markers_follow_the_projection() {
    let scene = compile(&fixture("login-parents")).unwrap();
    let parent = scene
        .containers
        .iter()
        .find(|frame| frame.authored.kind == "statechart-parent")
        .unwrap();
    assert_eq!(parent.members.len(), 4);
    let document = Html::parse_document(&html(&compile(&fixture("checkout")).unwrap()));
    let select = |query: &str| Selector::parse(query).unwrap();
    let paying = document
        .select(&select("g[data-node-label=\"支付中\"]"))
        .next()
        .unwrap();
    assert_eq!(
        paying.value().attr("data-statechart-mark"),
        Some("may-break")
    );
    assert!(paying.value().attr("data-statechart-bug-path").is_some());
    assert!(paying
        .select(&select("text"))
        .any(|text| text.text().collect::<String>() == "可能违反 chargedOnce"));
    assert_eq!(
        document.select(&select("[data-statechart-start]")).count(),
        1
    );
    assert_eq!(
        document
            .select(&select(
                "[data-composition-frame-kind=\"statechart-region\"]"
            ))
            .count(),
        0
    );
    let regions = Html::parse_document(&html(&compile(&fixture("login-regions")).unwrap()));
    assert_eq!(
        regions
            .select(&select(
                "[data-composition-frame-kind=\"statechart-region\"]"
            ))
            .count(),
        2
    );
    assert!(regions
        .select(&select("path[data-edge-label=\"登录 [除非 已锁定]\"]"))
        .next()
        .is_some());
}

#[test]
fn a_tampered_mark_or_a_missing_arrow_fails_the_check() {
    let scene = compile(&fixture("checkout")).unwrap();
    let original = html(&scene);
    assert!(findings(&original, &scene).is_empty());
    let remarked = original.replacen(
        " data-statechart-mark=\"may-break\"",
        " data-statechart-mark=\"breaks\"",
        1,
    );
    assert!(findings(&remarked, &scene)
        .iter()
        .any(|finding| finding.starts_with("statechart/artifact-mismatch")));
    let start = original.find("<path data-edge-from").unwrap();
    let end = start + original[start..].find("/>").unwrap() + 2;
    let unarrowed = format!("{}{}", &original[..start], &original[end..]);
    assert!(findings(&unarrowed, &scene)
        .iter()
        .any(|finding| finding.starts_with("statechart/artifact-mismatch")));
}

#[test]
fn the_report_cards_lead_and_author_cards_follow() {
    let mut document = fixture("checkout");
    document["cards"] = serde_json::json!([{"dot":"cyan","title":"备注","items":["作者卡片"]}]);
    let page = Html::parse_document(&html(&compile(&document).unwrap()));
    let select = |query: &str| Selector::parse(query).unwrap();
    let titles: Vec<String> = page
        .select(&select(".cards > .card h3"))
        .map(|title| title.text().collect())
        .collect();
    assert_eq!(
        titles,
        [
            "概况",
            "违反：一笔订单最多扣一次款（chargedOnce）",
            "成立的规则",
            "备注"
        ]
    );
    let items: Vec<String> = page
        .select(&select(".cards > .card:nth-child(2) li"))
        .map(|item| item.text().collect())
        .collect();
    assert_eq!(
        items,
        [
            "最短路径 3 步",
            "0. 购物车",
            "1. 去结账：支付中",
            "2. 网关扣款：支付中",
            "3. 网关又扣了一次：支付中"
        ]
    );
}
