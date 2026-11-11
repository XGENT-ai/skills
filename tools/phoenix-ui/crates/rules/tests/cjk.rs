//! The five `xgent/cjk-*` rules through the public rule-pack hooks.
//!
//! FakeDom stands in for a rendered page: a test declares the lines and fonts
//! a browser would have measured and reads the findings a user would see.
//! Missing facts and degenerate geometry must give no finding and never
//! panic: the release binary aborts on panic.

use impeccable_core::browser::dom::{FailedWebFont, HanFontFacts, HanFontRun, RenderedFont};
use impeccable_core::browser::fake_dom::FakeDom;
use impeccable_core::browser::ElId;
use impeccable_core::rule_pack::RulePack;
use phoenix_ui_rules::PACK;

const BODY: &str = "这是一段用来检查中文排版规则的正文内容";

fn page() -> (FakeDom, ElId) {
    let mut dom = FakeDom::new();
    let (_html, body) = dom.with_page();
    (dom, body)
}

fn passage(dom: &mut FakeDom, parent: ElId, tag: &str, text: &str) -> ElId {
    let el = dom.add(Some(parent), tag);
    dom.add_text(el, text);
    dom.set_style(el, "fontSize", "16px");
    el
}

/// `widths` are line widths in px, stacked top to bottom.
fn set_lines(dom: &mut FakeDom, el: ElId, widths: &[f64]) {
    let lines: Vec<(f64, f64, f64, f64)> = widths
        .iter()
        .enumerate()
        .map(|(row, width)| (0.0, row as f64 * 26.0, *width, 22.0))
        .collect();
    dom.set_text_lines(el, &lines);
}

fn cjk_details(dom: &FakeDom, el: ElId) -> Vec<(String, String)> {
    PACK.check_element_dom(dom, el)
        .into_iter()
        .filter(|finding| finding.type_.starts_with("xgent/cjk-"))
        .map(|finding| (finding.type_, finding.detail))
        .collect()
}

fn one(id: &str, detail: &str) -> Vec<(String, String)> {
    vec![(id.to_string(), detail.to_string())]
}

fn font(family: &str, glyphs: u32) -> RenderedFont {
    RenderedFont {
        family: family.to_string(),
        custom: false,
        glyphs,
    }
}

fn run(declared: &[&str], rendered: Vec<RenderedFont>) -> HanFontRun {
    HanFontRun {
        declared_families: declared.iter().map(|name| name.to_string()).collect(),
        weight: "400".to_string(),
        style: "normal".to_string(),
        stretch: "100%".to_string(),
        lang: "zh-CN".to_string(),
        rendered,
        failed_web_fonts: Vec::new(),
    }
}

fn failed(family: &str, index: usize, earlier_declared_rendered: bool) -> FailedWebFont {
    FailedWebFont {
        family: family.to_string(),
        weight: "400".to_string(),
        style: "normal".to_string(),
        stretch: "normal".to_string(),
        unicode_range: "U+0-10FFFF".to_string(),
        declaration_index: index,
        earlier_declared_rendered,
    }
}

fn with_fonts(dom: &mut FakeDom, parent: ElId, runs: Vec<HanFontRun>) -> ElId {
    let el = passage(dom, parent, "p", BODY);
    dom.set_han_fonts(el, HanFontFacts { runs });
    el
}

fn fallback_findings(dom: &FakeDom) -> Vec<(Option<ElId>, String)> {
    PACK.check_page_dom(dom)
        .into_iter()
        .filter(|finding| finding.finding.type_ == "xgent/cjk-font-fallback")
        .map(|finding| (finding.el, finding.finding.detail))
        .collect()
}

// ─── which text counts as Chinese ──────────────────────────────────────────

#[test]
fn only_han_dominant_body_text_takes_over_the_latin_rules() {
    let replaced = ["line-length", "tight-leading"];
    let cases: &[(&str, &str, bool)] = &[
        ("p", BODY, true),
        ("li", BODY, true),
        // Nine Han to eight Latin: more than half.
        ("p", "排版检查规则说明了abcdefgh", true),
        // Eight to eight is exactly half, which is not "mostly Han".
        ("p", "排版检查规则说明abcdefgh", false),
        // Whitespace and punctuation are not counted on either side.
        ("p", "排版，检查。规则！ a b c d？", true),
        ("p", "これはひらがなのぶんしょうです漢字", false),
        ("p", "한국어로 작성된 문장입니다 漢字", false),
        ("p", "Release notes for 张伟 and the team", false),
        // Headings and other tags keep whatever the built-in rules say.
        ("h2", BODY, false),
        ("div", BODY, false),
        // Ten characters of direct text is not body copy yet.
        ("p", "只有十个字的直接文本", false),
    ];
    for (tag, text, expected) in cases {
        let (mut dom, body) = page();
        let el = passage(&mut dom, body, tag, text);
        let ids = PACK.replaces_builtin(&dom, el);
        assert_eq!(ids == replaced, *expected, "<{tag}> {text}");
        assert!(*expected || ids.is_empty(), "<{tag}> {text}");
    }
}

#[test]
fn text_that_is_not_rendered_is_not_judged() {
    let (mut dom, body) = page();
    let el = passage(&mut dom, body, "p", BODY);
    dom.set_style(el, "display", "none");
    dom.set_style(el, "lineHeight", "19px");
    set_lines(&mut dom, el, &[700.0, 700.0]);
    assert!(PACK.replaces_builtin(&dom, el).is_empty());
    assert!(cjk_details(&dom, el).is_empty());
}

// ─── cjk-tight-leading ─────────────────────────────────────────────────────

#[test]
fn tight_leading_reports_multi_line_body_below_one_and_a_half() {
    let leading = |line_height: &str, widths: &[f64]| {
        let (mut dom, body) = page();
        let el = passage(&mut dom, body, "p", BODY);
        dom.set_style(el, "lineHeight", line_height);
        set_lines(&mut dom, el, widths);
        cjk_details(&dom, el)
    };
    let two = [320.0, 200.0];
    assert_eq!(
        leading("22.4px", &two),
        one(
            "xgent/cjk-tight-leading",
            "line-height 1.40x on Chinese body text (need >=1.5)"
        )
    );
    assert_eq!(
        leading("23.84px", &two),
        one(
            "xgent/cjk-tight-leading",
            "line-height 1.49x on Chinese body text (need >=1.5)"
        )
    );
    assert!(leading("24px", &two).is_empty());
    // `normal` depends on the font that rendered; it is not judged.
    assert!(leading("normal", &two).is_empty());
    // One line has no leading to read.
    assert!(leading("19.2px", &[320.0]).is_empty());
}

#[test]
fn tight_leading_leaves_headings_alone() {
    let (mut dom, body) = page();
    let el = passage(&mut dom, body, "h2", BODY);
    dom.set_style(el, "lineHeight", "19.2px");
    set_lines(&mut dom, el, &[320.0, 200.0]);
    assert!(cjk_details(&dom, el).is_empty());
}

// ─── cjk-line-length ───────────────────────────────────────────────────────

#[test]
fn line_length_reports_two_lines_past_forty_han_widths() {
    let length = |widths: &[f64], tracking: &str| {
        let (mut dom, body) = page();
        let el = passage(&mut dom, body, "p", BODY);
        dom.set_style(el, "lineHeight", "27px");
        dom.set_style(el, "letterSpacing", tracking);
        set_lines(&mut dom, el, widths);
        cjk_details(&dom, el)
    };
    // 16px type: 656px is 41 characters wide, 640px is 40.
    assert_eq!(
        length(&[832.0, 656.0, 120.0], "normal"),
        one(
            "xgent/cjk-line-length",
            "~52 Han-width chars on 2 of 3 rendered lines (aim for <=40)"
        )
    );
    assert!(length(&[640.0, 640.0, 640.0], "normal").is_empty());
    // One long line and a short tail is a sentence that wrapped once.
    assert!(length(&[832.0, 120.0], "normal").is_empty());
    // Tracking widens every character: 800px at 16px + 4px is 40.
    assert!(length(&[800.0, 800.0], "4px").is_empty());
}

// ─── cjk-orphan ────────────────────────────────────────────────────────────

#[test]
fn orphan_reports_a_last_line_of_one_or_two_han_characters() {
    let orphan = |tag: &str, lines: &[&str]| {
        let (mut dom, body) = page();
        let el = passage(&mut dom, body, tag, BODY);
        dom.set_text_line_texts(el, lines);
        cjk_details(&dom, el)
    };
    let first = "这是一段正文的第一行内容";
    assert_eq!(
        orphan("p", &[first, "了。"]),
        one("xgent/cjk-orphan", "last line holds only \"了。\"")
    );
    assert_eq!(
        orphan("h1", &[first, " 内容 "]),
        one("xgent/cjk-orphan", "last line holds only \"内容\"")
    );
    assert!(orphan("p", &[first, "三个字"]).is_empty());
    assert!(orphan("p", &[first, "ok"]).is_empty());
    assert!(orphan("p", &[first, "了a"]).is_empty());
    assert!(orphan("p", &[first, "。"]).is_empty());
    assert!(orphan("p", &["了"]).is_empty());
    // A line of collapsed whitespace is not a line.
    assert!(orphan("p", &["了", "  "]).is_empty());
}

// ─── cjk-split-unit ────────────────────────────────────────────────────────

#[test]
fn split_unit_reports_the_first_number_parted_from_its_unit() {
    let split = |lines: &[&str]| {
        let (mut dom, body) = page();
        let el = passage(&mut dom, body, "p", BODY);
        dom.set_text_line_texts(el, lines);
        cjk_details(&dom, el)
    };
    let hit = |number: &str, unit: &str| {
        one(
            "xgent/cjk-split-unit",
            &format!("number \"{number}\" and its unit \"{unit}\" sit on different lines"),
        )
    };
    assert_eq!(
        split(&["本次扫描总共耗时38 ", "秒才最终全部完成"]),
        hit("38", "秒")
    );
    assert_eq!(
        split(&["文件大小为 1,024", "字节，可以直接上传"]),
        hit("1,024", "字节")
    );
    assert_eq!(
        split(&["我们会在 3", "工作日内完成审核流程"]),
        hit("3", "工作日")
    );
    assert_eq!(
        split(&["已经有 99.5", "% 的页面通过了检查"]),
        hit("99.5", "%")
    );
    assert_eq!(
        split(&["全角数字 ３８", "秒也算数字的写法"]),
        hit("３８", "秒")
    );
    // Only the first split in an element is reported.
    assert_eq!(
        split(&["先等 5", "分钟再重试，最多 3", "次就放弃这个任务"]),
        hit("5", "分")
    );
    assert!(split(&["一共处理了 38", "正在等待的任务请求"]).is_empty());
    assert!(split(&["总共耗时三十八", "秒才最终全部完成"]).is_empty());
    assert!(split(&["本次扫描总共耗时38秒才最终全部完成"]).is_empty());
}

// ─── cjk-font-fallback ─────────────────────────────────────────────────────

#[test]
fn font_fallback_reports_a_failed_web_font_that_left_han_to_a_later_font() {
    let (mut dom, body) = page();
    let mut broken = run(
        &["Fixture Han", "sans-serif"],
        vec![font("PingFang SC", 15)],
    );
    broken
        .failed_web_fonts
        .push(failed("Fixture Han", 0, false));
    let el = with_fonts(&mut dom, body, vec![broken]);
    assert_eq!(
        fallback_findings(&dom),
        vec![(
            Some(el),
            "web font \"Fixture Han\" failed to load; Han text renders in PingFang SC".to_string()
        )]
    );
}

#[test]
fn font_fallback_ignores_a_failed_face_an_earlier_family_already_covered() {
    let (mut dom, body) = page();
    let mut covered = run(
        &["Songti SC", "Fixture Han", "serif"],
        vec![font("Songti SC", 15)],
    );
    covered
        .failed_web_fonts
        .push(failed("Fixture Han", 1, true));
    with_fonts(&mut dom, body, vec![covered]);
    assert!(fallback_findings(&dom).is_empty());
}

#[test]
fn font_fallback_reports_one_passage_split_across_fonts() {
    let (mut dom, body) = page();
    let el = with_fonts(
        &mut dom,
        body,
        vec![run(
            &["Hiragino Mincho ProN", "Songti SC"],
            vec![font("Hiragino Mincho ProN", 25), font("Songti SC", 12)],
        )],
    );
    assert_eq!(
        fallback_findings(&dom),
        vec![(
            Some(el),
            "Han text renders in 2 fonts: Hiragino Mincho ProN (25 glyphs), Songti SC (12 glyphs)"
                .to_string()
        )]
    );
}

#[test]
fn font_fallback_reports_a_list_with_no_han_font_and_no_generic() {
    let (mut dom, body) = page();
    let el = with_fonts(
        &mut dom,
        body,
        vec![run(&["Helvetica Neue"], vec![font("PingFang SC", 15)])],
    );
    assert_eq!(
        fallback_findings(&dom),
        vec![(
            Some(el),
            "no listed font renders Han and the list has no generic family; Han text renders in PingFang SC"
                .to_string()
        )]
    );
}

#[test]
fn font_fallback_accepts_a_font_list_that_worked() {
    let web_font = RenderedFont {
        family: "BrandHan-Regular".to_string(),
        custom: true,
        glyphs: 15,
    };
    let working: Vec<HanFontRun> = vec![
        run(&["sans-serif"], vec![font("PingFang SC", 15)]),
        run(&["Songti SC", "serif"], vec![font("Songti SC", 15)]),
        run(
            &["Helvetica Neue", "Songti SC", "sans-serif"],
            vec![font("Songti SC", 15)],
        ),
        run(&["Inter", "sans-serif"], vec![font("PingFang SC", 15)]),
        // A web font's platform name need not match the CSS family.
        run(&["Brand Han"], vec![web_font]),
        // The platform's own fallback chain, not a font the list named.
        run(
            &["system-ui"],
            vec![font("PingFang SC", 13), font("Songti SC", 2)],
        ),
    ];
    for facts in working {
        let (mut dom, body) = page();
        let label = format!("{:?}", facts.declared_families);
        with_fonts(&mut dom, body, vec![facts]);
        assert!(fallback_findings(&dom).is_empty(), "{label}");
    }
}

#[test]
fn font_fallback_judges_each_passage_and_reports_a_shared_outcome_once() {
    let stack = ["Hiragino Mincho ProN", "Songti SC"];
    // Two passages on one stack, each drawn whole by a different font, are
    // not a mix; the third one is, and it alone is reported.
    let (mut dom, body) = page();
    with_fonts(
        &mut dom,
        body,
        vec![run(&stack, vec![font("Hiragino Mincho ProN", 14)])],
    );
    with_fonts(
        &mut dom,
        body,
        vec![run(&stack, vec![font("Songti SC", 14)])],
    );
    let mixed = with_fonts(
        &mut dom,
        body,
        vec![run(
            &stack,
            vec![font("Hiragino Mincho ProN", 9), font("Songti SC", 9)],
        )],
    );
    assert_eq!(
        fallback_findings(&dom),
        vec![(
            Some(mixed),
            "Han text renders in 2 fonts: Hiragino Mincho ProN (9 glyphs), Songti SC (9 glyphs)"
                .to_string()
        )]
    );

    // The same broken font on three passages is one finding on the first.
    let (mut dom, body) = page();
    let broken = || {
        let mut facts = run(
            &["Fixture Han", "sans-serif"],
            vec![font("PingFang SC", 15)],
        );
        facts.failed_web_fonts.push(failed("Fixture Han", 0, false));
        facts
    };
    let first = with_fonts(&mut dom, body, vec![broken()]);
    with_fonts(&mut dom, body, vec![broken()]);
    with_fonts(&mut dom, body, vec![broken()]);
    assert_eq!(
        fallback_findings(&dom),
        vec![(
            Some(first),
            "web font \"Fixture Han\" failed to load; Han text renders in PingFang SC (+2 more elements with this font stack)"
                .to_string()
        )]
    );
}

// ─── missing facts and degenerate input ────────────────────────────────────

#[test]
fn no_measurement_means_no_finding() {
    let (mut dom, body) = page();
    let el = passage(&mut dom, body, "p", BODY);
    dom.set_style(el, "lineHeight", "19.2px");
    // No line rects, no line texts, no font facts: a DOM that measured nothing.
    assert!(cjk_details(&dom, el).is_empty());
    assert!(fallback_findings(&dom).is_empty());

    // Font facts on text the rules do not read are not judged either.
    let aside = passage(&mut dom, body, "div", BODY);
    dom.set_han_fonts(
        aside,
        HanFontFacts {
            runs: vec![run(&["Helvetica Neue"], vec![font("PingFang SC", 15)])],
        },
    );
    assert!(fallback_findings(&dom).is_empty());
}

#[test]
fn zero_sizes_empty_facts_and_parentless_elements_do_not_panic() {
    let (mut dom, body) = page();
    let zero = passage(&mut dom, body, "p", BODY);
    dom.set_style(zero, "fontSize", "0px");
    dom.set_style(zero, "lineHeight", "0px");
    dom.set_text_lines(zero, &[(0.0, 0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 0.0)]);
    dom.set_text_line_texts(zero, &["", ""]);
    dom.set_han_fonts(
        zero,
        HanFontFacts {
            runs: vec![run(&[], Vec::new())],
        },
    );
    assert!(cjk_details(&dom, zero).is_empty());

    let orphaned = dom.add(None, "p");
    dom.add_text(orphaned, BODY);
    dom.set_text_line_texts(orphaned, &[]);
    assert!(cjk_details(&dom, orphaned).is_empty());
    assert!(fallback_findings(&dom).is_empty());
}
