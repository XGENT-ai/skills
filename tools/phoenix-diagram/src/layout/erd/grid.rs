// Banded grid placement ported from the frozen renderers/erd/grid.mjs. It is not
// auto-layout: column widths and row band heights are measured from the boxes
// the author declares, so the same document always produces the same coordinates.
use crate::scene::Point;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Grid {
    pub origin: Point,
    pub gap_x: f64,
    pub gap_y: f64,
    /// Width of a box that omits its own.
    pub entity_w: f64,
    pub header_h: f64,
    pub row_h: f64,
}

impl Grid {
    pub const ERD_DEFAULT: Grid = Grid {
        origin: Point { x: 32.0, y: 40.0 },
        gap_x: 56.0,
        gap_y: 44.0,
        entity_w: 240.0,
        header_h: 30.0,
        row_h: 20.0,
    };

    /// The ERD grid: grid cells without a layout block mean the default grid,
    /// not absolute coordinates that were never supplied.
    pub fn erd(document: &Value) -> Option<Grid> {
        let raw = &document["layout"];
        if raw.is_null() {
            let placed = document["entities"].as_array().is_some_and(|entities| {
                entities
                    .iter()
                    .any(|entity| entity["row"].is_number() && entity["col"].is_number())
            });
            return placed.then_some(Self::ERD_DEFAULT);
        }
        if raw["mode"] != "grid" {
            return None;
        }
        Some(Self::ERD_DEFAULT.merged(raw))
    }

    /// Authored layout fields over these defaults.
    pub fn merged(mut self, raw: &Value) -> Grid {
        if let (Some(x), Some(y)) = (raw["origin"][0].as_f64(), raw["origin"][1].as_f64()) {
            self.origin = Point { x, y };
        }
        let number = |field: &str, fallback: f64| raw[field].as_f64().unwrap_or(fallback);
        self.gap_x = number("gapX", self.gap_x);
        self.gap_y = number("gapY", self.gap_y);
        self.entity_w = number("entityW", self.entity_w);
        self.header_h = number("headerH", self.header_h);
        self.row_h = number("rowH", self.row_h);
        self
    }
}

/// One measured box as the banded layout sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cell {
    pub row: Option<f64>,
    pub col: Option<f64>,
    pub width: f64,
    pub height: f64,
}

impl Cell {
    pub fn of(raw: &Value, width: f64, height: f64) -> Cell {
        Cell {
            row: raw["row"].as_f64(),
            col: raw["col"].as_f64(),
            width,
            height,
        }
    }
    // A box on a half column (a class grid's `col: 1.5`) is centred between two
    // columns: it sizes its row but neither column.
    fn placed(&self) -> Option<(i64, f64)> {
        let row = self.row.filter(|row| row.fract() == 0.0)?;
        let col = self
            .col
            .filter(|col| (col * 2.0).fract() == 0.0 && *col >= 0.0)?;
        Some((row as i64, col))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Bands {
    pub column_x: BTreeMap<i64, f64>,
    pub row_y: BTreeMap<i64, f64>,
    pub widths: BTreeMap<i64, f64>,
    pub heights: BTreeMap<i64, f64>,
}

/// Every column owns the widest box placed in it and every row the tallest, so
/// boxes in one band share a baseline. Indices with no box still contribute
/// their gap. `column_gaps` raises the gap after a column to a measured floor.
pub(crate) fn banded_layout(
    cells: &[Cell],
    grid: &Grid,
    column_gaps: Option<&BTreeMap<i64, f64>>,
) -> Bands {
    let mut bands = Bands::default();
    let mut max_col = -1_i64;
    for cell in cells {
        let Some((row, col)) = cell.placed() else {
            continue;
        };
        max_col = max_col.max(col.ceil() as i64);
        if col.fract() == 0.0 {
            let width = bands.widths.entry(col as i64).or_insert(0.0);
            *width = width.max(cell.width);
        }
        let height = bands.heights.entry(row).or_insert(0.0);
        *height = height.max(cell.height);
    }
    let mut x = grid.origin.x;
    for col in 0..=max_col {
        bands.column_x.insert(col, x);
        let floor = column_gaps
            .and_then(|gaps| gaps.get(&col))
            .copied()
            .unwrap_or(0.0);
        x += bands.widths.get(&col).copied().unwrap_or(0.0) + grid.gap_x.max(floor);
    }
    // A half-column box may be wider than its empty neighbouring bands. Keep
    // every grid box inside the left margin without changing integer-only grids.
    let mut left_inset = 0.0_f64;
    for cell in cells {
        let Some((_, col)) = cell.placed() else {
            continue;
        };
        if col.fract() == 0.0 {
            continue;
        }
        let midpoint =
            (centre(&bands, col.floor() as i64) + centre(&bands, col.ceil() as i64)) / 2.0;
        left_inset = left_inset.max(grid.origin.x - midpoint + cell.width / 2.0);
    }
    if left_inset > 0.0 {
        for column in bands.column_x.values_mut() {
            *column += left_inset;
        }
    }
    let mut y = grid.origin.y;
    let max_row = bands.heights.keys().copied().max().unwrap_or(-1).max(-1);
    for row in 0..=max_row {
        bands.row_y.insert(row, y);
        y += bands.heights.get(&row).copied().unwrap_or(0.0) + grid.gap_y;
    }
    bands
}

fn centre(bands: &Bands, col: i64) -> f64 {
    bands.column_x.get(&col).copied().unwrap_or(f64::NAN)
        + bands.widths.get(&col).copied().unwrap_or(0.0) / 2.0
}

/// The top-left corner of one box: its absolute `pos`, or its cell centred in
/// its column (the router reads the equal side gaps as corridors).
pub(crate) fn resolve_position(raw: &Value, cell: &Cell, bands: Option<&Bands>) -> Point {
    let invalid = Point {
        x: f64::NAN,
        y: f64::NAN,
    };
    if let (Some(x), Some(y), Some(2)) = (
        raw["pos"][0].as_f64(),
        raw["pos"][1].as_f64(),
        raw["pos"].as_array().map(Vec::len),
    ) {
        return Point { x, y };
    }
    let (Some(bands), Some((row, col))) = (bands, cell.placed()) else {
        return invalid;
    };
    let Some(row_y) = bands.row_y.get(&row).copied() else {
        return invalid;
    };
    if col.fract() != 0.0 {
        let (left, right) = (col.floor() as i64, col.ceil() as i64);
        if !bands.column_x.contains_key(&left) || !bands.column_x.contains_key(&right) {
            return invalid;
        }
        return Point {
            x: (centre(bands, left) + centre(bands, right)) / 2.0 - cell.width / 2.0,
            y: row_y,
        };
    }
    let Some(column_x) = bands.column_x.get(&(col as i64)).copied() else {
        return invalid;
    };
    let column_width = bands
        .widths
        .get(&(col as i64))
        .copied()
        .unwrap_or(cell.width);
    Point {
        x: column_x + (column_width - cell.width) / 2.0,
        y: row_y,
    }
}
