//! Browser measurement and screenshots for one inline static figure.
//!
//! The SVG never navigates alone: each theme is wrapped in the small host
//! page from the delivery plan, so theme resolution and containment are
//! measured exactly as an inline figure in a reader document.
use super::{
    cdp, discovery, evidence, observation, BrowserCheck, BrowserCoverage, BrowserOptions,
    BrowserReceipt, BrowserStatus, CheckStatus, Theme, DESKTOP_VIEWPORTS,
};
use crate::check::FileIdentity;
use crate::diagnostic::Diagnostic;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

const STATIC_CHECKS: [&str; 4] = [
    "rendered-dom",
    "viewport-containment",
    "offline-resources",
    "theme-state",
];

pub fn check(artifact: &Path, options: &BrowserOptions) -> BrowserReceipt {
    let mut receipt = BrowserReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: "browser-check".into(),
        status: BrowserStatus::Failed,
        artifact_path: artifact.to_owned(),
        artifact: FileIdentity::of(b""),
        browser_version: None,
        coverage: Vec::new(),
        checks: STATIC_CHECKS
            .iter()
            .map(|name| BrowserCheck {
                name: (*name).into(),
                status: CheckStatus::NotRun,
                scope: json!({"viewports":2}),
                evidence: Value::Null,
            })
            .collect(),
        screenshots: Vec::new(),
        diagnostics: Vec::new(),
    };
    if options.startup_timeout.is_zero()
        || options.startup_timeout > Duration::from_secs(90)
        || options.command_timeout.is_zero()
        || options.command_timeout > Duration::from_secs(15)
    {
        super::append_failure(
            &mut receipt,
            cdp::Error::new(
                "browser/options",
                "options",
                "Browser deadlines must be positive and bounded by startup=90 seconds and command=15 seconds",
            ),
        );
        return receipt;
    }
    let (artifact, svg) = match std::fs::canonicalize(artifact).and_then(|path| {
        let bytes = std::fs::read(&path)?;
        Ok((path, bytes))
    }) {
        Ok((path, bytes)) => {
            receipt.artifact_path = path.clone();
            receipt.artifact = FileIdentity::of(&bytes);
            (path, bytes)
        }
        Err(error) => {
            super::append_failure(
                &mut receipt,
                cdp::Error::new(
                    "browser/artifact",
                    "read",
                    format!("{}: {error}", artifact.display()),
                ),
            );
            return receipt;
        }
    };
    let Some(svg_text) = std::str::from_utf8(&svg).ok() else {
        super::append_failure(
            &mut receipt,
            cdp::Error::new("browser/artifact", "read", "The static SVG is not UTF-8"),
        );
        return receipt;
    };
    let executable = match discovery::find(options.executable.as_deref()) {
        Ok(path) => path,
        Err(error) => {
            if error.code == "browser/unavailable" {
                receipt.status = BrowserStatus::Unavailable;
            }
            super::append_failure(&mut receipt, error);
            return receipt;
        }
    };
    let host_directory = options
        .out_dir
        .clone()
        .or_else(|| artifact.parent().map(Path::to_owned))
        .unwrap_or_else(std::env::temp_dir);
    // Chrome needs an absolute file URL for the host page; out_dir may be relative.
    let host_directory = match std::fs::create_dir_all(&host_directory)
        .and_then(|()| std::fs::canonicalize(&host_directory))
    {
        Ok(canonical) => canonical,
        Err(error) => {
            super::append_failure(
                &mut receipt,
                cdp::Error::new(
                    "browser/host-page",
                    "host",
                    format!("{}: {error}", host_directory.display()),
                ),
            );
            return receipt;
        }
    };
    let stem = artifact
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let host_page = host_directory.join(format!("{stem}.static-host.html"));
    let mut evidence = if options.capture {
        match evidence::Evidence::new(&artifact, options.out_dir.as_deref(), Some("static")) {
            Ok(evidence) => Some(evidence),
            Err(error) => {
                super::append_failure(&mut receipt, error);
                return receipt;
            }
        }
    } else {
        None
    };
    let write_host = |theme: Theme| -> Result<(), cdp::Error> {
        let page = format!(
            "<!doctype html><html lang=\"en\" data-theme=\"{}\"><head><meta charset=\"utf-8\"><title>static figure</title><style>body{{margin:24px}}svg{{display:block;max-width:100%;height:auto}}</style></head><body>{svg_text}</body></html>",
            theme.as_str()
        );
        std::fs::write(&host_page, page).map_err(|e| {
            cdp::Error::new(
                "browser/host-page",
                "host",
                format!("{}: {e}", host_page.display()),
            )
        })
    };
    let mut browser = match cdp::Browser::launch(
        &executable,
        options.startup_timeout,
        options.command_timeout,
    ) {
        Ok(browser) => browser,
        Err(error) => {
            super::append_failure(&mut receipt, error);
            return receipt;
        }
    };
    let inspected = (|| -> Result<(), cdp::Error> {
        receipt.browser_version = Some(browser.version()?);
        browser.open_page()?;
        for theme in [Theme::Light, Theme::Dark] {
            write_host(theme)?;
            let viewport = DESKTOP_VIEWPORTS[0];
            let (metrics, mut events) = browser.inspect_static(&host_page, viewport, theme)?;
            if let Some(capture) = evidence.as_mut() {
                let png = browser.screenshot_static(viewport)?;
                events.extend(browser.take_events());
                capture.write_png(viewport, theme, true, &png)?;
            }
            let mut checks = classify(theme, &metrics, &events);
            if theme == Theme::Dark {
                if let Some(light) = receipt.coverage.first() {
                    if light.metrics["background"] == metrics["background"] {
                        for check in checks
                            .iter_mut()
                            .filter(|check| check.name == "theme-state")
                        {
                            check.status = CheckStatus::Failed;
                            check.evidence["lightDarkDiffer"] = json!(false);
                        }
                        if let Some(light) = receipt.coverage.first_mut() {
                            if let Some(previous) = light
                                .checks
                                .iter_mut()
                                .find(|check| check.name == "theme-state")
                            {
                                previous.status = CheckStatus::Failed;
                                previous.evidence["lightDarkDiffer"] = json!(false);
                            }
                            light.status = BrowserStatus::Failed;
                        }
                    }
                }
            }
            let status = if checks.iter().all(|check| check.status == CheckStatus::Pass) {
                BrowserStatus::Pass
            } else {
                BrowserStatus::Failed
            };
            receipt.coverage.push(BrowserCoverage {
                viewport,
                theme,
                status,
                vertical_scroll_accepted: true,
                overflow_disposition: "horizontal-forbidden".into(),
                metrics,
                checks,
            });
        }
        Ok(())
    })();
    let closed = browser.close();
    let trailing = browser.take_events();
    if let Some(last) = receipt.coverage.last_mut() {
        let observed = classify(last.theme, &last.metrics, &trailing);
        for check in observed
            .into_iter()
            .filter(|check| check.status == CheckStatus::Failed)
        {
            if let Some(previous) = last
                .checks
                .iter_mut()
                .find(|previous| previous.name == check.name)
            {
                if previous.status == CheckStatus::Pass {
                    *previous = check;
                    last.status = BrowserStatus::Failed;
                }
            }
        }
    }
    receipt.checks = observation::summarize_named(&receipt.coverage, &STATIC_CHECKS, 2);
    let inspected_ok = inspected.is_ok();
    if let Err(error) = inspected {
        super::append_failure(&mut receipt, error);
    }
    let host_removed = match std::fs::remove_file(&host_page) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
    };
    let closed_ok = closed.is_ok();
    if let Err(error) = closed {
        super::append_failure(&mut receipt, error);
    }
    receipt.checks.push(BrowserCheck {
        name: "browser-cleanup".into(),
        status: if closed_ok && host_removed {
            CheckStatus::Pass
        } else {
            CheckStatus::Failed
        },
        scope: json!({"ownedChild":true,"ownedProfile":true,"hostPageRemoved":true}),
        evidence: json!({"hostPage":host_page}),
    });
    if let Some(evidence) = evidence {
        receipt.screenshots = evidence.screenshots;
        let captured = receipt.screenshots.len() == 2;
        receipt.checks.push(BrowserCheck {
            name: "screenshot-capture".into(),
            status: if captured {
                CheckStatus::Pass
            } else {
                CheckStatus::Failed
            },
            scope: json!({"required":2,"themes":["light","dark"],"viewports":[DESKTOP_VIEWPORTS[0]],"longDiagrams":"full-page"}),
            evidence: json!({"captured":receipt.screenshots.len()}),
        });
    }
    for coverage in &receipt.coverage {
        for check in &coverage.checks {
            if check.status == CheckStatus::Failed {
                let mut diagnostic = Diagnostic::error(
                    None,
                    &format!("browser/{}", check.name),
                    "",
                    format!(
                        "{} failed at {}x{} ({})",
                        check.name,
                        coverage.viewport.width,
                        coverage.viewport.height,
                        coverage.theme.as_str()
                    ),
                );
                diagnostic.stage = "browser".into();
                diagnostic.evidence = json!({"viewport":coverage.viewport,"theme":coverage.theme,"scope":check.scope,"measurement":check.evidence});
                receipt.diagnostics.push(diagnostic);
            }
        }
    }
    if inspected_ok
        && closed_ok
        && host_removed
        && receipt.coverage.len() == 2
        && receipt
            .checks
            .iter()
            .all(|check| check.status == CheckStatus::Pass)
        && receipt.diagnostics.is_empty()
    {
        receipt.status = BrowserStatus::Pass;
    }
    receipt
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}

fn valid_rect(value: &Value) -> bool {
    ["left", "top", "right", "bottom", "width", "height"]
        .iter()
        .all(|key| number(value, key).is_some())
        && number(value, "width").is_some_and(|number| number > 0.0)
        && number(value, "height").is_some_and(|number| number > 0.0)
}

fn array_is_empty(metrics: &Value, key: &str) -> bool {
    metrics
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn valid_view_box(value: &Value) -> bool {
    ["x", "y", "width", "height"]
        .iter()
        .all(|key| number(value, key).is_some())
        && number(value, "width").is_some_and(|number| number > 0.0)
        && number(value, "height").is_some_and(|number| number > 0.0)
}

fn font_stack(value: &Value) -> Vec<String> {
    value
        .as_str()
        .map(|family| {
            family
                .split(',')
                .map(|entry| {
                    entry
                        .trim()
                        .trim_matches(|character| character == '\'' || character == '"')
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn classify(theme: Theme, metrics: &Value, events: &[Value]) -> Vec<BrowserCheck> {
    let mut checks = Vec::new();
    let mut add = |name: &str, pass: bool, scope: Value, evidence: Value| {
        checks.push(BrowserCheck {
            name: name.into(),
            status: if pass {
                CheckStatus::Pass
            } else {
                CheckStatus::Failed
            },
            scope,
            evidence,
        });
    };
    let viewport = DESKTOP_VIEWPORTS[0];

    let inner_width = number(metrics, "innerWidth");
    let inner_height = number(metrics, "innerHeight");
    let scroll_width = number(metrics, "scrollWidth");
    let complete = inner_width == Some(f64::from(viewport.width))
        && inner_height == Some(f64::from(viewport.height))
        && scroll_width.is_some_and(|number| number > 0.0);
    let overflow_x = scroll_width
        .zip(inner_width)
        .is_none_or(|(scroll, inner)| scroll > inner);
    add(
        "viewport-containment",
        complete && !overflow_x,
        json!({"horizontalOverflow":false,"viewport":viewport}),
        json!({"complete":complete,"innerWidth":inner_width,"innerHeight":inner_height,"scrollWidth":scroll_width,"overflowX":overflow_x}),
    );

    let nodes = metrics["nodes"].as_array();
    let nodes_ok = nodes.is_some_and(|nodes| {
        !nodes.is_empty()
            && nodes.iter().all(|node| {
                node["visible"] == true
                    && valid_rect(&node["bounds"])
                    && node["bodies"].as_array().is_some_and(|bodies| {
                        !bodies.is_empty() && bodies.iter().all(|body| body["visible"] == true)
                    })
                    && node["labels"].as_array().is_some_and(|labels| {
                        !labels.is_empty() && labels.iter().all(|label| label["visible"] == true)
                    })
            })
    });
    let edges_ok = metrics["edges"].as_array().is_some_and(|edges| {
        edges.iter().all(|edge| {
            edge["visible"] == true && number(edge, "length").is_some_and(|length| length > 0.0)
        })
    });
    let text = metrics["text"].as_array();
    let text_ok = text.is_some_and(|entries| {
        !entries.is_empty()
            && entries.iter().all(|entry| {
                entry["visible"] == true
                    && valid_rect(&entry["bounds"])
                    && entry["text"]
                        .as_str()
                        .is_some_and(|text| !text.trim().is_empty())
            })
    });
    let dom_ok = metrics["figures"] == 1
        && metrics["scripts"] == 0
        && valid_rect(&metrics["bounds"])
        && valid_view_box(&metrics["viewBox"])
        && number(metrics, "nodeCount").is_some_and(|count| {
            count > 0.0 && nodes.is_some_and(|nodes| count == nodes.len() as f64)
        })
        && number(metrics, "edgeCount").is_some_and(|count| {
            metrics["edges"]
                .as_array()
                .is_some_and(|edges| count == edges.len() as f64)
        })
        && metrics["rootRole"] == "img"
        && metrics["accessibleName"]
            .as_str()
            .is_some_and(|name| !name.trim().is_empty())
        && nodes_ok
        && edges_ok
        && text_ok
        && array_is_empty(metrics, "nodeTextClips")
        && array_is_empty(metrics, "canvasTextClips")
        && array_is_empty(metrics, "forbiddenElements")
        && array_is_empty(metrics, "eventHooks")
        && array_is_empty(metrics, "interactiveSemantics")
        && array_is_empty(metrics, "controls")
        && array_is_empty(metrics, "tabStops")
        && array_is_empty(metrics, "tabindexAttributes");
    add(
        "rendered-dom",
        dom_ok,
        json!({"staticFigures":1,"scripts":0,"accessibleName":true,"interactiveSemantics":false,"textClips":false}),
        json!({"figures":metrics["figures"],"scripts":metrics["scripts"],"bounds":metrics["bounds"],"viewBox":metrics["viewBox"],"nodeCount":metrics["nodeCount"],"edgeCount":metrics["edgeCount"],"minimumProjectedTextPx":metrics["minimumProjectedTextPx"],"nodeTextClips":metrics["nodeTextClips"],"canvasTextClips":metrics["canvasTextClips"],"forbiddenElements":metrics["forbiddenElements"],"eventHooks":metrics["eventHooks"],"interactiveSemantics":metrics["interactiveSemantics"],"controls":metrics["controls"],"tabStops":metrics["tabStops"],"tabindexAttributes":metrics["tabindexAttributes"]}),
    );

    let requests: Vec<_> = events
        .iter()
        .filter(|event| event["method"] == "Network.requestWillBeSent")
        .filter_map(|event| event.pointer("/params/request/url").and_then(Value::as_str))
        .filter(|url| unexpected_resource(url, metrics["documentUrl"].as_str()))
        .map(str::to_owned)
        .collect();
    let failed_resources: Vec<_> = events
        .iter()
        .filter(|event| event["method"] == "Network.loadingFailed")
        .cloned()
        .collect();
    let remote_resources: Vec<_> = metrics["resources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry["name"]
                .as_str()
                .is_none_or(|url| unexpected_resource(url, metrics["documentUrl"].as_str()))
        })
        .cloned()
        .collect();
    let images_ok = metrics["images"].as_array().is_some_and(|images| {
        images.iter().all(|image| {
            image["complete"] == true
                && number(image, "width").is_some_and(|width| width > 0.0)
                && number(image, "height").is_some_and(|height| height > 0.0)
        })
    });
    let font_faces = metrics["fontFaces"].as_array();
    let font_policy = metrics["fontPolicy"].as_str();
    let fonts_ok = match font_policy {
        Some("system") => {
            array_is_empty(metrics, "fontResources")
                && font_faces.is_some_and(Vec::is_empty)
                && text.is_some_and(|entries| {
                    !entries.is_empty()
                        && entries.iter().all(|entry| {
                            let stack = font_stack(&entry["fontFamily"]);
                            stack.len() == 5
                                && stack[0] == "ui-monospace"
                                && stack[1] == "SFMono-Regular"
                                && stack[2] == "Menlo"
                                && stack[3] == "Consolas"
                                && stack[4] == "monospace"
                        })
                })
        }
        Some("embedded") => {
            font_faces.is_some_and(|faces| {
                !faces.is_empty()
                    && faces.iter().all(|face| {
                        face["dataWoff2"] == true
                            && face["local"] == false
                            && face["family"].as_str().is_some_and(|family| {
                                family
                                    .trim()
                                    .trim_matches(|character| character == '\'' || character == '"')
                                    == "JetBrains Mono"
                            })
                    })
            }) && text.is_some_and(|entries| {
                !entries.is_empty()
                    && entries.iter().all(|entry| {
                        font_stack(&entry["fontFamily"]).first().map(String::as_str)
                            == Some("JetBrains Mono")
                    })
            })
        }
        _ => false,
    };
    let resources_ok = requests.is_empty()
        && failed_resources.is_empty()
        && remote_resources.is_empty()
        && images_ok
        && metrics["resources"].is_array()
        && metrics["fontsStatus"] == "loaded"
        && array_is_empty(metrics, "fontFaceErrors")
        && array_is_empty(metrics, "externalReferences")
        && fonts_ok;
    add(
        "offline-resources",
        resources_ok,
        json!({"remoteSchemes":"blocked and failed","fontsReady":true,"fontPolicy":"embedded|system"}),
        json!({"remoteRequests":requests,"loadingFailed":failed_resources,"remoteResources":remote_resources,"externalReferences":metrics["externalReferences"],"images":metrics["images"],"fontsStatus":metrics["fontsStatus"],"fontPolicy":font_policy,"fontFaces":metrics["fontFaces"],"fontResources":metrics["fontResources"],"fontFaceErrors":metrics["fontFaceErrors"]}),
    );

    let background = metrics["background"].as_str();
    let background_painted = background
        .is_some_and(|color| !color.trim().is_empty() && color != "none" && color != "transparent");
    add(
        "theme-state",
        metrics["resolvedTheme"] == theme.as_str() && background_painted,
        json!({"requested":theme,"background":"painted","lightDarkDiffer":true}),
        json!({"resolved":metrics["resolvedTheme"],"background":metrics["background"]}),
    );
    checks
}

fn unexpected_resource(resource: &str, document: Option<&str>) -> bool {
    let Ok(mut url) = url::Url::parse(resource) else {
        return true;
    };
    match url.scheme() {
        "data" | "blob" => false,
        "about" => resource != "about:blank",
        "file" => {
            let Some(Ok(mut document)) = document.map(url::Url::parse) else {
                return true;
            };
            url.set_query(None);
            url.set_fragment(None);
            document.set_query(None);
            document.set_fragment(None);
            url != document
        }
        _ => true,
    }
}
