// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Paul Bakaus.
// Adapted from tools/phoenix-ui/crates/browser/src/cdp.rs at
// 4a65a1525f9ed03c775a0e18935901e1edbb773f. See LICENSE-APACHE-2.0.
// Phoenix changes: diagram-only protocol subset, 90/15-second bounds, owned
// profile checks, resource events and full-height PNG capture. No UI detector.

use super::{Theme, Viewport};
use serde_json::{json, Map, Value};
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

#[derive(Debug)]
pub(super) struct Error {
    pub code: &'static str,
    pub stage: &'static str,
    pub message: String,
    pub retained: Option<PathBuf>,
    pub resources: Option<Value>,
}

impl Error {
    pub fn new(code: &'static str, stage: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            stage,
            message: message.into(),
            retained: None,
            resources: None,
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DirectoryIdentity {
    path: PathBuf,
    dev: u64,
    ino: u64,
}
impl DirectoryIdentity {
    fn capture(path: &Path) -> std::io::Result<Self> {
        let stat = fs::symlink_metadata(path)?;
        if !stat.is_dir() || stat.file_type().is_symlink() || stat.ino() == 0 {
            return Err(std::io::Error::other("directory identity is unavailable"));
        }
        Ok(Self {
            path: path.to_owned(),
            dev: stat.dev(),
            ino: stat.ino(),
        })
    }
    fn verify(&self) -> std::io::Result<()> {
        if Self::capture(&self.path)? != *self {
            return Err(std::io::Error::other("directory identity changed"));
        }
        Ok(())
    }
}

/// An exclusive creation supplies cleanup ownership, never a caller path.
pub(super) struct OwnedDirectory {
    pub path: PathBuf,
    identity: DirectoryIdentity,
    ancestors: Vec<DirectoryIdentity>,
}
impl OwnedDirectory {
    pub fn create(parent: &Path, prefix: &str) -> Result<Self> {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let parent = fs::canonicalize(parent)
            .map_err(|e| Error::new("browser/profile", "profile", e.to_string()))?;
        let mut ancestors = Vec::new();
        let mut current = Some(parent.as_path());
        while let Some(path) = current {
            ancestors.push(
                DirectoryIdentity::capture(path)
                    .map_err(|e| Error::new("browser/profile", "profile", e.to_string()))?,
            );
            current = path.parent();
        }
        ancestors.reverse();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for _ in 0..128 {
            for item in &ancestors {
                item.verify()
                    .map_err(|e| Error::new("browser/profile", "profile", e.to_string()))?;
            }
            let path = parent.join(format!(
                "{prefix}{}-{stamp}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => {
                    let identity = DirectoryIdentity::capture(&path)
                        .map_err(|e| Error::new("browser/profile", "profile", e.to_string()))?;
                    return Ok(Self {
                        path,
                        identity,
                        ancestors,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(Error::new("browser/profile", "profile", e.to_string())),
            }
        }
        Err(Error::new(
            "browser/profile",
            "profile",
            "Could not reserve a unique browser profile",
        ))
    }
    pub fn cleanup(&self) -> Result<()> {
        for attempt in 0..4 {
            for item in &self.ancestors {
                item.verify().map_err(|e| self.cleanup_error(e))?;
            }
            match self.identity.verify() {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(self.cleanup_error(e)),
                Ok(()) => {}
            }
            match fs::remove_dir_all(&self.path) {
                Ok(()) => return Ok(()),
                Err(e)
                    if attempt < 3
                        && matches!(
                            e.raw_os_error(),
                            Some(libc::EBUSY | libc::EPERM | libc::EACCES | libc::ENOTEMPTY)
                        ) =>
                {
                    std::thread::sleep(Duration::from_millis(100 * (attempt + 1)))
                }
                Err(e) => return Err(self.cleanup_error(e)),
            }
        }
        unreachable!()
    }
    pub fn verify(&self) -> Result<()> {
        for item in &self.ancestors {
            item.verify().map_err(|e| self.cleanup_error(e))?;
        }
        self.identity.verify().map_err(|e| self.cleanup_error(e))
    }
    pub fn remove_empty(&self) -> Result<()> {
        self.verify()?;
        fs::remove_dir(&self.path).map_err(|e| self.cleanup_error(e))
    }
    fn cleanup_error(&self, cause: std::io::Error) -> Error {
        let mut error = Error::new(
            "browser/cleanup",
            "cleanup",
            format!(
                "Temporary browser profile was preserved at {}: {cause}",
                self.path.display()
            ),
        );
        error.retained = Some(self.path.clone());
        error
    }
}

pub(super) fn chrome_args(profile: &Path) -> Vec<String> {
    [
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--disable-background-networking",
        "--disable-component-update",
        "--disable-default-apps",
        "--disable-sync",
        "--metrics-recording-only",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",
        "--force-device-scale-factor=1",
        "--remote-debugging-port=0",
    ]
    .iter()
    .map(|value| value.to_string())
    .chain([
        format!("--user-data-dir={}", profile.display()),
        "about:blank".into(),
    ])
    .collect()
}

struct Process {
    child: Child,
    profile: OwnedDirectory,
    closed: bool,
}
impl Process {
    fn fail(&mut self, failure: Error, startup_state: &str) -> Error {
        let pid = self.child.id();
        let cleanup = self.finish();
        startup_failure(failure, &self.profile, Some(pid), startup_state, cleanup)
    }
    fn finish(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        if self
            .child
            .try_wait()
            .map_err(|e| Error::new("browser/cleanup", "cleanup", e.to_string()))?
            .is_none()
        {
            if let Err(error) = self.child.kill() {
                if !matches!(self.child.try_wait(), Ok(Some(_))) {
                    return Err(Error::new(
                        "browser/cleanup",
                        "cleanup",
                        format!("Could not stop this browser child: {error}"),
                    ));
                }
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    let mut error = Error::new(
                        "browser/cleanup",
                        "cleanup",
                        "Browser child did not exit; its profile was preserved",
                    );
                    error.retained = Some(self.profile.path.clone());
                    return Err(error);
                }
            }
        }
        self.profile.cleanup()?;
        self.closed = true;
        Ok(())
    }
}

fn startup_failure(
    mut failure: Error,
    profile: &OwnedDirectory,
    pid: Option<u32>,
    startup_state: &str,
    cleanup: Result<()>,
) -> Error {
    let mut resources = json!({"processSpawned":pid.is_some(),"childPid":pid,"profile":profile.path,"startupState":startup_state,"cleanupStatus":if cleanup.is_ok(){"pass"}else{"failed"}});
    if let Err(error) = cleanup {
        resources["whileHandling"] =
            json!({"code":failure.code,"phase":failure.stage,"message":failure.message});
        failure = error;
    }
    failure.resources = Some(resources);
    failure
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

pub(super) struct Browser {
    process: Process,
    conn: Connection,
    target: Option<String>,
    session: Option<String>,
    command_timeout: Duration,
}

impl Browser {
    pub fn launch(
        executable: &Path,
        startup_timeout: Duration,
        command_timeout: Duration,
    ) -> Result<Self> {
        let started = Instant::now();
        let profile =
            OwnedDirectory::create(&std::env::temp_dir(), "phoenix-diagram-browser-profile-")?;
        if started.elapsed() >= startup_timeout {
            let failure = Error::new(
                "browser/startup-timeout",
                "startup",
                "Chrome startup deadline expired before the browser process was spawned",
            );
            let cleanup = profile.cleanup();
            return Err(startup_failure(
                failure,
                &profile,
                None,
                "before-spawn",
                cleanup,
            ));
        }
        let mut child = match Command::new(executable)
            .args(chrome_args(&profile.path))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(cause) => {
                let cleanup = profile.cleanup();
                return Err(startup_failure(
                    Error::new(
                        "browser/launch",
                        "launch",
                        format!("Could not launch {}: {cause}", executable.display()),
                    ),
                    &profile,
                    None,
                    "spawn-failed",
                    cleanup,
                ));
            }
        };
        let (tx, rx) = mpsc::channel();
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    match line {
                        Ok(line) => {
                            let _ = tx.send(line);
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        let mut process = Process {
            child,
            profile,
            closed: false,
        };
        let mut tail = String::new();
        let endpoint = loop {
            let remaining = startup_timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                let failure = Error::new(
                    "browser/startup-timeout",
                    "startup",
                    format!(
                        "Chrome startup timed out after {} ms. {tail}",
                        startup_timeout.as_millis()
                    ),
                );
                return Err(process.fail(failure, "waiting-for-devtools"));
            }
            match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(line) => {
                    if let Some(url) = line.strip_prefix("DevTools listening on ") {
                        break url.trim().to_owned();
                    }
                    tail.push_str(&line);
                    tail.push('\n');
                    if tail.len() > 8000 {
                        let start = tail
                            .char_indices()
                            .find(|(i, _)| *i >= tail.len() - 8000)
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        tail.drain(..start);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    std::thread::sleep(remaining.min(Duration::from_millis(10)))
                }
            }
            if let Ok(Some(status)) = process.child.try_wait() {
                let failure = Error::new(
                    "browser/launch",
                    "launch",
                    format!("Chrome exited before DevTools was ready ({status}). {tail}"),
                );
                return Err(process.fail(failure, "exited-before-devtools"));
            }
        };
        let conn =
            match Connection::connect(&endpoint, startup_timeout.saturating_sub(started.elapsed()))
            {
                Ok(conn) => conn,
                Err(failure) => return Err(process.fail(failure, "devtools-advertised")),
            };
        Ok(Self {
            process,
            conn,
            target: None,
            session: None,
            command_timeout,
        })
    }

    pub fn version(&mut self) -> Result<Value> {
        let version =
            self.conn
                .send(None, "Browser.getVersion", json!({}), self.command_timeout)?;
        if !version["product"]
            .as_str()
            .is_some_and(|product| !product.trim().is_empty())
        {
            return Err(Error::new(
                "browser/protocol",
                "cdp",
                "Browser.getVersion returned no browser product/version",
            ));
        }
        Ok(version)
    }

    pub fn open_page(&mut self) -> Result<()> {
        let created = self.conn.send(
            None,
            "Target.createTarget",
            json!({"url":"about:blank"}),
            self.command_timeout,
        )?;
        let target = required_string(&created, "targetId", "Target.createTarget")?;
        self.target = Some(target.clone());
        let attached = self.conn.send(
            None,
            "Target.attachToTarget",
            json!({"targetId":target,"flatten":true}),
            self.command_timeout,
        )?;
        let session = required_string(&attached, "sessionId", "Target.attachToTarget")?;
        self.session = Some(session);
        for method in [
            "Page.enable",
            "Runtime.enable",
            "Network.enable",
            "Log.enable",
        ] {
            self.send_page(method, json!({}))?;
        }
        self.send_page(
            "Network.emulateNetworkConditions",
            // URL block globs also match an embedded SVG's xmlns string.
            // Offline networking preserves file/data resources; attempted
            // remote requests remain failures in the measured event stream.
            json!({"offline":true,"latency":0,"downloadThroughput":-1,"uploadThroughput":-1}),
        )?;
        self.send_page(
            "Emulation.setTouchEmulationEnabled",
            json!({"enabled":false}),
        )?;
        Ok(())
    }

    pub fn inspect(
        &mut self,
        artifact: &Path,
        viewport: Viewport,
        theme: Theme,
    ) -> Result<(Value, Vec<Value>)> {
        self.conn.events.retain(|event| {
            event["method"].as_str().is_some_and(|method| {
                method.starts_with("Network.")
                    || method.starts_with("Runtime.")
                    || method.starts_with("Log.")
            })
        });
        self.send_page("Emulation.setDeviceMetricsOverride", json!({"width":viewport.width,"height":viewport.height,"deviceScaleFactor":1,"mobile":false}))?;
        let mut url = url::Url::from_file_path(artifact).map_err(|_| {
            Error::new(
                "browser/file-url",
                "navigate",
                "The bound artifact path cannot form a file URL",
            )
        })?;
        url.query_pairs_mut().append_pair("theme", theme.as_str());
        let navigation = self.send_page("Page.navigate", json!({"url":url.as_str()}))?;
        if let Some(error) = navigation.get("errorText").and_then(Value::as_str) {
            return Err(Error::new("browser/navigation", "navigate", error));
        }
        self.conn.wait_event(
            self.session.as_deref(),
            "Page.loadEventFired",
            self.command_timeout,
        )?;
        let metrics = self.evaluate(include_str!("probe.js"))?;
        let frame = self.send_page("Page.getFrameTree", json!({}))?;
        let actual_url = frame
            .pointer("/frameTree/frame/url")
            .and_then(Value::as_str);
        if actual_url != Some(url.as_str()) {
            return Err(Error::new(
                "browser/navigation",
                "probe",
                "The page no longer displays the bound artifact file URL",
            ));
        }
        let events = self.conn.events.drain(..).collect();
        Ok((metrics, events))
    }

    pub fn evaluate(&mut self, expression: &str) -> Result<Value> {
        let result = self.send_page(
            "Runtime.evaluate",
            json!({"expression":expression,"awaitPromise":true,"returnByValue":true}),
        )?;
        if let Some(details) = result.get("exceptionDetails") {
            return Err(Error::new(
                "browser/evaluate",
                "probe",
                details
                    .pointer("/exception/description")
                    .and_then(Value::as_str)
                    .or_else(|| details.get("text").and_then(Value::as_str))
                    .unwrap_or("Browser probe threw an exception"),
            ));
        }
        result.pointer("/result/value").cloned().ok_or_else(|| {
            Error::new(
                "browser/evaluate",
                "probe",
                "Browser probe returned no by-value result",
            )
        })
    }

    pub fn screenshot(&mut self, viewport: Viewport, full_page: bool) -> Result<String> {
        let mut params = json!({"format":"png","fromSurface":true,"captureBeyondViewport":false});
        if full_page {
            let layout = self.send_page("Page.getLayoutMetrics", json!({}))?;
            let height = layout
                .pointer("/cssContentSize/height")
                .or_else(|| layout.pointer("/contentSize/height"))
                .and_then(Value::as_f64)
                .filter(|height| height.is_finite() && *height >= f64::from(viewport.height))
                .ok_or_else(|| {
                    Error::new(
                        "browser/capture",
                        "capture",
                        "Chrome returned no finite full-page height",
                    )
                })?;
            params["captureBeyondViewport"] = json!(true);
            params["clip"] =
                json!({"x":0,"y":0,"width":viewport.width,"height":height.ceil(),"scale":1});
        }
        let result = self.send_page("Page.captureScreenshot", params)?;
        required_string(&result, "data", "Page.captureScreenshot")
    }

    pub fn close(&mut self) -> Result<()> {
        if let Some(target) = self.target.take() {
            let _ = self.conn.send(
                None,
                "Target.closeTarget",
                json!({"targetId":target}),
                self.command_timeout.min(Duration::from_secs(1)),
            );
        }
        let _ = self.conn.send(
            None,
            "Browser.close",
            json!({}),
            self.command_timeout.min(Duration::from_secs(1)),
        );
        self.process.finish()
    }

    pub fn take_events(&mut self) -> Vec<Value> {
        self.conn.events.drain(..).collect()
    }

    fn send_page(&mut self, method: &str, params: Value) -> Result<Value> {
        let session = self
            .session
            .as_deref()
            .ok_or_else(|| Error::new("browser/protocol", "cdp", "No attached page session"))?;
        self.conn
            .send(Some(session), method, params, self.command_timeout)
    }
}

fn required_string(value: &Value, key: &str, method: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            Error::new(
                "browser/protocol",
                "cdp",
                format!("{method} returned no {key}"),
            )
        })
}

struct Connection {
    ws: WebSocket<TcpStream>,
    next_id: u64,
    events: VecDeque<Value>,
}
impl Connection {
    fn connect(endpoint: &str, timeout: Duration) -> Result<Self> {
        let started = Instant::now();
        let url = url::Url::parse(endpoint)
            .map_err(|e| Error::new("browser/protocol", "connect", e.to_string()))?;
        if url.scheme() != "ws" || url.username() != "" || url.password().is_some() {
            return Err(Error::new(
                "browser/protocol",
                "connect",
                "Chrome advertised an unexpected DevTools endpoint",
            ));
        }
        let address: SocketAddr = format!(
            "{}:{}",
            url.host_str().unwrap_or(""),
            url.port().unwrap_or(80)
        )
        .parse()
        .map_err(|_| {
            Error::new(
                "browser/protocol",
                "connect",
                "DevTools endpoint is not a numeric loopback address",
            )
        })?;
        if !address.ip().is_loopback() {
            return Err(Error::new(
                "browser/protocol",
                "connect",
                "DevTools endpoint is not loopback",
            ));
        }
        if timeout.is_zero() {
            return Err(Error::new(
                "browser/startup-timeout",
                "connect",
                "Chrome startup deadline expired before connection",
            ));
        }
        let stream = TcpStream::connect_timeout(&address, timeout)
            .map_err(|e| Error::new("browser/protocol", "connect", e.to_string()))?;
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err(Error::new(
                "browser/startup-timeout",
                "handshake",
                "Chrome startup deadline expired before handshake",
            ));
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|e| Error::new("browser/protocol", "connect", e.to_string()))?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|e| Error::new("browser/protocol", "connect", e.to_string()))?;
        let _ = stream.set_nodelay(true);
        let request = endpoint
            .into_client_request()
            .map_err(|e| Error::new("browser/protocol", "connect", e.to_string()))?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(256 * 1024 * 1024))
            .max_frame_size(Some(256 * 1024 * 1024));
        let (ws, _) = tungstenite::client::client_with_config(request, stream, Some(config))
            .map_err(|e| Error::new("browser/protocol", "handshake", e.to_string()))?;
        Ok(Self {
            ws,
            next_id: 0,
            events: VecDeque::new(),
        })
    }

    fn send(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let deadline = Instant::now() + timeout;
        self.next_id += 1;
        let id = self.next_id;
        let mut request = Map::new();
        request.insert("id".into(), json!(id));
        request.insert("method".into(), json!(method));
        request.insert("params".into(), params);
        if let Some(session) = session {
            request.insert("sessionId".into(), json!(session));
        }
        self.ws
            .get_ref()
            .set_write_timeout(Some(timeout))
            .map_err(|e| Error::new("browser/protocol", "cdp", format!("{method}: {e}")))?;
        self.ws
            .send(Message::Text(Value::Object(request).to_string().into()))
            .map_err(|e| Error::new("browser/protocol", "cdp", format!("{method}: {e}")))?;
        loop {
            let message = self.read(deadline, method)?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = message.get("error") {
                    return Err(Error::new(
                        "browser/protocol",
                        "cdp",
                        format!(
                            "{method}: {}",
                            error
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("protocol error")
                        ),
                    ));
                }
                return message.get("result").cloned().ok_or_else(|| {
                    Error::new(
                        "browser/protocol",
                        "cdp",
                        format!("{method} returned no result"),
                    )
                });
            }
            if message.get("method").is_some() {
                self.events.push_back(message);
            }
        }
    }

    fn wait_event(
        &mut self,
        session: Option<&str>,
        method: &str,
        timeout: Duration,
    ) -> Result<Value> {
        let matches = |value: &Value| {
            value.get("method").and_then(Value::as_str) == Some(method)
                && value.get("sessionId").and_then(Value::as_str) == session
        };
        if let Some(index) = self.events.iter().position(matches) {
            return Ok(self.events.remove(index).unwrap());
        }
        let deadline = Instant::now() + timeout;
        loop {
            let message = self.read(deadline, method)?;
            if matches(&message) {
                return Ok(message);
            }
            self.events.push_back(message);
        }
    }

    fn read(&mut self, deadline: Instant, method: &str) -> Result<Value> {
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::new(
                    "browser/cdp-timeout",
                    "cdp",
                    format!("{method}: protocol deadline expired"),
                ));
            }
            self.ws
                .get_ref()
                .set_read_timeout(Some(remaining))
                .map_err(|e| Error::new("browser/protocol", "cdp", e.to_string()))?;
            match self.ws.read() {
                Ok(Message::Text(message)) => {
                    return serde_json::from_str(&message).map_err(|e| {
                        Error::new(
                            "browser/protocol",
                            "cdp",
                            format!("{method}: invalid protocol JSON: {e}"),
                        )
                    })
                }
                Ok(Message::Close(_)) => {
                    return Err(Error::new(
                        "browser/protocol",
                        "cdp",
                        format!("{method}: Chrome closed the protocol connection"),
                    ))
                }
                Ok(_) => continue,
                Err(tungstenite::Error::Io(error))
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Err(Error::new(
                        "browser/cdp-timeout",
                        "cdp",
                        format!("{method}: protocol deadline expired"),
                    ))
                }
                Err(error) => {
                    return Err(Error::new(
                        "browser/protocol",
                        "cdp",
                        format!("{method}: {error}"),
                    ))
                }
            }
        }
    }
}
