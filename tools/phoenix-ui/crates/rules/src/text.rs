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

use crate::truncate_chars;
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

/// A one-line string literal: double-quoted, single-quoted, or a template.
static STRING_LITERAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""([^"\n]*)"|'([^'\n]*)'|`([^`\n]*)`"#).expect("static string literal regex")
});

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

    // Tailwind class strings: every string literal on a line is read as one
    // class list, so a replacement written beside the removal on that line
    // (`focus:outline-none dark:focus:border-white`, `cn("focus:outline-none",
    // "[&:focus-visible]:outline-2")`) waives it.
    for (index, line) in blanked.split('\n').enumerate() {
        let classes = STRING_LITERAL_RE
            .captures_iter(line)
            .filter_map(|c| c.get(1).or(c.get(2)).or(c.get(3)))
            .flat_map(|m| m.as_str().split([' ', '\t', '\'', '"', '`']))
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if class_string_removes_focus(&classes) {
            hits.entry(index + 1)
                .or_insert_with(|| truncate_chars(&classes, 120));
        }
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
