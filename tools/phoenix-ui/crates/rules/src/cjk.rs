//! The `xgent/cjk-*` page rules: Chinese body copy and headings.
//!
//! The built-in `line-length` and `tight-leading` judge every script by Latin
//! numbers. Han characters are a full em wide and fill the em box, so a
//! Chinese paragraph needs a shorter measure and more leading than those
//! numbers allow. On Han-dominant body text these rules take over from the
//! two built-ins ([`replaces_builtin`]), and three more read what only a
//! rendered page can show: the last line, where a line broke, and which font
//! drew the characters.
//!
//! The rules judge and never measure. Line texts and rendered fonts come from
//! the DOM probe; where it has no answer there is no finding.

use impeccable_core::browser::dom::{
    has_direct_text_longer_than, pf0, HanFontRun, RenderedFont, GENERIC_FONT_FAMILIES,
};
use impeccable_core::browser::driver::is_reserved_node;
use impeccable_core::browser::element_checks::is_rendered_for_browser_rule;
use impeccable_core::browser::{BrowserFinding, Dom, ElFinding, ElId};
use impeccable_core::checks::measures::resolve_length_px;
use impeccable_core::checks::text_rules::QUALITY_TEXT_TAGS;
use impeccable_core::js;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::truncate_chars;
use crate::{CJK_FONT_FALLBACK, CJK_LINE_LENGTH, CJK_ORPHAN, CJK_SPLIT_UNIT, CJK_TIGHT_LEADING};

static HAN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\p{Han}").expect("static han regex"));
static PUNCT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\p{P}").expect("static punctuation regex"));
/// A number at the end of a line: ASCII or full-width digits, with `.` or `,`
/// between groups.
static NUMBER_TAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[0-9０-９]+(?:[.,][0-9０-９]+)*$").expect("static number regex"));

/// Units that follow a number, longest first so `工作日` is not read as `工`.
const UNIT_WORDS: &[&str] = &[
    "工作日",
    "小时",
    "公里",
    "公斤",
    "毫秒",
    "毫米",
    "厘米",
    "像素",
    "美元",
    "字节",
    "英寸",
    "星期",
    "季度",
    "平方",
    "立方",
];
const UNIT_CHARS: &str = "年月日号时分秒天周岁个位名人次项条件张页行列元角块万亿千百倍折成度点米克吨升斤层期届版代款台部辆套组份步级章节篇种类家只本册场轮双对瓶杯盒箱包首颗%％‰℃°";

const BUILTINS_REPLACED: &[&str] = &["line-length", "tight-leading"];

/// Whether Han characters are more than half of what the text shows, with
/// whitespace and punctuation left out of the count.
fn han_dominant(text: &str) -> bool {
    let hidden =
        text.chars().filter(|c| c.is_whitespace()).count() + PUNCT_RE.find_iter(text).count();
    let visible = text.chars().count().saturating_sub(hidden);
    visible > 0 && HAN_RE.find_iter(text).count() * 2 > visible
}

#[derive(Clone, Copy, PartialEq)]
enum Passage {
    Body,
    Heading,
}

/// The text these rules read: rendered, Han-dominant body copy (the tags the
/// built-in `line-length` reads) and headings.
fn passage(dom: &dyn Dom, el: ElId) -> Option<Passage> {
    let tag = js::to_lower_case(&dom.tag_name(el));
    let kind = if QUALITY_TEXT_TAGS.contains(&tag.as_str()) {
        if !has_direct_text_longer_than(dom, el, 10) {
            return None;
        }
        Passage::Body
    } else if matches!(tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
        if !dom
            .direct_text_nodes(el)
            .iter()
            .any(|node| !js::trim(node).is_empty())
        {
            return None;
        }
        Passage::Heading
    } else {
        return None;
    };
    (han_dominant(&dom.text_content(el)) && is_rendered_for_browser_rule(dom, el)).then_some(kind)
}

/// The built-in rules the Chinese rules stand in for on this element. It does
/// not depend on line count or on whether a Chinese rule is switched off: Latin
/// thresholds are the wrong measure for this text either way.
pub(crate) fn replaces_builtin(dom: &dyn Dom, el: ElId) -> &'static [&'static str] {
    if passage(dom, el) == Some(Passage::Body) {
        BUILTINS_REPLACED
    } else {
        &[]
    }
}

pub(crate) fn check_element(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    let Some(kind) = passage(dom, el) else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    if kind == Passage::Body {
        findings.extend(tight_leading(dom, el));
        findings.extend(line_length(dom, el));
    }
    let lines: Vec<String> = dom
        .text_line_texts(el)
        .unwrap_or_default()
        .iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() >= 2 {
        findings.extend(orphan(&lines));
        findings.extend(split_unit(&lines));
    }
    findings
}

pub(crate) fn check_page(dom: &dyn Dom) -> Vec<ElFinding> {
    font_fallback(dom)
}

// ─── cjk-tight-leading ─────────────────────────────────────────────────────

fn tight_leading(dom: &dyn Dom, el: ElId) -> Option<BrowserFinding> {
    if dom.text_line_rects(el)?.len() < 2 {
        return None;
    }
    let font_size = pf0(&dom.style(el, "fontSize"));
    if font_size <= 0.0 {
        return None;
    }
    // `normal` resolves to nothing: its value depends on the font that
    // happens to render, so it is not a threshold.
    let line_height = resolve_length_px(Some(&dom.style(el, "lineHeight")), font_size)?;
    let ratio = (line_height / font_size * 100.0).round() / 100.0;
    if !(ratio > 0.0 && ratio < 1.5) {
        return None;
    }
    Some(BrowserFinding::new(
        CJK_TIGHT_LEADING.id,
        format!("line-height {ratio:.2}x on Chinese body text (need >=1.5)"),
    ))
}

// ─── cjk-line-length ───────────────────────────────────────────────────────

fn line_length(dom: &dyn Dom, el: ElId) -> Option<BrowserFinding> {
    let rows = dom.text_line_rects(el)?;
    if rows.len() < 2 {
        return None;
    }
    // One Han character advances by the font size plus tracking, so a line's
    // width over that is how many Han characters wide it is, whatever mix of
    // scripts it holds and whichever font rendered it.
    let advance = pf0(&dom.style(el, "fontSize")) + pf0(&dom.style(el, "letterSpacing"));
    if advance <= 0.0 {
        return None;
    }
    let widths: Vec<f64> = rows
        .iter()
        .map(|row| (row.width / advance).round())
        .collect();
    let long = widths.iter().filter(|chars| **chars > 40.0).count();
    if long < 2 {
        return None;
    }
    let longest = widths.iter().copied().fold(0.0, f64::max);
    Some(BrowserFinding::new(
        CJK_LINE_LENGTH.id,
        format!(
            "~{longest:.0} Han-width chars on {long} of {} rendered lines (aim for <=40)",
            widths.len()
        ),
    ))
}

// ─── cjk-orphan ────────────────────────────────────────────────────────────

fn orphan(lines: &[String]) -> Option<BrowserFinding> {
    let last = lines.last()?;
    let mut buffer = [0u8; 4];
    let left: Vec<char> = last
        .chars()
        .filter(|c| !c.is_whitespace() && !PUNCT_RE.is_match(c.encode_utf8(&mut buffer)))
        .collect();
    if left.is_empty() || left.len() > 2 {
        return None;
    }
    if !left
        .iter()
        .all(|c| HAN_RE.is_match(c.encode_utf8(&mut buffer)))
    {
        return None;
    }
    Some(BrowserFinding::new(
        CJK_ORPHAN.id,
        format!("last line holds only \"{}\"", truncate_chars(last, 10)),
    ))
}

// ─── cjk-split-unit ────────────────────────────────────────────────────────

fn split_unit(lines: &[String]) -> Option<BrowserFinding> {
    lines.windows(2).find_map(|pair| {
        let number = NUMBER_TAIL_RE.find(&pair[0])?.as_str();
        let unit = leading_unit(&pair[1])?;
        Some(BrowserFinding::new(
            CJK_SPLIT_UNIT.id,
            format!("number \"{number}\" and its unit \"{unit}\" sit on different lines"),
        ))
    })
}

fn leading_unit(line: &str) -> Option<&str> {
    if let Some(word) = UNIT_WORDS.iter().find(|word| line.starts_with(**word)) {
        return Some(word);
    }
    let first = line.chars().next()?;
    UNIT_CHARS
        .contains(first)
        .then(|| &line[..first.len_utf8()])
}

// ─── cjk-font-fallback ─────────────────────────────────────────────────────

/// Each run is judged on its own; identical outcomes on one font stack are
/// then reported once, on the first element, so a site-wide broken font is
/// one finding and not one per paragraph.
fn font_fallback(dom: &dyn Dom) -> Vec<ElFinding> {
    struct Hit {
        stack: String,
        case: u8,
        el: ElId,
        detail: String,
        more: usize,
    }
    let mut hits: Vec<Hit> = Vec::new();
    for el in dom.query_all(None, "*").unwrap_or_default() {
        let Some(facts) = dom.han_font_facts(el) else {
            continue;
        };
        // The driver keeps reserved nodes from element rules only; this page
        // rule walks the document on its own.
        if is_reserved_node(dom, el) || passage(dom, el).is_none() {
            continue;
        }
        let mut seen: Vec<(String, u8)> = Vec::new();
        for run in &facts.runs {
            let Some((case, detail)) = unexpected_fallback(run) else {
                continue;
            };
            let stack = run.declared_families.join(", ").to_lowercase();
            if seen.contains(&(stack.clone(), case)) {
                continue;
            }
            seen.push((stack.clone(), case));
            match hits
                .iter_mut()
                .find(|hit| hit.stack == stack && hit.case == case)
            {
                Some(hit) => hit.more += 1,
                None => hits.push(Hit {
                    stack,
                    case,
                    el,
                    detail,
                    more: 0,
                }),
            }
        }
    }
    hits.into_iter()
        .map(|hit| {
            let detail = match hit.more {
                0 => hit.detail,
                1 => format!("{} (+1 more element with this font stack)", hit.detail),
                more => format!(
                    "{} (+{more} more elements with this font stack)",
                    hit.detail
                ),
            };
            ElFinding {
                el: Some(hit.el),
                finding: BrowserFinding::new(CJK_FONT_FALLBACK.id, detail),
            }
        })
        .collect()
}

/// The three ways Han ends up in a font the list did not plan for. Everything
/// else is the list working: a generic family resolving, a named font
/// rendering all the Han, or a Latin face in front passing Han on to the next
/// family or the generic.
fn unexpected_fallback(run: &HanFontRun) -> Option<(u8, String)> {
    if run.rendered.is_empty() || run.declared_families.is_empty() {
        return None;
    }
    let generic = |name: &String| GENERIC_FONT_FAMILIES.contains(&name.to_lowercase().as_str());
    // A web font's platform name can differ from the CSS family, so a custom
    // font counts as one the list named.
    let listed = |font: &RenderedFont| {
        font.custom
            || run
                .declared_families
                .iter()
                .any(|name| !generic(name) && name.eq_ignore_ascii_case(&font.family))
    };
    let names = run
        .rendered
        .iter()
        .map(|font| font.family.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(failed) = run
        .failed_web_fonts
        .iter()
        .find(|failed| !failed.earlier_declared_rendered)
    {
        return Some((
            1,
            format!(
                "web font \"{}\" failed to load; Han text renders in {names}",
                failed.family
            ),
        ));
    }
    if run.rendered.len() >= 2 && run.rendered.iter().any(listed) {
        let fonts = run
            .rendered
            .iter()
            .map(|font| {
                let plural = if font.glyphs == 1 { "" } else { "s" };
                format!("{} ({} glyph{plural})", font.family, font.glyphs)
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Some((
            2,
            format!("Han text renders in {} fonts: {fonts}", run.rendered.len()),
        ));
    }
    if !run.rendered.iter().any(listed) && !run.declared_families.iter().any(generic) {
        return Some((
            3,
            format!(
                "no listed font renders Han and the list has no generic family; Han text renders in {names}"
            ),
        ));
    }
    None
}
