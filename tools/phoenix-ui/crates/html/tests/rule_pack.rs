//! The static HTML engine runs the rule pack's text hook over HTML files after
//! every built-in pass, and a run with no pack is identical to the built-in
//! output.

use impeccable_core::findings::{finding_for, Finding};
use impeccable_core::registry::Antipattern;
use impeccable_core::rule_pack::RulePack;
use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::Path;

const MARKER: &str = "TODO(pack)";

static ROWS: &[Antipattern] = &[Antipattern {
    id: "testpack/todo-marker",
    category: "quality",
    scopes: None,
    severity: Some("warning"),
    name: "Unfinished copy marker",
    description: "Text still carries a TODO marker from drafting.",
    skill_section: None,
    skill_guideline: None,
}];

#[derive(Debug)]
struct TestPack;
static PACK: TestPack = TestPack;

impl RulePack for TestPack {
    fn registry(&self) -> &'static [Antipattern] {
        ROWS
    }
    fn check_text(&self, content: &str, file_path: &str, _ext: &str) -> Vec<Finding> {
        content
            .split('\n')
            .enumerate()
            .filter(|(_, line)| line.contains(MARKER))
            .map(|(i, line)| finding_for(&ROWS[0], file_path, line.trim(), (i + 1) as f64))
            .collect()
    }
}

/// A full page with one built-in finding (the side-tab stripe) plus a line
/// the pack is about.
const PAGE: &str = r#"<!DOCTYPE html>
<html><head><title>t</title><style>
.card { border-left: 4px solid #6366f1; background: #fff; }
</style></head>
<body>
<div class="card">A card</div>
<p>TODO(pack) write the real headline</p>
</body></html>
"#;

fn scan(options: &DetectHtmlOptions<'_>) -> Vec<Finding> {
    detect_html_source(PAGE, Path::new("/app/index.html"), options)
}

#[test]
fn text_hook_appends_after_built_ins() {
    let built_in = scan(&DetectHtmlOptions::default());
    assert!(
        !built_in.is_empty(),
        "the fixture must trip a built-in rule"
    );

    impeccable_core::rule_pack::install(&PACK);
    let with_pack = scan(&DetectHtmlOptions {
        rule_pack: Some(&PACK),
        ..Default::default()
    });
    assert_eq!(&with_pack[..built_in.len()], &built_in[..]);
    let pack_findings = &with_pack[built_in.len()..];
    assert_eq!(pack_findings.len(), 1);
    assert_eq!(pack_findings[0].antipattern, "testpack/todo-marker");
    // The text hook reports the source line.
    assert_eq!(pack_findings[0].line, 7.0);
    assert_eq!(
        pack_findings[0].snippet,
        "<p>TODO(pack) write the real headline</p>"
    );
}

#[test]
fn pack_findings_are_waivable_inline() {
    impeccable_core::rule_pack::install(&PACK);
    let page = PAGE.replace(
        "<body>",
        "<body>\n<!-- impeccable-disable testpack/todo-marker -->",
    );
    let findings = detect_html_source(
        &page,
        Path::new("/app/index.html"),
        &DetectHtmlOptions {
            rule_pack: Some(&PACK),
            ..Default::default()
        },
    );
    assert!(findings
        .iter()
        .all(|f| !f.antipattern.starts_with("testpack/")));
}
