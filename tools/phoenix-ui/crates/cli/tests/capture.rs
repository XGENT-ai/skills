//! `phoenix-ui capture` as a caller runs it: real Chrome, real pages, the
//! report and files it leaves behind. Browser-free checks (argument errors)
//! run everywhere; the rest skip when no browser is installed.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

fn pages() -> PathBuf {
    fixtures().join("capture/pages")
}

fn browser_available() -> bool {
    let env = std::env::vars().collect();
    let ok = impeccable_browser::discovery::find_browser(&env).is_ok();
    if !ok {
        eprintln!("skip: browser unavailable");
    }
    ok
}

struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!(
            "phoenix-capture-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn write(&self, name: &str, body: &str) -> PathBuf {
        let file = self.0.join(name);
        std::fs::write(&file, body).unwrap();
        file
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
    pid: u32,
}

fn phoenix(args: &[&str], cwd: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"));
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("PHOENIX_UI_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        command.env(k, v);
    }
    let child = command.spawn().expect("run phoenix-ui");
    let pid = child.id();
    let output = child.wait_with_output().unwrap();
    Output {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        pid,
    }
}

fn report(out: &Path) -> Value {
    let text = std::fs::read_to_string(out.join("capture-report.json")).expect("report written");
    serde_json::from_str(&text).unwrap()
}

fn shot(report: &Value, file: &str) -> Value {
    report["shots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["file"] == file)
        .cloned()
        .unwrap_or_else(|| panic!("no shot {file} in {report:#}"))
}

fn failure<'a>(report: &'a Value, state: &str) -> &'a Value {
    report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["state"] == state)
        .unwrap_or_else(|| panic!("no failure {state} in {report:#}"))
}

/// `tests/fixtures/capture/serve.mjs` over a directory; non-GET requests it
/// received are collected from its stdout.
struct Served {
    child: Child,
    url: String,
    writes: Arc<Mutex<Vec<String>>>,
}

impl Served {
    fn start(root: &Path) -> Served {
        let mut child = Command::new("node")
            .arg(fixtures().join("capture/serve.mjs"))
            .arg(root)
            .stdout(Stdio::piped())
            .spawn()
            .expect("node runs the fixture server");
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let first: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        let url = first["url"].as_str().unwrap().to_string();
        let writes = Arc::new(Mutex::new(Vec::new()));
        let sink = writes.clone();
        std::thread::spawn(move || {
            for line in lines.map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    sink.lock()
                        .unwrap()
                        .push(v["path"].as_str().unwrap_or("").to_string());
                }
            }
        });
        Served { child, url, writes }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn detect_json(target: &str, cwd: &Path, viewport: &str) -> Value {
    let out = phoenix(
        &["detect", "--json", "--viewport", viewport, target],
        cwd,
        &[],
    );
    assert!(
        out.code == 0 || out.code == 2,
        "detect failed: {}",
        out.stderr
    );
    serde_json::from_str(&out.stdout).unwrap()
}

#[test]
fn argument_and_state_file_errors_stop_before_the_browser() {
    let temp = Temp::new("args");
    let bad = temp.write(
        "states.json",
        r#"{ "states": [{ "name": "a", "steps": [{ "do": "clik" }] }] }"#,
    );
    let out = phoenix(
        &[
            "capture",
            "page.html",
            "--states",
            bad.to_str().unwrap(),
            "--out",
            "o",
        ],
        temp.path(),
        &[("PHOENIX_UI_BROWSER", "/nonexistent")],
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("找不到目标 page.html"),
        "{}",
        out.stderr
    );
    std::fs::write(temp.path().join("page.html"), "<p>x</p>").unwrap();
    let out = phoenix(
        &[
            "capture",
            "page.html",
            "--states",
            bad.to_str().unwrap(),
            "--out",
            "o",
        ],
        temp.path(),
        &[("PHOENIX_UI_BROWSER", "/nonexistent")],
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("states[0].steps[0].do: 未知动作 clik，是否想写 click？"),
        "{}",
        out.stderr
    );
    assert!(!temp.path().join("o").exists(), "no output before a run");
    let help = phoenix(&["capture", "--help"], temp.path(), &[]);
    assert_eq!(help.code, 0);
    assert!(
        help.stdout.contains("capture-server") && help.stdout.contains("退出码"),
        "{}",
        help.stdout
    );
}

/// ① The initial state's findings equal `detect --json` for the same page.
#[test]
fn initial_findings_match_detect() {
    if !browser_available() {
        return;
    }
    let temp = Temp::new("consistency");
    let served = Served::start(&fixtures());
    let cases = [
        (format!("{}xgent/controls.html", served.url), "1280x800"),
        (
            format!("{}capture/pages/entrance.html", served.url),
            "1280x800",
        ),
        (format!("{}xgent/controls.html", served.url), "390x844"),
    ];
    for (i, (url, viewport)) in cases.iter().enumerate() {
        let out = temp.path().join(format!("c{i}"));
        let run = phoenix(
            &[
                "capture",
                url,
                "--viewport",
                viewport,
                "--out",
                out.to_str().unwrap(),
            ],
            temp.path(),
            &[],
        );
        assert!(
            run.code == 0 || run.code == 2,
            "{}{}",
            run.stdout,
            run.stderr
        );
        let captured = shot(&report(&out), &format!("{viewport}.png"))["findings"].clone();
        assert_eq!(
            captured,
            detect_json(url, temp.path(), viewport),
            "{url} at {viewport}"
        );
    }
    // A local file is served over http but reports findings as its file:// URL.
    let local = fixtures().join("xgent/controls.html");
    let out = temp.path().join("local");
    let run = phoenix(
        &[
            "capture",
            local.to_str().unwrap(),
            "--viewport",
            "1280x800",
            "--out",
            out.to_str().unwrap(),
        ],
        temp.path(),
        &[],
    );
    assert!(
        run.code == 0 || run.code == 2,
        "{}{}",
        run.stdout,
        run.stderr
    );
    let captured = shot(&report(&out), "1280x800.png")["findings"].clone();
    let file_url = format!("file://{}", local.canonicalize().unwrap().display());
    let detected = detect_json(&file_url, temp.path(), "1280x800");
    assert!(
        !detected.as_array().unwrap().is_empty(),
        "the fixture has findings to compare"
    );
    assert_eq!(captured, detected);
    assert_eq!(captured[0]["file"], file_url.as_str());
}

/// ② Writes from fetch, forms, beacons, XHR, a cross-site frame and a
/// worker never reach the server; only the allowed path does.
#[test]
fn block_writes_covers_every_channel() {
    if !browser_available() {
        return;
    }
    let temp = Temp::new("writes");
    let served = Served::start(&pages());
    let out = temp.path().join("out");
    let run = phoenix(
        &[
            "capture",
            &format!("{}writes.html", served.url),
            "--viewport",
            "800x600",
            "--block-writes",
            "--allow-write",
            "/api/allowed",
            "--out",
            out.to_str().unwrap(),
        ],
        temp.path(),
        &[],
    );
    assert!(
        run.code == 0 || run.code == 2,
        "{}{}",
        run.stdout,
        run.stderr
    );
    std::thread::sleep(std::time::Duration::from_millis(300));
    let mut reached = served.writes.lock().unwrap().clone();
    reached.dedup();
    assert_eq!(
        reached,
        ["/api/allowed"],
        "only the allowed write reaches the server"
    );
    let report = report(&out);
    let entry = shot(&report, "800x600.png");
    let blocked: Vec<String> = entry["blockedWrites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            format!(
                "{} {}",
                w["method"].as_str().unwrap(),
                w["url"].as_str().unwrap().rsplit('/').next().unwrap()
            )
        })
        .collect();
    for expected in [
        "POST fetch",
        "POST beacon",
        "PUT xhr",
        "POST form",
        "POST xframe",
        "POST worker",
        "POST sw",
    ] {
        assert!(
            blocked.iter().any(|b| b == expected),
            "{expected} missing from {blocked:?}"
        );
    }
    assert!(entry["allowedWrites"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["url"].as_str().unwrap().ends_with("/api/allowed")));
    // A blocked write is reported once, not again as a failed request.
    let failed = entry["failedRequests"].to_string();
    assert!(!failed.contains("/api/fetch"), "{failed}");
    // Every shot carries the tool identity that captured it, so a later
    // partial re-capture by another engine cannot rewrite it.
    assert_eq!(
        entry["tool"]["binarySha256"],
        report["tool"]["binarySha256"]
    );
}

/// A file name already used by a derived output is refused before anything
/// runs, so one name still means one picture.
#[test]
fn a_name_used_by_a_derived_file_is_rejected_before_the_browser() {
    let temp = Temp::new("names");
    std::fs::write(temp.path().join("page.html"), "<p>x</p>").unwrap();
    let out = phoenix(
        &[
            "capture",
            "page.html",
            "--viewport",
            "desktop",
            "--name",
            "sheet-{viewport}.png",
            "--sheet",
            "--out",
            "o",
        ],
        temp.path(),
        &[("PHOENIX_UI_BROWSER", "/nonexistent")],
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("输出文件名冲突") && out.stderr.contains("sheet-desktop.png"),
        "{}",
        out.stderr
    );
    assert!(!temp.path().join("o").exists(), "no output before a run");
}

/// ⑤ Elements narrower than a mark box (4 px) report a derived problem
/// instead of panicking; the main pictures survive.
#[test]
fn marks_on_sub_4px_elements_report_a_problem_instead_of_panicking() {
    if !browser_available() {
        return;
    }
    let temp = Temp::new("tiny");
    let served = Served::start(&pages());
    let states = temp.write(
        "states.json",
        r##"{ "states": [
          { "name": "divider", "steps": [{ "do": "waitFor", "target": { "css": "#v" } }], "type": { "element": { "css": "#v" } }, "mark": [{ "css": "#v" }] },
          { "name": "rule", "steps": [{ "do": "waitFor", "target": { "css": "#h" } }], "type": { "element": { "css": "#h" } }, "mark": [{ "css": "#h" }] }
        ] }"##,
    );
    let out = temp.path().join("out");
    let run = phoenix(
        &[
            "capture",
            &format!("{}tiny.html", served.url),
            "--viewport",
            "400x300",
            "--states",
            states.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
        temp.path(),
        &[],
    );
    // The mark image is missing (derived problem → 2), but the process
    // survives and the main element pictures exist.
    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    let report = report(&out);
    for file in ["400x300-divider.png", "400x300-rule.png"] {
        let entry = shot(&report, file);
        assert!(
            out.join(file).exists(),
            "{file} missing; files: {:?}",
            std::fs::read_dir(&out)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect::<Vec<_>>()
        );
        let problems = entry["derived"]["problems"].to_string();
        assert!(
            problems.contains("标注图未产出") && problems.contains("小于 4 像素"),
            "{problems}"
        );
        assert!(!out.join(file.replace(".png", ".mark.png")).exists());
    }
}

/// ③ Credentials never appear in the report, stdout or stderr; ④ the
/// temporary browser profile is gone afterwards.
#[test]
fn secrets_are_redacted_and_the_profile_is_removed() {
    if !browser_available() {
        return;
    }
    let temp = Temp::new("secret");
    let served = Served::start(&pages());
    let states = temp.write(
        "states.json",
        r##"{ "states": [
          { "name": "typed", "steps": [
            { "do": "fill", "target": { "css": "#secret" }, "env": "CAPTURE_SECRET" },
            { "do": "waitFor", "target": { "text": "you typed" } }
          ] },
          { "name": "broken", "steps": [
            { "do": "fill", "target": { "css": "#secret" }, "env": "CAPTURE_SECRET" },
            { "do": "waitFor", "target": { "text": "never shown" }, "timeoutMs": 200 }
          ] }
        ] }"##,
    );
    let secret = "Zq-secret-4471";
    let target = served.url.replace("http://", "http://tester:Pw-basic-9@") + "secret.html";
    let out = temp.path().join("out");
    let run = phoenix(
        &[
            "capture",
            &target,
            "--viewport",
            "800x600",
            "--states",
            states.to_str().unwrap(),
            "--json",
            "--out",
            out.to_str().unwrap(),
        ],
        temp.path(),
        &[("CAPTURE_SECRET", secret)],
    );
    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    let report_text = std::fs::read_to_string(out.join("capture-report.json")).unwrap();
    for text in [&report_text, &run.stdout, &run.stderr] {
        assert!(
            !text.contains(secret) && !text.contains("Pw-basic-9"),
            "leak in {text}"
        );
    }
    let report: Value = serde_json::from_str(&report_text).unwrap();
    let typed = shot(&report, "800x600-typed.png");
    assert_eq!(typed["consoleErrors"][0], "echo ***");
    assert!(
        typed["url"].as_str().unwrap().ends_with("?token=***"),
        "{}",
        typed["url"]
    );
    assert!(
        typed["target"].as_str().unwrap().contains("tester:***@"),
        "{}",
        typed["target"]
    );
    let profiles: Vec<String> = std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(&format!("phoenix-ui-chrome-profile-{}-", run.pid)))
        .collect();
    assert!(profiles.is_empty(), "profiles left behind: {profiles:?}");
}

/// ⑤ The self-check flags unsettled, blank and undrawn shots, keeps tall
/// full pages and sized element shots, and never leaves an old picture
/// under a name the run could not produce.
#[test]
fn validity_rules_on_real_pages() {
    if !browser_available() {
        return;
    }
    let temp = Temp::new("validity");
    let run_page = |page: &str, extra: &[&str]| -> (Output, PathBuf) {
        let out = temp.path().join(page.replace('.', "-"));
        std::fs::create_dir_all(&out).unwrap();
        let target = pages().join(page);
        let mut args = vec![
            "capture",
            target.to_str().unwrap(),
            "--viewport",
            "400x300",
            "--out",
            out.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        (phoenix(&args, temp.path(), &[]), out)
    };

    let stale = temp.path().join("fade-html");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("400x300.png"), b"old picture").unwrap();
    let (run, out) = run_page("fade.html", &["--settle-ms", "300"]);
    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains("INVALID initial 400x300 400x300.invalid.png: 动画未稳定"),
        "{}",
        run.stdout
    );
    assert!(out.join("400x300.invalid.png").is_file());
    assert!(
        !out.join("400x300.png").exists(),
        "the old picture is removed"
    );

    let (run, out) = run_page("white.html", &[]);
    assert_eq!(run.code, 2);
    assert_eq!(
        shot(&report(&out), "400x300.png")["invalidReasons"][0],
        "整张为单一颜色，页面未绘出"
    );

    let (run, out) = run_page("canvas.html", &[]);
    assert_eq!(run.code, 2, "{}", run.stdout);
    assert_eq!(
        shot(&report(&out), "400x300.png")["invalidReasons"][0],
        "画布未绘出：#chart"
    );

    let (run, out) = run_page("tall.html", &["--type", "full"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let entry = shot(&report(&out), "400x300.png");
    assert_eq!(
        (
            entry["pixels"]["width"].as_u64(),
            entry["pixels"]["height"].as_u64()
        ),
        (Some(400), Some(3000))
    );

    let states = temp.write(
        "element.json",
        r##"{ "initial": { "type": { "element": { "css": "#box" } } } }"##,
    );
    let (run, out) = run_page(
        "tall.html",
        &["--dpr", "2", "--states", states.to_str().unwrap()],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let entry = shot(&report(&out), "400x300.png");
    assert_eq!(entry["type"], "element");
    assert_eq!(
        (
            entry["pixels"]["width"].as_u64(),
            entry["pixels"]["height"].as_u64()
        ),
        (Some(240), Some(160))
    );
}

/// ⑥ Failures say why and how to fix the locator; warnings do not stop a step.
#[test]
fn step_failures_and_warnings() {
    if !browser_available() {
        return;
    }
    let temp = Temp::new("failures");
    let out = temp.path().join("out");
    let page = pages().join("failures.html");
    let states = pages().join("failures.json");
    let run = phoenix(
        &[
            "capture",
            page.to_str().unwrap(),
            "--viewport",
            "800x600",
            "--states",
            states.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
        temp.path(),
        &[],
    );
    assert_eq!(run.code, 2, "{}", run.stdout);
    let report = report(&out);
    let hidden = failure(&report, "hidden-target")["reason"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        hidden.contains("display:none") && hidden.contains("先执行让它出现的那一步"),
        "{hidden}"
    );
    let hidden_hints = &failure(&report, "hidden-target")["hints"];
    assert_eq!(
        hidden_hints,
        &json!([r#"{"text":"被盖住"}"#, r#"{"text":"替代按钮"}"#])
    );
    assert!(
        run.stdout.contains(
            "FAILED hidden-target 800x600 step 1 (click): 找到 1 个匹配，但都不可见：隐藏（display:none）；先执行让它出现的那一步\n  可改为：{\"text\":\"被盖住\"}"
        ),
        "{}",
        run.stdout
    );
    let covered = shot(&report, "800x600-covered.png")["steps"][0].clone();
    assert_eq!(covered["result"], "warning");
    let covered_warning = covered["warnings"][0].as_str().unwrap();
    assert!(
        covered_warning.starts_with("目标中心被 span.veil 遮挡，动作照常执行")
            && covered_warning
                .ends_with(r#"；遮挡物：{"css":"span.veil"}；可用的可见元素：{"text":"替代按钮"}"#),
        "{covered}"
    );
    let disabled = shot(&report, "800x600-disabled.png")["steps"][0].clone();
    assert_eq!(
        disabled["warnings"][0],
        r#"目标处于禁用状态（disabled），动作照常执行；可用的可见元素：{"text":"被盖住"}、{"text":"替代按钮"}"#
    );
    let missing = failure(&report, "missing-text");
    assert_eq!(missing["hints"][0], r#"{"text":"被盖住"}"#);
    let waited = failure(&report, "wait-hidden")["reason"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        waited.contains("消失超时（300 ms）") && waited.contains("当前可见匹配 1 个"),
        "{waited}"
    );
    let framed = failure(&report, "framed")["reason"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(framed.contains("1 个 iframe"), "{framed}");
    for state in ["wait-framed", "wait-shadow"] {
        let reason = failure(&report, state)["reason"].as_str().unwrap();
        assert!(
            reason.contains("出现超时（300 ms）")
                && reason.contains(
                    "；选择器只作用于主文档：页面有 1 个 iframe、1 个开放 shadow root，可直接打开被嵌入的页面"
                ),
            "{state}: {reason}"
        );
    }
    assert!(
        run.stdout.contains("FAILED framed 800x600 step 1 (click)"),
        "{}",
        run.stdout
    );
    let dated = shot(&report, "800x600-date-filled.png")["steps"][0].clone();
    assert_eq!(dated["result"], "ok");
    assert!(
        dated["observed"].as_str().unwrap().contains("直接设值"),
        "{dated}"
    );
    let typed = failure(&report, "date-typed")["reason"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(typed.contains("改用 fill"), "{typed}");
}
