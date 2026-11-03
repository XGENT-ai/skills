//! Immutable, explicit static-HTML inputs for native capture, served from a
//! loopback port. Not a framework server. The native caller owns the
//! selection; private bound inputs are never HTTP routes.
use sha2::{Digest, Sha256};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const MAX_FILES: usize = 1024;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 128 * 1024 * 1024;

pub struct SnapshotSelection {
    pub root: PathBuf,
    pub entry: String,
    /// Explicit local static dependencies, including the entry. No directory crawling.
    pub served: Vec<String>,
    /// Spec, reference and other gate inputs. Hashed but never served unless also in served.
    pub bound: Vec<String>,
}
pub struct HtmlSnapshot {
    entry: String,
    served: BTreeSet<String>,
    files: BTreeMap<String, Vec<u8>>,
    manifest: Value,
    digest: String,
}
impl HtmlSnapshot {
    /// Reuse a caller-owned immutable snapshot. The caller must retain and verify
    /// the source binding before committing evidence; this constructor performs no disk reads.
    pub fn from_pinned(entry: String, files: BTreeMap<String, Vec<u8>>) -> Result<Self, String> {
        valid_relative(&entry)?;
        if !matches!(
            Path::new(&entry).extension().and_then(|s| s.to_str()),
            Some("html" | "htm")
        ) || !files.contains_key(&entry)
        {
            return Err("pinned snapshot requires its HTML entry".into());
        }
        if files.len() > MAX_FILES || files.values().map(Vec::len).sum::<usize>() > MAX_TOTAL_BYTES
        {
            return Err("snapshot exceeds capture budget".into());
        }
        for (name, bytes) in &files {
            valid_relative(name)?;
            if mime(name).is_none() || bytes.len() as u64 > MAX_FILE_BYTES {
                return Err(format!("unsupported capture input: {name}"));
            }
        }
        let served = files.keys().cloned().collect();
        Self::new(entry, served, files)
    }

    fn new(
        entry: String,
        served: BTreeSet<String>,
        files: BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, String> {
        let manifest = json!({"schema":"native-html-input-snapshot-v1","entry":entry,
            "files":files.iter().map(|(name,bytes)|json!({"path":name,"sha256":hash(bytes),"bytes":bytes.len(),"served":served.contains(name)})).collect::<Vec<_>>()});
        let digest = hash(&serde_json::to_vec(&manifest).map_err(|e| e.to_string())?);
        Ok(Self {
            entry,
            served,
            files,
            manifest,
            digest,
        })
    }

    pub fn freeze(selection: SnapshotSelection) -> Result<Self, String> {
        let root = fs::canonicalize(&selection.root).map_err(|e| format!("snapshot root: {e}"))?;
        if !root.is_dir() {
            return Err("snapshot root is not a directory".into());
        }
        valid_relative(&selection.entry)?;
        if !matches!(
            Path::new(&selection.entry)
                .extension()
                .and_then(|s| s.to_str()),
            Some("html" | "htm")
        ) {
            return Err(
                "snapshot requires an HTML entry; framework build binding is unsupported".into(),
            );
        }
        let served: BTreeSet<_> = selection.served.into_iter().collect();
        if !served.contains(&selection.entry) {
            return Err("selected entry is not served".into());
        }
        for name in &served {
            valid_relative(name)?;
            if name.split('/').any(|p| p.starts_with('.')) || mime(name).is_none() {
                return Err(format!("not an allowed static dependency: {name}"));
            }
        }
        let names: BTreeSet<_> = served.iter().cloned().chain(selection.bound).collect();
        if names.len() > MAX_FILES {
            return Err("snapshot exceeds file limit".into());
        }
        let mut files = BTreeMap::new();
        let mut total = 0;
        for name in names {
            let bytes = read_input(&root, &name)?;
            total += bytes.len();
            if total > MAX_TOTAL_BYTES {
                return Err("snapshot exceeds total byte limit".into());
            }
            files.insert(name, bytes);
        }
        Self::new(selection.entry, served, files)
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn manifest(&self) -> &Value {
        &self.manifest
    }
    pub fn entry(&self) -> &str {
        &self.entry
    }
    pub fn bytes(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(Vec::as_slice)
    }
    pub fn is_served(&self, name: &str) -> bool {
        self.served.contains(name)
    }
    /// Strict origin-form routes. Decode percent-encoded UTF-8, but never separators.
    pub fn serve_path(&self, target: &str) -> Option<String> {
        let raw = target.strip_prefix('/')?.split('?').next()?;
        let mut bytes = Vec::new();
        let input = raw.as_bytes();
        let mut i = 0;
        while i < input.len() {
            if input[i] == b'%' {
                let hex = std::str::from_utf8(input.get(i + 1..i + 3)?).ok()?;
                let byte = u8::from_str_radix(hex, 16).ok()?;
                if matches!(byte, b'/' | b'\\' | 0) {
                    return None;
                }
                bytes.push(byte);
                i += 3;
            } else {
                bytes.push(input[i]);
                i += 1;
            }
        }
        let name = String::from_utf8(bytes).ok()?;
        valid_relative(&name).ok()?;
        self.served.contains(&name).then_some(name)
    }
    /// Component previews: a passive CSP; scripts, frames and external
    /// resources are not executable.
    pub fn serve(self: &Arc<Self>) -> Result<SnapshotServer, String> {
        self.serve_with(Some(""))
    }
    /// Assembled-page review executes only the pinned page's scripts. Network
    /// APIs, frames, workers and form submission remain disabled by CSP.
    pub fn serve_assembled(self: &Arc<Self>) -> Result<SnapshotServer, String> {
        self.serve_with(Some(
            "script-src 'self' 'unsafe-inline'; worker-src 'none'; ",
        ))
    }
    /// Raster asset capture renders the entry as the project ships it, without a CSP.
    pub fn serve_open(self: &Arc<Self>) -> Result<SnapshotServer, String> {
        self.serve_with(None)
    }
    fn serve_with(
        self: &Arc<Self>,
        script_policy: Option<&'static str>,
    ) -> Result<SnapshotServer, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let host = addr.to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let snapshot = self.clone();
        let worker_host = host.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = respond(&mut stream, &worker_host, &snapshot, script_policy);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(SnapshotServer {
            host,
            entry: self.entry.clone(),
            stop,
            worker: Some(worker),
        })
    }
}
fn valid_relative(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains(['\\', '\0', '?', '#', ':'])
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("invalid snapshot path: {name}"));
    }
    Ok(())
}
fn read_input(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    valid_relative(name)?;
    let path = root.join(name);
    let meta = fs::metadata(&path).map_err(|e| format!("snapshot input {name}: {e}"))?;
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return Err(format!("unsupported or oversized input: {name}"));
    }
    fs::read(&path).map_err(|e| format!("snapshot input {name}: {e}"))
}
fn mime(name: &str) -> Option<&'static str> {
    Some(match Path::new(name).extension()?.to_str()? {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => return None,
    })
}
fn encode_path(path: &str) -> String {
    path.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
pub struct SnapshotServer {
    host: String,
    entry: String,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl SnapshotServer {
    pub fn entry_url(&self) -> String {
        format!("http://{}/{}", self.host, encode_path(&self.entry))
    }
}
impl Drop for SnapshotServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn respond(
    stream: &mut TcpStream,
    host: &str,
    snapshot: &HtmlSnapshot,
    script_policy: Option<&str>,
) -> Result<(), std::io::Error> {
    // BSD/macOS can inherit O_NONBLOCK from the listening socket. Explicitly
    // switch accepted streams back before write_all; otherwise large bodies
    // stop at EWOULDBLOCK and appear as valid-header/truncated-image responses.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1024];
    while bytes.len() <= 8192 && !bytes.windows(4).any(|x| x == b"\r\n\r\n") {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let request = std::str::from_utf8(&bytes).unwrap_or("");
    let mut lines = request.split("\r\n");
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next();
    let target = first.next();
    let protocol = first.next();
    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(key, _)| key.eq_ignore_ascii_case("host"))
        .map(|(_, value)| value.trim())
        .collect();
    let valid = bytes.len() <= 8192
        && method == Some("GET")
        && protocol == Some("HTTP/1.1")
        && first.next().is_none()
        && hosts == [host];
    let route = if valid {
        target.and_then(|t| snapshot.serve_path(t))
    } else {
        None
    };
    let (status, kind, body) = match route.as_deref() {
        Some(name) => ("200 OK", mime(name).unwrap(), snapshot.bytes(name).unwrap()),
        None => ("404 Not Found", "text/plain", b"Not found".as_slice()),
    };
    let csp = script_policy.map(|scripts| format!("Content-Security-Policy: default-src 'none'; {scripts}style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n")).unwrap_or_default();
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: private, max-age=3600, immutable\r\nX-Content-Type-Options: nosniff\r\n{csp}\r\n",
        body.len()
    )?;
    stream.write_all(body)
}
