// Protocol-only fixtures; never substitute for real Chrome or image review.
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn launcher(root: &Path, endpoint: Option<&str>) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join("fixture-browser");
    let profile = root.join("profile-path");
    let pid = root.join("child-pid");
    let args = root.join("launch-args");
    let source = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$$\" > {}\nprintf '%s\\n' \"$@\" > {}\nfor argument in \"$@\"; do\n case \"$argument\" in --user-data-dir=*) printf '%s' \"${{argument#--user-data-dir=}}\" > {};; esac\ndone\n{}\nexec /bin/sleep 30\n",
        shell_quote(&pid.to_string_lossy()),
        shell_quote(&args.to_string_lossy()),
        shell_quote(&profile.to_string_lossy()),
        endpoint
            .map(|endpoint| format!(
                "printf '%s\\n' {} >&2",
                shell_quote(&format!("DevTools listening on {endpoint}"))
            ))
            .unwrap_or_default()
    );
    fs::write(&path, source).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

pub(crate) fn assert_owned_browser_cleaned(root: &Path) {
    let profile = PathBuf::from(fs::read_to_string(root.join("profile-path")).unwrap());
    assert!(!profile.exists(), "profile retained: {}", profile.display());
    let pid = fs::read_to_string(root.join("child-pid"))
        .unwrap()
        .trim()
        .parse::<i32>()
        .unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "fixture child still alive"
    );
    let args = fs::read_to_string(root.join("launch-args")).unwrap();
    assert!(args.contains("--headless=new"));
    assert!(!args.contains("--no-sandbox"));
    assert!(!args.contains("--disable-setuid-sandbox"));
    assert_eq!(
        args.lines()
            .filter(|line| line.starts_with("--user-data-dir="))
            .count(),
        1
    );
}

#[derive(Clone)]
pub(crate) enum Mode {
    ProtocolError,
    ProtocolTimeout,
    Complete {
        remote: bool,
        change_artifact: Option<PathBuf>,
        long: bool,
    },
}

// This fixture tests protocol/receipt branches. It does not render DOM or stand
// in for the separately scheduled real-Chrome acceptance.
pub(crate) struct ProtocolFixture {
    pub(crate) endpoint: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) commands: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ProtocolFixture {
    pub(crate) fn new(mode: Mode) -> Self {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        };
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!(
            "ws://{}/devtools/browser/fixture",
            listener.local_addr().unwrap()
        );
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let worker_commands = commands.clone();
        let thread = std::thread::spawn(move || {
            let stream = loop {
                if worker_stop.load(Ordering::Relaxed) {
                    return;
                }
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => return,
                }
            };
            // macOS inherits O_NONBLOCK from the listener; CDP response writes
            // must use the accepted stream's blocking, deadline-bounded mode.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(250)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let Ok(mut socket) = tungstenite::accept(stream) else {
                return;
            };
            let (mut width, mut height) = (1440, 900);
            let mut theme = "light".to_owned();
            let mut document_url = String::new();
            let mut changed = false;
            while !worker_stop.load(Ordering::Relaxed) {
                let message = match socket.read() {
                    Ok(tungstenite::Message::Text(text)) => text,
                    Ok(_) => continue,
                    Err(tungstenite::Error::Io(e))
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        continue;
                    }
                    Err(error) => {
                        eprintln!("Protocol fixture read failed: {error}");
                        break;
                    }
                };
                let request: serde_json::Value = serde_json::from_str(&message).unwrap();
                worker_commands.lock().unwrap().push(request.clone());
                let method = request["method"].as_str().unwrap();
                let id = request["id"].clone();
                if method == "Browser.getVersion" && matches!(mode, Mode::ProtocolTimeout) {
                    continue;
                }
                if method == "Browser.getVersion" && matches!(mode, Mode::ProtocolError) {
                    let _=socket.send(tungstenite::Message::Text(json!({"id":id,"error":{"code":-32000,"message":"fixture protocol rejection"}}).to_string().into()));
                    continue;
                }
                let result = match method {
                    "Browser.getVersion" => {
                        json!({"product":"FixtureChrome/protocol-only","protocolVersion":"1.3"})
                    }
                    "Target.createTarget" => json!({"targetId":"fixture-target"}),
                    "Target.attachToTarget" => json!({"sessionId":"fixture-session"}),
                    "Emulation.setDeviceMetricsOverride" => {
                        width = request["params"]["width"].as_u64().unwrap() as u32;
                        height = request["params"]["height"].as_u64().unwrap() as u32;
                        json!({})
                    }
                    "Page.navigate" => {
                        document_url = request["params"]["url"].as_str().unwrap().to_owned();
                        let url =
                            url::Url::parse(request["params"]["url"].as_str().unwrap()).unwrap();
                        theme = url
                            .query_pairs()
                            .find(|(key, _)| key == "theme")
                            .unwrap()
                            .1
                            .into_owned();
                        let _=socket.send(tungstenite::Message::Text(json!({"method":"Page.loadEventFired","sessionId":"fixture-session","params":{"timestamp":1}}).to_string().into()));
                        if matches!(mode, Mode::Complete { remote: true, .. }) {
                            let _=socket.send(tungstenite::Message::Text(json!({"method":"Network.requestWillBeSent","sessionId":"fixture-session","params":{"request":{"url":"https://example.invalid/tracker"}}}).to_string().into()));
                        }
                        json!({"frameId":"fixture-frame"})
                    }
                    "Runtime.evaluate" => {
                        if !changed {
                            if let Mode::Complete {
                                change_artifact: Some(path),
                                ..
                            } = &mode
                            {
                                fs::write(path, b"changed during browser inspection").unwrap();
                                changed = true;
                            }
                        }
                        let mut metrics = fixture_metrics(
                            width,
                            height,
                            &theme,
                            matches!(mode, Mode::Complete { long: true, .. }),
                        );
                        metrics["documentUrl"] = json!(document_url);
                        json!({"result":{"type":"object","value":metrics}})
                    }
                    "Page.getFrameTree" => {
                        json!({"frameTree":{"frame":{"id":"fixture-frame","url":document_url}}})
                    }
                    "Page.getLayoutMetrics" => {
                        json!({"cssContentSize":{"x":0,"y":0,"width":width,"height":height.max(1500)}})
                    }
                    "Page.captureScreenshot" => {
                        use base64::Engine;
                        let image_height = if request["params"]["captureBeyondViewport"] == true {
                            height.max(1500)
                        } else {
                            height
                        };
                        json!({"data":base64::engine::general_purpose::STANDARD.encode(png(width,image_height))})
                    }
                    _ => json!({}),
                };
                if let Err(error) = socket.send(tungstenite::Message::Text(
                    json!({"id":id,"result":result}).to_string().into(),
                )) {
                    eprintln!("Protocol fixture response for {method} failed: {error}");
                    break;
                }
                if method == "Browser.close" {
                    break;
                }
            }
        });
        Self {
            endpoint,
            stop,
            commands,
            thread: Some(thread),
        }
    }
}
impl Drop for ProtocolFixture {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn fixture_metrics(width: u32, height: u32, theme: &str, long: bool) -> serde_json::Value {
    let bounds =
        json!({"x":10,"y":10,"left":10,"top":10,"right":210,"bottom":110,"width":200,"height":100});
    let controls:Vec<_>=(0..7).map(|index|json!({"selector":format!("fixture-control-{index}"),"present":true,"visible":true,"disabled":false,"hit":true,"focusable":true,"accessibleName":"Fixture control","bounds":bounds})).collect();
    // This reports a fixed protocol payload, not measured browser visibility.
    let semantic_visibility = json!({"ok":true,"expected":{"nodeIds":["fixture-node"],"relations":[]},"initialDetailLevel":"read","inspectionDetailLevel":"full","initial":{"nodes":[{"id":"fixture-node","visible":true,"bodies":[{"visible":true,"bounds":bounds,"reasons":[],"length":null}],"labels":[{"text":"Fixture node","sourcePath":"/components/0/label","visible":true,"bounds":bounds,"reasons":[],"length":null}]}],"edges":[],"text":[{"text":"Fixture node","sourcePath":"/components/0/label","visible":true,"bounds":bounds,"reasons":[],"length":null}],"context":[],"fine":[]},"inspection":{"context":[],"fine":[]},"restoration":{"camera":{"scale":1,"x":0,"y":0,"mode":"overview"},"previousCamera":{"scale":1,"x":0,"y":0,"mode":"overview"},"detailLevel":"read","bounds":bounds,"previousBounds":bounds,"fittedBounds":bounds,"ok":true},"failures":[]});
    json!({"innerWidth":width,"innerHeight":height,"scrollWidth":width,"scrollHeight":if long{height.max(1500)}else{height},"fullScrollHeight":if long{height.max(1500)}else{height},"resolvedTheme":theme,"readerLayout":"adaptive","readerOverflow":"authored","readerFit":"intrinsic-height","diagramType":"architecture","documentScrollUnclipped":true,"readerFlow":{"unclipped":true,"notesBelowDiagram":false,"notes":[],"excludedHeight":0,"failures":[]},"readerRect":bounds,"panelRect":bounds,"svgRect":bounds,"stageRect":bounds,"viewBox":{"x":0,"y":0,"width":200,"height":100},"svgCount":1,"nodeCount":1,"edgeCount":0,"semanticVisibility":semantic_visibility,"minimumProjectedNodeTextPx":7,"minimumProjectedNodeText":{"text":"Fixture node","projectedFontPx":7},"text":[{"text":"Fixture node","projectedFontPx":7,"bounds":bounds}],"nodeTextClips":[],"canvasTextClips":[],"cards":[],"controls":controls,"fontsStatus":"loaded","images":[],"resources":[],"media":[],"capabilities":{"canvas":true,"mediaRecorder":false},"motion":"still","hasLegend":false,"hasNavigationDock":true,"legendDockIntersectionArea":0,"dockStageIntersectionArea":0,"dockStageGap":20,"viewerChromeRequiredGap":10})
}

// Small valid grayscale PNG, encoded without an image/runtime dependency.
fn png(width: u32, height: u32) -> Vec<u8> {
    fn crc(bytes: &[u8]) -> u32 {
        let mut value = !0u32;
        for byte in bytes {
            value ^= u32::from(*byte);
            for _ in 0..8 {
                value = (value >> 1) ^ if value & 1 != 0 { 0xedb88320 } else { 0 };
            }
        }
        !value
    }
    fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(data);
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        output.extend_from_slice(&crc(&body).to_be_bytes());
    }
    let raw = vec![0u8; ((width as usize).div_ceil(8) + 1) * height as usize];
    let mut zlib = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(chunk) = chunks.next() {
        zlib.push(if chunks.peek().is_none() { 1 } else { 0 });
        let length = chunk.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut result = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = width.to_be_bytes().to_vec();
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[1, 0, 0, 0, 0]);
    chunk(&mut result, b"IHDR", &header);
    chunk(&mut result, b"IDAT", &zlib);
    chunk(&mut result, b"IEND", &[]);
    result
}
