use super::*;
use crate::geometry::text::{graphemes, is_han, is_js_whitespace, is_mark, word_boundaries};

pub(super) struct Note {
    pub x: f64,
    pub lines: Vec<String>,
    pub width: f64,
    pub baseline: f64,
    pub bottom: f64,
}

impl Note {
    pub fn rect(&self) -> Option<Rect> {
        self.width.is_finite().then_some(Rect {
            x: self.x,
            y: self.baseline - 7.0 * 1.2,
            width: self.width,
            height: self.bottom - (self.baseline - 7.0 * 1.2),
        })
    }
    pub fn rect_at(&self, y: f64) -> Option<Rect> {
        let top = y + 18.0 - 7.0 * 1.2;
        let bottom = y + 22.0 + (self.lines.len() as f64 - 1.0) * 11.0;
        self.width.is_finite().then_some(Rect {
            x: self.x,
            y: top,
            width: self.width,
            height: bottom - top,
        })
    }
}

pub(super) fn measure(layout: &Layout<'_>, message: &Value) -> Option<Note> {
    let raw = string(message, "note");
    if raw.is_empty() {
        return None;
    }
    let y = number(message, "y");
    let geometry = layout.geometry(message)?;
    if layout.legacy {
        return Some(Note {
            x: geometry.start.min(geometry.end) + 12.0,
            lines: vec![raw.into()],
            width: layout.units(raw) * 7.0 * 0.6,
            baseline: y + 18.0,
            bottom: y + 22.0,
        });
    }
    let from = layout.participant_index(string(message, "from"))?;
    let to = layout.participant_index(string(message, "to"))?;
    if from == to {
        return None;
    }
    let mut best: Option<(usize, f64)> = None;
    for gap in from.min(to)..from.max(to) {
        let width = layout.centers[gap + 1] - layout.centers[gap];
        if best.is_none_or(|(_, previous)| width > previous + 0.5) {
            best = Some((gap, width));
        }
    }
    let gap = best?.0;
    let x = layout.centers[gap] + 19.0;
    let width = (layout.centers[gap + 1] - 11.0 - x).max(1.0);
    let lines = if layout.units(raw) * 7.0 * 0.6 <= width {
        vec![raw.into()]
    } else {
        wrap(layout, raw, width)
    };
    let measured = lines
        .iter()
        .map(|line| layout.units(line) * 7.0 * 0.6)
        .fold(f64::NEG_INFINITY, f64::max);
    Some(Note {
        x,
        baseline: y + 18.0,
        bottom: y + 22.0 + (lines.len() as f64 - 1.0) * 11.0,
        lines,
        width: measured,
    })
}

fn whitespace_tokens(text: &str) -> Vec<String> {
    let mut result = vec![];
    let mut start = 0;
    let mut category = None;
    for (index, character) in text.char_indices() {
        let whitespace = is_js_whitespace(character);
        if category.is_some_and(|previous| previous != whitespace) {
            result.push(text[start..index].into());
            start = index;
        }
        category = Some(whitespace);
    }
    if start < text.len() {
        result.push(text[start..].into());
    }
    result
}

fn compact_tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];
    let mut indices = text.char_indices().peekable();
    while let Some((start, character)) = indices.next() {
        let whitespace = is_js_whitespace(character);
        let han = is_han(character);
        let mut end = start + character.len_utf8();
        while let Some((next_index, next)) = indices.peek().copied() {
            let take = if whitespace {
                is_js_whitespace(next)
            } else if han {
                is_han(next) || is_mark(next)
            } else {
                !is_js_whitespace(next) && !is_han(next)
            };
            if !take {
                break;
            }
            indices.next();
            end = next_index + next.len_utf8();
        }
        let token = &text[start..end];
        if han {
            let boundaries = word_boundaries(token);
            tokens.extend(
                boundaries
                    .windows(2)
                    .map(|pair| token[pair[0]..pair[1]].to_owned()),
            );
        } else if token
            .chars()
            .all(|character| "、。，；：！？）】》」』".contains(character))
            && !tokens.is_empty()
        {
            tokens.last_mut().unwrap().push_str(token);
        } else {
            tokens.push(token.to_owned());
        }
    }
    tokens
}

fn punctuation_break(piece: &str) -> usize {
    piece
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            let end = index + character.len_utf8();
            (end < piece.len() && "/.-?&=#_".contains(character)).then_some(end)
        })
        .unwrap_or(piece.len())
}

fn flush(line: &mut String, lines: &mut Vec<String>) {
    let value = line.trim_end_matches(is_js_whitespace);
    if !value.is_empty() {
        lines.push(value.to_owned());
    }
    line.clear();
}

fn wrap_words(layout: &Layout<'_>, words: &[String], width: f64) -> Vec<String> {
    let mut lines = vec![];
    let mut line = String::new();
    for word in words {
        if word.chars().all(is_js_whitespace) {
            if !line.is_empty() {
                line.push(' ');
            }
            continue;
        }
        if layout.units(&(line.clone() + word)) * 7.0 * 0.6 <= width {
            line.push_str(word);
            continue;
        }
        flush(&mut line, &mut lines);
        let mut piece = String::new();
        for cluster in graphemes(word) {
            while !piece.is_empty() && layout.units(&(piece.clone() + cluster)) * 7.0 * 0.6 > width
            {
                let cut = punctuation_break(&piece);
                lines.push(piece[..cut].to_owned());
                piece = piece[cut..].to_owned();
            }
            piece.push_str(cluster);
        }
        line = piece;
    }
    flush(&mut line, &mut lines);
    lines
}

fn wrap(layout: &Layout<'_>, text: &str, width: f64) -> Vec<String> {
    let original = wrap_words(layout, &whitespace_tokens(text), width);
    if !text.chars().any(is_han) {
        return original;
    }
    let compact = wrap_words(layout, &compact_tokens(text), width);
    if compact.len() < original.len() {
        compact
    } else {
        original
    }
}
