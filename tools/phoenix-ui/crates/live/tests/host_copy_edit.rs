use impeccable_live::copy_edit_agent::{
    choose_copy_edit_agent, describe_no_provider_error, run_copy_edit_batch_agent,
    run_copy_edit_post_apply_checks,
};
use impeccable_live::util::Env;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn unknown_live_verb_uses_current_public_identity_and_keeps_exit_code() {
    let (mut io, captured) = impeccable_common::Io::captured("", PathBuf::from("."), Env::new());
    assert_eq!(impeccable_live::run("live-unknown", &[], &mut io), 70);
    assert!(captured.stdout.borrow().is_empty());
    assert_eq!(
        String::from_utf8(captured.stderr.borrow().clone()).unwrap(),
        "phoenix-ui: verb 'live-unknown' is not implemented yet\n"
    );
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("phoenix-ui {name} {} {nanos}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(std::fs::canonicalize(path).unwrap())
    }

    fn cwd(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_secondary_cli_traps(dir: &Path) {
    for name in ["codex", "claude"] {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file = dir.join(name);
            std::fs::write(
                &file,
                "#!/bin/sh\nprintf '%s\\n' invoked >> \"$PHOENIX_UI_COPY_EDIT_TRAP\"\nexit 0\n",
            )
            .unwrap();
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        std::fs::write(
            dir.join(format!("{name}.cmd")),
            "@echo off\r\necho invoked>>\"%PHOENIX_UI_COPY_EDIT_TRAP%\"\r\nexit /b 0\r\n",
        )
        .unwrap();
    }
}

fn run_with_cli_traps(test_name: &str) -> bool {
    if std::env::var_os("PHOENIX_UI_COPY_EDIT_TRAP").is_some() {
        return true;
    }
    let fixture = Fixture::new("secondary runner traps");
    write_secondary_cli_traps(&fixture.0);
    let marker = fixture.0.join("invocations.log");
    for name in ["codex", "claude"] {
        let filename = if cfg!(windows) {
            format!("{name}.cmd")
        } else {
            name.into()
        };
        let status = Command::new(fixture.0.join(filename))
            .arg("--version")
            .env("PHOENIX_UI_COPY_EDIT_TRAP", &marker)
            .status()
            .unwrap();
        assert!(
            status.success() && marker.exists(),
            "CLI trap must record execution"
        );
        std::fs::remove_file(&marker).unwrap();
    }
    let mut paths = vec![fixture.0.clone()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test_name, "--nocapture"])
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env("PHOENIX_UI_COPY_EDIT_TRAP", &marker)
        .env_remove("PHOENIX_UI_LIVE_COPY_AGENT")
        .env_remove("IMPECCABLE_LIVE_COPY_AGENT")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!marker.exists(), "a secondary agent was probed or executed");
    false
}

#[test]
fn copy_edit_never_spawns_secondary_cli() {
    if !run_with_cli_traps("copy_edit_never_spawns_secondary_cli") {
        return;
    }
    // A separate test process gives provider discovery its own PATH, so
    // even the failing-before version cannot contact an installed agent.
    let mut env = impeccable_common::process_env();
    let available = || true;
    assert_eq!(
        choose_copy_edit_agent(&env, Some(&available)).as_deref(),
        Some("chat"),
        "default copy editing must use the current host"
    );
    let batch = json!({"entries": []});
    let mut host = |_: &serde_json::Value, _: Option<&serde_json::Value>| {
        Ok(json!({"status": "done", "appliedEntryIds": [], "files": [], "notes": []}))
    };
    for mode in ["codex", "claude", "veto"] {
        let error = run_copy_edit_batch_agent(
            &batch,
            ".",
            &env,
            Some(mode),
            None,
            Some(&mut host),
            Some(&available),
        )
        .unwrap_err();
        assert!(error.contains("disabled"), "{mode}: {error}");
        env.insert("PHOENIX_UI_LIVE_COPY_AGENT".into(), mode.into());
        assert_eq!(choose_copy_edit_agent(&env, Some(&available)), None);
        let error = run_copy_edit_batch_agent(
            &batch,
            ".",
            &env,
            None,
            None,
            Some(&mut host),
            Some(&available),
        )
        .unwrap_err();
        assert!(error.contains("disabled"), "{mode}: {error}");
    }
    env.remove("PHOENIX_UI_LIVE_COPY_AGENT");
    for mode in ["codex", "claude", "auto", "mock"] {
        env.insert("IMPECCABLE_LIVE_COPY_AGENT".into(), mode.into());
        assert_eq!(
            choose_copy_edit_agent(&env, Some(&available)).as_deref(),
            Some("chat")
        );
        let result = run_copy_edit_batch_agent(
            &batch,
            ".",
            &env,
            None,
            None,
            Some(&mut host),
            Some(&available),
        )
        .unwrap();
        assert_eq!(result["status"], "done");
    }
    let unavailable = || false;
    assert_eq!(choose_copy_edit_agent(&env, Some(&unavailable)), None);
    let guidance = describe_no_provider_error(&env, false);
    assert!(guidance.contains("manually"), "{guidance}");
    assert!(
        !Path::new(&std::env::var("PHOENIX_UI_COPY_EDIT_TRAP").unwrap()).exists(),
        "a secondary agent was probed or executed"
    );
}

fn http(port: u16, method: &str, target: &str, body: Option<serde_json::Value>) -> (u16, String) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    let request = agent.request(method, &format!("http://127.0.0.1:{port}{target}"));
    let response = match body {
        Some(body) => request.send_json(body),
        None => request.call(),
    };
    let response = match response {
        Ok(response) | Err(ureq::Error::Status(_, response)) => response,
        Err(error) => panic!("{method} {target}: {error}"),
    };
    (response.status(), response.into_string().unwrap())
}

struct LiveServer {
    fixture: Fixture,
    port: u16,
    token: String,
    thread: Option<std::thread::JoinHandle<i32>>,
}

impl LiveServer {
    fn start(mode: Option<&str>) -> Self {
        let fixture = Fixture::new("host copy edit");
        std::fs::write(
            fixture.0.join("index.html"),
            "<html><body><h1>Old copy</h1></body></html>\n",
        )
        .unwrap();
        std::fs::write(fixture.0.join("unrelated.txt"), "Keep this file.\n").unwrap();
        std::fs::write(
            fixture.0.join("package.json"),
            json!({"scripts": {"phoenix-ui:manual-edit-validate":
                "node -e \"require('node:fs').writeFileSync('validation-ran.txt', 'yes')\""}})
            .to_string(),
        )
        .unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut env: Env = impeccable_common::process_env();
        env.retain(|key, _| {
            key == "PHOENIX_UI_COPY_EDIT_TRAP"
                || (!key.starts_with("PHOENIX_UI_") && !key.starts_with("IMPECCABLE_"))
        });
        env.insert("IMPECCABLE_LIVE_COPY_AGENT".into(), "codex".into());
        if let Some(mode) = mode {
            env.insert("PHOENIX_UI_LIVE_COPY_AGENT".into(), mode.into());
        }
        env.insert(
            "HOME".into(),
            fixture.0.join("home").to_string_lossy().into_owned(),
        );
        env.insert(
            "USERPROFILE".into(),
            fixture.0.join("home").to_string_lossy().into_owned(),
        );
        let cwd = fixture.0.clone();
        let server_env = env.clone();
        let thread = std::thread::spawn(move || {
            let mut io = impeccable_common::Io::stdio();
            io.stdout = Box::new(std::io::sink());
            io.stderr = Box::new(std::io::sink());
            io.cwd = cwd;
            io.env = server_env;
            impeccable_live::live_server::run(&[format!("--port={port}")], &mut io)
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let info = loop {
            if let Some((info, _)) =
                impeccable_live::paths::read_live_server_info(fixture.cwd(), &env)
            {
                break info;
            }
            assert!(!thread.is_finished(), "live server exited before startup");
            assert!(Instant::now() < deadline, "live server did not start");
            std::thread::sleep(Duration::from_millis(10));
        };
        Self {
            fixture,
            port: info.raw["port"].as_u64().unwrap() as u16,
            token: info.raw["token"].as_str().unwrap().to_string(),
            thread: Some(thread),
        }
    }

    fn json(
        &self,
        method: &str,
        route: &str,
        body: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let route = format!("{route}?token={}", self.token);
        let (status, raw) = http(self.port, method, &route, body);
        assert_eq!(status, 200, "{raw}");
        serde_json::from_str(&raw).unwrap()
    }

    fn stash(&self) -> serde_json::Value {
        self.json("POST", "/manual-edit-stash", Some(json!({
            "token": self.token,
            "id": "aabbccdd", "pageUrl": "/", "element": {"tagName": "h1"},
            "ops": [{"ref": "h1", "tag": "h1", "originalText": "Old copy", "newText": "New copy",
                "sourceHint": {"file": "index.html", "line": 1}}]
        })))
    }
}

impl Drop for LiveServer {
    fn drop(&mut self) {
        let (status, body) = http(
            self.port,
            "POST",
            &format!("/stop?token={}", self.token),
            None,
        );
        assert_eq!(status, 200, "{body}");
        assert_eq!(self.thread.take().unwrap().join().unwrap(), 0);
    }
}

#[test]
fn host_poll_reply_commits_copy_without_a_second_runner() {
    if !run_with_cli_traps("host_poll_reply_commits_copy_without_a_second_runner") {
        return;
    }
    for mode in ["codex", "claude", "veto"] {
        let server = LiveServer::start(Some(mode));
        server.stash();
        let route = format!(
            "/poll?token={}&timeout=1&types=manual_edit_apply",
            server.token
        );
        http(server.port, "GET", &route, None);
        let rejected = server.json("POST", "/manual-edit-commit", None);
        assert_eq!(rejected["cleared"], 0, "{rejected}");
        assert!(
            rejected["failed"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("disabled"),
            "{rejected}"
        );
        assert_eq!(
            server.json("GET", "/manual-edit-stash", None)["totalCount"],
            1
        );
        assert!(!server.fixture.0.join("validation-ran.txt").exists());
    }
    let server = LiveServer::start(None);
    let stashed = server.stash();
    assert_eq!(stashed["pendingCount"], 1);
    let absent_host = server.json("POST", "/manual-edit-commit", None);
    assert_eq!(absent_host["cleared"], 0);
    assert!(
        absent_host["failed"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("manually"),
        "{absent_host}"
    );
    assert!(std::fs::read_to_string(server.fixture.0.join("index.html"))
        .unwrap()
        .contains("Old copy"));

    // A real poll establishes the current host; the browser Apply then
    // hands its staged request to that host rather than another provider.
    let poll_route = format!(
        "/poll?token={}&timeout=1&types=manual_edit_apply",
        server.token
    );
    let (_, first_poll) = http(server.port, "GET", &poll_route, None);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&first_poll).unwrap()["type"],
        "timeout"
    );
    let port = server.port;
    let commit_route = format!("/manual-edit-commit?token={}", server.token);
    let commit = std::thread::spawn(move || http(port, "POST", &commit_route, None));
    let poll_route = format!(
        "/poll?token={}&timeout=3000&leaseMs=10000&types=manual_edit_apply",
        server.token
    );
    let (status, raw) = http(server.port, "GET", &poll_route, None);
    assert_eq!(status, 200, "{raw}");
    let event: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(event["type"], "manual_edit_apply", "{event}");
    assert_eq!(event["batch"]["entries"][0]["id"], "aabbccdd");
    assert!(Path::new(event["evidencePath"].as_str().unwrap()).is_file());
    let (_, other_poll) = http(
        server.port,
        "GET",
        &format!(
            "/poll?token={}&timeout=1&types=manual_edit_apply",
            server.token
        ),
        None,
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&other_poll).unwrap()["type"],
        "timeout",
        "leased requests must not be delivered twice"
    );

    std::fs::write(
        server.fixture.0.join("index.html"),
        "<html><body><h1>New copy</h1></body></html>\n",
    )
    .unwrap();
    let ack = server.json("POST", "/poll", Some(json!({
        "token": server.token, "id": event["id"], "type": "done",
        "data": {"status": "done", "appliedEntryIds": ["aabbccdd"], "failed": [], "files": ["index.html"], "notes": []}
    })));
    assert_eq!(ack["ok"], true);
    let (status, raw) = commit.join().unwrap();
    assert_eq!(status, 200, "{raw}");
    let applied: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(applied["cleared"], 1, "{applied}");
    assert_eq!(applied["failed"], json!([]), "{applied}");
    assert_eq!(
        server.json("GET", "/manual-edit-stash", None)["totalCount"],
        0
    );
    assert_eq!(
        std::fs::read_to_string(server.fixture.0.join("validation-ran.txt")).unwrap(),
        "yes"
    );
    assert_eq!(
        std::fs::read_to_string(server.fixture.0.join("unrelated.txt")).unwrap(),
        "Keep this file.\n"
    );
}

#[test]
fn project_validation_is_preserved_and_phoenix_script_takes_precedence() {
    let fixture = Fixture::new("project validation");
    std::fs::write(fixture.0.join("index.html"), "<h1>Copy</h1>\n").unwrap();
    std::fs::write(fixture.0.join("package.json"), json!({"scripts": {
        "phoenix-ui:manual-edit-validate": "node -e \"process.stderr.write('validation rejected'); process.exit(1)\"",
        "impeccable:manual-edit-validate": "node -e \"process.exit(0)\""
    }}).to_string()).unwrap();
    let result = run_copy_edit_post_apply_checks(fixture.cwd(), &["index.html".into()]);
    assert_eq!(result["ok"], false);
    assert_eq!(
        result["failures"][0]["reason"],
        "manual_edit_validation_failed"
    );
    assert!(result["failures"][0]["message"]
        .as_str()
        .unwrap()
        .contains("validation rejected"));
    std::fs::write(fixture.0.join("package.json"), json!({"scripts": {
        "impeccable:manual-edit-validate": "node -e \"process.stderr.write('legacy validation rejected'); process.exit(1)\""
    }}).to_string()).unwrap();
    let result = run_copy_edit_post_apply_checks(fixture.cwd(), &["index.html".into()]);
    assert_eq!(result["ok"], false);
    assert!(result["failures"][0]["message"]
        .as_str()
        .unwrap()
        .contains("legacy validation rejected"));
}
