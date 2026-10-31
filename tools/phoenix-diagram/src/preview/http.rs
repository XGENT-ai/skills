use super::{PreviewFailure, PreviewResult};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    time::{Duration, Instant},
};

const MAX_HEADER: usize = 8 * 1024;
const MAX_CONNECTIONS: usize = 32;
const CONNECTION_DEADLINE: Duration = Duration::from_secs(2);

struct Reply {
    head: Vec<u8>,
    body: Arc<[u8]>,
    offset: usize,
    head_only: bool,
    artifact: bool,
}
impl Reply {
    fn new(
        code: u16,
        kind: &str,
        body: Arc<[u8]>,
        head_only: bool,
        artifact: bool,
        shell: bool,
    ) -> Self {
        let label = match code {
            200 => "OK",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            408 => "Request Timeout",
            431 => "Request Header Fields Too Large",
            _ => "Service Unavailable",
        };
        let mut head=format!("HTTP/1.1 {code} {label}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nCross-Origin-Resource-Policy: same-origin\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: SAMEORIGIN\r\n",body.len());
        if code == 405 {
            head.push_str("Allow: GET, HEAD\r\n");
        }
        if shell {
            head.push_str("Content-Security-Policy: default-src 'none'; frame-src 'self'; connect-src 'self'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\r\n");
        }
        head.push_str("\r\n");
        Self {
            head: head.into_bytes(),
            body,
            offset: 0,
            head_only,
            artifact,
        }
    }
    fn error(code: u16, head_only: bool) -> Self {
        Self::new(
            code,
            "text/plain; charset=utf-8",
            Arc::from(
                format!(
                    "{}\n",
                    match code {
                        403 => "Forbidden",
                        404 => "Not found",
                        405 => "Only GET and HEAD are supported",
                        409 => "The public output no longer belongs to this preview",
                        408 => "Request timed out",
                        431 => "Request headers exceed 8 KiB",
                        _ => "Bad request",
                    }
                )
                .into_bytes(),
            ),
            head_only,
            false,
            false,
        )
    }
    fn write(&mut self, stream: &mut TcpStream) -> std::io::Result<bool> {
        let total = self.head.len() + if self.head_only { 0 } else { self.body.len() };
        while self.offset < total {
            let bytes = if self.offset < self.head.len() {
                &self.head[self.offset..]
            } else {
                &self.body[self.offset - self.head.len()..]
            };
            match stream.write(bytes) {
                Ok(0) => return Ok(true),
                Ok(count) => self.offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }
}
struct Connection {
    stream: TcpStream,
    header: Vec<u8>,
    reply: Option<Reply>,
    deadline: Instant,
}
pub(crate) struct Server {
    listener: Option<TcpListener>,
    address: SocketAddr,
    prefix: String,
    shell: Arc<[u8]>,
    connections: Vec<Connection>,
}
impl Server {
    pub fn bind(port: u16, token: &str) -> PreviewResult<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .map_err(|e| PreviewFailure::io("preview/listen", e))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| PreviewFailure::io("preview/listen", e))?;
        let address = listener
            .local_addr()
            .map_err(|e| PreviewFailure::io("preview/listen", e))?;
        Ok(Self {
            listener: Some(listener),
            address,
            prefix: format!("/{token}/"),
            shell: Arc::from(include_bytes!("../../assets/preview-shell.html").as_slice()),
            connections: vec![],
        })
    }
    pub fn url(&self) -> String {
        format!("http://{}{}", self.address, self.prefix)
    }
    pub fn port(&self) -> u16 {
        self.address.port()
    }
    pub fn poll(
        &mut self,
        status: &[u8],
        artifact: Option<&Arc<[u8]>>,
        conflict: bool,
    ) -> PreviewResult<()> {
        if let Some(listener) = &self.listener {
            for _ in 0..MAX_CONNECTIONS {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if self.connections.len() >= MAX_CONNECTIONS {
                            let _ = stream.shutdown(Shutdown::Both);
                            continue;
                        }
                        stream
                            .set_nonblocking(true)
                            .map_err(|e| PreviewFailure::io("preview/connection", e))?;
                        self.connections.push(Connection {
                            stream,
                            header: Vec::new(),
                            reply: None,
                            deadline: Instant::now() + CONNECTION_DEADLINE,
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(PreviewFailure::io("preview/accept", error)),
                }
            }
        }
        let expected = self.address.to_string();
        self.connections.retain_mut(|connection| {
            if artifact.is_none()
                && connection
                    .reply
                    .as_ref()
                    .is_some_and(|reply| reply.artifact)
            {
                let _ = connection.stream.shutdown(Shutdown::Both);
                return false;
            }
            if Instant::now() >= connection.deadline {
                // Incomplete/slow connections cannot outlive shutdown or hold
                // the controller while its worker completes.
                let _ = connection.stream.shutdown(Shutdown::Both);
                return false;
            }
            if connection.reply.is_none() {
                let mut chunk = [0; 2048];
                loop {
                    match connection.stream.read(&mut chunk) {
                        Ok(0) => {
                            let _ = connection.stream.shutdown(Shutdown::Both);
                            return false;
                        }
                        Ok(count) => {
                            connection.header.extend_from_slice(&chunk[..count]);
                            if connection.header.len() > MAX_HEADER {
                                connection.reply = Some(Reply::error(
                                    431,
                                    connection.header.starts_with(b"HEAD "),
                                ));
                                break;
                            }
                            if connection
                                .header
                                .windows(4)
                                .any(|bytes| bytes == b"\r\n\r\n")
                            {
                                connection.reply = Some(route(
                                    &connection.header,
                                    &expected,
                                    &self.prefix,
                                    &self.shell,
                                    status,
                                    artifact,
                                    conflict,
                                ));
                                break;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => {
                            let _ = connection.stream.shutdown(Shutdown::Both);
                            return false;
                        }
                    }
                }
            }
            if let Some(reply) = connection.reply.as_mut() {
                match reply.write(&mut connection.stream) {
                    Ok(false) => return true,
                    _ => {
                        let _ = connection.stream.shutdown(Shutdown::Both);
                        return false;
                    }
                }
            }
            true
        });
        Ok(())
    }
    pub fn stop(&mut self) -> bool {
        drop(self.listener.take());
        for connection in self.connections.drain(..) {
            let _ = connection.stream.shutdown(Shutdown::Both);
        }
        match TcpStream::connect_timeout(&self.address, Duration::from_millis(100)) {
            Err(error) => matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::AddrNotAvailable
                    | std::io::ErrorKind::NotConnected
            ),
            Ok(stream) => {
                let _ = stream.shutdown(Shutdown::Both);
                false
            }
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        drop(self.listener.take());
        for connection in self.connections.drain(..) {
            let _ = connection.stream.shutdown(Shutdown::Both);
        }
    }
}
fn route(
    raw: &[u8],
    host: &str,
    prefix: &str,
    shell: &Arc<[u8]>,
    status: &[u8],
    artifact: Option<&Arc<[u8]>>,
    conflict: bool,
) -> Reply {
    let Ok(header) = std::str::from_utf8(raw) else {
        return Reply::error(400, raw.starts_with(b"HEAD "));
    };
    let Some((header, trailing)) = header.split_once("\r\n\r\n") else {
        return Reply::error(400, false);
    };
    let mut lines = header.split("\r\n");
    let Some(first) = lines.next() else {
        return Reply::error(400, false);
    };
    let parts: Vec<_> = first.split(' ').collect();
    let head_only = parts.first() == Some(&"HEAD");
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1") || !trailing.is_empty() {
        return Reply::error(400, head_only);
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Reply::error(400, head_only);
        };
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.bytes().any(|b| b.is_ascii_control() && b != b'\t')
            || headers
                .insert(name.to_ascii_lowercase(), value.trim())
                .is_some()
        {
            return Reply::error(400, head_only);
        }
    }
    let origin = format!("http://{host}");
    if headers.get("host") != Some(&host)
        || headers.get("origin").is_some_and(|value| *value != origin)
    {
        return Reply::error(403, head_only);
    }
    if !matches!(parts[0], "GET" | "HEAD") {
        return Reply::error(405, head_only);
    }
    if headers.contains_key("transfer-encoding")
        || headers
            .get("content-length")
            .is_some_and(|value| *value != "0")
    {
        return Reply::error(400, head_only);
    }
    let path = parts[1].split('?').next().unwrap_or_default();
    if path == prefix {
        return Reply::new(
            200,
            "text/html; charset=utf-8",
            shell.clone(),
            head_only,
            false,
            true,
        );
    }
    if path == format!("{prefix}status") {
        return Reply::new(
            200,
            "application/json; charset=utf-8",
            Arc::from(status),
            head_only,
            false,
            false,
        );
    }
    if path == format!("{prefix}artifact") {
        return match artifact {
            Some(bytes) => Reply::new(
                200,
                "text/html; charset=utf-8",
                bytes.clone(),
                head_only,
                true,
                false,
            ),
            None => Reply::error(if conflict { 409 } else { 404 }, head_only),
        };
    }
    Reply::error(404, head_only)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_routes_reject_cross_origin_alias_paths_and_methods() {
        let shell: Arc<[u8]> = Arc::from(b"shell".as_slice());
        let artifact: Arc<[u8]> = Arc::from(b"exact artifact".as_slice());
        let request = |first: &str, headers: &str| {
            format!("{first}\r\nHost: 127.0.0.1:1234\r\n{headers}\r\n").into_bytes()
        };
        let response = |bytes: &[u8]| {
            route(
                bytes,
                "127.0.0.1:1234",
                "/secret/",
                &shell,
                b"{}",
                Some(&artifact),
                false,
            )
        };
        assert!(response(&request("GET /secret/artifact HTTP/1.1", ""))
            .head
            .starts_with(b"HTTP/1.1 200"));
        for (first, headers, code) in [
            (
                "GET /secret/status HTTP/1.1",
                "Origin: https://other.example\r\n",
                403,
            ),
            ("POST /secret/status HTTP/1.1", "", 405),
            ("GET /secret/../status HTTP/1.1", "", 404),
            ("GET /status HTTP/1.1", "", 404),
        ] {
            let reply = response(&request(first, headers));
            assert!(String::from_utf8_lossy(&reply.head).starts_with(&format!("HTTP/1.1 {code}")));
        }
    }
}
