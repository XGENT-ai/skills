//! V-1: the shipped binary installs the XGENT rule pack.
//!
//! The binary is exercised as a user runs it: `detect --json` on the fixture
//! files reports the text rules, the engine's normal waivers still close them
//! (`detector.ignoreRules`, both inline directive prefixes, `--no-advisory`),
//! and every other verb keeps working with the pack installed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/xgent")
        .join(name)
}

fn run(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let output = binary()
        .args(args)
        .current_dir(cwd)
        .env_remove("PHOENIX_UI_HOME")
        .output()
        .expect("run phoenix-ui");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn detect_json(target: &Path, cwd: &Path, extra: &[&str]) -> Vec<Value> {
    let mut args = vec!["detect", "--json"];
    args.extend_from_slice(extra);
    let target = target.to_string_lossy().into_owned();
    args.push(&target);
    let (code, stdout, stderr) = run(&args, cwd);
    // detect exits 2 when a non-advisory finding is reported; the JSON body is
    // still the complete finding list.
    assert!(code == 0 || code == 2, "detect failed ({code}): {stderr}");
    serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("bad JSON: {error}\n{stdout}"))
}

fn xgent(findings: &[Value]) -> Vec<&Value> {
    findings
        .iter()
        .filter(|finding| {
            finding["antipattern"]
                .as_str()
                .unwrap_or("")
                .starts_with("xgent/")
        })
        .collect()
}

fn ids<'a>(findings: &'a [&'a Value]) -> Vec<(&'a str, f64)> {
    findings
        .iter()
        .map(|finding| {
            (
                finding["antipattern"].as_str().unwrap_or(""),
                finding["line"].as_f64().unwrap_or(-1.0),
            )
        })
        .collect()
}

struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!(
            "phoenix-xgent-{}-{}-{}",
            tag,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }
    fn write(&self, rel: &str, body: &str) -> PathBuf {
        let file = self.0.join(rel);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, body).unwrap();
        file
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn detect_reports_the_transition_rule() {
    let target = fixture("transition-all.css");
    let findings = detect_json(&target, target.parent().unwrap(), &[]);
    let hits = xgent(&findings);
    assert_eq!(
        ids(&hits),
        vec![
            ("xgent/transition-all", 3.0),
            ("xgent/transition-all", 7.0),
            ("xgent/transition-all", 11.0)
        ]
    );
    assert_eq!(
        hits[0]["snippet"].as_str().unwrap(),
        "transition: all 0.2s ease;"
    );
    assert_eq!(hits[0]["severity"].as_str().unwrap(), "advisory");
    assert_eq!(hits[0]["advisory"].as_bool(), Some(true));
    assert_eq!(hits[0]["category"].as_str().unwrap(), "quality");
    assert_eq!(
        hits[0]["name"].as_str().unwrap(),
        "Transition on all properties"
    );
}

#[test]
fn detect_reports_the_focus_rule() {
    let css = fixture("focus-outline.css");
    let findings = detect_json(&css, css.parent().unwrap(), &[]);
    let hits = xgent(&findings);
    assert_eq!(
        ids(&hits),
        vec![
            ("xgent/focus-outline-removed", 1.0),
            ("xgent/focus-outline-removed", 5.0),
        ]
    );
    assert_eq!(hits[0]["snippet"].as_str().unwrap(), ".btn:focus");
    assert_eq!(hits[0]["severity"].as_str().unwrap(), "warning");
    assert!(hits[0].get("advisory").is_none());

    let tsx = fixture("focus-outline.tsx");
    let findings = detect_json(&tsx, tsx.parent().unwrap(), &[]);
    let hits = xgent(&findings);
    assert_eq!(
        ids(&hits),
        vec![
            ("xgent/focus-outline-removed", 4.0),
            ("xgent/focus-outline-removed", 5.0),
            ("xgent/focus-outline-removed", 6.0),
        ]
    );
}

#[test]
fn clean_fixtures_produce_no_xgent_findings() {
    for name in [
        "transition-all.clean.css",
        "focus-outline.clean.css",
        "focus-outline.clean.tsx",
    ] {
        let target = fixture(name);
        let findings = detect_json(&target, target.parent().unwrap(), &[]);
        assert!(
            xgent(&findings).is_empty(),
            "{name}: {:#?}",
            xgent(&findings)
        );
    }
}

#[test]
fn directory_scan_contains_both_text_rules() {
    let dir = fixture(".");
    let findings = detect_json(&dir, &dir, &[]);
    let ids: Vec<&str> = xgent(&findings)
        .iter()
        .map(|finding| finding["antipattern"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"xgent/transition-all"), "{ids:?}");
    assert!(ids.contains(&"xgent/focus-outline-removed"), "{ids:?}");
}

#[test]
fn ignore_rules_config_disables_a_rule() {
    let temp = Temp::new("ignore-rules");
    temp.write(
        ".phoenix-ui/config.json",
        r#"{"detector":{"ignoreRules":["xgent/transition-all"]}}"#,
    );
    let file = temp.write("src/a.css", ".a { transition: all 0.2s ease; }\n");
    let findings = detect_json(&file, temp.path(), &[]);
    assert!(xgent(&findings).is_empty(), "{:#?}", xgent(&findings));

    // The same file without the ignored id still reports.
    temp.write(".phoenix-ui/config.json", "{}");
    let findings = detect_json(&file, temp.path(), &[]);
    assert_eq!(xgent(&findings).len(), 1);
}

#[test]
fn inline_directives_cover_both_prefixes() {
    let temp = Temp::new("inline");
    let file = temp.write(
        "src/a.css",
        "\
/* impeccable-disable-next-line xgent/transition-all */
.a { transition: all 0.2s ease; }
/* phoenix-ui-disable-next-line xgent/transition-all */
.b { transition: all 0.2s ease; }
.c { transition: all 0.2s ease; }
",
    );
    let findings = detect_json(&file, temp.path(), &[]);
    assert_eq!(ids(&xgent(&findings)), vec![("xgent/transition-all", 5.0)]);

    let file_all = temp.write(
        "src/all.css",
        "/* PHOENIX-UI-DISABLE */\n.a { transition: all 0.2s ease; }\n",
    );
    let findings = detect_json(&file_all, temp.path(), &[]);
    assert!(xgent(&findings).is_empty());
}

#[test]
fn no_advisory_hides_the_advisory_rule_only() {
    let transition = fixture("transition-all.css");
    let findings = detect_json(
        &transition,
        transition.parent().unwrap(),
        &["--no-advisory"],
    );
    assert!(xgent(&findings).is_empty(), "{:#?}", xgent(&findings));

    let focus = fixture("focus-outline.css");
    let findings = detect_json(&focus, focus.parent().unwrap(), &["--no-advisory"]);
    assert_eq!(xgent(&findings).len(), 2);
}

#[test]
fn install_keeps_the_other_verbs_working() {
    let cwd = std::env::temp_dir();
    let (code, stdout, stderr) = run(&["--version"], &cwd);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stdout.trim().is_empty());

    let (code, stdout, stderr) = run(&["detect", "--help"], &cwd);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("detect"), "{stdout}");
}

fn detect_json_url(url: &str, cwd: &Path, extra: &[&str]) -> Vec<Value> {
    let mut args = vec!["detect", "--json"];
    args.extend_from_slice(extra);
    args.push(url);
    let (code, stdout, stderr) = run(&args, cwd);
    assert!(code == 0 || code == 2, "detect failed ({code}): {stderr}");
    serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("bad JSON: {error}\n{stdout}"))
}

/// V-4: the single expectation table for the page fixtures, checked at both
/// widths. A fixture's findings must not depend on the viewport unless the
/// table says so; keeping the table here makes it the one source the browser
/// evidence is compared against.
const URL_EXPECTATIONS: &[(&str, &[&str])] = &[
    ("controls.html", &["xgent/control-height-mismatch"]),
    ("svg.html", &["xgent/unlabeled-inline-svg"]),
    ("chrome.html", &["xgent/redrawn-window-chrome"]),
    ("chrome-oklch.html", &["xgent/redrawn-window-chrome"]),
    (
        "emoji-icons.html",
        &["xgent/emoji-icon", "xgent/emoji-icon"],
    ),
    ("fonts.html", &["xgent/font-family-count"]),
    ("clean.html", &[]),
];

#[test]
fn url_scan_matches_the_fixture_expectations() {
    let env = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: browser unavailable");
        return;
    }
    let dir = fixture("");
    for (file, expected) in URL_EXPECTATIONS {
        let url = format!("file://{}", dir.join(file).display());
        for viewport in ["375x812", "1280x800"] {
            let findings = detect_json_url(&url, &dir, &["--viewport", viewport]);
            let actual: Vec<&str> = xgent(&findings)
                .iter()
                .map(|finding| finding["antipattern"].as_str().unwrap())
                .collect();
            assert_eq!(expected.to_vec(), actual, "{file} at {viewport}");
        }
    }
}
