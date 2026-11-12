//! Static server for local capture targets: absolute asset paths and module
//! scripts work as they do on a dev server, which `file://` does not give.
//! Loopback only, GET/HEAD only, never outside the root.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Server {
    host: String,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub fn url_for(&self, entry: &str) -> String {
        format!("http://{}/{}", self.host, encode_path(entry))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn serve(root: &Path) -> Result<Server, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let host = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .to_string();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let stop = Arc::new(AtomicBool::new(false));
    let (worker_stop, worker_host, root) = (stop.clone(), host.clone(), root.to_path_buf());
    let worker = thread::spawn(move || {
        while !worker_stop.load(Ordering::Acquire) {
            match listener.accept() {
                // One thread per connection: a browser may open a socket
                // early and send nothing on it for a while.
                Ok((mut stream, _)) => {
                    let (host, root) = (worker_host.clone(), root.clone());
                    thread::spawn(move || {
                        let _ = respond(&mut stream, &host, &root);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(_) => break,
            }
        }
    });
    Ok(Server {
        host,
        stop,
        worker: Some(worker),
    })
}

/// The file a request path names under `root`, or None when it must not be
/// served (traversal, dot segments, NUL, backslashes, undecodable bytes).
pub fn resolve(root: &Path, target: &str) -> Option<PathBuf> {
    let raw = target.strip_prefix('/')?.split(['?', '#']).next()?;
    let mut bytes = Vec::new();
    let input = raw.as_bytes();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            let hex = std::str::from_utf8(input.get(i + 1..i + 3)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(input[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(bytes).ok()?;
    if decoded.contains(['\0', '\\']) {
        return None;
    }
    let segments: Vec<&str> = decoded.split('/').collect();
    let last = segments.len() - 1;
    for (n, segment) in segments.iter().enumerate() {
        if segment.starts_with('.') || (segment.is_empty() && n != last) {
            return None;
        }
    }
    let mut path = root.join(decoded.trim_end_matches('/'));
    if path.is_dir() {
        path = path.join("index.html");
    }
    Some(path)
}

fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
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

/// The file and its content a request path may read, or None when it must
/// not be served (traversal, dot segments, NUL, backslashes, undecodable
/// bytes, missing, or a symlink that resolves outside the root).
fn read_within(root: &Path, target: &str) -> Option<(PathBuf, Vec<u8>)> {
    let path = resolve(root, target)?;
    // Symlinks must not widen the served tree: the real path of the file has
    // to stay inside the real root (canonicalize both, so a symlinked root
    // itself still works).
    let real_root = std::fs::canonicalize(root).ok()?;
    let real = std::fs::canonicalize(&path).ok()?;
    if !real.starts_with(&real_root) {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    Some((path, bytes))
}

fn respond(stream: &mut TcpStream, host: &str, root: &Path) -> std::io::Result<()> {
    // Accepted sockets can inherit O_NONBLOCK from the listener on macOS.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 2048];
    while bytes.len() <= 16384 && !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let request = String::from_utf8_lossy(&bytes);
    let mut lines = request.split("\r\n");
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let (method, target) = (first.next().unwrap_or(""), first.next().unwrap_or(""));
    let host_ok = lines
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.trim())
        .eq([host]);
    let (status, kind, body): (&str, &str, Vec<u8>) = if !matches!(method, "GET" | "HEAD") {
        (
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed".to_vec(),
        )
    } else if !host_ok {
        ("400 Bad Request", "text/plain", b"Bad host".to_vec())
    } else {
        match read_within(root, target) {
            Some((path, body)) => ("200 OK", content_type(&path), body),
            None => ("404 Not Found", "text/plain", b"Not found".to_vec()),
        }
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    )?;
    if method != "HEAD" {
        stream.write_all(&body)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rejects_traversal_and_dot_segments() {
        let root = Path::new("/srv/app");
        assert_eq!(resolve(root, "/a/b.css?v=1"), Some(root.join("a/b.css")));
        assert_eq!(
            resolve(root, "/%E4%BB%BB%E5%8A%A1.png"),
            Some(root.join("任务.png"))
        );
        for bad in [
            "/../etc/passwd",
            "/a/%2e%2e/b",
            "/.git/config",
            "/a/.env",
            "/a%5Cb",
            "/a%00",
            "/a//b",
            "noslash",
        ] {
            assert_eq!(resolve(root, bad), None, "{bad}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_stay_inside_the_root() {
        let dir =
            std::env::temp_dir().join(format!("phoenix-serve-symlinks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("app");
        let outside = dir.join("outside");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        // Synthetic content only: no real private files are touched.
        std::fs::write(outside.join("secret.txt"), "synthetic-marker").unwrap();
        std::fs::write(root.join("real.txt"), "inside").unwrap();
        std::fs::create_dir_all(root.join("real-dir")).unwrap();
        std::fs::write(root.join("real-dir/file.txt"), "nested").unwrap();
        std::os::unix::fs::symlink("../outside/secret.txt", root.join("file-link.txt")).unwrap();
        std::os::unix::fs::symlink("../outside", root.join("dir-link")).unwrap();
        std::os::unix::fs::symlink("real.txt", root.join("inside-link.txt")).unwrap();

        let read = |target: &str| {
            read_within(&root, target).and_then(|(_, bytes)| String::from_utf8(bytes).ok())
        };
        assert_eq!(read("/real.txt").as_deref(), Some("inside"));
        assert_eq!(read("/inside-link.txt").as_deref(), Some("inside"));
        // Links out of the root are refused; the page cannot read them.
        assert_eq!(read("/file-link.txt"), None);
        assert_eq!(read("/dir-link/secret.txt"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
