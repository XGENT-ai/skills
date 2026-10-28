//! The `xgent/` text rules: source-level checks for the text engines.
//!
//! Two rules live here (§5.3): `xgent/transition-all` over CSS declarations and
//! Tailwind class strings, and `xgent/focus-outline-removed` over CSS blocks
//! and Tailwind focus variants. Both are plain line scans over the source file;
//! neither parses a stylesheet, so both err toward the structural signals the
//! plan defines rather than guessing intent. Comments are blanked (not deleted)
//! before a scan, so an explanatory example cannot become a finding while line
//! numbers and offsets still point at the original source.

use std::collections::{BTreeMap, HashMap, HashSet};

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
/// quoted `style` attribute values on real tags — and blank everything else
/// with spaces, preserving byte offsets and newlines. CSS quoted in body copy,
/// in another attribute's value, or in raw text is not a declaration (F-08).
fn mask_markup_styles(source: &str) -> String {
    let mut keep = vec![false; source.len()];
    for (start, end) in markup_style_spans(source) {
        keep[start..end].fill(true);
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

/// Elements whose content is text, not markup: a tag quoted inside one of them
/// is not a tag.
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "textarea", "title"];

/// The stylesheet spans of markup, read tag by tag. A tag starts at `<name` or
/// `</name`; its attributes are split at their own boundaries — quoted values
/// and `{…}` expressions are read whole — so a `style="…"` written inside
/// another attribute's value is not an attribute. Raw-text element bodies are
/// skipped whole, so a `<style>` quoted in a script is not a style block.
fn markup_style_spans(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while let Some(offset) = source[index..].find('<') {
        let start = index + offset;
        index = start + 1;
        let closing = bytes.get(index) == Some(&b'/');
        let name_start = index + usize::from(closing);
        if !bytes.get(name_start).is_some_and(u8::is_ascii_alphabetic) {
            // `<!doctype>`, `<=`, or a stray `<` in text is not a tag.
            continue;
        }
        let name_end = source[name_start..]
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | ':' | '_')))
            .map_or(source.len(), |offset| name_start + offset);
        let name = source[name_start..name_end].to_ascii_lowercase();
        let Some((end, self_closing)) = read_attributes(source, name_end, !closing, &mut spans)
        else {
            break;
        };
        index = end;
        if !closing && !self_closing && RAW_TEXT_ELEMENTS.contains(&name.as_str()) {
            // An unclosed raw-text element runs to the end of the file.
            let Some(close) = closing_tag(source, end, &name) else {
                break;
            };
            if name == "style" {
                spans.push((end, close));
            }
            index = close;
        }
    }
    spans
}

/// Read one tag's attributes from `cursor` through its closing `>`, recording
/// quoted `style`, `:style`, and `v-bind:style` values when `record` is set.
/// Returns the offset after the tag and whether it closes itself; `None` when
/// the tag is never closed.
fn read_attributes(
    source: &str,
    mut cursor: usize,
    record: bool,
    spans: &mut Vec<(usize, usize)>,
) -> Option<(usize, bool)> {
    let bytes = source.as_bytes();
    loop {
        while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'/')
        {
            cursor += 1;
        }
        match *bytes.get(cursor)? {
            b'>' => return Some((cursor + 1, bytes[cursor - 1] == b'/')),
            b'{' => {
                cursor = brace_end(source, cursor)?;
                continue;
            }
            _ => {}
        }
        let name_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && !matches!(bytes[cursor], b'=' | b'>' | b'/' | b'"' | b'\'')
        {
            cursor += 1;
        }
        if cursor == name_start {
            // A stray quote is not an attribute.
            cursor += 1;
            continue;
        }
        let name = source[name_start..cursor].to_ascii_lowercase();
        let equals = skip_whitespace(source, cursor);
        if bytes.get(equals) != Some(&b'=') {
            continue;
        }
        cursor = skip_whitespace(source, equals + 1);
        match *bytes.get(cursor)? {
            quote @ (b'"' | b'\'') => {
                let value_start = cursor + 1;
                let value_end = value_start + source[value_start..].find(char::from(quote))?;
                if record && (name == "style" || name.ends_with(":style")) {
                    spans.push((value_start, value_end));
                }
                cursor = value_end + 1;
            }
            b'{' => cursor = brace_end(source, cursor)?,
            _ => {
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && bytes[cursor] != b'>'
                {
                    cursor += 1;
                }
            }
        }
    }
}

/// The offset after the `{…}` expression opening at `open`. Quoted strings
/// inside it are skipped, so neither a brace nor a `>` in one ends it.
fn brace_end(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut cursor = open;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor + 1);
                }
            }
            quote @ (b'"' | b'\'' | b'`') => {
                cursor += 1 + source[cursor + 1..].find(char::from(quote))?;
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// The offset of the `</name` that closes a raw-text element, matched without
/// regard to case.
fn closing_tag(source: &str, from: usize, name: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = from;
    while let Some(offset) = source[index..].find("</") {
        let start = index + offset;
        let name_end = start + 2 + name.len();
        let matches_name = bytes
            .get(start + 2..name_end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name.as_bytes()));
        let ends_name = !bytes
            .get(name_end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_'));
        if matches_name && ends_name {
            return Some(start);
        }
        index = start + 2;
    }
    None
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

/// The statically determined class lists in `blanked`, as
/// `(byte offset of the list's first string, class list)` in source order.
///
/// Each string literal is resolved to the complete expression it belongs to by
/// walking outward through the forms the rule can evaluate — `+`
/// concatenation, class-name calls, arrays with `join`, grouping parentheses —
/// until a boundary such as an attribute, assignment, or statement. A literal
/// that is one branch of a conditional, or an object property value, is a class
/// list of its own when that conditional or object stands alone. Anything else
/// around the expression makes the list unknown, and an unknown list is never
/// reported (F-03). The bracket structure is computed once per file, so each
/// walk reads only the neighbors of its own expression (F-12).
fn class_expressions(blanked: &str) -> Vec<(usize, String)> {
    let source = Source::new(blanked);
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut lists = Vec::new();
    for index in 0..source.literals.len() {
        let Some(span) = source.class_list_span(index) else {
            continue;
        };
        if !seen.insert(span) {
            continue;
        }
        if let Some((first, text)) = evaluate(&blanked[span.0..span.1]) {
            lists.push((span.0 + first, text));
        }
    }
    lists.sort_by_key(|(start, _)| *start);
    lists
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

/// Words before an expression that make it something other than a value the
/// class list is built from (`case "…":`, `typeof "…"`, `"…" in classes`).
const NON_VALUE_KEYWORDS: &[&str] = &[
    "case",
    "delete",
    "in",
    "instanceof",
    "new",
    "throw",
    "typeof",
    "void",
];

/// Words that precede an expression rather than call it: `return (…)` groups,
/// `return […]` is an array.
const EXPRESSION_KEYWORDS: &[&str] = &[
    "await",
    "case",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "in",
    "instanceof",
    "new",
    "of",
    "return",
    "throw",
    "typeof",
    "void",
    "yield",
];

/// Words whose parentheses hold a condition or binding, not a class list.
const CONDITION_KEYWORDS: &[&str] = &["catch", "for", "if", "switch", "while", "with"];

/// Words that begin another statement: a conditional ends before them.
const STATEMENT_KEYWORDS: &[&str] = &[
    "class", "const", "default", "do", "else", "export", "for", "function", "if", "import", "let",
    "return", "switch", "throw", "var", "while", "yield",
];

/// The most tokens a conditional's bound is looked for on each side.
const EXTENT_STEPS: usize = 256;

/// What stands immediately before an expression.
#[derive(Clone, Copy)]
enum Before {
    /// Nothing joins the expression to what precedes it: an attribute,
    /// assignment, statement, markup, or a neighboring value.
    Boundary,
    /// The `+` at this offset.
    Plus(usize),
    Comma,
    /// The open bracket at this offset.
    Open(usize),
    /// `?`: the expression is a conditional's first branch.
    Question,
    /// `&&`, `||`, or `??`: the expression is an operand of a logical operator.
    Logical,
    /// The `:` at this offset: a conditional's second branch or a property value.
    Colon(usize),
    /// An operator or keyword that gives the expression another meaning.
    Unknown,
}

/// What stands immediately after an expression.
#[derive(Clone, Copy)]
enum After {
    Boundary,
    /// The `+` at this offset.
    Plus(usize),
    Comma,
    /// The close bracket at this offset.
    Close(usize),
    /// `||` or `??`: the expression is the operand that becomes the value.
    Logical,
    /// `:` after a conditional's first branch, or after an object key.
    Colon,
    Unknown,
}

/// The comment-blanked source with its string literals and bracket structure,
/// computed once per file.
struct Source<'a> {
    text: &'a str,
    literals: Vec<Literal<'a>>,
    by_start: HashMap<usize, usize>,
    by_end: HashMap<usize, usize>,
    /// For each literal, the innermost open bracket around it.
    literal_parent: Vec<Option<usize>>,
    close_of: HashMap<usize, usize>,
    open_of: HashMap<usize, usize>,
    /// For each matched open bracket, the open bracket around it.
    parent_of: HashMap<usize, Option<usize>>,
}

impl<'a> Source<'a> {
    fn new(text: &'a str) -> Self {
        let literals = literals(text);
        let mut by_start = HashMap::new();
        let mut by_end = HashMap::new();
        for (index, literal) in literals.iter().enumerate() {
            by_start.insert(literal.start, index);
            by_end.insert(literal.end, index);
        }
        // A bracket inside a string is not structure. Templates and one-line
        // quoted strings are skipped whole; a quote pair spanning lines is more
        // likely an apostrophe in body copy (`Don't`) than a string, so its
        // text still counts. Overlapping candidates keep the earliest.
        let mut skips: Vec<(usize, usize)> = Vec::new();
        for literal in &literals {
            let code_string = literal.template || !literal.inner.contains('\n');
            if code_string && skips.last().is_none_or(|last| literal.start >= last.1) {
                skips.push((literal.start, literal.end));
            }
        }
        let bytes = text.as_bytes();
        let mut stack: Vec<usize> = Vec::new();
        let mut close_of = HashMap::new();
        let mut open_of = HashMap::new();
        let mut parent_of = HashMap::new();
        let mut literal_parent = vec![None; literals.len()];
        let mut next_literal = 0;
        let mut next_skip = 0;
        let mut index = 0;
        while index < bytes.len() {
            while next_literal < literals.len() && literals[next_literal].start <= index {
                literal_parent[next_literal] = stack.last().copied();
                next_literal += 1;
            }
            if let Some(&(start, end)) = skips.get(next_skip) {
                if start == index {
                    index = end;
                    next_skip += 1;
                    continue;
                }
            }
            match bytes[index] {
                b'(' | b'[' | b'{' => {
                    parent_of.insert(index, stack.last().copied());
                    stack.push(index);
                }
                closer @ (b')' | b']' | b'}') => {
                    let opener = match closer {
                        b')' => b'(',
                        b']' => b'[',
                        _ => b'{',
                    };
                    // An unmatched inner opener is dropped, not paired.
                    if let Some(position) = stack.iter().rposition(|open| bytes[*open] == opener) {
                        let open = stack[position];
                        stack.truncate(position);
                        close_of.insert(open, index);
                        open_of.insert(index, open);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        for parent in literal_parent.iter_mut().skip(next_literal) {
            *parent = stack.last().copied();
        }
        Source {
            text,
            literals,
            by_start,
            by_end,
            literal_parent,
            close_of,
            open_of,
            parent_of,
        }
    }

    /// The span of the class list literal `index` belongs to — the branch value
    /// when it is one branch of a standalone conditional or object — or `None`
    /// when that list cannot be read statically.
    fn class_list_span(&self, index: usize) -> Option<(usize, usize)> {
        let literal = &self.literals[index];
        let mut span = (literal.start, literal.end);
        let mut parent = self.literal_parent[index];
        // The branch value, once the walk has left a conditional or property.
        let mut branch: Option<(usize, usize)> = None;
        // An object handed to a call or array is that helper's data (a `clsx`
        // condition map, a `cva` variant table), not a class list.
        let mut in_object = false;
        loop {
            let before = self.before_kind(span.0);
            let after = self.after_kind(span.1);
            if matches!(before, Before::Unknown) || matches!(after, After::Unknown) {
                return None;
            }
            // A bracket that is not the literal's recorded parent (inside a
            // string, or unmatched) is structure the walk cannot read.
            if let Before::Open(open) = before {
                if parent != Some(open) {
                    return None;
                }
            }
            if let After::Close(close) = after {
                if parent.and_then(|open| self.close_of.get(&open)) != Some(&close) {
                    return None;
                }
            }

            // `+` binds tighter than every other form read here; once the walk
            // has left a branch, joining it to more text is not a list it can
            // evaluate.
            if let Before::Plus(plus) = before {
                if branch.is_some() {
                    return None;
                }
                span.0 = self.operand_before(plus)?;
                continue;
            }
            if let After::Plus(plus) = after {
                if branch.is_some() {
                    return None;
                }
                span.1 = self.operand_after(plus)?;
                continue;
            }

            // A property value: the object around it is the expression.
            if let Before::Colon(colon) = before {
                if self.is_property_colon(colon, parent) {
                    if !matches!(after, After::Comma | After::Close(_)) {
                        return None;
                    }
                    let open = parent?;
                    let close = *self.close_of.get(&open)?;
                    branch.get_or_insert(span);
                    in_object = true;
                    span = (open, close + 1);
                    parent = *self.parent_of.get(&open)?;
                    continue;
                }
            }

            // One branch of a conditional or logical expression.
            let branch_before = matches!(
                before,
                Before::Question | Before::Logical | Before::Colon(_)
            );
            let branch_after = match after {
                After::Logical => true,
                // After anything but a branch operator, `:` ends a key or label.
                After::Colon if matches!(before, Before::Question | Before::Logical) => true,
                After::Colon => return None,
                _ => false,
            };
            if branch_before || branch_after {
                branch.get_or_insert(span);
                let extent = self.conditional_extent(span, parent)?;
                if extent == span {
                    return None;
                }
                span = extent;
                continue;
            }

            // Siblings in a call or array form one class list.
            let group = parent.filter(|open| self.byte(*open) != b'{');
            if matches!(before, Before::Comma) || matches!(after, After::Comma) {
                if let Some(open) = group {
                    if branch.is_some() {
                        return None;
                    }
                    span = (open + 1, *self.close_of.get(&open)?);
                    continue;
                }
            }

            // The expression fills its bracket.
            if let (Before::Open(open), After::Close(close)) = (before, after) {
                match self.byte(open) {
                    // An attribute or markup expression container, or a block.
                    // `${…}` interpolates into a template, which is not read.
                    b'{' => {
                        if self.text[..open].ends_with('$') {
                            return None;
                        }
                        return Some(branch.unwrap_or(span));
                    }
                    b'(' => {
                        span = (self.call_start(open, in_object)?, close + 1);
                    }
                    _ => {
                        if in_object || self.is_index(open) {
                            return None;
                        }
                        span = (open, self.array_end(close)?);
                    }
                }
                parent = *self.parent_of.get(&open)?;
                continue;
            }

            // Boundaries on both sides. A comma there separates declarations or
            // properties; a bracket only bounds a block or container.
            let open_block = |open: usize| parent == Some(open) && self.byte(open) == b'{';
            let bounded_before = match before {
                Before::Boundary | Before::Comma => true,
                Before::Open(open) => open_block(open),
                _ => false,
            };
            let bounded_after = match after {
                After::Boundary | After::Comma => true,
                After::Close(_) => parent.is_some_and(open_block),
                _ => false,
            };
            return (bounded_before && bounded_after).then(|| branch.unwrap_or(span));
        }
    }

    fn before_kind(&self, start: usize) -> Before {
        let Some((at, ch)) = self.before(start) else {
            return Before::Boundary;
        };
        let previous = self.text[..at].chars().next_back();
        match ch {
            '+' => Before::Plus(at),
            ',' => Before::Comma,
            '(' | '[' | '{' => Before::Open(at),
            '?' if previous == Some('?') => Before::Logical,
            '?' => Before::Question,
            '&' if previous == Some('&') => Before::Logical,
            '|' if previous == Some('|') => Before::Logical,
            ':' => Before::Colon(at),
            // `==`, `!=`, `<=`, `+=` and the other compound forms compare or
            // update; a plain `=` assigns or sets an attribute.
            '=' if previous.is_some_and(|ch| "=!<>+-*/%&|^?".contains(ch)) => Before::Unknown,
            '=' | '>' | ';' | ')' | ']' | '}' | '"' | '\'' | '`' => Before::Boundary,
            ch if is_ident_char(ch) => {
                let end = at + ch.len_utf8();
                if NON_VALUE_KEYWORDS.contains(&&self.text[self.word_start(end)..end]) {
                    Before::Unknown
                } else {
                    Before::Boundary
                }
            }
            _ => Before::Unknown,
        }
    }

    fn after_kind(&self, end: usize) -> After {
        let Some((at, ch)) = self.after(end) else {
            return After::Boundary;
        };
        let next = self.text[at + ch.len_utf8()..].chars().next();
        match ch {
            '+' if matches!(next, Some('+' | '=')) => After::Unknown,
            '+' => After::Plus(at),
            ',' => After::Comma,
            ')' | ']' | '}' => After::Close(at),
            '?' if next == Some('?') => After::Logical,
            '|' if next == Some('|') => After::Logical,
            // A framework attribute after a quoted value (` :disabled`,
            // `@click`, `#slot`) starts the next attribute.
            ':' if at > end && next.is_some_and(is_ident_char) => After::Boundary,
            ':' => After::Colon,
            '@' | '#' | '{' | '>' | '<' | '/' | ';' | '"' | '\'' | '`' => After::Boundary,
            ch if is_ident_char(ch) => {
                if matches!(&self.text[at..self.word_end(at)], "in" | "instanceof") {
                    After::Unknown
                } else {
                    After::Boundary
                }
            }
            _ => After::Unknown,
        }
    }

    /// The start of the operand that ends just before the `+` at `plus`; a
    /// variable or number is an operand the list depends on.
    fn operand_before(&self, plus: usize) -> Option<usize> {
        let (at, ch) = self.before(plus)?;
        match ch {
            '"' | '\'' | '`' => Some(self.literals[*self.by_end.get(&(at + 1))?].start),
            ')' => {
                let open = *self.open_of.get(&at)?;
                let start = self.call_start(open, false)?;
                // `[…].join(…)` is one operand.
                if start < open && self.text[start..open].trim_end() == "join" {
                    if let Some((dot, '.')) = self.before(start) {
                        if let Some((close, ']')) = self.before(dot) {
                            let array = *self.open_of.get(&close)?;
                            if !self.is_index(array) {
                                return Some(array);
                            }
                        }
                    }
                }
                Some(start)
            }
            ']' => {
                let open = *self.open_of.get(&at)?;
                (!self.is_index(open)).then_some(open)
            }
            _ => None,
        }
    }

    /// The end of the operand that starts just after the `+` at `plus`.
    fn operand_after(&self, plus: usize) -> Option<usize> {
        let (at, ch) = self.after(plus + 1)?;
        match ch {
            '"' | '\'' | '`' => Some(self.literals[*self.by_start.get(&at)?].end),
            '(' => Some(*self.close_of.get(&at)? + 1),
            '[' => self.array_end(*self.close_of.get(&at)?),
            ch if is_ident_char(ch) => {
                let end = self.word_end(at);
                if EXPRESSION_KEYWORDS.contains(&&self.text[at..end]) {
                    return None;
                }
                let (paren, '(') = self.after(end)? else {
                    return None;
                };
                Some(*self.close_of.get(&paren)? + 1)
            }
            _ => None,
        }
    }

    /// Where the call or group whose parentheses open at `open` starts: the
    /// callee of a class-name call, or the parenthesis of a group. `None` when
    /// the parentheses hold a condition, call an expression the engine cannot
    /// read, or receive an object as a helper's data.
    fn call_start(&self, open: usize, in_object: bool) -> Option<usize> {
        let Some((at, ch)) = self.before(open) else {
            return Some(open);
        };
        if is_ident_char(ch) {
            let end = at + ch.len_utf8();
            let start = self.word_start(end);
            let word = &self.text[start..end];
            if CONDITION_KEYWORDS.contains(&word) {
                return None;
            }
            if EXPRESSION_KEYWORDS.contains(&word) {
                return Some(open);
            }
            return (!in_object).then_some(start);
        }
        match ch {
            ')' | ']' | '"' | '\'' | '`' | '.' => None,
            _ => Some(open),
        }
    }

    /// Whether the `[` at `open` indexes the value before it (`styles["…"]`)
    /// rather than opening an array.
    fn is_index(&self, open: usize) -> bool {
        let Some((at, ch)) = self.before(open) else {
            return false;
        };
        if is_ident_char(ch) {
            let end = at + ch.len_utf8();
            return !EXPRESSION_KEYWORDS.contains(&&self.text[self.word_start(end)..end]);
        }
        matches!(ch, ')' | ']' | '"' | '\'' | '`' | '.')
    }

    /// The end of the array closing at `close`, through a chained `.join(…)`.
    /// Any other member may transform the list.
    fn array_end(&self, close: usize) -> Option<usize> {
        let Some((dot, '.')) = self.after(close + 1) else {
            return Some(close + 1);
        };
        let name = skip_whitespace(self.text, dot + 1);
        let name_end = self.word_end(name);
        if &self.text[name..name_end] != "join" {
            return None;
        }
        let (paren, '(') = self.after(name_end)? else {
            return None;
        };
        Some(*self.close_of.get(&paren)? + 1)
    }

    /// Whether the `:` at `colon` separates an object key from its value.
    fn is_property_colon(&self, colon: usize, parent: Option<usize>) -> bool {
        let Some(open) = parent.filter(|open| self.byte(*open) == b'{') else {
            return false;
        };
        let Some((at, ch)) = self.before(colon) else {
            return false;
        };
        let key_start = match ch {
            '"' | '\'' => match self.by_end.get(&(at + 1)) {
                Some(key) => self.literals[*key].start,
                None => return false,
            },
            ']' => match self.open_of.get(&at) {
                Some(key) => *key,
                None => return false,
            },
            ch if is_ident_char(ch) => self.word_start(at + ch.len_utf8()),
            _ => return false,
        };
        matches!(self.before(key_start), Some((at, ch)) if ch == ',' || at == open)
    }

    /// The conditional or logical expression around a branch value at its own
    /// bracket level, bounded by a comma, semicolon, the enclosing bracket, an
    /// assignment, a property key, or a statement keyword. Brackets and strings
    /// inside it are skipped whole. `None` when the structure around it cannot
    /// be read, the value is appended to something else (`+=`), or no bound is
    /// found within [`EXTENT_STEPS`] tokens (prose quoting a value after a
    /// colon is not a conditional, and must not cost a scan of the file).
    fn conditional_extent(
        &self,
        span: (usize, usize),
        parent: Option<usize>,
    ) -> Option<(usize, usize)> {
        let close = match parent {
            Some(open) => Some(*self.close_of.get(&open)?),
            None => None,
        };
        let mut start = span.0;
        for step in 0.. {
            if step == EXTENT_STEPS || parent.is_some_and(|open| start <= open) {
                return None;
            }
            let Some((at, ch)) = self.before(start) else {
                break;
            };
            let previous = self.text[..at].chars().next_back();
            match ch {
                ',' | ';' => break,
                '(' | '[' | '{' if parent == Some(at) => break,
                '(' | '[' | '{' => return None,
                ')' | ']' | '}' => start = *self.open_of.get(&at)?,
                '"' | '\'' | '`' => start = self.literals[*self.by_end.get(&(at + 1))?].start,
                ':' if self.is_property_colon(at, parent) => break,
                '=' if previous.is_some_and(|ch| "+-*/%&|^?".contains(ch)) => return None,
                '=' if !previous.is_some_and(|ch| "=!<>".contains(ch))
                    && !matches!(self.text[at + 1..].chars().next(), Some('=' | '>')) =>
                {
                    break
                }
                '>' if previous == Some('=') => break,
                ch if is_ident_char(ch) => {
                    let end = at + ch.len_utf8();
                    let word_start = self.word_start(end);
                    if STATEMENT_KEYWORDS.contains(&&self.text[word_start..end]) {
                        break;
                    }
                    start = word_start;
                }
                _ => start = at,
            }
        }
        let mut end = span.1;
        for step in 0.. {
            if step == EXTENT_STEPS || close.is_some_and(|close| end > close) {
                return None;
            }
            let Some((at, ch)) = self.after(end) else {
                break;
            };
            let next = self.text[at + ch.len_utf8()..].chars().next();
            match ch {
                ',' | ';' => break,
                ')' | ']' | '}' if close == Some(at) => break,
                ')' | ']' | '}' => return None,
                '(' | '[' | '{' => end = *self.close_of.get(&at)? + 1,
                '"' | '\'' | '`' => end = self.literals[*self.by_start.get(&at)?].end,
                '=' if !matches!(next, Some('=' | '>'))
                    && !self.text[..at].ends_with(|ch: char| "=!<>+-*/%&|^?".contains(ch)) =>
                {
                    break
                }
                ch if is_ident_char(ch) => {
                    let word_end = self.word_end(at);
                    if STATEMENT_KEYWORDS.contains(&&self.text[at..word_end]) {
                        break;
                    }
                    end = word_end;
                }
                _ => end = at + ch.len_utf8(),
            }
        }
        Some((start, end))
    }

    fn byte(&self, index: usize) -> u8 {
        self.text.as_bytes()[index]
    }

    /// The last non-whitespace character before `index`, with its offset.
    fn before(&self, index: usize) -> Option<(usize, char)> {
        let index = skip_whitespace_back(self.text, index);
        let ch = self.text[..index].chars().next_back()?;
        Some((index - ch.len_utf8(), ch))
    }

    /// The first non-whitespace character at or after `index`, with its offset.
    fn after(&self, index: usize) -> Option<(usize, char)> {
        let index = skip_whitespace(self.text, index);
        let ch = self.text[index..].chars().next()?;
        Some((index, ch))
    }

    /// The start of the identifier that ends at `end`.
    fn word_start(&self, end: usize) -> usize {
        let mut start = end;
        while let Some(ch) = self.text[..start].chars().next_back() {
            if !is_ident_char(ch) {
                break;
            }
            start -= ch.len_utf8();
        }
        start
    }

    /// The end of the identifier that starts at `start`.
    fn word_end(&self, start: usize) -> usize {
        let mut end = start;
        while let Some(ch) = self.text[end..].chars().next() {
            if !is_ident_char(ch) {
                break;
            }
            end += ch.len_utf8();
        }
        end
    }
}

/// Evaluate one class-list expression to `(offset of its first string, class
/// list)`, or `None` when it is not a static class list.
fn evaluate(expression: &str) -> Option<(usize, String)> {
    let (tokens, first) = lex(expression)?;
    Some((first?, Parser::new(&tokens).parse()?))
}

/// A token of an expression: a string literal, an identifier, or one
/// punctuation character.
enum Token<'a> {
    Str { text: &'a str, template: bool },
    Ident(&'a str),
    Punct(char),
}

/// The tokens of one expression, and the offset of its first string. A string
/// ends at the next matching quote, the same rule that found the literals.
fn lex(text: &str) -> Option<(Vec<Token<'_>>, Option<usize>)> {
    let mut tokens = Vec::new();
    let mut first = None;
    let mut index = 0;
    while let Some(ch) = text[index..].chars().next() {
        if ch.is_whitespace() {
            index += ch.len_utf8();
        } else if matches!(ch, '"' | '\'' | '`') {
            let body = index + 1;
            let close = body + text[body..].find(ch)?;
            first.get_or_insert(index);
            tokens.push(Token::Str {
                text: &text[body..close],
                template: ch == '`',
            });
            index = close + 1;
        } else if is_ident_char(ch) {
            let start = index;
            while let Some(next) = text[index..].chars().next() {
                if !is_ident_char(next) {
                    break;
                }
                index += next.len_utf8();
            }
            tokens.push(Token::Ident(&text[start..index]));
        } else {
            tokens.push(Token::Punct(ch));
            index += ch.len_utf8();
        }
    }
    Some((tokens, first))
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
