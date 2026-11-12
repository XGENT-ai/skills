//! Open-license media kept in the local preview area.
mod audit;
mod fetch;
mod index;
mod look;
mod poster;
mod relevance;
mod rights;
mod sheet;
mod sources;

use impeccable_common::{jsp, Io};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

const USAGE: &str = "用法：phoenix-ui media <search|preview|fetch|audit> [选项]
  search \"<词>[|<词>…]\" [--kind <类别>] [--limit <1-30>] [--use all|page] [--sheet]
  preview <编号>…
  fetch <编号>… [--maps color,roughness,normal|all] [--resolution 1k|2k|4k|8k]
        [--video-size medium|small|preview|orig] [--max-mb <1-2048>]
        [--download-timeout-ms <ms>]
  audit [<路径>…]
  公共：--dir <素材区>（默认 .phoenix-ui/media）、--json
  联网超时：--timeout-ms <1000-120000>（默认 12000）

素材只用于设计稿与方案预览，只进当前项目 .phoenix-ui/ 下的素材区，不进入产品。
类别：all 全部、texture 材质贴图、design 设计器物、photo 摄影、blueprint 测绘图纸、
      map 地图、plate 版画、science 科学、video 视频。
分级：page 可上线、check 需核对、reference 仅参考；分级不授权素材进入当前产品。
search/preview/fetch 会联网；audit 只读、不联网。
--sheet/preview 生成 candidates.png，标注编号、分级、原图尺寸及观感数据。
fetch 图片须完整解码；视频须同时得到首帧，依赖本机浏览器及视频编码支持。
audit 查素材引用、相同副本及许可/署名；credit 提示由主代理判断并记录依据。
退出码：0 完成（含零条结果）；1 无法运行或全部信源不可用；2 部分失败或 audit 有发现。
";
const VALUED: &[&str] = &[
    "--kind",
    "--limit",
    "--use",
    "--dir",
    "--timeout-ms",
    "--maps",
    "--resolution",
    "--video-size",
    "--max-mb",
    "--download-timeout-ms",
];
const FLAGS: &[&str] = &["--sheet", "--json", "--help"];

struct Options {
    command: String,
    positional: Vec<String>,
    kind: String,
    limit: usize,
    page_only: bool,
    sheet: bool,
    dir: String,
    timeout_ms: u64,
    json: bool,
    fetch: fetch::FetchOptions,
}

fn nearest<'a>(word: &str, choices: &'a [&str]) -> &'a str {
    choices
        .iter()
        .min_by_key(|choice| {
            let mut row: Vec<usize> = (0..=choice.len()).collect();
            for (i, a) in word.bytes().enumerate() {
                let mut previous = row[0];
                row[0] = i + 1;
                for (j, b) in choice.bytes().enumerate() {
                    let old = row[j + 1];
                    row[j + 1] = (row[j] + 1)
                        .min(old + 1)
                        .min(previous + usize::from(a != b));
                    previous = old;
                }
            }
            row[choice.len()]
        })
        .copied()
        .unwrap_or("--help")
}

fn parse(args: &[String]) -> Result<Option<Options>, String> {
    if args.iter().any(|s| matches!(s.as_str(), "--help" | "-h")) {
        return Ok(None);
    }
    let command = args
        .first()
        .ok_or("请指定 search、preview、fetch 或 audit")?;
    if !["search", "preview", "fetch", "audit"].contains(&command.as_str()) {
        return Err(format!("未知子命令 {command}"));
    }
    let mut values = HashMap::new();
    let mut flags = Vec::new();
    let mut positional = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if !arg.starts_with('-') {
            positional.push(arg.clone());
            continue;
        }
        let (key, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(k, v)| (k, Some(v)));
        if FLAGS.contains(&key) {
            if inline.is_some() {
                return Err(format!("{key} 不带值"));
            }
            flags.push(key);
        } else if VALUED.contains(&key) {
            let value = match inline {
                Some(v) => v.to_string(),
                None => {
                    let v = args
                        .get(i)
                        .filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| format!("{key} 需要一个值"))?;
                    i += 1;
                    v.clone()
                }
            };
            if values.insert(key, value).is_some() {
                return Err(format!("{key} 只能给一次"));
            }
        } else {
            let choices: Vec<_> = VALUED.iter().chain(FLAGS).copied().collect();
            return Err(format!(
                "未知选项 {key}，是否想写 {}？",
                nearest(key, &choices)
            ));
        }
    }
    let allowed: &[&str] = match command.as_str() {
        "search" => &["--kind", "--limit", "--use", "--dir", "--timeout-ms"],
        "preview" => &["--dir", "--timeout-ms"],
        "fetch" => &[
            "--maps",
            "--resolution",
            "--video-size",
            "--max-mb",
            "--dir",
            "--timeout-ms",
            "--download-timeout-ms",
        ],
        _ => &["--dir"],
    };
    for key in values.keys() {
        if !allowed.contains(key) {
            return Err(format!("{command} 不接受 {key}"));
        }
    }
    if flags.contains(&"--sheet") && command != "search" {
        return Err(format!("{command} 不接受 --sheet"));
    }
    match command.as_str() {
        "search"
            if positional.len() != 1 || positional[0].split('|').any(|q| q.trim().is_empty()) =>
        {
            return Err("search 需要一个非空检索词组，多个词组用 | 分隔".into())
        }
        "preview" | "fetch" if positional.is_empty() => {
            return Err(format!("{command} 需要素材编号"))
        }
        _ => {}
    }
    let value =
        |key: &str, default: &str| values.get(key).cloned().unwrap_or_else(|| default.into());
    let number = |key: &str, default: u64, lo: u64, hi: u64| -> Result<u64, String> {
        match values.get(key) {
            None => Ok(default),
            Some(s) => s
                .parse::<u64>()
                .ok()
                .filter(|n| (lo..=hi).contains(n))
                .ok_or_else(|| format!("{key} 应为 {lo}–{hi} 的整数")),
        }
    };
    let choice = |key: &str, default: &str, allowed: &[&str]| -> Result<String, String> {
        let s = value(key, default);
        if allowed.contains(&s.as_str()) {
            Ok(s)
        } else {
            Err(format!("{key} 应为 {}", allowed.join("|")))
        }
    };
    let kind = choice(
        "--kind",
        "all",
        &[
            "all",
            "texture",
            "design",
            "photo",
            "blueprint",
            "map",
            "plate",
            "science",
            "video",
        ],
    )?;
    let limit = number("--limit", if kind == "all" { 12 } else { 6 }, 1, 30)? as usize;
    let maps = value("--maps", "color");
    let maps: Vec<String> = if maps == "all" {
        vec!["color".into(), "roughness".into(), "normal".into()]
    } else {
        maps.split(',').map(str::to_string).collect()
    };
    if maps
        .iter()
        .any(|m| !["color", "roughness", "normal"].contains(&m.as_str()))
    {
        return Err("--maps 应为 color、roughness、normal 的逗号列表或 all".into());
    }
    let mut unique_maps = Vec::new();
    for m in maps {
        if !unique_maps.contains(&m) {
            unique_maps.push(m);
        }
    }
    Ok(Some(Options {
        command: command.clone(),
        positional,
        kind,
        limit,
        page_only: choice("--use", "all", &["all", "page"])? == "page",
        sheet: flags.contains(&"--sheet"),
        dir: value("--dir", ".phoenix-ui/media"),
        timeout_ms: number("--timeout-ms", 12000, 1000, 120000)?,
        json: flags.contains(&"--json"),
        fetch: fetch::FetchOptions {
            maps: unique_maps,
            resolution: choice("--resolution", "1k", &["1k", "2k", "4k", "8k"])?,
            video_size: choice(
                "--video-size",
                "medium",
                &["medium", "small", "preview", "orig"],
            )?,
            max_mb: number("--max-mb", 256, 1, 2048)?,
            download_timeout_ms: number("--download-timeout-ms", 180000, 1, u64::MAX)?,
        },
    }))
}

/// Runs an explicitly requested media command.
pub fn run(args: &[String], io: &mut Io) -> i32 {
    run_with(args, io, &sources::Endpoints::default())
}

fn run_with(args: &[String], io: &mut Io, endpoints: &sources::Endpoints) -> i32 {
    let options = match parse(args) {
        Ok(Some(options)) => options,
        Ok(None) => {
            io.out(USAGE);
            return 0;
        }
        Err(message) => {
            io.err(&format!("{message}\n"));
            return 1;
        }
    };
    match execute(&options, io, endpoints) {
        Ok((result, code)) => {
            if options.json {
                io.out(&format!("{result}\n"));
            } else {
                print_result(io, &result);
            }
            code
        }
        Err(message) => {
            io.err(&format!("{message}\n"));
            1
        }
    }
}

fn shown(cwd: &Path, path: &Path) -> String {
    let cwd = cwd.to_string_lossy();
    jsp::relative(&cwd, &cwd, &path.to_string_lossy())
}

fn execute(
    options: &Options,
    io: &Io,
    endpoints: &sources::Endpoints,
) -> Result<(Value, i32), String> {
    let area = if options.command == "audit" {
        index::resolve_readonly(&io.cwd, &options.dir)?
    } else {
        index::resolve_area(&io.cwd, &options.dir)?
    };
    let mut result = json!({"schema":"phoenix-ui-media-v1", "command": options.command, "area":shown(&io.cwd,&area)});
    if options.command == "audit" {
        let files = index::read_sources(&area)?;
        let findings = audit::run(&area, &io.cwd, &options.positional)?;
        let code = if findings.is_empty() { 0 } else { 2 };
        if files.is_empty() {
            result["message"] = json!("素材区没有取用记录，无可核对");
        }
        result["findings"] = json!(findings);
        return Ok((result, code));
    }
    let agent = impeccable_context::http::agent_builder()
        .user_agent(&format!(
            "phoenix-ui-media/{} (+https://github.com/XGENT-ai/skills)",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(Duration::from_millis(options.timeout_ms))
        .build();
    let mut errors = Vec::new();
    let mut items = Vec::new();
    if options.command == "search" {
        let query = &options.positional[0];
        let queries: Vec<_> = query.split('|').map(str::trim).collect();
        let tasks: Vec<_> = queries
            .iter()
            .flat_map(|q| sources::tasks(&options.kind, q))
            .collect();
        let per = if options.kind == "all" {
            options.limit.div_ceil(15).max(2)
        } else {
            options.limit
        };
        let mut batches: Vec<_> = tasks.iter().map(|_| None).collect();
        std::thread::scope(|scope| {
            for (task, batch) in tasks.iter().zip(&mut batches) {
                let agent = &agent;
                scope.spawn(move || {
                    *batch = Some(sources::search(agent, endpoints, task, per));
                });
            }
        });
        let mut successes = 0;
        for (task, batch) in tasks.iter().zip(batches) {
            match batch.ok_or("检索任务未返回结果")? {
                Ok(batch) => {
                    successes += 1;
                    items.extend(batch.items);
                    errors.extend(
                        batch
                            .warnings
                            .into_iter()
                            .map(|message| json!({"source":task.provider,"message":message})),
                    );
                }
                Err(message) => errors.push(json!({"source":task.provider,"message":message})),
            }
        }
        result["query"] = json!(query);
        if queries.len() > 1 {
            result["queries"] = json!(queries);
        }
        result["kind"] = json!(options.kind);
        if successes == 0 {
            let all_network = errors.iter().all(|e| {
                e["message"]
                    .as_str()
                    .is_some_and(|m| m.contains("网络连接失败："))
            });
            let providers: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.provider).collect();
            result["message"] = json!(if all_network {
                format!("网络不可用：{} 个信源均无法连接", providers.len())
            } else {
                "全部信源不可用".into()
            });
            result["items"] = json!([]);
            result["count"] = json!(0);
            result["filtered"] = json!(0);
            result["errors"] = json!(errors);
            return Ok((result, 1));
        }
        let (ranked, filtered) = relevance::rank(items, query, usize::MAX);
        items = ranked
            .into_iter()
            .filter(|v| !options.page_only || v["use"] == "page")
            .take(options.limit)
            .collect();
        result["filtered"] = json!(filtered);
        items = index::remember(&area, items)?;
    } else {
        let saved = index::read_index(&area)?;
        for reference in &options.positional {
            match saved.get(reference) {
                Some(item) => items.push(item.clone()),
                None => errors.push(json!({"ref":reference,"message":"编号不在当前素材区索引中"})),
            }
        }
        if items.is_empty() {
            return Err("给出的编号都不在当前素材区索引中".into());
        }
    }
    if options.command == "fetch" {
        let (files, failures) =
            fetch::run(&agent, endpoints, &area, &items, &options.fetch, &io.env)?;
        errors.extend(failures);
        result["files"] = json!(files);
        result["sources"] = json!(shown(&io.cwd, &area.join("sources.json")));
    } else {
        if options.sheet || options.command == "preview" {
            let (sheet, failures) = sheet::render(&agent, &area, &io.cwd, &mut items)?;
            if !sheet.is_null() {
                result["sheet"] = sheet;
            }
            errors.extend(failures);
        }
        result["count"] = json!(items.len());
        result["items"] = json!(items);
    }
    let code = if errors.is_empty() { 0 } else { 2 };
    result["errors"] = json!(errors);
    Ok((result, code))
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
fn print_result(io: &mut Io, result: &Value) {
    if let Some(message) = result["message"].as_str() {
        io.out(&format!("{message}\n"));
    }
    if let Some(items) = result["items"].as_array() {
        for item in items {
            let label = rights::use_label(text(item, "use"));
            let license = text(item, "license");
            let grade = if license.is_empty() {
                label.to_string()
            } else {
                format!("{label}（{license}）")
            };
            io.out(&format!(
                "{}  {}  {}x{}  {}  {}\n",
                text(item, "ref"),
                grade,
                item["originalWidth"],
                item["originalHeight"],
                text(item, "provider"),
                text(item, "title")
            ));
        }
        io.out(&format!("{} 条素材\n", items.len()));
    }
    if let Some(files) = result["files"].as_array() {
        for file in files {
            io.out(&format!(
                "{}/{}  {}x{}  {} 字节  {}\n",
                text(result, "area"),
                text(file, "file"),
                file["width"],
                file["height"],
                file["bytes"],
                rights::use_label(text(file, "use"))
            ));
            if let Some(warnings) = file["warnings"].as_array() {
                for warning in warnings {
                    io.out(&format!("NOTE {}\n", warning.as_str().unwrap_or("")));
                }
            }
        }
        io.out(&format!(
            "已取用 {} 个文件；来源清单：{}\n",
            files.len(),
            text(result, "sources")
        ));
    }
    if let Some(findings) = result["findings"].as_array() {
        for finding in findings {
            io.out(&format!(
                "{} {}{} {}: {}\n",
                text(finding, "kind"),
                text(finding, "file"),
                finding["line"]
                    .as_u64()
                    .map(|n| format!(":{n}"))
                    .unwrap_or_default(),
                text(finding, "ref"),
                text(finding, "match")
            ));
        }
        io.out(&format!("核对发现 {} 项\n", findings.len()));
    }
    if let Some(errors) = result["errors"].as_array() {
        for error in errors {
            let source = text(error, "source");
            io.out(&format!(
                "{} {}: {}\n",
                if source.is_empty() {
                    "FAILED"
                } else {
                    "UNAVAILABLE"
                },
                if source.is_empty() {
                    text(error, "ref")
                } else {
                    source
                },
                text(error, "message")
            ));
        }
    }
    if let Some(file) = result["sheet"]["file"].as_str() {
        io.out(&format!("联系表：{file}\n"));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_arguments_fail_without_creating_an_area() {
        let dir = std::env::temp_dir().join(format!("phoenix-media-args-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for args in [
            vec!["search"],
            vec!["search", "map", "--limit", "0"],
            vec!["search", "map", "--limt", "3"],
            vec!["fetch", "C1", "--dir", "public/img"],
        ] {
            let (mut io, captured) = Io::captured("", dir.clone(), Default::default());
            assert_eq!(
                run(
                    &args.into_iter().map(String::from).collect::<Vec<_>>(),
                    &mut io
                ),
                1
            );
            assert!(!captured.stderr.borrow().is_empty());
            assert!(!dir.join(".phoenix-ui").exists());
            assert!(!dir.join("public").exists());
        }
        let (mut io, captured) = Io::captured("", dir.clone(), Default::default());
        assert_eq!(run(&["--help".into()], &mut io), 0);
        assert!(String::from_utf8_lossy(&captured.stdout.borrow()).contains("仅参考"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn search_isolates_sources_keeps_refs_and_never_serves_cached_results() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr());
        let mut sample: Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/media/api/commons.json"
        ))
        .unwrap();
        let mut first = sample["query"]["pages"][0].clone();
        first["title"] = json!("File:World map");
        first["imageinfo"][0]["url"] = json!(format!("{base}/image"));
        first["imageinfo"][0]["thumburl"] = json!(format!("{base}/image"));
        first["imageinfo"][0]["mime"] = json!("image/png");
        first["imageinfo"][0]["extmetadata"]["ObjectName"]["value"] = json!("World map");
        first["imageinfo"][0]["extmetadata"]["LicenseShortName"]["value"] = json!("CC0");
        let mut second = first.clone();
        second["pageid"] = json!(999);
        second["index"] = json!(2);
        second["imageinfo"][0]["extmetadata"]["LicenseShortName"]["value"] = json!("CC BY-NC");
        sample["query"]["pages"] = json!([first, second]);
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let worker = std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if let Some(request) = server.recv_timeout(Duration::from_millis(25)).unwrap() {
                    let bytes = if request.url().starts_with("/w/api.php") {
                        serde_json::to_vec(&sample).unwrap()
                    } else {
                        include_bytes!("../../../../tests/fixtures/media/sample.png").to_vec()
                    };
                    let _ = request.respond(tiny_http::Response::from_data(bytes));
                }
            }
        });
        let dead = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_url = format!("http://{}", dead.local_addr().unwrap());
        drop(dead);
        let mut endpoints = sources::Endpoints {
            commons: base,
            loc: dead_url.clone(),
            ..Default::default()
        };
        let dir = std::env::temp_dir().join(format!("phoenix-media-search-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let call = |args: &[&str], endpoints: &sources::Endpoints| {
            let (mut io, captured) = Io::captured("", dir.clone(), Default::default());
            let code = run_with(
                &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                &mut io,
                endpoints,
            );
            let out: Value = serde_json::from_slice(&captured.stdout.borrow()).unwrap();
            (code, out)
        };
        let (code, first) = call(
            &["search", "world map", "--kind", "map", "--sheet", "--json"],
            &endpoints,
        );
        assert_eq!(code, 2);
        assert_eq!(first["count"], 2);
        assert_eq!(first["errors"][0]["source"], "loc");
        assert!(dir.join(".phoenix-ui/media/candidates.png").exists());
        assert!(first["items"][0]["look"].is_object());
        let (_, again) = call(
            &["search", "world map", "--kind=map", "--use=page", "--json"],
            &endpoints,
        );
        assert_eq!(again["count"], 1);
        assert_eq!(again["items"][0]["ref"], first["items"][0]["ref"]);
        assert_eq!(again["items"][0]["use"], "page");
        let (mut io, captured) = Io::captured("", dir.clone(), Default::default());
        let code = run_with(
            &["search", "world map", "--kind=map"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            &mut io,
            &endpoints,
        );
        assert_eq!(code, 2);
        let output = captured.stdout.borrow();
        let plain = String::from_utf8_lossy(&output);
        assert!(plain.contains("可上线（CC0）"));
        assert!(plain.contains("仅参考（CC BY-NC）"));
        let before = std::fs::read(dir.join(".phoenix-ui/media/index.json")).unwrap();
        endpoints.commons = dead_url;
        let (code, offline) = call(&["search", "world map", "--kind=map", "--json"], &endpoints);
        assert_eq!(code, 1);
        assert_eq!(offline["count"], 0);
        assert!(offline["message"].as_str().unwrap().contains("网络不可用"));
        assert_eq!(
            std::fs::read(dir.join(".phoenix-ui/media/index.json")).unwrap(),
            before
        );
        let (code, _) = call(
            &[
                "search",
                "world map",
                "--kind=map",
                "--dir=.phoenix-ui/offline",
                "--json",
            ],
            &endpoints,
        );
        assert_eq!(code, 1);
        assert!(!dir.join(".phoenix-ui/offline/index.json").exists());
        stopped.store(true, Ordering::Relaxed);
        worker.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
