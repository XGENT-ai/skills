//! `phoenix-ui compare` in a real browser: a static proposal runs no script,
//! an interactive one does, neither reaches another host, and the choice
//! made on the page comes back through `--wait`. Skips when no browser is
//! installed.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::Value;

fn browser_available() -> bool {
    let env = std::env::vars().collect();
    let ok = impeccable_browser::discovery::find_browser(&env).is_ok();
    if !ok {
        eprintln!("skip: browser unavailable");
    }
    ok
}

fn phoenix(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
        .args(args)
        .current_dir(cwd)
        .env_remove("PHOENIX_UI_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run phoenix-ui");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A loopback listener standing in for "another host": every request line it
/// receives is recorded.
fn probe() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind probe");
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().map_while(Result::ok) {
            let mut line = String::new();
            let _ = BufReader::new(&stream).read_line(&mut line);
            sink.lock().unwrap().push(line.trim().to_string());
        }
    });
    (port, seen)
}

fn centre(png: &Path) -> [u8; 3] {
    let img = image::open(png)
        .unwrap_or_else(|e| panic!("{}: {e}", png.display()))
        .to_rgb8();
    img.get_pixel(img.width() / 2, img.height() / 2).0
}

#[test]
fn static_proposals_run_no_script_and_the_page_choice_reaches_wait() {
    if !browser_available() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("phoenix-compare-browser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (probe_port, probed) = probe();
    // Red until its own script turns it green; the script and the markup both try to reach the probe.
    let page = format!(
        "<!doctype html><html style=\"background:#f00\"><body style=\"margin:0;min-height:100vh\">\
         <img src=\"http://127.0.0.1:{p}/tag.png\" alt=\"\">\
         <script>document.documentElement.style.background='#0f0';\
         fetch('http://127.0.0.1:{p}/fetch').catch(function(){{}});\
         new Image().src='http://127.0.0.1:{p}/image.png';</script></body></html>",
        p = probe_port
    );
    for name in ["static", "live"] {
        std::fs::create_dir_all(dir.join(name)).unwrap();
        std::fs::write(dir.join(name).join("index.html"), &page).unwrap();
    }
    std::fs::write(
        dir.join("round.json"),
        r#"{"proposals":[
            {"id":"static","name":"静态方案","source":"static/index.html"},
            {"id":"live","name":"可交互方案","source":"live/index.html","interactive":true}]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("states.json"),
        r##"{"states":[
            {"name":"static","steps":[
                {"do":"click","target":{"css":"#view-single"}},
                {"do":"click","target":{"css":"#zoom-100"}},
                {"do":"wait","ms":1200}]},
            {"name":"live","steps":[
                {"do":"click","target":{"css":"#view-single"}},
                {"do":"click","target":{"css":"[data-tab=\"live\"]"}},
                {"do":"click","target":{"css":"#zoom-100"}},
                {"do":"wait","ms":1200}]},
            {"name":"choose","steps":[
                {"do":"click","target":{"css":"[data-choose=\"live\"]"}},
                {"do":"fill","target":{"css":"#liked"},"value":"列表清楚"},
                {"do":"click","target":{"css":"#confirm"}},
                {"do":"waitFor","target":{"css":"#done"}}]}]}"##,
    )
    .unwrap();

    let key = "browser";
    let mut server = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
        .args([
            "compare",
            "--detached-serve",
            "--key",
            key,
            "--payload",
            "round.json",
            "--no-open",
            "--timeout",
            "120",
        ])
        .current_dir(&dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn compare");
    let state_path = dir
        .join(".phoenix-ui/compare")
        .join(format!("{key}.state.json"));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !state_path.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let run = || -> Result<(), String> {
        let state: Value =
            serde_json::from_str(&std::fs::read_to_string(&state_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let url = state["url"].as_str().ok_or("no url")?.to_string();
        let (code, stdout, stderr) = phoenix(
            &[
                "capture",
                &url,
                "--viewport",
                "wide=1600x1000",
                "--states",
                "states.json",
                "--only",
                "static,live,choose",
                "--out",
                "shots",
            ],
            &dir,
        );
        if code != 0 {
            return Err(format!("capture exited {code}\n{stdout}\n{stderr}"));
        }
        let [r, g, _] = centre(&dir.join("shots/wide-static.png"));
        if !(r > 200 && g < 60) {
            return Err(format!("the static proposal's script ran or the page did not render: centre is rgb({r},{g},_)"));
        }
        let [r, g, _] = centre(&dir.join("shots/wide-live.png"));
        if !(g > 200 && r < 60) {
            return Err(format!(
                "the interactive proposal's script did not run: centre is rgb({r},{g},_)"
            ));
        }
        let (code, stdout, _) = phoenix(&["compare", "--wait", "--key", key, "--poll", "5"], &dir);
        if code != 0 {
            return Err(format!("--wait exited {code}: {stdout}"));
        }
        let answer: Value = stdout
            .lines()
            .find_map(|l| l.strip_prefix("ANSWER: "))
            .and_then(|raw| serde_json::from_str(raw).ok())
            .ok_or_else(|| format!("no ANSWER line: {stdout}"))?;
        if answer["proposalId"] != "live"
            || answer["liked"] != "列表清楚"
            || answer["baseline"] != false
        {
            return Err(format!("unexpected answer: {answer}"));
        }
        let requests = probed.lock().unwrap().clone();
        if !requests.is_empty() {
            return Err(format!("a proposal reached another host: {requests:?}"));
        }
        Ok(())
    };
    let result = run();
    let _ = server.kill();
    let _ = server.wait();
    let _ = std::fs::remove_dir_all(&dir);
    result.expect("compare browser scenario");
}
