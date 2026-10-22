//! Assertions over measured DOM facts, separate from transport execution.
use super::{
    BrowserCheck, BrowserCoverage, BrowserStatus, CheckStatus, Theme, Viewport,
    BROWSER_MEASUREMENT_CHECKS,
};
use serde_json::{json, Value};

pub(super) fn pending_checks() -> Vec<BrowserCheck> {
    BROWSER_MEASUREMENT_CHECKS
        .iter()
        .map(|name| BrowserCheck {
            name: (*name).into(),
            status: CheckStatus::NotRun,
            scope: json!({"viewports":10}),
            evidence: Value::Null,
        })
        .collect()
}

fn number(metrics: &Value, key: &str) -> Option<f64> {
    metrics
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}
fn array_is_empty(metrics: &Value, key: &str) -> bool {
    metrics
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}
fn valid_rect(value: &Value) -> bool {
    ["left", "top", "right", "bottom", "width", "height"]
        .iter()
        .all(|key| number(value, key).is_some())
        && number(value, "width").is_some_and(|n| n > 0.0)
        && number(value, "height").is_some_and(|n| n > 0.0)
}

pub(super) fn classify(
    viewport: Viewport,
    theme: Theme,
    metrics: Value,
    events: &[Value],
) -> BrowserCoverage {
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
        })
    };
    let inner_width = number(&metrics, "innerWidth");
    let inner_height = number(&metrics, "innerHeight");
    let scroll_width = number(&metrics, "scrollWidth");
    let scroll_height = number(&metrics, "scrollHeight");
    let complete = inner_width == Some(f64::from(viewport.width))
        && inner_height == Some(f64::from(viewport.height))
        && scroll_width.is_some_and(|n| n > 0.0)
        && scroll_height.is_some_and(|n| n > 0.0);
    let overflow_x = scroll_width
        .zip(inner_width)
        .is_none_or(|(scroll, inner)| scroll > inner);
    let overflow_y = scroll_height
        .zip(inner_height)
        .is_none_or(|(scroll, inner)| scroll > inner);
    let min_text = number(&metrics, "minimumProjectedNodeTextPx");
    let readability_ok = min_text.is_some_and(|number| number >= 6.0);
    let reader_fit = metrics["readerFit"].as_str();
    let unclipped = metrics["documentScrollUnclipped"] == true;
    let authored_architecture =
        reader_fit == Some("authored-height") && metrics["diagramType"] == "architecture";
    let automatic = metrics["readerLayout"] == "adaptive"
        && metrics["readerOverflow"] == "authored"
        && matches!(reader_fit, Some("intrinsic-height" | "width-first"));
    let vertical_scroll_accepted = overflow_y
        && !overflow_x
        && readability_ok
        && unclipped
        && (automatic || authored_architecture);
    let containment = complete
        && metrics["readerFlow"]["unclipped"] == true
        && !overflow_x
        && (!overflow_y || vertical_scroll_accepted)
        && (!authored_architecture || unclipped);
    add(
        "viewport-containment",
        containment,
        json!({"policy":"fit-or-reader-declared-readable-vertical-scroll"}),
        json!({"complete":complete,"overflowX":overflow_x,"overflowY":overflow_y,"verticalScrollAccepted":vertical_scroll_accepted,"documentScrollUnclipped":unclipped,"readerFlow":metrics["readerFlow"]}),
    );
    add(
        "theme-state",
        metrics["resolvedTheme"] == theme.as_str(),
        json!({"requested":theme}),
        json!({"resolved":metrics["resolvedTheme"]}),
    );
    let dom_ok = metrics["svgCount"] == 1
        && number(&metrics, "fullScrollHeight")
            .is_some_and(|height| height >= f64::from(viewport.height))
        && ["x", "y", "width", "height"]
            .iter()
            .all(|key| number(&metrics["viewBox"], key).is_some())
        && number(&metrics["viewBox"], "width").is_some_and(|width| width > 0.0)
        && number(&metrics["viewBox"], "height").is_some_and(|height| height > 0.0)
        && number(&metrics, "nodeCount").is_some_and(|n| n > 0.0)
        && valid_rect(&metrics["readerRect"])
        && valid_rect(&metrics["panelRect"])
        && valid_rect(&metrics["svgRect"])
        && metrics["semanticVisibility"]["ok"] == true;
    add(
        "rendered-dom",
        dom_ok,
        json!({"mainSvgCount":1,"nodeCount":"nonempty","readerPanelCanvas":true,"semanticVisibility":"all authored node bodies, primary labels and paths; context/fine at Full; fitted overview restored"}),
        json!({"svgCount":metrics["svgCount"],"nodeCount":metrics["nodeCount"],"reader":metrics["readerRect"],"panel":metrics["panelRect"],"svg":metrics["svgRect"],"semanticVisibility":metrics["semanticVisibility"]}),
    );
    // The frozen 6px floor is a desktop contract. Narrow samples still report
    // every projected text bound and require real visible, finite text.
    let text = metrics["text"].as_array();
    let finite_text = text.is_some_and(|entries| {
        !entries.is_empty()
            && entries.iter().all(|entry| {
                number(entry, "projectedFontPx").is_some_and(|n| n > 0.0)
                    && valid_rect(&entry["bounds"])
            })
    });
    let desktop = viewport.width >= 960;
    add(
        "projected-text",
        finite_text && (!desktop || readability_ok),
        json!({"minimumPx":if desktop {Some(6)} else {None},"measurement":"rendered source font projected at actual SVG width","desktopFloorApplicable":desktop}),
        json!({"minimumProjectedPx":min_text,"minimum":metrics["minimumProjectedNodeText"]}),
    );
    add(
        "text-containment",
        array_is_empty(&metrics, "nodeTextClips") && array_is_empty(&metrics, "canvasTextClips"),
        json!({"nodeCards":true,"canvas":true}),
        json!({"nodes":metrics["nodeTextClips"],"canvas":metrics["canvasTextClips"]}),
    );
    let cards_ok = metrics["cards"].as_array().is_some_and(|cards| {
        cards.iter().all(|card| {
            card["clippedText"] == false && number(card, "overlapWithSvg").is_some_and(|n| n <= 0.5)
        })
    });
    add(
        "card-clearance",
        cards_ok,
        json!({"visibleCards":true}),
        metrics["cards"].clone(),
    );
    let has_dock = metrics["hasNavigationDock"] == true;
    let required_gap = number(&metrics, "viewerChromeRequiredGap").unwrap_or(0.0);
    let chrome_ok = number(&metrics, "legendDockIntersectionArea").is_some_and(|n| n <= 0.5)
        && (!has_dock
            || (number(&metrics, "dockStageIntersectionArea").is_some_and(|n| n <= 0.5)
                && number(&metrics, "dockStageGap").is_some_and(|n| n >= required_gap - 1.0)));
    add(
        "viewer-chrome-clearance",
        chrome_ok,
        json!({"legendAndSvgStage":true}),
        json!({"legendIntersectionArea":metrics["legendDockIntersectionArea"],"stageIntersectionArea":metrics["dockStageIntersectionArea"],"gap":metrics["dockStageGap"],"requiredGap":required_gap}),
    );
    let controls_ok = metrics["controls"].as_array().is_some_and(|controls| {
        controls.len() == 7
            && controls.iter().all(|control| {
                control["present"] == true
                    && control["visible"] == true
                    && control["hit"] == true
                    && control["disabled"] == false
                    && control["focusable"] == true
                    && control["accessibleName"]
                        .as_str()
                        .is_some_and(|name| !name.trim().is_empty())
                    && valid_rect(&control["bounds"])
                    && number(&control["bounds"], "width").is_some_and(|n| n >= 20.0)
                    && number(&control["bounds"], "height").is_some_and(|n| n >= 20.0)
            })
    });
    add(
        "control-hit-rects",
        controls_ok,
        json!({"primaryControls":7,"minimumTargetPx":20,"hitTest":"actual elementFromPoint after document scroll"}),
        metrics["controls"].clone(),
    );
    let errors: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(event["method"].as_str(), Some("Runtime.exceptionThrown"))
                || event["method"] == "Runtime.consoleAPICalled"
                    && event["params"]["type"] == "error"
                || event["method"] == "Log.entryAdded"
                    && event["params"]["entry"]["level"] == "error"
        })
        .cloned()
        .collect();
    let media_ok = metrics["media"].as_array().is_some_and(|media| {
        media.iter().all(|entry| {
            entry["error"].is_null() && (entry["paused"] == true || entry["ended"] == true)
        })
    });
    add(
        "runtime-errors",
        errors.is_empty() && media_ok,
        json!({"pageExceptions":true,"consoleErrors":true,"mediaState":true}),
        json!({"errors":errors,"media":metrics["media"],"capabilities":metrics["capabilities"],"motion":metrics["motion"]}),
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
        images.iter().all(|entry| {
            entry["complete"] == true
                && number(entry, "width").is_some_and(|n| n > 0.0)
                && number(entry, "height").is_some_and(|n| n > 0.0)
        })
    });
    let resources_ok = requests.is_empty()
        && failed_resources.is_empty()
        && remote_resources.is_empty()
        && images_ok
        && metrics["fontsStatus"] == "loaded"
        && metrics["resources"].is_array();
    add(
        "offline-resources",
        resources_ok,
        json!({"remoteSchemes":"blocked and failed","fontsReady":true,"imagesLoaded":true}),
        json!({"remoteRequests":requests,"loadingFailed":failed_resources,"remoteResources":remote_resources,"images":metrics["images"],"fontsStatus":metrics["fontsStatus"]}),
    );
    let status = if checks.iter().all(|check| check.status == CheckStatus::Pass) {
        BrowserStatus::Pass
    } else {
        BrowserStatus::Failed
    };
    BrowserCoverage {
        viewport,
        theme,
        status,
        vertical_scroll_accepted,
        overflow_disposition: if !containment {
            "unexpected-overflow"
        } else if vertical_scroll_accepted {
            "readable-vertical-scroll"
        } else {
            "contained"
        }
        .into(),
        metrics,
        checks,
    }
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

pub(super) fn summarize(coverage: &[BrowserCoverage]) -> Vec<BrowserCheck> {
    BROWSER_MEASUREMENT_CHECKS.iter().map(|name| {
        let entries:Vec<_> = coverage.iter().filter_map(|entry| entry.checks.iter().find(|check| check.name==*name).map(|check|json!({"viewport":entry.viewport,"theme":entry.theme,"status":check.status}))).collect();
        let complete = entries.len()==10;
        let failed = coverage.iter().flat_map(|entry|&entry.checks).any(|entry|entry.name==*name && entry.status==CheckStatus::Failed);
        BrowserCheck {name:(*name).into(),status:if failed {CheckStatus::Failed} else if complete {CheckStatus::Pass} else {CheckStatus::NotRun},scope:json!({"requiredViewports":10,"executedViewports":entries.len()}),evidence:json!(entries)}
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn metrics() -> Value {
        json!({"innerWidth":1440,"innerHeight":900,"scrollWidth":1440,"scrollHeight":1600,"minimumProjectedNodeTextPx":7,"readerFit":"intrinsic-height","readerLayout":"adaptive","readerOverflow":"authored","documentScrollUnclipped":true,"readerFlow":{"unclipped":true,"failures":[]}})
    }
    #[test]
    fn normal_flow_notes_require_unclipped_evidence_after_height_adjustment() {
        // Legacy workflow at 390x844: the diagram ends at 691.27px while two
        // normal-flow cards extend the document to 991px. Its legal horizontal
        // camera pan is independent of below-fold notes and vertical clipping.
        let viewport = Viewport {
            width: 390,
            height: 844,
        };
        let baseline = json!({"innerWidth":390,"innerHeight":844,"scrollWidth":390,"scrollHeight":844,"fullScrollHeight":991,"minimumProjectedNodeTextPx":5.255,"readerFit":"contain","readerLayout":null,"readerOverflow":null,"documentScrollUnclipped":false,"readerFlow":{"unclipped":true,"notesBelowDiagram":true,"excludedHeight":283.578,"failures":[]}});
        let viewport_check = |metrics| {
            classify(viewport, Theme::Light, metrics, &[])
                .checks
                .into_iter()
                .find(|entry| entry.name == "viewport-containment")
                .unwrap()
        };
        assert_eq!(viewport_check(baseline.clone()).status, CheckStatus::Pass);
        for broken in [
            json!({"unclipped":false,"failures":[{"reason":"ancestor-overflow-y"}]}),
            json!(null),
        ] {
            let mut metrics = baseline.clone();
            metrics["readerFlow"] = broken;
            assert_eq!(viewport_check(metrics).status, CheckStatus::Failed);
        }
        let mut tall_diagram = baseline;
        tall_diagram["scrollHeight"] = json!(1100);
        tall_diagram["minimumProjectedNodeTextPx"] = json!(7);
        assert_eq!(viewport_check(tall_diagram).status, CheckStatus::Failed);
    }
    #[test]
    fn vertical_exceptions_require_declaration_readability_and_real_document_flow() {
        let viewport = Viewport {
            width: 1440,
            height: 900,
        };
        assert!(classify(viewport, Theme::Light, metrics(), &[]).vertical_scroll_accepted);
        for (key, value) in [
            ("readerFit", json!("explicit")),
            ("documentScrollUnclipped", json!(false)),
            ("minimumProjectedNodeTextPx", json!(5.99)),
            ("scrollWidth", json!(1441)),
        ] {
            let mut metrics = metrics();
            metrics[key] = value;
            assert!(
                !classify(viewport, Theme::Light, metrics, &[]).vertical_scroll_accepted,
                "{key}"
            );
        }
    }
    #[test]
    fn explicit_height_is_only_an_architecture_exception() {
        let mut metrics = metrics();
        metrics["readerFit"] = json!("authored-height");
        metrics["diagramType"] = json!("architecture");
        assert!(
            classify(
                Viewport {
                    width: 1440,
                    height: 900
                },
                Theme::Light,
                metrics.clone(),
                &[]
            )
            .vertical_scroll_accepted
        );
        metrics["diagramType"] = json!("workflow");
        assert!(
            !classify(
                Viewport {
                    width: 1440,
                    height: 900
                },
                Theme::Light,
                metrics,
                &[]
            )
            .vertical_scroll_accepted
        );
    }
    #[test]
    fn incomplete_measurements_and_remote_requests_do_not_become_pass() {
        let coverage = classify(
            Viewport {
                width: 1440,
                height: 900,
            },
            Theme::Light,
            json!({}),
            &[
                json!({"method":"Network.requestWillBeSent","params":{"request":{"url":"https://example.invalid/font"}}}),
            ],
        );
        assert_eq!(coverage.status, BrowserStatus::Failed);
        assert!(coverage
            .checks
            .iter()
            .find(|entry| entry.name == "offline-resources")
            .is_some_and(|entry| entry.status == CheckStatus::Failed));
        assert!(summarize(&[coverage])
            .iter()
            .all(|entry| entry.status != CheckStatus::Pass));
    }

    #[test]
    fn semantic_visibility_is_required_by_the_existing_rendered_dom_check() {
        let bounds = json!({"left":0,"top":0,"right":200,"bottom":100,"width":200,"height":100});
        let mut metrics = metrics();
        metrics["svgCount"] = json!(1);
        metrics["nodeCount"] = json!(1);
        metrics["fullScrollHeight"] = json!(1600);
        metrics["viewBox"] = json!({"x":0,"y":0,"width":200,"height":100});
        for key in ["readerRect", "panelRect", "svgRect"] {
            metrics[key] = bounds.clone();
        }
        metrics["semanticVisibility"] = json!({"ok":true});
        let dom = |metrics| {
            classify(
                Viewport {
                    width: 1440,
                    height: 900,
                },
                Theme::Light,
                metrics,
                &[],
            )
            .checks
            .into_iter()
            .find(|check| check.name == "rendered-dom")
            .unwrap()
        };
        assert_eq!(dom(metrics.clone()).status, CheckStatus::Pass);
        metrics["semanticVisibility"]["ok"] = json!(false);
        assert_eq!(dom(metrics.clone()).status, CheckStatus::Failed);
        metrics
            .as_object_mut()
            .unwrap()
            .remove("semanticVisibility");
        assert_eq!(dom(metrics).status, CheckStatus::Failed);
    }
}
