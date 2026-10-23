//! The `xgent/` text rules: source-level checks for the text engines.
//!
//! Two rules live here (§5.3): `xgent/transition-all` over CSS declarations and
//! Tailwind class strings, and `xgent/focus-outline-removed` over CSS blocks
//! and Tailwind focus variants. Both are plain line scans over the source file;
//! neither parses a stylesheet, so both err toward the structural signals the
//! plan defines rather than guessing intent. Comments are blanked (not deleted)
//! before a scan, so an explanatory example cannot become a finding while line
//! numbers and offsets still point at the original source.

use std::collections::BTreeMap;

use impeccable_core::findings::{finding_for, Finding};
use impeccable_core::js;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{FOCUS_OUTLINE_REMOVED, TRANSITION_ALL};

/// The extensions the text rules inspect (§5.3 通用约定). Any other extension
/// is not a source format these rules know how to read.
pub(crate) const TEXT_EXTS: &[&str] = &[
    ".css", ".scss", ".sass", ".less", ".html", ".htm", ".vue", ".svelte", ".astro", ".jsx",
    ".tsx", ".js", ".ts", ".mjs", ".cjs",
];

const CSS_LIKE_EXTS: &[&str] = &[".css", ".scss", ".sass", ".less"];
const JS_LIKE_EXTS: &[&str] = &[".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"];
/// Markup formats whose CSS lives only in `<style>` blocks and `style="…"`
/// attributes; their body copy is not stylesheet text (§5.3, F-08).
const MARKUP_EXTS: &[&str] = &[".html", ".htm", ".vue", ".svelte", ".astro"];

/// `transition`, `transition-property` set to `all`.
static TRANSITION_PROPERTY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\btransition(?:-property)?\s*:\s*['"]?all\b"#)
        .expect("static transition regex")
});

/// A Tailwind `transition-all` class, with or without variant prefixes.
static TRANSITION_CLASS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:^|[\s"'`{])(?:[\w-]+:)*transition-all(?:$|[\s"'`}])"#)
        .expect("static transition class regex")
});

/// An unnested CSS block: selector text, then declarations.
static CSS_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([^{}]+)\{([^{}]*)\}").expect("static css block regex"));

/// A `<style>` block's body in a markup file.
static STYLE_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<style\b[^>]*>(.*?)</style\s*>").expect("static style block regex")
});

/// A double-quoted `style="…"` attribute value in a markup file.
static STYLE_ATTR_DOUBLE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)(?:[^\w-]|^)style\s*=\s*"(?P<value>[^"]*)""#)
        .expect("static style attribute regex")
});

/// A single-quoted `style='…'` attribute value in a markup file.
static STYLE_ATTR_SINGLE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)(?:[^\w-]|^)style\s*=\s*'(?P<value>[^']*)'")
        .expect("static style attribute regex")
});

static DOUBLE_QUOTED_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""([^"]*)""#).expect("static double quoted regex"));
static SINGLE_QUOTED_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"'([^']*)'").expect("static single quoted regex"));
static BACKTICK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"`([^`]*)`").expect("static backtick regex"));

/// Declarations that already mark focus in place of the removed outline.
const REPLACEMENT_PROPERTIES: &[&str] = &[
    "box-shadow",
    "border",
    "background",
    "text-decoration",
    "outline-color",
];

/// Run both text rules over one source file.
pub(crate) fn check(content: &str, file_path: &str, ext: &str) -> Vec<Finding> {
    if !TEXT_EXTS.contains(&ext) {
        return Vec::new();
    }
    // Commented-out examples are not code: blank them before either scan.
    let blanked = blank_comments(content, ext);
    let mut findings = Vec::new();
    check_transition_all(content, &blanked, file_path, ext, &mut findings);
    check_focus_outline(&blanked, file_path, &mut findings);
    findings
}

fn check_transition_all(
    content: &str,
    blanked: &str,
    file_path: &str,
    ext: &str,
    findings: &mut Vec<Finding>,
) {
    if MARKUP_EXTS.contains(&ext) {
        // Body copy in markup quotes CSS examples; only the stylesheet
        // positions — `<style>` bodies and `style="…"` values — are
        // declarations (F-08). The Tailwind class branch still reads the
        // comment-blanked source, so a real class attribute keeps hitting.
        let styles = mask_markup_styles(blanked);
        for (index, ((line, classes), css)) in content
            .split('\n')
            .zip(blanked.split('\n'))
            .zip(styles.split('\n'))
            .enumerate()
        {
            if TRANSITION_PROPERTY_RE.is_match(css) || TRANSITION_CLASS_RE.is_match(classes) {
                findings.push(finding_for(
                    &TRANSITION_ALL,
                    file_path,
                    &truncate_chars(js::trim(line), 120),
                    (index + 1) as f64,
                ));
            }
        }
        return;
    }
    // CSS comments are blanked for CSS-like files so a commented-out
    // `transition: all` is not a hit, while the wildcard selector line
    // `* { transition: all }` stays a hit (comments and selectors must not be
    // conflated by a leading-`*` shortcut). JS/TS files keep their documented
    // behavior and only skip `//` lines.
    let skip_line_comments = JS_LIKE_EXTS.contains(&ext);
    let scanned: &str = if CSS_LIKE_EXTS.contains(&ext) {
        blanked
    } else {
        content
    };
    for (index, line) in scanned.split('\n').enumerate() {
        if skip_line_comments && js::trim_start(line).starts_with("//") {
            continue;
        }
        if TRANSITION_PROPERTY_RE.is_match(line) || TRANSITION_CLASS_RE.is_match(line) {
            findings.push(finding_for(
                &TRANSITION_ALL,
                file_path,
                &truncate_chars(js::trim(line), 120),
                (index + 1) as f64,
            ));
        }
    }
}

fn check_focus_outline(blanked: &str, file_path: &str, findings: &mut Vec<Finding>) {
    // One finding per line, in line order, whichever branch found the evidence.
    let mut hits: BTreeMap<usize, String> = BTreeMap::new();

    // Style blocks: a `:focus` rule that drops the outline and leaves nothing
    // else to mark focus. A file that writes any `:focus-visible` alternative
    // is treated as having replaced the outline already.
    if !blanked.contains(":focus-visible") {
        for capture in CSS_BLOCK_RE.captures_iter(blanked) {
            let (Some(selector), Some(declarations)) = (capture.get(1), capture.get(2)) else {
                continue;
            };
            let selector = selector.as_str();
            if !selector.replace(":focus-within", "").contains(":focus") {
                continue;
            }
            if !removes_outline(declarations.as_str())
                || has_focus_replacement(declarations.as_str())
            {
                continue;
            }
            let leading = selector.len() - js::trim_start(selector).len();
            let start = capture.get(0).map(|m| m.start()).unwrap_or(0) + leading;
            hits.entry(line_of_offset(blanked, start))
                .or_insert_with(|| truncate_chars(js::trim(selector), 120));
        }
    }

    // Tailwind class strings: hit and waiver share one variant-chain parser, so
    // `focus:outline-none dark:focus:border-white` and
    // `[&:focus-visible]:outline-2` read as an already-written replacement.
    for (start, class_string) in class_expressions(blanked) {
        if !class_string_removes_focus(&class_string) {
            continue;
        }
        hits.entry(line_of_offset(blanked, start))
            .or_insert_with(|| truncate_chars(js::trim(&class_string), 120));
    }

    for (line, snippet) in hits {
        findings.push(finding_for(
            &FOCUS_OUTLINE_REMOVED,
            file_path,
            &snippet,
            line as f64,
        ));
    }
}

/// Blank comments with spaces, keeping every byte offset and line break so line
/// numbers and snippets still point at the original source.
///
/// Block comments are blanked for every text extension (a JSX `{/* … */}` is a
/// block comment). JS-like files also blank `//` line comments, but only
/// outside string literals: the `//` in `"https://…"` is not a comment. Markup
/// files additionally blank `<!-- … -->` comments.
fn blank_comments(source: &str, ext: &str) -> String {
    let js_like = JS_LIKE_EXTS.contains(&ext);
    let html_comments = MARKUP_EXTS.contains(&ext);
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(open) = quote {
            out.push(byte);
            if js_like && byte == b'\\' {
                // Copy the escaped byte whole so a `\"` or `\`` neither closes
                // the string nor corrupts UTF-8.
                index += 1;
                if let Some(next) = bytes.get(index) {
                    out.push(*next);
                    index += 1;
                }
                continue;
            }
            if byte == open {
                quote = None;
            } else if byte == b'\n' && open != b'`' {
                // A plain JS string cannot span a line; this keeps an
                // apostrophe in JSX text (`Don't`) from swallowing the rest.
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index = blank_delimited(&mut out, bytes, index, b"/*", b"*/");
            continue;
        }
        if js_like && byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            out.extend_from_slice(b"  ");
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                out.push(b' ');
                index += 1;
            }
            continue;
        }
        if html_comments && bytes[index..].starts_with(b"<!--") {
            index = blank_delimited(&mut out, bytes, index, b"<!--", "-->".as_bytes());
            continue;
        }
        if js_like && matches!(byte, b'"' | b'\'' | b'`') {
            quote = Some(byte);
        }
        out.push(byte);
        index += 1;
    }
    // Only whole bytes are copied, and blanked ranges become ASCII spaces, so
    // the result is valid UTF-8.
    String::from_utf8(out).expect("comment blanking preserves utf-8")
}

/// Copy `opener`…`closer` as spaces (newlines kept) and return the index after
/// the closer. An unterminated delimiter blanks the rest of the file.
fn blank_delimited(
    out: &mut Vec<u8>,
    bytes: &[u8],
    start: usize,
    opener: &[u8],
    closer: &[u8],
) -> usize {
    out.extend(std::iter::repeat_n(b' ', opener.len()));
    let mut index = start + opener.len();
    while index < bytes.len() {
        if bytes[index..].starts_with(closer) {
            out.extend(std::iter::repeat_n(b' ', closer.len()));
            return index + closer.len();
        }
        let byte = bytes[index];
        if byte == b'\n' || byte == b'\r' {
            out.push(byte);
        } else {
            out.push(b' ');
        }
        index += 1;
    }
    index
}

/// Keep only the stylesheet positions of a markup file — `<style>` bodies and
/// `style="…"`/`style='…'` attribute values on real tags — and blank everything
/// else with spaces, preserving byte offsets and newlines. A `style="…"` quoted
/// in body copy is not a declaration (F-08).
fn mask_markup_styles(source: &str) -> String {
    let mut keep = vec![false; source.len()];
    for capture in STYLE_BLOCK_RE.captures_iter(source) {
        if let Some(body) = capture.get(1) {
            keep[body.start()..body.end()].fill(true);
        }
    }
    for (tag_start, tag_end) in tag_spans(source) {
        let tag = &source[tag_start..tag_end];
        for regex in [&*STYLE_ATTR_DOUBLE_RE, &*STYLE_ATTR_SINGLE_RE] {
            for capture in regex.captures_iter(tag) {
                if let Some(value) = capture.name("value") {
                    keep[tag_start + value.start()..tag_start + value.end()].fill(true);
                }
            }
        }
    }
    let mut masked = String::with_capacity(source.len());
    for (index, ch) in source.char_indices() {
        if keep[index] || ch == '\n' || ch == '\r' {
            masked.push(ch);
        } else {
            masked.extend(std::iter::repeat_n(' ', ch.len_utf8()));
        }
    }
    masked
}

/// The byte spans of the real tags in markup. A quoted tag in body copy is not
/// one, so the scan only accepts `<name` or `</name`. Quoted attribute values
/// may contain `>`, so the scan to the closing angle bracket tracks quotes;
/// `<script>` and `<style>` bodies are code and CSS text, not tag attributes.
fn tag_spans(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let Some(offset) = source[index..].find('<') else {
            break;
        };
        let start = index + offset;
        index = start + 1;
        let mut name_start = start + 1;
        let closing = bytes.get(name_start) == Some(&b'/');
        if closing {
            name_start += 1;
        }
        let mut name_end = name_start;
        while name_end < bytes.len()
            && (bytes[name_end].is_ascii_alphanumeric()
                || matches!(bytes[name_end], b'-' | b':' | b'_'))
        {
            name_end += 1;
        }
        if name_end == name_start {
            // `<!doctype>`, `<!-- … -->`, `<=`, or a stray `<` is not a tag.
            continue;
        }
        let name = source[name_start..name_end].to_ascii_lowercase();
        let mut cursor = name_end;
        let mut quote: Option<u8> = None;
        let mut end = None;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            match quote {
                Some(open) => {
                    if byte == open {
                        quote = None;
                    }
                }
                None if byte == b'"' || byte == b'\'' => quote = Some(byte),
                None if byte == b'>' => {
                    end = Some(cursor + 1);
                    break;
                }
                None => {}
            }
            cursor += 1;
        }
        let Some(end) = end else {
            break;
        };
        spans.push((start, end));
        index = end;
        if !closing && (name == "script" || name == "style") {
            let rest = source[end..].to_ascii_lowercase();
            if let Some(close) = rest.find(&format!("</{name}")) {
                index = end + close;
            }
        }
    }
    spans
}

/// Whether a CSS declaration list removes the outline: `outline`/`outline-style`
/// set to `none`, or `outline`/`outline-width` set to a complete zero length.
/// `0.5px` is not a zero length even though it starts with `0` (F-04).
fn removes_outline(declarations: &str) -> bool {
    declarations.split(';').any(|declaration| {
        let Some((property, value)) = declaration.split_once(':') else {
            return false;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = strip_css_important(value);
        match property.as_str() {
            "outline" => css_value_components(value)
                .into_iter()
                .any(|token| token.eq_ignore_ascii_case("none") || is_zero_length(token)),
            "outline-style" => value
                .split_ascii_whitespace()
                .next()
                .is_some_and(|token| token.eq_ignore_ascii_case("none")),
            "outline-width" => value
                .split_ascii_whitespace()
                .next()
                .is_some_and(|token| token.eq_ignore_ascii_case("none") || is_zero_length(token)),
            _ => false,
        }
    })
}

/// Drop a trailing `!important`, which is a declaration priority rather than
/// part of the value: `outline:none!important` removes the outline exactly like
/// the spaced form (F-11). CSS also allows whitespace between `!` and
/// `important`, so `none ! important` reads the same.
fn strip_css_important(value: &str) -> &str {
    let trimmed = value.trim();
    let Some(bang) = trimmed.rfind('!') else {
        return trimmed;
    };
    if trimmed[bang + 1..].trim().eq_ignore_ascii_case("important") {
        return trimmed[..bang].trim_end();
    }
    trimmed
}

/// The top-level whitespace-separated components of a CSS value. Function
/// arguments (`rgba(0, 0, 0, 0.5)`, `oklch(0 0 0)`) stay one component, so a
/// `0` inside a color function is never mistaken for a zero outline width.
fn css_value_components(value: &str) -> Vec<&str> {
    let mut components = Vec::new();
    let mut depth: usize = 0;
    let mut start: Option<usize> = None;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if ch.is_ascii_whitespace() && depth == 0 {
            if let Some(component_start) = start.take() {
                components.push(&value[component_start..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(component_start) = start {
        components.push(&value[component_start..]);
    }
    components
}

/// Whether one CSS value token is a complete zero length: an optional sign and
/// decimal number parsing to zero plus an optional unit (`0`, `0px`, `0.0rem`).
fn is_zero_length(token: &str) -> bool {
    let token = token.trim();
    if token.is_empty() {
        return false;
    }
    let number_end = token
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '+' || ch == '-'))
        .unwrap_or(token.len());
    let (number, unit) = token.split_at(number_end);
    if number.is_empty() || !unit.chars().all(|ch| ch.is_ascii_alphabetic() || ch == '%') {
        return false;
    }
    number.parse::<f64>().is_ok_and(|value| value == 0.0)
}

/// The statically determined class expressions in `blanked`, as
/// `(starting byte offset, class list)` in source order. A class list is the
/// whole expression a literal belongs to — `+` concatenation, array `join`, or
/// a class-name call — and an expression the engine cannot evaluate is skipped
/// rather than asserted to lack a replacement (F-03).
fn class_expressions(blanked: &str) -> Vec<(usize, String)> {
    let literals = literals(blanked);
    let brackets = Brackets::new(blanked, &literals);
    let mut candidates: Vec<(usize, usize, usize, String)> = Vec::new();
    for (first, last) in literal_runs(blanked, &literals) {
        let start = literals[first].start;
        let Some((window, text)) =
            expression_text(blanked, &literals, &brackets, start, literals[last].end)
        else {
            continue;
        };
        candidates.push((window.0, window.1, start, text));
    }
    // Literals nested in a larger expression are one class list, so the widest
    // window decides (`cn(["a", "b"].join(" "), "c")` reads as `a b c`).
    let mut kept: Vec<(usize, usize, usize, String)> = Vec::new();
    for candidate in candidates {
        if kept
            .iter()
            .any(|other| other.0 <= candidate.0 && candidate.1 <= other.1)
        {
            continue;
        }
        kept.retain(|other| !(candidate.0 <= other.0 && other.1 <= candidate.1));
        kept.push(candidate);
    }
    kept.into_iter()
        .map(|(_, _, first_literal, text)| (first_literal, text))
        .collect()
}

/// One string literal in the comment-blanked source.
struct Literal<'a> {
    start: usize,
    end: usize,
    inner: &'a str,
    template: bool,
}

/// Every string literal in `blanked`, in source order.
fn literals(blanked: &str) -> Vec<Literal<'_>> {
    let mut literals: Vec<Literal<'_>> = Vec::new();
    for regex in [&*DOUBLE_QUOTED_RE, &*SINGLE_QUOTED_RE, &*BACKTICK_RE] {
        for capture in regex.captures_iter(blanked) {
            let (Some(whole), Some(inner)) = (capture.get(0), capture.get(1)) else {
                continue;
            };
            literals.push(Literal {
                start: whole.start(),
                end: whole.end(),
                inner: inner.as_str(),
                template: whole.as_str().starts_with('`'),
            });
        }
    }
    literals.sort_by_key(|literal| literal.start);
    literals
}

/// `(first, last)` literal indexes of each run whose literals are joined by
/// static glue only.
fn literal_runs(blanked: &str, literals: &[Literal<'_>]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < literals.len() {
        let mut last = index;
        while last + 1 < literals.len()
            && literals[last].end <= literals[last + 1].start
            && combinable_gap(&blanked[literals[last].end..literals[last + 1].start])
        {
            last += 1;
        }
        runs.push((index, last));
        index = last + 1;
    }
    runs
}

/// Whether the text between two literals only joins them: `+` concatenation or
/// `,` arguments/elements, possibly wrapped in parentheses or brackets.
fn combinable_gap(gap: &str) -> bool {
    let mut joins = false;
    for ch in gap.chars() {
        match ch {
            '+' | ',' => joins = true,
            '(' | ')' | '[' | ']' => {}
            _ if ch.is_whitespace() => {}
            _ => return false,
        }
    }
    joins
}

/// The bracket pairs of `blanked`, with string literals skipped so a bracket
/// quoted in a class name or label cannot break the nesting.
struct Brackets {
    pairs: Vec<(usize, usize)>,
}

impl Brackets {
    fn new(blanked: &str, literals: &[Literal<'_>]) -> Self {
        let mut pairs = Vec::new();
        let mut stack: Vec<(char, usize)> = Vec::new();
        let mut index = 0;
        let mut literal_index = 0;
        while index < blanked.len() {
            if let Some(literal) = literals.get(literal_index) {
                if literal.start == index {
                    index = literal.end;
                    literal_index += 1;
                    continue;
                }
            }
            let Some(ch) = blanked[index..].chars().next() else {
                break;
            };
            match ch {
                '(' | '[' | '{' => stack.push((ch, index)),
                ')' | ']' | '}' => {
                    let opener = match ch {
                        ')' => '(',
                        ']' => '[',
                        _ => '{',
                    };
                    if let Some(position) = stack.iter().rposition(|(open, _)| *open == opener) {
                        stack.truncate(position + 1);
                        if let Some((_, open_index)) = stack.pop() {
                            pairs.push((open_index, index));
                        }
                    }
                }
                _ => {}
            }
            index += ch.len_utf8();
        }
        Self { pairs }
    }

    /// The smallest pair that contains `start..end`.
    fn enclosing(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        self.pairs
            .iter()
            .copied()
            .filter(|(open, close)| *open < start && end <= *close)
            .min_by_key(|(open, close)| close - open)
    }

    fn closer_of(&self, open: usize) -> Option<usize> {
        self.pairs
            .iter()
            .find(|(pair_open, _)| *pair_open == open)
            .map(|(_, close)| *close)
    }

    fn opener_of(&self, close: usize) -> Option<usize> {
        self.pairs
            .iter()
            .find(|(_, pair_close)| *pair_close == close)
            .map(|(open, _)| *open)
    }
}

/// The class list of one literal run, with the window it was read from.
/// `None` means the expression cannot be read statically.
fn expression_text(
    blanked: &str,
    literals: &[Literal<'_>],
    brackets: &Brackets,
    start: usize,
    end: usize,
) -> Option<((usize, usize), String)> {
    match group_window(blanked, brackets, start, end) {
        GroupWindow::Transformed => None,
        GroupWindow::Group(window_start, window_end) => {
            let window = (window_start, window_end);
            if let Some(text) = evaluate(blanked, literals, window) {
                return Some((window, text));
            }
            // The group is not the class expression (`return (<button … />)`),
            // so read the immediate expression around the literal instead.
            let statement = statement_window(blanked, brackets, start, end)?;
            if statement.comma {
                // A sibling argument or element is unknown.
                return None;
            }
            let window = (statement.start, statement.end);
            Some((window, evaluate(blanked, literals, window)?))
        }
        GroupWindow::None => {
            let statement = statement_window(blanked, brackets, start, end)?;
            let window = (statement.start, statement.end);
            Some((window, evaluate(blanked, literals, window)?))
        }
    }
}

fn evaluate(blanked: &str, literals: &[Literal<'_>], window: (usize, usize)) -> Option<String> {
    let tokens = tokens_in(blanked, literals, window.0, window.1)?;
    Parser::new(&tokens).parse()
}

/// The grouped expression a literal run belongs to, when there is one.
enum GroupWindow {
    /// No enclosing group: the statement window is the expression.
    None,
    /// The inside of the enclosing call, or the array literal with its `join`.
    Group(usize, usize),
    /// `[…].map(…)` and friends: a member call the engine cannot read.
    Transformed,
}

fn group_window(blanked: &str, brackets: &Brackets, start: usize, end: usize) -> GroupWindow {
    let Some((open, close)) = brackets.enclosing(start, end) else {
        return GroupWindow::None;
    };
    match blanked[open..].chars().next() {
        Some('(') => GroupWindow::Group(open + 1, close),
        Some('[') => match join_call_end(blanked, brackets, close) {
            Some(end) => GroupWindow::Group(open, end),
            None => GroupWindow::Transformed,
        },
        // A block or object body also holds statements: read the expression
        // inside it (`const label = "…";`), not the whole body.
        _ => GroupWindow::None,
    }
}

/// The end of a `[…].join(…)` member expression chained to the array closing at
/// `close`, or the array's own end when nothing is chained to it. Any other
/// member call may transform the list, so it makes the expression unreadable.
fn join_call_end(blanked: &str, brackets: &Brackets, close: usize) -> Option<usize> {
    let index = skip_whitespace(blanked, close + 1);
    if !blanked[index..].starts_with('.') {
        return Some(close + 1);
    }
    let after_dot = skip_whitespace(blanked, index + 1);
    if !blanked[after_dot..].starts_with("join") {
        return None;
    }
    let paren = skip_whitespace(blanked, after_dot + "join".len());
    if !blanked[paren..].starts_with('(') {
        return None;
    }
    Some(brackets.closer_of(paren)? + 1)
}

/// The immediate expression around a literal run, bounded by statement or
/// operator punctuation rather than by a containing group. A `+` continues the
/// expression; a bracket opened outside the run is walked through, and the
/// parse decides whether an unreadable operand remains.
fn statement_window(
    blanked: &str,
    brackets: &Brackets,
    start: usize,
    end: usize,
) -> Option<Window> {
    let (window_start, left_comma) = scan_left(blanked, brackets, start, end)?;
    let (window_end, right_comma) = scan_right(blanked, brackets, start, end)?;
    if window_start >= window_end {
        return None;
    }
    Some(Window {
        start: window_start,
        end: window_end,
        comma: left_comma || right_comma,
    })
}

/// A statement expression window, and whether a comma delimits it (a sibling
/// argument or element the engine cannot read).
struct Window {
    start: usize,
    end: usize,
    comma: bool,
}

fn scan_left(
    blanked: &str,
    brackets: &Brackets,
    start: usize,
    end: usize,
) -> Option<(usize, bool)> {
    let mut index = start;
    loop {
        index = skip_whitespace_back(blanked, index);
        let Some(ch) = blanked[..index].chars().next_back() else {
            return Some((0, false));
        };
        let char_start = index - ch.len_utf8();
        match ch {
            ')' | ']' | '}' => index = brackets.opener_of(char_start)?,
            '+' => index = char_start,
            '(' | '[' | '{' => {
                if brackets
                    .closer_of(char_start)
                    .is_some_and(|close| close >= start && close < end)
                {
                    // The run's own grouping: keep scanning past it.
                    index = char_start;
                    continue;
                }
                return Some((index, false));
            }
            ch if is_ident_char(ch) => return Some((index, false)),
            _ => return Some((index, ch == ',')),
        }
    }
}

fn scan_right(
    blanked: &str,
    brackets: &Brackets,
    start: usize,
    end: usize,
) -> Option<(usize, bool)> {
    let mut index = end;
    loop {
        index = skip_whitespace(blanked, index);
        let Some(ch) = blanked[index..].chars().next() else {
            return Some((blanked.len(), false));
        };
        match ch {
            ')' | ']' | '}' => {
                if brackets
                    .opener_of(index)
                    .is_some_and(|open| open >= start && open < end)
                {
                    index += ch.len_utf8();
                    continue;
                }
                return Some((index, false));
            }
            '+' => index += ch.len_utf8(),
            _ => return Some((index, ch == ',')),
        }
    }
}

/// A token of an expression window: a string literal, an identifier, or one
/// punctuation character.
enum Token<'a> {
    Str { text: &'a str, template: bool },
    Ident(&'a str),
    Punct(char),
}

fn tokens_in<'a>(
    blanked: &'a str,
    literals: &[Literal<'a>],
    start: usize,
    end: usize,
) -> Option<Vec<Token<'a>>> {
    let mut tokens = Vec::new();
    let mut index = start;
    let mut literal_index = literals.partition_point(|literal| literal.start < start);
    while index < end {
        if let Some(literal) = literals.get(literal_index) {
            if literal.start == index && literal.end <= end {
                tokens.push(Token::Str {
                    text: literal.inner,
                    template: literal.template,
                });
                index = literal.end;
                literal_index += 1;
                continue;
            }
        }
        let ch = blanked[index..end].chars().next()?;
        if ch.is_whitespace() {
            index += ch.len_utf8();
        } else if is_ident_char(ch) {
            let ident_start = index;
            while index < end {
                let next = blanked[index..end].chars().next()?;
                if !is_ident_char(next) {
                    break;
                }
                index += next.len_utf8();
            }
            tokens.push(Token::Ident(&blanked[ident_start..index]));
        } else {
            tokens.push(Token::Punct(ch));
            index += ch.len_utf8();
        }
    }
    Some(tokens)
}

/// Reads the small, static subset of JavaScript class expressions the rule can
/// evaluate: string literals, `+` concatenation, parentheses, arrays with
/// `join`, and calls whose arguments join with a space (the `cn`/`clsx`
/// convention). Anything else — identifiers, conditionals, spreads — is
/// unknown, and an unknown expression is never reported (F-03).
struct Parser<'a, 'b> {
    tokens: &'b [Token<'a>],
    index: usize,
}

impl<'a, 'b> Parser<'a, 'b> {
    fn new(tokens: &'b [Token<'a>]) -> Self {
        Parser { tokens, index: 0 }
    }

    /// The whole window, read as a comma-separated class list.
    fn parse(mut self) -> Option<String> {
        let value = self.list()?;
        (self.index == self.tokens.len()).then_some(value)
    }

    fn list(&mut self) -> Option<String> {
        let mut parts = vec![self.expression()?];
        while self.eat(',') {
            parts.push(self.expression()?);
        }
        Some(parts.join(" "))
    }

    fn expression(&mut self) -> Option<String> {
        let mut value = self.primary()?;
        while self.eat('+') {
            value.push_str(&self.primary()?);
        }
        Some(value)
    }

    fn primary(&mut self) -> Option<String> {
        match self.tokens.get(self.index)? {
            Token::Str { text, template } => {
                if *template && text.contains("${") {
                    return None;
                }
                let value = (*text).to_string();
                self.index += 1;
                Some(value)
            }
            Token::Punct('(') => {
                self.index += 1;
                let value = self.list()?;
                self.expect(')')?;
                Some(value)
            }
            Token::Punct('[') => self.array(),
            Token::Ident(_) => {
                // A class-name helper call: its arguments are the class list.
                self.index += 1;
                self.expect('(')?;
                let value = self.list()?;
                self.expect(')')?;
                Some(value)
            }
            _ => None,
        }
    }

    fn array(&mut self) -> Option<String> {
        self.expect('[')?;
        let mut parts = Vec::new();
        if !self.peek(']') {
            parts.push(self.expression()?);
            while self.eat(',') {
                if self.peek(']') {
                    break;
                }
                parts.push(self.expression()?);
            }
        }
        self.expect(']')?;
        let separator = if self.eat('.') {
            match self.tokens.get(self.index)? {
                Token::Ident("join") => {
                    self.index += 1;
                    self.expect('(')?;
                    let separator = self.expression()?;
                    self.expect(')')?;
                    separator
                }
                _ => return None,
            }
        } else {
            " ".to_string()
        };
        Some(parts.join(&separator))
    }

    fn peek(&self, punct: char) -> bool {
        matches!(self.tokens.get(self.index), Some(Token::Punct(next)) if *next == punct)
    }

    fn eat(&mut self, punct: char) -> bool {
        if self.peek(punct) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, punct: char) -> Option<()> {
        self.eat(punct).then_some(())
    }
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '$'
}

fn skip_whitespace(text: &str, mut index: usize) -> usize {
    while index < text.len() {
        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn skip_whitespace_back(text: &str, mut index: usize) -> usize {
    while index > 0 {
        let Some(ch) = text[..index].chars().next_back() else {
            break;
        };
        if !ch.is_whitespace() {
            break;
        }
        index -= ch.len_utf8();
    }
    index
}

fn has_focus_replacement(declarations: &str) -> bool {
    REPLACEMENT_PROPERTIES.iter().any(|replacement| {
        declarations.split(';').any(|declaration| {
            declaration
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
                .starts_with(replacement)
        })
    })
}

fn class_string_removes_focus(inner: &str) -> bool {
    let tokens: Vec<&str> = inner.split_whitespace().collect();
    let mut removes = false;
    let mut replacement = false;
    for token in &tokens {
        let (variants, utility) = split_variants(token);
        let focus = variants.iter().any(|variant| variant_marks_focus(variant));
        if utility == "outline-none" {
            removes |= focus;
        } else {
            replacement |= focus;
        }
    }
    let ring = tokens
        .iter()
        .any(|token| token.to_lowercase().contains("ring"));
    removes && !replacement && !ring
}

/// Whether one variant applies under keyboard focus: `focus`/`focus-visible`
/// themselves, arbitrary selector variants naming one of them (`[&:focus]`,
/// `[&:focus-visible]`), and bracketed data variants (`data-[focus]`), per
/// §5.3. Look-alike substrings are not focus itself: `focus-within`,
/// `not-focus`, and `group-focus` do not qualify (F-02).
fn variant_marks_focus(variant: &str) -> bool {
    let lowered = variant.to_ascii_lowercase();
    if lowered == "focus" || lowered == "focus-visible" {
        return true;
    }
    if lowered.starts_with('[') && lowered.ends_with(']') {
        let inner = &lowered[1..lowered.len() - 1];
        return arbitrary_selector_marks_focus(inner);
    }
    if let Some(inner) = lowered
        .strip_prefix("data-[")
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return inner == "focus" || inner == "focus-visible";
    }
    false
}

/// Whether an arbitrary-selector variant (`[&:focus]`, `[&:not(:focus)]`)
/// names focus as a positive condition. A focus pseudo-class inside a
/// `:not(…)` group describes the absence of focus, so it does not qualify.
fn arbitrary_selector_marks_focus(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    // One entry per open parenthesis: true for a `:not(`, false otherwise.
    let mut negated: Vec<bool> = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b':' => {
                let start = index + 1;
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'-' | b'_'))
                {
                    end += 1;
                }
                let name = &inner[start..end];
                if name == "not" && bytes.get(end) == Some(&b'(') {
                    negated.push(true);
                    index = end + 1;
                } else {
                    let focus = name == "focus" || name == "focus-visible";
                    if focus && !negated.iter().any(|inside_not| *inside_not) {
                        return true;
                    }
                    index = end;
                }
            }
            b'(' => {
                negated.push(false);
                index += 1;
            }
            b')' => {
                negated.pop();
                index += 1;
            }
            _ => index += 1,
        }
    }
    false
}

/// Split `a:b:util` into its variant chain and utility class. Colons inside
/// arbitrary selectors (`[&:focus-visible]`) and values (`data-[focus]`) belong
/// to the variant, not to the chain.
fn split_variants(token: &str) -> (Vec<&str>, &str) {
    let mut segments = Vec::new();
    let mut depth: usize = 0;
    let mut start = 0;
    for (index, ch) in token.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                segments.push(&token[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    segments.push(&token[start..]);
    let utility = segments.pop().unwrap_or("");
    (segments, utility)
}

fn line_of_offset(text: &str, index: usize) -> usize {
    let index = index.min(text.len());
    text.as_bytes()[..index]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
