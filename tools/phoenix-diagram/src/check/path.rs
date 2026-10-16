// Ported from the frozen Archify svg-path-analysis module; see NOTICE.md.
use crate::geometry::{cross_product, finite_point, finite_rect};
use crate::scene::{Point, Rect};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Subpath {
    pub points: Vec<Point>,
    pub segments: Vec<Segment>,
    pub straight_segments: Vec<Segment>,
    pub closed: bool,
    /// Bounds of the sampled points, not analytic Bezier extrema.
    pub bounds: Option<Rect>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathAnalysis {
    pub subpaths: Vec<Subpath>,
    /// Bounds of the sampled points; absent for empty or overflowing extents.
    pub bounds: Option<Rect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathError {
    pub code: &'static str,
    /// UTF-16 offset in `d`, matching the frozen JavaScript analyzer.
    pub token_offset: usize,
    pub reason: String,
    pub command: Option<char>,
}

impl fmt::Display for PathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at d offset {}.",
            self.reason, self.token_offset
        )
    }
}

impl std::error::Error for PathError {}

impl PathAnalysis {
    /// Project only actual straight pieces, retaining empty and disconnected parts.
    pub fn route_subpaths(&self) -> Vec<Vec<Point>> {
        let mut parts = Vec::new();
        for subpath in &self.subpaths {
            let mut continuous: Vec<Vec<Point>> = Vec::new();
            for segment in &subpath.straight_segments {
                if let Some(previous) = continuous
                    .last_mut()
                    .filter(|part| part.last() == Some(&segment.start))
                {
                    previous.push(segment.end);
                } else {
                    continuous.push(vec![segment.start, segment.end]);
                }
            }
            if continuous.is_empty() {
                continuous.push(Vec::new());
            }
            parts.extend(continuous);
        }
        parts
    }
}

#[derive(Clone, Copy)]
enum TokenKind {
    Command(char),
    Number(f64),
}

struct Token {
    kind: TokenKind,
    offset: usize,
}

fn failure(
    offset: usize,
    reason: impl Into<String>,
    command: Option<char>,
    unsupported: bool,
) -> PathError {
    PathError {
        code: if unsupported {
            "artifact/svg-path-unsupported"
        } else {
            "artifact/svg-path-malformed"
        },
        token_offset: offset,
        reason: reason.into(),
        command,
    }
}

// The sticky source regex permits adjacent signed/decimal numbers and only
// consumes an exponent when its digits are complete.
fn number_end(source: &[u8], start: usize) -> Option<usize> {
    let mut offset = start;
    if matches!(source.get(offset), Some(b'+' | b'-')) {
        offset += 1;
    }
    let integer_start = offset;
    while source.get(offset).is_some_and(u8::is_ascii_digit) {
        offset += 1;
    }
    let integer_digits = offset - integer_start;
    if source.get(offset) == Some(&b'.') {
        offset += 1;
        let fraction_start = offset;
        while source.get(offset).is_some_and(u8::is_ascii_digit) {
            offset += 1;
        }
        if integer_digits == 0 && fraction_start == offset {
            return None;
        }
    } else if integer_digits == 0 {
        return None;
    }
    if matches!(source.get(offset), Some(b'e' | b'E')) {
        let mut exponent_end = offset + 1;
        if matches!(source.get(exponent_end), Some(b'+' | b'-')) {
            exponent_end += 1;
        }
        let exponent_start = exponent_end;
        while source.get(exponent_end).is_some_and(u8::is_ascii_digit) {
            exponent_end += 1;
        }
        if exponent_end != exponent_start {
            offset = exponent_end;
        }
    }
    Some(offset)
}

fn tokenize(source: &str) -> Result<Vec<Token>, PathError> {
    let bytes = source.as_bytes();
    let mut tokens: Vec<Token> = Vec::new();
    let mut offset = 0;
    let mut comma = None;
    while let Some(&character) = bytes.get(offset) {
        if matches!(character, b' ' | b'\t' | b'\r' | b'\n') {
            offset += 1;
            continue;
        }
        if character == b',' {
            if comma.is_some()
                || !tokens
                    .last()
                    .is_some_and(|token| matches!(token.kind, TokenKind::Number(_)))
            {
                return Err(failure(offset, "unexpected comma", None, false));
            }
            comma = Some(offset);
            offset += 1;
            continue;
        }
        if b"MLHVQZACSTmlhvqzacst".contains(&character) {
            if let Some(offset) = comma {
                return Err(failure(
                    offset,
                    "a comma must separate numbers",
                    None,
                    false,
                ));
            }
            tokens.push(Token {
                kind: TokenKind::Command(character as char),
                offset,
            });
            offset += 1;
            continue;
        }
        let end = number_end(bytes, offset).ok_or_else(|| {
            failure(
                source[..offset].encode_utf16().count(),
                "expected a number or supported path command",
                None,
                false,
            )
        })?;
        let value = source[offset..end]
            .parse::<f64>()
            .map_err(|_| failure(offset, "coordinate is not finite", None, false))?;
        if !value.is_finite() {
            return Err(failure(offset, "coordinate is not finite", None, false));
        }
        tokens.push(Token {
            kind: TokenKind::Number(value),
            offset,
        });
        offset = end;
        comma = None;
    }
    if let Some(offset) = comma {
        return Err(failure(offset, "trailing comma", None, false));
    }
    Ok(tokens)
}

fn move_to(subpaths: &mut Vec<Subpath>, current: &mut Point, point: Point) {
    *current = point;
    subpaths.push(Subpath {
        points: vec![point],
        ..Subpath::default()
    });
}

fn line_to(subpath: &mut Subpath, current: &mut Point, point: Point) {
    let segment = Segment {
        start: *current,
        end: point,
    };
    subpath.segments.push(segment);
    subpath.straight_segments.push(segment);
    subpath.points.push(point);
    *current = point;
}

fn sampled_bounds(mut points: impl Iterator<Item = Point>) -> Option<Rect> {
    let first = points.next()?;
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }
    let bounds = Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };
    finite_rect(&bounds).then_some(bounds)
}

/// Analyze the frozen artifact subset M/L/H/V/Q/Z. A failure discards all
/// geometry; C/A/S/T are recognized and explicitly rejected by the contract.
pub fn analyze_svg_path(source: &str) -> Result<PathAnalysis, PathError> {
    let tokens = tokenize(source)?;
    let mut subpaths: Vec<Subpath> = Vec::new();
    let mut index = 0;
    let mut command = None;
    let mut command_offset = 0;
    let mut current = Point::default();
    while index < tokens.len() {
        if let TokenKind::Command(next) = tokens[index].kind {
            command = Some(next);
            command_offset = tokens[index].offset;
            index += 1;
        }
        let command_value = command.ok_or_else(|| {
            failure(
                tokens[index].offset,
                "a command is required after closepath",
                None,
                false,
            )
        })?;
        let upper = command_value.to_ascii_uppercase();
        if subpaths.is_empty() && upper != 'M' {
            return Err(failure(
                command_offset,
                "the first command must be moveto",
                command,
                false,
            ));
        }
        let arity = match upper {
            'M' | 'L' => 2,
            'H' | 'V' => 1,
            'Q' => 4,
            'Z' => {
                let subpath = subpaths.last_mut().expect("moveto precedes closepath");
                let start = subpath.points[0];
                line_to(subpath, &mut current, start);
                subpath.closed = true;
                command = None;
                continue;
            }
            _ => {
                return Err(failure(
                    command_offset,
                    "path command is not supported by artifact analysis",
                    command,
                    true,
                ))
            }
        };
        let parameter_offset = tokens.get(index).map_or(source.len(), |token| token.offset);
        let mut parameters = [0.0; 4];
        for parameter in parameters.iter_mut().take(arity) {
            let token = tokens.get(index);
            if let Some(Token {
                kind: TokenKind::Number(value),
                ..
            }) = token
            {
                *parameter = *value;
                index += 1;
            } else {
                return Err(failure(
                    token.map_or(source.len(), |token| token.offset),
                    format!("command {command_value} requires {arity} parameters"),
                    command,
                    false,
                ));
            }
        }
        let relative = command_value != upper;
        let point = |x, y| {
            if relative {
                Point {
                    x: current.x + x,
                    y: current.y + y,
                }
            } else {
                Point { x, y }
            }
        };
        let control = (upper == 'Q').then(|| point(parameters[0], parameters[1]));
        let end = match upper {
            'H' => Point {
                x: if relative {
                    current.x + parameters[0]
                } else {
                    parameters[0]
                },
                y: current.y,
            },
            'V' => Point {
                x: current.x,
                y: if relative {
                    current.y + parameters[0]
                } else {
                    parameters[0]
                },
            },
            'Q' => point(parameters[2], parameters[3]),
            _ => point(parameters[0], parameters[1]),
        };
        if !finite_point(end) || control.is_some_and(|point| !finite_point(point)) {
            return Err(failure(
                parameter_offset,
                "computed coordinate is not finite",
                command,
                false,
            ));
        }
        if upper == 'M' {
            move_to(&mut subpaths, &mut current, end);
            command = Some(if relative { 'l' } else { 'L' });
            continue;
        }
        if subpaths.last().expect("moveto precedes drawing").closed {
            let start = current;
            move_to(&mut subpaths, &mut current, start);
        }
        let subpath = subpaths.last_mut().expect("moveto precedes drawing");
        let Some(control) = control else {
            line_to(subpath, &mut current, end);
            continue;
        };
        let start = current;
        if cross_product(start, control, end).abs() <= 1e-9 {
            subpath.straight_segments.push(Segment { start, end });
        }
        for step in 1..=8 {
            let amount = f64::from(step) / 8.0;
            let remaining = 1.0 - amount;
            let coordinate = |start, control, end| {
                remaining * remaining * start
                    + 2.0 * remaining * amount * control
                    + amount * amount * end
            };
            let next = Point {
                x: coordinate(start.x, control.x, end.x),
                y: coordinate(start.y, control.y, end.y),
            };
            if !finite_point(next) {
                return Err(failure(
                    parameter_offset,
                    "sampled coordinate is not finite",
                    command,
                    false,
                ));
            }
            subpath.segments.push(Segment {
                start: current,
                end: next,
            });
            subpath.points.push(next);
            current = next;
        }
        current = end;
    }
    for subpath in &mut subpaths {
        subpath.bounds = sampled_bounds(subpath.points.iter().copied());
    }
    let bounds = sampled_bounds(
        subpaths
            .iter()
            .flat_map(|subpath| subpath.points.iter().copied()),
    );
    Ok(PathAnalysis { subpaths, bounds })
}
