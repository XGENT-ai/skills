//! The `xgent/` text rules: source-level checks for the text engines.
//!
//! Two rules live here (§5.3): `xgent/transition-all` over CSS declarations and
//! Tailwind class strings, and `xgent/focus-outline-removed` over CSS blocks
//! and Tailwind focus variants. Both are plain line scans over the source file;
//! neither parses a stylesheet, so both err toward the structural signals the
//! plan defines rather than guessing intent.

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

/// `outline`/`outline-style`/`outline-width` set to `none` or `0`.
static OUTLINE_REMOVAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:outline\s*:\s*(?:none|0)\b|outline-(?:style|width)\s*:\s*(?:none|0)\b)")
        .expect("static outline regex")
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
    let mut findings = Vec::new();
    check_transition_all(content, file_path, ext, &mut findings);
    check_focus_outline(content, file_path, &mut findings);
    findings
}

fn check_transition_all(content: &str, file_path: &str, ext: &str, findings: &mut Vec<Finding>) {
    // CSS comments are blanked for CSS-like files so a commented-out
    // `transition: all` is not a hit, while the wildcard selector line
    // `* { transition: all }` stays a hit (comments and selectors must not be
    // conflated by a leading-`*` shortcut). JS/TS files only skip `//` lines.
    let stripped;
    let scanned: &str = if CSS_LIKE_EXTS.contains(&ext) {
        stripped = strip_block_comments(content);
        &stripped
    } else {
        content
    };
    let skip_line_comments = JS_LIKE_EXTS.contains(&ext);
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

fn check_focus_outline(content: &str, file_path: &str, findings: &mut Vec<Finding>) {
    // One finding per line, in line order, whichever branch found the evidence.
    let mut hits: BTreeMap<usize, String> = BTreeMap::new();
    let stripped = strip_block_comments(content);

    // Style blocks: a `:focus` rule that drops the outline and leaves nothing
    // else to mark focus. A file that writes any `:focus-visible` alternative
    // is treated as having replaced the outline already.
    if !stripped.contains(":focus-visible") {
        for capture in CSS_BLOCK_RE.captures_iter(&stripped) {
            let (Some(selector), Some(declarations)) = (capture.get(1), capture.get(2)) else {
                continue;
            };
            let selector = selector.as_str();
            if !selector.replace(":focus-within", "").contains(":focus") {
                continue;
            }
            if !OUTLINE_REMOVAL_RE.is_match(declarations.as_str())
                || has_focus_replacement(declarations.as_str())
            {
                continue;
            }
            let leading = selector.len() - js::trim_start(selector).len();
            let start = capture.get(0).map(|m| m.start()).unwrap_or(0) + leading;
            hits.entry(line_of_offset(&stripped, start))
                .or_insert_with(|| truncate_chars(js::trim(selector), 120));
        }
    }

    // Tailwind class strings: hit and waiver share one variant-chain parser, so
    // `focus:outline-none dark:focus:border-white` and
    // `[&:focus-visible]:outline-2` read as an already-written replacement.
    let quoted: [&Regex; 3] = [&DOUBLE_QUOTED_RE, &SINGLE_QUOTED_RE, &BACKTICK_RE];
    for quote_re in quoted {
        for capture in quote_re.captures_iter(content) {
            let Some(inner) = capture.get(1) else {
                continue;
            };
            if !class_string_removes_focus(inner.as_str()) {
                continue;
            }
            let start = capture.get(0).map(|m| m.start()).unwrap_or(0);
            hits.entry(line_of_offset(content, start))
                .or_insert_with(|| truncate_chars(js::trim(inner.as_str()), 120));
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

/// Blank out `/* … */` comments with spaces, preserving newlines so every
/// offset still maps to its original line.
fn strip_block_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
            out.push(' ');
            index += 2;
            out.push(' ');
            let mut previous_star = false;
            while index < chars.len() {
                let inner = chars[index];
                index += 1;
                if inner == '\n' || inner == '\r' {
                    out.push(inner);
                } else {
                    out.push(' ');
                }
                if previous_star && inner == '/' {
                    break;
                }
                previous_star = inner == '*';
            }
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
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
        let focus = variants
            .iter()
            .any(|variant| variant.to_lowercase().contains("focus"));
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
