//! `phoenix-ui-rules`: the XGENT rule pack for the Phoenix UI engine.
//!
//! XGENT's own findings live in this crate and run only on the engines that
//! are handed [`PACK`]: the native `detect` binary (files and URLs) and the
//! hook verbs. The in-page WASM detector does not run a pack. The `cjk-*`
//! rules read a rendered page, so they report on URL scans only.
//!
//! Rule ids are namespaced `xgent/…` so [`impeccable_core::rule_pack::install`]
//! can never collide with a built-in id, and every rule applies the engine's
//! normal waivers (`detector.ignoreRules`, `hooks ignore-rule`, inline
//! directives, `--no-advisory`).

use impeccable_core::browser::{BrowserFinding, ElFinding};
use impeccable_core::findings::Finding;
use impeccable_core::registry::Antipattern;
use impeccable_core::rule_pack::RulePack;

pub mod cjk;
pub mod dom;
pub mod text;

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// `xgent/transition-all` (§5.3).
pub(crate) const TRANSITION_ALL: Antipattern = Antipattern {
    id: "xgent/transition-all",
    category: "quality",
    scopes: None,
    severity: Some("advisory"),
    name: "Transition on all properties",
    description: "transition: all (or Tailwind transition-all) animates every property that changes, including layout and colors nobody meant to animate, and makes later style changes animate by accident. List the properties that should move.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/focus-outline-removed` (§5.3).
pub(crate) const FOCUS_OUTLINE_REMOVED: Antipattern = Antipattern {
    id: "xgent/focus-outline-removed",
    category: "quality",
    scopes: None,
    severity: None,
    name: "Focus outline removed without replacement",
    description: "The focus outline is removed and nothing else marks keyboard focus, so keyboard users lose their place. Keep the outline, or replace it under :focus-visible with a ring, border, or background change that meets 3:1 contrast.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/wrapped-control-label` (§5.3).
pub(crate) const WRAPPED_CONTROL_LABEL: Antipattern = Antipattern {
    id: "xgent/wrapped-control-label",
    category: "quality",
    scopes: Some(&["layout"]),
    severity: None,
    name: "Control label wraps",
    description: "A button, tab, or navigation label breaks onto more than one line at this width. Clickable labels read as one unit; shorten the label, let the row reflow, or move the control.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/control-height-mismatch` (§5.3).
pub(crate) const CONTROL_HEIGHT_MISMATCH: Antipattern = Antipattern {
    id: "xgent/control-height-mismatch",
    category: "quality",
    scopes: Some(&["layout"]),
    severity: Some("advisory"),
    name: "Field and button heights differ",
    description: "A text field and the button beside it render at different heights in the same row, so the row reads untuned. Give fields and their adjacent buttons one shared control height.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/unlabeled-inline-svg` (§5.3).
pub(crate) const UNLABELED_INLINE_SVG: Antipattern = Antipattern {
    id: "xgent/unlabeled-inline-svg",
    category: "quality",
    scopes: None,
    severity: None,
    name: "Unlabeled inline SVG",
    description: "An inline SVG has neither an accessible name nor aria-hidden, so screen readers announce an unnamed graphic. Hide decorative art with aria-hidden=\"true\"; give meaningful graphics role=\"img\" and a name.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/redrawn-window-chrome` (§5.3).
pub(crate) const REDRAWN_WINDOW_CHROME: Antipattern = Antipattern {
    id: "xgent/redrawn-window-chrome",
    category: "slop",
    scopes: None,
    severity: Some("advisory"),
    name: "Re-drawn window chrome",
    description: "Red, amber, and green dots draw a fake window title bar. The visitor's browser already supplies chrome; show the real screenshot in a plain figure or let the content stand without a costume frame.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/emoji-icon` (§5.3).
pub(crate) const EMOJI_ICON: Antipattern = Antipattern {
    id: "xgent/emoji-icon",
    category: "slop",
    scopes: None,
    severity: Some("advisory"),
    name: "Emoji used as icons",
    description: "Emoji stand in for feature or step icons. They render differently on every platform and share no stroke or weight. Use one icon set or authored SVG, or lead with the heading.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/font-family-count` (§5.3).
pub(crate) const FONT_FAMILY_COUNT: Antipattern = Antipattern {
    id: "xgent/font-family-count",
    category: "slop",
    scopes: Some(&["type"]),
    severity: Some("advisory"),
    name: "Too many font families",
    description: "Visible text uses four or more font families. Most interfaces need a body face, an optional display face, and at most one accent; more reads as unplanned. Consolidate onto the type system's roles.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/cjk-tight-leading`: Chinese body text set below 1.5x.
pub(crate) const CJK_TIGHT_LEADING: Antipattern = Antipattern {
    id: "xgent/cjk-tight-leading",
    category: "quality",
    scopes: Some(&["type"]),
    severity: None,
    name: "Tight leading on Chinese text",
    description: "Multi-line Chinese body text is set below 1.5x line height. Han characters fill the em box, so lines set at Latin leading run together. Use 1.6 to 1.75 for body copy.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/cjk-line-length`: Chinese body lines past 40 characters.
pub(crate) const CJK_LINE_LENGTH: Antipattern = Antipattern {
    id: "xgent/cjk-line-length",
    category: "quality",
    scopes: Some(&["type", "layout"]),
    severity: None,
    name: "Chinese lines too long",
    description: "Chinese body lines run past 40 characters. Each Han character is a full em wide, so a measure tuned for 65 to 75 Latin characters is far too long for it. Keep body copy to about 25 to 40 characters per line.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/cjk-orphan`: a last line of one or two Han characters.
pub(crate) const CJK_ORPHAN: Antipattern = Antipattern {
    id: "xgent/cjk-orphan",
    category: "quality",
    scopes: Some(&["type"]),
    severity: Some("advisory"),
    name: "Last line holds one or two characters",
    description: "A Chinese paragraph or heading ends on a line of only one or two characters. Tighten the copy, change the measure, or break the line by meaning so the last line carries a full phrase.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/cjk-split-unit`: a number and its unit on different lines.
pub(crate) const CJK_SPLIT_UNIT: Antipattern = Antipattern {
    id: "xgent/cjk-split-unit",
    category: "quality",
    scopes: Some(&["type"]),
    severity: Some("advisory"),
    name: "Number split from its unit",
    description: "A number and the unit after it wrap onto different lines, so the reader meets the figure without what it counts. Keep them together with a no-wrap span or a non-breaking space.",
    skill_section: None,
    skill_guideline: None,
};

/// `xgent/cjk-font-fallback`: Han drawn by a font the list did not plan for.
pub(crate) const CJK_FONT_FALLBACK: Antipattern = Antipattern {
    id: "xgent/cjk-font-fallback",
    category: "quality",
    scopes: Some(&["type"]),
    severity: Some("advisory"),
    name: "Chinese text falls back to an unplanned font",
    description: "Han characters render in a font the font list did not plan for: a web font failed to load, one passage is split across fonts, or no listed font has the characters and the list has no generic family. List a font that covers Chinese, then a generic family.",
    skill_section: None,
    skill_guideline: None,
};

static ROWS: &[Antipattern] = &[
    TRANSITION_ALL,
    FOCUS_OUTLINE_REMOVED,
    WRAPPED_CONTROL_LABEL,
    CONTROL_HEIGHT_MISMATCH,
    UNLABELED_INLINE_SVG,
    REDRAWN_WINDOW_CHROME,
    EMOJI_ICON,
    FONT_FAMILY_COUNT,
    CJK_TIGHT_LEADING,
    CJK_LINE_LENGTH,
    CJK_ORPHAN,
    CJK_SPLIT_UNIT,
    CJK_FONT_FALLBACK,
];

/// The XGENT rule pack. A unit struct: all state is the static rows.
#[derive(Debug)]
pub struct XgentPack;

/// The one pack instance the binary installs and hands to every engine.
pub static PACK: XgentPack = XgentPack;

impl RulePack for XgentPack {
    fn registry(&self) -> &'static [Antipattern] {
        ROWS
    }

    fn check_text(&self, content: &str, file_path: &str, ext: &str) -> Vec<Finding> {
        text::check(content, file_path, ext)
    }

    fn check_element_dom(
        &self,
        dom: &dyn impeccable_core::browser::Dom,
        el: impeccable_core::browser::ElId,
    ) -> Vec<BrowserFinding> {
        let mut findings = dom::check_element(dom, el);
        findings.extend(cjk::check_element(dom, el));
        findings
    }

    fn replaces_builtin(
        &self,
        dom: &dyn impeccable_core::browser::Dom,
        el: impeccable_core::browser::ElId,
    ) -> &'static [&'static str] {
        cjk::replaces_builtin(dom, el)
    }

    fn check_page_dom(&self, dom: &dyn impeccable_core::browser::Dom) -> Vec<ElFinding> {
        let mut findings = dom::check_page(dom);
        findings.extend(cjk::check_page(dom));
        findings
    }
}
