//! The `compare` payload: one validation pass that reports every problem,
//! each naming the field it is about.

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// `file` is the real path; its parent directory is all the server serves.
    Html {
        file: PathBuf,
        interactive: bool,
    },
    Image {
        file: PathBuf,
        pixel_ratio: f64,
    },
    /// A loopback page: `host` as written in the URL (`[::1]` keeps its brackets).
    Url {
        url: String,
        scheme: String,
        host: String,
        port: u16,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub name: String,
    pub baseline: bool,
    pub source: Source,
    pub concept: Option<String>,
    pub typography: Option<String>,
    pub palette: Vec<String>,
    pub traits: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Serve {
    pub command: String,
    pub cwd: Option<String>,
}

/// A validated comparison: the baseline, when there is one, sits first and
/// the rest keep payload order. Route indexes refer to this order.
#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub title: Option<String>,
    pub question: Option<String>,
    pub lang: Lang,
    pub serve: Option<Serve>,
    pub proposals: Vec<Proposal>,
}

const HTML_EXT: &[&str] = &["html", "htm"];
const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];
const LOOPBACK: &[&str] = &["localhost", "127.0.0.1", "[::1]"];

pub fn parse(cwd: &Path, payload: &Value) -> Result<Round, Vec<String>> {
    let Some(top) = payload.as_object() else {
        return Err(vec!["载荷顶层必须是 JSON 对象".into()]);
    };
    let mut problems: Vec<String> = Vec::new();
    let title = optional_text(top.get("title"), "title", &mut problems);
    let question = optional_text(top.get("question"), "question", &mut problems);
    let lang = match top.get("lang") {
        None | Some(Value::Null) => Lang::Zh,
        Some(Value::String(s)) if s == "zh-CN" => Lang::Zh,
        Some(Value::String(s)) if s == "en" => Lang::En,
        Some(_) => {
            problems.push("lang 只能是 \"zh-CN\" 或 \"en\"".into());
            Lang::Zh
        }
    };
    let serve = match top.get("serve") {
        None | Some(Value::Null) => None,
        Some(Value::Object(s)) => {
            let command = s
                .get("command")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if command.is_empty() {
                problems.push("serve.command 必须是非空字符串，例如 \"pnpm dev\"".into());
            }
            let cwd = optional_text(s.get("cwd"), "serve.cwd", &mut problems);
            Some(Serve {
                command: command.to_string(),
                cwd,
            })
        }
        Some(_) => {
            problems
                .push("serve 必须是对象，例如 {\"command\": \"pnpm dev\", \"cwd\": \".\"}".into());
            None
        }
    };

    let items: &[Value] = top
        .get("proposals")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if items.len() < 2 {
        problems.push("proposals 至少需要两个方案；只有一个方案时不需要对比页".into());
    }
    let real_cwd = std::fs::canonicalize(cwd).ok();
    let mut proposals: Vec<Proposal> = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("proposals[{}]", i);
        let Some(p) = item.as_object() else {
            problems.push(format!("{} 必须是对象", at));
            continue;
        };
        let before = problems.len();
        let id = p.get("id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            problems.push(format!(
                "{}.id 必须非空，且只含字母、数字、下划线或连字符",
                at
            ));
        } else if seen_ids.iter().any(|s| s == id) {
            problems.push(format!("{}.id 与前面的方案重复（{}）", at, id));
        } else {
            seen_ids.push(id.to_string());
        }
        let name = p
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if name.is_empty() {
            problems.push(format!("{}.name 必须是非空字符串", at));
        }
        let baseline = flag(
            p.get("baseline"),
            &format!("{}.baseline", at),
            &mut problems,
        );
        let interactive = flag(
            p.get("interactive"),
            &format!("{}.interactive", at),
            &mut problems,
        );
        let has = |k: &str| p.get(k).is_some_and(|v| !v.is_null());
        let source = match (has("source"), has("url")) {
            (true, false) => file_source(
                cwd,
                real_cwd.as_deref(),
                p.get("source"),
                &at,
                &mut problems,
            ),
            (false, true) => url_source(p.get("url"), &at, &mut problems),
            _ => {
                problems.push(format!("{} 的 source 与 url 必须恰好给一个", at));
                None
            }
        };
        let is_html = matches!(source, Some(Source::Html { .. }));
        let is_image = matches!(source, Some(Source::Image { .. }));
        if interactive && source.is_some() && !is_html {
            problems.push(format!("{}.interactive 只用于 HTML 方案", at));
        }
        let mut pixel_ratio = 1.0;
        if has("pixelRatio") {
            match p.get("pixelRatio").and_then(Value::as_f64) {
                Some(n) if (1.0..=4.0).contains(&n) => pixel_ratio = n,
                _ => problems.push(format!("{}.pixelRatio 必须是 1–4 的数", at)),
            }
            if source.is_some() && !is_image {
                problems.push(format!("{}.pixelRatio 只用于截图方案", at));
            }
        }
        let concept = optional_text(p.get("concept"), &format!("{}.concept", at), &mut problems);
        let typography = optional_text(
            p.get("typography"),
            &format!("{}.typography", at),
            &mut problems,
        );
        let palette = text_list(p.get("palette"), &format!("{}.palette", at), &mut problems);
        let traits = text_list(p.get("traits"), &format!("{}.traits", at), &mut problems);
        let Some(source) = source else { continue };
        if problems.len() > before {
            continue;
        }
        let source = match source {
            Source::Html { file, .. } => Source::Html { file, interactive },
            Source::Image { file, .. } => Source::Image { file, pixel_ratio },
            url => url,
        };
        proposals.push(Proposal {
            id: id.to_string(),
            name: name.to_string(),
            baseline,
            source,
            concept,
            typography,
            palette,
            traits,
        });
    }
    let baselines = items
        .iter()
        .filter(|p| p.get("baseline") == Some(&Value::Bool(true)))
        .count();
    if baselines > 1 {
        problems.push(format!(
            "最多只能有一个基线方案（baseline: true），现在有 {} 个",
            baselines
        ));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    // Stable: the baseline moves to the front, the others keep their order.
    proposals.sort_by_key(|p| !p.baseline);
    Ok(Round {
        title,
        question,
        lang,
        serve,
        proposals,
    })
}

fn optional_text(v: Option<&Value>, at: &str, problems: &mut Vec<String>) -> Option<String> {
    match v {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Some(_) => {
            problems.push(format!("{} 必须是字符串", at));
            None
        }
    }
}

fn text_list(v: Option<&Value>, at: &str, problems: &mut Vec<String>) -> Vec<String> {
    match v {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) if a.iter().all(Value::is_string) => a
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Some(_) => {
            problems.push(format!("{} 必须是字符串数组", at));
            Vec::new()
        }
    }
}

fn flag(v: Option<&Value>, at: &str, problems: &mut Vec<String>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            problems.push(format!("{} 必须是 true 或 false", at));
            false
        }
    }
}

fn file_source(
    cwd: &Path,
    real_cwd: Option<&Path>,
    v: Option<&Value>,
    at: &str,
    problems: &mut Vec<String>,
) -> Option<Source> {
    let Some(raw) = v.and_then(Value::as_str).filter(|s| !s.trim().is_empty()) else {
        problems.push(format!("{}.source 必须是非空字符串", at));
        return None;
    };
    let ext = Path::new(raw)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let html = HTML_EXT.contains(&ext.as_str());
    if !html && !IMAGE_EXT.contains(&ext.as_str()) {
        problems.push(format!(
            "{}.source 的扩展名不支持（{}）：HTML 方案用 .html/.htm，截图方案用 .png/.jpg/.jpeg/.webp/.gif",
            at, raw
        ));
        return None;
    }
    let real = std::fs::canonicalize(cwd.join(raw))
        .ok()
        .filter(|p| p.is_file());
    let Some(real) = real else {
        problems.push(format!("{}.source 不是已存在的文件（{}）", at, raw));
        return None;
    };
    // Symlinks resolve first, so a link that leaves the project is refused too.
    if !real_cwd.is_some_and(|root| real.starts_with(root)) {
        problems.push(format!("{}.source 必须在当前项目目录之内（{}）", at, raw));
        return None;
    }
    Some(if html {
        Source::Html {
            file: real,
            interactive: false,
        }
    } else {
        Source::Image {
            file: real,
            pixel_ratio: 1.0,
        }
    })
}

fn url_source(v: Option<&Value>, at: &str, problems: &mut Vec<String>) -> Option<Source> {
    let Some(raw) = v
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        problems.push(format!("{}.url 必须是非空字符串", at));
        return None;
    };
    let Some(u) = crate::url::parse(raw) else {
        problems.push(format!(
            "{}.url 必须是 http:// 或 https:// 开头的本机地址（{}）",
            at, raw
        ));
        return None;
    };
    if !LOOPBACK.contains(&u.hostname.as_str()) {
        problems.push(format!(
            "{}.url 的主机只能是 localhost、127.0.0.1 或 [::1]（{}）；其他地址的页面先截图，改用截图方案",
            at, raw
        ));
        return None;
    }
    let port = u
        .port
        .parse::<u16>()
        .unwrap_or(if u.scheme == "https" { 443 } else { 80 });
    Some(Source::Url {
        url: raw.to_string(),
        scheme: u.scheme,
        host: u.hostname,
        port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Project(PathBuf);

    impl Project {
        fn new(tag: &str) -> Project {
            let dir = std::env::temp_dir().join(format!(
                "phoenix-compare-payload-{}-{}",
                tag,
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("variants/a")).unwrap();
            std::fs::create_dir_all(dir.join("shots")).unwrap();
            std::fs::write(dir.join("variants/a/index.html"), "<p>a</p>").unwrap();
            std::fs::write(dir.join("variants/a/B.HTM"), "<p>b</p>").unwrap();
            std::fs::write(dir.join("shots/c.PNG"), [0u8; 4]).unwrap();
            std::fs::write(dir.join("notes.txt"), "x").unwrap();
            Project(dir)
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn html(id: &str) -> Value {
        json!({ "id": id, "name": id, "source": "variants/a/index.html" })
    }

    fn problems(project: &Project, payload: Value) -> Vec<String> {
        parse(&project.0, &payload).expect_err("payload should be rejected")
    }

    fn rejected(project: &Project, payload: Value, needle: &str) {
        let found = problems(project, payload);
        assert!(
            found.iter().any(|p| p.contains(needle)),
            "expected a problem containing {needle:?}, got {found:?}"
        );
    }

    #[test]
    fn accepts_the_three_source_kinds_and_puts_the_baseline_first() {
        let project = Project::new("ok");
        let round = parse(
            &project.0,
            &json!({
                "title": " 订单页 ", "lang": "en", "serve": { "command": "pnpm dev" },
                "proposals": [
                    { "id": "a", "name": "A", "source": "variants/a/index.html", "interactive": true,
                      "concept": "流水账", "palette": ["#111", ""], "traits": [] },
                    { "id": "b", "name": "B", "source": "variants/a/B.HTM" },
                    { "id": "shot", "name": "C", "source": "shots/c.PNG", "pixelRatio": 2 },
                    { "id": "now", "name": "现状", "url": "http://localhost:5173/orders", "baseline": true },
                    { "id": "v4", "name": "D", "url": "https://127.0.0.1/" },
                    { "id": "v6", "name": "E", "url": "http://[::1]:3000/x" }
                ]
            }),
        )
        .expect("valid payload");
        assert_eq!(round.title.as_deref(), Some("订单页"));
        assert_eq!(round.question, None);
        assert_eq!(round.lang, Lang::En);
        assert_eq!(
            round.serve,
            Some(Serve {
                command: "pnpm dev".into(),
                cwd: None
            })
        );
        let ids: Vec<&str> = round.proposals.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["now", "a", "b", "shot", "v4", "v6"]);
        assert!(
            matches!(&round.proposals[0].source, Source::Url { host, port: 5173, .. } if host == "localhost")
        );
        assert!(matches!(
            &round.proposals[1].source,
            Source::Html {
                interactive: true,
                ..
            }
        ));
        assert_eq!(round.proposals[1].palette, ["#111"]);
        assert!(matches!(
            &round.proposals[2].source,
            Source::Html {
                interactive: false,
                ..
            }
        ));
        assert!(
            matches!(&round.proposals[3].source, Source::Image { pixel_ratio, .. } if *pixel_ratio == 2.0)
        );
        assert!(
            matches!(&round.proposals[4].source, Source::Url { scheme, port: 443, .. } if scheme == "https")
        );
        assert!(
            matches!(&round.proposals[5].source, Source::Url { host, port: 3000, .. } if host == "[::1]")
        );
    }

    #[test]
    fn lang_defaults_to_chinese_and_optional_fields_may_be_absent() {
        let project = Project::new("defaults");
        let round = parse(&project.0, &json!({ "proposals": [html("a"), html("b")] }))
            .expect("valid payload");
        assert_eq!(round.lang, Lang::Zh);
        assert_eq!(round.title, None);
        assert_eq!(round.serve, None);
        assert!(round.proposals[0].traits.is_empty());
    }

    #[test]
    fn rejects_shape_problems() {
        let project = Project::new("shape");
        assert_eq!(problems(&project, json!([1])).len(), 1);
        rejected(
            &project,
            json!({ "proposals": [html("a")] }),
            "至少需要两个方案",
        );
        rejected(
            &project,
            json!({ "proposals": [html("a"), html("a")] }),
            "proposals[1].id 与前面的方案重复",
        );
        rejected(
            &project,
            json!({ "proposals": [html("a"), { "id": "b c", "name": "B", "source": "variants/a/index.html" }] }),
            "proposals[1].id",
        );
        rejected(
            &project,
            json!({ "proposals": [html("a"), { "id": "b", "name": " ", "source": "variants/a/index.html" }] }),
            "proposals[1].name",
        );
        rejected(
            &project,
            json!({ "proposals": [html("a"), { "id": "b", "name": "B" }] }),
            "恰好给一个",
        );
        rejected(
            &project,
            json!({ "proposals": [html("a"), { "id": "b", "name": "B", "source": "variants/a/index.html", "url": "http://localhost/" }] }),
            "恰好给一个",
        );
        rejected(
            &project,
            json!({ "title": 3, "proposals": [html("a"), html("b")] }),
            "title 必须是字符串",
        );
        rejected(
            &project,
            json!({ "lang": "zh", "proposals": [html("a"), html("b")] }),
            "lang",
        );
        rejected(
            &project,
            json!({ "serve": { "cwd": "." }, "proposals": [html("a"), html("b")] }),
            "serve.command",
        );
    }

    #[test]
    fn rejects_sources_outside_the_contract() {
        let project = Project::new("source");
        let with = |extra: Value| json!({ "proposals": [html("a"), extra] });
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "variants/a/missing.html" })),
            "不是已存在的文件",
        );
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "notes.txt" })),
            "扩展名不支持",
        );
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "variants/a" })),
            "扩展名不支持",
        );
        let outside = project.0.with_extension("outside.html");
        std::fs::write(&outside, "<p>outside</p>").unwrap();
        let name = outside.file_name().unwrap().to_str().unwrap().to_string();
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": format!("../{}", name) })),
            "必须在当前项目目录之内",
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, project.0.join("variants/a/link.html")).unwrap();
            rejected(
                &project,
                with(json!({ "id": "b", "name": "B", "source": "variants/a/link.html" })),
                "必须在当前项目目录之内",
            );
        }
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn rejects_non_loopback_and_non_http_urls() {
        let project = Project::new("url");
        let with =
            |url: &str| json!({ "proposals": [html("a"), { "id": "b", "name": "B", "url": url }] });
        rejected(&project, with("http://example.com/"), "改用截图方案");
        rejected(
            &project,
            with("http://localhost.evil.test/"),
            "改用截图方案",
        );
        rejected(&project, with("http://192.168.1.2:3000/"), "改用截图方案");
        rejected(&project, with("file:///etc/hosts"), "http:// 或 https://");
        rejected(&project, with("localhost:5173"), "http:// 或 https://");
    }

    #[test]
    fn rejects_flags_on_the_wrong_kind_and_two_baselines() {
        let project = Project::new("flags");
        let with = |extra: Value| json!({ "proposals": [html("a"), extra] });
        rejected(
            &project,
            with(
                json!({ "id": "b", "name": "B", "url": "http://localhost/", "interactive": true }),
            ),
            "interactive 只用于 HTML 方案",
        );
        rejected(
            &project,
            with(
                json!({ "id": "b", "name": "B", "source": "variants/a/index.html", "pixelRatio": 2 }),
            ),
            "pixelRatio 只用于截图方案",
        );
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "shots/c.PNG", "pixelRatio": 5 })),
            "1–4",
        );
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "shots/c.PNG", "baseline": "yes" })),
            "baseline 必须是 true 或 false",
        );
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "shots/c.PNG", "palette": "#fff" })),
            "palette 必须是字符串数组",
        );
        rejected(
            &project,
            with(json!({ "id": "b", "name": "B", "source": "shots/c.PNG", "concept": 1 })),
            "concept 必须是字符串",
        );
        let mut a = html("a");
        a["baseline"] = json!(true);
        rejected(
            &project,
            json!({ "proposals": [a, { "id": "b", "name": "B", "source": "shots/c.PNG", "baseline": true }] }),
            "最多只能有一个基线",
        );
    }

    #[test]
    fn reports_every_problem_in_one_pass() {
        let project = Project::new("all");
        let found = problems(
            &project,
            json!({ "lang": "fr", "proposals": [
                { "id": "a", "name": "A", "url": "http://example.com/", "baseline": true },
                { "id": "b", "name": "", "source": "../outside.html", "baseline": true }
            ] }),
        );
        assert!(found.len() >= 5, "{found:?}");
    }
}
