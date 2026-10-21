// Ported from the frozen Archify 3.1.0 renderers/timeline/render-timeline.mjs; see NOTICE.md.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::format::number::js_round;
use crate::format::time::{self, Zone};
use crate::geometry::text::is_js_whitespace;
use crate::geometry::text_units;
use crate::i18n::Locale;
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::{
    Authored, Card, CompiledDiagram, Container, DiagramFacts, Node, Point, Rect, Text,
    TimelineEvent,
};
use serde_json::{json, Value};

const MARGIN: f64 = 28.0;
const TOP: f64 = 52.0;
const AXIS_H: f64 = 60.0;
const EDGE_PAD: f64 = 72.0;
const BREAK_W: f64 = 60.0;
const LANE_GAP: f64 = 10.0;
const LANE_PAD: f64 = 14.0;
// Room between a lane's time line and the nearest card, which the stem crosses.
const STEM_GAP: f64 = 16.0;
const ROW_GAP: f64 = 8.0;
const CARD_MAX_W: f64 = 200.0;
const CARD_PAD_X: f64 = 10.0;
const CARD_ACCENT: f64 = 12.0;
const TITLE_FONT: f64 = 12.5;
const TITLE_LINE: f64 = 15.0;
const TIME_FONT: f64 = 10.5;
const MIN_TICK_PX: f64 = 84.0;
const SEGMENT_FLOOR: f64 = 28.0;
const ADVANCE: f64 = 0.6;
const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const TICKS: [i64; 20] = [
    SECOND,
    2 * SECOND,
    5 * SECOND,
    10 * SECOND,
    15 * SECOND,
    30 * SECOND,
    MINUTE,
    2 * MINUTE,
    5 * MINUTE,
    10 * MINUTE,
    15 * MINUTE,
    30 * MINUTE,
    HOUR,
    2 * HOUR,
    3 * HOUR,
    6 * HOUR,
    12 * HOUR,
    DAY,
    2 * DAY,
    7 * DAY,
];
const UNITS: [(&str, i64); 4] = [("d", DAY), ("h", HOUR), ("min", MINUTE), ("s", SECOND)];
/// Legend order and paint of the event kinds.
pub(crate) const KINDS: [(&str, &str); 5] = [
    ("default", "external"),
    ("change", "frontend"),
    ("alert", "security"),
    ("action", "messagebus"),
    ("recovery", "backend"),
];

pub(crate) fn tone(kind: &str) -> &'static str {
    KINDS
        .iter()
        .find(|(candidate, _)| *candidate == kind)
        .map_or("external", |(_, tone)| tone)
}

fn units(text: &str) -> f64 {
    text_units(text)
}

/// The display clock: the zone's wall time, with seconds when any event has
/// them and dates when the events span more than one wall-clock day.
pub(crate) struct Clock {
    zone: Zone,
    seconds: bool,
    multi_day: bool,
}

impl Clock {
    pub(crate) fn of(timezone: &str, instants: &[i64]) -> Option<Self> {
        let zone = time::zone(timezone)?;
        let (first, last) = (*instants.iter().min()?, *instants.iter().max()?);
        let (a, b) = (time::wall(first, &zone), time::wall(last, &zone));
        Some(Self {
            zone,
            seconds: instants.iter().any(|t| t.rem_euclid(MINUTE) != 0),
            multi_day: (a.year, a.month, a.day) != (b.year, b.month, b.day),
        })
    }

    pub(crate) fn label(&self, ms: i64, seconds: bool, date: bool) -> String {
        let wall = time::wall(ms, &self.zone);
        let mut label = String::new();
        if date {
            label.push_str(&format!("{:02}-{:02} ", wall.month, wall.day));
        }
        label.push_str(&format!("{:02}:{:02}", wall.hour, wall.minute));
        if seconds {
            label.push_str(&format!(":{:02}", wall.second));
        }
        label
    }

    fn date(&self, ms: i64) -> String {
        let wall = time::wall(ms, &self.zone);
        format!("{}-{:02}-{:02}", wall.year, wall.month, wall.day)
    }

    pub(crate) fn offset_label(&self, ms: i64) -> String {
        time::offset_label(ms, &self.zone)
    }

    fn offset_ms(&self, ms: i64) -> i64 {
        // The reference derives the offset from the second-precision wall clock.
        let wall = time::wall(ms, &self.zone);
        let local = chrono::NaiveDate::from_ymd_opt(wall.year as i32, wall.month, wall.day)
            .and_then(|date| date.and_hms_opt(wall.hour, wall.minute, wall.second))
            .map_or(ms, |local| local.and_utc().timestamp_millis());
        local - ms.div_euclid(SECOND) * SECOND
    }

    /// The visible time of an event card. A date-only record shows its
    /// authored date and says so; it never shows the noon it is placed at.
    pub(crate) fn event_label(
        &self,
        at: &str,
        ms: i64,
        precision: &str,
        locale: &Locale,
    ) -> String {
        if precision == "day" {
            format!(
                "{} · {}",
                at.get(..10).unwrap_or(at),
                locale.text("timeline.precision.day", &[])
            )
        } else {
            self.label(ms, self.seconds, self.multi_day)
        }
    }
}

fn duration_label(ms: i64) -> String {
    let mut parts = Vec::new();
    let mut rest = ms;
    for (unit, size) in UNITS {
        let value = rest.div_euclid(size);
        if value != 0 {
            parts.push(format!("{value} {unit}"));
        }
        rest -= value * size;
        if parts.len() == 2 {
            break;
        }
    }
    if parts.is_empty() {
        "0 s".into()
    } else {
        parts.join(" ")
    }
}

// The visible break label is approximate (it carries ≈), so it rounds to the
// two most significant adjacent units.
fn approximate_duration(ms: i64) -> String {
    let number = crate::format::number::js_to_string;
    let Some(index) = UNITS.iter().position(|(_, size)| ms >= *size) else {
        return "0 s".into();
    };
    let (major, major_size) = UNITS[index];
    if index == UNITS.len() - 1 {
        return format!(
            "{} {major}",
            number(js_round(ms as f64 / major_size as f64))
        );
    }
    let (minor, minor_size) = UNITS[index + 1];
    let mut whole = (ms as f64 / major_size as f64).floor();
    let mut rest = js_round((ms as f64 - whole * major_size as f64) / minor_size as f64);
    if rest * minor_size as f64 >= major_size as f64 {
        whole += 1.0;
        rest = 0.0;
    }
    if rest != 0.0 {
        format!("{} {major} {} {minor}", number(whole), number(rest))
    } else {
        format!("{} {major}", number(whole))
    }
}

fn trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

// `String.split(/(\s+)/)`: words and the white space between them, in order.
fn words(text: &str) -> Vec<String> {
    let mut result = vec![String::new()];
    let mut space = false;
    for character in text.chars() {
        let white = is_js_whitespace(character);
        if white != space {
            result.push(String::new());
            space = white;
        }
        result.last_mut().unwrap().push(character);
    }
    if space {
        result.push(String::new());
    }
    result
}

fn wrap_title(text: &str, max_width: f64) -> Vec<String> {
    let limit = 6f64.max((max_width / (TITLE_FONT * ADVANCE * 1.05)).floor());
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words(text) {
        let joined = format!("{current}{word}");
        if units(trim(&joined)) <= limit {
            current = joined;
            continue;
        }
        if !trim(&current).is_empty() {
            lines.push(trim(&current).to_owned());
        }
        current = word.trim_start_matches(is_js_whitespace).to_owned();
        while units(&current) > limit {
            let characters: Vec<char> = current.chars().collect();
            let (mut cut, mut used) = (0, 0.0);
            while cut < characters.len() && used + units(&characters[cut].to_string()) <= limit {
                used += units(&characters[cut].to_string());
                cut += 1;
            }
            let cut = cut.max(1);
            lines.push(characters[..cut].iter().collect());
            current = characters[cut..].iter().collect();
        }
    }
    if !trim(&current).is_empty() {
        lines.push(trim(&current).to_owned());
    }
    lines
}

struct Event<'a> {
    index: usize,
    raw: &'a Value,
    id: &'a str,
    at: &'a str,
    t: i64,
    title: &'a str,
    lane: &'a str,
    kind: &'a str,
    precision: &'a str,
}

struct Break {
    from: i64,
    to: i64,
    length: i64,
    label: String,
    width: f64,
    x0: f64,
    x1: f64,
}

struct Segment {
    start: i64,
    end: i64,
    x0: f64,
    x1: f64,
}

struct Axis {
    segments: Vec<Segment>,
    breaks: Vec<Break>,
    scale: f64,
    x0: f64,
    x1: f64,
}

impl Axis {
    fn x(&self, time: i64) -> f64 {
        let segment = self
            .segments
            .iter()
            .find(|segment| time >= segment.start && time <= segment.end)
            .unwrap_or(&self.segments[0]);
        js_round((segment.x0 + (time - segment.start) as f64 * self.scale) * 100.0) / 100.0
    }
}

// A quiet period much longer than the ordinary rhythm becomes a fixed-width,
// disclosed break, so the axis stays proportional inside each segment.
fn axis(times: &[i64], lane_label_w: f64, axis_width: f64, compress: bool) -> Axis {
    let (first, last) = (times[0], times[times.len() - 1]);
    let span = last - first;
    let gaps: Vec<(i64, i64)> = times.windows(2).map(|pair| (pair[0], pair[1])).collect();
    let mut sorted: Vec<i64> = gaps.iter().map(|(from, to)| to - from).collect();
    sorted.sort_unstable();
    let median = sorted
        .get(sorted.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(0);
    let mut candidates: Vec<(i64, i64)> = if compress {
        gaps.iter()
            .copied()
            .filter(|(from, to)| {
                let length = (to - from) as f64;
                length > 8.0 * median as f64 && length > 0.1 * span as f64
            })
            .collect()
    } else {
        vec![]
    };
    candidates.sort_by_key(|(from, to)| std::cmp::Reverse(to - from));
    candidates.truncate(8);
    // Reserve room for proportional time before accepting a break.
    let mut breaks = Vec::new();
    let mut fixed = EDGE_PAD * 2.0 + SEGMENT_FLOOR;
    for (from, to) in candidates {
        let label = format!("≈ {}", approximate_duration(to - from));
        let width = BREAK_W.max((units(&label) * 10.5 * ADVANCE + 18.0).ceil());
        let added = width + SEGMENT_FLOOR;
        if fixed + added + MIN_TICK_PX > axis_width {
            continue;
        }
        fixed += added;
        breaks.push(Break {
            from,
            to,
            length: to - from,
            label,
            width,
            x0: 0.0,
            x1: 0.0,
        });
    }
    breaks.sort_by_key(|gap| gap.from);
    let mut segments = Vec::new();
    let mut start = first;
    for gap in &breaks {
        segments.push(Segment {
            start,
            end: gap.from,
            x0: 0.0,
            x1: 0.0,
        });
        start = gap.to;
    }
    segments.push(Segment {
        start,
        end: last,
        x0: 0.0,
        x1: 0.0,
    });
    let linear: i64 = segments
        .iter()
        .map(|segment| segment.end - segment.start)
        .sum();
    let x0 = MARGIN + lane_label_w;
    // Each segment gets a small floor width; the rest of the axis is shared in
    // proportion to time, one constant scale inside every segment.
    let drawable = axis_width
        - EDGE_PAD * 2.0
        - breaks.iter().fold(0.0, |sum, gap| sum + gap.width)
        - SEGMENT_FLOOR * segments.len() as f64;
    let scale = if linear > 0 {
        drawable.max(0.0) / linear as f64
    } else {
        0.0
    };
    let share = if linear > 0 {
        0.0
    } else {
        drawable.max(0.0) / segments.len() as f64
    };
    let mut x = x0 + EDGE_PAD;
    for (index, segment) in segments.iter_mut().enumerate() {
        let inner = (segment.end - segment.start) as f64 * scale + share;
        let empty = segment.end == segment.start;
        segment.x0 = x + SEGMENT_FLOOR / 2.0 + if empty { inner / 2.0 } else { 0.0 };
        segment.x1 = if empty {
            segment.x0
        } else {
            segment.x0 + inner
        };
        x += SEGMENT_FLOOR + inner;
        if let Some(gap) = breaks.get_mut(index) {
            gap.x0 = x;
            gap.x1 = x + gap.width;
            x += gap.width;
        }
    }
    Axis {
        segments,
        breaks,
        scale,
        x0,
        x1: x0 + axis_width,
    }
}

struct Tick {
    t: i64,
    x: f64,
    interval: Option<i64>,
}

// Ticks fall on the display zone's wall clock, so a +05:30 zone still ticks
// on its own whole hours.
fn ticks(axis: &Axis, clock: &Clock) -> Vec<Tick> {
    let mut ticks = Vec::new();
    for segment in &axis.segments {
        let length = segment.end - segment.start;
        if length <= 0 || axis.scale <= 0.0 {
            ticks.push(Tick {
                t: segment.start,
                x: segment.x0,
                interval: None,
            });
            continue;
        }
        let interval = TICKS
            .iter()
            .copied()
            .find(|candidate| *candidate as f64 * axis.scale >= MIN_TICK_PX)
            .unwrap_or(TICKS[TICKS.len() - 1]);
        let offset = clock.offset_ms(segment.start);
        let mut t =
            ((segment.start + offset) as f64 / interval as f64).ceil() as i64 * interval - offset;
        let before = ticks.len();
        while t <= segment.end {
            ticks.push(Tick {
                t,
                x: axis.x(t),
                interval: Some(interval),
            });
            t += interval;
        }
        // A segment too short for a regular tick still states where it starts.
        if ticks.len() == before {
            ticks.push(Tick {
                t: segment.start,
                x: segment.x0,
                interval: Some(interval),
            });
        }
    }
    ticks
}

struct EventCard<'a> {
    event: &'a Event<'a>,
    time: String,
    width: f64,
    height: f64,
    lines: Vec<String>,
    x: f64,
    left: f64,
    top: f64,
    line_y: f64,
    above: bool,
    row: usize,
}

struct Lane {
    id: String,
    label: String,
    raw: Option<usize>,
    y: f64,
    height: f64,
    line_y: f64,
}

fn text(path: String, content: &str, x: f64, y: f64, font_size: f64, anchor: &str) -> Text {
    Text {
        source_path: path,
        content: content.into(),
        position: Point { x, y },
        font_size,
        anchor: anchor.into(),
    }
}

fn authored(id: &str, path: String, kind: &str, facts: Value, provenance: &str) -> Authored {
    Authored {
        id: id.into(),
        source_path: path,
        kind: kind.into(),
        facts,
        geometry_provenance: vec![provenance.into()],
    }
}

fn container(
    authored: Authored,
    bounds: Rect,
    rule: Option<[Point; 2]>,
    text: Vec<Text>,
) -> Container {
    Container {
        authored,
        bounds,
        label_plate: None,
        rule,
        members: vec![],
        text,
    }
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Option<[Point; 2]> {
    Some([Point { x: x1, y: y1 }, Point { x: x2, y: y2 }])
}

struct LegendEntry {
    kind: &'static str,
    label: String,
    width: f64,
}

// The shared legend footprint: fixed-size swatch, gap and 13px labels in
// rows that wrap within the canvas.
fn legend(
    locale: &Locale,
    present: &[&str],
    width: f64,
) -> Result<(Vec<LegendEntry>, Vec<Vec<usize>>), Diagnostics> {
    let entries: Vec<LegendEntry> = KINDS
        .iter()
        .filter(|(kind, _)| present.contains(kind))
        .map(|(kind, _)| {
            let label = locale.text(&format!("legend.timeline.{kind}"), &[]);
            LegendEntry {
                kind,
                width: (14.0 + 8.0 + units(&label) * 13.0 * 0.62).ceil(),
                label,
            }
        })
        .collect();
    if entries.len() < 2 {
        return Ok((vec![], vec![]));
    }
    let available = width - MARGIN * 2.0;
    if let Some(entry) = entries.iter().find(|entry| entry.width > available) {
        let mut diagnostic = Diagnostic::error(
            Some("timeline"),
            "legend/label-too-wide",
            &format!("/meta/translations/legend.timeline.{}", entry.kind),
            format!(
                "timeline legend label for \"{}\" needs {}px but only {available}px is available",
                entry.kind, entry.width
            ),
        );
        diagnostic.stage = "layout".into();
        diagnostic.evidence =
            json!({"kind":entry.kind,"measuredWidthPx":entry.width,"availableWidthPx":available});
        diagnostic.supported_fixes = vec!["Shorten the legend label translation".into()];
        return Err(vec![diagnostic]);
    }
    let mut rows: Vec<Vec<usize>> = vec![vec![]];
    let mut cursor = 0.0;
    for (index, entry) in entries.iter().enumerate() {
        let row = rows.last_mut().unwrap();
        let required = if row.is_empty() { 0.0 } else { 22.0 } + entry.width;
        if !row.is_empty() && cursor + required > available {
            rows.push(vec![index]);
            cursor = entry.width;
        } else {
            row.push(index);
            cursor += required;
        }
    }
    Ok((entries, rows))
}

pub fn compile(
    input: &DiagramInput,
    _options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    let document = &input.document;
    let meta = &document["meta"];
    let locale = crate::i18n::resolve(meta);
    let timezone = meta["timezone"]
        .as_str()
        .filter(|zone| !zone.is_empty())
        .unwrap_or("UTC");
    let evidence = meta["evidence"].as_str().unwrap_or_default();
    let declared_lanes = document["lanes"].as_array().map_or(&[][..], Vec::as_slice);
    let authored_events = document["events"].as_array().map_or(&[][..], Vec::as_slice);
    let string = |value: &'_ Value, key: &str| value[key].as_str().unwrap_or_default().to_owned();
    let mut events: Vec<Event<'_>> = authored_events
        .iter()
        .enumerate()
        .map(|(index, raw)| Event {
            index,
            raw,
            id: raw["id"].as_str().unwrap_or_default(),
            at: raw["at"].as_str().unwrap_or_default(),
            t: raw["at"]
                .as_str()
                .and_then(time::parse_timestamp)
                .expect("the input gate admits only parsed timestamps"),
            title: raw["title"].as_str().unwrap_or_default(),
            lane: if declared_lanes.is_empty() {
                "__all"
            } else {
                raw["lane"].as_str().unwrap_or_default()
            },
            kind: raw["kind"].as_str().unwrap_or("default"),
            precision: raw["precision"].as_str().unwrap_or("time"),
        })
        .collect();
    events.sort_by_key(|event| (event.t, event.index));
    let mut times: Vec<i64> = events.iter().map(|event| event.t).collect();
    times.dedup();
    let clock = Clock::of(timezone, &times).expect("the input gate admits only known zones");
    let lane_label_w = if declared_lanes.is_empty() {
        0.0
    } else {
        declared_lanes
            .iter()
            .map(|lane| units(&string(lane, "label")) * 13.0 * ADVANCE * 1.06 + 24.0)
            .fold(88.0, f64::max)
            .ceil()
    };
    let axis_width = document["layout"]["width"].as_f64().unwrap_or(960.0);
    let axis = axis(
        &times,
        lane_label_w,
        axis_width,
        document["layout"]["breaks"].as_str() != Some("none"),
    );
    let ticks = ticks(&axis, &clock);
    let tick_seconds = ticks
        .iter()
        .any(|tick| tick.interval.is_some_and(|interval| interval < MINUTE));

    let mut cards: Vec<EventCard<'_>> = events
        .iter()
        .map(|event| {
            let time = clock.event_label(event.at, event.t, event.precision, &locale);
            let pad_left = CARD_ACCENT + 2.0;
            let natural = (units(event.title) * TITLE_FONT * ADVANCE * 1.05)
                .max(units(&time) * TIME_FONT * ADVANCE);
            let width = CARD_MAX_W.min(natural + pad_left + CARD_PAD_X).ceil();
            let lines = wrap_title(event.title, width - pad_left - CARD_PAD_X);
            let height = 9.0 + 12.0 + lines.len() as f64 * TITLE_LINE + 7.0;
            let x = axis.x(event.t);
            // The card is centred on its instant but never leaves the axis.
            let left = (x - width / 2.0)
                .max(axis.x0 + 4.0)
                .min(axis.x1 - width - 4.0);
            EventCard {
                event,
                time,
                width,
                height,
                lines,
                x,
                left,
                top: 0.0,
                line_y: 0.0,
                above: false,
                row: 0,
            }
        })
        .collect();

    // Within a lane, cards take whichever side of the time line lets them
    // sit closest to it without touching an earlier card; ties go below.
    let lane_list: Vec<(String, String, Option<usize>)> = if declared_lanes.is_empty() {
        vec![("__all".into(), String::new(), None)]
    } else {
        declared_lanes
            .iter()
            .enumerate()
            .map(|(index, lane)| (string(lane, "id"), string(lane, "label"), Some(index)))
            .collect()
    };
    let mut lanes = Vec::new();
    let mut y = TOP + AXIS_H;
    for (id, label, raw) in lane_list {
        let members: Vec<usize> = (0..cards.len())
            .filter(|index| cards[*index].event.lane == id)
            .collect();
        let mut ends: [Vec<f64>; 2] = [vec![], vec![]];
        let mut heights: [Vec<f64>; 2] = [vec![], vec![]];
        for &index in &members {
            let card = &cards[index];
            let fit = |side: usize| {
                ends[side]
                    .iter()
                    .position(|end| card.left >= end + 10.0)
                    .unwrap_or(ends[side].len())
            };
            let (above, below) = (fit(0), fit(1));
            let side = if above < below { 0 } else { 1 };
            let row = above.min(below);
            let (left, width, height) = (card.left, card.width, card.height);
            if row == ends[side].len() {
                ends[side].push(left + width);
                heights[side].push(height);
            } else {
                ends[side][row] = left + width;
                heights[side][row] = heights[side][row].max(height);
            }
            let card = &mut cards[index];
            card.above = side == 0;
            card.row = row;
        }
        let extent = |heights: &[f64]| {
            heights
                .iter()
                .fold(0.0, |sum, height| sum + height + ROW_GAP)
                - if heights.is_empty() { 0.0 } else { ROW_GAP }
        };
        let above_extent = extent(&heights[0]);
        let line_y = y
            + LANE_PAD
            + if above_extent != 0.0 {
                above_extent + STEM_GAP
            } else {
                10.0
            };
        let offsets = |heights: &[f64]| {
            let mut out = Vec::with_capacity(heights.len());
            let mut offset = STEM_GAP;
            for height in heights {
                out.push(offset);
                offset += height + ROW_GAP;
            }
            out
        };
        let (above_offset, below_offset) = (offsets(&heights[0]), offsets(&heights[1]));
        // Above the line a row is bottom-aligned to its edge nearest the line,
        // below it top-aligned, so every stem in a row has the same length.
        for &index in &members {
            let card = &mut cards[index];
            card.line_y = line_y;
            card.top = if card.above {
                line_y - above_offset[card.row] - card.height
            } else {
                line_y + below_offset[card.row]
            };
        }
        let bottom = match (below_offset.last(), heights[1].last()) {
            (Some(offset), Some(height)) => line_y + offset + height,
            _ => line_y + 10.0,
        };
        let height = 56f64.max(bottom - y + LANE_PAD);
        lanes.push(Lane {
            id,
            label,
            raw,
            y,
            height,
            line_y,
        });
        y += height + LANE_GAP;
    }

    let content_bottom = lanes
        .last()
        .map_or(TOP + AXIS_H, |lane| lane.y + lane.height);
    let width = (axis.x1 + MARGIN).ceil();
    let present: Vec<&str> = events.iter().map(|event| event.kind).collect();
    let (entries, rows) = legend(&locale, &present, width)?;
    let extra = rows.len().saturating_sub(1) as f64 * 22.0;
    let height = (content_bottom
        + MARGIN
        + if entries.is_empty() {
            0.0
        } else {
            34.0 + extra
        })
    .ceil();

    // Header: evidence badge and the axis caption with zone, date and
    // every disclosure the axis needs.
    let evidence_text = locale.text(&format!("timeline.evidence.{evidence}"), &[]);
    let evidence_w = (units(&evidence_text) * 10.5 * ADVANCE + 24.0).ceil();
    let first = times[0];
    let date = if clock.multi_day {
        String::new()
    } else {
        format!(" · {}", clock.date(first))
    };
    let zone = if timezone == "UTC" {
        "UTC".to_owned()
    } else {
        format!("{timezone} ({})", clock.offset_label(first))
    };
    let mut caption = format!("{}: {zone}{date}", locale.text("timeline.axis", &[]));
    if !axis.breaks.is_empty() {
        caption.push_str(&format!(
            " · {}",
            locale.text(
                "timeline.compressed",
                &[("count", &axis.breaks.len().to_string())]
            )
        ));
    }
    if events.iter().any(|event| event.precision == "day") {
        caption.push_str(&format!(
            " · {}",
            locale.text("timeline.precision.dayDisclosure", &[])
        ));
    }

    let mut containers = vec![container(
        authored(
            evidence,
            "/meta/evidence".into(),
            "timeline-evidence",
            json!({"evidence":evidence}),
            "renderer-owned-header",
        ),
        Rect {
            x: MARGIN,
            y: 14.0,
            width: evidence_w,
            height: 22.0,
        },
        None,
        vec![text(
            "/meta/evidence".into(),
            &evidence_text,
            MARGIN + evidence_w / 2.0 + 5.0,
            29.0,
            10.5,
            "middle",
        )],
    )];
    for lane in &lanes {
        let path = lane
            .raw
            .map_or_else(|| "/events".to_owned(), |index| format!("/lanes/{index}"));
        let mut lane_container = container(
            authored(
                &lane.id,
                path.clone(),
                "timeline-lane",
                lane.raw
                    .map_or(Value::Null, |index| declared_lanes[index].clone()),
                "timeline-lane-band",
            ),
            Rect {
                x: MARGIN - 8.0,
                y: lane.y,
                width: axis.x1 - MARGIN + 8.0,
                height: lane.height,
            },
            line(axis.x0 + 8.0, lane.line_y, axis.x1 - 8.0, lane.line_y),
            if lane.label.is_empty() {
                vec![]
            } else {
                vec![text(
                    format!("{path}/label"),
                    &lane.label,
                    MARGIN,
                    lane.line_y + 4.5,
                    13.0,
                    "start",
                )]
            },
        );
        lane_container.members = cards
            .iter()
            .filter(|card| card.event.lane == lane.id)
            .map(|card| card.event.id.to_owned())
            .collect();
        containers.push(lane_container);
    }
    // Tick labels never overprint each other or a break's label. On a
    // multi-day axis a tick names its date only when the date changes.
    let axis_y = TOP + AXIS_H - 10.0;
    let mut taken: Vec<(f64, f64)> = Vec::new();
    let mut last_date: Option<String> = None;
    for tick in &ticks {
        let wall = time::wall(tick.t, &clock.zone);
        let day = format!("{:02}-{:02}", wall.month, wall.day);
        let label = clock.label(
            tick.t,
            tick_seconds,
            clock.multi_day && last_date.as_deref() != Some(day.as_str()),
        );
        let half = units(&label) * 10.5 * ADVANCE / 2.0 + 5.0;
        let span = (tick.x - half, tick.x + half);
        if taken.iter().any(|(a, b)| span.0 < *b && *a < span.1) {
            continue;
        }
        taken.push(span);
        last_date = Some(day);
        containers.push(container(
            authored(
                &tick.t.to_string(),
                "/meta/timezone".into(),
                "timeline-tick",
                json!({"t":tick.t}),
                "timeline-wall-clock-tick",
            ),
            Rect {
                x: tick.x,
                y: axis_y - 4.0,
                width: 0.0,
                height: content_bottom - axis_y + 4.0,
            },
            line(tick.x, axis_y, tick.x, content_bottom),
            vec![text(
                "/meta/timezone".into(),
                &label,
                tick.x,
                axis_y - 10.0,
                10.5,
                "middle",
            )],
        ));
    }
    for (index, _) in axis.segments.iter().enumerate() {
        let from = if index > 0 {
            axis.breaks[index - 1].x1
        } else {
            axis.x0 + 8.0
        };
        let to = axis.breaks.get(index).map_or(axis.x1 - 8.0, |gap| gap.x0);
        containers.push(container(
            authored(
                &format!("axis-{index}"),
                "/events".into(),
                "timeline-axis",
                Value::Null,
                "timeline-proportional-segment",
            ),
            Rect {
                x: from,
                y: axis_y,
                width: to - from,
                height: 0.0,
            },
            line(from, axis_y, to, axis_y),
            vec![],
        ));
    }
    for gap in &axis.breaks {
        let mid = (gap.x0 + gap.x1) / 2.0;
        let mut marker = container(
            authored(
                &format!("break-{}", gap.from),
                "/layout/breaks".into(),
                "timeline-break",
                json!({"from":gap.from,"to":gap.to,"length":gap.length,"duration":duration_label(gap.length)}),
                "timeline-disclosed-break",
            ),
            Rect {
                x: gap.x0 + 6.0,
                y: axis_y,
                width: gap.width - 12.0,
                height: content_bottom - axis_y,
            },
            line(gap.x0, axis_y, gap.x1, axis_y),
            vec![text(
                "/layout/breaks".into(),
                &gap.label,
                mid,
                axis_y - 29.0,
                10.0,
                "middle",
            )],
        );
        marker.label_plate = Some(Rect {
            x: mid - gap.width / 2.0 + 4.0,
            y: axis_y - 41.0,
            width: gap.width - 8.0,
            height: 17.0,
        });
        containers.push(marker);
    }
    // Stems sit behind every card and join each card to its exact instant.
    for card in &cards {
        let mut stem = container(
            authored(
                card.event.id,
                format!("/events/{}/at", card.event.index),
                "timeline-stem",
                json!({"kind":card.event.kind}),
                "timeline-instant",
            ),
            Rect {
                x: card.x - 7.5,
                y: card.line_y - 7.5,
                width: 15.0,
                height: 15.0,
            },
            line(
                card.x,
                card.line_y,
                card.x,
                if card.above {
                    card.top + card.height
                } else {
                    card.top
                },
            ),
            vec![],
        );
        stem.members = vec![card.event.id.to_owned()];
        containers.push(stem);
    }

    let nodes = cards
        .iter()
        .map(|card| {
            let event = card.event;
            let text_x = card.left + CARD_ACCENT + 2.0;
            let mut y = card.top + 9.0 + 10.0;
            let mut texts = vec![text(
                format!("/events/{}/at", event.index),
                &card.time,
                text_x,
                y,
                TIME_FONT,
                "start",
            )];
            y += 2.0;
            for line in &card.lines {
                y += TITLE_LINE;
                texts.push(text(
                    format!("/events/{}/title", event.index),
                    line,
                    text_x,
                    y,
                    TITLE_FONT,
                    "start",
                ));
            }
            Node {
                authored: Authored {
                    id: event.id.into(),
                    source_path: format!("/events/{}", event.index),
                    kind: "timeline-event".into(),
                    facts: event.raw.clone(),
                    geometry_provenance: vec!["timeline-instant".into()],
                },
                bounds: Rect {
                    x: card.left,
                    y: card.top,
                    width: card.width,
                    height: card.height,
                },
                sigil: None,
                label: event.title.into(),
                text: texts,
                detail_rows: vec![],
            }
        })
        .collect();

    let mut legends = Vec::new();
    if !entries.is_empty() {
        let baseline = height - 16.0;
        let title_y = baseline - extra - 20.0;
        legends.push(Card {
            authored: authored(
                "legend-title",
                "/meta".into(),
                "legend-title",
                Value::Null,
                "resolved-legend-footprint",
            ),
            bounds: Rect {
                x: MARGIN,
                y: title_y - 10.0,
                width: 48.0,
                height: 14.0,
            },
            text: vec![text(
                "/meta".into(),
                &locale.text("legend.title", &[]),
                MARGIN,
                title_y,
                12.0,
                "start",
            )],
        });
        for (row_index, row) in rows.iter().enumerate() {
            let row_baseline = baseline - (rows.len() - row_index - 1) as f64 * 22.0;
            let mut x = MARGIN;
            for &index in row {
                let entry = &entries[index];
                legends.push(Card {
                    authored: authored(
                        &format!("legend-{}", entry.kind),
                        format!("/events/kind/{}", entry.kind),
                        entry.kind,
                        Value::Null,
                        "resolved-legend-footprint",
                    ),
                    bounds: Rect {
                        x,
                        y: row_baseline - 10.0,
                        width: entry.width,
                        height: 14.0,
                    },
                    text: vec![text(
                        format!("/events/kind/{}", entry.kind),
                        &entry.label,
                        x + 14.0 + 8.0,
                        row_baseline,
                        13.0,
                        "start",
                    )],
                });
                x += entry.width + 22.0;
            }
        }
    }

    Ok(CompiledDiagram {
        diagram_type: DiagramType::Timeline,
        schema_version: document["schema_version"].as_u64().unwrap_or(1) as u32,
        input_contract: input.input_contract,
        quality: input.quality,
        meta: meta.clone(),
        view_box: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        },
        reader_fit: "intrinsic-height".into(),
        nodes,
        relations: vec![],
        containers,
        text: vec![text(
            "/meta/timezone".into(),
            &caption,
            MARGIN + evidence_w + 14.0,
            29.5,
            11.5,
            "start",
        )],
        legends,
        cards: super::tree::cards(document),
        facts: DiagramFacts::Timeline {
            timezone: timezone.into(),
            evidence: evidence.into(),
            lanes: declared_lanes.to_vec(),
            events: authored_events
                .iter()
                .map(|raw| TimelineEvent {
                    id: string(raw, "id"),
                    lane: raw["lane"].as_str().map(str::to_owned),
                    at_ms: raw["at"]
                        .as_str()
                        .and_then(time::parse_timestamp)
                        .unwrap_or_default(),
                    precision: raw["precision"].as_str().unwrap_or("time").into(),
                    kind: raw["kind"].as_str().unwrap_or("default").into(),
                })
                .collect(),
            breaks: axis.breaks.iter().map(|gap| [gap.from, gap.to]).collect(),
        },
        diagnostics: vec![],
    })
}
