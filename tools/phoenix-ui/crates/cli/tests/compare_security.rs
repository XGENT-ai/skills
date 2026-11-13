//! End-to-end check of the compare server's gates over raw sockets: who may
//! post a choice, and which files a proposal's directory route gives out.

use std::io::{Read, Write};
use std::net::TcpStream;

#[test]
fn concurrent_starts_have_one_owner_and_the_key_can_be_reused_after_exit() {
    let dir = std::env::temp_dir().join(format!("phoenix-compare-start-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.html"), "<p>A</p>").unwrap();
    std::fs::write(dir.join("b.html"), "<p>B</p>").unwrap();
    std::fs::write(dir.join("round.json"), r#"{"proposals":[{"id":"a","name":"A","source":"a.html"},{"id":"b","name":"B","source":"b.html"}]}"#).unwrap();
    let command = || {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_phoenix-ui"));
        cmd.current_dir(&dir)
            .env("PHOENIX_UI_QUESTION_FORCE", "1")
            .env_remove("PHOENIX_UI_QUESTION_DISABLED");
        cmd
    };
    let start = || {
        command()
            .args([
                "compare",
                "--start",
                "--key",
                "shared",
                "--payload",
                "round.json",
                "--timeout",
                "10",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let children: Vec<_> = (0..8).map(|_| start()).collect();
    let outputs: Vec<_> = children
        .into_iter()
        .map(|c| c.wait_with_output().unwrap())
        .collect();
    let result = std::panic::catch_unwind(|| {
        assert_eq!(
            outputs.iter().filter(|o| o.status.success()).count(),
            1,
            "{outputs:?}"
        );
        for output in outputs.iter().filter(|o| !o.status.success()) {
            assert_eq!(output.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("已有运行中的对比页"),
                "{output:?}"
            );
        }
        let state: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join(".phoenix-ui/compare/shared.state.json")).unwrap(),
        )
        .unwrap();
        let port = state["port"].as_u64().unwrap() as u16;
        let winner = outputs.iter().find(|o| o.status.success()).unwrap();
        assert!(String::from_utf8_lossy(&winner.stdout).contains(state["url"].as_str().unwrap()));
        let origin = format!("http://127.0.0.1:{port}");
        assert_eq!(
            raw_request(
                port,
                "POST",
                "/answer?key=shared",
                &[("Content-Type", "application/json"), ("Origin", &origin)],
                Some(r#"{"proposalId":"b"}"#)
            )
            .0,
            200
        );
        let waiting = command()
            .args(["compare", "--wait", "--key", "shared", "--poll", "1"])
            .output()
            .unwrap();
        assert!(waiting.status.success());
        assert!(String::from_utf8_lossy(&waiting.stdout).contains(r#""proposalId":"b""#));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while TcpStream::connect(("127.0.0.1", port)).is_ok()
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
        assert!(
            start().wait_with_output().unwrap().status.success(),
            "key remains reusable"
        );
    });
    let _ = command()
        .args(["compare", "--stop", "--key", "shared"])
        .output();
    // Also end any duplicate service exposed by the regression's failing version.
    for output in &outputs {
        if let Some(url) = String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|l| l.strip_prefix("COMPARE URL: "))
        {
            if let Some(port) = url
                .trim_end_matches('/')
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok())
            {
                if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                    let origin = format!("http://127.0.0.1:{port}");
                    raw_request(
                        port,
                        "POST",
                        "/answer?key=shared",
                        &[("Origin", &origin), ("Content-Type", "application/json")],
                        Some(r#"{"proposalId":"a"}"#),
                    );
                }
            }
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

fn raw_request(
    port: u16,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let body = body.unwrap_or("");
    let mut req = format!("{} {} HTTP/1.1\r\n", method, target);
    let mut has_host = false;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host") {
            has_host = true;
        }
        req.push_str(&format!("{}: {}\r\n", k, v));
    }
    if !has_host {
        req.push_str(&format!("Host: 127.0.0.1:{}\r\n", port));
    }
    req.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    ));
    s.write_all(req.as_bytes()).unwrap();
    let mut out = Vec::new();
    let _ = s.read_to_end(&mut out);
    let text = String::from_utf8_lossy(&out).into_owned();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (status, text)
}

#[test]
fn choice_needs_key_and_loopback_origin_and_files_stay_in_the_proposal_directory() {
    let dir = std::env::temp_dir().join(format!("phoenix-compare-sec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("variants/a/sub")).unwrap();
    std::fs::create_dir_all(dir.join("variants/b")).unwrap();
    std::fs::create_dir_all(dir.join("shots")).unwrap();
    std::fs::write(
        dir.join("variants/a/index.html"),
        "<link rel=stylesheet href=style.css><p>A</p>",
    )
    .unwrap();
    std::fs::write(dir.join("variants/a/style.css"), "p{color:red}").unwrap();
    std::fs::write(dir.join("variants/a/.env"), "TOKEN=1").unwrap();
    std::fs::write(
        dir.join("variants/b/index.html"),
        "<script>document.title='b'</script>",
    )
    .unwrap();
    std::fs::write(dir.join("shots/c.png"), [0x89u8, b'P', b'N', b'G']).unwrap();
    std::fs::write(dir.join("secret.txt"), "outside the proposal directory").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("secret.txt"), dir.join("variants/a/leak.txt")).unwrap();
    let payload = r#"{"proposals":[
        {"id":"a","name":"方案 A","source":"variants/a/index.html"},
        {"id":"b","name":"方案 B","source":"variants/b/index.html","interactive":true},
        {"id":"c","name":"方案 C","source":"shots/c.png"}]}"#;
    std::fs::write(dir.join("round.json"), payload).unwrap();
    let key = "seckey";
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_phoenix-ui"))
        .args([
            "compare",
            "--detached-serve",
            "--key",
            key,
            "--payload",
            "round.json",
            "--no-open",
            "--timeout",
            "60",
        ])
        .current_dir(&dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn compare");

    let state_path = dir
        .join(".phoenix-ui/compare")
        .join(format!("{}.state.json", key));
    let answer_path = dir
        .join(".phoenix-ui/compare")
        .join(format!("{}.answer.json", key));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !state_path.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let run = || -> Result<(), String> {
        let state: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&state_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let port = state
            .get("port")
            .and_then(|p| p.as_u64())
            .ok_or("no port")? as u16;
        let json = ("Content-Type", "application/json");
        let origin = format!("http://127.0.0.1:{}", port);
        let keyed = format!("/answer?key={}", key);
        let body = r#"{"proposalId":"b","liked":"  列表清楚  ","disliked":""}"#;

        // Missing or wrong key: 401, and nothing lands on disk.
        let (st, _) = raw_request(port, "POST", "/answer", &[json], Some(body));
        assert_eq!(st, 401, "no key");
        let (st, _) = raw_request(port, "POST", "/answer?key=wrong", &[json], Some(body));
        assert_eq!(st, 401, "wrong key");
        let (st, _) = raw_request(port, "POST", "/heartbeat", &[], None);
        assert_eq!(st, 401, "no-key heartbeat");
        let (st, _) = raw_request(port, "GET", "/status", &[], None);
        assert_eq!(st, 401, "no-key status");

        // Right key but another site's Origin, a sandboxed frame's null Origin, or a foreign Host: 403.
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Origin", "https://evil.example")],
            Some(body),
        );
        assert_eq!(st, 403, "evil origin");
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Origin", "null")],
            Some(body),
        );
        assert_eq!(st, 403, "sandboxed proposal origin");
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Host", &format!("evil.example:{}", port))],
            Some(body),
        );
        assert_eq!(st, 403, "spoofed host");
        let (st, _) = raw_request(
            port,
            "GET",
            "/",
            &[("Host", &format!("evil.example:{}", port))],
            None,
        );
        assert_eq!(st, 403, "spoofed host GET");
        assert!(!answer_path.exists(), "rejected posts wrote an answer");

        // The shell carries the key for its own posts and frames only itself.
        let (st, page) = raw_request(port, "GET", "/", &[], None);
        assert_eq!(st, 200, "shell page");
        assert!(
            page.contains(r#""key":"seckey""#),
            "shell data carries the key"
        );
        assert!(
            page.contains("frame-src 'self';"),
            "shell frames only itself: {}",
            page.lines().take(12).collect::<Vec<_>>().join("\n")
        );
        let (st, _) = raw_request(port, "GET", &format!("/status?key={}", key), &[], None);
        assert_eq!(st, 200, "keyed status");

        // A proposal gets its own directory, under the isolating policy for its kind.
        let (st, page) = raw_request(port, "GET", "/p/0/index.html", &[], None);
        assert_eq!(st, 200, "proposal page");
        assert!(
            page.contains("script-src 'none'")
                && page.contains("connect-src 'none'")
                && page.contains("; sandbox;"),
            "static policy"
        );
        let (st, css) = raw_request(port, "GET", "/p/0/style.css", &[], None);
        assert_eq!(st, 200, "proposal stylesheet");
        assert!(css.contains("text/css"));
        let (st, page) = raw_request(port, "GET", "/p/1/index.html", &[], None);
        assert_eq!(st, 200, "interactive proposal page");
        assert!(
            page.contains("sandbox allow-scripts") && page.contains("connect-src 'none'"),
            "interactive policy"
        );

        // Nothing outside that directory, no dot files, no cross-kind routes.
        for target in [
            "/p/0/../x",
            "/p/0/%2e%2e/x",
            "/p/0/%2e%2e%2f%2e%2e%2fsecret.txt",
            "/p/0/sub%2f..%2f..%2f..%2fsecret.txt",
            "/p/0/sub/../../../secret.txt",
            "/p/0/.env",
            "/p/0/leak.txt",
            "/p/0/",
            "/p/0/missing.css",
            "/p/2/c.png",
            "/p/9/index.html",
            "/img/0",
            "/img/2/extra",
        ] {
            let (st, text) = raw_request(port, "GET", target, &[], None);
            assert_eq!(st, 404, "{target}");
            assert!(
                !text.contains("outside the proposal directory") && !text.contains("TOKEN=1"),
                "{target} leaked a file"
            );
        }
        let (st, _) = raw_request(port, "GET", "/img/2", &[], None);
        assert_eq!(st, 200, "screenshot proposal");

        // A choice must name a proposal of this round and be a JSON object.
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Origin", &origin)],
            Some(r#"{"proposalId":"nope"}"#),
        );
        assert_eq!(st, 400, "unknown proposal");
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Origin", &origin)],
            Some("[1]"),
        );
        assert_eq!(st, 400, "not an object");
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Origin", &origin)],
            Some(&format!(
                r#"{{"proposalId":"b","liked":"{}"}}"#,
                "x".repeat(70 * 1024)
            )),
        );
        assert_eq!(st, 400, "oversized body");
        assert!(!answer_path.exists(), "invalid choices wrote an answer");

        // With the key and a loopback Origin, 200 means the answer is on disk.
        let (st, _) = raw_request(
            port,
            "POST",
            &keyed,
            &[json, ("Origin", &origin)],
            Some(body),
        );
        assert_eq!(st, 200, "keyed answer");
        let answer = std::fs::read_to_string(&answer_path).map_err(|e| e.to_string())?;
        assert_eq!(
            answer.trim(),
            r#"{"proposalId":"b","name":"方案 B","baseline":false,"liked":"列表清楚","disliked":""}"#
        );
        Ok(())
    };
    let result = run();
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
    result.expect("compare security scenario");
}
