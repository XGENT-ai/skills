//! V-2: the two `xgent/` text rules through the public rule-pack hook.
//!
//! Positives pin the line number and snippet the engine reports; every
//! counterexample the plan names must produce zero findings; malformed input
//! must not panic (the release binary aborts on panic).

use impeccable_core::findings::Finding;
use impeccable_core::rule_pack::RulePack;
use phoenix_ui_rules::PACK;

fn scan(content: &str, ext: &str) -> Vec<Finding> {
    PACK.check_text(content, &format!("/app/example{ext}"), ext)
}

fn ids(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.antipattern.as_str())
        .collect()
}

fn lines(findings: &[Finding]) -> Vec<f64> {
    findings.iter().map(|finding| finding.line).collect()
}

fn snippets(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.snippet.as_str())
        .collect()
}

#[test]
fn transition_all_reports_css_declarations_and_tailwind_classes() {
    let content = "\
/* transition: all inside a comment is not a finding */
.card {
  transition: all 0.2s ease;
}
.grid { transition-property: all; }
* {
  transition: all 0.2s;
}
.chip { transition: opacity 0.2s ease; }
.fade { transition: transform 200ms ease, color 100ms linear; }
const classes = \"transition-all\";
const more = \"md:transition-all\";
const less = \"transition transition-colors\";
";
    let findings = scan(content, ".css");
    assert_eq!(
        ids(&findings),
        vec!["xgent/transition-all"; 5],
        "{findings:#?}"
    );
    assert_eq!(lines(&findings), vec![3.0, 5.0, 7.0, 11.0, 12.0]);
    assert_eq!(
        snippets(&findings),
        vec![
            "transition: all 0.2s ease;",
            ".grid { transition-property: all; }",
            "transition: all 0.2s;",
            "const classes = \"transition-all\";",
            "const more = \"md:transition-all\";",
        ]
    );
    assert_eq!(findings[0].severity, "advisory");
    assert_eq!(findings[0].advisory, Some(true));
    assert_eq!(findings[0].category.as_deref(), Some("quality"));
    assert_eq!(findings[0].name, "Transition on all properties");
    assert_eq!(findings[0].file, "/app/example.css");
}

#[test]
fn transition_all_ignores_non_hits() {
    let content = "\
/* transition: all 0.2s is fine in a comment */
.card { transition: opacity 0.2s ease; }
.chip { transition: transform 200ms ease, color 100ms linear; }
.panel { transition-property: opacity, transform; }
.btn { transition: color .2s; }
.code { --transition-all: 1; }
";
    assert!(
        scan(content, ".css").is_empty(),
        "{:#?}",
        scan(content, ".css")
    );
}

#[test]
fn transition_all_skips_js_line_comments_only() {
    let content = "\
// transition: all 0.2s is ignored in a line comment
const cls = \"transition-all\";
const ok = \"transition-colors\";
";
    let findings = scan(content, ".tsx");
    assert_eq!(lines(&findings), vec![2.0]);
}

#[test]
fn focus_outline_removed_reports_unreplaced_css_focus_rules() {
    let content = "\
.btn:focus {
  outline: none;
}

.field:focus-within .control:focus {
  outline: 0;
}

.tab:focus { outline-style: none; padding: 4px; }

.keep:focus { outline: none; box-shadow: 0 0 0 2px var(--ring); }

.only-within:focus-within { outline: none; }
";
    let findings = scan(content, ".css");
    assert_eq!(
        ids(&findings),
        vec!["xgent/focus-outline-removed"; 3],
        "{findings:#?}"
    );
    assert_eq!(lines(&findings), vec![1.0, 5.0, 9.0]);
    assert_eq!(
        snippets(&findings),
        vec![
            ".btn:focus",
            ".field:focus-within .control:focus",
            ".tab:focus"
        ]
    );
    assert_eq!(findings[0].severity, "warning");
    assert_eq!(findings[0].advisory, None);
    assert_eq!(
        findings[0].name,
        "Focus outline removed without replacement"
    );
}

#[test]
fn focus_outline_removed_accepts_written_replacements() {
    let content = "\
.a:focus { outline: none; box-shadow: 0 0 0 2px red; }
.b:focus { outline: none; border: 1px solid red; }
.c:focus { outline: none; background: white; }
.d:focus { outline: none; text-decoration: underline; }
.e:focus-within { outline: none; }
button { outline: none; }
";
    assert!(
        scan(content, ".css").is_empty(),
        "{:#?}",
        scan(content, ".css")
    );

    // Any `:focus-visible` elsewhere in the file turns the style branch off:
    // the alternative is written, even if it lives in another block.
    let file_level = "\
.x:focus { outline: none; }
.y:focus-visible { outline: 2px solid currentColor; }
";
    assert!(scan(file_level, ".css").is_empty());
}

#[test]
fn focus_outline_removed_reports_tailwind_classes() {
    let content = "\
const a = <button className=\"focus:outline-none\">Save</button>;
const b = <a className=\"hover:underline focus-visible:outline-none\">Docs</a>;
const c = <input className={`md:focus:outline-none`} />;
";
    let findings = scan(content, ".tsx");
    assert_eq!(
        ids(&findings),
        vec!["xgent/focus-outline-removed"; 3],
        "{findings:#?}"
    );
    assert_eq!(lines(&findings), vec![1.0, 2.0, 3.0]);
    assert_eq!(
        snippets(&findings),
        vec![
            "focus:outline-none",
            "hover:underline focus-visible:outline-none",
            "md:focus:outline-none",
        ]
    );
}

#[test]
fn focus_outline_removed_reads_literals_inside_grouped_jsx() {
    // A `return (…)` grouping parenthesis is not an argument list: the literal
    // is still a complete class string, not an unreadable expression.
    let content = "\
export function Actions() {\n  return (\n    <button className=\"focus:outline-none\">Save</button>\n  );\n}\n";
    assert_eq!(lines(&scan(content, ".tsx")), vec![3.0]);
}

#[test]
fn focus_outline_removed_accepts_any_written_focus_replacement() {
    let content = "\
const a = <button className=\"focus-visible:outline-none focus-visible:ring-2\">A</button>;
const b = <button className=\"focus:outline-none dark:focus:border-white\">B</button>;
const c = <div className=\"md:focus-visible:outline-2\" />;
const d = <div className=\"[&:focus-visible]:outline-2\" />;
const e = <div className=\"data-[focus]:outline-2\" />;
const f = <div className=\"outline-none\" />;
const g = <div className=\"focus:not(:focus-visible)\" />;
";
    assert!(
        scan(content, ".tsx").is_empty(),
        "{:#?}",
        scan(content, ".tsx")
    );
}

#[test]
fn focus_outline_removed_ignores_comment_examples() {
    // TSX line, block, and JSX comments are not class strings.
    let tsx = "\
// <button className=\"focus:outline-none\">Save</button>
/* <a className=\"focus-visible:outline-none\">Docs</a> */
{/* <input className={`md:focus:outline-none`} /> */}
const real = <button className=\"focus:outline-none\">Save</button>;
";
    let findings = scan(tsx, ".tsx");
    assert_eq!(lines(&findings), vec![4.0], "{findings:#?}");
    assert_eq!(snippets(&findings), vec!["focus:outline-none"]);

    // HTML comments are not class strings either.
    let html = "\
<!-- <button class=\"focus:outline-none\">Save</button> -->
<button class=\"focus:outline-none\">Save</button>
";
    let findings = scan(html, ".html");
    assert_eq!(lines(&findings), vec![2.0], "{findings:#?}");

    // `//` inside a string literal is part of the URL, not a line comment:
    // the class string after it on the same line still counts.
    let url = "const docs = \"https://example.com\"; const save = \"focus:outline-none\";\n";
    assert_eq!(lines(&scan(url, ".tsx")), vec![1.0]);

    // A commented-out CSS block is not a finding; the live one is.
    let css = "/* .btn:focus { outline: none; } */\n.btn:focus { outline: none; }\n";
    assert_eq!(lines(&scan(css, ".css")), vec![2.0]);
}

#[test]
fn focus_outline_removed_distinguishes_focus_variants_from_lookalikes() {
    let content = "\
const a = <div className=\"focus-within:outline-none\">A</div>;
const b = <div className=\"not-focus:outline-none\">B</div>;
const c = <div className=\"group-focus:outline-none\">C</div>;
const d = <div className=\"hover:outline-none\">D</div>;
const e = <div className=\"focus:outline-none\">E</div>;
const f = <div className=\"focus-visible:outline-none\">F</div>;
const g = <div className=\"[&:focus]:outline-none\">G</div>;
const h = <div className=\"[&:focus-visible]:outline-none\">H</div>;
const i = <div className=\"data-[focus]:outline-none\">I</div>;
const j = <div className=\"focus-within:outline-none focus:outline-none\">J</div>;
const k = <div className=\"focus:outline-none focus-within:border-white\">K</div>;
";
    let findings = scan(content, ".tsx");
    assert_eq!(
        lines(&findings),
        vec![5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0],
        "{findings:#?}"
    );
    assert_eq!(
        snippets(&findings),
        vec![
            "focus:outline-none",
            "focus-visible:outline-none",
            "[&:focus]:outline-none",
            "[&:focus-visible]:outline-none",
            "data-[focus]:outline-none",
            "focus-within:outline-none focus:outline-none",
            "focus:outline-none focus-within:border-white",
        ]
    );
}

#[test]
fn focus_outline_removed_ignores_negated_arbitrary_focus_conditions() {
    // A focus pseudo-class inside `:not(…)` applies when focus is absent;
    // only positive arbitrary focus conditions count as removed focus.
    let content = "\
const a = <div className=\"[&:focus]:outline-none\">A</div>;
const b = <div className=\"[&:focus-visible]:outline-none\">B</div>;
const c = <div className=\"[&:not(:focus)]:outline-none\">C</div>;
const d = <div className=\"[&:not(:focus-visible)]:outline-none\">D</div>;
const e = <div className=\"[&:not(:is(:focus))]:outline-none\">E</div>;
const f = <div className=\"[&:not(:focus):hover]:outline-none\">F</div>;
const g = <div className=\"[&:not(:hover):focus]:outline-none\">G</div>;
const h = <div className=\"focus-within:outline-none\">H</div>;
const i = <div className=\"not-focus:outline-none\">I</div>;
";
    let findings = scan(content, ".tsx");
    assert_eq!(lines(&findings), vec![1.0, 2.0, 7.0], "{findings:#?}");
    assert_eq!(
        snippets(&findings),
        vec![
            "[&:focus]:outline-none",
            "[&:focus-visible]:outline-none",
            "[&:not(:hover):focus]:outline-none",
        ]
    );
}

#[test]
fn focus_outline_removed_evaluates_static_class_expressions_as_one_string() {
    let content = "\
const single = <button className=\"focus:outline-none\">A</button>;
const concat = <button className={\"focus:outline-none \" + \"focus-visible:ring-2\"}>B</button>;
const dynamic = <button className={\"focus:outline-none \" + extra}>C</button>;
const dynamic_left = <button className={extra + \" focus:outline-none\"}>D</button>;
const parenthesized = <button className={\"focus:outline-none \" + (open ? \"ring-2\" : \"\")}>E</button>;
const static_removal = <button className={\"focus:outline-none \" + \"\"}>F</button>;
const template_dynamic = <button className={`focus:outline-none ${extra}`}>G</button>;
const bracket_concat = <button className={(\"focus:outline-none \") + (\"focus-visible:ring-2\")}>H</button>;
const array_join = <button className={[\"focus:outline-none\", \"focus-visible:ring-2\"].join(\" \")}>I</button>;
const call_join = <button className={cn(\"focus:outline-none\", \"focus-visible:ring-2\")}>J</button>;
const nested_join = <button className={cn([\"focus:outline-none\", \"focus-visible:ring-2\"].join(\" \"), \"text-sm\")}>K</button>;
const bracket_missing = <button className={(\"focus:outline-none \") + (\"text-sm\")}>L</button>;
const join_missing = <button className={[\"focus:outline-none\", \"mt-2\"].join(\" \")}>M</button>;
const call_missing = <button className={cn(\"focus:outline-none\", \"mt-2\")}>N</button>;
const call_dynamic_arg = <button className={cn(\"focus:outline-none\", extra)}>O</button>;
const join_dynamic_element = <button className={[\"focus:outline-none\", ...rest].join(\" \")}>P</button>;
const join_dynamic_separator = <button className={[\"focus:outline-none\", \"ring-2\"].join(sep)}>Q</button>;
";
    let findings = scan(content, ".tsx");
    assert_eq!(
        lines(&findings),
        vec![1.0, 6.0, 12.0, 13.0, 14.0],
        "{findings:#?}"
    );
    assert_eq!(
        snippets(&findings),
        vec![
            "focus:outline-none",
            "focus:outline-none",
            "focus:outline-none text-sm",
            "focus:outline-none mt-2",
            "focus:outline-none mt-2",
        ]
    );
}

#[test]
fn focus_outline_removed_reads_the_complete_class_expression() {
    // The class list is the outermost static expression a literal belongs to,
    // not the first group that parses on its own. Anything the engine cannot
    // read around it — a variable, a sibling condition, a member call, an
    // interpolation, a call that gives the literal another meaning — makes the
    // list unknown, and an unknown list is never reported (R3 F-03).
    let content = "\
const cn = (...parts: string[]) => parts.join(\" \");
const safe = <button className={cn((\"focus:outline-none\"), cn(\"focus-visible:ring-2\"))}>A</button>;
const missing = <button className={cn((\"focus:outline-none\"), cn(\"mt-2\"))}>B</button>;
const grouped_dynamic = <button className={(\"focus:outline-none \") + extra}>C</button>;
const nested_dynamic = <button className={cn((\"focus:outline-none\"), extra)}>D</button>;
const branch_sibling = <button className={cn((\"focus:outline-none\"), open && \"focus-visible:ring-2\")}>E</button>;
const conditional_sibling = <button className={cn(base, open ? \"focus:outline-none\" : \"\")}>F</button>;
const conditional = <button className={open ? \"focus:outline-none\" : \"focus-visible:ring-2\"}>G</button>;
const conditional_call = <button className={cn(open ? \"focus:outline-none\" : \"\")}>H</button>;
const object_key = <button className={clsx(\"focus-visible:ring-2\", { \"focus:outline-none\": active })}>I</button>;
const variant = cva(\"focus-visible:ring-2\", { variants: { tone: { plain: \"focus:outline-none\" } } });
const styles = { plain: \"focus:outline-none\" };
const transformed = <button className={\"focus:outline-none\".trim()}>J</button>;
const interpolated = <button className={`focus-visible:ring-2 ${open ? \"focus:outline-none\" : \"\"}`}>K</button>;
";
    let findings = scan(content, ".tsx");
    assert_eq!(lines(&findings), vec![3.0, 8.0, 9.0, 12.0], "{findings:#?}");
    assert_eq!(
        snippets(&findings),
        vec![
            "focus:outline-none mt-2",
            "focus:outline-none",
            "focus:outline-none",
            "focus:outline-none",
        ]
    );
}

#[test]
fn focus_outline_removed_survives_apostrophes_and_markup_neighbors() {
    // An apostrophe in body copy is not a string delimiter for the class
    // literal after it, and a spread or a framework attribute after the class
    // attribute ends the expression rather than making it unknown.
    let tsx = "\
export const a = <div><p>Don't</p><button className=\"focus:outline-none\">A</button></div>;
export const b = <button className=\"focus:outline-none\" {...rest}>B</button>;
export const c = 'z';
";
    let findings = scan(tsx, ".tsx");
    assert_eq!(lines(&findings), vec![1.0, 2.0], "{findings:#?}");

    let vue = "\
<template>
  <button class=\"focus:outline-none\" :disabled=\"busy\">A</button>
  <button class=\"focus:outline-none\" @click=\"go\">B</button>
  <button :class=\"'focus:outline-none'\">C</button>
  <button :class=\"['focus:outline-none', 'focus-visible:ring-2']\">D</button>
</template>
";
    let findings = scan(vue, ".vue");
    assert_eq!(lines(&findings), vec![2.0, 3.0, 4.0], "{findings:#?}");
}

#[test]
fn focus_outline_removed_scans_large_markup_in_linear_time() {
    // Every attribute literal in one large JSX return group is its own class
    // list; resolving each must not re-read the whole group (R3 F-12).
    let mut content = String::from("export function App(){ return (<>\n");
    for index in 0..6_000 {
        content.push_str(&format!(
            "<button className=\"px-2 py-1\" title=\"Save item {index}\">Save</button>\n"
        ));
    }
    content.push_str("</>); }\n");
    // Prose that quotes a value after a colon reads like a conditional branch;
    // looking for its bound must not scan the rest of the page.
    let mut prose = String::new();
    for index in 0..6_000 {
        prose.push_str(&format!(
            "<p>Step {index}: \"Save item {index}\" and continue</p>\n"
        ));
    }
    for (content, ext) in [(&content, ".tsx"), (&prose, ".html")] {
        let started = std::time::Instant::now();
        let findings = scan(content, ext);
        let elapsed = started.elapsed();
        assert!(findings.is_empty(), "{ext}: {findings:#?}");
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "{ext}: scanning {} bytes took {elapsed:?}",
            content.len()
        );
    }
}

#[test]
fn focus_outline_removed_reads_priority_suffix_after_the_value() {
    // `!important` is a declaration priority, not part of the value: the
    // compact form removes the outline exactly like the spaced one (F-11).
    let content = "\
.a:focus{outline:none!important}
.b:focus{outline:0!important}
.c:focus{outline-width:0!important}
.d:focus{outline-style:none!important}
.e:focus{outline:none ! important}
.f:focus{outline:0.5px solid black!important}
.g:focus{outline-width:0.5px!important}
.h:focus{outline:none!important;box-shadow:0 0 0 2px red}
.i:focus{outline-color:red!important}
";
    let findings = scan(content, ".css");
    assert_eq!(
        lines(&findings),
        vec![1.0, 2.0, 3.0, 4.0, 5.0],
        "{findings:#?}"
    );
    assert_eq!(
        snippets(&findings),
        vec![".a:focus", ".b:focus", ".c:focus", ".d:focus", ".e:focus"]
    );
}

#[test]
fn focus_outline_removed_matches_complete_zero_length_values() {
    let content = "\
.a:focus { outline: 0; }
.b:focus { outline: 0px; }
.c:focus { outline: 0rem; }
.d:focus { outline: 0.0; }
.e:focus { outline: 0.00px; }
.f:focus { outline-width: 0px; }
.g:focus { outline: 0.5px solid black; }
.h:focus { outline-width: 0.5px; }
.i:focus { outline: none; }
";
    let findings = scan(content, ".css");
    assert_eq!(
        lines(&findings),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 9.0],
        "{findings:#?}"
    );
    assert_eq!(
        snippets(&findings),
        vec![".a:focus", ".b:focus", ".c:focus", ".d:focus", ".e:focus", ".f:focus", ".i:focus"]
    );
}

#[test]
fn focus_outline_removed_ignores_color_function_zeros() {
    // `0` inside a color function is a color channel, not a zero width; the
    // shorthand's components are split on top-level whitespace only.
    let content = "\
.a:focus { outline: 1px solid rgba(0,0,0,0.5); }
.b:focus { outline: 2px solid oklch(0 0 0); }
.c:focus { outline: 0px solid black; }
";
    let findings = scan(content, ".css");
    assert_eq!(lines(&findings), vec![3.0], "{findings:#?}");
    assert_eq!(snippets(&findings), vec![".c:focus"]);
}

#[test]
fn transition_all_scans_only_css_positions_in_markup() {
    // Body copy and comments quote the declaration; only the <style> block,
    // the style attribute values, and the Tailwind class are real CSS.
    let content = "\
<!doctype html>
<html>
<head>
<style>
  .card { transition: all 0.2s ease; }
</style>
</head>
<body>
  <!-- <div class=\"transition-all\">example</div> -->
  <h4>transition: all</h4>
  <p>Never write transition: all in prose.</p>
  <pre><code>&lt;div style=\"transition: all .2s\"&gt;Example&lt;/div&gt;</code></pre>
  <p title=\"a &gt; b\">Escaped angle</p>
  <div title=\"a > b\" style=\"transition: all 0.2s ease\">Real attribute</div>
  <script>const tpl = '<div style=\"transition: all\">';</script>
  <div style=\"transition: all 0.2s ease\">Animated</div>
  <div style='transition: all 0.2s ease'>Single-quoted</div>
  <div class=\"transition-all\">Tailwind</div>
</body>
</html>
";
    for ext in [".html", ".htm", ".vue", ".svelte", ".astro"] {
        let findings = scan(content, ext);
        assert_eq!(
            lines(&findings),
            vec![5.0, 14.0, 16.0, 17.0, 18.0],
            "{ext}: {findings:#?}"
        );
        assert_eq!(
            snippets(&findings),
            vec![
                ".card { transition: all 0.2s ease; }",
                "<div title=\"a > b\" style=\"transition: all 0.2s ease\">Real attribute</div>",
                "<div style=\"transition: all 0.2s ease\">Animated</div>",
                "<div style='transition: all 0.2s ease'>Single-quoted</div>",
                "<div class=\"transition-all\">Tailwind</div>",
            ],
            "{ext}"
        );
    }
}

#[test]
fn transition_all_reads_markup_by_attribute_and_raw_text_context() {
    // A style example inside another attribute's value, a `<style>` string in
    // a script, and tags quoted in raw-text elements are not declarations;
    // bound style attributes, real style attributes after a brace value that
    // contains `>`, and real style blocks are (R3 F-08).
    let content = "\
<p title='Avoid style=\"transition: all .2s\"'>Guide</p>
<p data-example='style=\"transition: all .2s\"'>Guide</p>
<script>const tpl = \"<style>.x { transition: all .2s; }</style>\";</script>
<textarea><div style=\"transition: all .2s\">Example</div></textarea>
<title>Avoid <b style=\"transition: all .2s\"></title>
<div :style=\"{ transition: 'all .2s' }\">Bound</div>
<div class={a > b ? 'x' : 'y'} style=\"transition: all .2s\">Braced</div>
<style>.y { transition: all .2s; }</style>
";
    for ext in [".html", ".htm", ".vue", ".svelte", ".astro"] {
        let findings = scan(content, ext);
        assert_eq!(
            lines(&findings),
            vec![6.0, 7.0, 8.0],
            "{ext}: {findings:#?}"
        );
    }
    // An unclosed `<style>` is not a style block for the rest of the file.
    let unclosed =
        "<div style=\"transition: all .2s\">A</div>\n<style>.b { transition: all .2s }\n";
    assert_eq!(lines(&scan(unclosed, ".html")), vec![1.0]);
}

#[test]
fn findings_are_emitted_in_rule_then_line_order() {
    let content = ".a { transition: all .2s; }\n.b:focus { outline: none; }\n";
    let findings = scan(content, ".css");
    assert_eq!(
        ids(&findings),
        vec!["xgent/transition-all", "xgent/focus-outline-removed"]
    );
    assert_eq!(lines(&findings), vec![1.0, 2.0]);
}

#[test]
fn other_extensions_and_malformed_input_are_safe() {
    assert!(scan("transition: all 0.2s;\n", ".md").is_empty());
    assert!(scan("transition: all 0.2s;\n", "").is_empty());
    assert!(scan("", ".css").is_empty());
    assert!(scan("{{{{{{}}}}}}", ".css").is_empty());
    assert!(scan("/* unterminated transition: all", ".css").is_empty());
    assert!(scan("a:focus { outline: none;", ".css").is_empty());
    assert!(scan(&"-".repeat(10_000), ".tsx").is_empty());
    assert!(scan(&"focus:".repeat(5_000), ".css").is_empty());

    let crlf = "\r\n.card:focus {\r\n  outline: none;\r\n}\r\n";
    let findings = scan(crlf, ".css");
    assert_eq!(lines(&findings), vec![2.0]);
    assert_eq!(snippets(&findings), vec![".card:focus"]);

    // A closed multi-line block is reported at its selector line.
    let multiline = "a:focus {\n  outline: none;\n}\n";
    let findings = scan(multiline, ".css");
    assert_eq!(lines(&findings), vec![1.0]);
}

#[test]
fn registry_rows_match_the_reported_findings() {
    let rows = PACK.registry();
    let registered: Vec<&str> = rows.iter().map(|row| row.id).collect();
    assert_eq!(
        registered,
        vec![
            "xgent/transition-all",
            "xgent/focus-outline-removed",
            "xgent/wrapped-control-label",
            "xgent/control-height-mismatch",
            "xgent/unlabeled-inline-svg",
            "xgent/redrawn-window-chrome",
            "xgent/emoji-icon",
            "xgent/font-family-count",
        ]
    );
    for finding in scan("transition: all;\na:focus { outline: none; }\n", ".css") {
        let row = rows
            .iter()
            .find(|row| row.id == finding.antipattern)
            .expect("finding id resolves to a pack row");
        assert_eq!(finding.name, row.name);
        assert_eq!(finding.description, row.description);
    }
}
