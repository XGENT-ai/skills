//! `phoenix-ui-rules`: the XGENT rule pack for the Phoenix UI engine.
//!
//! The built-in rules are a byte-for-byte port of the upstream JS table and
//! never change here (§0.1 C-1). XGENT's own findings live in this crate and
//! run only on the engines that are handed [`PACK`]: the native `detect`
//! binary (files and URLs) and the hook verbs. The in-page WASM detector and
//! the browser extension do not run a pack (ADR-1).
//!
//! Rule ids are namespaced `xgent/…` so [`impeccable_core::rule_pack::install`]
//! can never collide with a built-in id, and every rule applies the engine's
//! normal waivers (`detector.ignoreRules`, `hooks ignore-rule`, inline
//! directives, `--no-advisory`).

use impeccable_core::browser::{BrowserFinding, ElFinding};
use impeccable_core::findings::Finding;
use impeccable_core::registry::Antipattern;
use impeccable_core::rule_pack::RulePack;

pub mod dom;
pub mod text;

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

static ROWS: &[Antipattern] = &[
    TRANSITION_ALL,
    FOCUS_OUTLINE_REMOVED,
    WRAPPED_CONTROL_LABEL,
    CONTROL_HEIGHT_MISMATCH,
    UNLABELED_INLINE_SVG,
    REDRAWN_WINDOW_CHROME,
    EMOJI_ICON,
    FONT_FAMILY_COUNT,
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
        dom::check_element(dom, el)
    }

    fn check_page_dom(&self, dom: &dyn impeccable_core::browser::Dom) -> Vec<ElFinding> {
        dom::check_page(dom)
    }
}
