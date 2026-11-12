//! Observe command traffic at the proxy boundary, separately from browser traffic.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{json, Value};

static NEXT: AtomicUsize = AtomicUsize::new(0);
// Observed on Linux Chrome / Chromium; see docs/phoenix-ui.md.
#[cfg(target_os = "linux")]
const BROWSER_RESIDUAL_HOSTS: &[&str] = &["update.googleapis.com", "android.clients.google.com"];
#[cfg(not(target_os = "linux"))]
const BROWSER_RESIDUAL_HOSTS: &[&str] = &[];
const PROXY_KEYS: &[&str] = &[
    "ALL_PROXY",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "all_proxy",
    "https_proxy",
    "http_proxy",
];

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phoenix-network-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join(".git")).unwrap();
        std::fs::write(
            path.join("PRODUCT.md"),
            "# Product\nNetwork boundary fixture.\n",
        )
        .unwrap();
        std::fs::write(path.join("DESIGN.md"), "# Design\nUse Georgia for prose.\n").unwrap();
        std::fs::copy(
            fixtures().join("network-contained.html"),
            path.join("page.html"),
        )
        .unwrap();
        Self(path)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media")
}

struct Proxy {
    address: SocketAddr,
    worker: Option<JoinHandle<Vec<String>>>,
}

impl Proxy {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let mut requests = Vec::new();
            for connection in listener.incoming() {
                let mut stream = connection.unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut reader = BufReader::new(&mut stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line.is_empty() {
                    continue;
                }
                if line.trim_end() == "STOP" {
                    break;
                }
                requests.push(line.trim_end().to_owned());
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) if line == "\r\n" || line == "\n" => break,
                        Ok(_) => {}
                    }
                }
                let _ = stream.write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
            requests
        });
        Self {
            address,
            worker: Some(worker),
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn finish(mut self) -> Vec<String> {
        // Joining after the command exits includes every request already accepted;
        // a timed sleep could race the proxy's last response and lose evidence.
        self.stop();
        self.worker.take().unwrap().join().unwrap()
    }

    fn stop(&self) {
        if let Ok(mut stream) = TcpStream::connect(self.address) {
            let _ = stream.write_all(b"STOP\r\n");
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.stop();
            let _ = worker.join();
        }
    }
}

fn command(project: &Project, proxy: Option<&Proxy>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_phoenix-ui"));
    command
        .current_dir(&project.0)
        .env_remove("PHOENIX_UI_HOME");
    for key in PROXY_KEYS {
        command.env_remove(key);
        if let Some(proxy) = proxy {
            command.env(key, proxy.url());
        }
    }
    command.env("NO_PROXY", "").env("no_proxy", "");
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command
}

fn run(mut command: Command, args: &[&str], input: Option<&str>) -> Output {
    command.args(args).stdin(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

fn output_text(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn local_commands_do_not_contact_the_proxy() {
    let project = Project::new();
    let target = project.0.to_str().unwrap();
    let hook = json!({
        "cwd": target,
        "session_id": "phoenix-network-boundary",
        "hook_event_name": "SessionStart"
    })
    .to_string();
    let cases: &[(&[&str], Option<&str>)] = &[
        (&["context", "--target", target], None),
        (&["hook"], Some(&hook)),
        (&["detect", "--no-config", "--json", "page.html"], None),
    ];
    for (args, input) in cases {
        let baseline = run(command(&project, None), args, *input);
        assert!(
            matches!(baseline.status.code(), Some(0 | 2)),
            "{args:?}: {}",
            output_text(&baseline)
        );
        let proxy = Proxy::start();
        let observed = run(command(&project, Some(&proxy)), args, *input);
        let requests = proxy.finish();
        assert_eq!(
            observed.status.code(),
            baseline.status.code(),
            "{args:?}: {}",
            output_text(&observed)
        );
        assert!(
            requests.is_empty(),
            "{args:?} made external requests: {requests:?}"
        );
    }
}

#[test]
fn explicit_media_search_contacts_sources_and_reports_network_unavailable() {
    let project = Project::new();
    let proxy = Proxy::start();
    let output = run(
        command(&project, Some(&proxy)),
        &["media", "search", "map"],
        None,
    );
    let requests = proxy.finish();
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    assert!(
        output_text(&output).contains("网络不可用"),
        "{}",
        output_text(&output)
    );
    assert!(
        requests
            .iter()
            .any(|line| line.starts_with("CONNECT commons.wikimedia.org:443 ")),
        "no media source CONNECT: {requests:?}"
    );
    assert!(!project.0.join(".phoenix-ui/media/index.json").exists());
}

struct Pages {
    server: Arc<tiny_http::Server>,
    stopped: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    url: String,
}

impl Pages {
    fn start() -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let url = format!("http://{}", server.server_addr());
        let worker_server = Arc::clone(&server);
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = Arc::clone(&stopped);
        let worker = thread::spawn(move || {
            while !worker_stopped.load(Ordering::Acquire) {
                let Ok(request) = worker_server.recv() else {
                    break;
                };
                let body = match request.url() {
                    "/network-contained.html" => Some(include_str!(
                        "../../../tests/fixtures/media/network-contained.html"
                    )),
                    "/network-external.html" => Some(include_str!(
                        "../../../tests/fixtures/media/network-external.html"
                    )),
                    _ => None,
                };
                let response = match body {
                    Some(body) => tiny_http::Response::from_string(body).with_header(
                        tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8")
                            .unwrap(),
                    ),
                    None => tiny_http::Response::from_string("").with_status_code(404),
                };
                let _ = request.respond(response);
            }
        });
        Self {
            server,
            stopped,
            worker: Some(worker),
            url,
        }
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.server.unblock();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn hosts(requests: &[String]) -> BTreeSet<String> {
    requests
        .iter()
        .map(|request| {
            let target = request
                .split_whitespace()
                .nth(1)
                .expect("proxy request target");
            let authority = target
                .split_once("://")
                .map_or(target, |(_, rest)| rest)
                .split('/')
                .next()
                .unwrap();
            authority.split(':').next().unwrap().to_owned()
        })
        .collect()
}

#[test]
fn browser_commands_only_request_page_resources_and_documented_residuals() {
    let env = std::env::vars().collect();
    let browser = match impeccable_browser::discovery::find_browser(&env) {
        Ok(browser) => browser,
        Err(error) => {
            eprintln!("skip: browser unavailable: {error}");
            return;
        }
    };
    let version = Command::new(&browser).arg("--version").output().unwrap();
    eprintln!("browser {}: {}", browser.display(), output_text(&version));
    let pages = Pages::start();
    for verb in ["detect", "capture"] {
        for external in [false, true] {
            let project = Project::new();
            let engine_proxy = Proxy::start();
            let browser_proxy = Proxy::start();
            let wrapper = project.0.join("browser.sh");
            let netlog = project.0.join("browser-netlog.json");
            std::fs::write(
                &wrapper,
                format!(
                    "#!/bin/sh\nexec {} --proxy-server={} --log-net-log={} \"$@\"\n",
                    shell_quote(browser.to_str().unwrap()),
                    shell_quote(&browser_proxy.url()),
                    shell_quote(netlog.to_str().unwrap())
                ),
            )
            .unwrap();
            std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
            let page = if external {
                "network-external.html"
            } else {
                "network-contained.html"
            };
            let url = format!("{}/{page}", pages.url);
            let mut cmd = command(&project, Some(&engine_proxy));
            cmd.env("PHOENIX_UI_BROWSER", &wrapper);
            let args = if verb == "detect" {
                vec![
                    "detect",
                    "--no-config",
                    "--json",
                    "--viewport",
                    "800x600",
                    &url,
                ]
            } else {
                vec![
                    "capture",
                    &url,
                    "--viewport",
                    "probe=800x600",
                    "--type",
                    "viewport",
                    "--settle-ms",
                    "0",
                    "--out",
                    "capture",
                ]
            };
            let output = run(cmd, &args, None);
            let engine_requests = engine_proxy.finish();
            let browser_requests = browser_proxy.finish();
            eprintln!("{verb} {page}: engine={engine_requests:?}; browser={browser_requests:?}");
            assert!(std::fs::metadata(&netlog).unwrap().len() > 0);
            if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
                // Preserve browser provenance before an assertion stops the test.
                // The existing workflow uploads this directory on failure too.
                let evidence = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../../local/phoenix-ui/ci-prepare");
                std::fs::create_dir_all(&evidence).unwrap();
                std::fs::copy(
                    &netlog,
                    evidence.join(format!("network-{verb}-{page}.netlog.json")),
                )
                .unwrap();
            }
            assert!(
                engine_requests.is_empty(),
                "engine made external requests: {engine_requests:?}"
            );
            assert!(
                matches!(output.status.code(), Some(0 | 2)),
                "{verb}: {}",
                output_text(&output)
            );
            if verb == "capture" {
                let report: Value = serde_json::from_slice(
                    &std::fs::read(project.0.join("capture/capture-report.json")).unwrap(),
                )
                .unwrap();
                assert!(
                    report["failures"].as_array().unwrap().is_empty(),
                    "{report:#}"
                );
                assert!(project.0.join("capture/probe.png").is_file(), "{report:#}");
            } else {
                serde_json::from_slice::<Value>(&output.stdout).expect("detect returned JSON");
            }
            let observed = hosts(&browser_requests);
            let mut allowed: BTreeSet<String> = BROWSER_RESIDUAL_HOSTS
                .iter()
                .map(|host| (*host).to_owned())
                .collect();
            if external {
                allowed.insert("boundary-probe.invalid".to_owned());
                assert!(
                    observed.contains("boundary-probe.invalid"),
                    "page resource bypassed proxy: {browser_requests:?}"
                );
            }
            assert!(
                observed.is_subset(&allowed),
                "unexpected browser hosts: {:?}",
                observed.difference(&allowed).collect::<Vec<_>>()
            );
        }
    }
}
