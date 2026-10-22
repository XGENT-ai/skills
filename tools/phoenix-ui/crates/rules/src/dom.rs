//! The `xgent/` DOM rules: four element checks and two page checks over the
//! browser/snapshot probe (§5.3).
//!
//! Every rule reads only what the snapshot contract already captures
//! (computed styles from the whitelist, geometry, attributes, text lines);
//! none adds a snapshot property (C-7).

use impeccable_core::browser::dom::{ancestors_inclusive, Rect};
use impeccable_core::browser::element_checks::{is_rendered_for_browser_rule, parse_rgb_or_any};
use impeccable_core::browser::{BrowserFinding, Dom, ElFinding, ElId};
use impeccable_core::color::{get_hue, has_chroma};
use impeccable_core::constants::CSS_GENERIC_FONTS;
use impeccable_core::js;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    CONTROL_HEIGHT_MISMATCH, EMOJI_ICON, FONT_FAMILY_COUNT, REDRAWN_WINDOW_CHROME,
    UNLABELED_INLINE_SVG, WRAPPED_CONTROL_LABEL,
};

/// The input types that render a text-editable field.
const FIELD_INPUT_TYPES: &[&str] = &[
    "text",
    "search",
    "email",
    "password",
    "number",
    "tel",
    "url",
    "date",
    "datetime-local",
    "month",
    "time",
    "week",
];

/// Button-ish input types beside a field.
const BUTTON_INPUT_TYPES: &[&str] = &["submit", "button", "reset"];

/// Tags whose text is code, not interface copy (font-family rule).
const FONT_SKIP_TAGS: &[&str] = &[
    "code", "pre", "kbd", "samp", "textarea", "input", "select", "svg",
];

/// Faces that are aliases of one another on every platform: one family.
const SYSTEM_FONT_ALIASES: &[&str] = &[
    "system-ui",
    "-apple-system",
    "blinkmacsystemfont",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
];

/// A trimmed emoji presentation sequence, and nothing else.
static EMOJI_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\p{Emoji_Presentation}(?:\x{FE0F}|\x{200D}\p{Extended_Pictographic}|[\x{1F3FB}-\x{1F3FF}])*$")
        .expect("static emoji regex")
});

/// The element hook: one call per element in the driver loop.
pub(crate) fn check_element(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    let mut findings = Vec::new();
    findings.extend(wrapped_control_label(dom, el));
    findings.extend(control_height_mismatch(dom, el));
    findings.extend(unlabeled_inline_svg(dom, el));
    findings.extend(redrawn_window_chrome(dom, el));
    findings
}

/// The page hook: runs once after the built-in page checks.
pub(crate) fn check_page(dom: &dyn Dom) -> Vec<ElFinding> {
    let mut findings = Vec::new();
    findings.extend(emoji_icon(dom));
    findings.extend(font_family_count(dom));
    findings
}

// ─── wrapped-control-label ─────────────────────────────────────────────────

fn wrapped_control_label(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    if !is_wrapped_label_candidate(dom, el) {
        return Vec::new();
    }
    if !is_rendered_for_browser_rule(dom, el) {
        return Vec::new();
    }
    let text = js::trim(&dom.text_content(el)).to_string();
    let chars = text.chars().count();
    if chars == 0 || chars > 40 {
        return Vec::new();
    }
    if subtree_has_br(dom, el) {
        return Vec::new();
    }
    // Only a leaf that owns its text can say its own label wrapped: a button
    // that stacks a title and a subtitle is two one-line leaves, not a
    // wrapped label (F-08).
    for descendant in descendants_inclusive(dom, el) {
        if !has_direct_text(dom, descendant) {
            continue;
        }
        if dom
            .children(descendant)
            .iter()
            .any(|child| !js::trim(&dom.text_content(*child)).is_empty())
        {
            continue;
        }
        let Some(rows) = dom.text_line_rects(descendant) else {
            continue;
        };
        if rows.len() >= 2 {
            let snippet = format!(
                "<{}> label wraps to {} lines: \"{}\"",
                tag_lower(dom, el),
                rows.len(),
                truncate_chars(&text, 40)
            );
            return vec![BrowserFinding::new(WRAPPED_CONTROL_LABEL.id, snippet)];
        }
    }
    Vec::new()
}

fn is_wrapped_label_candidate(dom: &dyn Dom, el: ElId) -> bool {
    let tag = tag_lower(dom, el);
    let role = role_of(dom, el);
    if tag == "button" || matches!(role.as_str(), "button" | "tab" | "menuitem") {
        return true;
    }
    if tag == "a" {
        return ancestors_inclusive(dom, el).iter().any(|ancestor| {
            let ancestor_tag = tag_lower(dom, *ancestor);
            let ancestor_role = role_of(dom, *ancestor);
            ancestor_tag == "nav"
                || matches!(ancestor_role.as_str(), "navigation" | "tablist" | "menubar")
        });
    }
    false
}

fn subtree_has_br(dom: &dyn Dom, el: ElId) -> bool {
    descendants_inclusive(dom, el)
        .iter()
        .any(|descendant| tag_lower(dom, *descendant) == "br")
}

// ─── control-height-mismatch ───────────────────────────────────────────────

fn control_height_mismatch(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    if !is_field_candidate(dom, el) || !is_rendered_for_browser_rule(dom, el) {
        return Vec::new();
    }
    let field = dom.rect(el);
    if !control_height(field.height) {
        return Vec::new();
    }
    let Some(parent) = dom.parent(el) else {
        return Vec::new();
    };
    for sibling in dom.children(parent) {
        if sibling == el || !is_button_partner(dom, sibling) {
            continue;
        }
        if !is_rendered_for_browser_rule(dom, sibling) {
            continue;
        }
        let partner = dom.rect(sibling);
        if !control_height(partner.height) || !same_row(field, partner) {
            continue;
        }
        if (field.height - partner.height).abs() > 2.0 {
            let snippet = format!(
                "{} {}px beside {} {}px",
                tag_lower(dom, el),
                field.height.round() as i64,
                tag_lower(dom, sibling),
                partner.height.round() as i64
            );
            return vec![BrowserFinding::new(CONTROL_HEIGHT_MISMATCH.id, snippet)];
        }
        return Vec::new();
    }
    Vec::new()
}

fn is_field_candidate(dom: &dyn Dom, el: ElId) -> bool {
    let tag = tag_lower(dom, el);
    if tag == "select" {
        return true;
    }
    if tag != "input" {
        return false;
    }
    let input_type = attribute(dom, el, "type").to_lowercase();
    input_type.is_empty() || FIELD_INPUT_TYPES.contains(&input_type.as_str())
}

fn is_button_partner(dom: &dyn Dom, el: ElId) -> bool {
    let tag = tag_lower(dom, el);
    if tag == "button" {
        return true;
    }
    if tag == "input" {
        let input_type = attribute(dom, el, "type").to_lowercase();
        if BUTTON_INPUT_TYPES.contains(&input_type.as_str()) {
            return true;
        }
    }
    role_of(dom, el) == "button"
}

fn control_height(height: f64) -> bool {
    (20.0..=80.0).contains(&height)
}

fn same_row(a: Rect, b: Rect) -> bool {
    let center_a = a.top + a.height / 2.0;
    let center_b = b.top + b.height / 2.0;
    let tolerance = 4.0f64.max(0.25 * a.height.min(b.height));
    if (center_a - center_b).abs() > tolerance {
        return false;
    }
    let gap = (b.left - a.right).max(a.left - b.right).max(0.0);
    gap <= 24.0
}

// ─── unlabeled-inline-svg ──────────────────────────────────────────────────

fn unlabeled_inline_svg(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    if tag_lower(dom, el) != "svg" {
        return Vec::new();
    }
    let ancestors = ancestors_inclusive(dom, el);
    if ancestors
        .iter()
        .skip(1)
        .any(|ancestor| tag_lower(dom, *ancestor) == "svg")
    {
        return Vec::new();
    }
    let rect = dom.rect(el);
    if rect.width < 16.0 || rect.height < 16.0 {
        return Vec::new();
    }
    if !is_rendered_for_browser_rule(dom, el) {
        return Vec::new();
    }
    let role = role_of(dom, el);
    if matches!(role.as_str(), "presentation" | "none") {
        return Vec::new();
    }
    if !attribute(dom, el, "aria-label").is_empty()
        || !attribute(dom, el, "aria-labelledby").is_empty()
    {
        return Vec::new();
    }
    if dom.children(el).iter().any(|child| {
        tag_lower(dom, *child) == "title" && !js::trim(&dom.text_content(*child)).is_empty()
    }) {
        return Vec::new();
    }
    for ancestor in ancestors.iter().skip(1) {
        let ancestor_tag = tag_lower(dom, *ancestor);
        if matches!(ancestor_tag.as_str(), "a" | "button" | "label" | "summary") {
            return Vec::new();
        }
        let ancestor_role = role_of(dom, *ancestor);
        if matches!(ancestor_role.as_str(), "button" | "link") {
            return Vec::new();
        }
        if !attribute(dom, *ancestor, "aria-label").is_empty()
            || !attribute(dom, *ancestor, "aria-labelledby").is_empty()
        {
            return Vec::new();
        }
    }
    let snippet = format!(
        "<svg {}×{}> has no accessible name and is not aria-hidden",
        rect.width.round() as i64,
        rect.height.round() as i64
    );
    vec![BrowserFinding::new(UNLABELED_INLINE_SVG.id, snippet)]
}

// ─── redrawn-window-chrome ─────────────────────────────────────────────────

fn redrawn_window_chrome(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    let children: Vec<ElId> = dom.children(el).into_iter().take(6).collect();
    for window in children.windows(3) {
        if window_dots(dom, window) {
            return vec![BrowserFinding::new(
                REDRAWN_WINDOW_CHROME.id,
                format!("three window-control dots in <{}>", tag_lower(dom, el)),
            )];
        }
    }
    Vec::new()
}

/// Three consecutive dots: red, amber, green, same size, same row, close
/// together. The hue order is what makes it a window frame instead of three
/// decorative dots.
fn window_dots(dom: &dyn Dom, dots: &[ElId]) -> bool {
    let mut rects = Vec::with_capacity(3);
    let mut hues = Vec::with_capacity(3);
    for dot in dots {
        if !is_rendered_for_browser_rule(dom, *dot) {
            return false;
        }
        let rect = dom.rect(*dot);
        if rect.width < 6.0
            || rect.width > 16.0
            || rect.height < 6.0
            || rect.height > 16.0
            || (rect.width - rect.height).abs() > 1.0
        {
            return false;
        }
        if !border_radius_covers_half(dom, *dot, rect.width) {
            return false;
        }
        let Some(color) = parse_rgb_or_any(&dom.style(*dot, "backgroundColor")) else {
            return false;
        };
        if color.a.unwrap_or(1.0) < 0.5 || !has_chroma(Some(&color), None) {
            return false;
        }
        let hue = normalize_hue(get_hue(Some(&color)));
        rects.push(rect);
        hues.push(hue);
    }
    let expected = [red_hue(hues[0]), amber_hue(hues[1]), green_hue(hues[2])];
    if expected.contains(&false) {
        return false;
    }
    let centers: Vec<f64> = rects
        .iter()
        .map(|rect| rect.top + rect.height / 2.0)
        .collect();
    if centers
        .iter()
        .any(|center| (center - centers[0]).abs() > 2.0)
    {
        return false;
    }
    rects.windows(2).all(|pair| {
        let gap = pair[1].left - pair[0].right;
        (0.0..=12.0).contains(&gap)
    })
}

fn border_radius_covers_half(dom: &dyn Dom, el: ElId, width: f64) -> bool {
    let radius = dom.style(el, "borderRadius");
    let first = radius.split_whitespace().next().unwrap_or("");
    if let Some(percent) = first.strip_suffix('%') {
        return js::parse_float(percent) >= 45.0;
    }
    let px = js::parse_float(first);
    !px.is_nan() && px >= 0.45 * width
}

fn normalize_hue(hue: f64) -> f64 {
    if hue >= 360.0 {
        hue - 360.0
    } else {
        hue
    }
}

fn red_hue(hue: f64) -> bool {
    (0.0..=20.0).contains(&hue) || (340.0..360.0).contains(&hue)
}

fn amber_hue(hue: f64) -> bool {
    (25.0..=60.0).contains(&hue)
}

fn green_hue(hue: f64) -> bool {
    (85.0..=160.0).contains(&hue)
}

// ─── emoji-icon ────────────────────────────────────────────────────────────

fn emoji_icon(dom: &dyn Dom) -> Vec<ElFinding> {
    let mut candidates: Vec<(ElId, String, String)> = Vec::new();
    for el in all_elements(dom) {
        if !is_rendered_for_browser_rule(dom, el) {
            continue;
        }
        let text = js::trim(&dom.text_content(el)).to_string();
        if text.is_empty() || !EMOJI_RE.is_match(&text) {
            continue;
        }
        let font_size = style_px(dom, el, "fontSize");
        if font_size < 20.0 {
            continue;
        }
        let heading = heading_after(dom, el);
        let Some(heading) = heading else {
            continue;
        };
        candidates.push((el, text, heading));
    }
    if candidates.len() < 2 {
        return Vec::new();
    }
    candidates
        .into_iter()
        .map(|(el, emoji, heading)| ElFinding {
            el: Some(el),
            finding: BrowserFinding::new(
                EMOJI_ICON.id,
                format!("emoji \"{emoji}\" used as an icon before <{heading}>"),
            ),
        })
        .collect()
}

/// The heading the emoji introduces: its next element sibling, or the next
/// sibling of its parent when the emoji is the parent's only element child.
fn heading_after(dom: &dyn Dom, el: ElId) -> Option<String> {
    if let Some(next) = dom.next_element_sibling(el) {
        if is_heading(dom, next) {
            return Some(tag_lower(dom, next));
        }
    }
    let parent = dom.parent(el)?;
    let siblings = dom.children(parent);
    if siblings.len() == 1 && siblings[0] == el {
        if let Some(next) = dom.next_element_sibling(parent) {
            if is_heading(dom, next) {
                return Some(tag_lower(dom, next));
            }
        }
    }
    None
}

fn is_heading(dom: &dyn Dom, el: ElId) -> bool {
    matches!(
        tag_lower(dom, el).as_str(),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    )
}

// ─── font-family-count ─────────────────────────────────────────────────────

fn font_family_count(dom: &dyn Dom) -> Vec<ElFinding> {
    // First appearance order, so the snippet reads as the page's own order.
    let mut families: Vec<(String, String)> = Vec::new();
    for el in all_elements(dom) {
        if !is_rendered_for_browser_rule(dom, el) {
            continue;
        }
        if !dom
            .direct_text_nodes(el)
            .iter()
            .any(|node| !js::trim(node).is_empty())
        {
            continue;
        }
        if ancestors_inclusive(dom, el)
            .iter()
            .any(|ancestor| FONT_SKIP_TAGS.contains(&tag_lower(dom, *ancestor).as_str()))
        {
            continue;
        }
        let raw = dom.style(el, "fontFamily");
        let first = raw.split(',').next().unwrap_or("");
        let name = family_name(first);
        if name.is_empty()
            || CSS_GENERIC_FONTS.contains(&name.as_str())
            || name.contains("icon")
            || name.contains("symbol")
            || name.contains("awesome")
        {
            continue;
        }
        let key = if SYSTEM_FONT_ALIASES.contains(&name.as_str()) {
            "system-ui".to_string()
        } else {
            name.clone()
        };
        if !families.iter().any(|(existing, _)| *existing == key) {
            families.push((key, format!("{name} ({})", tag_lower(dom, el))));
        }
    }
    if families.len() < 4 {
        return Vec::new();
    }
    let listed: Vec<&str> = families.iter().map(|(_, text)| text.as_str()).collect();
    vec![ElFinding {
        el: None,
        finding: BrowserFinding::new(
            FONT_FAMILY_COUNT.id,
            format!("{} families: {}", families.len(), listed.join(", ")),
        ),
    }]
}

fn family_name(value: &str) -> String {
    let trimmed = js::trim(value);
    let unquoted = trimmed.trim_matches(|ch| ch == '"' || ch == '\'');
    js::to_lower_case(js::trim(unquoted))
}

// ─── shared helpers ────────────────────────────────────────────────────────

fn all_elements(dom: &dyn Dom) -> Vec<ElId> {
    dom.query_all(None, "*").unwrap_or_default()
}

fn descendants_inclusive(dom: &dyn Dom, el: ElId) -> Vec<ElId> {
    fn walk(dom: &dyn Dom, el: ElId, out: &mut Vec<ElId>) {
        out.push(el);
        for child in dom.children(el) {
            walk(dom, child, out);
        }
    }
    let mut out = Vec::new();
    walk(dom, el, &mut out);
    out
}

fn has_direct_text(dom: &dyn Dom, el: ElId) -> bool {
    dom.direct_text_nodes(el)
        .iter()
        .any(|node| !js::trim(node).is_empty())
}

fn tag_lower(dom: &dyn Dom, el: ElId) -> String {
    js::to_lower_case(&dom.tag_name(el))
}

fn attribute(dom: &dyn Dom, el: ElId, name: &str) -> String {
    dom.attr(el, name).unwrap_or_default()
}

fn role_of(dom: &dyn Dom, el: ElId) -> String {
    js::to_lower_case(js::trim(&attribute(dom, el, "role"))).to_string()
}

fn style_px(dom: &dyn Dom, el: ElId, property: &str) -> f64 {
    let value = js::parse_float(&dom.style(el, property));
    if value.is_nan() {
        0.0
    } else {
        value
    }
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
