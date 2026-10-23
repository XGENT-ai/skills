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
";
    let findings = scan(content, ".tsx");
    assert_eq!(lines(&findings), vec![1.0, 6.0], "{findings:#?}");
    assert_eq!(
        snippets(&findings),
        vec!["focus:outline-none", "focus:outline-none"]
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
            vec![5.0, 12.0, 13.0, 14.0],
            "{ext}: {findings:#?}"
        );
        assert_eq!(
            snippets(&findings),
            vec![
                ".card { transition: all 0.2s ease; }",
                "<div style=\"transition: all 0.2s ease\">Animated</div>",
                "<div style='transition: all 0.2s ease'>Single-quoted</div>",
                "<div class=\"transition-all\">Tailwind</div>",
            ],
            "{ext}"
        );
    }
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
