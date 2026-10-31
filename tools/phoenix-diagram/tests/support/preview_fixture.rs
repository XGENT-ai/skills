// Real CLI processes, real files and raw loopback HTTP. No browser or fake worker.
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    os::unix::{fs::DirBuilderExt, fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

// Debug executables are large and the real worker rechecks their byte identity.
// This is a process-test deadline, independent of HTTP and shutdown deadlines.
const DEADLINE: Duration = Duration::from_secs(60);
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Workspace(pub PathBuf);
impl Workspace {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phoenix-preview-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    pub fn replace(&self, path: &Path, bytes: &[u8]) {
        let replacement = self.path(&format!("replace-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::write(&replacement, bytes).unwrap();
        fs::rename(replacement, path).unwrap();
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn source(label: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":label,"output":"preview.html","input_contract":"phoenix-v1","quality_profile":"standard","legend":{"mode":"hidden"}},"components":[{"id":"service","type":"backend","label":label,"pos":[120,120]}],"connections":[]})).unwrap()
}

pub struct Session {
    pub child: Child,
    pub url: url::Url,
    pub events: Vec<Value>,
    receiver: mpsc::Receiver<Result<Value, String>>,
    stderr: Arc<Mutex<String>>,
    _engine: Workspace,
}
impl Session {
    pub fn start(root: &Workspace, input: &Path, output: &Path) -> Self {
        let engine = Workspace::new();
        let executable = engine.path("phoenix-diagram");
        // A distinct, closed executable inode preserves the runtime's binding
        // contract and avoids inheriting a writable descriptor during exec.
        assert!(Command::new("/bin/cp")
            .arg(env!("CARGO_BIN_EXE_phoenix-diagram"))
            .arg(&executable)
            .status()
            .unwrap()
            .success());
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let mut child = Command::new(executable)
            .arg("preview")
            .arg("architecture")
            .arg(input)
            .arg(output)
            .args(["--input-contract", "phoenix-v1", "--port", "0", "--json"])
            .current_dir(&root.0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let err = child.stderr.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let result = line.map_err(|e| e.to_string()).and_then(|s| {
                    serde_json::from_str(&s).map_err(|e| format!("Non-NDJSON stdout: {e}: {s}"))
                });
                if sender.send(result).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(String::new()));
        let collected = stderr.clone();
        thread::spawn(move || {
            let mut text = String::new();
            let _ = BufReader::new(err).read_to_string(&mut text);
            *collected.lock().unwrap() = text;
        });
        let mut session = Self {
            child,
            url: url::Url::parse("http://127.0.0.1/").unwrap(),
            events: vec![],
            receiver,
            stderr,
            _engine: engine,
        };
        let started = session.next("started", 0);
        session.url = url::Url::parse(started["url"].as_str().expect("started URL")).unwrap();
        assert_eq!(session.url.scheme(), "http");
        assert_eq!(session.url.host_str(), Some("127.0.0.1"));
        assert!(session.url.port().is_some_and(|port| port > 0));
        assert!(!session.url.path().trim_matches('/').is_empty());
        session
    }
    pub fn next(&mut self, event: &str, after: u64) -> Value {
        let until = Instant::now() + DEADLINE;
        loop {
            let remaining = until.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "Timeout waiting {event} after {after}; {}",
                self.debug()
            );
            let value = self
                .receiver
                .recv_timeout(remaining)
                .unwrap_or_else(|e| panic!("Waiting {event}: {e}; {}", self.debug()))
                .unwrap_or_else(|e| panic!("{e}; {}", self.debug()));
            assert_eq!(value["schemaVersion"], 1, "{value}");
            assert_eq!(value["command"], "preview", "{value}");
            assert!(
                value["sessionId"].as_str().is_some_and(|id| !id.is_empty()),
                "{value}"
            );
            assert!(value["generation"].as_u64().is_some(), "{value}");
            assert!(
                value["lastSuccessfulGeneration"].as_u64().is_some(),
                "{value}"
            );
            if let Some(previous) = self.events.first() {
                assert_eq!(
                    value["sessionId"], previous["sessionId"],
                    "session identity drift"
                );
            }
            self.events.push(value.clone());
            if value["event"] == event
                && (event == "started" || value["generation"].as_u64().unwrap() > after)
            {
                return value;
            }
            assert_ne!(
                value["event"],
                "stopped",
                "Stopped before {event}; {}",
                self.debug()
            );
        }
    }
    pub fn debug(&self) -> String {
        format!(
            "events={:?}; stderr={}",
            self.events,
            self.stderr.lock().unwrap()
        )
    }
    pub fn generation(event: &Value) -> u64 {
        event["generation"].as_u64().unwrap()
    }
    pub fn address(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.url.port().unwrap())
            .parse()
            .unwrap()
    }
    pub fn host(&self) -> String {
        format!("127.0.0.1:{}", self.url.port().unwrap())
    }
    pub fn route(&self, name: &str) -> String {
        self.url.join(name).unwrap().path().to_owned()
    }
    pub fn request(&self, method: &str, path: &str, headers: &[(&str, &str)]) -> Response {
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
            self.host()
        );
        for (key, value) in headers {
            request.push_str(&format!("{key}: {value}\r\n"));
        }
        request.push_str("\r\n");
        self.raw(&request)
    }
    pub fn raw(&self, request: &str) -> Response {
        let mut socket =
            TcpStream::connect_timeout(&self.address(), Duration::from_secs(2)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket.write_all(request.as_bytes()).unwrap();
        let mut bytes = vec![];
        if let Err(error) = socket.read_to_end(&mut bytes) {
            assert!(
                error.kind() == std::io::ErrorKind::ConnectionReset && !bytes.is_empty(),
                "HTTP read: {error}; {}",
                self.debug()
            );
        }
        Response::parse(bytes)
    }
    pub fn status(&self) -> Value {
        let response = self.request("GET", &self.route("status"), &[]);
        assert_eq!(response.status, 200, "{response:?}");
        serde_json::from_slice(&response.body).unwrap()
    }
    pub fn artifact(&self) -> Response {
        self.request("GET", &self.route("artifact"), &[])
    }
    pub fn stop(&mut self, signal: i32, exit_code: i32) -> Value {
        assert_eq!(unsafe { libc::kill(self.child.id() as i32, signal) }, 0);
        let event = self.next("stopped", 0);
        assert_eq!(event["shutdownComplete"], true, "{event}");
        let status = self.wait_exit();
        assert_eq!(status.code(), Some(exit_code), "{status}; {}", self.debug());
        assert!(
            TcpStream::connect_timeout(&self.address(), Duration::from_millis(200)).is_err(),
            "stopped must follow real listener closure"
        );
        event
    }
    fn wait_exit(&mut self) -> ExitStatus {
        let until = Instant::now() + Duration::from_secs(6);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(
                Instant::now() < until,
                "Preview did not exit; {}",
                self.debug()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn unchanged_for(&mut self, generation: u64, last_success: u64, duration: Duration) {
        let until = Instant::now() + duration;
        while Instant::now() < until {
            let state = self.status();
            assert_eq!(state["generation"], generation, "{state}");
            assert_eq!(state["lastSuccessfulGeneration"], last_success, "{state}");
            let remaining = until
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(100));
            match self.receiver.recv_timeout(remaining) {
                Ok(Ok(event)) => {
                    assert_ne!(event["event"], "updated", "same bytes advanced: {event}");
                    self.events.push(event);
                }
                Ok(Err(error)) => panic!("{error}"),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(e) => panic!("preview disconnected: {e}; {}", self.debug()),
            }
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            // The process group was created by this test and contains only its own children.
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
impl Response {
    fn parse(bytes: Vec<u8>) -> Self {
        let split = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .unwrap_or_else(|| {
                panic!("Invalid HTTP response: {}", String::from_utf8_lossy(&bytes))
            });
        let head = std::str::from_utf8(&bytes[..split]).unwrap();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let headers = lines
            .map(|line| {
                let (key, value) = line.split_once(':').unwrap();
                (key.to_ascii_lowercase(), value.trim().to_owned())
            })
            .collect();
        Self {
            status,
            headers,
            body: bytes[split + 4..].to_vec(),
        }
    }
    pub fn safe_headers(&self) {
        assert_eq!(
            self.headers.get("cache-control").map(String::as_str),
            Some("no-store")
        );
        assert_eq!(
            self.headers.get("referrer-policy").map(String::as_str),
            Some("no-referrer")
        );
        assert!(!self.headers.contains_key("access-control-allow-origin"));
    }
}
