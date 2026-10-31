// Member signatures and their wrapping, ported from the frozen render-class.mjs.
// A member longer than its box wraps instead of being cut.
use crate::geometry::text::is_js_whitespace;
use crate::geometry::text_units;
use serde_json::Value;

// Member rows use a monospace face whose ASCII advance is the shared 0.6em.
pub(crate) const MEMBER_ADVANCE: f64 = 11.0 * 0.6;
const CONTINUATION_INDENT: usize = 2;
// A wrapped method hangs its parameters one step deeper.
const PARAMETER_INDENT: usize = 4;

pub(crate) fn glyph(visibility: &str) -> Option<&'static str> {
    match visibility {
        "public" => Some("+"),
        "protected" => Some("#"),
        "package" => Some("~"),
        "private" => Some("-"),
        _ => None,
    }
}

fn prefix(member: &Value) -> String {
    member["visibility"]
        .as_str()
        .and_then(glyph)
        .map(|glyph| format!("{glyph} "))
        .unwrap_or_default()
}

/// The one-line signature a member reads as.
pub(crate) fn signature(member: &Value, method: bool) -> String {
    let name = member["name"].as_str().unwrap_or("");
    if method {
        let returns = member["returns"]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(|value| format!(": {value}"))
            .unwrap_or_default();
        format!(
            "{}{name}({}){returns}",
            prefix(member),
            member["parameters"].as_str().unwrap_or("")
        )
    } else {
        let kind = member["type"]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(|value| format!(": {value}"))
            .unwrap_or_default();
        format!("{}{name}{kind}", prefix(member))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    pub text: String,
    pub indent: usize,
}

fn trim_start(value: &str) -> &str {
    value.trim_start_matches(is_js_whitespace)
}
fn trim_end(value: &str) -> &str {
    value.trim_end_matches(is_js_whitespace)
}

fn fill_lines(pieces: &[&str], limit: f64, first_indent: usize, indent: usize) -> Vec<Line> {
    let mut lines: Vec<Line> = vec![];
    let mut current = String::new();
    let room = |lines: &Vec<Line>| {
        limit
            - if lines.is_empty() {
                first_indent
            } else {
                indent
            } as f64
    };
    let push = |lines: &mut Vec<Line>, text: String| {
        let indent = if lines.is_empty() {
            first_indent
        } else {
            indent
        };
        lines.push(Line { text, indent });
    };
    for piece in pieces {
        if text_units(&format!("{current}{piece}")) <= room(&lines) {
            current.push_str(piece);
            continue;
        }
        if !current.is_empty() {
            let text = trim_end(&current).to_owned();
            push(&mut lines, text);
        }
        current = trim_start(piece).to_owned();
        // A single token longer than a whole line is split by character.
        while text_units(&current) > room(&lines) {
            let characters: Vec<char> = current.chars().collect();
            let mut cut = 0;
            let mut units = 0.0;
            while cut < characters.len()
                && units + text_units(&characters[cut].to_string()) <= room(&lines)
            {
                units += text_units(&characters[cut].to_string());
                cut += 1;
            }
            let cut = cut.max(1);
            push(&mut lines, characters[..cut].iter().collect());
            current = characters[cut..].iter().collect();
        }
    }
    if !current.is_empty() {
        let text = trim_end(&current).to_owned();
        push(&mut lines, text);
    }
    lines
}

// Pieces end after an opening parenthesis or a parameter comma, or start at
// the return type.
fn signature_pieces(text: &str) -> Vec<&str> {
    let mut pieces = vec![];
    let mut start = 0;
    for (index, _) in text.char_indices().skip(1) {
        let before = &text[..index];
        if before.ends_with('(') || before.ends_with(", ") || text[index..].starts_with("): ") {
            pieces.push(&text[start..index]);
            start = index;
        }
    }
    pieces.push(&text[start..]);
    pieces
}

fn parameter_pieces(text: &str) -> Vec<&str> {
    let mut pieces = vec![];
    let mut start = 0;
    for (index, _) in text.char_indices().skip(1) {
        if text[..index].ends_with(", ") {
            pieces.push(&text[start..index]);
            start = index;
        }
    }
    pieces.push(&text[start..]);
    pieces
}

/// A wrapped method is set the way a formatter sets it: the open parenthesis
/// ends the first line, parameters hang deeper, and `): Return` closes it.
pub(crate) fn wrap(text: &str, available: f64) -> Vec<Line> {
    let limit = (available / MEMBER_ADVANCE).floor().max(8.0);
    if text_units(text) <= limit {
        return vec![Line {
            text: text.into(),
            indent: 0,
        }];
    }
    let lines = fill_lines(&signature_pieces(text), limit, 0, CONTINUATION_INDENT);
    let (Some(open), Some(close)) = (text.find('('), text.rfind(')')) else {
        return lines;
    };
    if close < open {
        return lines;
    }
    let mut result = fill_lines(&[&text[..=open]], limit, 0, CONTINUATION_INDENT);
    result.extend(fill_lines(
        &parameter_pieces(&text[open + 1..close]),
        limit,
        PARAMETER_INDENT,
        PARAMETER_INDENT,
    ));
    result.extend(fill_lines(
        &[&text[close..]],
        limit,
        CONTINUATION_INDENT,
        CONTINUATION_INDENT,
    ));
    result
}
