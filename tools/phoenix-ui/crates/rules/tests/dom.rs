//! V-3: the six `xgent/` DOM rules through the public rule-pack hooks.
//!
//! Each rule gets a FakeDom positive that pins the snippet, the counterexamples
//! the plan calls out, and its boundary. Malformed geometry (zero sizes, no
//! parent) must never panic: the release binary aborts on panic.

use impeccable_core::browser::fake_dom::FakeDom;
use impeccable_core::browser::{Dom, ElFinding, ElId};
use impeccable_core::rule_pack::RulePack;
use phoenix_ui_rules::PACK;

fn element_type_ids(dom: &FakeDom, el: ElId) -> Vec<String> {
    PACK.check_element_dom(dom, el)
        .into_iter()
        .map(|finding| finding.type_)
        .collect()
}

fn element_details(dom: &FakeDom, el: ElId) -> Vec<String> {
    PACK.check_element_dom(dom, el)
        .into_iter()
        .map(|finding| finding.detail)
        .collect()
}

fn page_findings(dom: &FakeDom) -> Vec<ElFinding> {
    PACK.check_page_dom(dom)
}

fn all_elements(dom: &FakeDom) -> Vec<ElId> {
    (1..dom.els.len() as ElId).collect()
}

fn ids(findings: &[ElFinding]) -> Vec<(&str, Option<ElId>, &str)> {
    findings
        .iter()
        .map(|finding| {
            (
                finding.finding.type_.as_str(),
                finding.el,
                finding.finding.detail.as_str(),
            )
        })
        .collect()
}

fn page() -> (FakeDom, ElId) {
    let mut dom = FakeDom::new();
    let (_html, body) = dom.with_page();
    (dom, body)
}

// ─── wrapped-control-label ─────────────────────────────────────────────────

#[test]
fn wrapped_control_label_reports_a_wrapped_button_and_a_nav_link() {
    let (mut dom, body) = page();
    let button = dom.add(Some(body), "button");
    dom.add_text(button, "Continue to checkout");
    dom.set_text_lines(button, &[(0.0, 0.0, 140.0, 20.0), (0.0, 20.0, 60.0, 20.0)]);
    let nav = dom.add(Some(body), "nav");
    let link = dom.add(Some(nav), "a");
    dom.add_text(link, "Guides");
    dom.set_text_lines(link, &[(0.0, 0.0, 40.0, 18.0), (0.0, 18.0, 30.0, 18.0)]);
    let role_button = dom.add(Some(body), "div");
    dom.set_attr(role_button, "role", "button");
    dom.add_text(role_button, "Open menu");
    dom.set_text_lines(
        role_button,
        &[(0.0, 0.0, 50.0, 20.0), (0.0, 20.0, 40.0, 20.0)],
    );

    for el in [button, link, role_button] {
        assert_eq!(
            element_type_ids(&dom, el),
            vec!["xgent/wrapped-control-label"],
            "{el}"
        );
    }
    assert_eq!(
        element_details(&dom, button),
        vec!["<button> label wraps to 2 lines: \"Continue to checkout\""]
    );
    assert_eq!(
        element_details(&dom, link),
        vec!["<a> label wraps to 2 lines: \"Guides\""]
    );
}

#[test]
fn wrapped_control_label_ignores_intentional_two_line_content() {
    let (mut dom, body) = page();
    // A stacked title/subtitle button: two one-line leaves, not a wrapped label.
    let stacked = dom.add(Some(body), "button");
    let title = dom.add(Some(stacked), "span");
    dom.add_text(title, "Pro");
    dom.set_text_lines(title, &[(0.0, 0.0, 30.0, 20.0)]);
    let price = dom.add(Some(stacked), "span");
    dom.add_text(price, "$9/mo");
    dom.set_text_lines(price, &[(0.0, 20.0, 44.0, 18.0)]);
    assert!(element_type_ids(&dom, stacked).is_empty());

    // A <br> is an authored line break, not a wrap.
    let broken = dom.add(Some(body), "button");
    dom.add_text(broken, "Line one");
    dom.add(Some(broken), "br");
    dom.add_text(broken, "Line two");
    dom.set_text_lines(broken, &[(0.0, 0.0, 40.0, 20.0)]);
    assert!(element_type_ids(&dom, broken).is_empty());

    // A DOM that cannot report the lines stands down.
    let unmeasured = dom.add(Some(body), "button");
    dom.add_text(unmeasured, "Unmeasured label");
    assert!(element_type_ids(&dom, unmeasured).is_empty());

    // A non-control element is not a control label.
    let paragraph = dom.add(Some(body), "p");
    dom.add_text(paragraph, "Body copy");
    dom.set_text_lines(
        paragraph,
        &[(0.0, 0.0, 120.0, 20.0), (0.0, 20.0, 80.0, 20.0)],
    );
    assert!(element_type_ids(&dom, paragraph).is_empty());
}

#[test]
fn wrapped_control_label_boundary_is_forty_characters() {
    let (mut dom, body) = page();
    let forty: String = "-".repeat(40);
    let at_limit = dom.add(Some(body), "button");
    dom.add_text(at_limit, &forty);
    dom.set_text_lines(
        at_limit,
        &[(0.0, 0.0, 200.0, 20.0), (0.0, 20.0, 40.0, 20.0)],
    );
    assert_eq!(
        element_type_ids(&dom, at_limit),
        vec!["xgent/wrapped-control-label"]
    );

    let forty_one: String = "-".repeat(41);
    let over_limit = dom.add(Some(body), "button");
    dom.add_text(over_limit, &forty_one);
    dom.set_text_lines(
        over_limit,
        &[(0.0, 0.0, 200.0, 20.0), (0.0, 20.0, 40.0, 20.0)],
    );
    assert!(element_type_ids(&dom, over_limit).is_empty());
}

// ─── control-height-mismatch ───────────────────────────────────────────────

fn field_row(dom: &mut FakeDom, body: ElId, field_height: f64, button_height: f64) -> ElId {
    let parent = dom.add(Some(body), "div");
    let input = dom.add(Some(parent), "input");
    dom.set_attr(input, "type", "text");
    dom.set_rect(input, 0.0, 0.0, 200.0, field_height);
    let button = dom.add(Some(parent), "button");
    dom.set_rect(button, 220.0, 0.0, 80.0, button_height);
    input
}

#[test]
fn control_height_mismatch_reports_rows_off_by_more_than_two_pixels() {
    let (mut dom, body) = page();
    let input = field_row(&mut dom, body, 40.0, 44.0);
    assert_eq!(
        element_type_ids(&dom, input),
        vec!["xgent/control-height-mismatch"]
    );
    assert_eq!(
        element_details(&dom, input),
        vec!["input 40px beside button 44px"]
    );

    let (mut dom, body) = page();
    let input = field_row(&mut dom, body, 40.0, 43.0);
    assert_eq!(
        element_details(&dom, input),
        vec!["input 40px beside button 43px"]
    );
}

#[test]
fn control_height_mismatch_accepts_two_pixels_and_other_rows() {
    let (mut dom, body) = page();
    let input = field_row(&mut dom, body, 40.0, 42.0);
    assert!(element_type_ids(&dom, input).is_empty());

    // A partner not on the same row is not a pair.
    let (mut dom, body) = page();
    let parent = dom.add(Some(body), "div");
    let in_row = dom.add(Some(parent), "input");
    dom.set_attr(in_row, "type", "email");
    dom.set_rect(in_row, 0.0, 0.0, 200.0, 40.0);
    let other_row = dom.add(Some(parent), "button");
    dom.set_rect(other_row, 220.0, 40.0, 80.0, 56.0);
    assert!(element_type_ids(&dom, in_row).is_empty());

    // Too far apart horizontally is not the same row either.
    let (mut dom, body) = page();
    let parent = dom.add(Some(body), "div");
    let far = dom.add(Some(parent), "input");
    dom.set_attr(far, "type", "text");
    dom.set_rect(far, 0.0, 0.0, 200.0, 40.0);
    let far_button = dom.add(Some(parent), "button");
    dom.set_rect(far_button, 250.0, 0.0, 80.0, 56.0);
    assert!(element_type_ids(&dom, far).is_empty());

    // Out-of-range control heights are not measured as controls.
    let (mut dom, body) = page();
    let tiny = field_row(&mut dom, body, 19.0, 40.0);
    assert!(element_type_ids(&dom, tiny).is_empty());
    let (mut dom, body) = page();
    let huge = field_row(&mut dom, body, 81.0, 40.0);
    assert!(element_type_ids(&dom, huge).is_empty());
}

#[test]
fn control_height_mismatch_ignores_non_field_inputs() {
    let (mut dom, body) = page();
    let parent = dom.add(Some(body), "div");
    let checkbox = dom.add(Some(parent), "input");
    dom.set_attr(checkbox, "type", "checkbox");
    dom.set_rect(checkbox, 0.0, 0.0, 16.0, 16.0);
    let button = dom.add(Some(parent), "button");
    dom.set_rect(button, 30.0, 0.0, 80.0, 44.0);
    assert!(element_type_ids(&dom, checkbox).is_empty());
}

// ─── unlabeled-inline-svg ──────────────────────────────────────────────────

fn svg_of_size(dom: &mut FakeDom, body: ElId, size: f64) -> ElId {
    let svg = dom.add(Some(body), "svg");
    dom.set_rect(svg, 0.0, 0.0, size, size);
    svg
}

#[test]
fn unlabeled_inline_svg_reports_an_unnamed_graphic() {
    let (mut dom, body) = page();
    let svg = svg_of_size(&mut dom, body, 24.0);
    assert_eq!(
        element_type_ids(&dom, svg),
        vec!["xgent/unlabeled-inline-svg"]
    );
    assert_eq!(
        element_details(&dom, svg),
        vec!["<svg 24×24> has no accessible name and is not aria-hidden"]
    );
}

#[test]
fn unlabeled_inline_svg_accepts_names_hidden_decorations_and_control_context() {
    let (mut dom, body) = page();

    let labelled = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(labelled, "aria-label", "Search");
    assert!(element_type_ids(&dom, labelled).is_empty());

    let referenced = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(referenced, "aria-labelledby", "title-id");
    assert!(element_type_ids(&dom, referenced).is_empty());

    let decorative = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(decorative, "aria-hidden", "true");
    assert!(element_type_ids(&dom, decorative).is_empty());

    let presentation = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(presentation, "role", "presentation");
    assert!(element_type_ids(&dom, presentation).is_empty());

    let titled = svg_of_size(&mut dom, body, 24.0);
    let title = dom.add(Some(titled), "title");
    dom.add_text(title, "Logo");
    assert!(element_type_ids(&dom, titled).is_empty());

    // An SVG `title` attribute is an accessible name too (F-05).
    let titled_attribute = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(titled_attribute, "title", "Company logo");
    assert!(element_type_ids(&dom, titled_attribute).is_empty());

    let spaced_title = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(spaced_title, "title", "  Company logo  ");
    assert!(element_type_ids(&dom, spaced_title).is_empty());

    // A blank title attribute is not a name.
    let blank_title = svg_of_size(&mut dom, body, 24.0);
    dom.set_attr(blank_title, "title", "   ");
    assert_eq!(
        element_type_ids(&dom, blank_title),
        vec!["xgent/unlabeled-inline-svg"]
    );

    // A graphic that names its control (inside a button, link, label, summary,
    // role=button/link, or a labelled region) is covered by that context.
    let button = dom.add(Some(body), "button");
    dom.add_text(button, "Search");
    let in_button = dom.add(Some(button), "svg");
    dom.set_rect(in_button, 0.0, 0.0, 24.0, 24.0);
    assert!(element_type_ids(&dom, in_button).is_empty());

    let link = dom.add(Some(body), "a");
    let in_link = dom.add(Some(link), "svg");
    dom.set_rect(in_link, 0.0, 0.0, 24.0, 24.0);
    assert!(element_type_ids(&dom, in_link).is_empty());

    let role_link = dom.add(Some(body), "div");
    dom.set_attr(role_link, "role", "link");
    let in_role_link = dom.add(Some(role_link), "svg");
    dom.set_rect(in_role_link, 0.0, 0.0, 24.0, 24.0);
    assert!(element_type_ids(&dom, in_role_link).is_empty());

    let labelled_ancestor = dom.add(Some(body), "div");
    dom.set_attr(labelled_ancestor, "aria-label", "Region");
    let in_labelled_ancestor = dom.add(Some(labelled_ancestor), "svg");
    dom.set_rect(in_labelled_ancestor, 0.0, 0.0, 24.0, 24.0);
    assert!(element_type_ids(&dom, in_labelled_ancestor).is_empty());

    // An aria-hidden ancestor hides the whole subtree.
    let wrapper = dom.add(Some(body), "div");
    dom.set_attr(wrapper, "aria-hidden", "true");
    let hidden_svg = dom.add(Some(wrapper), "svg");
    dom.set_rect(hidden_svg, 0.0, 0.0, 24.0, 24.0);
    assert!(element_type_ids(&dom, hidden_svg).is_empty());

    let small = svg_of_size(&mut dom, body, 15.0);
    assert!(element_type_ids(&dom, small).is_empty());
    let exactly_sixteen = svg_of_size(&mut dom, body, 16.0);
    assert_eq!(
        element_type_ids(&dom, exactly_sixteen),
        vec!["xgent/unlabeled-inline-svg"]
    );
}

#[test]
fn unlabeled_inline_svg_only_checks_the_outermost_svg() {
    let (mut dom, body) = page();
    let outer = svg_of_size(&mut dom, body, 40.0);
    let inner = dom.add(Some(outer), "svg");
    dom.set_rect(inner, 0.0, 0.0, 24.0, 24.0);
    assert!(element_type_ids(&dom, inner).is_empty());
    assert_eq!(
        element_type_ids(&dom, outer),
        vec!["xgent/unlabeled-inline-svg"]
    );
}

// ─── redrawn-window-chrome ─────────────────────────────────────────────────

fn dot(dom: &mut FakeDom, parent: ElId, x: f64, color: &str) -> ElId {
    let dot = dom.add(Some(parent), "span");
    dom.set_rect(dot, x, 0.0, 12.0, 12.0);
    dom.set_style(dot, "borderRadius", "6px");
    dom.set_style(dot, "backgroundColor", color);
    dot
}

#[test]
fn redrawn_window_chrome_reports_three_ordered_dots() {
    let (mut dom, body) = page();
    let bar = dom.add(Some(body), "div");
    dot(&mut dom, bar, 0.0, "rgb(255, 0, 0)");
    dot(&mut dom, bar, 20.0, "rgb(255, 165, 0)");
    dot(&mut dom, bar, 40.0, "rgb(0, 160, 60)");
    assert_eq!(
        element_type_ids(&dom, bar),
        vec!["xgent/redrawn-window-chrome"]
    );
    assert_eq!(
        element_details(&dom, bar),
        vec!["three window-control dots in <div>"]
    );
}

#[test]
fn redrawn_window_chrome_accepts_oklch_colours() {
    // Tailwind v4's red-500, amber-500 and green-500, exactly as a project
    // writing oklch() would store them; the rule must read them the way the
    // built-in colour rules do (F-09).
    let (mut dom, body) = page();
    let bar = dom.add(Some(body), "div");
    dot(&mut dom, bar, 0.0, "oklch(0.637 0.237 25.331)");
    dot(&mut dom, bar, 20.0, "oklch(0.769 0.188 70.08)");
    dot(&mut dom, bar, 40.0, "oklch(0.723 0.219 149.579)");
    assert_eq!(
        element_type_ids(&dom, bar),
        vec!["xgent/redrawn-window-chrome"]
    );
}

#[test]
fn redrawn_window_chrome_rejects_reordered_or_off_geometry_dots() {
    let cases: Vec<fn(&mut FakeDom, ElId)> = vec![
        // Reversed hue order is decoration, not window chrome.
        |dom, bar| {
            dot(dom, bar, 0.0, "rgb(0, 160, 60)");
            dot(dom, bar, 20.0, "rgb(255, 165, 0)");
            dot(dom, bar, 40.0, "rgb(255, 0, 0)");
        },
        // Square corners.
        |dom, bar| {
            for (index, color) in ["rgb(255, 0, 0)", "rgb(255, 165, 0)", "rgb(0, 160, 60)"]
                .iter()
                .enumerate()
            {
                let id = dot(dom, bar, index as f64 * 20.0, color);
                dom.set_style(id, "borderRadius", "2px");
            }
        },
        // Not the same row.
        |dom, bar| {
            for (index, color) in ["rgb(255, 0, 0)", "rgb(255, 165, 0)", "rgb(0, 160, 60)"]
                .iter()
                .enumerate()
            {
                let id = dot(dom, bar, index as f64 * 20.0, color);
                let rect = dom.rect(id);
                dom.set_rect(
                    id,
                    rect.left,
                    if index == 2 { 4.0 } else { 0.0 },
                    12.0,
                    12.0,
                );
            }
        },
        // Too small, too large, or lacking chroma.
        |dom, bar| {
            for (index, color) in ["rgb(255, 0, 0)", "rgb(255, 165, 0)", "rgb(0, 160, 60)"]
                .iter()
                .enumerate()
            {
                let id = dot(dom, bar, index as f64 * 20.0, color);
                let size = [5.0, 17.0, 8.0][index];
                dom.set_rect(id, index as f64 * 20.0, 0.0, size, size);
            }
        },
        |dom, bar| {
            dot(dom, bar, 0.0, "rgb(128, 128, 128)");
            dot(dom, bar, 20.0, "rgb(255, 165, 0)");
            dot(dom, bar, 40.0, "rgb(0, 160, 60)");
        },
        // Transparent dots are not painted chrome.
        |dom, bar| {
            dot(dom, bar, 0.0, "rgba(255, 0, 0, 0.2)");
            dot(dom, bar, 20.0, "rgb(255, 165, 0)");
            dot(dom, bar, 40.0, "rgb(0, 160, 60)");
        },
    ];
    for (index, build) in cases.iter().enumerate() {
        let (mut dom, body) = page();
        let bar = dom.add(Some(body), "div");
        build(&mut dom, bar);
        assert!(element_type_ids(&dom, bar).is_empty(), "case {index}");
    }
}

#[test]
fn redrawn_window_chrome_ignores_aria_hidden_but_not_real_hiding() {
    // Window chrome is decorative and commonly aria-hidden for the
    // accessibility tree while still painted; the visual rule must keep
    // seeing it (F-07).
    let (mut dom, body) = page();
    let bar = dom.add(Some(body), "div");
    dom.set_attr(bar, "aria-hidden", "true");
    dot(&mut dom, bar, 0.0, "rgb(255, 0, 0)");
    dot(&mut dom, bar, 20.0, "rgb(255, 165, 0)");
    dot(&mut dom, bar, 40.0, "rgb(0, 160, 60)");
    assert_eq!(
        element_type_ids(&dom, bar),
        vec!["xgent/redrawn-window-chrome"]
    );

    // A dot's own aria-hidden hides it from the accessibility tree, not from
    // the page.
    let (mut dom, body) = page();
    let bar = dom.add(Some(body), "div");
    let red = dot(&mut dom, bar, 0.0, "rgb(255, 0, 0)");
    dom.set_attr(red, "aria-hidden", "true");
    dot(&mut dom, bar, 20.0, "rgb(255, 165, 0)");
    dot(&mut dom, bar, 40.0, "rgb(0, 160, 60)");
    assert_eq!(
        element_type_ids(&dom, bar),
        vec!["xgent/redrawn-window-chrome"]
    );

    // Real hiding still stands down, on the bar or on a dot.
    let cases: &[(&str, &str, bool)] = &[
        ("display", "none", false),
        ("visibility", "hidden", false),
        ("visibility", "collapse", false),
        ("opacity", "0", false),
        ("opacity", "0.01", false),
        ("opacity", "0.02", true),
        ("contentVisibility", "hidden", false),
    ];
    for &(property, value, reports) in cases {
        let (mut dom, body) = page();
        let bar = dom.add(Some(body), "div");
        dom.set_style(bar, property, value);
        dot(&mut dom, bar, 0.0, "rgb(255, 0, 0)");
        dot(&mut dom, bar, 20.0, "rgb(255, 165, 0)");
        dot(&mut dom, bar, 40.0, "rgb(0, 160, 60)");
        assert_eq!(
            !element_type_ids(&dom, bar).is_empty(),
            reports,
            "bar {property}: {value}"
        );

        let (mut dom, body) = page();
        let bar = dom.add(Some(body), "div");
        let red = dot(&mut dom, bar, 0.0, "rgb(255, 0, 0)");
        dot(&mut dom, bar, 20.0, "rgb(255, 165, 0)");
        dot(&mut dom, bar, 40.0, "rgb(0, 160, 60)");
        dom.set_style(red, property, value);
        assert_eq!(
            !element_type_ids(&dom, bar).is_empty(),
            reports,
            "dot {property}: {value}"
        );
    }
}

// ─── emoji-icon ────────────────────────────────────────────────────────────

fn emoji_before_heading(dom: &mut FakeDom, body: ElId, emoji: &str, heading: &str) -> ElId {
    let icon = dom.add(Some(body), "div");
    dom.add_text(icon, emoji);
    dom.set_style(icon, "fontSize", "24px");
    let heading = dom.add(Some(body), heading);
    dom.add_text(heading, "Feature");
    icon
}

#[test]
fn emoji_icon_reports_two_or_more_icon_emoji() {
    let (mut dom, body) = page();
    let rocket = emoji_before_heading(&mut dom, body, "🚀", "h2");
    let bolt = emoji_before_heading(&mut dom, body, "⚡", "h3");
    let findings = page_findings(&dom);
    assert_eq!(
        ids(&findings),
        vec![
            (
                "xgent/emoji-icon",
                Some(rocket),
                "emoji \"🚀\" used as an icon before <h2>"
            ),
            (
                "xgent/emoji-icon",
                Some(bolt),
                "emoji \"⚡\" used as an icon before <h3>"
            ),
        ]
    );
}

#[test]
fn emoji_icon_needs_two_candidates_and_a_heading() {
    let (mut dom, body) = page();
    // A lone emoji is not a pattern.
    let single = dom.add(Some(body), "div");
    dom.add_text(single, "🚀");
    dom.set_style(single, "fontSize", "24px");
    let heading = dom.add(Some(body), "h2");
    dom.add_text(heading, "Feature");
    assert!(page_findings(&dom).is_empty());

    // Emoji before a paragraph is not an icon heading pair.
    let (mut dom, body) = page();
    let before_paragraph = dom.add(Some(body), "div");
    dom.add_text(before_paragraph, "🚀");
    dom.set_style(before_paragraph, "fontSize", "24px");
    let paragraph = dom.add(Some(body), "p");
    dom.add_text(paragraph, "Copy");
    let other = dom.add(Some(body), "div");
    dom.add_text(other, "⚡");
    dom.set_style(other, "fontSize", "24px");
    let other_paragraph = dom.add(Some(body), "p");
    dom.add_text(other_paragraph, "Copy");
    assert!(page_findings(&dom).is_empty());

    // Text symbols are not emoji presentation characters.
    let (mut dom, body) = page();
    for heading_tag in ["h2", "h3"] {
        let icon = dom.add(Some(body), "div");
        dom.add_text(icon, "▶");
        dom.set_style(icon, "fontSize", "24px");
        let heading = dom.add(Some(body), heading_tag);
        dom.add_text(heading, "Feature");
    }
    assert!(page_findings(&dom).is_empty());

    // Small type is inline text, not an icon.
    let (mut dom, body) = page();
    for emoji in ["🚀", "⚡"] {
        let icon = dom.add(Some(body), "div");
        dom.add_text(icon, emoji);
        dom.set_style(icon, "fontSize", "18px");
        let heading = dom.add(Some(body), "h2");
        dom.add_text(heading, "Feature");
    }
    assert!(page_findings(&dom).is_empty());
}

#[test]
fn emoji_icon_accepts_an_only_child_before_a_heading() {
    let (mut dom, body) = page();
    let wrapper = dom.add(Some(body), "div");
    // Real pages inherit font-size: both the wrapper and its icon clear the
    // 20px threshold, so the candidate must be chosen by who owns the text,
    // not by size (F-06).
    dom.set_style(wrapper, "fontSize", "24px");
    let icon = dom.add(Some(wrapper), "span");
    dom.add_text(icon, "🚀");
    dom.set_style(icon, "fontSize", "24px");
    let heading = dom.add(Some(body), "h3");
    dom.add_text(heading, "Feature");
    let second = emoji_before_heading(&mut dom, body, "⚡", "h4");
    let findings = page_findings(&dom);
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert_eq!(findings[0].el, Some(icon));
    assert_eq!(findings[1].el, Some(second));
    assert_eq!(
        findings[0].finding.detail,
        "emoji \"🚀\" used as an icon before <h3>"
    );
}

#[test]
fn emoji_icon_wrapped_icons_report_the_text_owner_only() {
    // A wrapper's text_content is its child's emoji; only the child owns the
    // emoji as text, so one wrapped icon stays one candidate (F-06).
    let (mut dom, body) = page();
    let wrapper = dom.add(Some(body), "div");
    dom.set_style(wrapper, "fontSize", "24px");
    let icon = dom.add(Some(wrapper), "span");
    dom.add_text(icon, "🚀");
    dom.set_style(icon, "fontSize", "24px");
    let heading = dom.add(Some(body), "h2");
    dom.add_text(heading, "Feature");
    assert!(page_findings(&dom).is_empty(), "one wrapped icon alone");

    let wrapper_second = dom.add(Some(body), "div");
    dom.set_style(wrapper_second, "fontSize", "24px");
    let second_icon = dom.add(Some(wrapper_second), "span");
    dom.add_text(second_icon, "⚡");
    dom.set_style(second_icon, "fontSize", "24px");
    let second_heading = dom.add(Some(body), "h2");
    dom.add_text(second_heading, "Build");
    let findings = page_findings(&dom);
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert_eq!(findings[0].el, Some(icon));
    assert_eq!(findings[1].el, Some(second_icon));

    // Whitespace beside the wrapped icon does not make the wrapper its owner.
    let (mut dom, body) = page();
    let wrapper = dom.add(Some(body), "div");
    dom.set_style(wrapper, "fontSize", "24px");
    dom.add_text(wrapper, " ");
    let icon = dom.add(Some(wrapper), "span");
    dom.add_text(icon, "🚀");
    dom.set_style(icon, "fontSize", "24px");
    dom.add_text(wrapper, "\n  ");
    let heading = dom.add(Some(body), "h2");
    dom.add_text(heading, "Feature");
    let second = emoji_before_heading(&mut dom, body, "⚡", "h3");
    let findings = page_findings(&dom);
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert_eq!(findings[0].el, Some(icon));
    assert_eq!(findings[1].el, Some(second));
}

#[test]
fn emoji_icon_ignores_aria_hidden_but_not_real_hiding() {
    // Decorative emoji are commonly aria-hidden for the accessibility tree
    // while still painted; two visible icons still report (F-07).
    let (mut dom, body) = page();
    let rocket = emoji_before_heading(&mut dom, body, "🚀", "h2");
    dom.set_attr(rocket, "aria-hidden", "true");
    let bolt = emoji_before_heading(&mut dom, body, "⚡", "h3");
    dom.set_attr(bolt, "aria-hidden", "true");
    let findings = page_findings(&dom);
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert_eq!(findings[0].el, Some(rocket));
    assert_eq!(findings[1].el, Some(bolt));

    // An aria-hidden wrapper hides the icons from screen readers only.
    let (mut dom, body) = page();
    let section = dom.add(Some(body), "div");
    dom.set_attr(section, "aria-hidden", "true");
    emoji_before_heading(&mut dom, section, "🚀", "h2");
    emoji_before_heading(&mut dom, section, "⚡", "h2");
    assert_eq!(page_findings(&dom).len(), 2);

    // Real hiding still stands down.
    for &(property, value) in &[
        ("display", "none"),
        ("visibility", "hidden"),
        ("opacity", "0"),
        ("contentVisibility", "hidden"),
    ] {
        let (mut dom, body) = page();
        for emoji in ["🚀", "⚡"] {
            let icon = dom.add(Some(body), "div");
            dom.add_text(icon, emoji);
            dom.set_style(icon, "fontSize", "24px");
            dom.set_style(icon, property, value);
            let heading = dom.add(Some(body), "h2");
            dom.add_text(heading, "Feature");
        }
        assert!(
            page_findings(&dom).is_empty(),
            "{property}: {value} should not report"
        );
    }
}

// ─── font-family-count ─────────────────────────────────────────────────────

fn text_in(dom: &mut FakeDom, body: ElId, tag: &str, text: &str, family: &str) -> ElId {
    let el = dom.add(Some(body), tag);
    dom.add_text(el, text);
    dom.set_style(el, "fontFamily", family);
    el
}

#[test]
fn font_family_count_reports_four_visible_families() {
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "Body copy", "Inter");
    text_in(&mut dom, body, "p", "Intro copy", "Georgia");
    text_in(
        &mut dom,
        body,
        "h1",
        "Display",
        "\"Playfair Display\", serif",
    );
    text_in(&mut dom, body, "p", "More copy", "Roboto, Arial");
    let findings = page_findings(&dom);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].el, None);
    assert_eq!(
        ids(&findings),
        vec![(
            "xgent/font-family-count",
            None,
            "4 families: inter (p), georgia (p), playfair display (h1), roboto (p)"
        )]
    );
}

#[test]
fn font_family_count_ignores_generics_aliases_icons_and_code() {
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "Body", "Inter");
    text_in(&mut dom, body, "p", "Intro", "Georgia");
    text_in(&mut dom, body, "p", "Meta", "Roboto");
    assert!(page_findings(&dom).is_empty(), "three families");

    // System aliases are one family, not four.
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "Body", "system-ui");
    text_in(&mut dom, body, "p", "Intro", "-apple-system");
    text_in(&mut dom, body, "p", "Meta", "Inter");
    text_in(&mut dom, body, "p", "More", "Georgia");
    assert!(page_findings(&dom).is_empty());

    // CSS generics do not count as families.
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "Body", "serif");
    text_in(&mut dom, body, "p", "Intro", "sans-serif");
    text_in(&mut dom, body, "p", "Meta", "monospace");
    text_in(&mut dom, body, "p", "More", "Inter");
    text_in(&mut dom, body, "p", "Another", "Georgia");
    assert!(page_findings(&dom).is_empty());

    // Icon faces are not text families.
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "A", "Inter");
    text_in(&mut dom, body, "p", "B", "Georgia");
    text_in(&mut dom, body, "p", "C", "Roboto");
    text_in(&mut dom, body, "p", "D", "Material Icons");
    assert!(page_findings(&dom).is_empty());

    // Code/pre text does not vote.
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "A", "Inter");
    text_in(&mut dom, body, "p", "B", "Georgia");
    text_in(&mut dom, body, "p", "C", "Roboto");
    text_in(&mut dom, body, "code", "D", "Fira Code");
    assert!(page_findings(&dom).is_empty());

    // An element without its own text does not vote either.
    let (mut dom, body) = page();
    text_in(&mut dom, body, "p", "A", "Inter");
    text_in(&mut dom, body, "p", "B", "Georgia");
    text_in(&mut dom, body, "p", "C", "Roboto");
    let wrapper = dom.add(Some(body), "div");
    dom.set_style(wrapper, "fontFamily", "Lato");
    assert!(page_findings(&dom).is_empty());
}

// ─── robustness ────────────────────────────────────────────────────────────

#[test]
fn zero_sized_and_parentless_elements_do_not_panic() {
    let (mut dom, body) = page();
    let zero_svg = dom.add(Some(body), "svg");
    dom.set_rect(zero_svg, 0.0, 0.0, 0.0, 0.0);
    let parentless = dom.add(None, "button");
    dom.add_text(parentless, "Go");
    let empty = dom.add(Some(body), "button");
    for el in all_elements(&dom) {
        let _ = PACK.check_element_dom(&dom, el);
    }
    let _ = page_findings(&dom);
    assert!(element_type_ids(&dom, zero_svg).is_empty());
    assert!(element_type_ids(&dom, empty).is_empty());
    assert!(element_type_ids(&dom, parentless).is_empty());
}
