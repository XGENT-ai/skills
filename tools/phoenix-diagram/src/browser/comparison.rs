//! Browser evidence for every view of one architecture comparison.
use super::{
    cdp, discovery, evidence, observation, BrowserCheck, BrowserCoverage, BrowserOptions,
    BrowserStatus, CheckStatus, Theme, Viewport, DESKTOP_VIEWPORTS, NARROW_VIEWPORT,
};
use crate::{
    browser::BoundFile,
    check::FileIdentity,
    compare::{ComparisonBrowserReceipt, ComparisonCoverage, ComparisonScreenshot},
    diagnostic::Diagnostic,
};
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path, time::Duration};

const VIEWS: [&str; 3] = ["before", "delta", "after"];
const PARENT_CHECK: &str = "comparison-parent";

fn append(receipt: &mut ComparisonBrowserReceipt, error: cdp::Error) {
    let mut diagnostic = Diagnostic::error(None, error.code, "", error.message);
    diagnostic.stage = "browser".into();
    diagnostic.evidence = json!({"phase":error.stage,"retainedPath":error.retained});
    if let Some(resources) = error.resources {
        diagnostic.evidence["resources"] = *resources;
    }
    receipt.diagnostics.push(diagnostic);
}
fn number(value: &Value, key: &str) -> Option<f64> {
    value[key].as_f64().filter(|n| n.is_finite())
}
fn positive_dimension(value: &Value, key: &str) -> Option<u32> {
    number(value, key)
        .filter(|n| *n > 0.0 && n.fract() == 0.0 && *n <= u32::MAX as f64)
        .map(|n| n as u32)
}
fn rect(value: &Value) -> bool {
    ["left", "top", "right", "bottom", "width", "height"]
        .iter()
        .all(|key| number(value, key).is_some())
        && number(value, "width").is_some_and(|n| n > 0.0)
        && number(value, "height").is_some_and(|n| n > 0.0)
}
fn owned_document(value: &str, artifact: &Path, theme: Theme) -> bool {
    let Ok(mut expected) = url::Url::from_file_path(artifact) else {
        return false;
    };
    expected.query_pairs_mut().append_pair(
        "theme",
        match theme {
            Theme::Light => "light",
            Theme::Dark => "dark",
        },
    );
    value == expected.as_str()
}
fn control_ok(control: &Value, required: bool, roving: bool) -> bool {
    control["present"] == true
        && control["visible"] == true
        && rect(&control["bounds"])
        && control["accessibleName"]
            .as_str()
            .is_some_and(|name| !name.trim().is_empty())
        && (control["disabled"] == true && !required
            || control["disabled"] == false
                && control["hit"] == true
                && (control["focusable"] == true || roving && control["tabIndex"] == -1)
                && control["focused"] == true)
}
fn location_ok(parent: &Value) -> bool {
    let Some(changes) = parent["changes"].as_array() else {
        return false;
    };
    let Some(checks) = parent["locationChecks"].as_array() else {
        return false;
    };
    let expected: Vec<_> = if changes.len() > 1 {
        vec![&changes[0]["key"], &changes[changes.len() - 1]["key"]]
    } else {
        changes.iter().map(|c| &c["key"]).collect()
    };
    checks.len() == expected.len()
        && checks.iter().zip(expected).all(|(check, key)| {
            check["key"] == *key
                && key.as_str().is_some_and(|s| !s.is_empty())
                && check["ariaCurrent"] == "step"
                && check["selectedView"] == "delta"
                && check["painted"] == true
                && check["highlightedKeys"]
                    .as_array()
                    .is_some_and(|keys| !keys.is_empty() && keys.iter().all(|value| value == key))
        })
}
fn parent_ok(
    view: &str,
    viewport: Viewport,
    theme: Theme,
    metrics: &Value,
    artifact: &Path,
) -> bool {
    let parent = &metrics["comparisonParent"];
    let frame = &parent["frame"];
    let tabs = parent["tabs"].as_array();
    let changes = parent["changes"].as_array();
    let navigation = parent["navigation"].as_array();
    let expected_tabs: Vec<_> = VIEWS.into_iter().map(Value::from).collect();
    parent["viewId"] == view
        && parent["metadataValid"] == true
        && parent["ready"] == true
        && parent["activeFrameCount"] == 1
        && parent["frameViewIds"] == json!(expected_tabs)
        && number(parent, "innerWidth") == Some(viewport.width as f64)
        && number(parent, "innerHeight") == Some(viewport.height as f64)
        && number(parent, "scrollWidth")
            .is_some_and(|n| n > 0.0 && n <= viewport.width as f64 + 1.0)
        && number(parent, "fullScrollHeight").is_some_and(|n| n >= viewport.height as f64)
        && parent["resolvedTheme"]
            == match theme {
                Theme::Light => "light",
                Theme::Dark => "dark",
            }
        && parent["documentUrl"]
            .as_str()
            .is_some_and(|url| owned_document(url, artifact, theme))
        && rect(&frame["bounds"])
        && frame["viewId"] == view
        && frame["ready"] == true
        && frame["contentUnclipped"] == true
        && number(&frame["bounds"], "left").is_some_and(|n| n >= -1.0)
        && number(&frame["bounds"], "right").is_some_and(|n| n <= viewport.width as f64 + 1.0)
        && number(&frame["bounds"], "bottom")
            .zip(number(parent, "scrollY"))
            .zip(number(parent, "fullScrollHeight"))
            .is_some_and(|((bottom, scroll), height)| bottom + scroll <= height + 1.0)
        && positive_dimension(frame, "clientWidth")
            == positive_dimension(&metrics["frameViewport"], "width")
        && positive_dimension(frame, "clientHeight")
            == positive_dimension(&metrics["frameViewport"], "height")
        && positive_dimension(&metrics["frameViewport"], "width")
            == positive_dimension(metrics, "innerWidth")
        && positive_dimension(&metrics["frameViewport"], "height")
            == positive_dimension(metrics, "innerHeight")
        && tabs.is_some_and(|tabs| {
            tabs.len() == 3
                && VIEWS
                    .into_iter()
                    .all(|id| tabs.iter().filter(|t| t["viewId"] == id).count() == 1)
                && tabs.iter().all(|t| {
                    control_ok(t, true, true)
                        && t["tabIndex"] == json!(if t["viewId"] == view { 0 } else { -1 })
                        && t["selected"] == json!(t["viewId"] == view)
                })
        })
        && changes.is_some_and(|changes| {
            parent["expectedChanges"].as_u64() == Some(changes.len() as u64)
                && changes.iter().all(|c| control_ok(c, true, false))
                && (!changes.is_empty() || parent["emptyState"] == true)
        })
        && location_ok(parent)
        && navigation.is_some_and(|controls| {
            controls.len() == 3
                && ["overview", "previous", "next"]
                    .into_iter()
                    .all(|id| controls.iter().filter(|c| c["action"] == id).count() == 1)
                && controls
                    .iter()
                    .all(|control| control_ok(control, false, false))
        })
        && parent["resources"].as_array().is_some_and(|resources| {
            resources.iter().all(|entry| {
                entry["name"].as_str().is_some_and(|resource| {
                    let Ok(url) = url::Url::parse(resource) else {
                        return false;
                    };
                    matches!(url.scheme(), "data" | "blob")
                        || matches!(resource, "about:blank" | "about:srcdoc")
                        || resource == parent["documentUrl"].as_str().unwrap_or_default()
                })
            })
        })
}
const DELTA_PAINT_CHECK: &str = "comparison-delta-paint";
type PaintSignature = (String, bool, String);
#[derive(Clone)]
struct ExpectedPaint {
    kind: &'static str,
    state: String,
    origin: Option<&'static str>,
}
fn svg_box(value: &Value, drawing: bool) -> bool {
    let (Some(x), Some(y), Some(width), Some(height)) = (
        number(value, "x"),
        number(value, "y"),
        number(value, "width"),
        number(value, "height"),
    ) else {
        return false;
    };
    width >= 0.0
        && height >= 0.0
        && (!drawing || width > 0.0 || height > 0.0)
        && (x + width).is_finite()
        && (y + height).is_finite()
}
fn in_svg(inner: &Value, outer: &Value, drawing: bool) -> bool {
    svg_box(inner, drawing)
        && svg_box(outer, true)
        && number(outer, "width").is_some_and(|v| v > 0.0)
        && number(outer, "height").is_some_and(|v| v > 0.0)
        && number(inner, "x")
            .zip(number(outer, "x"))
            .is_some_and(|(a, b)| a >= b - 1.0)
        && number(inner, "y")
            .zip(number(outer, "y"))
            .is_some_and(|(a, b)| a >= b - 1.0)
        && (number(inner, "x").unwrap() + number(inner, "width").unwrap()
            <= number(outer, "x").unwrap() + number(outer, "width").unwrap() + 1.0)
        && (number(inner, "y").unwrap() + number(inner, "height").unwrap()
            <= number(outer, "y").unwrap() + number(outer, "height").unwrap() + 1.0)
}
fn paint_bounds(value: &Value, edge: bool) -> bool {
    ["left", "top", "right", "bottom", "width", "height"]
        .iter()
        .all(|key| number(value, key).is_some())
        && number(value, "width").is_some_and(|n| n >= 0.0)
        && number(value, "height").is_some_and(|n| n >= 0.0)
        && (if edge {
            number(value, "width").unwrap() > 0.0 || number(value, "height").unwrap() > 0.0
        } else {
            number(value, "width").unwrap() > 0.0 && number(value, "height").unwrap() > 0.0
        })
        && (number(value, "right").unwrap()
            - number(value, "left").unwrap()
            - number(value, "width").unwrap())
        .abs()
            <= 1.0
        && (number(value, "bottom").unwrap()
            - number(value, "top").unwrap()
            - number(value, "height").unwrap())
        .abs()
            <= 1.0
}
fn color_visible(value: &Value) -> bool {
    let Some(color) = value.as_str().map(str::trim) else {
        return false;
    };
    if color.is_empty() || matches!(color, "none" | "transparent") {
        return false;
    }
    let alpha = if let Some((_, tail)) = color.rsplit_once('/') {
        Some(tail)
    } else if color.starts_with("rgba(") {
        color.rsplit_once(',').map(|(_, tail)| tail)
    } else {
        None
    };
    alpha.is_none_or(|value| {
        value
            .trim()
            .trim_end_matches(')')
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .is_ok_and(|alpha| alpha.is_finite() && alpha > 0.0)
    })
}
fn visible_style(value: &Value, reader_camera: bool) -> bool {
    value["hidden"] != true
        && value["display"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s != "none")
        && value["visibility"] == "visible"
        && number(value, "opacity").is_some_and(|n| n > 0.0 && n <= 1.0)
        && (value["clipPath"] == "none"
            || reader_camera
                && value["readerCameraClip"] == true
                && value["clipPath"]
                    .as_str()
                    .is_some_and(|clip| clip.starts_with("inset(") && clip.ends_with(')')))
        && value["maskImage"] == "none"
}
fn painted_primitive(value: &Value, svg: &Value) -> bool {
    let edge = value["role"] == "edge";
    let stroke = color_visible(&value["stroke"])
        && number(value, "strokeOpacity").is_some_and(|n| n > 0.0)
        && number(value, "strokeWidth").is_some_and(|n| n > 0.0);
    let fill =
        color_visible(&value["fill"]) && number(value, "fillOpacity").is_some_and(|n| n > 0.0);
    paint_bounds(&value["bounds"], edge)
        && in_svg(&value["logicalBounds"], svg, true)
        && visible_style(value, false)
        && value["clientRects"].as_u64().is_some_and(|n| n > 0)
        && value["ancestors"].as_array().is_some_and(|ancestors| {
            !ancestors.is_empty() && ancestors.iter().all(|style| visible_style(style, true))
        })
        && if edge {
            stroke && number(value, "pathLength").is_some_and(|n| n > 0.0)
        } else {
            (fill || stroke)
                && (value["role"] == "body"
                    || value["text"].as_str().is_some_and(|s| !s.trim().is_empty()))
        }
}
fn add_expected_paint(
    expected: &mut std::collections::BTreeMap<PaintSignature, ExpectedPaint>,
    key: &str,
    kind: &'static str,
    state: &str,
    ghost: bool,
    origin: Option<&'static str>,
) {
    let roles: &[&str] = match kind {
        "component" => &["node"],
        "relationship" => &["edge", "label"],
        _ => &["frame", "label"],
    };
    let entry = ExpectedPaint {
        kind,
        state: state.into(),
        origin,
    };
    for role in roles {
        expected.insert((key.into(), ghost, (*role).into()), entry.clone());
    }
    if kind != "component"
        && state != "same"
        && (kind != "boundary" || !ghost || origin == Some("removed"))
    {
        expected.insert((key.into(), ghost, "symbol".into()), entry);
    }
}
fn expected_paint(
    data: &Value,
    metrics: &Value,
) -> Option<std::collections::BTreeMap<PaintSignature, ExpectedPaint>> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut changes = BTreeMap::new();
    let mut expected = BTreeMap::new();
    for (collection, kind) in [
        ("components", "component"),
        ("connections", "relationship"),
        ("boundaries", "boundary"),
    ] {
        for change in data["ir"]["changes"][collection].as_array()? {
            let id = change[if kind == "boundary" { "key" } else { "id" }]
                .as_str()
                .filter(|id| !id.is_empty())?;
            let state = change["status"].as_str()?;
            if !matches!(
                state,
                "added"
                    | "removed"
                    | "changed"
                    | "evidence-changed"
                    | "moved"
                    | "rerouted"
                    | "geometry-changed"
            ) {
                return None;
            }
            let key = format!("{kind}:{id}");
            if changes.insert(key.clone(), (kind, state, change)).is_some() {
                return None;
            }
        }
    }
    for (collection, kind) in [
        ("components", "component"),
        ("connections", "relationship"),
        ("boundaries", "boundary"),
    ] {
        let ids = data["headKeys"][collection].as_array()?;
        let ids: Option<Vec<_>> = ids.iter().map(Value::as_str).collect();
        let ids = ids?;
        if ids.iter().any(|id| id.is_empty())
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
        {
            return None;
        }
        // The existing Reader probe independently measures the head graph;
        // ghosts never replace those expected ordinary identities.
        if kind != "boundary" {
            let observed = if kind == "component" {
                metrics
                    .pointer("/semanticVisibility/expected/nodeIds")?
                    .as_array()?
                    .iter()
                    .map(Value::as_str)
                    .collect::<Option<Vec<_>>>()?
            } else {
                metrics
                    .pointer("/semanticVisibility/expected/relations")?
                    .as_array()?
                    .iter()
                    .map(|r| r["id"].as_str())
                    .collect::<Option<Vec<_>>>()?
            };
            if ids.iter().copied().collect::<BTreeSet<_>>()
                != observed.into_iter().collect::<BTreeSet<_>>()
            {
                return None;
            }
        }
        for id in ids {
            let key = format!("{kind}:{id}");
            let state = changes
                .get(&key)
                .map(|(_, state, _)| *state)
                .unwrap_or("same");
            if state == "removed" {
                return None;
            }
            add_expected_paint(&mut expected, &key, kind, state, false, None);
        }
    }
    for (key, (kind, state, change)) in changes {
        if state != "removed"
            && !expected.contains_key(&(
                key.clone(),
                false,
                match kind {
                    "component" => "node",
                    "relationship" => "edge",
                    _ => "frame",
                }
                .into(),
            ))
        {
            return None;
        }
        let classifications = change["classifications"].as_array()?;
        let geometry = classifications.iter().any(|c| c == "geometry");
        let topology = classifications.iter().any(|c| c == "topology");
        let origin = if state == "added" {
            None
        } else if state == "removed" || kind == "relationship" && topology {
            Some("removed")
        } else if matches!(kind, "component" | "relationship") && geometry
            || kind == "boundary" && matches!(state, "changed" | "geometry-changed")
        {
            Some("moved-from")
        } else {
            None
        };
        if let Some(origin) = origin {
            add_expected_paint(&mut expected, &key, kind, state, true, Some(origin));
        }
    }
    Some(expected)
}
fn delta_paint_ok(metrics: &Value, identity: &FileIdentity) -> bool {
    let data = &metrics["comparisonDeltaPaint"];
    let Ok(ir) = serde_json::to_vec(&data["ir"]) else {
        return false;
    };
    if FileIdentity::of(&ir) != *identity
        || data["inspectionDetailLevel"] != "full"
        || data["restoration"]["mode"] != "overview"
        || data["restoration"]["scale"] != 1
        || data["restoration"]["x"] != 0
        || data["restoration"]["y"] != 0
    {
        return false;
    }
    let Some(expected) = expected_paint(data, metrics) else {
        return false;
    };
    let Some(markers) = data["markers"].as_array() else {
        return false;
    };
    if markers.len() != expected.len() {
        return false;
    }
    let changes = &data["ir"]["changes"];
    let mut change_keys = std::collections::BTreeSet::new();
    for (collection, kind) in [
        ("components", "component"),
        ("connections", "relationship"),
        ("boundaries", "boundary"),
    ] {
        let Some(items) = changes[collection].as_array() else {
            return false;
        };
        for item in items {
            let Some(id) = item[if kind == "boundary" { "key" } else { "id" }].as_str() else {
                return false;
            };
            change_keys.insert(format!("{kind}:{id}"));
        }
    }
    let Some(controls) = metrics["comparisonParent"]["changes"].as_array() else {
        return false;
    };
    let control_keys: Option<std::collections::BTreeSet<_>> =
        controls.iter().map(|c| c["key"].as_str()).collect();
    let Some(control_keys) = control_keys else {
        return false;
    };
    if control_keys != change_keys.iter().map(String::as_str).collect()
        || controls.len() != change_keys.len()
    {
        return false;
    }
    let mut seen = HashSet::new();
    for marker in markers {
        let (Some(key), Some(ghost), Some(role)) = (
            marker["key"].as_str(),
            marker["ghost"].as_bool(),
            marker["role"].as_str(),
        ) else {
            return false;
        };
        let signature = (key.to_owned(), ghost, role.to_owned());
        let Some(requirement) = expected.get(&signature) else {
            return false;
        };
        if !seen.insert(signature)
            || marker["kind"] != requirement.kind
            || marker["state"] != requirement.state
            || marker["origin"].as_str() != requirement.origin
        {
            return false;
        }
        let Some(primitives) = marker["primitives"].as_array() else {
            return false;
        };
        if role == "label"
            && marker["declaredLabel"]
                .as_str()
                .is_some_and(|s| s.trim().is_empty())
            && primitives.is_empty()
        {
            if !svg_box(&marker["logicalBounds"], false) {
                return false;
            }
            continue;
        }
        if !in_svg(&marker["logicalBounds"], &data["svgBounds"], true)
            || primitives.is_empty()
            || !primitives
                .iter()
                .all(|p| painted_primitive(p, &data["svgBounds"]))
        {
            return false;
        }
        let count = |name: &str| primitives.iter().filter(|p| p["role"] == name).count();
        let roles_ok = match role {
            "node" => {
                count("body") == 1
                    && count("label") > 0
                    && primitives
                        .iter()
                        .all(|p| matches!(p["role"].as_str(), Some("body" | "label")))
            }
            "edge" => primitives.len() == 1 && count("edge") == 1,
            "frame" => primitives.len() == 1 && count("body") == 1,
            "label" => {
                count("label") > 0
                    && primitives
                        .iter()
                        .all(|p| matches!(p["role"].as_str(), Some("body" | "label")))
            }
            "symbol" => primitives.len() == 1 && count("symbol") == 1,
            _ => false,
        };
        if !roles_ok {
            return false;
        }
    }
    seen.len() == expected.len()
}

fn classify(
    view: &str,
    viewport: Viewport,
    theme: Theme,
    metrics: Value,
    events: &[Value],
    artifact: &Path,
    ir_identity: &FileIdentity,
) -> BrowserCoverage {
    let frame_viewport = Viewport {
        width: positive_dimension(&metrics["frameViewport"], "width").unwrap_or(0),
        height: positive_dimension(&metrics["frameViewport"], "height").unwrap_or(0),
    };
    let mut child = metrics.clone();
    child["fullScrollHeight"] = metrics["comparisonFrameScrollHeight"].clone();
    // The enclosing file navigation is owned by the comparison, not a remote
    // request made by the srcdoc. All other network/errors remain measured.
    let parent_url = metrics["comparisonParent"]["documentUrl"].as_str();
    let child_events: Vec<_> = events
        .iter()
        .filter(|event| {
            !(event["method"] == "Network.requestWillBeSent"
                && event.pointer("/params/request/url").and_then(Value::as_str) == parent_url
                && parent_url.is_some_and(|url| owned_document(url, artifact, theme)))
        })
        .cloned()
        .collect();
    let mut coverage = observation::classify(frame_viewport, theme, child, &child_events);
    let pass = parent_ok(view, viewport, theme, &metrics, artifact);
    coverage.checks.push(BrowserCheck{name:PARENT_CHECK.into(),status:if pass {CheckStatus::Pass}else{CheckStatus::Failed},scope:json!({"requestedViewport":viewport,"frameViewport":frame_viewport,"viewId":view,"parentTabsAndChangeNavigation":true,"changeLocations":"first-and-last authored change; every change control hit/focus measured","wholeFrameInDocumentFlow":true}),evidence:metrics["comparisonParent"].clone()});
    if view == "delta" {
        let painted = delta_paint_ok(&metrics, ir_identity);
        coverage.checks.push(BrowserCheck {name:DELTA_PAINT_CHECK.into(),status:if painted {CheckStatus::Pass}else{CheckStatus::Failed},scope:json!({"viewId":"delta","irIdentity":ir_identity,"allMarkerRoles":true,"ghosts":true,"readingDepth":"full","clipping":"finite SVG source viewBox and explicit clip/mask; declared Reader camera/pan allowed"}),evidence:metrics["comparisonDeltaPaint"].clone()});
        if !painted {
            coverage.status = BrowserStatus::Failed;
        }
    }
    coverage.viewport = viewport;
    coverage.metrics = metrics;
    if !pass {
        coverage.status = BrowserStatus::Failed;
        coverage.overflow_disposition = "unexpected-comparison-overflow".into();
    } else if number(&coverage.metrics["comparisonParent"], "fullScrollHeight")
        .is_some_and(|n| n > viewport.height as f64)
        && coverage.status == BrowserStatus::Pass
    {
        coverage.vertical_scroll_accepted = true;
        coverage.overflow_disposition = "comparison-document-flow".into();
    }
    coverage
}
fn summarize(coverage: &[ComparisonCoverage]) -> Vec<BrowserCheck> {
    let mut checks = vec![];
    for view in VIEWS {
        let entries: Vec<_> = coverage
            .iter()
            .filter(|entry| entry.view_id == view)
            .map(|entry| entry.coverage.clone())
            .collect();
        for mut check in observation::summarize(&entries) {
            check.name = format!("{view}/{}", check.name);
            check.scope = json!({"viewId":view,"measurementScope":check.scope});
            checks.push(check);
        }
        let parents:Vec<_>=entries.iter().filter_map(|entry|entry.checks.iter().find(|c|c.name==PARENT_CHECK).map(|check|json!({"viewport":entry.viewport,"theme":entry.theme,"status":check.status}))).collect();
        checks.push(BrowserCheck {
            name: format!("{view}/{PARENT_CHECK}"),
            status: if parents.iter().any(|c| c["status"] == "failed") {
                CheckStatus::Failed
            } else if parents.len() == 10 {
                CheckStatus::Pass
            } else {
                CheckStatus::NotRun
            },
            scope: json!({"viewId":view,"requiredViewports":10,"executedViewports":parents.len()}),
            evidence: json!(parents),
        });
    }
    let entries:Vec<_>=coverage.iter().filter(|entry|entry.view_id=="delta").filter_map(|entry|entry.coverage.checks.iter().find(|c|c.name==DELTA_PAINT_CHECK).map(|check|json!({"viewport":entry.coverage.viewport,"theme":entry.coverage.theme,"status":check.status}))).collect();
    checks.push(BrowserCheck {
        name: format!("delta/{DELTA_PAINT_CHECK}"),
        status: if entries.iter().any(|c| c["status"] == "failed") {
            CheckStatus::Failed
        } else if entries.len() == 10 {
            CheckStatus::Pass
        } else {
            CheckStatus::NotRun
        },
        scope: json!({"viewId":"delta","requiredViewports":10,"executedViewports":entries.len()}),
        evidence: json!(entries),
    });
    checks
}
fn terminal_checks(cleanup: bool, capture: Option<usize>) -> Vec<BrowserCheck> {
    let mut checks = vec![BrowserCheck {
        name: "browser-cleanup".into(),
        status: if cleanup {
            CheckStatus::Pass
        } else {
            CheckStatus::Failed
        },
        scope: json!({"ownedChild":true,"ownedProfile":true}),
        evidence: Value::Null,
    }];
    if let Some(count) = capture {
        checks.push(BrowserCheck{name:"screenshot-capture".into(),status:if count==12 {CheckStatus::Pass}else{CheckStatus::Failed},scope:json!({"required":12,"views":VIEWS,"themes":["light","dark"],"viewports":[DESKTOP_VIEWPORTS[0],DESKTOP_VIEWPORTS[3]],"longDiagrams":"full-page"}),evidence:json!({"captured":count})});
    }
    checks
}

/// `artifact` is the comparison HTML; `ir_identity` names the comparison IR
/// the Delta view must paint.
pub fn inspect(
    artifact: &Path,
    ir_identity: &FileIdentity,
    options: &BrowserOptions,
) -> ComparisonBrowserReceipt {
    let mut receipt = ComparisonBrowserReceipt {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        tool_version: crate::TOOL_VERSION.into(),
        command: if options.capture {
            "visual-check"
        } else {
            "browser-check"
        }
        .into(),
        kind: "architecture-comparison-browser".into(),
        status: BrowserStatus::Failed,
        diagnostics: vec![],
        artifact: BoundFile {
            path: artifact.to_owned(),
            identity: FileIdentity::of(b""),
        },
        browser_version: None,
        coverage: vec![],
        checks: summarize(&[]),
        screenshots: vec![],
    };
    if options.startup_timeout.is_zero()
        || options.startup_timeout > Duration::from_secs(90)
        || options.command_timeout.is_zero()
        || options.command_timeout > Duration::from_secs(15)
    {
        append(&mut receipt,cdp::Error::new("browser/options","options","Browser deadlines must be positive and bounded by startup=90 seconds and command=15 seconds"));
        return receipt;
    }
    let artifact = match std::fs::canonicalize(artifact).and_then(|path| {
        let bytes = std::fs::read(&path)?;
        Ok((path, bytes))
    }) {
        Ok((path, bytes)) => {
            receipt.artifact = BoundFile {
                path: path.clone(),
                identity: FileIdentity::of(&bytes),
            };
            path
        }
        Err(error) => {
            append(
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
    let executable = match discovery::find(options.executable.as_deref()) {
        Ok(path) => path,
        Err(error) => {
            if error.code == "browser/unavailable" {
                receipt.status = BrowserStatus::Unavailable;
            }
            append(&mut receipt, error);
            return receipt;
        }
    };
    let mut captures = Vec::new();
    if options.capture {
        for view in VIEWS {
            match evidence::Evidence::new(&artifact, options.out_dir.as_deref(), Some(view)) {
                Ok(e) => captures.push((view, e)),
                Err(error) => {
                    append(&mut receipt, error);
                    return receipt;
                }
            }
        }
    }
    let mut browser = match cdp::Browser::launch(
        &executable,
        options.startup_timeout,
        options.command_timeout,
    ) {
        Ok(browser) => browser,
        Err(error) => {
            append(&mut receipt, error);
            return receipt;
        }
    };
    let inspected = (|| {
        receipt.browser_version = Some(browser.version()?);
        browser.open_page()?;
        for view in VIEWS {
            for viewport in DESKTOP_VIEWPORTS.into_iter().chain([NARROW_VIEWPORT]) {
                for theme in [Theme::Light, Theme::Dark] {
                    let (metrics, mut events) =
                        browser.inspect_comparison(&artifact, view, viewport, theme)?;
                    let full_page = number(&metrics, "fullScrollHeight")
                        .is_some_and(|height| height > viewport.height as f64);
                    if let Some((_, capture)) = captures.iter_mut().find(|(id, _)| *id == view) {
                        if evidence::requires_capture(viewport, theme) {
                            let png = browser.screenshot_comparison(viewport, full_page)?;
                            capture.write_png(viewport, theme, full_page, &png)?;
                        }
                    }
                    events.extend(browser.take_events());
                    receipt.coverage.push(ComparisonCoverage {
                        view_id: view.into(),
                        coverage: classify(
                            view,
                            viewport,
                            theme,
                            metrics,
                            &events,
                            &artifact,
                            ir_identity,
                        ),
                    });
                }
            }
        }
        Ok::<_, cdp::Error>(())
    })();
    let closed = browser.close();
    let trailing = browser.take_events();
    if let Some(last) = receipt.coverage.last_mut() {
        let observed = classify(
            &last.view_id,
            last.coverage.viewport,
            last.coverage.theme,
            last.coverage.metrics.clone(),
            &trailing,
            &artifact,
            ir_identity,
        );
        for check in observed
            .checks
            .into_iter()
            .filter(|c| c.status == CheckStatus::Failed)
        {
            if let Some(previous) = last
                .coverage
                .checks
                .iter_mut()
                .find(|c| c.name == check.name)
            {
                if previous.status == CheckStatus::Pass {
                    *previous = check;
                    last.coverage.status = BrowserStatus::Failed;
                }
            }
        }
    }
    let inspected_ok = inspected.is_ok();
    if let Err(error) = inspected {
        append(&mut receipt, error);
    }
    let cleanup_ok = closed.is_ok();
    if let Err(error) = closed {
        append(&mut receipt, error);
    }
    for (view, capture) in captures {
        receipt
            .screenshots
            .extend(
                capture
                    .screenshots
                    .into_iter()
                    .map(|screenshot| ComparisonScreenshot {
                        view_id: view.into(),
                        screenshot,
                    }),
            );
    }
    receipt.checks = summarize(&receipt.coverage);
    receipt.checks.extend(terminal_checks(
        cleanup_ok,
        options.capture.then_some(receipt.screenshots.len()),
    ));
    for entry in &receipt.coverage {
        for check in &entry.coverage.checks {
            if check.status == CheckStatus::Failed {
                let mut d = Diagnostic::error(
                    None,
                    &format!("browser/{}", check.name),
                    "",
                    format!(
                        "{} failed in {} at {}x{}",
                        check.name,
                        entry.view_id,
                        entry.coverage.viewport.width,
                        entry.coverage.viewport.height
                    ),
                );
                d.stage = "browser".into();
                d.evidence = json!({"viewId":entry.view_id,"viewport":entry.coverage.viewport,"theme":entry.coverage.theme,"scope":check.scope,"measurement":check.evidence});
                receipt.diagnostics.push(d);
            }
        }
    }
    if inspected_ok
        && cleanup_ok
        && receipt.diagnostics.is_empty()
        && receipt.coverage.len() == 30
        && receipt
            .checks
            .iter()
            .all(|check| check.status == CheckStatus::Pass)
    {
        receipt.status = BrowserStatus::Pass;
    }
    receipt
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_metrics(width: u32, height: u32, theme: &str, long: bool) -> serde_json::Value {
        let bounds = json!({"x":10,"y":10,"left":10,"top":10,"right":210,"bottom":110,"width":200,"height":100});
        let controls:Vec<_>=(0..7).map(|index|json!({"selector":format!("fixture-control-{index}"),"present":true,"visible":true,"disabled":false,"hit":true,"focusable":true,"accessibleName":"Fixture control","bounds":bounds})).collect();
        // This reports a fixed protocol payload, not measured browser visibility.
        let semantic_visibility = json!({"ok":true,"expected":{"nodeIds":["fixture-node"],"relations":[]},"initialDetailLevel":"read","inspectionDetailLevel":"full","initial":{"nodes":[{"id":"fixture-node","visible":true,"bodies":[{"visible":true,"bounds":bounds,"reasons":[],"length":null}],"labels":[{"text":"Fixture node","sourcePath":"/components/0/label","visible":true,"bounds":bounds,"reasons":[],"length":null}]}],"edges":[],"text":[{"text":"Fixture node","sourcePath":"/components/0/label","visible":true,"bounds":bounds,"reasons":[],"length":null}],"context":[],"fine":[]},"inspection":{"context":[],"fine":[]},"restoration":{"camera":{"scale":1,"x":0,"y":0,"mode":"overview"},"previousCamera":{"scale":1,"x":0,"y":0,"mode":"overview"},"detailLevel":"read","bounds":bounds,"previousBounds":bounds,"fittedBounds":bounds,"ok":true},"failures":[]});
        json!({"innerWidth":width,"innerHeight":height,"scrollWidth":width,"scrollHeight":if long{height.max(1500)}else{height},"fullScrollHeight":if long{height.max(1500)}else{height},"resolvedTheme":theme,"readerLayout":"adaptive","readerOverflow":"authored","readerFit":"intrinsic-height","diagramType":"architecture","documentScrollUnclipped":true,"readerFlow":{"unclipped":true,"notesBelowDiagram":false,"notes":[],"excludedHeight":0,"failures":[]},"readerRect":bounds,"panelRect":bounds,"svgRect":bounds,"stageRect":bounds,"viewBox":{"x":0,"y":0,"width":200,"height":100},"svgCount":1,"nodeCount":1,"edgeCount":0,"semanticVisibility":semantic_visibility,"minimumProjectedNodeTextPx":7,"minimumProjectedNodeText":{"text":"Fixture node","projectedFontPx":7},"text":[{"text":"Fixture node","projectedFontPx":7,"bounds":bounds}],"nodeTextClips":[],"canvasTextClips":[],"cards":[],"controls":controls,"fontsStatus":"loaded","images":[],"resources":[],"media":[],"capabilities":{"canvas":true,"mediaRecorder":false},"motion":"still","hasLegend":false,"hasNavigationDock":true,"legendDockIntersectionArea":0,"dockStageIntersectionArea":0,"dockStageGap":20,"viewerChromeRequiredGap":10})
    }

    fn empty_ir() -> Value {
        json!({"schemaVersion":1,"changes":{"components":[],"connections":[],"boundaries":[]}})
    }
    fn ir_identity() -> FileIdentity {
        FileIdentity::of(&serde_json::to_vec(&empty_ir()).unwrap())
    }
    fn metrics(view: &str, viewport: Viewport, theme: Theme) -> Value {
        let height = viewport.height - 100;
        let mut metrics = fixture_metrics(
            viewport.width,
            height,
            match theme {
                Theme::Light => "light",
                Theme::Dark => "dark",
            },
            false,
        );
        let bounds = json!({"left":10,"top":10,"right":110,"bottom":40,"width":100,"height":30});
        let control = |disabled: bool| json!({"present":true,"visible":true,"disabled":disabled,"bounds":bounds,"hit":true,"focusable":true,"focused":!disabled,"accessibleName":"Protocol fixture control"});
        let tabs: Vec<_> = VIEWS
            .into_iter()
            .map(|id| {
                let mut c = control(false);
                c["viewId"] = json!(id);
                c["selected"] = json!(id == view);
                c["tabIndex"] = json!(if id == view { 0 } else { -1 });
                c["focusable"] = json!(id == view);
                c
            })
            .collect();
        let navigation: Vec<_> = ["overview", "previous", "next"]
            .into_iter()
            .map(|id| {
                let mut c = control(true);
                c["action"] = json!(id);
                c
            })
            .collect();
        let mut url = url::Url::from_file_path("/fixture/comparison.html").unwrap();
        url.query_pairs_mut().append_pair(
            "theme",
            match theme {
                Theme::Light => "light",
                Theme::Dark => "dark",
            },
        );
        metrics["documentUrl"] = json!("about:srcdoc");
        metrics["comparisonFrameScrollHeight"] = json!(height);
        metrics["frameViewport"] = json!({"width":viewport.width,"height":height});
        metrics["fullScrollHeight"] = json!(viewport.height);
        metrics["comparisonParent"] = json!({"viewId":view,"metadataValid":true,"ready":true,"activeFrameCount":1,"frameViewIds":VIEWS,"innerWidth":viewport.width,"innerHeight":viewport.height,"scrollWidth":viewport.width,"fullScrollHeight":viewport.height,"scrollY":0,"resolvedTheme":match theme{Theme::Light=>"light",Theme::Dark=>"dark"},"documentUrl":url.as_str(),"frame":{"viewId":view,"ready":true,"clientWidth":viewport.width,"clientHeight":height,"contentUnclipped":true,"bounds":{"left":0,"top":100,"right":viewport.width,"bottom":viewport.height,"width":viewport.width,"height":height}},"tabs":tabs,"changes":[],"navigation":navigation,"locationChecks":[],"expectedChanges":0,"emptyState":true,"resources":[]});
        if view == "delta" {
            metrics["comparisonDeltaPaint"] = json!({"ir":empty_ir(),"headKeys":{"components":["fixture-node"],"connections":[],"boundaries":[]},"inspectionDetailLevel":"full","restoration":{"mode":"overview","scale":1,"x":0,"y":0},"svgBounds":{"x":0,"y":0,"width":200,"height":100},"markers":[{"key":"component:fixture-node","kind":"component","role":"node","state":"same","ghost":false,"origin":null,"declaredLabel":"Fixture node","logicalBounds":{"x":20,"y":20,"width":50,"height":20},"primitives":[primitive("body",20.0,20.0,50.0,20.0),primitive("label",20.0,20.0,50.0,10.0)]}]});
        }
        metrics
    }
    #[test]
    fn comparison_coverage_uses_real_frame_dimensions_and_keeps_outer_viewport() {
        let viewport = DESKTOP_VIEWPORTS[0];
        let data = metrics("delta", viewport, Theme::Light);
        let coverage = classify(
            "delta",
            viewport,
            Theme::Light,
            data.clone(),
            &[],
            Path::new("/fixture/comparison.html"),
            &ir_identity(),
        );
        assert_eq!(
            coverage.status,
            BrowserStatus::Pass,
            "{:?}",
            coverage.checks
        );
        assert_eq!(coverage.viewport, viewport);
        assert_eq!(coverage.metrics["innerHeight"], 800);
        let mut forged = data;
        forged["frameViewport"]["height"] = json!(900);
        assert_eq!(
            classify(
                "delta",
                viewport,
                Theme::Light,
                forged,
                &[],
                Path::new("/fixture/comparison.html"),
                &ir_identity()
            )
            .status,
            BrowserStatus::Failed
        );
    }
    fn primitive(role: &str, x: f64, y: f64, width: f64, height: f64) -> Value {
        json!({"role":role,"text":if matches!(role,"body"|"edge") {Value::Null}else{json!("Synthetic painted glyph")},"bounds":{"left":x,"top":y,"right":x+width,"bottom":y+height,"width":width,"height":height},"logicalBounds":{"x":x,"y":y,"width":width,"height":height},"display":"inline","visibility":"visible","opacity":0.35,"fill":if role=="edge" {"none"}else{"rgb(100,100,100)"},"stroke":if role=="edge" {"rgb(100,100,100)"}else{"none"},"fillOpacity":1,"strokeOpacity":1,"strokeWidth":1,"clientRects":1,"clipPath":"none","maskImage":"none","ancestors":[{"hidden":false,"display":"inline","visibility":"visible","opacity":0.42,"clipPath":"none","maskImage":"none"}],"pathLength":if role=="edge" {json!(100)}else{Value::Null}})
    }
    fn delta_paint_metrics(viewport: Viewport) -> Value {
        let mut data = metrics("delta", viewport, Theme::Light);
        data["comparisonDeltaPaint"] = json!({"ir":{"schemaVersion":1,"changes":{"components":[],"connections":[{"id":"old-route","status":"removed","classifications":["topology"]}],"boundaries":[]}},"headKeys":{"components":["fixture-node"],"connections":[],"boundaries":[]},"inspectionDetailLevel":"full","restoration":{"mode":"overview","scale":1,"x":0,"y":0},"svgBounds":{"x":0,"y":0,"width":200,"height":100},"markers":[
            {"key":"component:fixture-node","kind":"component","role":"node","state":"same","ghost":false,"origin":null,"declaredLabel":"Fixture node","logicalBounds":{"x":20,"y":20,"width":50,"height":20},"primitives":[primitive("body",20.0,20.0,50.0,20.0),primitive("label",20.0,20.0,50.0,10.0)]},
            {"key":"relationship:old-route","kind":"relationship","role":"edge","state":"removed","ghost":true,"origin":"removed","declaredLabel":"","logicalBounds":{"x":20,"y":30,"width":100,"height":0},"primitives":[primitive("edge",20.0,30.0,100.0,0.0)]},
            {"key":"relationship:old-route","kind":"relationship","role":"label","state":"removed","ghost":true,"origin":"removed","declaredLabel":"","logicalBounds":{"x":0,"y":0,"width":0,"height":0},"primitives":[]},
            {"key":"relationship:old-route","kind":"relationship","role":"symbol","state":"removed","ghost":true,"origin":"removed","declaredLabel":"","logicalBounds":{"x":30,"y":30,"width":8,"height":8},"primitives":[primitive("symbol",30.0,30.0,8.0,8.0)]}
        ]});
        data["comparisonParent"]["expectedChanges"] = json!(1);
        data["comparisonParent"]["changes"] = json!([{"key":"relationship:old-route","present":true,"visible":true,"disabled":false,"bounds":{"left":10,"top":10,"right":110,"bottom":40,"width":100,"height":30},"hit":true,"focusable":true,"focused":true,"accessibleName":"Removed route"}]);
        data["comparisonParent"]["locationChecks"] = json!([{"key":"relationship:old-route","ariaCurrent":"step","selectedView":"delta","highlightedKeys":["relationship:old-route"],"painted":true}]);
        data
    }
    #[test]
    fn delta_paint_accepts_only_verified_reader_camera_clipping_on_ancestors() {
        let viewport = DESKTOP_VIEWPORTS[0];
        let mut baseline = delta_paint_metrics(viewport);
        let ir_identity =
            FileIdentity::of(&serde_json::to_vec(&baseline["comparisonDeltaPaint"]["ir"]).unwrap());
        for marker in baseline["comparisonDeltaPaint"]["markers"]
            .as_array_mut()
            .unwrap()
        {
            for primitive in marker["primitives"].as_array_mut().unwrap() {
                primitive["ancestors"].as_array_mut().unwrap().push(json!({
                    "hidden":false,"display":"block","visibility":"visible","opacity":1,
                    "clipPath":"inset(20px 40px)","maskImage":"none","readerCameraClip":true
                }));
            }
        }
        assert!(delta_paint_ok(&baseline, &ir_identity));
        for failure in ["unverified", "foreign-clip", "masked", "primitive-clip"] {
            let mut data = baseline.clone();
            let primitive = &mut data["comparisonDeltaPaint"]["markers"][0]["primitives"][0];
            let ancestor = primitive["ancestors"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap();
            match failure {
                "unverified" => ancestor["readerCameraClip"] = json!(false),
                "foreign-clip" => ancestor["clipPath"] = json!("url(#hidden)"),
                "masked" => ancestor["maskImage"] = json!("url(#mask)"),
                _ => {
                    primitive["clipPath"] = json!("inset(20px 40px)");
                    primitive["readerCameraClip"] = json!(true);
                }
            }
            assert!(!delta_paint_ok(&data, &ir_identity), "{failure}");
        }
    }

    #[test]
    fn delta_marker_paint_cannot_be_proved_by_dom_presence_alone() {
        let viewport = DESKTOP_VIEWPORTS[0];
        let baseline = delta_paint_metrics(viewport);
        let ir_identity =
            FileIdentity::of(&serde_json::to_vec(&baseline["comparisonDeltaPaint"]["ir"]).unwrap());
        assert_eq!(
            classify(
                "delta",
                viewport,
                Theme::Light,
                baseline.clone(),
                &[],
                Path::new("/fixture/comparison.html"),
                &ir_identity
            )
            .status,
            BrowserStatus::Pass,
            "low-opacity ghost is painted"
        );
        for failure in [
            "hidden",
            "empty-bounds",
            "no-paint",
            "non-finite",
            "clipped",
            "missing-ghost",
            "invented-marker",
            "wrong-ir",
            "source-clipped",
            "hidden-ancestor",
        ] {
            let mut data = baseline.clone();
            let path = &mut data["comparisonDeltaPaint"]["markers"][1]["primitives"][0];
            match failure {
                "hidden" => path["display"] = json!("none"),
                "empty-bounds" => {
                    path["bounds"]["width"] = json!(0);
                    path["pathLength"] = json!(0);
                }
                "no-paint" => path["stroke"] = json!("none"),
                "non-finite" => path["bounds"]["right"] = Value::Null,
                "clipped" => path["clipPath"] = json!("url(#hidden)"),
                "source-clipped" => path["logicalBounds"]["x"] = json!(-200),
                "hidden-ancestor" => path["ancestors"][0]["visibility"] = json!("hidden"),
                "wrong-ir" => data["comparisonDeltaPaint"]["ir"]["schemaVersion"] = json!(2),
                "invented-marker" => {
                    let marker = data["comparisonDeltaPaint"]["markers"][1].clone();
                    data["comparisonDeltaPaint"]["markers"]
                        .as_array_mut()
                        .unwrap()
                        .push(marker);
                }
                _ => {
                    data["comparisonDeltaPaint"]["markers"]
                        .as_array_mut()
                        .unwrap()
                        .remove(1);
                }
            }
            assert_eq!(
                classify(
                    "delta",
                    viewport,
                    Theme::Light,
                    data,
                    &[],
                    Path::new("/fixture/comparison.html"),
                    &ir_identity
                )
                .status,
                BrowserStatus::Failed,
                "{failure}"
            );
        }
    }
}
