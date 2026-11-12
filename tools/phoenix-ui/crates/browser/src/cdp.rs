//! Headless Chrome launch and a small Chrome DevTools Protocol client.
//!
//! Why hand-written over `tungstenite` rather than `headless_chrome`: the
//! JS engine's observable behaviour is puppeteer's (its launch flags,
//! `newPage` setup, `goto` lifecycle semantics, `evaluate` error text,
//! `screenshot` parameters). `headless_chrome` re-implements those with its
//! own defaults (different flags, its own navigation wait, a tab abstraction
//! that hides lifecycle events, and a `fetch` feature that downloads
//! browsers), so matching puppeteer would mean fighting the crate. The
//! protocol surface we need is tiny (a dozen methods, six events), and a
//! flat JSON-RPC layer over one websocket keeps every CDP call visible and
//! auditable against puppeteer's source.
//!
//! Everything here is synchronous: one websocket, one thread; events that
//! arrive while a command is in flight are queued and drained by the page
//! state machine after every call.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use impeccable_core::browser::dom::RenderedFont;
use serde_json::{Map, Value, json};
use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

/// puppeteer's `launch` timeout (`timeout: 30000` in BrowserLauncher).
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
/// puppeteer's per-command `protocolTimeout` (180 s).
const PROTOCOL_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone)]
pub struct CdpError {
    pub message: String,
}

impl CdpError {
    pub fn new(message: impl Into<String>) -> Self {
        CdpError {
            message: message.into(),
        }
    }
}

impl From<String> for CdpError {
    fn from(message: String) -> Self {
        CdpError { message }
    }
}

pub type CdpResult<T> = Result<T, CdpError>;

/// puppeteer's `ChromeLauncher.defaultArgs()` (headless, no extensions) plus
/// the user args, in the order puppeteer emits them. `--remote-debugging-port`
/// and `--user-data-dir` are appended by the launcher afterwards, as
/// puppeteer's `computeLaunchArguments` does.
pub fn default_chrome_args(user_args: &[String], dangerous_no_sandbox: bool) -> Vec<String> {
    let mut args: Vec<String> = [
        "--allow-pre-commit-input",
        "--disable-background-networking",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-breakpad",
        "--disable-client-side-phishing-detection",
        "--disable-component-extensions-with-background-pages",
        "--disable-component-update",
        "--disable-domain-reliability",
        "--no-pings",
        "--no-default-browser-check",
        "--gaia-url=http://127.0.0.1:9",
        "--google-apis-url=http://127.0.0.1:9",
        "--lso-url=http://127.0.0.1:9",
        "--disable-crash-reporter",
        "--disable-default-apps",
        "--disable-dev-shm-usage",
        "--disable-hang-monitor",
        "--disable-infobars",
        "--disable-ipc-flooding-protection",
        "--disable-popup-blocking",
        "--disable-prompt-on-repost",
        "--disable-renderer-backgrounding",
        "--disable-search-engine-choice-screen",
        "--disable-sync",
        "--enable-automation",
        "--export-tagged-pdf",
        "--force-color-profile=srgb",
        "--generate-pdf-document-outline",
        "--metrics-recording-only",
        "--no-first-run",
        "--password-store=basic",
        "--use-mock-keychain",
        "--disable-features=Translate,AcceptCHFrame,MediaRouter,OptimizationHints,WebUIReloadButton,ProcessPerSiteUpToMainFrameThreshold,IsolateSandboxedIframes,NetworkTimeServiceQuerying,PreconnectToSearch,AimEnabled,AiModeOmniboxEntryPoint,AimServerEligibilityEnabled,AimEntrypointEligibility,OptimizationGuideModelDownloading,OptimizationHintsFetching,OptimizationTargetPrediction,CertificateTransparencyComponentUpdater,AutofillServerCommunication,InterestFeedContentSuggestions,HashPrefixRealTimeLookups,SafeBrowsingHashPrefixRealTimeLookups,SafeBrowsingHashRealTimeOverOhttp",
        "--enable-features=PdfOopif",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if dangerous_no_sandbox && !user_args.iter().any(|a| a == "--no-sandbox") {
        args.push("--no-sandbox".to_string());
    }
    args.push("--headless=new".to_string());
    args.push("--hide-scrollbars".to_string());
    args.push("--mute-audio".to_string());
    args.push("--disable-extensions".to_string());
    if user_args.iter().all(|a| a.starts_with('-')) {
        args.push("about:blank".to_string());
    }
    args.extend(user_args.iter().cloned());
    args
}

/// A running headless browser: the process, its temp profile, and the
/// browser-level CDP connection.
pub struct Browser {
    child: Child,
    user_data_dir: PathBuf,
    conn: Connection,
    /// Sessions of `worker` targets whose exceptions count as page errors,
    /// keyed by session id → owning page session id.
    worker_owner: HashMap<String, String>,
}

impl Browser {
    /// Launch `executable` headless the way puppeteer does and connect to its
    /// DevTools websocket.
    pub fn launch(
        executable: &std::path::Path,
        user_args: &[String],
        dangerous_no_sandbox: bool,
    ) -> CdpResult<Browser> {
        let mut args = default_chrome_args(user_args, dangerous_no_sandbox);
        args.push("--remote-debugging-port=0".to_string());
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let user_data_dir = std::env::temp_dir().join(format!(
            "phoenix-ui-chrome-profile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&user_data_dir).map_err(|e| {
            CdpError::new(format!("Failed to create a temporary browser profile: {e}"))
        })?;
        args.push(format!("--user-data-dir={}", user_data_dir.display()));

        let mut child = match Command::new(executable)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&user_data_dir);
                return Err(CdpError::new(format!(
                    "Failed to launch the browser process! {}: {e}",
                    executable.display()
                )));
            }
        };
        let stderr = child.stderr.take();
        let (tx, rx) = mpsc::channel::<String>();
        if let Some(stderr) = stderr {
            // Drain stderr for the life of the process so a chatty browser
            // never blocks on a full pipe; forward lines for endpoint discovery.
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(l) => {
                            let _ = tx.send(l);
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        let started = Instant::now();
        let mut collected = String::new();
        let ws_url = loop {
            let remaining = LAUNCH_TIMEOUT.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_dir_all(&user_data_dir);
                return Err(CdpError::new(format!(
                    "Timed out after {} ms while waiting for the WS endpoint URL to appear in stdout!",
                    LAUNCH_TIMEOUT.as_millis()
                )));
            }
            match rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
                Ok(line) => {
                    if let Some(rest) = line.strip_prefix("DevTools listening on ") {
                        break rest.trim().to_string();
                    }
                    collected.push_str(&line);
                    collected.push('\n');
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = child.wait();
                    let _ = std::fs::remove_dir_all(&user_data_dir);
                    return Err(CdpError::new(format!(
                        "Failed to launch the browser process!{}",
                        if collected.trim().is_empty() {
                            String::new()
                        } else {
                            format!(" {}", collected.trim())
                        }
                    )));
                }
            }
            if let Ok(Some(status)) = child.try_wait() {
                // Give the reader a moment to flush the last lines.
                while let Ok(line) = rx.recv_timeout(Duration::from_millis(50)) {
                    collected.push_str(&line);
                    collected.push('\n');
                }
                let _ = std::fs::remove_dir_all(&user_data_dir);
                return Err(CdpError::new(format!(
                    "Failed to launch the browser process! (exit {status}){}",
                    if collected.trim().is_empty() {
                        String::new()
                    } else {
                        format!(" {}", collected.trim())
                    }
                )));
            }
        };
        let conn = match Connection::connect(&ws_url) {
            Ok(c) => c,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_dir_all(&user_data_dir);
                return Err(e);
            }
        };
        Ok(Browser {
            child,
            user_data_dir,
            conn,
            worker_owner: HashMap::new(),
        })
    }

    /// `browser.close()`: `Browser.close`, wait for exit; dropping removes
    /// the temp profile.
    pub fn close(mut self) {
        let _ = self
            .conn
            .send(None, "Browser.close", json!({}), Duration::from_secs(5));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                _ => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break;
                }
            }
        }
    }

    /// Browser-reported capture environment, independent of page script overrides.
    pub fn version(&mut self) -> CdpResult<Value> {
        self.conn
            .send(None, "Browser.getVersion", json!({}), PROTOCOL_TIMEOUT)
    }

    /// `browser.newPage()`: a fresh target in the default context, attached
    /// flat, with puppeteer's page setup applied.
    pub fn new_page(&mut self) -> CdpResult<Page<'_>> {
        self.open_page(None)
    }

    /// A page in its own browser context: no cookies, storage or cache are
    /// shared with earlier pages, so every capture state starts fresh. The
    /// context is disposed when the page closes.
    pub fn new_isolated_page(&mut self) -> CdpResult<Page<'_>> {
        let context = self.conn.send(
            None,
            "Target.createBrowserContext",
            json!({}),
            PROTOCOL_TIMEOUT,
        )?;
        let id = context
            .get("browserContextId")
            .and_then(Value::as_str)
            .ok_or_else(|| CdpError::new("Target.createBrowserContext returned no id"))?
            .to_string();
        self.open_page(Some(id))
    }

    fn open_page(&mut self, context: Option<String>) -> CdpResult<Page<'_>> {
        let mut params = json!({ "url": "about:blank" });
        if let Some(id) = &context {
            params["browserContextId"] = json!(id);
        }
        let created = self
            .conn
            .send(None, "Target.createTarget", params, PROTOCOL_TIMEOUT);
        let created = match created {
            Ok(c) => c,
            Err(e) => {
                if let Some(id) = &context {
                    let _ = self.conn.send(
                        None,
                        "Target.disposeBrowserContext",
                        json!({ "browserContextId": id }),
                        PROTOCOL_TIMEOUT,
                    );
                }
                return Err(e);
            }
        };
        let target_id = created
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or_else(|| CdpError::new("Target.createTarget returned no targetId"))?
            .to_string();
        let attached = self.conn.send(
            None,
            "Target.attachToTarget",
            json!({ "targetId": target_id, "flatten": true }),
            PROTOCOL_TIMEOUT,
        )?;
        let session_id = attached
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| CdpError::new("Target.attachToTarget returned no sessionId"))?
            .to_string();
        let mut page = Page {
            browser: self,
            session_id,
            target_id,
            frames: HashMap::new(),
            main_frame_id: String::new(),
            page_errors: Vec::new(),
            swapped: false,
            same_document_navigation: false,
            iframe_sessions: HashSet::new(),
            response_capture: None,
            execution_contexts: HashMap::new(),
            browser_context: context,
            console_errors: Vec::new(),
            network_log: None,
            screencast: None,
            nav_watch: None,
        };
        page.initialize()?;
        Ok(page)
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.user_data_dir);
    }
}

/// Origin-scoped basic-auth injection state (JS `applyOriginScopedAuth`,
/// issue #657): while armed, `Fetch.requestPaused` events on the session are
/// answered inline with `Fetch.continueRequest`, adding the Authorization
/// header only for same-origin requests.
struct OriginAuth {
    origin: String,
    header: String,
}

/// Non-GET requests are failed instead of sent, except allow-listed paths
/// (exact pathname, or a prefix when the entry ends in `*`).
struct WriteBlock {
    allow: Vec<String>,
    blocked: Vec<Value>,
    allowed: Vec<Value>,
}

impl WriteBlock {
    fn allows(&self, path: &str) -> bool {
        self.allow.iter().any(|a| match a.strip_suffix('*') {
            Some(prefix) => path.starts_with(prefix),
            None => path == a,
        })
    }
}

/// Request interception registered for one page session. Auto-attached
/// iframe/worker sessions point at their page through `intercept_owner`.
#[derive(Default)]
struct Interceptor {
    auth: Option<OriginAuth>,
    block: Option<WriteBlock>,
}

/// One websocket to the browser; JSON-RPC ids and an event queue.
struct Connection {
    ws: WebSocket<TcpStream>,
    next_id: u64,
    events: VecDeque<Value>,
    interceptors: HashMap<String, Interceptor>,
    intercept_owner: HashMap<String, String>,
}

impl Connection {
    fn connect(ws_url: &str) -> CdpResult<Connection> {
        let parsed = url_host_port(ws_url)
            .ok_or_else(|| CdpError::new(format!("Unexpected DevTools endpoint: {ws_url}")))?;
        let stream = TcpStream::connect(&parsed).map_err(|e| {
            CdpError::new(format!("Could not connect to the browser at {ws_url}: {e}"))
        })?;
        let _ = stream.set_nodelay(true);
        let request = ws_url
            .into_client_request()
            .map_err(|e| CdpError::new(format!("Bad DevTools endpoint {ws_url}: {e}")))?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(256 * 1024 * 1024))
            .max_frame_size(Some(256 * 1024 * 1024));
        let (ws, _response) =
            tungstenite::client::client_with_config(request, stream, Some(config)).map_err(
                |e| CdpError::new(format!("WebSocket handshake with the browser failed: {e}")),
            )?;
        Ok(Connection {
            ws,
            next_id: 0,
            events: VecDeque::new(),
            interceptors: HashMap::new(),
            intercept_owner: HashMap::new(),
        })
    }

    /// JS `page.on('request', ...)` handler body from `applyOriginScopedAuth`:
    /// answer a paused request, attaching Authorization only when the request
    /// URL's origin equals the armed origin. A write block fails non-GET
    /// requests first. Returns true when the message was a
    /// `Fetch.requestPaused` for an armed session (consumed).
    fn maybe_handle_request_paused(&mut self, msg: &Value) -> bool {
        if msg.get("method").and_then(Value::as_str) != Some("Fetch.requestPaused") {
            return false;
        }
        let Some(session_id) = msg.get("sessionId").and_then(Value::as_str) else {
            return false;
        };
        let session_id = session_id.to_string();
        let owner = self
            .intercept_owner
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| session_id.clone());
        let Some(interceptor) = self.interceptors.get_mut(&owner) else {
            return false;
        };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let Some(request_id) = params.get("requestId").and_then(Value::as_str) else {
            return true;
        };
        let req_url = params
            .pointer("/request/url")
            .and_then(Value::as_str)
            .unwrap_or("");
        let parsed = url::Url::parse(req_url).ok();
        if let Some(block) = &mut interceptor.block {
            let method = params
                .pointer("/request/method")
                .and_then(Value::as_str)
                .unwrap_or("GET");
            if method != "GET" {
                let path = parsed.as_ref().map(|u| u.path()).unwrap_or("");
                // Query strings can carry tokens; the record keeps origin + path.
                let shown = match &parsed {
                    Some(u) => format!("{}{}", u.origin().ascii_serialization(), u.path()),
                    None => req_url.split(['?', '#']).next().unwrap_or("").to_string(),
                };
                let record = json!({
                    "method": method,
                    "url": shown,
                    "resourceType": params.get("resourceType").cloned().unwrap_or(Value::Null),
                    "at": impeccable_common::iso_now(),
                });
                if !block.allows(path) {
                    block.blocked.push(record);
                    let _ = self.post(
                        Some(&session_id),
                        "Fetch.failRequest",
                        json!({ "requestId": request_id, "errorReason": "BlockedByClient" }),
                    );
                    return true;
                }
                block.allowed.push(record);
            }
        }
        let auth = interceptor
            .auth
            .as_ref()
            .map(|a| (a.origin.clone(), a.header.clone()));
        // JS: invalid request URL -> continue without auth.
        let header = auth.and_then(|(origin, header)| {
            parsed
                .as_ref()
                .is_some_and(|u| u.origin().ascii_serialization() == origin)
                .then_some(header)
        });
        let mut cont = Map::new();
        cont.insert("requestId".into(), json!(request_id));
        if let Some(header) = header {
            // JS `{ ...request.headers(), authorization: header }`: puppeteer's
            // request.headers() lowercases names, so authorization overrides.
            let mut headers: Vec<Value> = Vec::new();
            if let Some(obj) = params
                .pointer("/request/headers")
                .and_then(Value::as_object)
            {
                for (k, v) in obj {
                    let name = k.to_ascii_lowercase();
                    if name == "authorization" {
                        continue;
                    }
                    headers.push(json!({ "name": name, "value": v.as_str().unwrap_or("") }));
                }
            }
            headers.push(json!({ "name": "authorization", "value": header }));
            cont.insert("headers".into(), Value::Array(headers));
        }
        // JS `void request.continue(...).catch(() => {})`.
        let _ = self.post(
            Some(&session_id),
            "Fetch.continueRequest",
            Value::Object(cont),
        );
        true
    }

    /// Write a command and return its id without waiting for the reply.
    fn post(&mut self, session_id: Option<&str>, method: &str, params: Value) -> CdpResult<u64> {
        self.next_id += 1;
        let id = self.next_id;
        let mut msg = Map::new();
        msg.insert("id".into(), json!(id));
        msg.insert("method".into(), json!(method));
        msg.insert("params".into(), params);
        if let Some(sid) = session_id {
            msg.insert("sessionId".into(), json!(sid));
        }
        let text = Value::Object(msg).to_string();
        self.ws
            .send(Message::Text(text.into()))
            .map_err(|e| CdpError::new(format!("Protocol error ({method}): {e}")))?;
        Ok(id)
    }

    /// Send a command and wait for its result. Events that arrive meanwhile
    /// are queued. A `{ error }` reply becomes `Protocol error (method): message`
    /// (puppeteer's wording).
    fn send(
        &mut self,
        session_id: Option<&str>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> CdpResult<Value> {
        let id = self.post(session_id, method, params)?;
        let deadline = Instant::now() + timeout;
        loop {
            let msg = match self.read_one(deadline)? {
                Some(m) => m,
                None => {
                    return Err(CdpError::new(format!(
                        "{method} timed out. Increase the 'protocolTimeout' setting in launch/connect calls for a higher timeout if needed."
                    )));
                }
            };
            if msg.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(err) = msg.get("error") {
                    let text = err.get("message").and_then(Value::as_str).unwrap_or("");
                    let data = err.get("data").and_then(Value::as_str);
                    let full = match data {
                        Some(d) if !d.is_empty() => format!("{text} {d}"),
                        _ => text.to_string(),
                    };
                    return Err(CdpError::new(format!("Protocol error ({method}): {full}")));
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
            if msg.get("method").is_some() {
                self.events.push_back(msg);
            }
            // Replies to fire-and-forget posts are dropped.
        }
    }

    /// Read one message, or `None` when `deadline` passes first.
    fn read_one(&mut self, deadline: Instant) -> CdpResult<Option<Value>> {
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let _ = self
                .ws
                .get_ref()
                .set_read_timeout(Some(remaining.max(Duration::from_millis(1))));
            match self.ws.read() {
                Ok(Message::Text(t)) => match serde_json::from_str::<Value>(&t) {
                    Ok(mut v) => {
                        // Answer paused requests inline so a pending command
                        // (goto, evaluate) whose completion depends on them
                        // cannot deadlock the single-threaded client.
                        if self.maybe_handle_request_paused(&v) {
                            continue;
                        }
                        // Chrome sends the next screencast frame only after an
                        // ack; answer here so frames keep coming while a
                        // command is awaited. The frame itself is still queued,
                        // stamped with its arrival on this process's clock.
                        if v.get("method").and_then(Value::as_str) == Some("Page.screencastFrame") {
                            v["receivedAt"] = json!(impeccable_common::now_ms() / 1000.0);
                            if let (Some(sid), Some(frame)) = (
                                v.get("sessionId")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                                v.pointer("/params/sessionId").cloned(),
                            ) {
                                let _ = self.post(
                                    Some(&sid),
                                    "Page.screencastFrameAck",
                                    json!({ "sessionId": frame }),
                                );
                            }
                        }
                        return Ok(Some(v));
                    }
                    Err(_) => continue,
                },
                Ok(Message::Close(_)) => {
                    return Err(CdpError::new("Protocol error: Target closed"));
                }
                Ok(_) => continue,
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None);
                }
                Err(e) => {
                    return Err(CdpError::new(format!(
                        "Protocol error: connection closed ({e})"
                    )));
                }
            }
        }
    }

    /// Pull queued events, then any events already readable without waiting.
    fn take_events(&mut self) -> Vec<Value> {
        let mut out: Vec<Value> = self.events.drain(..).collect();
        // Non-blocking sweep of what's already on the wire.
        let deadline = Instant::now() + Duration::from_millis(1);
        while let Ok(Some(msg)) = self.read_one(deadline) {
            if msg.get("method").is_some() {
                out.push(msg);
            }
        }
        out
    }
}

fn url_host_port(ws_url: &str) -> Option<String> {
    let rest = ws_url.strip_prefix("ws://")?;
    let host_port = rest.split('/').next()?;
    if host_port.contains(':') {
        Some(host_port.to_string())
    } else {
        Some(format!("{host_port}:80"))
    }
}

#[derive(Default, Debug)]
struct FrameState {
    parent: Option<String>,
    loader_id: String,
    lifecycle: HashSet<String>,
    has_started_loading: bool,
}

/// puppeteer's `Viewport` subset we use.
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// A puppeteer `Page`: one attached target with lifecycle tracking, page
/// error capture, and the evaluate/screenshot helpers the URL engine uses.
pub struct Page<'a> {
    browser: &'a mut Browser,
    session_id: String,
    target_id: String,
    frames: HashMap<String, FrameState>,
    main_frame_id: String,
    page_errors: Vec<String>,
    swapped: bool,
    same_document_navigation: bool,
    /// Auto-attached OOPIF sessions whose Page events feed the frame map.
    iframe_sessions: HashSet<String>,
    response_capture: Option<crate::response_capture::ResponseCapture>,
    execution_contexts: HashMap<i64, String>,
    browser_context: Option<String>,
    console_errors: Vec<String>,
    network_log: Option<crate::network_log::NetworkLog>,
    /// Screencast frames `(arrival seconds since epoch, base64 JPEG)` while recording.
    screencast: Option<Vec<(f64, String)>>,
    /// Main-frame loading seen since `watch_navigation`: (loader before the
    /// action, started, stopped).
    nav_watch: Option<(String, bool, bool)>,
}

/// One key press for `Input.dispatchKeyEvent`, parsed from `Enter`,
/// `Control+A`, `Meta+K`, `a`, ...
#[derive(Debug, Clone, PartialEq)]
pub struct KeyStroke {
    key: String,
    code: String,
    key_code: u32,
    text: Option<String>,
    modifiers: u32,
}

impl KeyStroke {
    pub fn parse(spec: &str) -> Result<KeyStroke, String> {
        let parts: Vec<&str> = spec.split('+').collect();
        let (last, mods) = match parts.split_last() {
            Some((last, mods)) if !last.is_empty() => (*last, mods),
            // `+` itself, or `Shift++`.
            _ if spec.ends_with('+') => ("+", &parts[..parts.len().saturating_sub(2)]),
            _ => return Err(format!("无法识别的按键 {spec}")),
        };
        let mut modifiers = 0;
        for m in mods {
            modifiers |= match *m {
                "Alt" => 1,
                "Control" => 2,
                "Meta" => 4,
                "Shift" => 8,
                other => {
                    return Err(format!(
                        "无法识别的修饰键 {other}（可用 Alt、Control、Meta、Shift）"
                    ))
                }
            };
        }
        let named: &[(&str, &str, u32, Option<&str>)] = &[
            ("Enter", "Enter", 13, Some("\r")),
            ("Tab", "Tab", 9, None),
            ("Escape", "Escape", 27, None),
            ("Backspace", "Backspace", 8, None),
            ("Delete", "Delete", 46, None),
            ("Space", "Space", 32, Some(" ")),
            ("ArrowUp", "ArrowUp", 38, None),
            ("ArrowDown", "ArrowDown", 40, None),
            ("ArrowLeft", "ArrowLeft", 37, None),
            ("ArrowRight", "ArrowRight", 39, None),
            ("Home", "Home", 36, None),
            ("End", "End", 35, None),
            ("PageUp", "PageUp", 33, None),
            ("PageDown", "PageDown", 34, None),
        ];
        if let Some((name, code, key_code, text)) = named.iter().find(|n| n.0 == last) {
            let key = if *name == "Space" { " " } else { name };
            return Ok(KeyStroke {
                key: key.to_string(),
                code: code.to_string(),
                key_code: *key_code,
                text: text.filter(|_| modifiers & 7 == 0).map(str::to_string),
                modifiers,
            });
        }
        let mut chars = last.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err(format!(
                "无法识别的按键 {last}（可用单个字符或 Enter、Tab、Escape、Backspace、Delete、Space、ArrowUp/Down/Left/Right、Home、End、PageUp、PageDown）"
            ));
        };
        let upper = c.to_ascii_uppercase();
        let code = if c.is_ascii_alphabetic() {
            format!("Key{upper}")
        } else if c.is_ascii_digit() {
            format!("Digit{c}")
        } else {
            String::new()
        };
        let key_code = if c.is_ascii_alphanumeric() {
            upper as u32
        } else {
            0
        };
        Ok(KeyStroke {
            key: c.to_string(),
            code,
            key_code,
            // A chord with Control/Meta/Alt sends no character.
            text: (modifiers & 7 == 0).then(|| c.to_string()),
            modifiers,
        })
    }
}

/// An opaque, document-bound isolated execution context. Never falls back to
/// the page world or a newly navigated document.
pub struct IsolatedWorld {
    unique_id: String,
    frame_id: String,
}

/// A raw `Runtime.evaluate` outcome.
pub enum EvalOutcome {
    Value(Value),
    /// The page threw: message as puppeteer's `createEvaluationError` renders it.
    Exception(String),
}

impl<'a> Page<'a> {
    fn send(&mut self, method: &str, params: Value) -> CdpResult<Value> {
        let sid = self.session_id.clone();
        let out = self
            .browser
            .conn
            .send(Some(&sid), method, params, PROTOCOL_TIMEOUT);
        self.pump_events();
        out
    }

    /// Start a fresh diagnostic journal before navigation. Cache, authentication,
    /// and service-worker behavior remain unchanged. No response is refetched.
    pub fn begin_response_capture(&mut self) -> CdpResult<()> {
        // Blink counts decoded strings in its inspector buffer (UTF-16 may
        // need twice their UTF-8 bytes). Reserve representation headroom while
        // ResponseCapture still enforces the original decoded-byte limits.
        self.send(
            "Network.enable",
            json!({
                "maxTotalBufferSize": crate::response_capture::MAX_TOTAL * 2,
                "maxResourceBufferSize": crate::response_capture::MAX_BODY * 2
            }),
        )?;
        self.response_capture = Some(crate::response_capture::ResponseCapture::default());
        Ok(())
    }

    /// All requests observed for the current document, including failed/missing dependencies.
    pub fn observed_response_urls(&mut self) -> CdpResult<Vec<String>> {
        self.pump_events();
        let capture=self.response_capture.as_ref().ok_or_else(||CdpError::new("response capture is not enabled"))?;
        let loader=self.frames.get(&self.main_frame_id).map(|f|f.loader_id.as_str()).unwrap_or("");
        Ok(capture.urls(&self.main_frame_id,loader))
    }

    /// Retrieve observed main-document response payloads for exact URLs.
    /// This is transport evidence only: it does not prove rendered use or fidelity.
    pub fn response_evidence(
        &mut self,
        urls: &[String],
    ) -> CdpResult<crate::response_capture::ResponseEvidence> {
        self.pump_events();
        let capture = self
            .response_capture
            .as_ref()
            .ok_or_else(|| CdpError::new("response capture is not enabled"))?;
        let frame = self.main_frame_id.clone();
        let loader = self
            .frames
            .get(&frame)
            .map(|f| f.loader_id.clone())
            .unwrap_or_default();
        let pending = capture.pending(urls, &frame, &loader);
        for (index, request_id) in pending {
            let body = self
                .send("Network.getResponseBody", json!({"requestId": request_id}))
                .map_err(|e| e.message);
            self.response_capture
                .as_mut()
                .unwrap()
                .store_body(index, body);
        }
        Ok(self
            .response_capture
            .as_ref()
            .unwrap()
            .evidence(urls, &frame, &loader))
    }

    /// puppeteer's `CdpPage` + `FrameManager.initialize` for a new target.
    fn initialize(&mut self) -> CdpResult<()> {
        self.send("Page.enable", json!({}))?;
        let tree = self.send("Page.getFrameTree", json!({}))?;
        if let Some(ft) = tree.get("frameTree") {
            self.handle_frame_tree(ft, None);
        }
        self.send("Page.setLifecycleEventsEnabled", json!({ "enabled": true }))?;
        // URL mode injects only the snapshot producer (plain JS) and runs the
        // rules natively in this process over `SnapshotDom`: no WebAssembly is compiled next
        // to the page, so the page's Content-Security-Policy no longer gates
        // the scan and `Page.setBypassCSP` is not needed. Leaving CSP enforced keeps the
        // scan passive — a strict-CSP page's blocked inline scripts stay
        // blocked, matching the puppeteer engine (which never bypassed CSP).
        self.send("Runtime.enable", json!({}))?;
        self.send("Network.enable", json!({}))?;
        self.send("Performance.enable", json!({}))?;
        self.send("Log.enable", json!({}))?;
        self.send(
            "Target.setAutoAttach",
            json!({ "autoAttach": true, "waitForDebuggerOnStart": true, "flatten": true }),
        )?;
        Ok(())
    }

    fn handle_frame_tree(&mut self, tree: &Value, parent: Option<String>) {
        let Some(frame) = tree.get("frame") else {
            return;
        };
        let Some(id) = frame.get("id").and_then(Value::as_str) else {
            return;
        };
        let id = id.to_string();
        let parent_id = frame
            .get("parentId")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .or(parent);
        if parent_id.is_none() && self.main_frame_id.is_empty() {
            self.main_frame_id = id.clone();
        }
        let entry = self.frames.entry(id.clone()).or_default();
        entry.parent = parent_id;
        if let Some(children) = tree.get("childFrames").and_then(Value::as_array) {
            for child in children {
                self.handle_frame_tree(child, Some(id.clone()));
            }
        }
    }

    /// Drain queued CDP events into page state (frames, errors, auto-attach).
    fn pump_events(&mut self) {
        let events = self.browser.conn.take_events();
        for ev in events {
            self.on_event(&ev);
        }
    }

    fn on_event(&mut self, ev: &Value) {
        let method = ev.get("method").and_then(Value::as_str).unwrap_or("");
        let session = ev.get("sessionId").and_then(Value::as_str).unwrap_or("");
        let params = ev.get("params").cloned().unwrap_or(Value::Null);
        if session == self.session_id {
            match method {
                "Runtime.executionContextCreated" => {
                    if let (Some(id), Some(unique)) = (
                        params.pointer("/context/id").and_then(Value::as_i64),
                        params.pointer("/context/uniqueId").and_then(Value::as_str),
                    ) {
                        if self.execution_contexts.len() < 512 {
                            self.execution_contexts.insert(id, unique.to_owned());
                        }
                    }
                }
                "Runtime.executionContextDestroyed" => {
                    if let Some(id) = params.get("executionContextId").and_then(Value::as_i64) {
                        self.execution_contexts.remove(&id);
                    }
                }
                "Runtime.executionContextsCleared" => self.execution_contexts.clear(),
                _ => {}
            }
            if let Some(capture) = &mut self.response_capture {
                capture.event(method, &params);
            }
            match method {
                "Runtime.consoleAPICalled" => {
                    let kind = params.get("type").and_then(Value::as_str).unwrap_or("");
                    if kind == "error" || kind == "assert" {
                        let text: Vec<String> = params
                            .get("args")
                            .and_then(Value::as_array)
                            .map(|args| args.iter().map(console_arg_text).collect())
                            .unwrap_or_default();
                        let text = text.join(" ");
                        let text = if kind == "assert" {
                            format!("Assertion failed: {text}")
                        } else {
                            text
                        };
                        self.push_console_error(text);
                    }
                }
                "Log.entryAdded" => {
                    let entry = params.get("entry").cloned().unwrap_or(Value::Null);
                    if entry.get("level").and_then(Value::as_str) == Some("error")
                        && entry.get("source").and_then(Value::as_str) != Some("network")
                    {
                        let text = entry.get("text").and_then(Value::as_str).unwrap_or("");
                        self.push_console_error(text.to_string());
                    }
                }
                "Page.screencastFrame" => {
                    if let Some(frames) = &mut self.screencast {
                        // Arrival time, not Chrome's frame timestamp: that clock
                        // can run tens of ms ahead of ours, which would show a
                        // later frame as an earlier moment.
                        let ts = ev.get("receivedAt").and_then(Value::as_f64).unwrap_or(0.0);
                        if let Some(data) = params.get("data").and_then(Value::as_str) {
                            frames.push((ts, data.to_string()));
                        }
                    }
                }
                "Page.frameStartedLoading" | "Page.frameStoppedLoading" => {
                    let main = params.get("frameId").and_then(Value::as_str)
                        == Some(self.main_frame_id.as_str());
                    if let (true, Some(watch)) = (main, &mut self.nav_watch) {
                        if method == "Page.frameStartedLoading" {
                            watch.1 = true;
                            watch.2 = false;
                        } else if watch.1 {
                            watch.2 = true;
                        }
                    }
                }
                _ => {}
            }
        }
        let is_page_session = session == self.session_id || self.iframe_sessions.contains(session);
        if is_page_session {
            if let Some(log) = &mut self.network_log {
                log.event(method, &params);
            }
        }
        match method {
            "Target.attachedToTarget" => {
                let Some(new_session) = params.get("sessionId").and_then(Value::as_str) else {
                    return;
                };
                let new_session = new_session.to_string();
                let ttype = params
                    .pointer("/targetInfo/type")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let waiting = params
                    .get("waitingForDebugger")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                // Only targets hanging off this page (or its iframes) matter.
                if !is_page_session {
                    if waiting {
                        let _ = self.browser.conn.post(
                            Some(&new_session),
                            "Runtime.runIfWaitingForDebugger",
                            json!({}),
                        );
                    }
                    return;
                }
                // Fire-and-forget setup, in order, so the target is configured
                // before it resumes (puppeteer's FrameManager / WebWorker init).
                let conn = &mut self.browser.conn;
                match ttype {
                    "iframe" => {
                        self.iframe_sessions.insert(new_session.clone());
                        let _ = conn.post(Some(&new_session), "Page.enable", json!({}));
                        let _ = conn.post(
                            Some(&new_session),
                            "Page.setLifecycleEventsEnabled",
                            json!({ "enabled": true }),
                        );
                        let _ = conn.post(Some(&new_session), "Runtime.enable", json!({}));
                        let _ = conn.post(Some(&new_session), "Network.enable", json!({}));
                    }
                    "worker" => {
                        self.browser
                            .worker_owner
                            .insert(new_session.clone(), self.session_id.clone());
                        let _ = conn.post(Some(&new_session), "Runtime.enable", json!({}));
                    }
                    _ => {}
                }
                // A write block must cover requests from iframes, workers and
                // service workers too; arm it while the target is still paused.
                // `setBypassServiceWorker` on the page only stops it from
                // answering page requests — the worker itself can still fetch,
                // so its own session needs interception.
                let owner = conn
                    .intercept_owner
                    .get(session)
                    .cloned()
                    .unwrap_or_else(|| session.to_string());
                let blocking = conn
                    .interceptors
                    .get(&owner)
                    .is_some_and(|i| i.block.is_some());
                if blocking
                    && matches!(
                        ttype,
                        "iframe" | "worker" | "shared_worker" | "service_worker"
                    )
                {
                    conn.intercept_owner.insert(new_session.clone(), owner);
                    let _ = conn.post(
                        Some(&new_session),
                        "Fetch.enable",
                        json!({ "patterns": [{ "urlPattern": "*", "requestStage": "Request" }] }),
                    );
                    if ttype == "iframe" {
                        let _ = conn.post(
                            Some(&new_session),
                            "Network.setBypassServiceWorker",
                            json!({ "bypass": true }),
                        );
                    }
                }
                let _ = conn.post(
                    Some(&new_session),
                    "Target.setAutoAttach",
                    json!({ "autoAttach": true, "waitForDebuggerOnStart": true, "flatten": true }),
                );
                if waiting {
                    let _ = conn.post(
                        Some(&new_session),
                        "Runtime.runIfWaitingForDebugger",
                        json!({}),
                    );
                }
            }
            "Target.detachedFromTarget" => {
                if let Some(sid) = params.get("sessionId").and_then(Value::as_str) {
                    self.iframe_sessions.remove(sid);
                    self.browser.worker_owner.remove(sid);
                }
            }
            "Runtime.exceptionThrown" => {
                let counts = session == self.session_id
                    || self
                        .browser
                        .worker_owner
                        .get(session)
                        .map(|owner| *owner == self.session_id)
                        .unwrap_or(false);
                if counts {
                    if let Some(details) = params.get("exceptionDetails") {
                        let message = client_error_message(details);
                        self.push_console_error(message.clone());
                        // detect-url.mjs: first line, trimmed, 160 chars, deduped.
                        let first = message.split('\n').next().unwrap_or("");
                        let trimmed = impeccable_core::js::trim(first);
                        let sliced: String = trimmed.chars().take(160).collect();
                        if !sliced.is_empty() && !self.page_errors.contains(&sliced) {
                            self.page_errors.push(sliced);
                        }
                    }
                }
            }
            _ if !is_page_session => {}
            "Page.frameAttached" => {
                let (Some(id), Some(parent)) = (
                    params.get("frameId").and_then(Value::as_str),
                    params.get("parentFrameId").and_then(Value::as_str),
                ) else {
                    return;
                };
                if !self.frames.contains_key(id) {
                    self.frames.insert(
                        id.to_string(),
                        FrameState {
                            parent: Some(parent.to_string()),
                            ..Default::default()
                        },
                    );
                }
            }
            "Page.frameDetached" => {
                let Some(id) = params.get("frameId").and_then(Value::as_str) else {
                    return;
                };
                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("remove");
                if reason == "swap" {
                    if id == self.main_frame_id {
                        self.swapped = true;
                    }
                } else {
                    self.remove_frame_recursively(id);
                }
            }
            "Page.frameNavigated" => {
                let Some(frame) = params.get("frame") else {
                    return;
                };
                let Some(id) = frame.get("id").and_then(Value::as_str) else {
                    return;
                };
                let is_main = frame.get("parentId").and_then(Value::as_str).is_none();
                if is_main && id != self.main_frame_id {
                    // Main frame id changed (cross-process swap): carry state over.
                    let old = self.frames.remove(&self.main_frame_id).unwrap_or_default();
                    self.frames.insert(id.to_string(), old);
                    self.main_frame_id = id.to_string();
                } else if !self.frames.contains_key(id) {
                    self.frames.insert(
                        id.to_string(),
                        FrameState {
                            parent: frame
                                .get("parentId")
                                .and_then(Value::as_str)
                                .map(String::from),
                            ..Default::default()
                        },
                    );
                }
                let ntype = params
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("Navigation");
                if ntype == "BackForwardCacheRestore" {
                    self.swapped = true;
                }
            }
            "Page.navigatedWithinDocument" => {
                if params.get("frameId").and_then(Value::as_str)
                    == Some(self.main_frame_id.as_str())
                {
                    self.same_document_navigation = true;
                }
            }
            "Page.frameStartedLoading" => {
                if let Some(id) = params.get("frameId").and_then(Value::as_str) {
                    if let Some(f) = self.frames.get_mut(id) {
                        f.has_started_loading = true;
                    }
                }
            }
            "Page.frameStoppedLoading" => {
                if let Some(id) = params.get("frameId").and_then(Value::as_str) {
                    if let Some(f) = self.frames.get_mut(id) {
                        f.lifecycle.insert("DOMContentLoaded".into());
                        f.lifecycle.insert("load".into());
                    }
                }
            }
            "Page.lifecycleEvent" => {
                let (Some(id), Some(name)) = (
                    params.get("frameId").and_then(Value::as_str),
                    params.get("name").and_then(Value::as_str),
                ) else {
                    return;
                };
                let loader = params.get("loaderId").and_then(Value::as_str).unwrap_or("");
                if let Some(f) = self.frames.get_mut(id) {
                    if name == "init" {
                        f.loader_id = loader.to_string();
                        f.lifecycle.clear();
                    }
                    f.lifecycle.insert(name.to_string());
                }
            }
            _ => {}
        }
    }

    fn remove_frame_recursively(&mut self, id: &str) {
        let children: Vec<String> = self
            .frames
            .iter()
            .filter(|(_, f)| f.parent.as_deref() == Some(id))
            .map(|(k, _)| k.clone())
            .collect();
        for c in children {
            self.remove_frame_recursively(&c);
        }
        self.frames.remove(id);
    }

    /// LifecycleWatcher#checkLifecycle over the frame tree.
    fn lifecycle_complete(&self, frame_id: &str, expected: &str) -> bool {
        let Some(frame) = self.frames.get(frame_id) else {
            return false;
        };
        if !frame.lifecycle.contains(expected) {
            return false;
        }
        for (id, child) in &self.frames {
            if child.parent.as_deref() == Some(frame_id)
                && child.has_started_loading
                && !self.lifecycle_complete(id, expected)
            {
                return false;
            }
        }
        true
    }

    /// Explicit screenshot-environment preference; never injected as page CSS.
    pub fn set_reduced_motion(&mut self, reduce: bool) -> CdpResult<()> {
        self.send("Emulation.setEmulatedMedia", json!({"features": [{
            "name": "prefers-reduced-motion", "value": if reduce { "reduce" } else { "no-preference" }
        }]}))?;
        Ok(())
    }

    pub fn set_viewport(&mut self, viewport: Viewport) -> CdpResult<()> {
        self.send(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "mobile": false,
                "width": viewport.width,
                "height": viewport.height,
                "deviceScaleFactor": 1,
                "screenOrientation": { "angle": 0, "type": "portraitPrimary" },
            }),
        )?;
        self.send(
            "Emulation.setTouchEmulationEnabled",
            json!({ "enabled": false }),
        )?;
        Ok(())
    }

    /// JS detect-url.mjs#applyOriginScopedAuth(page, href, credentials):
    /// page.authenticate is page-wide (a cross-origin redirect that then 401s
    /// would receive the credentials), so Authorization is attached only to
    /// requests on the scan origin, via Fetch interception.
    pub fn apply_origin_scoped_auth(
        &mut self,
        href: &str,
        username: &str,
        password: &str,
    ) -> CdpResult<()> {
        let Ok(parsed) = url::Url::parse(href) else {
            return Ok(());
        };
        let origin = parsed.origin().ascii_serialization();
        if origin.is_empty() {
            return Ok(());
        }
        // JS `basicAuthHeader(credentials)`.
        use base64::Engine as _;
        let header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
        );
        // JS `page.setRequestInterception(true)`.
        self.send("Fetch.enable", json!({}))?;
        self.browser
            .conn
            .interceptors
            .entry(self.session_id.clone())
            .or_default()
            .auth = Some(OriginAuth { origin, header });
        Ok(())
    }

    /// Fail every non-GET request of this page, its iframes, workers and
    /// service workers, except `allow` paths; service worker sessions get
    /// their own interception (arming happens on auto-attach) because
    /// `Network.setBypassServiceWorker` only covers the page's own loads.
    pub fn enable_write_block(&mut self, allow: Vec<String>) -> CdpResult<()> {
        self.send("Network.setBypassServiceWorker", json!({ "bypass": true }))?;
        self.send(
            "Fetch.enable",
            json!({ "patterns": [{ "urlPattern": "*", "requestStage": "Request" }] }),
        )?;
        self.browser
            .conn
            .interceptors
            .entry(self.session_id.clone())
            .or_default()
            .block = Some(WriteBlock {
            allow,
            blocked: Vec::new(),
            allowed: Vec::new(),
        });
        Ok(())
    }

    /// `(blocked, allowed)` non-GET requests seen by the write block.
    pub fn write_log(&mut self) -> (Vec<Value>, Vec<Value>) {
        self.pump_events();
        match self
            .browser
            .conn
            .interceptors
            .get(&self.session_id)
            .and_then(|i| i.block.as_ref())
        {
            Some(b) => (b.blocked.clone(), b.allowed.clone()),
            None => (Vec::new(), Vec::new()),
        }
    }

    /// Device metrics as `set_viewport`, with a device pixel ratio.
    pub fn set_device_metrics(&mut self, width: u32, height: u32, dpr: f64) -> CdpResult<()> {
        self.send(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "mobile": false,
                "width": width,
                "height": height,
                "deviceScaleFactor": dpr,
                "screenOrientation": { "angle": 0, "type": "portraitPrimary" },
            }),
        )?;
        self.send(
            "Emulation.setTouchEmulationEnabled",
            json!({ "enabled": false }),
        )?;
        Ok(())
    }

    /// Emulate `prefers-color-scheme` and `prefers-reduced-motion` together.
    pub fn set_media(&mut self, color_scheme: &str, reduced_motion: bool) -> CdpResult<()> {
        self.send(
            "Emulation.setEmulatedMedia",
            json!({ "features": [
                { "name": "prefers-color-scheme", "value": color_scheme },
                { "name": "prefers-reduced-motion", "value": if reduced_motion { "reduce" } else { "no-preference" } },
            ]}),
        )?;
        Ok(())
    }

    fn push_console_error(&mut self, text: String) {
        let text: String = impeccable_core::js::trim(&text).chars().take(500).collect();
        if !text.is_empty()
            && self.console_errors.len() < 50
            && !self.console_errors.contains(&text)
        {
            self.console_errors.push(text);
        }
    }

    /// Console errors, assertion failures, uncaught exceptions and non-network
    /// error log entries of the page so far (deduped, ≤50, each ≤500 chars).
    pub fn console_errors(&mut self) -> Vec<String> {
        self.pump_events();
        self.console_errors.clone()
    }

    /// Start recording request methods, statuses and failures.
    pub fn begin_network_log(&mut self) {
        self.network_log = Some(crate::network_log::NetworkLog::default());
    }

    pub fn network_log(&mut self) -> Vec<crate::network_log::NetworkEntry> {
        self.pump_events();
        self.network_log
            .as_ref()
            .map(|l| l.entries())
            .unwrap_or_default()
    }

    pub fn layout_metrics(&mut self) -> CdpResult<Value> {
        self.send("Page.getLayoutMetrics", json!({}))
    }

    /// Base64 PNG. `clip` is `[x, y, width, height]` in CSS pixels at scale 1,
    /// so the output is in device pixels.
    pub fn screenshot(
        &mut self,
        clip: Option<[f64; 4]>,
        beyond_viewport: bool,
    ) -> CdpResult<String> {
        let mut params = json!({
            "format": "png", "optimizeForSpeed": false,
            "fromSurface": true, "captureBeyondViewport": beyond_viewport,
        });
        if let Some([x, y, width, height]) = clip {
            params["clip"] =
                json!({ "x": x, "y": y, "width": width, "height": height, "scale": 1 });
        }
        let res = self.send("Page.captureScreenshot", params)?;
        res.get("data")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| CdpError::new("screenshot returned no PNG"))
    }

    /// `Input.dispatchMouseEvent` with the left button.
    pub fn mouse(
        &mut self,
        kind: &str,
        x: f64,
        y: f64,
        click_count: u32,
        buttons: u32,
    ) -> CdpResult<()> {
        let button = if kind == "mouseMoved" && buttons == 0 {
            "none"
        } else {
            "left"
        };
        self.send(
            "Input.dispatchMouseEvent",
            json!({ "type": kind, "x": x, "y": y, "button": button, "buttons": buttons,
                    "clickCount": click_count, "pointerType": "mouse" }),
        )?;
        Ok(())
    }

    /// Press and release one key (puppeteer: `keyDown` when it types a
    /// character, `rawKeyDown` otherwise).
    pub fn key(&mut self, stroke: &KeyStroke) -> CdpResult<()> {
        let mut down = json!({
            "type": if stroke.text.is_some() { "keyDown" } else { "rawKeyDown" },
            "key": stroke.key, "code": stroke.code, "modifiers": stroke.modifiers,
            "windowsVirtualKeyCode": stroke.key_code,
        });
        if let Some(text) = &stroke.text {
            down["text"] = json!(text);
            down["unmodifiedText"] = json!(text);
        }
        // Headless Chrome has no platform shortcut for select-all.
        if stroke.modifiers & 6 != 0 && stroke.key.eq_ignore_ascii_case("a") {
            down["commands"] = json!(["selectAll"]);
        }
        self.send("Input.dispatchKeyEvent", down)?;
        self.send(
            "Input.dispatchKeyEvent",
            json!({ "type": "keyUp", "key": stroke.key, "code": stroke.code,
                    "modifiers": stroke.modifiers, "windowsVirtualKeyCode": stroke.key_code }),
        )?;
        Ok(())
    }

    pub fn insert_text(&mut self, text: &str) -> CdpResult<()> {
        self.send("Input.insertText", json!({ "text": text }))?;
        Ok(())
    }

    /// Backend node ids of non-ignored accessibility nodes with `role` (and
    /// the exact accessible `name` when given), in document order.
    pub fn query_ax(&mut self, role: &str, name: Option<&str>) -> CdpResult<Vec<(i64, String)>> {
        let doc = self.send("DOM.getDocument", json!({ "depth": 0 }))?;
        let root = doc
            .pointer("/root/backendNodeId")
            .and_then(Value::as_i64)
            .ok_or_else(|| CdpError::new("DOM.getDocument returned no backendNodeId"))?;
        let mut params = json!({ "backendNodeId": root, "role": role });
        if let Some(name) = name {
            params["accessibleName"] = json!(name);
        }
        let res = self.send("Accessibility.queryAXTree", params)?;
        let mut out = Vec::new();
        for node in res
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if node.get("ignored").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            if let Some(id) = node.get("backendDOMNodeId").and_then(Value::as_i64) {
                let name = node
                    .pointer("/name/value")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                out.push((id, name));
            }
        }
        Ok(out)
    }

    /// Call `function` in the main world as `function(args, ...nodes)` with
    /// the given backend nodes resolved to elements.
    pub fn call_with_nodes(
        &mut self,
        function: &str,
        args: &Value,
        nodes: &[i64],
    ) -> CdpResult<Value> {
        // Unresolvable nodes become `null` so arguments stay aligned with `nodes`.
        let mut arguments = vec![json!({ "value": args })];
        let mut this = None;
        for id in nodes {
            let res = self.send(
                "DOM.resolveNode",
                json!({ "backendNodeId": id, "objectGroup": "phoenix-capture" }),
            );
            match res
                .ok()
                .and_then(|r| r.pointer("/object/objectId").cloned())
            {
                Some(object) => {
                    this.get_or_insert_with(|| object.clone());
                    arguments.push(json!({ "objectId": object }));
                }
                None => arguments.push(json!({ "value": null })),
            }
        }
        let outcome = match this {
            None => self.evaluate(&format!("({function})({args})")),
            Some(this) => {
                let res = self.send(
                    "Runtime.callFunctionOn",
                    json!({ "objectId": this, "functionDeclaration": function, "arguments": arguments,
                            "returnByValue": true, "awaitPromise": true, "userGesture": true }),
                );
                let _ = self.send(
                    "Runtime.releaseObjectGroup",
                    json!({ "objectGroup": "phoenix-capture" }),
                );
                let res = res?;
                Ok(match res.get("exceptionDetails") {
                    Some(details) => EvalOutcome::Exception(client_error_message(details)),
                    None => EvalOutcome::Value(value_from_remote_object(
                        res.get("result").unwrap_or(&Value::Null),
                    )),
                })
            }
        }?;
        match outcome {
            EvalOutcome::Value(v) => Ok(v),
            EvalOutcome::Exception(message) => Err(CdpError::new(message)),
        }
    }

    /// Run `source` in every new document of this page before its own scripts.
    pub fn add_init_script(&mut self, source: &str) -> CdpResult<()> {
        self.send(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({ "source": source }),
        )?;
        Ok(())
    }

    /// Start a JPEG screencast at `max_width` × `max_height` device pixels and
    /// wait up to `first_frame` for the first frame. Returns whether it came.
    pub fn start_screencast(
        &mut self,
        max_width: u32,
        max_height: u32,
        first_frame: Duration,
    ) -> CdpResult<bool> {
        self.screencast = Some(Vec::new());
        self.send(
            "Page.startScreencast",
            json!({ "format": "jpeg", "quality": 90, "maxWidth": max_width,
                    "maxHeight": max_height, "everyNthFrame": 1 }),
        )?;
        let deadline = Instant::now() + first_frame;
        while self.screencast.as_ref().is_some_and(Vec::is_empty) && Instant::now() < deadline {
            self.idle(Duration::from_millis(10))?;
        }
        Ok(self.screencast.as_ref().is_some_and(|f| !f.is_empty()))
    }

    /// Stop the screencast and return its frames in arrival order.
    pub fn stop_screencast(&mut self) -> Vec<(f64, String)> {
        let _ = self.send("Page.stopScreencast", json!({}));
        self.screencast.take().unwrap_or_default()
    }

    /// Wait while still answering events (screencast acks, interception).
    pub fn idle(&mut self, duration: Duration) -> CdpResult<()> {
        let deadline = Instant::now() + duration;
        while let Some(msg) = self.browser.conn.read_one(deadline)? {
            if msg.get("method").is_some() {
                self.on_event(&msg);
            }
        }
        self.pump_events();
        Ok(())
    }

    /// Start watching main-frame loads before dispatching an action.
    pub fn watch_navigation(&mut self) {
        self.pump_events();
        let loader = self
            .frames
            .get(&self.main_frame_id)
            .map(|f| f.loader_id.clone())
            .unwrap_or_default();
        self.nav_watch = Some((loader, false, false));
    }

    /// After an action: if the main frame started loading within `grace`,
    /// wait until loading stops (up to `timeout`). Returns whether a new
    /// document committed.
    pub fn settle_navigation(&mut self, grace: Duration, timeout: Duration) -> CdpResult<bool> {
        self.idle(grace)?;
        let Some((loader, started, _)) = self.nav_watch.clone() else {
            return Ok(false);
        };
        if !started {
            self.nav_watch = None;
            return Ok(false);
        }
        let deadline = Instant::now() + timeout;
        loop {
            if self.nav_watch.as_ref().is_some_and(|w| w.2) {
                self.nav_watch = None;
                let current = self
                    .frames
                    .get(&self.main_frame_id)
                    .map(|f| f.loader_id.as_str());
                return Ok(current != Some(loader.as_str()));
            }
            match self.browser.conn.read_one(deadline)? {
                Some(msg) => {
                    if msg.get("method").is_some() {
                        self.on_event(&msg);
                    }
                }
                None => {
                    return Err(CdpError::new(format!(
                        "导航在 {} ms 内未完成加载",
                        timeout.as_millis()
                    )))
                }
            }
        }
    }

    /// `page.goto(url, { waitUntil, timeout })`. `wait_until` is puppeteer's
    /// name (`load`, `domcontentloaded`, `networkidle0`, `networkidle2`).
    pub fn goto(&mut self, url: &str, wait_until: &str, timeout: Duration) -> CdpResult<()> {
        let expected = match wait_until {
            "load" => "load",
            "domcontentloaded" => "DOMContentLoaded",
            "networkidle0" => "networkIdle",
            "networkidle2" => "networkAlmostIdle",
            other => {
                return Err(CdpError::new(format!(
                    "Unknown value for options.waitUntil: {other}"
                )));
            }
        };
        self.pump_events();
        let initial_loader = self
            .frames
            .get(&self.main_frame_id)
            .map(|f| f.loader_id.clone())
            .unwrap_or_default();
        self.swapped = false;
        self.same_document_navigation = false;
        let deadline = Instant::now() + timeout;
        let timeout_msg = format!("Navigation timeout of {} ms exceeded", timeout.as_millis());

        let sid = self.session_id.clone();
        let frame_id = self.main_frame_id.clone();
        let nav_id = self.browser.conn.post(
            Some(&sid),
            "Page.navigate",
            json!({ "url": url, "frameId": frame_id }),
        )?;
        // Race the navigate reply against the lifecycle watcher's timeout.
        let ensure_new_document;
        loop {
            let msg = match self.browser.conn.read_one(deadline)? {
                Some(m) => m,
                None => return Err(CdpError::new(timeout_msg)),
            };
            if msg.get("id").and_then(Value::as_u64) == Some(nav_id) {
                if let Some(err) = msg.get("error") {
                    let text = err.get("message").and_then(Value::as_str).unwrap_or("");
                    return Err(CdpError::new(format!(
                        "Protocol error (Page.navigate): {text}"
                    )));
                }
                let result = msg.get("result").cloned().unwrap_or(Value::Null);
                ensure_new_document = result
                    .get("loaderId")
                    .and_then(Value::as_str)
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);
                if let Some(text) = result.get("errorText").and_then(Value::as_str) {
                    if !text.is_empty() && text != "net::ERR_HTTP_RESPONSE_CODE_FAILURE" {
                        return Err(CdpError::new(format!("{text} at {url}")));
                    }
                }
                break;
            }
            if msg.get("method").is_some() {
                self.on_event(&msg);
                if !self.frames.contains_key(&self.main_frame_id) {
                    return Err(CdpError::new("Navigating frame was detached"));
                }
            }
        }
        // Wait for the lifecycle + (new-document | same-document) condition.
        loop {
            self.pump_events();
            if !self.frames.contains_key(&self.main_frame_id) {
                return Err(CdpError::new("Navigating frame was detached"));
            }
            let lifecycle_ok = self.lifecycle_complete(&self.main_frame_id.clone(), expected);
            if lifecycle_ok {
                let loader_changed = self
                    .frames
                    .get(&self.main_frame_id)
                    .map(|f| f.loader_id != initial_loader)
                    .unwrap_or(false);
                let new_doc = self.swapped || loader_changed;
                if ensure_new_document {
                    if new_doc {
                        return Ok(());
                    }
                } else if self.same_document_navigation {
                    return Ok(());
                }
            }
            match self.browser.conn.read_one(deadline)? {
                Some(msg) => {
                    if msg.get("method").is_some() {
                        self.on_event(&msg);
                    }
                }
                None => return Err(CdpError::new(timeout_msg)),
            }
        }
    }

    /// Create an opt-in world with separate JS globals/prototypes, on the current
    /// main document. No universal cross-origin access is granted.
    pub fn create_isolated_world(&mut self) -> CdpResult<IsolatedWorld> {
        self.pump_events();
        let frame_id = self.main_frame_id.clone();
        let loader_id = self
            .frames
            .get(&frame_id)
            .map(|f| f.loader_id.clone())
            .unwrap_or_default();
        if frame_id.is_empty() || loader_id.is_empty() {
            return Err(CdpError::new("no current document for isolated capture"));
        }
        let created=self.send("Page.createIsolatedWorld",json!({"frameId":frame_id,"worldName":"impeccable-diagnostic-capture","grantUniveralAccess":false}))?;
        let id = created["executionContextId"]
            .as_i64()
            .ok_or_else(|| CdpError::new("isolated world returned no context id"))?;
        let unique_id = self
            .execution_contexts
            .get(&id)
            .cloned()
            .ok_or_else(|| CdpError::new("isolated context identity unavailable"))?;
        let world = IsolatedWorld {
            unique_id,
            frame_id,
        };
        Ok(world)
    }

    /// Inspect native DOM coverage, including authored closed shadow roots that
    /// page-world selectors cannot see. This never treats hidden trees as absent.
    pub fn capture_dom_coverage(&mut self) -> CdpResult<Value> {
        let tree = self.send("DOM.getDocument", json!({"depth":-1,"pierce":true}))?;
        let root = tree
            .get("root")
            .ok_or_else(|| CdpError::new("native DOM coverage unavailable"))?;
        let mut stack = vec![root];
        let mut nodes = 0;
        let mut closed = 0;
        while let Some(node) = stack.pop() {
            nodes += 1;
            if nodes > 5000 {
                return Err(CdpError::new("native capture DOM exceeds 5000 nodes"));
            }
            if node.get("shadowRootType").and_then(Value::as_str) == Some("closed") {
                closed += 1;
            }
            for key in ["children", "shadowRoots", "pseudoElements"] {
                if let Some(children) = node.get(key).and_then(Value::as_array) {
                    stack.extend(children);
                }
            }
            for key in ["contentDocument", "templateContent"] {
                if let Some(child) = node.get(key).filter(|v| v.is_object()) {
                    stack.push(child);
                }
            }
        }
        Ok(json!({"inspectedNodes":nodes,"closedShadowRoots":closed}))
    }

    /// Inspector-owned stylesheet; never a page-authored style element.
    pub fn create_capture_stylesheet(&mut self, world: &IsolatedWorld) -> CdpResult<String> {
        self.send("DOM.enable", json!({}))?;
        self.send("CSS.enable", json!({}))?;
        let value = self.send("CSS.createStyleSheet", json!({"frameId":world.frame_id}))?;
        value["styleSheetId"].as_str().map(str::to_owned)
            .ok_or_else(|| CdpError::new("capture stylesheet unavailable"))
    }

    pub fn set_capture_stylesheet(&mut self, sheet: &str, text: &str) -> CdpResult<()> {
        if text.len() > 65536 {
            return Err(CdpError::new("capture stylesheet exceeds bound"));
        }
        self.send(
            "CSS.setStyleSheetText",
            json!({"styleSheetId":sheet,"text":text}),
        )?;
        Ok(())
    }

    /// The fonts that rendered each node matching `selector`, in document
    /// order. Reads layout results, so a node must generate boxes
    /// (`visibility: hidden` does, `display: none` does not).
    pub fn platform_fonts(&mut self, selector: &str) -> CdpResult<Vec<Vec<RenderedFont>>> {
        self.send("DOM.enable", json!({}))?;
        self.send("CSS.enable", json!({}))?;
        let document = self.send("DOM.getDocument", json!({"depth":0}))?;
        let root = document["root"]["nodeId"].clone();
        let found = self.send(
            "DOM.querySelectorAll",
            json!({"nodeId":root,"selector":selector}),
        )?;
        let mut out = Vec::new();
        for node in found["nodeIds"].as_array().into_iter().flatten() {
            let value = self.send("CSS.getPlatformFontsForNode", json!({"nodeId":node}))?;
            out.push(
                value["fonts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|font| RenderedFont {
                        family: font["familyName"].as_str().unwrap_or("").to_string(),
                        custom: font["isCustomFont"].as_bool().unwrap_or(false),
                        glyphs: font["glyphCount"].as_f64().unwrap_or(0.0) as u32,
                    })
                    .collect(),
            );
        }
        Ok(out)
    }

    /// Main-document generated boxes from native layout, indexed in querySelectorAll order.
    /// Do not infer pseudo geometry from its owner's rectangle.
    pub fn capture_pseudo_geometry(&mut self, world: &IsolatedWorld, retained: &[Value]) -> CdpResult<Value> {
        // Text-only generated boxes need no image geometry. Query candidates in
        // the isolated world before the bounded native layout lookup.
        let candidates = self.evaluate_value_in_world(world,
            "[...document.querySelectorAll('*')].flatMap((el,index)=>['before','after'].filter(p=>{const s=getComputedStyle(el,'::'+p);return s.backgroundImage!=='none'||s.content.includes('url(')}).map(pseudo=>({index,pseudo})))")?;
        let mut candidates = candidates.as_array().ok_or_else(|| CdpError::new("pseudo candidates unavailable"))?.clone();
        // Suppression can remove a pseudo's only URL. Keep its identity eligible
        // for a fresh native geometry lookup; never reuse its previous box.
        for candidate in retained {
            if !candidates.contains(candidate) { candidates.push(candidate.clone()); }
        }
        if candidates.len()>256 {return Err(CdpError::new("capture pseudo image candidates exceed bound"));}
        if candidates.is_empty() {return Ok(json!([]));}
        let tree = self.send("DOM.getDocument", json!({"depth":-1}))?;
        let mut stack = vec![(tree["root"].clone(), String::new())];
        let mut index = 0usize;
        let mut pseudos = Vec::new();
        while let Some((node, selector)) = stack.pop() {
            let owner = index;
            if node["nodeType"] == 1 { index += 1; }
            if index > 5000 { return Err(CdpError::new("capture DOM exceeds bound")); }
            if let Some(items) = node["pseudoElements"].as_array() {
                for pseudo in items {
                    if !matches!(pseudo["pseudoType"].as_str(), Some("before" | "after" | "marker")) { continue; }
                    if !candidates.iter().any(|c|c["index"].as_u64()==Some(owner as u64)&&c["pseudo"]==pseudo["pseudoType"]) {continue;}
                    if pseudos.len() >= 256 { return Err(CdpError::new("capture pseudo elements exceed bound")); }
                    let layout = self.send("DOM.getBoxModel", json!({"nodeId":pseudo["nodeId"]}));
                    let bounds = layout.ok().and_then(|v| {
                        let q = v["model"]["border"].as_array()?;
                        if q.len()!=8 {return None;}
                        let values: Option<Vec<f64>>=q.iter().map(Value::as_f64).collect();
                        let values=values?;
                        let xs=[values[0],values[2],values[4],values[6]];
                        let ys=[values[1],values[3],values[5],values[7]];
                        let x=xs.into_iter().fold(f64::INFINITY,f64::min);
                        let y=ys.into_iter().fold(f64::INFINITY,f64::min);
                        Some(json!({"x":x,"y":y,"w":xs.into_iter().fold(f64::NEG_INFINITY,f64::max)-x,"h":ys.into_iter().fold(f64::NEG_INFINITY,f64::max)-y}))
                    });
                    pseudos.push(json!({"index":owner,"pseudo":format!("::{}",pseudo["pseudoType"].as_str().unwrap()),"selector":selector,"backendNodeId":pseudo["backendNodeId"],"box":bounds}));
                }
            }
            if let Some(children)=node["children"].as_array() {
                let elements: Vec<_>=children.iter().filter(|n|n["nodeType"]==1).collect();
                for (i,child) in elements.into_iter().enumerate().rev() {
                    let path=if selector.is_empty(){":root".to_owned()}else{format!("{selector}>:nth-child({})",i+1)};
                    stack.push((child.clone(),path));
                }
            }
        }
        Ok(json!(pseudos))
    }

    pub fn evaluate_value_in_world(
        &mut self,
        world: &IsolatedWorld,
        expression: &str,
    ) -> CdpResult<Value> {
        let result = self.evaluate_with_context(expression, Some(&world.unique_id))?;
        match result {
            EvalOutcome::Value(v) => Ok(v),
            EvalOutcome::Exception(message) => Err(CdpError::new(message)),
        }
    }

    /// `page.evaluate(<expression string>)`: `Runtime.evaluate` with
    /// `awaitPromise` + `returnByValue`, in the main world.
    pub fn evaluate(&mut self, expression: &str) -> CdpResult<EvalOutcome> {
        self.evaluate_with_context(expression, None)
    }

    fn evaluate_with_context(
        &mut self,
        expression: &str,
        unique_context: Option<&str>,
    ) -> CdpResult<EvalOutcome> {
        let mut params = json!({"expression":expression,"returnByValue":true,"awaitPromise":true,"userGesture":true});
        if let Some(id) = unique_context {
            params["uniqueContextId"] = json!(id);
        }
        let res = self.send("Runtime.evaluate", params);
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                // ExecutionContext#rewriteError
                if e.message.contains("Object reference chain is too long")
                    || e.message.contains("Object couldn't be returned by value")
                {
                    return Ok(EvalOutcome::Value(Value::Null));
                }
                if e.message.ends_with("Cannot find context with specified id")
                    || e.message.ends_with("Inspected target navigated or closed")
                {
                    return Err(CdpError::new(
                        "Execution context was destroyed, most likely because of a navigation.",
                    ));
                }
                return Err(e);
            }
        };
        if let Some(details) = res.get("exceptionDetails") {
            return Ok(EvalOutcome::Exception(client_error_message(details)));
        }
        let remote = res.get("result").cloned().unwrap_or(Value::Null);
        Ok(EvalOutcome::Value(value_from_remote_object(&remote)))
    }

    /// `page.evaluate` that treats a page exception as an engine error, the
    /// way `await page.evaluate(...)` throws in the JS.
    pub fn evaluate_value(&mut self, expression: &str) -> CdpResult<Value> {
        match self.evaluate(expression)? {
            EvalOutcome::Value(v) => Ok(v),
            EvalOutcome::Exception(message) => Err(CdpError::new(message)),
        }
    }

    /// Preserve transparency when capturing isolated component paint.
    pub fn set_transparent_background(&mut self) -> CdpResult<()> {
        self.send("Emulation.setDefaultBackgroundColorOverride", json!({"color":{"r":0,"g":0,"b":0,"a":0}}))?;
        Ok(())
    }

    /// Capture the current viewport without Chromium's beyond-viewport resize.
    /// Use for observations that must not trigger responsive source selection.
    pub fn screenshot_viewport(&mut self) -> CdpResult<String> {
        let res = self.send(
            "Page.captureScreenshot",
            json!({
                "format": "png", "optimizeForSpeed": false,
                "fromSurface": true, "captureBeyondViewport": false,
            }),
        )?;
        res.get("data")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| CdpError::new("viewport screenshot returned no PNG"))
    }

    /// `page.screenshot({ encoding: 'base64', clip, captureBeyondViewport: true })`.
    /// Returns base64 PNG data.
    pub fn screenshot_clip(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> CdpResult<String> {
        // Page.screenshot: roundRectangle(normalizeRectangle(clip)).
        let (x, width) = if width < 0.0 {
            (x + width, -width)
        } else {
            (x, width)
        };
        let (y, height) = if height < 0.0 {
            (y + height, -height)
        } else {
            (y, height)
        };
        let round = |v: f64| impeccable_core::js::math_round(v);
        let res = self.send(
            "Page.captureScreenshot",
            json!({
                "format": "png",
                "optimizeForSpeed": false,
                "fromSurface": true,
                "clip": { "x": round(x), "y": round(y), "width": round(width), "height": round(height), "scale": 1 },
                "captureBeyondViewport": true,
            }),
        )?;
        Ok(res
            .get("data")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string())
    }

    /// The deduped `pageerror` messages so far.
    pub fn page_errors(&mut self) -> Vec<String> {
        self.pump_events();
        self.page_errors.clone()
    }

    /// `page.close()`: `Target.closeTarget`, errors swallowed like the JS
    /// `page.close().catch(() => {})`.
    pub fn close(self) {
        let target_id = self.target_id.clone();
        let _ = self.browser.conn.send(
            None,
            "Target.closeTarget",
            json!({ "targetId": target_id }),
            Duration::from_secs(10),
        );
        // Detached sessions may keep queued events; drop anything stale.
        self.browser.conn.events.clear();
        // Interception (origin-scoped auth, write block) dies with the page;
        // a shared browser's next page must not inherit it.
        let conn = &mut self.browser.conn;
        conn.interceptors.remove(&self.session_id);
        conn.intercept_owner
            .retain(|_, owner| *owner != self.session_id);
        if let Some(id) = &self.browser_context {
            let _ = conn.send(
                None,
                "Target.disposeBrowserContext",
                json!({ "browserContextId": id }),
                Duration::from_secs(10),
            );
        }
    }
}

/// One `console.error` argument as DevTools prints it.
fn console_arg_text(arg: &Value) -> String {
    match arg.get("value") {
        Some(Value::String(s)) => s.clone(),
        Some(v) if !v.is_null() => v.to_string(),
        _ => arg
            .get("description")
            .or_else(|| arg.get("unserializableValue"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                arg.get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            }),
    }
}

/// puppeteer `valueFromPrimitiveRemoteObject` for `returnByValue` results.
fn value_from_remote_object(remote: &Value) -> Value {
    if let Some(unser) = remote.get("unserializableValue").and_then(Value::as_str) {
        return match unser {
            "-0" => json!(-0.0),
            "NaN" | "Infinity" | "-Infinity" => Value::Null,
            other => Value::String(other.to_string()),
        };
    }
    remote.get("value").cloned().unwrap_or(Value::Null)
}

/// `String(err?.message || err)` over puppeteer's `createClientError` /
/// `createEvaluationError`: the message for an error object, the stringified
/// primitive otherwise.
pub fn client_error_message(details: &Value) -> String {
    let exception = details.get("exception");
    let Some(exception) = exception else {
        return details
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    };
    let etype = exception.get("type").and_then(Value::as_str).unwrap_or("");
    let subtype = exception
        .get("subtype")
        .and_then(Value::as_str)
        .unwrap_or("");
    let has_object_id = exception.get("objectId").is_some();
    if (etype != "object" || subtype != "error") && !has_object_id {
        // A thrown primitive: String(value).
        if let Some(unser) = exception.get("unserializableValue").and_then(Value::as_str) {
            return match unser {
                "-0" => "0".to_string(),
                other if other.ends_with('n') => other.trim_end_matches('n').to_string(),
                other => other.to_string(),
            };
        }
        return match exception.get("value") {
            None => "undefined".to_string(),
            Some(Value::Null) => "null".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Number(n)) => {
                impeccable_core::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
            }
            Some(other) => other.to_string(),
        };
    }
    // getErrorDetails
    let description = exception
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut lines: Vec<&str> = if exception.get("description").is_some() {
        description.split("\n    at ").collect()
    } else {
        Vec::new()
    };
    let frames = details
        .pointer("/stackTrace/callFrames")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let size = frames.min(lines.len().saturating_sub(1));
    if size > 0 {
        let keep = lines.len() - size;
        lines.truncate(keep);
    }
    let name = exception
        .get("className")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut message = lines.join("\n");
    if !name.is_empty() {
        let prefix = format!("{name}: ");
        if message.starts_with(&prefix) {
            message = message[prefix.len()..].to_string();
        }
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_extraction_matches_puppeteer() {
        let details = json!({
            "text": "Uncaught",
            "exception": {
                "type": "object", "subtype": "error", "className": "ReferenceError",
                "description": "ReferenceError: foo is not defined\n    at <anonymous>:1:1",
                "objectId": "1"
            },
            "stackTrace": { "callFrames": [{ "functionName": "", "url": "", "lineNumber": 0, "columnNumber": 0 }] }
        });
        assert_eq!(client_error_message(&details), "foo is not defined");
        let primitive =
            json!({ "text": "Uncaught", "exception": { "type": "string", "value": "boom" } });
        assert_eq!(client_error_message(&primitive), "boom");
        let none = json!({ "text": "Uncaught SyntaxError" });
        assert_eq!(client_error_message(&none), "Uncaught SyntaxError");
        let syntax = json!({
            "text": "Uncaught",
            "exception": { "type": "object", "subtype": "error", "className": "SyntaxError",
                "description": "SyntaxError: Unexpected token '}'", "objectId": "2" }
        });
        assert_eq!(client_error_message(&syntax), "Unexpected token '}'");
    }

    #[test]
    fn default_args_shape() {
        let args = default_chrome_args(&[], false);
        assert_eq!(args[0], "--allow-pre-commit-input");
        assert!(args.contains(&"--headless=new".to_string()));
        assert!(args.contains(&"--hide-scrollbars".to_string()));
        assert!(args.contains(&"--disable-component-update".to_string()));
        assert!(args.contains(&"--gaia-url=http://127.0.0.1:9".to_string()));
        let disabled = args
            .iter()
            .find_map(|arg| arg.strip_prefix("--disable-features="))
            .unwrap();
        assert!(disabled
            .split(',')
            .any(|feature| feature == "SafeBrowsingHashPrefixRealTimeLookups"));
        assert_eq!(args.last().unwrap(), "about:blank");
        let with = default_chrome_args(&["--no-sandbox".to_string()], false);
        assert_eq!(with.last().unwrap(), "--no-sandbox");
        assert!(with.contains(&"about:blank".to_string()));
    }
}
