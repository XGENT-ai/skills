// Ported from the frozen shared/automatic-labels.mjs and spatial-grid.mjs.
use crate::geometry::{
    normalize_route_points, rects_overlap, segment_rect_clearance, Point, Rect, EPSILON,
};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub relation_index: isize,
    pub bounds: Rect,
    pub position: Point,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub relation_index: isize,
    pub points: Vec<Point>,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub view_box: Rect,
    pub placement_bottom: f64,
    pub fallback_ring: bool,
    pub keep_fallback_near_route: bool,
    pub grid_sweep: bool,
    pub max_own_route_distance: f64,
}

impl Options {
    pub fn new(view_box: Rect) -> Self {
        Self {
            view_box,
            placement_bottom: view_box.height,
            fallback_ring: true,
            keep_fallback_near_route: false,
            grid_sweep: false,
            max_own_route_distance: f64::INFINITY,
        }
    }
}

#[derive(Clone, Copy)]
struct Segment {
    relation_index: isize,
    start: Point,
    end: Point,
}

struct SegmentGrid {
    buckets: HashMap<(i64, i64), Vec<usize>>,
    overflow: Vec<usize>,
    count: usize,
}

impl SegmentGrid {
    fn range(rect: Rect, margin: f64) -> Option<(i64, i64, i64, i64)> {
        let coordinates = [
            ((rect.x - margin) / 120.0).floor(),
            ((rect.x + rect.width + margin) / 120.0).floor(),
            ((rect.y - margin) / 120.0).floor(),
            ((rect.y + rect.height + margin) / 120.0).floor(),
        ];
        if coordinates
            .iter()
            .any(|value| !value.is_finite() || value.abs() > 9_007_199_254_740_991.0)
        {
            return None;
        }
        if (coordinates[1] - coordinates[0] + 1.0) * (coordinates[3] - coordinates[2] + 1.0)
            > 4096.0
        {
            return None;
        }
        Some((
            coordinates[0] as i64,
            coordinates[1] as i64,
            coordinates[2] as i64,
            coordinates[3] as i64,
        ))
    }

    fn new(segments: &[Segment]) -> Self {
        let mut grid = Self {
            buckets: HashMap::new(),
            overflow: Vec::new(),
            count: segments.len(),
        };
        for (index, segment) in segments.iter().enumerate() {
            let bounds = segment_bounds(*segment);
            let Some((x0, x1, y0, y1)) = Self::range(bounds, 0.0) else {
                grid.overflow.push(index);
                continue;
            };
            for x in x0..=x1 {
                for y in y0..=y1 {
                    grid.buckets.entry((x, y)).or_default().push(index);
                }
            }
        }
        grid
    }

    fn query(&self, rect: Rect, margin: f64) -> Vec<usize> {
        let Some((x0, x1, y0, y1)) = Self::range(rect, margin) else {
            return (0..self.count).collect();
        };
        let mut seen = vec![false; self.count];
        let mut found = Vec::new();
        for x in x0..=x1 {
            for y in y0..=y1 {
                if let Some(bucket) = self.buckets.get(&(x, y)) {
                    for &index in bucket {
                        if !seen[index] {
                            seen[index] = true;
                            found.push(index);
                        }
                    }
                }
            }
        }
        for &index in &self.overflow {
            if !seen[index] {
                seen[index] = true;
                found.push(index);
            }
        }
        found
    }
}

fn segment_bounds(segment: Segment) -> Rect {
    Rect {
        x: segment.start.x.min(segment.end.x),
        y: segment.start.y.min(segment.end.y),
        width: (segment.end.x - segment.start.x).abs(),
        height: (segment.end.y - segment.start.y).abs(),
    }
}

fn rect_at(label: &Label, position: Point) -> Label {
    Label {
        bounds: Rect {
            x: position.x - label.bounds.width / 2.0,
            y: position.y - 10.0,
            ..label.bounds
        },
        position,
        ..label.clone()
    }
}

struct Placement<'a> {
    components: &'a [Rect],
    titles: &'a [Rect],
    obstacles: Vec<Rect>,
    segments: Vec<Segment>,
    grid: SegmentGrid,
    options: &'a Options,
}

impl Placement<'_> {
    fn inside(&self, rect: &Rect) -> bool {
        rect.x >= 0.0
            && rect.y >= 0.0
            && rect.x + rect.width <= self.options.view_box.width
            && rect.y + rect.height <= self.options.view_box.height
    }

    fn masks_route(&self, label: &Label) -> bool {
        self.grid.query(label.bounds, 4.0).iter().any(|&index| {
            let segment = self.segments[index];
            segment.relation_index != label.relation_index
                && segment_rect_clearance(segment.start, segment.end, &label.bounds) + EPSILON < 4.0
        })
    }

    fn overlaps_label(&self, label: &Label, index: usize, placed: &[Label], gap: f64) -> bool {
        placed.iter().enumerate().any(|(other_index, other)| {
            index != other_index && rects_overlap(&label.bounds, &other.bounds, gap)
        })
    }

    fn clear(&self, label: &Label, index: usize, placed: &[Label]) -> bool {
        self.inside(&label.bounds)
            && label.bounds.y + label.bounds.height <= self.options.placement_bottom
            && !self
                .obstacles
                .iter()
                .any(|obstacle| rects_overlap(&label.bounds, obstacle, 2.0))
            && !self.overlaps_label(label, index, placed, 2.0)
            && !self.masks_route(label)
            && (!self.options.max_own_route_distance.is_finite()
                || self.segments.iter().any(|segment| {
                    segment.relation_index == label.relation_index
                        && segment_rect_clearance(segment.start, segment.end, &label.bounds)
                            <= self.options.max_own_route_distance
                }))
    }

    fn needs_placement(&self, label: &Label, index: usize, placed: &[Label]) -> bool {
        !self.inside(&label.bounds)
            || self
                .components
                .iter()
                .any(|rect| rects_overlap(&label.bounds, rect, -2.0))
            || self
                .titles
                .iter()
                .any(|rect| rects_overlap(&label.bounds, rect, 0.0))
            || self.overlaps_label(label, index, placed, 0.0)
            || self.masks_route(label)
    }

    fn sliding_candidates(
        &self,
        label: &Label,
        index: usize,
        segment: Segment,
        placed: &[Label],
    ) -> Vec<Label> {
        let (a, b) = (segment.start, segment.end);
        let horizontal = (a.y - b.y).abs() < EPSILON;
        if !horizontal && (a.x - b.x).abs() >= EPSILON {
            return Vec::new();
        }
        let (a_axis, b_axis, original, size, cross_size) = if horizontal {
            (
                a.x,
                b.x,
                label.bounds.x,
                label.bounds.width,
                label.bounds.height,
            )
        } else {
            (
                a.y,
                b.y,
                label.bounds.y,
                label.bounds.height,
                label.bounds.width,
            )
        };
        let low = a_axis.min(b_axis) + 8.0;
        let high = a_axis.max(b_axis) - size - 8.0;
        if high < low {
            return Vec::new();
        }
        let offsets = if horizontal {
            [a.y - 20.0, a.y + 10.0, a.y - 28.0, a.y + 18.0]
        } else {
            [
                a.x - label.bounds.width - 6.0,
                a.x + 6.0,
                a.x - label.bounds.width - 14.0,
                a.x + 14.0,
            ]
        };
        let mut candidates = Vec::new();
        for offset in offsets {
            let band = if horizontal {
                Rect {
                    x: low,
                    y: offset,
                    width: high - low + size,
                    ..label.bounds
                }
            } else {
                Rect {
                    x: offset,
                    y: low,
                    height: high - low + size,
                    ..label.bounds
                }
            };
            let mut blockers: Vec<_> = self
                .obstacles
                .iter()
                .copied()
                .map(|rect| (rect, 2.0))
                .collect();
            blockers.extend(
                placed
                    .iter()
                    .enumerate()
                    .filter(|(other_index, _)| *other_index != index)
                    .map(|(_, other)| (other.bounds, 2.0)),
            );
            blockers.extend(
                self.grid
                    .query(band, 4.0)
                    .into_iter()
                    .filter_map(|segment_index| {
                        let other = self.segments[segment_index];
                        (other.relation_index != label.relation_index)
                            .then(|| (segment_bounds(other), 4.0))
                    }),
            );
            let mut positions = vec![original.min(high).max(low), low, high];
            for (rect, gap) in blockers {
                let (along, across, along_size, across_size) = if horizontal {
                    (rect.x, rect.y, rect.width, rect.height)
                } else {
                    (rect.y, rect.x, rect.height, rect.width)
                };
                if offset + cross_size + gap <= across || across + across_size + gap <= offset {
                    continue;
                }
                positions.extend([along - gap - size, along + along_size + gap]);
            }
            let mut unique = Vec::new();
            for position in positions {
                if position >= low && position <= high && !unique.contains(&position) {
                    unique.push(position);
                }
            }
            unique.sort_by(|left, right| {
                (left - original).abs().total_cmp(&(right - original).abs())
            });
            for position in unique {
                let (x, y) = if horizontal {
                    (position, offset)
                } else {
                    (offset, position)
                };
                candidates.push(rect_at(
                    label,
                    Point {
                        x: x + label.bounds.width / 2.0,
                        y: y + 10.0,
                    },
                ));
            }
        }
        candidates
    }

    fn parallel_on_low_side(&self, label: &Label, a: Point, b: Point, horizontal: bool) -> bool {
        let (low, high, across) = if horizontal {
            (a.x.min(b.x), a.x.max(b.x), a.y)
        } else {
            (a.y.min(b.y), a.y.max(b.y), a.x)
        };
        let mut nearest: Option<f64> = None;
        for other in &self.segments {
            if other.relation_index == label.relation_index {
                continue;
            }
            let (start_along, end_along, start_across, end_across) = if horizontal {
                (other.start.x, other.end.x, other.start.y, other.end.y)
            } else {
                (other.start.y, other.end.y, other.start.x, other.end.x)
            };
            if (start_across - end_across).abs() > EPSILON {
                continue;
            }
            let distance = start_across - across;
            if distance.abs() < EPSILON || distance.abs() > 36.0 {
                continue;
            }
            if high.min(start_along.max(end_along)) - low.max(start_along.min(end_along)) <= 0.0 {
                continue;
            }
            if nearest.is_none_or(|previous| distance.abs() < previous.abs()) {
                nearest = Some(distance);
            }
        }
        nearest.is_some_and(|distance| distance < 0.0)
    }

    fn grid_candidates(&self, label: &Label) -> Vec<Point> {
        let mut ranked = Vec::new();
        for segment in self
            .segments
            .iter()
            .filter(|segment| segment.relation_index == label.relation_index)
        {
            let (a, b) = (segment.start, segment.end);
            if (a.y - b.y).abs() < EPSILON && (a.x - b.x).abs() >= label.bounds.width + 16.0 {
                let (above, below) = if self.parallel_on_low_side(label, a, b, true) {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                };
                for fraction in [0.5, 0.25, 0.75, 0.375, 0.625, 0.125, 0.875] {
                    let x = a.x + (b.x - a.x) * fraction;
                    if (x - a.x).abs().min((x - b.x).abs()) < label.bounds.width / 2.0 + 6.0 {
                        continue;
                    }
                    ranked.extend([
                        (above, Point { x, y: a.y - 10.0 }),
                        (below, Point { x, y: a.y + 20.0 }),
                        (2.0, Point { x, y: a.y + 3.0 }),
                        (3.0, Point { x, y: a.y - 18.0 }),
                        (3.0, Point { x, y: a.y + 28.0 }),
                    ]);
                }
            } else if (a.x - b.x).abs() < EPSILON && (a.y - b.y).abs() >= label.bounds.height + 16.0
            {
                let left_first = !self.parallel_on_low_side(label, a, b, false);
                for fraction in [0.5, 0.25, 0.75, 0.375, 0.625, 0.125, 0.875] {
                    let y = a.y + (b.y - a.y) * fraction;
                    if (y - a.y).abs().min((y - b.y).abs()) < label.bounds.height / 2.0 + 6.0 {
                        continue;
                    }
                    ranked.push((2.0, Point { x: a.x, y: y + 3.0 }));
                    for (step, offset) in
                        [6.0, 14.0, 22.0, 30.0, 38.0, 46.0, 54.0].iter().enumerate()
                    {
                        let tier = if step == 0 {
                            0.0
                        } else if step == 1 {
                            1.0
                        } else {
                            2.0 + step as f64
                        };
                        ranked.extend([
                            (
                                tier + if left_first { 0.0 } else { 0.5 },
                                Point {
                                    x: a.x - label.bounds.width / 2.0 - offset,
                                    y: y + 3.0,
                                },
                            ),
                            (
                                tier + if left_first { 0.5 } else { 0.0 },
                                Point {
                                    x: a.x + label.bounds.width / 2.0 + offset,
                                    y: y + 3.0,
                                },
                            ),
                        ]);
                    }
                }
            }
        }
        ranked.sort_by(|(left, _), (right, _)| left.total_cmp(right));
        ranked.into_iter().map(|(_, position)| position).collect()
    }

    fn replacement(&self, index: usize, rerank: bool, placed: &[Label]) -> Option<Label> {
        let label = &placed[index];
        if !rerank && !self.needs_placement(label, index, placed) {
            return None;
        }
        if rerank {
            for position in self.grid_candidates(label) {
                let candidate = rect_at(label, position);
                let padded = Label {
                    bounds: Rect {
                        x: candidate.bounds.x - 1.0,
                        y: candidate.bounds.y - 1.0,
                        width: candidate.bounds.width + 2.0,
                        height: candidate.bounds.height + 2.0,
                    },
                    ..candidate.clone()
                };
                if self.clear(&padded, index, placed) {
                    return Some(candidate);
                }
            }
        }
        let own_segments: Vec<_> = self
            .segments
            .iter()
            .copied()
            .filter(|segment| segment.relation_index == label.relation_index)
            .collect();
        for segment in &own_segments {
            let (a, b) = (segment.start, segment.end);
            let horizontal =
                (a.y - b.y).abs() < EPSILON && (a.x - b.x).abs() >= label.bounds.width + 4.0;
            let vertical = !horizontal
                && (a.x - b.x).abs() < EPSILON
                && (a.y - b.y).abs() >= label.bounds.height + 16.0;
            if !horizontal && !vertical {
                continue;
            }
            for fraction in [0.5, 0.25, 0.75, 0.125, 0.875] {
                let positions = if horizontal {
                    let x = a.x + (b.x - a.x) * fraction;
                    if (x - a.x).abs().min((x - b.x).abs()) < 8.0 {
                        continue;
                    }
                    [
                        Point { x, y: a.y - 10.0 },
                        Point { x, y: a.y + 20.0 },
                        Point { x, y: a.y - 18.0 },
                        Point { x, y: a.y + 28.0 },
                    ]
                } else {
                    let y = a.y + (b.y - a.y) * fraction;
                    if (y - a.y).abs().min((y - b.y).abs()) < 8.0 {
                        continue;
                    }
                    [
                        Point {
                            x: a.x - label.bounds.width / 2.0 - 6.0,
                            y: y + 3.0,
                        },
                        Point {
                            x: a.x + label.bounds.width / 2.0 + 6.0,
                            y: y + 3.0,
                        },
                        Point {
                            x: a.x - label.bounds.width / 2.0 - 14.0,
                            y: y + 3.0,
                        },
                        Point {
                            x: a.x + label.bounds.width / 2.0 + 14.0,
                            y: y + 3.0,
                        },
                    ]
                };
                for position in positions {
                    let candidate = rect_at(label, position);
                    if self.clear(&candidate, index, placed) {
                        return Some(candidate);
                    }
                }
            }
        }
        for segment in &own_segments {
            for candidate in self.sliding_candidates(label, index, *segment, placed) {
                if self.clear(&candidate, index, placed) {
                    return Some(candidate);
                }
            }
        }
        if !self.options.fallback_ring {
            return None;
        }
        let mut anchors = vec![label.position];
        anchors.extend(own_segments.iter().map(|segment| Point {
            x: (segment.start.x + segment.end.x) / 2.0,
            y: (segment.start.y + segment.end.y) / 2.0,
        }));
        let horizontal_step = label.bounds.width / 2.0 + 12.0;
        let far_step = label.bounds.width + 20.0;
        let offsets = [
            (0.0, -28.0),
            (0.0, 38.0),
            (-horizontal_step, -28.0),
            (horizontal_step, -28.0),
            (-horizontal_step, 38.0),
            (horizontal_step, 38.0),
            (-far_step, -52.0),
            (far_step, -52.0),
            (-far_step, 62.0),
            (far_step, 62.0),
            (-far_step, -76.0),
            (far_step, -76.0),
            (-far_step, 86.0),
            (far_step, 86.0),
        ];
        for anchor in anchors {
            for (dx, dy) in offsets {
                let candidate = rect_at(
                    label,
                    Point {
                        x: anchor.x + dx,
                        y: anchor.y + dy,
                    },
                );
                if self.clear(&candidate, index, placed)
                    && (!self.options.keep_fallback_near_route
                        || own_segments.iter().any(|segment| {
                            segment_rect_clearance(segment.start, segment.end, &candidate.bounds)
                                <= label.bounds.height * 2.0
                        }))
                {
                    return Some(candidate);
                }
            }
        }
        None
    }
}

pub fn place_automatic_labels(
    labels: &[Label],
    routes: &[Route],
    components: &[Rect],
    titles: &[Rect],
    options: &Options,
) -> Vec<Label> {
    let segments: Vec<_> = routes
        .iter()
        .flat_map(|route| {
            normalize_route_points(&route.points)
                .windows(2)
                .map(|pair| Segment {
                    relation_index: route.relation_index,
                    start: pair[0],
                    end: pair[1],
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let grid = SegmentGrid::new(&segments);
    let placement = Placement {
        components,
        titles,
        obstacles: components.iter().chain(titles).copied().collect(),
        segments,
        grid,
        options,
    };
    let mut placed = labels.to_vec();
    let mut pending: Vec<_> = labels
        .iter()
        .enumerate()
        .filter_map(|(index, label)| (!label.pinned).then_some(index))
        .collect();
    for pass in 0..labels.len() {
        if pending.is_empty() {
            break;
        }
        let mut moved = false;
        for &index in &pending {
            if let Some(replacement) =
                placement.replacement(index, options.grid_sweep && pass == 0, &placed)
            {
                placed[index] = replacement;
                moved = true;
            }
        }
        if !moved {
            break;
        }
        pending.retain(|&index| placement.needs_placement(&placed[index], index, &placed));
    }
    placed
}

pub fn reserved_label_rect(
    label: &Label,
    points: &[Point],
    routes: &[Route],
    labels: &[Label],
    components: &[Rect],
    view_box: Rect,
    placement_bottom: f64,
) -> Option<Label> {
    let mut context_labels = vec![Label {
        relation_index: -1,
        ..label.clone()
    }];
    context_labels.extend(labels.iter().map(|other| Label {
        relation_index: -2,
        ..other.clone()
    }));
    let mut context_routes = vec![Route {
        relation_index: -1,
        points: points.to_vec(),
    }];
    context_routes.extend_from_slice(routes);
    let mut options = Options::new(view_box);
    options.placement_bottom = placement_bottom;
    options.fallback_ring = false;
    let reserved =
        place_automatic_labels(&context_labels, &context_routes, components, &[], &options)
            .remove(0);
    (!components
        .iter()
        .any(|component| rects_overlap(&reserved.bounds, component, -2.0)))
    .then_some(reserved)
}
