//! `phoenix-ui capture`: drive a page into named states, screenshot each at
//! several viewports, and write a provenance report. It proves only that the
//! page rendered like this after these steps and that the picture is not
//! broken. Whether it shows the claimed state, and whether that is good, is
//! for the caller and the reviewer to decide; the command never scores.

mod page_js;
pub mod plan;
mod report;
mod serve;
mod steps;
mod validity;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use impeccable_browser::cdp::{Browser, CdpError, Page, Viewport};
use impeccable_browser::{BrowserEngine, ScanCredentials};
use impeccable_common::Io;
use impeccable_comp::raster::Image;
use impeccable_core::findings::Finding;
use impeccable_detect::config::{filter_detection_findings, DetectionConfig};
use impeccable_detect::ScanOptions;
use serde_json::{json, Map, Value};

use plan::{Plan, Shape, ShotType, State};
use validity::{CanvasBox, Framing, ShotMeasure, ShotSpec};

const NAVIGATION_TIMEOUT: Duration = Duration::from_secs(30);

pub const USAGE: &str = "用法：phoenix-ui capture <目标> [选项]
      phoenix-ui capture stitch <图片> <图片>... --match <px,px,...> [--crop X,Y,W,H] [--label a,b,...] [--columns N] --out <文件.png>

按步骤把页面操作到命名状态，在多个视口下截图，并逐张写出出处报告。
命令只证明“页面在这些步骤后渲染成这样、截图本身没坏”；图里是不是所称的状态、好不好，
由调用方开图判断。它不打分，也不读写项目源码。
与 capture-server（内部截图服务）和 component-review capture（组件评审取证）是不同的命令。

目标：
  http(s)://… 或 file://…    原样打开；URL 里的 user:pass@ 只注入同源请求，不写进报告
  本地 .html/.htm 文件或目录 由内置静态服务器经 http://127.0.0.1:<随机端口> 提供，目录入口为 index.html

选项：
  --viewport <列表>     逗号分隔：desktop（1440x900）、mobile（390x844）、<宽>x<高>、<标签>=<宽>x<高>；
                        默认 desktop,mobile；宽高 100–4096
  --dpr <n>             像素比 1–4，默认 1
  --color-scheme <值>   light 或 dark，默认 light
  --reduced-motion      仿真 prefers-reduced-motion: reduce
  --states <文件>       状态文件（见下）；不给时只截初始状态
  --only <名,...>       只截列出的状态；initial 表示初始状态
  --type <类型>         状态未声明类型时的截图类型：viewport（默认）或 full
  --out <目录>          输出目录，默认 .phoenix-ui/capture
  --name <模板>         文件名模板，默认 {viewport}{-state}.png；占位符 {viewport} {state}
                        {-state} {width} {height} {dpr} {scheme}；展开后重名会在启动前报错
  --report <文件>       报告路径，默认 <out>/capture-report.json；已存在时按文件名合并
  --block-writes        中止页面、iframe 与 worker 发出的所有非 GET 请求并逐条记录
  --allow-write <路径>  与 --block-writes 同用：放行该 URL 路径（末尾 * 为前缀匹配），可重复
  --settle-ms <ms>      截图前等待有限动画结束的预算，默认 2000
  --timeout-ms <ms>     waitFor 的默认超时，默认 5000
  --mask                另出遮字版 <名>.mask.png（文字透明，图形与图片保留）
  --thumbs              另出扫视缩略图 <名>.thumb.png（视口宽 ≥600 缩到 480 宽，否则 180 宽）
  --sheet               每个视口另出状态并排图 sheet-<视口>.png
  --json                把报告 JSON 打到 stdout，代替逐行输出
  --help                显示本帮助

状态文件（JSON）：
  {
    \"initial\": { \"type\": \"full\", \"frames\": { \"count\": 8, \"from\": \"entry\" } },
    \"states\": [
      { \"name\": \"open-filter\",
        \"steps\": [
          { \"do\": \"fill\", \"target\": { \"role\": \"textbox\", \"name\": \"用户名\" }, \"value\": \"demo\" },
          { \"do\": \"fill\", \"target\": { \"css\": \"#password\" }, \"env\": \"DEMO_PASSWORD\" },
          { \"do\": \"click\", \"target\": { \"role\": \"button\", \"name\": \"登录\" } },
          { \"do\": \"waitFor\", \"target\": { \"text\": \"任务\" } },
          { \"do\": \"click\", \"target\": { \"role\": \"button\", \"name\": \"筛选\" } }
        ],
        \"type\": \"viewport\", \"frames\": 8,
        \"mark\": [{ \"role\": \"button\", \"name\": \"应用筛选\" }] }
    ]
  }
  每个状态都在新打开的页面（不共享 cookie 与存储）里从头执行自己的步骤。
  状态名：小写字母、数字、连字符；initial 为保留名。
  定位：{\"css\": ...}、{\"text\": ...}（先找文字完全相等的可见元素，再找包含者）、
        {\"role\": ..., \"name\": ...}（name 可省）；都可加 \"nth\": n（可见匹配中的第 n 个，从 1 起）。
        只在主文档里找，不进 iframe 与 Shadow DOM。
  动作 do：click、dblclick、hover（target）；fill（target，value 或 env）；type（value 或 env，可选 target）；
        select（原生 <select> 的 target，value 为选项值或文字）；press（key：Enter、Tab、Escape、Control+A…）；
        drag（target，dx，dy）；scroll（dy，可选 dx、target）；wait（ms ≤10000）；
        waitFor（target，until: visible|hidden，可选 timeoutMs）。
  type：viewport、full 或 {\"element\": <定位>}。
  frames：2–60 帧，或 {\"count\": n, \"from\": \"last-step\"|\"entry\"}（entry 只用于 initial），输出 <名>.frames.png。
  mark：要在标注图 <名>.mark.png 上按顺序编号框出的定位数组。
  env 读取同名环境变量，未设置时该状态直接记为失败；它的值和 URL 里的密码在报告、终端与错误信息中
  一律替换为 ***。

有效性自检：
  步骤全部执行、有限动画已结束、截图区域内文字没有大面积透明、不是整张单色、没有异常全黑带、
  画布已绘出、尺寸与截图类型相符。无效图写成 <名>.invalid.png；同名旧图（含失败状态的旧图）会被删除，
  不会冒充本轮状态。派生图（序列帧、遮字版、缩略图、标注图、并排图）不替代主截图。

报告（schema phoenix-ui-capture-v1）：
  每张图记录目标、实际地址、视口、像素比、配色、截图类型、像素尺寸、拍摄时间、浏览器、命令版本、
  步骤与实际观察、控制台错误、失败的请求与图片、横向溢出、被拦截的写请求、页面规则发现
  （findings：与 detect 相同的规则与项目配置）和派生图；失败状态记在 failures。

退出码：
  0  全部状态完成，主截图全部有效
  1  无法运行：参数或状态文件错误、浏览器不可用、目标无法打开、静态服务器无法启动；不写报告
  2  已运行，但有失败状态、无效图、派生图未产出或规则扫描出错；报告已写

浏览器：使用本机 Chrome、Chromium、Edge 或 Brave（PHOENIX_UI_BROWSER 可指定可执行文件）；
资料目录是临时的，结束后删除；只访问目标页面及它自己加载的资源。
";

#[derive(Debug, Clone, PartialEq)]
struct Vp {
    label: String,
    width: u32,
    height: u32,
}

struct Options {
    target: String,
    viewports: Vec<Vp>,
    dpr: f64,
    scheme: String,
    reduced_motion: bool,
    states: Option<PathBuf>,
    only: Option<Vec<String>>,
    default_type: ShotType,
    out: PathBuf,
    name: String,
    report: PathBuf,
    report_shown: String,
    block_writes: bool,
    allow_write: Vec<String>,
    settle_ms: u64,
    timeout_ms: u64,
    json: bool,
}

const FLAGS: &[&str] = &["--reduced-motion", "--block-writes", "--json", "--help"];
const VALUED: &[&str] = &[
    "--viewport",
    "--dpr",
    "--color-scheme",
    "--states",
    "--only",
    "--type",
    "--out",
    "--name",
    "--report",
    "--allow-write",
    "--settle-ms",
    "--timeout-ms",
];

fn parse_args(args: &[String], cwd: &Path) -> Result<Option<Options>, String> {
    let mut positional = Vec::new();
    let mut values: HashMap<&str, Vec<String>> = HashMap::new();
    let mut flags: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        }
        if !arg.starts_with('-') {
            positional.push(arg.to_string());
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (arg, None),
        };
        if let Some(flag) = FLAGS.iter().find(|f| **f == name) {
            if inline.is_some() {
                return Err(format!("{name} 不带值"));
            }
            flags.push(flag);
        } else if let Some(key) = VALUED.iter().find(|f| **f == name) {
            let value = match inline {
                Some(v) => v,
                None => match args.get(i) {
                    Some(v) if !v.starts_with("--") => {
                        i += 1;
                        v.clone()
                    }
                    _ => return Err(format!("{name} 需要一个值")),
                },
            };
            values.entry(key).or_default().push(value);
        } else {
            let all: Vec<&str> = FLAGS.iter().chain(VALUED).copied().collect();
            let hint = plan::nearest(name, &all)
                .map(|n| format!("，是否想写 {n}？"))
                .unwrap_or_default();
            return Err(format!("未知选项 {name}{hint}"));
        }
    }
    let target = match positional.as_slice() {
        [one] => one.clone(),
        [] => return Err("缺少目标：页面 URL、本地 .html 文件或目录".into()),
        many => return Err(format!("只能给一个目标，收到 {}", many.join(" "))),
    };
    let single = |key: &str| -> Result<Option<String>, String> {
        match values.get(key).map(Vec::as_slice) {
            None => Ok(None),
            Some([v]) => Ok(Some(v.clone())),
            Some(_) => Err(format!("{key} 只能给一次")),
        }
    };
    let number = |key: &str, default: f64, lo: f64, hi: f64| -> Result<f64, String> {
        match single(key)? {
            None => Ok(default),
            Some(v) => match v.parse::<f64>() {
                Ok(n) if n.is_finite() && (lo..=hi).contains(&n) => Ok(n),
                _ => Err(format!("{key} 应为 {lo}–{hi} 的数，收到 {v}")),
            },
        }
    };
    let viewports =
        parse_viewports(&single("--viewport")?.unwrap_or_else(|| "desktop,mobile".into()))?;
    let scheme = single("--color-scheme")?.unwrap_or_else(|| "light".into());
    if scheme != "light" && scheme != "dark" {
        return Err(format!(
            "--color-scheme 只能是 light 或 dark，收到 {scheme}"
        ));
    }
    let default_type = match single("--type")?.as_deref() {
        None | Some("viewport") => ShotType::Viewport,
        Some("full") => ShotType::Full,
        Some(other) => {
            return Err(format!(
                "--type 只能是 viewport 或 full，收到 {other}；元素截图在状态文件里写 \"type\": {{\"element\": <定位>}}"
            ));
        }
    };
    let only = single("--only")?.map(|v| {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });
    if only.as_ref().is_some_and(Vec::is_empty) {
        return Err("--only 需要至少一个状态名".into());
    }
    let out_shown = single("--out")?.unwrap_or_else(|| ".phoenix-ui/capture".into());
    let out = normalize(&cwd.join(&out_shown));
    let report_shown = single("--report")?
        .unwrap_or_else(|| format!("{}/capture-report.json", out_shown.trim_end_matches('/')));
    let block_writes = flags.contains(&"--block-writes");
    let allow_write = values.get("--allow-write").cloned().unwrap_or_default();
    if !allow_write.is_empty() && !block_writes {
        return Err("--allow-write 只和 --block-writes 一起用".into());
    }
    if let Some(bad) = allow_write.iter().find(|p| !p.starts_with('/')) {
        return Err(format!(
            "--allow-write 写 URL 路径，以 / 开头，如 /api/session，收到 {bad}"
        ));
    }
    Ok(Some(Options {
        target,
        viewports,
        dpr: number("--dpr", 1.0, 1.0, 4.0)?,
        scheme,
        reduced_motion: flags.contains(&"--reduced-motion"),
        states: single("--states")?.map(|p| cwd.join(p)),
        only,
        default_type,
        out,
        name: single("--name")?.unwrap_or_else(|| "{viewport}{-state}.png".into()),
        report: normalize(&cwd.join(&report_shown)),
        report_shown,
        block_writes,
        allow_write,
        settle_ms: number("--settle-ms", 2000.0, 0.0, 60000.0)? as u64,
        timeout_ms: number("--timeout-ms", 5000.0, 1.0, 60000.0)? as u64,
        json: flags.contains(&"--json"),
    }))
}

fn parse_viewports(list: &str) -> Result<Vec<Vp>, String> {
    let mut out: Vec<Vp> = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (label, size) = match item {
            "desktop" => ("desktop".to_string(), "1440x900"),
            "mobile" => ("mobile".to_string(), "390x844"),
            _ => match item.split_once('=') {
                Some((label, size)) => {
                    let ok = !label.is_empty()
                        && label
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                    if !ok {
                        return Err(format!("视口标签 {label} 只能用小写字母、数字和连字符"));
                    }
                    (label.to_string(), size)
                }
                None => (item.to_string(), item),
            },
        };
        let dims = size
            .split_once(['x', 'X'])
            .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
            .filter(|(w, h)| (100..=4096).contains(w) && (100..=4096).contains(h));
        let Some((width, height)) = dims else {
            return Err(format!(
                "无法识别视口 {item}：用 desktop、mobile、<宽>x<高> 或 <标签>=<宽>x<高>，宽高 100–4096"
            ));
        };
        if out.iter().any(|v| v.label == label) {
            return Err(format!("视口标签 {label} 重复"));
        }
        out.push(Vp {
            label,
            width,
            height,
        });
    }
    if out.is_empty() {
        return Err("--viewport 需要至少一个视口".into());
    }
    Ok(out)
}

fn format_dpr(dpr: f64) -> String {
    if dpr.fract() == 0.0 {
        format!("{}", dpr as u32)
    } else {
        format!("{dpr}")
    }
}

/// Expand `--name` for one viewport and state (`None` = initial).
fn expand_name(
    template: &str,
    vp: &Vp,
    state: Option<&str>,
    dpr: f64,
    scheme: &str,
) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let end = rest[start..]
            .find('}')
            .map(|e| start + e)
            .ok_or_else(|| format!("--name 模板 {template} 有未闭合的 {{"))?;
        out.push_str(&match &rest[start + 1..end] {
            "viewport" => vp.label.clone(),
            "state" => state.unwrap_or("initial").to_string(),
            "-state" => state.map(|s| format!("-{s}")).unwrap_or_default(),
            "width" => vp.width.to_string(),
            "height" => vp.height.to_string(),
            "dpr" => format_dpr(dpr),
            "scheme" => scheme.to_string(),
            other => {
                return Err(format!(
                    "--name 模板里的 {{{other}}} 不是占位符（可用 {{viewport}} {{state}} {{-state}} {{width}} {{height}} {{dpr}} {{scheme}}）"
                ));
            }
        });
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    if !out.ends_with(".png") || out.len() <= 4 || out.contains(['/', '\\']) || out.starts_with('.')
    {
        return Err(format!(
            "--name 展开为 {out}：文件名须以 .png 结尾，且不含目录"
        ));
    }
    Ok(out)
}

/// Where the page comes from.
struct Source {
    /// What `goto` opens (before a local server starts: empty).
    url: String,
    credentials: Option<ScanCredentials>,
    /// Each finding's `file`: the URL, or `file://` for a local target.
    findings_file: String,
    /// The local file a DESIGN.md lookup starts from.
    local_path: Option<String>,
    /// Local target: (server root, entry file name).
    serve: Option<(PathBuf, String)>,
}

fn resolve_target(target: &str, cwd: &Path) -> Result<Source, String> {
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("file://")
    {
        let (url, credentials) = impeccable_browser::split_scan_url(target);
        let local_path = lower
            .starts_with("file://")
            .then(|| impeccable_detect::cli::file_url_to_local_path(&url))
            .flatten();
        return Ok(Source {
            findings_file: url.clone(),
            url,
            credentials,
            local_path,
            serve: None,
        });
    }
    let path = normalize(&cwd.join(target));
    let (root, entry) = if path.is_dir() {
        if !path.join("index.html").is_file() {
            return Err(format!(
                "目录 {target} 下没有 index.html；直接给出要打开的 .html 文件"
            ));
        }
        (path.clone(), "index.html".to_string())
    } else if path.is_file() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "html" && ext != "htm" {
            return Err(format!("本地目标须是 .html/.htm 文件或目录，收到 {target}"));
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        (
            path.parent().map(Path::to_path_buf).unwrap_or_default(),
            name,
        )
    } else {
        return Err(format!(
            "找不到目标 {target}：给出 http(s):// 或 file:// 地址，或已存在的 .html 文件、目录"
        ));
    };
    let entry_path = root.join(&entry);
    Ok(Source {
        url: String::new(),
        credentials: None,
        findings_file: format!("file://{}", entry_path.display()),
        local_path: Some(entry_path.to_string_lossy().into_owned()),
        serve: Some((root, entry)),
    })
}

/// The basic-auth password as written in the target (percent-encoded form).
fn raw_password(target: &str) -> Option<String> {
    let rest = target.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let userinfo = authority.rsplit_once('@')?.0;
    userinfo.split_once(':').map(|(_, p)| p.to_string())
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// `to` relative to the directory `from` (both absolute, normalized).
fn relative(from: &Path, to: &Path) -> String {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".to_string(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

/// One planned picture: a state at a viewport.
struct Planned<'p> {
    name: String,
    state: Option<&'p State>,
    shape: &'p Shape,
}

impl Planned<'_> {
    fn label(&self) -> &str {
        self.state.map(|s| s.name.as_str()).unwrap_or("initial")
    }
}

struct Ctx<'a> {
    opts: &'a Options,
    source: &'a Source,
    env: &'a HashMap<String, String>,
    scan: ScanOptions,
    config: DetectionConfig,
}

struct Captured {
    png: Vec<u8>,
    image: Image,
    spec: ShotSpec,
    measure: ShotMeasure,
    raw_measure: Value,
    shot_type: ShotType,
    steps: Vec<Value>,
    console: Vec<String>,
    network: Vec<impeccable_browser::network_log::NetworkEntry>,
    blocked: Vec<Value>,
    allowed: Vec<Value>,
    findings: Result<Vec<Finding>, String>,
    captured_at: String,
    url: String,
}

enum Failed {
    /// The page did not load.
    Open(String),
    Step {
        index: usize,
        action: String,
        reason: String,
        hints: Vec<String>,
    },
}

impl From<CdpError> for Failed {
    fn from(e: CdpError) -> Self {
        Failed::Step {
            index: 0,
            action: "capture".into(),
            reason: e.message,
            hints: Vec::new(),
        }
    }
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    let opts = match parse_args(args, &io.cwd) {
        Ok(Some(opts)) => opts,
        Ok(None) => {
            io.out(USAGE);
            return 0;
        }
        Err(message) => {
            io.err(&format!(
                "capture: {message}\n运行 phoenix-ui capture --help 查看用法\n"
            ));
            return 1;
        }
    };
    let mut secrets = Vec::new();
    match capture(&opts, io, &mut secrets) {
        Ok(code) => code,
        Err(message) => {
            io.err(&format!(
                "capture: {}\n",
                report::redact_str(&message, &secrets)
            ));
            1
        }
    }
}

fn capture(opts: &Options, io: &mut Io, secrets: &mut Vec<String>) -> Result<i32, String> {
    let cwd = io.cwd.clone();
    let mut source = resolve_target(&opts.target, &cwd)?;
    if let Some(c) = &source.credentials {
        secrets.push(c.password.clone());
    }
    secrets.extend(raw_password(&opts.target));
    let plan = match &opts.states {
        None => Plan::default(),
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("无法读取状态文件 {}：{e}", path.display()))?;
            plan::parse_states(&text).map_err(|errors| {
                let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                format!(
                    "状态文件 {} 有 {} 处问题：\n  {}",
                    path.display(),
                    lines.len(),
                    lines.join("\n  ")
                )
            })?
        }
    };
    if let Some(only) = &opts.only {
        for name in only {
            if name != "initial" && !plan.states.iter().any(|s| &s.name == name) {
                let known: Vec<&str> = std::iter::once("initial")
                    .chain(plan.states.iter().map(|s| s.name.as_str()))
                    .collect();
                return Err(format!(
                    "--only 里的 {name} 不在状态文件中（可选：{}）",
                    known.join("、")
                ));
            }
        }
    }
    let wanted = |name: &str| {
        opts.only
            .as_ref()
            .is_none_or(|o| o.iter().any(|n| n == name))
    };
    let mut planned: Vec<(usize, Planned)> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for (v, vp) in opts.viewports.iter().enumerate() {
        let states = std::iter::once((None, &plan.initial))
            .filter(|_| wanted("initial"))
            .chain(
                plan.states
                    .iter()
                    .filter(|s| wanted(&s.name))
                    .map(|s| (Some(s), &s.shape)),
            );
        for (state, shape) in states {
            let name = expand_name(
                &opts.name,
                vp,
                state.map(|s| s.name.as_str()),
                opts.dpr,
                &opts.scheme,
            )?;
            if names.contains(&name) {
                return Err(format!(
                    "--name 模板 {} 让两张图都叫 {name}；加入 {{viewport}} 或 {{-state}} 等占位符区分",
                    opts.name
                ));
            }
            names.push(name.clone());
            planned.push((v, Planned { name, state, shape }));
        }
    }
    // A missing credential fails its state before anything starts.
    let mut missing_env: HashMap<String, (usize, String, String)> = HashMap::new();
    for state in &plan.states {
        for (i, step) in state.steps.iter().enumerate() {
            if let Some(var) = step.env() {
                match io.env.get(var).filter(|v| !v.is_empty()) {
                    Some(value) => secrets.push(value.clone()),
                    None => {
                        missing_env.entry(state.name.clone()).or_insert((
                            i + 1,
                            step.name().to_string(),
                            format!("环境变量 {var} 未设置"),
                        ));
                    }
                }
            }
        }
    }

    let home = io
        .home()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (scan, config) = impeccable_detect::cli::url_scan_options(
        &cwd.to_string_lossy(),
        &home,
        source.local_path.as_deref(),
        Some(&phoenix_ui_rules::PACK),
    );
    let mut browser = BrowserEngine::new(io.env.clone())
        .launch()
        .map_err(|e| format!("浏览器不可用：{}", e.message))?;
    let server = match &source.serve {
        Some((root, entry)) => {
            let server = serve::serve(root).map_err(|e| format!("静态服务器无法启动：{e}"))?;
            source.url = server.url_for(entry);
            Some(server)
        }
        None => None,
    };
    let product = browser
        .version()
        .ok()
        .and_then(|v| v.get("product").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    let ctx = Ctx {
        opts,
        source: &source,
        env: &io.env,
        scan,
        config,
    };

    let mut run = report::Run::default();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut cleared: Vec<String> = Vec::new();
    let mut opened = false;
    let mut scan_failed = false;
    for (v, item) in &planned {
        let vp = &opts.viewports[*v];
        let stem = item.name.trim_end_matches(".png").to_string();
        cleared.push(stem.clone());
        let fail = |index: usize, action: &str, reason: String, hints: Vec<String>| {
            json!({ "state": item.label(), "viewport": vp.label, "file": item.name, "step": index,
                    "do": action, "reason": reason, "hints": hints })
        };
        if let Some((index, action, reason)) = item.state.and_then(|s| missing_env.get(&s.name)) {
            run.failures
                .push(fail(*index, action, reason.clone(), Vec::new()));
            continue;
        }
        let mut page = browser
            .new_isolated_page()
            .map_err(|e| format!("无法打开新页面：{}", e.message))?;
        let outcome = shoot(&mut page, &ctx, vp, item);
        page.close();
        let mut shot = match outcome {
            Ok(shot) => shot,
            Err(Failed::Open(reason)) if !opened => {
                browser.close();
                return Err(format!("目标无法打开：{reason}"));
            }
            Err(Failed::Open(reason)) => {
                run.failures.push(fail(
                    0,
                    "open",
                    format!("页面无法打开：{reason}"),
                    Vec::new(),
                ));
                continue;
            }
            Err(Failed::Step {
                index,
                action,
                reason,
                hints,
            }) => {
                opened = true;
                run.failures.push(fail(index, &action, reason, hints));
                continue;
            }
        };
        opened = true;
        if item.state.is_none() {
            shot.findings = scan_initial(&mut browser, &ctx, vp);
        }
        let reasons = validity::check(&shot.image, &shot.measure, &shot.spec);
        let valid = reasons.is_empty();
        let mut entry = shot_entry(&ctx, vp, item, &shot, &product);
        entry["valid"] = json!(valid);
        entry["invalidReasons"] = json!(reasons);
        if !valid {
            entry["invalidFile"] = json!(format!("{stem}.invalid.png"));
        }
        if let Err(error) = &shot.findings {
            scan_failed = true;
            entry["findingsError"] = json!(error);
        }
        let written = if valid {
            item.name.clone()
        } else {
            format!("{stem}.invalid.png")
        };
        files.push((written, std::mem::take(&mut shot.png)));
        run.shots.push(entry);
    }
    browser.close();
    drop(server);

    std::fs::create_dir_all(&opts.out)
        .map_err(|e| format!("无法创建输出目录 {}：{e}", opts.out.display()))?;
    for stem in &cleared {
        for suffix in [".png", ".invalid.png"] {
            let _ = std::fs::remove_file(opts.out.join(format!("{stem}{suffix}")));
        }
    }
    for (name, bytes) in &files {
        std::fs::write(opts.out.join(name), bytes)
            .map_err(|e| format!("无法写入 {}：{e}", opts.out.join(name).display()))?;
    }

    let existing = std::fs::read_to_string(&opts.report)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let mut head = Map::new();
    head.insert(
        "tool".into(),
        json!({ "name": "phoenix-ui capture", "version": env!("CARGO_PKG_VERSION"), "binarySha256": binary_sha256() }),
    );
    head.insert("updatedAt".into(), json!(impeccable_common::iso_now()));
    let report_dir = opts
        .report
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    head.insert("outDir".into(), json!(relative(&report_dir, &opts.out)));
    for entry in run.shots.iter_mut().chain(run.failures.iter_mut()) {
        report::redact(entry, secrets);
    }
    let mut merged = report::merge(existing, &run, head);
    report::redact(&mut merged, secrets);
    if let Some(parent) = opts.report.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("无法创建报告目录 {}：{e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&merged).unwrap_or_default();
    std::fs::write(&opts.report, format!("{text}\n"))
        .map_err(|e| format!("无法写入报告 {}：{e}", opts.report.display()))?;

    if opts.json {
        io.out(&format!("{text}\n"));
    } else {
        for line in report::lines(&run, &opts.report_shown) {
            io.out(&format!("{}\n", report::redact_str(&line, secrets)));
        }
    }
    let invalid = run.shots.iter().any(|s| s["valid"] != json!(true));
    Ok(if invalid || !run.failures.is_empty() || scan_failed {
        2
    } else {
        0
    })
}

fn binary_sha256() -> String {
    use sha2::{Digest, Sha256};
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .map(|bytes| format!("{:x}", Sha256::digest(&bytes)))
        .unwrap_or_default()
}

/// Device, media, credentials and write block, identical for every page of a run.
fn setup(page: &mut Page<'_>, ctx: &Ctx<'_>, vp: &Vp) -> Result<(), CdpError> {
    page.set_device_metrics(vp.width, vp.height, ctx.opts.dpr)?;
    page.set_media(&ctx.opts.scheme, ctx.opts.reduced_motion)?;
    page.begin_network_log();
    if let Some(c) = &ctx.source.credentials {
        page.apply_origin_scoped_auth(&ctx.source.url, &c.username, &c.password)?;
    }
    if ctx.opts.block_writes {
        page.enable_write_block(ctx.opts.allow_write.clone())?;
    }
    Ok(())
}

fn settle(page: &mut Page<'_>, budget_ms: u64) -> Result<(), CdpError> {
    page.evaluate_value(&format!("({})({budget_ms})", page_js::SETTLE))?;
    Ok(())
}

fn shoot(
    page: &mut Page<'_>,
    ctx: &Ctx<'_>,
    vp: &Vp,
    item: &Planned<'_>,
) -> Result<Captured, Failed> {
    let opts = ctx.opts;
    setup(page, ctx, vp)?;
    page.goto(&ctx.source.url, "networkidle0", NAVIGATION_TIMEOUT)
        .map_err(|e| Failed::Open(e.message))?;
    let mut steps = Vec::new();
    if let Some(state) = item.state {
        let env = |name: &str| ctx.env.get(name).cloned();
        let step_ctx = steps::Context {
            env: &env,
            default_timeout: Duration::from_millis(opts.timeout_ms),
        };
        for (i, step) in state.steps.iter().enumerate() {
            let outcome = steps::execute(page, step, &step_ctx).map_err(|e| Failed::Step {
                index: i + 1,
                action: step.name().to_string(),
                reason: e.reason,
                hints: e.hints,
            })?;
            let mut record = json!({
                "index": i + 1,
                "do": step.name(),
                "target": step.target().map(|t| t.display()),
                "result": if outcome.warnings.is_empty() { "ok" } else { "warning" },
                "observed": outcome.observed,
            });
            if let plan::Action::Press(_, key) = step {
                record["target"] = json!(key);
            }
            if !outcome.warnings.is_empty() {
                record["warnings"] = json!(outcome.warnings);
            }
            steps.push(record);
        }
    }
    let shot_type = item
        .shape
        .shot_type
        .clone()
        .unwrap_or_else(|| opts.default_type.clone());
    settle(page, opts.settle_ms)?;
    if shot_type == ShotType::Full {
        impeccable_browser::reveal_sweep(page)?;
        settle(page, opts.settle_ms)?;
    }
    let (region, element) = match &shot_type {
        ShotType::Viewport => (Value::Null, None),
        ShotType::Full => (json!("full"), None),
        ShotType::Element(target) => {
            let at = steps::locate(
                page,
                target,
                steps::Use {
                    scroll: true,
                    ..Default::default()
                },
            )
            .map_err(|e| Failed::Step {
                index: 0,
                action: "element".into(),
                reason: format!("截图元素：{}", e.reason),
                hints: e.hints,
            })?;
            let [x, y, w, h] = at.doc;
            let rounded = [x.round(), y.round(), w.round().max(1.0), h.round().max(1.0)];
            (json!(rounded), Some(rounded))
        }
    };
    let raw = page.evaluate_value(&format!("({})({region})", page_js::MEASURE))?;
    let measure = shot_measure(&raw);
    let doc_height = raw["docHeight"]
        .as_f64()
        .unwrap_or(vp.height as f64)
        .max(1.0);
    let (clip, beyond, framing) = match (&shot_type, element) {
        (ShotType::Full, _) => (
            Some([0.0, 0.0, vp.width as f64, doc_height]),
            true,
            Framing::Full {
                doc_height,
                top: 0.0,
            },
        ),
        (ShotType::Element(_), Some(rect)) => (
            Some(rect),
            true,
            Framing::Element {
                width: rect[2],
                height: rect[3],
            },
        ),
        _ => (None, false, Framing::Viewport),
    };
    let b64 = page.screenshot(clip, beyond)?;
    let captured_at = impeccable_common::iso_now();
    let png = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| CdpError::new(format!("截图数据无法解码：{e}")))?;
    let image = impeccable_comp::png_io::decode_png(&png)
        .map_err(|e| CdpError::new(format!("截图不是有效 PNG：{e}")))?
        .image;
    let findings = if item.state.is_some() {
        scan(page, ctx, vp)
    } else {
        Ok(Vec::new())
    };
    let url = page
        .evaluate_value("location.href")
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| ctx.source.url.clone());
    let (blocked, allowed) = page.write_log();
    Ok(Captured {
        png,
        image,
        spec: ShotSpec {
            framing,
            viewport_width: vp.width,
            viewport_height: vp.height,
            dpr: opts.dpr,
        },
        measure,
        raw_measure: raw,
        shot_type,
        steps,
        console: page.console_errors(),
        network: page.network_log(),
        blocked,
        allowed,
        findings,
        captured_at,
        url,
    })
}

fn shot_measure(raw: &Value) -> ShotMeasure {
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    ShotMeasure {
        finite_animations: raw["finiteAnimations"].as_u64().unwrap_or(0),
        text_chars: raw["textChars"].as_u64().unwrap_or(0),
        transparent_chars: raw["transparentChars"].as_u64().unwrap_or(0),
        canvases: raw["canvases"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| CanvasBox {
                selector: c["selector"].as_str().unwrap_or("canvas").to_string(),
                x: num(c, "x"),
                y: num(c, "y"),
                width: num(c, "width"),
                height: num(c, "height"),
            })
            .collect(),
        root_luminance: raw["rootLuminance"].as_f64().unwrap_or(1.0),
    }
}

/// Rule findings for the page as it is now, filtered by the project config.
fn scan(page: &mut Page<'_>, ctx: &Ctx<'_>, vp: &Vp) -> Result<Vec<Finding>, String> {
    let options = ScanOptions {
        viewport: Some((vp.width, vp.height)),
        ..ctx.scan.clone()
    };
    impeccable_browser::scan_loaded_page(
        page,
        &ctx.source.findings_file,
        &options,
        Viewport {
            width: vp.width,
            height: vp.height,
        },
    )
    .map(|f| filter_detection_findings(f, &ctx.config))
    .map_err(|e| e.message)
}

/// The initial state's findings come from a fresh page scanned right after
/// load, the timing `detect <url>` uses, so both report the same findings
/// even when an entrance animation is still running.
fn scan_initial(browser: &mut Browser, ctx: &Ctx<'_>, vp: &Vp) -> Result<Vec<Finding>, String> {
    let mut page = browser.new_isolated_page().map_err(|e| e.message)?;
    let result = setup(&mut page, ctx, vp)
        .and_then(|_| page.goto(&ctx.source.url, "networkidle0", NAVIGATION_TIMEOUT))
        .map_err(|e| e.message)
        .and_then(|_| scan(&mut page, ctx, vp));
    page.close();
    result
}

fn shot_entry(ctx: &Ctx<'_>, vp: &Vp, item: &Planned<'_>, shot: &Captured, product: &str) -> Value {
    let opts = ctx.opts;
    let mut failed_requests: Vec<Value> = Vec::new();
    for request in shot
        .network
        .iter()
        // The browser's own favicon probe is not a request the page made.
        .filter(|n| !n.url.ends_with("/favicon.ico"))
        .filter_map(|n| {
            let url: String = n.url.chars().take(300).collect();
            match (&n.error, n.status) {
                (Some(error), _) if !n.canceled && !error.contains("BLOCKED_BY_CLIENT") => {
                    Some(json!({ "method": n.method, "url": url, "error": error }))
                }
                (None, Some(status)) if status >= 400 => {
                    Some(json!({ "method": n.method, "url": url, "status": status }))
                }
                _ => None,
            }
        })
    {
        if !failed_requests.contains(&request) {
            failed_requests.push(request);
        }
    }
    let mut entry = json!({
        "file": item.name,
        "valid": true,
        "invalidReasons": [],
        "target": opts.target,
        "url": shot.url,
        "state": item.label(),
        "viewport": { "label": vp.label, "width": vp.width, "height": vp.height },
        "dpr": opts.dpr,
        "colorScheme": opts.scheme,
        "reducedMotion": opts.reduced_motion,
        "type": shot.shot_type.name(),
        "pixels": { "width": shot.image.width, "height": shot.image.height },
        "capturedAt": shot.captured_at,
        "browser": { "product": product },
        "steps": shot.steps,
        "consoleErrors": shot.console,
        "failedRequests": failed_requests,
        "failedImages": shot.raw_measure["failedImages"],
        "horizontalOverflow": shot.raw_measure["overflow"],
        "blockedWrites": shot.blocked,
        "allowedWrites": shot.allowed,
        "findings": shot.findings.as_ref().map(|f| json!(f)).unwrap_or_else(|_| json!([])),
        "derived": {},
    });
    if let ShotType::Element(target) = &shot.shot_type {
        entry["element"] = json!(target.display());
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn viewports_and_names() {
        let vps = parse_viewports("desktop,mobile,1280x800,user-1280=1280x720").unwrap();
        let labels: Vec<&str> = vps.iter().map(|v| v.label.as_str()).collect();
        assert_eq!(labels, ["desktop", "mobile", "1280x800", "user-1280"]);
        assert!(parse_viewports("50x50").is_err());
        assert!(parse_viewports("Desk=1x1").is_err());
        assert!(parse_viewports("desktop,desktop").is_err());
        let vp = &vps[2];
        assert_eq!(
            expand_name("{viewport}{-state}.png", vp, None, 1.0, "light").unwrap(),
            "1280x800.png"
        );
        assert_eq!(
            expand_name(
                "U1-{state}-{width}-{scheme}.png",
                vp,
                Some("open"),
                2.0,
                "dark"
            )
            .unwrap(),
            "U1-open-1280-dark.png"
        );
        assert_eq!(
            expand_name(
                "{viewport}-{state}-h{height}-dpr{dpr}.png",
                vp,
                None,
                1.5,
                "light"
            )
            .unwrap(),
            "1280x800-initial-h800-dpr1.5.png"
        );
        assert!(expand_name("{view}.png", vp, None, 1.0, "light").is_err());
        assert!(expand_name("shots/{viewport}.png", vp, None, 1.0, "light").is_err());
        assert!(expand_name("{viewport}.jpg", vp, None, 1.0, "light").is_err());
    }

    #[test]
    fn options_are_checked_before_anything_runs() {
        let cwd = Path::new("/work");
        let err = |a: &[&str]| parse_args(&args(a), cwd).err().unwrap();
        assert_eq!(
            err(&["page.html", "--viewpor", "desktop"]),
            "未知选项 --viewpor，是否想写 --viewport？"
        );
        assert_eq!(err(&[]), "缺少目标：页面 URL、本地 .html 文件或目录");
        assert_eq!(
            err(&["a.html", "--dpr", "5"]),
            "--dpr 应为 1–4 的数，收到 5"
        );
        assert_eq!(
            err(&["a.html", "--allow-write", "/api"]),
            "--allow-write 只和 --block-writes 一起用"
        );
        let opts = parse_args(
            &args(&[
                "a.html",
                "--out",
                "shots",
                "--type=full",
                "--block-writes",
                "--allow-write",
                "/api/*",
            ]),
            cwd,
        )
        .unwrap()
        .unwrap();
        assert_eq!(opts.out, PathBuf::from("/work/shots"));
        assert_eq!(
            opts.report,
            PathBuf::from("/work/shots/capture-report.json")
        );
        assert_eq!(opts.default_type, ShotType::Full);
        assert_eq!(opts.allow_write, ["/api/*"]);
        assert!(parse_args(&args(&["a.html", "--help"]), cwd)
            .unwrap()
            .is_none());
    }

    #[test]
    fn report_paths_and_passwords() {
        assert_eq!(relative(Path::new("/a/out"), Path::new("/a/out")), ".");
        assert_eq!(
            relative(Path::new("/a/out/logs"), Path::new("/a/out")),
            ".."
        );
        assert_eq!(
            relative(Path::new("/a/r"), Path::new("/a/out/x")),
            "../out/x"
        );
        assert_eq!(
            raw_password("https://u:p%40ss@h.example/x").as_deref(),
            Some("p%40ss")
        );
        assert_eq!(raw_password("https://h.example/a@b"), None);
    }
}
