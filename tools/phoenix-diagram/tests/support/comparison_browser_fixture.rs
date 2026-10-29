// Protocol-only comparison fixture. It reports synthetic DOM facts and blank
// PNGs; it never renders a browser and cannot prove real visual acceptance.
use super::BrowserOptions;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const VIEWS: [&str; 3] = ["before", "delta", "after"];
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct Model {
    ir: Value,
    head: Value,
}
fn empty_ir() -> Value {
    json!({"schemaVersion":1,"changes":{"components":[],"connections":[],"boundaries":[]}})
}
fn model(document_url: &str) -> Model {
    use scraper::{Html, Selector};
    let path = url::Url::parse(document_url)
        .unwrap()
        .to_file_path()
        .unwrap();
    let text = fs::read_to_string(path).unwrap();
    let document = Html::parse_document(&text);
    let selector =
        Selector::parse("script#phoenix-diagram-comparison[type='application/json']").unwrap();
    let Some(script) = document.select(&selector).next() else {
        return Model {
            ir: empty_ir(),
            head: json!({"graph":{"nodes":[{"id":"fixture-node","label":"Fixture node"}],"relations":[]},"facts":{"boundaries":[]}}),
        };
    };
    let metadata: Value = serde_json::from_str(&script.text().collect::<String>()).unwrap();
    let selector = Selector::parse("iframe[data-comparison-view='after']").unwrap();
    let frame = document.select(&selector).next().unwrap();
    let head = Html::parse_document(frame.attr("srcdoc").unwrap());
    let selector =
        Selector::parse("script#phoenix-diagram-metadata[type='application/json']").unwrap();
    let head = serde_json::from_str(
        &head
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .collect::<String>(),
    )
    .unwrap();
    Model {
        ir: metadata["comparisonIr"].clone(),
        head,
    }
}
fn paint_primitive(role: &str) -> Value {
    let edge = role == "edge";
    let height = if edge { 0 } else { 10 };
    json!({"role":role,"text":if role=="body" || edge {Value::Null}else{json!("Synthetic painted glyph")},"bounds":{"left":20,"top":20,"right":70,"bottom":20+height,"width":50,"height":height},"logicalBounds":{"x":20,"y":20,"width":50,"height":height},"display":"inline","visibility":"visible","opacity":0.35,"fill":if edge {"none"}else{"rgb(100,100,100)"},"stroke":if edge {"rgb(100,100,100)"}else{"none"},"fillOpacity":1,"strokeOpacity":1,"strokeWidth":1,"clientRects":1,"clipPath":"none","maskImage":"none","ancestors":[{"hidden":false,"display":"inline","visibility":"visible","opacity":0.42,"clipPath":"none","maskImage":"none"}],"pathLength":if edge {json!(50)}else{Value::Null}})
}
fn append_layer(
    markers: &mut Vec<Value>,
    key: &str,
    kind: &str,
    state: &str,
    ghost: bool,
    origin: Option<&str>,
    label: &str,
) {
    let mut roles = match kind {
        "component" => vec!["node"],
        "relationship" => vec!["edge", "label"],
        _ => vec!["frame", "label"],
    };
    if kind != "component"
        && state != "same"
        && (kind != "boundary" || !ghost || origin == Some("removed"))
    {
        roles.push("symbol");
    }
    for role in roles {
        let primitives = match role {
            "node" => vec![paint_primitive("body"), paint_primitive("label")],
            "frame" => vec![paint_primitive("body")],
            "edge" => vec![paint_primitive("edge")],
            "label" if label.trim().is_empty() => vec![],
            "label" => vec![paint_primitive("body"), paint_primitive("label")],
            _ => vec![paint_primitive("symbol")],
        };
        let logical = if primitives.is_empty() {
            json!({"x":0,"y":0,"width":0,"height":0})
        } else {
            json!({"x":20,"y":20,"width":50,"height":10})
        };
        markers.push(json!({"key":key,"kind":kind,"role":role,"state":state,"ghost":ghost,"origin":origin,"declaredLabel":label,"logicalBounds":logical,"primitives":primitives}));
    }
}
fn paint_data(model: &Model) -> Value {
    use std::collections::BTreeMap;
    let mut changes = BTreeMap::new();
    for (collection, kind) in [
        ("components", "component"),
        ("connections", "relationship"),
        ("boundaries", "boundary"),
    ] {
        for change in model.ir["changes"][collection].as_array().unwrap() {
            let id = change[if kind == "boundary" { "key" } else { "id" }]
                .as_str()
                .unwrap();
            changes.insert(format!("{kind}:{id}"), (kind, change));
        }
    }
    let mut markers = vec![];
    let mut head_keys = json!({"components":[],"connections":[],"boundaries":[]});
    for (collection, kind, pointer) in [
        ("components", "component", "/graph/nodes"),
        ("connections", "relationship", "/graph/relations"),
        ("boundaries", "boundary", "/facts/boundaries"),
    ] {
        for item in model.head.pointer(pointer).unwrap().as_array().unwrap() {
            let id = if kind == "boundary" {
                format!(
                    "{}:{}",
                    item["kind"].as_str().unwrap(),
                    item["label"].as_str().unwrap()
                )
            } else {
                item["id"].as_str().unwrap().to_owned()
            };
            head_keys[collection]
                .as_array_mut()
                .unwrap()
                .push(json!(id));
            let key = format!("{kind}:{id}");
            let state = changes
                .get(&key)
                .map(|(_, c)| c["status"].as_str().unwrap())
                .unwrap_or("same");
            append_layer(
                &mut markers,
                &key,
                kind,
                state,
                false,
                None,
                item["label"].as_str().unwrap_or_default(),
            );
        }
    }
    for (key, (kind, change)) in changes {
        let state = change["status"].as_str().unwrap();
        let classifications = change["classifications"].as_array().unwrap();
        let geometry = classifications.iter().any(|c| c == "geometry");
        let topology = classifications.iter().any(|c| c == "topology");
        let origin = if state == "added" {
            None
        } else if state == "removed" || kind == "relationship" && topology {
            Some("removed")
        } else if matches!(kind, "component" | "relationship") && geometry
            || kind == "boundary" && matches!(state, "changed" | "geometry-changed")
        {
            Some("moved-from")
        } else {
            None
        };
        if let Some(origin) = origin {
            let label = if kind == "component" {
                change["baseLabel"].as_str()
            } else if kind == "boundary" {
                change["label"].as_str()
            } else {
                change["base"]["label"].as_str()
            }
            .unwrap_or_default();
            append_layer(&mut markers, &key, kind, state, true, Some(origin), label);
        }
    }
    json!({"ir":model.ir,"headKeys":head_keys,"inspectionDetailLevel":"full","restoration":{"mode":"overview","scale":1,"x":0,"y":0},"svgBounds":{"x":0,"y":0,"width":200,"height":100},"markers":markers})
}

pub struct Fixture {
    directory: PathBuf,
    executable: PathBuf,
    stop: Arc<AtomicBool>,
    commands: Arc<Mutex<Vec<Value>>>,
    thread: Option<JoinHandle<()>>,
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
impl Fixture {
    pub fn new() -> Self {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let directory = std::env::temp_dir().join(format!(
            "phoenix-comparison-protocol-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!(
            "ws://{}/devtools/browser/comparison-fixture",
            listener.local_addr().unwrap()
        );
        let executable = directory.join("fixture-browser");
        let launch=format!("#!/bin/sh\nprintf '%s\\n' \"$$\" > {}\nfor argument in \"$@\"; do case \"$argument\" in --user-data-dir=*) printf '%s' \"${{argument#--user-data-dir=}}\" > {};; esac; done\nprintf '%s\\n' {} >&2\nexec /bin/sleep 30\n",shell_quote(&directory.join("child-pid").to_string_lossy()),shell_quote(&directory.join("profile-path").to_string_lossy()),shell_quote(&format!("DevTools listening on {endpoint}")));
        fs::write(&executable, launch).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let worker_commands = commands.clone();
        let thread = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
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
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_millis(250)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let Ok(mut socket) = tungstenite::accept(stream) else {
                    return;
                };
                let (mut width, mut height) = (1440u32, 900u32);
                let mut theme = "light".to_owned();
                let mut document_url = String::new();
                let mut document_model = None;
                while !worker_stop.load(Ordering::Relaxed) {
                    let message = match socket.read() {
                        Ok(tungstenite::Message::Text(message)) => message,
                        Ok(_) => continue,
                        Err(tungstenite::Error::Io(e))
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                            ) =>
                        {
                            continue
                        }
                        Err(_) => break,
                    };
                    let request: Value = serde_json::from_str(&message).unwrap();
                    worker_commands.lock().unwrap().push(request.clone());
                    let method = request["method"].as_str().unwrap();
                    let id = request["id"].clone();
                    let result = match method {
                        "Browser.getVersion" => {
                            json!({"product":"Synthetic comparison CDP fixture; no real Chrome","protocolVersion":"1.3"})
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
                            document_model.get_or_insert_with(|| model(&document_url));
                            let url = url::Url::parse(&document_url).unwrap();
                            theme = url
                                .query_pairs()
                                .find(|(key, _)| key == "theme")
                                .unwrap()
                                .1
                                .into_owned();
                            let _=socket.send(tungstenite::Message::Text(json!({"method":"Page.loadEventFired","sessionId":"fixture-session","params":{"timestamp":1}}).to_string().into()));
                            json!({"frameId":"fixture-frame"})
                        }
                        "Runtime.evaluate" => {
                            let expression = request["params"]["expression"].as_str().unwrap();
                            if let Some((_, tail)) = expression.split_once("var viewId = ") {
                                let parsed = serde_json::Deserializer::from_str(tail)
                                    .into_iter::<Value>()
                                    .next()
                                    .unwrap()
                                    .unwrap();
                                let view = parsed.as_str().unwrap();
                                assert!(VIEWS.contains(&view));
                                json!({"result":{"type":"object","value":comparison_metrics(width,height,&theme,view,&document_url,document_model.as_ref().unwrap())}})
                            } else {
                                json!({"result":{"type":"boolean","value":true}})
                            }
                        }
                        "Page.getFrameTree" => {
                            json!({"frameTree":{"frame":{"id":"fixture-frame","url":document_url}}})
                        }
                        "Page.getLayoutMetrics" => {
                            json!({"cssContentSize":{"x":0,"y":0,"width":width,"height":height.max(1500)}})
                        }
                        "Page.captureScreenshot" => {
                            use base64::Engine;
                            let image_height = if request["params"]["captureBeyondViewport"] == true
                            {
                                height.max(1500)
                            } else {
                                height
                            };
                            json!({"data":base64::engine::general_purpose::STANDARD.encode(png(width,image_height))})
                        }
                        _ => json!({}),
                    };
                    if socket
                        .send(tungstenite::Message::Text(
                            json!({"id":id,"result":result}).to_string().into(),
                        ))
                        .is_err()
                    {
                        break;
                    }
                    if method == "Browser.close" {
                        break;
                    }
                }
            }
        });
        Self {
            directory,
            executable,
            stop,
            commands,
            thread: Some(thread),
        }
    }
    pub fn options(&self, out_dir: &Path, capture: bool) -> BrowserOptions {
        BrowserOptions {
            executable: Some(self.executable.clone()),
            capture,
            out_dir: Some(out_dir.to_owned()),
            startup_timeout: Duration::from_secs(3),
            command_timeout: Duration::from_secs(3),
        }
    }
    #[allow(dead_code)]
    pub fn commands(&self) -> Vec<Value> {
        self.commands.lock().unwrap().clone()
    }
}
impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn comparison_metrics(
    width: u32,
    height: u32,
    theme: &str,
    view: &str,
    document_url: &str,
    model: &Model,
) -> Value {
    let full_height = height.max(1500);
    let frame_height = full_height - 100;
    let mut metrics = fixture_metrics(width, frame_height, theme, false);
    let nodes = model.head["graph"]["nodes"].as_array().unwrap();
    let relations = model.head["graph"]["relations"].as_array().unwrap();
    metrics["nodeCount"] = json!(nodes.len());
    metrics["edgeCount"] = json!(relations.len());
    metrics["semanticVisibility"]["expected"]["nodeIds"] =
        json!(nodes.iter().map(|n| n["id"].clone()).collect::<Vec<_>>());
    metrics["semanticVisibility"]["expected"]["relations"] = json!(relations
        .iter()
        .enumerate()
        .map(
            |(index, r)| json!({"key":index.to_string(),"id":r["id"],"from":r["from"],"to":r["to"]})
        )
        .collect::<Vec<_>>());
    let node_template = metrics["semanticVisibility"]["initial"]["nodes"][0].clone();
    metrics["semanticVisibility"]["initial"]["nodes"] = json!(nodes
        .iter()
        .map(|node| {
            let mut entry = node_template.clone();
            entry["id"] = node["id"].clone();
            entry
        })
        .collect::<Vec<_>>());
    metrics["semanticVisibility"]["initial"]["edges"] =
        metrics["semanticVisibility"]["expected"]["relations"].clone();
    let bounds = json!({"left":10,"top":10,"right":110,"bottom":40,"width":100,"height":30});
    let control = |disabled: bool| json!({"present":true,"visible":true,"disabled":disabled,"bounds":bounds,"hit":true,"focusable":true,"tabIndex":0,"focused":!disabled,"accessibleName":"Synthetic fixture control"});
    let tabs: Vec<_> = VIEWS
        .into_iter()
        .map(|id| {
            let mut c = control(false);
            c["viewId"] = json!(id);
            c["selected"] = json!(id == view);
            c["tabIndex"] = json!(if id == view { 0 } else { -1 });
            c["focusable"] = json!(id == view);
            c
        })
        .collect();
    let mut changes = vec![];
    for (collection, kind) in [
        ("components", "component"),
        ("connections", "relationship"),
        ("boundaries", "boundary"),
    ] {
        for change in model.ir["changes"][collection].as_array().unwrap() {
            let id = change[if kind == "boundary" { "key" } else { "id" }]
                .as_str()
                .unwrap();
            let mut entry = control(false);
            entry["key"] = json!(format!("{kind}:{id}"));
            changes.push(entry);
        }
    }
    let locations: Vec<_> = if changes.len() > 1 {
        vec![&changes[0], &changes[changes.len() - 1]]
    } else {
        changes.iter().collect()
    };
    let locations:Vec<_>=locations.iter().map(|change|json!({"key":change["key"],"ariaCurrent":"step","selectedView":"delta","highlightedKeys":[change["key"]],"painted":true})).collect();
    let navigation: Vec<_> = ["overview", "previous", "next"]
        .into_iter()
        .map(|id| {
            let mut c = control(true);
            c["action"] = json!(id);
            c
        })
        .collect();
    metrics["documentUrl"] = json!("about:srcdoc");
    metrics["comparisonFrameScrollHeight"] = json!(frame_height);
    metrics["frameViewport"] = json!({"width":width,"height":frame_height});
    metrics["fullScrollHeight"] = json!(full_height);
    metrics["comparisonParent"] = json!({"viewId":view,"metadataValid":true,"ready":true,"activeFrameCount":1,"frameViewIds":VIEWS,"innerWidth":width,"innerHeight":height,"scrollWidth":width,"fullScrollHeight":full_height,"scrollY":0,"resolvedTheme":theme,"documentUrl":document_url,"frame":{"viewId":view,"ready":true,"clientWidth":width,"clientHeight":frame_height,"contentUnclipped":true,"bounds":{"left":0,"top":100,"right":width,"bottom":full_height,"width":width,"height":frame_height}},"tabs":tabs,"changes":changes,"navigation":navigation,"locationChecks":locations,"expectedChanges":changes.len(),"emptyState":changes.is_empty(),"resources":[]});
    if view == "delta" {
        metrics["comparisonDeltaPaint"] = paint_data(model);
    }
    metrics
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
