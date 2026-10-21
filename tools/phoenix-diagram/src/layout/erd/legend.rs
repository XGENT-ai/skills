// Legend selection, footprint and placement ported from the frozen
// renderers/shared/legend.mjs; erd and class own their swatches.
use crate::geometry::{rects_overlap, segment_intersects_rect, text_units};
use crate::scene::{Point, Rect};
use serde_json::Value;
use std::collections::HashSet;

const DEFAULT_FONT_SIZE: f64 = 8.0;
const ITEM_GAP: f64 = 22.0;
const LINE_GAP: f64 = 22.0;
pub(crate) const SWATCH_GAP: f64 = 8.0;
const TEXT_ADVANCE_EM: f64 = 0.62;
const INTERACTIVE_BADGE_ALLOWANCE: f64 = 21.0;

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub kind: String,
    pub label: String,
    /// The authored entry or translation that owns the label, if any.
    pub source: Option<String>,
    pub interactive: bool,
    pub swatch_width: f64,
    pub trailing_width: f64,
    /// Extra room before the swatch (the erd key/end group divider).
    pub lead: f64,
}

/// Catalog entries visible under the authored legend mode and overrides.
pub(crate) fn resolve(
    config: &Value,
    catalog: &[(&str, String)],
    present: &HashSet<String>,
) -> Vec<Entry> {
    let mode = config["mode"].as_str().unwrap_or("auto");
    if mode == "hidden" {
        return vec![];
    }
    catalog
        .iter()
        .filter_map(|(kind, label)| {
            let entry = &config["entries"][*kind];
            let selected = mode == "all" || present.contains(*kind);
            let visible = entry["visible"] == true || (selected && entry["visible"] != false);
            if !visible {
                return None;
            }
            let authored = entry["label"].as_str().filter(|label| !label.is_empty());
            Some(Entry {
                kind: (*kind).into(),
                label: authored.map_or_else(|| label.clone(), str::to_owned),
                source: authored.map(|_| format!("/meta/legend/entries/{kind}/label")),
                // The erd and class catalogs are static: their kinds are not node kinds.
                interactive: false,
                swatch_width: 14.0,
                trailing_width: 0.0,
                lead: 0.0,
            })
        })
        .collect()
}

fn rendered_font(font: f64) -> f64 {
    if font < 8.0 {
        font + 0.5
    } else {
        font + 2.0
    }
}

fn entry_width(entry: &Entry, font: f64) -> f64 {
    (entry.swatch_width
        + SWATCH_GAP
        + text_units(&entry.label) * rendered_font(font) * TEXT_ADVANCE_EM
        + if entry.interactive {
            INTERACTIVE_BADGE_ALLOWANCE
        } else {
            0.0
        }
        + entry.trailing_width)
        .ceil()
}

pub(crate) struct Footprint {
    pub widths: Vec<f64>,
    pub rows: Vec<Vec<usize>>,
    pub min_width: f64,
    pub extra_height: f64,
}

/// One footprint calculation owns both canvas sizing and final placement.
pub(crate) fn footprint(entries: &[Entry], width: f64, font: f64) -> Footprint {
    if entries.is_empty() {
        return Footprint {
            widths: vec![],
            rows: vec![],
            min_width: 0.0,
            extra_height: 0.0,
        };
    }
    let widths: Vec<f64> = entries
        .iter()
        .map(|entry| entry_width(entry, font))
        .collect();
    let mut rows: Vec<Vec<usize>> = vec![vec![]];
    let mut cursor = 0.0;
    for (index, entry_width) in widths.iter().enumerate() {
        let row = rows.last_mut().unwrap();
        let required = if row.is_empty() { 0.0 } else { ITEM_GAP } + entry_width;
        if !row.is_empty() && cursor + required > width {
            rows.push(vec![index]);
            cursor = *entry_width;
        } else {
            row.push(index);
            cursor += required;
        }
    }
    Footprint {
        min_width: widths.iter().fold(0.0, |a, b| a.max(*b)),
        extra_height: (rows.len() - 1) as f64 * LINE_GAP,
        widths,
        rows,
    }
}

/// Sizing probes measure at the shared default font; the drawn legend uses the
/// band font, as the source does.
pub(crate) fn sizing_footprint(entries: &[Entry], width: f64) -> Footprint {
    footprint(entries, width, DEFAULT_FONT_SIZE)
}

pub(crate) enum Obstacle {
    Segment(Point, Point),
    Rect(Rect),
}

pub(crate) struct Band {
    pub x: f64,
    pub baseline: f64,
    pub width: f64,
    pub font: f64,
    pub min_title_y: f64,
}

pub(crate) struct Positioned {
    pub index: usize,
    pub x: f64,
    pub baseline: f64,
    pub width: f64,
}

pub(crate) struct Placement {
    pub entries: Vec<Positioned>,
    pub title_y: f64,
    pub font: f64,
}

pub(crate) enum Unfit {
    TooWide {
        kind: String,
        width: f64,
        available: f64,
    },
    VerticalOverflow {
        rows: usize,
        top: f64,
        available: f64,
    },
    Collision {
        kind: String,
        rect: Rect,
    },
}

pub(crate) fn measure(
    entries: &[Entry],
    band: &Band,
    obstacles: &[Obstacle],
) -> Result<Option<Placement>, Unfit> {
    if entries.is_empty() {
        return Ok(None);
    }
    let footprint = footprint(entries, band.width, band.font);
    if let Some(index) = footprint
        .widths
        .iter()
        .position(|width| *width > band.width)
    {
        return Err(Unfit::TooWide {
            kind: entries[index].kind.clone(),
            width: footprint.widths[index],
            available: band.width,
        });
    }
    let title_y = band.baseline - footprint.extra_height - 20.0;
    let top = title_y - 10.0;
    if top < band.min_title_y {
        return Err(Unfit::VerticalOverflow {
            rows: footprint.rows.len(),
            top,
            available: band.min_title_y,
        });
    }
    let mut positioned = vec![];
    for (row_index, row) in footprint.rows.iter().enumerate() {
        let mut x = band.x;
        let baseline = band.baseline - (footprint.rows.len() - row_index - 1) as f64 * LINE_GAP;
        for index in row {
            positioned.push(Positioned {
                index: *index,
                x,
                baseline,
                width: footprint.widths[*index],
            });
            x += footprint.widths[*index] + ITEM_GAP;
        }
    }
    let rects = std::iter::once((
        "title".to_owned(),
        Rect {
            x: band.x,
            y: top,
            width: 48.0,
            height: 14.0,
        },
    ))
    .chain(positioned.iter().map(|entry| {
        (
            entries[entry.index].kind.clone(),
            Rect {
                x: entry.x,
                y: entry.baseline - 10.0,
                width: entry.width,
                height: 14.0,
            },
        )
    }));
    for (kind, rect) in rects {
        let hit = obstacles.iter().any(|obstacle| match obstacle {
            Obstacle::Segment(start, end) => segment_intersects_rect(*start, *end, &rect, 0.0),
            Obstacle::Rect(other) => rects_overlap(other, &rect, 0.0),
        });
        if hit {
            return Err(Unfit::Collision { kind, rect });
        }
    }
    Ok(Some(Placement {
        entries: positioned,
        title_y,
        font: band.font,
    }))
}

impl Placement {
    pub fn rendered_font(&self) -> f64 {
        rendered_font(self.font)
    }
}
