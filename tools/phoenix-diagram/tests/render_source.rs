use phoenix_diagram::{
    check::check_html,
    input::{parse_input, CompileOptions, DiagramType, InputContract, Quality},
    layout,
    render::{render, EmbeddedAssets},
};
use scraper::{Html, Selector};
use serde_json::Value;

const TEMPLATE: &str = r#"<!doctype html><html lang="{{PHOENIX:LOCALE}}" data-preset="{{PHOENIX:PRESET}}"><head><meta charset="utf-8"><title>{{PHOENIX:TITLE}}</title></head><body><h1>{{PHOENIX:HEADING}}</h1>{{PHOENIX:SUBTITLE}}<div class="diagram-container">{{PHOENIX:SVG}}</div>{{PHOENIX:CARDS}}<script id="phoenix-diagram-metadata" type="application/json">{{PHOENIX:METADATA}}</script></body></html>"#;

fn inspect(document: &Value, kind: DiagramType, contract: &str, quality: Quality) -> Vec<String> {
    let options = CompileOptions {
        input_contract: Some(if contract == "legacy-2.17" {
            InputContract::Legacy217
        } else {
            InputContract::PhoenixV1
        }),
        quality: Some(quality),
        ..Default::default()
    };
    let input = parse_input(&serde_json::to_vec(document).unwrap(), kind, &options).unwrap();
    let scene = layout::compile(&input, &options).unwrap();
    let html = render(&scene, &EmbeddedAssets { template: TEMPLATE }).unwrap();
    let result = check_html(&html, quality);
    result
        .diagnostics
        .iter()
        .filter(|d| d.severity == phoenix_diagram::diagnostic::Severity::Error)
        .map(|d| format!("{} {} {}", d.code, d.subject.path, d.evidence))
        .collect()
}

#[test]
fn compiled_architecture_svgs_pass_the_independent_artifact_checks() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/architecture/oracle.json")).unwrap();
    let mut failures = Vec::new();
    for case in oracle["cases"].as_array().unwrap() {
        for contract in ["legacy-2.17", "phoenix-v1"] {
            if case["oracles"][contract]["success"] != true {
                continue;
            }
            let quality = if case["input"]["meta"]["quality_profile"] == "showcase" {
                Quality::Showcase
            } else {
                Quality::Standard
            };
            let errors = inspect(&case["input"], DiagramType::Architecture, contract, quality);
            if !errors.is_empty() {
                failures.push(format!(
                    "{contract}/{}: {}",
                    case["name"],
                    errors.join("; ")
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} artifact failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn compiled_workflow_svgs_pass_the_independent_artifact_checks() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/workflow/oracle.json")).unwrap();
    let mut failures = Vec::new();
    for case in oracle["cases"].as_array().unwrap() {
        if case["ok"] != true {
            continue;
        }
        let quality = if case["quality"] == "showcase" {
            Quality::Showcase
        } else {
            Quality::Standard
        };
        let errors = inspect(
            &case["document"],
            DiagramType::Workflow,
            case["contract"].as_str().unwrap(),
            quality,
        );
        if !errors.is_empty() {
            failures.push(format!(
                "{}/{}: {}",
                case["contract"],
                case["name"],
                errors.join("; ")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} artifact failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn architecture_presentation_matches_the_real_frozen_svg() {
    let oracle: Value = serde_json::from_str(include_str!(
        "fixtures/architecture/presentation-oracle.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["ok"] == true)
    {
        let contract = if case["contract"] == "legacy-2.17" {
            InputContract::Legacy217
        } else {
            InputContract::PhoenixV1
        };
        let quality = if case["quality"] == "showcase" {
            Quality::Showcase
        } else {
            Quality::Standard
        };
        let options = CompileOptions {
            input_contract: Some(contract),
            quality: Some(quality),
            ..Default::default()
        };
        let input = parse_input(
            &serde_json::to_vec(&case["document"]).unwrap(),
            DiagramType::Architecture,
            &options,
        )
        .unwrap();
        let scene = layout::compile(&input, &options).unwrap();
        let html = render(&scene, &EmbeddedAssets { template: TEMPLATE }).unwrap();
        let dom = Html::parse_document(std::str::from_utf8(&html).unwrap());
        let key = format!("{}/{}", case["contract"], case["name"]);
        let repaired_footer = contract == InputContract::PhoenixV1
            && case["name"] == "production-deployment.architecture";
        for expected in case["geometry"]["texts"].as_array().unwrap() {
            let is_legend_text = expected["content"] == "Legend"
                || case["geometry"]["legends"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|legend| legend["texts"].as_array().unwrap().contains(expected));
            let mut expected = expected.clone();
            if repaired_footer {
                // This one formerly occluded title now sits 2px below the
                // child frame (bottom 660); its 16px height and 2px inset
                // grow the automatic canvas and its legend by exactly 20px.
                if expected["content"] == "AWS eu-west-1 / disaster recovery" {
                    expected["y"] = Value::from(662.0 + expected["font"].as_f64().unwrap() + 4.0);
                } else if is_legend_text {
                    expected["y"] = Value::from(expected["y"].as_f64().unwrap() + 20.0);
                }
            }
            let found = dom.select(&Selector::parse("svg text").unwrap()).any(|t| {
                t.text().collect::<String>() == expected["content"].as_str().unwrap()
                    && [("x", "x"), ("y", "y"), ("font-size", "font")].iter().all(
                        |(attr, field)| {
                            t.value()
                                .attr(attr)
                                .and_then(|v| v.parse::<f64>().ok())
                                .is_some_and(|v| {
                                    (v - expected[field].as_f64().unwrap()).abs() < 0.00001
                                })
                        },
                    )
            });
            if !found {
                failures.push(format!("{key}: missing exact text {expected}"));
            }
        }
        for legend in case["geometry"]["legends"].as_array().unwrap() {
            let kind = legend["kind"].as_str().unwrap();
            let found = dom
                .select(&Selector::parse("g[data-graph-role='legend']").unwrap())
                .find(|e| e.value().attr("data-legend-semantic-kind") == Some(kind));
            for expected in legend["swatches"].as_array().unwrap() {
                let mut expected = expected.clone();
                if repaired_footer {
                    expected["y"] = Value::from(expected["y"].as_f64().unwrap() + 20.0);
                }
                if !found.is_some_and(|g| {
                    g.select(&Selector::parse("rect").unwrap()).any(|r| {
                        r.value().attr("class") == expected["class"].as_str()
                            && ["x", "y", "width", "height", "rx"].iter().all(|attr| {
                                r.value()
                                    .attr(attr)
                                    .and_then(|v| v.parse::<f64>().ok())
                                    .is_some_and(|v| {
                                        (v - expected[attr].as_f64().unwrap()).abs() < 0.00001
                                    })
                            })
                    })
                }) {
                    failures.push(format!("{key}: missing real {kind} swatch"));
                }
            }
        }
        for expected in case["geometry"]["edges"].as_array().unwrap() {
            let edge = dom
                .select(&Selector::parse("path[data-graph-role='edge']").unwrap())
                .find(|e| {
                    e.value()
                        .attr("data-edge-key")
                        .and_then(|v| v.parse::<u64>().ok())
                        == expected["key"].as_u64()
                })
                .unwrap();
            let actual =
                phoenix_diagram::check::path::analyze_svg_path(edge.value().attr("d").unwrap())
                    .unwrap();
            let source =
                phoenix_diagram::check::path::analyze_svg_path(expected["d"].as_str().unwrap())
                    .unwrap();
            let actual_points: Vec<_> = actual
                .subpaths
                .iter()
                .flat_map(|p| p.points.iter())
                .collect();
            let source_points: Vec<_> = source
                .subpaths
                .iter()
                .flat_map(|p| p.points.iter())
                .collect();
            if actual_points.len() != source_points.len()
                || actual_points
                    .iter()
                    .zip(source_points)
                    .any(|(a, b)| (a.x - b.x).abs() > 0.00001 || (a.y - b.y).abs() > 0.00001)
            {
                failures.push(format!(
                    "{key}: actual rounded route differs for edge {}",
                    expected["key"]
                ));
            }
        }
        for legend in &scene.legends {
            if let Some(value) = input.document.pointer(&legend.authored.source_path) {
                if value != &legend.authored.facts {
                    failures.push(format!(
                        "{key}: derived legend facts replaced authored value at {}",
                        legend.authored.source_path
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} presentation differences:\n{}",
        failures.len(),
        failures
            .iter()
            .take(16)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn architecture_default_showcase_enforces_the_same_geometry_as_persisted_showcase() {
    let document = serde_json::json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Ownership","output":"architecture.html","legend":{"mode":"hidden"}},"components":[{"id":"a","type":"backend","label":"A","pos":[80,120]},{"id":"b","type":"backend","label":"B","pos":[240,120]},{"id":"c","type":"database","label":"C","pos":[400,120]}],"connections":[],"boundaries":[{"kind":"region","label":"Owned","wraps":["a","c"]}]});
    for contract in [InputContract::Legacy217, InputContract::PhoenixV1] {
        let options = CompileOptions {
            input_contract: Some(contract),
            ..Default::default()
        };
        let parsed = parse_input(
            &serde_json::to_vec(&document).unwrap(),
            DiagramType::Architecture,
            &options,
        )
        .unwrap();
        let default = layout::compile(&parsed, &options);
        let mut marked = document.clone();
        marked["meta"]["quality_profile"] = serde_json::json!("showcase");
        let parsed = parse_input(
            &serde_json::to_vec(&marked).unwrap(),
            DiagramType::Architecture,
            &options,
        )
        .unwrap();
        let persisted = layout::compile(&parsed, &options);
        assert_eq!(
            default.is_ok(),
            persisted.is_ok(),
            "{contract:?}: default bypassed showcase geometry gates"
        );
    }
}
