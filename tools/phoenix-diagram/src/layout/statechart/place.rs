//! Compound layered placement: every container (a region or a parent state)
//! lays out its direct children in columns by BFS depth, bottom-up, and the
//! result is accumulated into absolute boxes top-down.
use crate::scene::Rect;
use crate::statechart::chart::{Child, Region};

pub(super) const PAD: f64 = 20.0;
/// Left padding when a start state sits in the first column, so its initial
/// marker stays inside the container.
pub(super) const START_PAD: f64 = 44.0;
pub(super) const TITLE_BAND: f64 = 44.0;
pub(super) const TITLE_H: f64 = 28.0;
pub(super) const TITLE_INSET: (f64, f64) = (14.0, 8.0);
const COLUMN_GAP: f64 = 88.0;
/// Room between rows in a top-down chart for the arrow labels between them.
const BAND_GAP_DOWN: f64 = 76.0;
const ROW_GAP: f64 = 88.0;
const REGION_GAP: f64 = 28.0;
const SWEEPS: usize = 4;
/// Widest container content that keeps the smallest state text (12px) at the
/// 6px reading floor in the 930px reader, with margins and loops around it.
pub(super) const MAX_WIDTH: f64 = 1760.0;
/// Tallest chart, as a multiple of its width, that fits a phone screen once
/// the reader scales it to the screen width.
pub(super) const MAX_TALL: f64 = 1.6;

/// Measured sizes the placement reads, indexed like the chart regions.
pub(super) struct Sizes {
    pub leaves: Vec<Vec<(f64, f64)>>,
    /// Room above a leaf for its self-loop and that loop's label; the same
    /// room is kept below so the leaf stays centred with its neighbours.
    pub loop_room: Vec<Vec<f64>>,
    pub titles: Vec<Vec<f64>>,
    /// Region name plate widths; empty for a single unnamed region.
    pub plates: Vec<f64>,
    /// Label width of each arrow of each region.
    pub labels: Vec<Vec<f64>>,
}

pub(super) struct Placement {
    pub leaves: Vec<Vec<Rect>>,
    pub frames: Vec<Vec<Rect>>,
    pub titles: Vec<Vec<Rect>>,
    pub regions: Vec<Rect>,
    pub bounds: Rect,
}

struct Kid {
    child: Child,
    x: f64,
    y: f64,
    block: Option<Block>,
}

struct Block {
    width: f64,
    height: f64,
    kids: Vec<Kid>,
}

struct Ctx<'a> {
    region: &'a Region,
    r: usize,
    sizes: &'a Sizes,
}

impl Ctx<'_> {
    fn path(&self, child: Child) -> &[String] {
        match child {
            Child::State(leaf) => &self.region.states[leaf].path,
            Child::Parent(parent) => &self.region.parents[parent].path,
        }
    }

    fn children(&self, child: Child) -> &[Child] {
        match child {
            Child::State(_) => &[],
            Child::Parent(parent) => &self.region.parents[parent].children,
        }
    }

    /// The shallowest depth and earliest draw position of the leaves below.
    fn rank(&self, child: Child) -> (u32, usize) {
        match child {
            Child::State(leaf) => (self.region.states[leaf].depth, leaf),
            Child::Parent(_) => self
                .children(child)
                .iter()
                .map(|kid| self.rank(*kid))
                .min()
                .expect("a parent has children"),
        }
    }

    fn layout(&self, children: &[Child], own: Option<&[String]>, title: Option<f64>) -> Block {
        let blocks: Vec<Option<Block>> = children
            .iter()
            .map(|child| match child {
                Child::State(_) => None,
                Child::Parent(parent) => Some(self.layout(
                    self.children(*child),
                    Some(self.path(*child)),
                    Some(self.sizes.titles[self.r][*parent]),
                )),
            })
            .collect();
        let size = |k: usize| match (&children[k], &blocks[k]) {
            (Child::State(leaf), _) => {
                let (w, h) = self.sizes.leaves[self.r][*leaf];
                (w, h + 2.0 * self.sizes.loop_room[self.r][*leaf])
            }
            (_, Some(block)) => (block.width, block.height),
            _ => unreachable!("parents have blocks"),
        };

        let measured: Vec<(f64, f64)> = (0..children.len()).map(size).collect();

        // Columns by depth, compressed to consecutive indices.
        let ranks: Vec<(u32, usize)> = children.iter().map(|child| self.rank(*child)).collect();
        let mut depths: Vec<u32> = ranks.iter().map(|(depth, _)| *depth).collect();
        depths.sort_unstable();
        depths.dedup();
        let column: Vec<usize> = ranks
            .iter()
            .map(|(depth, _)| depths.binary_search(depth).expect("listed depth"))
            .collect();

        // Arrows between two different children of this container.
        let owner = |path: &[String]| -> Option<usize> {
            children
                .iter()
                .position(|child| path.starts_with(self.path(*child)))
        };
        let mut induced: Vec<(usize, usize, f64)> = Vec::new();
        for (a, arrow) in self.region.arrows.iter().enumerate() {
            if own.is_some_and(|own| arrow.from.as_slice() == own) {
                continue;
            }
            if own.is_some_and(|own| !arrow.from.starts_with(own) || !arrow.to.starts_with(own)) {
                continue;
            }
            if let (Some(from), Some(to)) = (owner(&arrow.from), owner(&arrow.to)) {
                if from != to {
                    induced.push((from, to, self.sizes.labels[self.r][a]));
                }
            }
        }

        // Order within each column: draw order, then median sweeps.
        let columns = depths.len();
        let mut order: Vec<Vec<usize>> = vec![Vec::new(); columns];
        let mut by_rank: Vec<usize> = (0..children.len()).collect();
        by_rank.sort_by_key(|k| ranks[*k].1);
        for k in by_rank {
            order[column[k]].push(k);
        }
        let mut position = vec![0usize; children.len()];
        let refresh = |order: &[Vec<usize>], position: &mut [usize]| {
            for col in order {
                for (at, k) in col.iter().enumerate() {
                    position[*k] = at;
                }
            }
        };
        refresh(&order, &mut position);
        for sweep in 0..SWEEPS {
            let forward = sweep % 2 == 0;
            let cols: Vec<usize> = if forward {
                (1..columns).collect()
            } else {
                (0..columns.saturating_sub(1)).rev().collect()
            };
            for col in cols {
                let neighbour = if forward { col - 1 } else { col + 1 };
                let mut keyed: Vec<(f64, usize, usize)> = order[col]
                    .iter()
                    .map(|k| {
                        let mut near: Vec<usize> = induced
                            .iter()
                            .filter_map(|(a, b, _)| {
                                if *a == *k && column[*b] == neighbour {
                                    Some(position[*b])
                                } else if *b == *k && column[*a] == neighbour {
                                    Some(position[*a])
                                } else {
                                    None
                                }
                            })
                            .collect();
                        near.sort_unstable();
                        let median = if near.is_empty() {
                            position[*k] as f64
                        } else if near.len() % 2 == 1 {
                            near[near.len() / 2] as f64
                        } else {
                            (near[near.len() / 2 - 1] + near[near.len() / 2]) as f64 / 2.0
                        };
                        (median, ranks[*k].1, *k)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                order[col] = keyed.into_iter().map(|(_, _, k)| k).collect();
                refresh(&order, &mut position);
            }
        }

        // Coordinates: bands run along the main axis, items across it. A
        // container runs left to right unless that is too wide to read; then
        // it takes whichever shape is closer to 1.6:1, so only the containers
        // that must turn do.
        let starts_first =
            order.first().into_iter().flatten().any(
                |k| matches!(children[*k], Child::State(leaf) if self.region.states[leaf].start),
            );
        let (left, top, right, bottom) = match title {
            Some(_) => (
                if starts_first { START_PAD } else { PAD },
                TITLE_BAND,
                PAD,
                PAD,
            ),
            None => (if starts_first { START_PAD } else { 0.0 }, 0.0, 0.0, 0.0),
        };
        let arrange = |vertical: bool, lines: usize| -> Arrangement {
            let main = |k: usize| {
                if vertical {
                    measured[k].1
                } else {
                    measured[k].0
                }
            };
            let cross = |k: usize| {
                if vertical {
                    measured[k].0
                } else {
                    measured[k].1
                }
            };
            let band_main: Vec<f64> = (0..columns)
                .map(|band| order[band].iter().map(|k| main(*k)).fold(0.0, f64::max))
                .collect();
            let band_cross: Vec<f64> = (0..columns)
                .map(|band| {
                    let items = &order[band];
                    items.iter().map(|k| cross(*k)).sum::<f64>()
                        + ROW_GAP * items.len().saturating_sub(1) as f64
                })
                .collect();
            let gaps: Vec<f64> = (0..columns.saturating_sub(1))
                .map(|band| {
                    if vertical {
                        return BAND_GAP_DOWN;
                    }
                    let widest = induced
                        .iter()
                        .filter(|(a, b, _)| {
                            let (lo, hi) = (column[*a].min(column[*b]), column[*a].max(column[*b]));
                            lo == band && hi == band + 1
                        })
                        .map(|(_, _, width)| *width)
                        .fold(0.0, f64::max);
                    COLUMN_GAP.max(widest + 40.0)
                })
                .collect();
            // A long left-to-right chain may wrap into stacked lines of
            // similar length.
            let total = band_main.iter().sum::<f64>() + gaps.iter().sum::<f64>();
            let mut line_of = vec![0usize; columns];
            let mut start = 0.0;
            for band in 0..columns {
                let middle = start + band_main[band] / 2.0;
                line_of[band] = ((middle / (total / lines as f64)) as usize).min(lines - 1);
                start += band_main[band] + gaps.get(band).copied().unwrap_or(0.0);
            }
            let used = line_of.last().map_or(1, |last| last + 1);
            let line_cross: Vec<f64> = (0..used)
                .map(|line| {
                    (0..columns)
                        .filter(|band| line_of[*band] == line)
                        .map(|band| band_cross[band])
                        .fold(0.0, f64::max)
                })
                .collect();
            let mut positions = vec![(0.0, 0.0); children.len()];
            let (mut along, mut line_top, mut content_main) = (0.0, 0.0, 0.0f64);
            for band in 0..columns {
                if band > 0 && line_of[band] != line_of[band - 1] {
                    line_top += line_cross[line_of[band - 1]] + BAND_GAP_DOWN;
                    along = 0.0;
                }
                let mut across = line_top + (line_cross[line_of[band]] - band_cross[band]) / 2.0;
                for k in &order[band] {
                    let room = match children[*k] {
                        Child::State(leaf) => self.sizes.loop_room[self.r][leaf],
                        Child::Parent(_) => 0.0,
                    };
                    let (x, y) = if vertical {
                        (across, along + room)
                    } else {
                        (along, across + room)
                    };
                    positions[*k] = (x, y);
                    across += cross(*k) + ROW_GAP;
                }
                along += band_main[band];
                content_main = content_main.max(along);
                if band + 1 < columns && line_of[band + 1] == line_of[band] {
                    along += gaps[band];
                }
            }
            // Every other wrapped line runs right to left, so the step from
            // one line to the next stays short at the turn.
            if used > 1 {
                for band in (0..columns).filter(|band| line_of[*band] % 2 == 1) {
                    for k in &order[band] {
                        positions[*k].0 = content_main - positions[*k].0 - measured[*k].0;
                    }
                }
            }
            for position in &mut positions {
                *position = (left + position.0, top + position.1);
            }
            let content_cross = line_cross.iter().sum::<f64>() + BAND_GAP_DOWN * (used - 1) as f64;
            let (content_w, content_h) = if vertical {
                (content_cross, content_main)
            } else {
                (content_main, content_cross)
            };
            let width = match title {
                Some(title) => (left + content_w + right).max(TITLE_INSET.0 + title + PAD),
                None => left + content_w,
            };
            Arrangement {
                width: width.ceil(),
                height: (top + content_h + bottom).ceil(),
                positions,
            }
        };
        // The whole chart must also stay short enough for a phone screen,
        // where the reader scales it to the screen width.
        let fits = |shape: &Arrangement| {
            shape.width <= MAX_WIDTH && (own.is_some() || shape.height <= MAX_TALL * shape.width)
        };
        let mut shapes = vec![arrange(false, 1)];
        if !fits(&shapes[0]) {
            shapes.push(arrange(true, 1));
            if own.is_none() {
                shapes.extend([arrange(false, 2), arrange(false, 3)]);
            }
        }
        let chosen = match shapes.iter().position(fits) {
            Some(at) => shapes.swap_remove(at),
            None => {
                let best = (0..shapes.len())
                    .min_by(|a, b| shapes[*a].score().total_cmp(&shapes[*b].score()))
                    .expect("at least one shape");
                shapes.swap_remove(best)
            }
        };
        let mut blocks = blocks;
        let kids = (0..children.len())
            .map(|k| Kid {
                child: children[k],
                x: chosen.positions[k].0,
                y: chosen.positions[k].1,
                block: blocks[k].take(),
            })
            .collect();
        Block {
            width: chosen.width,
            height: chosen.height,
            kids,
        }
    }
}

/// One candidate shape of a container and where its children sit.
struct Arrangement {
    width: f64,
    height: f64,
    positions: Vec<(f64, f64)>,
}

impl Arrangement {
    /// Distance from the preferred 1.6:1 shape; past the readable width it
    /// always loses to a shape that fits.
    fn score(&self) -> f64 {
        let fit = (self.width / self.height / 1.6).ln().abs();
        if self.width > MAX_WIDTH {
            1000.0 + self.width
        } else {
            fit
        }
    }
}

fn emit(block: &Block, ox: f64, oy: f64, r: usize, sizes: &Sizes, out: &mut Placement) {
    for kid in &block.kids {
        let (x, y) = (ox + kid.x, oy + kid.y);
        match (kid.child, &kid.block) {
            (Child::State(leaf), _) => {
                let (width, height) = sizes.leaves[r][leaf];
                out.leaves[r][leaf] = Rect {
                    x,
                    y,
                    width,
                    height,
                };
            }
            (Child::Parent(parent), Some(inner)) => {
                out.frames[r][parent] = Rect {
                    x,
                    y,
                    width: inner.width,
                    height: inner.height,
                };
                out.titles[r][parent] = Rect {
                    x: x + TITLE_INSET.0,
                    y: y + TITLE_INSET.1,
                    width: sizes.titles[r][parent],
                    height: TITLE_H,
                };
                emit(inner, x, y, r, sizes, out);
            }
            _ => unreachable!("parents have blocks"),
        }
    }
}

pub(super) fn place(regions: &[Region], sizes: &Sizes, origin: (f64, f64)) -> Placement {
    let mut out = Placement {
        leaves: regions
            .iter()
            .map(|r| vec![Rect::default(); r.states.len()])
            .collect(),
        frames: regions
            .iter()
            .map(|r| vec![Rect::default(); r.parents.len()])
            .collect(),
        titles: regions
            .iter()
            .map(|r| vec![Rect::default(); r.parents.len()])
            .collect(),
        regions: Vec::new(),
        bounds: Rect::default(),
    };
    let framed = regions.len() > 1;
    let mut blocks: Vec<Block> = regions
        .iter()
        .enumerate()
        .map(|(r, region)| {
            let ctx = Ctx { region, r, sizes };
            ctx.layout(&region.top, None, framed.then(|| sizes.plates[r]))
        })
        .collect();
    let widest = blocks.iter().map(|block| block.width).fold(0.0, f64::max);
    let (mut y, x) = (origin.1, origin.0);
    for (r, block) in blocks.iter_mut().enumerate() {
        if framed {
            block.width = widest;
            out.regions.push(Rect {
                x,
                y,
                width: block.width,
                height: block.height,
            });
        }
        emit(block, x, y, r, sizes, &mut out);
        y += block.height + REGION_GAP;
    }
    let mut right: f64 = origin.0;
    let mut bottom: f64 = origin.1;
    let mut grow = |rect: &Rect| {
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    };
    out.leaves
        .iter()
        .chain(&out.frames)
        .flatten()
        .for_each(&mut grow);
    out.regions.iter().for_each(&mut grow);
    out.bounds = Rect {
        x: origin.0,
        y: origin.1,
        width: right - origin.0,
        height: bottom - origin.1,
    };
    out
}

/// The width of one line of text plus its plate padding.
pub(super) fn text_width(text: &str, font: f64) -> f64 {
    (crate::geometry::text_units(text) * font * 0.6 + 12.0).ceil()
}
