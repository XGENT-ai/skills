use super::*;

pub(super) struct Occupant {
    pub message_index: usize,
    pub kind: &'static str,
    pub path: String,
    pub bounds: Rect,
}

pub(super) fn occupants(layout: &Layout<'_>) -> Vec<Occupant> {
    let mut result = vec![];
    for (index, message) in list(&layout.document["messages"]).iter().enumerate() {
        for (kind, field, bounds) in [
            ("message label", "label", layout.label_rect(message)),
            ("message route", "y", layout.route_rect(message)),
            (
                "note",
                "note",
                if layout.legacy {
                    None
                } else {
                    layout.notes[index].as_ref().and_then(notes::Note::rect)
                },
            ),
        ] {
            if let Some(bounds) = bounds {
                result.push(Occupant {
                    message_index: index,
                    kind,
                    path: format!("/messages/{index}/{field}"),
                    bounds,
                });
            }
        }
    }
    result
}

fn nominal(layout: &Layout<'_>, segment: &Value) -> Rect {
    Rect {
        x: 56.0,
        y: number(segment, "from") - 22.0,
        width: (layout.units(string(segment, "label")) * 5.2 + 14.0).max(42.0),
        height: 18.0,
    }
}

fn misplaced(layout: &Layout<'_>, bounds: &Rect, segment: &Value, identity: Option<usize>) -> bool {
    let half = bounds.height / 2.0;
    if 72.0 + layout.header_height() - bounds.y > half {
        return true;
    }
    if bounds.y >= number(segment, "from") - 22.0 {
        return false;
    }
    list(&layout.document["segments"])
        .iter()
        .enumerate()
        .any(|(index, other)| {
            Some(index) != identity
                && (bounds.y + bounds.height).min(number(other, "to"))
                    - bounds.y.max(number(other, "from"))
                    > half
        })
}

fn legacy_label(layout: &Layout<'_>, segment: &Value, identity: Option<usize>) -> Rect {
    let occupied = occupants(layout);
    let mut bounds = nominal(layout, segment);
    for _ in 0..4 {
        if !occupied
            .iter()
            .any(|item| layout.overlaps(&bounds, &item.bounds, 2.0))
        {
            break;
        }
        bounds.y -= 22.0;
    }
    if layout.legacy || !misplaced(layout, &bounds, segment, identity) {
        return bounds;
    }
    let other_titles: Vec<_> = list(&layout.document["segments"])
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != identity)
        .map(|(_, other)| nominal(layout, other))
        .collect();
    let headers: Vec<_> = (0..layout.centers.len())
        .map(|index| layout.participant_bounds(index))
        .collect();
    let mut inside = Rect {
        y: number(segment, "from") + 4.0,
        ..bounds
    };
    for _ in 0..4 {
        if inside.y + inside.height > number(segment, "to") - 2.0 {
            break;
        }
        if !misplaced(layout, &inside, segment, identity)
            && !occupied
                .iter()
                .map(|item| &item.bounds)
                .chain(other_titles.iter())
                .chain(headers.iter())
                .any(|obstacle| layout.overlaps(&inside, obstacle, 2.0))
        {
            return inside;
        }
        inside.y += 22.0;
    }
    bounds
}

// `identity = None` models the source's cloned segment used by bounded repair:
// the original segment is then another phase, rather than the cloned value.
pub(super) fn label(layout: &Layout<'_>, segment: &Value, identity: Option<usize>) -> Rect {
    let bounds = legacy_label(layout, segment, identity);
    if layout.legacy {
        return bounds;
    }
    let headers: Vec<_> = (0..layout.centers.len())
        .map(|index| layout.participant_bounds(index))
        .collect();
    let covered: Vec<_> = headers
        .iter()
        .filter(|header| layout.overlaps(&bounds, header, 0.0))
        .collect();
    if covered.is_empty() || bounds.y + bounds.height > number(segment, "from") {
        return bounds;
    }
    let others: Vec<_> = list(&layout.document["segments"])
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != identity)
        .collect();
    if others.iter().any(|(_, other)| {
        number(other, "from") <= number(segment, "from")
            || (number(other, "from") < number(segment, "to")
                && number(other, "to") > number(segment, "from"))
    }) {
        return bounds;
    }
    let candidate = Rect {
        y: covered
            .iter()
            .map(|header| header.y + header.height)
            .fold(f64::NEG_INFINITY, f64::max)
            + 2.0,
        ..bounds
    };
    let bottom = candidate.y + candidate.height;
    if candidate.y > number(segment, "from")
        || bottom < number(segment, "from")
        || bottom > number(segment, "to") - 2.0
    {
        return bounds;
    }
    let mut obstacles: Vec<_> = occupants(layout)
        .into_iter()
        .map(|item| item.bounds)
        .chain(headers)
        .chain(
            others
                .iter()
                .map(|(index, other)| legacy_label(layout, other, Some(*index))),
        )
        .collect();
    if !obstacles
        .iter()
        .any(|obstacle| layout.overlaps(&candidate, obstacle, 2.0))
    {
        return candidate;
    }
    obstacles.extend(
        list(&layout.document["activations"])
            .iter()
            .filter_map(|activation| {
                let index = layout.participant_index(string(activation, "participant"))?;
                Some(Rect {
                    x: layout.centers[index] - 5.0,
                    y: number(activation, "from"),
                    width: 10.0,
                    height: number(activation, "to") - number(activation, "from"),
                })
            }),
    );
    let mut boundaries = vec![candidate.x];
    for obstacle in &obstacles {
        if candidate.y < obstacle.y + obstacle.height + 2.0
            && obstacle.y < candidate.y + candidate.height + 2.0
        {
            let x = obstacle.x + obstacle.width + 2.0;
            if !boundaries.contains(&x) {
                boundaries.push(x);
            }
        }
    }
    boundaries.retain(|x| *x >= candidate.x && *x + candidate.width <= layout.width - 48.0);
    boundaries.sort_by(f64::total_cmp);
    for x in boundaries {
        let shifted = Rect { x, ..candidate };
        if !obstacles
            .iter()
            .any(|obstacle| layout.overlaps(&shifted, obstacle, 2.0))
        {
            return shifted;
        }
    }
    bounds
}

pub(super) fn blocker(
    layout: &Layout<'_>,
    segment: &Value,
    identity: Option<usize>,
) -> Option<Occupant> {
    let bounds = label(layout, segment, identity);
    occupants(layout)
        .into_iter()
        .find(|item| layout.overlaps(&bounds, &item.bounds, 2.0))
}
