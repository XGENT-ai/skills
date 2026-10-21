use super::{number, selector, visible_text, ContentFinding};
use crate::layout::timeline::Clock;
use scraper::ElementRef;
use serde_json::{json, Value};

fn finding(code: &'static str, path: &str, message: &str, evidence: Value) -> ContentFinding {
    (code, path.into(), message.into(), evidence)
}

fn text_of(element: ElementRef<'_>) -> String {
    visible_text(&element.text().collect::<String>())
}

/// Timelines and waterfalls state time only: a drawn relationship or arrow
/// would read as runtime causality the input never declared.
pub(super) fn relationships(svg: ElementRef<'_>) -> Option<ContentFinding> {
    let drawn = svg
        .select(&selector(
            "[data-edge-from], [data-edge-to], [marker-start], [marker-mid], [marker-end]",
        ))
        .count();
    (drawn > 0).then(|| {
        finding(
            "artifact/semantic-count",
            "/svg/relationships",
            "A time diagram draws a relationship path or arrow marker",
            json!({"drawn":drawn}),
        )
    })
}

// Every event keeps its title, its time label in the declared zone and its
// lane; the caption discloses the zone and any date-only placement.
pub(super) fn content(svg: ElementRef<'_>, metadata: &Value) -> Vec<ContentFinding> {
    let mut findings: Vec<ContentFinding> = relationships(svg).into_iter().collect();
    let facts = &metadata["facts"];
    let timezone = facts["timezone"].as_str().unwrap_or("UTC");
    let events = facts["events"].as_array().map_or(&[][..], Vec::as_slice);
    let authored = metadata
        .pointer("/graph/nodes")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let instants: Vec<i64> = events
        .iter()
        .filter_map(|event| event["atMs"].as_i64())
        .collect();
    let Some(clock) = Clock::of(timezone, &instants) else {
        findings.push(finding(
            "artifact/semantic-text",
            "/metadata/facts/timezone",
            "Timeline facts do not name a usable zone and instants",
            json!({"timezone":timezone}),
        ));
        return findings;
    };
    let locale = crate::i18n::resolve(&metadata["meta"]);
    let lanes = facts["lanes"].as_array().map_or(&[][..], Vec::as_slice);
    let drawn_lanes: Vec<_> = svg.select(&selector("g[data-timeline-lane]")).collect();
    if drawn_lanes.len() != lanes.len().max(1) {
        findings.push(finding(
            "artifact/semantic-count",
            "/metadata/facts/lanes",
            "Drawn lanes differ from the declared lanes",
            json!({"lanes":lanes.len(),"drawn":drawn_lanes.len()}),
        ));
    }
    let lane_line = |id: &str| {
        drawn_lanes
            .iter()
            .find(|lane| lane.value().attr("data-timeline-lane") == Some(id))
            .and_then(|lane| lane.select(&selector("line")).next())
            .map(|line| number(line, "y1", f64::NAN))
    };
    for (index, lane) in lanes.iter().enumerate() {
        let id = lane["id"].as_str().unwrap_or_default();
        let label = drawn_lanes
            .iter()
            .find(|drawn| drawn.value().attr("data-timeline-lane") == Some(id))
            .and_then(|drawn| drawn.select(&selector("text")).next());
        if label.is_none_or(|label| {
            text_of(label) != visible_text(lane["label"].as_str().unwrap_or_default())
        }) {
            findings.push(finding(
                "artifact/semantic-text",
                &format!("/metadata/facts/lanes/{index}"),
                "A lane label is missing or changed",
                json!({"lane":id}),
            ));
        }
    }
    for (index, event) in events.iter().enumerate() {
        let id = event["id"].as_str().unwrap_or_default();
        let path = format!("/metadata/facts/events/{index}");
        let Some(raw) = authored.iter().find(|raw| raw["id"].as_str() == Some(id)) else {
            continue;
        };
        let Some(group) = svg
            .select(&selector("g[data-node-id]"))
            .find(|group| group.value().attr("data-node-id") == Some(id))
        else {
            continue;
        };
        let at = raw["at"].as_str().unwrap_or_default();
        let ms = event["atMs"].as_i64().unwrap_or_default();
        let texts: Vec<_> = group.select(&selector("text")).collect();
        let title = texts
            .iter()
            .find(|text| text.value().attr("data-node-label").is_some());
        if title.is_none_or(|title| {
            text_of(*title) != visible_text(raw["title"].as_str().unwrap_or_default())
        }) {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "An event title is missing or changed",
                json!({"id":id}),
            ));
        }
        let expected = clock.event_label(
            at,
            ms,
            event["precision"].as_str().unwrap_or("time"),
            &locale,
        );
        if !texts.iter().any(|text| {
            text.value().attr("data-node-label").is_none()
                && text_of(*text) == visible_text(&expected)
        }) {
            findings.push(finding(
                "artifact/semantic-text",
                &path,
                "An event's time label differs from its instant in the declared zone",
                json!({"id":id,"expected":expected}),
            ));
        }
        let lane = event["lane"].as_str().unwrap_or("__all");
        let lane = if lanes.is_empty() { "__all" } else { lane };
        let dot = group
            .select(&selector("circle"))
            .last()
            .map(|circle| number(circle, "cy", f64::NAN));
        let on_lane = dot
            .zip(lane_line(lane))
            .is_some_and(|(dot, line)| (dot - line).abs() <= crate::geometry::EPSILON);
        if group.value().attr("data-timeline-at") != Some(at)
            || group.value().attr("data-timeline-ms") != Some(ms.to_string().as_str())
            || !on_lane
        {
            findings.push(finding(
                "artifact/semantic-relation",
                &path,
                "An event's instant or lane binding differs from its facts",
                json!({"id":id,"lane":lane,"atMs":ms}),
            ));
        }
    }
    if svg
        .select(&selector("g[data-timeline-evidence]"))
        .next()
        .and_then(|badge| badge.value().attr("data-timeline-evidence"))
        != facts["evidence"].as_str()
        || svg.value().attr("data-timeline-ui").is_none()
    {
        findings.push(finding(
            "artifact/semantic-relation",
            "/metadata/facts/evidence",
            "The evidence badge differs from the declared evidence",
            json!({"evidence":facts["evidence"]}),
        ));
    }
    let caption = svg
        .select(&selector("text[data-timeline-axis-caption]"))
        .next()
        .map(text_of)
        .unwrap_or_default();
    let day = events.iter().any(|event| event["precision"] == "day");
    if !caption.contains(&visible_text(timezone))
        || (day
            && !caption.contains(&visible_text(
                &locale.text("timeline.precision.dayDisclosure", &[]),
            )))
    {
        findings.push(finding(
            "artifact/semantic-text",
            "/svg/data-timeline-axis-caption",
            "The axis caption does not state the zone or the date-only disclosure",
            json!({"timezone":timezone,"datePrecision":day}),
        ));
    }
    findings
}
