use super::*;
use crate::geometry::composition::{self, Frame, LabelRect, Obstacle, RoutedRelation};

fn span(layout: &Layout<'_>, message: &Value) -> Option<(f64, f64)> {
    let from = layout.centers[layout.participant_index(string(message, "from"))?];
    let to = layout.centers[layout.participant_index(string(message, "to"))?];
    Some((from.min(to), from.max(to)))
}

fn message_rects(layout: &Layout<'_>, index: usize, y: f64) -> Vec<Rect> {
    let mut message = layout.document["messages"][index].clone();
    message["y"] = json!(y);
    [
        layout.label_rect(&message),
        layout.route_rect(&message),
        layout.note_rect_at(index, y),
    ]
    .into_iter()
    .flatten()
    .collect()
}

// A repair is offered only if this one authored edit succeeds on its own.
fn message_fits_at(layout: &Layout<'_>, index: usize, y: f64) -> bool {
    if y < 160.0 || (!layout.automatic && y > layout.height - 83.0) {
        return false;
    }
    let messages = list(&layout.document["messages"]);
    let Some((left, right)) = span(layout, &messages[index]) else {
        return false;
    };
    let moved = message_rects(layout, index, y);
    messages.iter().enumerate().all(|(other_index, other)| {
        if index == other_index {
            return true;
        }
        let Some((other_left, other_right)) = span(layout, other) else {
            return true;
        };
        let other_y = number(other, "y");
        if (other_y - y).abs() < 28.0 && left < other_right && other_left < right {
            return false;
        }
        let other_rects = message_rects(layout, other_index, other_y);
        !moved.iter().any(|bounds| {
            other_rects
                .iter()
                .any(|other| layout.overlaps(bounds, other, 2.0))
        })
    })
}

fn rounded_bounds(bounds: Rect) -> Value {
    let round = |value: f64| (value * 10.0 + 0.5).floor() / 10.0;
    json!({"x":round(bounds.x),"y":round(bounds.y),"width":round(bounds.width),"height":round(bounds.height)})
}

fn composition_errors(layout: &Layout<'_>) -> Diagnostics {
    let routed: Vec<_> = list(&layout.document["messages"])
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            let from = layout.participant_index(string(message, "from"))?;
            let to = layout.participant_index(string(message, "to"))?;
            let y = number(message, "y");
            Some(RoutedRelation::new(
                index,
                message.clone(),
                vec![
                    Point {
                        x: layout.centers[from],
                        y,
                    },
                    Point {
                        x: layout.centers[to],
                        y,
                    },
                ],
            ))
        })
        .collect();
    let obstacles: Vec<_> = list(&layout.document["participants"])
        .iter()
        .enumerate()
        .map(|(index, participant)| Obstacle {
            id: string(participant, "id").into(),
            bounds: layout.participant_bounds(index),
        })
        .collect();
    let mut errors = vec![];
    for hit in composition::collect_route_obstacle_hits(&routed, &obstacles, 0.0) {
        errors.push(diagnostic(
            "geometry/route-obstacle",
            &format!("/messages/{}/y", hit.relation_index),
            "Message crosses an opaque participant header",
            json!(hit),
            vec!["Move the message below participant headers while preserving its order".into()],
        ));
    }
    let frames: Vec<_> = list(&layout.document["segments"])
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let mut frame = Frame::new(
                "segment",
                index.to_string(),
                Rect {
                    x: 48.0,
                    y: number(segment, "from"),
                    width: layout.width - 96.0,
                    height: number(segment, "to") - number(segment, "from"),
                },
            );
            frame.radius = 10.0;
            frame
        })
        .collect();
    for hit in composition::collect_border_runs(&routed, &frames) {
        errors.push(diagnostic(
            "composition/container-border-run",
            &format!("/messages/{}/y", hit.relation_index),
            "Message follows a segment border",
            json!(hit),
            vec!["Move the message y clearly inside its intended segment".into()],
        ));
    }
    if layout.quality != Quality::Showcase {
        return errors;
    }
    for hit in composition::collect_proper_crossings(
        &routed,
        &composition::CrossingOptions {
            contract: layout.contract,
            ..Default::default()
        },
    ) {
        errors.push(diagnostic(
            "composition/proper-crossing",
            &format!("/messages/{}/y", hit.left_relation_index),
            "Messages have a proper crossing",
            json!(hit),
            vec!["Separate the authored message y values".into()],
        ));
    }
    for hit in composition::collect_ambiguous_corridors(
        &routed,
        &composition::CorridorOptions {
            contract: layout.contract,
            ..Default::default()
        },
    ) {
        errors.push(diagnostic(
            "composition/ambiguous-corridor",
            &format!("/messages/{}/y", hit.left_relation_index),
            "Unrelated messages share an ambiguous corridor",
            json!(hit),
            vec!["Separate the authored message y values".into()],
        ));
    }
    for hit in composition::collect_route_rhythm_issues(&routed, 16.0, 8.0) {
        errors.push(diagnostic(
            hit.code,
            &format!("/messages/{}/y", hit.relation_index),
            "Message has a cramped route segment",
            json!(hit),
            vec!["Increase participant spacing without changing message endpoints".into()],
        ));
    }
    let labels: Vec<_> = list(&layout.document["messages"])
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            layout
                .label_rect(message)
                .map(|bounds| LabelRect::new(index as isize, message.clone(), bounds))
        })
        .collect();
    for hit in composition::collect_label_route_clearance(&labels, &routed, 4.0) {
        errors.push(diagnostic(
            "composition/label-route-clearance",
            &format!("/messages/{}/label", hit.label_relation_index),
            "Message label hides or approaches another message route",
            json!(hit),
            vec!["Spread message y values or shorten the identified label".into()],
        ));
    }
    if !layout.legacy {
        for hit in
            composition::collect_label_canvas_overflow(&labels, &[layout.width, layout.height], 0.5)
        {
            errors.push(diagnostic(
                "composition/label-canvas-containment",
                &format!("/messages/{}/label", hit.relation_index),
                "Message label extends beyond the canvas",
                json!(hit),
                vec![
                    "Shorten the label, reorder participants, or enlarge the authored canvas"
                        .into(),
                ],
            ));
        }
    }
    errors
}

fn note_errors(layout: &Layout<'_>) -> Diagnostics {
    if !layout.readable {
        return vec![];
    }
    let messages = list(&layout.document["messages"]);
    let mut errors = vec![];
    let floor = layout.height - 45.0;
    for (index, message) in messages.iter().enumerate() {
        let Some(note) = &layout.notes[index] else {
            continue;
        };
        let Some(bounds) = note.rect() else { continue };
        let path = format!("/messages/{index}/note");
        if note.bottom > floor {
            let required = (layout.height + note.bottom - floor)
                .ceil()
                .max(legend::required_height(layout, layout.width));
            let fix = if layout.automatic {
                format!("set /meta/viewBox to [{}, {required}]", layout.width)
            } else {
                format!("set /meta/viewBox/1 to {required}")
            };
            errors.push(diagnostic("sequence/note-canvas-limit",&path,
                "Wrapped note extends below the canvas content limit",
                json!({"lines":note.lines.len(),"note":rounded_bounds(bounds),"canvasLimitY":floor,"viewBoxHeight":layout.height,"requiredViewBoxHeight":required}),vec![fix]));
        }
        let mut blockers = vec![];
        for (other_index, other) in messages.iter().enumerate() {
            if index == other_index || number(other, "y") <= number(message, "y") {
                continue;
            }
            let hit = [
                ("message label", layout.label_rect(other)),
                ("message route", layout.route_rect(other)),
                (
                    "note",
                    layout.notes[other_index]
                        .as_ref()
                        .and_then(notes::Note::rect),
                ),
            ]
            .into_iter()
            .find(|(_, other)| other.is_some_and(|other| layout.overlaps(&bounds, &other, 2.0)));
            if let Some((kind, Some(rect))) = hit {
                blockers.push((other_index, kind, rect));
            }
        }
        blockers
            .sort_by(|a, b| number(&messages[a.0], "y").total_cmp(&number(&messages[b.0], "y")));
        if let Some((other, kind, rect)) = blockers.first() {
            let required = (note.bottom + 22.0).ceil();
            let other_path = format!("/messages/{other}/y");
            let fixes = if message_fits_at(layout, *other, required) {
                vec![format!("set {other_path} to {required}")]
            } else {
                vec![]
            };
            errors.push(diagnostic("sequence/note-overlap",&path,
                "Wrapped note reaches a later message",
                json!({"lines":note.lines.len(),"note":rounded_bounds(bounds),"blocker":{"message":messages[*other]["label"],"path":other_path,"y":number(&messages[*other],"y"),"kind":kind,"bounds":rounded_bounds(*rect)},"requiredY":required}),fixes));
        }
    }
    errors
}

pub(super) fn validate(layout: &Layout<'_>, labels: &[Rect], legend_visible: bool) -> Diagnostics {
    let participants = list(&layout.document["participants"]);
    let messages = list(&layout.document["messages"]);
    let segments = list(&layout.document["segments"]);
    let mut errors = vec![];
    if layout.height - 65.0 - 142.0 < 120.0 {
        errors.push(diagnostic(
            "sequence/timeline-height",
            "/meta/viewBox/1",
            "Canvas leaves less than 120px of readable timeline",
            json!({"height":layout.height,"requiredHeight":327}),
            vec!["set /meta/viewBox/1 to at least 327".into()],
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for (index, participant) in participants.iter().enumerate() {
        if !ids.insert(string(participant, "id")) {
            errors.push(diagnostic(
                "sequence/duplicate-participant",
                &format!("/participants/{index}/id"),
                "Participant ids must be unique",
                json!({"id":participant["id"]}),
                vec![],
            ));
        }
        let width = layout.units(string(participant, "label")) * 6.8;
        if width > layout.participant_width + 6.0 {
            errors.push(diagnostic("sequence/participant-label-overflow",&format!("/participants/{index}/label"),
                "Participant label is wider than its header",
                json!({"viewBoxWidth":layout.width,"participantWidth":layout.participant_width,"text":participant["label"],"requiredWidth":width,"availableWidth":layout.participant_width+6.0}),
                vec!["Shorten the participant label while preserving its role, or use feasible spread columns".into()]));
        }
        let sub = string(participant, "sublabel");
        let required = layout.units(sub) * layout.sublabel_minimum * 0.6;
        if !sub.is_empty() && required > layout.participant_width - 8.0 {
            errors.push(diagnostic("sequence/participant-sublabel-overflow",&format!("/participants/{index}/sublabel"),
                "Participant sublabel cannot fit at its readable minimum",
                json!({"viewBoxWidth":layout.width,"participantWidth":layout.participant_width,"text":sub,"requiredWidth":required,"availableWidth":layout.participant_width-8.0,"minimumFontPx":layout.sublabel_minimum}),
                vec!["Shorten the sublabel while preserving its role or protocol; move supplementary detail into a card".into()]));
        }
    }
    for (index, message) in messages.iter().enumerate() {
        for field in ["from", "to"] {
            if layout.participant_index(string(message, field)).is_none() {
                errors.push(diagnostic(
                    "sequence/unknown-endpoint",
                    &format!("/messages/{index}/{field}"),
                    "Message references an unknown participant",
                    json!({"field":field,"unknownNodeId":message[field]}),
                    vec![],
                ));
            }
        }
        let y = number(message, "y");
        if !y.is_finite() || y < 160.0 || y > layout.height - 83.0 {
            errors.push(diagnostic(
                "sequence/message-timeline-limit",
                &format!("/messages/{index}/y"),
                "Message sits outside the readable timeline",
                json!({"y":y,"minimumY":160,"maximumY":layout.height-83.0}),
                vec![],
            ));
        }
        if let Some((left, right)) = span(layout, message) {
            if !layout.legacy && string(message, "from") == string(message, "to") {
                errors.push(diagnostic("sequence/self-message-unsupported",&format!("/messages/{index}"),
                    "Self-message geometry is unsupported; preserve this step as a note on a real message or a card without inventing a participant",
                    json!({"participant":message["from"],"participantPath":format!("/participants/{}",layout.participant_index(string(message,"from")).unwrap()),"y":y,"supportedMessageGeometry":"horizontal-between-distinct-participants"}),vec![]));
            } else if right - left < 60.0 {
                errors.push(diagnostic(
                    "sequence/message-span",
                    &format!("/messages/{index}"),
                    "Message participants are less than 60px apart",
                    json!({"distance":right-left,"minimumDistance":60}),
                    vec![],
                ));
            }
        }
    }
    errors.extend(composition_errors(layout));
    let mut placed: Vec<_> = messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            span(layout, message).map(|bounds| (index, number(message, "y"), bounds))
        })
        .collect();
    placed.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (at, (index, y, (left, right))) in placed.iter().enumerate() {
        for (other, other_y, (other_left, other_right)) in placed
            .iter()
            .skip(at + 1)
            .take_while(|other| other.1 - y < 28.0)
        {
            if left < other_right && other_left < right {
                errors.push(diagnostic("sequence/message-spacing",&format!("/messages/{other}/y"),
                    "Messages less than 28px apart share horizontal space",json!({"messageIndex":index,"otherMessageIndex":other,"y":y,"otherY":other_y,"minimumDistance":28}),vec!["Spread the authored message y values while preserving their order".into()]));
            }
        }
    }
    let message_labels: Vec<_> = messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| layout.label_rect(message).map(|bounds| (index, bounds)))
        .collect();
    for (at, (index, bounds)) in message_labels.iter().enumerate() {
        for (other, other_bounds) in message_labels.iter().skip(at + 1) {
            if layout.overlaps(bounds, other_bounds, -2.0) {
                errors.push(diagnostic(
                    "sequence/message-label-overlap",
                    &format!("/messages/{index}/label"),
                    "Message label plates overlap",
                    json!({"label":bounds,"otherMessageIndex":other,"otherLabel":other_bounds}),
                    vec!["Spread message y values or shorten the identified labels".into()],
                ));
            }
        }
    }
    errors.extend(note_errors(layout));
    for (index, segment) in segments.iter().enumerate() {
        let from = number(segment, "from");
        let to = number(segment, "to");
        if to <= from {
            errors.push(diagnostic(
                "sequence/segment-range",
                &format!("/segments/{index}/to"),
                "Segment end must be greater than its start",
                json!({"from":from,"to":to}),
                vec![],
            ));
        }
        if from < 72.0 || to > layout.height - 45.0 {
            errors.push(diagnostic(
                "sequence/segment-canvas-limit",
                &format!("/segments/{index}"),
                "Segment extends outside the canvas content range",
                json!({"from":from,"to":to,"minimumY":72,"maximumY":layout.height-45.0}),
                vec![],
            ));
        }
        let bounds = labels[index];
        if bounds.x + bounds.width > layout.width - 48.0 {
            errors.push(diagnostic("sequence/segment-label-overflow",&format!("/segments/{index}/label"),
                "Segment title exceeds its frame width",json!({"label":bounds,"availableWidth":(layout.width-48.0-bounds.x).max(0.0),"requiredViewBoxWidth":(bounds.x+bounds.width+48.0).ceil()}),vec!["Shorten the segment title or widen the canvas while preserving the phase meaning".into()]));
        }
        if layout.legacy {
            continue;
        }
        for (other, other_bounds) in labels.iter().enumerate().skip(index + 1) {
            if layout.overlaps(&bounds, other_bounds, 0.0) {
                errors.push(diagnostic(
                    "sequence/segment-label-pair-overlap",
                    &format!("/segments/{index}/label"),
                    "Segment titles overlap",
                    json!({"label":bounds,"otherSegmentIndex":other,"otherLabel":other_bounds}),
                    vec![
                        "Leave more title space between the phases or shorten their titles".into(),
                    ],
                ));
            }
        }
        if !layout.readable {
            continue;
        }
        for edge in ["from", "to"] {
            let border = number(segment, edge);
            for (message_index, message) in messages.iter().enumerate() {
                let y = number(message, "y");
                if (border - y).abs() < 4.0 {
                    errors.push(diagnostic("sequence/segment-message-border-run",&format!("/segments/{index}/{edge}"),
                        "Segment border runs along a message",
                        json!({"segmentLabel":segment["label"],"edge":edge,"borderY":border,"messageIndex":message_index,"messageLabel":message["label"],"messageY":y,"minimumClearancePx":4}),
                        vec![format!("set /segments/{index}/{edge} to at least {} to leave the message above it",y+4.0),format!("set /segments/{index}/{edge} to at most {} to leave the message below it",y-4.0)]));
                }
            }
        }
        if let Some(covered) = segments::blocker(layout, segment, Some(index)) {
            let mut clear = None;
            'search: for distance in 1..=300 {
                for candidate in [from + distance as f64, from - distance as f64] {
                    if candidate < 72.0 || candidate >= to {
                        continue;
                    }
                    let mut moved = segment.clone();
                    moved["from"] = json!(candidate);
                    if segments::blocker(layout, &moved, None).is_none() {
                        clear = Some(candidate);
                        break 'search;
                    }
                }
            }
            let path = format!("/segments/{index}/from");
            errors.push(diagnostic("sequence/segment-label-overlap",&path,
                "Segment title still covers a message label, route, or note after bounded placement",
                json!({"label":rounded_bounds(bounds),"attempts":4,"covered":{"kind":covered.kind,"message":messages[covered.message_index]["label"],"path":covered.path,"bounds":rounded_bounds(covered.bounds)}}),
                clear.map_or_else(Vec::new,|candidate|vec![format!("set {path} to {candidate}")])));
        }
    }
    for (index, activation) in list(&layout.document["activations"]).iter().enumerate() {
        if layout
            .participant_index(string(activation, "participant"))
            .is_none()
        {
            errors.push(diagnostic(
                "sequence/activation-participant",
                &format!("/activations/{index}/participant"),
                "Activation references an unknown participant",
                json!({"participant":activation["participant"]}),
                vec![],
            ));
        }
        if number(activation, "to") <= number(activation, "from") {
            errors.push(diagnostic(
                "sequence/activation-range",
                &format!("/activations/{index}/to"),
                "Activation end must be greater than its start",
                json!({"from":activation["from"],"to":activation["to"]}),
                vec![],
            ));
        }
    }
    if let Some(center) = layout.centers.last() {
        let right = center + layout.participant_width / 2.0;
        if right > layout.width - 40.0 {
            errors.push(diagnostic("sequence/participants-canvas-limit","/meta/viewBox/0",
                "Participant row exceeds the canvas width",json!({"right":right,"viewBoxWidth":layout.width,"requiredViewBoxWidth":(right+40.0).ceil()}),vec![format!("set /meta/viewBox/0 to at least {}",(right+40.0).ceil())]));
        }
    }
    if !layout.legacy
        && layout.readable
        && layout.document["meta"].get("legend").is_none()
        && !legend_visible
    {
        let required = legend::required_height(layout, layout.width);
        if required > layout.height {
            errors.push(diagnostic("sequence/legend-content-limit","/meta/viewBox/1",
                "Timeline content leaves no room for the implicit legend",json!({"contentBottom":layout.content_bottom,"viewBoxHeight":layout.height,"requiredViewBoxHeight":required}),vec![format!("set /meta/viewBox/1 to at least {required}")]));
        }
    }
    errors
}
