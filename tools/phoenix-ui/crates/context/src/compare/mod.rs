//! `phoenix-ui compare`: built full-page proposals side by side on a loopback
//! page, with the user's pick returned through an answer file.
//!
//! Same two-step shape as `serve-question`: `--start` re-spawns this binary
//! detached (`--detached-serve`) and prints the address and key, `--wait`
//! polls for the answer, `--stop` ends the round.

mod payload;
mod server;

use crate::jsp;
use crate::serve_question::{
    env_set, kill_pid, pid_probe, random_key, read_json_file, sleep_ms, Args,
};
use crate::util::{exists, js_number_to_string, js_trim, json_pretty, now_ms, safe_read};
use impeccable_common::Io;
use serde_json::{json, Map, Value};
use std::path::Path;

const USAGE: &str = "用法：phoenix-ui compare --schema
      phoenix-ui compare --start --payload <文件> [--key <键>] [--open] [--timeout <秒>] [--idle-grace <秒>]
      phoenix-ui compare --wait --key <键> [--poll <秒>]
      phoenix-ui compare --stop --key <键>

把已经做出来的整页方案（本地 HTML 文件、本机地址、截图，可含现状基线）放进一张本机对比页并排展示。
用户在页上切桌面/手机视口、看 100% 实际尺寸、用键盘切换，选定一个并附说明后，选择经本机服务回到调用方。
命令只负责把方案原样摆出来、把用户点的那个带回去；它不评价、不推荐、不替用户选，也不读写项目源码。
至少两个方案才启动。构建之前选方向用 serve-question，元素级变体用 live，组件评审用 component-review。

动作：
  --schema              打印示例载荷与字段说明
  --start               校验载荷，起一个脱离终端、只监听 127.0.0.1 的对比页服务，打印地址与键后返回；
                        随后由调用方为用户打开地址（加 --open 时由命令用系统打开方式打开）
  --wait                取回用户的选择；--poll 秒内没有结果就返回，可重复运行
  --stop                结束该键的服务并丢弃未领取的选择

选项：
  --payload <文件>      载荷 JSON（--start 必需），路径相对当前目录
  --key <键>            这一轮比较的键：1–40 个字母、数字、下划线或连字符；--start 省略时随机生成
  --open                --start 时用系统打开方式打开对比页
  --timeout <秒>        页面一直没有打开时服务保留多久，默认 900；0 表示不限
  --idle-grace <秒>     页面关闭后服务再保留多久（期间重新打开同一地址可继续选），默认 600
  --poll <秒>           --wait 单次等待的时长，默认 60
  --help                显示本帮助

退出码与输出：
  --start   0  COMPARE URL: <地址>、COMPARE KEY: <键>，以及下一步要运行的 --wait 命令
            1  载荷读不到或校验不过（一次列出全部问题）；该键已有运行中的服务或未领取的选择；服务没有就绪
            2  当前会话没有浏览器，没有起服务；改用结构化提问
  --wait    0  ANSWER: {\"proposalId\",\"name\",\"baseline\",\"liked\",\"disliked\"}；
               选的是基线时另有一行 BASELINE KEPT:
            3  WAITING: 还没有选择，再运行一次 --wait
            4  PAGE CLOSED: 对比页已关闭且没有选择；输出里写明怎样再问一次与无回答时的处置
            2  服务已不在且没有选择：这是服务故障，不是用户的决定
  --stop    0  stopped

方案来源（每个方案恰好给 source 或 url 之一）：
  source 为 .html/.htm   由对比页服务提供该文件所在目录里的文件，资源用相对路径；默认不执行脚本、
                         不向对比页服务之外发任何请求；\"interactive\": true 允许内联与同目录的经典脚本，仍不能联网。
                         越出该目录（../）或以 / 开头的资源取不到，模块脚本也取不到：这类页面用本机地址
  source 为图片          .png/.jpg/.jpeg/.webp/.gif，按原图显示；\"pixelRatio\" 写截图时的像素比（1–4）
  url                    只接受 localhost、127.0.0.1、[::1] 的 http(s) 地址；开发服务器要开着，没开时该方案
                         显示不可用与载荷 serve 里的启动说明；页面拒绝被嵌入时用「在新标签页打开」或改用截图
  路径必须在当前目录之内。

状态文件在 .phoenix-ui/compare/：<键>.state.json、<键>.answer.json（--wait 取走后删除）、<键>.log。
没有浏览器的会话（PHOENIX_UI_QUESTION_DISABLED，或 CI、无显示的 SSH/Linux）--start 直接退出 2；
确需启动时设 PHOENIX_UI_QUESTION_FORCE=1。
";

const SCHEMA_NOTE: &str = "
字段说明：
  title、question    可省；title 省略时页面用「方案对比」。
  lang               \"zh-CN\"（默认）或 \"en\"，决定对比页外壳的语言。
  serve              可省；有本机地址方案时写上启动开发服务器的 command 与 cwd，方案连不上时页面显示它。
  proposals          至少两个。每个方案：
    id               必填，字母、数字、下划线或连字符，彼此不同；ANSWER 里原样返回。
    name             必填，显示在栏头。
    source | url     恰好给一个：source 是当前目录内的 .html/.htm 或图片文件，url 是本机地址（localhost、127.0.0.1、[::1]）。
    baseline         最多一个为 true：现状。它排在第一位并带「现状」标记，选它表示保持现状。
    interactive      只用于 HTML 方案：允许它的内联与同目录经典脚本运行，仍不能联网。
    pixelRatio       只用于截图方案：截图时的像素比，1–4，省略为 1；100% 时按“像素宽 ÷ pixelRatio”显示。
    concept、typography、palette、traits
                     方案信息，都可省；concept 与 typography 是一句话，palette 是色值数组，traits 是特征标签数组。
                     省略或为空的项在页面上不显示。
";

fn schema_json() -> Value {
    json!({
      "title": "订单页改版方案",
      "question": "选一个继续做，或保持现状",
      "lang": "zh-CN",
      "serve": { "command": "pnpm dev", "cwd": "." },
      "proposals": [
        { "id": "current", "name": "现状", "url": "http://localhost:5173/orders", "baseline": true },
        { "id": "ledger", "name": "方案 A · 账本", "source": ".phoenix-ui/compare/variants/ledger/index.html",
          "concept": "把订单当作一页流水账来读", "typography": "思源宋体 + JetBrains Mono",
          "palette": ["#1f1c18", "#efe5d0", "#a33327"], "traits": ["密排表格", "单色强调"] },
        { "id": "board", "name": "方案 B · 看板", "source": ".phoenix-ui/compare/variants/board/index.html", "interactive": true },
        { "id": "sketch", "name": "方案 C · 截图", "source": ".phoenix-ui/review/sketch-mobile.png", "pixelRatio": 2 }
      ]
    })
}

pub(crate) fn state_file(qdir: &str, key: &str) -> String {
    jsp::join(&[qdir, &format!("{}.state.json", key)])
}

pub(crate) fn answer_file(qdir: &str, key: &str) -> String {
    jsp::join(&[qdir, &format!("{}.answer.json", key)])
}

fn read_state(qdir: &str, key: &str) -> Option<Map<String, Value>> {
    let text = safe_read(&state_file(qdir, key))?;
    serde_json::from_str::<Value>(&text)
        .ok()?
        .as_object()
        .cloned()
}

fn last_beat(state: &Map<String, Value>) -> Option<f64> {
    state
        .get("lastBeat")
        .and_then(Value::as_f64)
        .filter(|b| *b != 0.0)
}

/// A recent heartbeat, or a live server process.
fn alive(state: &Map<String, Value>) -> bool {
    if last_beat(state).is_some_and(|b| now_ms() - b < 12000.0) {
        return true;
    }
    let Some(pid) = state.get("pid").and_then(Value::as_f64) else {
        return false;
    };
    match pid_probe(pid as i64) {
        Ok(()) => true,
        Err(unsignalable) => unsignalable,
    }
}

const ACTIONS: &[&str] = &[
    "--schema",
    "--start",
    "--wait",
    "--stop",
    "--detached-serve",
];
const SWITCHES: &[&str] = &["--open", "--no-open"];
const VALUED: &[&str] = &["--payload", "--key", "--timeout", "--idle-grace", "--poll"];

fn arg_problems(argv: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut actions: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        i += 1;
        if ACTIONS.contains(&arg) {
            if !actions.contains(&arg) {
                actions.push(arg);
            }
        } else if VALUED.contains(&arg) {
            let Some(value) = argv
                .get(i)
                .filter(|v| !v.is_empty() && !v.starts_with("--"))
            else {
                problems.push(format!("{} 需要一个值", arg));
                continue;
            };
            i += 1;
            if arg == "--key"
                && !(value.len() <= 40
                    && value
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            {
                problems.push("--key 只接受 1–40 个字母、数字、下划线或连字符".into());
            }
            if matches!(arg, "--timeout" | "--idle-grace" | "--poll")
                && !value
                    .parse::<f64>()
                    .is_ok_and(|n| n.is_finite() && n >= 0.0)
            {
                problems.push(format!("{} 需要一个不小于 0 的秒数（{}）", arg, value));
            }
        } else if !SWITCHES.contains(&arg) {
            problems.push(format!("未知的选项或多余的参数：{}", arg));
        }
    }
    let public: Vec<&str> = actions
        .iter()
        .copied()
        .filter(|a| *a != "--detached-serve")
        .collect();
    if actions.is_empty() {
        problems.push("需要 --schema、--start、--wait、--stop 之一".into());
    } else if actions.len() > 1 {
        problems.push(format!(
            "{} 不能同时使用",
            if public.len() > 1 {
                public.join("、")
            } else {
                actions.join("、")
            }
        ));
    }
    problems
}

fn seconds(a: &Args, name: &str, fallback: f64) -> f64 {
    a.arg(name)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(fallback)
}

pub fn run(argv: &[String], io: &mut Io) -> i32 {
    let a = Args { argv };
    if a.has("help") || argv.iter().any(|x| x == "-h") {
        io.out(USAGE);
        return 0;
    }
    let problems = arg_problems(argv);
    if !problems.is_empty() {
        for p in &problems {
            io.err(&format!("compare：{}\n", p));
        }
        io.err(&format!("\n{}", USAGE));
        return 1;
    }
    let cwd = io.cwd.to_string_lossy().into_owned();
    let qdir = jsp::join(&[&cwd, ".phoenix-ui", "compare"]);
    if a.has("schema") {
        io.out(&format!("{}\n{}", json_pretty(&schema_json()), SCHEMA_NOTE));
        return 0;
    }
    if a.has("wait") {
        return wait(&a, io, &qdir);
    }
    if a.has("stop") {
        let Some(key) = a.arg("key") else {
            io.err("compare：--stop 需要 --key <键>\n");
            return 1;
        };
        if let Some(pid) =
            read_state(&qdir, &key).and_then(|s| s.get("pid").and_then(Value::as_f64))
        {
            kill_pid(pid as i64);
        }
        let _ = std::fs::remove_file(answer_file(&qdir, &key));
        let _ = std::fs::remove_file(state_file(&qdir, &key));
        io.out("stopped\n");
        return 0;
    }
    if a.has("start") {
        return start(&a, io, &cwd, &qdir);
    }
    serve_detached(&a, io, &cwd, qdir)
}

fn load_round(io: &mut Io, cwd: &str, payload_path: &str) -> Option<payload::Round> {
    let payload = match read_json_file(&jsp::resolve(cwd, &[payload_path])) {
        Ok(v) => v,
        Err(msg) => {
            io.err(&format!(
                "compare：载荷读不到或不是 JSON（{}）：{}\n",
                payload_path, msg
            ));
            return None;
        }
    };
    match payload::parse(Path::new(cwd), &payload) {
        Ok(round) => Some(round),
        Err(problems) => {
            io.err(&format!(
                "compare：载荷有 {} 个问题，没有启动对比页：\n",
                problems.len()
            ));
            for p in &problems {
                io.err(&format!("  - {}\n", p));
            }
            io.err("运行 phoenix-ui compare --schema 查看示例载荷与字段说明。\n");
            None
        }
    }
}

fn key_available(io: &mut Io, qdir: &str, key: &str) -> bool {
    // An uncollected pick belongs to the previous round; a new round on the
    // same key would hand it to this round's first --wait. Checked before
    // liveness: a server that just answered still has a fresh heartbeat on
    // file, and "stop it first" would discard the pick.
    if exists(&answer_file(qdir, key)) {
        io.err(&format!(
            "compare：键 {} 还留有上一轮未领取的选择；先运行 --wait --key {} 领取，或 --stop --key {} 丢弃，再重新 --start。\n",
            key, key, key
        ));
        return false;
    }
    if read_state(qdir, key).is_some_and(|s| alive(&s)) {
        io.err(&format!(
            "compare：键 {} 已有运行中的对比页；先运行 --stop --key {}，或换一个键。\n",
            key, key
        ));
        return false;
    }
    true
}

fn start(a: &Args, io: &mut Io, cwd: &str, qdir: &str) -> i32 {
    let env = io.env.clone();
    if env_set(&env, "PHOENIX_UI_QUESTION_DISABLED") {
        io.out("compare：本会话已停用浏览器页面（PHOENIX_UI_QUESTION_DISABLED），没有启动对比页；改用结构化提问。\n");
        return 2;
    }
    if !env_set(&env, "PHOENIX_UI_QUESTION_FORCE") {
        let headless = env_set(&env, "CI")
            || (env_set(&env, "SSH_CONNECTION") && !env_set(&env, "DISPLAY"))
            || (cfg!(target_os = "linux")
                && !env_set(&env, "DISPLAY")
                && !env_set(&env, "WAYLAND_DISPLAY"));
        if headless {
            io.out("compare：当前环境没有浏览器（CI、无头或远程会话），没有启动对比页；改用结构化提问。确需启动时设 PHOENIX_UI_QUESTION_FORCE=1。\n");
            return 2;
        }
    }
    let Some(pp) = a.arg("payload") else {
        io.err("compare：--start 需要 --payload <文件>\n");
        return 1;
    };
    if load_round(io, cwd, &pp).is_none() {
        return 1;
    }
    let key = a.arg("key").unwrap_or_else(random_key);
    if !key_available(io, qdir, &key) {
        return 1;
    }
    if let Err(e) = std::fs::create_dir_all(qdir) {
        io.err(&format!("compare：无法创建状态目录：{}\n", e));
        return 1;
    }
    let log_file = jsp::join(&[qdir, &format!("{}.log", key)]);
    let log = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_file);
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "phoenix-ui".to_string());
    let mut child_args: Vec<String> = vec![
        "compare".into(),
        "--detached-serve".into(),
        "--key".into(),
        key.clone(),
        "--payload".into(),
        pp.clone(),
        "--timeout".into(),
        js_number_to_string(seconds(a, "timeout", 900.0)),
    ];
    if let Some(g) = a.arg("idle-grace") {
        child_args.push("--idle-grace".into());
        child_args.push(g);
    }
    if !a.has("open") {
        child_args.push("--no-open".into());
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&child_args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null());
    match log {
        Ok(f) => {
            let f2 = f.try_clone().ok();
            cmd.stdout(std::process::Stdio::from(f));
            if let Some(f2) = f2 {
                cmd.stderr(std::process::Stdio::from(f2));
            }
        }
        Err(_) => {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
    }
    impeccable_common::proc::detach(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            io.err(&format!("compare：无法启动对比页服务：{}\n", e));
            return 1;
        }
    };
    let deadline = now_ms() + 8000.0;
    let state = loop {
        if let Some(state) = read_state(qdir, &key)
            .filter(|s| s.get("pid").and_then(Value::as_u64) == Some(u64::from(child.id())))
        {
            break Some(state);
        }
        match child.try_wait() {
            Ok(None) if now_ms() < deadline => sleep_ms(100),
            _ => break None,
        }
    };
    let Some(state) = state else {
        // A late child must not publish a service after its caller reports failure.
        let _ = child.kill();
        let _ = child.wait();
        let tail = safe_read(&log_file)
            .map(|t| {
                let lines: Vec<&str> = js_trim(&t).split('\n').collect();
                lines[lines.len().saturating_sub(4)..].join("\n  ")
            })
            .unwrap_or_default();
        io.err(&format!(
            "compare：对比页服务没有就绪{}\n",
            if tail.is_empty() {
                String::new()
            } else {
                format!("\n  {}", tail)
            }
        ));
        let rel = jsp::relative(cwd, cwd, &log_file);
        io.err(&format!(
            "compare：日志在 {}。沙箱内的命令不能监听本机端口时会这样：改用宿主不受限（可联网或不带沙箱）的命令通道重跑一次本命令。\n",
            if rel.is_empty() { log_file.clone() } else { rel }
        ));
        return 1;
    };
    io.out(&format!(
        "COMPARE URL: {}\n",
        state.get("url").and_then(Value::as_str).unwrap_or_default()
    ));
    io.out(&format!("COMPARE KEY: {}\n", key));
    io.out("现在为用户打开这个地址：宿主有内置浏览器就用它，否则用系统打开方式（macOS `open`、Linux `xdg-open`），再不行就把地址给用户。不要替用户在页面上提交选择。\n");
    io.out(&format!(
        "然后用这条命令取回选择：{} --wait --key {}\n",
        crate::provider::detect(&env, cwd).verb_cmd("compare"),
        key
    ));
    0
}

fn wait(a: &Args, io: &mut Io, qdir: &str) -> i32 {
    let Some(key) = a.arg("key") else {
        io.err("compare：--wait 需要 --key <键>\n");
        return 1;
    };
    let poll_sec = seconds(a, "poll", 60.0);
    let deadline = now_ms() + poll_sec * 1000.0;
    loop {
        if let Some(raw) = safe_read(&answer_file(qdir, &key)) {
            let raw = js_trim(&raw).to_string();
            io.out(&format!("ANSWER: {}\n", raw));
            let kept = serde_json::from_str::<Value>(&raw)
                .is_ok_and(|v| v.get("baseline") == Some(&Value::Bool(true)));
            if kept {
                io.out("BASELINE KEPT: 用户选择保持现状，不要修改现有实现\n");
            }
            let _ = std::fs::remove_file(answer_file(qdir, &key));
            let _ = std::fs::remove_file(state_file(qdir, &key));
            return 0;
        }
        let state = read_state(qdir, &key);
        if !state.as_ref().is_some_and(alive) {
            io.out(&format!(
                "compare：对比页服务已不在，而且没有选择。这是服务故障，不是用户的决定：用同一份载荷重新运行 --start --key {}，为用户重新打开地址，再 --wait。\n",
                key
            ));
            return 2;
        }
        if state
            .as_ref()
            .and_then(last_beat)
            .is_some_and(|b| now_ms() - b > 15000.0)
        {
            io.out(&format!(
                "PAGE CLOSED: 对比页已关闭且没有选择。用结构化提问再问一次；仍无回答时：有现状基线就保持现状并在交付里写明待选，没有基线就按你的推荐继续并写明未经用户选择。处理完运行 --stop --key {}\n",
                key
            ));
            return 4;
        }
        if now_ms() >= deadline {
            io.out(&format!(
                "WAITING: {} 秒内还没有选择；再运行一次 --wait --key {}\n",
                js_number_to_string(poll_sec),
                key
            ));
            return 3;
        }
        sleep_ms(1000);
    }
}

fn serve_detached(a: &Args, io: &mut Io, cwd: &str, qdir: String) -> i32 {
    let (Some(key), Some(pp)) = (a.arg("key"), a.arg("payload")) else {
        io.err("compare：内部启动缺少 --key 或 --payload\n");
        return 1;
    };
    let Some(round) = load_round(io, cwd, &pp) else {
        return 1;
    };
    let lock = match std::fs::create_dir_all(&qdir).and_then(|()| {
        std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(jsp::join(&[&qdir, &format!("{}.lock", key)]))
    }) {
        Ok(file) => file,
        Err(e) => {
            io.err(&format!("compare：无法打开启动锁：{}\n", e));
            return 1;
        }
    };
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            io.err(&format!(
                "compare：键 {} 已有运行中的对比页；换一个键或先 --stop。\n",
                key
            ));
            return 1;
        }
        Err(e) => {
            io.err(&format!("compare：无法锁定键 {}：{}\n", key, e));
            return 1;
        }
    }
    // Keep this inode and hold its OS lock through serving. Process exit releases
    // ownership even after a crash; unlinking it would let a second inode be locked.
    if !key_available(io, &qdir, &key) {
        return 1;
    }
    let idle = seconds(a, "idle-grace", 600.0);
    let opts = server::Options {
        qdir,
        key,
        timeout_sec: seconds(a, "timeout", 900.0),
        idle_grace_ms: (if idle > 0.0 { idle } else { 600.0 }) * 1000.0,
        open: !a.has("no-open"),
    };
    let result = server::serve(round, opts, io);
    drop(lock);
    result
}
